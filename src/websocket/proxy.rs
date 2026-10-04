use base64::Engine;
use reqwest::Url;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::errors::TikTokLiveError;
use crate::structs::config::proxy_port;

enum Credentials {
    Anonymous,
    UserPass { user: String, pass: String },
}

pub async fn open_tunnel(proxy_url: &str, host: &str, port: u16) -> Result<TcpStream, TikTokLiveError> {
    let url = Url::parse(proxy_url).map_err(|e| TikTokLiveError::InvalidUrl(format!("proxy url {proxy_url}: {e}")))?;
    let Some(proxy_host) = url.host_str() else {
        return Err(TikTokLiveError::InvalidUrl(format!("proxy url has no host: {proxy_url}")));
    };
    let kind = match url.scheme() {
        "http" | "https" => ProxyKind::HttpConnect,
        "socks5" | "socks5h" => ProxyKind::Socks5,
        other => return Err(TikTokLiveError::InvalidUrl(format!("unsupported proxy scheme '{other}' (http, https, socks5, socks5h)"))),
    };
    let credentials = credentials(&url)?;
    let mut tcp = TcpStream::connect((proxy_host, proxy_port(&url))).await?;
    match kind {
        ProxyKind::HttpConnect => http_connect(&mut tcp, host, port, &credentials).await?,
        ProxyKind::Socks5 => socks5_connect(&mut tcp, host, port, &credentials).await?,
    }
    Ok(tcp)
}

enum ProxyKind {
    HttpConnect,
    Socks5,
}

fn credentials(url: &Url) -> Result<Credentials, TikTokLiveError> {
    if url.username().is_empty() {
        return Ok(Credentials::Anonymous);
    }
    let decode = |raw: &str| urlencoding::decode(raw).map(|s| s.into_owned()).map_err(|e| TikTokLiveError::InvalidUrl(format!("proxy credentials: {e}")));
    let mut pass = String::new();
    for raw in url.password().iter() {
        pass = decode(raw)?;
    }
    Ok(Credentials::UserPass { user: decode(url.username())?, pass })
}

async fn http_connect(tcp: &mut TcpStream, host: &str, port: u16, credentials: &Credentials) -> Result<(), TikTokLiveError> {
    let mut request = format!("CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n");
    if let Credentials::UserPass { user, pass } = credentials {
        let token = base64::engine::general_purpose::STANDARD.encode(format!("{user}:{pass}"));
        request.push_str(&format!("Proxy-Authorization: Basic {token}\r\n"));
    }
    request.push_str("\r\n");
    tcp.write_all(request.as_bytes()).await?;

    let mut buf = vec![0u8; 4096];
    let mut total = 0usize;
    loop {
        let n = tcp.read(&mut buf[total..]).await?;
        if n == 0 {
            return Err(TikTokLiveError::Proxy("proxy closed connection during CONNECT".into()));
        }
        total += n;
        if buf[..total].windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if total >= buf.len() {
            return Err(TikTokLiveError::Proxy("proxy response headers too large".into()));
        }
    }
    let head = String::from_utf8(buf[..total].to_vec()).map_err(|e| TikTokLiveError::Proxy(format!("proxy response not utf8: {e}")))?;
    let Some(status_line) = head.lines().next() else {
        return Err(TikTokLiveError::Proxy("proxy returned an empty response".into()));
    };
    match status_line.split_whitespace().nth(1) {
        Some("200") => Ok(()),
        Some(code) => Err(TikTokLiveError::Proxy(format!("CONNECT rejected with {code}: {status_line}"))),
        None => Err(TikTokLiveError::Proxy(format!("malformed CONNECT response: {status_line}"))),
    }
}

async fn socks5_connect(tcp: &mut TcpStream, host: &str, port: u16, credentials: &Credentials) -> Result<(), TikTokLiveError> {
    let greeting: &[u8] = match credentials {
        Credentials::Anonymous => &[0x05, 0x01, 0x00],
        Credentials::UserPass { .. } => &[0x05, 0x02, 0x00, 0x02],
    };
    tcp.write_all(greeting).await?;
    let mut choice = [0u8; 2];
    tcp.read_exact(&mut choice).await?;
    match (choice, credentials) {
        ([0x05, 0x00], _) => {}
        ([0x05, 0x02], Credentials::UserPass { user, pass }) => socks5_auth(tcp, user, pass).await?,
        (other, _) => return Err(TikTokLiveError::Proxy(format!("socks5: no acceptable auth method (reply {other:02x?})"))),
    }

    let host_len = u8::try_from(host.len()).map_err(|e| TikTokLiveError::Proxy(format!("socks5: host name too long: {e}")))?;
    let mut request = vec![0x05, 0x01, 0x00, 0x03, host_len];
    request.extend_from_slice(host.as_bytes());
    request.extend_from_slice(&port.to_be_bytes());
    tcp.write_all(&request).await?;

    let mut reply = [0u8; 4];
    tcp.read_exact(&mut reply).await?;
    if reply[1] != 0x00 {
        return Err(TikTokLiveError::Proxy(format!("socks5 connect failed with reply code {}", reply[1])));
    }
    let bound_len = match reply[3] {
        0x01 => 4,
        0x04 => 16,
        0x03 => usize::from(tcp.read_u8().await?),
        other => return Err(TikTokLiveError::Proxy(format!("socks5: unknown address type {other}"))),
    };
    let mut bound = vec![0u8; bound_len + 2];
    tcp.read_exact(&mut bound).await?;
    Ok(())
}

async fn socks5_auth(tcp: &mut TcpStream, user: &str, pass: &str) -> Result<(), TikTokLiveError> {
    let too_long = |e: std::num::TryFromIntError| TikTokLiveError::Proxy(format!("socks5: credentials too long: {e}"));
    let mut auth = vec![0x01, u8::try_from(user.len()).map_err(too_long)?];
    auth.extend_from_slice(user.as_bytes());
    auth.push(u8::try_from(pass.len()).map_err(too_long)?);
    auth.extend_from_slice(pass.as_bytes());
    tcp.write_all(&auth).await?;
    let mut status = [0u8; 2];
    tcp.read_exact(&mut status).await?;
    if status[1] != 0x00 {
        return Err(TikTokLiveError::Proxy("socks5: username/password rejected".into()));
    }
    Ok(())
}

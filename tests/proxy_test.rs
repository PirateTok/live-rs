mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{http_fixture, ok, ttwid_cookie};
use piratetok_live_rs::errors::TikTokLiveError;
use piratetok_live_rs::http::ttwid::{fetch_ttwid, TtwidRequest};
use piratetok_live_rs::websocket::proxy::open_tunnel;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

async fn echo(mut sock: TcpStream) {
    let mut buf = [0u8; 64];
    loop {
        let n = sock.read(&mut buf).await.unwrap();
        if n == 0 {
            return;
        }
        sock.write_all(&buf[..n]).await.unwrap();
    }
}

async fn ping(mut tunnel: TcpStream) {
    tunnel.write_all(b"ping").await.unwrap();
    let mut back = [0u8; 4];
    tunnel.read_exact(&mut back).await.unwrap();
    assert_eq!(&back, b"ping");
}

async fn http_proxy(reply: &'static str) -> (u16, Arc<Mutex<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen = Arc::new(Mutex::new(String::new()));
    let record = seen.clone();
    tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 2048];
        let mut read = 0;
        while !buf[..read].windows(4).any(|w| w == b"\r\n\r\n") {
            read += sock.read(&mut buf[read..]).await.unwrap();
        }
        *record.lock().unwrap() = String::from_utf8_lossy(&buf[..read]).to_string();
        sock.write_all(reply.as_bytes()).await.unwrap();
        echo(sock).await;
    });
    (port, seen)
}

#[tokio::test]
async fn http_connect_with_basic_auth() {
    let (port, seen) = http_proxy("HTTP/1.1 200 Connection established\r\n\r\n").await;
    let tunnel = open_tunnel(&format!("http://us%40er:p%3Ass@127.0.0.1:{port}"), "example.test", 443).await.unwrap();
    ping(tunnel).await;
    let head = seen.lock().unwrap().clone();
    assert!(head.starts_with("CONNECT example.test:443 HTTP/1.1\r\n"), "{head}");
    assert!(head.contains("Proxy-Authorization: Basic dXNAZXI6cDpzcw==\r\n"), "us@er:p:ss decoded then base64: {head}");
}

#[tokio::test]
async fn http_connect_rejection_is_an_error() {
    let (port, _) = http_proxy("HTTP/1.1 407 Proxy Authentication Required\r\n\r\n").await;
    let err = open_tunnel(&format!("http://127.0.0.1:{port}"), "example.test", 443).await.unwrap_err();
    assert!(matches!(err, TikTokLiveError::Proxy(ref m) if m.contains("407")), "{err}");
}

async fn socks5_proxy(expect_auth: bool, relay: bool) -> (u16, Arc<Mutex<Vec<u8>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let record = seen.clone();
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = listener.accept().await.unwrap();
            let record = record.clone();
            tokio::spawn(async move {
                let mut head = [0u8; 2];
                sock.read_exact(&mut head).await.unwrap();
                let mut methods = vec![0u8; usize::from(head[1])];
                sock.read_exact(&mut methods).await.unwrap();
                if expect_auth {
                    assert!(methods.contains(&0x02));
                    sock.write_all(&[0x05, 0x02]).await.unwrap();
                    let mut ver_ulen = [0u8; 2];
                    sock.read_exact(&mut ver_ulen).await.unwrap();
                    let mut user = vec![0u8; usize::from(ver_ulen[1])];
                    sock.read_exact(&mut user).await.unwrap();
                    let plen = sock.read_u8().await.unwrap();
                    let mut pass = vec![0u8; usize::from(plen)];
                    sock.read_exact(&mut pass).await.unwrap();
                    assert_eq!((user.as_slice(), pass.as_slice()), (b"bob".as_slice(), b"s3cret".as_slice()));
                    sock.write_all(&[0x01, 0x00]).await.unwrap();
                } else {
                    sock.write_all(&[0x05, 0x00]).await.unwrap();
                }
                let mut req = [0u8; 4];
                sock.read_exact(&mut req).await.unwrap();
                let host = match req[3] {
                    0x03 => {
                        let len = sock.read_u8().await.unwrap();
                        let mut name = vec![0u8; usize::from(len)];
                        sock.read_exact(&mut name).await.unwrap();
                        String::from_utf8(name).unwrap()
                    }
                    0x01 => {
                        let mut ip = [0u8; 4];
                        sock.read_exact(&mut ip).await.unwrap();
                        std::net::Ipv4Addr::from(ip).to_string()
                    }
                    other => panic!("atyp {other}"),
                };
                let port = sock.read_u16().await.unwrap();
                record.lock().unwrap().extend_from_slice(format!("{host}:{port}").as_bytes());
                sock.write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await.unwrap();
                if relay {
                    let mut upstream = TcpStream::connect((host.as_str(), port)).await.unwrap();
                    tokio::io::copy_bidirectional(&mut sock, &mut upstream).await.ok();
                } else {
                    echo(sock).await;
                }
            });
        }
    });
    (port, seen)
}

#[tokio::test]
async fn socks5_with_username_password() {
    let (port, seen) = socks5_proxy(true, false).await;
    ping(open_tunnel(&format!("socks5://bob:s3cret@127.0.0.1:{port}"), "webcast-ws.tiktok.com", 443).await.unwrap()).await;
    assert_eq!(String::from_utf8(seen.lock().unwrap().clone()).unwrap(), "webcast-ws.tiktok.com:443");
}

#[tokio::test]
async fn socks5_without_auth() {
    let (port, seen) = socks5_proxy(false, false).await;
    ping(open_tunnel(&format!("socks5h://127.0.0.1:{port}"), "example.test", 8443).await.unwrap()).await;
    assert_eq!(String::from_utf8(seen.lock().unwrap().clone()).unwrap(), "example.test:8443");
}

#[tokio::test]
async fn unsupported_scheme_is_rejected() {
    let err = open_tunnel("ftp://127.0.0.1:21", "example.test", 443).await.unwrap_err();
    assert!(matches!(err, TikTokLiveError::InvalidUrl(ref m) if m.contains("ftp")), "{err}");
}

#[tokio::test]
async fn http_requests_go_through_socks5() {
    let origin = http_fixture(|_, nth| ok(&ttwid_cookie(nth), "")).await;
    let (port, seen) = socks5_proxy(false, true).await;
    let proxy = format!("socks5h://127.0.0.1:{port}");
    let request = TtwidRequest {
        url: &origin.base,
        timeout: Duration::from_secs(5),
        user_agent: "t/1",
        proxy: Some(&proxy),
        attempts: 1,
        retry_delay: Duration::ZERO,
    };
    assert_eq!(fetch_ttwid(&request).await.unwrap(), "tok0");
    assert!(String::from_utf8(seen.lock().unwrap().clone()).unwrap().starts_with("127.0.0.1:"), "request went via the socks5 proxy");
    assert_eq!(origin.hits("/"), 1);
}

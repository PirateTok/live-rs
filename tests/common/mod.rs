#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Minimal HTTP/1.1 origin for offline tests. `route(path, nth)` returns the
/// full raw response for the nth (0-based) request to that path.
pub struct HttpFixture {
    pub base: String,
    hits: Arc<Mutex<HashMap<String, usize>>>,
}

impl HttpFixture {
    pub fn hits(&self, path: &str) -> usize {
        self.hits.lock().unwrap().get(path).copied().unwrap_or(0)
    }
}

pub async fn http_fixture<F>(route: F) -> HttpFixture
where
    F: Fn(&str, usize) -> String + Send + Sync + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/", listener.local_addr().unwrap());
    let hits: Arc<Mutex<HashMap<String, usize>>> = Arc::default();
    let counter = hits.clone();
    let route = Arc::new(route);
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = listener.accept().await.unwrap();
            let counter = counter.clone();
            let route = route.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let mut read = 0;
                while !buf[..read].windows(4).any(|w| w == b"\r\n\r\n") {
                    let n = sock.read(&mut buf[read..]).await.unwrap();
                    if n == 0 {
                        return;
                    }
                    read += n;
                }
                let head = String::from_utf8_lossy(&buf[..read]).to_string();
                let target = head.split_whitespace().nth(1).unwrap_or("/").to_string();
                let path = match target.split_once("://") {
                    Some((_, rest)) => format!("/{}", rest.split_once('/').map(|(_, p)| p).unwrap_or("")),
                    None => target,
                };
                let key = path.split('?').next().unwrap().to_string();
                let nth = {
                    let mut map = counter.lock().unwrap();
                    let n = map.entry(key.clone()).or_insert(0);
                    *n += 1;
                    *n - 1
                };
                let resp = route(&path, nth);
                sock.write_all(resp.as_bytes()).await.unwrap();
            });
        }
    });
    HttpFixture { base, hits }
}

/// No-auth SOCKS5 relay; records every requested `host:port` target.
pub async fn socks5_relay() -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("socks5h://{}", listener.local_addr().unwrap());
    let targets: Arc<Mutex<Vec<String>>> = Arc::default();
    let record = targets.clone();
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = listener.accept().await.unwrap();
            let record = record.clone();
            tokio::spawn(async move {
                let mut head = [0u8; 2];
                sock.read_exact(&mut head).await.unwrap();
                let mut methods = vec![0u8; usize::from(head[1])];
                sock.read_exact(&mut methods).await.unwrap();
                sock.write_all(&[0x05, 0x00]).await.unwrap();
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
                    other => panic!("address type {other}"),
                };
                let port = sock.read_u16().await.unwrap();
                record.lock().unwrap().push(format!("{host}:{port}"));
                sock.write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await.unwrap();
                let mut upstream = tokio::net::TcpStream::connect((host.as_str(), port)).await.unwrap();
                tokio::io::copy_bidirectional(&mut sock, &mut upstream).await.ok();
            });
        }
    });
    (url, targets)
}

pub fn ok(extra_headers: &str, body: &str) -> String {
    format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n{extra_headers}\r\n{body}", body.len())
}

pub fn status(code: u16, body: &str) -> String {
    format!("HTTP/1.1 {code} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
}

pub fn ttwid_cookie(n: usize) -> String {
    format!("Set-Cookie: ttwid=tok{n}; Path=/; HttpOnly\r\n")
}

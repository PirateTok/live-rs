use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use piratetok_live_rs::errors::TikTokLiveError;
use piratetok_live_rs::http::ttwid::{fetch_ttwid, TtwidRequest};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Local HTTP fixture: the first `withhold` requests get 200 without a ttwid
/// cookie (what tiktok.com does intermittently), later ones get the cookie.
async fn spawn_fixture(withhold: u32, cookie_line: &'static str) -> (String, Arc<AtomicU32>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let hits = Arc::new(AtomicU32::new(0));
    let counter = hits.clone();
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = listener.accept().await.expect("accept");
            let mut buf = vec![0u8; 4096];
            let mut read = 0;
            while !buf[..read].windows(4).any(|w| w == b"\r\n\r\n") {
                let n = sock.read(&mut buf[read..]).await.expect("read");
                if n == 0 {
                    break;
                }
                read += n;
            }
            let n = counter.fetch_add(1, Ordering::SeqCst) + 1;
            let cookies = if n > withhold { cookie_line } else { "Set-Cookie: tt_csrf_token=x; Path=/\r\n" };
            let resp = format!("HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n{cookies}\r\n");
            sock.write_all(resp.as_bytes()).await.expect("write");
        }
    });
    (format!("http://{addr}/"), hits)
}

fn request(url: &str, attempts: u32, delay: Duration) -> TtwidRequest<'_> {
    TtwidRequest {
        url,
        timeout: Duration::from_secs(5),
        user_agent: "fixture/1.0",
        proxy: None,
        attempts,
        retry_delay: delay,
    }
}

const GOOD: &str = "Set-Cookie: tt_csrf_token=x; Path=/\r\nSet-Cookie: ttwid=1%7Cabc%7C123%7Csig; Path=/; HttpOnly\r\n";

#[tokio::test]
async fn missing_cookie_is_retried_until_it_arrives() {
    let (url, hits) = spawn_fixture(3, GOOD).await;
    let ttwid = fetch_ttwid(&request(&url, 8, Duration::from_millis(10))).await.expect("ttwid after retries");
    assert_eq!(ttwid, "1%7Cabc%7C123%7Csig");
    assert_eq!(hits.load(Ordering::SeqCst), 4, "3 cookie-less responses + 1 success");
}

#[tokio::test]
async fn first_response_with_cookie_needs_one_request() {
    let (url, hits) = spawn_fixture(0, GOOD).await;
    fetch_ttwid(&request(&url, 8, Duration::from_millis(10))).await.expect("ttwid");
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn gives_up_after_the_attempt_budget() {
    let (url, hits) = spawn_fixture(u32::MAX, GOOD).await;
    let err = fetch_ttwid(&request(&url, 8, Duration::from_millis(5))).await.expect_err("never gets a cookie");
    assert!(matches!(err, TikTokLiveError::InvalidResponse(ref m) if m.contains("after 8 attempts")), "{err}");
    assert_eq!(hits.load(Ordering::SeqCst), 8);
}

#[tokio::test]
async fn empty_ttwid_value_counts_as_missing() {
    let (url, hits) = spawn_fixture(u32::MAX, "Set-Cookie: ttwid=; Path=/\r\n").await;
    let err = fetch_ttwid(&request(&url, 3, Duration::from_millis(5))).await.expect_err("empty ttwid");
    assert!(matches!(err, TikTokLiveError::InvalidResponse(_)), "{err}");
    assert_eq!(hits.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn transport_error_is_not_retried() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    drop(listener);
    let url = format!("http://{addr}/");
    let started = Instant::now();
    let err = fetch_ttwid(&request(&url, 8, Duration::from_secs(1))).await.expect_err("connection refused");
    assert!(matches!(err, TikTokLiveError::Http(_)), "{err}");
    assert!(started.elapsed() < Duration::from_secs(1), "no retry sleeps on transport errors");
}

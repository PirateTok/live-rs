#![deny(unused_must_use)]
#![deny(for_loops_over_fallibles)]
#![deny(dead_code)]
#![deny(unused_variables)]
#![deny(unused_assignments)]

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::Engine;
use clap::Parser;
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use piratetok_live_rs::errors::TikTokLiveError;
use piratetok_live_rs::http::api::{fetch_room_id, FetchParams};
use piratetok_live_rs::http::ttwid::{fetch_ttwid, TtwidRequest};
use piratetok_live_rs::http::ua::{random_ua, system_timezone};
use piratetok_live_rs::structs::config::{CdnEndpoint, TIKTOK_WEB_URL, TTWID_FETCH_ATTEMPTS, TTWID_RETRY_DELAY};
use piratetok_live_rs::websocket::frames::{build_enter_room, build_heartbeat};

type WsMessage = tokio_tungstenite::tungstenite::Message;
type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WsSink = SplitSink<Ws, WsMessage>;

#[derive(Parser)]
#[command(about = "Record raw WSS frames from a live room as [u32_le length][raw bytes]; Ctrl+C to stop")]
struct Args {
    #[arg(help = "TikTok username (with or without @)")]
    username: String,

    #[arg(long, default_value = "captures", help = "directory for capture_<username>.bin")]
    dir: PathBuf,
}

#[derive(Clone)]
struct Counters {
    running: Arc<AtomicBool>,
    frames: Arc<AtomicU64>,
    bytes: Arc<AtomicU64>,
}

enum Flow {
    Continue,
    Stop,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_writer(std::io::stderr).init();
    let args = Args::parse();
    let output_path = args.dir.join(format!("capture_{}.bin", args.username.trim_start_matches('@')));
    let counters = Counters {
        running: Arc::new(AtomicBool::new(true)),
        frames: Arc::new(AtomicU64::new(0)),
        bytes: Arc::new(AtomicU64::new(0)),
    };
    let start = Instant::now();
    ctrlc_handler(counters.clone(), output_path.clone(), start);

    let ua = random_ua().to_string();
    let room_id = resolve_room(&args.username, &ua).await;
    let ttwid = acquire_ttwid(&ua).await;
    let ws = open_socket(&room_id, &ttwid, &ua).await;
    record(ws, &room_id, &output_path, &counters, start).await;
    print_stats(&output_path, &counters, start);
}

async fn resolve_room(username: &str, ua: &str) -> String {
    eprintln!("[record] resolving @{username}...");
    let params = FetchParams {
        timeout: Duration::from_secs(10),
        user_agent: Some(ua),
        ..Default::default()
    };
    match fetch_room_id(username, params).await {
        Ok(room) => {
            eprintln!("[record] room_id={}", room.room_id);
            room.room_id
        }
        Err(e) => {
            eprintln!("[record] FATAL: {e}");
            std::process::exit(1);
        }
    }
}

async fn acquire_ttwid(ua: &str) -> String {
    eprintln!("[record] fetching ttwid...");
    let request = TtwidRequest {
        url: TIKTOK_WEB_URL,
        timeout: Duration::from_secs(10),
        user_agent: ua,
        proxy: None,
        attempts: TTWID_FETCH_ATTEMPTS,
        retry_delay: TTWID_RETRY_DELAY,
    };
    match fetch_ttwid(&request).await {
        Ok(ttwid) => ttwid,
        Err(e) => {
            eprintln!("[record] FATAL: ttwid fetch failed: {e}");
            std::process::exit(1);
        }
    }
}

async fn open_socket(room_id: &str, ttwid: &str, ua: &str) -> Ws {
    let host = CdnEndpoint::Global.host();
    let ws_url = build_ws_url(host, room_id, &system_timezone());
    let key_bytes: [u8; 16] = rand::random();
    let ws_key = base64::engine::general_purpose::STANDARD.encode(key_bytes);
    let request = match http::Request::builder()
        .method("GET")
        .uri(&ws_url)
        .header("Host", host)
        .header("Upgrade", "websocket")
        .header("Connection", "Upgrade")
        .header("Sec-WebSocket-Key", &ws_key)
        .header("Sec-WebSocket-Version", "13")
        .header("User-Agent", ua)
        .header("Referer", "https://www.tiktok.com/")
        .header("Origin", "https://www.tiktok.com")
        .header("Cookie", format!("ttwid={ttwid}"))
        .body(())
    {
        Ok(request) => request,
        Err(e) => panic!("ws request build: {e}"),
    };

    eprintln!("[record] connecting WSS...");
    match tokio_tungstenite::connect_async(request).await {
        Ok((ws, _response)) => ws,
        Err(e) => {
            eprintln!("[record] FATAL: WSS connect failed: {e}");
            std::process::exit(1);
        }
    }
}

async fn record(ws: Ws, room_id: &str, output_path: &Path, counters: &Counters, start: Instant) {
    let (mut write, mut read) = ws.split();
    send_or_die(&mut write, build_heartbeat(room_id), "heartbeat").await;
    send_or_die(&mut write, build_enter_room(room_id), "enter room").await;

    let mut file = match File::create(output_path) {
        Ok(file) => file,
        Err(e) => panic!("create {}: {e}", output_path.display()),
    };
    eprintln!("[record] connected! writing to {}", output_path.display());
    eprintln!("[record] Ctrl+C to stop\n");

    let (hb_tx, mut hb_rx) = mpsc::channel::<Vec<u8>>(4);
    tokio::spawn(heartbeat_task(room_id.to_string(), counters.running.clone(), hb_tx));

    while counters.running.load(Ordering::Relaxed) {
        let flow = tokio::select! {
            hb = hb_rx.recv() => send_heartbeat(&mut write, hb).await,
            msg = read.next() => handle_message(msg, &mut write, &mut file, counters, start).await,
        };
        match flow {
            Flow::Continue => {}
            Flow::Stop => break,
        }
    }

    match file.flush() {
        Ok(()) => {}
        Err(e) => panic!("flush {}: {e}", output_path.display()),
    }
}

async fn send_or_die(write: &mut WsSink, frame: Result<Vec<u8>, TikTokLiveError>, what: &str) {
    let bytes = match frame {
        Ok(bytes) => bytes,
        Err(e) => panic!("build {what}: {e}"),
    };
    match write.send(WsMessage::Binary(bytes.into())).await {
        Ok(()) => {}
        Err(e) => panic!("send {what}: {e}"),
    }
}

async fn send_heartbeat(write: &mut WsSink, hb: Option<Vec<u8>>) -> Flow {
    let Some(hb) = hb else {
        tracing::warn!("heartbeat task ended");
        return Flow::Stop;
    };
    match write.send(WsMessage::Binary(hb.into())).await {
        Ok(()) => Flow::Continue,
        Err(e) => {
            tracing::error!(error = %e, "heartbeat send failed");
            Flow::Stop
        }
    }
}

async fn handle_message(msg: Option<Result<WsMessage, tokio_tungstenite::tungstenite::Error>>, write: &mut WsSink, file: &mut File, counters: &Counters, start: Instant) -> Flow {
    let Some(msg) = msg else {
        eprintln!("[record] websocket stream ended");
        return Flow::Stop;
    };
    match msg {
        Ok(WsMessage::Binary(data)) => {
            write_frame(file, &data, counters, start);
            Flow::Continue
        }
        Ok(WsMessage::Ping(data)) => match write.send(WsMessage::Pong(data)).await {
            Ok(()) => Flow::Continue,
            Err(e) => {
                tracing::warn!(error = %e, "pong send failed");
                Flow::Stop
            }
        },
        Ok(WsMessage::Close(frame)) => {
            eprintln!("[record] server closed connection: {frame:?}");
            Flow::Stop
        }
        Ok(WsMessage::Text(text)) => {
            tracing::warn!(len = text.len(), "unexpected text frame, not recorded");
            Flow::Continue
        }
        Ok(WsMessage::Pong(..)) | Ok(WsMessage::Frame(..)) => Flow::Continue,
        Err(e) => {
            tracing::error!(error = %e, "WSS error");
            Flow::Stop
        }
    }
}

async fn heartbeat_task(room_id: String, running: Arc<AtomicBool>, tx: mpsc::Sender<Vec<u8>>) {
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    interval.tick().await;
    while running.load(Ordering::Relaxed) {
        interval.tick().await;
        let frame = match build_heartbeat(&room_id) {
            Ok(frame) => frame,
            Err(e) => {
                tracing::error!(error = %e, "heartbeat build failed");
                return;
            }
        };
        match tx.send(frame).await {
            Ok(()) => {}
            Err(e) => {
                tracing::warn!(error = %e, "heartbeat receiver gone");
                return;
            }
        }
    }
}

fn write_frame(file: &mut File, data: &[u8], counters: &Counters, start: Instant) {
    let len = match u32::try_from(data.len()) {
        Ok(len) => len,
        Err(e) => panic!("frame larger than u32: {e}"),
    };
    let size = u64::from(len);
    match file.write_all(&len.to_le_bytes()).and_then(|()| file.write_all(data)) {
        Ok(()) => {}
        Err(e) => panic!("write frame: {e}"),
    }
    let n = counters.frames.fetch_add(1, Ordering::Relaxed) + 1;
    counters.bytes.fetch_add(size, Ordering::Relaxed);
    if n % 50 == 0 {
        eprintln!("[record] {n} frames, {} bytes, {}s", counters.bytes.load(Ordering::Relaxed), start.elapsed().as_secs());
    }
}

fn ctrlc_handler(counters: Counters, output_path: PathBuf, start: Instant) {
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {}
            Err(e) => tracing::error!(error = %e, "ctrl-c listener failed"),
        }
        counters.running.store(false, Ordering::Relaxed);
        eprintln!();
        print_stats(&output_path, &counters, start);
        std::process::exit(0);
    });
}

fn print_stats(path: &Path, counters: &Counters, start: Instant) {
    let frames = counters.frames.load(Ordering::Relaxed);
    let bytes = counters.bytes.load(Ordering::Relaxed);
    let elapsed = start.elapsed();
    eprintln!("\n=== CAPTURE STATS ===");
    eprintln!("file:       {}", path.display());
    eprintln!("frames:     {frames}");
    eprintln!("payload:    {bytes} bytes");
    match std::fs::metadata(path) {
        Ok(meta) => eprintln!("file size:  {} bytes (payload + {frames}x4 framing)", meta.len()),
        Err(e) => tracing::warn!(error = %e, "capture file not readable"),
    }
    eprintln!("duration:   {:.1}s", elapsed.as_secs_f64());
    let ms = elapsed.as_millis();
    if ms > 0 {
        let fps_x10 = u128::from(frames) * 10_000 / ms;
        eprintln!("rate:       {}.{} frames/s, {} bytes/s", fps_x10 / 10, fps_x10 % 10, u128::from(bytes) * 1000 / ms);
    }
}

fn build_ws_url(host: &str, room_id: &str, tz: &str) -> String {
    let last_rtt = format!("{:.3}", 100.0 + rand::random::<f64>() * 100.0);
    let params: &[(&str, &str)] = &[
        ("version_code", "180800"),
        ("device_platform", "web"),
        ("cookie_enabled", "true"),
        ("screen_width", "1920"),
        ("screen_height", "1080"),
        ("browser_language", "en-US"),
        ("browser_platform", "Linux x86_64"),
        ("browser_name", "Mozilla"),
        ("browser_version", "5.0 (X11)"),
        ("browser_online", "true"),
        ("tz_name", tz),
        ("app_name", "tiktok_web"),
        ("sup_ws_ds_opt", "1"),
        ("update_version_code", "2.0.0"),
        ("compress", "gzip"),
        ("webcast_language", "en"),
        ("ws_direct", "1"),
        ("aid", "1988"),
        ("live_id", "12"),
        ("app_language", "en"),
        ("client_enter", "1"),
        ("room_id", room_id),
        ("identity", "audience"),
        ("history_comment_count", "6"),
        ("last_rtt", &last_rtt),
        ("heartbeat_duration", "10000"),
        ("resp_content_type", "protobuf"),
        ("did_rule", "3"),
    ];
    let query: String = params.iter().map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v))).collect::<Vec<_>>().join("&");
    format!("wss://{host}/webcast/im/ws_proxy/ws_reuse_supplement/?{query}")
}

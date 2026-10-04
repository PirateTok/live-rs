//! End-to-end client run against local fakes: HTTP origin (room id + ttwid)
//! and a scripted WebSocket server. Proves wiring of connect → heartbeat/enter_room
//! → decode → reconnect budget → ttwid reuse/rotation → Disconnected, offline.

mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{http_fixture, ok, ttwid_cookie};
use futures_util::{SinkExt, StreamExt};
use piratetok_live_rs::errors::TikTokLiveError;
use piratetok_live_rs::structs::config::Endpoints;
use piratetok_live_rs::structs::proto::frames::WebcastPushFrame;
use piratetok_live_rs::structs::proto::messages::{WebcastChatMessage, WebcastMessage, WebcastResponse};
use piratetok_live_rs::structs::TikTokLiveEvent;
use piratetok_live_rs::TikTokLive;
use prost::Message;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::Message as WsMessage;

const ROOM: &str = r#"{"statusCode":0,"data":{"user":{"roomId":"7001","id":"55","status":2},"liveRoom":{"status":2}}}"#;

fn chat_frame() -> Vec<u8> {
    let chat = WebcastChatMessage {
        comment: "hello offline".into(),
        ..Default::default()
    };
    let response = WebcastResponse {
        messages: vec![WebcastMessage {
            r#type: "WebcastChatMessage".into(),
            payload: chat.encode_to_vec(),
            ..Default::default()
        }],
        ..Default::default()
    };
    WebcastPushFrame {
        payload_type: "msg".into(),
        payload: response.encode_to_vec(),
        ..Default::default()
    }
    .encode_to_vec()
}

struct WsLog {
    cookies: Vec<String>,
    first_frames: Vec<String>,
}

async fn ws_server() -> (String, Arc<Mutex<WsLog>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("ws://{}", listener.local_addr().unwrap());
    let log = Arc::new(Mutex::new(WsLog {
        cookies: Vec::new(),
        first_frames: Vec::new(),
    }));
    let shared = log.clone();
    tokio::spawn(async move {
        for conn in 0.. {
            let (stream, _) = listener.accept().await.unwrap();
            let cookie_log = shared.clone();
            let frame_log = shared.clone();
            let callback = move |req: &Request, resp: Response| -> Result<Response, ErrorResponse> {
                let cookie = req.headers().get("Cookie").map(|v| v.to_str().unwrap().to_string()).unwrap_or_default();
                cookie_log.lock().unwrap().cookies.push(cookie);
                if conn == 1 {
                    return Err(http::Response::builder().status(415).header("Handshake-Msg", "DEVICE_BLOCKED").body(None).unwrap());
                }
                Ok(resp)
            };
            let Ok(mut ws) = tokio_tungstenite::accept_hdr_async(stream, callback).await else {
                continue;
            };
            if conn == 0 {
                for _ in 0..2 {
                    let Some(Ok(WsMessage::Binary(data))) = ws.next().await else { panic!("expected binary frame") };
                    let frame = WebcastPushFrame::decode(data.as_ref()).unwrap();
                    frame_log.lock().unwrap().first_frames.push(frame.payload_type);
                }
                ws.send(WsMessage::Binary(chat_frame().into())).await.unwrap();
            }
            ws.close(None).await.ok();
        }
    });
    (base, log)
}

#[tokio::test]
async fn reconnect_lifecycle_against_local_fakes() {
    let origin = http_fixture(|path, nth| match path {
        p if p.starts_with("/api-live/user/room") => ok("Content-Type: application/json\r\n", ROOM),
        "/" => ok(&ttwid_cookie(nth), ""),
        other => panic!("unexpected path {other}"),
    })
    .await;
    let (ws_base, log) = ws_server().await;

    let mut stream = TikTokLive::builder("someone")
        .endpoints(Endpoints {
            web: origin.base.clone(),
            webcast: format!("{}webcast/", origin.base),
            ws: Some(ws_base),
        })
        .max_retries(2)
        .stale_timeout(Duration::from_secs(5))
        .connect()
        .await
        .unwrap();

    let mut seen = Vec::new();
    loop {
        match tokio::time::timeout(Duration::from_secs(20), stream.next_event()).await.expect("event within 20s") {
            Ok(TikTokLiveEvent::Connected { room_id }) => seen.push(format!("connected:{room_id}")),
            Ok(TikTokLiveEvent::Chat(msg)) => seen.push(format!("chat:{}", msg.comment)),
            Ok(TikTokLiveEvent::Reconnecting { attempt, max_retries, delay_secs }) => seen.push(format!("reconnecting:{attempt}/{max_retries}:{delay_secs}s")),
            Ok(TikTokLiveEvent::Disconnected) => seen.push("disconnected".into()),
            Ok(other) => panic!("unexpected event {other:?}"),
            Err(TikTokLiveError::ConnectionClosed) => break,
            Err(e) => panic!("unexpected error {e}"),
        }
    }

    assert_eq!(seen, vec!["connected:7001", "chat:hello offline", "reconnecting:1/2:2s", "reconnecting:2/2:2s", "disconnected"]);
    let log = log.lock().unwrap();
    assert_eq!(log.first_frames, vec!["hb", "im_enter_room"], "heartbeat then enter_room right after connect");
    assert_eq!(log.cookies, vec!["ttwid=tok0", "ttwid=tok0", "ttwid=tok1"], "ttwid reused across the close, rotated after DEVICE_BLOCKED");
    assert_eq!(origin.hits("/"), 2, "two ttwid fetches: initial + rotation");
    assert_eq!(origin.hits("/api-live/user/room"), 1);
}

#[tokio::test]
async fn whole_client_runs_through_socks5_proxy() {
    let origin = http_fixture(|path, nth| match path {
        p if p.starts_with("/api-live/user/room") => ok("Content-Type: application/json\r\n", ROOM),
        "/" => ok(&ttwid_cookie(nth), ""),
        other => panic!("unexpected path {other}"),
    })
    .await;
    let (ws_base, log) = ws_server().await;
    let (proxy, targets) = common::socks5_relay().await;
    let ws_target = ws_base.trim_start_matches("ws://").to_string();
    let http_target = origin.base.trim_start_matches("http://").trim_end_matches('/').to_string();

    let mut stream = TikTokLive::builder("someone")
        .endpoints(Endpoints {
            web: origin.base.clone(),
            webcast: format!("{}webcast/", origin.base),
            ws: Some(ws_base),
        })
        .proxy(proxy)
        .max_retries(0)
        .connect()
        .await
        .unwrap();
    let mut chats = 0;
    loop {
        match tokio::time::timeout(Duration::from_secs(20), stream.next_event()).await.expect("event within 20s") {
            Ok(TikTokLiveEvent::Chat(..)) => chats += 1,
            Ok(TikTokLiveEvent::Disconnected) => break,
            Ok(..) => {}
            Err(e) => panic!("unexpected error {e}"),
        }
    }
    assert_eq!(chats, 1);
    assert_eq!(log.lock().unwrap().cookies, vec!["ttwid=tok0"]);
    let targets = targets.lock().unwrap().clone();
    assert!(targets.contains(&ws_target), "WSS went through the proxy: {targets:?}");
    assert_eq!(targets.iter().filter(|t| **t == http_target).count(), 2, "room id + ttwid went through the proxy: {targets:?}");
}

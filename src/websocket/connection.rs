use std::time::Duration;

use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use prost::Message;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::decode::mapper;
use crate::errors::TikTokLiveError;
use crate::structs::proto::frames::WebcastPushFrame;
use crate::structs::proto::messages::WebcastResponse;
use crate::structs::TikTokLiveEvent;
use crate::websocket::frames::{build_ack, build_enter_room, build_heartbeat, decompress_if_gzipped};

type WsMessage = tokio_tungstenite::tungstenite::Message;

pub async fn run_websocket(ws_url: &str, cookies: &str, user_agent: &str, room_id: &str, heartbeat_interval: Duration, stale_timeout: Duration, _proxy: Option<&str>, tx: mpsc::Sender<TikTokLiveEvent>) -> Result<(), TikTokLiveError> {
    let host = url_host(ws_url)?;
    let ws_key = generate_ws_key();

    let request = http::Request::builder()
        .method("GET")
        .uri(ws_url)
        .header("Host", &host)
        .header("Upgrade", "websocket")
        .header("Connection", "Upgrade")
        .header("Sec-WebSocket-Key", &ws_key)
        .header("Sec-WebSocket-Version", "13")
        .header("User-Agent", user_agent)
        .header("Referer", "https://www.tiktok.com/")
        .header("Origin", "https://www.tiktok.com")
        .header("Accept-Language", "en-US,en;q=0.9")
        .header("Accept-Encoding", "gzip, deflate")
        .header("Cache-Control", "no-cache")
        .header("Cookie", cookies)
        .body(())
        .map_err(|e| TikTokLiveError::invalid(format!("ws request build: {e}")))?;

    let (ws_stream, _) = match tokio_tungstenite::connect_async(request).await {
        Ok(pair) => pair,
        Err(tokio_tungstenite::tungstenite::Error::Http(resp)) => {
            let handshake_msg = extract_header(&resp, "Handshake-Msg");

            if handshake_msg == "DEVICE_BLOCKED" {
                return Err(TikTokLiveError::DeviceBlocked);
            }

            let handshake_status = extract_header(&resp, "Handshake-Status");

            return Err(TikTokLiveError::invalid(format!(
                "handshake rejected: msg={handshake_msg} status={handshake_status}"
            )));
        }
        Err(e) => return Err(e.into()),
    };
    let (mut write, mut read) = ws_stream.split();

    info!("websocket connected");

    let hb_bytes = build_heartbeat(room_id)?;
    write.send(WsMessage::Binary(hb_bytes.into())).await?;

    let enter_bytes = build_enter_room(room_id)?;
    write.send(WsMessage::Binary(enter_bytes.into())).await?;

    let mut heartbeat_tick = interval(heartbeat_interval);
    heartbeat_tick.tick().await; // skip first immediate tick

    let room_id_owned = room_id.to_string();

    let stale_sleep = tokio::time::sleep(stale_timeout);
    tokio::pin!(stale_sleep);

    loop {
        tokio::select! {
            _ = heartbeat_tick.tick() => {
                let hb = build_heartbeat(&room_id_owned)?;
                if let Err(e) = write.send(WsMessage::Binary(hb.into())).await {
                    error!("heartbeat send failed: {e}");
                    break;
                }
                debug!("heartbeat sent");
            }
            _ = &mut stale_sleep => {
                info!("stale: no data for {:?}, closing", stale_timeout);
                break;
            }
            msg = read.next() => {
                // Reset stale timer on any message
                stale_sleep.as_mut().reset(tokio::time::Instant::now() + stale_timeout);

                match msg {
                    Some(Ok(WsMessage::Binary(data))) => {
                        if let Err(e) = process_binary(&data, &mut write, &tx).await {
                            warn!("frame processing error: {e}");
                        }
                    }
                    Some(Ok(WsMessage::Ping(data))) => {
                        let _ = write.send(WsMessage::Pong(data)).await;
                    }
                    Some(Ok(WsMessage::Close(_))) => {
                        info!("server sent close frame");
                        break;
                    }
                    Some(Err(e)) => {
                        error!("websocket read error: {e}");
                        break;
                    }
                    None => {
                        info!("websocket stream ended");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

async fn process_binary<S>(data: &[u8], write: &mut S, tx: &mpsc::Sender<TikTokLiveEvent>) -> Result<(), TikTokLiveError>
where
    S: SinkExt<WsMessage> + Unpin,
    S::Error: std::fmt::Display,
{
    let frame = WebcastPushFrame::decode(data)?;

    match frame.payload_type.as_str() {
        "msg" => {
            let decompressed = decompress_if_gzipped(&frame.payload)?;
            let response = WebcastResponse::decode(decompressed.as_slice())?;

            if response.needs_ack && !response.internal_ext.is_empty() {
                let ack = build_ack(frame.log_id, response.internal_ext.as_bytes())?;
                let _ = write.send(WsMessage::Binary(ack.into())).await;
            }

            for message in &response.messages {
                let events = mapper::decode_message(&message.r#type, &message.payload);
                for event in events {
                    let _ = tx.send(event).await;
                }
            }
        }
        "im_enter_room_resp" => {
            info!("room entry confirmed");
        }
        "hb" => {
            debug!("heartbeat response");
        }
        other => {
            debug!("unhandled payload type: {other}");
        }
    }

    Ok(())
}

fn url_host(url: &str) -> Result<String, TikTokLiveError> {
    let stripped = url
        .strip_prefix("wss://")
        .or_else(|| url.strip_prefix("ws://"))
        .ok_or_else(|| TikTokLiveError::InvalidUrl("not a ws/wss url".into()))?;

    let host = stripped.split('/').next().ok_or_else(|| TikTokLiveError::InvalidUrl("no host in url".into()))?;

    Ok(host.to_string())
}

fn generate_ws_key() -> String {
    let bytes: [u8; 16] = rand::random();
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn extract_header(resp: &http::Response<Option<Vec<u8>>>, name: &str) -> String {
    match resp.headers().get(name) {
        Some(v) => match v.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => "?".to_string(),
        },
        None => "?".to_string(),
    }
}

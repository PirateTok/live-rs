use std::fmt::Display;
use std::time::Duration;

use base64::Engine;
use futures_util::{Sink, SinkExt, StreamExt};
use prost::Message;
use reqwest::Url;
use tokio::sync::mpsc;
use tokio::time::{interval, Instant};
use tracing::{debug, info};

use crate::decode::mapper;
use crate::errors::TikTokLiveError;
use crate::structs::proto::frames::WebcastPushFrame;
use crate::structs::proto::messages::WebcastResponse;
use crate::structs::TikTokLiveEvent;
use crate::websocket::frames::{build_ack, build_enter_room, build_heartbeat, decompress_if_gzipped};
use crate::websocket::proxy::open_tunnel;

type WsMessage = tokio_tungstenite::tungstenite::Message;
type WsError = tokio_tungstenite::tungstenite::Error;

pub struct WsSession<'a> {
    pub url: &'a str,
    pub cookies: &'a str,
    pub user_agent: &'a str,
    pub room_id: &'a str,
    pub heartbeat_interval: Duration,
    pub stale_timeout: Duration,
    pub proxy: Option<&'a str>,
    pub accept_language: &'a str,
}

enum Flow {
    Continue,
    Stop,
}

pub async fn run_websocket(session: &WsSession<'_>, tx: mpsc::Sender<TikTokLiveEvent>) -> Result<(), TikTokLiveError> {
    let url = Url::parse(session.url).map_err(|e| TikTokLiveError::InvalidUrl(format!("{}: {e}", session.url)))?;
    let Some(host) = url.host_str() else {
        return Err(TikTokLiveError::InvalidUrl(format!("no host in {}", session.url)));
    };
    let Some(port) = url.port_or_known_default() else {
        return Err(TikTokLiveError::InvalidUrl(format!("no port for {}", session.url)));
    };
    let mut authority = host.to_string();
    for explicit in url.port().iter() {
        authority.push_str(&format!(":{explicit}"));
    }
    let request = build_request(session, &authority)?;

    for proxy in session.proxy.iter() {
        let tunnel = open_tunnel(proxy, host, port).await?;
        let (ws, _response) = handshake(tokio_tungstenite::client_async_tls_with_config(request, tunnel, None, None).await)?;
        return event_loop(ws, session, tx).await;
    }
    let (ws, _response) = handshake(tokio_tungstenite::connect_async(request).await)?;
    event_loop(ws, session, tx).await
}

fn build_request(session: &WsSession<'_>, authority: &str) -> Result<http::Request<()>, TikTokLiveError> {
    let key_bytes: [u8; 16] = rand::random();
    http::Request::builder()
        .method("GET")
        .uri(session.url)
        .header("Host", authority)
        .header("Upgrade", "websocket")
        .header("Connection", "Upgrade")
        .header("Sec-WebSocket-Key", base64::engine::general_purpose::STANDARD.encode(key_bytes))
        .header("Sec-WebSocket-Version", "13")
        .header("User-Agent", session.user_agent)
        .header("Referer", "https://www.tiktok.com/")
        .header("Origin", "https://www.tiktok.com")
        .header("Accept-Language", session.accept_language)
        .header("Accept-Encoding", "gzip, deflate")
        .header("Cache-Control", "no-cache")
        .header("Cookie", session.cookies)
        .body(())
        .map_err(|e| TikTokLiveError::invalid(format!("ws request build: {e}")))
}

fn handshake<T>(result: Result<T, WsError>) -> Result<T, TikTokLiveError> {
    match result {
        Ok(pair) => Ok(pair),
        Err(WsError::Http(resp)) => {
            let status = resp.status();
            match resp.headers().get("Handshake-Msg").map(|v| v.as_bytes()) {
                Some(b"DEVICE_BLOCKED") => Err(TikTokLiveError::DeviceBlocked),
                Some(msg) => Err(TikTokLiveError::invalid(format!("handshake rejected: http {status} handshake-msg={msg:?}"))),
                None => Err(TikTokLiveError::invalid(format!("handshake rejected: http {status}"))),
            }
        }
        Err(e) => Err(e.into()),
    }
}

async fn event_loop<S>(ws: tokio_tungstenite::WebSocketStream<S>, session: &WsSession<'_>, tx: mpsc::Sender<TikTokLiveEvent>) -> Result<(), TikTokLiveError>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let (mut write, mut read) = ws.split();
    info!("websocket connected");
    write.send(WsMessage::Binary(build_heartbeat(session.room_id)?.into())).await?;
    write.send(WsMessage::Binary(build_enter_room(session.room_id)?.into())).await?;

    let mut heartbeat = interval(session.heartbeat_interval);
    heartbeat.tick().await;
    let stale = tokio::time::sleep(session.stale_timeout);
    tokio::pin!(stale);

    loop {
        tokio::select! {
            _tick = heartbeat.tick() => {
                match write.send(WsMessage::Binary(build_heartbeat(session.room_id)?.into())).await {
                    Ok(()) => debug!("heartbeat sent"),
                    Err(e) => {
                        tracing::error!(error = %e, "heartbeat send failed");
                        break;
                    }
                }
            }
            () = &mut stale => {
                info!("stale: no data for {:?}, closing", session.stale_timeout);
                break;
            }
            msg = read.next() => {
                stale.as_mut().reset(Instant::now() + session.stale_timeout);
                match handle(msg, &mut write, &tx).await {
                    Flow::Continue => {}
                    Flow::Stop => break,
                }
            }
        }
    }
    Ok(())
}

async fn handle<W>(msg: Option<Result<WsMessage, WsError>>, write: &mut W, tx: &mpsc::Sender<TikTokLiveEvent>) -> Flow
where
    W: Sink<WsMessage> + Unpin,
    W::Error: Display,
{
    let Some(msg) = msg else {
        info!("websocket stream ended");
        return Flow::Stop;
    };
    match msg {
        Ok(WsMessage::Binary(data)) => match process_binary(&data, write, tx).await {
            Ok(()) => Flow::Continue,
            Err(TikTokLiveError::ConnectionClosed) => {
                tracing::warn!("event receiver dropped, closing websocket");
                Flow::Stop
            }
            Err(e) => {
                tracing::warn!(error = %e, "frame processing error");
                Flow::Continue
            }
        },
        Ok(WsMessage::Ping(data)) => match write.send(WsMessage::Pong(data)).await {
            Ok(()) => Flow::Continue,
            Err(e) => {
                tracing::warn!(error = %e, "pong send failed");
                Flow::Stop
            }
        },
        Ok(WsMessage::Close(frame)) => {
            info!(?frame, "server sent close frame");
            Flow::Stop
        }
        Ok(WsMessage::Text(..) | WsMessage::Pong(..) | WsMessage::Frame(..)) => Flow::Continue,
        Err(e) => {
            tracing::error!(error = %e, "websocket read error");
            Flow::Stop
        }
    }
}

async fn process_binary<W>(data: &[u8], write: &mut W, tx: &mpsc::Sender<TikTokLiveEvent>) -> Result<(), TikTokLiveError>
where
    W: Sink<WsMessage> + Unpin,
    W::Error: Display,
{
    let frame = WebcastPushFrame::decode(data)?;
    match frame.payload_type.as_str() {
        "msg" => {}
        "im_enter_room_resp" => {
            info!("room entry confirmed");
            return Ok(());
        }
        other => {
            debug!("payload type {other}");
            return Ok(());
        }
    }
    let response = WebcastResponse::decode(decompress_if_gzipped(&frame.payload)?.as_slice())?;
    if response.needs_ack && !response.internal_ext.is_empty() {
        match write.send(WsMessage::Binary(build_ack(frame.log_id, response.internal_ext.as_bytes())?.into())).await {
            Ok(()) => debug!("ack sent"),
            Err(e) => tracing::warn!(error = %e, "ack send failed"),
        }
    }
    for message in &response.messages {
        for event in mapper::decode_message(&message.r#type, &message.payload) {
            match tx.send(event).await {
                Ok(()) => {}
                Err(e) => {
                    tracing::warn!(error = %e, "event receiver dropped");
                    return Err(TikTokLiveError::ConnectionClosed);
                }
            }
        }
    }
    Ok(())
}

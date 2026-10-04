use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::info;

use crate::errors::TikTokLiveError;
use crate::http::api::fetch_room_id;
use crate::http::ttwid::fetch_ttwid;
use crate::http::ua::system_timezone;
use crate::reconnect::{judge, AttemptEnd, ReconnectBudget, SessionAction, SessionExit, Verdict};
use crate::structs::config::{CdnEndpoint, TikTokLiveConfig};
use crate::structs::TikTokLiveEvent;
use crate::websocket::connection::run_websocket;

pub struct TikTokLive;

impl TikTokLive {
    pub fn builder(username: &str) -> TikTokLiveBuilder {
        TikTokLiveBuilder { config: TikTokLiveConfig::new(username) }
    }
}

pub struct TikTokLiveBuilder {
    config: TikTokLiveConfig,
}

impl TikTokLiveBuilder {
    pub fn cdn(mut self, cdn: CdnEndpoint) -> Self {
        self.config.cdn = cdn;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    pub fn heartbeat_interval(mut self, interval: Duration) -> Self {
        self.config.heartbeat_interval = interval;
        self
    }

    pub fn max_retries(mut self, n: u32) -> Self {
        self.config.max_retries = n;
        self
    }

    pub fn stale_timeout(mut self, timeout: Duration) -> Self {
        self.config.stale_timeout = timeout;
        self
    }

    pub fn proxy(mut self, url: impl Into<String>) -> Self {
        self.config.proxy = Some(url.into());
        self
    }

    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.config.user_agent = Some(ua.into());
        self
    }

    pub fn cookies(mut self, cookies: impl Into<String>) -> Self {
        self.config.cookies = Some(cookies.into());
        self
    }

    pub fn language(mut self, lang: impl Into<String>) -> Self {
        self.config.language = lang.into();
        self
    }

    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.config.region = region.into();
        self
    }

    pub fn compress(mut self, compress: bool) -> Self {
        self.config.compress = compress;
        self
    }

    pub async fn connect(self) -> Result<TikTokLiveStream, TikTokLiveError> {
        let config = self.config;
        info!("fetching room id for {}", config.username);
        let room_id = fetch_room_id(&config.username, config.fetch_params()).await?.room_id;

        let (tx, rx) = mpsc::channel(256);
        emit(&tx, TikTokLiveEvent::Connected { room_id: room_id.clone() }).await?;

        let task = tokio::spawn(supervise(config, room_id, tx));
        Ok(TikTokLiveStream { rx, task })
    }
}

pub struct TikTokLiveStream {
    rx: mpsc::Receiver<TikTokLiveEvent>,
    task: JoinHandle<()>,
}

impl TikTokLiveStream {
    pub async fn next_event(&mut self) -> Result<TikTokLiveEvent, TikTokLiveError> {
        self.rx.recv().await.ok_or(TikTokLiveError::ConnectionClosed)
    }
}

impl Drop for TikTokLiveStream {
    fn drop(&mut self) {
        self.task.abort();
    }
}

struct Session {
    ttwid: String,
    user_agent: String,
}

enum Credentials {
    Fresh,
    Held(Session),
}

struct Attempt {
    end: AttemptEnd,
    credentials: Credentials,
}

async fn supervise(config: TikTokLiveConfig, room_id: String, tx: mpsc::Sender<TikTokLiveEvent>) {
    match reconnect_loop(&config, &room_id, &tx).await {
        Ok(()) => info!("reconnect loop finished"),
        Err(e) => tracing::warn!(error = %e, "reconnect loop stopped"),
    }
}

async fn reconnect_loop(config: &TikTokLiveConfig, room_id: &str, tx: &mpsc::Sender<TikTokLiveEvent>) -> Result<(), TikTokLiveError> {
    let tz = system_timezone();
    let mut credentials = Credentials::Fresh;
    let mut budget = ReconnectBudget::new(config.max_retries);
    loop {
        let outcome = run_attempt(config, room_id, &tz, credentials, tx).await;
        credentials = outcome.credentials;
        let (attempt, delay) = match budget.record(outcome.end) {
            Verdict::Retry { attempt, delay } => (attempt, delay),
            Verdict::GiveUp { attempt } => {
                info!("max retries ({}) exceeded at attempt {attempt}", config.max_retries);
                break;
            }
        };
        emit(
            tx,
            TikTokLiveEvent::Reconnecting {
                attempt,
                max_retries: config.max_retries,
                delay_secs: delay.as_secs(),
            },
        )
        .await?;
        info!("reconnecting in {}s (attempt {}/{})", delay.as_secs(), attempt, config.max_retries);
        tokio::time::sleep(delay).await;
    }
    emit(tx, TikTokLiveEvent::Disconnected).await
}

async fn run_attempt(config: &TikTokLiveConfig, room_id: &str, tz: &str, credentials: Credentials, tx: &mpsc::Sender<TikTokLiveEvent>) -> Attempt {
    let session = match ensure_session(config, credentials).await {
        Ok(session) => session,
        Err(e) => {
            tracing::warn!(error = %e, "ttwid acquisition failed");
            return Attempt {
                end: judge(SessionExit::NoTtwid, Duration::ZERO).end,
                credentials: Credentials::Fresh,
            };
        }
    };

    let started = Instant::now();
    let ws_url = build_ws_url(room_id, tz, config);
    let cookie = config.ws_cookie(&session.ttwid);
    let accept_language = config.accept_language();
    let result = run_websocket(
        &ws_url,
        &cookie,
        &session.user_agent,
        room_id,
        config.heartbeat_interval,
        config.stale_timeout,
        config.proxy.as_deref(),
        &accept_language,
        tx.clone(),
    )
    .await;
    let lived = started.elapsed();

    let exit = match result {
        Ok(()) => SessionExit::Closed,
        Err(TikTokLiveError::DeviceBlocked) => {
            tracing::warn!("DEVICE_BLOCKED — rotating ttwid + UA");
            SessionExit::DeviceBlocked
        }
        Err(e) => {
            tracing::error!(error = %e, lived_secs = lived.as_secs(), "websocket error");
            SessionExit::Errored
        }
    };
    let judgement = judge(exit, lived);
    Attempt {
        end: judgement.end,
        credentials: apply(judgement.session, session),
    }
}

fn apply(action: SessionAction, session: Session) -> Credentials {
    match action {
        SessionAction::Keep => Credentials::Held(session),
        SessionAction::Rotate => Credentials::Fresh,
    }
}

async fn ensure_session(config: &TikTokLiveConfig, credentials: Credentials) -> Result<Session, TikTokLiveError> {
    match credentials {
        Credentials::Held(session) => Ok(session),
        Credentials::Fresh => {
            let user_agent = config.resolved_user_agent();
            let ttwid = fetch_ttwid(&config.ttwid_request(&user_agent)).await?;
            Ok(Session { ttwid, user_agent })
        }
    }
}

async fn emit(tx: &mpsc::Sender<TikTokLiveEvent>, event: TikTokLiveEvent) -> Result<(), TikTokLiveError> {
    match tx.send(event).await {
        Ok(()) => Ok(()),
        Err(e) => {
            tracing::warn!(error = %e, "event receiver dropped");
            Err(TikTokLiveError::ConnectionClosed)
        }
    }
}

fn build_ws_url(room_id: &str, tz: &str, config: &TikTokLiveConfig) -> String {
    let last_rtt = format!("{:.3}", 100.0 + rand::random::<f64>() * 100.0);
    let browser_lang = config.browser_language();
    let heartbeat_ms = config.heartbeat_interval.as_millis().to_string();
    let params: &[(&str, &str)] = &[
        ("version_code", "180800"),
        ("device_platform", "web"),
        ("cookie_enabled", "true"),
        ("screen_width", "1920"),
        ("screen_height", "1080"),
        ("browser_language", &browser_lang),
        ("browser_platform", "Linux x86_64"),
        ("browser_name", "Mozilla"),
        ("browser_version", "5.0 (X11)"),
        ("browser_online", "true"),
        ("tz_name", tz),
        ("app_name", "tiktok_web"),
        ("sup_ws_ds_opt", "1"),
        ("update_version_code", "2.0.0"),
        ("compress", if config.compress { "gzip" } else { "" }),
        ("webcast_language", &config.language),
        ("ws_direct", "1"),
        ("aid", "1988"),
        ("live_id", "12"),
        ("app_language", &config.language),
        ("client_enter", "1"),
        ("room_id", room_id),
        ("identity", "audience"),
        ("history_comment_count", "6"),
        ("last_rtt", &last_rtt),
        ("heartbeat_duration", &heartbeat_ms),
        ("resp_content_type", "protobuf"),
        ("did_rule", "3"),
    ];

    let query: String = params.iter().map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v))).collect::<Vec<_>>().join("&");

    format!("wss://{}/webcast/im/ws_proxy/ws_reuse_supplement/?{query}", config.cdn.host())
}

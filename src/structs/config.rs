use std::fmt;
use std::sync::LazyLock;
use std::time::Duration;

use reqwest::Url;
use serde_json::Value;

use crate::http::api::FetchParams;
use crate::http::ttwid::TtwidRequest;
use crate::http::ua::{random_ua, system_locale};

pub const TIKTOK_WEB_URL: &str = "https://www.tiktok.com/";
pub const TIKTOK_WEBCAST_URL: &str = "https://webcast.tiktok.com/webcast/";
pub const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
pub const PROFILE_CACHE_TTL: Duration = Duration::from_secs(300);
pub const PROFILE_TTWID_TIMEOUT: Duration = Duration::from_secs(10);
pub const PROFILE_SCRAPE_TIMEOUT: Duration = Duration::from_secs(15);
pub const TTWID_FETCH_ATTEMPTS: u32 = 8;
pub const TTWID_RETRY_DELAY: Duration = Duration::from_millis(750);
pub const HEALTHY_SESSION: Duration = Duration::from_secs(30);
pub const DEVICE_BLOCKED_DELAY: Duration = Duration::from_secs(2);
pub const MAX_BACKOFF: Duration = Duration::from_secs(30);
pub const HTTP_PROXY_DEFAULT_PORT: u16 = 8080;
pub const HTTPS_PROXY_DEFAULT_PORT: u16 = 443;
pub const SOCKS5_DEFAULT_PORT: u16 = 1080;

pub static TIKTOK_ENDPOINTS: LazyLock<Endpoints> = LazyLock::new(Endpoints::tiktok);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoints {
    pub web: String,
    pub webcast: String,
    pub ws: Option<String>,
}

impl Endpoints {
    pub fn tiktok() -> Self {
        Self {
            web: TIKTOK_WEB_URL.to_string(),
            webcast: TIKTOK_WEBCAST_URL.to_string(),
            ws: None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub enum CdnEndpoint {
    Eu,
    Us,
    #[default]
    Global,
}

impl CdnEndpoint {
    pub fn host(&self) -> &str {
        match self {
            CdnEndpoint::Eu => "webcast-ws.eu.tiktok.com",
            CdnEndpoint::Us => "webcast-ws.us.tiktok.com",
            CdnEndpoint::Global => "webcast-ws.tiktok.com",
        }
    }
}

impl fmt::Display for CdnEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.host())
    }
}

#[derive(Clone, Debug)]
pub struct TikTokLiveConfig {
    pub username: String,
    pub cdn: CdnEndpoint,
    pub timeout: Duration,
    pub heartbeat_interval: Duration,
    pub max_retries: u32,
    pub stale_timeout: Duration,
    pub proxy: Option<String>,
    pub user_agent: Option<String>,
    pub cookies: Option<String>,
    pub language: String,
    pub region: String,
    pub compress: bool,
    pub endpoints: Endpoints,
}

impl TikTokLiveConfig {
    pub fn new(username: impl Into<String>) -> Self {
        let (language, region) = system_locale();
        Self {
            username: username.into(),
            cdn: CdnEndpoint::default(),
            timeout: HTTP_TIMEOUT,
            heartbeat_interval: Duration::from_secs(10),
            max_retries: 5,
            stale_timeout: Duration::from_secs(60),
            proxy: None,
            user_agent: None,
            cookies: None,
            language,
            region,
            compress: true,
            endpoints: Endpoints::tiktok(),
        }
    }

    pub fn browser_language(&self) -> String {
        format!("{}-{}", self.language, self.region)
    }

    pub fn accept_language(&self) -> String {
        format!("{}-{},{};q=0.9", self.language, self.region, self.language)
    }

    pub fn resolved_user_agent(&self) -> String {
        match &self.user_agent {
            Some(ua) => ua.clone(),
            None => random_ua().to_string(),
        }
    }

    pub fn ws_cookie(&self, ttwid: &str) -> String {
        match &self.cookies {
            Some(extra) => format!("ttwid={ttwid}; {extra}"),
            None => format!("ttwid={ttwid}"),
        }
    }

    pub fn ws_base(&self) -> String {
        match &self.endpoints.ws {
            Some(base) => base.trim_end_matches('/').to_string(),
            None => format!("wss://{}", self.cdn.host()),
        }
    }

    pub fn ttwid_request<'a>(&'a self, user_agent: &'a str) -> TtwidRequest<'a> {
        TtwidRequest {
            url: &self.endpoints.web,
            timeout: self.timeout,
            user_agent,
            proxy: self.proxy.as_deref(),
            attempts: TTWID_FETCH_ATTEMPTS,
            retry_delay: TTWID_RETRY_DELAY,
        }
    }

    pub fn fetch_params(&self) -> FetchParams<'_> {
        FetchParams {
            timeout: self.timeout,
            cookies: None,
            user_agent: self.user_agent.as_deref(),
            proxy: self.proxy.as_deref(),
            language: Some(&self.language),
            region: Some(&self.region),
            endpoints: &self.endpoints,
        }
    }
}

impl Default for FetchParams<'_> {
    fn default() -> Self {
        Self {
            timeout: HTTP_TIMEOUT,
            cookies: None,
            user_agent: None,
            proxy: None,
            language: None,
            region: None,
            endpoints: &TIKTOK_ENDPOINTS,
        }
    }
}

pub struct Locale {
    pub language: String,
    pub region: String,
    pub browser_language: String,
}

pub fn fetch_locale(params: &FetchParams<'_>) -> Locale {
    let (system_language, system_region) = system_locale();
    let language = match params.language {
        Some(l) => l.to_string(),
        None => system_language,
    };
    let region = match params.region {
        Some(r) => r.to_string(),
        None => system_region,
    };
    let browser_language = format!("{language}-{region}");
    Locale { language, region, browser_language }
}

pub fn fetch_user_agent(params: &FetchParams<'_>) -> String {
    match params.user_agent {
        Some(ua) => ua.to_string(),
        None => random_ua().to_string(),
    }
}

pub fn proxy_port(url: &Url) -> u16 {
    match (url.port(), url.scheme()) {
        (Some(port), _) => port,
        (None, "https") => HTTPS_PROXY_DEFAULT_PORT,
        (None, "socks5" | "socks5h") => SOCKS5_DEFAULT_PORT,
        (None, _) => HTTP_PROXY_DEFAULT_PORT,
    }
}

pub fn json_str(v: &Value, pointer: &str) -> String {
    match v.pointer(pointer).and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => String::new(),
    }
}

pub fn json_i64(v: &Value, pointer: &str) -> i64 {
    match v.pointer(pointer).and_then(Value::as_i64) {
        Some(n) => n,
        None => 0,
    }
}

pub fn json_bool(v: &Value, pointer: &str) -> bool {
    match v.pointer(pointer).and_then(Value::as_bool) {
        Some(b) => b,
        None => false,
    }
}

pub fn reconnect_backoff(attempt: u32) -> Duration {
    match 1u64.checked_shl(attempt) {
        Some(secs) => Duration::from_secs(secs).min(MAX_BACKOFF),
        None => MAX_BACKOFF,
    }
}

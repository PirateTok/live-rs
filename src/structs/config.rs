use std::fmt;
use std::time::Duration;

use crate::http::api::FetchParams;
use crate::http::ttwid::TtwidRequest;
use crate::http::ua::{random_ua, system_locale};

pub const TTWID_URL: &str = "https://www.tiktok.com/";
pub const PROFILE_CACHE_TTL: Duration = Duration::from_secs(300);
pub const PROFILE_TTWID_TIMEOUT: Duration = Duration::from_secs(10);
pub const PROFILE_SCRAPE_TIMEOUT: Duration = Duration::from_secs(15);
pub const TTWID_FETCH_ATTEMPTS: u32 = 8;
pub const TTWID_RETRY_DELAY: Duration = Duration::from_millis(750);
pub const HEALTHY_SESSION: Duration = Duration::from_secs(30);
pub const DEVICE_BLOCKED_DELAY: Duration = Duration::from_secs(2);
pub const MAX_BACKOFF: Duration = Duration::from_secs(30);

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
}

impl TikTokLiveConfig {
    pub fn new(username: impl Into<String>) -> Self {
        let (language, region) = system_locale();
        Self {
            username: username.into(),
            cdn: CdnEndpoint::default(),
            timeout: Duration::from_secs(10),
            heartbeat_interval: Duration::from_secs(10),
            max_retries: 5,
            stale_timeout: Duration::from_secs(60),
            proxy: None,
            user_agent: None,
            cookies: None,
            language,
            region,
            compress: true,
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

    pub fn ttwid_request<'a>(&'a self, user_agent: &'a str) -> TtwidRequest<'a> {
        TtwidRequest {
            url: TTWID_URL,
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
        }
    }
}

pub fn reconnect_backoff(attempt: u32) -> Duration {
    match 1u64.checked_shl(attempt) {
        Some(secs) => Duration::from_secs(secs).min(MAX_BACKOFF),
        None => MAX_BACKOFF,
    }
}

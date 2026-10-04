use std::time::Duration;

use piratetok_live_rs::structs::config::{reconnect_backoff, TikTokLiveConfig, DEVICE_BLOCKED_DELAY, HEALTHY_SESSION, MAX_BACKOFF, TTWID_FETCH_ATTEMPTS, TTWID_RETRY_DELAY};

#[test]
fn backoff_doubles_then_caps() {
    let secs: Vec<u64> = (1..=6).map(|a| reconnect_backoff(a).as_secs()).collect();
    assert_eq!(secs, vec![2, 4, 8, 16, 30, 30]);
    assert_eq!(reconnect_backoff(64), MAX_BACKOFF);
    assert_eq!(reconnect_backoff(u32::MAX), MAX_BACKOFF);
}

#[test]
fn ttwid_retry_budget_matches_spec() {
    assert_eq!(TTWID_FETCH_ATTEMPTS, 8);
    assert_eq!(TTWID_RETRY_DELAY, Duration::from_millis(750));
    assert_eq!(HEALTHY_SESSION, Duration::from_secs(30));
    assert_eq!(DEVICE_BLOCKED_DELAY, Duration::from_secs(2));
}

#[test]
fn ws_cookie_appends_session_cookies() {
    let mut config = TikTokLiveConfig::new("someone");
    assert_eq!(config.ws_cookie("abc"), "ttwid=abc");
    config.cookies = Some("sessionid=x; sid_tt=y".into());
    assert_eq!(config.ws_cookie("abc"), "ttwid=abc; sessionid=x; sid_tt=y");
}

#[test]
fn user_agent_override_wins_over_pool() {
    let mut config = TikTokLiveConfig::new("someone");
    assert!(!config.resolved_user_agent().is_empty());
    config.user_agent = Some("custom/1.0".into());
    assert_eq!(config.resolved_user_agent(), "custom/1.0");
}

#[test]
fn fetch_params_carry_connection_settings() {
    let mut config = TikTokLiveConfig::new("someone");
    config.proxy = Some("http://127.0.0.1:8080".into());
    config.language = "ro".into();
    config.region = "RO".into();
    let params = config.fetch_params();
    assert_eq!(params.proxy, Some("http://127.0.0.1:8080"));
    assert_eq!(params.language, Some("ro"));
    assert_eq!(params.region, Some("RO"));
    assert_eq!(params.cookies, None);
}

# Changelog

## 0.4.0

### Breaking
- `http::ttwid::fetch_ttwid(timeout, Option<&str>, Option<&str>)` → `fetch_ttwid(&TtwidRequest)`. `TtwidRequest { url, timeout, user_agent, proxy, attempts, retry_delay }` makes the endpoint injectable (offline fixtures), and the retry now lives inside `fetch_ttwid`, so `ProfileCache` gets it too. Build one from config with `TikTokLiveConfig::ttwid_request(&ua)` or the sanctum consts (`TTWID_URL`, `TTWID_FETCH_ATTEMPTS`, `TTWID_RETRY_DELAY`).
- `ProfileCache::cached()` returns `CacheLookup::{Hit(SigiProfile), Miss}` instead of `Option<SigiProfile>`.
- `ProfileCache` picks its UA from the pool once, at construction (`.user_agent()` still overrides), instead of per ttwid fetch.
- `record_capture` bin: needs `--features cli`; args via clap; writes `<--dir, default captures>/capture_<username>.bin` (the explicit output-path positional is gone).

### Added
- `reconnect` module (public): `judge(SessionExit, lived) -> Judgement { end, session: Keep|Rotate }` and `ReconnectBudget::record(AttemptEnd) -> Verdict::{Retry{attempt, delay}, GiveUp}`. The client loop runs on these, so the policy can be tested offline.
- Offline tests: `tests/ttwid_mock_test.rs` (a local HTTP fixture withholds the cookie N times → retry until success; always missing → error after the budget; empty `ttwid=` counts as missing; connection refused → no retry), `tests/reconnect_policy_test.rs` (backoff/give-up, healthy reset, DEVICE_BLOCKED short delay, `max_retries(0)`, scripted run counting ttwid fetches).
- `PROFILE_CACHE_TTL`, `PROFILE_TTWID_TIMEOUT`, `PROFILE_SCRAPE_TIMEOUT` moved into `structs::config`.

### Fixed
- `record_capture`: a closed heartbeat channel no longer busy-loops; pong/heartbeat send failures are logged and stop the recording instead of being dropped.

## 0.3.0

### Breaking
- `TikTokLiveStream::next_event()` now returns `Result<TikTokLiveEvent, TikTokLiveError>` instead of `Option<TikTokLiveEvent>`. The end of the stream is `Err(TikTokLiveError::ConnectionClosed)`. Migrate `while let Some(event) = stream.next_event().await` → `while let Ok(event) = stream.next_event().await`.
- `TikTokLive`, `TikTokLiveBuilder` and `TikTokLiveStream` moved from `lib.rs` into a private `client` module and are re-exported, so the paths `piratetok_live_rs::{TikTokLive, TikTokLiveBuilder, TikTokLiveStream}` are unchanged.
- In-source rustdoc moved to `docs/API.md` (lawkeeper `DEAD:2`: no comments under `src/`).

### Fixed
- ttwid: a cookie-less tiktok.com response is retried up to 8 times, 750 ms apart. A ttwid fetch failure inside the reconnect loop is now a failed attempt with backoff. It no longer ends the stream silently (cf. live-rs PR #2, live-js #1).
- The ttwid + UA session is reused across reconnects. It's rotated only on `DEVICE_BLOCKED` or after a connection that died within 30 s.
- `max_retries` counts consecutive failures: a session that stays up 30 s resets the budget. Before, a long-running stream died after `max_retries` lifetime drops.
- `heartbeat_duration` WSS param now follows `heartbeat_interval` (was hardcoded 10000).
- build.rs discipline scanner: Windows `\` paths are normalized, so `src/bin/` and `src/main.rs` exemptions apply (#1).

### Added
- `structs::config` (sanctum) exposes the reconnect tuning: `TTWID_FETCH_ATTEMPTS`, `TTWID_RETRY_DELAY`, `HEALTHY_SESSION`, `DEVICE_BLOCKED_DELAY`, `MAX_BACKOFF`, `reconnect_backoff()`, plus `TikTokLiveConfig::{resolved_user_agent, ws_cookie, fetch_params}`.
- Crate root denies `unused_must_use`, `for_loops_over_fallibles`, `dead_code`, `unused_variables`, `unused_assignments`.
- `tests/reconnect_config_test.rs` (offline).

### CLI
- `src/main.rs`: events without a decoded user/gift details are logged via `tracing::warn` and skipped instead of printing `?`; connect/room-info failures go through `tracing`. Usage moved to the README "CLI" section.

### Meta
- Homepage → https://piratetok.rosint.org.

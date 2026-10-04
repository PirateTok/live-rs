# piratetok-live-rs API

Reference docs for the public surface. Source files under `src/` carry no comments (lawkeeper `DEAD:2`), so the prose lives here.

## Flow

1. Resolve the TikTok username to a room ID (`/api-live/user/room`).
2. Get a `ttwid` cookie (anonymous GET to tiktok.com).
3. Open the WebSocket and stream protobuf-encoded events.

No signing server, no x_bogus, no msToken. Just ttwid.

## `TikTokLive`

`TikTokLive::builder(username)` returns a `TikTokLiveBuilder`. The username works with or without `@`.

## `TikTokLiveBuilder`

| Method | Default | Notes |
|---|---|---|
| `cdn(CdnEndpoint)` | `Global` | `Eu` / `Us` / `Global`. All three hit the same backend; the edge depends on IP geography, not hostname. |
| `timeout(Duration)` | 10 s | HTTP request timeout. |
| `heartbeat_interval(Duration)` | 10 s | WSS heartbeat; also sent to TikTok as `heartbeat_duration`. |
| `max_retries(u32)` | 5 | Consecutive failed attempts before `Disconnected`. `0` disables reconnection. A session that stays up 30 s resets the count. |
| `stale_timeout(Duration)` | 60 s | No data for this long → close and reconnect. |
| `proxy(url)` | none | HTTP/HTTPS/SOCKS5 for HTTP calls; WSS tunnels through HTTP CONNECT (http/https proxies). |
| `user_agent(ua)` | random pool | Fixed UA instead of the built-in pool. With the pool, a fresh UA is picked whenever the ttwid is rotated. |
| `cookies(str)` | none | Session cookies appended next to ttwid in the WSS cookie header. For 18+ room info pass cookies to `fetch_room_info` instead. |
| `language(code)` | system locale, `en` | API params + `Accept-Language`. |
| `region(code)` | system locale, `US` | API params. |
| `compress(bool)` | `true` | Request gzip WSS frames. Decode handles both, so toggling is always safe. |
| `connect()` | | Resolves the room ID, emits `Connected`, then runs the reconnect loop in a background task. |

## Reconnect loop

- ttwid fetch: a cookie-less response is retried up to `TTWID_FETCH_ATTEMPTS` (8) times, `TTWID_RETRY_DELAY` (750 ms) apart. If it still fails, that's a failed attempt, not a fatal error.
- The ttwid + UA session is reused across reconnects. It's rotated on `DEVICE_BLOCKED` (retry after `DEVICE_BLOCKED_DELAY`, 2 s) and when a connection dies before `HEALTHY_SESSION` (30 s).
- Backoff: `reconnect_backoff(attempt)` = 2^attempt s, capped at `MAX_BACKOFF` (30 s).
- Emits `Reconnecting { attempt, max_retries, delay_secs }` before each retry, and `Disconnected` when done.
- All tuning constants live in `structs::config` (the sanctum).

## `TikTokLiveStream`

`next_event().await -> Result<TikTokLiveEvent, TikTokLiveError>`: the next event, or `Err(ConnectionClosed)` once the stream has finished (after `Disconnected`). Dropping the stream aborts the background task and closes the WebSocket.

## `TikTokLiveConfig` (`structs::config`)

Internal config the builder fills. Fields mirror the builder. Helpers: `browser_language()` (`en-US`), `accept_language()` (`en-US,en;q=0.9`), `resolved_user_agent()`, `ws_cookie(ttwid)`, `fetch_params()`.

## Room info / audience

See README: `fetch_room_info` (cookies only needed for 18+ rooms) and `fetch_room_audience` (login-gated, `SessionRequired` without cookies). The `RoomUserSeq` event carries the top-viewers box for free via `msg.top_viewers()`.

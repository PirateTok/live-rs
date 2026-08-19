use crate::errors::TikTokLiveError;
use crate::http::ua::random_ua;

const TIKTOK_URL: &str = "https://www.tiktok.com/";

/// Fetch a fresh ttwid cookie from TikTok via unauthenticated GET.
///
/// The ttwid is a device fingerprint token set via `Set-Cookie` on any
/// request to tiktok.com. It requires no login, no signing, no browser.
/// This is the sole credential needed for WSS live stream connections.
///
/// Uses a random UA from the built-in pool. Pass a custom UA to override.
pub async fn fetch_ttwid(timeout: std::time::Duration, user_agent: Option<&str>, proxy: Option<&str>) -> Result<String, TikTokLiveError> {
    let ua = user_agent.unwrap_or_else(|| random_ua());

    let mut builder = reqwest::Client::builder()
        .timeout(timeout)
        .user_agent(ua)
        .redirect(reqwest::redirect::Policy::none());

    if let Some(proxy_url) = proxy {
        builder = builder.proxy(reqwest::Proxy::all(proxy_url).map_err(TikTokLiveError::Http)?);
    }

    let client = builder.build().map_err(TikTokLiveError::Http)?;

    // TikTok serves the ttwid Set-Cookie intermittently on unauthenticated
    // GETs to tiktok.com (observed ~1 in 5 on some routes/regions, and more
    // reliably over HTTP/2 than HTTP/1.1). A single attempt therefore often
    // fails with "no ttwid cookie". Retry a bounded number of times with a
    // short delay before giving up. Transport errors (timeouts, connection
    // failures) still propagate immediately via `?`.
    const MAX_ATTEMPTS: u32 = 8;
    const RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(750);

    let mut attempt: u32 = 0;
    loop {
        attempt += 1;

        let resp = client.get(TIKTOK_URL).send().await?;

        for cookie_header in resp.headers().get_all("set-cookie") {
            let value = cookie_header
                .to_str()
                .map_err(|e| TikTokLiveError::invalid(format!("set-cookie header: {e}")))?;

            if let Some(ttwid) = extract_ttwid(value) {
                return Ok(ttwid);
            }
        }

        if attempt >= MAX_ATTEMPTS {
            return Err(TikTokLiveError::invalid("no ttwid cookie in tiktok.com response"));
        }

        tokio::time::sleep(RETRY_DELAY).await;
    }
}

/// Extract the ttwid value from a Set-Cookie header string.
/// Format: `ttwid=1|<base64>|<ts>|<hmac>; Path=/; ...`
fn extract_ttwid(set_cookie: &str) -> Option<String> {
    if !set_cookie.starts_with("ttwid=") {
        return None;
    }

    let value = set_cookie.strip_prefix("ttwid=")?;
    let end = match value.find(';') {
        Some(pos) => pos,
        None => value.len(),
    };
    let ttwid = &value[..end];

    if ttwid.is_empty() {
        return None;
    }

    Some(ttwid.to_string())
}

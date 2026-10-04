use std::time::Duration;

use reqwest::header::HeaderMap;

use crate::errors::TikTokLiveError;

#[derive(Clone, Debug)]
pub struct TtwidRequest<'a> {
    pub url: &'a str,
    pub timeout: Duration,
    pub user_agent: &'a str,
    pub proxy: Option<&'a str>,
    pub attempts: u32,
    pub retry_delay: Duration,
}

pub async fn fetch_ttwid(request: &TtwidRequest<'_>) -> Result<String, TikTokLiveError> {
    let client = build_client(request)?;
    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        let resp = client.get(request.url).send().await?;
        let status = resp.status();
        let Some(ttwid) = find_ttwid(resp.headers())? else {
            if attempt >= request.attempts {
                return Err(TikTokLiveError::ttwid(format!("no ttwid cookie after {attempt} attempts (last http {status})")));
            }
            tracing::warn!(attempt, %status, "no ttwid cookie in response, retrying");
            tokio::time::sleep(request.retry_delay).await;
            continue;
        };
        return Ok(ttwid);
    }
}

fn build_client(request: &TtwidRequest<'_>) -> Result<reqwest::Client, TikTokLiveError> {
    let mut builder = reqwest::Client::builder().timeout(request.timeout).user_agent(request.user_agent).redirect(reqwest::redirect::Policy::none());
    for proxy in request.proxy.iter() {
        builder = builder.proxy(reqwest::Proxy::all(*proxy)?);
    }
    Ok(builder.build()?)
}

fn find_ttwid(headers: &HeaderMap) -> Result<Option<String>, TikTokLiveError> {
    for header in headers.get_all("set-cookie") {
        let value = header.to_str().map_err(|e| TikTokLiveError::invalid(format!("set-cookie header: {e}")))?;
        let Some(rest) = value.strip_prefix("ttwid=") else {
            continue;
        };
        let Some(token) = rest.split(';').next() else {
            continue;
        };
        if token.is_empty() {
            continue;
        }
        return Ok(Some(token.to_string()));
    }
    Ok(None)
}

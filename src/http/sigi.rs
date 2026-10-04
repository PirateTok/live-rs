use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::Value;

use crate::errors::TikTokLiveError;
use crate::http::api::FetchParams;
use crate::structs::config::{fetch_locale, fetch_user_agent, json_bool, json_i64, json_str};

#[derive(Clone, Debug)]
pub struct SigiProfile {
    pub user_id: String,
    pub unique_id: String,
    pub nickname: String,
    pub bio: String,
    pub avatar_thumb: String,
    pub avatar_medium: String,
    pub avatar_large: String,
    pub verified: bool,
    pub private_account: bool,
    pub is_organization: bool,
    pub room_id: String,
    pub bio_link: Option<String>,
    pub follower_count: i64,
    pub following_count: i64,
    pub heart_count: i64,
    pub video_count: i64,
    pub friend_count: i64,
}

impl SigiProfile {
    const MARKER: &'static str = r#"id="__UNIVERSAL_DATA_FOR_REHYDRATION__""#;
}

pub async fn scrape_profile(username: &str, ttwid: &str, params: &FetchParams<'_>) -> Result<SigiProfile, TikTokLiveError> {
    let clean = normalize_username(username);
    let locale = fetch_locale(params);
    let mut headers = HeaderMap::new();
    headers.insert("Cookie", HeaderValue::from_str(&profile_cookie(ttwid, params)).map_err(TikTokLiveError::invalid)?);
    headers.insert(
        "Accept-Language",
        HeaderValue::from_str(&format!("{}-{},{};q=0.9", locale.language, locale.region, locale.language)).map_err(TikTokLiveError::invalid)?,
    );
    let mut builder = reqwest::Client::builder().timeout(params.timeout).user_agent(fetch_user_agent(params)).default_headers(headers);
    for proxy in params.proxy.iter() {
        builder = builder.proxy(reqwest::Proxy::all(*proxy)?);
    }
    let html = builder.build()?.get(format!("{}@{clean}", params.endpoints.web)).send().await?.text().await?;
    parse_profile(&clean, &html)
}

pub fn normalize_username(username: &str) -> String {
    username.trim().trim_start_matches('@').to_lowercase()
}

fn profile_cookie(ttwid: &str, params: &FetchParams<'_>) -> String {
    let mut cookie = format!("ttwid={ttwid}");
    for extra in params.cookies.iter() {
        for pair in extra.split("; ") {
            if !pair.is_empty() && !pair.starts_with("ttwid=") {
                cookie.push_str("; ");
                cookie.push_str(pair);
            }
        }
    }
    cookie
}

pub fn parse_profile(username: &str, html: &str) -> Result<SigiProfile, TikTokLiveError> {
    let blob: Value = serde_json::from_str(extract_sigi_json(html)?)?;
    let Some(detail) = blob.pointer("/__DEFAULT_SCOPE__/webapp.user-detail") else {
        return Err(TikTokLiveError::ProfileScrape("missing __DEFAULT_SCOPE__/webapp.user-detail".into()));
    };
    match json_i64(detail, "/statusCode") {
        0 => {}
        10222 => return Err(TikTokLiveError::ProfilePrivate(username.to_string())),
        10221 | 10223 => return Err(TikTokLiveError::ProfileNotFound(username.to_string())),
        code => return Err(TikTokLiveError::ProfileError(code)),
    }
    let Some(user) = detail.pointer("/userInfo/user") else {
        return Err(TikTokLiveError::ProfileScrape("missing userInfo.user".into()));
    };
    let Some(stats) = detail.pointer("/userInfo/stats") else {
        return Err(TikTokLiveError::ProfileScrape("missing userInfo.stats".into()));
    };
    Ok(SigiProfile {
        user_id: json_str(user, "/id"),
        unique_id: json_str(user, "/uniqueId"),
        nickname: json_str(user, "/nickname"),
        bio: json_str(user, "/signature"),
        avatar_thumb: json_str(user, "/avatarThumb"),
        avatar_medium: json_str(user, "/avatarMedium"),
        avatar_large: json_str(user, "/avatarLarger"),
        verified: json_bool(user, "/verified"),
        private_account: json_bool(user, "/privateAccount"),
        is_organization: json_i64(user, "/isOrganization") != 0,
        room_id: json_str(user, "/roomId"),
        bio_link: user.pointer("/bioLink/link").and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string),
        follower_count: json_i64(stats, "/followerCount"),
        following_count: json_i64(stats, "/followingCount"),
        heart_count: json_i64(stats, "/heartCount"),
        video_count: json_i64(stats, "/videoCount"),
        friend_count: json_i64(stats, "/friendCount"),
    })
}

fn extract_sigi_json(html: &str) -> Result<&str, TikTokLiveError> {
    let missing = |what: &str| TikTokLiveError::ProfileScrape(what.to_string());
    let marker = html.find(SigiProfile::MARKER).ok_or_else(|| missing("SIGI script tag not found in HTML"))?;
    let after_marker = &html[marker..];
    let start = marker + after_marker.find('>').ok_or_else(|| missing("no > after SIGI marker"))? + 1;
    let end = start + html[start..].find("</script>").ok_or_else(|| missing("no </script> after SIGI JSON"))?;
    let json = &html[start..end];
    if json.is_empty() {
        return Err(missing("empty SIGI JSON blob"));
    }
    Ok(json)
}

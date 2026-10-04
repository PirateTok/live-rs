use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};

use crate::errors::TikTokLiveError;
use crate::http::parse::{parse_audience, parse_owner_id, parse_room_id, parse_room_info};
use crate::http::ua::system_timezone;
use crate::structs::config::{fetch_locale, fetch_user_agent, Endpoints};
use crate::structs::events::{RoomAudience, RoomInfo};

pub use crate::http::parse::RoomIdResponse;

#[derive(Clone, Debug)]
pub struct FetchParams<'a> {
    pub timeout: Duration,
    pub cookies: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub proxy: Option<&'a str>,
    pub language: Option<&'a str>,
    pub region: Option<&'a str>,
    pub endpoints: &'a Endpoints,
}

#[derive(Clone, Copy, Debug)]
pub enum AnchorId<'a> {
    Known(&'a str),
    FromRoomInfo,
}

fn build_client(params: &FetchParams<'_>) -> Result<reqwest::Client, TikTokLiveError> {
    let mut headers = HeaderMap::new();
    headers.insert("Referer", HeaderValue::from_static("https://www.tiktok.com/"));
    for cookies in params.cookies.iter() {
        if !cookies.is_empty() {
            headers.insert("Cookie", HeaderValue::from_str(cookies).map_err(TikTokLiveError::invalid)?);
        }
    }
    let mut builder = reqwest::Client::builder().timeout(params.timeout).user_agent(fetch_user_agent(params)).default_headers(headers);
    for proxy in params.proxy.iter() {
        builder = builder.proxy(reqwest::Proxy::all(*proxy)?);
    }
    Ok(builder.build()?)
}

async fn get(params: &FetchParams<'_>, url: &str) -> Result<(u16, String), TikTokLiveError> {
    let resp = build_client(params)?.get(url).send().await?;
    let status = resp.status().as_u16();
    Ok((status, resp.text().await?))
}

pub async fn fetch_room_id(username: &str, params: FetchParams<'_>) -> Result<RoomIdResponse, TikTokLiveError> {
    let clean = username.trim().trim_start_matches('@');
    let locale = fetch_locale(&params);
    let url = format!(
        "{}api-live/user/room?aid=1988&app_name=tiktok_web&device_platform=web_pc&app_language={}&browser_language={}&region={}&user_is_login=false&uniqueId={clean}&sourceType=54&staleTime=600000",
        params.endpoints.web, locale.language, locale.browser_language, locale.region
    );
    let (status, body) = get(&params, &url).await?;
    parse_room_id(clean, status, &body)
}

pub async fn fetch_room_info(room_id: &str, params: FetchParams<'_>) -> Result<RoomInfo, TikTokLiveError> {
    let locale = fetch_locale(&params);
    let tz = system_timezone();
    let url = format!(
        "{}room/info/?aid=1988&app_name=tiktok_web&device_platform=web_pc&app_language={lang}&browser_language={}&browser_name=Mozilla&browser_online=true&browser_platform=Win32&browser_version=5.0+(Windows+NT+10.0%3B+Win64%3B+x64)&cookie_enabled=true&focus_state=true&from_page=user&screen_height=1080&screen_width=1920&tz_name={}&webcast_language={lang}&room_id={room_id}",
        params.endpoints.webcast,
        locale.browser_language,
        urlencoding::encode(&tz),
        lang = locale.language,
    );
    let (status, body) = get(&params, &url).await?;
    parse_room_info(status, &body)
}

pub async fn fetch_room_audience(room_id: &str, anchor: AnchorId<'_>, params: FetchParams<'_>) -> Result<RoomAudience, TikTokLiveError> {
    let anchor_id = match anchor {
        AnchorId::Known(id) => id.to_string(),
        AnchorId::FromRoomInfo => parse_owner_id(&fetch_room_info(room_id, params.clone()).await?.raw_json)?,
    };
    let locale = fetch_locale(&params);
    let url = format!(
        "{}ranklist/online_audience/?aid=1988&app_name=tiktok_web&device_platform=web_pc&app_language={}&browser_language={}&channel=tiktok_web&room_id={room_id}&anchor_id={anchor_id}",
        params.endpoints.webcast, locale.language, locale.browser_language
    );
    let (status, body) = get(&params, &url).await?;
    parse_audience(status, &body)
}

use serde_json::Value;

use crate::errors::TikTokLiveError;
use crate::structs::config::{json_bool, json_i64, json_str};
use crate::structs::events::{AudienceViewer, RoomAudience, RoomInfo, StreamUrl};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomIdResponse {
    pub room_id: String,
    pub anchor_id: String,
}

pub fn parse_room_id(username: &str, http_status: u16, body: &str) -> Result<RoomIdResponse, TikTokLiveError> {
    if http_status == 403 || http_status == 429 {
        return Err(TikTokLiveError::TikTokBlocked(format!("http {http_status}")));
    }
    if body.trim().is_empty() {
        return Err(TikTokLiveError::TikTokBlocked(format!("empty response (http {http_status})")));
    }
    let json: Value = match serde_json::from_str(body) {
        Ok(json) => json,
        Err(e) => return Err(TikTokLiveError::TikTokBlocked(format!("non-JSON response (http {http_status}): {e}"))),
    };
    let Some(code) = json.get("statusCode").and_then(Value::as_i64) else {
        return Err(TikTokLiveError::invalid("no statusCode in response"));
    };
    match code {
        0 => {}
        19881007 => return Err(TikTokLiveError::UserNotFound(username.to_string())),
        other => return Err(TikTokLiveError::ApiError(other)),
    }
    let Some(room_id) = json.pointer("/data/user/roomId").and_then(Value::as_str) else {
        return Err(TikTokLiveError::RoomIdMissing);
    };
    if room_id.is_empty() || room_id == "0" {
        return Err(TikTokLiveError::HostNotOnline("no active room".into()));
    }
    let status = live_status(&json)?;
    if status != 2 {
        return Err(TikTokLiveError::HostNotOnline(format!("status={status}")));
    }
    Ok(RoomIdResponse {
        room_id: room_id.to_string(),
        anchor_id: json_str(&json, "/data/user/id"),
    })
}

fn live_status(json: &Value) -> Result<i64, TikTokLiveError> {
    for pointer in ["/data/liveRoom/status", "/data/user/status"] {
        let Some(status) = json.pointer(pointer).and_then(Value::as_i64) else {
            continue;
        };
        return Ok(status);
    }
    Err(TikTokLiveError::HostNotOnline("no live status in response".into()))
}

pub fn parse_room_info(http_status: u16, body: &str) -> Result<RoomInfo, TikTokLiveError> {
    if body.trim().is_empty() {
        return Err(TikTokLiveError::invalid(format!("empty response from room/info (http {http_status})")));
    }
    let json: Value = serde_json::from_str(body)?;
    for code in json.get("status_code").and_then(Value::as_i64).iter() {
        match *code {
            0 => {}
            4003110 => return Err(TikTokLiveError::AgeRestricted("18+ room — pass session cookies to fetch_room_info()".into())),
            other => return Err(TikTokLiveError::ApiError(other)),
        }
    }
    let Some(Value::Object(..)) = json.get("data") else {
        return Err(TikTokLiveError::invalid("missing 'data' in room info"));
    };
    Ok(RoomInfo {
        title: json_str(&json, "/data/title"),
        viewers: json_i64(&json, "/data/user_count"),
        likes: json_i64(&json, "/data/stats/like_count"),
        total_viewers: json_i64(&json, "/data/stats/total_user"),
        stream_url: parse_stream_urls(&json)?,
        raw_json: body.to_string(),
    })
}

fn parse_stream_urls(json: &Value) -> Result<Option<StreamUrl>, TikTokLiveError> {
    let Some(raw) = json.pointer("/data/stream_url/live_core_sdk_data/pull_data/stream_data").and_then(Value::as_str) else {
        return Ok(None);
    };
    let nested: Value = serde_json::from_str(raw)?;
    let flv = |quality: &str| nested.pointer(&format!("/data/{quality}/main/flv")).and_then(Value::as_str).map(str::to_string);
    let hd_candidates = [flv("hd"), flv("uhd")];
    Ok(Some(StreamUrl {
        flv_origin: flv("origin"),
        flv_hd: hd_candidates.into_iter().find_map(|candidate| candidate),
        flv_sd: flv("sd"),
        flv_ld: flv("ld"),
        flv_ao: flv("ao"),
    }))
}

pub fn parse_owner_id(room_info_json: &str) -> Result<String, TikTokLiveError> {
    let json: Value = serde_json::from_str(room_info_json)?;
    let owner = json_str(&json, "/data/owner/id_str");
    if owner.is_empty() {
        return Err(TikTokLiveError::invalid("no owner id in room info"));
    }
    Ok(owner)
}

pub fn parse_audience(http_status: u16, body: &str) -> Result<RoomAudience, TikTokLiveError> {
    if body.trim().is_empty() {
        return Err(TikTokLiveError::invalid(format!("empty response from online_audience (http {http_status})")));
    }
    let json: Value = serde_json::from_str(body)?;
    let Some(code) = json.get("status_code").and_then(Value::as_i64) else {
        return Err(TikTokLiveError::invalid("no status_code in online_audience response"));
    };
    match code {
        0 => {}
        20003 => return Err(TikTokLiveError::SessionRequired("audience roster needs login — pass session cookies to fetch_room_audience()".into())),
        other => return Err(TikTokLiveError::invalid(format!("online_audience status_code={other} {}", json_str(&json, "/data/message")))),
    }
    let Some(Value::Object(..)) = json.get("data") else {
        return Err(TikTokLiveError::invalid("missing 'data' in online_audience"));
    };
    let mut viewers = Vec::new();
    for ranks in json.pointer("/data/ranks").and_then(Value::as_array).iter() {
        for rank in ranks.iter() {
            let Some(user) = rank.get("user") else {
                continue;
            };
            viewers.push(audience_viewer(rank, user));
        }
    }
    Ok(RoomAudience {
        total: json_i64(&json, "/data/total"),
        anonymous: json_i64(&json, "/data/anonymous"),
        viewers,
        raw_json: body.to_string(),
    })
}

fn audience_viewer(rank: &Value, user: &Value) -> AudienceViewer {
    let id_str = json_str(user, "/id_str");
    let user_id = if id_str.is_empty() { json_i64(user, "/id").to_string() } else { id_str };
    AudienceViewer {
        rank: json_i64(rank, "/rank"),
        score: json_i64(rank, "/score"),
        user_id,
        username: json_str(user, "/display_id"),
        nickname: json_str(user, "/nickname"),
        sec_uid: json_str(user, "/sec_uid"),
        avatar_url: user.pointer("/avatar_thumb/url_list/0").and_then(Value::as_str).map(str::to_string),
        follower_count: json_i64(user, "/follow_info/follower_count"),
        verified: json_bool(user, "/verified"),
        is_follower: json_bool(user, "/is_follower"),
        is_following: json_bool(user, "/is_following"),
        is_subscriber: json_bool(user, "/is_subscribe"),
    }
}

use piratetok_live_rs::errors::TikTokLiveError;
use piratetok_live_rs::http::parse::{parse_audience, parse_owner_id, parse_room_id, parse_room_info, RoomIdResponse};

const LIVE: &str = r#"{"statusCode":0,"data":{"user":{"roomId":"7001","id":"55","status":2},"liveRoom":{"status":2}}}"#;

#[test]
fn f1_live_user_gives_room_and_anchor() {
    assert_eq!(
        parse_room_id("u", 200, LIVE).unwrap(),
        RoomIdResponse {
            room_id: "7001".into(),
            anchor_id: "55".into()
        }
    );
}

#[test]
fn f1_error_mapping() {
    let cases: Vec<(u16, &str, fn(&TikTokLiveError) -> bool)> = vec![
        (200, r#"{"statusCode":19881007}"#, |e| matches!(e, TikTokLiveError::UserNotFound(u) if u == "u")),
        (200, r#"{"statusCode":4003}"#, |e| matches!(e, TikTokLiveError::ApiError(4003))),
        (403, "", |e| matches!(e, TikTokLiveError::TikTokBlocked(m) if m.contains("403"))),
        (429, r#"{"statusCode":0}"#, |e| matches!(e, TikTokLiveError::TikTokBlocked(m) if m.contains("429"))),
        (200, "   ", |e| matches!(e, TikTokLiveError::TikTokBlocked(m) if m.contains("empty"))),
        (200, "<html>captcha</html>", |e| matches!(e, TikTokLiveError::TikTokBlocked(m) if m.contains("non-JSON"))),
        (200, r#"{"statusCode":0,"data":{"user":{"roomId":"0"}}}"#, |e| matches!(e, TikTokLiveError::HostNotOnline(..))),
        (
            200,
            r#"{"statusCode":0,"data":{"user":{"roomId":"7001","status":4},"liveRoom":{"status":4}}}"#,
            |e| matches!(e, TikTokLiveError::HostNotOnline(m) if m == "status=4"),
        ),
        (200, r#"{"statusCode":0,"data":{"user":{}}}"#, |e| matches!(e, TikTokLiveError::RoomIdMissing)),
    ];
    for (status, body, check) in cases {
        let err = parse_room_id("u", status, body).expect_err(body);
        assert!(check(&err), "{status} {body} -> {err}");
    }
}

#[test]
fn f1_offline_error_message_is_not_ip_blocked() {
    let err = parse_room_id("u", 200, r#"{"statusCode":0,"data":{"user":{"roomId":"0"}}}"#).unwrap_err();
    assert!(!err.to_string().contains("blocked"), "{err}");
}

#[test]
fn f9_room_info_fields_and_stream_urls() {
    let stream_data = r#"{"data":{"origin":{"main":{"flv":"o.flv"}},"uhd":{"main":{"flv":"uhd.flv"}},"sd":{"main":{"flv":"sd.flv"}}}}"#;
    let body = serde_json::json!({
        "status_code": 0,
        "data": {"title": "t", "user_count": 12, "stats": {"like_count": 34, "total_user": 56},
                 "stream_url": {"live_core_sdk_data": {"pull_data": {"stream_data": stream_data}}}}
    })
    .to_string();
    let info = parse_room_info(200, &body).unwrap();
    assert_eq!((info.title.as_str(), info.viewers, info.likes, info.total_viewers), ("t", 12, 34, 56));
    let urls = info.stream_url.unwrap();
    assert_eq!(urls.flv_origin.as_deref(), Some("o.flv"));
    assert_eq!(urls.flv_hd.as_deref(), Some("uhd.flv"), "hd falls back to uhd");
    assert_eq!(urls.flv_sd.as_deref(), Some("sd.flv"));
    assert_eq!(urls.flv_ld, None);
}

#[test]
fn f9_age_restricted_and_errors() {
    let err = parse_room_info(200, r#"{"status_code":4003110}"#).unwrap_err();
    assert!(matches!(err, TikTokLiveError::AgeRestricted(ref m) if m.contains("cookies")), "{err}");
    assert!(matches!(parse_room_info(200, r#"{"status_code":10011}"#).unwrap_err(), TikTokLiveError::ApiError(10011)));
    assert!(matches!(parse_room_info(500, "").unwrap_err(), TikTokLiveError::InvalidResponse(ref m) if m.contains("500")));
    assert!(matches!(parse_room_info(200, r#"{"status_code":0}"#).unwrap_err(), TikTokLiveError::InvalidResponse(..)));
}

#[test]
fn f10_audience_roster() {
    let body = serde_json::json!({
        "status_code": 0,
        "data": {"total": 40, "anonymous": 3, "ranks": [
            {"rank": 1, "score": 900, "user": {"id_str": "11", "display_id": "alice", "nickname": "A", "sec_uid": "s1",
              "avatar_thumb": {"url_list": ["a.jpg"]}, "follow_info": {"follower_count": 7}, "verified": true,
              "is_follower": true, "is_following": false, "is_subscribe": true}},
            {"rank": 2, "score": 5},
            {"rank": 3, "score": 1, "user": {"id": 22, "display_id": "bob", "nickname": "B"}}
        ]}
    })
    .to_string();
    let audience = parse_audience(200, &body).unwrap();
    assert_eq!((audience.total, audience.anonymous, audience.viewers.len()), (40, 3, 2));
    let a = &audience.viewers[0];
    assert_eq!((a.user_id.as_str(), a.username.as_str(), a.avatar_url.as_deref(), a.follower_count), ("11", "alice", Some("a.jpg"), 7));
    assert!(a.verified && a.is_follower && !a.is_following && a.is_subscriber);
    assert_eq!(audience.viewers[1].user_id, "22", "numeric id fallback");
}

#[test]
fn f10_audience_errors() {
    let err = parse_audience(200, r#"{"status_code":20003}"#).unwrap_err();
    assert!(matches!(err, TikTokLiveError::SessionRequired(ref m) if m.contains("cookies")), "{err}");
    let err = parse_audience(200, r#"{"status_code":10011,"data":{"message":"bad anchor"}}"#).unwrap_err();
    assert!(matches!(err, TikTokLiveError::InvalidResponse(ref m) if m.contains("10011") && m.contains("bad anchor")), "{err}");
    assert!(matches!(parse_audience(200, "").unwrap_err(), TikTokLiveError::InvalidResponse(..)));
    assert!(matches!(parse_audience(200, "{}").unwrap_err(), TikTokLiveError::InvalidResponse(..)));
}

#[test]
fn f10_owner_id_from_room_info() {
    assert_eq!(parse_owner_id(r#"{"data":{"owner":{"id_str":"99"}}}"#).unwrap(), "99");
    assert!(parse_owner_id(r#"{"data":{}}"#).is_err());
}

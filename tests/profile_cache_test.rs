mod common;

use common::{http_fixture, ok, ttwid_cookie};
use piratetok_live_rs::errors::TikTokLiveError;
use piratetok_live_rs::helpers::profile_cache::{CacheLookup, ProfileCache};
use piratetok_live_rs::http::sigi::parse_profile;
use piratetok_live_rs::structs::config::Endpoints;

fn sigi(status: i64) -> String {
    let blob = serde_json::json!({"__DEFAULT_SCOPE__": {"webapp.user-detail": {
        "statusCode": status,
        "userInfo": {
            "user": {"id": "1", "uniqueId": "alice", "nickname": "Alice", "signature": "bio", "avatarThumb": "t", "avatarMedium": "m",
                     "avatarLarger": "l", "verified": true, "privateAccount": false, "isOrganization": 0, "roomId": "", "bioLink": {"link": "x.y"}},
            "stats": {"followerCount": 10, "followingCount": 2, "heartCount": 3, "videoCount": 4, "friendCount": 5}
        }
    }}});
    format!(r#"<html><script id="__UNIVERSAL_DATA_FOR_REHYDRATION__" type="application/json">{blob}</script></html>"#)
}

#[test]
fn parse_profile_fields() {
    let p = parse_profile("alice", &sigi(0)).unwrap();
    assert_eq!((p.nickname.as_str(), p.avatar_large.as_str(), p.follower_count, p.bio_link.as_deref()), ("Alice", "l", 10, Some("x.y")));
    assert!(matches!(parse_profile("p", &sigi(10222)), Err(TikTokLiveError::ProfilePrivate(..))));
    assert!(matches!(parse_profile("p", &sigi(10221)), Err(TikTokLiveError::ProfileNotFound(..))));
    assert!(matches!(parse_profile("p", "<html></html>"), Err(TikTokLiveError::ProfileScrape(..))));
}

#[tokio::test]
async fn cache_hits_and_negative_cache_against_local_origin() {
    let origin = http_fixture(|path, nth| match path {
        "/" => ok(&ttwid_cookie(nth), ""),
        "/@alice" => ok("", &sigi(0)),
        "/@ghost" => ok("", &sigi(10221)),
        "/@priv" => ok("", &sigi(10222)),
        other => panic!("unexpected path {other}"),
    })
    .await;
    let endpoints = Endpoints {
        web: origin.base.clone(),
        webcast: origin.base.clone(),
        ws: None,
    };
    let cache = ProfileCache::new().endpoints(endpoints);

    assert!(matches!(cache.cached("alice"), CacheLookup::Miss));
    let first = cache.fetch("@Alice").await.unwrap();
    let second = cache.fetch("alice").await.unwrap();
    assert_eq!(first.unique_id, second.unique_id);
    assert!(matches!(cache.cached("ALICE"), CacheLookup::Hit(..)));
    assert_eq!(origin.hits("/@alice"), 1, "second fetch served from cache");

    for _ in 0..2 {
        assert!(matches!(cache.fetch("ghost").await, Err(TikTokLiveError::ProfileNotFound(..))));
        assert!(matches!(cache.fetch("priv").await, Err(TikTokLiveError::ProfilePrivate(..))));
    }
    assert_eq!(origin.hits("/@ghost"), 1, "not-found is negatively cached");
    assert_eq!(origin.hits("/@priv"), 1, "private is negatively cached");
    assert_eq!(origin.hits("/"), 1, "ttwid fetched once per cache");

    cache.invalidate("alice");
    cache.fetch("alice").await.unwrap();
    assert_eq!(origin.hits("/@alice"), 2, "invalidate forces a refetch");
}

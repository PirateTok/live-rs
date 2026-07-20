//! Audience — who's actually in the room, by name.
//!
//! Two sources, both shown:
//!   1. HTTP full roster (`fetch_room_audience`) — every named viewer, like the
//!      web viewer panel. Needs session cookies (TikTok gates it behind login).
//!   2. WSS top-viewers box (`RoomUserSeq.top_viewers()`) — the top-3 next to
//!      the viewer counter. No cookies needed, updates live.
//!
//! Usage:
//!   cargo run --example audience -- <tiktok_username> [cookies]
//!
//! Without cookies you only get source 2. With cookies you get both:
//!   cargo run --example audience -- username "sessionid=abc; sid_tt=abc"

use piratetok_live_rs::http::api::{fetch_room_audience, fetch_room_id, FetchParams};
use piratetok_live_rs::structs::TikTokLiveEvent;
use piratetok_live_rs::TikTokLive;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <tiktok_username> [cookies]", args[0]);
        return;
    }

    let username = &args[1];
    let cookies = args.get(2).map(|s| s.as_str());
    let timeout = std::time::Duration::from_secs(10);

    let room = match fetch_room_id(username, FetchParams { timeout, ..Default::default() }).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {e}");
            return;
        }
    };
    println!("room_id: {}  anchor_id: {}", room.room_id, room.anchor_id);

    // Source 1: full roster over HTTP (login-gated)
    match fetch_room_audience(
        &room.room_id,
        Some(&room.anchor_id),
        FetchParams { timeout, cookies, ..Default::default() },
    )
    .await
    {
        Ok(audience) => {
            println!();
            println!("=== Full roster: {} in room ({} anonymous) ===", audience.total, audience.anonymous);
            for v in &audience.viewers {
                let mut tags = Vec::new();
                if v.verified {
                    tags.push("verified");
                }
                if v.is_follower {
                    tags.push("follower");
                }
                if v.is_subscriber {
                    tags.push("sub");
                }
                let tags = if tags.is_empty() { String::new() } else { format!(" [{}]", tags.join(",")) };
                println!(
                    "  #{:<3} @{} ({}) score={} followers={}{}",
                    v.rank, v.username, v.nickname, v.score, v.follower_count, tags
                );
            }
        }
        Err(e) => {
            eprintln!();
            eprintln!("Full roster unavailable: {e}");
            if cookies.is_none() {
                eprintln!("Hint: pass session cookies as the second argument to get the full list");
            }
        }
    }

    // Source 2: live top-viewers box over WSS (no cookies)
    println!();
    println!("=== Watching top-viewers box (WSS, ctrl-c to quit) ===");
    let mut stream = match TikTokLive::builder(username).connect().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Connect failed: {e}");
            return;
        }
    };

    while let Some(event) = stream.next_event().await {
        match event {
            TikTokLiveEvent::RoomUserSeq(msg) => {
                let top = msg.top_viewers();
                if top.is_empty() {
                    println!("{} viewers (no top box in this update)", msg.viewer_count);
                    continue;
                }
                let names: Vec<String> = top
                    .iter()
                    .filter_map(|c| c.user.as_ref().map(|u| format!("#{} {} ({})", c.rank, u.nickname, c.score)))
                    .collect();
                println!("{} viewers | top: {}", msg.viewer_count, names.join(" | "));
            }
            TikTokLiveEvent::Disconnected => break,
            _ => {}
        }
    }
}

#![deny(unused_must_use)]
#![deny(for_loops_over_fallibles)]
#![deny(dead_code)]
#![deny(unused_variables)]
#![deny(unused_assignments)]

use clap::Parser;
use piratetok_live_rs::http::api::{fetch_room_info, FetchParams};
use piratetok_live_rs::structs::proto::messages::WebcastGiftMessage;
use piratetok_live_rs::structs::proto::user::UserIdentity;
use piratetok_live_rs::structs::TikTokLiveEvent;
use piratetok_live_rs::TikTokLive;
use tracing::info;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(about = "Connect to a TikTok Live stream and print events")]
struct Args {
    #[arg(short, long, env = "TIKTOK_USERNAME", help = "TikTok username (with or without @)")]
    username: String,

    #[arg(long, help = "Also fetch room info (title, stream URLs)")]
    room_info: bool,

    #[arg(long, env = "TIKTOK_COOKIES", help = "Session cookies for 18+ room info (only with --room-info)")]
    cookies: Option<String>,
}

enum Flow {
    Continue,
    Stop,
}

#[tokio::main]
async fn main() {
    let dotenv = dotenvy::dotenv();
    let filter = match EnvFilter::builder().with_default_directive(LevelFilter::INFO.into()).from_env() {
        Ok(filter) => filter,
        Err(e) => panic!("invalid RUST_LOG: {e}"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();
    match dotenv {
        Ok(path) => info!(path = %path.display(), "loaded .env"),
        Err(e) => tracing::warn!(error = %e, "no .env loaded"),
    }

    let args = Args::parse();

    let mut stream = match TikTokLive::builder(&args.username).connect().await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "failed to connect");
            return;
        }
    };

    loop {
        let event = match stream.next_event().await {
            Ok(event) => event,
            Err(e) => {
                tracing::warn!(error = %e, "event stream closed");
                break;
            }
        };
        match handle_event(event, &args).await {
            Flow::Continue => {}
            Flow::Stop => break,
        }
    }
}

async fn handle_event(event: TikTokLiveEvent, args: &Args) -> Flow {
    match event {
        TikTokLiveEvent::Connected { room_id } => {
            println!("[connected] room_id={room_id}");
            if args.room_info {
                print_room_info(&room_id, args).await;
            }
        }
        TikTokLiveEvent::Chat(msg) => print_user_line("chat", &msg.user, &format!(": {}", msg.comment)),
        TikTokLiveEvent::Gift(msg) => print_gift(&msg),
        TikTokLiveEvent::Like(msg) => print_user_line("like", &msg.user, &format!(" ({} total)", msg.total_like_count)),
        TikTokLiveEvent::Follow(msg) => print_user_line("follow", &msg.user, ""),
        TikTokLiveEvent::Share(msg) => print_user_line("share", &msg.user, ""),
        TikTokLiveEvent::Join(msg) => println!("[join] member_count={}", msg.member_count),
        TikTokLiveEvent::RoomUserSeq(msg) => println!("[viewers] {} watching, {} total", msg.viewer_count, msg.total_user),
        TikTokLiveEvent::LiveEnded(_) => {
            println!("[control] stream ended");
            return Flow::Stop;
        }
        TikTokLiveEvent::Envelope(msg) => {
            for info in msg.envelope_info.iter() {
                println!("[envelope] from={} diamonds={}", info.send_user_name, info.diamond_count);
            }
        }
        TikTokLiveEvent::Disconnected => {
            println!("[disconnected]");
            return Flow::Stop;
        }
        _ => {}
    }
    Flow::Continue
}

async fn print_room_info(room_id: &str, args: &Args) {
    let params = FetchParams {
        cookies: args.cookies.as_deref(),
        ..Default::default()
    };
    match fetch_room_info(room_id, params).await {
        Ok(info) => {
            println!("[room] title=\"{}\" viewers={} likes={}", info.title, info.viewers, info.likes);
            for urls in info.stream_url.iter() {
                let Some(url) = [&urls.flv_sd, &urls.flv_ld, &urls.flv_origin].into_iter().find_map(|url| url.as_deref()) else {
                    continue;
                };
                println!("[stream] {url}");
            }
        }
        Err(e) => tracing::warn!(error = %e, "room info fetch failed"),
    }
}

fn print_user_line(tag: &str, user: &Option<UserIdentity>, rest: &str) {
    let Some(user) = user else {
        tracing::warn!(tag, "event without user");
        return;
    };
    println!("[{tag}] {}{rest}", user.nickname);
}

fn print_gift(msg: &WebcastGiftMessage) {
    let diamonds = msg.diamond_total();
    let Some(details) = &msg.gift_details else {
        tracing::warn!(gift_id = msg.gift_id, "gift without details");
        return;
    };
    let gift_name = &details.gift_name;
    let detail = match (msg.is_combo_gift(), msg.is_streak_over()) {
        (true, true) => format!(" sent {gift_name} x{} (streak ended, {diamonds} diamonds)", msg.repeat_count),
        (true, false) => return,
        (false, _) => format!(" sent {gift_name} ({diamonds} diamonds)"),
    };
    print_user_line("gift", &msg.user, &detail);
}

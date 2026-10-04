#![deny(unused_must_use)]
#![deny(for_loops_over_fallibles)]
#![deny(dead_code)]
#![deny(unused_variables)]
#![deny(unused_assignments)]

pub mod decode;
pub mod errors;
pub mod helpers;
pub mod http;
pub mod reconnect;
pub mod structs;
pub mod websocket;

mod client;

pub use client::{TikTokLive, TikTokLiveBuilder, TikTokLiveStream};

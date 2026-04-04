use std::collections::BTreeMap;

use super::linker::{BattleUserArmies, BattleUserInfo};

// -- response container (used for both HTTP fetch and websocket frame payload) --

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastResponse {
    #[prost(message, repeated, tag = "1")]
    pub messages: Vec<WebcastMessage>,
    #[prost(string, tag = "2")]
    pub cursor: String,
    #[prost(int64, tag = "3")]
    pub fetch_interval: i64,
    #[prost(int64, tag = "4")]
    pub now: i64,
    #[prost(string, tag = "5")]
    pub internal_ext: String,
    #[prost(int32, tag = "6")]
    pub fetch_type: i32,
    #[prost(btree_map = "string, string", tag = "7")]
    pub route_params_map: BTreeMap<String, String>,
    #[prost(int32, tag = "8")]
    pub heart_beat_duration: i32,
    #[prost(bool, tag = "9")]
    pub needs_ack: bool,
    #[prost(string, tag = "10")]
    pub push_server: String,
    #[prost(bool, tag = "11")]
    pub is_first: bool,
    #[prost(string, tag = "12")]
    pub history_comment_cursor: String,
    #[prost(bool, tag = "13")]
    pub history_no_more: bool,
}

#[derive(Clone, PartialEq, Eq, Hash, ::prost::Message)]
pub struct WebcastMessage {
    #[prost(string, tag = "1")]
    pub r#type: String,
    #[prost(bytes = "vec", tag = "2")]
    pub payload: Vec<u8>,
    #[prost(int64, tag = "3")]
    pub msg_id: i64,
    #[prost(int32, tag = "4")]
    pub msg_type: i32,
    #[prost(int64, tag = "5")]
    pub offset: i64,
    #[prost(bool, tag = "6")]
    pub is_history: bool,
}

// -- common types --

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CommonMessageData {
    #[prost(string, tag = "1")]
    pub method: String,
    #[prost(int64, tag = "2")]
    pub msg_id: i64,
    #[prost(int64, tag = "3")]
    pub room_id: i64,
    #[prost(int64, tag = "4")]
    pub create_time: i64,
    #[prost(string, tag = "12")]
    pub log_id: String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Image {
    #[prost(string, repeated, tag = "1")]
    pub url_list: Vec<String>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FollowInfo {
    #[prost(int64, tag = "1")]
    pub following_count: i64,
    #[prost(int64, tag = "2")]
    pub follower_count: i64,
    #[prost(int64, tag = "3")]
    pub follow_status: i64,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PayGrade {
    #[prost(string, tag = "3")]
    pub name: String,
    #[prost(int64, tag = "6")]
    pub level: i64,
    #[prost(int64, tag = "25")]
    pub score: i64,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PrivilegeLogExtra {
    #[prost(string, tag = "1")]
    pub data_version: String,
    #[prost(string, tag = "2")]
    pub privilege_id: String,
    #[prost(string, tag = "5")]
    pub level: String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BadgeImage {
    #[prost(message, optional, tag = "2")]
    pub image: Option<Image>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BadgeText {
    #[prost(string, tag = "2")]
    pub key: String,
    #[prost(string, tag = "3")]
    pub default_pattern: String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BadgeString {
    #[prost(string, tag = "2")]
    pub content_str: String,
}

/// badge_scene: ADMIN=1, SUBSCRIBER=4, RANK_LIST=6, USER_GRADE=8, FANS=10
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BadgeStruct {
    #[prost(int32, tag = "1")]
    pub display_type: i32,
    #[prost(int32, tag = "3")]
    pub badge_scene: i32,
    #[prost(bool, tag = "11")]
    pub display: bool,
    #[prost(message, optional, tag = "12")]
    pub log_extra: Option<PrivilegeLogExtra>,
    #[prost(message, optional, tag = "20")]
    pub image_badge: Option<BadgeImage>,
    #[prost(message, optional, tag = "21")]
    pub text_badge: Option<BadgeText>,
    #[prost(message, optional, tag = "22")]
    pub string_badge: Option<BadgeString>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FansClubData {
    #[prost(string, tag = "1")]
    pub club_name: String,
    #[prost(int32, tag = "2")]
    pub level: i32,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FansClubMember {
    #[prost(message, optional, tag = "1")]
    pub data: Option<FansClubData>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UserAttr {
    #[prost(bool, tag = "1")]
    pub is_muted: bool,
    #[prost(bool, tag = "2")]
    pub is_admin: bool,
    #[prost(bool, tag = "3")]
    pub is_super_admin: bool,
    #[prost(int64, tag = "4")]
    pub mute_duration: i64,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AuthenticationInfo {
    #[prost(string, tag = "1")]
    pub custom_verify: String,
    #[prost(string, tag = "2")]
    pub enterprise_verify_reason: String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SubscribeInfo {
    #[prost(bool, tag = "2")]
    pub is_subscribe: bool,
    #[prost(int64, tag = "5")]
    pub subscriber_count: i64,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FansClubInfo {
    #[prost(int64, tag = "2")]
    pub fans_level: i64,
    #[prost(int64, tag = "3")]
    pub fans_score: i64,
    #[prost(int64, tag = "5")]
    pub fans_count: i64,
    #[prost(string, tag = "6")]
    pub fans_club_name: String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UserIdentity {
    #[prost(int64, tag = "1")]
    pub user_id: i64,
    #[prost(string, tag = "3")]
    pub nickname: String,
    #[prost(string, tag = "5")]
    pub bio_description: String,
    #[prost(message, optional, tag = "9")]
    pub avatar_thumb: Option<Image>,
    #[prost(message, optional, tag = "10")]
    pub avatar_medium: Option<Image>,
    #[prost(message, optional, tag = "11")]
    pub avatar_large: Option<Image>,
    #[prost(bool, tag = "12")]
    pub verified: bool,
    #[prost(int64, tag = "16")]
    pub create_time: i64,
    #[prost(int64, tag = "17")]
    pub modify_time: i64,
    #[prost(message, optional, tag = "22")]
    pub follow_info: Option<FollowInfo>,
    #[prost(message, optional, tag = "23")]
    pub pay_grade: Option<PayGrade>,
    #[prost(message, optional, tag = "24")]
    pub fans_club: Option<FansClubMember>,
    #[prost(int32, tag = "31")]
    pub top_vip_no: i32,
    #[prost(message, optional, tag = "32")]
    pub user_attr: Option<UserAttr>,
    #[prost(int64, tag = "34")]
    pub pay_score: i64,
    #[prost(int64, tag = "35")]
    pub fan_ticket_count: i64,
    #[prost(string, tag = "38")]
    pub unique_id: String,
    #[prost(bool, tag = "39")]
    pub with_commerce: bool,
    #[prost(string, tag = "46")]
    pub sec_uid: String,
    #[prost(message, optional, tag = "53")]
    pub authentication_info: Option<AuthenticationInfo>,
    #[prost(message, optional, tag = "63")]
    pub subscribe_info: Option<SubscribeInfo>,
    #[prost(message, repeated, tag = "64")]
    pub badge_list: Vec<BadgeStruct>,
    #[prost(message, optional, tag = "66")]
    pub fans_club_info: Option<FansClubInfo>,
    #[prost(bool, tag = "1002")]
    pub allow_find_by_contacts: bool,
    #[prost(string, tag = "1018")]
    pub constellation: String,
    #[prost(int64, tag = "1024")]
    pub follow_status: i64,
    #[prost(bool, tag = "1029")]
    pub is_follower: bool,
    #[prost(bool, tag = "1030")]
    pub is_following: bool,
    #[prost(string, tag = "1043")]
    pub verified_reason: String,
    #[prost(bool, tag = "1090")]
    pub is_subscribe: bool,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UserIdentityContext {
    #[prost(bool, tag = "1")]
    pub is_gift_giver_of_anchor: bool,
    #[prost(bool, tag = "2")]
    pub is_subscriber_of_anchor: bool,
    #[prost(bool, tag = "3")]
    pub is_mutual_following_with_anchor: bool,
    #[prost(bool, tag = "4")]
    pub is_follower_of_anchor: bool,
    #[prost(bool, tag = "5")]
    pub is_moderator_of_anchor: bool,
    #[prost(bool, tag = "6")]
    pub is_anchor: bool,
}

// -- event messages --

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastChatMessage {
    #[prost(message, optional, tag = "1")]
    pub common: Option<CommonMessageData>,
    #[prost(message, optional, tag = "2")]
    pub user: Option<UserIdentity>,
    #[prost(string, tag = "3")]
    pub comment: String,
    #[prost(string, tag = "14")]
    pub content_language: String,
    #[prost(int32, tag = "16")]
    pub quick_chat_scene: i32,
    #[prost(int32, tag = "17")]
    pub communityflagged_status: i32,
    #[prost(message, optional, tag = "18")]
    pub user_identity: Option<UserIdentityContext>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastLikeMessage {
    #[prost(message, optional, tag = "1")]
    pub common: Option<CommonMessageData>,
    #[prost(int32, tag = "2")]
    pub like_count: i32,
    #[prost(int32, tag = "3")]
    pub total_like_count: i32,
    #[prost(message, optional, tag = "5")]
    pub user: Option<UserIdentity>,
    #[prost(int64, tag = "9")]
    pub effect_cnt: i64,
    #[prost(int64, tag = "12")]
    pub room_message_heat_level: i64,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GiftDetails {
    #[prost(int64, tag = "5")]
    pub id: i64,
    #[prost(bool, tag = "10")]
    pub combo: bool,
    #[prost(int32, tag = "11")]
    pub gift_type: i32,
    #[prost(int32, tag = "12")]
    pub diamond_count: i32,
    #[prost(string, tag = "16")]
    pub gift_name: String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastGiftMessage {
    #[prost(message, optional, tag = "1")]
    pub common: Option<CommonMessageData>,
    #[prost(int32, tag = "2")]
    pub gift_id: i32,
    #[prost(int64, tag = "3")]
    pub fan_ticket_count: i64,
    #[prost(int32, tag = "4")]
    pub group_count: i32,
    #[prost(int32, tag = "5")]
    pub repeat_count: i32,
    #[prost(int32, tag = "6")]
    pub combo_count: i32,
    #[prost(message, optional, tag = "7")]
    pub user: Option<UserIdentity>,
    #[prost(message, optional, tag = "8")]
    pub to_user: Option<UserIdentity>,
    #[prost(int32, tag = "9")]
    pub repeat_end: i32,
    #[prost(uint64, tag = "11")]
    pub group_id: u64,
    #[prost(message, optional, tag = "15")]
    pub gift_details: Option<GiftDetails>,
    #[prost(bool, tag = "25")]
    pub is_first_sent: bool,
    #[prost(message, optional, tag = "32")]
    pub user_identity: Option<UserIdentityContext>,
    #[prost(bool, tag = "44")]
    pub multi_generate_message: bool,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastMemberMessage {
    #[prost(bytes = "vec", tag = "1")]
    pub common_raw: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub user_blob: Vec<u8>,
    #[prost(int32, tag = "3")]
    pub member_count: i32,
    #[prost(int32, tag = "10")]
    pub action: i32,
    #[prost(string, tag = "20")]
    pub effect_display_type: String,
    #[prost(string, tag = "21")]
    pub effect_action: String,
    #[prost(int64, tag = "28")]
    pub toast_visible: i64,
    #[prost(int64, tag = "33")]
    pub show_wave: i64,
    #[prost(int64, tag = "35")]
    pub enter_effect_target: i64,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Contributor {
    #[prost(int32, tag = "1")]
    pub coin_count: i32,
    #[prost(message, optional, tag = "2")]
    pub user: Option<UserIdentity>,
    #[prost(int32, tag = "3")]
    pub rank: i32,
    #[prost(int64, tag = "4")]
    pub delta: i64,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastRoomUserSeqMessage {
    #[prost(message, optional, tag = "1")]
    pub common: Option<CommonMessageData>,
    #[prost(message, repeated, tag = "2")]
    pub ranks_list: Vec<Contributor>,
    #[prost(int32, tag = "3")]
    pub viewer_count: i32,
    #[prost(string, tag = "4")]
    pub pop_str: String,
    #[prost(message, repeated, tag = "5")]
    pub seats_list: Vec<Contributor>,
    #[prost(int64, tag = "6")]
    pub popularity: i64,
    #[prost(int32, tag = "7")]
    pub total_user: i32,
    #[prost(int64, tag = "8")]
    pub anonymous: i64,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastSocialMessage {
    #[prost(message, optional, tag = "1")]
    pub common: Option<CommonMessageData>,
    #[prost(message, optional, tag = "2")]
    pub user: Option<UserIdentity>,
    #[prost(int64, tag = "3")]
    pub share_type: i64,
    #[prost(int64, tag = "4")]
    pub action: i64,
    #[prost(string, tag = "5")]
    pub share_target: String,
    #[prost(int32, tag = "6")]
    pub follow_count: i32,
    #[prost(int64, tag = "7")]
    pub share_display_style: i64,
    #[prost(int32, tag = "8")]
    pub share_count: i32,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastLiveIntroMessage {
    #[prost(message, optional, tag = "1")]
    pub common: Option<CommonMessageData>,
    #[prost(int64, tag = "2")]
    pub room_id: i64,
    #[prost(int32, tag = "3")]
    pub audit_status: i32,
    #[prost(string, tag = "4")]
    pub content: String,
    #[prost(message, optional, tag = "5")]
    pub host: Option<UserIdentity>,
    #[prost(int32, tag = "6")]
    pub intro_mode: i32,
    #[prost(string, tag = "8")]
    pub language: String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastRoomMessage {
    #[prost(message, optional, tag = "1")]
    pub common: Option<CommonMessageData>,
    #[prost(string, tag = "2")]
    pub content: String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CaptionData {
    #[prost(string, tag = "1")]
    pub language: String,
    #[prost(string, tag = "2")]
    pub text: String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastCaptionMessage {
    #[prost(message, optional, tag = "1")]
    pub common: Option<CommonMessageData>,
    #[prost(uint64, tag = "2")]
    pub time_stamp: u64,
    #[prost(message, optional, tag = "4")]
    pub caption_data: Option<CaptionData>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastControlExtra {
    #[prost(int64, tag = "2")]
    pub reason_no: i64,
    #[prost(string, tag = "8")]
    pub source: String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastControlMessage {
    #[prost(message, optional, tag = "1")]
    pub common: Option<CommonMessageData>,
    #[prost(int32, tag = "2")]
    pub action: i32,
    #[prost(string, tag = "3")]
    pub tips: String,
    #[prost(message, optional, tag = "4")]
    pub extra: Option<WebcastControlExtra>,
    #[prost(int32, tag = "9")]
    pub float_style: i32,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastGoalUpdateMessage {
    #[prost(message, optional, tag = "1")]
    pub common: Option<CommonMessageData>,
    #[prost(int64, tag = "4")]
    pub contributor_id: i64,
    #[prost(string, tag = "6")]
    pub contributor_display_id: String,
    #[prost(int64, tag = "9")]
    pub contribute_count: i64,
    #[prost(int64, tag = "10")]
    pub contribute_score: i64,
    #[prost(int64, tag = "11")]
    pub gift_repeat_count: i64,
    #[prost(string, tag = "12")]
    pub contributor_id_str: String,
    #[prost(bool, tag = "13")]
    pub pin: bool,
    #[prost(bool, tag = "14")]
    pub unpin: bool,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastImDeleteMessage {
    #[prost(message, optional, tag = "1")]
    pub common: Option<CommonMessageData>,
    #[prost(int64, repeated, tag = "2")]
    pub delete_msg_ids_list: Vec<i64>,
    #[prost(int64, repeated, tag = "3")]
    pub delete_user_ids_list: Vec<i64>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastRankUpdate {
    #[prost(int64, tag = "1")]
    pub rank_type: i64,
    #[prost(int64, tag = "2")]
    pub owner_rank: i64,
    #[prost(bool, tag = "5")]
    pub show_entrance_animation: bool,
    #[prost(int64, tag = "6")]
    pub countdown: i64,
    #[prost(int64, tag = "8")]
    pub related_tab_rank_type: i64,
    #[prost(int64, tag = "9")]
    pub request_first_show_type: i64,
    #[prost(int64, tag = "10")]
    pub supported_version: i64,
    #[prost(bool, tag = "11")]
    pub owner_on_rank: bool,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastRankTabInfo {
    #[prost(int64, tag = "1")]
    pub rank_type: i64,
    #[prost(string, tag = "2")]
    pub title: String,
    #[prost(int64, tag = "4")]
    pub list_lynx_type: i64,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastRankUpdateMessage {
    #[prost(message, optional, tag = "1")]
    pub common: Option<CommonMessageData>,
    #[prost(message, repeated, tag = "2")]
    pub updates_list: Vec<WebcastRankUpdate>,
    #[prost(int64, tag = "3")]
    pub group_type: i64,
    #[prost(int64, tag = "5")]
    pub priority: i64,
    #[prost(message, repeated, tag = "6")]
    pub tabs_list: Vec<WebcastRankTabInfo>,
    #[prost(bool, tag = "7")]
    pub is_animation_loop_play: bool,
    #[prost(bool, tag = "8")]
    pub animation_loop_for_off: bool,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastPollMessage {
    #[prost(message, optional, tag = "1")]
    pub common: Option<CommonMessageData>,
    #[prost(int32, tag = "2")]
    pub message_type: i32,
    #[prost(int64, tag = "3")]
    pub poll_id: i64,
    #[prost(bytes = "vec", tag = "4")]
    pub start_content_blob: Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    pub end_content_blob: Vec<u8>,
    #[prost(bytes = "vec", tag = "6")]
    pub update_content_blob: Vec<u8>,
    #[prost(int32, tag = "7")]
    pub poll_kind: i32,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastEnvelopeMessage {
    #[prost(message, optional, tag = "1")]
    pub common: Option<CommonMessageData>,
    #[prost(message, optional, tag = "2")]
    pub envelope_info: Option<EnvelopeInfo>,
    #[prost(int32, tag = "3")]
    pub display: i32,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EnvelopeInfo {
    #[prost(string, tag = "1")]
    pub envelope_id: String,
    #[prost(int32, tag = "2")]
    pub business_type: i32,
    #[prost(string, tag = "3")]
    pub envelope_idc: String,
    #[prost(string, tag = "4")]
    pub send_user_name: String,
    #[prost(int32, tag = "5")]
    pub diamond_count: i32,
    #[prost(int32, tag = "6")]
    pub people_count: i32,
    #[prost(int32, tag = "7")]
    pub unpack_at: i32,
    #[prost(string, tag = "8")]
    pub send_user_id: String,
    #[prost(bytes = "vec", tag = "9")]
    pub send_user_avatar_raw: Vec<u8>,
    #[prost(string, tag = "10")]
    pub create_at: String,
    #[prost(string, tag = "11")]
    pub room_id: String,
    #[prost(int32, tag = "12")]
    pub follow_show_status: i32,
    #[prost(int32, tag = "13")]
    pub skin_id: i32,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastRoomPinMessage {
    #[prost(message, optional, tag = "1")]
    pub common: Option<CommonMessageData>,
    #[prost(bytes = "vec", tag = "2")]
    pub pinned_message: Vec<u8>,
    #[prost(string, tag = "30")]
    pub original_msg_type: String,
    #[prost(uint64, tag = "31")]
    pub timestamp: u64,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastUnauthorizedMemberMessage {
    #[prost(message, optional, tag = "1")]
    pub common: Option<CommonMessageData>,
    #[prost(int32, tag = "2")]
    pub action: i32,
    #[prost(bytes = "vec", tag = "3")]
    pub nick_name_prefix_blob: Vec<u8>,
    #[prost(string, tag = "4")]
    pub nick_name: String,
    #[prost(bytes = "vec", tag = "5")]
    pub enter_text_blob: Vec<u8>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastLinkMicMethod {
    #[prost(bytes = "vec", tag = "1")]
    pub common_raw: Vec<u8>,
    #[prost(int32, tag = "2")]
    pub message_type: i32,
    #[prost(int64, tag = "5")]
    pub user_id: i64,
    #[prost(int64, tag = "8")]
    pub channel_id: i64,
    #[prost(int64, tag = "21")]
    pub to_user_id: i64,
    #[prost(int64, tag = "26")]
    pub start_time_ms: i64,
    #[prost(string, tag = "37")]
    pub anchor_link_mic_id_str: String,
    #[prost(int64, tag = "38")]
    pub rival_anchor_id: i64,
    #[prost(string, tag = "40")]
    pub rival_linkmic_id_str: String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastLinkMicBattle {
    #[prost(bytes = "vec", tag = "1")]
    pub common_raw: Vec<u8>,
    #[prost(int64, tag = "2")]
    pub battle_id: i64,
    #[prost(int32, tag = "4")]
    pub action: i32,
    #[prost(btree_map = "int64, message", tag = "5")]
    pub battle_result: BTreeMap<i64, BattleUserArmies>,
    #[prost(btree_map = "int64, message", tag = "9")]
    pub armies: BTreeMap<i64, BattleUserArmies>,
    #[prost(btree_map = "int64, message", tag = "10")]
    pub anchor_info: BTreeMap<i64, BattleUserInfo>,
    #[prost(message, repeated, tag = "14")]
    pub team_users: Vec<BattleUserArmies>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastLinkMicArmies {
    #[prost(bytes = "vec", tag = "1")]
    pub common_raw: Vec<u8>,
    #[prost(int64, tag = "2")]
    pub battle_id: i64,
    #[prost(btree_map = "int64, message", tag = "3")]
    pub battle_items: BTreeMap<i64, BattleUserArmies>,
    #[prost(int64, tag = "4")]
    pub channel_id: i64,
    #[prost(int32, tag = "7")]
    pub battle_status: i32,
    #[prost(int64, tag = "8")]
    pub from_user_id: i64,
    #[prost(int64, tag = "9")]
    pub gift_id: i64,
    #[prost(int32, tag = "10")]
    pub gift_count: i32,
    #[prost(int32, tag = "12")]
    pub total_diamond_count: i32,
    #[prost(int32, tag = "13")]
    pub repeat_count: i32,
    #[prost(message, repeated, tag = "14")]
    pub team_armies: Vec<BattleUserArmies>,
    #[prost(bool, tag = "15")]
    pub trigger_critical_strike: bool,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastLinkMessage {
    #[prost(bytes = "vec", tag = "1")]
    pub common_raw: Vec<u8>,
    #[prost(int32, tag = "2")]
    pub message_type: i32,
    #[prost(int64, tag = "3")]
    pub linker_id: i64,
    #[prost(int32, tag = "4")]
    pub scene: i32,
    #[prost(bytes = "vec", tag = "20")]
    pub list_change_content_blob: Vec<u8>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastLinkLayerMessage {
    #[prost(bytes = "vec", tag = "1")]
    pub common_raw: Vec<u8>,
    #[prost(int32, tag = "2")]
    pub message_type: i32,
    #[prost(int64, tag = "3")]
    pub channel_id: i64,
    #[prost(int32, tag = "4")]
    pub scene: i32,
    #[prost(string, tag = "5")]
    pub source: String,
    #[prost(string, tag = "6")]
    pub centerized_idc: String,
    #[prost(int64, tag = "7")]
    pub rtc_room_id: i64,
    #[prost(bytes = "vec", tag = "118")]
    pub group_change_blob: Vec<u8>,
    #[prost(bytes = "vec", tag = "200")]
    pub business_blob: Vec<u8>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastLinkMicLayoutStateMessage {
    #[prost(bytes = "vec", tag = "1")]
    pub common_raw: Vec<u8>,
    #[prost(int64, tag = "2")]
    pub room_id: i64,
    #[prost(int32, tag = "3")]
    pub layout_state: i32,
    #[prost(string, tag = "6")]
    pub layout_key: String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastGiftPanelUpdateMessage {
    #[prost(bytes = "vec", tag = "1")]
    pub common_raw: Vec<u8>,
    #[prost(int64, tag = "2")]
    pub room_id: i64,
    #[prost(int64, tag = "3")]
    pub panel_ts_or_version: i64,
    #[prost(bytes = "vec", tag = "10")]
    pub panel_blob: Vec<u8>,
    #[prost(bytes = "vec", tag = "11")]
    pub gift_list_blob: Vec<u8>,
    #[prost(bytes = "vec", tag = "12")]
    pub vault_blob: Vec<u8>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastInRoomBannerMessage {
    #[prost(bytes = "vec", tag = "1")]
    pub common_raw: Vec<u8>,
    #[prost(bytes = "vec", repeated, tag = "2")]
    pub raw_data_entries: Vec<Vec<u8>>,
    #[prost(int32, tag = "3")]
    pub position: i32,
    #[prost(int32, tag = "4")]
    pub action_type: i32,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WebcastGuideMessage {
    #[prost(bytes = "vec", tag = "1")]
    pub common_raw: Vec<u8>,
    #[prost(int32, tag = "2")]
    pub guide_type: i32,
    #[prost(int64, tag = "5")]
    pub duration_ms: i64,
    #[prost(string, tag = "7")]
    pub scene: String,
}

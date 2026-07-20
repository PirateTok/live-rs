use crate::structs::proto::messages::{
    Contributor, WebcastCaptionMessage, WebcastChatMessage, WebcastControlMessage,
    WebcastEnvelopeMessage, WebcastGiftMessage, WebcastGiftPanelUpdateMessage,
    WebcastGoalUpdateMessage, WebcastGuideMessage, WebcastImDeleteMessage,
    WebcastInRoomBannerMessage, WebcastLikeMessage, WebcastLinkLayerMessage, WebcastLinkMessage,
    WebcastLinkMicArmies, WebcastLinkMicBattle, WebcastLinkMicLayoutStateMessage,
    WebcastLinkMicMethod, WebcastLiveIntroMessage, WebcastMemberMessage, WebcastPollMessage,
    WebcastRankUpdateMessage, WebcastRoomMessage, WebcastRoomPinMessage,
    WebcastRoomUserSeqMessage, WebcastSocialMessage, WebcastUnauthorizedMemberMessage,
};
use crate::structs::proto::messages_ext::{
    WebcastAccessControlMessage, WebcastAccessRecallMessage, WebcastAlertBoxAuditResultMessage,
    WebcastBarrageMessage, WebcastBindingGiftMessage, WebcastBoostCardMessage,
    WebcastBottomMessage, WebcastEmoteChatMessage, WebcastGameRankNotifyMessage,
    WebcastGiftBroadcastMessage, WebcastGiftDynamicRestrictionMessage, WebcastGiftPromptMessage,
    WebcastHourlyRankMessage, WebcastLinkMicBattlePunishFinish, WebcastLinkMicFanTicketMethod,
    WebcastLinkStateMessage, WebcastLinkmicBattleTaskMessage, WebcastLiveGameIntroMessage,
    WebcastMarqueeAnnouncementMessage, WebcastMsgDetectMessage, WebcastNoticeMessage,
    WebcastNotifyMessage, WebcastOecLiveShoppingMessage, WebcastPartnershipDropsUpdateMessage,
    WebcastPartnershipGameOfflineMessage, WebcastPartnershipPunishMessage,
    WebcastPerceptionMessage, WebcastQuestionNewMessage, WebcastRankTextMessage,
    WebcastRoomVerifyMessage, WebcastSpeakerMessage, WebcastSubCapsuleMessage,
    WebcastSubNotifyMessage, WebcastSubPinEventMessage, WebcastSubscriptionNotifyMessage,
    WebcastSystemMessage, WebcastToastMessage, WebcastViewerPicksUpdateMessage,
};

/// Events received from a TikTok Live stream.
///
/// ## Gift streaks
///
/// Some gifts are "combo" gifts that fire multiple events during a streak.
/// Use [`WebcastGiftMessage::is_combo_gift`] and [`WebcastGiftMessage::is_streak_over`]
/// to handle them correctly. See the `gift_tracker` example for a complete pattern.
///
/// ## Convenience events
///
/// `Follow`, `Share`, `Join`, `LiveEnded` are sub-routed from raw events.
/// Both the raw event AND the convenience event fire for the same message.
#[derive(Clone, Debug)]
pub enum TikTokLiveEvent {
    // lifecycle
    Connected { room_id: String },
    Reconnecting { attempt: u32, max_retries: u32, delay_secs: u64 },
    Disconnected,

    // core events
    Chat(WebcastChatMessage),
    Gift(WebcastGiftMessage),
    Like(WebcastLikeMessage),
    Member(WebcastMemberMessage),
    Social(WebcastSocialMessage),
    RoomUserSeq(WebcastRoomUserSeqMessage),
    Control(WebcastControlMessage),

    // sub-routed convenience events (also fire raw event above)
    Follow(WebcastSocialMessage),
    Share(WebcastSocialMessage),
    Join(WebcastMemberMessage),
    LiveEnded(WebcastControlMessage),

    // useful events
    LiveIntro(WebcastLiveIntroMessage),
    RoomMessage(WebcastRoomMessage),
    Caption(WebcastCaptionMessage),
    GoalUpdate(WebcastGoalUpdateMessage),
    ImDelete(WebcastImDeleteMessage),

    // niche events
    RankUpdate(WebcastRankUpdateMessage),
    Poll(WebcastPollMessage),
    Envelope(WebcastEnvelopeMessage),
    RoomPin(WebcastRoomPinMessage),
    UnauthorizedMember(WebcastUnauthorizedMemberMessage),
    LinkMicMethod(WebcastLinkMicMethod),
    LinkMicBattle(WebcastLinkMicBattle),
    LinkMicArmies(WebcastLinkMicArmies),
    LinkMessage(WebcastLinkMessage),
    LinkLayer(WebcastLinkLayerMessage),
    LinkMicLayoutState(WebcastLinkMicLayoutStateMessage),
    GiftPanelUpdate(WebcastGiftPanelUpdateMessage),
    InRoomBanner(WebcastInRoomBannerMessage),
    Guide(WebcastGuideMessage),

    // extended events
    EmoteChat(WebcastEmoteChatMessage),
    QuestionNew(WebcastQuestionNewMessage),
    SubNotify(WebcastSubNotifyMessage),
    Barrage(WebcastBarrageMessage),
    HourlyRank(WebcastHourlyRankMessage),
    MsgDetect(WebcastMsgDetectMessage),
    LinkMicFanTicket(WebcastLinkMicFanTicketMethod),
    RoomVerify(WebcastRoomVerifyMessage),
    OecLiveShopping(WebcastOecLiveShoppingMessage),
    GiftBroadcast(WebcastGiftBroadcastMessage),
    RankText(WebcastRankTextMessage),
    GiftDynamicRestriction(WebcastGiftDynamicRestrictionMessage),
    ViewerPicksUpdate(WebcastViewerPicksUpdateMessage),

    // secondary events
    SystemMessage(WebcastSystemMessage),
    LiveGameIntro(WebcastLiveGameIntroMessage),
    AccessControl(WebcastAccessControlMessage),
    AccessRecall(WebcastAccessRecallMessage),
    AlertBoxAuditResult(WebcastAlertBoxAuditResultMessage),
    BindingGift(WebcastBindingGiftMessage),
    BoostCard(WebcastBoostCardMessage),
    BottomMessage(WebcastBottomMessage),
    GameRankNotify(WebcastGameRankNotifyMessage),
    GiftPrompt(WebcastGiftPromptMessage),
    LinkState(WebcastLinkStateMessage),
    LinkMicBattlePunishFinish(WebcastLinkMicBattlePunishFinish),
    LinkmicBattleTask(WebcastLinkmicBattleTaskMessage),
    MarqueeAnnouncement(WebcastMarqueeAnnouncementMessage),
    Notice(WebcastNoticeMessage),
    Notify(WebcastNotifyMessage),
    PartnershipDropsUpdate(WebcastPartnershipDropsUpdateMessage),
    PartnershipGameOffline(WebcastPartnershipGameOfflineMessage),
    PartnershipPunish(WebcastPartnershipPunishMessage),
    Perception(WebcastPerceptionMessage),
    Speaker(WebcastSpeakerMessage),
    SubCapsule(WebcastSubCapsuleMessage),
    SubPinEvent(WebcastSubPinEventMessage),
    SubscriptionNotify(WebcastSubscriptionNotifyMessage),
    Toast(WebcastToastMessage),

    // unknown passthrough
    Unknown { method: String, payload: Vec<u8> },
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct RoomInfo {
    pub title: String,
    pub viewers: i64,
    pub likes: i64,
    pub total_viewers: i64,
    pub stream_url: Option<StreamUrl>,
    pub raw_json: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct StreamUrl {
    pub flv_origin: Option<String>,
    pub flv_hd: Option<String>,
    pub flv_sd: Option<String>,
    pub flv_ld: Option<String>,
    pub flv_ao: Option<String>,
}

/// Full audience roster from `/webcast/ranklist/online_audience/`.
///
/// Returned by [`fetch_room_audience`](crate::http::api::fetch_room_audience).
/// `total` counts everyone in the room; `viewers` lists the non-anonymous ones
/// TikTok is willing to name (the same list the web viewer panel shows).
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct RoomAudience {
    pub total: i64,
    pub anonymous: i64,
    pub viewers: Vec<AudienceViewer>,
    pub raw_json: String,
}

/// One named viewer from the audience roster.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct AudienceViewer {
    pub rank: i64,
    /// Contribution score (coins) this session — the panel's sort key.
    pub score: i64,
    pub user_id: String,
    /// The @username.
    pub username: String,
    pub nickname: String,
    pub sec_uid: String,
    pub avatar_url: Option<String>,
    pub follower_count: i64,
    pub verified: bool,
    /// Follows the streamer.
    pub is_follower: bool,
    /// The streamer follows them.
    pub is_following: bool,
    pub is_subscriber: bool,
}

impl WebcastRoomUserSeqMessage {
    /// The top-viewers box next to the viewer counter (usually the top 3
    /// ranked by contribution score). Entries without a decoded user are
    /// skipped; the rest come back sorted by rank.
    pub fn top_viewers(&self) -> Vec<&Contributor> {
        let mut top: Vec<&Contributor> =
            self.ranks_list.iter().filter(|c| c.user.is_some()).collect();
        top.sort_by_key(|c| c.rank);
        top
    }
}

impl WebcastGiftMessage {
    pub fn is_combo_gift(&self) -> bool {
        match &self.gift_details {
            Some(g) => g.gift_type == 1,
            None => false,
        }
    }

    pub fn is_streak_over(&self) -> bool {
        if !self.is_combo_gift() {
            return true;
        }
        self.repeat_end == 1
    }

    pub fn diamond_total(&self) -> i64 {
        let per_gift = match &self.gift_details {
            Some(g) => g.diamond_count as i64,
            None => 0,
        };
        per_gift * (self.repeat_count as i64).max(1)
    }
}

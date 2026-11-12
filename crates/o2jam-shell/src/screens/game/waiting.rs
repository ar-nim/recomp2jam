//! Queued session-bridge sends behind the shell screens: the
//! wire-queue outbox the session seam drains (#139 cluster 1).

use o2jam_net::gift::{ClaimGift, GetGiftMessageList};
use o2jam_net::play::SetRoomMusic;
use o2jam_net::shop::{EquipItem, SyncItemPurchase, SyncMusicPurchase};
use o2jam_net::{CreateRoom, ExitPlaying, JoinRoom, Ready, SetRoomTeam, StartGame};

/// Queued session-bridge sends behind the shell screens (#139
/// cluster 1: the Data Clump of all thirteen `pending_*` wire
/// queues). Every queueing arm stages its pinned body here and the
/// session seam drains it once through a `take_*` accessor; the
/// `has_pending_*` arms peek without draining. Pure data — no
/// screen, paint, or input logic — so unsent traffic reads without
/// touching the `ScreenGame` God class. `ScreenGame` keeps thin
/// delegates under the same names, so the session bridge and the
/// snapshot surface never change.
#[derive(Default)]
pub(crate) struct PendingOutbox {
    /// Committed create-room body awaiting the session send (2004).
    pub(crate) pending_create_room: Option<CreateRoom>,
    /// Committed locked-room join awaiting the session send (3002).
    pub(crate) pending_join: Option<JoinRoom>,
    /// Queued team send awaiting the session (4004; the toggle owns it).
    pub(crate) pending_room_team: Option<SetRoomTeam>,
    /// Queued ready-clear awaiting the session (4008; the team switch owns it).
    pub(crate) pending_team_ready: bool,
    /// Queued song pick awaiting the session (4000; the dialog row owns it).
    pub(crate) pending_music_send: Option<SetRoomMusic>,
    /// Queued RETRY press awaiting the session (4010 `0xfaa`, empty;
    /// the result RETRY button and its Confirm arm own it).
    pub(crate) pending_result_retry: bool,
    /// Queued BACK press awaiting the session (4021 `0xfb5`, empty;
    /// the result BACK button and its Back arm own it).
    pub(crate) pending_result_back: bool,
    /// Queued battle result entry awaiting the session (4042 `0xfca`,
    /// empty; the song-end Result entry in team play owns it).
    pub(crate) pending_result_battle_entry: bool,
    /// Queued cart buy awaiting the session send (5000; the cart
    /// `BUYITEM` owns it).
    pub(crate) pending_shop_buy: Option<SyncItemPurchase>,
    /// Queued song buy awaiting the session send (5010; the row
    /// buy buttons and the cart `BUYITEM` own it).
    pub(crate) pending_music_buy: Option<SyncMusicPurchase>,
    /// Committed equip send awaiting the session (5004; the apply
    /// button owns it).
    pub(crate) pending_equip: Option<EquipItem>,
    /// Committed gift claim awaiting the session (5023; MOVE owns it).
    pub(crate) pending_gift_claim: Option<ClaimGift>,
    /// Gift-list fetch awaiting the session (5017; queued on entry).
    pub(crate) pending_gift_list: bool,
}

impl PendingOutbox {
    /// Queued team send for the session bridge, if any (4004). One
    /// take sends once; the next press replaces the queued team.
    pub(crate) fn take_room_team(&mut self) -> Option<SetRoomTeam> {
        self.pending_room_team.take()
    }
    /// Queued ready-clear for the session bridge, if any (4008).
    /// The team switch queues it directly behind its 4004.
    pub(crate) fn take_team_ready(&mut self) -> Option<Ready> {
        if self.pending_team_ready {
            self.pending_team_ready = false;
            Some(Ready)
        } else {
            None
        }
    }
    /// Whether a team-button pair is queued for the session bridge.
    #[must_use]
    pub(crate) fn has_pending_team_send(&self) -> bool {
        self.pending_room_team.is_some() || self.pending_team_ready
    }
    /// Queued song pick for the session bridge, if any (4000). One
    /// take sends once; the next row press replaces the queued pick.
    pub(crate) fn take_music_send(&mut self) -> Option<SetRoomMusic> {
        self.pending_music_send.take()
    }
    /// Whether a dialog row pick is queued for the session bridge.
    #[must_use]
    pub(crate) fn has_pending_music_send(&self) -> bool {
        self.pending_music_send.is_some()
    }
    /// Queued RETRY press for the session bridge, if any (`StartGame`
    /// 4010, `0xfaa` — the exe's result RETRY relay event,
    /// `FUN_004c2f00` range, `ev_77ce98f1…`). One take sends once;
    /// the next press replaces the queue.
    pub(crate) fn take_result_retry(&mut self) -> Option<StartGame> {
        if self.pending_result_retry {
            self.pending_result_retry = false;
            Some(StartGame)
        } else {
            None
        }
    }
    /// Queued BACK press for the session bridge, if any (`ExitPlaying`
    /// 4021, `0xfb5` — the exe's result BACK relay event,
    /// `docs/cresult-anchors.md` `ev_77ce98f1…`). One take sends once;
    /// the next press replaces the queue.
    pub(crate) fn take_result_back(&mut self) -> Option<ExitPlaying> {
        if self.pending_result_back {
            self.pending_result_back = false;
            Some(ExitPlaying)
        } else {
            None
        }
    }
    /// Queued battle result entry for the session bridge, if any
    /// (`GetLiveState` 4042, `0xfca` — byte-verified empty,
    /// `docs/relay.md` §10, gated on the battle global `0x7789e6`).
    /// One take sends once; the next battle entry replaces the queue.
    pub(crate) fn take_result_battle_entry(&mut self) -> Option<o2jam_net::relay::GetLiveState> {
        if self.pending_result_battle_entry {
            self.pending_result_battle_entry = false;
            Some(o2jam_net::relay::GetLiveState)
        } else {
            None
        }
    }
    /// Committed create-room body awaiting the session send (2004).
    pub(crate) fn take_create_room(&mut self) -> Option<CreateRoom> {
        self.pending_create_room.take()
    }
    /// Committed locked-room join awaiting the session send (3002).
    pub(crate) fn take_join(&mut self) -> Option<JoinRoom> {
        self.pending_join.take()
    }
    /// Committed equip send for the session bridge, if any (5004).
    /// One take sends once; the next apply replaces the queued send.
    pub(crate) fn take_equip_send(&mut self) -> Option<EquipItem> {
        self.pending_equip.take()
    }
    /// Whether an equip send is queued for the session bridge.
    #[must_use]
    pub(crate) fn has_pending_equip_send(&self) -> bool {
        self.pending_equip.is_some()
    }
    /// Committed gift claim for the session bridge, if any (5023).
    /// One take sends once; the next MOVE replaces the queued claim.
    pub(crate) fn take_gift_claim(&mut self) -> Option<ClaimGift> {
        self.pending_gift_claim.take()
    }
    /// Gift-list fetch for the session bridge, if any (5017; queued
    /// on avatar-room entry). One take sends once.
    pub(crate) fn take_gift_list_request(&mut self) -> Option<GetGiftMessageList> {
        if self.pending_gift_list {
            self.pending_gift_list = false;
            Some(GetGiftMessageList)
        } else {
            None
        }
    }
    /// Queued cart buy for the session bridge, if any (5000). One
    /// body per `BUYITEM` commit; the seam drains it.
    pub(crate) fn take_shop_buy(&mut self) -> Option<SyncItemPurchase> {
        self.pending_shop_buy.take()
    }
    /// Queued song buy for the session bridge, if any (5010). One
    /// body per row or cart commit; the seam drains it.
    pub(crate) fn take_music_buy(&mut self) -> Option<SyncMusicPurchase> {
        self.pending_music_buy.take()
    }
}

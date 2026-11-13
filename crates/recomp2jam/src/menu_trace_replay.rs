//! Deterministic shop/MyRoom menu-trace replay (ticket #208).
//!
//! The portable input seam is [`ScreenGame::ingest_snapshot`](o2jam_shell::GameLoop::ingest_snapshot):
//! every trace event becomes one [`InputSnapshot`](o2jam_shell::InputSnapshot) at that seam,
//! never an OS key/mouse injection — `SendInput`/XTEST stay reserved for the integration-layer
//! probes of the window boundary. The live caller chain this guards is
//! `window::WindowHandler` (frame freeze) → `session::Session::ingest_snapshot` →
//! `ScreenGame::ingest_snapshot` → `apply_pointer_down`/`apply_pointer_up`.
//!
//! Verdict rules (the skill's accepted/rejected/applied log):
//!
//! - `Applied`: the event fed one snapshot through the seam.
//! - `Accepted`: the event needed no seam feed — an `await-screen` sync point the game already
//!   satisfies (the screen name *or* the open side screen matches, since shop/MyRoom ride side
//!   overlays while `screen_name` names the lobby underneath).
//! - `Rejected`: the harness refused the event with a reason and fed nothing — `text` (needs
//!   window IME injection, integration layer only), unknown controls, clicks without coordinates
//!   or outside the 800x600 canvas. Rejections log, never silently drop.
//!
//! An `await-screen` the game does not satisfy fails the run ([`ReplayError::SyncMissed`]): a
//! missed sync point means the scenario diverged, not that one event was skippable.
//!
//! Timestamps ride the trace's microsecond clock for ordering only; the seam applies events in
//! sequence order with no sleeps, so replays are instant and deterministic. Staging is fully
//! synthetic (no `install-root`, no server): the catalog stages through
//! [`ScreenGame::set_avatar_source`], the #191 live-chain entry
//! (`main.rs::run_play` → `setup_online` → `stage_online_identity` → `set_avatar_source` →
//! `ShopState::set_catalog`), so the traces drive the same staging seam the live path uses.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use o2jam_file::{AvatarItemRow, AvatarItemTable};
use o2jam_net::{
    ChannelState, CharacterInfo, Gender, GiftItemInfo, RoomListEntry, EQUIPMENT_SLOTS,
};
use o2jam_render::Rect;
use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
use o2jam_shell::screens::side::{
    MYROOM_APPLY_CODE, MYROOM_BAG_FIRST_CODE, MYROOM_BAG_LEFT_CODE, MYROOM_BAG_RIGHT_CODE,
    MYROOM_GIFT_FIRST_CODE, MYROOM_MOVE_CODE, MYROOM_SLOT_FIRST_CODE, SHOP_BAG_LEFT_CODE,
    SHOP_BAG_RIGHT_CODE, SHOP_BUYITEM_CODE, SHOP_ITEMBUY_FIRST_CODE, SHOP_SCROLL_LEFT_CODE,
    SHOP_SCROLL_RIGHT_CODE,
};
use o2jam_shell::screens::SideScreen;
use o2jam_shell::{GameLoop, InputEdge, InputSnapshot, MenuKey, Screen, ScreenGame};

use crate::trace::{load_validated, InputTrace, TraceEvent};

/// Room chrome button rect (clear of the room row bands, the shared
/// test-geometry convention): the harness stages one room control per
/// scenario here and the entry traces click its center.
pub const ROOM_BUTTON_RECT: Rect = Rect {
    x: 400,
    y: 380,
    w: 40,
    h: 20,
};
/// Click point inside [`ROOM_BUTTON_RECT`] (test geometry: the
/// entry traces click here).
#[cfg(test)]
const ROOM_BUTTON_CLICK: (i32, i32) = (410, 385);

/// Room→item-shop chrome code (`IDC_BUTTON_SROOM_ITEMSHOP`,
/// `ROOM_CONTROLS`): the entry edge the shop traces drive.
pub const ROOM_ITEMSHOP_CODE: u32 = 0x1320_0008;
/// Room→avatar-room chrome code (`IDC_BUTTON_SROOM_MYROOM`,
/// `ROOM_CONTROLS`): the entry edge the MyRoom/gift traces drive.
pub const ROOM_MYROOM_CODE: u32 = 0x1320_0009;

/// Whether the harness fed the event through the snapshot seam,
/// accepted it without a feed, or refused it with a reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventVerdict {
    /// Fed through `ingest_snapshot`.
    Applied,
    /// Needed no feed (a satisfied `await-screen` sync point).
    Accepted,
    /// Refused with a logged reason; fed nothing.
    Rejected,
}

/// One trace event's replay outcome.
#[derive(Debug, Clone)]
pub struct LoggedEvent {
    /// Trace sequence number.
    pub sequence: u64,
    /// Portable control name from the trace.
    pub control: String,
    /// Action from the trace.
    pub action: String,
    /// What the harness did with the event.
    pub verdict: EventVerdict,
    /// Why a rejected event was refused (`None` otherwise).
    pub reason: Option<String>,
    /// Screen after the event.
    pub screen: String,
    /// Open side screen after the event, if any.
    pub side: Option<String>,
    /// State hash after the event.
    pub hash: u64,
}

/// One full trace run: every event's verdict plus the end state.
#[derive(Debug, Clone)]
pub struct ReplayReport {
    /// Trace scenario name.
    pub scenario: String,
    /// Per-event outcomes in sequence order.
    pub events: Vec<LoggedEvent>,
    /// Side screen open at the end, if any.
    pub final_side: Option<String>,
    /// State hash at the end.
    pub final_hash: u64,
}

impl ReplayReport {
    /// Events fed through the snapshot seam.
    #[must_use]
    pub fn applied(&self) -> usize {
        self.events
            .iter()
            .filter(|event| event.verdict == EventVerdict::Applied)
            .count()
    }

    /// Events the harness refused.
    #[must_use]
    pub fn rejected(&self) -> usize {
        self.events
            .iter()
            .filter(|event| event.verdict == EventVerdict::Rejected)
            .count()
    }

    /// Verdict per event, in sequence order (the determinism key).
    #[must_use]
    pub fn verdicts(&self) -> Vec<EventVerdict> {
        self.events.iter().map(|event| event.verdict).collect()
    }
}

/// Why a replay run failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    /// The trace itself is unusable (empty, or a hand-built event the
    /// validator would reject).
    Trace(String),
    /// An `await-screen` sync point the game never reached: expected
    /// screen/side, actual screen/side.
    SyncMissed {
        sequence: u64,
        expected: String,
        screen: String,
        side: Option<String>,
    },
    /// The two repeat runs diverged.
    Nondeterministic(String),
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Trace(reason) => write!(f, "unusable trace: {reason}"),
            Self::SyncMissed {
                sequence,
                expected,
                screen,
                side,
            } => write!(
                f,
                "event {sequence}: awaited {expected:?}, at screen {screen:?} side {side:?}"
            ),
            Self::Nondeterministic(reason) => write!(f, "repeat runs diverged: {reason}"),
        }
    }
}

impl std::error::Error for ReplayError {}

/// Load one committed menu trace by file name (synthetic fixtures,
/// no game data needed).
///
/// # Errors
///
/// Missing file or a trace the schema validator rejects.
pub fn load_menu_trace(name: &str) -> anyhow::Result<InputTrace> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/input-trace")
        .join(name);
    load_validated(&path)
}

/// Replay one trace through a staged game, logging every event.
///
/// # Errors
///
/// [`ReplayError::Trace`] for an empty trace or a defense-in-depth
/// refusal the validator should have caught structurally (still
/// logged, never dropped); [`ReplayError::SyncMissed`] for an
/// unreached `await-screen`.
pub fn replay_once(game: &mut ScreenGame, trace: &InputTrace) -> Result<ReplayReport, ReplayError> {
    if trace.events.is_empty() {
        return Err(ReplayError::Trace("trace has no events".to_owned()));
    }
    let mut events = Vec::with_capacity(trace.events.len());
    let mut snapshot_sequence = 0u64;
    for event in &trace.events {
        let (verdict, reason) = apply_event(game, event, &mut snapshot_sequence)?;
        events.push(LoggedEvent {
            sequence: event.sequence,
            control: event.control.clone(),
            action: event.action.clone(),
            verdict,
            reason,
            screen: game.screen_name().to_owned(),
            side: game.side().map(side_name).map(str::to_owned),
            hash: game.state_hash(),
        });
    }
    Ok(ReplayReport {
        scenario: trace.scenario.clone(),
        events,
        final_side: game.side().map(side_name).map(str::to_owned),
        final_hash: game.state_hash(),
    })
}

/// Replay one trace twice from fresh staged games and compare the
/// runs: per-event verdicts, per-event hashes, and the final state
/// hash must all match.
///
/// # Errors
///
/// Whatever [`replay_once`] reports, plus
/// [`ReplayError::Nondeterministic`] naming the first divergence.
pub fn replay_twice(
    stage: impl Fn(&mut ScreenGame),
    trace: &InputTrace,
) -> Result<(ReplayReport, ReplayReport), ReplayError> {
    let mut first_game = ScreenGame::new();
    stage(&mut first_game);
    let first = replay_once(&mut first_game, trace)?;
    let mut second_game = ScreenGame::new();
    stage(&mut second_game);
    let second = replay_once(&mut second_game, trace)?;
    compare_reports(&first, &second)?;
    Ok((first, second))
}

/// Name the first divergence between two repeat runs, if any.
fn compare_reports(first: &ReplayReport, second: &ReplayReport) -> Result<(), ReplayError> {
    if first.scenario != second.scenario {
        return Err(ReplayError::Nondeterministic(format!(
            "scenario {:?} != {:?}",
            first.scenario, second.scenario
        )));
    }
    if first.verdicts() != second.verdicts() {
        return Err(ReplayError::Nondeterministic(
            "per-event verdicts differ".to_owned(),
        ));
    }
    for (first_event, second_event) in first.events.iter().zip(second.events.iter()) {
        if first_event.hash != second_event.hash {
            return Err(ReplayError::Nondeterministic(format!(
                "event {} hash {} != {}",
                first_event.sequence, first_event.hash, second_event.hash
            )));
        }
    }
    if first.final_hash != second.final_hash {
        return Err(ReplayError::Nondeterministic(format!(
            "final hash {} != {}",
            first.final_hash, second.final_hash
        )));
    }
    if first.final_side != second.final_side {
        return Err(ReplayError::Nondeterministic(format!(
            "final side {:?} != {:?}",
            first.final_side, second.final_side
        )));
    }
    Ok(())
}

/// Feed one event at the snapshot seam (or assert/refuse it) and
/// report its verdict.
fn apply_event(
    game: &mut ScreenGame,
    event: &TraceEvent,
    snapshot_sequence: &mut u64,
) -> Result<(EventVerdict, Option<String>), ReplayError> {
    if let Some(lane) = event.control.strip_prefix("lane-") {
        let lane: usize = lane
            .parse()
            .map_err(|_| ReplayError::Trace(format!("bad lane {:?}", event.control)))?;
        if lane > 6 {
            return Ok(refused(format!("lane {lane} outside 0..=6")));
        }
        let down = edge_direction(event, "press", "release")?;
        feed_snapshot(game, snapshot_sequence, |snapshot| {
            snapshot.push_edge(InputEdge::Lane { lane, down });
        });
        return Ok((EventVerdict::Applied, None));
    }
    if let Some(menu) = event.control.strip_prefix("menu-") {
        let key = match menu {
            "confirm" => MenuKey::Confirm,
            "back" => MenuKey::Back,
            "up" => MenuKey::Up,
            "down" => MenuKey::Down,
            _ => return Ok(refused(format!("unknown menu {:?}", event.control))),
        };
        let down = edge_direction(event, "press", "release")?;
        feed_snapshot(game, snapshot_sequence, |snapshot| {
            snapshot.push_edge(InputEdge::Menu { key, down });
        });
        return Ok((EventVerdict::Applied, None));
    }
    match event.control.as_str() {
        "click" => apply_click(game, event, snapshot_sequence),
        "await-screen" => apply_sync_point(game, event),
        "text" => Ok(refused(
            "text needs window IME injection; integration-layer probe only".to_owned(),
        )),
        other => Ok(refused(format!("unknown control {other:?}"))),
    }
}

/// Press (`true`) or release (`false`) for a two-action edge.
fn edge_direction(event: &TraceEvent, press: &str, release: &str) -> Result<bool, ReplayError> {
    match event.action.as_str() {
        action if action == press => Ok(true),
        action if action == release => Ok(false),
        _ => Err(ReplayError::Trace(format!(
            "event {}: action {:?} is neither {press:?} nor {release:?}",
            event.sequence, event.action
        ))),
    }
}

/// A refused event: logged with its reason, nothing fed.
fn refused(reason: String) -> (EventVerdict, Option<String>) {
    (EventVerdict::Rejected, Some(reason))
}

/// Freeze one snapshot, number it, and sweep it through the seam.
fn feed_snapshot(
    game: &mut ScreenGame,
    snapshot_sequence: &mut u64,
    fill: impl FnOnce(&mut InputSnapshot),
) {
    let mut snapshot = InputSnapshot::new();
    fill(&mut snapshot);
    snapshot.set_sequence(*snapshot_sequence);
    *snapshot_sequence = snapshot_sequence.saturating_add(1);
    game.ingest_snapshot(&snapshot);
}

/// One click event: press holds the button at the point, release
/// frees it (the press/release split behind the #111 arm evidence).
fn apply_click(
    game: &mut ScreenGame,
    event: &TraceEvent,
    snapshot_sequence: &mut u64,
) -> Result<(EventVerdict, Option<String>), ReplayError> {
    let Some((x, y)) = event.x.zip(event.y) else {
        return Ok(refused("click needs x and y".to_owned()));
    };
    if !(0..800).contains(&x) || !(0..600).contains(&y) {
        return Ok(refused(format!(
            "click ({x},{y}) outside the 800x600 canvas"
        )));
    }
    match event.action.as_str() {
        "press" => {
            let mut snapshot = InputSnapshot::press_at(x, y);
            snapshot.set_sequence(*snapshot_sequence);
            *snapshot_sequence = snapshot_sequence.saturating_add(1);
            game.ingest_snapshot(&snapshot);
            Ok((EventVerdict::Applied, None))
        }
        "release" => {
            let mut snapshot = InputSnapshot::release_at(x, y);
            snapshot.set_sequence(*snapshot_sequence);
            *snapshot_sequence = snapshot_sequence.saturating_add(1);
            game.ingest_snapshot(&snapshot);
            Ok((EventVerdict::Applied, None))
        }
        _ => Err(ReplayError::Trace(format!(
            "event {}: click action {:?} is neither press nor release",
            event.sequence, event.action
        ))),
    }
}

/// One `await-screen` sync point: satisfied by the flow screen or by
/// the open side overlay (shop/MyRoom ride sides, never the flow).
fn apply_sync_point(
    game: &mut ScreenGame,
    event: &TraceEvent,
) -> Result<(EventVerdict, Option<String>), ReplayError> {
    if event.action != "reach" {
        return Err(ReplayError::Trace(format!(
            "event {}: await-screen action {:?} is not reach",
            event.sequence, event.action
        )));
    }
    let Some(expected) = event.screen.as_deref() else {
        return Err(ReplayError::Trace(format!(
            "event {}: await-screen needs a screen",
            event.sequence
        )));
    };
    let screen = game.screen_name();
    let side = game.side().map(side_name).map(str::to_owned);
    if screen == expected || side.as_deref() == Some(expected) {
        return Ok((EventVerdict::Accepted, None));
    }
    Err(ReplayError::SyncMissed {
        sequence: event.sequence,
        expected: expected.to_owned(),
        screen: screen.to_owned(),
        side,
    })
}

/// Lowercase side vocabulary matching `screen_name` (the sync-point
/// namespace covers both lobbies and side overlays).
fn side_name(side: SideScreen) -> &'static str {
    match side {
        SideScreen::Shop => "shop",
        SideScreen::MusicShop => "musicshop",
        SideScreen::MyRoom => "myroom",
        SideScreen::Planet => "planet",
        SideScreen::Lanking => "lanking",
        SideScreen::Payment => "payment",
        SideScreen::Tutorial => "tutorial",
    }
}

/// Synthetic avatar-item catalog: `row_count` type-2 goods in table
/// order (slots count 0..), the deterministic stand-in for the
/// `Itemdata_Korea.dat` join — no copyrighted bytes, no game data.
#[must_use]
pub fn menu_test_catalog(row_count: u32) -> AvatarItemTable {
    AvatarItemTable {
        rows: (1..=row_count)
            .map(|item_id| AvatarItemRow {
                item_id,
                item_type: 2,
                price: 100,
                params: [0, 0, (item_id - 1) << 24, 0],
                flags: (0, 0),
                name: format!("Gear{item_id}"),
                description: String::new(),
                sprites: Vec::new(),
            })
            .collect(),
    }
}

/// Walk a fresh game to the Room lobby (the documented Avi → Planet →
/// Channel → Room walk, synthetic channel/room lists).
pub fn stage_room_lobby(game: &mut ScreenGame) {
    game.set_channels(vec![ChannelState {
        gateway_id: 0,
        channel_id: 2,
        capacity: 100,
        population: 5,
        active: true,
    }]);
    game.set_rooms(vec![RoomListEntry {
        number: 3,
        state_code: 1,
        title: "Jam".to_owned(),
        has_password: false,
        music_id: 0,
        difficulty: 0,
        key_mode: 2,
        game_mode: 0,
        speed: 1,
        capacity: 8,
        user_count: 5,
        min_level: 0,
        max_level: 0,
        skills: Vec::new(),
        premium: false,
        room_type: 0,
    }]);
    game.lane_key(3, true);
    game.click_planet(136, 88);
    game.menu_key(MenuKey::Confirm, true);
    debug_assert_eq!(game.screen(), Screen::Room);
}

/// Stage one room chrome button on the shared test rect.
pub fn stage_room_button(game: &mut ScreenGame, code: u32, id: &str) {
    game.load_scene(
        Screen::Room,
        vec![ScenePlacement {
            role: PlacementRole::Single,
            id: id.to_owned(),
            code,
            sprite: None,
            rect: ROOM_BUTTON_RECT,
        }],
        HashMap::new(),
    );
}

/// Stage the shop side placements the goods traces click: two
/// `ITEMBUY` row buttons, the item-list scroll pager pair, the
/// `BUYITEM` commit, and the owned-panel pager pair. Coordinates
/// match the trace `x`/`y` points; geometry stays synthetic and
/// documented (never parsed `.bnd` rects).
#[must_use]
pub fn shop_test_placements() -> Vec<ScenePlacement> {
    let mut placements = Vec::new();
    let mut place = |code: u32, id: &str, rect: Rect| {
        placements.push(ScenePlacement {
            role: PlacementRole::Single,
            id: id.to_owned(),
            code,
            sprite: None,
            rect,
        });
    };
    place(
        SHOP_ITEMBUY_FIRST_CODE,
        "IDC_BUTTON_SSHOP_ITEMBUY1",
        Rect {
            x: 100,
            y: 100,
            w: 81,
            h: 17,
        },
    );
    place(
        SHOP_ITEMBUY_FIRST_CODE + 1,
        "IDC_BUTTON_SSHOP_ITEMBUY2",
        Rect {
            x: 100,
            y: 120,
            w: 81,
            h: 17,
        },
    );
    place(
        SHOP_SCROLL_LEFT_CODE,
        "Shop_ItemScroll_Left",
        Rect {
            x: 0,
            y: 0,
            w: 18,
            h: 18,
        },
    );
    place(
        SHOP_SCROLL_RIGHT_CODE,
        "Shop_ItemScroll_Right",
        Rect {
            x: 527,
            y: 0,
            w: 18,
            h: 18,
        },
    );
    place(
        SHOP_BUYITEM_CODE,
        "IDC_BUTTON_SSHOP_BUYITEM",
        Rect {
            x: 600,
            y: 500,
            w: 60,
            h: 20,
        },
    );
    place(
        SHOP_BAG_LEFT_CODE,
        "IDC_BUTTON_SSHOP_BAGLEFT",
        Rect {
            x: 640,
            y: 500,
            w: 30,
            h: 20,
        },
    );
    place(
        SHOP_BAG_RIGHT_CODE,
        "IDC_BUTTON_SSHOP_BAGRIGHT",
        Rect {
            x: 675,
            y: 500,
            w: 30,
            h: 20,
        },
    );
    placements
}

/// Stage the MyRoom side placements the equip/gift traces click: one
/// slot selector, three bag cells, the bag pager pair, the
/// `INVENTORY` apply, the first gift cell, and the gift `MOVE`.
#[must_use]
pub fn myroom_test_placements() -> Vec<ScenePlacement> {
    let mut placements = Vec::new();
    let mut place = |code: u32, id: &str, rect: Rect| {
        placements.push(ScenePlacement {
            role: PlacementRole::Single,
            id: id.to_owned(),
            code,
            sprite: None,
            rect,
        });
    };
    let cell = |x: i32, y: i32| Rect { x, y, w: 40, h: 20 };
    place(
        MYROOM_SLOT_FIRST_CODE,
        "IDC_BUTTON_SMYROOM_INSTRUMENT",
        cell(100, 30),
    );
    for (cell_index, x) in [200, 250, 300].into_iter().enumerate() {
        place(
            MYROOM_BAG_FIRST_CODE + cell_index as u32,
            &format!("IDC_IMAGE_SMYROOM_MYBAGITEM{}", cell_index + 1),
            cell(x, 10),
        );
    }
    place(MYROOM_BAG_LEFT_CODE, "Shop_Btn_BagLeft", cell(100, 10));
    place(MYROOM_BAG_RIGHT_CODE, "Shop_Btn_BagRight", cell(150, 10));
    place(MYROOM_APPLY_CODE, "Myroom_Inventory", cell(450, 10));
    place(
        MYROOM_GIFT_FIRST_CODE,
        "IDC_BUTTON_SMYROOM_GIFTBOX_EX1",
        cell(400, 10),
    );
    place(MYROOM_MOVE_CODE, "Myroom_Btn_GiftMove", cell(500, 10));
    placements
}

/// Stage the Room lobby with the item-shop chrome and the goods
/// side placements behind it (the shop traces enter through the
/// staged room button, exactly like the live click path).
pub fn stage_shop_browser(game: &mut ScreenGame) {
    stage_room_lobby(game);
    game.set_avatar_source(Vec::new(), menu_test_catalog(20));
    stage_room_button(game, ROOM_ITEMSHOP_CODE, "IDC_BUTTON_SROOM_ITEMSHOP");
    game.load_side_scene(shop_test_placements(), HashMap::new());
}

/// Stage the Room lobby with the avatar-room chrome and the MyRoom
/// side placements (gift-box contents included).
pub fn stage_myroom_overlay(game: &mut ScreenGame, item_gifts: Vec<GiftItemInfo>) {
    stage_room_lobby(game);
    game.set_avatar_source(Vec::new(), menu_test_catalog(13));
    stage_room_button(game, ROOM_MYROOM_CODE, "IDC_BUTTON_SROOM_MYROOM");
    game.set_myroom_items(menu_test_catalog(13));
    game.set_myroom_character(menu_test_character(item_gifts));
    game.load_side_scene(myroom_test_placements(), HashMap::new());
}

/// Stage the gift-box claim scene: the avatar-room overlay with one
/// staged gift (id 7, item 2, from "Giver").
pub fn stage_giftbox_claim(game: &mut ScreenGame) {
    stage_myroom_overlay(
        game,
        vec![GiftItemInfo {
            gift_id: 7,
            item_id: 2,
            sender: "Giver".to_owned(),
        }],
    );
}

/// Synthetic staging for one menu-trace family: `shop` (all three
/// shop traces), `myroom-equip`, or `myroom-gift`.
///
/// Returns `None` for an unknown family (the CLI names the valid
/// ones instead of guessing).
#[must_use]
pub fn stage_for_family(family: &str) -> Option<fn(&mut ScreenGame)> {
    match family {
        "shop" => Some(stage_shop_browser),
        "myroom-equip" => Some(|game| stage_myroom_overlay(game, Vec::new())),
        "myroom-gift" => Some(stage_giftbox_claim),
        _ => None,
    }
}

/// Replay one validated trace twice on synthetic staging and check
/// the verdict log plus the state hashes match, failing on any
/// refusal, missed sync point, or divergence.
///
/// # Errors
///
/// Unknown staging family, any refused event, a missed
/// `await-screen`, or a repeat-run divergence.
pub fn replay_menu_file(trace: &InputTrace, family: &str) -> anyhow::Result<ReplayReport> {
    let Some(stage) = stage_for_family(family) else {
        anyhow::bail!("unknown staging family {family:?} (shop, myroom-equip, myroom-gift)");
    };
    let (first, _) = replay_twice(stage, trace).map_err(|error| anyhow::anyhow!("{error}"))?;
    if first.rejected() > 0 {
        anyhow::bail!(
            "{} of {} events refused:\n{}",
            first.rejected(),
            first.events.len(),
            describe_report(&first)
        );
    }
    Ok(first)
}

/// Synthetic avatar-room owner: three owned goods plus optional
/// staged gift-box items.
#[must_use]
pub fn menu_test_character(item_gifts: Vec<GiftItemInfo>) -> CharacterInfo {
    CharacterInfo {
        invalid: false,
        nickname: "Tester".to_owned(),
        gender: Gender::Any,
        gem: 0,
        point: 0,
        o2_cash: 0,
        level: 12,
        battles: 0,
        wins: 0,
        losses: 0,
        experience: 3450,
        bonus_point: 0,
        is_administrator: false,
        equipment: [0; EQUIPMENT_SLOTS],
        inventory: vec![1, 0, 2, 3],
        unread_gifts: 0,
        item_gifts,
        music_gifts: Vec::new(),
        music_cash: 0,
        item_cash: 0,
        cash_point: 0,
        attributive_items: Vec::new(),
        penalty_count: 0,
        penalty_level: 0,
    }
}

/// One-line verdict summary per event for failure output.
fn describe_report(report: &ReplayReport) -> String {
    let mut out = String::new();
    for event in &report.events {
        let _ = writeln!(
            out,
            "seq={} {}/{} {:?} screen={} side={:?} hash={}",
            event.sequence,
            event.control,
            event.action,
            event.verdict,
            event.screen,
            event.side,
            event.hash
        );
        if let Some(reason) = &event.reason {
            let _ = writeln!(out, "  rejected: {reason}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trace of one raw event (hand-built for harness-rule probes;
    /// committed traces always arrive through [`load_menu_trace`]).
    fn raw_event(sequence: u64, control: &str, action: &str) -> TraceEvent {
        TraceEvent {
            sequence,
            time_ms: sequence,
            time_us: Some(sequence * 1000),
            control: control.to_owned(),
            action: action.to_owned(),
            x: None,
            y: None,
            text: None,
            screen: None,
            control_id: None,
            label: None,
        }
    }

    fn raw_trace(scenario: &str, events: Vec<TraceEvent>) -> InputTrace {
        InputTrace {
            schema_version: crate::trace::SCHEMA_VERSION.to_owned(),
            scenario: scenario.to_owned(),
            time_base: crate::trace::TimeBase {
                name: "menu-monotonic".to_owned(),
                unit: "microseconds".to_owned(),
            },
            events,
        }
    }

    fn click_event(sequence: u64, action: &str, x: i32, y: i32) -> TraceEvent {
        TraceEvent {
            sequence,
            time_ms: 1000 + sequence,
            time_us: Some((1000 + sequence) * 1000),
            control: "click".to_owned(),
            action: action.to_owned(),
            x: Some(x),
            y: Some(y),
            text: None,
            screen: None,
            control_id: None,
            label: None,
        }
    }

    /// The trace entry click lands on the staged room button: pins
    /// the trace/harness coordinate agreement (a drift would log a
    /// rejected-or-no-op click instead of entering the side).
    fn assert_entry_click_is_staged(trace: &InputTrace, x: i32, y: i32) {
        let press = trace.events.first().expect("trace has an entry click");
        assert_eq!((press.x, press.y), (Some(x), Some(y)));
    }

    /// Replay once more on a fresh staged game for end-state reads
    /// (outbox drains happen here, after the hashed compare).
    fn replay_for_state(
        stage: impl Fn(&mut ScreenGame),
        trace: &InputTrace,
    ) -> (ScreenGame, ReplayReport) {
        let mut game = ScreenGame::new();
        stage(&mut game);
        let report = replay_once(&mut game, trace).expect("state replay succeeds");
        (game, report)
    }

    /// Live caller chain (Tier 0 seam guard — the sweep seam is the
    /// whole behavior for input routing):
    ///   window::WindowHandler (frame freeze) → session::Session::ingest_snapshot →
    ///   ScreenGame::ingest_snapshot → apply_pointer_down/up
    /// Archives absent -> skip cleanly is N/A: staging is fully
    /// synthetic (no install-root reads, no server).
    #[test]
    fn room_shop_entry_replay_is_deterministic() {
        // Live chain: ... → click_room_control (ITEMSHOP arm) → visit_side(Shop).
        let trace = load_menu_trace("shop-entry-from-room.json").expect("committed trace loads");
        assert_entry_click_is_staged(&trace, ROOM_BUTTON_CLICK.0, ROOM_BUTTON_CLICK.1);
        let (first, second) =
            replay_twice(stage_shop_browser, &trace).expect("entry replay succeeds");
        assert_eq!(
            first.rejected(),
            0,
            "no event refused:\n{}",
            describe_report(&first)
        );
        assert_eq!(first.applied(), 2, "press and release feed the seam");
        assert_eq!(first.final_side.as_deref(), Some("shop"));
        assert_eq!(
            first.events.last().map(|event| event.screen.as_str()),
            Some("room")
        );
        assert_eq!(first.final_hash, second.final_hash);
        assert_eq!(first.verdicts(), second.verdicts());
        let (game, _) = replay_for_state(stage_shop_browser, &trace);
        assert_eq!(game.side(), Some(SideScreen::Shop));
        assert_eq!(game.screen(), Screen::Room);
    }

    /// Live chain: window::WindowHandler → session::Session::ingest_snapshot →
    /// ScreenGame::ingest_snapshot → ShopState::apply_action (pager arms).
    /// Synthetic staging; the pagers hold at the ends.
    #[test]
    fn shop_browse_pager_replay_turns_pages_and_holds() {
        let trace = load_menu_trace("shop-goods-browse-pager.json").expect("committed trace loads");
        let (first, _) = replay_twice(stage_shop_browser, &trace).expect("browse replay succeeds");
        assert_eq!(
            first.rejected(),
            0,
            "no event refused:\n{}",
            describe_report(&first)
        );
        assert_eq!(first.final_side.as_deref(), Some("shop"));
        // Twenty goods over eight rows: next, next, hold, prev lands
        // on page 1; the empty owned panel holds page 0.
        let (game, _) = replay_for_state(stage_shop_browser, &trace);
        assert_eq!(game.shop_page(), 1, "pager turns and steps back");
        assert_eq!(game.shop_bag_page(), 0, "empty owned panel holds");
        assert_eq!(game.shop_list_pages(), 3, "twenty goods over eight rows");
    }

    /// Live chain: window::WindowHandler → session::Session::ingest_snapshot →
    /// ScreenGame::ingest_snapshot → ShopState::toggle_row / cart commit outbox.
    /// The #194 close leg (cart + BUYITEM round trip) at the seam.
    #[test]
    fn shop_row_cart_buyitem_replay_commits_the_cart() {
        let trace = load_menu_trace("shop-row-cart-buyitem.json").expect("committed trace loads");
        let (first, _) = replay_twice(stage_shop_browser, &trace).expect("buy replay succeeds");
        assert_eq!(
            first.rejected(),
            0,
            "no event refused:\n{}",
            describe_report(&first)
        );
        assert_eq!(first.final_side.as_deref(), Some("shop"));
        let (mut game, _) = replay_for_state(stage_shop_browser, &trace);
        assert_eq!(game.shop_cart(), vec![1, 2], "row toggles check the cart");
        assert!(
            game.take_shop_buy().is_some(),
            "BUYITEM queues the 5000 commit"
        );
    }

    /// Live chain: window::WindowHandler → session::Session::ingest_snapshot →
    /// ScreenGame::ingest_snapshot → apply_myroom_action (slot/bag/apply arms).
    #[test]
    fn myroom_bag_slot_equip_replay_applies_the_pick() {
        let trace = load_menu_trace("myroom-bag-slot-equip.json").expect("committed trace loads");
        assert_entry_click_is_staged(&trace, ROOM_BUTTON_CLICK.0, ROOM_BUTTON_CLICK.1);
        let (first, _) = replay_twice(|game| stage_myroom_overlay(game, Vec::new()), &trace)
            .expect("equip replay succeeds");
        assert_eq!(
            first.rejected(),
            0,
            "no event refused:\n{}",
            describe_report(&first)
        );
        assert_eq!(first.final_side.as_deref(), Some("myroom"));
        let (mut game, _) = replay_for_state(|game| stage_myroom_overlay(game, Vec::new()), &trace);
        let send = game
            .take_equip_send()
            .expect("INVENTORY queues the equip send");
        assert_eq!((send.slot, send.item_id), (0, 1), "slot 0 wears bag cell 0");
    }

    /// Live chain: window::WindowHandler → session::Session::ingest_snapshot →
    /// ScreenGame::ingest_snapshot → apply_myroom_claim (gift-box arm).
    #[test]
    fn giftbox_claim_replay_queues_the_claim() {
        use o2jam_net::gift::GiftType;

        let trace = load_menu_trace("giftbox-claim.json").expect("committed trace loads");
        assert_entry_click_is_staged(&trace, ROOM_BUTTON_CLICK.0, ROOM_BUTTON_CLICK.1);
        let (first, _) = replay_twice(stage_giftbox_claim, &trace).expect("claim replay succeeds");
        assert_eq!(
            first.rejected(),
            0,
            "no event refused:\n{}",
            describe_report(&first)
        );
        assert_eq!(first.final_side.as_deref(), Some("myroom"));
        let (mut game, _) = replay_for_state(stage_giftbox_claim, &trace);
        assert_eq!(game.myroom().gift_cell(), Some(0), "gift cell 0 picks");
        let claim = game.take_gift_claim().expect("MOVE queues the gift claim");
        assert_eq!(claim.gift_type, GiftType::Item);
        assert_eq!((claim.gift_id, claim.resource_id), (7, 2));
    }

    #[test]
    fn divergent_staging_diverges_hashes() {
        // Negative control: the repeat-twice comparison is not
        // vacuous — the same entry trace on shop-staged vs
        // myroom-staged games must land on different end states.
        let trace = load_menu_trace("shop-entry-from-room.json").expect("committed trace loads");
        // Drop the trailing shop sync: the click pair alone enters
        // whichever side the staging names.
        let mut body = trace.clone();
        body.events.pop();
        let mut shop_game = ScreenGame::new();
        stage_shop_browser(&mut shop_game);
        let shop_report = replay_once(&mut shop_game, &body).expect("shop replay succeeds");
        let mut myroom_game = ScreenGame::new();
        stage_myroom_overlay(&mut myroom_game, Vec::new());
        let myroom_report = replay_once(&mut myroom_game, &body).expect("myroom replay succeeds");
        assert_eq!(shop_report.rejected(), 0);
        assert_eq!(myroom_report.rejected(), 0);
        assert_ne!(shop_report.final_side, myroom_report.final_side);
        assert_ne!(shop_report.final_hash, myroom_report.final_hash);
    }

    #[test]
    fn text_events_reject_for_the_window_layer() {
        let mut event = raw_event(0, "text", "type");
        event.text = Some("hi".to_owned());
        let trace = raw_trace("text-probe", vec![event]);
        let mut game = ScreenGame::new();
        let report = replay_once(&mut game, &trace).expect("refusals log, never fail");
        assert_eq!(report.rejected(), 1);
        assert!(report.events[0]
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("IME")));
    }

    #[test]
    fn unknown_controls_reject_without_a_silent_drop() {
        let trace = raw_trace("unknown-probe", vec![raw_event(0, "telepathy", "press")]);
        let mut game = ScreenGame::new();
        let report = replay_once(&mut game, &trace).expect("refusals log, never fail");
        assert_eq!(report.rejected(), 1);
        assert_eq!(report.applied(), 0);
    }

    #[test]
    fn await_screen_mismatch_fails_the_run() {
        let mut event = raw_event(0, "await-screen", "reach");
        event.screen = Some("shop".to_owned());
        let trace = raw_trace("sync-probe", vec![event]);
        let mut game = ScreenGame::new();
        let error = replay_once(&mut game, &trace).expect_err("missed sync must fail");
        assert!(matches!(error, ReplayError::SyncMissed { .. }), "{error}");
    }

    #[test]
    fn out_of_bounds_clicks_reject_at_the_seam() {
        let trace = raw_trace("oob-probe", vec![click_event(0, "press", 900, 100)]);
        let mut game = ScreenGame::new();
        let report = replay_once(&mut game, &trace).expect("refusals log, never fail");
        assert_eq!(report.rejected(), 1);
    }

    #[test]
    fn empty_traces_refuse_before_any_feed() {
        let trace = raw_trace("empty-probe", Vec::new());
        let mut game = ScreenGame::new();
        let error = replay_once(&mut game, &trace).expect_err("empty trace must fail");
        assert!(matches!(error, ReplayError::Trace(_)), "{error}");
    }
}

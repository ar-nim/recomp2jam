//! Waiting-room paint generation: seats, staged covers, captions,
//! badge, role-owned confirm face, and the flat fallback.
//!
//! Kept behind re-exports in `super::paint`, so callers and the
//! existing scenario guards stay unchanged (#139 paint split).

use super::*;

/// Seats shown on the waiting screen: self plus the seven wire slots.
pub(crate) const WAITING_SEATS: usize = 8;
/// Peer slots carried beside self in [`JoinedRoom`].
const WAITING_PEERS: usize = 7;
/// Waiting rows share the lobby row height.
const WAITING_ROW_H: i32 = 8;

/// One waiting-room seat view: display name plus team/ready state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WaitingSeat {
    team: u8,
    name: String,
    pub(crate) kind: SeatKind,
}

/// Seat occupancy: only occupied seats carry a name and ready flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SeatKind {
    Occupied { ready: bool },
    Empty,
    Locked,
}

impl WaitingSeat {
    /// Display name (`None` for vacant/locked seats).
    #[must_use]
    pub(crate) fn display_name(&self) -> Option<&str> {
        match self.kind {
            SeatKind::Occupied { .. } => Some(&self.name),
            SeatKind::Empty | SeatKind::Locked => None,
        }
    }

    /// Ready flag (vacant/locked seats are never ready).
    #[must_use]
    pub(crate) fn is_ready(&self) -> bool {
        matches!(self.kind, SeatKind::Occupied { ready: true })
    }
}

/// Team button letter (`0` -> `A` .. `7` -> `H`); `?` when out of range.
fn team_letter(team: u8) -> char {
    if team < 8 {
        (b'A' + team) as char
    } else {
        '?'
    }
}

/// Our own seat: name is local-only (the wire sends no self nickname).
fn self_seat(room: &JoinedRoom) -> WaitingSeat {
    // Own readiness rides the member slot like every peer (the ready
    // toggle folds there); unknown seats keep the unready look.
    let ready = room
        .slots
        .iter()
        .find(|slot| slot.index == room.member_id)
        .and_then(|slot| slot.member.as_ref())
        .is_some_and(|member| member.ready);
    WaitingSeat {
        team: room.team,
        name: "YOU".to_owned(),
        kind: SeatKind::Occupied { ready },
    }
}

/// Vacant seat placeholder.
fn empty_seat() -> WaitingSeat {
    WaitingSeat {
        team: 0,
        name: String::new(),
        kind: SeatKind::Empty,
    }
}

/// View for one wire slot: named when occupied, locked/empty otherwise.
fn peer_seat(slot: &RoomSlot) -> WaitingSeat {
    match &slot.member {
        Some(member) => WaitingSeat {
            team: member.team,
            name: member.nickname.clone(),
            kind: SeatKind::Occupied {
                ready: member.ready,
            },
        },
        None if slot.state_code == RoomSlotState::Locked as i32 => WaitingSeat {
            team: 0,
            name: String::new(),
            kind: SeatKind::Locked,
        },
        None => empty_seat(),
    }
}

/// Eight seat views: our seat first, then the wire slots, padded empty.
///
/// The wire carries seven peer slots beside self (`JoinedRoom.slots`);
/// together they are the eight waiting-room seats. OPEN: `slots`
/// includes our own index (`member_id`), so this view paints our
/// member twice — the seat-to-cover mapping needs evidence (next step:
/// a 7.55 waiting capture with two players seated in known slots, then
/// compare which cover each name lands on).
#[must_use]
pub(crate) fn waiting_seats(room: &JoinedRoom) -> Vec<WaitingSeat> {
    let mut seats = Vec::with_capacity(WAITING_SEATS);
    seats.push(self_seat(room));
    for slot in room.slots.iter().take(WAITING_PEERS) {
        seats.push(peer_seat(slot));
    }
    while seats.len() < WAITING_SEATS {
        seats.push(empty_seat());
    }
    seats
}

/// One seat row: seat, team, name, flag (`R` ready, `-` waiting).
/// Vacant seats keep their index with a `---`/`LOCK` placeholder.
pub(crate) fn waiting_row_text(index: usize, seat: &WaitingSeat) -> String {
    match seat.kind {
        SeatKind::Occupied { ready } => {
            let flag = if ready { 'R' } else { '-' };
            format!(
                "{} {} {} {}",
                index,
                team_letter(seat.team),
                seat.name,
                flag
            )
        }
        SeatKind::Empty => format!("{index} - ---"),
        SeatKind::Locked => format!("{index} - LOCK"),
    }
}

/// Ready-dot ink: green when ready, dim otherwise.
pub(crate) fn ready_dot_color(ready: bool) -> u32 {
    if ready {
        pack_xrgb(0x30, 0xE0, 0x30)
    } else {
        pack_xrgb(0x50, 0x50, 0x50)
    }
}

/// Top-left of seat `index`: four columns by two rows, relative to frame
/// (Wine `ab-wait.png` + siblings: four seats across the top row,
/// four across the bottom — seat 4 opens the second row under seat 0).
#[must_use]
pub(crate) fn seat_origin(index: usize, w: usize, h: usize) -> (i32, i32) {
    let col = (index % 4) as i32;
    let row = (index / 4) as i32;
    (
        w as i32 / 16 + col * (w as i32 / 4),
        h as i32 / 4 + row * WAITING_ROW_H,
    )
}

/// Seat rows plus ready dots (no background fill: shared by the flat
/// fallback and the art scene).
fn paint_waiting_rows(frame: &mut Frame<'_>, room: &JoinedRoom) {
    for (index, seat) in waiting_seats(room).iter().enumerate() {
        paint_waiting_seat(frame, index, seat);
    }
}

/// Paint the eight waiting-room seats with names and ready flags.
pub(crate) fn paint_waiting_members(frame: &mut Frame<'_>, room: &JoinedRoom) {
    fill_background(frame, Screen::Waiting);
    paint_title(frame, "WAITING");
    paint_waiting_rows(frame, room);
}

/// Song behind the `MUSICNAME` caption (`IDC_TEXT_SWAIT_MUSICNAME`,
/// code `0X16400017`): the id the caption falls back to, plus the
/// staged catalog title when one names that id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitingSong<'a> {
    /// Song id the caption names.
    pub music_id: u16,
    /// Staged catalog title of that id, if the catalog names it.
    pub title: Option<&'a str>,
}

/// Live caption sources for the staged waiting rects. Every field
/// has a pinned shell source (`docs/caption-sources.md`): the map
/// name rides the room word, the BPM rides the staged catalog
/// column for the room's song, and the counts ride the staged
/// inventory. `Default` is the no-source state (mismatch `-0-`,
/// zeroed counts, no title) — the exe paints exactly this without
/// a staged record.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WaitingCaptions<'a> {
    /// Song behind the `MUSICNAME` caption: the master's staged pick
    /// when one stands, else the joined room's song.
    pub song: Option<WaitingSong<'a>>,
    /// Staged catalog BPM of the room's song, if it names one.
    pub song_bpm: Option<f32>,
    /// Staged inventory counts, one per skill slot.
    pub skill_counts: [u32; 3],
    /// Difficulty and speed behind the `LEVEL` badge, from the same
    /// pick-or-room source as `song`.
    pub level: Option<WaitingLevel>,
    /// Pinned notice standing over the top-bar room line (the exe's
    /// notice box above the START control,
    /// `docs/music-availability.md` §4); `None` paints the room title.
    pub notice: Option<&'static str>,
}

/// Difficulty and speed behind the `LEVEL` badge: the two bytes the
/// exe's frame formula reads (`0x7789e7` difficulty, `0x7789e9`
/// speed, struct `0x7789d8`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitingLevel {
    /// Difficulty code (`Difficulty::from_code`).
    pub difficulty: u8,
    /// Speed code (`GameSpeed::from_code`).
    pub speed: u8,
}

/// Full-canvas sheet behind the Waiting seats. `State_Waiting.ojs`
/// carries two 800x600 records: record 0 is the eight-card grid, record
/// 1 the two-card VS sheet (decoded from the pinned Interface archive).
/// The exe's image control sets one record; stacking both is what paints
/// the VS sheet over the top cards in the 0.749 A/B.
const WAITING_BACKDROP_SPRITE: &str = "State_Waiting.ojs";
const WAITING_PLAIN_RECORD: usize = 0;
const WAITING_VERSUS_RECORD: usize = 1;

/// Waiting select-overlay controls from `State_Waiting` (`ControlList`
/// codes, paired to `State_Waiting.bnd` by ordinal). The exe resets all
/// of them to frame 0 at base-ready: popup `0x16500074`, other-show
/// `0x16500073`, wait-text `0x16500067`, its animation `0x16500068`, and
/// the orange select arrow `0x16500070` (waiting setup `0x4aac80`,
/// state update `0x4ae350`, MUSELECT handler `0x4ac280`). Only the
/// role-2 (Challenger) branch arms the popup.
const WAITING_SELECT_OVERLAY_CODES: [u32; 5] = [
    0x1650_0074,
    0x1650_0073,
    0x1650_0067,
    0x1650_0068,
    0x1650_0070,
];

/// Paint the one backdrop record the exe's image control selected.
/// Bound staging puts the full-canvas placement first, so this paints
/// before the remaining scene figures; the generic pass below skips it.
fn paint_waiting_backdrop(frame: &mut Frame<'_>, art: &SceneArt, role: MemberRole) {
    let Some(placed) = art
        .placements
        .iter()
        .find(|placed| placed.sprite.as_deref() == Some(WAITING_BACKDROP_SPRITE))
    else {
        return;
    };
    let record = if matches!(role, MemberRole::Champion | MemberRole::Challenger) {
        WAITING_VERSUS_RECORD
    } else {
        WAITING_PLAIN_RECORD
    };
    let shares_body = shared_sprite_names(&art.placements).contains(WAITING_BACKDROP_SPRITE);
    paint_placement_layer(frame, placed, &art.figures, record, shares_body, None);
}

/// Paint staged waiting art (team buttons, avatar covers) with the
/// live seats, ready dots, and pinned captions on top: staged
/// placements anchor every region (covers, team grid, READY, skills,
/// counts, map, BPM, speed, music, wins), while the flat fallback
/// below keeps the invented grid. Without staged cover placements
/// the legacy rows keep their slot so the bare scene still reads.
///
/// `confirm_armed` is the local player's ready/start request standing:
/// the role-owned confirm body then paints its armed record instead of
/// its rest record (see [`paint_waiting_confirm_face`]).
/// `worn_figures_staged` suppresses the baked sketch of
/// `Wait_Cover_Avatar.ojs`; the 7.55 capture shows item-composed
/// figures instead of that placeholder (issue #336).
///
/// `member_role` mirrors the exe's word at VA `0x77ee86` (D10). At a
/// normal base-ready room the word is 3 (Spectator — live winedbg read
/// 2026-11-18, `~/scratch/p343-wine-role4`), so the image control paints
/// only record 0 of `State_Waiting.ojs` (the eight light-blue cards)
/// and none of the Challenger-only select overlay. Champion/Challenger
/// (1/2) select record 1, the VS sheet; only Challenger arms the popup.
pub fn paint_waiting_scene(
    frame: &mut Frame<'_>,
    room: &JoinedRoom,
    art: &SceneArt,
    captions: &WaitingCaptions<'_>,
    confirm_armed: bool,
    worn_figures_staged: bool,
    member_role: MemberRole,
) {
    let seats = staged_seats(room);
    // Seam: `ScreenGame::is_room_master` (seat id 0), mirrored here as
    // `room.member_id == 0`. See [`waiting_control_armed_for_role`].
    let is_room_master = room.member_id == 0;
    paint_waiting_backdrop(frame, art, member_role);
    art.paint_backdrop_live(frame, Screen::Waiting, |placed| {
        placed.sprite.as_deref() != Some(WAITING_BACKDROP_SPRITE)
            && waiting_seat_clone_live(placed, &seats)
            && waiting_control_armed_for_role(&placed.id, is_room_master)
            && !waiting_paints_one_record(&placed.id)
            && !(worn_figures_staged && placed.sprite.as_deref() == Some(WAIT_COVER_SPRITE))
            && !(member_role != MemberRole::Challenger
                && WAITING_SELECT_OVERLAY_CODES.contains(&placed.code))
    });
    // The confirm face keeps the backdrop's stacking slot: it painted in
    // the backdrop pass before it selected its own record.
    paint_waiting_confirm_face(frame, art, is_room_master, confirm_armed);
    paint_waiting_seat_marks(frame, &seats, art);
    if waiting_has_staged_covers(&art.placements) {
        paint_waiting_staged(frame, room, art, captions);
    } else {
        paint_waiting_rows(frame, room);
    }
}

/// Whether a Waiting control paints one selected record instead of its
/// staged layer stack, so the rest stack leaves it out: the `LEVEL`
/// badge ([`paint_waiting_level_badge`]) and the role-owned confirm
/// body ([`paint_waiting_confirm_face`]). The exe's image control
/// draws the one frame its setter staged (`docs/ui-draw.md` steps 3-6;
/// a frame swap never stacks frames), so stacking the badge's 60
/// frames would paint the last one (`MXR`) over the live pick, and
/// stacking the confirm body's 3 frames would paint its armed face
/// over the rest face forever.
fn waiting_paints_one_record(id: &str) -> bool {
    id == WAITING_LEVEL_ID
        || id == waiting_confirm_control_id(true)
        || id == waiting_confirm_control_id(false)
}

/// Record index of the armed confirm face: the third record of
/// `Wait_Start.ojs`/`Wait_Ready.ojs` (both carry 3 raw 142x37 records).
/// The exe's kind-`0x2000` dispatch arm (file `0x4f6b7c`, reached from
/// the press setter `0x4f72b0`) sets absolute frame 3 when the control's
/// press state word is non-zero, and frames are 1-based
/// (`docs/room-enums.md:145`: frame n paints record n-1).
const WAITING_CONFIRM_ARMED_RECORD: usize = 2;

/// Paint the role-owned Waiting confirm body: exactly one record, its
/// rest record (index 0, the dim face) or the armed record
/// ([`WAITING_CONFIRM_ARMED_RECORD`]) while the local request stands.
///
/// The exe arms this face through the control's press state word
/// `[ctrl+0x60]`: the START click handler sets it (VA `0x4b22d0`, file
/// `0xb237d`) and the read-back arm of the same handler only re-arms a
/// cleared control (file `0xb242f`); the seat-ready update arms it while
/// the local ready byte stands and clears it with an explicit rest frame
/// (VA `0x4b5850`, file `0xb590b`-`0xb594f`). A body without a third
/// record (no pressed art staged) keeps its rest record, never a
/// stand-in paint.
fn paint_waiting_confirm_face(
    frame: &mut Frame<'_>,
    art: &SceneArt,
    is_room_master: bool,
    armed: bool,
) {
    let confirm_id = waiting_confirm_control_id(is_room_master);
    let Some(placement) = art.placements.iter().find(|placed| placed.id == confirm_id) else {
        return;
    };
    let shared = shared_sprite_names(&art.placements);
    let shares_body = placement
        .sprite
        .as_deref()
        .is_some_and(|sprite| shared.contains(sprite));
    let pressed_record_staged = placement
        .sprite
        .as_ref()
        .and_then(|sprite| art.figures.get(sprite))
        .is_some_and(|figure| figure.layer(WAITING_CONFIRM_ARMED_RECORD).is_some());
    let record = if armed && pressed_record_staged {
        WAITING_CONFIRM_ARMED_RECORD
    } else {
        0
    };
    paint_placement_layer(frame, placement, &art.figures, record, shares_body, None);
}

/// The Waiting confirm control the local role owns
/// (`docs/room-enums.md` "Waiting master gate"): the master arm enables
/// `START 0x16202012` (file `0xae3d3`) and the non-master arm enables
/// `READY 0x16202013` (file `0xae835`), so the same staged rect fires a
/// different request per role (`ScreenGame::press_kind_at`).
#[must_use]
pub const fn waiting_confirm_control_id(is_room_master: bool) -> &'static str {
    if is_room_master {
        "IDC_BUTTON_SWAIT_START"
    } else {
        "IDC_BUTTON_SWAIT_READY"
    }
}

/// Whether the Waiting setup arms one staged control for this role.
///
/// The exe swaps role-paired controls in its Waiting dialog setup (func
/// file `0xae350`; the `+0x30` local flag read at file `0xae38e` picks
/// the arm):
///
/// - master arm — enables `START 0x16202012` (file `0xae3d3`), disables
///   `READY 0x16202013` (file `0xae3ff` plus the frame reset through VA
///   `0x42f870`), and frame-resets both inactive-dress overlays
///   (`READYBUTTEONINACTIVE 0x1650006e` fetch file `0xae796`,
///   `SPEEDBUTTEONINACTIVE 0x1650006f` fetch file `0xae7cb`; reset at
///   file `0xae7c6`/`0xae7fb`);
/// - non-master arm (file `0xae805`) — enables READY (file `0xae835`),
///   disables START (file `0xae859`), and leaves the inactive overlays
///   untouched.
///
/// `MASTER_SKILLSEL`/`SKILLSEL` carry the same pairing (ControlList
/// comments name them "when master" / "when not master"). Disable-vs-hide
/// stays OPEN (a frame reset paints nothing, so this shell hides); when a
/// guest shows the inactive overlays is OPEN too — the state-driven update
/// shows both while `[0x77ee86] == 3` (file `0xaec29`), and the word's
/// meaning has no pin yet. Next evidence step: pin `[0x77ee86]` (the room
/// or player state the guest overlay waits on) against a live capture.
///
/// The song rect at `.bnd` rect 2 `(540,493)-(594,542)` is the third
/// role-swapped region: `IDC_BUTTON_SWAIT_MUSELECT 0x16200003`
/// (`Wait_MusicSelect.ojs`, the blue 곡선택 button) sits under its
/// `BPM` cover `IDC_IMAGE_SWAIT_COVERMUSIC 0x1650001d`
/// (`Wait_Cover_Music.ojs`, `.bnd` rect 28, authored over the button).
/// The owner decision (2026-11-16, ticket #278) sets this shell rule:
/// the host sees the button with no cover, a peer sees the cover with no
/// button. The exe display rule is OPEN — no qualified capture pins it
/// (`docs/room-enums.md` "The waiting song rect"; next evidence step is
/// a Wine/exe capture with a provenance manifest, gated by #240), so
/// this rule is the owner's call, not an exe reading.
/// `IDC_IMAGE_SWAIT_ALBUMMODE 0x1650001c` paints for no role; whether
/// an album-mode room arms it is OPEN. Click routing is untouched: the
/// host still opens the song dialog from this rect (dispatch case file
/// `0xb2622`), which is exe behavior, not a shell gap.
#[must_use]
pub(crate) fn waiting_control_armed_for_role(id: &str, is_room_master: bool) -> bool {
    match id {
        "IDC_BUTTON_SWAIT_START" | "IDC_BUTTON_SWAIT_MASTER_SKILLSEL" => is_room_master,
        "IDC_BUTTON_SWAIT_READY"
        | "IDC_BUTTON_SWAIT_SKILLSEL"
        | "IDC_IMAGE_SWAIT_READYBUTTEONINACTIVE"
        | "IDC_IMAGE_SWAIT_SPEEDBUTTEONINACTIVE" => !is_room_master,
        "IDC_BUTTON_SWAIT_MUSELECT" => is_room_master,
        "IDC_IMAGE_SWAIT_COVERMUSIC" => !is_room_master,
        "IDC_IMAGE_SWAIT_ALBUMMODE" => false,
        _ => true,
    }
}

/// Seat avatar slot (`IDC_IMAGE_SWAIT_AVATAR`, code `0x1670002E`): a
/// spriteless `SET 0x0C` member whose rect is seat-relative (the seat
/// template's own origin), so the painter adds the seat parent's rect
/// origin — the same write-back the clone expansion applies to the
/// members that carry art.
pub(crate) const WAITING_AVATAR_SLOT_ID: &str = "IDC_IMAGE_SWAIT_AVATAR";

/// Native avatar slot of one seat: the spriteless template rect added
/// onto the seat parent's origin (the clone write-back rule), `None`
/// while either placement is unstaged — the slot then stays empty,
/// never a raw-rect paint at the frame corner.
#[must_use]
fn waiting_seat_slot_rect(placements: &[ScenePlacement], seat: usize) -> Option<Rect> {
    let slot = placements
        .iter()
        .find(|placed| placed.id == WAITING_AVATAR_SLOT_ID)
        .map(|placed| placed.rect)?;
    let parent = WAITING_SEAT_PARENT_IDS
        .get(seat)
        .and_then(|id| placements.iter().find(|placed| placed.id == *id))
        .map(|placed| placed.rect)?;
    Some(Rect {
        x: parent.x.saturating_add(slot.x),
        y: parent.y.saturating_add(slot.y),
        w: slot.w,
        h: slot.h,
    })
}

/// Paint one worn figure into one seat slot, centred on the slot's
/// scaled box (the shared seat-slot path: self and peers alike). The
/// visible box centers, so the head-to-feet stack lands inside the
/// slot (ticket #171) — the same helper the room panel uses.
fn paint_figure_in_seat_slot(
    frame: &mut Frame<'_>,
    figure: &AvatarFigure,
    slot: &Rect,
    now_ms: u64,
) {
    let scaled = scale_rect_to_frame(slot, frame.w, frame.h);
    figure.paint_visible_centered(frame, &scaled, frame.w, frame.h, now_ms);
}

/// Paint the local player's worn figure into their own seat's avatar
/// slot (seat 0, the `IDC_BUTTON_SWAIT_EXTEND1` instance the exe seats
/// the room host on). Thin wrapper over the shared seat-slot path.
pub fn paint_waiting_seat_avatar(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    figure: Option<&AvatarFigure>,
    now_ms: u64,
) {
    let Some(figure) = figure else {
        return;
    };
    if let Some(slot) = waiting_seat_slot_rect(placements, 0) {
        paint_figure_in_seat_slot(frame, figure, &slot, now_ms);
    }
}

/// Paint each occupied seat's staged figure into its own avatar slot
/// through the same slot path as the local seat (seat-relative rect
/// plus the seat parent's origin). Vacant and locked seats keep their
/// bare plate even when a figure is staged; unstaged seats stay empty.
/// No template child ever paints at the raw rect: every figure goes
/// through its seat's parent offset.
pub fn paint_waiting_seat_avatars(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    room: &JoinedRoom,
    figures: &[Option<AvatarFigure>],
    now_ms: u64,
) {
    let seats = staged_seats(room);
    for (index, staged_figure) in figures.iter().enumerate() {
        if seats.get(index).is_none_or(|seat| !seat.present) {
            continue;
        }
        let (Some(figure), Some(slot)) = (
            staged_figure.as_ref(),
            waiting_seat_slot_rect(placements, index),
        ) else {
            continue;
        };
        paint_figure_in_seat_slot(frame, figure, &slot, now_ms);
    }
}

/// Seat-cover control ids in seat order (`COVERAVATAR1..8`, codes
/// `0x1650003D..44` → `.bnd` rects 60..67): each cover rect positions
/// its own seat, anchor-free bodies included (the placement lesson).
const WAITING_COVER_IDS: [&str; 8] = [
    "IDC_IMAGE_SWAIT_COVERAVATAR1",
    "IDC_IMAGE_SWAIT_COVERAVATAR2",
    "IDC_IMAGE_SWAIT_COVERAVATAR3",
    "IDC_IMAGE_SWAIT_COVERAVATAR4",
    "IDC_IMAGE_SWAIT_COVERAVATAR5",
    "IDC_IMAGE_SWAIT_COVERAVATAR6",
    "IDC_IMAGE_SWAIT_COVERAVATAR7",
    "IDC_IMAGE_SWAIT_COVERAVATAR8",
];
/// Team-button control ids in team order (`TEAMA..H`, codes
/// `0x16201004..0B` → `.bnd` rects 3..10).
pub(crate) const WAITING_TEAM_IDS: [&str; 8] = [
    "IDC_BUTTON_SWAIT_TEAMA",
    "IDC_BUTTON_SWAIT_TEAMB",
    "IDC_BUTTON_SWAIT_TEAMC",
    "IDC_BUTTON_SWAIT_TEAMD",
    "IDC_BUTTON_SWAIT_TEAME",
    "IDC_BUTTON_SWAIT_TEAMF",
    "IDC_BUTTON_SWAIT_TEAMG",
    "IDC_BUTTON_SWAIT_TEAMH",
];
/// Room-title text (`TEXT_SWAIT_ROOMNAME`, code `0x16400016` → rect 21).
pub(crate) const WAITING_ROOMNAME_ID: &str = "IDC_TEXT_SWAIT_ROOMNAME";
/// Selected-song text (`TEXT_SWAIT_MUSICNAME`, code `0x16400017` →
/// rect 22, top bar beside the title).
pub(crate) const WAITING_MUSICNAME_ID: &str = "IDC_TEXT_SWAIT_MUSICNAME";
/// First ink pixel inside a bound Waiting caption rect, in native
/// pixels. 7.55 Wine capture `~/scratch/ab-wait.png`: the `MUSICNAME`
/// rect starts `(573,15)` and its `디젤오 (O2Ver.1)[3K]` ink starts
/// `(579,22)`. The rect is still the `.bnd` pin; this is only the
/// GDI cell bearing our rasterizer must add so bare caption ink lands
/// where the exe's glyphs do (no shell scrim).
const WAITING_CAPTION_INK_BEARING: (i32, i32) = (6, 7);
/// Difficulty/speed badge art (`LEVEL`, code `0x1650001B` → rect 26
/// `(218,12)-(273,40)`, immediately left of the room title).
pub(crate) const WAITING_LEVEL_ID: &str = "IDC_IMAGE_SWAIT_LEVEL";
/// Champion win-count text (`TEXT_SWAIT_WINNERSCROE`, code `0x1640006D`
/// → rect 108, the Wins box inside the champion card).
pub(crate) const WAITING_WINNERSCORE_ID: &str = "IDC_TEXT_SWAIT_WINNERSCROE";
/// Scroll-speed value art (`SPEED_NUM`, code `0x16500066` → rect 101).
pub(crate) const WAITING_SPEED_NUM_ID: &str = "IDC_IMAGE_SWAIT_SPEED_NUM";
/// Skill-slot art ids top to bottom (`1/2/3SKILL`, codes
/// `0x16500058..5A` → rects 87..89, the X boxes of the SKILL panel).
pub(crate) const WAITING_SKILL_IDS: [&str; 3] = [
    "IDC_IMAGE_SWAIT_1SKILL",
    "IDC_IMAGE_SWAIT_2SKILL",
    "IDC_IMAGE_SWAIT_3SKILL",
];
/// Map-name text (`TEXT_SWAIT_MAPNAME`, code `0x16400018` →
/// rect 23: `(code & 0xFFF) − 1`, beside the title).
pub(crate) const WAITING_MAPNAME_ID: &str = "IDC_TEXT_SWAIT_MAPNAME";
/// BPM text (`TEXT_SWAIT_BPM`, code `0x16400069` → rect 104).
pub(crate) const WAITING_BPM_ID: &str = "IDC_TEXT_SWAIT_BPM";
/// Skill-count texts (`TEXT_SWAIT_1/2/3SKILLCOUNT`, codes
/// `0x1640005E..60` → rects 93..95, beside the X boxes).
pub(crate) const WAITING_SKILLCOUNT_IDS: [&str; 3] = [
    "IDC_TEXT_SWAIT_1SKILLCOUNT",
    "IDC_TEXT_SWAIT_2SKILLCOUNT",
    "IDC_TEXT_SWAIT_3SKILLCOUNT",
];

/// One staged seat view: the wire member plus the champion mark; the
/// self seat carries no level (the wire sends no self nickname or
/// level — `self_seat` documents the local-only name).
pub(crate) struct StagedSeat {
    pub(crate) name: String,
    pub(crate) level: Option<i32>,
    pub(crate) team: u8,
    pub(crate) ready: bool,
    pub(crate) present: bool,
    pub(crate) locked: bool,
    pub(crate) champion: bool,
    /// `RoomMember.master`: the room creator's seat.
    pub(crate) master: bool,
    /// `RoomMember.playing_state` nonzero (the member is playing).
    pub(crate) playing: bool,
    /// The room names a song (`JoinedRoom.music_id` nonzero).
    pub(crate) has_song: bool,
}

/// Seat parents of the `SET 0x0C 0x00015000` seat template, in seat
/// order: `IDC_BUTTON_SWAIT_EXTEND1..8` (`0x16015022..29`) carry the
/// eight seat rects, and each one hosts one clone of the template's
/// 12 members (`docs/ui-loader.md` step 8: the group key is
/// `code & 0xFF000` = `0x15000`).
pub(crate) const WAITING_SEAT_PARENT_IDS: [&str; WAITING_SEATS] = [
    "IDC_BUTTON_SWAIT_EXTEND1",
    "IDC_BUTTON_SWAIT_EXTEND2",
    "IDC_BUTTON_SWAIT_EXTEND3",
    "IDC_BUTTON_SWAIT_EXTEND4",
    "IDC_BUTTON_SWAIT_EXTEND5",
    "IDC_BUTTON_SWAIT_EXTEND6",
    "IDC_BUTTON_SWAIT_EXTEND7",
    "IDC_BUTTON_SWAIT_EXTEND8",
];

/// A seat indicator body of the seat template and the live flag that
/// arms it. Bodies with no pinned flag stay dark (see
/// [`seat_indicator`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeatIndicator {
    /// `Wait_ReadyWindow.ojs`: the seat's name plate, one frame per
    /// team colour (eight layers = teams A–H).
    TeamPlate,
    /// `Wait_BossMark.ojs`: the master's star.
    MasterStar,
    /// `Wait_ReadyText.ojs`: the ready plate.
    ReadyPlate,
    /// `Wait_NoMusicImage.ojs`: the room names no song yet.
    NoSong,
    /// `Wait_Playing.ojs`: the member is playing.
    Playing,
}

impl SeatIndicator {
    /// Body name of this indicator.
    const fn sprite(self) -> &'static str {
        match self {
            SeatIndicator::TeamPlate => "Wait_ReadyWindow.ojs",
            SeatIndicator::MasterStar => "Wait_BossMark.ojs",
            SeatIndicator::ReadyPlate => "Wait_ReadyText.ojs",
            SeatIndicator::NoSong => "Wait_NoMusicImage.ojs",
            SeatIndicator::Playing => "Wait_Playing.ojs",
        }
    }

    /// Icon-control frame for this seat (frame *n* paints layer *n−1*;
    /// the setter clamps, see [`icon_frame_layer`]).
    fn frame(self, seat: &StagedSeat) -> Option<u8> {
        if !seat.present {
            return None;
        }
        match self {
            SeatIndicator::TeamPlate => Some(seat.team.saturating_add(1)),
            SeatIndicator::MasterStar => seat.master.then_some(1),
            SeatIndicator::ReadyPlate => seat.ready.then_some(1),
            SeatIndicator::NoSong => (!seat.has_song).then_some(1),
            SeatIndicator::Playing => seat.playing.then_some(1),
        }
    }
}

/// Indicator body behind one seat-template sprite name. Bodies with no
/// pinned flag return `None` and stay dark:
///
/// - `Wait_PCRoomMark.ojs` — PC-room badge; the wire carries no
///   matching field (OPEN: find the flag the exe reads).
/// - `Wait_Downloding.ojs` / `Wait_Downloding_Bar.ojs` — song download
///   progress; the shell models no download (OPEN).
/// - `Waiting_P2P_Image.ojs` — P2P versus banner; needs the room's
///   venue class (OPEN).
/// - `Wait_NoGemImage.ojs` — gem-less marker; `RoomMember.gem` exists,
///   but which value arms the plate is untraced (OPEN).
fn seat_indicator(sprite: &str) -> Option<SeatIndicator> {
    [
        SeatIndicator::TeamPlate,
        SeatIndicator::MasterStar,
        SeatIndicator::ReadyPlate,
        SeatIndicator::NoSong,
        SeatIndicator::Playing,
    ]
    .into_iter()
    .find(|indicator| indicator.sprite() == sprite)
}

/// Seat index owning one staged clone (`None` for other placements).
fn seat_clone_index(placed: &ScenePlacement) -> Option<usize> {
    if placed.role != PlacementRole::Clone {
        return None;
    }
    WAITING_SEAT_PARENT_IDS
        .iter()
        .position(|id| *id == placed.id)
}

/// Whether one seat-template clone reaches the frame: occupied seats
/// show the armed indicators, vacant and locked seats keep their bare
/// cover plate (the exe clones the template per seated player only).
fn waiting_seat_clone_live(placed: &ScenePlacement, seats: &[StagedSeat]) -> bool {
    let Some(index) = seat_clone_index(placed) else {
        return true;
    };
    let Some(seat) = seats.get(index) else {
        return false;
    };
    if !seat.present {
        return false;
    }
    let Some(indicator) = placed.sprite.as_deref().and_then(seat_indicator) else {
        return false;
    };
    indicator.frame(seat).is_some()
}

/// Paint every armed seat indicator at its clone rect with the seat's
/// frame. The clone base pass already painted frame 0 there, so this
/// pass owns the live frame; unarmed bodies were filtered out above.
fn paint_waiting_seat_marks(frame: &mut Frame<'_>, seats: &[StagedSeat], art: &SceneArt) {
    for placed in &art.placements {
        let Some(index) = seat_clone_index(placed) else {
            continue;
        };
        let Some(seat) = seats.get(index) else {
            continue;
        };
        let Some(indicator) = placed.sprite.as_deref().and_then(seat_indicator) else {
            continue;
        };
        let (Some(figure_name), Some(icon_frame)) =
            (placed.sprite.as_deref(), indicator.frame(seat))
        else {
            continue;
        };
        let Some(figure) = art.figures.get(figure_name) else {
            continue;
        };
        let Some(layer) = icon_frame_layer(icon_frame, figure.len()).and_then(|i| figure.layer(i))
        else {
            continue;
        };
        let dest = scale_rect_to_frame(&placed.rect, frame.w, frame.h);
        blit_rgb555(
            frame,
            &layer.image.view(),
            dest.x + layer.dx,
            dest.y + layer.dy,
            Some(layer.colorkey),
        );
    }
}

/// Staged seats in cover order: self first, then the seven wire slots
/// (`JoinedRoom.slots`), padded empty — the same eight seats as
/// [`waiting_seats`], with levels and the champion mark kept.
pub(crate) fn staged_seats(room: &JoinedRoom) -> Vec<StagedSeat> {
    let mut seats = Vec::with_capacity(WAITING_SEATS);
    let own = room.slots.iter().find(|slot| slot.index == room.member_id);
    seats.push(StagedSeat {
        name: "YOU".to_owned(),
        level: None,
        team: room.team,
        ready: own
            .and_then(|slot| slot.member.as_ref())
            .is_some_and(|member| member.ready),
        present: true,
        locked: false,
        champion: room.champion == Some(0),
        master: room.member_id == 0,
        playing: own
            .and_then(|slot| slot.member.as_ref())
            .is_some_and(|member| member.playing_state != 0),
        has_song: room.music_id != 0,
    });
    for (offset, slot) in room.slots.iter().take(WAITING_PEERS).enumerate() {
        let index = u8::try_from(offset + 1).unwrap_or(u8::MAX);
        seats.push(staged_peer_seat(
            slot,
            slot.index != room.member_id,
            room.champion == Some(index),
        ));
    }
    while seats.len() < WAITING_SEATS {
        seats.push(empty_staged_seat());
    }
    seats
}

/// Staged view of one wire slot. The local member already has seat 0;
/// its wire slot is also present in `slots`, and a duplicate master
/// flag there would paint the creator emblem on a second seat.
fn staged_peer_seat(slot: &RoomSlot, is_peer: bool, champion: bool) -> StagedSeat {
    match &slot.member {
        Some(member) => StagedSeat {
            name: member.nickname.clone(),
            level: Some(member.level),
            team: member.team,
            ready: member.ready,
            present: true,
            locked: false,
            champion,
            master: is_peer && member.master,
            playing: member.playing_state != 0,
            has_song: !member.music_ids.is_empty(),
        },
        None if slot.state_code == RoomSlotState::Locked as i32 => StagedSeat {
            name: String::new(),
            level: None,
            team: 0,
            ready: false,
            present: false,
            locked: true,
            champion: false,
            master: false,
            playing: false,
            has_song: false,
        },
        None => empty_staged_seat(),
    }
}

/// Vacant staged seat.
fn empty_staged_seat() -> StagedSeat {
    StagedSeat {
        name: String::new(),
        level: None,
        team: 0,
        ready: false,
        present: false,
        locked: false,
        champion: false,
        master: false,
        playing: false,
        has_song: false,
    }
}

/// Role prefix of one seat: the champion seat names `CHAMPION`, every
/// other occupied seat `CHALLENGER` (the reference 1v1 pair); rooms
/// without a champion mark name no roles (normal rooms carry none).
pub(crate) fn staged_role(has_champion: bool, seat: &StagedSeat) -> Option<&'static str> {
    if !has_champion || !seat.present {
        return None;
    }
    Some(if seat.champion {
        "CHAMPION"
    } else {
        "CHALLENGER"
    })
}

/// One staged seat line: role plus team plus `Lv` (unknown levels
/// show `-`: the self seat carries no wire level) plus name plus the
/// ready flag — the legacy row shape, role-prefixed.
pub(crate) fn staged_seat_text(seat: &StagedSeat, role: Option<&str>) -> String {
    let flag = if seat.ready { 'R' } else { '-' };
    let level = seat.level.map_or("-".to_owned(), |level| level.to_string());
    let team = team_letter(seat.team);
    match role {
        Some(role) => format!("{role} {team} Lv:{level} {} {flag}", seat.name),
        None => format!("{team} Lv:{level} {} {flag}", seat.name),
    }
}

/// Scaled frame origin of one waiting control (`None` while the staged
/// art names no such control — the exe leaves untouched icons alone).
pub(crate) fn waiting_bnd_origin(
    placements: &[ScenePlacement],
    id: &str,
    frame: &Frame<'_>,
) -> Option<(i32, i32)> {
    let placement = placements.iter().find(|placed| placed.id == id)?;
    let scaled = scale_rect_to_frame(&placement.rect, frame.w, frame.h);
    Some((scaled.x, scaled.y))
}

/// Scaled origin of a caption's first ink pixel: its bound rect origin
/// plus the reference-measured GDI cell bearing
/// ([`WAITING_CAPTION_INK_BEARING`]), both in frame pixels.
fn waiting_caption_ink_origin(
    placements: &[ScenePlacement],
    id: &str,
    frame: &Frame<'_>,
) -> Option<(i32, i32)> {
    let (rect_x, rect_y) = waiting_bnd_origin(placements, id, frame)?;
    let bearing = scale_rect_to_frame(
        &Rect {
            x: 0,
            y: 0,
            w: u32::try_from(WAITING_CAPTION_INK_BEARING.0).unwrap_or(0),
            h: u32::try_from(WAITING_CAPTION_INK_BEARING.1).unwrap_or(0),
        },
        frame.w,
        frame.h,
    );
    Some((
        rect_x.saturating_add(i32::try_from(bearing.w).unwrap_or(0)),
        rect_y.saturating_add(i32::try_from(bearing.h).unwrap_or(0)),
    ))
}

/// Ink one staged caption without any shell plate: the exe's captions
/// are transparent TEXT controls over their own art, so only glyphs
/// land in the rect ([`waiting_caption_ink_origin`]).
fn paint_waiting_bare_caption(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    id: &str,
    text: &str,
) {
    if let Some((x, y)) = waiting_caption_ink_origin(placements, id, frame) {
        draw_text(frame, x, y, text, art_title_ink());
    }
}

/// True while one cover binds the staged cover body (or a
/// sprite-less slot, as in synthetic probes): any other body on the
/// cover id stays untouched, like the exe's unmatched icons.
fn cover_stages_body(placements: &[ScenePlacement], cover_id: &str) -> bool {
    placements
        .iter()
        .find(|placed| placed.id == cover_id)
        .is_none_or(|placed| {
            placed
                .sprite
                .as_deref()
                .is_none_or(|sprite| sprite == WAIT_COVER_SPRITE)
        })
}

/// True while staged art binds at least one seat cover: the staged
/// path anchors to verbatim `.bnd` rects, otherwise the legacy grid
/// keeps the bare scene readable.
fn waiting_has_staged_covers(placements: &[ScenePlacement]) -> bool {
    WAITING_COVER_IDS
        .iter()
        .any(|id| placements.iter().any(|placed| &placed.id == id))
}

/// Staged waiting regions, each at its verbatim `.bnd` rect: title,
/// the difficulty/speed badge, seat covers, team highlight, wins,
/// speed, skills, skill counts, map name, BPM, and song (the song
/// caption resolves through the staged catalog, `M{id}` while it
/// names no such song; the BPM caption resolves through its BPM
/// column, `-0-` on mismatch).
/// `INSTNAME` (`TEXT_SWAIT_INSTNAME`, `0x16400019`) stays unpainted:
/// zero code immediates name it (OPEN 1), so no shell source reaches
/// its rect. `NOTICE_TEXT`/`NOTICEBOARD` (`0x16500062/63`) stay
/// unpainted: the text body is imageless and its dialog `+0x46c`
/// fill is untraced (OPEN 2).
pub(crate) fn paint_waiting_staged(
    frame: &mut Frame<'_>,
    room: &JoinedRoom,
    art: &SceneArt,
    captions: &WaitingCaptions<'_>,
) {
    let placements = &art.placements;
    match captions.notice {
        Some(notice) => paint_waiting_top_line(frame, notice, placements),
        None => paint_waiting_title(frame, room, placements),
    }
    paint_waiting_level_badge(frame, captions.level, art);
    paint_waiting_covers(frame, room, placements);
    paint_waiting_team_highlight(frame, room, placements);
    paint_waiting_wins(frame, room, placements);
    paint_waiting_speed(frame, room, placements);
    paint_waiting_skills(frame, room, placements);
    paint_waiting_skillcounts(frame, &captions.skill_counts, placements);
    paint_waiting_mapname(frame, room, placements);
    paint_waiting_bpm(frame, captions.song_bpm, placements);
    paint_waiting_music(frame, placements, captions.song);
}

/// Room title at the `ROOMNAME` rect (top bar, rect 21).
pub(crate) fn paint_waiting_title(
    frame: &mut Frame<'_>,
    room: &JoinedRoom,
    placements: &[ScenePlacement],
) {
    paint_waiting_top_line(frame, &room.title, placements);
}

/// Text on the `ROOMNAME` line: the room title, or the standing notice
/// (the START gate's refusal, `docs/music-availability.md` §4a). The
/// exe sets that notice on its own box above the START control; the box
/// sheet is not pinned, so the pinned top-bar line stands in — the same
/// stand-in shape the song dialog's notice uses for its status line.
fn paint_waiting_top_line(frame: &mut Frame<'_>, text: &str, placements: &[ScenePlacement]) {
    if let Some((x, y)) = waiting_bnd_origin(placements, WAITING_ROOMNAME_ID, frame) {
        paint_room_detail_line(frame, x, y, text, art_title_ink());
    }
}

/// Speed cells per difficulty family in `Wait_Level.ojs` — the
/// `GameSpeed` chain 0..11 (`0.5`, `1`, `1.5`, ..., `6`, `R`), pinned
/// by the exe's `12*difficulty` term (file `0x4b33f4`) and by the
/// 60-frame body (5 families x 12 cells).
const WAITING_LEVEL_SPEEDS: usize = 12;

/// `Wait_Level.ojs` frame the badge paints: `12*difficulty + speed`.
/// The exe stages `index + 1` and setter VA `0x42f870` subtracts the
/// one before its image lookup, so the staged layer is the plain
/// product (files `0x4b33f4`-`0x4b3442`, `0x4b805a`-`0x4b80a0`).
/// A speed code past the 12 cells names no cell: `None` paints
/// nothing rather than a neighbouring speed, and a difficulty family
/// past the staged body stays empty for the same reason.
#[must_use]
pub(crate) fn waiting_level_layer(level: WaitingLevel) -> Option<usize> {
    let cell = usize::from(level.speed);
    (cell < WAITING_LEVEL_SPEEDS)
        .then(|| usize::from(level.difficulty) * WAITING_LEVEL_SPEEDS + cell)
}

/// Difficulty/speed badge at the `LEVEL` rect (rect 26
/// `(218,12)-(273,40)`, immediately left of the room title): one
/// `Wait_Level.ojs` frame carrying the family letter and the speed
/// suffix (`EX1`, `NX2`, `HX5`; EX green, NX orange-yellow, HX red —
/// the body's own colours). The exe resets the control to no image
/// when no song stands (file `0x4ba88d`, setter arg 0), so a missing
/// level source paints nothing; `INSTNAME`-style stand-ins never
/// reach this rect.
///
/// Two exe rules stay OPEN and never paint here: the MX family shift
/// for an EX chart above level 3 (`0x4b3372`-`0x4b33b7`, `+0x30`;
/// the shell stages no per-chart level) and the AX family for a
/// flagged song id (`0x7789d8 & 0xf0000000`, writer untraced). Both
/// are recorded in `docs/caption-sources.md`.
fn paint_waiting_level_badge(frame: &mut Frame<'_>, level: Option<WaitingLevel>, art: &SceneArt) {
    let Some(level) = level else {
        return;
    };
    let Some(layer) = waiting_level_layer(level) else {
        return;
    };
    let Some(placed) = art
        .placements
        .iter()
        .find(|placed| placed.id == WAITING_LEVEL_ID)
    else {
        return;
    };
    let shared = shared_sprite_names(&art.placements);
    let shares_body = placed
        .sprite
        .as_deref()
        .is_some_and(|sprite| shared.contains(sprite));
    paint_placement_layer(frame, placed, &art.figures, layer, shares_body, None);
}

/// Seat lines at the cover rects (rects 60..67): champion names its
/// role and wins box, the challenger its role; empty covers stay
/// untouched so the staged empty panels show (reference bottom row).
/// Locked covers name `LOCK` (their art carries no lock body).
pub(crate) fn paint_waiting_covers(
    frame: &mut Frame<'_>,
    room: &JoinedRoom,
    placements: &[ScenePlacement],
) {
    let has_champion = room.champion.is_some();
    for (cover_id, seat) in WAITING_COVER_IDS.iter().zip(staged_seats(room)) {
        if !cover_stages_body(placements, cover_id) {
            continue;
        }
        let Some((x, y)) = waiting_bnd_origin(placements, cover_id, frame) else {
            continue;
        };
        if seat.locked {
            paint_room_detail_line(frame, x, y, "LOCK", art_title_ink());
        } else if seat.present {
            let role = staged_role(has_champion, &seat);
            paint_room_detail_line(frame, x, y, &staged_seat_text(&seat, role), art_title_ink());
            paint_cover_ready_dot(frame, x, y, seat.ready);
        }
    }
}

/// Ready dot beside one cover line (the `Wait_ReadyText.ojs` plate is
/// staged art; the dot is the live flag, same inks as the lobby).
fn paint_cover_ready_dot(frame: &mut Frame<'_>, x: i32, y: i32, ready: bool) {
    fill_rect(
        frame,
        &Rect {
            x: x - 6,
            y,
            w: 3,
            h: 5,
        },
        ready_dot_color(ready),
    );
}

/// Team highlight at our team button rect (rects 3..10): the exe
/// highlight arm (`FUN_0048c810`) lights exactly the pressed button;
/// the amber bar is the live mark over the staged `Wait_Team?.ojs`.
pub(crate) fn paint_waiting_team_highlight(
    frame: &mut Frame<'_>,
    room: &JoinedRoom,
    placements: &[ScenePlacement],
) {
    let index = usize::from(room.team);
    let (Some(button), Some(expected)) =
        (WAITING_TEAM_IDS.get(index), WAIT_TEAM_SPRITES.get(index))
    else {
        return;
    };
    let staged = placements.iter().find(|placed| &placed.id == button);
    if staged.is_none_or(|placed| placed.sprite.as_deref() == Some(*expected)) {
        if let Some((x, y)) = waiting_bnd_origin(placements, button, frame) {
            paint_cursor(frame, x + 6, y);
        }
    }
}

/// Champion win count at the `WINNERSCORE` rect (rect 108, the Wins
/// box): the wire streak only (`JoinedRoom.champion_streak`); past
/// it nothing paints (per-member tallies stay server-side).
pub(crate) fn paint_waiting_wins(
    frame: &mut Frame<'_>,
    room: &JoinedRoom,
    placements: &[ScenePlacement],
) {
    let Some(streak) = room.champion_streak else {
        return;
    };
    if room.champion.is_none() {
        return;
    }
    if let Some((x, y)) = waiting_bnd_origin(placements, WAITING_WINNERSCORE_ID, frame) {
        paint_room_detail_line(frame, x, y, &streak.to_string(), art_title_ink());
    }
}

/// Scroll-speed label at the `SPEED_NUM` rect (rect 101): the shared
/// `X0.5`..`XR` chain (`room-enums.md`, codes 0..11). The exe's chip
/// art carries its own plate, so the caption inks bare glyphs at the
/// reference bearing — no black box over the chip.
pub(crate) fn paint_waiting_speed(
    frame: &mut Frame<'_>,
    room: &JoinedRoom,
    placements: &[ScenePlacement],
) {
    paint_waiting_bare_caption(
        frame,
        placements,
        WAITING_SPEED_NUM_ID,
        &room_speed_label(room.speed),
    );
}

/// Skill-ring ids at the `1/2/3SKILL` rects (rects 87..89, the X
/// boxes): empty slots stay untouched so the staged X art shows.
/// The counts paint beside them at the `SKILLCOUNT` rects from the
/// staged inventory ([`paint_waiting_skillcounts`]).
pub(crate) fn paint_waiting_skills(
    frame: &mut Frame<'_>,
    room: &JoinedRoom,
    placements: &[ScenePlacement],
) {
    for (slot, skill_id) in WAITING_SKILL_IDS.iter().zip(room.skills.iter()) {
        if let Some((x, y)) = waiting_bnd_origin(placements, slot, frame) {
            paint_room_detail_line(frame, x, y, &skill_id.to_string(), art_title_ink());
        }
    }
}

/// Map-cycle names behind the `MAPNAME` caption, in room-word order
/// (map-cycle path, rdata file `0x249990`): the exe centers entry
/// `low == 0 ? 3 : low % 4` of (`None`, `한가한대전장의오후`,
/// `패자의역습`, `원하지않은결말`). The CP949-sourced Korean entries
/// paint their real glyphs now that the embedded Noto Sans KR subset
/// carries Hangul (formerly ASCII stand-ins — the micro-font had no
/// CP949 glyphs). The `0x4ad350` applier `Random`/`NoteFactory` arm
/// stays OPEN (`docs/caption-sources.md` OPEN 3) and never paints.
const WAITING_MAP_CYCLE_NAMES: [&str; 4] =
    ["None", "한가한대전장의오후", "패자의역습", "원하지않은결말"];

/// Map-cycle slot for one room word (files `0xb3cc6`–`0xb3ce3`):
/// the low word, with word 0 selecting slot 3. Byte access keeps
/// the low word without a narrowing cast (PERF-07).
#[must_use]
fn waiting_map_cycle_slot(arena: i32) -> usize {
    let bytes = arena.to_le_bytes();
    let low = u16::from_le_bytes([bytes[0], bytes[1]]);
    if low == 0 {
        WAITING_MAP_CYCLE_NAMES.len() - 1
    } else {
        usize::from(low) % WAITING_MAP_CYCLE_NAMES.len()
    }
}

/// `MAPNAME` caption for one room word: the map-cycle table entry
/// (pinned path only).
#[must_use]
pub fn waiting_map_name(arena: i32) -> &'static str {
    WAITING_MAP_CYCLE_NAMES
        .get(waiting_map_cycle_slot(arena))
        .copied()
        .unwrap_or(WAITING_MAP_CYCLE_NAMES[0])
}

/// Map name at the `MAPNAME` rect: the map-cycle entry of the room
/// word. An unstaged rect stays untouched.
pub(crate) fn paint_waiting_mapname(
    frame: &mut Frame<'_>,
    room: &JoinedRoom,
    placements: &[ScenePlacement],
) {
    if let Some((x, y)) = waiting_bnd_origin(placements, WAITING_MAPNAME_ID, frame) {
        paint_room_detail_line(frame, x, y, waiting_map_name(room.arena), art_title_ink());
    }
}

/// Mismatch BPM caption (rdata file `0x24a1d0`): the exe stores
/// `-0-` when no song record matches the room song.
const WAITING_BPM_MISMATCH: &str = "-0-";

/// `BPM` caption for one staged catalog lookup: the `OJNList.dat`
/// row +20 BPM column formatted with `%0.f` (rdata file `0x24a1d4`,
/// store file `0xbe1d5`); a missing or non-finite lookup falls back
/// to the pinned mismatch default, never an invented tempo.
#[must_use]
pub fn waiting_bpm_caption(song_bpm: Option<f32>) -> String {
    match song_bpm {
        Some(bpm) if bpm.is_finite() => format!("{bpm:.0}"),
        _ => WAITING_BPM_MISMATCH.to_owned(),
    }
}

/// BPM at the `BPM` rect: the catalog column or `-0-`. An unstaged
/// rect stays untouched.
pub(crate) fn paint_waiting_bpm(
    frame: &mut Frame<'_>,
    song_bpm: Option<f32>,
    placements: &[ScenePlacement],
) {
    if let Some((x, y)) = waiting_bnd_origin(placements, WAITING_BPM_ID, frame) {
        paint_room_detail_line(frame, x, y, &waiting_bpm_caption(song_bpm), art_title_ink());
    }
}

/// `SKILLCOUNT` caption for one inventory slot: the count word at
/// owned-item vector `+8` as decimal, space-padded to width 3
/// (helper `0x62c109`); slots 2–3 additionally clamp at 999, so
/// wider counts show `999` there.
#[must_use]
pub fn waiting_skillcount_text(count: u32, cap_at_999: bool) -> String {
    let shown = if cap_at_999 && count > 999 {
        999
    } else {
        count
    };
    format!("{shown:>3}")
}

/// Skill counts at the `1/2/3SKILLCOUNT` rects: the staged inventory
/// counts beside the rings (slot 1 uncapped, slots 2–3 capped at
/// 999). An unstaged rect stays untouched.
pub(crate) fn paint_waiting_skillcounts(
    frame: &mut Frame<'_>,
    counts: &[u32; 3],
    placements: &[ScenePlacement],
) {
    for (slot, (id, count)) in WAITING_SKILLCOUNT_IDS.iter().zip(counts.iter()).enumerate() {
        if let Some((x, y)) = waiting_bnd_origin(placements, id, frame) {
            paint_room_detail_line(
                frame,
                x,
                y,
                &waiting_skillcount_text(*count, slot > 0),
                art_title_ink(),
            );
        }
    }
}

/// Selected-song label at the `MUSICNAME` rect (rect 22, top bar):
/// the staged catalog title for the caption's song when it names one
/// (room-fill string flow, `docs/channel-fill.md`), else the bare
/// song id (the song table lives server-side, as in the lobby). The
/// caption names the master's staged pick at once and the joined
/// room's song otherwise ([`WaitingSong`]); ticket #274 removed the
/// shell-invented readout panel that used to ink this data above the
/// chat rows. Its remaining black scrim is gone too: the title now
/// rides the staged plate as bare glyphs at the reference bearing
/// ([`paint_waiting_bare_caption`]).
/// The `COVERMUSIC` button, the select arrow, the `Random` cards, and
/// BACK stay staged art — recorded negatives, not gaps (chat now has a
/// live source: [`paint_waiting_chat`]):
///
/// - Random cards: the 117-entry `STATE_WAITING` block names no per-card
///   entries; the big `?` card sits on `RANDMAP` (`0x1650004F` → rect 78)
///   while the small `?` cards sit on the spriteless `EFFECT_1/2` rects
///   (68/69) — caption and frames baked into the staged backdrop.
///   `SelectMusic_Random*.ojs` bodies belong to `DAILOG_SELECTMUSIC`
///   filter buttons, not to waiting cards, and no wire struct carries
///   card picks — so the painter stages art only.
pub(crate) fn paint_waiting_music(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    song: Option<WaitingSong<'_>>,
) {
    let Some(song) = song else {
        return;
    };
    paint_waiting_bare_caption(
        frame,
        placements,
        WAITING_MUSICNAME_ID,
        &waiting_music_caption(song.music_id, song.title),
    );
}

/// `MUSICNAME` caption for one room song: the staged catalog title
/// when it names the song, else the bare `M{id}` id (never an
/// invented title — `INSTNAME` stays unpainted for the same reason:
/// no staged source names it).
#[must_use]
pub(crate) fn waiting_music_caption(music_id: u16, song_title: Option<&str>) -> String {
    match song_title {
        Some(title) if !title.is_empty() => title.to_owned(),
        _ => room_music_label(music_id),
    }
}

/// Dark strip behind one seat row so seat ink stays readable over
/// staged art (vacant rows get one too: the placeholder must read).
fn paint_seat_scrim(frame: &mut Frame<'_>, x: i32, y: i32, text: &str) {
    let text_w = text_width(text) as i32;
    fill_rect(
        frame,
        &Rect {
            x: x - 8,
            y: y - 1,
            w: (text_w + 12).max(0) as u32,
            h: 7,
        },
        scrim_color(),
    );
}

/// Paint one seat row plus its ready dot (vacant seats have no dot).
fn paint_waiting_seat(frame: &mut Frame<'_>, index: usize, seat: &WaitingSeat) {
    let occupied = seat.display_name().is_some();
    let (x, y) = seat_origin(index, frame.w, frame.h);
    let row = waiting_row_text(index, seat);
    paint_seat_scrim(frame, x, y, &row);
    let ink = if occupied {
        pack_xrgb(0xFF, 0xFF, 0xFF)
    } else {
        pack_xrgb(0x70, 0x70, 0x70)
    };
    draw_text(frame, x, y, &row, ink);
    if occupied {
        fill_rect(
            frame,
            &Rect {
                x: x - 6,
                y,
                w: 3,
                h: 5,
            },
            ready_dot_color(seat.is_ready()),
        );
    }
}

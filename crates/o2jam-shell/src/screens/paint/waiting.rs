//! Waiting-room paint generation: seats, staged covers, captions,
//! badge, role-owned confirm face, and the flat fallback.
//!
//! Kept behind re-exports in `super::paint`, so callers and the
//! existing scenario guards stay unchanged (#139 paint split).

use super::*;

/// Seats shown on the waiting screen: self plus the seven wire slots.
pub(crate) const WAITING_SEATS: usize = 8;
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
/// Wire slot `i` owns cover `i`; the local member owns cover
/// `member_id` when the 3003 snapshot has no row for it. Peers are no
/// longer shifted by one, and self is no longer forced to cover 0.
#[must_use]
pub(crate) fn waiting_seats(room: &JoinedRoom) -> Vec<WaitingSeat> {
    let mut seats: Vec<WaitingSeat> = (0..WAITING_SEATS).map(|_| empty_seat()).collect();
    for slot in room.slots.iter() {
        if let Some(seat) = seats.get_mut(usize::from(slot.index)) {
            *seat = peer_seat(slot);
        }
    }
    let own = usize::from(room.member_id);
    if seats
        .get(own)
        .is_some_and(|seat| matches!(seat.kind, SeatKind::Empty))
    {
        seats[own] = self_seat(room);
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
    /// Locally staged owned-item selectors, when a source named them.
    /// `None` keeps the ring ids from `JoinedRoom.skills` and the
    /// counts from `skill_counts` (OPEN 4: no wire fill is pinned).
    pub skill_slots: Option<&'a WaitingSkillSlots>,
    /// Difficulty and speed behind the `LEVEL` badge, from the same
    /// pick-or-room source as `song`.
    pub level: Option<WaitingLevel>,
    /// Whether the staged song id carries the exe's variant flag
    /// (`0x7789d8 & 0xf0000000`): yellow random caption plus AX badge
    /// family (`FUN_004b2e30`).
    pub random: bool,
    /// Pinned notice standing over the top-bar room line (the exe's
    /// notice box above the START control,
    /// `docs/music-availability.md` §4); `None` paints the room title.
    pub notice: Option<&'static str>,
    /// One-based channel number behind the two `CHNUM` digit wells:
    /// the resident 1003 `channel_id` plus one, staged on login.
    pub channel_number: Option<u16>,
    /// Room number behind the three `RMNUM` digit wells: the number
    /// from the accepted 3002 join request, staged on join.
    pub room_number: Option<i32>,
    /// Selected decorative effect layer (exe global `0x7789e0`).
    pub effect: WaitingEffect,
    /// Selected Waiting chat tab (exe toggle group `0x1620100E..11`).
    pub chat_channel: WaitingChatChannel,
    /// Whether either authored messenger lamp shows its alert frame.
    pub messenger_lamp_lit: bool,
    /// Whether the two WAITING stamp controls show their armed frame.
    pub wait_stamp_lit: bool,
    /// Local nickname from 2001, used on the self cover when the 3003
    /// slot list does not carry the guest's own member.
    pub local_name: Option<&'a str>,
}

/// Difficulty, speed and EX chart level behind the `LEVEL` badge:
/// the two bytes the exe's frame formula reads (`0x7789e7` difficulty,
/// `0x7789e9` speed, struct `0x7789d8`) plus the catalog word its MX
/// branch tests (record `+0x14`, `FUN_004b2e30`).

/// Captions whose exe source is still OPEN (7.55 sha256
/// `83a04541…72c823b`, `docs/caption-sources.md` OPEN 1/2). Keeping
/// the codes and next evidence steps next to the painter records the
/// negative: no shell value reaches these rects, so they stay silent
/// instead of growing an invented caption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitingOpenCaption {
    /// `IDC_TEXT_SWAIT_INSTNAME`, no code immediate in the whole image.
    InstrumentName,
    /// `IDC_IMAGE_SWAIT_NOTICE_TEXT`, imageless; its content comes from
    /// the untraced dialog `+0x46c` fill.
    Notice,
}

impl WaitingOpenCaption {
    /// Control code the caption would need a source for.
    #[must_use]
    pub const fn code(self) -> u32 {
        match self {
            Self::InstrumentName => 0x1640_0019,
            Self::Notice => 0x1650_0062,
        }
    }

    /// Next evidence step that can lift this caption out of OPEN.
    #[must_use]
    pub const fn next_evidence_step(self) -> &'static str {
        match self {
            Self::InstrumentName => concat!(
                "full-image scan finds zero immediates for 0x16400019; trace the cached ",
                "pointer/array reader or record the negative, then pin 4006 SetInstrument"
            ),
            Self::Notice => concat!(
                "read the content behind the 0x50b320 call on dialog +0x46c, file 0xbca0f, ",
                "and pin the writer that fills it"
            ),
        }
    }
}

/// Which decorative effect layer the exe's Waiting toggle selects.
/// The 7.55 init sets global `0x7789e0` to 1 (`FUN_00495110`, file
/// `0x914b`), so BOTTOM is the default. The two staged buttons
/// `WAIT_TOP_EFFECT_CODE`/`WAIT_BOTTOM_EFFECT_CODE` write 0/1
/// (`FUN_004b7c60`, files `0xb7e4e`/`0xb7e5a`) and the toggle mapper
/// lights exactly the pressed control (`FUN_0048c810`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WaitingEffect {
    /// `IDC_BUTTON_SWAIT_TOPEFFECT`: flag 0, Playing layer 1.
    Top,
    /// `IDC_BUTTON_SWAIT_BOTTOMEFFECT`: flag 1, Playing layer 2.
    #[default]
    Bottom,
}

impl WaitingEffect {
    /// Effect selected by a staged Waiting button code (`None` for
    /// every other control). Mirrors the exe's two-code comparison.
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        match code {
            WAIT_TOP_EFFECT_CODE => Some(Self::Top),
            WAIT_BOTTOM_EFFECT_CODE => Some(Self::Bottom),
            _ => None,
        }
    }

    /// Staged control code of this effect.
    #[must_use]
    pub const fn code(self) -> u32 {
        match self {
            Self::Top => WAIT_TOP_EFFECT_CODE,
            Self::Bottom => WAIT_BOTTOM_EFFECT_CODE,
        }
    }

    /// Note-effect layer in the Playing renderers (`flag + 1`, files
    /// `0x4e792b`/`0x4ee6db`/`0x4f2a9b`). The layer is a pure pin; no
    /// Playing pixel path consumes it yet.
    #[must_use]
    pub const fn note_layer(self) -> u8 {
        match self {
            Self::Top => 1,
            Self::Bottom => 2,
        }
    }
}

/// Top/bottom effect button codes from `STATE_WAITING` (`ControlList`
/// names `Wait_Btn_EffectTop/Bottom`).
pub const WAIT_TOP_EFFECT_CODE: u32 = 0x1620_1020;
pub const WAIT_BOTTOM_EFFECT_CODE: u32 = 0x1620_1021;

/// Staged control ids of the two effect toggles.
const WAIT_EFFECT_IDS: [&str; 2] = [
    "IDC_BUTTON_SWAIT_TOPEFFECT",
    "IDC_BUTTON_SWAIT_BOTTOMEFFECT",
];

/// Waiting chat tab selected by the four `STATE_WAITING` buttons
/// `IDC_BUTTON_SWAIT_CHAT_ALL/FRIEND/GUILD/WHISPER` (`0x1620100E..11`).
/// `FUN_004b7c60` arms them as one four-control toggle group
/// (`FUN_0048c810` with base `0x1620100e`, count 4) and writes zero to
/// the dialog's `+0x298` word on every tab; the WHISPER arm also zeroes
/// the 256-byte draft buffer at `+0x18c` (files `0xb7d5d`-`0xb7e29`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WaitingChatChannel {
    /// Room chat (`0x1620100E`), also the Waiting setup default.
    #[default]
    All,
    /// Friend chat (`0x1620100F`).
    Friend,
    /// Guild chat (`0x16201010`).
    Guild,
    /// Whisper chat (`0x16201011`); additionally clears the draft.
    Whisper,
}

impl WaitingChatChannel {
    /// Tab selected by one staged control code (`None` for other codes).
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        match code {
            0x1620_100E => Some(Self::All),
            0x1620_100F => Some(Self::Friend),
            0x1620_1010 => Some(Self::Guild),
            0x1620_1011 => Some(Self::Whisper),
            _ => None,
        }
    }

    /// Staged control code of this tab.
    #[must_use]
    pub const fn code(self) -> u32 {
        match self {
            Self::All => 0x1620_100E,
            Self::Friend => 0x1620_100F,
            Self::Guild => 0x1620_1010,
            Self::Whisper => 0x1620_1011,
        }
    }
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
    /// EX chart level of the named song, when the catalog stages one.
    pub ex_chart_level: Option<u16>,
}

impl WaitingLevel {
    /// Whether the named chart selects the MX family: the exe adds
    /// `0x30` only for difficulty 0 (EX) with a level word above 3
    /// (files `0x4b3372`-`0x4b33b7`). Other charts keep their family.
    #[must_use]
    pub fn uses_mx_family(self) -> bool {
        self.difficulty == 0 && self.ex_chart_level.is_some_and(|level| level > 3)
    }
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
    paint_waiting_scene_with_confirm(
        frame,
        room,
        art,
        captions,
        WaitingConfirmState::new(0, confirm_armed),
        worn_figures_staged,
        member_role,
    );
}

/// Role and press state of the local seat's Waiting confirm control.
///
/// `master_seat_id` is the `+0x30` role word's source: the creator is
/// seat 0 until a 3007 leave names another seat (docs/relay.md §10).
/// `armed` is the request standing behind the pressed record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitingConfirmState {
    master_seat_id: u8,
    armed: bool,
}

impl WaitingConfirmState {
    /// Pair the announced master seat with the local request's face.
    #[must_use]
    pub const fn new(master_seat_id: u8, armed: bool) -> Self {
        Self {
            master_seat_id,
            armed,
        }
    }
}

/// Paint one Waiting frame with the exe's announced master seat.
/// Callers without a live room use [`paint_waiting_scene`] and its
/// seat-0 default.
pub fn paint_waiting_scene_with_confirm(
    frame: &mut Frame<'_>,
    room: &JoinedRoom,
    art: &SceneArt,
    captions: &WaitingCaptions<'_>,
    confirm: WaitingConfirmState,
    worn_figures_staged: bool,
    member_role: MemberRole,
) {
    let seats = staged_seats(room, captions.local_name);
    let is_room_master = room.member_id == confirm.master_seat_id;
    paint_waiting_backdrop(frame, art, member_role);
    art.paint_backdrop_live(frame, Screen::Waiting, |placed| {
        placed.sprite.as_deref() != Some(WAITING_BACKDROP_SPRITE)
            && waiting_seat_clone_live(placed, &seats)
            && waiting_control_armed_for_role(&placed.id, is_room_master)
            && !waiting_paints_one_record(&placed.id)
            && !waiting_header_chip_hidden(&placed.id)
            && !(worn_figures_staged && placed.sprite.as_deref() == Some(WAIT_COVER_SPRITE))
            && !(member_role != MemberRole::Challenger
                && WAITING_SELECT_OVERLAY_CODES.contains(&placed.code))
    });
    // The confirm face keeps the backdrop's stacking slot: it painted in
    // the backdrop pass before it selected its own record.
    paint_waiting_confirm_face(frame, art, is_room_master, confirm.armed);
    paint_waiting_seat_marks(frame, &seats, art);
    paint_waiting_stamp(frame, art, captions.wait_stamp_lit);
    if waiting_has_staged_covers(&art.placements) {
        paint_waiting_staged(frame, room, art, captions);
    } else {
        paint_waiting_rows(frame, room);
    }
}

/// Whether a Waiting control paints one selected record instead of its
/// staged layer stack, so the rest stack leaves it out: the `LEVEL`
/// badge ([`paint_waiting_level_badge`]), the role-owned confirm
/// body ([`paint_waiting_confirm_face`]), and the five top-bar number
/// wells ([`paint_waiting_header_numbers`]). The exe's image control
/// draws the one frame its setter staged (`docs/ui-draw.md` steps 3-6;
/// a frame swap never stacks frames), so stacking the badge's 60
/// frames would paint the last one (`MXR`), stacking the confirm body's
/// 3 frames would paint its armed face forever, and stacking
/// `Wait_RoomNumber`'s ten glyphs would show `99`/`999`.
fn waiting_paints_one_record(id: &str) -> bool {
    id == WAITING_LEVEL_ID
        || id == waiting_confirm_control_id(true)
        || id == waiting_confirm_control_id(false)
        || WAITING_CHANNEL_NUMBER_IDS.contains(&id)
        || WAITING_ROOM_NUMBER_IDS.contains(&id)
        || WAIT_EFFECT_IDS.contains(&id)
        || WAIT_CHAT_TAB_IDS.contains(&id)
        || WAIT_MESSENGER_LAMP_IDS.contains(&id)
        || WAIT_STAMP_IDS.contains(&id)
}

/// Header class chips have no staged wire source. The p343 matched exe
/// base frame leaves both wells at the sheet's baked art; stacking the
/// `Wait_Premium.ojs` and `Wait_FreeUser.ojs` layers paints captions
/// the exe does not show. No field arms them in the 3003 snapshot, so
/// they stay out until a pinned source stages one.
fn waiting_header_chip_hidden(id: &str) -> bool {
    matches!(id, "IDC_IMAGE_SWAIT_SKILLSHOW" | "IDC_IMAGE_SWAIT_FREEUSER")
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
///
/// #765 extends the gate with the rest of the statically pinned set
/// (`FUN_004b4a00` evidence `ev_c5b7cdce…`, `FUN_004b4d30` evidence
/// `ev_1365b919…`): master arms MAPLEFT/RIGHT `0x1620000C/0D` and the
/// six `1/2/3SKILLLEFT/RIGHT 0x16200050..55`, and clears the three
/// skill-cover images `0x1650005B..5D`; guests disable those buttons
/// and show the covers. The guest's per-slot cover frame and owned-item
/// fill stay OPEN (caption-sources OPEN 4), as does the exact display
/// rule for the song rect above. The disabled/hidden distinction stays
/// a shell rule: the exe writes an arm word and a frame, this painter
/// omits the body.
#[must_use]
pub(crate) fn waiting_control_armed_for_role(id: &str, is_room_master: bool) -> bool {
    match id {
        "IDC_BUTTON_SWAIT_START"
        | "IDC_BUTTON_SWAIT_MASTER_SKILLSEL"
        | "IDC_BUTTON_SWAIT_MAPLEFT"
        | "IDC_BUTTON_SWAIT_MAPRIGHT"
        | "IDC_BUTTON_SWAIT_1SKILLLEFT"
        | "IDC_BUTTON_SWAIT_1SKILLRIGHT"
        | "IDC_BUTTON_SWAIT_2SKILLLEFT"
        | "IDC_BUTTON_SWAIT_2SKILLRIGHT"
        | "IDC_BUTTON_SWAIT_3SKILLLEFT"
        | "IDC_BUTTON_SWAIT_3SKILLRIGHT" => is_room_master,
        "IDC_BUTTON_SWAIT_READY"
        | "IDC_BUTTON_SWAIT_SKILLSEL"
        | "IDC_IMAGE_SWAIT_READYBUTTEONINACTIVE"
        | "IDC_IMAGE_SWAIT_SPEEDBUTTEONINACTIVE"
        | "IDC_IMAGE_SWAIT_COVERMUSIC"
        | "IDC_IMAGE_SWAIT_COVERSKILLLEFT"
        | "IDC_IMAGE_SWAIT_COVERSKILLRIGHT"
        | "IDC_IMAGE_SWAIT_COVERSKILLSEL" => !is_room_master,
        "IDC_BUTTON_SWAIT_MUSELECT" => is_room_master,
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
    paint_waiting_seat_avatar_at(frame, placements, figure, now_ms, 0);
}

/// Same self-figure fallback as [`paint_waiting_seat_avatar`], but at
/// the wire `member_id` cover (a guest is not necessarily seat 0).
pub fn paint_waiting_seat_avatar_at(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    figure: Option<&AvatarFigure>,
    now_ms: u64,
    seat_index: usize,
) {
    let Some(figure) = figure else {
        return;
    };
    if let Some(slot) = waiting_seat_slot_rect(placements, seat_index) {
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
    let seats = staged_seats(room, None);
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
    /// This member's `4035`/`4037` music-state byte says a download runs.
    pub(crate) downloading: bool,
    /// Music-state 2 (`NoAccess`) arms the seat's NoGem plate
    /// (exe seat `+0x217c`, refresh slot 7).
    pub(crate) no_gem: bool,
    /// A nonzero free-pass word arms the PC-room badge
    /// (exe seat `+0x212c`, refresh slot 8).
    pub(crate) pc_room: bool,
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
    /// `Wait_NoGemImage.ojs`: music-state 2, the song needs a gem.
    NoGem,
    /// `Wait_PCRoomMark.ojs`: a nonzero free-pass word.
    PCRoom,
    /// `Wait_Playing.ojs`: the member is playing.
    Playing,
    /// `Wait_Downloding.ojs`: the member is downloading the room song.
    DownloadingBadge,
    /// `Wait_Downloding_Bar.ojs`: the same member's download bar.
    DownloadingBar,
}

impl SeatIndicator {
    /// Body name of this indicator.
    const fn sprite(self) -> &'static str {
        match self {
            SeatIndicator::TeamPlate => "Wait_ReadyWindow.ojs",
            SeatIndicator::MasterStar => "Wait_BossMark.ojs",
            SeatIndicator::ReadyPlate => "Wait_ReadyText.ojs",
            SeatIndicator::NoSong => "Wait_NoMusicImage.ojs",
            SeatIndicator::NoGem => "Wait_NoGemImage.ojs",
            SeatIndicator::PCRoom => "Wait_PCRoomMark.ojs",
            SeatIndicator::Playing => "Wait_Playing.ojs",
            SeatIndicator::DownloadingBadge => "Wait_Downloding.ojs",
            SeatIndicator::DownloadingBar => "Wait_Downloding_Bar.ojs",
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
            SeatIndicator::NoGem => seat.no_gem.then_some(1),
            SeatIndicator::PCRoom => seat.pc_room.then_some(1),
            SeatIndicator::Playing => seat.playing.then_some(1),
            SeatIndicator::DownloadingBadge | SeatIndicator::DownloadingBar => {
                seat.downloading.then_some(1)
            }
        }
    }
}

/// Indicator body behind one seat-template sprite name. The exe's
/// seat refresh (`FUN_004b2e30` -> `FUN_0052a2b0`, dossier
/// `ev_ee564ef6...`) stages the twelve authored children by index; the
/// child vector includes the two text/avatar children, so slots 7/8/11
/// are `Wait_NoGemImage.ojs`, `Wait_PCRoomMark.ojs` and
/// `Wait_Playing.ojs`. Slot 7 arms on `music_state == 2` (NoAccess),
/// slot 8 on a nonzero free-pass word, slot 11 on playing-state 1.
/// Bodies with no pinned flag return `None` and stay dark:
///
/// - `Waiting_P2P_Image.ojs` — P2P versus banner; needs the room's
///   venue class (OPEN).
/// - `Wait_ReadyWindow.ojs` team frames come from the authored team
///   plate (already staged by the cover painter), not this flag pass.
fn seat_indicator(sprite: &str) -> Option<SeatIndicator> {
    [
        SeatIndicator::TeamPlate,
        SeatIndicator::MasterStar,
        SeatIndicator::ReadyPlate,
        SeatIndicator::NoSong,
        SeatIndicator::NoGem,
        SeatIndicator::PCRoom,
        SeatIndicator::Playing,
        SeatIndicator::DownloadingBadge,
        SeatIndicator::DownloadingBar,
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
pub(crate) fn paint_waiting_seat_marks(
    frame: &mut Frame<'_>,
    seats: &[StagedSeat],
    art: &SceneArt,
) {
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

/// Wire `music-state=4` is the exe's downloading marker (§2 of
/// `docs/music-availability.md`; the 4037 body `0004` in matched-ab-339).
fn is_downloading(member: &o2jam_net::RoomMember) -> bool {
    member.music_state == o2jam_net::MusicState::Downloading.code()
}

/// Wire `music-state=2` is the exe's NoGem marker (`FUN_004b2e30`
/// refresh slot 7, seat `+0x217c`).
fn is_no_gem(member: &o2jam_net::RoomMember) -> bool {
    member.music_state == o2jam_net::MusicState::NoAccess.code()
}

/// Staged seats in cover order: wire slot `i` owns cover `i`, and our
/// own seat owns cover `member_id` when the 3003 snapshot does not
/// list it. The p343 matched capture pins both: the slot-0 master
/// (star, PC, READY) on the left cover, the guest on the next one.
pub(crate) fn staged_seats(room: &JoinedRoom, local_name: Option<&str>) -> Vec<StagedSeat> {
    let mut seats: Vec<StagedSeat> = (0..WAITING_SEATS).map(|_| empty_staged_seat()).collect();
    for slot in room.slots.iter() {
        let index = usize::from(slot.index);
        let Some(seat) = seats.get_mut(index) else {
            continue;
        };
        *seat = staged_slot_seat(
            slot,
            room.champion == Some(u8::try_from(index).unwrap_or(u8::MAX)),
        );
    }
    let own = usize::from(room.member_id);
    if seats.get(own).is_some_and(|seat| !seat.present) {
        let own_slot = room
            .slots
            .iter()
            .find(|slot| slot.index == room.member_id)
            .and_then(|slot| slot.member.as_ref());
        seats[own] = StagedSeat {
            name: local_name.unwrap_or("YOU").to_owned(),
            level: own_slot.map(|member| member.level),
            team: own_slot.map_or(room.team, |member| member.team),
            ready: own_slot.is_some_and(|member| member.ready),
            present: true,
            locked: false,
            champion: room.champion == Some(room.member_id),
            master: own_slot.is_some_and(|member| member.master),
            playing: own_slot.is_some_and(|member| member.playing_state != 0),
            has_song: own_slot.is_some_and(|member| !member.music_ids.is_empty()),
            downloading: own_slot.is_some_and(is_downloading),
            no_gem: own_slot.is_some_and(is_no_gem),
            pc_room: own_slot.is_some_and(|member| member.free_pass != 0),
        };
    }
    seats
}

/// Staged view of one wire slot, including its own master flag.
fn staged_slot_seat(slot: &RoomSlot, champion: bool) -> StagedSeat {
    match &slot.member {
        Some(member) => StagedSeat {
            name: member.nickname.clone(),
            level: Some(member.level),
            team: member.team,
            ready: member.ready,
            present: true,
            locked: false,
            champion,
            master: member.master,
            playing: member.playing_state != 0,
            has_song: !member.music_ids.is_empty(),
            downloading: is_downloading(member),
            no_gem: is_no_gem(member),
            pc_room: member.free_pass != 0,
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
            downloading: false,
            no_gem: false,
            pc_room: false,
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
        downloading: false,
        no_gem: false,
        pc_room: false,
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
    paint_waiting_bare_caption_ink(frame, placements, id, text, art_title_ink());
}

/// Ink one staged caption in an explicit tone (the random flag's
/// yellow), with no shell plate or other glyph decoration.
fn paint_waiting_bare_caption_ink(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    id: &str,
    text: &str,
    ink: u32,
) {
    if let Some((x, y)) = waiting_caption_ink_origin(placements, id, frame) {
        draw_text(frame, x, y, text, ink);
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
    paint_waiting_header_numbers(frame, art, captions.channel_number, captions.room_number);
    paint_waiting_level_badge(frame, captions.level, captions.random, art);
    paint_waiting_covers(frame, room, placements, captions.local_name);
    paint_waiting_team_highlight(frame, room, placements);
    paint_waiting_effect_toggle(frame, art, captions.effect);
    paint_waiting_chat_tabs(frame, art, captions.chat_channel);
    paint_waiting_messenger_lamps(frame, art, captions.messenger_lamp_lit);
    paint_waiting_wins(frame, room, placements);
    let speed = captions.level.map_or(room.speed, |level| level.speed);
    paint_waiting_speed(frame, speed, placements);
    paint_waiting_skills(frame, room, captions.skill_slots, placements);
    paint_waiting_skillcounts(
        frame,
        &captions.skill_counts,
        captions.skill_slots,
        placements,
    );
    paint_waiting_mapname(frame, room, placements);
    paint_waiting_bpm(frame, captions.song_bpm, placements);
    paint_waiting_music(frame, placements, captions.song, captions.random);
}

/// Top-bar digit wells: `IDC_IMAGE_SWAIT_CHNUM_10/01` show the one-
/// based channel number and `IDC_IMAGE_SWAIT_RMNUM_100/010/001` show
/// the joined room number. `Wait_RoomNumber.ojs` decodes ten glyph
/// layers in digit order, so each well paints exactly its own layer.
/// The values come from the accepted wire rows: 1003 `channel_id + 1`
/// (the same rule as the Channel dialog digits) and the 3002 room
/// number. An unstaged value leaves its well untouched.
const WAITING_CHANNEL_NUMBER_IDS: [&str; 2] =
    ["IDC_IMAGE_SWAIT_CHNUM_10", "IDC_IMAGE_SWAIT_CHNUM_01"];
const WAITING_ROOM_NUMBER_IDS: [&str; 3] = [
    "IDC_IMAGE_SWAIT_RMNUM_100",
    "IDC_IMAGE_SWAIT_RMNUM_010",
    "IDC_IMAGE_SWAIT_RMNUM_001",
];

/// Decimal digit of `value` at one weight (100, 10, or 1).
fn waiting_number_digit(value: u32, weight: u32) -> usize {
    usize::try_from(value / weight % 10).unwrap_or(0)
}

/// Paint one selected digit layer at its bound well.
fn paint_waiting_number_well(
    frame: &mut Frame<'_>,
    art: &SceneArt,
    id: &str,
    digit: usize,
    shares_body: bool,
) {
    let Some(placed) = art.placements.iter().find(|placed| placed.id == id) else {
        return;
    };
    paint_placement_layer(frame, placed, &art.figures, digit, shares_body, None);
}

/// Paint the five top-bar wells from the staged wire values. The body
/// is shared across the wells, so its rects (not its record anchors)
/// position each glyph ([`shared_sprite_names`]).
fn paint_waiting_header_numbers(
    frame: &mut Frame<'_>,
    art: &SceneArt,
    channel_number: Option<u16>,
    room_number: Option<i32>,
) {
    let shares_body = shared_sprite_names(&art.placements).contains("Wait_RoomNumber.ojs");
    if let Some(channel) = channel_number {
        let shown = u32::from(channel).saturating_add(1);
        for (id, weight) in WAITING_CHANNEL_NUMBER_IDS.iter().zip([10, 1]) {
            paint_waiting_number_well(
                frame,
                art,
                id,
                waiting_number_digit(shown, weight),
                shares_body,
            );
        }
    }
    if let Some(room) = room_number {
        let shown = u32::try_from(room.max(0)).unwrap_or(0);
        for (id, weight) in WAITING_ROOM_NUMBER_IDS.iter().zip([100, 10, 1]) {
            paint_waiting_number_well(
                frame,
                art,
                id,
                waiting_number_digit(shown, weight),
                shares_body,
            );
        }
    }
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

/// `Wait_Level.ojs` frame the badge paints: `12*difficulty + speed`,
/// plus the `0x30` MX family shift for an EX chart above level 3.
/// The exe stages `index + 1` and setter VA `0x42f870` subtracts the
/// one before its image lookup, so the staged layer is the plain
/// sum (files `0x4b33f4`-`0x4b3442`, `0x4b805a`-`0x4b80a0`).
/// A speed code past the 12 cells names no cell: `None` paints
/// nothing rather than a neighbouring speed, and a family past the
/// staged body stays empty for the same reason.
#[must_use]
pub(crate) fn waiting_level_layer(level: WaitingLevel) -> Option<usize> {
    let cell = usize::from(level.speed);
    (cell < WAITING_LEVEL_SPEEDS).then(|| {
        let family = usize::from(level.difficulty) * WAITING_LEVEL_SPEEDS;
        let mx_shift = if level.uses_mx_family() { 0x30 } else { 0 };
        family + cell + mx_shift
    })
}

/// `Wait_Level.ojs` AX-family base frame (`speed + 0x25`, files
/// `0x4b33d4`-`0x4b33e9` in `FUN_004b2e30`). A variant-flagged song
/// drops the difficulty family entirely: even the MX shift is reset to
/// zero when the flag stands.
const WAITING_LEVEL_AX_BASE: usize = 0x25;

/// Frame the badge paints for one level plus the song's random flag.
#[must_use]
pub(crate) fn waiting_badge_layer(level: WaitingLevel, random: bool) -> Option<usize> {
    if random {
        usize::from(level.speed)
            .checked_add(WAITING_LEVEL_AX_BASE)
            .filter(|cell| *cell < WAITING_LEVEL_AX_BASE + WAITING_LEVEL_SPEEDS)
    } else {
        waiting_level_layer(level)
    }
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
/// The MX family shift for an EX chart above level 3 rides the catalog
/// word (`0x4b3372`-`0x4b33b7`, `+0x30`; #761). The AX family for a
/// flagged song id (`0x7789d8 & 0xf0000000`; #759) shifts by `+0x25`
/// with the yellow random caption. Both shifts live in
/// `waiting_badge_layer`; unresolved writers stay recorded in
/// `docs/caption-sources.md`.
fn paint_waiting_level_badge(
    frame: &mut Frame<'_>,
    level: Option<WaitingLevel>,
    random: bool,
    art: &SceneArt,
) {
    let Some(level) = level else {
        return;
    };
    let Some(layer) = waiting_badge_layer(level, random) else {
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
    local_name: Option<&str>,
) {
    let has_champion = room.champion.is_some();
    for (cover_id, seat) in WAITING_COVER_IDS.iter().zip(staged_seats(room, local_name)) {
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

/// Paint the two effect toggles as an exclusive pair: record 1 on the
/// selected control, record 0 on the other. The exe's toggle mapper
/// (`FUN_0048c810`) walks the two control codes and calls its frame
/// setter with 1 for the pressed one and 0 for the other; the Waiting
/// setup repeats the same selection from global `0x7789e0`
/// (`FUN_004ba6c0`, files `0x4ba929`-`0x4ba94b`).
fn paint_waiting_effect_toggle(frame: &mut Frame<'_>, art: &SceneArt, effect: WaitingEffect) {
    paint_waiting_toggle_group(frame, art, &WAIT_EFFECT_IDS, effect.code());
}

/// Chat-tab ids and messenger-lamp ids from the `STATE_WAITING` block
/// (`ControlList` codes `0x1620100E..11` and `0x1650006B/6C`).
const WAIT_CHAT_TAB_IDS: [&str; 4] = [
    "IDC_BUTTON_SWAIT_CHAT_ALL",
    "IDC_BUTTON_SWAIT_CHAT_FRIEND",
    "IDC_BUTTON_SWAIT_CHAT_GUILD",
    "IDC_BUTTON_SWAIT_CHAT_WHISPER",
];
const WAIT_MESSENGER_LAMP_IDS: [&str; 2] = [
    "IDC_IMAGE_SWAIT_MESSENGER_ALRAM",
    "IDC_IMAGE_SWAIT_MESSENGER_ALRAM1",
];
/// WAITING stamp bodies (`0x16500067/68`) from the `STATE_WAITING` block.
const WAIT_STAMP_IDS: [&str; 2] = ["IDC_IMAGE_SWAIT_WAITTEXTIMG", "IDC_IMAGE_SWAIT_WAITIMG_ANI"];

/// Paint one exclusive toggle group: record 1 on the selected code,
/// record 0 on every sibling. Both the effect pair and the four chat
/// tabs are staged through `FUN_0048c810`.
fn paint_waiting_toggle_group(
    frame: &mut Frame<'_>,
    art: &SceneArt,
    ids: &[&str],
    active_code: u32,
) {
    let shared = shared_sprite_names(&art.placements);
    for id in ids {
        let Some(placed) = art.placements.iter().find(|placed| placed.id == *id) else {
            continue;
        };
        let shares_body = placed
            .sprite
            .as_deref()
            .is_some_and(|sprite| shared.contains(sprite));
        let record = usize::from(placed.code == active_code);
        paint_placement_layer(frame, placed, &art.figures, record, shares_body, None);
    }
}

/// Paint the four chat tabs: the exe's toggle mapper lights only the
/// selected tab (`FUN_0048c810` with base `0x1620100e`, count 4).
fn paint_waiting_chat_tabs(frame: &mut Frame<'_>, art: &SceneArt, channel: WaitingChatChannel) {
    paint_waiting_toggle_group(frame, art, &WAIT_CHAT_TAB_IDS, channel.code());
}

/// Paint the two messenger lamps from one alert word. The exe starts
/// an 800 ms animation on both controls (`FUN_004c0690`, file
/// `0xc06e5`); the shell pins the ON interval but not the individual
/// animation frames (OPEN: one qualified messenger-alert capture).
fn paint_waiting_messenger_lamps(frame: &mut Frame<'_>, art: &SceneArt, lit: bool) {
    let alert_code = if lit { 0x1650_006B } else { 0 };
    paint_waiting_toggle_group(frame, art, &WAIT_MESSENGER_LAMP_IDS, alert_code);
}

/// Paint the armed WAITING stamp: the exe shows the wait-text image
/// and its animation control when another seat is playing, then clears
/// both through the 5-second updater (`FUN_004ba6c0` files
/// `0xba7b7`-`0xba89f`, `FUN_004b9dc0` files `0xba60e`-`0xba68a`).
/// Record 0 is a pinned stand-in for the animation's own frames; no
/// new frame sequence is invented.
fn paint_waiting_stamp(frame: &mut Frame<'_>, art: &SceneArt, lit: bool) {
    if !lit {
        return;
    }
    let shared = shared_sprite_names(&art.placements);
    for id in WAIT_STAMP_IDS {
        let Some(placed) = art.placements.iter().find(|placed| placed.id == id) else {
            continue;
        };
        let shares_body = placed
            .sprite
            .as_deref()
            .is_some_and(|sprite| shared.contains(sprite));
        paint_placement_layer(frame, placed, &art.figures, 0, shares_body, None);
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
/// `X0.5`..`XR` chain (`room-enums.md`, codes 0..11). The caller
/// supplies the pick-or-room speed so a staged pick survives the 4001
/// room echo (#535); the exe's chip carries its own plate, so the
/// caption inks bare glyphs at the reference bearing — no black box.
pub(crate) fn paint_waiting_speed(frame: &mut Frame<'_>, speed: u8, placements: &[ScenePlacement]) {
    paint_waiting_bare_caption(
        frame,
        placements,
        WAITING_SPEED_NUM_ID,
        &room_speed_label(speed),
    );
}

/// One owned skill in a Waiting selector: id at owned-vector `+4`,
/// count at `+8` (stride `0xc`, handler `FUN_004bb760`, evidence
/// `ev_826be8349ca6a899ad81a9c9c0825f83c0b58c0d5167e1e90b0a511a8a7af56a`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitingSkillItem {
    /// Owned-item id the exe turns into the ring art.
    pub id: i32,
    /// Owned-item count the exe prints at the `SKILLCOUNT` rect.
    pub count: u32,
}

/// Direction of one `1/2/3SKILLLEFT/RIGHT` press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitingSkillStep {
    /// LEFT: decrement with wrap to the last owned item.
    Previous,
    /// RIGHT: increment with wrap to the first owned item.
    Next,
}

/// Locally staged owned-skill selectors, one list per Waiting slot.
/// The server fill of the three owned-item vectors stays OPEN
/// (`docs/caption-sources.md` OPEN 4), so an empty list leaves the
/// existing caption sources untouched instead of inventing items.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WaitingSkillSlots {
    items: [Vec<WaitingSkillItem>; 3],
    selected: [usize; 3],
}

impl WaitingSkillSlots {
    /// Stage the owned items of one slot (0..3). A new list starts
    /// from the first item, as the exe's dialog ctor does for `+0x35c`.
    pub fn set_slot(&mut self, slot: usize, items: Vec<WaitingSkillItem>) -> bool {
        let Some(staged) = self.items.get_mut(slot) else {
            return false;
        };
        *staged = items;
        if let Some(selected) = self.selected.get_mut(slot) {
            *selected = 0;
        }
        true
    }

    /// Selected owned item of one slot, when a list names one.
    #[must_use]
    pub fn selected(&self, slot: usize) -> Option<&WaitingSkillItem> {
        let items = self.items.get(slot)?;
        let index = *self.selected.get(slot)?;
        items.get(index)
    }

    /// Cycle one slot's selection with the exe's wrap rule. Returns
    /// false when no owned item exists (no invented selection).
    pub fn cycle(&mut self, slot: usize, step: WaitingSkillStep) -> bool {
        let Some(items) = self.items.get(slot) else {
            return false;
        };
        if items.is_empty() {
            return false;
        }
        let Some(selected) = self.selected.get_mut(slot) else {
            return false;
        };
        *selected = match step {
            WaitingSkillStep::Previous => {
                if *selected == 0 {
                    items.len() - 1
                } else {
                    *selected - 1
                }
            }
            WaitingSkillStep::Next => {
                if *selected + 1 >= items.len() {
                    0
                } else {
                    *selected + 1
                }
            }
        };
        true
    }
}

/// Skill-ring ids at the `1/2/3SKILL` rects (rects 87..89, the X
/// boxes): empty slots stay untouched so the staged X art shows.
/// The counts paint beside them at the `SKILLCOUNT` rects from the
/// staged inventory ([`paint_waiting_skillcounts`]). With staged
/// owned-item lists, the selected id and count replace those two
/// sources together.
pub(crate) fn paint_waiting_skills(
    frame: &mut Frame<'_>,
    room: &JoinedRoom,
    slots: Option<&WaitingSkillSlots>,
    placements: &[ScenePlacement],
) {
    for (slot, slot_id) in WAITING_SKILL_IDS.iter().enumerate() {
        let text = slots
            .and_then(|slots| slots.selected(slot))
            .map(|item| item.id.to_string())
            .or_else(|| room.skills.get(slot).map(i32::to_string));
        let Some(text) = text else {
            continue;
        };
        if let Some((x, y)) = waiting_bnd_origin(placements, slot_id, frame) {
            paint_room_detail_line(frame, x, y, &text, art_title_ink());
        }
    }
}

/// Direction of one master map-arrow press (`FUN_004bb570`, VA
/// `0x4bb570`, evidence `ev_c9d69d8472c39643a5b3f030978707add16ef0d4682e3a17b965114d71ea3282`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitingMapStep {
    /// `MAPLEFT 0x1620000C`: step -1 (file `0x4b832d`).
    Previous,
    /// `MAPRIGHT 0x1620000D`: step +1 (file `0x4b8384`).
    Next,
}

/// Stage words the exe skips in the normal map cycle (rdata
/// `0x649ac4`, bytes evidence
/// `ev_df240aa2d3d7f8da4d9632a68c2362d71b8d3961170694925efa7976a84b4d1e`).
const WAITING_MAP_SKIP_STAGES: [i32; 4] = [140, 170, 180, 240];

/// Frame count the exe uses when `IDC_IMAGE_SWAIT_MAP 0x1650001A` has
/// no staged image (file `0x4bb575`). Staged Waiting art supplies the
/// real count from `Wait_Map.ojs` instead.
pub const WAITING_MAP_DEFAULT_FRAMES: i32 = 14;

/// Next room word for one map-arrow press.
///
/// Normal branch: add the step, skip the four pinned stage words, and
/// send the result through 0xfa2. `None` is the exe's random-wrap arm
/// (current word at frame 1 with PREVIOUS, or at the frame count with
/// NEXT): it calls `FUN_004bb2c0` to OR in a random flag, whose seed and
/// live wire echo are still OPEN. The high-word branch is pinned and
/// deterministic: PREVIOUS restores the frame count, NEXT restores 1.
#[must_use]
pub fn waiting_map_cycle(arena: i32, step: WaitingMapStep, frame_count: i32) -> Option<i32> {
    if arena & !0xFFFF != 0 {
        return Some(match step {
            WaitingMapStep::Previous => frame_count,
            WaitingMapStep::Next => 1,
        });
    }
    let delta = match step {
        WaitingMapStep::Previous => -1,
        WaitingMapStep::Next => 1,
    };
    if (step == WaitingMapStep::Previous && arena == 1)
        || (step == WaitingMapStep::Next && arena == frame_count)
    {
        return None;
    }
    let mut next = arena.checked_add(delta)?;
    for skipped in WAITING_MAP_SKIP_STAGES {
        if next == skipped {
            next = next.checked_add(delta)?;
        }
    }
    Some(next)
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
    slots: Option<&WaitingSkillSlots>,
    placements: &[ScenePlacement],
) {
    for (slot, (id, count)) in WAITING_SKILLCOUNT_IDS.iter().zip(counts.iter()).enumerate() {
        let count = slots
            .and_then(|slots| slots.selected(slot))
            .map_or(*count, |item| item.count);
        if let Some((x, y)) = waiting_bnd_origin(placements, id, frame) {
            paint_room_detail_line(
                frame,
                x,
                y,
                &waiting_skillcount_text(count, slot > 0),
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
///
/// Yellow caption text at VA `0x649848` (EV `ev_46c80b83…`); the
/// exe sets colour `0xffff00` (`0x4b30b2`) and stores this string in
/// place of the catalog title when the song id's top nibble stands.
const WAITING_RANDOM_CAPTION: &str = "<< Random 선택 >>";
const WAITING_RANDOM_INK: u32 = pack_xrgb(0xFF, 0xFF, 0x00);

/// Ink the staged `MUSICNAME` caption: the catalog title for a plain
/// song, or the yellow random caption when the variant flag stands.
pub(crate) fn paint_waiting_music(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    song: Option<WaitingSong<'_>>,
    random: bool,
) {
    let Some(song) = song else {
        return;
    };
    if random {
        paint_waiting_bare_caption_ink(
            frame,
            placements,
            WAITING_MUSICNAME_ID,
            WAITING_RANDOM_CAPTION,
            WAITING_RANDOM_INK,
        );
    } else {
        paint_waiting_bare_caption(
            frame,
            placements,
            WAITING_MUSICNAME_ID,
            &waiting_music_caption(song.music_id, song.title),
        );
    }
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

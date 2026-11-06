//! Note highway: seven lanes, judgement line, falling notes.
//!
//! Pure layout + paint over [`LevelChart`] onsets (`lane, ms` from
//! [`LevelChart::hit_onsets_ms`]); timing comes from an explicit
//! [`PlayingView::advance_to`], so tests drive the clock. Layout is
//! relative to the frame: full-width lanes, line one sixth up from
//! the bottom, `lookahead` ms of chart visible above the line.

use o2jam_game::{
    lane_effect::{lane_map_for_modifier_byte, SkillRing},
    CoolStreak, Judgement, LaneMap, LongTail, MeasureReshuffle, PeerBars, PillStock, QueueEvent,
    RankDelivery, Score, TimingWindows, JAM_PROGRESS_PER_LEVEL, LIFE_MAX, PEER_SLOT_COUNT,
};
use o2jam_render::{
    draw_number, draw_text, fill_rect, number_width, pack_xrgb, text_width, visible_spans, Frame,
    LaneCurtain, Rect,
};

/// Lanes across the highway.
pub const HIGHWAY_LANES: u8 = 7;
/// How far ahead notes appear, in song ms.
pub const DEFAULT_LOOKAHEAD_MS: u64 = 2000;
/// Note block height in pixels.
pub const NOTE_H: i32 = 8;
/// Peer life-bar width in pixels (one bar per 4015 member slot).
pub const PEER_BAR_WIDTH: u32 = 3;
/// Peer life-bar height in pixels.
pub const PEER_BAR_HEIGHT: u32 = 20;
/// Peer life-bar strip top row.
pub const PEER_BAR_TOP: i32 = 4;
/// Gap between peer life bars (and the frame edge) in pixels.
pub const PEER_BAR_GAP: i32 = 1;
/// Jam edge-bar width in pixels (layout-relative; the staged
/// `Jamgauge_right_bar.ojs` / `Jamgauge_left_bar.ojs` shells ring the
/// same edges once `paint_playing_overlay` binds them).
pub const JAM_BAR_WIDTH: u32 = 4;
/// Jam edge-bar height in pixels (clamped to the frame in paint).
pub const JAM_BAR_HEIGHT: u32 = 24;
/// Jam edge-bar top row (below the peer strip at 4 + 20).
pub const JAM_BAR_TOP: i32 = 28;
/// Gap between a jam edge-bar and the frame edge in pixels.
pub const JAM_BAR_EDGE_GAP: i32 = 1;
/// Odd-lane skin tint (even lanes keep the flat Playing backdrop so the
/// staged `Playing1.opi` gauges ring unchanged edges; lifecycle §10).
/// Procedural stand-in for the skinned lane panels (sprite-55 bodies in
/// `Playing1.opi`; exact panel codecs TBD in `docs/file-formats/opi.md`).
pub const LANE_SKIN_ODD: u32 = pack_xrgb(0x0E, 0x0E, 0x16);
/// Note-cap highlight row above each falling note (mimics the sprite-55
/// note caps; staged `_Note_Cool.ojt` / `_Note_Good.ojt` / `_Note_Bad.ojt`
/// / `_Note_Miss.ojt` runtime refs resolve to bare `.oja` siblings —
/// `docs/file-formats/opi.md` — until those figures bind).
pub const NOTE_CAP: u32 = pack_xrgb(0xB0, 0xE0, 0xFF);
/// Long-note tail beam ink (heads keep the `NOTE_BLUE` core below).
pub const TAIL_BEAM: u32 = pack_xrgb(0x20, 0x80, 0xE0);
/// Judgement-line receptor pad at rest (one pad per lane).
pub const RECEPTOR_DIM: u32 = pack_xrgb(0x2A, 0x2A, 0x38);
/// Judgement-line receptor pad flash on a struck lane.
pub const RECEPTOR_LIT: u32 = pack_xrgb(0xC0, 0xE8, 0xFF);
/// Hit-burst column rising above the line after a strike.
pub const HIT_BURST: u32 = pack_xrgb(0xFF, 0xFF, 0xFF);
/// Hit-burst lifetime in song ms (visual only; timing untouched).
pub const BURST_MS: u64 = 150;
/// Receptor pad height in pixels.
pub const RECEPTOR_H: i32 = 5;
/// Hit-burst column height in pixels.
pub const BURST_H: i32 = 24;

/// Lane layout for one frame size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighwayGeometry {
    /// Left edge of lane 0.
    pub x0: i32,
    /// Lane width in pixels.
    pub lane_w: u32,
    /// Judgement-line row.
    pub line_y: i32,
    /// Frame height (notes never draw below the line).
    pub h: i32,
}

impl HighwayGeometry {
    /// Full-width lanes; line one sixth up from the bottom.
    #[must_use]
    pub fn centered(frame_w: usize, frame_h: usize) -> Self {
        let lane_w = (frame_w / HIGHWAY_LANES as usize).max(1) as u32;
        Self {
            x0: 0,
            lane_w,
            line_y: frame_h as i32 - frame_h as i32 / 6,
            h: frame_h as i32,
        }
    }

    /// Pixel column of one lane (clamped to lane count).
    #[must_use]
    pub fn lane_rect(&self, lane: u8) -> Rect {
        let lane = lane.min(HIGHWAY_LANES - 1);
        Rect {
            x: self.x0 + i32::from(lane) * self.lane_w as i32,
            y: 0,
            w: self.lane_w,
            h: self.h as u32,
        }
    }
}

/// Scrolling notes for one chart.
#[derive(Debug, Clone, Default)]
pub struct PlayingView {
    /// `(lane, onset ms)`, sorted by onset.
    onsets: Vec<(u8, u64)>,
    /// Current song position in ms.
    now_ms: u64,
    /// Visible window ahead in ms.
    lookahead_ms: u64,
}

impl PlayingView {
    /// Notes to show (empty until a chart loads).
    #[must_use]
    pub fn new(mut onsets: Vec<(u8, u64)>) -> Self {
        onsets.sort_by_key(|(_, at)| *at);
        Self {
            onsets,
            now_ms: 0,
            lookahead_ms: DEFAULT_LOOKAHEAD_MS,
        }
    }

    /// Move the song clock (tests and the frame pump call this).
    pub fn advance_to(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
    }

    /// Remove a struck note so it stops falling; false when absent.
    pub fn strike(&mut self, lane: u8, at_ms: u64) -> bool {
        if let Some(i) = self
            .onsets
            .iter()
            .position(|(hit_lane, hit_at)| *hit_lane == lane && *hit_at == at_ms)
        {
            self.onsets.remove(i);
            true
        } else {
            false
        }
    }

    /// Notes on screen as `(lane, y top)`; `y` runs from the top
    /// (lookahead ahead) to the judgement line (due now).
    #[must_use]
    pub fn visible(&self, geom: &HighwayGeometry) -> Vec<(u8, i32)> {
        let mut notes = Vec::new();
        for (lane, at) in &self.onsets {
            if let Some(y) = y_for_onset(geom, *at, self.now_ms, self.lookahead_ms) {
                notes.push((*lane, y));
            }
        }
        notes
    }

    /// Remap every onset through a skill-ring lane table.
    pub fn remap_lanes(&mut self, map: &LaneMap) {
        for (lane, _) in &mut self.onsets {
            if let Some(destination) = map.destination(*lane) {
                *lane = destination;
            }
        }
        self.onsets.sort_by_key(|(_, at)| *at);
    }

    /// Paint highway, line, and visible notes.
    pub fn paint(&self, frame: &mut Frame<'_>, geom: &HighwayGeometry) {
        self.paint_shell(frame, geom);
        for (lane, y) in self.visible(geom) {
            Self::paint_note(frame, geom, lane, y);
        }
    }

    /// Paint the highway through a skill-ring curtain: notes outside the
    /// item-text spans (hidden keeps the top 80%, sudden the bottom
    /// 20%, dark the middle 20%, reverse-dark the outer 40% pair —
    /// `o2jam-render::visible_spans`, clip math OPEN) are not drawn.
    /// `Full` paints everything, like [`PlayingView::paint`].
    pub fn paint_curtained(
        &self,
        frame: &mut Frame<'_>,
        geom: &HighwayGeometry,
        curtain: LaneCurtain,
    ) {
        self.paint_shell(frame, geom);
        if curtain == LaneCurtain::Full {
            for (lane, y) in self.visible(geom) {
                Self::paint_note(frame, geom, lane, y);
            }
            return;
        }
        let highway = Rect {
            x: 0,
            y: 0,
            w: frame.w as u32,
            h: geom.line_y.max(0) as u32,
        };
        let spans = visible_spans(curtain, &highway);
        for (lane, y) in self.visible(geom) {
            if spans
                .iter()
                .any(|span| y >= span.y && y < span.y + span.h as i32)
            {
                Self::paint_note(frame, geom, lane, y);
            }
        }
    }

    /// Background, lane skins, receptors, and the judgement line
    /// (no notes). Even lanes keep the flat Playing backdrop; odd lanes
    /// carry the tint above so staged `Playing1.opi` edge gauges keep
    /// ringing a readable highway.
    fn paint_shell(&self, frame: &mut Frame<'_>, geom: &HighwayGeometry) {
        let w = frame.w as i32;
        fill_rect(
            frame,
            &Rect {
                x: 0,
                y: 0,
                w: w as u32,
                h: frame.h as u32,
            },
            pack_xrgb(0x0A, 0x0A, 0x0A),
        );
        for lane in 0..HIGHWAY_LANES {
            let column = geom.lane_rect(lane);
            if lane % 2 == 1 {
                fill_rect(
                    frame,
                    &Rect {
                        x: column.x,
                        y: 0,
                        w: column.w,
                        h: column.h,
                    },
                    LANE_SKIN_ODD,
                );
            }
            fill_rect(
                frame,
                &Rect {
                    x: column.x + column.w as i32 - 1,
                    y: 0,
                    w: 1,
                    h: column.h,
                },
                pack_xrgb(0x30, 0x30, 0x30),
            );
            Self::paint_receptor(frame, geom, lane, RECEPTOR_DIM);
        }
        fill_rect(
            frame,
            &Rect {
                x: geom.x0,
                y: geom.line_y,
                w: geom.lane_w * u32::from(HIGHWAY_LANES),
                h: 2,
            },
            pack_xrgb(0xE0, 0xE0, 0xE0),
        );
    }

    /// One lane's receptor pad resting just above the judgement line.
    fn paint_receptor(frame: &mut Frame<'_>, geom: &HighwayGeometry, lane: u8, ink: u32) {
        let column = geom.lane_rect(lane);
        fill_rect(
            frame,
            &Rect {
                x: column.x + 1,
                y: geom.line_y - RECEPTOR_H,
                w: column.w.saturating_sub(2),
                h: u32::try_from(RECEPTOR_H).unwrap_or(0),
            },
            ink,
        );
    }

    /// One falling note block: the `NOTE_BLUE` core the soak harness
    /// (`real_art.rs` Playing row) and the shell overlay tests key on,
    /// plus a cap highlight row above it toward the sprite-55 note art.
    fn paint_note(frame: &mut Frame<'_>, geom: &HighwayGeometry, lane: u8, y: i32) {
        let column = geom.lane_rect(lane);
        let w = column.w.saturating_sub(2);
        fill_rect(
            frame,
            &Rect {
                x: column.x + 1,
                y: y - 1,
                w,
                h: 1,
            },
            NOTE_CAP,
        );
        fill_rect(
            frame,
            &Rect {
                x: column.x + 1,
                y,
                w,
                h: NOTE_H as u32,
            },
            pack_xrgb(0x40, 0xC0, 0xFF),
        );
    }
}

/// Item-text curtain for one skill ring (geometry stays built in
/// `o2jam-render::visible_spans`): the lane rings show everything,
/// each sight ring covers or reveals its fifth of the highway.
fn curtain_for_ring(ring: SkillRing) -> LaneCurtain {
    match ring {
        SkillRing::Hidden => LaneCurtain::Hidden,
        SkillRing::Sudden => LaneCurtain::Sudden,
        SkillRing::Dark => LaneCurtain::Dark,
        SkillRing::ReverseDark => LaneCurtain::ReverseDark,
        SkillRing::Mirror | SkillRing::Random | SkillRing::Panic => LaneCurtain::Full,
    }
}

/// Filled rows for `progress_in_level` (`0..50`) over a `bar_h`-row jam
/// bar. Integer math keeps the exe's cumulative-progress shape
/// (each level spans [`JAM_PROGRESS_PER_LEVEL`]; no reset on grant).
fn jam_fill_rows(progress_in_level: u64, bar_h: u32) -> u32 {
    let filled = progress_in_level
        .saturating_mul(u64::from(bar_h))
        .checked_div(JAM_PROGRESS_PER_LEVEL)
        .unwrap_or(0);
    u32::try_from(filled).unwrap_or(u32::MAX).min(bar_h)
}

/// Row for one onset at `now_ms` (`None` while struck/past or beyond
/// the lookahead). Shared by falling notes and long-note tail beams so
/// both ride the same scroll speed.
fn y_for_onset(geom: &HighwayGeometry, at_ms: u64, now_ms: u64, lookahead_ms: u64) -> Option<i32> {
    if at_ms <= now_ms {
        return None;
    }
    let ahead = at_ms.saturating_sub(now_ms);
    if ahead > lookahead_ms || lookahead_ms == 0 {
        return None;
    }
    let px_per_ms = geom.line_y as f32 / lookahead_ms as f32;
    Some(geom.line_y - (ahead as f32 * px_per_ms) as i32)
}

/// Word shown for each judgement tier.
fn word_of(judgement: Judgement) -> &'static str {
    match judgement {
        Judgement::Cool => "COOL",
        Judgement::Good => "GOOD",
        Judgement::Bad => "BAD",
        Judgement::Miss => "MISS",
    }
}

/// One hittable note: rated once, by key press or by timeout.
/// Long notes carry a tail: after the head is struck the lane is
/// held until the tail, then released exactly there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingNote {
    lane: u8,
    at_ms: u64,
    tail_ms: Option<u64>,
    sample: u32,
    judged: bool,
    holding: bool,
    tail_judged: bool,
}

impl PendingNote {
    /// Tap note (no tail).
    fn tap(lane: u8, at_ms: u64, sample: u32) -> Self {
        Self {
            lane,
            at_ms,
            tail_ms: None,
            sample,
            judged: false,
            holding: false,
            tail_judged: true,
        }
    }

    /// Long note: head at `at_ms`, release at `tail_ms`.
    fn hold(lane: u8, at_ms: u64, tail_ms: u64, sample: u32) -> Self {
        Self {
            lane,
            at_ms,
            tail_ms: Some(tail_ms),
            sample,
            judged: false,
            holding: false,
            tail_judged: false,
        }
    }

    /// Done when the head is judged and the tail (if any) is judged.
    fn done(&self) -> bool {
        self.judged && self.tail_judged
    }
}

/// Live play: scrolling view plus verified judgement/scoring.
///
/// Key presses rate the nearest unjudged note in that lane inside
/// the bad window; notes aging past it rate Miss on advance. Presses
/// with no note nearby rate nothing (the original's empty-press
/// behavior is unverified, so nothing is guessed).
///
/// Skill rings plug in here without touching net/render: remap lanes
/// through a [`LaneMap`], hide notes behind a [`LaneCurtain`], and ask
/// [`LivePlay::should_rebuild_map`] per measure for the panic-style
/// per-measure reshuffle (`MeasureReshuffle`: rebuild every plain
/// measure, once per hold span — `FUN_00574330`).
#[derive(Debug)]
pub struct LivePlay {
    view: PlayingView,
    pending: Vec<PendingNote>,
    score: Score,
    pills: PillStock,
    peers: PeerBars,
    streak: CoolStreak,
    windows: TimingWindows,
    reshuffle: MeasureReshuffle,
    curtain: LaneCurtain,
    now_ms: u64,
    last_word: Option<(Judgement, u64)>,
    sounds: Vec<u32>,
    /// Last strike per lane in song ms (visual hit-burst only; scoring
    /// and timing never read this).
    lane_flash: [Option<u64>; HIGHWAY_LANES as usize],
}

impl Default for LivePlay {
    fn default() -> Self {
        Self {
            view: PlayingView::default(),
            pending: Vec::new(),
            score: Score::default(),
            pills: PillStock::default(),
            peers: PeerBars::default(),
            streak: CoolStreak::default(),
            windows: TimingWindows::default(),
            reshuffle: MeasureReshuffle::default(),
            curtain: LaneCurtain::Full,
            now_ms: 0,
            last_word: None,
            sounds: Vec::new(),
            lane_flash: [None; HIGHWAY_LANES as usize],
        }
    }
}

impl LivePlay {
    /// Notes to play (empty until a chart loads); each onset carries
    /// its key-sound sample id.
    #[must_use]
    pub fn new(onsets: &[(u8, u64, u32)]) -> Self {
        let mut pending: Vec<PendingNote> = onsets
            .iter()
            .map(|(lane, at_ms, sample)| PendingNote::tap(*lane, *at_ms, *sample))
            .collect();
        pending.sort_by_key(|note| note.at_ms);
        Self {
            view: PlayingView::new(
                onsets
                    .iter()
                    .map(|(lane, at_ms, _)| (*lane, *at_ms))
                    .collect(),
            ),
            pending,
            score: Score::default(),
            pills: PillStock::default(),
            peers: PeerBars::default(),
            streak: CoolStreak::default(),
            windows: TimingWindows::default(),
            reshuffle: MeasureReshuffle::default(),
            curtain: LaneCurtain::Full,
            now_ms: 0,
            last_word: None,
            sounds: Vec::new(),
            lane_flash: [None; HIGHWAY_LANES as usize],
        }
    }

    /// Notes with long-note tails: `(lane, head ms, tail ms, sample)`.
    /// A tail equal to (or before) its head degrades to a tap.
    #[must_use]
    pub fn new_holds(entries: &[(u8, u64, u64, u32)]) -> Self {
        let mut pending: Vec<PendingNote> = entries
            .iter()
            .map(|(lane, head_ms, tail_ms, sample)| {
                LongTail::new(*head_ms, *tail_ms).map_or_else(
                    || PendingNote::tap(*lane, *head_ms, *sample),
                    |tail| PendingNote::hold(*lane, tail.head_ms, tail.tail_ms, *sample),
                )
            })
            .collect();
        pending.sort_by_key(|note| note.at_ms);
        Self {
            view: PlayingView::new(
                entries
                    .iter()
                    .map(|(lane, head_ms, _, _)| (*lane, *head_ms))
                    .collect(),
            ),
            pending,
            score: Score::default(),
            pills: PillStock::default(),
            peers: PeerBars::default(),
            streak: CoolStreak::default(),
            windows: TimingWindows::default(),
            reshuffle: MeasureReshuffle::default(),
            curtain: LaneCurtain::Full,
            now_ms: 0,
            last_word: None,
            sounds: Vec::new(),
            lane_flash: [None; HIGHWAY_LANES as usize],
        }
    }

    /// Skill-ring curtain worn for this run (default: none).
    pub fn set_curtain(&mut self, curtain: LaneCurtain) {
        self.curtain = curtain;
    }

    /// Song setup for one skill ring: remap lanes through the ring's
    /// table, restart the per-measure reshuffle state, and hang the
    /// ring's item-text curtain.
    ///
    /// Panic keeps reshuffling after setup: whenever
    /// [`Self::should_rebuild_map`] says so, build a fresh table with
    /// [`SkillRing::build_lane_map`] and feed it to [`Self::remap_lanes`].
    pub fn apply_skill_ring(&mut self, ring: SkillRing, next_random: &mut impl FnMut() -> u32) {
        let map = ring.build_lane_map(HIGHWAY_LANES as usize, next_random);
        self.reshuffle = MeasureReshuffle::default();
        self.set_curtain(curtain_for_ring(ring));
        self.remap_lanes(&map);
    }

    /// Song setup from the room modifier byte (session byte at VA
    /// `0x7789e4`, mirrored from room field `+0x20d`): the verified
    /// lane path (byte 2 shuffles all lanes, byte 0 the narrow table,
    /// anything else plays straight through) with a fresh reshuffle
    /// state and no curtain — the setup copies static visibility
    /// params for bytes 0 and 2, and the remaining clip math is
    /// untraced, so rings with curtains are worn via
    /// [`Self::apply_skill_ring`] instead.
    pub fn apply_modifier_byte(&mut self, modifier: u8, next_random: &mut impl FnMut() -> u32) {
        let map = lane_map_for_modifier_byte(modifier, HIGHWAY_LANES as usize, next_random);
        self.reshuffle = MeasureReshuffle::default();
        self.set_curtain(LaneCurtain::Full);
        self.remap_lanes(&map);
    }

    /// Remap every pending lane through a skill-ring table
    /// (mirror / shuffle built by the `LaneMap` constructors).
    /// Flashes clear: they are frame-space visuals for struck lanes.
    pub fn remap_lanes(&mut self, map: &LaneMap) {
        for note in &mut self.pending {
            if let Some(destination) = map.destination(note.lane) {
                note.lane = destination;
            }
        }
        self.pending.sort_by_key(|note| note.at_ms);
        self.view.remap_lanes(map);
        self.lane_flash = [None; HIGHWAY_LANES as usize];
    }

    /// Whether the lane table needs a fresh build for the next measure
    /// (panic-style ring: every plain measure, once per hold span).
    pub fn should_rebuild_map(&mut self, measure_has_hold: bool) -> bool {
        self.reshuffle.should_rebuild_map(measure_has_hold)
    }

    /// Peer bars mirrored from S→C `GameStatsUpdate` (4015).
    #[must_use]
    pub fn peers(&self) -> &PeerBars {
        &self.peers
    }

    /// Apply one peer stats write (Life-`0`/Jam-`1` u16 to the slot) and
    /// return its `(0x12, type, slot, 0)` queue forward, if any.
    pub fn apply_peer_stats(&mut self, slot: u8, kind: u16, value: u16) -> Option<QueueEvent> {
        self.peers.apply_stats_update(slot, kind, value)
    }

    /// Apply one 8-byte peer rank broadcast; returns per-position
    /// deliveries under the rank-remap rule.
    pub fn apply_peer_ranks(&mut self, packet: [u8; PEER_SLOT_COUNT]) -> Vec<RankDelivery> {
        self.peers.apply_rank_broadcast(packet)
    }

    /// Move the song clock; notes past the bad window rate Miss.
    /// Heads missed by timeout kill their tails too, and tails with
    /// no release by their window end rate Miss (missing the release
    /// is a miss — long-note rule).
    pub fn advance_to(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
        self.view.advance_to(now_ms);
        let windows = self.windows;
        let mut late = Vec::new();
        for note in &mut self.pending {
            if !note.judged && note.at_ms.saturating_add(windows.bad) < now_ms {
                note.judged = true;
                note.tail_judged = true;
                note.holding = false;
                late.push(Judgement::Miss);
            } else if let Some(tail) = note.tail_ms {
                let hold = LongTail {
                    head_ms: note.at_ms,
                    tail_ms: tail,
                };
                if note.holding && !note.tail_judged && hold.is_expired(now_ms, &windows) {
                    note.tail_judged = true;
                    note.holding = false;
                    late.push(Judgement::Miss);
                }
            }
        }
        for judgement in late {
            self.rate(judgement);
        }
    }

    /// Strike a lane: rate the nearest unjudged note in range.
    /// Returns the judgement, or `None` for an empty press.
    pub fn strike_lane(&mut self, lane: u8, now_ms: u64) -> Option<Judgement> {
        self.advance_to(now_ms);
        let mut best: Option<(usize, u64)> = None;
        for (i, note) in self.pending.iter().enumerate() {
            if note.judged || note.lane != lane {
                continue;
            }
            let gap = note.at_ms.abs_diff(now_ms);
            if gap <= self.windows.bad && best.is_none_or(|(_, seen)| gap < seen) {
                best = Some((i, gap));
            }
        }
        let (i, _) = best?;
        let offset = now_ms as i64 - self.pending[i].at_ms as i64;
        let (lane, at_ms, sample) = (
            self.pending[i].lane,
            self.pending[i].at_ms,
            self.pending[i].sample,
        );
        self.pending[i].judged = true;
        self.pending[i].holding = self.pending[i].tail_ms.is_some();
        self.view.strike(lane, at_ms);
        self.sounds.push(sample);
        self.flash_lane(lane, now_ms);
        let judgement = self.windows.judge(offset);
        self.rate(judgement);
        Some(judgement)
    }

    /// Release a held lane: rate the tail of the nearest held note.
    /// Releasing mid-hold (before the tail window) is a Miss;
    /// a release inside the window judges with the strike tiers.
    /// Returns the judgement, or `None` when no lane is held.
    pub fn release_lane(&mut self, lane: u8, now_ms: u64) -> Option<Judgement> {
        self.advance_to(now_ms);
        let mut best: Option<(usize, u64)> = None;
        for (i, note) in self.pending.iter().enumerate() {
            if !note.holding || note.tail_judged || note.lane != lane {
                continue;
            }
            let tail = note.tail_ms.unwrap_or(note.at_ms);
            let gap = tail.abs_diff(now_ms);
            if best.is_none_or(|(_, seen)| gap < seen) {
                best = Some((i, gap));
            }
        }
        let (i, _) = best?;
        let tail = LongTail {
            head_ms: self.pending[i].at_ms,
            tail_ms: self.pending[i].tail_ms.unwrap_or(self.pending[i].at_ms),
        };
        self.pending[i].holding = false;
        self.pending[i].tail_judged = true;
        self.flash_lane(lane, now_ms);
        let judgement = if tail.is_early(now_ms, &self.windows) {
            Judgement::Miss
        } else {
            tail.judge_release(now_ms, &self.windows)
        };
        self.rate(judgement);
        Some(judgement)
    }

    /// Running score and combo.
    #[must_use]
    pub fn score(&self) -> &Score {
        &self.score
    }

    /// Take queued key-sound sample ids (one per struck note).
    pub fn drain_sounds(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.sounds)
    }

    /// Song over: every head and tail judged and the tail (1.5 s) played
    /// out. Empty charts never finish on their own (nothing to play).
    #[must_use]
    pub fn is_finished(&self, now_ms: u64) -> bool {
        let Some(last) = self
            .pending
            .iter()
            .map(|note| note.tail_ms.unwrap_or(note.at_ms))
            .max()
        else {
            return false;
        };
        now_ms >= last.saturating_add(1500) && self.pending.iter().all(|note| note.done())
    }

    fn rate(&mut self, judgement: Judgement) {
        self.streak.note(judgement, &mut self.pills);
        self.score.apply_rated(judgement, &mut self.pills);
        self.last_word = Some((judgement, self.now_ms));
    }

    /// Stamp a visual-only hit flash on one lane (never read by
    /// timing or scoring).
    fn flash_lane(&mut self, lane: u8, now_ms: u64) {
        if let Some(slot) = self.lane_flash.get_mut(lane as usize) {
            *slot = Some(now_ms);
        }
    }

    /// Whether one lane's flash is still lit at `now_ms`.
    fn flash_lit(&self, lane: u8, now_ms: u64) -> bool {
        self.lane_flash
            .get(lane as usize)
            .copied()
            .flatten()
            .is_some_and(|at| now_ms.saturating_sub(at) < BURST_MS)
    }

    /// Long-note tail beams between head and tail rows (visual only;
    /// heads and releases still rate through `strike_lane` /
    /// `release_lane`). Held heads beam from the judgement line;
    /// untouched holds beam head-to-tail. Beams obey the worn curtain.
    fn paint_hold_tails(&self, frame: &mut Frame<'_>, geom: &HighwayGeometry) {
        for note in &self.pending {
            let Some(tail_ms) = note.tail_ms else {
                continue;
            };
            if note.tail_judged || tail_ms <= self.now_ms {
                continue;
            }
            let Some(top) = y_for_onset(geom, tail_ms, self.now_ms, DEFAULT_LOOKAHEAD_MS) else {
                continue;
            };
            let bottom = if note.holding {
                geom.line_y
            } else {
                let Some(head_y) = y_for_onset(geom, note.at_ms, self.now_ms, DEFAULT_LOOKAHEAD_MS)
                else {
                    continue;
                };
                head_y + NOTE_H
            };
            if top >= bottom || !self.tail_shown(frame, geom, top) {
                continue;
            }
            let column = geom.lane_rect(note.lane);
            fill_rect(
                frame,
                &Rect {
                    x: column.x + column.w as i32 / 2 - 1,
                    y: top,
                    w: 2,
                    h: u32::try_from(bottom - top).unwrap_or(0),
                },
                TAIL_BEAM,
            );
        }
    }

    /// Whether a tail row survives the worn sight-ring curtain (same
    /// span rule as falling notes).
    fn tail_shown(&self, frame: &Frame<'_>, geom: &HighwayGeometry, y: i32) -> bool {
        if self.curtain == LaneCurtain::Full {
            return true;
        }
        let highway = Rect {
            x: 0,
            y: 0,
            w: frame.w as u32,
            h: geom.line_y.max(0) as u32,
        };
        visible_spans(self.curtain, &highway)
            .iter()
            .any(|span| y >= span.y && y < span.y + span.h as i32)
    }

    /// Lit receptor pads plus burst columns on flashed lanes (the
    /// judgement-line key effects: `KeyEffect_Tutorial_%d` siblings in
    /// `Playing1.opi` mark the same per-lane anchors).
    fn paint_hit_bursts(&self, frame: &mut Frame<'_>, geom: &HighwayGeometry, now_ms: u64) {
        for lane in 0..HIGHWAY_LANES {
            if !self.flash_lit(lane, now_ms) {
                continue;
            }
            let column = geom.lane_rect(lane);
            PlayingView::paint_receptor(frame, geom, lane, RECEPTOR_LIT);
            fill_rect(
                frame,
                &Rect {
                    x: column.x + 2,
                    y: geom.line_y - RECEPTOR_H - BURST_H,
                    w: column.w.saturating_sub(4),
                    h: u32::try_from(BURST_H).unwrap_or(0),
                },
                HIT_BURST,
            );
        }
    }

    /// Judgement word above the line with the running combo beneath it
    /// (the original stacks combo under the word; the top-left combo
    /// digits stay as the HUD mirror). Words keep plain white ink.
    fn paint_judgement_word(&self, frame: &mut Frame<'_>, geom: &HighwayGeometry, now_ms: u64) {
        let Some((judgement, at)) = self.last_word else {
            return;
        };
        if now_ms.saturating_sub(at) >= 600 {
            return;
        }
        let word = word_of(judgement);
        let text_w = i32::try_from(text_width(word)).unwrap_or(0);
        draw_text(
            frame,
            frame.w as i32 / 2 - text_w / 2,
            geom.line_y - 40,
            word,
            pack_xrgb(0xFF, 0xFF, 0xFF),
        );
        if self.score.combo > 0 {
            let digits = i32::try_from(number_width(self.score.combo)).unwrap_or(0);
            draw_number(
                frame,
                frame.w as i32 / 2 - digits / 2,
                geom.line_y - 24,
                self.score.combo,
                pack_xrgb(0xFF, 0xFF, 0x40),
            );
        }
    }

    /// Peer life bars (top-right strip): one bar per 4015 member slot,
    /// filled green up to `life / LIFE_MAX` over a dark-red empty bar.
    fn paint_peer_bars(&self, frame: &mut Frame<'_>) {
        let width = i32::try_from(PEER_BAR_WIDTH).unwrap_or(0);
        let count = i32::try_from(self.peers.life_levels().len()).unwrap_or(0);
        let edge = i32::try_from(frame.w).unwrap_or(0);
        let mut x = edge - count * (width + PEER_BAR_GAP) - PEER_BAR_GAP;
        for level in self.peers.life_levels() {
            Self::paint_peer_bar(frame, x.max(0), level);
            x += width + PEER_BAR_GAP;
        }
    }

    /// One empty bar plus its life fill, rising from the bottom.
    fn paint_peer_bar(frame: &mut Frame<'_>, x: i32, life: u16) {
        let height = i32::try_from(PEER_BAR_HEIGHT).unwrap_or(0);
        fill_rect(
            frame,
            &Rect {
                x,
                y: PEER_BAR_TOP,
                w: PEER_BAR_WIDTH,
                h: PEER_BAR_HEIGHT,
            },
            pack_xrgb(0x30, 0x10, 0x10),
        );
        let filled = i32::from(life.min(LIFE_MAX)) * height / i32::from(LIFE_MAX);
        fill_rect(
            frame,
            &Rect {
                x,
                y: PEER_BAR_TOP + height - filled,
                w: PEER_BAR_WIDTH,
                h: u32::try_from(filled).unwrap_or(0),
            },
            pack_xrgb(0x20, 0xE0, 0x20),
        );
    }

    /// Jam edge-bars plus level digits from [`JamState`]: Cool feeds +2,
    /// Good feeds +1, each level spans [`JAM_PROGRESS_PER_LEVEL`] cumulative
    /// progress (`FUN_00570260` / `FUN_00570510`), and a pill-saved Bad
    /// feeds like a Cool (`apply_rated`). The bars fill where the staged
    /// `Jamgauge_right_bar.ojs` / `Jamgauge_left_bar.ojs` shells sit
    /// (VAs `0x64b1b8` / `0x64b1d0`, loader `FUN_004e7850`); the digits
    /// stand in for the `Note_JamNum.ojs` (`0x64c0fc` → `0x50d0ae`) art
    /// until staged figures bind.
    fn paint_jam_gauge(&self, frame: &mut Frame<'_>) {
        let bar_h = Self::jam_bar_height(frame.h);
        if bar_h <= 0 {
            self.paint_jam_level(frame);
            return;
        }
        let filled = jam_fill_rows(
            self.score.jam.level_progress(),
            u32::try_from(bar_h).unwrap_or(0),
        );
        let width = i32::try_from(JAM_BAR_WIDTH).unwrap_or(0);
        let edge = i32::try_from(frame.w).unwrap_or(0);
        Self::paint_jam_bar(frame, JAM_BAR_EDGE_GAP, filled, bar_h);
        Self::paint_jam_bar(frame, edge - width - JAM_BAR_EDGE_GAP, filled, bar_h);
        self.paint_jam_level(frame);
    }

    /// Jam bar height clamped to the frame below the peer strip.
    fn jam_bar_height(frame_h: usize) -> i32 {
        let full = i32::try_from(JAM_BAR_HEIGHT).unwrap_or(0);
        let room = i32::try_from(frame_h).unwrap_or(0) - JAM_BAR_TOP;
        full.min(room).max(0)
    }

    /// One empty jam bar plus its progress fill, rising from the bottom.
    fn paint_jam_bar(frame: &mut Frame<'_>, x: i32, filled: u32, bar_h: i32) {
        let filled = i32::try_from(filled).unwrap_or(bar_h).min(bar_h);
        fill_rect(
            frame,
            &Rect {
                x,
                y: JAM_BAR_TOP,
                w: JAM_BAR_WIDTH,
                h: u32::try_from(bar_h).unwrap_or(0),
            },
            pack_xrgb(0x10, 0x10, 0x30),
        );
        fill_rect(
            frame,
            &Rect {
                x,
                y: JAM_BAR_TOP + bar_h - filled,
                w: JAM_BAR_WIDTH,
                h: u32::try_from(filled).unwrap_or(0),
            },
            pack_xrgb(0xFF, 0x80, 0x00),
        );
    }

    /// Jam level digits, top-centered (digit bodies follow `Note_JamNum`).
    fn paint_jam_level(&self, frame: &mut Frame<'_>) {
        let level = self.score.jam.level();
        let digits = i32::try_from(number_width(level)).unwrap_or(0);
        let center = i32::try_from(frame.w).unwrap_or(0) / 2;
        draw_number(
            frame,
            center - digits / 2,
            PEER_BAR_TOP,
            level,
            pack_xrgb(0xFF, 0xC0, 0x40),
        );
    }

    /// Paint highway, notes, tails, peer bars, jam gauge, score, combo,
    /// bursts, and word.
    pub fn paint(&mut self, frame: &mut Frame<'_>, geom: &HighwayGeometry, now_ms: u64) {
        self.advance_to(now_ms);
        self.view.paint_curtained(frame, geom, self.curtain);
        self.paint_hold_tails(frame, geom);
        self.paint_hit_bursts(frame, geom, now_ms);
        self.paint_peer_bars(frame);
        self.paint_jam_gauge(frame);
        let points = u32::try_from(self.score.points).unwrap_or(u32::MAX);
        draw_number(frame, 16, 16, points, pack_xrgb(0xFF, 0xFF, 0xFF));
        if self.score.combo > 0 {
            draw_number(frame, 16, 32, self.score.combo, pack_xrgb(0xFF, 0xFF, 0x40));
        }
        self.paint_judgement_word(frame, geom, now_ms);
    }
}

#[cfg(test)]
mod live_play_tests {
    use super::*;
    use o2jam_game::{STAT_JAM, STAT_LIFE};

    fn geom() -> HighwayGeometry {
        HighwayGeometry::centered(70, 60)
    }

    #[test]
    fn strike_rates_cool_on_exact_hit() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        assert_eq!(live.score().combo, 1);
        assert_eq!(live.score().points, 200);
        assert!(live.view.visible(&geom()).is_empty());
    }

    #[test]
    fn struck_notes_queue_their_samples() {
        let mut live = LivePlay::new(&[(2, 1000, 7), (2, 1100, 9)]);
        assert_eq!(live.drain_sounds(), Vec::<u32>::new());
        live.strike_lane(2, 1010);
        assert_eq!(live.drain_sounds(), vec![7]);
        assert_eq!(live.drain_sounds(), Vec::<u32>::new());
    }

    #[test]
    fn strike_rates_good_on_early_hit() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        assert_eq!(live.strike_lane(2, 990), Some(Judgement::Good));
        assert_eq!(live.score().points, 100);
    }

    #[test]
    fn strike_rates_bad_on_late_hit() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        assert_eq!(live.strike_lane(2, 1020), Some(Judgement::Bad));
        assert_eq!(live.score().combo, 0);
        assert_eq!(live.score().points, 4);
    }

    #[test]
    fn late_notes_rate_miss_on_advance() {
        let mut live = LivePlay::new(&[(1, 1000, 7)]);
        live.advance_to(1000 + 25 + 1);
        assert_eq!(live.score().counts[3], 1);
        assert_eq!(live.score().points, 0);
    }

    #[test]
    fn empty_press_rates_nothing() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        assert_eq!(live.strike_lane(0, 1000), None);
        assert_eq!(live.score().counts, [0, 0, 0, 0]);
    }

    #[test]
    fn nearest_note_wins_in_lane() {
        let mut live = LivePlay::new(&[(2, 1000, 7), (2, 1100, 9)]);
        assert_eq!(live.strike_lane(2, 1010), Some(Judgement::Good));
        assert_eq!(live.strike_lane(2, 1100), Some(Judgement::Cool));
        assert_eq!(live.score().combo, 2);
    }

    #[test]
    fn paint_shows_score_combo_and_word() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.strike_lane(0, 1000);
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            1100,
        );
        // Score "200" top-left, combo "1" below, "COOL" centered.
        assert_eq!(buf[16 * 70 + 16], pack_xrgb(0xFF, 0xFF, 0xFF));
        assert_eq!(buf[32 * 70 + 17], pack_xrgb(0xFF, 0xFF, 0x40));
        let word_x = 35 - text_width("COOL") as i32 / 2;
        assert_eq!(
            buf[10 * 70 + word_x as usize + 2],
            pack_xrgb(0xFF, 0xFF, 0xFF)
        );
    }

    #[test]
    fn jam_fill_rows_scales_verified_progress_to_height() {
        assert_eq!(jam_fill_rows(0, 24), 0);
        assert_eq!(jam_fill_rows(2, 50), 2);
        assert_eq!(jam_fill_rows(1, 50), 1);
        assert_eq!(jam_fill_rows(50, 24), 24);
        assert_eq!(jam_fill_rows(60, 24), 24);
    }

    #[test]
    fn jam_gauge_empty_shows_shell_without_fill() {
        let mut live = LivePlay::default();
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            0,
        );
        let shell = pack_xrgb(0x10, 0x10, 0x30);
        let fill = pack_xrgb(0xFF, 0x80, 0x00);
        assert_eq!(buf[(JAM_BAR_TOP as usize) * 70 + 2], shell);
        assert!(!buf.contains(&fill));
    }

    #[test]
    fn jam_gauge_cool_and_good_fill_both_edge_bars() {
        let mut live = LivePlay::new(&[(0, 1000, 7), (0, 1100, 7)]);
        live.strike_lane(0, 1000);
        live.strike_lane(0, 1110);
        assert_eq!(live.score().jam.level_progress(), 3);
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            1110,
        );
        let fill = pack_xrgb(0xFF, 0x80, 0x00);
        let bottom = (JAM_BAR_TOP + 24 - 1) as usize;
        assert_eq!(buf[bottom * 70 + 2], fill);
        assert_eq!(buf[bottom * 70 + 66], fill);
        assert_eq!(
            buf[(JAM_BAR_TOP as usize) * 70 + 2],
            pack_xrgb(0x10, 0x10, 0x30)
        );
    }

    #[test]
    fn jam_gauge_grant_advances_level_digits_and_empties_fill() {
        let notes: Vec<(u8, u64, u32)> = (0..25u32)
            .map(|i| (0, 1000 + u64::from(i) * 100, 7))
            .collect();
        let mut live = LivePlay::new(&notes);
        for i in 0..25u32 {
            assert_eq!(
                live.strike_lane(0, 1000 + u64::from(i) * 100),
                Some(Judgement::Cool)
            );
        }
        assert_eq!(
            (live.score().jam.level(), live.score().jam.level_progress()),
            (1, 0)
        );
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            3400,
        );
        let jam_ink = pack_xrgb(0xFF, 0xC0, 0x40);
        // Level 1 glyph top row is 0b010: left pixel keeps the lane skin
        // (x 34 rides odd lane 3).
        assert_eq!(buf[4 * 70 + 34], LANE_SKIN_ODD);
        assert_eq!(buf[4 * 70 + 35], jam_ink);
        let bottom = (JAM_BAR_TOP + 24 - 1) as usize;
        assert_eq!(buf[bottom * 70 + 2], pack_xrgb(0x10, 0x10, 0x30));
    }

    #[test]
    fn jam_gauge_pill_saved_bad_feeds_progress_like_cool() {
        let notes: Vec<(u8, u64, u32)> = (0..16u32)
            .map(|i| (0, 1000 + u64::from(i) * 100, 7))
            .collect();
        let mut live = LivePlay::new(&notes);
        for i in 0..15u32 {
            assert_eq!(
                live.strike_lane(0, 1000 + u64::from(i) * 100),
                Some(Judgement::Cool)
            );
        }
        assert_eq!(live.score().jam.progress(), 30);
        assert_eq!(live.strike_lane(0, 2500 + 20), Some(Judgement::Bad));
        assert_eq!(live.score().jam.progress(), 32);
        assert_eq!(live.score().combo, 15);
    }

    #[test]
    fn finishes_after_tail_with_all_judged() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        assert!(!live.is_finished(3000));
        live.strike_lane(0, 1000);
        assert!(!live.is_finished(2000));
        assert!(live.is_finished(2500));
    }

    #[test]
    fn empty_chart_never_finishes() {
        let live = LivePlay::default();
        assert!(!live.is_finished(u64::MAX));
    }

    #[test]
    fn hold_rates_head_then_tail_release() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        assert_eq!(live.release_lane(2, 2000), Some(Judgement::Cool));
        assert_eq!(live.score().combo, 2);
        assert_eq!(live.score().points, 400);
        assert!(live.is_finished(3500));
    }

    #[test]
    fn hold_early_release_is_a_miss() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        assert_eq!(live.release_lane(2, 1500), Some(Judgement::Miss));
        assert_eq!(live.score().combo, 0);
    }

    #[test]
    fn hold_missing_release_rates_miss_on_advance() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        live.advance_to(2000 + 25 + 1);
        assert_eq!(live.release_lane(2, 2000 + 25 + 1), None);
        assert_eq!(live.score().counts, [1, 0, 0, 1]);
        assert_eq!(live.score().combo, 0);
    }

    #[test]
    fn hold_missing_head_kills_the_tail() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7)]);
        live.advance_to(1000 + 25 + 1);
        assert_eq!(live.release_lane(2, 2000), None);
        assert_eq!(live.score().counts[3], 1);
        assert!(live.is_finished(3500));
    }

    #[test]
    fn empty_tail_degrades_to_tap() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 1000, 7)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        assert_eq!(live.release_lane(2, 1000), None);
        assert!(live.is_finished(2500));
    }

    #[test]
    fn lane_remap_moves_pending_and_view() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.remap_lanes(&LaneMap::mirrored(7));
        assert_eq!(live.strike_lane(6, 1000), Some(Judgement::Cool));
        assert_eq!(live.strike_lane(0, 1000), None);
    }

    #[test]
    fn lane_remap_keeps_time_order() {
        let mut live = LivePlay::new(&[(0, 1100, 7), (6, 1000, 9)]);
        live.remap_lanes(&LaneMap::mirrored(7));
        assert_eq!(live.strike_lane(0, 1000), Some(Judgement::Cool));
        assert_eq!(live.strike_lane(6, 1100), Some(Judgement::Cool));
    }

    #[test]
    fn sudden_curtain_hides_notes_above_the_bottom_fifth() {
        let geom = HighwayGeometry::centered(70, 60);
        let mut view = PlayingView::new(vec![(0, 1500), (0, 2900)]);
        view.advance_to(1000);
        assert_eq!(view.visible(&geom).len(), 2);
        let mut full = vec![0u32; 70 * 60];
        view.paint_curtained(
            &mut Frame {
                buf: &mut full,
                w: 70,
                h: 60,
            },
            &geom,
            LaneCurtain::Full,
        );
        let mut sudden = vec![0u32; 70 * 60];
        view.paint_curtained(
            &mut Frame {
                buf: &mut sudden,
                w: 70,
                h: 60,
            },
            &geom,
            LaneCurtain::Sudden,
        );
        let note = pack_xrgb(0x40, 0xC0, 0xFF);
        assert!(full.contains(&note));
        assert!(!sudden.contains(&note));
    }

    #[test]
    fn hidden_curtain_keeps_notes_at_the_top() {
        let geom = HighwayGeometry::centered(70, 60);
        let mut view = PlayingView::new(vec![(0, 2900)]);
        view.advance_to(1000);
        let mut hidden = vec![0u32; 70 * 60];
        view.paint_curtained(
            &mut Frame {
                buf: &mut hidden,
                w: 70,
                h: 60,
            },
            &geom,
            LaneCurtain::Hidden,
        );
        assert!(hidden.contains(&pack_xrgb(0x40, 0xC0, 0xFF)));
    }

    #[test]
    fn reshuffle_rebuilds_every_plain_measure() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        assert!(live.should_rebuild_map(false));
        assert!(live.should_rebuild_map(false));
        assert!(live.should_rebuild_map(true));
        assert!(!live.should_rebuild_map(true));
    }

    fn stub_words(words: Vec<u32>) -> impl FnMut() -> u32 {
        let mut words = words.into_iter();
        move || words.next().expect("stub randomness exhausted")
    }

    #[test]
    fn mirror_ring_flips_lanes_with_no_curtain() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        let mut never_called = || -> u32 { panic!("mirror draws no randomness") };
        live.apply_skill_ring(SkillRing::Mirror, &mut never_called);
        assert_eq!(live.curtain, LaneCurtain::Full);
        assert_eq!(live.strike_lane(6, 1000), Some(Judgement::Cool));
    }

    #[test]
    fn random_ring_shuffles_lanes_with_no_curtain() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.apply_skill_ring(
            SkillRing::Random,
            &mut stub_words(vec![3, 1, 4, 8, 5, 9, 6]),
        );
        assert_eq!(live.curtain, LaneCurtain::Full);
        // Fixed draws move source lane 0 to lane 3 (golden order).
        assert_eq!(live.strike_lane(3, 1000), Some(Judgement::Cool));
        assert_eq!(live.strike_lane(0, 1000), None);
    }

    #[test]
    fn hidden_ring_keeps_lanes_and_covers_the_bottom() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        live.apply_skill_ring(SkillRing::Hidden, &mut stub_words(vec![]));
        assert_eq!(live.curtain, LaneCurtain::Hidden);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
    }

    #[test]
    fn sudden_ring_keeps_lanes_and_shows_only_the_bottom() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        live.apply_skill_ring(SkillRing::Sudden, &mut stub_words(vec![]));
        assert_eq!(live.curtain, LaneCurtain::Sudden);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
    }

    #[test]
    fn dark_ring_keeps_lanes_and_shows_only_the_middle() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        live.apply_skill_ring(SkillRing::Dark, &mut stub_words(vec![]));
        assert_eq!(live.curtain, LaneCurtain::Dark);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
    }

    #[test]
    fn reverse_dark_ring_keeps_lanes_and_covers_the_middle() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        live.apply_skill_ring(SkillRing::ReverseDark, &mut stub_words(vec![]));
        assert_eq!(live.curtain, LaneCurtain::ReverseDark);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
    }

    #[test]
    fn panic_ring_shuffles_and_rebuilds_each_plain_measure() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.apply_skill_ring(SkillRing::Panic, &mut stub_words(vec![3, 1, 4, 8, 5, 9, 6]));
        assert_eq!(live.curtain, LaneCurtain::Full);
        assert_eq!(live.strike_lane(3, 1000), Some(Judgement::Cool));
        assert!(live.should_rebuild_map(false));
        assert!(live.should_rebuild_map(false));
        assert!(live.should_rebuild_map(true));
        assert!(!live.should_rebuild_map(true));
    }

    #[test]
    fn modifier_byte_two_shuffles_without_curtain() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.apply_modifier_byte(2, &mut stub_words(vec![3, 1, 4, 8, 5, 9, 6]));
        assert_eq!(live.curtain, LaneCurtain::Full);
        assert_eq!(live.strike_lane(3, 1000), Some(Judgement::Cool));
    }

    #[test]
    fn modifier_byte_zero_builds_the_narrow_table_without_curtain() {
        let mut live = LivePlay::new(&[(0, 1000, 7), (5, 1100, 9)]);
        live.apply_modifier_byte(0, &mut stub_words(vec![2, 0, 1]));
        assert_eq!(live.curtain, LaneCurtain::Full);
        // Fixed draws move source lane 0 to lane 2; lanes outside
        // the 4 slots stay put.
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        assert_eq!(live.strike_lane(5, 1100), Some(Judgement::Cool));
    }

    #[test]
    fn unknown_modifier_byte_plays_straight_through() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        live.apply_modifier_byte(9, &mut stub_words(vec![]));
        assert_eq!(live.curtain, LaneCurtain::Full);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
    }

    #[test]
    fn peer_stats_update_forwards_queue_event_and_stores_life() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        let event = live.apply_peer_stats(3, STAT_LIFE, 500);
        assert_eq!(event.map(|forward| forward.category), Some(0x12));
        assert_eq!(event.map(|forward| forward.slot), Some(3));
        assert_eq!(live.peers().life(3), Some(500));
        assert_eq!(live.apply_peer_stats(8, STAT_LIFE, 500), None);
        assert_eq!(live.apply_peer_stats(0, 9, 500), None);
        live.apply_peer_stats(0, STAT_JAM, 7);
        assert_eq!(live.peers().jam(0), Some(7));
    }

    #[test]
    fn peer_rank_broadcast_wires_through_global_match() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        let deliveries = live.apply_peer_ranks([0; PEER_SLOT_COUNT]);
        assert_eq!(deliveries.len(), PEER_SLOT_COUNT);
        assert!(deliveries.iter().all(|found| found.slot.is_none()));
        assert_eq!(live.peers().rank_row(0), Some(0));
    }

    #[test]
    fn paint_shows_peer_life_fill_over_empty_bar() {
        let full_green = pack_xrgb(0x20, 0xE0, 0x20);
        let empty_red = pack_xrgb(0x30, 0x10, 0x10);
        let bar_x =
            70 - PEER_SLOT_COUNT as i32 * (PEER_BAR_WIDTH as i32 + PEER_BAR_GAP) - PEER_BAR_GAP;
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        let mut bare = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut bare,
                w: 70,
                h: 60,
            },
            &geom(),
            0,
        );
        assert_eq!(bare[PEER_BAR_TOP as usize * 70 + bar_x as usize], empty_red);
        live.apply_peer_stats(0, STAT_LIFE, LIFE_MAX);
        let mut filled = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut filled,
                w: 70,
                h: 60,
            },
            &geom(),
            0,
        );
        assert_eq!(
            filled[PEER_BAR_TOP as usize * 70 + bar_x as usize],
            full_green
        );
    }

    #[test]
    fn word_fades_after_timeout() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.strike_lane(0, 1000);
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            1700,
        );
        // The faded word leaves the highway skin, which tints odd lanes.
        let word_x = 35 - text_width("COOL") as i32 / 2;
        let lane = (word_x + 2) / 10;
        let skin = if lane % 2 == 1 {
            LANE_SKIN_ODD
        } else {
            pack_xrgb(0x0A, 0x0A, 0x0A)
        };
        assert_eq!(buf[10 * 70 + word_x as usize + 2], skin);
    }

    #[test]
    fn odd_lanes_carry_skin_tint() {
        let mut live = LivePlay::default();
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            0,
        );
        assert_eq!(buf[5 * 70], pack_xrgb(0x0A, 0x0A, 0x0A));
        assert_eq!(buf[5 * 70 + 12], LANE_SKIN_ODD);
    }

    #[test]
    fn notes_carry_cap_highlight_over_blue_core() {
        let geom = HighwayGeometry::centered(70, 60);
        let mut view = PlayingView::new(vec![(0, 1500)]);
        view.advance_to(1000);
        let mut buf = vec![0u32; 70 * 60];
        view.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom,
        );
        let note_y = 50 - (500.0 * 50.0 / 2000.0) as i32;
        assert_eq!(buf[note_y as usize * 70 + 2], pack_xrgb(0x40, 0xC0, 0xFF));
        assert_eq!(buf[(note_y - 1) as usize * 70 + 2], NOTE_CAP);
    }

    #[test]
    fn receptors_rest_above_the_line() {
        let geom = HighwayGeometry::centered(70, 60);
        let view = PlayingView::default();
        let mut buf = vec![0u32; 70 * 60];
        view.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom,
        );
        assert_eq!(buf[47 * 70 + 2], RECEPTOR_DIM);
        assert_eq!(buf[50 * 70], pack_xrgb(0xE0, 0xE0, 0xE0));
    }

    #[test]
    fn hold_tail_beam_spans_head_to_tail() {
        // Lane 3 sits clear of the edge jam bars and peer strip.
        let mut live = LivePlay::new_holds(&[(3, 1500, 1900, 7)]);
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            1000,
        );
        assert!(buf.contains(&TAIL_BEAM));
        assert_eq!(buf[30 * 70 + 34], TAIL_BEAM);
    }

    #[test]
    fn strike_lights_receptor_and_burst_then_fades() {
        // Lane 3 sits clear of the edge jam bars and peer strip.
        let mut live = LivePlay::new(&[(3, 1000, 7)]);
        live.strike_lane(3, 1000);
        let mut lit = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut lit,
                w: 70,
                h: 60,
            },
            &geom(),
            1050,
        );
        assert_eq!(lit[47 * 70 + 33], RECEPTOR_LIT);
        assert_eq!(lit[40 * 70 + 34], HIT_BURST);
        let mut faded = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut faded,
                w: 70,
                h: 60,
            },
            &geom(),
            1200,
        );
        assert_eq!(faded[47 * 70 + 33], RECEPTOR_DIM);
        assert_eq!(faded[40 * 70 + 34], LANE_SKIN_ODD);
    }

    #[test]
    fn judgement_word_stacks_combo_beneath() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.strike_lane(0, 1000);
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            1100,
        );
        let amber = pack_xrgb(0xFF, 0xFF, 0x40);
        let word_x = 35 - text_width("COOL") as i32 / 2;
        assert_eq!(
            buf[10 * 70 + word_x as usize + 2],
            pack_xrgb(0xFF, 0xFF, 0xFF)
        );
        assert!(
            (24..30).any(|y| (0..70).any(|x| buf[y * 70 + x] == amber)),
            "combo cluster missing under the word"
        );
    }

    #[test]
    fn sudden_curtain_hides_tail_beams() {
        let mut live = LivePlay::new_holds(&[(0, 1500, 2900, 7)]);
        live.set_curtain(LaneCurtain::Sudden);
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            1000,
        );
        assert!(!buf.contains(&TAIL_BEAM));
    }

    #[test]
    fn remap_clears_lane_flashes() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.strike_lane(0, 1000);
        live.remap_lanes(&LaneMap::mirrored(7));
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            1050,
        );
        // Lane 0 receptor and burst rows peek past the edge jam bar.
        assert_eq!(buf[47 * 70 + 7], RECEPTOR_DIM);
        assert_eq!(buf[40 * 70 + 6], pack_xrgb(0x0A, 0x0A, 0x0A));
    }
}

#[cfg(test)]
mod highway_tests {
    use super::*;

    fn geom() -> HighwayGeometry {
        HighwayGeometry::centered(70, 60)
    }

    #[test]
    fn lanes_tile_full_width() {
        let geom = geom();
        assert_eq!(geom.lane_w, 10);
        assert_eq!(geom.lane_rect(0).x, 0);
        assert_eq!(geom.lane_rect(6).x, 60);
        assert_eq!(geom.line_y, 50);
    }

    #[test]
    fn due_note_sits_on_line_and_future_note_tops_out() {
        let mut view = PlayingView::new(vec![(0, 1000), (3, 3000)]);
        let geom = geom();
        view.advance_to(1000);
        // Onset == now is due (struck), not falling.
        assert_eq!(view.visible(&geom), vec![(3, 0)]);
        view.advance_to(2000);
        assert_eq!(view.visible(&geom), vec![(3, 25)]);
    }

    #[test]
    fn old_and_distant_notes_stay_hidden() {
        let mut view = PlayingView::new(vec![(1, 500), (2, 9999)]);
        view.advance_to(1000);
        assert!(view.visible(&geom()).is_empty());
    }

    #[test]
    fn paint_draws_line_and_note_pixels() {
        let geom = HighwayGeometry::centered(70, 60);
        let mut view = PlayingView::new(vec![(0, 1500)]);
        view.advance_to(1000);
        let mut buf = vec![0u32; 70 * 60];
        view.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom,
        );
        // Judgement line spans the lanes; the note sits above it.
        assert_eq!(buf[50 * 70], pack_xrgb(0xE0, 0xE0, 0xE0));
        assert_eq!(buf[50 * 70 + 69], pack_xrgb(0xE0, 0xE0, 0xE0));
        let note_y = 50 - (500.0 * 50.0 / 2000.0) as i32;
        assert_eq!(buf[note_y as usize * 70 + 2], pack_xrgb(0x40, 0xC0, 0xFF));
        assert_eq!(buf[0], pack_xrgb(0x0A, 0x0A, 0x0A));
    }

    #[test]
    fn empty_view_paints_highway_without_notes() {
        let geom = HighwayGeometry::centered(21, 12);
        let view = PlayingView::default();
        assert!(view.visible(&geom).is_empty());
        let mut buf = vec![0u32; 21 * 12];
        view.paint(
            &mut Frame {
                buf: &mut buf,
                w: 21,
                h: 12,
            },
            &geom,
        );
        assert_eq!(buf[10 * 21], pack_xrgb(0xE0, 0xE0, 0xE0));
    }
}

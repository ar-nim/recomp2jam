//! Rating-event sink dispatch (`+0xa14`, 7.55).
//!
//! Behavior paraphrased from the executable (never original bytes).
//! The play object keeps a nullable sink pointer at game `+0xa14`:
//! construction zeroes it, song setup attaches the code address
//! `0x56e170` (file `0x16e170`) during load, and chart reload
//! detaches it (`docs/systems-survey.md` §1). Judgement sites
//! null-check it and dispatch a small integer event code; when it
//! is null the game still scores locally, so offline play never
//! needs it.
//!
//! The dispatcher target `FUN_0056e170` (file `0x16e170`, REA
//! `ev_37e64f21…`, observed) subtracts 1 from the code, bounds it
//! to 0..=15, and jumps through the 16-entry table at `0x56ec5c`
//! (file bytes verified, #81). Arms, each named from its body:
//!
//! - **1** → `0x16e436` — judgement fan-out on the rating word:
//!   Cool `0x10` / Good `0x20` / Bad `0x40` / Miss `0x80` / paired-tail
//!   kill `0x180`. The scoring arm: tier-count + sound relay
//!   (`FUN_0056ddf0`), life/jam gauge deltas (`+0x1820` helpers), the
//!   judgement store `FUN_00570510`, the strike-effect trigger
//!   `FUN_005703f0`, and the long-note split — heads take the direct
//!   arming branch (virtual fx + `+0x17fc`/`+0xb2c` flags, files
//!   `0x16e55e`–`0x16e5ab`), tails on Cool/Good run the combo helper
//!   and self-redispatch code 14 (files `0x16e5b8`–`0x16e5d6`,
//!   `0x16e735`–`0x16e753`), Bad/Miss with any hold bit redispatch
//!   code 14 (files `0x16e8ce`–`0x16e8d0`, `0x16e90c`–`0x16e91f`).
//! - **2** → `0x16e3f3` — autoplay/BGM channel event (B22): when the
//!   payload has no tail bit, the strike-effect trigger runs
//!   (`FUN_005703f0`).
//! - **3, 4, 12, 13, 15** → epilogue — NOP arms (codes 12 and 13 are
//!   still emitted: code 12 at play start from `FUN_00575d70` file
//!   `0x175dfc` with the song-position word, code 13 unused).
//! - **5** → `0x16e1b1` — highway note spawn: unjudged records
//!   (`+0xe & 0x30 == 0`) are painted through `FUN_0056eca0` at the
//!   computed lane y. Emitted by the hold-bonus scan.
//! - **6** → `0x16e210` — armed-hold body repaint: anim rect via
//!   `FUN_0056b880`, clamp to 0..=0x1e0, `FUN_0056eea0` +
//!   two `FUN_0056eca0` paints. Emitted by the pass-event and
//!   hold-bonus scans.
//! - **7** → `0x16e97d` — measure/BGA strip paint on the beat object
//!   (global `0x762b44`, `+0xf6c`): rect `FUN_0056b1c0` + blit
//!   `FUN_0056adc0`. Emitted by the pass-event scan.
//! - **8** → `0x16e9fd` — scroll timebase: the float arg becomes the
//!   global timebase `0x791d70` and the per-lane scroll tables
//!   (`0x791d14`/`0x791d30`/`0x791d34`/`0x791d38`/`0x791d54`) are
//!   recomputed; arg stored at `this+8`. Emitted by `FUN_00575d70`.
//! - **9** → `0x16e9e5` — lane strike-effect trigger only
//!   (`FUN_005703f0` with `lane + 2`). Emitted by the second judge's
//!   walk-advance bookkeeping.
//! - **10** → `0x16ebaa` — song-end marker: `this+0x3c` word = 4.
//!   Emitted by the song-end scan.
//! - **11** → `0x16eb7d` — beat scalar: the float arg through
//!   `FUN_00570f70` into `this+0xfec` (`FUN_0056be30`). Emitted by
//!   `FUN_00575d70` on the `+0x844` float change.
//! - **14** → `0x16ebbd` — combo break + hold teardown: Bad/Miss words
//!   zero the combo counter `+0x3972` (file `0x16ebcf`); the fx
//!   virtual (`0x12`/`0x65`) fires; per-lane hold flags clear
//!   (`+0xb2c`/`+0x17fc` = 0, `+0x17e0` = 1); `this+0x1818` takes the
//!   `FUN_00411e60` getter. Emitted for hold heads struck Bad/Miss and
//!   redispatched from the code-1 arms.
//! - **16** → `0x16e413` — twin of arm 2 (strike-effect trigger when
//!   the payload has no tail bit). Emitted by the scroll-line scan.
//!
//! Only codes 1/9/14 are observed at judgement call sites and carry
//! the server-visible effects below; every other arm drives local
//! paint, scroll, or bookkeeping state only.
//!
//! What the client reports to the server per rating event:
//!
//! - Judgement (code 1, five rating words): the local scorer
//!   `FUN_00570510` (file `0x170510`, REA `ev_73f13ae6…`, observed)
//!   runs for state, then the relay dispatcher `FUN_0056ddf0`
//!   (file `0x16ddf0`, REA `ev_bdc7caf8…`, observed) and the jam
//!   channel decide the network. Jam is reported only when the
//!   scorer returns nonzero — its Cool/Good arms return 1 exactly
//!   on a jam level grant (threshold strictly below progress after
//!   the feed), so the gateway tick `4014` with kind 1 (Jam) fires
//!   on level-ups, never per hit. Life is never reported here: the
//!   tick site `FUN_004eaec0` (file `0xeaec0`, REA `ev_695a26f5…`,
//!   observed) sends kind 0 (Life) on its own cadence, and the
//!   end-of-song tally sends 4016. Bad/Miss and the pill-saved Bad
//!   (which runs the Cool scorer arm without the return check)
//!   never report Jam. Metering the return is therefore the whole
//!   jam/life truth.
//! - Sound (code 9): the note-sound helper `FUN_005703f0`
//!   (file `0x1703f0`, REA `ev_6e3f4371…`, observed) plays the
//!   payload sample; no scorer, no relay, no gateway send.
//! - Hold bookkeeping (code 14 / `0xe`): combo reset on the Bad/Miss
//!   words plus local hold flags; no scorer, no relay, no gateway.
//!
//! This module owns the sink because the sink lives in the score
//! region (file `0x16e170`) and mutates only game state
//! ([`Score`], [`CoolStreak`], [`PillStock`], [`LongNoteBonus`]).
//! Wire encoding stays in `o2jam-net` (`UpdateGameStats`, relay
//! packs); the outcome below carries the abstract intents the
//! caller maps. No `o2jam-game` dependency on `o2jam-net` is added.
//!
//! OPEN with next steps: the exact mode bytes gating the relay
//! dispatcher (`0x77cd33`, `0x77ee84`, `+0xb54`) and the Miss sound
//! gate stay abstracted behind [`RatingSink::relay_live`] — next
//! step is a Ghidra dossier on the writers of those cells plus a
//! live capture of when `0x56ddf0` sends. The sound helper's
//! `+0xb54 == 1` suppression and the `0x778a50 == 1` solo branch are
//! pinned at the read sites; their writers stay OPEN. The pill /
//! long-note mode-`6` exemptions follow the existing crate rule
//! (documented, not modeled).

use crate::{CoolStreak, HoldSpan, Judgement, LongNoteBonus, PillStock, Score};

/// Judgement rating event (code 1).
pub const JUDGEMENT_EVENT: u8 = 1;
/// Sound-only event (code 9).
pub const SOUND_EVENT: u8 = 9;
/// Hold bookkeeping event (code 14 / `0xe`).
pub const HOLD_EVENT: u8 = 14;

/// Rating word for Cool (`0x10`).
pub const RATING_COOL: u32 = 0x10;
/// Rating word for Good (`0x20`).
pub const RATING_GOOD: u32 = 0x20;
/// Rating word for Bad (`0x40`).
pub const RATING_BAD: u32 = 0x40;
/// Rating word for Miss (`0x80`).
pub const RATING_MISS: u32 = 0x80;
/// Rating word for the paired-tail kill (`0x180`).
pub const RATING_TAIL_KILL: u32 = 0x180;

/// Game-side selector the relay dispatcher takes for Cool (`4`).
pub const RELAY_SELECTOR_COOL: u8 = 4;
/// Selector for Good (`3`).
pub const RELAY_SELECTOR_GOOD: u8 = 3;
/// Selector for Bad (`2`): the cool-side branch.
pub const RELAY_SELECTOR_BAD: u8 = 2;
/// Selector for Miss (`1`): the miss-side branch.
pub const RELAY_SELECTOR_MISS: u8 = 1;

/// Solo player-count the jam branch reads (`0x778a50 == 1`).
pub const SOLO_PLAYER_COUNT: u32 = 1;

/// Rating word carried by a judgement event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RatingClass {
    /// Cool (`0x10`): scorer tier 4.
    Cool,
    /// Good (`0x20`): scorer tier 3.
    Good,
    /// Bad (`0x40`): scorer tier 2, or tier 4 when pill-saved.
    Bad,
    /// Miss (`0x80`): scorer tier 1.
    Miss,
    /// Paired-tail kill (`0x180`): scorer tier 1 only.
    TailKill,
}

impl RatingClass {
    /// The payload word this class carries.
    #[must_use]
    pub fn as_word(self) -> u32 {
        match self {
            Self::Cool => RATING_COOL,
            Self::Good => RATING_GOOD,
            Self::Bad => RATING_BAD,
            Self::Miss => RATING_MISS,
            Self::TailKill => RATING_TAIL_KILL,
        }
    }

    /// One of the five rating words the sink handles; anything else
    /// is ignored by the exe (no scorer, no dispatcher, no sound).
    #[must_use]
    pub fn from_word(word: u32) -> Option<Self> {
        match word {
            RATING_COOL => Some(Self::Cool),
            RATING_GOOD => Some(Self::Good),
            RATING_BAD => Some(Self::Bad),
            RATING_MISS => Some(Self::Miss),
            RATING_TAIL_KILL => Some(Self::TailKill),
            _ => None,
        }
    }

    /// The tier [`Score`] applies for this class (the tail kill
    /// scores as Miss).
    #[must_use]
    pub fn judgement(self) -> Judgement {
        match self {
            Self::Cool => Judgement::Cool,
            Self::Good => Judgement::Good,
            Self::Bad => Judgement::Bad,
            Self::Miss | Self::TailKill => Judgement::Miss,
        }
    }

    /// The relay selector this class passes to the dispatcher.
    /// `None` for the tail kill, which never reaches it.
    #[must_use]
    pub fn relay_selector(self) -> Option<u8> {
        match self {
            Self::Cool => Some(RELAY_SELECTOR_COOL),
            Self::Good => Some(RELAY_SELECTOR_GOOD),
            Self::Bad => Some(RELAY_SELECTOR_BAD),
            Self::Miss => Some(RELAY_SELECTOR_MISS),
            Self::TailKill => None,
        }
    }
}

/// Which relay pack an edge stages (`ComboBroken` vs `ComboStarted`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayEdge {
    /// Combo (re)started (`CO2JamCoolPack`).
    Started,
    /// Combo broke (`CO2JamMissPack`).
    Broken,
}

/// The `+0x39b0` edge state behind the dispatcher (`0` = unset,
/// `1` = cool side, `2` = miss side).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RelayComboState {
    /// Round start: neither edge has fired yet.
    #[default]
    Unset,
    /// Cool side (`1`): next Cool edge is silent, Miss breaks.
    CoolSide,
    /// Miss side (`2`): next Miss edge is silent, Cool starts.
    MissSide,
}

/// Channel value for the jam tick (kind 1 on the wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JamReport {
    /// Jam level after the grant (the `4014` kind-`1` value word).
    pub value: u16,
    /// Hit sequence (shared counter, incremented before the read).
    pub sequence: i32,
    /// Running score at the grant.
    pub score: u32,
    /// Long-note bonus accumulator at the grant.
    pub long_note_score: i32,
}

/// Solo jam notification: the local UI queue, never the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JamLocalQueue {
    /// Local member id stamped on the queue forward.
    pub local_id: u8,
}

/// One note-sound trigger through the audio backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundTrigger {
    /// Chart payload word the sample decodes from.
    pub payload: u32,
    /// Lane that struck (0..7).
    pub lane: u8,
    /// Sample id the helper derives (`(payload & 0xfff) - 1`).
    pub sample_id: u32,
}

impl SoundTrigger {
    /// The helper's second argument (`lane + 2`).
    #[must_use]
    pub fn sound_track(self) -> i32 {
        i32::from(self.lane) + 2
    }
}

/// What one rating event did. Every field is `None` (or false) when
/// the event reports nothing — the common case.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RatingOutcome {
    /// Gateway jam tick to send (multiplayer, level-up only).
    pub jam_gateway: Option<JamReport>,
    /// Local queue forward (solo, level-up only; never on the wire).
    pub jam_local: Option<JamLocalQueue>,
    /// Relay combo edge to stage (Live only, on the transition).
    pub relay_edge: Option<RelayEdge>,
    /// Note sound to play through the audio backend.
    pub sound: Option<SoundTrigger>,
    /// Whether the hold tick ran (`+10` bonus, `+1` combo).
    pub hold_tick: bool,
    /// Whether a Cool/Good hold head armed the per-lane hold.
    pub hold_head_armed: bool,
    /// Whether hold bookkeeping ran (the code-14 body).
    pub hold_bookkeeping: bool,
    /// Whether a pill was spent to keep the combo (Bad only).
    pub pill_spent: bool,
    /// Whether a pill was minted by the Cool streak.
    pub pill_granted: bool,
}

/// The rating-event sink state: score, streak, pills, hold bonus,
/// relay edge, and the shared 4014 sequence counter.
#[derive(Debug, Default)]
pub struct RatingSink {
    score: Score,
    streak: CoolStreak,
    pills: PillStock,
    long_note: LongNoteBonus,
    relay_state: RelayComboState,
    /// Shared `4014` counter (`+0x2180`, incremented per send).
    sequence: i32,
    local_id: u8,
    player_count: u32,
    relay_live: bool,
}

impl RatingSink {
    /// A fresh sink: unset relay edge, sequence at zero.
    #[must_use]
    pub fn new(local_id: u8, player_count: u32) -> Self {
        Self {
            local_id,
            player_count,
            ..Self::default()
        }
    }

    /// The running score (points, combo, jam, life).
    #[must_use]
    pub fn score(&self) -> &Score {
        &self.score
    }

    /// Pills currently held.
    #[must_use]
    pub fn pills(&self) -> &PillStock {
        &self.pills
    }

    /// Long-note bonus accumulated.
    #[must_use]
    pub fn long_note(&self) -> &LongNoteBonus {
        &self.long_note
    }

    /// Whether the relay dispatcher is armed (Live mode).
    #[must_use]
    pub fn is_relay_live(&self) -> bool {
        self.relay_live
    }

    /// Arm or disarm the relay dispatcher (Live on/off).
    pub fn set_relay_live(&mut self, live: bool) {
        self.relay_live = live;
    }

    /// Dispatch one event code (`1` / `9` / `14`; others are silent).
    pub fn dispatch(&mut self, code: u8, payload: u32, class_word: u32, lane: u8) -> RatingOutcome {
        match code {
            JUDGEMENT_EVENT => RatingClass::from_word(class_word)
                .map_or_else(RatingOutcome::default, |class| {
                    self.handle_judgement(payload, class, lane)
                }),
            SOUND_EVENT => self.handle_sound(payload, lane),
            HOLD_EVENT => RatingClass::from_word(class_word)
                .map_or_else(RatingOutcome::default, |class| self.handle_hold(class)),
            _ => RatingOutcome::default(),
        }
    }

    /// Apply one judgement (code 1) with its rating word.
    ///
    /// Order mirrors the exe: relay attempt, streak/pill, scorer,
    /// sound, hold branch, then the jam return check.
    pub fn handle_judgement(
        &mut self,
        payload: u32,
        class: RatingClass,
        lane: u8,
    ) -> RatingOutcome {
        let relay_edge = self.attempt_relay_edge(class);
        let level_before = self.score.jam.level();
        let (pill_spent, pill_granted) = self.apply_score_path(payload, class);
        let hold_tick = self.apply_hold_tick(payload, class);
        // The scorer return is the jam grant: Cool/Good only, and
        // only when the level actually rose. The pill-saved Bad
        // feeds like Cool but never reaches the return check.
        let jam = self.jam_report_if_granted(class, level_before);
        let (jam_gateway, jam_local) = self.route_jam(jam);
        let sound = self.judgement_sound(payload, class, lane);
        let hold_head_armed = self.is_hold_head(payload, class);
        let hold_bookkeeping = self.judgement_hold_bookkeeping(payload, class);
        RatingOutcome {
            jam_gateway,
            jam_local,
            relay_edge,
            sound,
            hold_tick,
            hold_head_armed,
            hold_bookkeeping,
            pill_spent,
            pill_granted,
        }
    }

    /// Play one sound-only event (code 9): nothing else happens.
    #[must_use]
    pub fn handle_sound(&self, payload: u32, lane: u8) -> RatingOutcome {
        RatingOutcome {
            sound: Some(SoundTrigger {
                payload,
                lane,
                sample_id: sample_id_of(payload),
            }),
            ..RatingOutcome::default()
        }
    }

    /// Apply one hold bookkeeping event (code 14): combo reset on
    /// Bad/Miss words, otherwise a no-op. No server report.
    pub fn handle_hold(&mut self, class: RatingClass) -> RatingOutcome {
        if matches!(class, RatingClass::Bad | RatingClass::Miss) {
            self.score.combo = 0;
        }
        RatingOutcome {
            hold_bookkeeping: true,
            ..RatingOutcome::default()
        }
    }

    /// Attempt the relay edge for one judgement (Live only).
    fn attempt_relay_edge(&mut self, class: RatingClass) -> Option<RelayEdge> {
        if !self.relay_live {
            return None;
        }
        let want = match class {
            RatingClass::Cool | RatingClass::Good | RatingClass::Bad => RelayComboState::CoolSide,
            RatingClass::Miss => RelayComboState::MissSide,
            RatingClass::TailKill => return None,
        };
        if self.relay_state == want {
            return None;
        }
        self.relay_state = want;
        Some(match want {
            RelayComboState::CoolSide => RelayEdge::Started,
            _ => RelayEdge::Broken,
        })
    }

    /// Run the streak + scorer path and report pill movements.
    ///
    /// Hold tails take the combo-silent scorer variant
    /// ([`Score::apply_rated_without_combo`]): the exe's scorer has no
    /// combo write on the tail arms, so a Cool/Good tail's single `+1`
    /// lands exclusively in [`Self::apply_hold_tick`] (B46) and a
    /// Bad/Miss tail's break lands exclusively in the code-14
    /// bookkeeping arm (file `0x16ebcf`).
    fn apply_score_path(&mut self, payload: u32, class: RatingClass) -> (bool, bool) {
        if class == RatingClass::TailKill {
            self.score.apply_rated(Judgement::Miss, &mut self.pills);
            return (false, false);
        }
        let is_tail = HoldSpan::marks_hold_tail(payload);
        let pills_before = self.pills.count();
        let granted = self.streak.note(class.judgement(), &mut self.pills);
        if is_tail {
            self.score
                .apply_rated_without_combo(class.judgement(), &mut self.pills);
        } else {
            self.score.apply_rated(class.judgement(), &mut self.pills);
        }
        let spent = self.pills.count() < pills_before && !granted;
        (spent, granted)
    }

    /// Run the hold tick on Cool/Good tails (`FUN_00541620`, files
    /// `0x16e5c1`/`0x16e73e`): the tail's one and only combo `+1`.
    fn apply_hold_tick(&mut self, payload: u32, class: RatingClass) -> bool {
        let is_tail = matches!(class, RatingClass::Cool | RatingClass::Good)
            && HoldSpan::marks_hold_tail(payload);
        if is_tail {
            self.run_combo_helper();
        }
        is_tail
    }

    /// Run the combo helper `FUN_00541620` (file `0x141620`) — the
    /// shared adder behind the Cool/Good tail arms (files `0x16e5c1`/
    /// `0x16e73e`) and the per-frame hold-bonus scan (file `0x178200`,
    /// B11): one long-note bonus tick (`+10`) plus `+1` combo with no
    /// cap — the exe ticks the counter once per held frame over the
    /// open span and never saturates it (B46).
    pub fn run_combo_helper(&mut self) {
        self.long_note.hold_tick();
        self.score.combo = self.score.combo.saturating_add(1);
        self.score.max_combo = self.score.max_combo.max(self.score.combo);
    }

    /// Build the jam report when the level rose on Cool/Good.
    fn jam_report_if_granted(
        &mut self,
        class: RatingClass,
        level_before: u32,
    ) -> Option<JamReport> {
        if !matches!(class, RatingClass::Cool | RatingClass::Good) {
            return None;
        }
        if self.score.jam.level() <= level_before {
            return None;
        }
        self.sequence = self.sequence.saturating_add(1);
        Some(JamReport {
            value: saturate_jam(self.score.jam.level()),
            sequence: self.sequence,
            score: saturate_score(self.score.points),
            long_note_score: saturate_long_note(self.long_note.points()),
        })
    }

    /// Route a jam report to gateway vs solo queue.
    fn route_jam(&mut self, jam: Option<JamReport>) -> (Option<JamReport>, Option<JamLocalQueue>) {
        match jam {
            Some(_report) if self.player_count == SOLO_PLAYER_COUNT => (
                None,
                Some(JamLocalQueue {
                    local_id: self.local_id,
                }),
            ),
            other => (other, None),
        }
    }

    /// Sound for a judgement: tails and the tail kill stay silent.
    fn judgement_sound(&self, payload: u32, class: RatingClass, lane: u8) -> Option<SoundTrigger> {
        if class == RatingClass::TailKill || HoldSpan::marks_hold_tail(payload) {
            return None;
        }
        Some(SoundTrigger {
            payload,
            lane,
            sample_id: sample_id_of(payload),
        })
    }

    /// Whether this judgement arms a hold head (Cool/Good only).
    fn is_hold_head(&self, payload: u32, class: RatingClass) -> bool {
        matches!(class, RatingClass::Cool | RatingClass::Good) && HoldSpan::marks_hold_head(payload)
    }

    /// Whether the judgement recurses into bookkeeping: any hold bit
    /// on Bad/Miss, and tails on Cool/Good (heads take the direct
    /// arming branch instead).
    fn judgement_hold_bookkeeping(&self, payload: u32, class: RatingClass) -> bool {
        match class {
            RatingClass::Bad | RatingClass::Miss => payload & HoldSpan::HOLD_HEAD_TEST_MASK != 0,
            RatingClass::Cool | RatingClass::Good => HoldSpan::marks_hold_tail(payload),
            RatingClass::TailKill => false,
        }
    }
}

/// Sample id the helper derives from a payload word.
fn sample_id_of(payload: u32) -> u32 {
    (payload & 0x0fff).saturating_sub(1)
}

/// Saturate a jam level onto its channel word.
fn saturate_jam(level: u32) -> u16 {
    u16::try_from(level).unwrap_or(u16::MAX)
}

/// Saturate the running total onto the wire.
fn saturate_score(points: u64) -> u32 {
    u32::try_from(points).unwrap_or(u32::MAX)
}

/// Saturate the hold bonus onto its wire slot.
fn saturate_long_note(points: u64) -> i32 {
    i32::try_from(points).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HOLD_TICK_BONUS_POINTS;

    fn tap_payload() -> u32 {
        42
    }

    fn head_payload() -> u32 {
        HoldSpan::HOLD_HEAD_TEST_BITS
    }

    fn tail_payload() -> u32 {
        HoldSpan::HOLD_TAIL_TEST_MASK
    }

    fn shipped_tail_payload() -> u32 {
        HoldSpan::HOLD_HEAD_TEST_BITS | HoldSpan::HOLD_TAIL_TEST_MASK
    }

    fn live_sink() -> RatingSink {
        let mut sink = RatingSink::new(3, 2);
        sink.set_relay_live(true);
        sink
    }

    #[test]
    fn cool_level_up_reports_gateway_jam() {
        let mut sink = live_sink();
        for _ in 0..25 {
            sink.handle_judgement(tap_payload(), RatingClass::Cool, 0);
        }
        let outcome = sink.handle_judgement(tap_payload(), RatingClass::Cool, 0);
        let report = outcome
            .jam_gateway
            .expect("the 26th Cool grants jam level 1");
        assert_eq!(report.value, 1);
        assert_eq!(report.sequence, 1);
        assert!(report.score > 0);
        assert_eq!(outcome.jam_local, None);
        // The relay already started on the first Cool, not the grant.
        assert_eq!(outcome.relay_edge, None);
    }

    #[test]
    fn cool_below_threshold_reports_no_jam() {
        let mut sink = live_sink();
        for _ in 0..24 {
            let outcome = sink.handle_judgement(tap_payload(), RatingClass::Cool, 0);
            assert_eq!(outcome.jam_gateway, None);
            assert_eq!(outcome.jam_local, None);
        }
    }

    #[test]
    fn good_level_up_reports_gateway_jam() {
        let mut sink = live_sink();
        for _ in 0..50 {
            sink.handle_judgement(tap_payload(), RatingClass::Good, 0);
        }
        // 50 Goods park progress exactly on the threshold (50).
        let outcome = sink.handle_judgement(tap_payload(), RatingClass::Good, 0);
        assert_eq!(outcome.jam_gateway.map(|report| report.value), Some(1));
    }

    #[test]
    fn solo_cool_level_up_queues_locally_and_never_on_the_wire() {
        let mut sink = RatingSink::new(7, SOLO_PLAYER_COUNT);
        for _ in 0..25 {
            sink.handle_judgement(tap_payload(), RatingClass::Cool, 0);
        }
        let outcome = sink.handle_judgement(tap_payload(), RatingClass::Cool, 0);
        assert_eq!(outcome.jam_local, Some(JamLocalQueue { local_id: 7 }));
        assert_eq!(outcome.jam_gateway, None);
    }

    #[test]
    fn bad_never_reports_jam_even_when_pill_saved_levels_up() {
        let mut sink = live_sink();
        // 24 Cools feed 48 progress. A pill-saved Bad feeds +2
        // without a return check, so it reaches exactly 50.
        for _ in 0..24 {
            sink.handle_judgement(tap_payload(), RatingClass::Cool, 0);
        }
        sink.pills.add(1);
        let saved = sink.handle_judgement(tap_payload(), RatingClass::Bad, 0);
        assert!(saved.pill_spent);
        assert_eq!(saved.jam_gateway, None);
        assert_eq!(saved.jam_local, None);
    }

    #[test]
    fn miss_never_reports_jam_and_resets_progress() {
        let mut sink = live_sink();
        for _ in 0..25 {
            sink.handle_judgement(tap_payload(), RatingClass::Cool, 0);
        }
        let outcome = sink.handle_judgement(tap_payload(), RatingClass::Miss, 0);
        assert_eq!(outcome.jam_gateway, None);
        assert_eq!(outcome.jam_local, None);
        assert_eq!(sink.score().jam.level(), 0);
    }

    #[test]
    fn sound_event_plays_its_payload_sample_and_reports_nothing() {
        let sink = RatingSink::new(0, 2);
        let outcome = sink.handle_sound(0x0107, 2);
        assert_eq!(outcome.jam_gateway, None);
        assert_eq!(outcome.jam_local, None);
        assert_eq!(outcome.relay_edge, None);
        assert!(!outcome.hold_tick);
        let trigger = outcome.sound.expect("code 9 always sounds");
        assert_eq!(trigger.sample_id, 262);
        assert_eq!(trigger.sound_track(), 4);
    }

    #[test]
    fn dispatch_sound_code_reports_no_server_payload() {
        let mut sink = live_sink();
        let outcome = sink.dispatch(SOUND_EVENT, 0x0107, 0, 2);
        assert!(outcome.sound.is_some());
        assert_eq!(outcome.jam_gateway, None);
        assert_eq!(outcome.relay_edge, None);
    }

    #[test]
    fn hold_event_on_bad_or_miss_resets_combo_and_reports_nothing() {
        let mut sink = live_sink();
        sink.handle_judgement(tap_payload(), RatingClass::Cool, 0);
        sink.handle_judgement(tap_payload(), RatingClass::Cool, 0);
        assert_eq!(sink.score().combo, 2);
        let outcome = sink.handle_hold(RatingClass::Bad);
        assert!(outcome.hold_bookkeeping);
        assert_eq!(sink.score().combo, 0);
        assert_eq!(outcome.jam_gateway, None);
        assert_eq!(outcome.relay_edge, None);
        assert_eq!(outcome.sound, None);
    }

    #[test]
    fn hold_event_on_cool_keeps_combo() {
        let mut sink = live_sink();
        sink.handle_judgement(tap_payload(), RatingClass::Cool, 0);
        let outcome = sink.handle_hold(RatingClass::Cool);
        assert!(outcome.hold_bookkeeping);
        assert_eq!(sink.score().combo, 1);
    }

    #[test]
    fn cool_head_arms_hold_and_sounds_without_bookkeeping() {
        let mut sink = live_sink();
        let outcome = sink.handle_judgement(head_payload(), RatingClass::Cool, 1);
        assert!(outcome.hold_head_armed);
        assert!(outcome.sound.is_some());
        assert!(!outcome.hold_tick);
        assert!(!outcome.hold_bookkeeping);
    }

    #[test]
    fn hold_head_continues_combo_once_at_the_judge_arm() {
        // B46: a Cool/Good hold head feeds the combo counter exactly
        // once, at the judge's hold-arming call of the combo helper
        // (exe files 0x17613d/0x176d86) — the sink's own bookkeeping
        // arm is not involved for heads.
        let mut sink = live_sink();
        let before = sink.score().combo;
        let outcome = sink.handle_judgement(head_payload(), RatingClass::Cool, 1);
        assert!(outcome.hold_head_armed);
        assert!(!outcome.hold_tick);
        assert!(!outcome.hold_bookkeeping);
        assert_eq!(sink.score().combo, before + 1);
    }

    #[test]
    fn cool_tail_adds_exactly_one_combo_through_the_sink_helper() {
        // B46: the rated tail's single +1 comes solely from the combo
        // helper on the sink's Cool tail arm (exe file 0x16e5c1); the
        // scorer carries no combo write on the tail arm (file
        // 0x170510), so the tail feeds one combo, not two.
        let mut sink = live_sink();
        let before = sink.score().combo;
        let outcome = sink.handle_judgement(shipped_tail_payload(), RatingClass::Cool, 1);
        assert!(outcome.hold_tick);
        assert!(outcome.hold_bookkeeping);
        assert!(!outcome.hold_head_armed);
        assert_eq!(outcome.sound, None);
        assert_eq!(sink.long_note().points(), HOLD_TICK_BONUS_POINTS);
        assert_eq!(sink.score().combo, before + 1);
    }

    #[test]
    fn good_tail_continues_combo_through_the_good_arm() {
        // B46: the Good tail arm mirrors Cool (exe file 0x16e73e).
        let mut sink = live_sink();
        let before = sink.score().combo;
        let outcome = sink.handle_judgement(shipped_tail_payload(), RatingClass::Good, 1);
        assert!(outcome.hold_tick);
        assert_eq!(sink.score().combo, before + 1);
    }

    #[test]
    fn bad_tail_breaks_combo_only_through_the_bookkeeping_event() {
        // B46: a Bad tail's combo break is the code-14 arm's reset
        // (exe file 0x16ebcf), not a scorer write; the judgement
        // itself only flags the bookkeeping redispatch.
        let mut sink = live_sink();
        sink.handle_judgement(tap_payload(), RatingClass::Cool, 0);
        assert_eq!(sink.score().combo, 1);
        let outcome = sink.handle_judgement(shipped_tail_payload(), RatingClass::Bad, 0);
        assert!(outcome.hold_bookkeeping);
        assert!(!outcome.hold_tick);
        assert_eq!(
            sink.score().combo,
            1,
            "the judgement leaves the counter alone"
        );
        sink.handle_hold(RatingClass::Bad);
        assert_eq!(sink.score().combo, 0, "the code-14 arm breaks it");
    }

    #[test]
    fn hold_tick_bumps_combo_once_per_event_without_a_cap() {
        // B46: the verified per-tick +1 (B11) rides the same helper
        // with no cap — one dispatch, one combo, however long the
        // hold runs.
        let mut sink = live_sink();
        let before = sink.score().combo;
        for _ in 0..500 {
            sink.run_combo_helper();
        }
        assert_eq!(sink.score().combo, before + 500);
    }

    #[test]
    fn bad_head_sounds_with_bookkeeping_but_never_ticks() {
        let mut sink = live_sink();
        let outcome = sink.handle_judgement(head_payload(), RatingClass::Bad, 0);
        assert!(outcome.sound.is_some());
        assert!(outcome.hold_bookkeeping);
        assert!(!outcome.hold_tick);
        assert!(!outcome.hold_head_armed);
    }

    #[test]
    fn bad_tail_is_silent_with_bookkeeping() {
        let mut sink = live_sink();
        let outcome = sink.handle_judgement(tail_payload(), RatingClass::Bad, 0);
        assert_eq!(outcome.sound, None);
        assert!(outcome.hold_bookkeeping);
        assert!(!outcome.hold_tick);
    }

    #[test]
    fn relay_miss_breaks_once_and_cool_starts_once() {
        let mut sink = live_sink();
        let first = sink.handle_judgement(tap_payload(), RatingClass::Cool, 0);
        assert_eq!(first.relay_edge, Some(RelayEdge::Started));
        let again = sink.handle_judgement(tap_payload(), RatingClass::Cool, 0);
        assert_eq!(again.relay_edge, None);
        let broken = sink.handle_judgement(tap_payload(), RatingClass::Miss, 0);
        assert_eq!(broken.relay_edge, Some(RelayEdge::Broken));
        let again_miss = sink.handle_judgement(tap_payload(), RatingClass::Miss, 0);
        assert_eq!(again_miss.relay_edge, None);
        let restarted = sink.handle_judgement(tap_payload(), RatingClass::Good, 0);
        assert_eq!(restarted.relay_edge, Some(RelayEdge::Started));
    }

    #[test]
    fn relay_bad_from_cool_side_is_silent() {
        // Bad's selector takes the cool branch, so from cool it
        // stages nothing; from miss it stages a start (quirk pinned).
        let mut sink = live_sink();
        sink.handle_judgement(tap_payload(), RatingClass::Cool, 0);
        let bad = sink.handle_judgement(tap_payload(), RatingClass::Bad, 0);
        assert_eq!(bad.relay_edge, None);
    }

    #[test]
    fn relay_bad_from_miss_side_stages_a_start() {
        let mut sink = live_sink();
        sink.handle_judgement(tap_payload(), RatingClass::Miss, 0);
        let bad = sink.handle_judgement(tap_payload(), RatingClass::Bad, 0);
        assert_eq!(bad.relay_edge, Some(RelayEdge::Started));
    }

    #[test]
    fn relay_stays_silent_when_live_is_off() {
        let mut sink = RatingSink::new(0, 2);
        let outcome = sink.handle_judgement(tap_payload(), RatingClass::Cool, 0);
        assert_eq!(outcome.relay_edge, None);
        let miss = sink.handle_judgement(tap_payload(), RatingClass::Miss, 0);
        assert_eq!(miss.relay_edge, None);
    }

    #[test]
    fn tail_kill_scores_miss_without_relay_sound_or_jam() {
        let mut sink = live_sink();
        sink.handle_judgement(tap_payload(), RatingClass::Cool, 0);
        let outcome = sink.handle_judgement(tap_payload(), RatingClass::TailKill, 0);
        assert_eq!(outcome.relay_edge, None);
        assert_eq!(outcome.sound, None);
        assert_eq!(outcome.jam_gateway, None);
        assert_eq!(outcome.jam_local, None);
        assert_eq!(sink.score().combo, 0);
        // Counts land in the miss bucket.
        assert_eq!(sink.score().counts[3], 1);
    }

    #[test]
    fn unknown_rating_word_and_code_are_silent() {
        let mut sink = live_sink();
        let unknown = sink.dispatch(JUDGEMENT_EVENT, tap_payload(), 0x11, 0);
        assert_eq!(unknown, RatingOutcome::default());
        let other_code = sink.dispatch(5, tap_payload(), RATING_COOL, 0);
        assert_eq!(other_code, RatingOutcome::default());
    }

    #[test]
    fn fifteen_cools_mint_one_pill_through_the_sink() {
        let mut sink = live_sink();
        let mut granted_at = None;
        for index in 0..15 {
            let outcome = sink.handle_judgement(tap_payload(), RatingClass::Cool, 0);
            if outcome.pill_granted {
                granted_at = Some(index);
            }
        }
        assert_eq!(granted_at, Some(14));
        assert_eq!(sink.pills().count(), 1);
    }

    #[test]
    fn paint_timing_and_nop_codes_report_nothing() {
        // B47: the 16-way dispatcher's remaining arms (2, 5-8, 10,
        // 11, 16, and the NOP arms 3/4/12/13/15) drive only local
        // paint, scroll, and bookkeeping state — no scorer, no jam
        // report, no relay edge, no sound, no hold effect reaches the
        // sink's outcome for them.
        let mut sink = live_sink();
        for code in [2_u8, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 15, 16] {
            let outcome = sink.dispatch(code, tap_payload(), RATING_COOL, 0);
            assert_eq!(outcome, RatingOutcome::default(), "code {code}");
        }
    }

    #[test]
    fn dispatch_judgement_matches_direct_use() {
        let mut sink = live_sink();
        let outcome = sink.dispatch(JUDGEMENT_EVENT, tap_payload(), RATING_COOL, 0);
        assert!(outcome.sound.is_some());
        assert_eq!(outcome.relay_edge, Some(RelayEdge::Started));
    }

    #[test]
    fn jam_report_carries_post_grant_score_and_sequence() {
        let mut sink = live_sink();
        for _ in 0..25 {
            sink.handle_judgement(tap_payload(), RatingClass::Cool, 0);
        }
        let outcome = sink.handle_judgement(tap_payload(), RatingClass::Cool, 0);
        let report = outcome.jam_gateway.expect("grant");
        assert_eq!(report.score, saturate_score(sink.score().points));
        assert_eq!(report.sequence, 1);
        // The second grant needs progress past 100: 24 more Cools
        // park at exactly 100, the 25th crosses it.
        for _ in 0..24 {
            sink.handle_judgement(tap_payload(), RatingClass::Cool, 0);
        }
        let second = sink.handle_judgement(tap_payload(), RatingClass::Cool, 0);
        assert_eq!(second.jam_gateway.map(|r| r.sequence), Some(2));
    }
}

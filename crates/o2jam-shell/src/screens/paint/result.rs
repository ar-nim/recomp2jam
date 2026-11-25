//! Result-screen paint generation: verdict and grade frames,
//! pinned GDI readouts, rank rows, staged OPI pane, portrait chips,
//! and the staged scene painters behind `FUN_004c2390`.
//!
//! Kept behind re-exports in `super::paint`, so callers and the
//! existing scenario guards stay unchanged (#139 paint split).

use super::*;
/// Battle verdict art mounted by the result builder `FUN_004c2390`
/// (file `0xc2390`): `StateResult_Top.ojs` plus
/// `StateResult_Bottom.ojs` plus `Result_WinLose.ojs` (file `0x24a35c`).
/// The result scene paints the `Result_WinLose.ojs` banner only
/// (frame from [`winlose_frame`], `FUN_004c3120`); the play-scene
/// `Winner_image.ojs` banner is a separate art — never re-read in
/// the result region (no rank or role visibility toggle exists on
/// its controls) and its entry rides unloaded in `STATE_PLAYING`
/// (decl `0xE6`, see [`WINNER_IMAGE_SPRITE`]). Winner is rank 1 by
/// team score only (`RoomTeams::winner`); team totals stay
/// server-side (per-member 4014/4016). The grade plate runs on the
/// pinned threshold table ([`grade_frame`]).
pub const RESULT_TOP_SPRITE: &str = "StateResult_Top.ojs";
/// Bottom half mounted by `FUN_004c2390` (file `0xc2390`).
pub const RESULT_BOTTOM_SPRITE: &str = "StateResult_Bottom.ojs";
/// Win/lose verdict mounted by `FUN_004c2390` (file `0xc2390`).
pub const RESULT_WINLOSE_SPRITE: &str = "Result_WinLose.ojs";
/// Winner banner from `STATE_PLAYING` (file `0x24b014`): control
/// `IDC_IMAGE_SPLAY_WINNER` (entry 230, code `0x1B500012`). The
/// decl (`0xE6`) loads entries 0–229 only, so the entry rides
/// unloaded and the sprite never stages in the Playing scene; its
/// `State_Playing_network.bnd` rect 18 (`(265,472)-(389,482)`) is
/// the exit-path variant's placement. Frames anchor `(264,454)`
/// 80×13 (both images; `Challenger_image.ojs` mirrors at
/// `(455,454)`). Kept as the documented-unstaged name — the
/// shell paints it nowhere (a never-staged name is pinned so a
/// future network-variant staging cannot collide silently).
pub const WINNER_IMAGE_SPRITE: &str = "Winner_image.ojs";
/// Event-ticket URL string in the 7.55 exe: file `0x24a2d0`, VA
/// `0x64a2d0` (`ev_5c1c7b322fe96ea565a2d68cf06cee0e7fc23660be9e080fe3494856d2a8f4dc`).
/// The Result enter painter `FUN_004c1b90` gates the web arm
/// (`ev_3bd9ba0997e292073a218baa203e613f5977a6ea27abc5d06aadb094dc46db1a`).
/// This is a static string pin, not a claim that the GET fires in the
/// qualified solo boots: those requested only `hot_time.php`.
#[cfg(test)]
pub const RESULT_TICKET_URL_FILE_OFFSET: u32 = 0x24a2d0;
/// Load-image VA of [`RESULT_TICKET_URL_FILE_OFFSET`] (VA = file + image base).
#[cfg(test)]
pub const RESULT_TICKET_URL_VIRTUAL_ADDRESS: u32 = 0x64a2d0;
/// The retired pin at `0x24a338` names `PointNum.ojs` art, not the URL.
/// Kept only so guards can reject a regression to that string district.
#[cfg(test)]
pub const RESULT_POINT_NUM_ART_FILE_OFFSET: u32 = 0x24a338;

/// Grade plate bound by `STATE_RESULT` rect 7
/// (`IDC_IMAGE_SRESULT_RANKINGFIELD`, `Result_Imgs_Rank.ojs`): eight
/// frames, all anchored (395,531) — 35×41 ×2, 39×41 ×4, 38×41, 79×41
/// (dossier `ev_c61d99f8…`). Frame *n* paints image *n−1*, so grade
/// 1..=8 shows images 0..=7.
pub const RESULT_RANK_SPRITE: &str = "Result_Imgs_Rank.ojs";

/// Life-readout digit font mounted by `FUN_004c2390` at `this+0x10d10`
/// from the Playing archive (`Playing1.opi`): the life-readout
/// painter (VA `0x4c44c0`, file `0xc44c0`) loads this body for every
/// frame-select/draw pair (files `0xc45a6`/`0xc45ce`/`0xc45e7`/`0xc460c`,
/// dossier `ev_48c71a81…`). Ten layers in digit order 0–9, all 18 px
/// tall anchored (0,0) (`ev_a03451ed…`).
pub const RESULT_LIFE_FONT_SPRITE: &str = "PointNum.ojs";
/// Bottom-counter digit font mounted at `this+0x10d0c` from the
/// Interface archive (`Interface1.opi`): the bottom-counter painter
/// (VA `0x4c4630`, file `0xc4630`) loads this body at files
/// `0xc4716`/`0xc473e`/`0xc4757`/`0xc477c` (dossier `ev_63391623…`) —
/// its sole use site image-wide (`ev_8f5c69ce…`). Ten layers in digit
/// order 0–9, 13–14 px tall anchored (0,0).
pub const RESULT_BOTTOM_FONT_SPRITE: &str = "Result_NumFont_Blue.ojs";

/// Cell geometry of the two exe digit-sprite fields (both painters
/// build eight cells as `last − 0xa8 ..= last` in 0x18 (24 px) steps
/// and kern digit 1 by +5: `ev_d4362eae…`).
pub(crate) const RESULT_LIFE_Y: i32 = 77;
pub(crate) const RESULT_LIFE_LAST_X: i32 = 460;
pub(crate) const RESULT_BOTTOM_Y: i32 = 536;
pub(crate) const RESULT_BOTTOM_LAST_X: i32 = 176;
pub(crate) const RESULT_DIGIT_PITCH: i32 = 24;
pub(crate) const RESULT_DIGIT_CELLS: usize = 8;

/// Music-name box interior on the native canvas: the
/// `Result_EachResult.ojs` body (`STATE_RESULT` rect 4
/// `IDC_IMAGE_SRESULT_RESULTTAP`, 800×350 anchored (0,250)) bakes its
/// own pane into this inner box (body-local interior (70,54)–
/// (733,268) plus the (0,250) anchor; the baked frame borders at
/// x 68–69/734–735 and y 52–53/269–270 stay art). The exe's Result
/// fill selects staged body `0x17100001` and its left tab selects
/// `0x17500004`; no chart cover enters this path (#294 T4, files
/// `0xc3172`/`0xc2c8b`). The rect names baked art geometry only.
pub const RESULT_MUSIC_BOX: Rect = Rect {
    x: 70,
    y: 304,
    w: 664,
    h: 215,
};

/// Sprites whose frames the result painter selects one of (the
/// verdict banner plus the grade plate): kept out of the whole-stack
/// backdrop pass so unused frames never bleed through, then painted
/// at the exe-selected frame only.
pub(crate) const RESULT_FRAME_BODIES: [&str; 2] = [RESULT_WINLOSE_SPRITE, RESULT_RANK_SPRITE];

/// Sprites the result builder `FUN_004c2390` (file `0xc2390`) mounts,
/// in load order: top, bottom, then the win/lose verdict.
#[must_use]
pub fn result_loader_sprites() -> [&'static str; 3] {
    [
        RESULT_TOP_SPRITE,
        RESULT_BOTTOM_SPRITE,
        RESULT_WINLOSE_SPRITE,
    ]
}

/// Battle verdict: team standing by server-side total (`RoomTeams`
/// mirrors the 4014/4016 accumulation; `Winner_image.ojs` and
/// `Result_WinLose.ojs` render this same comparison server-side).
/// Solo or team, grades play no part (mapping OPEN).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleVerdict {
    /// Local team holds rank 1 (exe win-flag 1 → frame 1 → image 0).
    Win,
    /// Local team holds rank 2 (exe win-flag 2 → frame 2 → image 1,
    /// the LOSE! banner of the reference battle photo).
    Lose,
    /// Local team ranks third or worse (exe win-flag anything else
    /// → frame 3 → image 2).
    Other,
}

/// Distinct seated teams in seat order (each `TEAMx` button index once).
#[must_use]
pub(crate) fn seated_battle_teams(teams: &RoomTeams) -> Vec<u8> {
    let mut seen = Vec::new();
    for seat in 0..teams.seats() {
        if let Some(team) = teams.team_of(seat) {
            if !seen.contains(&team) {
                seen.push(team);
            }
        }
    }
    seen
}

/// Zero-based standing of `team` among the seated teams: highest
/// combined total first, ties toward the lower team id (the
/// `RoomTeams::winner` first-seat-wins order). `None` while `team`
/// holds no seat.
#[must_use]
pub(crate) fn battle_team_rank(teams: &RoomTeams, team: u8) -> Option<usize> {
    let mut standings: Vec<(u64, u8)> = seated_battle_teams(teams)
        .into_iter()
        .map(|candidate| (teams.team_total(candidate), candidate))
        .collect();
    standings.sort_by(|left, right| right.0.cmp(&left.0).then(left.1.cmp(&right.1)));
    standings.iter().position(|(_, seated)| *seated == team)
}

/// Standing verdict for the local team: rank 1 wins, rank 2 loses,
/// rank 3+ falls to the generic other arm. `None` while the local
/// team is unknown or unseated (empty room, solo with no seat claimed).
#[must_use]
pub fn battle_verdict(teams: &RoomTeams, self_team: Option<u8>) -> Option<BattleVerdict> {
    match battle_team_rank(teams, self_team?)? {
        0 => Some(BattleVerdict::Win),
        1 => Some(BattleVerdict::Lose),
        _ => Some(BattleVerdict::Other),
    }
}

/// Result verdict state the banner painter maps to a WinLose frame
/// (`FUN_004c3120`): solo plays take the solo banner, battles the
/// win-flag frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultVerdict {
    /// Solo play (the exe's player-count branch): frame 4, the solo
    /// banner image.
    Solo,
    /// Team battle with the resolved rank-1 verdict, if any (`None`
    /// while the winner or the local team is unknown — no banner,
    /// the exe's observer frame-0 arm being OPEN, dossier note 10).
    Battle(Option<BattleVerdict>),
}

/// WinLose frame for one verdict (`FUN_004c3120`, dossier
/// `ev_38f32576…`): solo draws frame 4; a battle win-flag byte of 1
/// draws frame 1, 2 draws frame 2, anything else frame 3. The shell
/// resolves the same standing from team totals (rank 1 → `Win`,
/// rank 2 → `Lose`, rank 3+ → `Other`), so every dossier frame is
/// reachable. Frame 4 clamps to the fourth image at draw time like
/// the exe setter.
#[must_use]
pub fn winlose_frame(verdict: ResultVerdict) -> Option<u8> {
    match verdict {
        ResultVerdict::Solo => Some(4),
        ResultVerdict::Battle(Some(BattleVerdict::Win)) => Some(1),
        ResultVerdict::Battle(Some(BattleVerdict::Lose)) => Some(2),
        ResultVerdict::Battle(Some(BattleVerdict::Other)) => Some(3),
        ResultVerdict::Battle(None) => None,
    }
}

/// Grade plate frame chooser (`FUN_004c4dd0`), re-exported from
/// `o2jam-game` where the pinned rule lives next to the ratio and
/// promotion arms it consumes.
pub use o2jam_game::grade_frame;

/// Grade LETTERS stay OPEN: the exe grade strings (`ALL COMBO`,
/// `COOL 95%`, …) carry zero `.text` references (full-image LE32
/// scan, `docs/dialog-strings.md`) — result grades are `.ojs`-driven
/// (the plate frames above), so no letter mapping is painted and
/// the select-music GRADE cell stays blank. Named next step: a
/// live rig capture of the grade plate at controlled score bands
/// (or a code consumer for the strings) before any letter is
/// invented.
///
/// Live grade plate for one finished play (`FUN_004c4dd0`, computed at
/// Result paint): the frame comes from the live [`o2jam_game::Score`]
/// over the pinned reference word ([`o2jam_game::Score::grade_frame`]).
/// `None` paints no plate — no chart word, or the ratio sits under
/// the lowest threshold. The staged frame paints at (395,531)
/// through [`paint_result_scene`].
#[must_use]
pub fn result_grade_frame(score: &o2jam_game::Score, note_count: u16) -> Option<u8> {
    score.grade_frame(note_count)
}

/// White ink of the pinned result fields (the exe text helper runs
/// at color `0xffffff`, Arial).
pub(crate) fn result_number_ink() -> u32 {
    pack_xrgb(0xFF, 0xFF, 0xFF)
}

/// True while `frame` is the exe's native canvas: the pinned result
/// anchors live on the 800×600 surface the exe ran on, and the
/// fixed-size glyph text must not scatter over smaller frames (the
/// window and shot pipeline compose at native and scale once at
/// present time).
#[must_use]
pub(crate) fn frame_is_native(frame: &Frame<'_>) -> bool {
    frame.w == usize::try_from(NATIVE_WIDTH).unwrap_or(0)
        && frame.h == usize::try_from(NATIVE_HEIGHT).unwrap_or(0)
}

/// Played points clamped into the exe's 32-bit score field (the
/// `%8d` read; our u64 accumulator saturates instead of wrapping).
#[must_use]
pub(crate) fn result_score_field(score: &o2jam_game::Score) -> u32 {
    u32::try_from(score.points).unwrap_or(u32::MAX)
}

/// Paint the pinned result numbers from the live `Score`
/// (docs/cresult-anchors.md, top-panel painter `FUN_004c38a0`, file
/// `0xc38a0`): COOL/GOOD/BAD/MISS at y=125, MAX/JAM COMBO at y=130,
/// total score at (605,533) — white GDI text under the baked
/// `StateResult_Top` headers, never digit sprites. Each field draws its
/// decimal from the anchor's left edge: every text call in the top
/// painter, the row painter (`FUN_004c3b40`, file `0xc3b40`), and the
/// width wrapper (`FUN_004c43d0`) passes identical trailing text-helper
/// args `(0, 3, 4)`, and the body of `FUN_0042c730` is not dumped, so
/// left-vs-right anchoring stays OPEN (dossier note 5) and the verified
/// passed x is kept as the left edge. JAM COMBO reads the jam level
/// chain (the wire's u16 `jam` field; mapping OPEN 3); non-native
/// frames paint nothing.
pub(crate) fn paint_result_numbers(frame: &mut Frame<'_>, score: &o2jam_game::Score) {
    if !frame_is_native(frame) {
        return;
    }
    let ink = result_number_ink();
    draw_number(frame, RESULT_COOL_X, RESULT_COUNT_Y, score.counts[0], ink);
    draw_number(frame, RESULT_GOOD_X, RESULT_COUNT_Y, score.counts[1], ink);
    draw_number(frame, RESULT_BAD_X, RESULT_COUNT_Y, score.counts[2], ink);
    draw_number(frame, RESULT_MISS_X, RESULT_COUNT_Y, score.counts[3], ink);
    draw_number(
        frame,
        RESULT_MAX_COMBO_X,
        RESULT_COMBO_Y,
        score.max_combo,
        ink,
    );
    draw_number(
        frame,
        RESULT_JAM_COMBO_X,
        RESULT_COMBO_Y,
        score.jam.level(),
        ink,
    );
    draw_number(
        frame,
        RESULT_SCORE_X,
        RESULT_SCORE_Y,
        result_score_field(score),
        ink,
    );
}

/// Pinned result number anchors (docs/cresult-anchors.md, top-panel
/// painter `FUN_004c38a0`): the judgement counts share row y=125,
/// the combo fields sit at y=130, and the total score reads `%8d`
/// at (605,533).
pub(crate) const RESULT_COOL_X: i32 = 151;
/// GOOD column x.
pub(crate) const RESULT_GOOD_X: i32 = 240;
/// BAD column x.
pub(crate) const RESULT_BAD_X: i32 = 337;
/// MISS column x.
pub(crate) const RESULT_MISS_X: i32 = 426;
/// Judgement-count row y.
pub(crate) const RESULT_COUNT_Y: i32 = 125;
/// MAX COMBO column x.
pub(crate) const RESULT_MAX_COMBO_X: i32 = 505;
/// JAM COMBO column x.
pub(crate) const RESULT_JAM_COMBO_X: i32 = 596;
/// Combo row y.
pub(crate) const RESULT_COMBO_Y: i32 = 130;
/// Total score column x.
pub(crate) const RESULT_SCORE_X: i32 = 605;
/// Total score row y.
pub(crate) const RESULT_SCORE_Y: i32 = 533;
/// `State_Result.bnd` (7 rects, parsed from 7.55 `Interface1.opi`;
/// rect # + corners, paired by code ordinal `(code & 0xFFF) - 1`):
///
/// | # | corners | control |
/// |---|---|---|
/// | 1 | (0,0)-(800,600) | `IDC_BACK_SRESULT_1` `State_Result.ojs` |
/// | 2 | (29,316)-(58,406) | `IDC_IMAGE_SRESULT_RANKTAPFIELD` |
/// | 3 | (29,406)-(58,496) | `IDC_IMAGE_SRESULT_RESULTTAPFIELD` |
/// | 4 | (0,0)-(800,600) | `IDC_IMAGE_SRESULT_RESULTTAP` `Result_EachResult.ojs` |
/// | 5 | (711,569)-(794,593) | `IDC_BUTTON_SRESULT_BACK` |
/// | 6 | (625,570)-(708,592) | `IDC_BUTTON_SRESULT_RETRY` |
/// | 7 | (395,531)-(474,572) | `IDC_IMAGE_SRESULT_RANKINGFIELD` `Result_Imgs_Rank.ojs` |
///
/// Negative evidence: no rect covers any numeric readout — the
/// counts/combos/score rows below are exe-written GDI text (top
/// painter `FUN_004c38a0`, file `0xc38a0`: GDI word fields through
/// the `%5d`/`%4d`/`%3d` width wrapper plus the `%8d` total, and
/// the two 8-cell sprite fields through the 24 px-pitch digit
/// painters with the digit-`1` +5 kern), all re-verified with the
/// `rea` static pass (digit-writer calls with the constants
/// above). So the `RESULT_*` anchors stay exe-pinned, never
/// bnd-sourced; only the buttons and the grade plate have rects.
/// Pinned RETRY button rect (`docs/cresult-anchors.md` rect 6,
/// `IDC_BUTTON_SRESULT_RETRY`, (625,570)-(708,592)): the click
/// fallback while staged art binds no such control.
pub(crate) const RESULT_RETRY_RECT: Rect = Rect {
    x: 625,
    y: 570,
    w: 83,
    h: 22,
};
/// Pinned BACK button rect (rect 5, `IDC_BUTTON_SRESULT_BACK`,
/// (711,569)-(794,593)): same fallback role.
pub(crate) const RESULT_BACK_RECT: Rect = Rect {
    x: 711,
    y: 569,
    w: 83,
    h: 24,
};
/// Staged control id behind [`RESULT_RETRY_RECT`].
pub(crate) const RESULT_RETRY_ID: &str = "IDC_BUTTON_SRESULT_RETRY";
/// Staged control id behind [`RESULT_BACK_RECT`].
pub(crate) const RESULT_BACK_ID: &str = "IDC_BUTTON_SRESULT_BACK";
/// Staged control code behind [`RESULT_RETRY_RECT`] (`0x17202006`:
/// low word `0x006` pairs with `State_Result.bnd` rect 6 by the
/// code-ordinal rule, like the room BACK pairing). Test-pinned:
/// stages the result sweep driver with the real staged codes.
#[cfg(test)]
pub(crate) const RESULT_RETRY_CODE: u32 = 0x1720_2006;
/// Staged control code behind [`RESULT_BACK_RECT`] (`0x17200005`:
/// low word `0x005` pairs with `State_Result.bnd` rect 5).
/// Test-pinned like [`RESULT_RETRY_CODE`].
#[cfg(test)]
pub(crate) const RESULT_BACK_CODE: u32 = 0x1720_0005;

/// Rank number column x (`%d` at x=91, drawn only with 2+ players).
pub(crate) const RESULT_RANK_X: i32 = 91;
/// Nickname column x (at 318+22·row, truncates over 100 chars).
pub(crate) const RESULT_ROW_NAME_X: i32 = 128;
/// Judgements column x (`%5d /%5d /%3d /%3d` at 317+22·row).
pub(crate) const RESULT_ROW_JUDGE_X: i32 = 230;
/// Combo-pair column x (`%4d /%4d`: MAX then JAM COMBO).
pub(crate) const RESULT_ROW_COMBO_X: i32 = 416;
/// Row score column x (`%8d`).
pub(crate) const RESULT_ROW_SCORE_X: i32 = 530;
/// Bonus column x (`+ %d`).
pub(crate) const RESULT_ROW_BONUS_X: i32 = 626;
/// Speed column x (`X 0.5`…`X R`).
pub(crate) const RESULT_ROW_SPEED_X: i32 = 685;
/// Row text y for every column but the nickname (317+22·row: only
/// the judgement and nickname heights are pinned, so the other
/// columns share the judgement line).
pub(crate) const RESULT_ROW_TEXT_Y: i32 = 317;
/// Nickname text y (318+22·row).
pub(crate) const RESULT_ROW_NAME_Y: i32 = 318;
/// Row pitch (row base y = 320+22·row).
pub(crate) const RESULT_ROW_PITCH: i32 = 22;
/// Seats on the member table: rows never exceed eight.
pub(crate) const RESULT_RANK_ROWS_MAX: usize = 8;
/// Nickname display limit (the exe truncates over 100 chars).
pub(crate) const RESULT_ROW_NAME_LIMIT: usize = 100;

/// One result rank-table row from live data (row painter
/// `FUN_004c3b40`, file `0xc3b40`): every column is `Option` — rows
/// without data stay unpainted with documented reasons, never
/// invented values. Rank bytes ride `PeerBars` (nonzero deliveries
/// only); names ride the joined room (`YOU` is local-only, the wire
/// sends no self nickname); judgements/combos/score/reward/speed
/// ride the admitted 4018 member record, falling back to the local
/// `Score`/`LongNoteBonus` only where no record names that seat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultRankRow {
    /// Rank number (`%d` at x=91, 2+ players only).
    pub rank: Option<u8>,
    /// Nickname at x=128 (truncated past 100 display chars).
    pub name: Option<String>,
    /// COOL/GOOD/BAD/MISS at x=230 (record `+0x2142/44/46/48`).
    pub judgements: Option<[u32; 4]>,
    /// MAX then JAM combo at x=416 (record `+0x215c`/`+0x214c`).
    pub combos: Option<(u32, u32)>,
    /// Score at x=530 (record `+0x2160`).
    pub score: Option<u32>,
    /// Reward at x=626 (record `+0x2164`; local hold bonus fallback).
    pub bonus: Option<u64>,
    /// Scroll speed at x=685 (record `+0x2168`, else room code).
    pub speed: Option<u8>,
    /// Member record `+0x31` — the first 4043 `MemberInfo` seat byte:
    /// a 1-based index into the eight constant portrait slots
    /// (`FUN_004c3b40`), doubling as the `FUN_004c49a0` battle-team
    /// bucket (`0` skips the row). `None` where no member record
    /// feeds the row (the local record's `+0x31` writer stays
    /// unpinned — OPEN).
    pub seat_byte: Option<u8>,
}

/// Nickname as shown: first 100 display chars (the exe truncates
/// over 100 chars of CP949; the shell counts chars).
#[must_use]
pub(crate) fn truncate_result_name(name: &str) -> String {
    if name.chars().count() > RESULT_ROW_NAME_LIMIT {
        name.chars().take(RESULT_ROW_NAME_LIMIT).collect()
    } else {
        name.to_owned()
    }
}

/// Combined judgement text (`%5d /%5d /%3d /%3d`).
#[must_use]
pub(crate) fn result_judgements_text(counts: [u32; 4]) -> String {
    format!(
        "{:>5} /{:>5} /{:>3} /{:>3}",
        counts[0], counts[1], counts[2], counts[3]
    )
}

/// Combined combo text (`%4d /%4d`: MAX then JAM).
#[must_use]
pub(crate) fn result_combos_text(combos: (u32, u32)) -> String {
    format!("{:>4} /{:>4}", combos.0, combos.1)
}

/// Row score text (`%8d`).
#[must_use]
pub(crate) fn result_score_text(score: u32) -> String {
    format!("{score:>8}")
}

/// Row bonus text (`+ %d`).
#[must_use]
pub(crate) fn result_bonus_text(bonus: u64) -> String {
    format!("+ {bonus}")
}

/// Text-line y of display row `row` (`None` on overflow).
#[must_use]
pub(crate) fn result_rank_text_y(row: usize) -> Option<i32> {
    let step = i32::try_from(row).ok()?;
    let offset = step.checked_mul(RESULT_ROW_PITCH)?;
    RESULT_ROW_TEXT_Y.checked_add(offset)
}

/// Nickname-line y of display row `row` (`None` on overflow).
#[must_use]
pub(crate) fn result_rank_name_y(row: usize) -> Option<i32> {
    let step = i32::try_from(row).ok()?;
    let offset = step.checked_mul(RESULT_ROW_PITCH)?;
    RESULT_ROW_NAME_Y.checked_add(offset)
}

/// Paint one rank row: each `Some` column draws white GDI text from
/// its pinned left edge, each `None` column stays untouched.
pub(crate) fn paint_result_rank_row(
    frame: &mut Frame<'_>,
    row: usize,
    entry: &ResultRankRow,
    ink: u32,
) {
    let Some(y) = result_rank_text_y(row) else {
        return;
    };
    if let Some(rank) = entry.rank {
        draw_text(frame, RESULT_RANK_X, y, &rank.to_string(), ink);
    }
    if let Some(name) = entry.name.as_ref() {
        if let Some(name_y) = result_rank_name_y(row) {
            let shown = truncate_result_name(name);
            if !shown.is_empty() {
                draw_text(frame, RESULT_ROW_NAME_X, name_y, &shown, ink);
            }
        }
    }
    if let Some(counts) = entry.judgements {
        draw_text(
            frame,
            RESULT_ROW_JUDGE_X,
            y,
            &result_judgements_text(counts),
            ink,
        );
    }
    if let Some(combos) = entry.combos {
        draw_text(
            frame,
            RESULT_ROW_COMBO_X,
            y,
            &result_combos_text(combos),
            ink,
        );
    }
    if let Some(score) = entry.score {
        draw_text(frame, RESULT_ROW_SCORE_X, y, &result_score_text(score), ink);
    }
    if let Some(bonus) = entry.bonus {
        draw_text(frame, RESULT_ROW_BONUS_X, y, &result_bonus_text(bonus), ink);
    }
    if let Some(speed) = entry.speed {
        draw_text(frame, RESULT_ROW_SPEED_X, y, &room_speed_label(speed), ink);
    }
}

/// Paint the rank table (row painter `FUN_004c3b40`): up to eight
/// rows from live data, native canvas only. Empty models paint
/// nothing — the row band keeps its backdrop.
pub(crate) fn paint_result_rank_rows(frame: &mut Frame<'_>, rows: &[ResultRankRow]) {
    if !frame_is_native(frame) {
        return;
    }
    let ink = result_number_ink();
    for (row, entry) in rows.iter().enumerate().take(RESULT_RANK_ROWS_MAX) {
        paint_result_rank_row(frame, row, entry, ink);
    }
}

/// Paint one selected frame of a result body (the verdict banner or
/// the grade plate): frame *n* paints image *n−1* through the
/// saturating [`icon_frame_layer`], blitted 1:1 at the layer's
/// record anchor scaled into the frame — both pinned bodies anchor
/// every frame themselves (banner images at y=190 with their own x;
/// plate frames at (395,531)), so the anchor alone positions them
/// exactly where the exe draws (`docs/cresult-anchors.md`). A
/// missing body, frame, or layer paints nothing.
pub(crate) fn paint_result_body_frame(
    frame: &mut Frame<'_>,
    figures: &HashMap<String, AvatarFigure>,
    sprite: &str,
    body_frame: Option<u8>,
) {
    let Some(body_frame) = body_frame else {
        return;
    };
    let Some(figure) = figures.get(sprite) else {
        return;
    };
    let Some(layer_index) = icon_frame_layer(body_frame, figure.len()) else {
        return;
    };
    let Some(layer) = figure.layer(layer_index) else {
        return;
    };
    let (x, y) = anchor_in_frame(layer.dx, layer.dy, frame.w, frame.h);
    o2jam_render::blit_rgb555_clipped(
        frame,
        &layer.image.view(),
        x,
        y,
        Some(layer.colorkey),
        Some(&result_viewport_rect()),
    );
}

/// Stretch the album cover over one rect, opaque: JPEG pixels carry
/// no transparency, and any 5-bit colorkey would punch holes where
/// cover colors match the key (the render cover blit compares decoded
/// colors). Nearest-neighbor sampling matches `blit_image_in_rect`;
/// pixels clip to the frame. 5-bit channels widen to 8 by
/// bit-replication, matching the render RGB555 expansion.
pub(crate) fn paint_album_cover(frame: &mut Frame<'_>, cover: &Rgb555Image, dest: Rect) {
    if cover.w == 0 || cover.h == 0 || dest.w == 0 || dest.h == 0 {
        return;
    }
    let dest_w = dest.w as usize;
    let dest_h = dest.h as usize;
    let frame_w = i32::try_from(frame.w).unwrap_or(i32::MAX);
    let frame_h = i32::try_from(frame.h).unwrap_or(i32::MAX);
    for row in 0..dest_h {
        let Some(y) = i32::try_from(row)
            .ok()
            .and_then(|offset| dest.y.checked_add(offset))
        else {
            continue;
        };
        if y < 0 || y >= frame_h {
            continue;
        }
        let src_row = row * cover.h / dest_h;
        for col in 0..dest_w {
            let Some(x) = i32::try_from(col)
                .ok()
                .and_then(|offset| dest.x.checked_add(offset))
            else {
                continue;
            };
            if x < 0 || x >= frame_w {
                continue;
            }
            let word = cover.words[src_row * cover.w + col * cover.w / dest_w];
            // 5-bit channels widened by bit-replication (the render
            // RGB555 expansion).
            let (r, g, b) = (
                ((word >> 10) & 31) as u8,
                ((word >> 5) & 31) as u8,
                (word & 31) as u8,
            );
            frame.buf[y as usize * frame.w + x as usize] = pack_xrgb(
                (r << 3) | (r >> 2),
                (g << 3) | (g >> 2),
                (b << 3) | (b >> 2),
            );
        }
    }
}

/// One eight-cell digit-sprite field through the exe painters
/// (`FUN_004c44c0`/`FUN_004c4630`, files `0xc44c0`/`0xc4630`): the
/// decimal digits of `value` (at least one, so 0 paints a 0) fill the
/// rightmost cells at the 24 px pitch, least significant digit
/// rightmost, digit 1 kerned +5 (painter constant, both bodies);
/// leading cells the count does not reach stay unpainted. Each digit
/// selects the font body's layer of the same index (the frame pushed
/// to the frame-select is the raw digit value, not ASCII) and blits
/// at the cell origin under the body colour key. Native canvas only
/// (the exe fields are fixed native pixels); a missing font or digit
/// layer paints nothing.
pub(crate) fn paint_result_digit_field(
    frame: &mut Frame<'_>,
    font: Option<&AvatarFigure>,
    value: u32,
    last_x: i32,
    y: i32,
) {
    let Some(font) = font else {
        return;
    };
    if !frame_is_native(frame) {
        return;
    }
    let mut count = 1usize;
    while count < RESULT_DIGIT_CELLS && value >= 10_u32.pow(count as u32) {
        count += 1;
    }
    for place in 0..count {
        let back = i32::try_from(count - 1 - place).unwrap_or(0);
        let digit = (value / 10_u32.pow((count - 1 - place) as u32)) % 10;
        let kern = if digit == 1 { 5 } else { 0 };
        let x = last_x - back * RESULT_DIGIT_PITCH + kern;
        let Some(layer) = font.layer(digit as usize) else {
            continue;
        };
        let (dx, dy) = anchor_in_frame(x + layer.dx, y + layer.dy, frame.w, frame.h);
        blit_rgb555(frame, &layer.image.view(), dx, dy, Some(layer.colorkey));
    }
}

/// Paint the result backdrop: the staged scene minus the frame
/// bodies ([`RESULT_FRAME_BODIES`]) in file order, so their unused
/// frames never stack through the whole-layer pass. The Top/Bottom
/// panels (`StateResult_Top.ojs` at (0,0), `StateResult_Bottom.ojs`
/// at (0,250), overlapping y=250..316 with Bottom on top) ride this
/// pass: the fill (`FUN_004c3120`, file `0xc3120`) never issues draw
/// calls for the Top/Bottom row objects (`this+0x60[0..1]`), and the
/// per-frame draw-all (slot 9, `FUN_0048c570`) owns every control —
/// so staging their bodies here is the visible equivalent. How the
/// post-mount commit call registers them with that renderer stays
/// OPEN (dossier note 6). Empty art falls back to the flat
/// background; placements without a loaded figure are skipped.
pub(crate) fn paint_result_backdrop(frame: &mut Frame<'_>, art: &SceneArt) {
    if !art.has_figures() {
        fill_background(frame, Screen::Result);
        return;
    }
    let stacked: Vec<ScenePlacement> = art
        .placements
        .iter()
        .filter(|placed| result_placement_visible(placed.rect))
        .filter(|placed| {
            placed
                .sprite
                .as_deref()
                .is_none_or(|sprite| !RESULT_FRAME_BODIES.contains(&sprite))
        })
        .cloned()
        .collect();
    paint_scene_figures(frame, &stacked, &art.figures);
}

/// Paint the pinned result numbers, the rank rows, and the per-row
/// portrait chips (shared by the flat fallback and the staged scene;
/// the numbers and rows are GDI text the exe paints regardless of
/// art). The music-name pane is baked into the staged OPI body and
/// needs no per-song layer; the portrait chips resolve before the
/// rows draw, the exe attach order.
pub(crate) fn paint_result_content(
    frame: &mut Frame<'_>,
    score: &o2jam_game::Score,
    rows: &[ResultRankRow],
    portrait_slots: Option<&[ResultPortraitSlot; 8]>,
) {
    paint_result_numbers(frame, score);
    paint_result_rank_rows(frame, rows);
    paint_result_portrait_chips(frame, rows, portrait_slots);
}

/// Mode `0xf` portrait-slot table (`FUN_004c2390`, files
/// `0xc245d`–`0xc24b8`): the eight constants the builder pushes
/// through `0x421610(obj, entry, 22)` into `obj+0x15c` of each slot
/// (`docs/cresult-anchors.md`, "Peer-row portraits",
/// `ev_ccdb4679…`).
pub const RESULT_PORTRAIT_TABLE_MODE_F: [u16; 8] = [
    0x7c02, 0x7ee2, 0x7be0, 0x4f2a, 0x37bc, 0x001c, 0x4d5a, 0x5923,
];
/// Mode `0x10` portrait-slot table (files `0xc24bf`–`0xc251a`).
pub const RESULT_PORTRAIT_TABLE_MODE_10: [u16; 8] = [
    0xf802, 0xfde2, 0xf7e0, 0x9e6a, 0x6f5c, 0x001c, 0x9a9a, 0xb263,
];
/// Display-mode words selecting a table (field `+0x24` of the
/// display-mode object `0x415cc0([0x76816c+0x80])`, getter file
/// `0x15d2a`).
pub const RESULT_DISPLAY_MODE_TABLE_F: u32 = 0xf;
pub const RESULT_DISPLAY_MODE_TABLE_10: u32 = 0x10;
/// `obj+0x15e` of every slot: `trunc(22/100·256)` (f64 constants
/// 100.0 at `0x644548`, 256.0 at `0x644540`).
pub(crate) const RESULT_PORTRAIT_NUMERATOR: u8 = 56;
/// `obj+0x160` of every slot: `22/100` as f64.
pub(crate) const RESULT_PORTRAIT_RATIO: f64 = 0.22;

/// One constant portrait slot object of the eight at result
/// `this+0x5c`: an image-less sprite subclass (vtable `0x6444d8`,
/// ctor `0x420b00`) built through the `0x40d100` null-name
/// early-exit path — no archive body is ever loaded. Each slot is
/// parameterized only by its mode-table u16; the two derived fields
/// are identical across the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResultPortraitSlot {
    /// Mode-table constant at `obj+0x15c`, consumed as a paint
    /// argument by the subclass draw path (`mov 0x15c(%ecx),%dx`,
    /// file `0x2260f`, onward to `0x421cf0`).
    pub word: u16,
}

impl ResultPortraitSlot {
    /// `obj+0x15e` of every slot: `trunc(22/100·256)` — the
    /// builder's second argument through `0x421610`.
    pub const NUMERATOR: u8 = RESULT_PORTRAIT_NUMERATOR;
    /// `obj+0x160` of every slot: `22/100` as f64.
    pub const RATIO: f64 = RESULT_PORTRAIT_RATIO;
}

/// Build the eight constant portrait slots for a display-mode word.
/// Mode `0xf`/`0x10` write their tables; any other mode writes
/// neither (the exe table keeps its prior contents) — the shell
/// stages no prior table, so `None` resolves no chips.
#[must_use]
pub fn result_portrait_slots(display_mode_word: u32) -> Option<[ResultPortraitSlot; 8]> {
    let table = match display_mode_word {
        RESULT_DISPLAY_MODE_TABLE_F => RESULT_PORTRAIT_TABLE_MODE_F,
        RESULT_DISPLAY_MODE_TABLE_10 => RESULT_PORTRAIT_TABLE_MODE_10,
        _ => return None,
    };
    Some(table.map(|word| ResultPortraitSlot { word }))
}

/// Resolve a member seat byte (member record `+0x31`, the first 4043
/// `MemberInfo` seat byte) to its constant slot: the row painter
/// `FUN_004c3b40` uses the byte as a 1-based, unguarded index into
/// the eight `this+0x5c` objects (byte 1 → element 0, byte 8 →
/// element 7; files `0xc3be2`–`0xc3bfe`). Byte 0 is the
/// `FUN_004c49a0` battle-team skip (file `0xc4a87`), and the exe
/// reads out of bounds for byte 0 and bytes 9+ — the shell resolves
/// neither instead of inventing the out-of-range object.
#[must_use]
pub(crate) fn portrait_slot_for_seat_byte(
    seat_byte: u8,
    slots: &[ResultPortraitSlot; 8],
) -> Option<ResultPortraitSlot> {
    let index = usize::from(seat_byte).checked_sub(1)?;
    slots.get(index).copied()
}

/// Paint the per-row portrait chips (row painter `FUN_004c3b40`):
/// each row's seat byte resolves to its constant slot before the row
/// draws, exactly the exe attach order. The slot art source stays
/// OPEN — the eight objects are image-less and what the subclass
/// draw path renders from `+0x15c` is unsettled (next step per
/// `docs/cresult-anchors.md`: read the `0x6444d8` vtable draw path
/// or capture a populated battle row) — so the resolution runs and
/// nothing visible paints yet; no chip pixels are invented.
pub(crate) fn paint_result_portrait_chips(
    frame: &mut Frame<'_>,
    rows: &[ResultRankRow],
    slots: Option<&[ResultPortraitSlot; 8]>,
) {
    let Some(slots) = slots else {
        return;
    };
    for row in rows.iter().take(RESULT_RANK_ROWS_MAX) {
        let Some(_chip) = row
            .seat_byte
            .and_then(|byte| portrait_slot_for_seat_byte(byte, slots))
        else {
            continue;
        };
        // OPEN: `_chip.word` (`obj+0x15c`) is the paint argument of
        // the slot's draw path; its visual meaning (colour chip,
        // gauge, nothing) lands with the pinned art source.
    }
    let _ = frame;
}

/// Paint the flat results fallback: card, staged-pane ink, the pinned
/// result numbers, the rank rows, and the portrait-chip resolution.
/// The retired text verdict stays gone — the banner is a sprite frame
/// of the staged art.
pub(crate) fn paint_result(
    frame: &mut Frame<'_>,
    score: &o2jam_game::Score,
    rows: &[ResultRankRow],
    portrait_slots: Option<&[ResultPortraitSlot; 8]>,
) {
    paint_card(
        frame,
        pack_xrgb(0x10, 0x10, 0x08),
        pack_xrgb(0xC0, 0xA0, 0x20),
    );
    paint_result_content(frame, score, rows, portrait_slots);
}

/// One staged result paint: everything the scene layers over the
/// backdrop, grouped so the painter keeps a readable signature
/// (`docs/cresult-anchors.md`; the music-name pane is baked into the
/// staged OPI body).
#[derive(Debug, Clone, Copy)]
pub struct ResultPaint<'a> {
    /// Constant portrait slots for the current display mode
    /// (`result_portrait_slots`; `None` where no display-mode word
    /// is pinned — the exe writes no table for other modes).
    pub portrait_slots: Option<[ResultPortraitSlot; 8]>,
    /// Live score behind the pinned GDI numbers.
    pub score: &'a o2jam_game::Score,
    /// Rank-table rows from live data.
    pub rows: &'a [ResultRankRow],
    /// Staged `.bnd` art for the scene.
    pub art: &'a SceneArt,
    /// Verdict the banner frame maps from.
    pub verdict: ResultVerdict,
    /// Grade plate frame (1..=8; `None` paints no plate).
    pub grade: Option<u8>,
    /// Life-readout value for the eight `PointNum.ojs` cells
    /// (y=77, last cell x=460). The exe sums globals `0x77ee58 +
    /// 0x77ee60` (file `0xc394e`); both writers are register-held
    /// (only absolute reads exist — dossier `ev_a1662e22…`), so the
    /// shell maps no live value yet (`docs/cresult-anchors.md` OPEN
    /// 3). `None` paints nothing — no value is invented.
    pub life_readout: Option<u32>,
    /// Bottom-counter value for the eight `Result_NumFont_Blue.ojs`
    /// cells (y=536, last cell x=176). The exe reads the u16 global
    /// `0x76fc66` (file `0xc3a7f`), whose writer is likewise
    /// register-held and whose semantic stays OPEN. `None` paints
    /// nothing.
    pub bottom_counter: Option<u32>,
}

/// Paint the staged result scene (`docs/cresult-anchors.md`): the
/// `.bnd`-bound block art first (minus the frame bodies), then the
/// exe-selected verdict banner and grade plate frames, then the
/// avatar figure, the pinned GDI numbers, and the rank rows on top —
/// the fill order of `FUN_004c3120` (banner, then top-panel painter,
/// then rows). The music-name pane is the staged OPI body's ink.
pub fn paint_result_scene(frame: &mut Frame<'_>, paint: &ResultPaint<'_>) {
    paint_result_backdrop(frame, paint.art);
    paint_result_body_frame(
        frame,
        &paint.art.figures,
        RESULT_WINLOSE_SPRITE,
        winlose_frame(paint.verdict),
    );
    paint_result_body_frame(
        frame,
        &paint.art.figures,
        RESULT_RANK_SPRITE,
        paint.grade.filter(|g| *g >= 1),
    );
    paint_result_digit_field_if_staged(
        frame,
        &paint.art.figures,
        RESULT_LIFE_FONT_SPRITE,
        paint.life_readout,
        RESULT_LIFE_LAST_X,
        RESULT_LIFE_Y,
    );
    paint_result_digit_field_if_staged(
        frame,
        &paint.art.figures,
        RESULT_BOTTOM_FONT_SPRITE,
        paint.bottom_counter,
        RESULT_BOTTOM_LAST_X,
        RESULT_BOTTOM_Y,
    );
    paint_result_content(
        frame,
        paint.score,
        paint.rows,
        paint.portrait_slots.as_ref(),
    );
}

/// [`paint_result_digit_field`] gated on a staged value: the exe
/// fields paint from globals whose writers are register-held (OPEN),
/// so the shell paints a field only when the caller stages a resolved
/// value and never invents one.
pub(crate) fn paint_result_digit_field_if_staged(
    frame: &mut Frame<'_>,
    figures: &HashMap<String, AvatarFigure>,
    sprite: &str,
    value: Option<u32>,
    last_x: i32,
    y: i32,
) {
    if let Some(value) = value {
        paint_result_digit_field(frame, figures.get(sprite), value, last_x, y);
    }
}

/// One six-byte record of the grade/wire table (exe insert helper
/// `FUN_004c5940`, file `0xc5940`, ev `f95a.05cd…`): a two-byte id,
/// a four-byte word, and a one-byte marker. Only the storage shape
/// and one record per grade band are pinned; what each field means
/// to the grade chooser stays OPEN.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResultGradeBandRecord {
    /// Two-byte id/value field.
    pub id: u16,
    /// Four-byte word field.
    pub word: u32,
    /// One-byte marker field.
    pub marker: u8,
}

impl ResultGradeBandRecord {
    /// Exe vector stride for one band record. The field packing
    /// inside those six bytes stays OPEN; the struct keeps the
    /// named fields separate instead of inventing an overlap.
    pub const STORED_BYTES: usize = 6;
}

/// The six grade bands as six-byte records. The band frames and
/// thresholds (stored as thousandths) mirror the pinned chooser
/// ([`o2jam_game::grade_frame`]); the record fields are our storage
/// mirror, not a claim about the exe's field semantics.
#[must_use]
pub fn grade_band_records() -> [ResultGradeBandRecord; 6] {
    let bands = [
        (300u32, 1u16),
        (500, 2),
        (700, 3),
        (800, 4),
        (900, 5),
        (950, 6),
    ];
    std::array::from_fn(|index| {
        let (threshold_thousandths, frame) = bands[index];
        ResultGradeBandRecord {
            id: frame,
            word: threshold_thousandths,
            marker: u8::try_from(frame).unwrap_or(0),
        }
    })
}

/// Native viewport the Result page draws clip against (page-clamp
/// `FUN_0042afd0`, file `0x2afd0`, ev `5092.1018…`). Larger
/// framebuffers still clip art to the 800×600 page the exe painted.
#[must_use]
pub(crate) fn result_viewport_rect() -> Rect {
    Rect {
        x: 0,
        y: 0,
        w: u32::try_from(NATIVE_WIDTH).unwrap_or(0),
        h: u32::try_from(NATIVE_HEIGHT).unwrap_or(0),
    }
}

/// Whether one staged `.bnd` rect lies inside the native page. The
/// Result backdrop drops art authored wholly outside it instead of
/// letting it paint into a larger framebuffer.
#[must_use]
pub(crate) fn result_placement_visible(rect: Rect) -> bool {
    let viewport = result_viewport_rect();
    let right = rect.x.saturating_add(i32::try_from(rect.w).unwrap_or(i32::MAX));
    let bottom = rect.y.saturating_add(i32::try_from(rect.h).unwrap_or(i32::MAX));
    right > viewport.x && rect.x < viewport.x + i32::try_from(viewport.w).unwrap_or(0)
        && bottom > viewport.y
        && rect.y < viewport.y + i32::try_from(viewport.h).unwrap_or(0)
}

/// Scroll step of the pinned list helper (`FUN_0050edb0`, file
/// `0x10edb0`, ev `c5fe.35c5…`): the requested position is clamped
/// into the visible window and at zero, and the signed change is the
/// difference from the previous position. The shell stages no Result
/// scroll list, so this is a pure seam mirror.
#[cfg(test)]
#[must_use]
pub(crate) fn result_scroll_step(requested: i32, previous: i32, max_top: i32) -> (i32, i32) {
    let clamped = requested.min(max_top).max(0);
    (clamped, clamped - previous)
}

#[cfg(test)]
mod result_banner_tests {
    use std::collections::HashMap;

    use o2jam_game::RoomTeams;
    use o2jam_net::frame::decode_frame;
    use o2jam_net::{GameRankUpdate, WireMessage};
    use o2jam_render::{expand_rgb555, pack_xrgb, Frame, Rgb555Image};

    use super::{
        battle_verdict, paint_result_scene, winlose_frame, BattleVerdict, ResultPaint,
        ResultVerdict, RESULT_WINLOSE_SPRITE,
    };
    use crate::avatar::{AvatarFigure, FigureLayer};
    use crate::screens::paint::SceneArt;

    /// Loss for local team 0 behind team 1's higher submitted total,
    /// the same mirror `OnlineSession::apply_score_completed` stages.
    fn losing_teams() -> RoomTeams {
        let mut teams = RoomTeams::new();
        teams.set_team(0, 0);
        teams.set_team(1, 1);
        teams.submit(0, 200);
        teams.submit(1, 50_000);
        teams
    }

    /// Four synthetic banner layers at the pinned frame anchors,
    /// each carrying a distinct solid ink (black colorkey).
    fn banner_art() -> SceneArt {
        let mut figure = AvatarFigure::default();
        for (index, (dx, dy)) in [(100, 190), (100, 190), (151, 190), (126, 190)]
            .into_iter()
            .enumerate()
        {
            figure.push_layer(FigureLayer {
                image: Rgb555Image {
                    w: 2,
                    h: 2,
                    words: vec![0x0001u16 << (index + 1); 4],
                },
                dx,
                dy,
                colorkey: 0x0000,
            });
        }
        SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([(RESULT_WINLOSE_SPRITE.to_owned(), figure)]),
        }
    }

    /// Painted banner ink for one verdict (`frame_ink` in the paint
    /// suite): frame *n* selects image *n−1*.
    fn banner_ink(index: usize) -> u32 {
        let (red, green, blue) = expand_rgb555(0x0001u16 << (index + 1));
        pack_xrgb(red, green, blue)
    }

    /// Paint one whole native result scene and return its pixels.
    fn paint_banner(verdict: ResultVerdict) -> Vec<u32> {
        let art = banner_art();
        let score = o2jam_game::Score::default();
        let mut buffer = vec![0u32; 800 * 600];
        paint_result_scene(
            &mut Frame {
                buf: &mut buffer,
                w: 800,
                h: 600,
            },
            &ResultPaint {
                portrait_slots: None,
                score: &score,
                rows: &[],
                art: &art,
                verdict,
                grade: None,
                life_readout: None,
                bottom_counter: None,
            },
        );
        buffer
    }

    /// Pixel at a banner anchor in an 800×600 result buffer.
    fn ink_at(buffer: &[u32], x: usize, y: usize) -> u32 {
        buffer[y * 800 + x]
    }

    /// Decode one full 4040 frame, the same codec the session's
    /// `apply_rank_update` uses before storing `last_rank_places`.
    fn decoded_places(first: u8, second: u8) -> GameRankUpdate {
        let ranks = GameRankUpdate {
            member_ranks: [first, second, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        };
        let frame = decode_frame(&ranks.encode()).expect("4040 frame");
        GameRankUpdate::decode_frame(&frame).expect("4040 body")
    }

    #[test]
    fn decoded_4040_places_never_change_the_result_banner_frame() {
        // Live caller chain: `main.rs::run_play` → `setup_online` →
        // `run_windowed_session` → `OnlineSession::paint` →
        // `ScreenGame::paint_result` → `ScreenGame::result_verdict` →
        // `battle_verdict` → `paint_result_scene` → `winlose_frame`.
        // The 4040 arm ends earlier: `OnlineSession::apply_rank_update`
        // → `self.last_rank_places`; no banner consumer exists.
        // Evidence: `docs/net-protocol.md` §4c `4040 GameRankUpdate` —
        // handler VA `0x559a60` (file `0x159a60`) stores each place at
        // `+0x217d` (writer `0x1414ad`, REA `ev_4684b913…`); the banner
        // reads win flag `+0x21d2` (files `0xc31ca`/`0xc320c`/
        // `0xc3300`/`0xc3343`) and the rank column reads `+0x10d2c`
        // (`FUN_004c49a0`), with no `+0x217d` reader (`ev_94ff31c9…`).
        // Red proof: wiring the decoded first 4040 place into
        // `battle_verdict` flipped this local team Lose→Win; the
        // verdict assertion failed with `Win` vs `Lose` (frame 1 vs 2)
        // and the painted pixel failed too: win ink `16` vs expected
        // lose ink `33` at (100,190).
        // The mutation was reverted; this guard then passed.
        let teams = losing_teams();
        let verdict = battle_verdict(&teams, Some(0));
        let banner = winlose_frame(ResultVerdict::Battle(verdict));
        assert_eq!(verdict, Some(BattleVerdict::Lose));
        assert_eq!(banner, Some(2));
        let expected_ink = banner_ink(1);
        let baseline_pixels = paint_banner(ResultVerdict::Battle(verdict));
        assert_eq!(ink_at(&baseline_pixels, 100, 190), expected_ink);

        let first = decoded_places(0, 1);
        let second = decoded_places(1, 0);
        assert_ne!(first.member_ranks, second.member_ranks);

        let first_pixels = paint_banner(ResultVerdict::Battle(battle_verdict(&teams, Some(0))));
        let second_pixels = paint_banner(ResultVerdict::Battle(battle_verdict(&teams, Some(0))));
        assert_eq!(ink_at(&first_pixels, 100, 190), expected_ink);
        assert_eq!(ink_at(&second_pixels, 100, 190), expected_ink);
        assert_ne!(ink_at(&second_pixels, 100, 190), banner_ink(0));
        assert_eq!(first_pixels, baseline_pixels);
        assert_eq!(second_pixels, baseline_pixels);

        assert_eq!(battle_verdict(&teams, Some(0)), verdict);
        assert_eq!(
            winlose_frame(ResultVerdict::Battle(battle_verdict(&teams, Some(0)))),
            banner
        );

        // Positive control: a changed 4018 total does move the banner.
        let mut winning = teams;
        winning.submit(0, 100_000);
        let winning_verdict = battle_verdict(&winning, Some(0));
        assert_eq!(winning_verdict, Some(BattleVerdict::Win));
        assert_eq!(
            winlose_frame(ResultVerdict::Battle(winning_verdict)),
            Some(1)
        );
        let winning_pixels = paint_banner(ResultVerdict::Battle(winning_verdict));
        assert_eq!(ink_at(&winning_pixels, 100, 190), banner_ink(0));
        assert_ne!(ink_at(&winning_pixels, 100, 190), expected_ink);
    }
}

#[cfg(test)]
mod result_page_and_band_tests {
    use o2jam_render::{expand_rgb555, pack_xrgb, Frame, Rect, Rgb555Image};

    use super::{
        grade_band_records, paint_album_cover, paint_information_message, paint_result_body_frame,
        paint_result_scene, result_placement_visible, result_scroll_step, result_viewport_rect,
        quit_fallback_rects, BattleVerdict, ResultPaint, ResultVerdict, ResultGradeBandRecord,
        RESULT_RANK_SPRITE,
    };
    use crate::screens::paint::SceneArt;
    use crate::avatar::{AvatarFigure, FigureLayer, PlacementRole, ScenePlacement};
    use std::collections::HashMap;

    /// One opaque 1×1 layer at an arbitrary native anchor.
    fn single_layer_figure(dx: i32, dy: i32, word: u16) -> AvatarFigure {
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image {
                w: 1,
                h: 1,
                words: vec![word],
            },
            dx,
            dy,
            colorkey: 0x0000,
        });
        figure
    }

    /// Framebuffer colour of one RGB555 word.
    fn ink(word: u16) -> u32 {
        let (red, green, blue) = expand_rgb555(word);
        pack_xrgb(red, green, blue)
    }

    /// One score at a pinned band boundary; the live chooser is the
    /// independent source of truth for the plate, not this test.
    fn band_score(points: u64) -> o2jam_game::Score {
        o2jam_game::Score {
            points,
            ..o2jam_game::Score::default()
        }
    }

    /// #1072 record-storage seam. Live chain: `main.rs::run_play` →
    /// `ScreenGame::paint_result` → `ScreenGame::grade` →
    /// `result_grade_frame` → `grade_frame`; the record struct mirrors
    /// the six-byte vector behind the chooser and wire reader.
    /// `seam: the vector stride and band count are the whole storage
    /// contract; field semantics stay OPEN in `docs/pins/result-playing.md`.`
    #[test]
    fn grade_table_stages_six_byte_band_records() {
        let records = grade_band_records();
        assert_eq!(records.len(), 6, "one record per pinned grade band");
        assert_eq!(ResultGradeBandRecord::STORED_BYTES, 6);
        assert_eq!(
            records.map(|record| record.id),
            [1, 2, 3, 4, 5, 6],
            "the record ids must name the six band frames"
        );
        assert_eq!(
            records.map(|record| record.word),
            [300u32, 500, 700, 800, 900, 950],
            "the four-byte words carry the pinned thresholds (thousandths)"
        );
        assert_eq!(
            records.map(|record| record.marker),
            [1, 2, 3, 4, 5, 6],
            "each band carries its own marker"
        );
        // The chooser still climbs the pinned table independently.
        assert_eq!(
            super::result_grade_frame(&band_score(3_135), 51),
            Some(1)
        );
        assert_eq!(
            super::result_grade_frame(&band_score(9_928), 51),
            Some(6)
        );
    }

    /// #1073 page clip. Live chain: `main.rs::run_play` →
    /// `ScreenGame::paint_result` → `paint_result_scene` →
    /// `paint_result_body_frame` → the viewport-gated page blit.
    #[test]
    fn result_body_frame_clips_to_the_active_native_page() {
        let mut figure = single_layer_figure(0, 0, 0x03E0);
        figure.push_layer(FigureLayer {
            image: Rgb555Image {
                w: 1,
                h: 1,
                words: vec![0x7C00],
            },
            dx: 700,
            dy: 10,
            colorkey: 0x0000,
        });
        let mut figures = HashMap::new();
        figures.insert(RESULT_RANK_SPRITE.to_owned(), figure);
        let mut buf = vec![0u32; 1000 * 700];
        paint_result_body_frame(
            &mut Frame {
                buf: &mut buf,
                w: 1000,
                h: 700,
            },
            &figures,
            RESULT_RANK_SPRITE,
            Some(2),
        );
        // The anchor scales to x=875 in this larger framebuffer: it
        // is inside the buffer but outside the 800x600 exe page.
        assert_eq!(buf[12 * 1000 + 875], 0, "art outside the page is clipped");
        assert_eq!(result_viewport_rect().w, 800);
    }

    /// #1074 active-page art. Live chain: `main.rs::run_play` →
    /// `ScreenGame::paint_result` → `paint_result_scene` →
    /// `paint_result_backdrop` → the staged placement's page draw.
    #[test]
    fn result_backdrop_drops_art_authored_outside_the_page() {
        let sprite = "OutsidePage.ojs";
        let mut figures = HashMap::new();
        figures.insert(sprite.to_owned(), single_layer_figure(-200, 0, 0x03E0));
        let art = SceneArt {
            placements: vec![ScenePlacement {
                id: "IDC_OUTSIDE".to_owned(),
                code: 0x1710_0099,
                sprite: Some(sprite.to_owned()),
                rect: Rect {
                    x: 900,
                    y: 0,
                    w: 10,
                    h: 10,
                },
                role: PlacementRole::Single,
            }],
            figures,
        };
        assert!(
            !result_placement_visible(art.placements[0].rect),
            "a rect wholly outside the page is not staged"
        );
        let mut buf = vec![0u32; 800 * 600];
        super::paint_result_backdrop(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &art,
        );
        assert_eq!(buf[0], 0, "its anchor cannot bleed back into the page");
    }

    /// #1075 unclamped caller rect. `seam: the caller-rect arm is a
    /// pure page-draw contract with no live Result caller; the pane's
    /// retired chart-cover path is not revived here (story OPEN).`
    #[test]
    fn result_caller_supplied_rect_skips_the_page_clamp() {
        let cover = Rgb555Image {
            w: 1,
            h: 1,
            words: vec![0x7C00],
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_album_cover(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &cover,
            Rect {
                x: 900,
                y: 0,
                w: 10,
                h: 10,
            },
        );
        assert!(
            buf.iter().all(|pixel| *pixel == 0),
            "an off-page caller rect paints nothing; it is never clamped in"
        );
    }

    /// #1077 fixed-rect notice gate. Live chain: `0x55c0a0` 2016
    /// handler → `FUN_0050ce50` → caption helper → fixed rect; our
    /// painter route is `paint_information_message` →
    /// `paint_cp949_into_rect` → `paint_text_into_rect`.
    #[test]
    fn result_notice_gate_keeps_empty_text_off_the_fixed_rect() {
        let (message, _, _) = quit_fallback_rects();
        let frame_rect = |text: &[u8]| {
            let mut buf = vec![0u32; 800 * 600];
            paint_information_message(
                &mut Frame {
                    buf: &mut buf,
                    w: 800,
                    h: 600,
                },
                None,
                text,
            );
            let width = 800usize;
            let top = usize::try_from(message.y).expect("rect top fits usize");
            let left = usize::try_from(message.x).expect("rect left fits usize");
            let rows = usize::try_from(message.h).expect("rect height fits usize");
            let columns = usize::try_from(message.w).expect("rect width fits usize");
            (top..top + rows)
                .flat_map(|y| (left..left + columns).map(move |x| (y, x)))
                .map(|(y, x)| buf[y * width + x])
                .collect::<Vec<_>>()
        };
        let empty = frame_rect(b"");
        let noticed = frame_rect(b"NOTICE");
        assert!(
            !empty.contains(&ink(0x7FFF)),
            "an empty caption paints no white ink"
        );
        assert!(
            noticed.contains(&ink(0x7FFF)),
            "the fixed rect inks a nonempty caption in white"
        );
    }

    /// #1078 scroll clamp. `seam: pure math mirror of the pinned
    /// scroll helper; the shell stages no Result scroll list.`
    #[test]
    fn result_scroll_clamps_at_the_window_and_zero() {
        assert_eq!(result_scroll_step(9, 1, 4), (4, 3), "window clamp");
        assert_eq!(result_scroll_step(-5, 2, 4), (0, -2), "zero clamp");
        assert_eq!(result_scroll_step(3, 1, 4), (3, 2), "signed change");
    }

    /// Tier-3 paint legs need a full ResultPaint; this keeps the
    /// seam tests independent of any staged archive.
    #[test]
    fn result_seam_scene_paints_without_staged_art() {
        let mut figures = HashMap::new();
        figures.insert(RESULT_RANK_SPRITE.to_owned(), single_layer_figure(395, 531, 0x7C00));
        let art = SceneArt {
            placements: Vec::new(),
            figures,
        };
        let score = o2jam_game::Score::default();
        let mut buf = vec![0u32; 800 * 600];
        paint_result_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &ResultPaint {
                portrait_slots: None,
                score: &score,
                rows: &[],
                art: &art,
                verdict: ResultVerdict::Battle(Some(BattleVerdict::Other)),
                grade: None,
                life_readout: None,
                bottom_counter: None,
            },
        );
        assert_eq!(buf[531 * 800 + 395], 0, "no grade stages no plate");
    }
}

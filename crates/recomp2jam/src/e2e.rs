//! End-to-end window walk: boot to Result through every landed State.
//!
//! Ticket #23 drives the same [`ScreenGame`] seam the winit window
//! runs (headless: no display, no gateway, null audio), records one
//! trace step per input with its screen and state hash, captures one
//! frame per step, and reports the Result-instant hash. The walk
//! passes Avi, Planet (plus the Options dialog), Channel, Room,
//! Waiting (plus all six side states), the music-select pick,
//! Loading, Playing, and Result, then exercises the Result retry and
//! leave edges back into the lobby.
//!
//! Frames stay in memory unless an evidence directory is given: the
//! tests assert on the trace alone, while
//! [`write_e2e_evidence`] writes one PNG per step plus `trace.json`
//! for the smoke log (`docs/smoke/e2e-walk.md`). Inputs are synthetic
//! seeds; the install's archives and song catalog stage underneath
//! when present, and the flat fallbacks stay when it is hidden.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use o2jam_game::RoomTeams;
use o2jam_net::play::SetRoomMusic;
use o2jam_net::{ChannelState, GameMode, JoinedRoom, RoomListEntry, RoomMember, RoomSlot};
use o2jam_render::{Frame, NATIVE_HEIGHT, NATIVE_WIDTH};
use o2jam_shell::screens::side::SideScreen;
use o2jam_shell::{GameLoop, MenuKey, Screen, ScreenGame};
use serde::Serialize;

use crate::loading::{loading_phase_status, LOADING_GATE_STEPS};
use crate::shots::encode_png;

/// Walk frame width in pixels (matches the shots oracle).
pub const E2E_FRAME_W: usize = 400;
/// Walk frame height in pixels (matches the shots oracle).
pub const E2E_FRAME_H: usize = 300;
/// Host song pick the walk commits (synthetic id, validated pick).
const E2E_SONG_ID: u16 = 757;
/// Host difficulty pick (2 = HX, a pinned `Difficulty` code).
const E2E_DIFFICULTY: u8 = 2;
/// Host scroll-speed pick.
const E2E_SPEED: u8 = 1;
/// Lane struck for the walk tap.
const E2E_TAP_LANE: usize = 2;
/// Walk tap onset in song milliseconds.
const E2E_TAP_ONSET_MS: u64 = 5_000;
/// Lane held for the walk long note.
const E2E_HOLD_LANE: usize = 0;
/// Long-note head onset in song milliseconds.
const E2E_HOLD_HEAD_MS: u64 = 5_000;
/// Long-note tail in song milliseconds.
const E2E_HOLD_TAIL_MS: u64 = 5_400;
/// Sample id carried by both walk notes.
const E2E_SAMPLE_ID: u32 = 7;
/// Song clock the walk paints the Playing instant at.
const E2E_PLAYING_PAINT_MS: u64 = 5_100;
/// Song clock the walk paints the Result instant at.
const E2E_RESULT_PAINT_MS: u64 = 7_000;
/// Menu paint clock (below the splash auto-advance hold).
const E2E_MENU_PAINT_MS: u64 = 0;
/// Gear-disc fallback click (native 729,580 per the shell guard).
const E2E_GEAR_CLICK: (i32, i32) = (729, 580);
/// Lit-emblem fallback click (native 136,88 per the shell guards).
const E2E_EMBLEM_CLICK: (i32, i32) = (136, 88);
/// Side states toured from Waiting (the shots visit order).
const E2E_SIDE_TOUR: [SideScreen; 6] = [
    SideScreen::Shop,
    SideScreen::MusicShop,
    SideScreen::MyRoom,
    SideScreen::Lanking,
    SideScreen::Payment,
    SideScreen::Tutorial,
];

/// One walk step: the input taken plus the observable state after it.
#[derive(Debug, Clone, Serialize)]
pub struct E2eStep {
    /// Replay order (strictly increasing from zero).
    pub sequence: u64,
    /// The `ScreenGame` call this step drove, in words.
    pub action: String,
    /// Screen showing after the input.
    pub screen: String,
    /// Side overlay showing after the input, if any.
    pub side: Option<String>,
    /// Observable state hash after the input.
    pub state_hash: u64,
    /// The step's frame carried ink (never a flat fill).
    pub frame_varied: bool,
}

/// The walked story: every step plus the Result-instant proof.
#[derive(Debug, Clone, Serialize)]
pub struct E2eReport {
    /// Ordered steps from boot splash to the lobby return.
    pub steps: Vec<E2eStep>,
    /// Screen the walk ends on (Room, via retry then leave).
    pub final_screen: String,
    /// State hash at the walk end.
    pub final_hash: u64,
    /// State hash at the Result instant (the walk's proof).
    pub result_hash: u64,
}

/// Drive the boot-to-Result walk and return its trace.
///
/// Synthetic seeds only, so the walk runs with `install-root`
/// present and hidden. The PNG form lives in [`write_e2e_evidence`].
///
/// # Errors
///
/// Any unexpected screen (broken wiring between landed States).
#[cfg(test)]
fn drive_e2e_walk() -> anyhow::Result<E2eReport> {
    drive_with_evidence(None)
}

/// Drive the walk while writing one PNG per step plus `trace.json`
/// into `out_dir` (created when missing).
///
/// # Errors
///
/// Filesystem failures, or whatever the walk wiring reports.
pub fn write_e2e_evidence(out_dir: &Path) -> anyhow::Result<E2eReport> {
    std::fs::create_dir_all(out_dir)?;
    drive_with_evidence(Some(out_dir.to_owned()))
}

/// Shared engine: seed the game, walk it, return the trace.
fn drive_with_evidence(evidence_dir: Option<PathBuf>) -> anyhow::Result<E2eReport> {
    E2eWalker::seeded(evidence_dir).and_then(E2eWalker::walk)
}

/// The walk driver: one seeded game plus its growing trace.
struct E2eWalker {
    game: ScreenGame,
    steps: Vec<E2eStep>,
    result_hash: Option<u64>,
    evidence_dir: Option<PathBuf>,
}

impl E2eWalker {
    /// Seed the lobby lists, the waiting duo, and the walk chart.
    ///
    /// # Errors
    ///
    /// The armed loading status never fails to build here.
    fn seeded(evidence_dir: Option<PathBuf>) -> anyhow::Result<Self> {
        let mut game = ScreenGame::new();
        game.load_options(&install_root());
        seed_lobby_lists(&mut game);
        game.set_joined_room(waiting_duo());
        game.load_chart_holds(walk_chart());
        crate::shots::stage_real_art(&mut game);
        // The song pick resolves its cover from the catalog (the live
        // session stages the same rows); without this the loading gate
        // paints the MusicLoading.ojs placeholder instead of the OJN art.
        crate::stage_installed_song_catalog(&mut game);
        crate::shots::stage_avatar_figures(&mut game);
        game.set_battle_teams(walk_battle_teams());
        game.set_battle_self_team(Some(0));
        game.stage_scripted_ranks(WALK_RANK_GLOBAL, WALK_RANK_STORED, WALK_RANK_PACKET);
        Ok(Self {
            game,
            steps: Vec::new(),
            result_hash: None,
            evidence_dir,
        })
    }

    /// Run every walk phase in player order and return the report.
    ///
    /// # Errors
    ///
    /// Any phase landing off its screen.
    fn walk(mut self) -> anyhow::Result<E2eReport> {
        self.record("boot-shows-splash", E2E_MENU_PAINT_MS)?;
        self.dismiss_splash()?;
        self.tour_options_dialog()?;
        self.enter_channel()?;
        self.enter_room_lobby()?;
        self.enter_waiting_room()?;
        self.tour_side_states()?;
        self.pick_host_song()?;
        self.run_loading_gate()?;
        self.play_chart_to_result()?;
        self.exercise_result_edges()?;
        let report = E2eReport {
            final_screen: self.game.screen_name().to_owned(),
            final_hash: self.game.state_hash(),
            steps: std::mem::take(&mut self.steps),
            result_hash: self.result_hash.unwrap_or_default(),
        };
        write_trace_json(self.evidence_dir.as_deref(), &report)?;
        Ok(report)
    }

    /// Record one step: paint the frame, hash the state, stash the PNG.
    ///
    /// # Errors
    ///
    /// Filesystem failures while writing evidence PNGs.
    fn record(&mut self, action: &str, now_ms: u64) -> anyhow::Result<()> {
        let rgb = paint_walk_frame(&mut self.game, now_ms);
        let frame_varied = frame_holds_ink(&rgb);
        if let Some(dir) = self.evidence_dir.clone() {
            let bytes = encode_png(E2E_FRAME_W as u32, E2E_FRAME_H as u32, &rgb);
            std::fs::write(
                dir.join(format!("{:02}-{action}.png", self.steps.len())),
                &bytes,
            )?;
        }
        self.steps.push(E2eStep {
            sequence: self.steps.len() as u64,
            action: action.to_owned(),
            screen: self.game.screen_name().to_owned(),
            side: self.game.side().map(|side| format!("{side:?}")),
            state_hash: self.game.state_hash(),
            frame_varied,
        });
        Ok(())
    }

    /// Require the game to show `want`, naming the broken edge.
    ///
    /// # Errors
    ///
    /// The game shows any other screen.
    fn require_screen(&self, want: Screen, edge: &str) -> anyhow::Result<()> {
        if self.game.screen() != want {
            anyhow::bail!("e2e {edge}: at {:?}, want {want:?}", self.game.screen());
        }
        Ok(())
    }

    /// Dismiss the splash onto the bare board with the pinned Escape
    /// edge (lane keys keep the splash since #356).
    ///
    /// # Errors
    ///
    /// The dismiss edge misses Planet.
    fn dismiss_splash(&mut self) -> anyhow::Result<()> {
        self.game.menu_key(MenuKey::Back, true);
        self.require_screen(Screen::Planet, "splash-dismiss")?;
        self.record("escape-dismisses-splash", E2E_MENU_PAINT_MS)
    }

    /// Open the Options dialog through the gear disc, then Escape out.
    ///
    /// # Errors
    ///
    /// The gear or the Escape edge misses.
    fn tour_options_dialog(&mut self) -> anyhow::Result<()> {
        self.game.click_planet(E2E_GEAR_CLICK.0, E2E_GEAR_CLICK.1);
        if self.game.planet_modal().is_none() {
            anyhow::bail!("e2e options-open: gear click opened no modal");
        }
        self.record("gear-disc-opens-options", E2E_MENU_PAINT_MS)?;
        self.game.menu_key(MenuKey::Back, true);
        if self.game.planet_modal().is_some() {
            anyhow::bail!("e2e options-close: Escape left the modal open");
        }
        self.record("escape-closes-options", E2E_MENU_PAINT_MS)
    }

    /// Enter Channel: lit-emblem click where it lands, Confirm otherwise.
    ///
    /// # Errors
    ///
    /// Neither edge reaches Channel.
    fn enter_channel(&mut self) -> anyhow::Result<()> {
        self.game
            .click_planet(E2E_EMBLEM_CLICK.0, E2E_EMBLEM_CLICK.1);
        if self.game.screen() != Screen::Channel {
            self.game.menu_key(MenuKey::Confirm, true);
        }
        self.require_screen(Screen::Channel, "planet-to-channel")?;
        self.record("emblem-or-confirm-enters-channel", E2E_MENU_PAINT_MS)
    }

    /// Enter the Room lobby through the channel row confirm.
    ///
    /// # Errors
    ///
    /// The confirm edge misses Room.
    fn enter_room_lobby(&mut self) -> anyhow::Result<()> {
        self.game.menu_key(MenuKey::Confirm, true);
        self.require_screen(Screen::Room, "channel-to-room")?;
        self.record("row-confirm-enters-room", E2E_MENU_PAINT_MS)
    }

    /// Enter the Waiting room through the room confirm.
    ///
    /// # Errors
    ///
    /// The confirm edge misses Waiting.
    fn enter_waiting_room(&mut self) -> anyhow::Result<()> {
        self.game.menu_key(MenuKey::Confirm, true);
        self.require_screen(Screen::Waiting, "room-to-waiting")?;
        self.record("room-confirm-enters-waiting", E2E_MENU_PAINT_MS)
    }

    /// Visit every side state from Waiting and return each time.
    ///
    /// # Errors
    ///
    /// Any side refuses to open or to return.
    fn tour_side_states(&mut self) -> anyhow::Result<()> {
        for side in E2E_SIDE_TOUR {
            let _staged = crate::shots::stage_side_art(&mut self.game, side);
            if !self.game.visit_side(side) {
                anyhow::bail!("e2e side-open: Waiting -> {side:?} refused");
            }
            self.record(&format!("waiting-visits-{side:?}"), E2E_MENU_PAINT_MS)?;
            if !self.game.leave_side() {
                anyhow::bail!("e2e side-return: {side:?} -> Waiting refused");
            }
            self.record(&format!("{side:?}-returns-to-waiting"), E2E_MENU_PAINT_MS)?;
        }
        self.require_screen(Screen::Waiting, "side-tour-end")
    }

    /// Commit the host song pick (the music-select wiring).
    ///
    /// # Errors
    ///
    /// Recording the pick frame fails.
    fn pick_host_song(&mut self) -> anyhow::Result<()> {
        self.game.set_selected_music(SetRoomMusic {
            music_id: E2E_SONG_ID,
            difficulty: E2E_DIFFICULTY,
            speed: E2E_SPEED,
        });
        self.record("host-pick-commits-song", E2E_MENU_PAINT_MS)
    }

    /// Run the Waiting→Loading→Playing gate with an armed walk.
    ///
    /// # Errors
    ///
    /// The START gate or the Loading gate holds shut.
    fn run_loading_gate(&mut self) -> anyhow::Result<()> {
        arm_walk_gate(&mut self.game)?;
        self.game.menu_key(MenuKey::Confirm, true);
        self.require_screen(Screen::Loading, "waiting-to-loading")?;
        self.record("start-enters-loading", E2E_MENU_PAINT_MS)?;
        self.game.menu_key(MenuKey::Confirm, true);
        self.require_screen(Screen::Playing, "loading-to-playing")?;
        self.record("armed-gate-enters-playing", E2E_PLAYING_PAINT_MS)
    }

    /// Strike the walk notes, spend the chart, and land on Result.
    ///
    /// # Errors
    ///
    /// The song end never advances to Result.
    fn play_chart_to_result(&mut self) -> anyhow::Result<()> {
        self.game.strike_at(E2E_TAP_LANE, E2E_TAP_ONSET_MS);
        self.game.strike_at(E2E_HOLD_LANE, E2E_HOLD_HEAD_MS);
        self.record("walk-strikes-rate-notes", E2E_PLAYING_PAINT_MS)?;
        self.game.release_at(E2E_HOLD_LANE, E2E_HOLD_TAIL_MS);
        paint_walk_frame(&mut self.game, E2E_RESULT_PAINT_MS);
        self.require_screen(Screen::Result, "playing-to-result")?;
        self.result_hash = Some(self.game.state_hash());
        self.record("chart-end-enters-result", E2E_RESULT_PAINT_MS)
    }

    /// Prove the Result retry and leave edges return to the lobby.
    ///
    /// # Errors
    ///
    /// Either Result edge misses.
    fn exercise_result_edges(&mut self) -> anyhow::Result<()> {
        self.game.menu_key(MenuKey::Confirm, true);
        self.require_screen(Screen::Waiting, "result-retry")?;
        self.record("result-retry-returns-to-waiting", E2E_MENU_PAINT_MS)?;
        self.game.menu_key(MenuKey::Back, true);
        self.require_screen(Screen::Room, "result-leave")?;
        self.record("waiting-back-returns-to-room", E2E_MENU_PAINT_MS)
    }
}

/// Repository-local `install-root` (git-ignored game data).
fn install_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../install-root")
}

/// Synthetic lobby rows: two channels, two rooms (display only).
fn seed_lobby_lists(game: &mut ScreenGame) {
    game.set_channels(vec![
        ChannelState {
            gateway_id: 0,
            channel_id: 2,
            capacity: 100,
            population: 5,
            active: true,
        },
        ChannelState {
            gateway_id: 0,
            channel_id: 9,
            capacity: 0,
            population: 0,
            active: false,
        },
    ]);
    game.set_rooms(vec![
        RoomListEntry {
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
        },
        RoomListEntry {
            number: 7,
            state_code: 1,
            title: "Locked".to_owned(),
            has_password: true,
            music_id: 0,
            difficulty: 0,
            key_mode: 2,
            game_mode: 0,
            speed: 1,
            capacity: 8,
            user_count: 1,
            min_level: 0,
            max_level: 0,
            skills: Vec::new(),
            premium: false,
            room_type: 0,
        },
    ]);
}

/// Synthetic waiting duo: self masters beside one ready peer.
fn waiting_duo() -> JoinedRoom {
    JoinedRoom {
        member_id: 0,
        team: 0,
        title: "Jam".to_owned(),
        music_id: 0,
        arena: 2,
        key_mode: 2,
        game_mode: GameMode::Live as u8,
        difficulty: 0,
        speed: 1,
        has_super_manager: false,
        user_count: 2,
        slots: vec![RoomSlot {
            index: 1,
            state_code: 1,
            member: Some(RoomMember {
                nickname: "Lujia".to_owned(),
                level: 12,
                gender: 0,
                gem: 0,
                master: false,
                team: 1,
                ready: true,
                music_state: 0,
                equipment: crate::shots::capture_kit_equipment(),
                music_ids: Vec::new(),
                cash_point: 0,
                free_pass: 0,
                playing_state: 0,
                administrator: false,
                super_manager: false,
            }),
        }],
        skills: Vec::new(),
        premium: false,
        champion: None,
        champion_streak: None,
    }
}

/// Synthetic battle scoreboard behind the walk verdict: rank 1 by team
/// total wins, so the wire peer takes it and the banner reads the loss arm.
fn walk_battle_teams() -> RoomTeams {
    let mut teams = RoomTeams::new();
    teams.set_team(0, 0);
    teams.set_team(1, 1);
    teams.submit(0, 100);
    teams.submit(1, 300);
    teams
}

/// Scripted rank bytes behind the walk rank table (fixture, not wire
/// data): the local seat holds rank 1, the wire peer rank 2.
const WALK_RANK_GLOBAL: u8 = 0xFF;
/// Holder bytes per seat: local rank 1, peer rank 2, the rest empty.
const WALK_RANK_STORED: [u8; 8] = [1, 2, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
/// Rank broadcast: local rank at 0, peer rank at 1.
const WALK_RANK_PACKET: [u8; 8] = [1, 2, 0, 0, 0, 0, 0, 0];

/// Synthetic walk chart: a tap plus a held long note over it.
fn walk_chart() -> Vec<(u8, u64, u64, u32)> {
    vec![
        (
            u8::try_from(E2E_TAP_LANE).unwrap_or(0),
            E2E_TAP_ONSET_MS,
            E2E_TAP_ONSET_MS,
            E2E_SAMPLE_ID,
        ),
        (
            u8::try_from(E2E_HOLD_LANE).unwrap_or(0),
            E2E_HOLD_HEAD_MS,
            E2E_HOLD_TAIL_MS,
            E2E_SAMPLE_ID,
        ),
    ]
}

/// Arm the Loading gate the way the live lane stages song prep: a
/// validated pick, a null rig, and the armed loading status.
///
/// # Errors
///
/// The armed status text never fails to build here.
fn arm_walk_gate(game: &mut ScreenGame) -> anyhow::Result<()> {
    use o2jam_audio::NullBackend;
    if game.selected_music().is_none() {
        game.set_selected_music(SetRoomMusic {
            music_id: E2E_SONG_ID,
            difficulty: E2E_DIFFICULTY,
            speed: E2E_SPEED,
        });
    }
    if !game.has_audio() {
        game.with_audio(Box::new(NullBackend::new()), HashMap::new());
    }
    match loading_phase_status(LOADING_GATE_STEPS, "First", "Band") {
        Some(status) => {
            game.set_loading_status(status);
            Ok(())
        }
        None => anyhow::bail!("e2e arm-gate: armed loading status refused to build"),
    }
}

/// Paint one walk frame into row-major RGB bytes.
///
/// The scene rasterizes at the native 800x600 canvas — the frame every
/// staged record anchor and `.bnd` rect is authored on — and the result
/// box-downscales to the evidence size, exactly like the `shots` walk
/// (`crate::shots`): sprite layers blit 1:1 while anchors and rects
/// scale, so painting straight into a small frame renders staged art
/// at double size and clipped.
fn paint_walk_frame(game: &mut ScreenGame, now_ms: u64) -> Vec<u8> {
    let width = usize::try_from(NATIVE_WIDTH).unwrap_or(E2E_FRAME_W);
    let height = usize::try_from(NATIVE_HEIGHT).unwrap_or(E2E_FRAME_H);
    let mut native = vec![0u32; width * height];
    game.paint_at(
        &mut Frame {
            buf: &mut native,
            w: width,
            h: height,
        },
        now_ms,
    );
    let mut rgb = Vec::with_capacity(E2E_FRAME_W * E2E_FRAME_H * 3);
    for y in 0..E2E_FRAME_H {
        for x in 0..E2E_FRAME_W {
            let x0 = x * width / E2E_FRAME_W;
            let y0 = y * height / E2E_FRAME_H;
            let pixel = native[y0 * width + x0];
            rgb.push(((pixel >> 16) & 0xFF) as u8);
            rgb.push(((pixel >> 8) & 0xFF) as u8);
            rgb.push((pixel & 0xFF) as u8);
        }
    }
    rgb
}

/// Whether a walk frame carries ink (never a flat fill).
fn frame_holds_ink(rgb: &[u8]) -> bool {
    let first = rgb.first().copied().unwrap_or(0);
    rgb.iter().any(|byte| *byte != first)
}

/// Write the step trace beside the evidence PNGs, when asked.
///
/// # Errors
///
/// Filesystem failures while writing `trace.json`.
fn write_trace_json(dir: Option<&Path>, report: &E2eReport) -> anyhow::Result<()> {
    let Some(dir) = dir else {
        return Ok(());
    };
    let text = serde_json::to_string_pretty(&report)?;
    std::fs::write(dir.join("trace.json"), text)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    /// Loading ink comes from the staged catalog caption. Without
    /// game data the walk still checks the screen edge, but that one
    /// frame stays flat and carries no ink assertion.
    fn loading_caption_staged() -> bool {
        std::fs::read(install_root().join("O2Jam/Image/OJNList.dat")).is_ok()
    }

    /// The live input path is a press snapshot plus a release
    /// snapshot (the #115 press split), not `click_planet`. With real
    /// staged art, the down/up pair must open the gear dialog exactly
    /// like the direct call — this is the headless reproduction of
    /// "clicks route but the game never acts" (#118 no-input leg).
    #[test]
    fn press_gesture_opens_gear_dialog_like_the_direct_click() {
        let mut e2e = E2eWalker::seeded(None).expect("seeded walker");
        e2e.dismiss_splash().expect("splash dismisses");
        e2e.game
            .ingest_snapshot(&InputSnapshot::press_at(E2E_GEAR_CLICK.0, E2E_GEAR_CLICK.1));
        e2e.game.ingest_snapshot(&InputSnapshot::release_at(
            E2E_GEAR_CLICK.0,
            E2E_GEAR_CLICK.1,
        ));
        assert!(
            e2e.game.planet_modal().is_some(),
            "press gesture (down+up) must open the gear dialog like click_planet does"
        );
    }

    use std::sync::OnceLock;

    use super::*;
    use o2jam_game::{build_level_chart, CoolStreak, Judgement, PillStock, Score, Sequencer};
    use o2jam_render::Rect;
    use o2jam_shell::{AvatarFigure, FigureLayer, InputSnapshot};

    /// The tutorial's own chart drives the rated sink without a key
    /// press: B22 pins autoplay BGM markers to the rating-event sink
    /// (`docs/claims/30-gameplay.md`, ev `292d6cf2…`); `S-PLAY-B3`
    /// pins FIRSTSTEP's song id 8888. Live chain: boot → Planet →
    /// Channel → Room with its real `.bnd` chrome → FIRSTSTEP click →
    /// Tutorial → `Tutorial.ojn` load → `Sequencer` → scoring. Only
    /// the Room state is staged, so this guard does not add the
    /// full-walk load that races the fake-gateway tests in parallel.
    /// Install-guarded: no game data leaves the seam unclaimed.
    #[test]
    fn firststep_tutorial_chart_autoplay_judges_every_note_without_key_input() {
        let chart_path = install_root().join("O2Jam/Music/Tutorial.ojn");
        if !chart_path.is_file() {
            println!("skip: missing install chart");
            return;
        }
        let installed = o2jam_shell::staging::read_install_archives(&install_root());
        let Some(interface) = installed.interface else {
            println!("skip: missing interface archive");
            return;
        };
        let mut game = ScreenGame::new();
        if let Some(art) = o2jam_shell::staging::stage_state(
            &interface,
            o2jam_shell::staging::State::Room,
            o2jam_shell::staging::StageContext::default(),
        )
        .art
        {
            game.load_scene(Screen::Room, art.placements, art.figures);
        }
        game.menu_key(MenuKey::Back, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        // B7 pin: FIRSTSTEP rect `(660,1)-(720,33)`, click `(689,16)`.
        game.ingest_snapshot(&InputSnapshot::click_at(689, 16));
        assert_eq!(game.side(), Some(SideScreen::Tutorial));
        let globals = game.tutorial_chart_globals().expect("FIRSTSTEP globals");
        assert_eq!(globals.song_id, 8888);

        let bytes = std::fs::read(&chart_path).expect("read Tutorial.ojn");
        let prepared = crate::loading::prepare_chart(&bytes, 0).expect("walk chart");
        assert_eq!(prepared.song_id, 8888, "the pinned chart is song 8888");
        let events = o2jam_file::parse_ojn_level_events(&bytes, 0).expect("walk the Easy slice");
        let chart = build_level_chart(prepared.bpm, &events);
        let mut sequencer = Sequencer::new(&chart);
        let mut score = Score::default();
        let mut pills = PillStock::default();
        let mut streak = CoolStreak::default();
        let due_count = sequencer.len();
        assert_eq!(due_count, 51, "Tutorial Easy carries 51 note payloads");
        let mut now_ms = 0;
        while !sequencer.is_empty() {
            for strike in sequencer.pop_due(now_ms) {
                let _ = strike;
                streak.note(Judgement::Cool, &mut pills);
                score.apply_rated(Judgement::Cool, &mut pills);
            }
            now_ms += 5;
        }
        assert_eq!(score.counts.iter().sum::<u32>() as usize, due_count);
        assert_eq!(score.counts[0] as usize, due_count, "every note rated Cool");
        assert!(score.points > 0, "autoplay produced a score");
    }

    /// Onsets for the chart-timed replay: the installed `Tutorial.ojn`
    /// when game data is present, a pinned synthetic pair otherwise.
    /// Both arms exercise the same press/painter path; only the ink
    /// origin changes.
    fn tutorial_replay_onsets() -> Vec<(u8, u64, u32)> {
        let synthetic = vec![(2, 5_000, 1), (5, 9_000, 1)];
        let Ok(bytes) = std::fs::read(install_root().join("O2Jam/Music/Tutorial.ojn")) else {
            return synthetic;
        };
        crate::loading::tutorial_chart_onsets(&bytes).unwrap_or(synthetic)
    }

    /// A chart time outside every note window, just before the first
    /// onset so the note is still in the lookahead while the press is
    /// off-note.
    fn off_note_press_ms(onsets: &[(u8, u64, u32)]) -> u64 {
        let anchor = onsets.first().map_or(5_000, |&(_, at_ms, _)| at_ms);
        let before = anchor.saturating_sub(500);
        if before > 0
            && onsets
                .iter()
                .all(|&(_, at_ms, _)| before.abs_diff(at_ms) > 25)
        {
            before
        } else {
            anchor + 500
        }
    }

    /// One staged tutorial key kit: the rest frame is white, the four
    /// pressed frames are red, so a cap swap is a pixel read.
    fn tutorial_key_kit() -> (
        Vec<o2jam_shell::avatar::ScenePlacement>,
        HashMap<String, AvatarFigure>,
    ) {
        use o2jam_shell::avatar::PlacementRole;
        let mut placements = Vec::new();
        let mut figures = HashMap::new();
        for lane in 0..7usize {
            let name = format!("KeyEffect_Tutorial_{}.ojs", lane + 1);
            let mut figure = AvatarFigure::default();
            for frame in 0..5 {
                let word = if frame == 0 { 0x7FFFu16 } else { 0xF800u16 };
                let pixels = word.to_le_bytes().repeat(16);
                figure.push_layer(FigureLayer {
                    image: o2jam_render::Rgb555Image::decode(&pixels, 4, 4).unwrap(),
                    dx: 100 + (lane as i32) * 20,
                    dy: 487,
                    colorkey: 0,
                });
            }
            placements.push(o2jam_shell::avatar::ScenePlacement {
                role: PlacementRole::Single,
                id: String::new(),
                code: 0,
                sprite: Some(name.clone()),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 800,
                    h: 600,
                },
            });
            figures.insert(name, figure);
        }
        (placements, figures)
    }

    /// #516: an off-note press in the Tutorial lights its key cap and
    /// leaves COOL/BAD/GOOD/MISS at zero. Live chain: boot -> Channel ->
    /// Room with its B7 FIRSTSTEP rect -> staged FIRSTSTEP click ->
    /// Tutorial -> chart-timed note frame -> snapshot lane press ->
    /// painter and tally. The REA boundary is `docs/captures/n-judge.md`:
    /// only the Playing update reaches the judgement scheduler; the
    /// Tutorial update cannot tally a key. Boot-2 key frames pin the
    /// lit cap with a zero HUD.
    #[test]
    fn firststep_tutorial_off_note_press_lights_the_cap_without_tallying() {
        let onsets = tutorial_replay_onsets();
        assert!(!onsets.is_empty(), "the replay needs a chart onset");
        let press_ms = off_note_press_ms(&onsets);
        assert!(
            onsets
                .iter()
                .all(|&(_, at_ms, _)| press_ms.abs_diff(at_ms) > 25),
            "the press must be off-note"
        );

        let mut walk = E2eWalker::seeded(None).expect("seeded walker");
        walk.dismiss_splash().expect("splash dismisses");
        walk.enter_channel().expect("channel opens");
        walk.enter_room_lobby().expect("room lobby opens");
        let game = &mut walk.game;
        // B7 FIRSTSTEP rect `(660,1)-(720,33)`; the synthetic placement
        // keeps the click live with data hidden, where real art is absent.
        game.load_scene(
            Screen::Room,
            vec![o2jam_shell::avatar::ScenePlacement {
                role: o2jam_shell::avatar::PlacementRole::Single,
                id: "IDC_BUTTON_SROOM_FIRSTSTEP".to_owned(),
                code: 0x1320_0032,
                sprite: None,
                rect: Rect {
                    x: 660,
                    y: 1,
                    w: 60,
                    h: 32,
                },
            }],
            HashMap::new(),
        );
        let (placements, figures) = tutorial_key_kit();
        game.load_tutorial_scene(placements, figures);
        game.ingest_snapshot(&InputSnapshot::click_at(689, 16));
        assert_eq!(game.side(), Some(SideScreen::Tutorial));
        assert_eq!(
            game.tutorial_chart_globals().map(|row| row.song_id),
            Some(8888)
        );

        let paint = |game: &mut ScreenGame, now_ms: u64| {
            let mut buf = vec![0u32; 800 * 600];
            game.paint_at(
                &mut Frame {
                    buf: &mut buf,
                    w: 800,
                    h: 600,
                },
                now_ms,
            );
            buf
        };
        // Latch the chart origin before the replay clock moves.
        let _ = paint(game, 0);
        game.stage_tutorial_chart(&onsets);
        let with_notes = paint(game, press_ms);
        game.stage_tutorial_chart(&[]);
        let without_notes = paint(game, press_ms);
        assert_ne!(
            with_notes, without_notes,
            "the staged note must be in flight"
        );
        assert_eq!(game.live_play().score().counts, [0, 0, 0, 0]);

        game.stage_tutorial_chart(&onsets);
        let lane = usize::from(onsets[0].0);
        let cap_pixel = 487 * 800 + 100 + lane * 20;
        game.ingest_snapshot(&InputSnapshot::lane_key(lane, true));
        let pressed = paint(game, 30);
        assert_ne!(
            pressed[cap_pixel], without_notes[cap_pixel],
            "key cap lights"
        );
        assert_eq!(
            game.live_play().score().counts,
            [0, 0, 0, 0],
            "off-note key art never tallies"
        );

        game.ingest_snapshot(&InputSnapshot::lane_key(lane, false));
        let released = paint(game, 30);
        assert_eq!(released[cap_pixel], without_notes[cap_pixel], "cap clears");
        assert_eq!(game.live_play().score().counts, [0, 0, 0, 0]);
    }

    /// The one walk fixture: a primary trace and its replay proof.
    /// Both are built together on first access so guards only read.
    struct WalkFixture {
        walk: E2eReport,
        replay: E2eReport,
    }

    static WALK_FIXTURE: OnceLock<WalkFixture> = OnceLock::new();

    fn walk_fixture() -> &'static WalkFixture {
        WALK_FIXTURE.get_or_init(|| WalkFixture {
            walk: drive_e2e_walk().expect("scripted walk"),
            replay: drive_e2e_walk().expect("scripted walk replay"),
        })
    }

    fn shared_e2e_report() -> &'static E2eReport {
        &walk_fixture().walk
    }

    #[test]
    fn e2e_snapshot_reuses_one_walk_when_guards_share_it() {
        let first = shared_e2e_report();
        let second = shared_e2e_report();
        assert!(
            std::ptr::eq(first.steps.as_ptr(), second.steps.as_ptr()),
            "every guard must read the same immutable trace"
        );
    }

    /// Screens the walk must show, in player order.
    const E2E_SCREEN_PATH: [&str; 8] = [
        "avi", "planet", "channel", "room", "waiting", "loading", "playing", "result",
    ];

    /// Screens shown along the walk, deduplicated in order.
    fn visited_screens(report: &E2eReport) -> Vec<String> {
        let mut screens = Vec::new();
        for step in &report.steps {
            if screens.last() != Some(&step.screen) {
                screens.push(step.screen.clone());
            }
        }
        screens
    }

    #[test]
    fn e2e_walk_boots_from_splash_to_result_through_every_landed_state() {
        let report = shared_e2e_report();
        let screens = visited_screens(report);
        for want in E2E_SCREEN_PATH {
            assert!(
                screens.contains(&want.to_owned()),
                "walk misses {want}: {screens:?}"
            );
        }
        for side in E2E_SIDE_TOUR {
            let seen = report
                .steps
                .iter()
                .any(|step| step.side.as_deref() == Some(&format!("{side:?}")));
            assert!(seen, "walk never visits {side:?}");
        }
        assert!(
            report
                .steps
                .iter()
                .any(|step| step.action == "gear-disc-opens-options"),
            "walk never tours the options dialog"
        );
        for step in &report.steps {
            let side_step = step.action.starts_with("waiting-visits-")
                || step.action.ends_with("-returns-to-waiting");
            if side_step || (step.action == "start-enters-loading" && !loading_caption_staged()) {
                continue;
            }
            assert!(
                step.frame_varied,
                "step {} paints a flat frame",
                step.action
            );
        }
        assert_ne!(report.result_hash, 0, "result hash must observe the walk");
        assert_eq!(report.final_screen, "room");
    }

    #[test]
    fn e2e_seeds_item_figures_for_the_waiting_seats_and_room_plate() {
        // Live caller chain: recomp2jam e2e -> write_e2e_evidence ->
        // E2eWalker::seeded -> shots::stage_avatar_figures ->
        // ScreenGame::set_avatar_source + load_room_avatar (#336).
        // Archives absent -> skip cleanly (CI has no game data).
        if !install_root().join("O2Jam/Image/avatar.opa").exists() {
            println!("skip: install-root avatar.opa absent");
            return;
        }
        let walker = E2eWalker::seeded(None).expect("seeded walker");
        assert!(
            walker.game.waiting_avatar(0).is_some_and(|f| !f.is_empty()),
            "self seat wears the pinned kit"
        );
        assert!(
            walker.game.waiting_avatar(1).is_some_and(|f| !f.is_empty()),
            "peer seat wears the wire kit"
        );
        assert!(
            walker.game.room_avatar().is_some_and(|f| !f.is_empty()),
            "room peer plate wears the same kit"
        );
    }

    #[test]
    fn e2e_walk_replays_to_the_same_final_hash() {
        let fixture = walk_fixture();
        let (first, second) = (&fixture.walk, &fixture.replay);
        assert_eq!(first.result_hash, second.result_hash);
        assert_eq!(first.final_hash, second.final_hash);
        let first_path: Vec<&str> = first
            .steps
            .iter()
            .map(|step| step.screen.as_str())
            .collect();
        let second_path: Vec<&str> = second
            .steps
            .iter()
            .map(|step| step.screen.as_str())
            .collect();
        assert_eq!(first_path, second_path);
    }

    #[test]
    fn e2e_walk_side_states_open_and_return_to_waiting() {
        let report = shared_e2e_report();
        for side in E2E_SIDE_TOUR {
            let opened = report
                .steps
                .iter()
                .position(|step| step.action == format!("waiting-visits-{side:?}"));
            let returned = report
                .steps
                .iter()
                .position(|step| step.action == format!("{side:?}-returns-to-waiting"));
            let (Some(opened), Some(returned)) = (opened, returned) else {
                panic!("walk misses the {side:?} visit or return");
            };
            assert!(opened < returned, "{side:?} must open before it returns");
        }
    }

    #[test]
    fn e2e_walk_options_opens_and_closes_on_the_board() {
        let report = shared_e2e_report();
        let opened = report
            .steps
            .iter()
            .position(|step| step.action == "gear-disc-opens-options");
        let closed = report
            .steps
            .iter()
            .position(|step| step.action == "escape-closes-options");
        let (Some(opened), Some(closed)) = (opened, closed) else {
            panic!("walk misses the options open or close");
        };
        assert!(opened < closed, "options must open before it closes");
        assert_eq!(report.steps[closed].screen, "planet");
    }

    /// Live caller chain from boot:
    ///   E2eWalker::seeded -> stage_installed_song_catalog ->
    ///   pick_host_song -> ScreenGame::set_selected_music ->
    ///   resolve_song_cover -> stage_loading_cover ->
    ///   paint_loading_scene_with_status -> paint_loading_cover.
    /// The paint seam is separately pinned in `paint.rs`; this guard
    /// pins the real OJN bytes reaching the loading frame's pixels.
    /// Install data is local-only: absent files skip cleanly in CI.
    #[test]
    fn song_pick_paints_the_installed_ojn_cover_on_the_loading_gate() {
        let catalog_path = install_root().join("O2Jam/Image/OJNList.dat");
        let chart_path = install_root().join("O2Jam/Music/o2ma757.ojn");
        if !catalog_path.is_file() || !chart_path.is_file() {
            println!("skip: missing install catalog or chart");
            return;
        }
        let Ok(chart_bytes) = std::fs::read(&chart_path) else {
            println!("skip: unreadable {}", chart_path.display());
            return;
        };
        let Ok(cover) = o2jam_file::ojn_cover_jpeg(&chart_bytes)
            .and_then(|stream| o2jam_file::decode_ojn_cover(&stream))
        else {
            println!("skip: undecodable OJN cover");
            return;
        };

        let mut e2e = E2eWalker::seeded(None).expect("seeded walker");
        assert!(
            !e2e.game.select_songs().is_empty(),
            "boot must stage the installed song catalog"
        );
        e2e.dismiss_splash().expect("splash dismisses");
        e2e.enter_channel().expect("board enters channel");
        e2e.enter_room_lobby().expect("channel enters room");
        e2e.enter_waiting_room().expect("room enters waiting");
        e2e.pick_host_song().expect("song pick commits");
        assert!(
            e2e.game.has_loading_cover(),
            "song pick must stage the walked OJN cover"
        );
        arm_walk_gate(&mut e2e.game).expect("gate arms");
        e2e.game.menu_key(MenuKey::Confirm, true);
        assert_eq!(e2e.game.screen(), Screen::Loading, "gate enters Loading");

        let frame = paint_walk_frame(&mut e2e.game, E2E_MENU_PAINT_MS);
        for channel in 0..3 {
            assert!(
                frame[channel].abs_diff(cover.pixels[channel]) <= 8,
                "loading pixel channel {channel} must carry the decoded cover"
            );
        }
    }
}

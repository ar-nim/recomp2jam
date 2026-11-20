# Journey audit — Spec: Playing state (#293)

Comparison leg, 2026-11-18. Worktree `cb2/ja-b`, base `2105ac6`, no exe boots.
Exe 7.55 pin: `OTwo.exe` 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.

Comparator classes: `pixel-region`, `wire-sequence`, `input-response`,
`state-path`, `data-effect`, `NA-DESIGN`.

## Anchor evidence for this spec

- `~/scratch/no-response/f1-playing-qualified/citation.txt` sha256
  `8773f5cc0efb9486e782e230423d3a0439cb6182f20f22ffd790a5939d1e8517`
  cites the provenance-backed exe Loading frame
  (`artifacts/screenshots/loading.png`, sha256
  `b555123f6d099f64330f000ea52cd4f2eeeb3716f9fa4139a6bf362466eaaaed`).
  `drive.log` sha256 `89fe88f82bdc21a112a2fa479aff12f85959a0156014af46c96d2a89e578bc58`
  logs the READY gate passing, the full 4011/4012/4048 start gate, then
  `stage playing FAILED reason=no 4014 with play pixels` after a 90 s poll;
  `wire/room-stage.log` sha256
  `5233b743bd22ef7158d3120d36af1e1ee52e8648b99cbb881174de55e1fb9838`
  has only keepalives afterwards.
- `~/scratch/wedge327/dbg/attach316.log` sha256
  `ad8dd3aea77f2311a007d372ccccfa4b7c4cb163dc06876d23a2669a32b23f34`
  (evidence id `ev_327_stop_316_ad8dd3ae…`, Wine 11.22, wpid 316): one stopped
  state shows `[state] = 0x0064B39C` = `CStateNormalPlaying` primary vtable,
  but the render thread is inside a DirectDraw/wined3d wait from page dispatch
  (`0x42b486`/`0x42b5ce`); the loading frame stays on screen.
  The state object and the pixels disagree: state-path claims can be made,
  pixel and input claims cannot.
- Rust design references (not exe proof): `docs/screenshots/playing.png` sha256
  `09b3d70f92a549e39779dc5ed2195126eb440772d56807994588cbea2a26684a` and
  vintage `docs/screenshots/reference-4x3.png` sha256
  `926f03245ce3350053c580be7328d209183ab2337ccb234dc86f25110b12e5cf`.

## Per-story matrix (12 stories)

| # | Story (short) | Comparator | Our journey (code / guard / capture) | Exe evidence | Verdict | Notes / mapping |
|---|---|---|---|---|---|---|
| 1 | [x] Seven lanes and note rendering per exe | pixel-region | `HighwayGeometry` columns `(5,28)…(165,28)`, judgement row 487 (`crates/o2jam-shell/src/playing.rs:1172-1230`), `y_for_onset` linear scroll (1582), `lane_clip_rects` (1223), staged heads; guards `play_skin_decodes_every_lane_when_present` (`shots.rs:2883`), `hidden_curtain_culls_wide_note_head_overhang_past_its_lane_column` (`playing.rs:4376`) | Static pins: file `0x56e1cb`–`0x56e3d3` draw dispatcher, song clock `0x791d70`, per-lane speed `0x791d54` (`docs/scroll-map.md`, A26 `ev_f4662376…`, A33/A34); no provenance-backed exe Playing frame — f1 `drive.log:90` has no `playing.png` and no 4014 | BLOCKED | Map to **#326** (Playing evidence wedge) and **#245/#249** (capture tooling). The stopped-state read proves the state object exists but the frame never paints; no pixel verdict possible. No new issue. |
| 2 | [ ] Judgement windows pinned and enforced | state-path + input-response | `TimingWindows {6,18,25}` and `windows_judge_verified_7_55_tiers` (`crates/o2jam-game/src/lib.rs:1802`); `LivePlay::strike_lane` / `judge_armed_tail` (`playing.rs:2145-2169`); seam guards `release_strike_rates_the_armed_tail_exact_cool` (4175), `release_tiers_the_tail_with_the_shared_windows` | Static exe pin judgement `FUN_00576540` file `0x176540`, sites `0x176646/55/64` + `0x176b7f/8e/9d`, ev `14992eb2…` (`docs/gameplay.md`, `docs/claims-gameplay.md`); no live key sweep pack | MATCHED | Window values and tier order match the static oracle. The live key-up routing confirmation is story 9 and is BLOCKED under #326; do not claim a live latency number here. |
| 3 | [x] Long notes with hold and judge tails | state-path + pixel-region | `HoldSpan` / `HoldState` (`o2jam-game/src/lib.rs`), `hold_tail_scrolls_down_the_highway_as_a_note` (`playing.rs:5985`), `armed_hold_keeps_the_pressed_key_down_past_the_burst`, `skinned_highway_renders_hold_tails_and_tick_feedback` (`shots.rs:3146`); per-tick +1 combo guard `hold_tick_count_paints_under_the_caption_plate` | Static pins B11/B12 (hold flag and tail rating), `docs/gameplay.md` files `0x1760ff`/`0x176c3b`/`0x1774b4`, claim B46 (head/tail rating feed OPEN); no exe Playing frame | MATCHED | Head/tail and tick rules match the pinned behavior. The untraced `NoteTail` 0..=4 style table is story 12 (BLOCKED); no new issue. |
| 4 | [x] Life and Jam via stats exchange at pinned moments | wire-sequence + state-path | `UpdateGameStats` 4014 / `GameStatsUpdate` 4015 (`o2jam-net/src/play.rs:283-412`), `poll_stats` / `send_stats_ticks` (`crates/recomp2jam/src/session.rs:1979-2052`), peer mirror `apply_peer_echo` (`game/mod.rs:3682`); guards `life_tick_follows_a_life_drain` (session.rs:6671), `peer_echo_seats_a_gauge_in_playing_paint` (game/mod.rs:7228), `life_delta_sets_match_the_verified_switch_arms` (`lib.rs:2461`) | `docs/net-protocol.md` §4c: 4014 type/u16 value/seq/score/longNote — Jam on Cool/Good tails files `0x16e660`/`0x16e7dd`, Life at `0x4eaec0`; 4015 echo shape; f1 wire has no guest 4014 (wedge) | MATCHED | Wire layout and the drain order agree with the exe pin. A paired exe capture of 4014/4015 and life/jam paint is a live-lane item (#326), not a divergence in the modeled exchange. |
| 5 | [x] Song clock anchored to the audio backend | state-path (+ timing) | `CpalBackend::samples_played` → `song_clock_totals_backend_samples_not_wall_time` (`game/mod.rs:7046`); `song_clock_replay.rs` equivalence guard `song_clock_equivalence_replay_stays_inside_the_pinned_windows` (463) at 44.1/48/96 kHz with a half-rate negative control | Exe paces on `timeGetTime` (claim B38 `efdaac19…`); **ADR-0001** records the approved sample-clock deviation; no exe wall-trace pack | MISMATCHED | Known, owner-approved divergence: exe wall clock vs Rust sample clock. No new issue (ADR-0001); the live-device divergence number remains OPEN in `docs/parity/playing.md` and the live lane. |
| 6 | [x] Score submitted with pinned field layout | data-effect + wire-sequence | `SubmitScore` 4016 (`o2jam-net/src/play.rs:456`), `tally_packet` (`session.rs:2558`) fills cool/good/bad/miss/maxCombo/jam/maxJam/score/life/speed/longNote with saturating conversions; guard `result_entry_submits_live_tally_then_finalizes_and_exits` (`session.rs:5789`) | `docs/net-protocol.md` §4c: 4016 field-for-field layout corroborated by Memoryer; no exe 4016 bytes through a proxy in any pack | MATCHED | Field layout and send chain match the static pin. Live exe 4016 bytes on a qualified play are on the live-lane list (#326); no new issue. |
| 7 | [x] Finished song transitioning to Result | state-path | `finished_song_lands_on_result_without_invented_score_digits` (`crates/o2jam-shell/tests/scenes.rs:290`), `e2e_walk_boots_from_splash_to_result_through_every_landed_state`; `send_tally` then `FinalizeRank`/`ExitPlaying` (`session.rs:2540-2552`) | `docs/lifecycle.md` §10 pins Playing → Result; no qualified exe frame at the edge | MATCHED | State-path and guard agree with the lifecycle pin; the Rust e2e walk proves our half. No live exe edge frame, but no observed divergence. |
| 8 | [x] Judged lines countable against injections | input-response | `CaptureAdapter::record_judged_before_drain` (`crates/o2jam-shell/src/window/capture.rs:116`), guards `capture_log_records_judged_strikes_on_delivery` (279), `judged_lines_precede_drained_edges_in_fake_sink` (312); rig `~/scratch/t280/findings.md` reports `injections=14 judged=14 lanes=7/7` | `~/scratch/no-response/f1-playing-qualified/drive.log` (sha256 `89fe88f8…c58`) never reaches the highway; no exe judged-line stream exists. `docs/captures/t76/findings.md` records no playing capture; `docs/captures/t280/findings.md:120` records the stale "60 judged" rig-runbook claim needing re-pin or retraction | BLOCKED | Map to **#326** + **#245/#249/#250** (capture/tooling lineage). Rust tooling is countable; exe side has no capture. No new issue. |
| 9 | [ ] Live key-down/head and key-up/tail routing captured | input-response | `LivePlay::release_lane` → `judge_armed_tail` (`playing.rs:2169`), guards `keyup_strikes_nearest_record_then_judges_the_armed_tail`, `early_release_misses_the_tail_and_bonus_keeps_ticking`, `tail_judgement_is_one_shot_per_armed_cursor`; `o2jam-shell/src/input_latency.rs` for receipt stamps | Static exe pin: inversion `FUN_005714b0` file `0x1714b0`, key-up branches `0x176c3b`/`0x176c41`, bit-25 test `0x176c42`, tail compare `0x176646`–`0x17667a`, ev `c01db4a8…`; no epoch-stamped injector capture | BLOCKED | Map to **#326** (no live highway) + **#245/#249** (paired input capture). Static routing is modeled; exe confirmation is exactly the wedge. No new issue. |
| 10 | [ ] DI scancode-to-lane table pinned against live keyboard | input-response | winit keymap SDF/Space/JKL in `crates/o2jam-shell/src/window.rs`; synthetic keymap guards only; no DI scancode reader | Static exe pin `Option.ini` KEY_1–7 table file `0x173290` (ev `fc65241f…`), `FUN_00544b50`/`FUN_005429b0` poster, `ev_3d707204…`; no SendInput per-lane sweep pack | BLOCKED | Map to **#326** + **#245** (capture tooling). Requires one qualified highway with per-lane judged counts; no new issue. |
| 11 | [x] Boot-time OJM bank loader at `0x550db0` classified | state-path (paint classification) | `boot_bgm_bank_path`/`BOOT_BGM_BANK_FILES` in `crates/recomp2jam/src/main.rs` name the seven archives and stage bank 2 for scene cues; `docs/parity/playing.md` Timing row and `docs/parity/playing-input-timing-plan.md` Cell 2 carry the corrected label | Observer exe sha256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`: REA caller chain `ev_b9fd1f27433ada2023ce7cb81fa25f6f828821a93b11eb7540478c5b6a562636` (PE entry → sound init → bank loader) and bank names `ev_a9fb8aefefa70e2a45105090df82976fd44527a81a7029fd83d4f64e1a2bfd3e` | MATCHED | Old per-frame-effect label refuted on the observer's own exe; no cadence or paint claim follows from this VA. |
| 12 | [ ] `NoteTail` 0..=4 style table applied to hold art | pixel-region + data-effect | `playing.rs:1795` reads the `option.ini` NoteTail display option; hold tails use one shared staged head body (`playing.rs:2327-2340`), no per-style frame table | Static pin table VA `0x67fe8c` (`docs/gameplay.md` B11 per-tick claim), hold bit rules files `0x1760ff`/`0x176c3b`/`0x1774b4`; no staged-art exe Playing frame | BLOCKED | Needs the REA consumer dossier plus a staged-art Playing capture; map the capture half to **#326** + **#245**. No new issue. |

## Summary matrix — Playing

| Verdict | Stories | Rationale |
|---|---|---|
| MATCHED | 2, 3, 4, 6, 7 | Static 7.55 pins match Rust models and named guards: windows `0x176540` (ev `14992eb2…`), 4014/4015/4016 in `docs/net-protocol.md` §4c, hold rules B11/B12, lifecycle §10. |
| MISMATCHED | 5 | Exe wall clock vs Rust sample clock — known owner-approved deviation **ADR-0001**; no new issue. |
| BLOCKED | 1, 8, 9, 10, 11, 12 | No provenance-backed exe Playing frame or input stream: f1 `drive.log` sha256 `89fe88f8…c58` / `citation.txt` sha256 `8773f5cc…8517` stop at Loading, `attach316.log` sha256 `ad8dd3ae…2b23f34` shows the render-thread wedge. Map to **#326**, plus **#245/#249/#250** for capture tooling. |
| NA-DESIGN | — | None. |

## WAVE 1 #378 addendum — guest Playing in a two-member room

Fix `dae62a5` (rig-side, no client behavior change) makes the bot master send
its eleven `4048` tick-sync frames after `4010/4012` and submit a zero `4016`
after the guest's `4017`. The root cause was server-side: Memoryer's
`ScoreTracker.SyncTick` (`/home/arnim/mozart-src/Source/Memoryer`,
`Memoryer.Server/Services/ScoreTracker.cs:265-272`) withholds the `4050`
broadcast until every tracked member finishes the latency burst; the old bot
never sent one. `ScoreTracker.SubmitScore` (`:284-344`) withholds `4018`
until every tracked member submits `4016`. Red guards:
`scripts/tests/test_master_play_chain.py`; gates: `scripts/gate-check.sh
--no-push` green on both install-root arms.

Qualified exe pack `~/scratch/p378/guest-playing-1/guest` (one exe guest +
fixed bot master; **not** a two-exe cell): manifest
`aae3d446784e17defb21fdb23f3553fb7fa769a6c88e4c906805ce1f014e50c4`,
citation `96345ebb11a552880d5b4a2384aebaac43e02abd3a788d99dcb23a20174f307d`,
wire `99f360510b75a5abb8c5eae824772e8c1ef612c3bdcc57dd6ebab65638d6033e`,
`guest-drive.log`
`1e1c5891fcf0f879735f5112338f61be9a8a93ec3df3b0b80615645cfadf89c7`,
host `091d1df23cdc31439f8fa9605ec36572b17fdf87ffca984303cc188037533208`.
Containment `route=sink` / `on_default_sink=0` / `preboot_breach=0`; `cite
--claim exe` PASS. This pins the following spec stories:

- Story 1 (seven lanes / note rendering): the guest Highway paints after
  `S2C 4050`, with BGA and HUD behind three lanes in this 757/3KEY room;
  `artifacts/screenshots/guest-playing-entry.png`
  `98aa85b8e773293ffbf10af076afd7e2f7c6445a2b776fa22ff6aea9e346965b`,
  wire `:67`. The seven-lane geometry itself stays pinned by `s8` and the
  static dispatcher.
- Story 4 (life/Jam stats exchange): live two-member `C2S 4014` life `1000 →
  0` with `S2C 4015 member=01` echoes, wire `:68-91`; later `MISS 32` frame
  `guest-play-08.png`
  `9d31b6e797a8c58f7d9ef6d4850adbd687e0c83dc8289bc4a70f3bb963f7c440`.
  The master-side 4014 and the reactive master 4016 did not fire in this
  pack (the guest never submitted 4016), so no full score-table claim.
- The 23-action matrix (`293-playing-story-matrix.md`) gains five guest-half
  cells: ENTER-BATTLE-PEER (peer + server), LIFE-MISS-DRAIN (peer),
  BGA-HUD-ART (peer), TWO-CLIENT-ROSTER (peer).

Chart-timed DIK replay (`chart757-level1.txt`, level 1, 40 events, lead
400 ms) was applied, but all notes were missed (COOL 0 / MISS 32), so stories
2, 8, 9 and 10 keep their prior verdicts. The `4016/4017/4018` Result arm
(story 7) also stays unpinned for a two-member room. The second-exe master
half of `E-MULTI / #378` remains OPEN.

Live-lane carry-forward (all mapped, no new issues):
1. One qualified Playing capture with epoch-stamped injector and judged log,
   one stimulus per boot (#326 + #249); until then stories 1/9/10/12 cannot
   be compared as pixels or inputs.
2. Paired 4014/4015 capture with life/jam paint, and exe 4016 bytes through a
   TCP proxy on a qualified play (stories 4 and 6, #326 class).
3. The `0x550db0` REA dossier is closed as a boot-time OJM bank loader
   (story 11); story 12 still needs the `0x67fe8c` consumer dossier and a
   staged-art capture.
4. Re-pin or retract the "60 judged" rig-runbook claim contradicted by
   `docs/captures/t76/findings.md` / `t280/findings.md` (story 8).

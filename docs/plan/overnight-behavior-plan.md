# Overnight autonomous plan — behavior pack + ranked delta

Goal: close the owner-visible delta from `docs/coverage-audit.md`
autonomously. Evidence-first, data-driven, clean-room. The controller
(main agent, driven by `/goal-loop`) sequences streams; subagents do
the work and NEVER commit (the controller reviews and commits per
stream). Gates before every commit: `cargo fmt --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` — with `install-root` present AND hidden
(restore after). Conventional Commits, pushed to `main`.

## Standing rules (all streams)

- Clean-room: behavior from our docs/evidence, never original bytes;
  no disassembly or decompiled code in the repo. The exe arbitrates.
  decomp2jam (`~/Projects/open3jam/decomp2jam`) is roadmap/corroboration
  only — any claim it inspires gets exe-verified on our side first.
- Every constant/behavior carries a doc comment tracing to its
  evidence (doc + offset/VA or capture). Anything unverifiable stays
  labeled OPEN with a named next step — never invented.
- No `ask` tool. No human gates. Blockers get one retry, then a
  documented blocker and the stream moves on.
- Agents run `cargo check -p <their crate>` while working (the
  controller runs full gates at commit time, sequentially, to avoid
  Cargo.lock contention). Only Stream C may add dependencies.
- File ownership is exclusive per stream; do not touch other streams'
  files. Docs owned by the controller (`channel-fill.md`, `planet.md`,
  `lifecycle.md`, `parity.md`, this file).

## Stream A — Scene BGM + mixer (owner: crates/o2jam-audio)

Pinned contract (`~/scratch/bgm-evidence/findings.md`, corroborated by
decomp2jam `SceneMusic`, cross-checked 5/5 in `docs/coverage-audit.md`):

- Selector semantics: `scene_music(id, loop)`; `id = -1` = silence;
  bank model: one BGM stream at a time (a new selection replaces the
  old — CXO2's mixer agrees: Play detaches the source from all other
  groups).
- Per-state id table (immediate-id callers of file `0x151820`):
  Avi→8 (once), Planet→9 (loop), Room/Payment/Payment2→3, Shop→4,
  Myroom→5, Waiting→6, MusicShop→0x10, Result→0x13, option preview→0,
  transitions→−1. Record indexing is ONE-BASED over `Music/BGM.ojm`
  records; empty records preserved.
- Work: minimal BGM channel in `o2jam-audio` (reuse cpal backend +
  lewton via existing decode; loop flag = stream repeat); a
  `SceneMusic`-shaped API owned by the shell: `play_scene_music(id,
  loop)`, `stop_scene_music()`; wire Avi (on splash enter → id 8, no
  loop) and Planet (on board enter → id 9, loop; on leave → silence).
  Wire Room/Waiting/Result ids only if their scene-entry hooks are
  trivially reachable without touching Stream B/F files; otherwise
  leave for later streams.
- Audio output: cpal default device; this is user-initiated desktop
  software — speakers are fine. Tests: null backend only; every test
  skips cleanly without install-root (BGM.ojm absent → no-op + log).
- Files: `crates/o2jam-audio/**`; shell glue ONLY in
  `crates/o2jam-shell/src/audio*` if a new module is needed (coordinate:
  prefer adding the call in `recomp2jam/src/main.rs` windowed setup and
  `ScreenGame` scene-change hook — see Stream B boundary below).

## Stream C — Hangul text rasterizer (owner: new `crates/o2jam-text`)

Owner decision (memory `e8d01`): modern cross-platform rasterizer,
exe pins positions/sizes. Work:

- New crate `o2jam-text`: CP949 decode (bytes → Unicode) + rasterize
  via `ab_glyph` + system Noto Sans CJK KR
  (`/usr/share/fonts/noto-cjk/NotoSansCJK-*.ttc`, OFL); measure
  (advance widths) for `text_width` parity. Load font at runtime from
  system paths; WITHOUT the font (CI) every API degrades to the
  current ASCII stand-in behavior — no test may fail for a missing
  font. No font binary is vendored into the repo.
- Ship a `TextPainter` the shell can ask for `draw(text, x, y)` and
  `width(text)` with CP949 input; keep the existing micro-font path as
  fallback.
- The ONLY stream allowed to add crates.io deps (`ab_glyph`) and touch
  the workspace `Cargo.toml`/`Cargo.lock`.
- Files: `crates/o2jam-text/**`, root `Cargo.toml`, `Cargo.lock`.

## Stream B — Planet behavior pack (owner: crates/o2jam-shell planet
  parts + `crates/recomp2jam/src/main.rs`; starts AFTER Stream C lands)

- Emblem captions: TEXT controls `0x19400007..0B` → `State_Planet.bnd`
  rects 6..10 (94x13, y125-138); strings pinned in `docs/planet.md`
  (사람손 (초급) 0x247ae0, 고수손 (중급) 0x247af0, 신의손 (고급)
  0x247b00, 퀘스트 0x247acc, 튜토리얼 0x247ad4) via Stream C rasterizer.
- Emblem-gated dialog: boot shows the BARE board; a lit-emblem click
  (HARD/NORMAL/EASY pinned rects) opens the channel dialog (existing
  `Planet→Channel` edge); X closes back. The earlier "auto-open at
  boot" observation (launcher-preselected sessions only) is documented
  in the code comment as a caveat; preselection handling stays OPEN.
- Dispatch table: control-code → action wiring lives in a data table
  (e.g. `PLANET_ACTIONS: &[(code, rect, action)]`) — no per-button
  literals in handlers. Power (`IDC_BUTTON_SPLANET_EXIT`, bnd rect 26)
  → quit-confirm dialog (rig-verified popup); gear
  (`IDC_BUTTON_SPLANET_OPTION`) → opens the options dialog
  (`Dialog_Option_*` art stages; which tab opens first stays OPEN).
- BGM hook: scene-entry calls Stream A's `play_scene_music` (Avi 8
  once / Planet 9 loop / leave −1).
- Files: `crates/o2jam-shell/src/screens/{paint,side,game}.rs`
  (planet/channel parts only), `crates/recomp2jam/src/{main,shots}.rs`
  (window/shot wiring), planet caption constants. NOT
  `docs/{channel-fill,planet,lifecycle,parity}.md` (controller owns).

## Stream D — Jam gauge (owner: crates/o2jam-game)

- Exe-verify first (charter): read the jam threshold table at file
  `0x170260` (VA `0x570260`; decomp2jam has it `matched` as
  `jam_threshold_table` — verify the raw bytes + the scorer call sites
  on our side: Cool +2 / Good +1, `30` per jam, overflow carries; the
  table read is data, allowed). Confirm Bad/Miss feed nothing.
- Implement `JamGauge` in `o2jam-game` mirroring `LifeBar`'s shape:
  difficulty-independent per the verified data; scenario-named tests
  including the table-byte pin.
- Files: `crates/o2jam-game/**`.

## Stream E — Evidence closes (owner: ~/scratch + docs drafts to the
  controller; no repo edits)

- Dialog-open trigger: who mounts the 0x8C dialog on emblem click.
  Static-only attempt first (byte scans around the emblem handler
  region + the vftable slot-5 builders); if dynamic-only, document the
  exact next dynamic step (memory watch on `parent+0xF6C` during an
  emblem click under the Wine rig, stopped-state reads). Deliverable:
  a findings note the controller merges into `docs/channel-fill.md`.
- Playing combo/avatar anchors: attempt the named next step for the
  Attempt-13 wedge (winedbg stopped-state read of the play-stage init;
  if the wedge persists, document the blocker with the tee trace and
  stop — no guessing).
- Files: `~/scratch/overnight-evidence/**` only.

## Stream F — Waiting/room interactions (owner: crates/o2jam-shell
  waiting/room parts; starts AFTER Stream B lands)

- Team buttons 4004 + ready 4008 wiring per `docs/gameplay.md` (the
  fold gap: member numbering off slot indices — fix or document),
  create-room + password dialogs (stage `Dialog_CreateRoom`/
  `Dialog_Password` art at their bnds; form behavior stays OPEN where
  unevidenced).
- Files: `crates/o2jam-shell/src/screens/{paint,side,game}.rs`
  (waiting/room parts), `crates/recomp2jam/src/shots.rs`.

## Sequencing

1. Wave 1 (parallel): A, C, D, E.
2. Controller commits each landed stream (gates both ways, sequential
   to avoid Cargo.lock contention), updates `docs/parity.md` rows.
3. Wave 2: B (needs C). Then wave 3: F (needs B).
4. Continue down `docs/coverage-audit.md`'s ranked delta while budget
   remains; otherwise stop with a summary: done / blocked-with-next-
   step / remaining budget.

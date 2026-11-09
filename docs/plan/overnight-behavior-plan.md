# Gap-closure program — subsystem parity with the 7.55 exe

Goal: close the gap between the clean-room Rust client and every
subsystem of the original exe until the application functions
identically. Owner directives (2026-11-10):

1. **When in doubt, static analysis and REA; the exe is the source of
   truth.** A static miss (zero LE32 hits, no dossier) means "go
   dynamic" (Wine rig), never "negative".
2. Close the gap subsystem by subsystem, ranked by owner-visible value
   (`docs/coverage-audit.md`'s delta list), each sprint verified
   against the oracle.

The controller (main agent) sequences sprints; subagents do the work
and NEVER commit — the controller reviews, runs gates, commits
(Conventional Commits), pushes, and updates `docs/parity.md`.
Backend sprints (audio/game crates) may run in parallel; sprints
touching `o2jam-shell` scenes run sequentially (shared files).

## Standing rules (all sprints)

- **Evidence-first**: every behavior constant/doc comment traces to a
  pin (doc + file offset/VA, REA evidence ID, or capture path). Claims
  inspired by decomp2jam (`~/Projects/open3jam/decomp2jam`) get exe
  verification on our side before coding. Unverifiable items stay
  labeled OPEN with a named next evidence step — never invented.
- **Clean-room**: behavior described in our own words; no original
  bytes, disassembly, or decompiled code in the repo. Data-byte reads
  (tables) are allowed; describe values in words with offsets.
- **Data-driven**: geometry/behavior wiring rides parsed archives and
  control codes (owner rule `f38f3`: the `.bnd` controls, code only
  parses; hardcoded constants are scaffolding). New UI wiring uses a
  control-code → action dispatch table, not per-button literals.
- **REA access**: `rea` CLI is primary here — Pi Pocket's core has no
  MCP loader, so `mcp__rea__*` never appears in this environment
  (the same MCP server is registered for the Pi CLI at
  `~/.pi/agent/mcp.json`, where `/reload` makes it available;
  same engine). Snapshot profile mismatches are resolved with a
  fresh `rea analyze` (no stale `--snapshot`).
- **Gates before every commit**: `cargo fmt --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` — with `install-root` present AND hidden
  (restore after). The hidden-install-root run is **controller-serial
  at commit time, never by parallel sprint agents** — renaming
  `install-root` races concurrent `cargo test` in other sprints.
  Agents run `cargo check -p <crate>` only (shared tree); only the
  font sprint may add crates.io deps / touch root
  `Cargo.toml`/`Cargo.lock`.
- **No `ask`, no human gates.** A failed subagent gets one retry, then
  a documented blocker; the sprint moves on.
- **Audio containment** (memory `979fb`): rig sessions and tests use
  the null audio path; the user-invoked desktop run may use speakers.
- Docs owned by the controller: `channel-fill.md`, `planet.md`,
  `lifecycle.md`, `parity.md`, `progress.md`, this file.

## Sprint shape (fixed, all sprints)

1. **REA static pass** — dossier/decompile the pinned VAs and data
   tables; record behavior in our own words with offsets.
2. **Implement** — Rust, evidence-cited doc comments, consuming parsed
   data; owner rules (`f38f3`, `e8d01`, clean-room) hold.
3. **Tests** — scenario-named; real-data paths skip cleanly without
   `install-root`; suites green both ways.
4. **Live verify** — Wine rig (recipe: `~/scratch/planet-verify/
   findings.md`; SendInput helper; winedbg stopped-state discipline)
   where the state is reachable.
5. **A/V compare** — `recomp2jam-av-compare` scripts vs real captures
   where frames exist (`~/scratch/av-reports/` conventions).
6. **Parity row** — controller updates `docs/parity.md` with verdict +
   evidence citations.

## Sprint 0 — planet behavior pack (owner corrections, all pinned)

The four owner corrections plus captions; absorbs the halted plan's
streams A (BGM) and C (font).

- **S0.1 font** (owner `e8d01`): new `crates/o2jam-text` — CP949
  decode + `ab_glyph` + system Noto Sans CJK KR
  (`/usr/share/fonts/noto-cjk/NotoSansCJK-*.ttc`, OFL; no vendored
  font). Font-absent degrades to the ASCII stand-in; no test may fail
  for a missing font. Files: `crates/o2jam-text/**`, root
  `Cargo.toml`, `Cargo.lock`.
- **S0.2 scene BGM**: selector contract `play_scene_music(id, loop)`,
  `id = -1` silence, one BGM at a time (new selection replaces).
  Pinned id table (immediate callers of file `0x151820`; double-sourced
  in `docs/coverage-audit.md`): Avi→8 once, Planet→9 loop,
  Room/Payment/Payment2→3, Shop→4, Myroom→5, Waiting→6,
  MusicShop→0x10, Result→0x13, option preview→0. Records ONE-BASED
  over `Music/BGM.ojm`. Minimal BGM channel in `o2jam-audio`
  (cpal + lewton via existing decode; loop = repeat until replaced).
  Files: `crates/o2jam-audio/**` + a minimal scene-change hook in
  `crates/recomp2jam/src/main.rs` **only** (named here so wave-2
  S0.3 edits to the same file cannot collide unnoticed).
- **S0.3 planet pack** (`o2jam-shell` planet/channel parts):
  emblem captions (TEXT `0x19400007..0B` → `State_Planet.bnd` rects
  6..10; strings pinned `docs/planet.md`: 사람손 (초급) `0x247ae0`,
  고수손 (중급) `0x247af0`, 신의손 (고급) `0x247b00`, 퀘스트
  `0x247acc`, 튜토리얼 `0x247ad4`) via the S0.1 rasterizer;
  **emblem-gated dialog** — boot shows the bare board, a lit-emblem
  click opens the channel dialog, X closes (the earlier boot auto-open
  observation came from launcher-preselected sessions — documented
  caveat; preselection handling stays OPEN); **dispatch table**
  control-code → action: power (`IDC_BUTTON_SPLANET_EXIT`, bnd rect 15 — corrected: the file has 17 rects, so rect 26 cannot exist; OPTION is rect 16)
  → quit-confirm (rig-verified popup), gear (`IDC_BUTTON_SPLANET_OPTION`)
  → options dialog (`Dialog_Option_*` art; first-open tab OPEN);
  BGM hooks via S0.2 (Avi 8 once / Planet 9 loop / leave −1).
  Files: `crates/o2jam-shell/src/screens/{paint,side,game}.rs`
  (planet/channel parts), `crates/recomp2jam/src/{main,shots}.rs`.

## Sprints 1–7 — ranked delta (`docs/coverage-audit.md`)

- **S1 Scene BGM playback** — covered by S0.2; this sprint finishes the
  remaining state hooks (Room/Waiting/Result ids at their scene
  entries) once S0.3 lands.
- **S2 Jam gauge** (`crates/o2jam-game`): verify the threshold table
  bytes at file `0x170260` (VA `0x570260`) on our side, plus the
  scorer call sites; implement `JamGauge` mirroring `LifeBar`.
  Prior hypothesis: Cool +2 / Good +1, 30 per jam, overflow carries,
  Bad/Miss feed nothing — verify, don't assume.
- **S3 Shell clock wiring**: `LifeBar`/`JamGauge`/`HoldState` and the
  6/18/25 ms windows into the Playing scene on the sample clock
  (currently wall clock; dead-at-0 gate invisible).
- **S4 Playing/result digits + WinLose + grade**: bnd-pinned rects
  (result 7, playing EXIT…VOLUME run); grade strings are provably
  code-unreferenced (mission-side) — pin the mapping before painting.
- **S5 Waiting/room completion**: team buttons + covers
  (`Wait_TeamA..H.ojs`, 4004/4008 send path, `0x48C810` map), create/
  password dialogs (`Dialog_CreateRoom`/`Dialog_Password` bnds), 4007
  slot refresh, result SubmitScore/FinalizeRank.
- **S6 Rating-event sink** (`+0xa14`, attach `0x56E170`, codes 1/9/14):
  unlocks jam/life server sync truths.
- **S7 Loading-gate validation** (`0x5718B0` → `0x4F8B80` → `0x412180`
  → `0x54DB40`): honest loading screen behavior.

## Evidence closes (parallel, any wave)

- **E1 channel dialog-open trigger** (emblem → 0x8C mount): static
  writer-site hunt; if dynamic-only, the named next step is a memory
  watch on `parent+0xF6C` during an emblem click (stopped-state
  winedbg). Writes draft evidence to `docs/overnight-evidence/`
  **in-repo** (described behavior + offsets + evidence IDs only,
  no original bytes), controller merges into `docs/channel-fill.md`.
- **E2 Attempt-13 wedge** (playing never starts under Wine): one
  hypothesis per session on the play-stage init (DXSurface suspect;
  relay refuted), else documented blocker with the tee trace.

## Sequencing

0. Controller creates `docs/overnight-evidence/` (in-repo evidence
   drafts; committable) before Wave 1.
1. Wave 1 (parallel): S0.1 (font), S0.2 (BGM), S2 (jam), E1, E2.
2. Controller commits each landed unit (gates both ways, sequential),
   updates `parity.md`.
3. Wave 2: S0.3 (planet pack; needs S0.1+S0.2). Then S1 completion,
   S3 (needs S0.2+S2), then S4–S7 sequentially (shared shell files).
4. Continue while budget remains; stop with a summary: done /
   blocked-with-next-step / remaining budget.

## Supersession

The gap-closure sprints (S0–S7) completed 2026-11-10 (all landed,
`docs/parity.md` updated). Gameplay-scene parity stays deferred on the
E2 wedge. Further application work continues under
`docs/plan/usable-client-plan.md` — its **interactive-verifiable**
standing rule (window path + OS-boundary trace + smoke note) governs
every later sprint, per the owner directive that interactive
verification is the whole point.

## Definition of done

Every subsystem row in `docs/parity.md` reaches PASS (exe evidence +
implementation + passing test together), or stays OPEN with a named
next evidence step. That checkpoint is the "identical functioning
application" claim.

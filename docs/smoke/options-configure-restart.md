# Options configure/restart session — smoke note (2026-11-18)

Ticket #18, last acceptance box. Offline run: release
`recomp2jam play --window --no-sound` at `d191bb8`, no Memoryer and no
gateway. Private Xvfb `:277` at 800x600; `DISPLAY` pinned,
`WAYLAND_DISPLAY`/`WAYLAND_SOCKET` removed, `WINIT_UNIX_BACKEND=x11`.
`scripts/rig-run.sh` opened a **per-run owned null sink** for each phase
(`options18_null_configure_*`, `options18_null_verify_*`): audio sidecars
record `source_verified=yes`, `route=unseen` (silent client),
`on_default_sink=0`, `preboot_breach=0`; both phases ended
`state=captured` and both `stop` calls exited 0. No owner audio device
was ever the route. An earlier launch attempt was refused by rig-run
(exit 10) because the driver put the private display in the caller's
`DISPLAY`; the driver now leaves the caller display untouched and lets
rig-run export `:277`. No product code was changed for that workaround.

Real input only: `fixtures/input-trace/options-configure-restart.json`
(sha256 `9cd5aa399cfb4081571c8bd3b88df01453bc923c61ffd1ececaceb0c075d1330`)
was replayed by `~/scratch/options-18/drive_session.py`
(sha256 `a388a6fabcb75b5355c71e5b74f9eb3bc45fab9e995d9949ce8a6cc5654287c6`)
through python-xlib XTEST at the winit window. Clicks/keys/text go
through `WindowHandler` → frozen snapshot → `GameLoop::ingest_snapshot`;
`ini`/snapshot rows in the trace are read-only checkpoints, never a
state-walk shortcut. Aims are native pixels: gear `(729,580)` from
`scripts/wine-scene.py` `AIMS["option-gear"]`; dialog controls are
`Dialog_Option_*.bnd` rect centres from `o2jam-dump rect` plus the
`(217,42)` dialog origin (`docs/geometry-audit.md`).

## What was driven, what persisted

Configure phase (16 events): Escape dismissed Avi → planet; gear
`(729,580)` opened Key; `KEY1 (320,362)` armed lane 0 and XTEST `a`
rebound it; Key SAVE `(409,532)`; SOUNDSET `(405,123)`, BGM Down
`(301,212)` and Sound SAVE; GAMESET `(509,123)`, guide Short
`(404,403)` and Game SAVE; Escape closed the dialog. Then the client
process was stopped and restarted in a fresh rig run. Verify phase
(5 events): old key `s` and new key `a`, then gear again.

`option.ini` lived in the private root only (never committed). Its
sha256 chain: `f35155ce…` (before) → `e5852e93…` (Key commit:
`KEY_1=30`, DirectInput A) → `e6c73112…` (Sound commit:
`GameBGVolume=247`) → `2e06946d…` (Game commit: `NoteTail=1`, Short)
→ **same `2e06946d…` after restart** and after reopening the dialog.
Full before/after text and parsed values are in the manifest.

Live after restart: `s` logged `key-down KeyS` with no `judged` line;
`a` logged `key-down KeyA` plus `judged 2858 lane 0`, and moved
avi → planet (state `8a5514f3b73148ba` → `69d750f8ee62f6e6`). The
reopened options sheet reached state `5b8692ca9b9246cf`, the same
modal state and content hash (`c323f98e0bcf7044`) as the pre-restart
sheet.

Evidence: `~/scratch/options-18/session/manifest.json`
(sha256 `df6a0a7effab1435d19c2db9b3e7019dd04aafa1ca0d92bb8475d83459963b30`)
holds every applied event with applied time, newly written log lines,
and a per-step frame state/content hash. Configure: 25 applied-event
log lines, 54 `frame` lines, 3 PNG dumps, `input.log` sha256
`1d8a81a633e16e7cf90d452bb345616a51d37d0aa412838304b1bb2bed325067`.
Verify: 8 log lines, 35 frames, 2 PNGs, `input.log` sha256
`3b486867f998d4ec65147de022d783c8ab3782c5081e280270b70ec5af2763ca`.
Frames and ini bytes stay in scratch; they may contain genuine art.

## Gates (final tree, both arms)

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test --workspace`: exit codes `0/0/0` with
`install-root` present and `0/0/0` with the worktree data symlink renamed
aside and restored (path rechecked; the symlink now points at this worktree's
own persistent pool slot). Each arm reported 28
`test result:` lines (19 binary + 9 doc-test sections) and
`2624 passed; 0 failed` in total.

## Deltas found (new rows, never silent fixes)

1. **Volume and guide effects after restart stay OPEN.** The ini
   values persist and the dialog reopens with the same ink, but this
   offline no-sound session never enters Playing, so no gain or guide
   ink A/B was observed. Next step: a chart-backed Playing entry with
   the same ini, capture the BGM gain and the guide strip.
2. **Silent audio route is containment, not gain parity.** `route=unseen`
   means `--no-sound` opened no stream; the null sink and
   `on_default_sink=0` prove containment only. Next step: a sound-on
   run on the owned null sink to compare levels.
3. **Hidden install-root arm failed at `d191bb8`** in three tests that
   asserted varied Loading ink without the staged catalog
   (`e2e_walk…`, `shots::walk_covers…`, `staging_attempt…`). Fixed by
   skipping only that data-less ink assertion while keeping the screen
   and wiring checks; both arms were re-run green after the fix.

# Native 800x600 A/B capture path (issue #334)

Use this path for blit-fidelity work. It replaces the #329/#332 downsampled
comparison: Wine 800x600 was bicubic-scaled to 400x300 while Rust frames were
point-sampled from a native paint. That pair could hide native-pixel placement
errors under ~2 px. Keep the old pipeline only as a documented fallback when a
native capture cannot stage a scene; label every such result downsampled.

## Capture the Rust side

All commands run from the worktree root. Game data stays in git-ignored
`install-root/`; no game art, music, or capture bytes enter the repository.

1. Copy data and build the release client. In a git worktree,
   `scripts/worktree-setup.sh` already made the real copy; elsewhere copy it
   yourself (remove any old symlink first — never `ln -s` or `cp -al`):

   ```bash
   cp -a /path/to/install-root install-root   # skip when it already exists
   cargo build --release -p recomp2jam
   ```

2. Check the local Memoryer gateway on `127.0.0.1:15010`. Read the session
   token from its database; never write the token into a manifest, log, or
   issue comment:

   ```bash
   python3 - <<'PY'
   import sqlite3
   conn = sqlite3.connect('/home/arnim/memoryer-local/O2JAM.db')
   print(conn.execute(
       'select TUSER_ID from t_o2jam_login where USER_INDEX_ID=? '
       'order by LOGIN_TIME desc limit 1', ('3',)).fetchone()[0])
   PY
   ```

3. Use `fixtures/input-trace/u1-windowed-online.json` (planet → channel → room
   → waiting → loading → playing → result), or a truncated copy that ends with
   `await-screen loading`. Record the trace hash. The #334 example used the
   first 19 events of that fixture, hash
   `ab89594201fe7efad709b795b94cc42cb5d04afdd857d437ddf0876175e4ef0d`.

4. Replay it through the real window with the repo driver. It starts a private
   Xvfb, injects XTEST events, strips Wayland variables, and passes
   `--no-sound`, so no game audio can reach the owner's speakers:

   ```bash
   python3 scripts/replay-trace-native.py \
     --trace /path/to/native-walk-to-loading.json \
     --binary target/release/recomp2jam \
     --out "$HOME/scratch/native-ab-334/run2" \
     --display :351 --user-id 3 --token "$TOKEN" \
     --gateway 127.0.0.1:15010 --gateway-id 0 --channel-id 0 \
     --music-id 757 --no-sound --await-timeout 60
   ```

5. Verify before scoring: driver exit 0, one applied event per trace event,
   and `frames/frame-*-<scene>.png` at 800x600 for each expected scene. The
   driver's `input.log` has `frame N … screen=<scene>` lines for the sync
   points. Reject runs with the wrong scene, an unblocked popup, or missing
   frames; do not repair them by hand-picking pixels.

## Pair with the reference side

Keep each side's producer identity. For 7.55, the exe is
`OTwo.exe`, 5,029,888 bytes, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
The #334 example used the 800x600 Wine screenshots in
`~/scratch/no-response/c3-waiting-qualified/artifacts/screenshots/` for
planet/channel/room/waiting and
`~/scratch/no-response/c1-start-cue/artifacts/screenshots/waiting-cue-012.png`
for loading (that file is mislabeled as `waiting-cue-012` in the source pack).
Confirm the scene visually on both sides before metrics.

Copy frames byte-for-byte into a local-only pack. The #334 pack layout is:

```text
pack/
  manifest.json          # Wine pack conventions: pair/build/params/seed/artifacts
  provenance.json        # producer identity, command, tool versions, transformations
  artifact-manifest.json # per-file sha256 + size; excludes itself
  frames/ref/<scene>.png frames/test/<scene>.png
  media/ref/<scene>.mkv  media/test/<scene>.mkv
  av-batch-native.json   # native pairs; no width/height fields
  regions-v1-native.json region-metrics.json
  reports/<scene>/summary.json
  test-capture/          # driver manifest, input/log/applied/client logs
```

The committed helpers build that layout from the two source packs. They copy
bytes only into the local pack; no game data enters the repository:

```bash
python3 scripts/native_ab_pack.py build \
  --pack "$HOME/scratch/native-ab-334/pack" \
  --wine-lobby "$HOME/scratch/no-response/c3-waiting-qualified" \
  --wine-loading "$HOME/scratch/no-response/c1-start-cue" \
  --rust-run "$HOME/scratch/native-ab-334/run2" \
  --trace "$HOME/scratch/native-ab-334/native-walk-to-loading.json" \
  --legacy-map "$HOME/scratch/ab-parity-332/regions-v1.json" \
  --thresholds .agents/skills/recomp2jam-av-compare/references/default-thresholds.json

python3 scripts/native_ab_regions.py \
  --pack "$HOME/scratch/native-ab-334/pack"

python3 .agents/skills/recomp2jam-av-compare/scripts/batch_compare_av.py \
  "$HOME/scratch/native-ab-334/pack/av-batch-native.json"

python3 scripts/native_ab_pack.py refresh \
  --pack "$HOME/scratch/native-ab-334/pack"
```

The builder finds the Rust scene frame by the driver's `frame-*-<scene>.png`
name; it refuses zero or multiple matches instead of hand-picking pixels.
It wraps each still once as FFV1 `bgr0` plus silent PCM (`-t 1`, 48 kHz
stereo) only because `compare_av.py` requires an audio stream. No
game-audio parity is claimed. Do not pass `--width`/`--height` or `--fps`:
both inputs are already native 800x600, and resampling would reintroduce the
old fault. Re-run `refresh` after reports change to restore full provenance.

## Region maps

`regions-v1.json` from #332 is normalized to 400x300. Either rescale every
rect by the documented factor (the #334 baseline multiplied each coordinate
by 2) or re-derive native boxes. State which method was used. Carry over the
`gameplay` / `text` / `mixed` classes unchanged; a rescale does not make a
caption crop gameplay. Score only `gameplay` crops and gate on the worst
scored region.

## Example results and fallback

**Stale but pinned:** the #334 native run under
`~/scratch/native-ab-334/pack/` (hash-pinned in closed #334's record) is kept
for provenance. Do not delete it. Its five scenes all failed the skill-default
gate, but it predates #339's matched-state method and #343's waiting fix, so
do not cite its numbers as current parity. Per-region numbers remain in that
pack's `region-metrics.json`. The #329/#332 downsampled media remains
available as a fallback, but any report using it must state the two different
filters and the hidden sub-2-pixel placement error.

## Current example: #346 matched-scenario baseline

The current reference numbers are `docs/captures/t346/region-metrics.json`
from `~/scratch/p346/pack/`, built from one matched replay on one Memoryer
(`docs/captures/t346/proof.json`, `raw-proof.json`, `plan.json`, `trace.json`).
The same wire/server state ran on both legs; containment was `route=sink`,
`on_default_sink=0`, `preboot_breach=0`. The pack `manifest.json` records
`server_boot=2026-11-18T15:14:16+08:00 server_pid=92326` with a post-hoc
identity note; the pid and boot time were unchanged on the post-leg re-check.
All scenes still FAIL the default worst-region gate (SSIM ≥ 0.95, PSNR ≥ 35
dB). Weighted SSIM, old → new:

| Scene | Weighted SSIM | Worst scored region | Residual |
|---|---|---|---|
| planet | 0.793 → 0.793 | 0.729 `emblem_strip` | unchanged from #334 |
| channel | 0.690 → 0.872 | 0.606 `row_bars` | row-bar art |
| room | 0.728 → 0.915 | 0.875 `peer_avatar` | peer plate art |
| waiting | 0.240 → 0.912 | 0.878 `seat_grid_left` | #343 fixed right seats; left-seat art remains |
| loading | 0.185 → 0.156 | 0.032 `logo_plate` | Rust stays on Waiting pixels after matched start |

Waiting states also have matched verdicts: download 0.880, gate-closed 0.705
(`seat_grid_right_top`), back-to-base 0.878. Full per-scene numbers, proof
filters, and the false-start investigation are in
`docs/captures/t346/findings.md`. Copy the still pairs byte-for-byte into a
local pack and run `scripts/native_ab_regions.py --pack <pack>`; no capture
bytes enter the repository.

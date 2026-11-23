# #873 — tutorial guide records on the standard boot path (7.55)

Oracle: `OTwo.exe` 7.55, 5,029,888 B, sha256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
Statics: REA CLI 6.9.0 / Ghidra 12.2. Offsets are file offsets, VA =
file + `0x400000`. No disassembly or decompiled source is stored here.

## Static pins

| Id | Source | Pin |
|---|---|---|
| `S-873-TEXT` | REA `ev_0bd93599…` (xrefs of `0x42f870`), `ev_e3dc9b5c…` (`0x4f19e0`), `ev_7696adff…` (`0x4f0d30`), `ev_49cf8f68…` (`0x42f870`), `ev_e22dd01d…` (`0x41e0f0`) | Setup stores `Guide_Ani.ojs` at state `+0x910`, `Guide_Text.ojs` at `+0x914`, `Guide_Key.ojs` at `+0x918`. Tutorial Update crosses the 13 deadlines from `0x67e2c0`, applies the frame setter to the `+0x914` control (the lesson body), latches a 1 s re-cue, and increments the page counter. The setter writes a one-based frame and publishes `frame - 1` as the image record. Page N therefore shows `Guide_Text.ojs` record N-1. |
| `S-873-COACH` | REA `ev_9110a027…` (`0x42f5a0`), `ev_7db20465…` (`0x42f400`), `ev_ca776124…` (`0x42f040`), `ev_7c8bdf16…` (xrefs of `0x41e0f0`) | Setup arms the `+0x910` coach control in mode 3 with a 66 ms (`0x42`) tick and 12 records. The generic sprite tick advances that one-based frame every tick, wraps to record 0 after record 11, and publishes `frame - 1` through the same image-index writer. Coach record at chart time t is `floor(t / 66 ms) mod 12`; the page writer never targets the coach control. |

## Implementation

`paint_tutorial_scene` (`crates/o2jam-shell/src/screens/side.rs`) receives
`TutorialPaintState { frames, key_lights, now_ms }`. The frame pair comes
from `TutorialGuideFrames::for_page_and_chart_ms(self.tutorial_page,
chart_ms)`: `page - 1` for the bubble, the 66 ms cycle for the coach. The
#868 clock supplies `tutorial_page` and `chart_ms`; geometry stays the
parsed anchors and staged placements.

Red proof first: `tutorial_deadline_pages_select_lesson_records_and_coach_advances_per_exe_tick`
failed before the change with `page 2 selects record 1` (left `524543`,
right `1048831`) and passed after wiring. Boundary guard:
`tutorial_guide_frames_pin_page_records_and_coach_wrap`. The old story-12
guard was renamed to `tutorial_guide_text_follows_chart_pages_not_wall_clock_or_keys`
and now paints record 1 at 45 s.

## Live proof (close condition)

Recipe re-run of `n-page-captures.json` from `~/scratch/tutorial-captures-2`,
copied and repinned under `~/scratch/fix873/`. Native Rust standard-path
boot, no `--join-room`, no CREATE click, one owned Xvfb `:131`, one
shared Memoryer (`pid 37333`, boot epoch `1795246022`, unchanged),
ffmpeg x11grab per deadline. Trace `S873-NPAGE-542-546-868`.

- Pack: `~/scratch/fix873/recipe/runs/n-page-fix873/`
- Build: `ec0b174`, binary sha256
  `449883800da8c5b257eff828176d6f181b7124cbfc9d66dc1023867584c8f23a`
- Manifest sha256 `2429841264639a0610d9bc8aa2327f2efcbc7eaa3cd1068145edc934596ac4ad`
- Pack hash index `hashes.txt` sha256
  `9603310b8cb6f6b45aafe7945c64d3fb9ee30e88455045aea7290d950a0784cc`
- Audio: `PASS — house sink route accepted`, `on_default_sink=0`
- `run-manifest.sh verify`: artifact set complete, provenance complete,
  audio verified

| Check | Result |
|---|---|
| Deadline captures | 13 of 13, one per pinned deadline 5.0 … 227.0 s |
| `Guide_Text.ojs` bubble crop vs shipped records | best indices `[0,1,2,3,4,5,6,7,8,9,10,11,12]`; page match 13/13; mean absolute delta 0.0 |
| `Guide_Ani.ojs` coach crop vs shipped records | best indices `[5,2,7,5,7,9,11,2,0,4,2,4,6]`; 8 distinct records; mean absolute delta 0.0 |
| Coach 66 ms cadence | 12 of 12 frame-matched captures predict the paired coach record exactly from the paint frame's chart time (`artifacts/stats/capture-frame-matches.json`, `coach-cadence-verdict.json`) |

Two deadline captures could not be matched to a logged client paint
frame (the guide pixels are pinned there, but the frame hash is not
recoverable through the X11 grab); no cadence claim is made for those
two frames beyond their exact record pairing. The overall verdict for
this recipe is `pass=true` in `coach-cadence-verdict.json`.

## Gates

`bash scripts/gate-check.sh --arm quick --no-push` → exit 0; archive
canary ran. `bash scripts/gate-check.sh --arm hidden --no-push` → exit 0.

## Verdict

#542 (bubble records for pages 2–13) MET. #546 (coach cadence instead of
a frozen first record) MET with the pinned 66 ms, 12-record loop. No
invented per-page coach table was used: the mapping is derived from the
exe's record writers and animation armer.

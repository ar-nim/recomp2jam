# #868 — tutorial chart clock on the standard boot path (7.55)

Oracle: `OTwo.exe` 7.55, 5,029,888 B, sha256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
Static pins: REA CLI 6.9.0 / Ghidra 12.2, file offset = VA − `0x400000`.
Behavior is described in words with addresses only; no decompiled
original code is stored here.

## Static evidence

| Id | Source | Pin |
|---|---|---|
| `S-868-STATE` | REA static, evidence `ev_e3dc9b5c0f4d71f00168d19014e98b5a101ed4c1daffc870e13c677288d4abca` | `CStateTutorial` Update `0x4f19e0` requires play state 2 (from `0x570a50`), then compares the 13 page deadlines at `this+0x898` with the value returned by `0x5711e0` while `this+0x8cc < 13`. Instruction `0x4f1b8f` loads the tutorial state's bound object at `+0x620` into ECX before the clock call at `0x4f1b95`; a due page latches a 1000 ms re-cue and calls `0x42f870`. |
| `S-868-CLOCK` | REA static, evidence `ev_b42ec15e0f332b0bba1fa7b69b55295df9ea603d7d3418546dafb46bf79165f4` | `0x5711e0` reads a 32-bit millisecond field at `+8` of that bound object and converts it to seconds by dividing by 1000. It is a chart-time counter, not a sound-cursor read. |
| `S-868-ORIGIN` | REA static, evidence `ev_f42d28f7c25ca6fdbf3addcbb8c3908da7eb86d705f013c478b3367cb00ec9d7` | The song-clock update `0x575d70` gates on play state `+0x828 == 2` and rebases its millisecond counter on `timeGetTime()` at `+0x834` (claim B38, `efdaac19…`). The tutorial's page clock is therefore the wall-derived song timer, not an audio sample counter. |

## Rust implementation

- `OnlineSession::paint` → `GameLoop::paint` on `ScreenGame` →
  `paint_at` → `paint_side` → `tutorial_chart_ms` /
  `advance_tutorial_page` / `tutorial_notes.advance_to`.
- On the standard boot the first step opens the tutorial as a side
  visit while the flow stays in Room. There is no Playing entry, so
  `song_anchor_sample` is never armed and `samples_played()` can stay
  parked: both chart notes and page deadlines froze at 0.
- Fix: `visit_side` stamps a wall origin for the Tutorial visit;
  `GameLoop::paint` feeds time since that origin into the tutorial
  paint and lane-key lights. Playing keeps its approved sample clock
  (ADR-0001). The explicit `paint_at` and fixed-clock seams stay exact.
- Guard:
  `tutorial_chart_advances_on_visit_wall_time_when_song_samples_are_parked`
  names the live chain from boot to `GameLoop::paint`.

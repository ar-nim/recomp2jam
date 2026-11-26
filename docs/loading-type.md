# Loading-screen typography routing (7.55) — spec

How the loading gate's three GDI fonts map to the shell's font cells.
Resolved 2026-11-26: the exe never reads them, so no caption routes
through any cell and the gate paints no runtime text.

Version pin: `AngelO2Jam.exe` 5,029,888 B, SHA-256
`83a04541…72c823b`. Behavior described in our own words with file
offsets; REA evidence ids live in `docs/gdi-text.md`.

## Answer first

| Gate font (`0x4e47a0`, `0x4e47e8`–`0x4e483a`) | Cell | Status |
|---|---|---|
| Trebuchet MS 27×65 w400 (`+0x524`) | `Cell::Loading` (27, 65) | Verified one-to-one |
| Arial 7×14 w400 (`+0x528`) | none | No dedicated cell |
| Gulimche 6×12 w400 (`+0x52c`) | `Cell::Box` (6, 12) | Verified shared face |

## Cell verification (`crates/o2jam-render/src/font.rs`)

`Cell::Loading.size()` returns `(27, 65)`: `CreateFontA` width 27,
height 65 maps to the cell one-to-one. Face: Trebuchet MS, weight
400 (`docs/gdi-text.md` pinned font tables). `bitmap_micro()` is
false for `Loading`, so its TTF-missing fallback is the 5×7 bitmap —
an approximation documented on the method. Pinned by test
`loading_cell_pins_the_gate_trebuchet_face`.

## Resolution: no caption routes through any gate cell (2026-11-26)

The routing spec that lived here told a future owner to add a runtime
`TITLE - ARTIST` line through `Cell::Loading`. That instruction is
withdrawn. The exe creates the three gate fonts and never reads them:
a whole-`.text` search finds no reader of `+0x524`/`+0x528`/`+0x52c` in
either Loading class (`docs/pins/misc-tail.md`, senior REA pass). Every
credit line on the 7.55 gate is baked into the OJN cover JPEG. Our
client painted a runtime caption there as a stand-in; the owner ruled
on 2026-11-26 to remove it, and `paint_loading_scene_with_status` now
inks no text.

Guards: `loading_scene_paints_no_runtime_song_text`,
`loading_cover_paints_full_bleed_with_no_overlay`,
`loading_paints_no_staged_song_caption`,
`loading_shot_staged_status_changes_no_pixel`.

`LoadingStatus::title`/`artist` stay as staged data for the wire and
live legs; nothing paints them.

## Approximations (labelled)

- The bar rect stays an OPEN placeholder owned by
  `loading-bar-geometry` (no verified rect; `MusicLoading.ojs` is a
  single 800x600 body) and is hidden until its exe pin.

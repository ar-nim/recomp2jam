# Loading-screen typography routing (7.55) — spec

How the loading gate's three GDI fonts map to the shell's font
cells, and which loading caption goes through which cell. Spec only:
`loading-bar-geometry` owns `paint.rs` — the call sites below are
for that owner to change, not this spec.

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

## Routing spec (for the `paint.rs` owner)

The song title/artist line (`LoadingStatus::caption`, the `TITLE -
ARTIST` join) takes the gate's display face: `Cell::Loading`.

Exact call sites in `crates/o2jam-shell/src/screens/paint.rs`:

1. `paint_loading_song` (line 2186): replace the
   `paint_room_detail_line` call (line 2191, which routes through
   `draw_text` → `Cell::Box`) with `draw_text_cell` /
   `text_width_cell` at `Cell::Loading`, plus a scrim sized to that
   cell (width from its advance + 8, height 65 + 4 padding).
2. Import block (line 43): add
   `use o2jam_render::font::{draw_text_cell, text_width_cell, Cell};`
   (rustfmt order: `draw_text_cell, text_width_cell, Cell`).
3. `paint_loading_scene_with_status` docs: the Trebuchet/Arial gap
   note lapses for the caption once routed (Arial 7×14 still has no
   cell; the OJN/Vorbis path stays a gap).
4. Tests: `loading_cover_paints_full_bleed_behind_bar_and_caption`
   bottom-right expectation changes (the 69-row scrim covers it on a
   64×48 frame); add a routing test asserting the Loading advance
   exceeds the Box advance for the same caption.

## Approximations (labelled)

- Which caption takes which gate font is unpinned (`gdi-text.md`
  OPEN 5 — Trebuchet 27×65 vs Arial 7×14 vs Gulimche 6×12); giving
  the song line the display face is the spec's recommendation, not
  an exe pin.
- Bar and caption rects stay OPEN placeholders owned by
  `loading-bar-geometry` (no verified rect; `MusicLoading.ojs` is a
  single 800×600 body).

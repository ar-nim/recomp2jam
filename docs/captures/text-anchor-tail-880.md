# Text-anchor tail (#880) — select-dialog cell bearing

Version pin: 7.55 `OTwo.exe` SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
Evidence pack and full per-scene tables: `~/scratch/fix-880-findings.md`.

## Verdict

The original paired-shot pack is not scene-identical for the three scenes
the canonical checker flagged. Its music-select exe frame has no song
dialog while its rust frame has one; musicshop compares a promo face with
a 3KEY/cart face; myroom differs in wallet, bag and pager data. Those
pair differences, not scene-wide baseline offsets, drive the reported
medians (3.5 / 5.0 / 2.5). Shifting a whole scene to satisfy such a pair
would fake parity.

On scene-matched re-pairs the tail has one pinned painter-family bug:
select-dialog TEXT rows ink 5 px too high.

## Pin

`Dialog_SelectMusic.bnd` band 3 starts at y 150. The `SET 0x0B
0x00017000` TEXT template for `IDC_TEXT_DSMUSIC_TITLE` (`0x27400012`,
code ordinal 17) adds y 6, so row 1's GDI cell top is y 156. The 7.55
song-select reference frame (`~/scratch/av-reports/frames/ref_musicselect.png`,
#349) inks that row from y 161. Our rasterizer anchors glyph cap tops at
the cell top, so the exe's cell carries a +5 px ink bearing. Geometry
still comes from the staged band and template; only the ink origin moves.

`paint_slot` in `crates/o2jam-shell/src/screens/select_paint.rs` now uses
`SELECT_TEXT_INK_BEARING_Y = 5`. Guard
`select_row_ink_starts_at_the_gdi_cell_bearing_not_the_template_top`
asserts the first ink row is y 161 from an independent literal. A
native `musicselect-native` companion shot makes the 800x600 anchor pair
reproducible.

## Checker

Canonical `scripts/check-text-anchors.py` on the original pack fails
(224/872 lines with |dy|>3). On the re-pairs, before the bearing the
music-select median is 6.5; after it, all touched scenes pass:

```
music-select median 2.0 | musicshop median 1.8 | myroom median 1.2
all lines: n=156, |dy|>3: 43 (down from 69 on the same pairs)
```

Remaining tail lines are art-edge / panel-content differences; the
checker's valley splitter can also match the wrong half of a taller
modern-font line. No other shared helper shift is justified by the
evidence.

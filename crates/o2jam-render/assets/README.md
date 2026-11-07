# `assets/` — embedded text face

`NotoSansKR-Regular-subset.ttf` — the one font `o2jam-render/src/font.rs`
embeds with `include_bytes!`. SIL Open Font License 1.1 (`OFL.txt`,
the license Google Fonts ships with the source font).

## Provenance

- Source: `google/fonts` repo, `ofl/notosanskr/NotoSansKR[wght].ttf`
  (variable font, OFL 1.1, Copyright 2014-2021 Adobe, Reserved Font
  Name "Source"), downloaded 2026-11-08.
- Instance: `python3 -m fontTools.varLib.instancer NotoSansKR[wght].ttf
  wght=400` (Regular weight pinned; `OS/2.usWeightClass` set to 400 by
  the instancer).
- Subset (`fontTools.subset`):
  `--unicodes="U+0020-007E,U+00A0-00FF,U+2010-2027,U+3130-318F,U+AC00-D7A3"
  --layout-features=kern,ccmp --name-IDs=0,1,2,3,6,13,14 --glyph-names
  --notdef-outline` — Basic Latin, Latin-1, dashes/quotes, Hangul
  compatibility jamo, and all 11,172 Hangul syllables (CP949-sourced
  chat, captions, map names, song titles).
- Renamed for the OFL Modified-Version rule: family
  `Noto Sans KR Subset` (`NotoSansKRSubset-Regular`); `STAT`/`vhea`/
  `vmtx` dropped. The rename avoids the reserved-font-name issue (the
  reserved name is "Source", which we never use) and marks the
  modification clearly.
- Result: 2,393,324 bytes, glyf outlines, 11,477 glyphs.
  SHA-256 `b768541814958688104bc9257885e8c17e716ff6ad0f3a29c06485a43346532f`.

## Regenerating

Re-run the three steps above with any fontTools ≥ 4.68 and re-apply
the name-table rename. `font.rs` tests pin Latin + Hangul coverage,
so a broken subset fails `cargo test -p o2jam-render` immediately.

# Sprite codec coverage — real 7.55 archives

Result of a read-only walk of every named body in the three sprite
archives, classifying each by codec and cross-referencing the lost
ones against `ControlList` sprite references. No source was changed;
archives stayed read-only; this document is the only file created in
the repo. Numbers come from the real bytes, not estimates.

## Answer: counts per archive per codec

| Archive | Named | Sprite bodies | raw-RGB555 (render) | .oji raster (prefixed render) | strided atlas (render) | .oja anim (skip) | JPEG (render) | data/`.bnd`/`.txt` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `Interface1.opi` | 1587 | 1532 | 1480 | 5 | 0 | 0 | 47 | 55 |
| `Playing1.opi` | 627 | 621 | 610 | 5 | 3 | 3 | 0 | 6 |
| `avatar.opa` | 7133 | 7131 | 7131 | 0 | 0 | 0 | 0 | 2 |
| **Total** | **9347** | **9284** | **9221 (99.3%)** | **10** | **3** | **3** | **47** | **63** |

Body-level render coverage: `Interface1.opi` 100% (1532/1532),
`Playing1.opi` 99.5% (618/621 — only the 3 `.oja` effect envelopes
still drop their frame data), `avatar.opa` 100% — 9281/9284 bodies
(99.97%). Image-record level: all 63,796 records decode — 63,760 raw
(99.94%) plus the 36 non-raw records inside the 10 `.oji` bodies
(prefixed walk, gaps ③+④). The "compressed `.ojs`/`.ojt` frame" class
is empty: no plain sprite body carries a single non-raw image, so
every formerly lost frame sat in an `.oji` or a `.oja` envelope — the
strided atlases (gap ①), the 47 JPEG album-art bodies (6.8 MB, gap ②)
and all 10 `.oji` bodies (gaps ③+④) render now. Lost bytes: only the
3 `.oja` frame blobs in `Playing1.opi` (2,891,152 B total); none in
`Interface1.opi`, none in `avatar.opa`.

## Lost bodies and the scene elements they belong to

Cross-reference = quoted sprite token in the archive's `ControlList`
file (`ControlList_Interface.txt`, `ControlList_Playing.txt`),
including the runtime `_` stem rule (`resolve.rs`). Bodies with no
`ControlList` reference are drawn by scene code.

### `Interface1.opi` — formerly 5 lost bodies, all render now

The 47 `ATI*/ati*_{id}.ojs` JPEG album-art panels (music-select album
rows, fetched by id convention; no `ControlList` reference) decode
now — see "JPEG album-art bodies" below.

- `Lanking_NoticeBoard.oji` — rank notice board in five states:
  `IDC_IMAGE_SSHOP_NOTICEBOARD`, `IDC_IMAGE_SMYROOM_NOTICEBOARD`,
  `IDC_IMAGE_SWAIT_NOTICEBOARD`, `IDC_IMAGE_MSHOP_NOTICEBOARD`,
  `IDC_IMAGE_LANKING_NOTICEBOARD` (renders since gap ③).
- `Myroom_MybagSelect.oji` / `Shop_MybagSelect.oji` — my-bag buttons
  (`IDC_IMAGE_SMYROOM_MYBAGSELECT`, `IDC_IMAGE_SSHOP_MYBAGSELECT`;
  repeated-y edge spans, render since gap ④).
- `SelectAlbum_Bar.oji` (15 frames) / `SelectMusic_Bar.oji` (10
  frames) — music-select list bars (code-drawn; render since gap ③).

### `Playing1.opi` — formerly 11 lost bodies; 8 render, 3 `.oja` remain

- `Note_Combo4/5/6.ojt` — combo milestone glow animation, referenced
  by `IDC_IMAGE_SPLAY_NOTECOMBO` through runtime `_Note_Combo.ojt`
  (renders since gap ① wired: archive loader + `PlaySkin` glow).
- `Avatar_Shadow.oji` — six avatar shadow slots
  (`IDC_IMAGE_SPLAY_SHADOW1..6`; renders since gap ③).
- `Playing_ChatWindow.oji` — in-play chat window
  (`IDC_IMAGE_SPLAY_CHATWINDOW`; repeated-y row groups, renders
  since gap ④).
- `BG_Ani.oji` (4-frame background animation),
  `Lanking_NoticeBoard_Playing.oji`, `Playing_Loding_Lankview.oji` —
  code-drawn (no `ControlList` reference; render since gap ③).
- `Note_Click.oja`, `LongNote_Effect.oja`, `Playing_Effect_Jam.oja` —
  tap / hold / jam effect animation envelopes; their frame data is
  the only sprite loss left (gap ⑤). Their raw `.ojt`
  siblings (`_Note_Click.ojt`, `_Playing_Effect_Jam.ojt`,
  `LongNote_Effect.ojt`) do render.

### `avatar.opa` — no losses

All 7131 sprite bodies render. Item art is referenced from
`Itemdata_Korea.dat` shop tables; the archive holds no `ControlList`.

## `.oji` body detail: fully mapped (prefixed row groups)

All 36 `.oji` frames fail `is_raw_rgb555`; the prefixed walk
(`parse_sprite55_prefixed`) maps all of them (10 bodies). Each image
stores a run of row groups of `[u16 width][u16 x][u16 y]` +
`width` RGB555 pixels plus one zero terminator word; `y` walks rows
in order — the first group sits on row 0, a group repeats the previous
row (x runs accumulate along it) or steps one row down, so every row
carries at least one span — every span stays inside its row
(`x + width <= w`), and the decode pastes spans onto a
colour-key-cleared canvas: uncovered pixels are transparent, never
invented. Shapes by stored delta (`size − w·h·2` per image):

- **Full-row spans** (positive delta, 28 images; each row costs its
  6-byte header): `SelectAlbum_Bar` +92 ×15, `SelectMusic_Bar` +140
  ×10, `Lanking_NoticeBoard` +188, `Lanking_NoticeBoard_Playing`
  +138, `Playing_Loding_Lankview` +902 (its bottom rows trim both
  sides for rounded corners).
- **Narrow spans** (negative delta, 5 images; margins skipped
  outright): `BG_Ani` −29,022 ×4 and `Avatar_Shadow` −244 — the same
  row-group layout, previously guessed "compressed"; they render
  through the same route.
- **Repeated-y spans** (negative delta, 3 images; only frame
  outlines stored, ~10% of raw pixels): `Playing_ChatWindow`
  −592,540 (671 groups over its 558 rows — 96 rows carry two x runs,
  one border row packs 18), `Myroom_MybagSelect`/`Shop_MybagSelect`
  −9,854 ×2 (134 groups over 69 rows — 65 rows carry the two edge
  spans, four full-width fill rows; both bodies open on a full row of
  one stored colour, `0x00DD`). Same walk, no new codec: the earlier
  "RLE-like compressed frames" guess is retired — the row groups just
  repeat the `y` word. The strict order walk (in-order rows, each
  carried at least once, ending on row `h − 1`) accepts all 36
  images; skipped, dropped, or reordered rows still reject.

## JPEG album-art bodies: decoded (codec gap ②)

All 47 bodies in `Interface1.opi` (`ATI*`/`ati*_{id}.ojs`) are
complete `FF D8 … FF D9` JPEG streams, every one 800x600 with 3
components. Marker-walk classification pins the scan classes: 21
baseline (SOF0) and 26 progressive (SOF2) — a baseline-only decoder
would have left 26 of 47 panels blank, so decode covers progressive
too. The exe consumed them through its IJG build (`CDXImageJPG`,
RTTI file `0x27d5d8`, IJG region VA `0x75f9f0`; docs/claims.md A37);
decode is re-implemented on the pure-Rust `zune-jpeg` crate (the
`image` >= 0.25 default), forced to RGB888 output for any input
colorspace, and `o2jam_render::Rgb888Image`/`blit_rgb888` paint the
XRGB8888 `Frame` 1:1 (800x600 covers the native canvas exactly).
Tests (`crates/o2jam-file/src/jpeg.rs`): synthetic baseline / 4:4:4 /
4:2:0 / progressive fixtures built from raw segment bytes (all-zero
DCT blocks decode to an exact mid-gray 128), corrupt-table and
non-JPEG rejections, marker-walk classification, and the real-body
soak — 47/47 decode to 800x600x3 with the 21 + 26 split, and
`ATI_1001.ojs` paints a corner pixel within ±2 of the (200, 200,
200) pinned by an independent decoder (PIL 13.0). No SOF class
outside {0xC0, 0xC2} occurs in the 47; classes the crate rejects
(arithmetic coding, lossless) stay unsupported because none ships.

## Recommended codec implementation order

1. ~~**Strided atlas playback** (`Note_Combo4/5/6.ojt`)~~ — **DONE**:
   the shell routes strided bodies now. `o2jam-shell/src/avatar.rs`
   (`strided_sprite55` stride search +
   `AvatarFigure::from_sprite55_strided`) loads them as scene figures
   through the archive loader (the `_Note_Combo.ojt` runtime ref
   resolves `Note_Combo<stage>.ojt`), and `playing.rs` stages the glow
   as the `PlaySkin` combo-glow animation
   (`COMBO_GLOW_SPRITE` / `from_archive_for_stage`), painted behind
   the combo digits. Frames come from `parse_sprite55_strided` only.
2. ~~**JPEG album art** (47 bodies)~~ — **DONE**: `o2jam-file`
   `decode_jpeg_body` decodes every body to RGB888 (21 baseline SOF0
   + 26 progressive SOF2, all 800x600x3) on the pure-Rust
   `zune-jpeg` crate (default features off, so the crate compiles
   under its own `forbid(unsafe_code)` scalar path), and
   `o2jam-render` `Rgb888Image` + `blit_rgb888` paint the `Frame`.
3. ~~**`.oji` prefixed frames** (28 images)~~ — **DONE**: the shell
   routes them now. `o2jam-file/src/sprites.rs`
   (`parse_sprite55_prefixed`) walks each image as `h` row groups —
   `[u16 width][u16 x][u16 y]` + `width` RGB555 pixels plus one zero
   terminator word, `y` positional, spans inside their row — and
   `o2jam-shell/src/avatar.rs` (`AvatarFigure::from_sprite55_prefixed`)
   pastes the spans onto a colour-key-cleared `w×h` canvas at the
   record anchor through the archive loader (`sprite55_figure`). The
   strict walk also took the two negative-delta bodies (see the
   detail section), so 33 of the 36 `.oji` frames render.
4. ~~**`.oji` repeated-y frames** (`Playing_ChatWindow`, both
   MybagSelect)~~ — **DONE**: `parse_sprite55_prefixed` accepts
   repeated y words (rows in order, x runs accumulate along a row,
   every row carried at least once, walk ends on row `h − 1`), so
   all 36 `.oji` frames decode; the shell already routes them
   unchanged (`from_sprite55_prefixed` pastes every span onto the
   cleared canvas). `sprites.rs` pins the real bodies with span
   counts and per-row run histograms. The shell tests flipped with
   the decode: `oji_repeated_y_bodies_render_from_the_archives`
   renders the three bodies, and
   `oji_prefixed_bodies_render_from_the_archives` sweeps all ten.
5. **`.oja` frame data + `a`/`b`/`c` words** (3 bodies) — biggest RE
   unknown; envelopes parse today (`parse_oja`), frames do not.

## Method and version pin

Read-only Python 3.14.9 walk of the trailing entry tables (16-byte
header, 152-byte slots; bodies sliced by `size`, never
`reserved_size`). Classifiers mirror `sprites.rs`/`opi.rs`: plain
sprite-55 exact accounting (chained offsets, zero trailing word,
`8 + 20·count + blob == len`), strided = integral
`frames_per_image > 1`, JPEG `FF D8 … FF D9`, `.oja` magic `0x555A`.
Slot and file totals match `docs/file-formats/opi.md`
(1596/633/7149 slots, 1587/627/7133 files, 31 free slots); the 9221
raw bodies match the "~9.2k verified" claim in `sprites.rs`.

Client 7.55, `install-root/O2Jam/Image/`:

- `Interface1.opi` 75,100,702 B,
  sha256 `d66541198965d622ff0177b306d916885ae4d64ae07b2c2264fcbdd850d50584`
- `Playing1.opi` 221,737,026 B,
  sha256 `8c1c7566dc857f2bf0f5f58a5d581e2b2576f4f8a83fa551b31666aed9eed408`
- `avatar.opa` 144,761,749 B,
  sha256 `ced3ed1e82aaf7136003a7dd7ff94ee1395b95d38035589275c2959534b29c16`

# Sprite codec coverage — real 7.55 archives

Result of a read-only walk of every named body in the three sprite
archives, classifying each by codec and cross-referencing the lost
ones against `ControlList` sprite references. No source was changed;
archives stayed read-only; this document is the only file created in
the repo. Numbers come from the real bytes, not estimates.

## Answer: counts per archive per codec

| Archive | Named | Sprite bodies | raw-RGB555 (render) | .oji raster (prefixed render) | strided atlas (render) | .oja anim (render) | JPEG (render) | data/`.bnd`/`.txt` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `Interface1.opi` | 1587 | 1532 | 1480 | 5 | 0 | 0 | 47 | 55 |
| `Playing1.opi` | 627 | 621 | 610 | 5 | 3 | 3 | 0 | 6 |
| `avatar.opa` | 7133 | 7131 | 7131 | 0 | 0 | 0 | 0 | 2 |
| **Total** | **9347** | **9284** | **9221 (99.3%)** | **10** | **3** | **3** | **47** | **63** |

Body-level render coverage: `Interface1.opi` 100% (1532/1532),
`Playing1.opi` 100% (621/621 — the 3 `.oja` effect envelopes decode
since gap ⑤), `avatar.opa` 100% — 9284/9284 bodies (100%).
Image-record level: all 63,796 records decode — 63,760 raw (99.94%)
plus the 36 non-raw records inside the 10 `.oji` bodies (prefixed
walk, gaps ③+④) and the 22 plane frames inside the 3 `.oja` bodies
(16 tabulated + 6 recordless trailing, gap ⑤). The "compressed `.ojs`/`.ojt` frame" class is empty: no
plain sprite body carries a single non-raw image, so every formerly
lost frame sat in an `.oji` or a `.oja` envelope — the strided
atlases (gap ①), the 47 JPEG album-art bodies (6.8 MB, gap ②), all
10 `.oji` bodies (gaps ③+④) and all 3 `.oja` bodies (gap ⑤) render
now. Lost bytes: none in any archive — `Interface1.opi`,
`avatar.opa`, and the formerly lost 2,891,152 B of `.oja` blobs in
`Playing1.opi` all decode (see the `.oja` section below).

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

### `Playing1.opi` — all 11 formerly lost bodies render

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
  tap / long-note / jam effect animations; decode through the
  `.oja` frame-plane reader since gap ⑤ (see the section below).
  `Note_Click.oja` and `LongNote_Effect.oja` are byte-identical
  (87,875 B each); their raw `.ojt` siblings (`_Note_Click.ojt`,
  `_Playing_Effect_Jam.ojt`, `LongNote_Effect.ojt`) keep rendering
  alongside.

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

## `.oja` bodies: fully mapped (gap ⑤, frame planes)

All 22 frames across the three `.oja` bodies decode
(`parse_oja55` in `crates/o2jam-file/src/sprites.rs`, routed in
`o2jam-shell/src/avatar.rs` via `AvatarFigure::from_oja55`): the
ten tabulated Jam frames, the six click/long frames, and the six
recordless Jam trailing frames (below). The reference toolkit
throws on these bodies; the exe reads per frame a pixel plane then
a byte plane with no codec between (claims C7) — the actual stored
bytes are compacted, not fixed-size:

- Each 20-byte frame record carries `x y w h` plus the three raw
  words, now read as: `a` = byte offset of the frame's data from the
  blob start (the planes chain exactly, `a[n+1] == a[n] + b[n] +
  c[n]` on every container), `b` = stored byte length of the frame's
  row-group pixel plane, `c` = stored byte length of the alpha plane
  that follows it.
- Each pixel plane is the same prefixed row-group walk as the `.oji`
  bodies — `[u16 width][u16 x][u16 y]` + `width` RGB555 words, one
  zero terminator — except rows move forward only and may start and
  end above rows `0`/`h-1` (the ring circles and flash fan only
  cover the rows they reach; the jam flash's first frame paints
  rows 24..102 of 300). Every walk's terminator lands exactly on
  `a + b`; the walk plus the exact chain is the layout proof.
- The alpha plane (`c` bytes) tracks the covered-pixel count: exact
  on all ten `Playing_Effect_Jam.oja` frames (`c == Σ span widths`),
  up to 3 bytes short on the click pair (1/2/1/3/2/0 per frame —
  the packer's alpha pass dropped edge pixels; pinned in tests,
  not assumed away). Its values rise with pixel luminance (dim ring
  edge ≈ 0–9, bright core ≈ 255): an intensity map. The exe uploads
  it beside the pixels; the exact blend role stays open (claims C7
  OPEN), and the shell's figures paste the pixel planes opaque for
  now — the alpha rides `Oja55Frame::alpha` for a blended paint.
- The planes chain straight after the record table on all three
  real bodies (smallest plane-base shift 0; `a[n + 1] == a[n] +
  b[n] + c[n]` on every container). An earlier revision of this
  note claimed an 80-byte record-copy block before the Jam's first
  plane — that was a misread: the bytes at the blob start are
  frame 0's first row groups (spans `(154, 24, 1)`, `(143, 25, 3)`,
  …), and frame 0's walk lands exactly at `a + b` from shift 0.
- The Jam blob's 1,623,039 untabulated trailing bytes decode as six
  recordless frames (`Oja55::trailing_frames`): no records exist,
  so the parser scans the block for bottom-anchored plane walks —
  a candidate is a plane only when its row-group walk lands on a
  terminator at the canvas's bottom row (all six real walks do;
  mask bytes only ever produce walks that stop high). Geometry
  stays the tabulated canvas (300x300): every span sits inside it,
  and each real frame spans it fully (`max(x + width) == 300`,
  `max(y) == 299`), so no anchor or size is invented. Frame 10 is
  a rows-25..299 partial top (281 spans, 166,676 B plane), frames
  11..15 are full canvases (300 spans, 181,802 B each); each plane
  is followed by a mask slice running to the next plane (the last
  to the blob end): 89,694 B then 89,700 B ×5 — 299 rows of 300
  alpha bytes per mask (canvas-row maps, zero past the glow rim,
  values rising with pixel luminance), not the tabulated records'
  byte-per-covered-pixel alpha; the first mask is six bytes short
  of the 299x300 grid. Content check: each mask's nonzero rows
  track its paired plane's pixel rows (plane 10 stores rows 25..181
  and mask 0 glows rows 0..182; plane 11 stores rows 0..233 and
  mask 1 glows rows 0..234 — the mask rim extends a row or two
  past the stored pixels). The shell routes them as
  layers 10..15 at the body-wide `(0, 0)` anchor every tabulated
  Jam record carries; gameplay consumers that stage the record
  table only (the Playing skin) keep the ten exe-visible frames.
- Honest negatives: the trailing block opens with a 9,159-byte
  alpha-profile orphan (`Oja55::trailing`) — 30 full 300-byte rows
  plus one partial row 30, value 0 below row 30, but no pixel
  plane anywhere follows or precedes it; its frame is missing from
  the blob and stays raw. The accounting is exact: 9,159 +
  (166,676 + 89,694) + 5 × (181,802 + 89,700) = 1,623,039, zero
  leftover. The trailing strip keeps the exe-reading question (C7)
  open: the exe's fixed `w*h*2 + w*h` per-frame read cannot
  consume the compacted planes, matching claims C9's "the exe
  cannot decode these either".

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
5. ~~**`.oja` frame data + `a`/`b`/`c` words** (3 bodies)~~ —
   **DONE**: `o2jam-file` `parse_oja55` decodes every frame
   (row-group pixel plane landing on its zero terminator exactly at
   `a + b`, alpha plane sliced at `c`; plane-base scan over any
   leading block — shift 0 on all three real bodies), and
   `o2jam-shell/src/avatar.rs` routes them
   (`AvatarFigure::from_oja55`, one layer per frame at its record
   anchor). The Jam trailing block decodes too: six recordless
   frames plus a 9,159-B orphan mask block — see the `.oja`
   section above.

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

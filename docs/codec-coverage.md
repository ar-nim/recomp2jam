# Sprite codec coverage — real 7.55 archives

Result of a read-only walk of every named body in the three sprite
archives, classifying each by codec and cross-referencing the lost
ones against `ControlList` sprite references. No source was changed;
archives stayed read-only; this document is the only file created in
the repo. Numbers come from the real bytes, not estimates.

## Answer: counts per archive per codec

| Archive | Named | Sprite bodies | raw-RGB555 (render) | .oji raster (skip) | strided atlas (skip) | .oja anim (skip) | JPEG (render) | data/`.bnd`/`.txt` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `Interface1.opi` | 1587 | 1532 | 1480 | 5 | 0 | 0 | 47 | 55 |
| `Playing1.opi` | 627 | 621 | 610 | 5 | 3 | 3 | 0 | 6 |
| `avatar.opa` | 7133 | 7131 | 7131 | 0 | 0 | 0 | 0 | 2 |
| **Total** | **9347** | **9284** | **9221 (99.3%)** | **10** | **3** | **3** | **47** | **63** |

Body-level render coverage: `Interface1.opi` 99.7% (1527/1532: the
47 JPEG album-art bodies decode since codec gap ②), `Playing1.opi`
98.2%, `avatar.opa` 100%. Image-record level: 63,796 records total,
63,760 raw (99.94%); all 36 non-raw records live inside the 10 `.oji`
bodies. The "compressed `.ojs`/`.ojt` frame" class is empty: no plain
sprite body carries a single non-raw image, so every lost frame sits
in an `.oji` or a `.oja` envelope — the strided atlases (gap ①) and
the 47 JPEG album-art bodies (6.8 MB, gap ②) render now. Lost bytes:
4.2 MB in `Interface1.opi`, 4.1 MB in `Playing1.opi`, none in
`avatar.opa`.

## Lost bodies and the scene elements they belong to

Cross-reference = quoted sprite token in the archive's `ControlList`
file (`ControlList_Interface.txt`, `ControlList_Playing.txt`),
including the runtime `_` stem rule (`resolve.rs`). Bodies with no
`ControlList` reference are drawn by scene code.

### `Interface1.opi` — 5 lost bodies

The 47 `ATI*/ati*_{id}.ojs` JPEG album-art panels (music-select album
rows, fetched by id convention; no `ControlList` reference) decode
now — see "JPEG album-art bodies" below.

- `Lanking_NoticeBoard.oji` — rank notice board in five states:
  `IDC_IMAGE_SSHOP_NOTICEBOARD`, `IDC_IMAGE_SMYROOM_NOTICEBOARD`,
  `IDC_IMAGE_SWAIT_NOTICEBOARD`, `IDC_IMAGE_MSHOP_NOTICEBOARD`,
  `IDC_IMAGE_LANKING_NOTICEBOARD`.
- `Myroom_MybagSelect.oji` / `Shop_MybagSelect.oji` — my-bag buttons
  (`IDC_IMAGE_SMYROOM_MYBAGSELECT`, `IDC_IMAGE_SSHOP_MYBAGSELECT`).
- `SelectAlbum_Bar.oji` (15 frames) / `SelectMusic_Bar.oji` (10
  frames) — music-select list bars (code-drawn).

### `Playing1.opi` — 11 lost bodies (the in-play scene)

- `Note_Combo4/5/6.ojt` — combo milestone glow animation, referenced
  by `IDC_IMAGE_SPLAY_NOTECOMBO` through runtime `_Note_Combo.ojt`
  (renders since gap ① wired: archive loader + `PlaySkin` glow).
- `Avatar_Shadow.oji` — six avatar shadow slots
  (`IDC_IMAGE_SPLAY_SHADOW1..6`).
- `Playing_ChatWindow.oji` — in-play chat window
  (`IDC_IMAGE_SPLAY_CHATWINDOW`).
- `BG_Ani.oji` (4-frame background animation),
  `Lanking_NoticeBoard_Playing.oji`, `Playing_Loding_Lankview.oji` —
  code-drawn (no `ControlList` reference).
- `Note_Click.oja`, `LongNote_Effect.oja`, `Playing_Effect_Jam.oja` —
  tap / hold / jam effect animation envelopes; their raw `.ojt`
  siblings (`_Note_Click.ojt`, `_Playing_Effect_Jam.ojt`,
  `LongNote_Effect.ojt`) do render.

### `avatar.opa` — no losses

All 7131 sprite bodies render. Item art is referenced from
`Itemdata_Korea.dat` shop tables; the archive holds no `ControlList`.

## `.oji` body detail: one codec mapped, one remaining

All 36 `.oji` frames fail `is_raw_rgb555`. The prefixed walk
(`parse_sprite55_prefixed`) now maps 33 of them (7 bodies): each
image stores `h` row groups of `[u16 width][u16 x][u16 y]` +
`width` RGB555 pixels plus one zero terminator word; `y` counts
rows positionally `0..h-1` and every span stays inside its row
(`x + width <= w`), so the decode pastes spans onto a
colour-key-cleared canvas — uncovered pixels are transparent, never
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
- **Still un-mapped** (3 images): the walk fails mid-image with
  repeating y words — `Playing_ChatWindow` −592,540 (stores 11% of
  raw), `Myroom_MybagSelect`/`Shop_MybagSelect` −9,854 ×2. A real
  codec (RLE-like) to reverse.

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
4. **`.oji` compressed frames** (3 images left) — the row-group walk
   cleared `BG_Ani` and `Avatar_Shadow`; `Playing_ChatWindow` and the
   two MybagSelect bodies still need codec RE.
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

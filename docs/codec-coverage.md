# Sprite codec coverage — real 7.55 archives

Result of a read-only walk of every named body in the three sprite
archives, classifying each by codec and cross-referencing the lost
ones against `ControlList` sprite references. No source was changed;
archives stayed read-only; this document is the only file created in
the repo. Numbers come from the real bytes, not estimates.

## Answer: counts per archive per codec

| Archive | Named | Sprite bodies | raw-RGB555 (render) | .oji raster (skip) | strided atlas (skip) | .oja anim (skip) | JPEG (skip) | data/`.bnd`/`.txt` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `Interface1.opi` | 1587 | 1532 | 1480 | 5 | 0 | 0 | 47 | 55 |
| `Playing1.opi` | 627 | 621 | 610 | 5 | 3 | 3 | 0 | 6 |
| `avatar.opa` | 7133 | 7131 | 7131 | 0 | 0 | 0 | 0 | 2 |
| **Total** | **9347** | **9284** | **9221 (99.3%)** | **10** | **3** | **3** | **47** | **63** |

Body-level render coverage: `Interface1.opi` 96.6%, `Playing1.opi`
98.2%, `avatar.opa` 100%. Image-record level: 63,796 records total,
63,760 raw (99.94%); all 36 non-raw records live inside the 10 `.oji`
bodies. The "compressed `.ojs`/`.ojt` frame" class is empty: no plain
sprite body carries a single non-raw image, so every lost frame sits
in an `.oji`, plus the three strided atlases, three `.oja` envelopes,
and 47 JPEG album-art bodies (6.8 MB). Lost bytes: 4.2 MB in
`Interface1.opi`, 4.1 MB in `Playing1.opi`, none in `avatar.opa`.

## Lost bodies and the scene elements they belong to

Cross-reference = quoted sprite token in the archive's `ControlList`
file (`ControlList_Interface.txt`, `ControlList_Playing.txt`),
including the runtime `_` stem rule (`resolve.rs`). Bodies with no
`ControlList` reference are drawn by scene code.

### `Interface1.opi` — 52 lost bodies

- 47 `ATI*/ati*_{id}.ojs` JPEG album-art panels (music-select album
  rows, fetched by id convention; no `ControlList` reference).
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

## `.oji` body detail: two un-mapped sub-codecs

All 36 `.oji` frames fail `is_raw_rgb555`, in two shapes
(`size − w·h·2` per image):

- **Prefixed** (constant positive delta, 28 images):
  `SelectAlbum_Bar` +92 ×15, `SelectMusic_Bar` +140 ×10,
  `Lanking_NoticeBoard` +188, `Lanking_NoticeBoard_Playing` +138,
  `Playing_Loding_Lankview` +902. Likely a small per-image header
  over raw rows; one dump decides.
- **Compressed** (negative delta, 8 images): `Playing_ChatWindow`
  −592,540 (stores 11% of raw), `BG_Ani` −29,022 ×4, `MybagSelect`
  −9,854 ×2, `Avatar_Shadow` −244. A real codec (RLE-like) to reverse.

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
2. **JPEG album art** (47 bodies) — largest single lost class;
   standard decoder, no format work beyond dimensions.
3. **`.oji` prefixed frames** (28 images) — near-trivial: strip the
   constant prefix, decode rows as RGB555.
4. **`.oji` compressed frames** (8 images) — needs codec RE; visible
   in chat window and background animation.
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

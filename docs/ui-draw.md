# UI control draw path (7.55) — chain VERIFIED to the page boundary

How a control's sprite reaches the display in `AngelO2Jam.exe` 5,029,888 B,
SHA-256 `83a04541…72c823b` (twin `OTwo.exe`). Method: REA 6.7.0 with its
Ghidra 12.1.2 provider on the read-only installed exe (session evidence
`ev_cebc4dc7…`); RTTI structures decoded from read-only byte walks (pool
`0x664e00`–`0x666700`, TD cluster `0x67d100`+, e.g. `ev_a4d1e047…`);
Python 3.14 for byte decoding. File offset = VA − `0x400000`. No
disassembly or decompiled code is reproduced.

## Answer: the blit call chain

1. `CCImageBox::Draw` `0x50a890` (file `0x10a890`, dossier
   `ev_cc0942ba…`) overrides base `CControlBase::Draw` `0x50da40`
   (vftables: CCImageBox `0x64bf54` slot 4, CControlBase `0x64c12c`
   slot 4). Base draw stores flags at `+0x10` and lazily creates one
   `0x170`-byte view object at control `+0x58` (`0x42ee00` ctor
   neighborhood).
2. The control matches its sprite stem against `"%s.ojs"` (`0x64b124`)
   or `"oji"` (`0x64bf80`) and stages the image on a view slot:
   `.ojs` via `0x42fc70` with layer 1, `.oji` via `0x42fc10` with
   layer 2; the staged position is `(0, 0)` on the normal path.
3. Staging resolves the layer through the image's vtable slot 7
   (`0x424100`, `ev_015a885f…`): layer → `u16 +0x148`, and a callback
   pointer at `+0x164`: layer 1 installs thunk `0x422680`
   (`jmp [vtbl+0x10]` → `CDXSurface::Draw` `0x42b4d0`), layers 2/3/5
   install `0x422690` (`jmp [vtbl+0x14]` → `DrawClip` `0x42b5e0`).
   `CDXSprite` mirrors this with its own SetLayer `0x420d40`
   (vftable `0x6444d8`, `.\Display\Sprite.cpp`).
4. Frame advance `0x42f040` (`ev_4e3689c1…`) walks animation modes
   3–9 (setter `0x42f5a0`): current frame `+0x164`, frame count
   `+0x148`, rest frame `+0x168`, loop counters `+0x14c/+0x150`, then
   calls `0x42fff0` → sets the image frame (`0x41e0f0`, clamped to
   count `+0x11c`) and draws at the view position.
5. The positioned draw `0x41df80` (`ev_7418d033…`) is the
   **anchor math**: destination = view position **plus** the sprite's
   per-frame anchor — two signed 16-bit words at frame-record offsets
   `+0`/`+2`, record stride `0x14` (matches the OPI sprite-55 record),
   anchor array at object `+0x48`, current frame `+0x4d`. It then
   invokes the stored callback (`+0x164`) with (screen, x, y, z).
6. `CDXSurface::Draw` `0x42b4d0` (`ev_867ac248…`) picks the frame's
   surface/width/height arrays (`+0x79`, `+0x4b`, `+0x4c` at frame
   `+0x4d`) and calls helper `0x42b450` with flag `0x10`;
   `DrawClip` `0x42b5e0` (`ev_85764b28…`) adds manual clipping and
   calls it with flag `0x11`.
7. Helper `0x42b450` (`ev_08a33fe4…`) fetches page
   `pages[+0x134]` from array `+0x1e4` and calls the page object's
   vtable slot 7 (`+0x1c`) with (x, y, src, rect, flags). Status
   `0x887700c2` (DDERR_SURFACELOST) triggers restore `0x42bbb0`
   (restore + reload via `0x42a580`/`0x42a8d0`).
8. Page upload (`0x42a580`): create page surface, `SetRect`, lock
   (`0x42bc40`), upload pixels to a texture at `+0x18c` with stride
   `>>1` (`0x42a970`), unlock (`0x42bda0`); 565 formats upload a
   second array. The D3D variant `0x423390` sets `DDSCAPS_TEXTURE`
   (`0x1000`), rounds sizes up to powers of two, and asserts through
   `.\Display\DXTexture.cpp` `m_pd3dDevice->GetDDS…` strings at
   `0x6447f8`.

The page objects are created through the display wrapper built in
`.\Display\Display.cpp` (`FUN_00413030`): `DirectDrawCreateEx` with
IID_IDirectDraw7 (`0x651668`) lands the raw COM pointer at `+8`
(correction: `docs/systems-survey.md` said this import had no direct
text reference — it does, here); fullscreen 800×600 comes from
`FUN_00431b70(..., 800, 600)`. Surfaces are created as
IDirectDrawSurface7 (IID at `0x651848`). The page's slot-7 blit body
is the one member of the chain still unnamed (see OPEN).

## Anchor and `.bnd` rect → destination pixels

The draw path never reads the `.bnd` rect. A `CCImageBox` stages
position `(0, 0)`, so the destination top-left is the sprite's own
per-frame anchor alone. The rect↔anchor byte equality already
verified in `docs/bnd-semantics.md` is authored data, not a runtime
transform: the applier draws at the anchor; the rect's only proven
role remains load-time pairing. Whether any control class stages a
non-zero view position (`+0x18/+0x1c`) from its rect is unresolved.

## Colorkey

The sprite-55 colorkey (`0x0000`/`0x7D5F`) is not compared on the CPU
in the draw path: byte sweeps for `cmp`-style `0x7d5f` immediates and
for DDRAW `BltFast`-shaped indirect calls (`call [reg+0x50/0x54]`)
return nothing; the loader `0x4249f0` validates the `0x555`/`0x555a`
pixel format and allocates frame arrays, and the transparency then
rides the device-side path inside the display wrapper. Which device
call consumes the key is OPEN. Flag `0x10` vs `0x11` selects the
clipped variant, not colorkeying.

## Clipping

Two variants exist per image (layer callbacks, step 3). `DrawClip`
`0x42b5e0` clamps source and destination against the stored view
bounds `+0x4e`–`+0x51` and skips the blit when width or height drops
below 1 — pixel-exact viewport clipping, not DirectDraw clipper
lists. `Draw` `0x42b4d0` performs no clipping.

## Draw order across controls

The control-list render loop `0x5dec30` (`ev_ff581cfa…`) iterates the
control array `0..count` in stored order and builds D3D quads with
per-entry-code handler dispatch (code tables `0x68f3a0`–`0x68f84c`,
lists at state `+0x320`/`+0x520`; teardown shape `0x5d30e0`). Stored
order is file order (recipe step 5), so painting is file order with
the full-screen entry 1 first. Layering adds one nuance: `.ojs`
sprites ride layer 1, `.oji` sprites layer 2 through the clipped
callback, so overlay-class art composites above the sprite layer
regardless of file position.

## Per-hit effect entries (playing scene burst art)

The `0x5dec30` render loop's playing-scene entries come from
`ControlList_Playing.txt` `STATE_PLAYING` (read-only archive body,
read with the verified control-text decoder). Two per-hit families
ride the entry array:

- `IDC_IMAGE_SPLAY_KEYEFFECT1..7` (codes `0x1B500074`–`0x1B50007A`,
  rect ordinals 116–122) name `Keyeffect1..7.ojs`, Korean comment
  "7개의 긴 바" (seven long bars): the staged lane beams, one body per
  lane, five raw frames each (rising fill from the bottom quarter),
  full 480-row columns 28/22/28/32/28/22/28 wide anchored at the lane
  x (5/33/55/83/115/143/165, 0) — the lane-geometry pins used by
  `crates/o2jam-shell/src/playing.rs` `NATIVE_LANES`.
- `IDC_IMAGE_SPLAY_NOTECLICK1..7` (codes `0x1B500093`–`0x1B500099`,
  rect ordinals 147–153) name `_Note_Click.ojt`, comment "폭발이미지"
  (explosion image): the per-hit burst. The playing applier resolves
  the stem through `%s%d_%d.ojt` (`0x64b154`, site `0x4e814e`,
  `docs/stage-bga.md`) to `Note_Click<stage>_<variant>.ojt`;
  `Playing1.opi` stages 36 stages × 2 variants, each a square
  multi-frame explosion (e.g. `Note_Click1_1.ojt`: eight raw 128x128
  frames, `Note_Click7_2.ojt`: ten 256x256 frames; all anchors `(0,0)`,
  hundreds of distinct RGB555 colors per frame — the colorful star
  flash). All seven entries share one body per song.

Version pin: `AngelO2Jam.exe` 5,029,888 B, SHA-256 `83a04541…72c823b`
(the header hash). Method: read-only byte walks with the same tools as
the header (Python 3.14 byte survey). Negative findings:

- The exe carries no literal `"Keyeffect"`/`"KeydownImage"` strings;
  control-list names resolve through the `%s.ojs` stem (file
  `0x24b0ec`). `Keyeffect.ojs` (slot 34, one static 7-lane beam body)
  is absent from the control list — the exe never names it, so the
  shell's `KeyEffect` release-beam binding stays a stand-in.
- The playing scene's BG-skin kit beams are real and this scene's
  staging (earlier "another scene" reading retracted): the format
  strings `KeyEffect_7Key%d_%d.ojs`, `KeyEffect_Black_*_%d.ojs`,
  `KeyEffect_{3,7}Key_{Left,Right}%d_%d.ojs` (file `0x24abdc`–
  `0x24ad00`) sit inside the playing staging cluster — neighbors of
  `MUSIC\%sojn` (`0x24abd0`), `Playing_NormalBG%d.ojs` (`0x24ac44`),
  `Playing_P2PBG%d.ojs` (`0x24acec`), `Note_Click` (`0x24b160`),
  `Jamgauge_right_bar.ojs` (`0x24b1b8`), `LongNote_Effect.ojt`
  (`0x24b280`), `Normal_TimeBar.ojs` (`0x24b424`), `Playing_Exit.ojs`
  (`0x24b450`) — and are referenced from the kit stage functions
  `0x4dc750` (file `0xdc750`) and `0x4dcab0`, with
  `KeyEffect_Tutorial_%d.ojs` from `0x4dd290` (Tutorial, file
  `0xdd3e6`). They are 5-frame full-column stems, not the explosion;
  the bright press/held lane beams trace here (see the beam color
  source section below).
- The burst draw point is unbound: rect ordinals 147–153 sit past the
  17 `State_Playing.bnd` rects (same shape as the long-combo plate),
  so the exe places the explosion at the struck receptor at runtime
  (photo: "key flashes on the line", `docs/playing-reference.md`).

Binding: `playing.rs` stages `Note_Click1_1.ojt` (`HIT_BURST_SPRITE`,
deterministic stage 1 variant 1; the exe variant is `rand()%2+1`) and
paints one frame per `RECEPTOR_FRAME_MS` of flash age, centered on the
receptor pad, over the beam column — the exe's file order puts the
`NOTECLICK` entries after `KEYDOWN`. The procedural white column
(`HIT_BURST`) stays the no-body fallback.

## Held lane beam color source — body pixels, not the blit (VERIFIED)

The reference photo's bright cyan beam band traces to sprite-body
pixels, not the draw chain: steps 6–8 pass only `(x, y, src, rect,
flags)` — no color word, no per-lane tint — from `Draw`/`DrawClip`
through helper `0x42b450` to the page blit, so per-lane tinting and a
tinted beam surface are both excluded by the verified chain. Which
body stages decides the color:

- Kit bodies: `Playing1.opi` `KeyEffect_7Key1_1..7.ojs` (kit 1 pairs
  `Playing_NormalBG1.ojs`; slots at archive offsets 208693199,
  208832507, 208943015, 209082323, 209255231, 209394539, 209505047;
  read-only byte walk, same 7.55 version pin) — five raw RGB555
  frames each, full 480-row columns 29/23/29/36/29/23/29 wide,
  anchors (24/54/78/108/145/175/199, 0), colorkey `0x0000`. Lane
  inks: bright red family, brightest (255,33,41) on lanes 1/3/5/7;
  sky-blue (132,181,255) on lanes 2/6 — the photo's bright cyan/blue
  band; amber (255,214,115) on lane 4. Kit-1 frames are 94–100%
  inked, and every column's brightest ink concentrates at rows
  470..479 — the glow band over the judgement line. Kits 2 and 3 and
  the Black kit share the identical palette, so the beam color is
  lane-fixed, not kit-fixed.
- The control-list `IDC_IMAGE_SPLAY_KEYEFFECT1..7` bodies
  (`Keyeffect1..7.ojs`, archive offsets 183347707..184116355) keep
  the five-frame rising-fill shape but dark lane inks — reds
  8..74,0,0, blues up to 16,33,82, amber 115,74,0, with a ≤1.3% bright
  accent ((214,0,8), (33,115,231)) on frame 0 — the dim "seven long
  bars" underlay; they cannot produce the photo's bright column.
  `Keyeffect.ojs` (slot 34, archive offset 6137344) stays
  exe-unnamed. `LongNote_Effect.ojt` (archive offset 40940469) frames
  are white-ink art (brightest 248,248,248), not the beam color.

Shell binding (`playing.rs`): `PlaySkin` stages the kit-1 bodies
(`LANE_BEAM_STEM`, deterministic kit 1, like `Note_Click1_1`) as
`lane_beams`; held lanes paint the steady last frame bottom-anchored
at the line and press flashes step frames per `RECEPTOR_FRAME_MS`
through the same helper as receptors/bursts. The `Keyeffect.ojs`
binding remains only as the untouched-hold slice and the `Keyeffect`
flash fallback; the procedural inks stay the no-body fallback.

OPEN: the kit anchors (x 24..199) sit off the control-list lane pins
(5..165), so the kit draw carries a dialog/skin view offset the chain
does not show (extends OPEN 3); which kit index a default room
selects (assumed 1) and how the exe steps the five frames (assumed
receptor-like) are untraced.

## Verdicts on recipe claims (`docs/opi-pattern.md`)

- Step 5 pair + step 7 anchor==rect: **VERIFIED in code** — dest is
  the anchor; no rect input at blit time (steps above).
- Step 7 "blit each sprite into its rect": **REFINED** — into its
  anchor position, which equals the rect for these archives; no rect
  clamp or offset is applied at draw time.
- Step 8 file-order painting, background first: **VERIFIED** at code
  level (list-order loop) with the layer-2 overlay nuance above.
- Step 7 colorkey runs transparent: **CONSISTENT**, consumption point
  device-side (OPEN).

## Ghidra function dossiers (REA evidence ids)

Blit applier `CDXSurface::Draw` `ev_867ac2486893ba03af990fbfaa4933a0
670f393ceb5eb73ce9a31b9ecd09b2d9`; `DrawClip`
`ev_85764b281e158f88617f337f50c98a363bae0576e7146e5e1c17640d2cec78ac`;
page dispatch helper
`ev_08a33fe43a2c0d6edae1d9b8036575949531b1ff6454ba21d0b28b201c0b992b`;
SetLayer `ev_015a885f2a9d9100f6f1410bc9dff84f03e2045d0d60c948c9d9512d
fd8e45a6`; anchor adder
`ev_7418d033fe0f29f7830726eb274652dc349a2d198e549ef2877f9f1d3a9e64a8`;
frame advance
`ev_4e3689c1051a3f803da07d4b4dbdaae0e752cc1b8d7a54be00105aaaf344b8af`;
control draw `ev_cc0942bae04784fb029e2c16ff6eb4ee17bfa3688453f079db12
f2c991649fec`; list render loop
`ev_ff581cfab5e29b01ea0c4b612e78fb0f467f9c4ca8a252f02afe4b9dbc460c57`.

## OPEN

1. Page-object class and its slot-7 body: pages are created via the
   Display.cpp wrapper (raw IDirectDraw7 at `+8`, factory slot at
   display-vtable `+0x18`) and receive (x, y, src, rect, flags) at
   `+0x1c`; the wrapper class has no RTTI COL, so its vftable is
   unnamed. CDXTexture (`0x6447e8`, one virtual) is the texture
   holder, not the page.
2. Colorkey consumption at the device call (D3DX chroma vs DDRAW
   `SetColorKey`); the `0x7d5f`/`0x0000` values are authoring data.
3. Dialog-relative rects: `CCImageBox` always stages `(0,0)`; nothing
   in this chain translates by a dialog origin, so dialog-frame
   handling must set view positions or draw into dialog-local
   surfaces (CWindow `0x6448bc` / CDialogBase `0x67d7e0` unexamined).
   The BG-skin kit beams' anchors (x 24..199 vs the control-list lane
   pins 5..165) ride this same unresolved offset.
4. `.bnd` rect runtime consumers beyond pairing (hit-testing?) — not
   in the blit path.
5. `CDXSprite`'s role (Sprite.cpp twin of the image classes; clip
   range layers 2–6 differs from the image classes' 2/3/5).

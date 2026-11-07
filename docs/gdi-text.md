# GDI text stack (7.55) — painters, fonts, backing — PINNED

How `AngelO2Jam.exe` 5,029,888 B, SHA-256 `83a04541…72c823b` (twin
`OTwo.exe`) renders text through GDI: which CDXSurface members draw,
which fonts they select, what surface backs the text, and whether text
is prerendered or drawn live. This grounds the text pass of the
recomp renderer (owner decision: modern rasterizer such as ab_glyph +
OFL faces, not GDI parity — the exe still pins positions, sizes,
colors, and effects).

Version pin: VA = file offset + `0x400000`. Method: read-only objdump
and byte walks over the installed exe (offset-only scans, no new
dossier — the standing precedent of `channel-fill.md`); existing
REA evidence ids are cited where a dossier already pins the site. No
disassembly or decompiled code is reproduced; behavior is described in
our own words.

## Answer first

| Field | Verdict |
|---|---|
| Draw API | `TextOutA` (IAT `0x63f074`) and `DrawTextA` (IAT `0x63f468`), always with `SetBkMode(hdc, TRANSPARENT)` |
| Backing | GDI draws straight into a DirectDraw **page surface**: the painter takes the page's DC through a COM vtable call, never the window DC |
| Text rasterization | Live glyph rasterization by GDI at every call — no glyph atlas, no per-string bitmap cache |
| Prerender vs live | Scene numbers: painted **once per fill** into the loaded sprite body surface (persists across frames). Control/dialog text: drawn **live every frame** by `Draw` overrides |
| Faces | `Trebuchet MS` (VA `0x64ae78`), `Arial` (`0x64a320`), Gulimche CP949 `굴림체` (`0x64407c`) — all `CreateFontA` weight 400, charset `DEFAULT_CHARSET` (1), quality `PROOF_QUALITY` (2) |
| Ink | Per-call `SetTextColor`; result screen = white `0xffffff` over a black `0x000000` outline halo (rows 3 × cols 4 block copies) |

## The toolkit (imports)

GDI32: `CreateFontA` `0x63f03c`, `GetTextExtentPoint32A` `0x63f038`,
`GetTextMetricsA` `0x63f04c`, `SelectObject` `0x63f078`,
`SetTextColor` `0x63f070`, `SetBkMode` `0x63f06c`, `SetBkColor`
`0x63f054`, `TextOutA` `0x63f074`, `MoveToEx` `0x63f064`, `LineTo`
`0x63f068`, `DeleteDC` `0x63f044`, `DeleteObject` `0x63f040`,
`GetStockObject` `0x63f050`, `CreateSolidBrush` `0x63f060`,
`Rectangle` `0x63f058`. USER32 adds `DrawTextA` `0x63f468`, `GetDC`
`0x63f398`, `ReleaseDC` `0x63f394`. There is **no**
`CreateCompatibleDC`/`CreateDIBSection`/`BitBlt`: no memory-DC
backing store exists anywhere in the exe.

## Surface-DC backing (the CDXSurface text members)

The painters are members of the same CDX class family as
`CDXSurface::Draw` `0x42b4d0` (`ev_867ac248…`):

- **DC getter `0x42c0a0`**: picks the page
  `pages[this+0x134]` from the page array `this+0x1e4` (the same
  page objects the blit helper `0x42b450` uses, `ev_08a33fe4…`),
  then calls page-vtable slot 17 (`+0x44`) — the page's GetDC —
  caching the HDC at `this+0x1ec` behind a nest counter at
  `this+0x20c`. On a null page/DC the surface restore `0x42bbb0`
  runs (the DDERR_SURFACELOST path pinned in ui-draw step 7).
- **DC release `0x42c160`**: decrements the nest counter; at zero,
  page-vtable slot 26 (`+0x68`) — ReleaseDC — runs and the cached
  HDC is zeroed.
- **Font select `0x42c290`**: `SelectObject(hdc, font->+0x10)`,
  previous font parked at `this+0x21c`; **restore `0x42c2f0`**
  reselects the old font and releases the DC. Every painter follows
  get → `SetBkMode(TRANSPARENT)` → select → `SetTextColor` → draw →
  restore → release.

So text pixels land in the sprite page surface itself, and the page
composites per frame through the normal quad path (`0x5dec30`,
`ev_ff581cfa…`). How a GDI-drawn texture page reaches the device
stays with ui-draw OPEN 1/2 (page class unnamed, device-side colorkey
and texture sync open).

## Font creation, cache, and metrics

- **Cache lookup `0x41c860(name, width, height, weight)`**: name
  copied into a global 260-byte buffer at `0x7681a0`; a global font
  vector at `0x7682b8` (size `0x7682bc`, capacity `0x7682c0`) is
  scanned by face and metrics; a hit returns the cached descriptor, a
  miss creates one. Every state builds fonts through this one door
  (callers include display init `0x413030`, the result builder
  `0x4c2390`, the loading gate `0x4e47a0`, the text-box stager
  `0x50f280`, and the tooltip/message cluster `0x5137b0`/`0x5142e0`).
- **Creator `0x41c620`** (thiscall; descriptor with face buffer at
  `+0x14`, HFONT at `+0x10`): calls `CreateFontA` with
  `nHeight=[+0x10]`, `nWidth=[+0xc]`, escapement/orientation 0,
  `fnWeight=[+0x14]`, no italic/underline/strikeout, `charset=1`
  (DEFAULT_CHARSET), output/clip precision 0, `quality=2`
  (PROOF_QUALITY), pitch/family 0, face `+0x14`. If that fails it
  retries once with the hardcoded face Gulimche `0x64407c`; if that
  fails too, error. Requested width/height/weight are stored at
  `+0x11c`/`+0x120`/`+0x12c` (re-create without re-asking: `0x41c820`).
- **Cell metrics**: after creation the descriptor measures the
  single character `'A'` (VA `0x6442a4`) with `GetTextExtentPoint32A`
  (helper `0x41c7e0`) on a `GetDC` of the main window (display global
  `0x768128` → `+0x10` through `0x431e80`) and caches cell
  width/height at `+0x124`/`+0x128`. That creation DC is never
  released (import counts: GetDC ×2, ReleaseDC ×1) — exe behavior,
  replicated or not at our discretion.

### Pinned font tables (name, width × height, weight)

| Owner | Fonts (stored at) | Sites |
|---|---|---|
| Loading gate `0x4e47a0` (`ev_68d920c2…`) | Trebuchet MS 27×65 w400 → `+0x524`; Arial 7×14 w400 → `+0x528`; Gulimche 6×12 w400 → `+0x52c` | `0x4e47e8`–`0x4e483a` |
| Result builder `0x4c2390` (`ev_4c519351…`) | Gulimche 8×16 w400 → state `+0x64`; Arial 8×16 w400 → state `+0x68` | `0x4c2871`–`0x4c289b` |
| TEXT control stager `0x50f280` | Gulimche 6×12 w400 → control `+0x164` | `0x50f2b6`–`0x50f2c4` |

## The painter family (CDXSurface members)

| Painter | Signature (thiscall) | Behavior |
|---|---|---|
| Plain `0x42c4c0` | (text, x, y, ink, font) | `SetBkMode(1)` → select → `SetTextColor(ink)` → `TextOutA(hdc, x, y, text, strlen)` |
| Outline `0x42c730` | (text, x, y, ink, shadow, rows, cols, font) | If rows > 2 and cols > 2: `SetTextColor(shadow)` and `TextOutA` at **every** (x+j, y+i), i < rows, j < cols; then `SetTextColor(ink)` and one center copy at (x+1, y+1) — a 1 px halo. Otherwise delegates to plain |
| Fit `0x42c360` | (text, rect, ink, font) | `GetTextExtentPoint32A`; if wider than the rect → rect painter with `DT_WORDBREAK` (0x10); else plain at the rect's top-left |
| Rect `0x42c650` | (text, rect, ink, font, dtflags) | Refreshes the font cell cache (`0x41c7e0` + copy to `font+0x124`, then `0x41c820`), `SetBkMode(1)`, select, `SetTextColor`, `DrawTextA(hdc, text, strlen, rect, dtflags)` |
| Rect outline `0x42c8a0` | (text, rect, ink, shadow, rows, cols, font) | The rect twin of outline: per-cell offset `RECT`s, shadow `DrawTextA` pass, then ink pass on the interior rect |
| Rect outline 2 `0x42cab0` | same shape | Second shadow+ink `DrawTextA` variant with a plain fallback |
| Decorated `0x42c570` | (x, y, x2, y2, colors) | `MoveToEx`/`LineTo` underline/frame strokes beside a `DrawTextA` (single call site `0x4305a0`) |
| Display plain `0x42c420` | — | Two-pass plain draws against the global target getter `0x415d00` (`display->+0x10`) |

Import-site ownership (corrects the loose cluster wording in
`channel-fill.md` OPEN 2): `TextOutA` sites `0x42c534` (plain
`0x42c4c0`) and `0x42c7e1`/`0x42c856` (outline `0x42c730`);
`DrawTextA` sites `0x42c6ec` (rect `0x42c650`), `0x42c997`/`0x42ca55`
(rect outline `0x42c8a0`), `0x42cb8a`/`0x42cc2b` (variant
`0x42cab0`), plus the tooltip/message USER32 cluster at
`0x515xxx`+ already pinned in `channel-fill.md` (CCToolTip::Draw
`0x5137b0` region).

## Draw sites

- **Result screen** — the fill `0x4c3120` (runs once from enter,
  `ev_38f32576…`) drives every text call: top painter `0x4c38a0`
  (one direct call `0x4c3af6`, the total score, plus six numeric
  fields through the width wrapper `0x4c43d0` at
  `0x4c3988`–`0x4c3a6d`), row painter `0x4c3b40`
  (sites `0x4c3ea3`–`0x4c4106`, `ev_88c3ca31…`). Every site passes
  ink `0xffffff`, shadow `0x000000`, rows 3, cols 4 — so the outline
  branch always runs: white digits with a 1 px black halo. Numbers and
  total score select the Arial descriptor from body `+0x68`;
  nickname rows select the Gulimche descriptor from body `+0x64`
  (site `0x4c3e79`–`0x4c3ea3`). This closes `cresult-anchors.md`
  OPEN 5: the trailing args `(0, 3, 4)` are shadow/rows/cols, the
  eighth argument is the font descriptor, and the x/y are the
  TextOutA/DrawTextA origin (left/top aligned) — passed x is the
  left edge, as `paint.rs` already assumes.
- **TEXT controls** — slot-9 painter `0x50f320` (pinned in
  `channel-fill.md`): reads the caption at control `+0x60` and the
  rect at `+0x3c`, dispatches by mode fields — `+0x17c` set → rect
  outline (`0x42c8a0`, or `0x42c730` fed rect.left/rect.top);
  `+0x190`==0 → fit `0x42c360`; `+0x190`==1 → rect `0x42c650` with
  dtflags 0x1 (`DT_CENTER`). Ink `+0x174` (ctor default `0xffffff`),
  shadow/rows/cols `+0x178`/`+0x180`/`+0x184` (the setter-trio
  triple), font handle `+0x164`. The surface argument defaults to
  `0x415d00` — the display's current target (`+0x10` of the global
  `0x76816c` → `+0x80`).
- **Dialogs and tooltips** — shared dialog text painters
  `0x430430`/`0x4304a0` behind the dialog classes `0x50b320`,
  `0x50be50`, `0x50cb50` and the tooltip/message callers
  `0x513990`/`0x513c60`/`0x514144` (the `Rectangle` +
  `CreateSolidBrush` + `DrawTextA` cluster `0x515xxx`+ from
  `channel-fill.md`).
- **Wrapper cluster** `0x418ab0`/`0x418b30`/`0x418b80` call plain/
  outline/variant painters; `0x418b30` has no direct caller (see
  OPEN). `CStateBase` slot-9 draw-all `0x48c570` also measures text
  once (`0x48c9cf`).

## Prerender vs live — verdict

GDI rasterizes glyphs at every paint call; the exe holds no glyph
atlas and no per-string bitmap. What persists differs by site:

1. **Scene numbers (result screen): prerendered per fill.** The
   painters' `this` is the fetched body CDXSurface (the background
   body argument, per `cresult-anchors.md`): the halo pass and ink
   pass write into that body's page surface once, at fill time. The
   body surface persists and composites per frame, so the text rides
   the background quad for the rest of the screen's life.
2. **Control and dialog text: live each frame.** `0x50f320` is the
   slot-9 `Draw` override invoked every frame by the render loop and
   the child-array paint (verdict 3, `ev_ff581cfa…`); dialogs paint
   through the same per-frame control walk. Text redraws whenever the
   stored caption or style changes — which for chat/list rows is
   constantly.

A bitmap-font substitution therefore matches positions, colors, and
the outline look, but not glyph shapes: Arial/Gulimche at 8×16 and
6×12, Trebuchet MS at 27×65. The faithful pass needs: named-font
descriptors (face, width, height, weight) with an "A"-cell advance
cache; transparent background; left/top anchoring with per-call ink;
the outline mode (shadow copies in a rows×cols block plus ink
center); the fit/wrap rect mode with `DT_*` flags; and per-state
font tables mirroring the pinned ones above (owner decision recorded
in the repo memory: rasterize with a modern cross-platform font
stack, exe pins placement only).

## OPEN

1. Page-object class identity and its slots 7/17/26 (blit/GetDC/
   ReleaseDC) — carries over ui-draw OPEN 1; likewise the device-side
   texture sync for GDI-drawn pages (ui-draw OPEN 2).
2. Owner of the wrapper cluster `0x418ab0`/`0x418b30`/`0x418b80`:
   `0x418b30` has zero direct call sites, so it is reached through a
   vtable or thunk; the owning dialog classes are unnamed.
3. Consumer of the decorated painter `0x42c570`: single call site
   `0x4305a0`; which dialog uses underlined/framed text is unpinned.
4. Strings drawn by the display-level painters `0x42c420` and
   `0x4153b0` (call site `0x4154b4`) — possibly a startup/version
   stamp; not traced to a literal.
5. Loading-gate font consumers: reads of `+0x524`/`+0x528`/`+0x52c`
   (which captions take Trebuchet 27×65 vs Arial 7×14 vs Gulimche
   6×12) are unpinned; the loading bar/caption geometry remains a
   placeholder per `channel-fill.md` OPEN 5.
6. CCTextBox mode-field writers (`+0x17c`/`+0x190`): the pinned
   setter trio covers `+0x174`/`+0x164`/`+0x178..0x184`; who sets the
   two mode words (and which shipped dialogs use which painter) is
   OPEN, subject to the same channel-TITLE join gap as
   `channel-fill.md` OPEN 2.

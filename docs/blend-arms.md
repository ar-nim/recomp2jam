# Blend/alpha draw arms of the entry code (7.55) — VERIFIED position arms

Question: do the `0x50000000` / `0xA0000000` tests on the draw-call
argument pick source-draw vs alpha-blend vs color-fill? Answer: **no
blend anywhere**. Both masks are staging-position selectors; every arm
paints the same sprite as an opaque source draw. `o2jam-render` needs
no alpha compositing for the control sprite path.

Version pin: `AngelO2Jam.exe` 5,029,888 B, SHA-256 `83a04541…72c823b`
(install-root copy, hash re-verified this session). Method: read-only
byte walks plus a local capstone 5.0.7 linear sweep of VA
`0x401000`–`0x5f5000` for the two mask immediates (instruction-aligned
sites only), a PE import walk, and direct-call xref scans. VA = file +
`0x400000`. No disassembly or decompiled code is reproduced. Offsets
not already covered by a dossier are unfiled evidence pending an REA
record (same convention as `docs/claims-render.md`).

## The masks and the complete site list

- `code & 0x50000000` — bits 30 and 28: **origin arm**.
- `code & 0xA0000000` — bits 31 and 29: **rect arm**.
- The origin arm is tested first; a code carrying both groups takes
  the origin arm. Both clear in `CCImageBox::Draw` → nothing staged.
- `0x50c4b0` additionally compares the stored flag word equal to the
  whole constants `0xA0000000` and `0x90000000` (exact-code arms, not
  masks).

Instruction-aligned test sites (linear sweep; none elsewhere, and the
D3D quad builder `0x5dec30` has no tests of either mask):

| Function | Sites (VA) |
|---|---|
| `CCImageBox::Draw` `0x50a890` (dossier `ev_cc0942ba…`) | `0x50a9bd`, `0x50a9e8`, `0x50aae8`, `0x50ab13`, `0x50ac13`, `0x50ac3e`, `0x50ad92`, `0x50adb8` |
| button-class create `0x4f60c0` (named "CCButton init" by `docs/claims-render.md` verdict 2) | `0x4f6113`, `0x4f61e9`, `0x4f6215`, `0x4f6364`, `0x4f638a`, `0x4f6447`, `0x4f6472` |
| unnamed sibling create/draw `0x4f5d70` | `0x4f5dd2`, `0x4f5eae`, `0x4f5eed`, `0x4f5f68`, `0x4f5fa6` |
| SET-group applier `0x50e4e0` | `0x50e5d9` |
| unnamed slider-style helper `0x50ee50` (3 internal callers `0x50ece2`/`0x50ed9a`/`0x50ee36`) | `0x50eefa` |
| shared text-draw helper `0x50c4b0` (35 direct callers, `0x435b7c`–`0x4b28c6`) | `0x50c88f` (`cmp == 0xA0000000`), `0x50c8d9` (`cmp == 0x90000000`) |
| text-control create `0x50f280` (font "굴림체" string `0x64407c`, object at `+0x164`) | `0x50f2a3` |

## Arm semantics per function

`CCImageBox::Draw(code, entry)` `0x50a890` — four stem branches, each
with the same two arms; the arm picks only the staged position:

- `.ojs` (suffix `0x64b124`, lookup `0x40ced0`) → stager `0x42fc70`,
  layer 1.
- `"oji"` (suffix `0x64bf80`, lookup `0x40d100`) → stager `0x42fc10`,
  layer 2.
- `"oja"` (suffix `0x64ad98`, **new**) → stager `0x42fc10`, layer 5.
- `"ojt"` (suffix `0x6446cc`) → 0x60-byte object (ctor `0x42da40`,
  load `0x42dbb0`, registered into manager `0x76816c` by `0x418290`),
  staged by `0x42fcd0` into the same view object `+0x58`.
- Origin arm: stage at `(0, 0)`. Rect arm: fetch the rect (below) and
  stage at its `x1`/`y1`. Tail: animation mode 1 (`0x42f5a0`).
- Draw releases any staged sprite first (`0x409c30(0,0)`) and restages
  every call.

Button-class create `0x4f60c0` — same arm shapes over `.ojs` (layer 1),
any other image suffix (layer 2), and `.ojt` (object path); archive
slot 0 instead of the entry's slot field. Pre-guard: on rect-arm codes
it releases the previously staged sprite (`0x409c30(1,0)`) unless that
sprite's class bits are `0x2xxxx` (`vtbl+0xc` code `& 0xF0000 ==
0x20000`).

Unnamed sibling `0x4f5d70` — both arms stage at `(0, 0)`; the arms
differ only in that the origin arm also registers the image with
manager `0x76816c` (`0x4177f0` / `0x417820`). Rect-arm codes clear the
staged sprite first; archive select must succeed (result `!= 3`); tail
animation mode 2.

SET-group applier `0x50e4e0` — drives SET children by loop index
(child count from set `+0x08`, children pointer set `+0x0C`): child 0
gets animation mode 6; child 1 gets mode 6 and, when its own entry
code carries `0xA0000000`, is moved to its own bound rect corner —
`0x548640` with the child's code (so the loader pairing rule of
`ev_3a869bb7…` applies) and move via `0x50dd90`.

Slider-style helper `0x50ee50` — float thumb position (fields `+0xa8`,
`+0xb8`, ratio `+0xc8`, mode flag `+0x64`): with `code & 0xA0000000`
set it fills the rect at `+0x98` through USER32 `SetRect` (IAT
`0x63f390`, branch polarity re-verified on raw bytes `0x50ef00` `74
6d`), positions the child sprite (`0x42ff10`), and persists the rect
with the write-back helper `0x5487b0` (`ev_3a869bb7…`); without the
mask it takes an argument-driven path at `0x50ef6f`.

Text-draw helper `0x50c4b0` — exact arms on the stored flag word
`+0x10`: equal `0xA0000000` → position the sprite and nudge singleton
`0x77acc8` (query `0x514340`, set `0x5143e0`, offset `−height`);
equal `0x90000000` → singleton `0x77b090` (`0x514340`/`0x514360`) and
sprite call `0x42ff30`; any other code → plain position (`0x42ff10`).
Zero low bits mean these codes pair no rect (same mechanics as
`docs/claims-render.md` verdict 5).

## The rect arm's rect is record rect 0

Rect helper `0x548640(out, n)` computes the ordinal `(n & 0xFFF) − 1`
into the current record of the screen-record holder `0x768124` (record
vector `+0x3c`, current index `+0x4c`, assert `0x5f1eca` on overflow).
Every draw-side call passes the literal `1`, so the rect arm stages at
**rect ordinal 0 of the current screen record** — not the control's
own paired rect. The SET applier is the exception: it passes the
child's own code, pairing the child's own rect. This refines
`docs/claims-render.md` verdict 2: the `x1`/`y1` fed to the stager
come from record rect 0 via low word 1.

## Alpha formula

None is pinned by these flags — four independent negatives:

1. No alpha constant or blend arithmetic in any arm; the only
   immediates are positions, layers, and the rect ordinal.
2. No blend import at any site; the single import reached is
   `USER32!SetRect` (a rect store).
3. The blit flag word stays `0x10`/`0x11` (unclipped/clipped —
   `docs/ui-draw.md` step 6); the draw-call argument never reaches the
   page dispatch `ev_08a33fe4…`, which receives only
   (x, y, src, rect, flags).
4. The quad-building render loop `0x5dec30` contains no tests of
   either mask.

Renderer decision for `crates/o2jam-render`: the opaque colorkey
primitives (`paint_native_scene`, `blit_image_anchored`) are
exe-faithful for both arms. `blend_xrgb`, `fill_rect_alpha`, and the
`*_alpha` blits have no consumer on the control draw path; keep them
for other systems only if one is later proven.

## Which controls request which arm (observed codes)

- Origin arm (`0x50000000` group): Keyeffect lane beams
  `0x1B500074`–`0x1B50007A`, Note_Click bursts `0x1B500093`–
  `0x1B500099` (both families stage at `(0,0)` and ride the per-frame
  anchor, `docs/ui-draw.md`), and `Playing_Exit` `0x1B200001`.
- Rect arm (`0xA0000000` group): avatar-dialog images `0x21100001`,
  `0x21200002`. Refinement of `docs/claims-render.md` verdict 1:
  those codes carry bit 29, not bit 28, so they take the rect arm —
  the anchor==rect probes still hold, and more directly, because the
  staged position is the record-rect corner itself.
- Codes carrying both groups (`0x3xxxxxxx`, `0x7xxxxxxx`,
  `0xFxxxxxxx`): none observed in the documented lists.
- Exact codes `0xA0000000` / `0x90000000`: observed only as the
  text-helper's ticker variants; no ControlList entry documented.

## OPEN

1. RTTI class-name join for `0x4f5d70`, `0x4f60c0`, `0x50c4b0`,
   `0x50ee50`, `0x50f280` (unchanged from `docs/ui-loader.md` OPEN 1).
2. Manager `0x76816c` (registration `0x4177f0`/`0x417820`/`0x418290`)
   and the singletons `0x77acc8`/`0x77b090`: render-time consumers
   unpinned.
3. Device-side alpha (D3D quad vertex colors, render state) is
   unexamined; it is not keyed to these flags, so any fade effect must
   live there — separate question, separate note.
4. OJA format undocumented; layer-5 staging for `.oja` stems is new
   (consistent with the layer 2/3/5 callback set in `docs/ui-draw.md`
   step 3).
5. These new offsets are unfiled as REA dossier records; file before
   implementing against them.

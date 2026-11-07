# Reference scene composer vs ours — read-only comparison

Compares the Workshop Center scene composer against our shell
composer. Read-only corroboration: nothing was copied; every rule
below is restated in our own words and tagged CONFIRMED-BY-EXE (with
offsets) or NEEDS-EXE-CHECK.

Version pins. Reference: `o2jam-workshop-center` @ `11a22dc5…02ffe`
(2026-09-20, clean tree), files `src/pages/ScenePage.tsx`,
`src/features/scene/{sceneUtils,model}.ts`, `src/o2jam/controllist.ts`,
`src/o2jam/{opi,sprite}.ts`. Ours: `crates/o2jam-shell/src/avatar.rs`,
`screens/paint.rs`, `crates/o2jam-file/src/{resolve,control}.rs`, plus
`docs/opi-pattern.md`, `docs/bnd-semantics.md`, `docs/ui-loader.md`.
Exe oracle: `AngelO2Jam.exe` 5,029,888 B, SHA-256 `83a04541…72c823b`
(file offset = VA − `0x400000`).

## Pairing: the one hard divergence

The reference pairs controls to `.bnd` rects **positionally**: the
parser assigns each entry (base excluded) a running file-order index
and reads `bounds[index]`; the SET-file-order counter is shared with
top-level entries. Ours pairs by code ordinal,
`(code & 0xFFF) − 1`, bounds-checked, miss → all-zero rect.
CONFIRMED-BY-EXE: pairing step VA `0x548640` = file `0x148640`,
dossier `ev_3a869bb7…` (`docs/ui-loader.md` step 7,
`docs/bnd-semantics.md`). The positional reading fits 38/38 bound
Interface blocks only because low words run sequentially — the
reference implements the consequence, not the rule; it would mispair
`State_Playing` (17 rects serve the trailing `EXIT`…`VOLUME` run).
Its base-entry exclusion (first entry never takes a rect) matches the
exe effect — the only zero-low-word rows are block self rows —
CONFIRMED-BY-EXE (histogram, `docs/opi-pattern.md` field table).
Its fallback on a missing bound (draw at the sprite record anchor)
matches our all-zero-rect + anchor behavior.

## Rules the reference implements that ours lacks

1. **SET-group anchor placement** — members whose group byte matches
   a `SET` id are placed relative to every same-`type`/`group`
   top-level control's rect, the member rect read as an offset inside
   each anchor (row templates multiply across rows). Ours pairs SET
   children individually and applies no group semantics. Grouping and
   recursion exist in the exe (wrapper `0x48A620`, `ev_1be2d784…`);
   the anchor-relative interpretation is NEEDS-EXE-CHECK
   (`docs/bnd-semantics.md` OPEN 3).
2. **Dialog scenes** — `DIALOG_*` blocks are first-class scenes
   (27-dialog array, `0x4DA630`/`0x4E4EE0` appliers,
   `ev_9e526b62…`); ours stages no dialog block art (only the
   channel-bar fallback). NEEDS-EXE-CHECK beyond the avatar dialog.
3. **Parent-offset (block origin) shift** — every rect is drawn
   offset by the earliest-bound control's rect origin (mirrors the
   exe write-back helper `0x5487B0`); dialogs are thereby
   client-relative. Helper exists CONFIRMED-BY-EXE; generic dialog
   application NEEDS-EXE-CHECK (`docs/ui-loader.md` OPEN 3,
   `docs/bnd-semantics.md` OPEN 2).
4. **Per-frame relative offsets** — multi-frame sprites draw frame 0
   at the placement origin and frame *i* at
   `anchor(i) − anchor(0)`, plus fps playback. Ours paints multi-image
   bodies as stacked layers (unbound) or collapsed to the rect origin
   (bound, `paint_without_anchors`). NEEDS-EXE-CHECK; corroborates
   our OPEN "layers vs frames".
5. **Type-based position source** — buttons (type `0x20`) and
   toggles (`0x01`) default to their sprite record anchor even when
   bound; the rest default to the bound origin. Ours: a nonzero rect
   always wins. NEEDS-EXE-CHECK (ties to our `BpPoint` anchor≠rect
   note in `avatar.rs`).
6. **Text controls** — `IDC_TEXT*`/`IDC_EDIT*` render styled text
   (align, color, size, outline) inside their rect; inputs are type
   `0x30`. Exe confirms the control classes (16 factory arms
   `0x48B2D0`, CC* RTTI at file `0x27E705`); styling is
   NEEDS-EXE-CHECK.
7. **Scrollbar params as controls** — `O2_SBS_HORZ/VERT w h` are kept
   as parameterized controls; ours stages nothing (step 9: they are
   not files — both agree on that).
8. **Unclaimed rects surfaced** — leftover rects render with
   synthesized ids. No exe basis: the exe leaves them unloaded
   (Playing tail past `VOLUME`; step 5, decl `0xE6`). Editor
   affordance; ours is exe-faithful.
9. **Codec breadth** — the reference decodes all sprite codecs
   across `.ojs/.oji/.ojt/.oja`; ours decodes raw RGB555 only
   (documented TBD).

## Divergences

- **Colorkey 0**: the reference maps a stored key of `0` to a default
  magenta (`255,82,255`) matched with a tolerance threshold; ours
  blits `0x0000` literally and matches exactly. Our archive walks
  show `0x0000` stored in 3444/5787 bodies, but stored-0 semantics
  (alias vs literal black) are NEEDS-EXE-CHECK.
- **Origin shift on states too**: the reference shifts every bound
  block, not only dialogs — a no-op while `State_*.bnd` rect 0 is
  `(0,0)-(800,600)` (CONFIRMED-BY-EXE span), but mechanistically
  broader than the exe evidence.
- **Editor-only behaviors** (hide toggles with the background entry
  pre-hidden, drag reorder of draw order, BMP frame replace, block
  add/rename/remove, per-sprite anchor overrides): no runtime rule;
  its reorder proves list order = draw order, corroborating our
  file-order painting (step 8, background-first).

## Agreements (corroboration, not evidence)

File-order painting; entry 0/self draws nothing; stub blocks stage
nothing; sprite-less entries draw no art; case-insensitive sprite
lookup (ours exact-first then one case pass); rect positions, never
clips; 800x600 absolute space, exclusive `(x2, y2)`; single-image
anchor == rect duplication (three probes, step 7); trailing
parameter-less `SET` runs folded by the parser.

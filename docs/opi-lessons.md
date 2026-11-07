# OPI rendering lessons — how sprites are placed (7.55)

Earned through the playing-screen parity pass against a real-game
photo (`docs/screenshots/reference-4x3.png`). Every rule below is
exe-verified; the reference toolkit (o2jam-workshop-center) and real
bytes corroborated along the way.

## The placement rules

1. **Rects pair by code ordinal, never file order.**
   `(entry.code & 0xFFF) − 1` indexes the `.bnd` rect table; a miss
   stores an all-zero rect — nothing is skipped (exe `0x548640`,
   ev `3a869bb7…`). File order only looked correct because ids are
   sequential.
2. **Bound art paints at its rect origin alone.** Sprite records for
   bound bodies carry absolute canvas anchors that duplicate their
   rect origins byte-exact; adding both double-positions the art
   off-screen. Rect never clips — clipping is viewport-level
   (`DrawClip` exists, but the blit never reads the rect).
3. **Unbound (all-zero rect) art paints at its record anchor**,
   offset from the placement origin. Anchors are signed i16 — parse
   them signed; 144 real sprites carry negatives.
4. **`_`-prefixed refs are stage-numbered.** `_Playing_BG.ojs`
   strips `_` + extension and appends the room arena stage
   (`Playing_BG<1..36>.ojs`, global `0x7789dc`, picked in the
   waiting room via 4002). The judgement/combo/click/jam kits follow
   the same rule — one mechanism, every scene.
5. **Paint order is entry file order** (background first,
   topmost-last), `.oji` bodies over `.ojs` (layer 2).
6. **Colorkey comes from the image record's u16 field**, decoded to
   the target color space before compare (555→565 re-encode exists
   device-side); never compare raw-space literals.
7. **Mode families gate staging.** The Playing block carries
   mode-twin families (note-BG, life-gauge, BP-buffer per 3/5/7 key
   and P2P variants); only the member the joined room names paints.
   Staging all of them scatters art across the screen.
8. **Half-res frames must be scaled to their placement**, not blitted
   1:1 — the exe scales, our 1:1 shortcut showed quarter art.
9. **Digits and captions are sprites too**: `Scroe`/`PlayTimeNum`/
   `Note_ComboNum` are ten-frame fonts (0–9); bind all ten or fall
   back. Room numbers use the same pattern (`ROOMNO1/10/100`).

## The debug method that found them

Compare region by region against a real-game screenshot, then for
each misplaced sprite trace the chain: ControlList entry → ordinal
rect → staging → painter math. Every bug we fixed was one of:
double-add (rect + anchor), wrong pairing (file order), family
over-staging, missing scale, or invented coordinates. The exe draw
chain (`CCImageBox::Draw` → anchor add `0x41DF80` → page dispatch)
is the final arbiter; data-side inference alone produced plausible
but wrong rules.

## Remaining art-vintage deltas (documented, not bugs)

The photo's rooftop stage and gold combo-digit styling come from
assets outside the archived 36 stages / procedural fonts; layout,
sizes, and positions match.

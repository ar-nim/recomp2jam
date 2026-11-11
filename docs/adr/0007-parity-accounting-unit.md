# Parity accounting unit

The claim (`docs/claims.md` row) is the atomic unit of parity
accounting. A claim carries the evidence (level, anchor, evidence id)
and drives one or more cells in `docs/parity.md`. Cells are reporting
units: a (scene, dimension) row in the parity ledger whose verdict is
computed from the claims that drive it, plus implementation guards and
its own captured evidence.

## Ids

Claims keep the existing `A`/`B`/`C`/`D` prefix (domain: rendering,
gameplay, file formats, net) with sequential numbering; ids are never
reused or renumbered. Retired claims stay in place, marked refuted.

Cells get ids `SCENE.DIM` where scene is the uppercase GLOSSARY.md
scene name (AVI, CHANNEL, ROOM, WAITING, LOADING, PLAYING, RESULT,
PLANET, SHOP, MSHOP, MYROOM, LANKING, PAYMENT, PAYMENT2, TUTORIAL)
and DIM is the dimension (COMP, INPUT, TIME, TRANS, GAME, MUSIC,
CHAT). The side-state section gets one id per state bullet
(`SHOP.COMP`, `MSHOP.INPUT`, …).

## Join

One direction only: each claim row carries a `Cells` column naming the
cell ids it drives. A cell never lists claim ids. Rationale: the claim
is where evidence lives, so the join starts there; the parity row can
then be audited from its driving claims without a second index.

## Rules

- A claim may drive many cells (A1 drives ROOM.COMP, WAIT.COMP,
  CHAN.COMP, …); a cell always has at least one driving claim.
- A cell's verdict may not rest on a claim whose level is ASSUMED or
  OPEN unless the row says so explicitly (the re-audit enforces this).
- `docs/claims.md` stays hand-written; the consistency check
  (wayfinder ticket #48) validates id shape, level vocabulary,
  non-empty anchors, no duplicate ids, and that every `Cells` entry
  names a real cell.

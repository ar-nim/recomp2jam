# ADR-0015: Keep equipment-slot paint order; pin the 7.55 draw-order table as a divergence

Date: 2026-11-20

## Status

Accepted (ticket #908). Divergence stays OPEN with a named next evidence step.

## Context

The 7.55 avatar paint walks a fixed 5×36 byte table at VA `0x64c770`
(file `0x24c770`; file offset = VA − `0x400000`). Ticket #908 required the
bytes to be extracted, compared with our painter's equipment-slot order,
and either consumed by the painter or documented as a deliberate
divergence. The bytes and the consumer are now pinned in `docs/equip.md`
(`Draw-order table (#908)`), with REA 6.9.0 / Ghidra 12.2 evidence IDs and
the 7.55 SHA-256 `83a04541…72c823b`.

Our `load_worn_figure` stacks bodies in equipment-array order (17 cells,
ascending), and inside one item row in part-block order 0..3 from
`pick_wear_sprites`; `AvatarFigure` blits one frame per stacked run in
that order. The exe instead visits indices into a 36-dword part-surface
array at `CAvatar + 0x150`. The two orders differ from the first body:
the idle row starts `0x23, 0x0b, 0x1e, 0x14, 0x22`, while slot order
starts `0x00, 0x01, 0x02, …`.

The exe's array indices are per-part surfaces filled by a restage from
29 item pairs (`CAvatar + 0x68`), with secondary surfaces for a few item
indices at array slots `0x1e..0x22`. Our figure has no representation of
those 29 part slots or of the secondary surfaces. Consuming the table now
would require inventing a mapping from equipment cells and part blocks to
those indices — an assumption, not behavior evidence.

## Decision

Do not consume the table in the painter. Keep equipment-slot paint order
and record the mismatch as an OPEN divergence in `docs/equip.md` and here.
The extracted bytes are pinned as evidence; no z-order guard is claimed.

## Consequences

- The avatar's layer order stays slot order until the part-slot mapping
  is pinned. This is an OPEN parity gap, not a verdict that order is
  invisible: no live overlapping-part capture has settled the pixels.
- A later ticket may consume the table only after the 29 exe part slots
  are mapped to equipment ids and part blocks. Next evidence step: trace
  the restage caller (`FUN_0052b480`, VA `0x52b480`) back to the 7.55
  equipment table and read the 29 pairs in a stopped Wine session, then
  compare one overlapping-kit capture against the table order.
- `docs/equip.md` divergence 2 points here instead of leaving a bare
  OPEN note.

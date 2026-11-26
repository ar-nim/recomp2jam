# ADR-0016: Land exe parity for a missing pose slot

Date: 2026-11-20; parity landed 2026-11-26 (ticket #913).

## Status

Landed parity. The picker reads only the exact pose slot of the wearer's
parity and returns no body when it is empty. Guards cover the selector,
the staged figure, and the female wearer's male-slot case. The earlier
fallback and its divergence were an OPEN record, not the final behavior.

## Context

`docs/equip.md` divergence 1 records that the 7.55 exe returns null
for an empty pose slot: a part block with no sprite in the kit's chosen
pose and wearer parity contributes no body, and the painter draws
nothing there. Our `pick_wear_sprites` instead falls back to the other
sex parity, then to the first non-empty pose slot of the wearer parity,
so a garment never vanishes. That fallback can paint a wrong-parity or
wrong-pose substitute where the exe paints no pixels.

The exe behavior is pinned by the avatar restage and sprite-slot
selector described in `docs/equip.md` (`Draw-order table (#908)`):
restage VA `0x52b480` builds one pose word from the wearer's sex bit and
the kit's instrument type; selector VA `0x52e730` reads only that pose's
slot and returns zero when the item row has no sprite there. REA 6.9.0 /
Ghidra 12.2 evidence IDs:
`ev_25a328cdd807fb4b48bb126dcb375bcc489d58251a33fb1cce51cfc10fcb85fc`
and
`ev_407fcd2991e5ff7c2d30e1b024f09eb661a6a5a9c044219c136bf8790c521fab`.
Oracle: `OTwo.exe`, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.

A parity patch was first tried, TDD style: three new guards failed on the
old picker, then passed after the change. The full workspace gate then
failed five `connect.rs` fixtures that gave a female wearer only a male
pose slot. Ticket #913 fixed those fixtures first: each wearer now has
its own pose sprite. It then reapplied the picker change and the three
guards in one integrated commit.

## Decision

`pick_wear_sprites` reads one slot per part block: the kit's pose variant
at the wearer's parity offset. It does not try the other parity or an
other pose. An empty exact slot contributes no body, matching the exe's
null return at selector VA `0x52e730`. The old fallback is removed.

## Consequences

- A kit whose item row lacks the kit's pose paints nothing for that
  block. This now matches the exe pin; `docs/equip.md` divergence 1 is
  resolved for the missing-pose-slot case.
- The five `connect.rs` fixtures give each wearer parity its own sprite;
  they no longer pin the withdrawn fallback.
- The default naked-kit fallback (#243/#375) is unchanged: it composes
  missing *equipment rows*, not missing pose slots.

# ADR-0016: Keep the missing-pose-slot fallback and record the draw-nothing divergence

Date: 2026-11-20

## Status

Accepted (ticket #909) before any code change. Parity remains OPEN with
a named next step; no guard is claimed for this ticket.

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
`83a04541b8219ba340681766b33de2bfbb5ff51b813ef4d2e9dd1def72c823b`.

A parity patch was tried first, TDD style: three new guards failed on the
old picker, then passed after the picker read only the exact pose and
parity slot. The full workspace gate then failed five guards in
`crates/recomp2jam/src/connect.rs` (`avatar_io_tests`:
`stacks_figure_from_memory_without_disk`,
`character_equipment_selects_worn_layers`,
`male_and_other_heroes_both_stack`; `connect_flow_tests`:
`builds_worn_figure_from_archive`,
`stacks_worn_avatar_from_local_archive`). Those fixtures build a one-row
item table with art only in male slot 2 and call the stacker for a female
wearer; they expect the cross-parity body. That file is outside this
ticket's owned paths, so the patch and its guards were withdrawn rather
than land a red workspace.

## Decision

Keep the existing `pick_wear_sprites` fallback for now and record the
difference from the exe as an OPEN divergence. Do not claim parity for
an empty pose slot and do not ship a guard that pins the wrong behavior.
The ADR lands before any code change; no code change lands in this
ticket.

## Consequences

- A kit whose item row lacks the kit's pose can still show another pose
  or parity as a stand-in. This is a known FAIL against the exe, not a
  silent choice; `docs/equip.md` divergence 1 points here.
- Next evidence step: hand the five `connect.rs` fixtures to the lane that
  owns that file, change them to give each wearer parity its own sprite,
  then reapply the withdrawn picker change and its three guards in one
  integrated commit. The patch is kept at
  `~/scratch/lane-avatar-guards/parity-attempt.patch` for that handoff.
- The default naked-kit fallback (#243/#375) is unchanged: it composes
  missing *equipment rows*, not missing pose slots.

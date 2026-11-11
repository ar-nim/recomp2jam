# Playing scroll map — note-row y from song time (7.55) — PINNED

The note-row scroll map: how the 7.55 client turns `(onset, now)` into
a pixel y on the highway. Closes `docs/bnd-audit.md` 32 and the
audit-32 arm of the PLAYING.COMP parity row (`docs/parity/playing.md`),
child of map #66 (#106).

Version pin: `AngelO2Jam.exe` == `OTwo.exe`, 5,029,888 bytes,
SHA-256 `83a04541…72c823b`. VA = file offset + `0x400000`.

Method: read-only byte walks over the installed exe (r2/objdump slices
in git-ignored `~/scratch/scroll106/`, LE32 reference scans). No
disassembly or decompiled code is reproduced; behavior is described in
our own words. One dossier pass per the ADR-0009 stop rule; the shape
came back unambiguous, so no rig session was spent.

## Answer first — the map

Constant-velocity linear, integer, per-lane speed factor:

```
y = 480 - (onset_ms - now_ms) * speed[lane] + (lane_height[lane] - 6)
```

- `480` (`0x1e0`) is the judgement-row y: every note arm loads
  `mov $0x1e0` and subtracts the scrolled distance.
- `now_ms` is the integer song clock, global `0x791d70`
  (file `0x391d70`), written once per refresh by a float→int
  conversion (fistp) off the float song clock at VA `0x56ea52`
  (file `0x16ea52`).
- `speed[lane]` is a per-lane integer constant, table `0x791d54`
  (7 dwords, file `0x391d54`), copied from the per-lane geometry
  table at play-object `+0x10` by the init loop at
  VAs `0x56ea7c`–`0x56ea80` (file `0x16ea7c`–`0x16ea80`).
- `lane_height[lane] - 6` is the per-lane head-height offset, table
  `0x791d38` (file `0x391d38`), computed as table `0x791d14` minus 6
  at VAs `0x56eaea`–`0x56eaf7` (file `0x16eaea`–`0x16eaf7`).
- A note with `y < 0` is not drawn (the `js` skip at VA `0x56e1f3`);
  the draw helper re-clips against the bottom band
  (`480 - y < 7` at VAs `0x56ed4e`–`0x56ed66`). This is the item-driven
  y-band culling of claim B26, not an easing step.

The same multiply appears in every note arm of the draw dispatcher:
heads (VAs `0x56e1cb`–`0x56e1f0`), long-note tail ends
(VAs `0x56e247`–`0x56e29a`), and the second note-type variant
(VAs `0x56e33e`–`0x56e3d3`). A scalar variant of one arm multiplies by
the single global `0x791d34` (file `0x391d34`) instead of the per-lane
table — `0x791d34` is the play object's float `+0xc` times 2.0,
rounded (VAs `0x56eb03`–`0x56eb14`) — and is linear in the same way
(VAs `0x56e99b`–`0x56e9b2`).

## Where it lives

- The y computation and the table init live in one event handler,
  function VA `0x56e170` (file `0x16e170`), registered into the play
  object's `+0xa14` callback slot at VA `0x570323`
  (file `0x170323`). The sequencer district invokes that slot with
  message `0xc` / `0xb` (VAs `0x575df1` / `0x575e71`), i.e. the
  speed/offset tables are refreshed on play events (song start, mode
  change), not per frame.
- The note-draw helper (claim A34's `note_draw` parent, VA `0x56eca0`,
  record read at VA `0x56ede0`, blit `0x56b730`) receives y as an
  argument and contains no FPU instructions: the whole draw dispatcher
  is integer code.

## Negative result — no easing, no stepped table

- No floating-point unit instruction occurs anywhere in the
  note-row y path or the note draw; there is no easing curve, no
  sine/square table, no frame-fraction math.
- The only time-keyed value lookups in the play district are the two
  list walkers at VAs `0x57a0f0` and `0x57a240`. Both walk a keyed
  node list and return a float from the matching node to the
  sequencer/BGA callback path (the `+0xa14` messages above); neither
  feeds a note-row y. The note-row map is closed-form linear.

## Consequence for the shell

The shell's `y_for_onset` (`crates/o2jam-shell/src/playing.rs`)
— `y = line_y - ahead * (line_y / lookahead)`, shared by notes and
tail beams — is the same shape: constant velocity, y(onset == now) at
the judgement row, linear in `ahead`. The linear map survives as
EXE-truth; no shell change. Two numeric arms stay separately owned by
other audits (exe judgement row `480` vs the staged receptor row; exe
per-lane integer speed vs the shell's `line_y / lookahead` scale) —
both are linear-scale constants, not shape deviations.

Follow-up per the ticket's guard plan (not this ticket): a
`y_for_onset` characterization guard (constant velocity over the
pinned table row) rides a follow-up shell note in the PLAYING.COMP
parity row.

## Confidence

High for shape (constant-velocity linear, per-lane integer speed,
judgement row 480, culled when y < 0): every note arm in the only
draw dispatcher computes the same closed-form expression, and the
negative sweep found no competing writer. Medium for the exact speed
constants' provenance (geometry object `+0x10` table content is
runtime state; its seed table is not part of this ruling).

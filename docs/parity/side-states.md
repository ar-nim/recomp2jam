## Side states (reachable from lobbies)

All **OPEN** for data/input (no fetch opcodes, no buy/gift/cart/search
flows — documented so no pass is ever claimed). Composition 2026-11-10:
the walk now visits every side screen from Waiting through
`visit_side` with its genuine bound art staged (`side_stage_data`:
`.bnd` placements paired by code ordinal plus decoded figures, the
painter consumes every placement through `paint_side_scene`), and
shots `shop.png`, `musicshop.png`, `myroom.png`, `lanking.png`,
`payment.png`, `tutorial.png` show the file-native frames. Without
`install-root` the side shots stay skipped (clean skip, CI arm) and
the walk list tests assert both arms. The musicselect chosung grid
now counts its row cells and x-pitch from the staged INDEX-button
inventory (`select_chosung_layout`, FILE per the audit; row tops and
left edge stay CANDIDATE constants). Contradiction correction:
`State_Myroom.bnd` **exists** on the shipped archive (111 rects,
full-bleed r0) — the older "bind file declared but absent" note was
wrong; Myroom stages bound like every other side screen.

`STATE_DRUM_PLAYING` / `STATE_EXIT` (stubs `0x00`; drum never wired,
no `CStateDrum*` class; exit is teardown, not a screen).
  PASS (backdrop staging) / OPEN (entry path with named next step).
- `STATE_DRUM_PLAYING` / `STATE_EXIT` (stubs `0x00`; drum never wired,
  no `CStateDrum*` class; exit is teardown, not a screen).

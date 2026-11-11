## The pairing-rule family (known mistake + siblings)

The known pattern: a file-order/positional rule asserted where the exe
uses an explicit ordinal or id field. Source claim A2 ("rect *i* pairs
entry *i+1* in file order") is a DATA walk that the EXE refines: the
applier looks each entry up by `code & 0xFFF − 1` and bounds-checks
(A1). Siblings found by hunting for the same shape:

- **S1** — "SET entries consume `.bnd` rects in file order exactly like
  top-level entries" (`o2jam-file/src/control.rs:17`). EXE matches SET
  children by `code & 0xFF000` against the set key instead (A19).
- **S2** — the Playing 17-rect subset is applied as a positional zip
  ("rect *i* → entry 213+*i*", `o2jam-shell/src/avatar.rs:357`). The
  exe assigns rects by per-entry ordinal; sequential low words in that
  run make the zip *happen* to agree (A26).
- **S3** — RESOLVED: the exe pairs Room/Waiting entries by code
  ordinal at control creation (wrapper VA `0x48b09c`); the one-entry
  surplus takes the all-zero rect (A27).
- **S4** — `resolve_order_slot` keys id-less OJM banks by fetch order.
  The exe has no order lookup anywhere on load/play paths; M30 banks
  key by the explicit `ref` id, new-variant entries all land in slot 0
  (C28). The fallback is ours, labeled as such (C29).
- **S5** — difficulty mapping guessed by position: "+20 lo/hi/+24
  ascending triple" (C34) names header fields by slot order, 13/13 fit.
  The exe reads levels as `u16[difficulty]` over the song-record words
  {+24 lo, +26 hi, +28} (= header +20 lo/hi, +24) and pins the
  ascending order via the difficulty-keyed life sets (C34/C35).
- **S6** — RESOLVED: the D13 value sets are exe-pinned (icon frames,
  level-table indexing, icon grid, label chain); the Memoryer ordinals
  stand as corroboration.
- **S7** — RESOLVED to the loop level: the render loop paints stored
  (file) order (A5); the only z-order nuance found is the
  `.oji`-over-`.ojs` layer split.

Lesson: wherever a sequential rule is claimed, ask which explicit
field (code low word, code nibble, M30 `ref`, header per-level field)
actually indexes the table — and what the exe does on an out-of-range
index (all-zero rect, slot 0, default arm).

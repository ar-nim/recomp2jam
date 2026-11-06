# Codebase audit (rust-best-practices + clean-code)

Method: `rust-best-practices` P0 checklist (ownership, errors, concurrency,
unsafe, numerics) plus `clean-code` six disciplines, scored 0–10 per crate
(10 = names reveal intent, small functions, consistent errors, clean tests).
Context7 (`softbuffer` docs) used where dependency semantics mattered.
Automated baseline: `cargo fmt --check` clean,
`cargo clippy --workspace --all-targets -- -D warnings` clean.

## Scores (2026-11-06 pass)

| Crate | Score | Notes |
|---|---|---|
| o2jam-audio (358) | 9 | Small fns, good tests. Inline-qualified `std::` paths (`std::sync::Arc`); one production `expect` (poisoned mutex in the realtime callback — standard, unreturnable there). Stale Day-0 doc fixed this pass. |
| o2jam-file (4124, one file) | 7 | God-file smell (§6): OJM + OJN + OPI + sprites + shop tables in one `lib.rs`. Parsers themselves are clean (bounds-checked, synthetic-tested). Split by format is the fix. |
| o2jam-game (1474) | 8 | Verified-behavior docs excellent. Deep pass pending. |
| o2jam-net (file ~3500 across 9 modules) | 8 | `message.rs` (1638) still growing; `RoomInfo` (slot snapshot) vs `RoomListEntry` (list row) naming is one rename away from confusion. Only 2 production `expect`s repo-wide live here (framing/timeout paths map to `Error`). |
| o2jam-render (808, one file) | 8 | Cohesive pixel math; font tables + blits could split later. |
| o2jam-shell (~2500) | 9 | `Shell::run` embeds the whole `Handler` (~150 lines); `screens.rs` (936) growing per screen. `present()` silently skipped mismatched frames — fixed this pass via Context7-verified stretch. |
| recomp2jam (1192) | 8 | `main.rs` mixes CLI + bank/audio setup + autoplay; `connect.rs` mixes flow + avatar file IO. No production `unwrap`/`expect` (verified by sweep). |

## Fixed this pass

- `stretch_blit` (render) + present-path rework (shell): every window size
  shows the whole frame (Context7: softbuffer draws mismatched buffers
  unscaled). 3 new tests.
- Stale module docs: audio Day-0, dump Day-0, file OJN-stub lines.
- String encoder now truncates including the terminator (prior pass,
  recorded here: code contradicted its own pinned docs).

## Backlog (priority order)

1. Split `o2jam-file/lib.rs` by format (`ojm.rs`, `ojn.rs`, `opi.rs`, shop tables).
2. ~~Extract shell `Handler` out of `Shell::run`~~ DONE (`window.rs`; `run()` is 10 lines). Split `screens.rs` per screen — DONE (`screens/paint.rs` + `screens/game.rs`; tests moved alongside).
3. ~~`message.rs`/`rooms.rs` naming consolidation~~ DONE (`RoomInfo` → `CreatedRoom`; `RoomListEntry`/`JoinedRoom` kept).
4. Pedantic polish: DONE mechanical classes (`must_use` ×13, infallible
   `From` ×4, or-pattern merges, `is_some_and`/`map_or`, elided lifetimes,
   `LivePlay::new` takes a slice, explicit enum imports, reasoned `ignore`,
   `Ipv4Addr::LOCALHOST`, ctor field order, `u8::from(!male)`-free parity
   test, per-layer colorkeys); `# Errors` docs DONE (zero `missing_errors_doc` remaining).
   ACCEPTED with rationale (not churned): protocol-bounded `as` casts
   (target-dependent `usize` widths have no `From`; values are
   length-checked), `assert!(is_empty())` over `assert_eq!(_, [])`,
   prose-word `doc_markdown` hits, test decimal literals, `Error::transport`
   by value (point-free `map_err`), wire-mandated bool fields.
5. ~~Overlong functions~~ DONE (`fn main` 194 → dispatch + `run_command`/`run_info`/`run_play` + setup helpers; `too_many_lines` clean).

# Codebase audit (rust-best-practices + clean-code)

Method: `rust-best-practices` P0 checklist (ownership, errors, concurrency,
unsafe, numerics) plus `clean-code` six disciplines, scored 0–10 per crate
(10 = names reveal intent, small functions, consistent errors, clean tests).
Context7 (`softbuffer` docs) used where dependency semantics mattered.
Automated baseline: `cargo fmt --check` clean,
`cargo clippy --workspace --all-targets -- -D warnings` clean.

## Scores (2026-11-06 refresh)

Line counts are totals including tests; prod counts exclude `#[cfg(test)]` modules.

| Crate | Score | Notes (delta vs prior pass) |
|---|---|---|
| o2jam-audio (855) | 9 | Zero production `unwrap`/`expect` now (was 1): realtime callback uses `try_lock` + `map_or`, fallbacks use `unwrap_or`. Only nit left: inline-qualified `std::sync::{Arc, Mutex}` paths — fix with one `use` line. (→ same score, cleaner) |
| o2jam-file (7600 across 8 modules) | 9 | Split DONE (+2): `ojm.rs`, `ojn.rs`, `opi.rs`, `sprites.rs`, `tables.rs`, `control.rs`, `loading.rs`; `lib.rs` prod is 211 lines (rest is tests). 117 tests green. New growth point: `tables.rs` (~1238 prod lines) — split order/song-wheel/shop lookups if it keeps growing. |
| o2jam-game (2370 across 4 modules) | 10 | Deep pass landed (+1): `chart.rs`, `lane_effect.rs`, `peer.rs` beside `lib.rs`; small fns, zero production panics, 68 tests. |
| o2jam-net (11664 across 12 modules) | 8 | Naming DONE (`CreatedRoom`); zero production `unwrap`/`expect` (framing/timeout map to `Error`, `client.rs` uses `map_err(Error::transport)`). But `message.rs` 1638 → 4251 (prod 2537): gift/shop/wallet/channel families keep landing in the catch-all. Fan-out is the fix. (→ same score: handling up, shape down) |
| o2jam-render (1245, one file) | 9 | Prod is 599 lines, longest fn 36 (`letter_glyph` table), 35 tests (+1). Single-file still fits; split glyph tables vs blits vs scene only past ~800 prod lines. |
| o2jam-shell (6638 across 8 files) | 9 | `Handler` extracted (`window.rs`, `run()` is 10 lines); `screens/` split into `paint`/`game`/`side`. Longest prod fn 35 lines. One production `expect` remains (`window.rs:96`, window creation at startup — unreturnable there, accepted with rationale). Size tripled but shape held. (→ same score) |
| recomp2jam (main 814, connect 1412) | 8 | No production `unwrap`/`expect`; `connect.rs` flow fns small (longest 51). Still: `main.rs` has zero tests, `run_command` is a 126-line dispatch (extract `build_song_flow`), and `connect.rs` still mixes login flow with avatar file IO (`fetch_avatar`, `build_worn_figure`). (→ same score, sharper notes) |
| o2jam-dump (123, new row) | 9 | Thin CLI over tested library fns: small `run_*` fns, shared `read_input`, `anyhow` errors. No tests of its own — a `run_hex` golden test is the fix if touched. |

## Fixed since the prior pass

- `o2jam-file` split by format (backlog #1 DONE): god-file → 8 modules,
  `lib.rs` prod down to 211 lines.
- `o2jam-net` naming consolidation (backlog #3 DONE): `RoomInfo` →
  `CreatedRoom`; `RoomListEntry`/`JoinedRoom` kept with doc comments
  recording the wire names.
- Production `unwrap`/`expect` eliminated repo-wide except one startup
  `expect` (`window.rs:96`): audio mutex → `try_lock`/`map_or`, net
  framing/timeout → `Error`, verified by `#[cfg(test)]`-stripped sweep.
- Older fixes, kept for the record: `stretch_blit` + present-path rework
  (every window size shows the whole frame); stale module docs (audio,
  dump, file OJN-stub); string encoder truncates including the
  terminator; `LaneMap` zero-width early return; `SongClock` saturating
  `try_into().unwrap_or(MAX)`; dump dispatch `run_*` split.

## Deep-read fixes (this pass)

- `LaneMap::shuffled_narrow`: zero width divided by zero — early identity
  return plus regression test (only panic-on-input found in the sweep).
- `SongClock::now_ms` + shell clock: `as u64` → `try_into().unwrap_or(MAX)`
  with saturating add (one shared idiom).
- Dump dispatch split into `run_*` fns with a shared `read_input`.

## Backlog (priority order)

1. ~~Split `o2jam-file/lib.rs` by format~~ DONE (8 modules). Follow-up:
   split `tables.rs` (~1238 prod lines) into order/song-wheel/shop
   lookups if it keeps growing.
2. ~~Extract shell `Handler` out of `Shell::run`~~ DONE (`window.rs`;
   `run()` is 10 lines). ~~Split `screens.rs` per screen~~ DONE
   (`paint.rs` + `game.rs` + `side.rs`; tests moved alongside).
3. ~~`message.rs`/`rooms.rs` naming consolidation~~ DONE (`RoomInfo` →
   `CreatedRoom`; `RoomListEntry`/`JoinedRoom` kept).
4. NEW: fan gift/shop/wallet/channel families out of
   `o2jam-net/message.rs` (prod 2537 lines) into topical modules, the
   way `rooms`/`play`/`character`/`relay` already work. Small fns
   throughout (longest 57), so this is organization, not rescue.
5. NEW: `recomp2jam` test gap — `main.rs` has zero tests. Extract pure
   builders (`build_song_flow`, `ConnectOptions` assembly) and
   golden-test `print_connect_report`; separate avatar file IO
   (`fetch_avatar`, `build_worn_figure`) from the login flow in
   `connect.rs`. Shrink the 126-line `run_command` dispatch via the
   extraction.
6. Pedantic polish: DONE mechanical classes (`must_use` ×13, infallible
   `From` ×4, or-pattern merges, `is_some_and`/`map_or`, elided lifetimes,
   `LivePlay::new` takes a slice, explicit enum imports, reasoned `ignore`,
   `Ipv4Addr::LOCALHOST`, ctor field order, `u8::from(!male)`-free parity
   test, per-layer colorkeys); `# Errors` docs DONE (zero `missing_errors_doc` remaining).
   ACCEPTED with rationale (not churned): protocol-bounded `as` casts
   (target-dependent `usize` widths have no `From`; values are
   length-checked), `assert!(is_empty())` over `assert_eq!(_, [])`,
   prose-word `doc_markdown` hits, test decimal literals, `Error::transport`
   by value (point-free `map_err`), wire-mandated bool fields,
   `window.rs:96` startup `expect` (unreturnable, like audio's old
   mutex `expect`). Tiny polish left: audio `use std::sync::{Arc, Mutex}`.
7. ~~Overlong functions~~ DONE (`fn main`, dump dispatch). `run_autoplay` (74) kept: single level, tested. (`fn main` 194 → dispatch + `run_command`/`run_info`/`run_play` + setup helpers; `too_many_lines` clean).

## Regression evidence (2026-11-06 refresh)

- `cargo test --workspace`: 803 passed, 0 failed, 1 ignored (audio: 1
  ignored hardware test) — per suite: audio 19, file 117, game 68,
  net 325, render 35, shell 176, shell `real_art` 7, recomp2jam
  integration 22, misc 34. All doc-tests pass.
- `cargo fmt --check`: clean. `cargo clippy --workspace --all-targets
  -- -D warnings`: clean.

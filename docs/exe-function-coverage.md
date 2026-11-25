# Exe function-coverage ledger

First function-level coverage figure for the shipped client. It answers one
question: of the exe's procedures, how many are game behavior, and how many
of those already appear in repo evidence?

Version pin: `OTwo.exe` 7.55, 5,029,888 B,
sha256=`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
Provider: REA workflow 1 over Ghidra 12.2 (REA 6.10.1 sources). The Ghidra
program references a PDB but no PDB companion was admitted, so recovered names
are provider observations, not original source names. Base commit:
`3c2e47f093e882306fde4c10c6b5b31b1e7b5cad`.

## Headline figures

| bucket | procedures | share |
|---|---:|---:|
| total | 11,451 | 100% |
| library / CRT / imported | 5,215 | 45.5% |
| game-reachable | 1,129 | 9.9% |
| dead-or-unclassified | 5,107 | 44.6% |

**Pinned of game-reachable: 232 / 1,129 = 20.5%.**

Repo pins in the other buckets: 87 library, 514 dead-or-unclassified.

## Method

1. `binary_overview` reported 11,451 procedures. `list_procedures` retained the
   full inventory; all 115 pages were read with `inspect_analysis_view`
   (`native-inventory`, page size 100). No page failed; no sampling was used.
   Evidence start `ev6fb4f8cdb7c38fc0757e03d87ac32d5b96c20fe852c2d38cd7a083ba77aba1b6`,
   inventory parent `ev602acb5b0a916e88a734d4ced848c27267ed3d256bf3f48fbe95e7970b4a2bcf`.
2. Forward call graphs were read from 21 game dispatch seeds: the WndProc chain
   (`0x5ac690`, `0x5a5190`, `0x5a50d0`, `0x479cb0`), the state install switch
   (`0x477990`), the game-TCP dispatcher (`0x558080`), documented staging
   loaders and appliers (`0x4e7850`, `0x48b2d0`, `0x50d060`, `0x4793a0`,
   `0x4ad350`, `0x4bb2c0`), documented packet handlers, and the function
   entries containing repo-pinned vtable `+0x5c` send sites. Graph queries used
   `get_call_graph` (direction `forward`) plus `native-observation` paging;
   depth reached level 15. Two more forward graphs from CRT startup
   (`0x5fb1d8`, `0x5fb3b8`) supplied the startup-only rule below.
3. Each procedure got exactly one bucket, using the first rule that fired.
   Reachable graph nodes that matched a library rule stayed library.
4. Coverage scanned `docs/` and `crates/` (`.md`, `.rs`, `.txt`) for `FUN_0x` /
   `FUN_00` procedure names and bare six-digit `0x` addresses. Any hit pinned
   the procedure. A named function in a doc or guard comment counts as pinned;
   a test does not need to be green for this count.

Full per-procedure results, including the rule that fired and pin files, are in
`/home/arnim/scratch/lanes-2026-11-25/procedures.csv`. The query log and counts
are in `findings-coverage.md` in the same directory.

## Bucket rules

Rules ran in this order; counts are per rule.

| rule | count | meaning |
|---|---:|---|
| `library:external-import` | 347 | Import address, DLL function |
| `library:thunk` | 108 | Thunk entry, including thunks to CRT helpers |
| `library:name-msvc-crt` | 521 | Leading underscore MSVC runtime name |
| `library:name-msvc-eh` | 221 | C++ exception or frame-handler name |
| `library:name-cpp-stl` | 136 | STL, ATL, concurrency, or library-class name |
| `library:name-mfc` | 22 | MFC class name |
| `library:name-crt-helper` | 27 | Other named CRT helper |
| `library:crt-region-0x5f0000plus` | 3,055 | Address at or above `0x5f0000`, where named neighbors are CRT and STL code |
| `library:crt-startup-chain-only` | 778 | In the CRT startup graph but in no game dispatch graph |
| `library:only-library-callers` | 0 | Closure rule ran; no new procedure qualified |
| `game:dispatch-closure` | 1,129 | In a game dispatch forward graph and not library |
| `dead:no-observed-caller` | 5,107 | No observed caller in either graph |

The "only from other library functions" closure was computed from all observed
call edges; it added nothing beyond the name, region, import, thunk and startup
rules.

The dead bucket is **dead-or-unclassified**, not proven dead code. It contains
mainly procedures with no observed caller in these bounded traversals, plus
code that can only be entered by unresolved indirect calls. The Lanking UI
surface is documented as inert in 7.55: it has no installed state class and its
sort and pager control words have no literal handler. Its pinned reply handler
(`FUN_0x55b230`) folds ranking rows and is counted game-reachable through the
TCP dispatcher. No other Lanking procedure was assigned to the dead bucket
without a per-procedure pin.

## Confidence limits

- The call graph is provider-observed, not a whole-program CFG. Every seed
  graph reported call references without resolved destinations; the state
  install graph alone reported 13. Of 6,662 game graph node rows, 486 had
  partial relationship coverage.
- Indirect calls are unresolved. Vtable `+0x5c` send users and ControlList
  handlers were seeded from repo-pinned sites, but a vtable slot can reach code
  that static call refs do not show. The game-reachable figure is a lower
  bound; the dead bucket is an upper bound.
- The CRT region rule uses a name-neighborhood fingerprint at `0x5f0000` and
  above. Unnamed library code below that address may sit in the dead bucket.
- Inventory rows do not include provider body sizes. The CSV stores
  `address_gap_to_next` as an upper bound only; it is not a function size.
- Pin matching is textual. Of 964 bare-address hits on game-reachable
  procedures, 127 occur near the words "file" or "offset", where a six-digit
  file offset can coincide with a six-digit VA. The headline count follows the
  literal bare-address rule, so it is an upper bound on independently pinned
  behavior. A mention is not a passing test or a runtime verification.

## How to read this

- Use the four headline figures for planning scale, not for correctness claims.
- A *library* procedure is support code. It can be called by game code; that
  does not make its behavior game behavior under this ledger.
- A *game-reachable* procedure has an observed path from a named game dispatch
  surface. It may still be unpinned: 897 of the 1,129 are unpinned and are the
  main research backlog.
- A *pinned* procedure is only named in repo text. Check the cited evidence in
  `docs/` before treating the name as a behavioral guarantee.
- A *dead-or-unclassified* procedure needs a new caller pin or an indirect-call
  resolution before it can be promoted. Do not read it as safe to delete.
- For a specific procedure, open `procedures.csv` and read its `bucket`, `rule`,
  `pin_hits` and `pin_files` columns. For how a count was produced, read
  `findings-coverage.md` and the JSONL query logs beside it.

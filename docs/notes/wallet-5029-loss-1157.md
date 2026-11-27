# #1157: a 5029 wallet reply is dropped by the lobby drain

Status: **DIAGNOSED, not fixed.** The fix belongs in
`OnlineSession::apply_lobby_frame` and is re-sliced from here.

Evidence class: clean-room Rust trace logs and probe counts from the
live wallet leg. No original bytes were read for this note.

## The symptom

`live_wallet_digits_yield_to_the_message_sheet_from_boot` fails with
`wallet wallet refresh never folded within 10s` at
`crates/recomp2jam/src/live_input_legs.rs:2813`. It is full-suite only in
practice, at about 1 full run in 8, locked or unlocked, and almost never
when the leg runs alone.

## The losing interleaving

Two instrumented legs, one red and one green, with tracing at DEBUG.
Times are `tv_nsec` on the leg's own clock base; only the differences
matter.

| event | red | green |
|---|---|---|
| BACK clicked (5007 and 5028 queued) | 924028584 | 273175384 |
| gateway saw 5028 | 924105990 | 273266888 |
| gateway wrote 5029 | 926212398 | 277372430 |
| BACK paint done, wallet | all `None` | all `None` |
| priming paint done, wallet | all `None` | `gem: Some(15527)` |
| session trace | `skipping non-chat frame in lobby drain id=5029` | no such line |

In the BACK paint, `poll_wire_at` sends 5007 and 5028, and
`poll_wallet_answer` takes its 1 ms drain **before** the gateway has
written 5029. The reply then lands while `poll_lobby_chat` owns the
socket for its own 1 ms drain. `apply_lobby_frame` has no 5029 arm, so
its default arm logs `skipping non-chat frame in lobby drain` and
**drops the frame**. Nothing defers or retries it. `paint_until_wallet`
begins only after that paint, its signal has already been sent, and it
polls an empty socket to its deadline.

In the green leg the reply arrives about 4 ms later and lands in the
*next* paint's wallet drain, so it folds.

## Loss site

`OnlineSession::apply_lobby_frame`, default arm, in
`crates/recomp2jam/src/session.rs`. Reached from `poll_lobby_chat` inside
`poll_wire_at`, after `poll_wallet_answer` in the same paint.

## Probe counts

Method: run the wallet leg binary `--exact` many times, either beside
noise processes or as a group in contention, with an optional reply
delay injected into the test gateway. One command is red when any of its
legs reports the symptom. A one-trial sweep is not evidence and is not
listed.

| slice | commands red |
|---|---|
| 1 wallet leg beside 4 `session::` noise processes | 0 / 10 |
| 1 wallet leg beside 4 `live_` noise processes | 0 / 10 |
| 20 wallet legs in contention, injected delay 1 to 8 ms | 10 / 10 |
| 20 wallet legs in contention, **no** injected delay | 3 / 10 (3 red legs of 200) |
| 1 leg, injected delay 1 ms | 0 / 10 |
| 1 leg, injected delay 2 ms | 10 / 10 |
| 1 leg, injected delay 3 ms | 10 / 10 |
| 1 leg, injected delay 4 to 8 ms | 0 / 10 |

Two conclusions follow.

- The race is **shipped**. It goes red with no injected delay once
  enough legs contend. The delay only amplifies it.
- The losing window is narrow and sits at about 2 to 3 ms after the
  5028: the reply must arrive after the wallet drain has given up and
  before the lobby drain has finished. That is a scheduling window, not
  fixed timeout arithmetic.

## What a fix must satisfy

- The 5029 must fold when it arrives in any drain on the Room or
  Channel screen, not only in the wallet drain.
- A regression guard must go red on the interleaving above and green
  with the cause removed.
- No change to the leg's deadline, to any lock, to any skip, or to test
  order. Three green full runs are not a fix; the leg was red before
  and the rate is known.

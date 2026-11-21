# Live-path reachability bar (tests = features)

A guard pins a behavior only when the live user path reaches the guarded
code, or the guard states why the seam is the whole behavior. Seam-green
alone is never evidence that the feature works.

Owner principle: **tests = features**. Retro 2026-11-13 (#191/#192/#193):
three player-visible tickets closed on green seam guards while the features
stayed broken live. The guards passed; the features did not.

## Tiers and where each stops

| Tier | Drives | Cannot see |
|---|---|---|
| 0 Seam unit | `ScreenGame` directly | `main.rs`, `session.rs`, install-root reads, window, real server |
| 1 Window-input seam | extracted arm helper vs `ScreenGame` | `EventLoop`, OS input |
| 2 Scripted gateway | `connect`/`session` round trips (ADR-0014) | real server, install-root, window |
| 3 Headless live-path | `live_boot_gate`, `session_walk`, `live_stats` | real input, real display |
| 4 Owner playtest | real window + server | automation |

Tier 0 is where #191/#192/#193 failed: the seam guard called the setter
directly, so it never asked whether the default boot reached it.

- Tier 0 guards prove logic at the seam, not reachability. A `ScreenGame`
  constructed in the test bypasses `main.rs::run_play` and
  `session::OnlineSession`.
- Tier 2 is the TDD loop (ADR-0014): scripted gateway first, live confirm
  second. It still does not read installed archives or drive a window.
- Tier 3 must skip cleanly when `install-root` is absent (CI has no game
  data). A skip is a skip, never a pass.
- Tier 4 is the last leg for input and display questions.

## Required annotation for every behavior guard

Every guard named under `## Guards` in a closing comment carries one of:

1. **Live chain** — a caller chain from a user entry to the guarded symbol,
   using real names, for example:

   `main.rs::run_play` → `setup_online` → `connect::connect_and_login_with_transport`
   → `stage_online_identity` → `ScreenGame::set_avatar_source`
   → `ShopState::set_catalog`

   The chain starts at a user entry (`main.rs::run_play`, the `shots` walk,
   `session_walk::drive_walk`, or a `live_boot_gate` test) and ends at the
   guarded symbol.

2. **Seam suffices** — `seam: <reason>`, with one of these reasons only:

   - pure parser, codec, or math with no runtime caller beyond the seam;
   - behavior defined entirely by a seam contract (ADR or claim cites it);
   - the code cannot be reached by any live path by design.

A guard with neither annotation is a seam-only guard. It cannot close a
player-visible ticket.

## Player-visible close leg

A player-visible ticket closes only after one of:

- owner playtest;
- a headless live-path walk (`live_boot_gate`, `session_walk`, `live_stats`);
- a scripted-gateway round trip through the real boot entry.

Seam guards alone never close such a ticket. Docs and presentational fixes
use `(none -- docs: <why>)` or `(none -- presentational: <why>)` and skip
this leg.

## Live-input legs (ticket #739)

The fast scripted gateway guards stay in `session.rs`; one live leg per
player-visible family now sits beside them in
`crates/recomp2jam/src/live_input_legs.rs`. Each leg boots through
`connect::connect_and_login_with_transport` and
`assemble_online_session`, drives Menu/pointer snapshots through
`GameLoop::ingest_snapshot` (no direct `menu_key` or staged scene
load), and asserts the wire opcode on a recording gateway. Staged
button points come from the parsed `.bnd` placements; the two lobby
faces keep their pinned e2e coordinates. Missing `install-root` skips
the leg cleanly.

## Relay notice wire bracket (ticket #696)

`relay_notice_ok_click_comes_after_the_1003_answer_with_no_wire_gap`
is a tier-2 scripted-gateway guard. Live chain:
`main.rs::run_online` → `connect::connect_and_login_with_transport` →
`stage_channel_list_after_relay_gate` → `enter_screen(Channel)` →
`open_relay_down_dialog` → OK at (400,365) →
`dismiss_relay_down_dialog`. The gateway holds the socket through the
click, records the 1003 answer instant, and proves no later frame
arrives between that answer and the OK click. It pins the bracket
(answer < sheet < OK) from relay pin `64caf0c4…8ebd629bbec7208`
without inventing an exe timeout.

## Solo Result BACK from the settled panel (ticket #683)

`settled_solo_result_back_click_posts_4021_and_takes_the_4022_answer`
is a tier-2 scripted-gateway guard. Live chain:
`main.rs::run_play` → `OnlineSession::ingest_snapshot` →
`ScreenGame::click_result` (rect 5, (752,581)) →
`take_result_back` → `flush_result_wire` → `C2S 4021` → `S2C 4022`
→ Room. The qualifying settled panel comes from
`~/scratch/p366-solo/s3-result-timed` (frame `3ea6c481…`, miss 20,
gem 15527, rank F) and is painted before the click. The clicked-cause
frame itself stays OPEN: boot 3 clicked the mid-animation panel and
boot 4 never reached Result. Next capture: one qualified frame-clock
pack with the settled SCORE panel immediately before the rect-5 click
and the 4021/4022 tap in the same manifest.

| Family | Guard |
|---|---|
| scene BACK/exits (`#734` class) | `live_click_waiting_back_sends_the_room_leave_from_boot` |
| dialog OK/dismiss | `live_click_dialog_ok_releases_the_held_channel_rows` |
| Waiting join/ready/start/back | `live_click_guest_ready_then_back_from_boot`, `live_click_master_start_then_back_from_boot` |
| Messenger host P0 (#631, tier-1) | `live_room_messenger_press_reaches_its_labeled_p0_from_boot` |

`live_room_messenger_press_reaches_its_labeled_p0_from_boot` boots through
`connect::connect_and_login_with_transport`, walks to the Room board through
pointer snapshots only, then presses the staged `IDC_BUTTON_SROOM_MESSENGER`
rect by its parsed code `0x13200048`. The chain ends at
`ScreenGame::click_room_control` → `RoomControlAction::Unwired`; the guard
asserts the labeled P0 record, no side screen or liveness sheet, and no wire
frame. This is one live caller chain into the Messenger family, not a
exe mount pin: the four unpinned hosts stay `N-HOST` (#646/#648/#650/#652)
and the master arm stays `N-MASTER` (#644).

## Worked example: #191, #192, #193

- #191 — seam guard `shop_board_stages_goods_from_avatar_source` called
  `set_avatar_source` directly and passed. The item shop stayed a static
  page live. The guard never named the live chain
  `main.rs::run_play` → `setup_online` → `stage_online_identity` →
  `ScreenGame::set_avatar_source`, so nobody checked whether the default
  boot reached the staging call. Reopened within the hour; the actual caller
  gap was one level above the seam.
- #192 — same pattern for the music shop.
- #193 — the clock guard for the avatar breathing frame passed while the
  room avatar stayed static live. It pinned the accumulator seam, not the
  live paint path.

In all three, the suite was green and the feature was broken. The fix was
not a stronger seam test; it was tracing the chain.

## Template for a new guard

```rust
#[test]
fn <scenario_name>() {
    // Live caller chain (or seam-suffices):
    //   main.rs::run_play -> ... -> ScreenGame::<symbol>
    // (or) seam: <reason>
    // Archives absent -> skip cleanly (CI has no game data).
    ...
}
```

Closing comment:

```
## Guards
`<guard_name>` — live chain: `main.rs::run_play` → ... → `ScreenGame::<symbol>`
`<guard_name>` — seam: <reason>
```

## Red-proof

A guard name closes a fix only with its PASS line and its FAIL line in the
same closing report: break the fix, watch it fail, restore the fix, watch it
pass. The reviewer re-runs both. A guard with no FAIL line is hollow and does
not close the fix.

## Seam-suffices is not a loophole

Do not mark `seam:` to avoid tracing. The reviewer checks the reason against
the tier table above. If a live caller exists, name it.

## Reopen cheaply

If a guard passes and the feature is broken live, reopen and trace the chain.
No defensiveness. Reopening is the correct move; the senior reopened #191,
#192, and #193 within the hour.

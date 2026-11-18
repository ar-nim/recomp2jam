# Journey audit — #297 Spec: Payment2 state

Base: `2105ac6`. Reused evidence only; no exe boot.

## Evidence anchors (artifact + hash)

| Id | Artifact | Hash / provenance |
|---|---|---|
| `B8` | `~/scratch/side-scenes/b8-cash/`, exe 7.55 (`83a04541…72c823b`): the only qualified billing-window capture in the reused packs is the Item Shop `Cash Charge` page; no gift page is visited | manifest sha256 `f9d4a6953c55b75b4f04db3c0ece198bd2032361ac65dc8275ce07ccb6aa3f8b`; `cash-charge.png` sha256 `27fd5435fbe64c96315fcb29f0947bfc02d794b318c8a3047dccf15d176be671` |
| `STATIC` | `docs/parity/payment.md`, `docs/net-protocol.md §2` (`payment_input.aspx` vs `gift_input.aspx`, files `0x2466d0`–`0x24a2d0`), `side.rs:3981-4024` labels | repo files at `2105ac6` |

Our journey: boot stages `State_Payment2.bnd` (`live_art.rs:150`), the
`SideScreen::Payment` visit merges both views (`side.rs:228`), the notice names
both pages (`side.rs:4046`), and `payment_entry_queues_no_wire_call` forbids a
success claim. The exe's second billing page is named only by static HTTP pins;
no reused pack reaches it.

## Summary matrix

| Verdict | Count | Stories |
|---|---:|---|
| MATCHED | 2 | 2, 3 |
| MISMATCHED | 0 | — |
| BLOCKED | 4 | 1, 4, 5, 6 |
| NA-DESIGN | 0 | — |

New issues: none for #297. The visible web-window frame divergence is already
filed as #362 under #296 and applies to both billing views.

Mapped, not filed: #345 liveness (black web client area), #245/#249 (missing
gift-page capture and billing gateway trace).

BLOCKED for live lane: completion/failure exchange, qualified gift-page side
visit, gateway zero-completion-frame trace, gift 12-rect page pin.

## Per-story table

| # | Story (short) | Comparator | Our journey ref | Exe evidence | Verdict | Notes |
|---|---|---|---|---|---|---|
| 1 | Confirmation completes/fails per exe | wire-sequence | no completion send; `payment_entry_queues_no_wire_call` | `B8` has no billing completion exchange and `wire_tap=0` | BLOCKED | No exe completion/failure frame exists in reused evidence; needs a gateway replay. |
| 2 | Failure paths shown honestly | data-effect | staged view inks the unwired notice and both page names; no success claim | `B8` `cash-charge.png` shows a web window with a black client area in the rig — no success readout | MATCHED | Both journeys avoid a false success. Pixel/chrome divergence is recorded under #362, not a separate issue here. |
| 3 | No success before a pinned completion | data-effect / negative | `payment_entry_queues_no_wire_call` plus notice text (`side.rs:4040`) | No completion pin in `STATIC` or `B8` | MATCHED | Negative invariant holds; the exe leg stays OPEN, so no success state is invented. |
| 4 | Payment2 is a separate gift page, not a confirmation step | state-path | `side.rs:3994` `payment_labels()` distinguishes the two pages, but both merge into one `SideScreen::Payment` | Only `payment_input.aspx`-side chrome captured (`B8`); `gift_input.aspx` is a static name | BLOCKED | Need one qualified gift-page visit to confirm the separate-page journey. |
| 5 | Negative guard without pinned completion | wire-sequence | guard exists; no gateway trace | `B8` `wire_tap=0` | BLOCKED | Need a gateway trace with zero completion frames plus the notice pixel. |
| 6 | 12-rect bound layout pinned to gift page | state-path / pixel-region | `staging/tests.rs:415-442` stages both sheets from real block lines; notice lands in the largest rect | `STATIC` names 12 bound placements; no gift-page capture to map them to | BLOCKED | Staging alone is not the page mapping; needs the gift-page capture from story 4. |

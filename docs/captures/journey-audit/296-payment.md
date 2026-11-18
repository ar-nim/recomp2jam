# Journey audit — #296 Spec: Payment state

Base: `2105ac6`. Reused evidence only; no exe boot.

## Evidence anchors (artifact + hash)

| Id | Artifact | Hash / provenance |
|---|---|---|
| `B8` | `~/scratch/side-scenes/b8-cash/`, exe 7.55 (`83a04541…72c823b`); `artifacts/input.log` clicks Item Shop cash-charge plate `(92,355)`; `cash-charge.png` is a titled `Cash Charge` web window with an X and a black client area over Item Shop; `itemshop.png` shows the shop wallet readout | manifest sha256 `f9d4a6953c55b75b4f04db3c0ece198bd2032361ac65dc8275ce07ccb6aa3f8b`; `cash-charge.png` sha256 `27fd5435fbe64c96315fcb29f0947bfc02d794b318c8a3047dccf15d176be671`; `itemshop.png` sha256 `0c561c6648fe29aaef1b64ca867de05baef9fae7ab12afbcd9c1a42652293ee8` |
| `R-PAY` | `docs/screenshots/payment.png` our generated shot: blue panel with `BILLING UNWIRED payment_input.aspx gift_input.aspx` and no web-window chrome | sha256 `e913c53f08470d85a624bcfa5facdb97b58bf2d55afd5479bdc0911dfe113ac2` |
| `STATIC` | `docs/parity/payment.md`, `docs/net-protocol.md §2`, `docs/smoke/payment-tutorial-hand-session.md`, `docs/captures/t240/wine-citation.txt` (pointer, not a side-state capture) | repo files at `2105ac6` |

Our journey: boot stages both billing sheets (`live_art.rs:150` →
`game/mod.rs:11897-11957`) → `visit_side(Payment)` → painter inks the notice
into the largest staged rect (`side.rs:4040`, `payment_side_notice`) →
`payment_entry_queues_no_wire_call` negative guard. The exe journey in `B8`
reaches a titled web window from the Item Shop cash-charge plate; no purchase
round trip happens on that view.

## Summary matrix

| Verdict | Count | Stories |
|---|---:|---|
| MATCHED | 2 | 1, 2 |
| MISMATCHED | 2 | 3, 5 |
| BLOCKED | 2 | 4, 6 |

New issue: #362 (Payment web view shape and rect differ from the exe Cash
Charge window). The divergence is novel: no OPEN blocker ticket owns the
display-only web-window chrome, and the honest no-balance/no-success rule is
kept in the issue.

Mapped, not filed: #345 liveness note for the black web client area; #245/#249
for the missing host/session gateway trace.

BLOCKED for live lane: billing HTTP host/session replay; staged-rect pin to the
billing page; live gateway zero-5000-frame trace.

## Per-story table

| # | Story (short) | Comparator | Our journey ref | Exe evidence | Verdict | Notes |
|---|---|---|---|---|---|---|
| 1 | No wallet balance invented | data-effect | `payment_side_notice` paints only the notice and page names; `side.rs:4055` wallet rows feed boards, not the Payment view | `B8` `cash-charge.png` has no cash/gem readout inside the web window; `itemshop.png` shows balances on the shop, a different host | MATCHED | Both journeys show no balance on the billing view. The spec's old "balance" story stays ASSUMED but no divergence is observed. |
| 2 | Web-billing side visit, not purchase continuation | state-path | `SideScreen::Payment` side visit + `payment_entry_queues_no_wire_call` (`game/mod.rs:11942`) | `B8` input log opens the web window from the Item Shop charge plate; no 5000-family exchange on that view | MATCHED | Both are display-only side visits; the old purchase-continuation claim stays REFUTED. |
| 3 | Wired per exe, display-only chrome documented | pixel-region | `R-PAY` shows a blue notice panel with no title bar/X; `docs/parity/payment.md` Composition row calls this PASS on a static pin | `B8` `cash-charge.png` shows a `Cash Charge` title bar, an X close and a black web client area | MISMATCHED | Named divergence: our display-only stand-in is text on the staged rect, not the exe's web-window frame. Novel issue #362; the checked spec box was static-pin-only. |
| 4 | Page names and hosts pinned to HTTP lines | wire-sequence | `side.rs:3981-4024` names `payment_input.aspx`/`gift_input.aspx` and URL paths; `net-protocol.md §2` gives file offsets `0x2466d0`–`0x24a2d0` | `B8` has `wire_tap=0`; no HTTP request line captured | BLOCKED | Need one gateway replay naming host and session join. |
| 5 | Rect-to-page mapping pinned | pixel-region / state-path | `side.rs:228` merges `State_Payment` + `State_Payment2` placements; notice goes in the largest rect; `R-PAY` hash above | `B8` `cash-charge.png` puts the web window at its own chrome rect over the shop board | MISMATCHED | Named divergence: our staged rect/panel geometry and the exe's web-window rect are not 1:1. Folded into #362. |
| 6 | No-wire negative pinned against live gateway | wire-sequence | `payment_entry_queues_no_wire_call` guard | `B8` `wire_tap=0` (no gateway trace in the pack) | BLOCKED | Need a gateway trace with zero 5000-family frames on entry; #245 class. |

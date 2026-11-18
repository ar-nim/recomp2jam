## `STATE_PAYMENT` — billing views (`STATE_PAYMENT`/`STATE_PAYMENT2`; `CDPayment` web billing)

### b8 pin — `EV-B8-CASH-FRAME` (issue #362)

Exe 7.55 `OTwo.exe` sha256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`; qualified pack `~/scratch/side-scenes/b8-cash/` manifest sha256 `f9d4a6953c55b75b4f04db3c0ece198bd2032361ac65dc8275ce07ccb6aa3f8b`; `artifacts/screenshots/cash-charge.png` sha256 `27fd5435fbe64c96315fcb29f0947bfc02d794b318c8a3047dccf15d176be671`; host `artifacts/screenshots/itemshop.png` sha256 `0c561c6648fe29aaef1b64ca867de05baef9fae7ab12afbcd9c1a42652293ee8`. `artifacts/input.log` clicks the Item Shop charge plate at native `(92,355)`. Only rects and control codes are recorded here; no game bytes enter the repo.

`ControlList_Interface.txt:1688-1694` mounts `DIALOG_CHARGECASH` → `Dialog_ChargeCash.bnd`. Rects pair by code ordinal `(code & 0xFFF) - 1`; children are parent-relative to the frame origin:

| Control | Native rect (parent-relative → absolute) | Image check |
|---|---|---|
| `IDC_BACK_CHARGECASH` `0x83100001`, ordinal 1, `Dialog_ChargeCash.ojs` anchor `(171,3)` | `(171,3)-(648,595)` | titled frame with X, over the shop board |
| `IDC_IMAGE_CHARGECASH_WEB` `0x83500002`, ordinal 2 | `(19,55)-(459,561)` → `(190,58)-(630,564)` | `cash-charge.png` has exactly RGB `(25,24,25)` on x 190..629, y 58..563 (440x506); the exe RGB555 body decodes one step lower |
| `IDC_BUTTON_CHARGECASH_CLOSE` `0x83200003`, ordinal 3, `ChargeCash_Btn_Close.ojs` anchor `(400,2)` | `(400,2)-(448,38)` → `(571,5)-(619,41)` | black X at the frame's upper right |
| Host `IDC_BUTTON_SSHOP_CHARGEMONEY` `0x14200004`, `State_ItemShop.bnd` rect 4 | `(56,340)-(147,364)` | contains the b8 click `(92,355)`; host board is `State_ItemShop.bnd` rect 1 `(0,0)-(800,600)` |

### Tier-4 live-click re-check (`EV-B8-CASH-FRAME`, reopened #362)

The reopened negative (`~/scratch/live-4/cash-rust-final/pack`, web-client MAE
114.065) was a capture-script artifact: the scratch driver copied its own
`itemshop-rust.png` into `cash-charge-rust.png` (latest mtime; no new frame
dump after the click), so the compared pixels were the **pre-click** shop
board. In the corrected live run `~/scratch/p362b-live/run2`, the root
capture `rootshots/cash-root.png` shows the pinned `Cash Charge` chrome over
the shop board after the same `(92,355)` click; its native crop
`artifacts/screenshots/cash-charge-rust-root.png` (sha256
`d04024813b799dbcde6c8c3c10c442d9b706ba86e1461f963ef480541389fbc9`) has MAE
2.042 in the frame `(171,3)-(648,595)` and 0.667 in the web client
`(190,58)-(630,564)` versus b8; title MAE stays 17.747 where the unwired
notice inks the title band. Both live packs record the same post-click
backbuffer content hash `e10aa2fdad5e8b0d`, which the headless live-path guard
reproduces exactly. No production remount change was needed: the click path
already mounts the shop host art and paints the staged dialog; the new
guard `live_boot_charge_plate_snapshot_click_paints_chrome`
drives the input snapshot through `OnlineSession` and fails when the dialog
paint is removed.

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | `CDPayment` web billing views; the Item Shop charge plate opens the `EV-B8-CASH-FRAME` chrome | (ev: `EV-B8-CASH-FRAME`) Live boot stages `Dialog_ChargeCash.bnd` with the Item Shop host board; the plate click leaves the shop and opens Payment, whose painter draws the staged frame and X close at the pinned rects. The unwired notice inks inside the staged web client; no balance or success is shown. Guards: `cash_charge_sheet_stages_the_b8_frame_client_and_close`, `itemshop_charge_plate_at_b8_click_opens_cash_charge`, `live_boot_itemshop_charge_plate_paints_cash_charge_chrome` (live chain boot → plate → Payment → painter), `live_boot_charge_plate_snapshot_click_paints_chrome` (live snapshot click → paint), `payment_entry_queues_no_wire_call`, `payment_side_without_staged_rects_paints_no_notice` (ADR-0003) | (R) PASS (v: cb2/tt-362) |
| Input | Billing entry and the web bridge | (ev: `EV-B8-CASH-FRAME`, docs/smoke/payment-tutorial-hand-session.md) Charge plate and Payment entry queue no wire call on any send seam (guards `payment_entry_queues_no_wire_call`, `live_boot_itemshop_charge_plate_paints_cash_charge_chrome`). OPEN: the live web bridge and gateway replay naming host/session join; next step: gateway replay through the billing endpoints | (R) OPEN (v: cb2/tt-362) |
| Timing | None documented | n/a | (R) N/A (no timed behavior — no dispatch exists to guard; next step: none) (v: 9692d41) |
| Transitions | Lobby ↔ Payment/Payment2 via the side visit | (ev: docs/smoke/payment-tutorial-hand-session.md) The hand session visits both billing views and returns (trace `fixtures/input-trace/payment-tutorial-hand-session.json`). Guards: `payment_tutorial_hand_session_visits_both_and_returns`, `flow_allows_every_documented_edge` | (R) PASS (v: 9692d41) |

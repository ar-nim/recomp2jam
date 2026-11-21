# Payment story matrix — Spec #296 (WAVE 5, payment lane)

Method: `docs/captures/story-matrix/TEMPLATE.md`. Base `f1987b5`; worktree
`/home/arnim/Projects/recomp2jam-wt144`, branch `cb2/story-payment`. No code
change, no push. The 7.55 executable is the oracle (`OTwo.exe`, 5,029,888 B,
SHA-256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`).
Memoryer is a wire witness, never an exe rule. All packs stay in git-ignored
scratch; only hashes and text enter the repo.

Actor mapping for this side screen: `master` is the local account holder who
clicks the Item Shop charge plate (b8's `tester` session); `solo` is the
offline/stand-in seam, which cannot make a 7.55 billing claim; `peer` and
`channel` are other players and board observers (a local modal sheet is not
visible to them); `server` names a protocol frame with no local click. The
billing HTTP host is an environment actor and is graded `NA-DESIGN` where its
content is concerned: the host is dead and page presentation is the owner's
decision, not a live exe claim to chase.

## 1. Evidence inventory (written before any cell was graded)

| Source | Pin (full sha256 unless noted) | What it names / verdict |
|---|---|---|
| `~/scratch/side-scenes/b8-cash` (qualified exe pack, status PASS) | citation manifest `f9d4a6953c55b75b4f04db3c0ece198bd2032361ac65dc8275ce07ccb6aa3f8b`; `artifacts/screenshots/cash-charge.png` `27fd5435fbe64c96315fcb29f0947bfc02d794b318c8a3047dccf15d176be671`; `artifacts/screenshots/itemshop.png` `0c561c6648fe29aaef1b64ca867de05baef9fae7ab12afbcd9c1a42652293ee8`; `artifacts/input.log` `1e3911b987f57c2e0c970aeea77b60173162ea82ffa3588ea00c42d77d506b11`; local `manifest.json` file hash `7cb327c1c67a9f4c1c9028475a1b605dc27d8b6ef6b5f565fcd3cc2a5975960b` | Charge plate `(92,355)` opens the titled `Cash Charge` chrome over the Item Shop board. Frame `(171,3)-(648,595)`, client `(190,58)-(630,564)`, close `(571,5)-(619,41)`. No X press, no HTTP request, no game-TCP tap (`wire_tap=0`). Citation/manifest hash drift is recorded; frame hashes agree. |
| `docs/parity/payment.md` `EV-B8-CASH-FRAME` | repo file at `f1987b5` | Native `.bnd` rect pairing by code ordinal; RGB `(25,24,25)` client body; hover-free referent. Static pin, not a second boot. |
| #362 (`905ef9c`, `776cfe7`, CLOSED) + `~/scratch/p362-findings.md`, `~/scratch/p362b-findings.md` | corrected live root frame `d04024813b799dbcde6c8c3c10c442d9b706ba86e1461f963ef480541389fbc9`; web-client MAE `114.065 → 0.667`, frame `105.948 → 2.042`, title `17.747`; post-click backbuffer `e10aa2fdad5e8b0d` | Chrome fix on the boot-staged path; live-4's "state changed, pixels did not" was a capture fault (stale mtime PNG, no post-click dump). `live_boot_charge_plate_snapshot_click_paints_chrome` covers live snapshot click → `click_shop` → `visit_side(Payment)` → painter. The close *press* is now pinned by boot 3 below. |
| `~/scratch/story-payment/boot3` (this lane, qualified exe pack after re-package) | manifest `391212db8ff478efc97b9a761631cee6668441b60d17b164356406580b72ce6c`; provenance `fe8a1ea9f620e0f7888e3aa89beb45071205ac9dcad1be912f8bd36f9f628960`; citation `7138ab3cb3fc1da27a763315b2bc44603b41ef4a8c3c074c3571cc2c86876554` (internal manifest sha `2a14b8d2…`); post-X frame `artifacts/screenshots/itemshop-after-x.png` `ce0e097066d1f53488af4039c85125e5e927b2cb900bcc7042bce633574ff5fd`; `x-close-observation.md` `6355c1d6…c83c72` | One SendInput press at `(590,23)` at 13:56:26 dismissed the chrome: post-X frame MAE 1.889 vs this pack's `itemshop.png`, 53.492 vs `cash-charge.png`. `run-manifest verify`, `rig-audio verify`, `cite --claim exe` all exit 0; runner `status` stays FAILED on a false-negative label guard (see §3). |
| `docs/captures/journey-audit/178-item-shop-story-matrix.md` | charge row hash `d0402481…`; X-close row GAP `E-X`; wallet wire `a97287d11b9e93b5c4326e4a93b6d1f7a5d01551ed34928a975b759cb8c8d31d` | Cross-state pins: the Item Shop lane grades the same plate and pins the shared `5028/5029` wallet source. Its X-close GAP is superseded for this lane by boot 3; the Item Shop matrix itself is not edited. |
| `docs/captures/journey-audit/296-payment.md` (old audit at `2105ac6`) | `R-PAY` `e913c53f08470d85a624bcfa5facdb97b58bf2d55afd5479bdc0911dfe113ac2` | Stories 3 and 5 were MISMATCHED against the unwired stand-in. Stale after #362: the chrome path now exists. Kept as a contradiction, not a current verdict. |
| `docs/smoke/payment-tutorial-hand-session.md`, `payment_entry_queues_no_wire_call`, `payment_side_without_staged_rects_paints_no_notice` | repo files at `f1987b5` | Our seam journey states the stall; no wire on entry. Guards are fix state, never exe evidence. |
| `docs/net-protocol.md` §2/§4, `docs/lifecycle.md:193`, `docs/gifts.md:36` | `payment_input.aspx` / `gift_input.aspx` strings at files `0x2466d0`–`0x24a2d0`; 5015 StartPayment; `gift_input.aspx` web-only | Static names and TCP families only. No HTTP request line, session join, or completion frame in any reused pack. External-host content is NA-DESIGN. |

Every GAP below names a runnable next step. Green guards alone never grade an
exe-behavior cell; a server state shared by both clients is not a scenario
delta.

## 2. Matrix (16 actions × 5 observer actors = 80 cells)

| Action (raised by) | solo | master | peer | channel | server |
|---|---|---|---|---|---|
| **ENTER-CHARGE-PLATE** (self:master click `(92,355)`) | NA-DESIGN — offline seam has no gateway Item Shop | EVIDENCED — `<charge plate click>` → titled `Cash Charge` chrome over the shop board → pack `27fd5435…be671`, input `1e3911b9…`, #362 close record | NA-DESIGN — b8 has no seated peer; the sheet is local | NA-DESIGN — modal sheet never reaches the channel board | NA-DESIGN — b8 `wire_tap=0`; no 5015/5000-family frame is claimed |
| **CHARGE-CHROME-RECTS** (self:master, staged sheet) | NA-DESIGN — synthetic staged rects are our presentation seam | EVIDENCED — `<boot staging>` → frame `(171,3)-(648,595)`, client `(190,58)-(630,564)`, close sprite `(571,5)-(619,41)` → pack `27fd5435…be671`, `EV-B8-CASH-FRAME`, #362 `905ef9c` + `776cfe7`; our corrected root `d0402481…` matches (frame MAE 2.042) | NA-DESIGN — another player cannot see a local dialog | NA-DESIGN — no board pixel for a local sheet | NA-DESIGN — rects come from `.bnd`, not a wire field |
| **WEB-CLIENT-BODY** (self:external host pixels) | NA-DESIGN — no web bridge offline | EVIDENCED — `<charge plate open>` → black client fill RGB `(25,24,25)` on x 190..629, y 58..563 → pack `27fd5435…be671`, `docs/parity/payment.md`; corrected root MAE 0.667 after the capture-fault fix | NA-DESIGN — local client area only | NA-DESIGN — not painted on the board | NA-DESIGN — no wire body carries the client area |
| **PAGE-NAME-STRING** (self:external host) | NA-DESIGN — no billing host offline | EVIDENCED — `<billing view named in exe>` → `payment_input.aspx` path `np/billing/payment/` → dossier file offset `0x2466d0`–`0x24a2d0`, `docs/lifecycle.md:193`; this is a static name, not a loaded page | NA-DESIGN — no page name shown to another player | NA-DESIGN — no board label | NA-DESIGN — no opcode for an HTTP page name |
| **WALLET-HOST-SEPARATION** (self:master) | NA-DESIGN — offline shop fixtures are not exe billing | EVIDENCED — `<charge sheet over shop>` → wallet `G 15527`/`W 0` stays on the host Item Shop board, not inside the web client → `itemshop.png` `0c561c66…` paired with `cash-charge.png` `27fd5435…be671` | NA-DESIGN — no peer readout in this local sheet | NA-DESIGN — no channel observer pixel | NA-DESIGN — no wallet frame on this transition in any tap |
| **WALLET-SYNC-FAMILY** (self:server `5028→5029`) | NA-DESIGN — no gateway wallet offline | EVIDENCED — `<shop wallet refresh>` → `G 15527`/`W 0` on the host board behind the chrome → `itemshop.png` `0c561c66…`, shops wire `a97287d1…` (cross-state pin, not a Payment-page claim) | NA-DESIGN — private wallet | NA-DESIGN — board observer sees no shop wallet | LIVE-FILLED — `C2S 5028` empty → `S2C 5029 body a73c000000000000` = 15527 / 0, tap `a97287d1…`; shared once with #178/#179 |
| **NO-BALANCE-IN-VIEW** (self:external host) | NA-DESIGN — our stand-in's no-balance rule is a guard seam, not exe proof | EVIDENCED — `<charge sheet open>` → no cash/gem readout in the web client; the only wallet is on the host board → pack `27fd5435…be671`, `0c561c66…` (negative pack pin) | NA-DESIGN — local view only | NA-DESIGN — local view only | NA-DESIGN — no balance frame pinned on entry |
| **NO-SUCCESS-CLAIM** (self:external host) | NA-DESIGN — guard-only negative under ADR-0003 | EVIDENCED — `<charge sheet open>` → no success, receipt, or completion readout in b8's client area → pack `27fd5435…be671`; honest-invariant guard is fix state, not this evidence | NA-DESIGN — local view only | NA-DESIGN — local view only | NA-DESIGN — no completion exchange in b8 (`wire_tap=0`) |
| **NOTICE-STAND-IN** (self:solo seam) | NA-DESIGN — `BILLING UNWIRED payment_input.aspx gift_input.aspx` is our presentation stand-in (`e913c53f…`, smoke note), not an exe string on the web client | NA-DESIGN — same owner presentation seam; #362 pins the chrome, not the notice text | NA-DESIGN — notice is local | NA-DESIGN — notice is local | NA-DESIGN — no wire field for notice ink |
| **NO-WIRE-ENTRY** (self:master) | NA-DESIGN — no gateway seam offline | EVIDENCED — #429 CLOSED — W5 wd-charge NEGATIVE PIN: chrome 362a6201…264a, X-close b78ce9b0…2b1c, zero 5000-family frames ; FINAL SWEEP #382 (prior GAP): GAP — no qualified exe game-TCP tap spans one charge-plate click, so "zero 5000-family frames" is unproven; OPEN — run `--wire-tap` through the charge click and log every opcode with the frame hash | NA-DESIGN — no peer wire consequence in this local visit | NA-DESIGN — board observer has no side-sheet wire | EVIDENCED — #429 CLOSED — W5 wd-charge NEGATIVE PIN: chrome 362a6201…264a, X-close b78ce9b0…2b1c, zero 5000-family frames ; FINAL SWEEP #382 (prior GAP): GAP — b8 `wire_tap=0`; OPEN — same tap, with `5015`/`5000` family absence logged as a negative |
| **X-CLOSE-PRESS** (self:master click `(590,23)`) | NA-DESIGN — no dialog offline | EVIDENCED — `<one SendInput press at (590,23)>` → chrome dismissed, host Item Shop board revealed → post-X frame `ce0e097066d1f53488af4039c85125e5e927b2cb900bcc7042bce633574ff5fd`, boot3 citation `7138ab3c…`, rect `(571,5)-(619,41)` from b8 | NA-DESIGN — local modal | NA-DESIGN — local modal | NA-DESIGN — no opcode expected for a local close; a tap is not the closing evidence |
| **X-CLOSE-CLIENT-FIX** (self:master, fix state) | NA-DESIGN — no dialog offline | OPEN-FIX — our client paints the close sprite but has no close-press dispatch (`side.rs` has only `SHOP_CHARGE_MONEY_CODE`/`OpenCashCharge`); exe behavior is now pinned above, so the parked fix is explicit; verify — wire the pinned close rect and pass a live-chain guard before closing #362's residual | NA-DESIGN — local modal | NA-DESIGN — local modal | NA-DESIGN — no wire field in a local close |
| **RETURN-TO-HOST** (self:master after close or BACK) | NA-DESIGN — our `leave_side` hand session is a seam test, not an exe claim | EVIDENCED — `<X close press>` → Item Shop board with wallet `G 15527`/`W 0` back on screen → post-X frame `ce0e0970…` (MAE 1.889 vs same-pack `itemshop.png`), boot3 citation `7138ab3c…`; BACK return stays unpinned | NA-DESIGN — local return | NA-DESIGN — local return | NA-DESIGN — no wire frame on dialog dismissal in any reused pack |
| **LOCAL-SHEET-VISIBILITY** (self:master) | NA-DESIGN — no gateway session offline | EVIDENCED — `<charge click>` → chrome paints over the Item Shop host board, which stays visible around it → pack `27fd5435…be671` + host `0c561c66…` | NA-DESIGN — no peer seat observes a local modal | NA-DESIGN — no board row changes | NA-DESIGN — local paint, not a wire event |
| **HTTP-REQUEST-LINE** (self:external host) | NA-DESIGN — dead host offline | NA-DESIGN — no request line is present in b8 (`wire_tap=0`) and the host/session leg cannot be replayed; owner presentation decision owns page shape, so this is not chased as an exe cell | NA-DESIGN — no other player sees an HTTP line | NA-DESIGN — no board observer sees a web request | NA-DESIGN — HTTP is outside the game-TCP family; static offsets in §2 are the only pin |
| **RECT-TO-PAGE-MAP** (self:master, staged `State_Payment` placements) | NA-DESIGN — synthetic placements are a staging seam | NA-DESIGN — #428 CLOSED — client-side 9-placement declaration map only (81f2056); mount xref/frame OPEN — client-side declaration geometry; mount xref/frame OPEN (#750) ; FINAL SWEEP #382 (prior GAP): GAP — b8 pins the `Dialog_ChargeCash` rects but not which of the 9 `State_Payment` bound placements belongs to the payment page; OPEN — REA xref of the `STATE_PAYMENT` mount plus one qualified exe side-visit frame before any page-content claim | NA-DESIGN — local layout | NA-DESIGN — local layout | NA-DESIGN — layout comes from `.bnd`, not a wire field |

Counts after #382 final sweep: **80 cells** — 13 EVIDENCED, 1 LIVE-FILLED, 1 OPEN-FIX, 65 NA-DESIGN. Every disposition names a pack/pin/ticket or an owner item.
Prior wave counts and their GAP wording are retired; the disposition text above is current.

## 3. Boot ledger (3 attempts spent of 4; one shared Memoryer lane)

| Attempt | Display / sink / prefix | Reached | Result |
|---|---|---|---|
| 1 (failed) | `:170`, `rig_null_3679124`, shared Korean prefix | Planet only | Scratch driver copy resolved `SCRIPT_DIR` to its own folder, so its first SendInput raised `FileNotFoundError`; `~/scratch/story-payment/boot1` kept as FAILED, no cell filled |
| 2 (failed preflight) | `:170`, `rig_null_3690454`, shared prefix claimed by another lane | no client launched | `rig-run.sh` refused at prefix-claim (exit 15); owned sink released; no pixels and no cell |
| 3 (valid exe pixels) | `:170`, `rig_null_3693685`, private prefix copy `~/scratch/story-payment/prefix` | Planet → Channel → Room → MyRoom → Messenger → Item Shop → Cash Charge → X close → Item Shop | One press at `(590,23)`; post-X frame hashed and re-packaged; manifest/provenance/audio/cite all verify. Runner label guard false-negative on blue Item Shop pixels, so `status` stays FAILED while the pack is citing exe (see §4) |

Preflight at 13:50–13:52 found two other agents' clients (python3 PIDs
3657329, 3661476) on the shared Memoryer; both 60 s polls were contended, so
boot 1 waited for an idle socket. The server (dotnet PID 92326, boot epoch
1794986056) was never restarted or signalled; every transition probed the
NORMAL-mode popup and the driver dismissed it at `(400,365)`, plus the side
MESSAGE popup at `(400,368)`. Containment: private Xvfb, owned null sink,
`route=sink`, `on_default_sink=0`, `preboot_breach=0`, peak 0.0 dBFS, exe hash
pinned. All owned processes were stopped by recorded PID files. Boot 4 is
unspent: the remaining gaps need a wire tap or REA xref, not another pixel
boot. The dead billing host, page content, host/session join, and HTTP
request lines are graded NA-DESIGN and were not chased.

## 4. Contradictions and limits

- The old audit's chrome MISMATCH verdict (`e913c53f…` stand-in) is stale
  after #362; the corrected root capture `d0402481…` and the new live-path
  guard supersede it. The contradiction is kept, not silently rewritten.
- The live-4 negative ("state changed, pixels did not") was a capture fault;
  it does not grade any cell as a client divergence. Its corrected capture
  is the evidence used here.
- Boot 3's runner `status` is FAILED only because its label guard's blue
  title predicate also fires on the Item Shop board (0.543 on both frames).
  The raw post-click probe was kept unedited and re-manifested; the pack
  now passes `run-manifest verify`, `rig-audio verify` and `cite --claim exe`.
  The failed label is run metadata, not an exe observation.
- #362's title-band residual (MAE 17.747) is our notice ink, not a rect
  mismatch; owner presentation decision keeps that ink. Exe glyph
  rasterization is presentation, not gameplay.
- The exe RGB555 client body decodes one step lower than the b8 screenshot;
  the ±1 channel delta is capture conversion, not a second painted colour.
- No story is closed by a matrix cell. Missing exe evidence stays GAP or
  NA-DESIGN; a later conformance mismatch becomes a bug, not a story edit.

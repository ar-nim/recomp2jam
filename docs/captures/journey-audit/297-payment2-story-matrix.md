# Payment2 story matrix — Spec #297 (WAVE 5, payment lane)

Method: `docs/captures/story-matrix/TEMPLATE.md`. Base `f1987b5`; worktree
`/home/arnim/Projects/recomp2jam-wt144`, branch `cb2/story-payment`. No code
change, no push. The 7.55 executable is the oracle (`OTwo.exe`, 5,029,888 B,
SHA-256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`).
Memoryer is a wire witness, never an exe rule. All packs stay in git-ignored
scratch; only hashes and text enter the repo.

Actor mapping follows the #296 matrix: `master` is the local account holder on
the side-screen path, `solo` is the offline/stand-in seam, `peer`/`channel`
cannot observe a local billing sheet, and `server` names a protocol frame.
The second billing page lives on a dead external HTTP host. Its content,
completion, failure, and request line are graded `NA-DESIGN` (owner
presentation decision); the static `gift_input.aspx` name and the shared
charge chrome are the only admissible pins here.

## 1. Evidence inventory (written before any cell was graded)

| Source | Pin (full sha256 unless noted) | What it names / verdict |
|---|---|---|
| `~/scratch/side-scenes/b8-cash` (qualified exe pack, status PASS) | citation manifest `f9d4a6953c55b75b4f04db3c0ece198bd2032361ac65dc8275ce07ccb6aa3f8b`; `cash-charge.png` `27fd5435fbe64c96315fcb29f0947bfc02d794b318c8a3047dccf15d176be671`; `input.log` `1e3911b987f57c2e0c970aeea77b60173162ea82ffa3588ea00c42d77d506b11` | The only qualified billing-window pack. It reaches the Item Shop `Cash Charge` page; no gift page, completion, or failure exchange is captured. `wire_tap=0`. |
| `docs/parity/payment.md` `EV-B8-CASH-FRAME`, #362 (`905ef9c`, `776cfe7`, CLOSED) + p362b findings | corrected live root `d04024813b799dbcde6c8c3c10c442d9b706ba86e1461f963ef480541389fbc9`; web-client MAE 0.667; guard `live_boot_charge_plate_snapshot_click_paints_chrome` | Cross-state charge chrome and rect pin for the side-view family; explicitly **not** a gift-page pixel. Live-4's negative was a capture fault. The X-close press is now pinned by the boot-3 pack below. |
| `docs/net-protocol.md` §2/§4, `docs/lifecycle.md:193`, `docs/gifts.md:36`/OPEN | `gift_input.aspx` at file `0x24b65f`, page strings `0x2466d0`–`0x24a2d0`; URL `billing/gift/gift_input.aspx`; `5017/5018/5019/5023/5024` gift-box TCP family | Static name and TCP counterpart only. `gift_input.aspx` is web-only; `5040` recipient gating lives on the dead host. No HTTP request line, completion, or failure frame. |
| `~/scratch/story-payment/boot3` (this lane, qualified exe pack after re-package) | manifest `391212db8ff478efc97b9a761631cee6668441b60d17b164356406580b72ce6c`; provenance `fe8a1ea9f620e0f7888e3aa89beb45071205ac9dcad1be912f8bd36f9f628960`; citation `7138ab3cb3fc1da27a763315b2bc44603b41ef4a8c3c074c3571cc2c86876554`; post-X frame `ce0e097066d1f53488af4039c85125e5e927b2cb900bcc7042bce633574ff5fd` | Cross-state: the same charge dialog's pinned close press dismisses it and returns to the host Item Shop board. No gift-page pixel is claimed; runner status FAILED is a label-guard false negative (see #296 §3) while cite/manifest/audio verify green. |
| `docs/smoke/payment-tutorial-hand-session.md`, `payment_labels()`, `payment_entry_queues_no_wire_call`, `staging/tests.rs` 12-placement test | repo files at `f1987b5` | Our journey visits both views and returns, inks the unwired notice, queues no wire, and stages 12 `State_Payment2` placements. Guards are fix state, never exe evidence. |
| `docs/captures/journey-audit/297-payment2.md` (old audit at `2105ac6`) | MATCHED 2 / BLOCKED 4; no gift-page pack | Pre-#362 audit; its chrome caveat is carried by the shared #362 fix. Its blocked rows stay blocked where they are host-bound, and are graded NA-DESIGN rather than chased. |
| `~/scratch/story-shops/item-interact/exe` (cross-state wire) | wire `a97287d11b9e93b5c4326e4a93b6d1f7a5d01551ed34928a975b759cb8c8d31d`; `itemshop.png` `d30dc7c451fbfbf47317be396e00aa132b68a292322c3b9f18f6425f9cef4fb2` | Shared `C2S 5028` → `S2C 5029 body a73c000000000000` (15527/0); no purchase/completion frame on that leg. |

Every GAP below names a runnable next step. Green guards alone never grade an
exe-behavior cell, and no server state is treated as a per-client observation.

## 2. Matrix (16 actions × 5 observer actors = 80 cells)

| Action (raised by) | solo | master | peer | channel | server |
|---|---|---|---|---|---|
| **ENTER-GIFT-SIDE** (self:master gift-page entry) | NA-DESIGN — no gateway side visit offline | GAP — no qualified exe gift-page entry visit; OPEN — REA xref of the `CStatePayment2`/`CDPayment` gift mount and one captured mount frame, not a boot into the dead host | NA-DESIGN — local sheet | NA-DESIGN — local sheet | NA-DESIGN — `gift_input.aspx` is HTTP-only; no dedicated game opcode pinned |
| **SHARED-CHARGE-CHROME** (self:master, #362 cross-state) | NA-DESIGN — synthetic stand-in only | EVIDENCED — `<charge plate (92,355)>` → titled frame/client/X chrome for the side-view family; pack `27fd5435…be671`, rect pin `EV-B8-CASH-FRAME`, #362 `905ef9c` + `776cfe7`; corrected root `d0402481…` MAE 2.042. Cross-state only: no gift-page frame exists | NA-DESIGN — local modal | NA-DESIGN — local modal | NA-DESIGN — rects come from `.bnd`, not a wire field |
| **GIFT-PAGE-NAME** (self:external host) | NA-DESIGN — dead host offline | EVIDENCED — `<billing page named in exe>` → `gift_input.aspx`, URL path `billing/gift/` → dossier file `0x24b65f`, `0x2466d0`–`0x24a2d0`, `docs/lifecycle.md:193`; static name, not a loaded page | NA-DESIGN — no other player sees the name | NA-DESIGN — no board label | NA-DESIGN — no opcode carries an HTTP page name |
| **SEPARATE-PAGE-CLAIM** (self:master gift vs payment) | NA-DESIGN — offline seam paints one stand-in notice | GAP — our `payment_labels()` names both pages but no exe frame proves a distinct gift view; OPEN — REA xref of the `CDPayment` two-page dispatch plus one qualified exe mount frame before changing the side model | NA-DESIGN — local view | NA-DESIGN — local view | NA-DESIGN — no wire field distinguishes the two web pages |
| **CONFIRMATION-COMPLETION** (self:external host) | NA-DESIGN — no billing host offline | NA-DESIGN — no completion/failure exchange in any pack; dead external host and owner presentation decision own this seam, so no live leg is justified | NA-DESIGN — no other player sees a web result | NA-DESIGN — no board result | NA-DESIGN — no 5015/5016 completion frame pinned for billing (shop 5000-family pins are not billing completion) |
| **FAILURE-PATH-HONESTY** (self:solo seam) | NA-DESIGN — our notice is the presentation stand-in, not the exe's failure UI | NA-DESIGN — exe failure presentation lives on the dead host; no failure frame in b8, so the honest notice is our decision, not a matched claim | NA-DESIGN — local view | NA-DESIGN — local view | NA-DESIGN — no failure opcode pinned |
| **NO-SUCCESS-BEFORE-COMPLETION** (self:master, negative) | NA-DESIGN — guard-only negative (`payment_entry_queues_no_wire_call`) | NA-DESIGN — b8's black client area shows no success marker (pack `27fd5435…be671`), but no gift-page completion exists to compare; this stays our invariant, not an exe story | NA-DESIGN — local view | NA-DESIGN — local view | NA-DESIGN — no completion frame; a zero-frame tap would be a negative, not a success rule |
| **NO-BALANCE-GIFT-PAGE** (self:external host) | NA-DESIGN — no web bridge offline | NA-DESIGN — page content is dead-host territory; b8's no-balance negative is charge-page only and is not promoted to a gift-page claim | NA-DESIGN — local view | NA-DESIGN — local view | NA-DESIGN — no gift-page wallet frame pinned |
| **GIFT-12-RECT-LAYOUT** (self:master, staged `State_Payment2`) | NA-DESIGN — synthetic placements are a staging seam | GAP — 12 bound placements are staged and guarded, but no exe frame maps them to the gift page; OPEN — REA xref of `State_Payment2.bnd` to its `CDPayment` mount plus a qualified mount frame | NA-DESIGN — local layout | NA-DESIGN — local layout | NA-DESIGN — layout comes from `.bnd`, not a wire field |
| **NO-WIRE-COMPLETION** (self:master entry) | NA-DESIGN — no gateway seam offline | GAP — no exe tap spans a gift-side visit, so "zero completion frames" is unproven; OPEN — `--wire-tap` one side visit and log every opcode, with the absence named as a negative | NA-DESIGN — no peer consequence | NA-DESIGN — board observer has no side-sheet wire | GAP — b8 `wire_tap=0`; OPEN — same tap, naming the absence of a 5015/5016 completion reply |
| **X-CLOSE-PRESS** (self:master click `(590,23)`, cross-state) | NA-DESIGN — no dialog offline | EVIDENCED — `<one SendInput press at (590,23)>` → shared billing dialog dismissed → post-X frame `ce0e097066d1f53488af4039c85125e5e927b2cb900bcc7042bce633574ff5fd`, boot3 citation `7138ab3c…`; cross-state, not a gift-page pixel | NA-DESIGN — local modal | NA-DESIGN — local modal | NA-DESIGN — a local close has no opcode; no wire claim |
| **X-CLOSE-CLIENT-FIX** (self:master, fix state) | NA-DESIGN — no dialog offline | OPEN-FIX — our side view still has no close-press dispatch (only `SHOP_CHARGE_MONEY_CODE`/`OpenCashCharge` exists); exe close is now pinned, so #362's residual is explicit; verify — a live-chain guard from the pinned close rect back to the host board | NA-DESIGN — local modal | NA-DESIGN — local modal | NA-DESIGN — no wire field in a local close |
| **RETURN-TO-HOST** (self:master after close or BACK) | NA-DESIGN — hand-session `leave_side` is our seam | EVIDENCED — `<X close press>` → Item Shop board with wallet words back in view → post-X frame `ce0e0970…`, boot3 citation `7138ab3c…`; BACK from a gift view stays unpinned | NA-DESIGN — local return | NA-DESIGN — local return | NA-DESIGN — no dismissal frame in any reused tap |
| **LOCAL-SHEET-VISIBILITY** (self:master) | NA-DESIGN — no gateway session offline | EVIDENCED — cross-state: the pinned billing chrome paints over the host Item Shop board, which stays visible around it → pack `27fd5435…be671` + host `0c561c66…`; not a gift-page claim | NA-DESIGN — no peer seat sees a local sheet | NA-DESIGN — no board row change | NA-DESIGN — local paint, not a wire event |
| **HTTP-REQUEST-LINE** (self:external host) | NA-DESIGN — dead host offline | NA-DESIGN — no request line in b8 and no replayable host/session leg; owner presentation decision, so not chased as an exe cell | NA-DESIGN — no other player sees HTTP | NA-DESIGN — no board observer sees HTTP | NA-DESIGN — HTTP is outside game TCP; file offsets in §1 are the only pin |
| **WALLET-SYNC-FAMILY** (self:server `5028→5029`) | NA-DESIGN — no gateway wallet offline | EVIDENCED — `<shop wallet refresh>` → `G 15527`/`W 0` on the host board behind the side sheet → `itemshop.png` `d30dc7c4…`, shops wire `a97287d1…` (cross-state pin) | NA-DESIGN — private wallet | NA-DESIGN — board observer sees no shop wallet | LIVE-FILLED — `C2S 5028` empty → `S2C 5029 body a73c000000000000`, tap `a97287d1…`; shared once with #178/#179 |

Counts: **80 cells** — 6 EVIDENCED, 5 GAP, 1 LIVE-FILLED, 1 OPEN-FIX,
67 NA-DESIGN. Every GAP names its next evidence step. A cell is a story
candidate, not a closure: closing still needs the live caller chain and the
project gates.

## 3. Boot ledger (shared with #296; 3 attempts spent of 4)

| Attempt | Containment | Result |
|---|---|---|
| 1 (failed) | `:170`, owned null sink, shared prefix | Scratch driver path fault; no SendInput, Planet only, no cell |
| 2 (failed preflight) | `:170`, owned null sink, shared prefix claimed by another lane | Prefix-claim refusal before client launch; no cell |
| 3 (valid exe pixels) | `:170`, `rig_null_3693685`, private prefix copy | Reached `Cash Charge`, one press at `(590,23)`, post-X host board; qualified re-packaged pack cited exe; label guard false-negative documented in #296 |

Preflight at 13:50–13:52 found two other agents' clients (python3 PIDs
3657329, 3661476) on the shared Memoryer; both 60 s polls were contended.
The server (dotnet PID 92326, boot epoch 1794986056) was never restarted or
signalled; the NORMAL-mode popup was dismissed at every transition. Private
Xvfb, owned sink, `route=sink`, `on_default_sink=0`, `preboot_breach=0`, peak
0.0 dBFS, pinned exe hash; owned PIDs only were stopped. Boot 4 is unspent:
remaining gaps need REA xrefs or a wire tap, and the dead gift host is not a
boot target.

## 4. Contradictions and limits

- The old #297 audit called page content BLOCKED; this matrix grades it
  NA-DESIGN because the host is dead and page presentation is an owner
  decision. Static page names remain EVIDENCED as dossier facts.
- Our client merges both billing views into one `SideScreen::Payment`; no exe
  gift-page frame proves or refutes that model. It stays a GAP with a REA
  next step, never a silent MATCHED verdict.
- The charge-page negative pins (no balance, no success) are cross-state
  anchors for the same chrome, not gift-page evidence.
- Boot 3's runner `status` is FAILED because its blue-title guard fires on
  the Item Shop board too; the unedited post-click frame was re-manifested
  and the pack cites exe. This is a label-guard fault, not exe behavior.
- A qualified wire tap with zero completion frames would be a negative on
  the game-TCP lane; it cannot prove anything about the dead HTTP host.
- No story is closed by a matrix cell. Only new exe evidence can change an
  exe-derived story; a client mismatch becomes a bug.

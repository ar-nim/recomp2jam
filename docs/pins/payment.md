# Static pins — Payment / Payment2 (spec #296 / #297)

Target pin: `OTwo.exe` 7.55, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
This lane did not open REA. Every evidence ID below is copied from a
pin already in the repo; a row with `ev pending senior REA` has no
evidence ID yet. Behavior wording is ours; no disassembly is copied.

Existing pins drawn on: `docs/pins/misc-tail.md` (window-hub web
rebuilds and Print Screen), `docs/pins/web-asset.md` (browser-host
sizing), `docs/pins/input-msg.md` (cash-bill close-then-open),
`docs/web-url-archive-audit.md` (URL literals), `docs/parity/payment.md`
(staged chrome maps), `docs/net-protocol.md` §2 (page name offsets).

## Pins already in the repo

| Story | VA / file offset | Behavior in our words | Evidence | Our client |
|---|---|---|---|---|
| #1099 | `0x4f3d00` / `0xf3d00` | On the `WM_APP+0x32` arm with state word 12 and a stored web handle, close that handle, clear it, then rebuild the Payment URL and mount the view again. | dossier `ev 67c4.702e.0830.2902.68df.a4a8.17a4.8d43.d352.34bb.fd6e.37dd.849a.4c5f.4638.fbee`; file-offset mapping `ev 2cd9.9930.66c5.f9bb.eb36.dae7.936d.93c7.33de.3f20.9c2a.dabd.6193.b412.4951.78fd`; window hub `ev dbeb.88ad.7f31.83f8.9d0c.b173.ce38.b3b2.8bab.55c5.8662.2866.5ba0.cef4.2a9b.d42a` (`docs/pins/misc-tail.md`) | `BillingWindowArm::RebuildPayment` then `BillingWebHandle::rebuild` in `screens/messenger.rs` / `window.rs`, guarded at Tier 1 and Tier 3. |
| #1100 | `0x4f5020` / `0xf5020` (opener `0x4f4bd0`) | Same close-and-clear step for state word 13. The opener reads `USER_INDEX_ID` from `Identifaction.ini` and mounts either the jamcard page or the coupon page. | dossier `ev f1f9.55f4.bd48.eefb.9bb6.b75e.2e5f.7f35.6508.9642.0a89.b57e.4f8a.1571.b639.ac95`; file-offset mapping `ev 66a6.cf56.7ac3.6d06.c994.6570.eaf9.4262.4e47.35d6.658b.e966.69b4.4ed4.4a3a.52f6`; caller-body batch `ev 70f0.aea9.070f.54f5.6973.c2c4.0036.049b.8ad6.27c7.4e21.d099.31c0.a472.0625.1f3d` | `BillingWebPage::{JamCard, Coupon}` builds the two URLs from the session id; the branch is an explicit argument, not an invented gate. |
| #427 (static half) | literals at files `0x24b638`, `0x24b670`, `0x24b844`, `0x24b87c`; page names `0x2466d0`–`0x24a2d0` | `http://game1.o2jam.com/np/billing/payment/payment_input.aspx?USER_INDEX_ID=<id>`, the gift sibling, `http://game1.o2jam.com/client/jamcard.asp?USER_INDEX_ID=<id>`, and `http://o2jam.nopp.co.kr/client/coupon2/coupon.phtml?USER_INDEX_ID=<id>`. Cash-bill has five publisher hosts (`docs/web-url-archive-audit.md` lines 48–53). | URL set and anchors `ev 7ea5b5e2f6feeace61f18a900834abc69a834808fe1d8c88ec6c8c5880203b68`, `ev 0f4801dce99fa1415203fcc0515c7c0b5d8acf142540c536d74a31b67093a618`, `ev_73fb69f5837511dc6adfc3ecb39d291763a80eedbf7be422f572c649eda93752` (`docs/web-url-archive-audit.md`); page-name offsets from `docs/net-protocol.md` §2 | `BillingWebPage::url` pins host, path and session join. No HTTP request line is emitted by this client; the live half stays OPEN. |
| #1101 (path half) | `0x57d800` / `0x17d800`; writer `0x42d0a0`; folder suffix at `0x64427c`; name at `0x6447d0` | On Print Screen (`0x2c`) outside states 10 and 11, the writer requests 800x600 at quality 80 and writes `ScreenShot%03d.jpg` below `My Pictures\오투잼_스크린샷` with a per-session counter. | dossier `ev efb6.f8db.ebe7.5702.f288.dd84.8af9.3d97.c971.7b21.4f57.0507.d38f.a1bd.529b.6bf2`; writer dossier `ev 5447.ceb6.c70e.2299.892a.85c5.4fda.8243.deae.5ce2.ab33.ef2b.b236.7b40.3eb3.5134`; suffix bytes `ev f22e.89b3.08c5.34f5.033b.0794.1554.de5d.40e4.3e38.2724.6365.a314.c231.c330.273e`; name literal `ev 4e83.46c0.b7ce.8c08.29fd.5a45.3eb1.480b.394e.5417.c998.6c1f.dca2.f850.c8de.6c1c` | `print_screen_file_name`, `print_screen_relative_path`, `ScreenShotCounter::press`; no JPEG encoder is wired. |
| #1102 (branch half) | `0x418dc0` / `0x18dc0`; queue `0x478d50` | When the remaining-shot word reaches zero the Print Screen arm posts the UI queue instead of calling the writer. | `ev 52e5.59f7.df3d.44f6.e10c.9c57.ce6d.aa2f.dd77.a376.1c27.b615.eb00.0092.62fb.6928`; writer caller chain in `docs/pins/misc-tail.md` | `PrintScreenAction::Notice`; no caption is invented. |

## Contradiction kept visible

`crates/o2jam-shell/src/screens/side.rs:6474` stores the gift path as
`billing/gift/gift_input.aspx`; the URL literal inventory
(`docs/web-url-archive-audit.md` line 59) names
`http://game1.o2jam.com/np/billing/gift/gift_input.aspx?USER_INDEX_ID=<id>`.
The exe literal wins for the rebuilt URL; the shorter `side.rs` label
is left untouched (senior-owned file). The disagreement is not
resolved silently.

## Hand-offs (no REA read in this lane)

- **#427 live half** — no HTTP request line or session join is
  captured by any qualified pack (`docs/captures/journey-audit/296-payment-story-matrix.md`).
  Need one local web stub that records the exact GET line and the
  session id for Payment, GiftInput, JamCard and Coupon. Not an REA
  question; rig/web-bridge work.
- **#467 BACK/dismissal frame** — the X arm is pinned, the BACK arm
  is not. **ev pending senior REA.** Question for the senior: at
  `0x5a5190` (file `0x1a5190`), read the BACK/return control arm for
  state words 12 and 13; name the handler it calls and whether that
  handler closes the stored web handle or only hides the chrome.
  Then one qualified BACK click pack with a host-board frame hash.
- **#469 gift-page entry frame** — the staged 12-rect chrome map is
  pinned (`docs/parity/payment.md`), but no qualified exe gift mount
  frame exists. **ev pending senior REA.** Question for the senior:
  xref the `CStatePayment2` / `CDPayment` gift mount around
  `0x4f4bd0` (file `0xf4bd0`) and name which control opens the gift
  page; then capture one qualified mount frame with URL and input log.
- **#1101 JPEG bytes** — this client has no JPEG encoder and no
  player Print Screen writer. Need one Tier 4 Wine leg that presses
  Print Screen outside states 10/11 and checks the written file's
  JPEG header, 800x600 dimensions and quality tables. No REA needed;
  the static writer pin is already above.
- **#1102 caption text** — **ev pending senior REA.** Question for
  the senior: read `0x418dc0` (file `0x18dc0`) at the exhausted
  branch and the consumer of UI queue `0x478d50`; name the event code
  and any caption literal. Until then the guard claims only that no
  numbered file name is produced.
- **Screenshot counter base** — the pin names the `%03d` format and
  a per-session counter, not the first value. Guards pin the format
  and the increment, not whether the first file is `000` or `001`.
  **ev pending senior REA:** read the counter load/store in
  `0x42d0a0` (file `0x2d0a0`) to settle the base.

## Census rows touched

No row in `docs/interaction-census.md` is edited by this lane. The
staged billing chrome rows in `#296`/`#297` keep their current
status; the new guards pin host/URL/close-order behavior, not a new
exe mount frame.

## Exhausted PrintScreen counter — event contract (2026-11-25 senior REA pass)

`0x418dc0` (file `0x18dc0`) checks the remaining-counter word at
object `+0x9c`. Zero posts UI event `(0x12, 0x67, 0, 0)` and no
caption literal exists in this handler. Nonzero passes the current
counter value at `+0x98` to the filename/capture helpers, increments
the counter, and decrements the remaining word. The #1102
"caption text" hand-off is therefore resolved: the observable branch
is the exact event code, not popup text.

Evidence: `ev_c03ce8c9615cd035da6ff9a8b8046c4af72a85938a908b3ecf67ec3ba33094c8`.

## Payment BACK dispatch — static result (2026-11-25 senior REA pass)

The window hub `0x5a5190` has no dedicated Payment/Payment2 BACK
arm. Its only state-12/13 web actions are on `WM_APP+0x232`: state 12
calls `0x4f3d00` (close + rebuild Payment), state 13 calls `0x4f5020`
(close + rebuild gift), each gated by the stored `+0xf6c` handle.
Key-down goes to `0x5a50d0`; close goes through `0x5a5600`. Therefore
the static half of #467 is answered: no separate BACK-to-web-handle
handler exists in the hub. Whether the Payment chrome consumes BACK
locally or lets it fall through remains a runtime/capture question.

Evidence: `ev_2d48ffdaaa073cc12ae3f6a5023dc479f5013f24adba4f5e2d8b94069c551297`.

## Payment2 opener contradiction — `0x4f4bd0` is jamcard/coupon, not a gift page (2026-11-25 senior REA pass)

The state-13 rebuild calls `0x4f4bd0`. That opener reads
`USER_INDEX_ID` from `Identifaction.ini`, then selects on the global
at `0x76fc5c`: value 0 mounts `client/jamcard.asp`, value 1 mounts
`client/coupon2/coupon.phtml`. It does not select or mount a gift page.
The existing row that calls this the "gift URL opener" is therefore a
story/pin contradiction: the handler is the jamcard/coupon opener.
The separate gift-page entry frame requested by #469 remains OPEN;
its actual opener must be found before a capture can be qualified.

Evidence: `ev_d9abcf573e099939d66bcd3b0fd97e3873e1773949e10111f988184fe05b37a7`.

## Music-shop web query contract — `mids`/`nid`/`hid`/`wid` (2026-11-25 senior REA pass)

`0x4d3170` builds a query in this exact order:
`mids=<comma-separated decimal ids>&nid=<comma-separated decimal ids>&hid=<comma-separated decimal ids>&wid=<comma-separated decimal ids>`.
Each list is a vector of 32-bit ids rendered with base-10 `itoa`, with
commas between elements and no trailing comma. Empty vectors produce an
empty value after the key. The four vectors are selected from the song
records before serialization; the HTTP method and target page are not
chosen in this function, so they remain OPEN rather than invented.

Evidence: `ev_e9b08097360cb2aad493a6e5426189e29c9a1f4c0aa30643a947acb572dfefe9`.

## Jamcard AJAX endpoint static search (2026-11-25 senior REA pass)

A complete initialized-memory byte sweep for ASCII `ajax_buy.php`
returned no match (evidence
`ev_7432c7efd30e9ac0161742ed9952c1ae7af5b7da73a26bc5131be5f876fa3faf`).
The provider string inventory also returned no `ajax_buy.php` row
(`ev_91aca4466704c49306fe0c410a21d056cf4d43c417f9b1a67e4aad30960181a9`).
This does not prove the URL cannot be built from fragments, but it
means there is no direct literal to pin. #1147's parameter shape and
native purchase/check semantics remain OPEN; no encrypted parameter
contract may be invented from the archived PHP failure alone.

## Hot-time/ticket event URL addendum (2026-11-25 senior REA pass)

`0x4f5730` first calls the state/eligibility helper `0x48a060`; failure
posts error the pinned negative error code through `0x5a8840`. Success sets state word 8,
then opens exactly
`http://o2jam.nopp.co.kr/client/event/2009/06_moa/hot_time.php`
with no query string. The earlier Result-time `0x4c1b90` path is the one
that appends `USER_INDEX_ID`, `&title=`, and the ticket-add branch; the
separator/key source for that path remains the room-title string already
pinned in `docs/gdi-text.md`, not a new invented parameter.

Evidence: `ev_fdf4d3e0996e039f0f065cab829d622a59f008fe27fd2d3c46d14a025429a6fb`.

## Coupon registration form — archived body contract (2026-11-25 senior web pass)

The surviving `coupon_in.phtml?myid=` capture at
the 2008-09-01 14:07:16 capture was fetched as raw EUC-KR HTML and decoded locally.
Raw body SHA-256:
`a283a1d95fe293ab43a485ee925b9eedfbcbc7c157c63fc9fa3fc6368d8aaa67`.
Local evidence: `/home/arnim/scratch/impl-2026-11-25/web-evidence/coupon_in-2008-09-01.html`
and its UTF-8 view.

The form is named `frmCouponCheck`. It submits by POST to the relative
`coupon_in_proc.phtml`. Fields are `cp_id` (select) and `frmCouponNum`
(text, maximum 30 characters, uppercased on key-up). The select values are
`KHCJR`, `TRGDB`, `TRKWJ`, `TRIYG`, and `VILIV`. The coupon number must
contain a hyphen, have length 11 or 17, and match either
`[A-Za-z0-9]{5}-[A-Za-z0-9]{5}` or
`[A-Za-z0-9]{5}-[A-Za-z0-9]{5}-[A-Za-z0-9]{5}`.
The page shows a local error when `myid` is absent. This is an adjacent
2008 page, not proof of the 7.55 landing page's script, but it pins the
surviving form contract for a native replacement.

Consequence for #1146: a native coupon form can implement these fields,
uppercasing, validation and a body-ignoring submit/report-back seam. It
must not claim that the dead host accepted a coupon.

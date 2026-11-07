# Shop gifts, coupons, and free passes (7.55)

Version pinned to `AngelO2Jam.exe` 5,029,888 B,
SHA-256 `83a04541…72c823b` (file offset = VA − `0x400000`).
Claim style: static `objdump -d` analysis in our own words, no
disassembly reproduced. No REA dossier covers these sites (the
shared snapshot holds loading/song-path records only), so file
offsets below are the evidence; no `ev_…` id is cited.
Corroboration: Mozart.Encore Memoryer `Source/Memoryer`
(`Messages/Requests|Responses`, `Controllers/*Controller.cs`,
`Messages/Commands.cs`) — clean-room C# for v7.55, never copied,
field names/order only.

## Wire map (odd/even pairs like the rest of game TCP)

| Id | Dir | Name | Body | Status |
|---|---|---|---|---|
| 5015 | C→S | `StartPayment` (web-coupon charge trigger) | empty | VERIFIED |
| 5016 | S→C | `StartPaymentResponse` | empty (handler reads no fields) | dispatch VERIFIED, tail OPEN |
| 5017 | C→S | `GetGiftMessageList` | empty | VERIFIED |
| 5018 | S→C | `GetGiftMessageListResponse` | `i32` flag + `i16`-counted entries | dispatch VERIFIED, widths pinned exe-ignored |
| 5019 | C→S | `ReadGiftMessage` | `i32` message id | VERIFIED |
| 5023 | C→S | `ClaimGift` | `u8` type + `i32` gift + `i32` resource + `u8` flag (10 B) | VERIFIED |
| 5024 | S→C | `ClaimGiftResponse` | `i32` flag + verdict `u8` (5 B) | flag VERIFIED, verdict pinned exe-ignored |
| 5025 | C→S | `SyncFreePass` | empty | VERIFIED |
| 5026 | S→C | `SyncFreePassResponse` | `u32` result + optional wallet snapshot | dispatch VERIFIED, tail pinned exe-ignored |
| 5028 | C→S | `SyncCashPoint` | empty | VERIFIED |
| 5029 | S→C | `SyncCashPointResponse` | `i32` gem + `i32` cash point (8 B) | VERIFIED (dispatched + two-word handler) |
| 5040 | C→S | `CheckGiftFreePass` | null-terminated nickname | absent on 7.55 wire (Memoryer-only shape, kept for interop) |
| 5041 | S→C | `CheckGiftFreePassResponse` | `i32` flag + nickname | absent on 7.55 wire (Memoryer-only shape, kept for interop) |

There is no coupon TCP opcode. `coupon.phtml` is an HTTP URL
(`http://o2jam.nopp.co.kr/client/coupon2/coupon.phtml?`, in-exe
string table), redeemed in the billing web view; the TCP side of
that flow is the 5015/5016 trigger plus the 5028/5029 wallet
refresh. Likewise `gift_input.aspx` is web-only; the gift-box
flows above are its TCP counterpart.

## C→S evidence (send builders stamp opcode `u16` + length `4`, then append)

- 5017: case appends nothing between the header stamp and the
  send call (VA `0x491974`, file `0x91974`) → empty body.
- 5019: case appends one 4-byte word from its arg (VA `0x4919c2`,
  file `0x919c2`); the gift-list UI path pushes one message id
  with the opcode (VA `0x4954b4`, file `0x954b4`) → one `i32`.
  Memoryer's `ReadGiftMessageRequest` (`int GiftMessageId`, no
  reply) agrees.
- 5023: case bounds-checks its first arg as an index into a
  `0x24`-stride gift-box table, then appends 1 + 4 + 4 + 1 bytes:
  head byte (VA `0x4a5038`), words at entry `+4`/`+8` (VAs
  `0x4a505a`/`0x4a507d`), then a final byte — a lookup result for
  item entries, `0` for music entries (VAs `0x4a50aa`/`0x4a50d6`)
  (VA `0x4a4f98`, file `0xa4f98`). Matches Memoryer's
  `ClaimGiftRequest` (`GiftType` byte, gift id, resource id,
  attributive `bool`) field-for-field. Call site passes the box
  index plus a zero word (VA `0x4a4622`, file `0xa4622`).
- 5025: case appends nothing (VA `0x4f400c`, file `0xf400c`) →
  empty; three virtual-send sites pass a placeholder zero (VAs
  `0x4f4184`/`0x4f41c6`/`0x4f51b4`).
- 5015: case appends nothing (VA `0x491a27`, file `0x91a27`) →
  empty. Five trigger sites push the opcode (VAs
  `0x4902fc`/`0x499ffb`/`0x49a150`/`0x4c84f2`/`0x4c8646`).
- 5028: case appends nothing (VA `0x491436`, file `0x91436`) →
  empty; virtual-send site VA `0x48ee81` (file `0x8ee81`).
- 5040: absent on 7.55 wire. Raw-byte scan finds no `0xb0 0x13`
  opcode immediate (the sole `b0 13` bytes are a call rel32 at
  file `0x162fdc`) and no `0xb1 0x13` anywhere; `0x13b0` also falls
  outside the `0x138b`-based receive range (`0x25 > 0x24` → default)
  → the exe never sends it; shape (`StringMessageField Name`) is
  Memoryer-only, kept for server interop.

## S→C evidence (dispatch `cmpl`, then handler)

- 5016: `cmpl $0x1398` (VA `0x490a09`, file `0x90a09`) routes to
  a bare UI notify that reads no body fields (VA `0x4910c1`) →
  modeled empty.
- 5018: `cmpl $0x139a` (VA `0x490a4a`, file `0x90a4a`) routes to
  a handler that never reads the body: it derives a count from
  the UI table bounds (`0x76fc80` minus `0x76fc7c`, divided by
  the `0x168` UI stride) and emits UI event `0xc` when nonzero
  (VA `0x490e86`, file `0x90e86`) → dispatch VERIFIED, body
  exe-ignored; widths (`i32` flag + `i16` count + `i32`/`u8`/
  4×string entries) are Memoryer-only (`GetGiftMessageListResponse`).
- 5024: `cmpl $0x13a0` (VA `0x4a55b1`, file `0xa55b1`) routes to
  a handler taking the error path unless the first word is zero
  (VA `0x4a6135`, file `0xa6135`) → validity flag VERIFIED;
  the success path (`0x4a6156` on) works the `0x24`-stride
  gift-box table and never reads the verdict slot (`0x14(%ebp)`),
  so the verdict byte is exe-ignored, Memoryer-only
  (`ClaimGiftResponse`: Success 0 / NotDefined 1–2 / DbError 3 /
  UnknownError 4, kept for server interop).
- 5026: `cmpl $0x13a2` (VAs `0x4f40f2`/`0x4f40fe`, files
  `0xf40f2`/`0xf40fe`) routes to a handler that references
  neither body slot (`0x10`/`0x14(%ebp)`): it checks the session
  flag, emits the pass UI notice, then forwards `0xc(%ebp)` to
  the pass applier at VA `0x4f4300` (file `0xf4300`, via VA
  `0x4f424f`, file `0xf424f`), which itself reads no stack body
  bytes → dispatch VERIFIED, tail exe-ignored; wallet tail (5×`i32`,
  `u32` pass kind, `i32` cash point, duration string, `i32` expiry
  seconds, per `SyncFreePassResponse` + `SubscriptionTimeExpiryCodec`)
  is Memoryer-only (kept for server interop). A 4-byte result-only
  body means "no snapshot" (nullable trailing member, as in
  `ChannelLoginResponse`).
- 5029: VERIFIED on 7.55. The `0x138b`-based receive switch routes
  `0x13a5` (index 26) through byte-table file `0x158ae0` (value 9)
  to stub VA `0x5588d7` (file `0x1588d7`), which calls handler VA
  `0x563900` (file `0x163900`): it reads two `i32` via the fixed-size
  reader (`0x555d20`) — word 1 via `0x540090` into `0x77cd00+0x2c`
  (same slot as SellItem extra and 5001 prefix), word 2 via `0x540d00`
  into `0x77cd00+0x2128` (wallet block copied at file `0x4e93d1`).
  Names `gem`, `cash_point` follow Memoryer order. 5041: absent on
  7.55 wire — no `0xb1 0x13` bytes and no `0x13b1` dispatch in
  `.text`; shape is Memoryer-only (`{invalid i32, username}`), kept
  for server interop.

## Verdict

- Request shapes 5015/5017/5019/5023/5025/5028: VERIFIED
  (append widths at the sites above).
- Response dispatch 5016/5018/5024/5026: VERIFIED; 5024 first
  word: VERIFIED. 5018 widths, 5024 verdict byte, and 5026 tail:
  PINNED as exe-ignored (handlers consume no such bytes — see
  sites above); wire shapes kept Memoryer-compatible for server
  interop.
- 5029: VERIFIED (dispatched at index 26 via byte-table value 9 to
  stub file `0x1588d7` → handler file `0x163900`; two words with
  the slots above). 5040/5041: absent on 7.55 wire (no send or
  dispatch sites; shapes kept Memoryer-only); implemented for
  server interop and labeled as such in
  `crates/o2jam-net/src/message.rs`.
- Coupon redemption itself: web-only (`coupon.phtml`), OPEN.

## Shell display (`crates/o2jam-shell/src/screens/side.rs`)

`WalletState` feeds the payment/result gem displays from decoded wire
words only: 5029 `SyncCashPointResponse` (`gem`, `cash_point`) and
the 5001 purchase words (`SyncItemPurchasePrefix` gem/point/O2-cash/
cash-point plus `SyncItemPurchaseWallets` music/item cash). Every
amount starts `None`; the TOTAL GEM bar and wallet rows render `"--"`
until the matching words arrive, so no value shows without data.

## Tests (`crates/o2jam-net/src/message.rs`)

Bare-frame vectors for 5015/5016/5017/5025/5028; single-word
vector + round-trip + short/trailing/wrong-command for 5019;
10-byte vector + round-trip + bad-type/bad-flag/short/trailing
for 5023; empty-list decode + empty-frame vector + one-entry
round-trip + invalid-flag + negative-count + bad-entry-type +
short for 5018; code map + 5-byte vector + verdict-bytes
round-trip (1–4) + short/trailing for 5024; code maps for
`GiftType`/`FreePassType`/`SyncFreePassResult`; result-only
decode + result-only frame vector + full-info round-trip +
short-info + trailing for 5026; 8-byte vector + round-trip +
short for 5029; name vector + round-trip for 5040; round-trip +
missing-user for 5041.

## OPEN

- Whether the server ever sends 5041 unprompted on 7.55 (no
dispatch exists, so the client would drop it).
- `5040` recipient-exists gating lives in web `gift_input.aspx`
  (URL string at file `0x24b65f`); no TCP trigger exists to verify.

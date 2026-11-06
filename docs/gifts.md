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
| 5018 | S→C | `GetGiftMessageListResponse` | `i32` flag + `i16`-counted entries | dispatch VERIFIED, widths Memoryer |
| 5019 | C→S | `ReadGiftMessage` | `i32` message id | VERIFIED |
| 5023 | C→S | `ClaimGift` | `u8` type + `i32` gift + `i32` resource + `u8` flag (10 B) | VERIFIED |
| 5024 | S→C | `ClaimGiftResponse` | `i32` flag + verdict `u8` (5 B) | flag VERIFIED, verdict Memoryer |
| 5025 | C→S | `SyncFreePass` | empty | VERIFIED |
| 5026 | S→C | `SyncFreePassResponse` | `u32` result + optional wallet snapshot | dispatch + first word VERIFIED, tail Memoryer |
| 5028 | C→S | `SyncCashPoint` | empty | VERIFIED |
| 5029 | S→C | `SyncCashPointResponse` | `i32` gem + `i32` cash point (8 B) | Memoryer only, exe site OPEN |
| 5040 | C→S | `CheckGiftFreePass` | null-terminated nickname | Memoryer only, exe site OPEN |
| 5041 | S→C | `CheckGiftFreePassResponse` | `i32` flag + nickname | Memoryer only, exe site OPEN |

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
- 5040: NO `0x13b0` immediate anywhere in `.text` (raw-byte push
  scan + full disassembly grep) → the exe never sends it; shape
  (`StringMessageField Name`) is Memoryer-only.

## S→C evidence (dispatch `cmpl`, then handler)

- 5016: `cmpl $0x1398` (VA `0x490a09`, file `0x90a09`) routes to
  a bare UI notify that reads no body fields (VA `0x4910c1`) →
  modeled empty.
- 5018: `cmpl $0x139a` (VA `0x490a4a`, file `0x90a4a`) routes to
  a handler walking a `0x168`-stride message table (VA
  `0x490e86`, file `0x90e86`) → list-of-records corroborated;
  exact widths (`i32` flag + `i16` count + `i32`/`u8`/4×string
  entries) per Memoryer's `GetGiftMessageListResponse`.
- 5024: `cmpl $0x13a0` (VA `0x4a55b1`, file `0xa55b1`) routes to
  a handler taking the error path unless the first word is zero
  (VA `0x4a6135`, file `0xa6135`) → validity flag VERIFIED;
  verdict byte per Memoryer's `ClaimGiftResponse`
  (Success 0 / NotDefined 1–2 / DbError 3 / UnknownError 4).
- 5026: `cmpl $0x13a2` (VAs `0x4f40f2`/`0x4f40fe`, files
  `0xf40f2`/`0xf40fe`) routes to a handler forwarding the first
  body word to the pass applier (VA `0x4f424f`, file `0xf424f`)
  → result word VERIFIED; wallet tail (5×`i32`, `u32` pass kind,
  `i32` cash point, duration string, `i32` expiry seconds, per
  `SyncFreePassResponse` + `SubscriptionTimeExpiryCodec`) is
  Memoryer corroboration. A 4-byte result-only body means "no
  snapshot" (nullable trailing member, as in `ChannelLoginResponse`).
- 5029/5041: no `0x13a5`/`0x13b1` immediates in `.text` → exe
  neither dispatches nor reads them; shapes are Memoryer-only
  (`{gem, cash_point}`; `{invalid i32, username}`).

## Verdict

- Request shapes 5015/5017/5019/5023/5025/5028: VERIFIED
  (append widths at the sites above).
- Response dispatch 5016/5018/5024/5026: VERIFIED; 5024 first
  word + 5026 first word: VERIFIED; remaining response bytes:
  Memoryer-corroborated.
- 5029/5040/5041: OPEN on the exe side (no sites); implemented
  for Memoryer interop, labeled as such in
  `crates/o2jam-net/src/message.rs`.
- Coupon redemption itself: web-only (`coupon.phtml`), OPEN.

## Tests (`crates/o2jam-net/src/message.rs`)

Bare-frame vectors for 5015/5016/5017/5025/5028; single-word
vector + round-trip + short/trailing/wrong-command for 5019;
10-byte vector + round-trip + bad-type/bad-flag/short/trailing
for 5023; empty-list + one-entry round-trip + invalid-flag +
negative-count + bad-entry-type + short for 5018; code map +
5-byte vector + round-trip + short/trailing for 5024; code maps
for `GiftType`/`FreePassType`/`SyncFreePassResult`; result-only +
full-info round-trip + short-info + trailing for 5026; 8-byte
vector + round-trip + short for 5029; name vector + round-trip
for 5040; round-trip + missing-user for 5041.

## OPEN

- 5018 entry field widths exe-side; 5024 verdict-byte handling
  exe-side (handler ignores it past the validity word).
- 5026 wallet-tail consumption exe-side (only the result word
  reaches the applier at the traced site).
- Whether 5029/5040/5041 travel on 7.55 wire at all (server may
  send 5029/5041 unprompted; client trigger for 5040 unlocated).
- `5040` recipient-exists gating vs web `gift_input.aspx`.

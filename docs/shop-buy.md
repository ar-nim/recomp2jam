# Shop buy/sell + rename flows (7.55)

Version pinned to `AngelO2Jam.exe` 5,029,888 B,
SHA-256 `83a04541…72c823b` (file offset = VA − `0x400000`;
`.text` VMA `0x401000`, file off `0x1000`).
Claim style: static `objdump -d` analysis in our own words, no
disassembly reproduced. No REA dossier covers these sites (the
shared snapshot holds 55 loading/song-path records only), so file
offsets below are the evidence; no `ev_…` id is cited.

Odd/even pairing like the rest of game TCP: request C→S `N`,
response S→C `N+1` (`docs/net-protocol.md` §3).

## Wire (verified shapes)

| Message | Id | Body |
|---|---|---|
| `SyncItemPurchase` (request) | 5000 (`0x1388`) | empty (opcode word + length word 4, no appends) |
| `SellItem` (request) | 5002 (`0x138a`) | `i32` selection, `i32` entry (8 B LE) |
| `SellItemResponse` | 5003 (`0x138b`) | `i32` result + `i32` extra iff result `0` (4 or 8 B) |
| `ChangeName` (request) | 5030 (`0x13a6`) | null-term name + `i32` selection + `i32` entry |
| `ChangeNameResponse` | 5031 (`0x13a7`) | `i32` result (4 B LE) |
| `CheckName` (request) | 5032 (`0x13a8`) | null-term name |
| `CheckNameResponse` | 5033 (`0x13a9`) | `i32` verdict + null-term string |
| `SyncCashPoint` (request) | 5028 (`0x13a4`) | empty (kept from earlier work) |

Packet helpers used as evidence vocabulary: the fixed-size
packet reader (`0x555d20`: copy `len` bytes from the packet
cursor, advance it), the fixed-size packet writer (`0x555d70`:
same for the send buffer, fixes the length word), the string
writer (`0x555e30`: appends `strlen + 1` bytes, terminator
included), the string reader (`0x555ea0`: reads `strlen + 1`
bytes). All four identified by their `strlen` (`0x5f1bc0`) +
copy (`0x5f2450`) calls.

## Send switch (`0x138a`-based, file `0xa4df0` fn)

One builder indexed by `opcode − 0x138a` (range-checked
`≤ 0x24`, byte table file `0xa54ac`, dword table file
`0xa5484`). Full index bytes (offset → case; `9` = default/end
at VA `0x4a5455`):

`0,9,1,9,9,2,9,9,3,9,9,9,9,9,9,9,9,9,9,9,9,4,9,9,9,9,9,9,5,9,9,9,6,9,7,9,8`

- index 0 (5002 SellItem) → case VA `0x4a510c` (file `0xa510c`):
  appends two `i32` words (two `push 4 / call 0x555d70`): first
  the `0x77ccc0` selection cell (bounds-checked `jl`/`jle`
  against the entry count `0x77cce4`, entry fetched via
  `0x4a3250`), then the fetched pair's first word. Call sites
  pass ignored zeros (VAs `0x49cee2`/`0x4a4c9e`, files
  `0x9cee2`/`0xa4c9e`); values come from shop UI state.
- index 2 (5004 EquipItem) → VA `0x4a4ec8`: the two-arg append
  case from `docs/equip.md` (cross-checks this table walk).
- index 21 (5023 ClaimGift) → VA `0x4a4f98` (matches the
  existing `ClaimGift` evidence — second cross-check).
- index 28 (5030 ChangeName) → case VA `0x4a520f` (file
  `0xa520f`): string append via `0x555e30` from the rename
  buffer (object `+0xb8`), then two `i32` appends (selection
  cell `0x77ccc0`, one `0x537b80` lookup word). Live call sites
  pass ignored zeros (VAs `0x49ce55`/`0x4a4cba`, files `0x9ce55`/
  `0xa4cba`); values come from shop UI state.
- index 30 (5032 CheckName) → default here: 5032 goes through
  its own opcode-arg builder (VA `0x45c3af`, file `0x5c3af`),
  which writes the header then appends one null-terminated
  string from the rename buffer (object `+0x288`) via `0x555e30`.
  Call site VA `0x45c200` (file `0x5c200`) passes ignored zeros;
  a `strlen` gate (VA `0x45c1cb`, file `0x5c1cb`) shows a notice
  instead of sending when the buffer already matches.
- index 8 (5010) → the empty-send case (header length 4, no
  appends); the other listed builders each append their own
  fields.

## Receive switch (`0x138b`-based, file `0x158200` fn)

Server packets dispatch on the id word: `0x1389` special-cased,
else index `id − 0x138b` (range-checked `≤ 0x24`, byte table
file `0x158ae0`, dword table file `0x158aa0`). Full index bytes:

`0,15,1,15,15,15,15,15,2,15,15,3,15,4,15,5,15,15,15,15,15,6,15,7,8,15,9,15,10,15,11,15,12,15,13,15,14`

(`15` = default/end at VA `0x558937`; even ids correctly map to
default — they are C→S requests with no S→C handler.)

- index 0 (5003) → stub VA `0x558842` → handler VA `0x560c80`
  (file `0x160c80`): reads `i32` result; nonzero returns at
  once, zero reads a second `i32` (into `0x768b4c`, applied via
  `0x540090`). So: result always, extra iff result `0`. The
  shop-context case (VA `0x4a5c76`, file `0xa5c76`) treats `0`
  as success and `0xffffffa0` as failure.
- index 2 (5005) → handler VA `0x560cf0` (file `0x160cf0`;
  stub VA `0x558853`, file `0x158853`): reads the result word
  via the fixed-size reader (`0x555d20`), returns at once unless
  it is `0`, then reads four more `i32` the same way (5 words
  total on success) and forwards the first and third tail words
  with opcode `0x138d` and `0x12` into the UI event queue
  (`0x478d50`, `ecx 0x762b44`): queue arg `0x10` carries the first
  tail word, arg `0x14` the third. Result `1` and any other nonzero
  value exit with no further reads. SETTLED: the wire shape is
  result plus a four-word tail iff result `0`. Roles PINNED:
  tail word 1 = slot selector (consumer VA `0x4a5fe8`, file
  `0xa5fe8`: `0x10` indexes table `0x64ca38` at VA `0x4a6008`,
  zero takes the `0x77ed38` default path, and the word is the
  remove key for `0x4a3dc0` at VA `0x4a60b9`); tail word 3 = shop
  entry/item id (consumer `0x14` looked up in `0x77ccbc` via
  `0x537b80` at VA `0x4a5ff8`, low word fed to equip-apply
  `0x4a3c80` at VAs `0x4a6089`/`0x4a60ea`, zero selects remove).
  Tail words 2 and 4 are read to advance the cursor then dropped
  (wire-present, unconsumed → reserved). The shop-context
  `0x138d` consumer works on the already-decoded words (lookups
  above) and is not a wire read.
- index 26 (5029) → stub VA `0x5588d7` → handler VA `0x563900`
  (file `0x163900`): reads two `i32` via the fixed-size reader
  (`0x555d20`) into `0x768b44` and `0x768b58`, then applies the
  first via `0x540090` (writes `0x77cd00+0x2c`, the same slot the
  `SellItem` success word and the 5001 prefix words update) and
  the second via `0x540d00` (writes `0x77cd00+0x2128`, inside the
  wallet block copied at VA `0x4e93d1`, file `0xe93d1`).
  SETTLED: two-word body, both words exe-verified; names follow
  Memoryer order (`gem`, `cash_point`).
- index 28 (5031) → stub VA `0x5588e5` → handler VA `0x563950`
  (file `0x163950`): reads a single `i32`; `0` runs the rename
  applier (`0x540bf0`). Exact 4-byte body, VERIFIED. Its trace
  call stamps `$0x13a7` (VA `0x563983`, file `0x163983`).
  Sibling dispatches `cmpl $0x13a7` at files `0x9d5d4`/`0xa5554`.
- index 30 (5033) → stub VA `0x5588f3` → handler VA `0x5639a0`
  (file `0x1639a0`): reads `i32`, then one string via `0x555ea0`,
  merges the word with `0xff000000`. Body = word + string,
  VERIFIED; the string's role stays OPEN. Its trace call stamps
  `$0x13a9` (VA `0x5639fe`, file `0x1639fe`). A second
  `0x13a9`-based switch covers 5033+ in other contexts
  (files `0x9d62b`/`0xa55cc`).
- index 25 (5028) → default: no S→C handler (it is a C→S
  request — correct). Its empty send is the existing
  `SyncCashPoint` type (send case VA `0x491436`, file `0x91436`;
  virtual-send site VA `0x48ee81`, file `0x8ee81`; dispatch
  `cmpl $0x13a4` file `0x9129a`).
- index 27 (5030) → default: no S→C handler (it is a C→S
  request — correct).

RTTI note (names only): `CDChangeNickName` exists in `.text`
strings, so a rename UI class ships in 7.55; both rename
requests have live send paths per the sites above.

## OPEN (not claimed)

- `SyncItemPurchase` 5000 request body: SETTLED as empty-only.
  The empty send is byte-verified (case `0x1388` at VA `0x4f3fa8`,
  file `0xf3fa8`, in the opcode-arg builder dispatching on the id
  word at VA `0x4f3f7f`, file `0xf3f7f`; common transmit tail VA
  `0x4f4051`, file `0xf4051`): opcode word plus length word 4, no
  appends — modeled as `SyncItemPurchase`. No byte-verified string
  body for 5000 exists: the walked string-body case (VA `0x491744`,
  file `0x91744` — previously mis-recorded as file `0x191744`)
  belongs to the `0x7d0`-based dispatch (index 15, so the wire id is
  `0x7df`, written from the dispatch arg); its `push $0x1388` is the
  trailing queue-send helper (`0x556270`) word — the same slot that
  carries `0x0` / `0x3e8` / `0xbb8` in the sibling cases — and never
  reaches the wire. Sibling string cases: index 12 (`0x7dc`, one
  append) and index 17 (`0x7e1`, two appends). `0x1388` matches no
  case in that builder (falls to the default at VA `0x491a6f`, file
  `0x91a6f`), and the `$0x1388` immediates at the reserve/commit
  helper (`0x411ea0`) sites are a 5000-byte size, not an id. The
  `0x517f70` / `0x517fb0` / `0x518000` getters serve those
  message-text cases; cart serialization through them is
  unresolved and unclaimed.
- `SyncItemPurchase` 5001 response body: CONFIRMED undecodable
  standalone (stays OPEN, no `WireMessage` type). Exclusive handler VA
  `0x560a40` (file `0x160a40`); sole caller is the `0x1389` stub
  at VA `0x558831` (file `0x158831`), reached from the id check at
  VA `0x55833d` (file `0x15833d`). Walked reads in wire order:
  - Prefix: 4×`i32` via the fixed-size reader (`0x555d20`) into
    `0x768b44`/`0x768b48`/`0x768b4c`/`0x768b58` (push sites files
    `0x160a59`/`0x160a68`/`0x160a77`/`0x160a86`). PINNED: word 1 is
    the gem slot (applied via `0x540090` to `0x77cd00+0x2c` — the same
    slot the `SyncCashPoint` gem word and the `SellItem` success word
    update); word 4 is the cash slot (applied via `0x540d00` to
    `0x77cd00+0x2128`, inside the wallet block copied at VA `0x4e93d1`,
    file `0xe93d1`). Word 2 (`0x768b48`) is write-only — no reader
    anywhere in `.text` — so it is modeled `reserved`. Word 3
    (`0x768b4c`) is a shop-checked wallet word (compared against shop
    state at VA `0x499f26`, file `0x99f26`, and pushed for price
    display alongside the gem word at VAs `0x4a221b`/`0x4a3544`/
    `0x4f45c4`, files `0xa221b`/`0xa3544`/`0xf45c4`); its exact game
    meaning stays OPEN. The sibling handler at VA `0x560e00` (file
    `0x160e00`) reads all six wallet words contiguously
    (`0x768b44`/`48`/`4c`/`50`/`54`, then `0x768b58` last, files
    `0x160e12`–`0x160e6b`) with the same two appliers, corroborating
    the six-word block. Modeled as `SyncItemPurchasePrefix` (first 16
    bytes only, `decode_prefix`/`encode_inner`).
  - Entry list: PINNED as before — bound snapshotted from the GLOBAL
    dword `0x77cce4` (VA `0x560ab4`, file `0x160ab4`), not from the
    packet, so the length is unknowable without that context: one shop
    entry word per entry (`N = 0x77cce4` words, each written into the
    shop list `0x77ccbc` via `0x537c10` at VA `0x560afc`, file
    `0x160afc` — the same helper/table triple `0x77ccbc`/`0x77ccc0`/
    `0x77cce4` the sell/equip/rename send paths read).
  - Post-list wallets: 2×`i32` into `0x768b50`/`0x768b54` (files
    `0x160b03`/`0x160b12`). PINNED as wallet words 5–6: the same
    globals the sibling handler reads contiguously, and both are
    shop-checked (`0x768b50` at VA `0x499f39`, file `0x99f39`;
    `0x768b54` at VA `0x499f7d`, file `0x99f7d`). Exact meanings OPEN.
  - 12-byte blob: PROVEN ABSENT from the wire. The `push $0xc` +
    `push $0` + `call 0x5f1ef0` sequence (call site file `0x160b3e`)
    zeroes a 12-byte stack scratch: `0x5f1ef0` (file `0x1f1ef0`) is a
    byte-fill (`rep stos`, file `0x1f1f4f`), and the cleared buffer
    feeds four `0x7795fc`-table render calls (VAs `0x560b6e`/
    `0x560b83`) — no packet read happens between the post-list words
    and the M count. The earlier "12-byte blob" wording was a misread
    of that `memset`; no wire bytes, no type.
  - M-pair list: packet-counted `i32` M (file `0x160b8a`), then two
    4-byte reads per entry (files `0x160bb6`/`0x160bc4`) — 8 wire
    bytes per pair. PINNED widths: the first word is whole-consumed as
    the entry id (validated by `0x537e90` on `0x77ccbc`, whose `0xff`
    return skips the pair at file `0x160bf5`, then resolved by
    `0x538240`, plus item-record lookups `0x52dba0` on `0x77b540`
    with `[eax+0xa]`/`[eax+0xb]` feeding render `0x4a9b60` on
    `0x7795fc`); only the LOW word of the second word is consumed
    (`movzx cx`, file `0x160bd8`) — the high half is read to advance
    the cursor then dropped. Modeled as `ShopRestockPair`
    (`entry_id`, `aux_low`, reserved `aux_high`). Game meanings OPEN.
  - Trailing word: single `i32` (file `0x160c54`), then a forward
    stamping `$0x1389` + `$0x12` into the UI event queue (`0x478d50`;
    `$0x1389` push file `0x160c68`, call file `0x160c75`). PINNED: the
    `$0x1389` is the event id, not wire. The shop-context `0x1389`
    consumer (VA `0x4f4144`, file `0xf4144`, dispatched at file
    `0xf410e`) branches on the forwarded word: nonzero stores it to
    `[ecx+0x64]` and notifies `0x13a1` (VA `0x4f417f`, file
    `0xf417f`); zero takes the `0x4f4300` refresh path (VA `0x4f419e`,
    file `0xf419e`). The word's meaning stays OPEN.
- `SellItem`/`ChangeName` word semantics: first word is the
  bounds-checked `0x77ccc0` selection index (role VERIFIED);
  the second word is the fetched entry's id word (usage
  corroborated — sibling paths feed such words to the
  item-record helper `0x52dba0`; exact game meaning OPEN).
- `EquipItemResponse` 5005: SETTLED as result plus a four-word
  success tail (`result == 0` → 20 B; else 4 B). Handler file
  `0x160cf0` reads the tail only on `0`; words 1 and 3 are
  forwarded as queue args `0x10`/`0x14` (`0x478d50` with
  `0x138d`/`0x12`), and the `0x4a5fe8` consumer pins them:
  word 1 = slot selector (`0x64ca38` index, remove key),
  word 3 = shop entry/item id (`0x77ccbc` lookup, equip-apply id).
  Words 2 and 4 are read-and-dropped (reserved). Modeled as
  `EquipTail { slot_select, reserved_a, entry_id, reserved_b }`.
  `docs/equip.md` still records the old single-word read and needs
  a follow-up edit (out of scope here).
- `SyncCashPointResponse` 5029: SETTLED as two words, both
  exe-verified at file `0x163900`. First word → `0x77cd00+0x2c`
  (shared gem/currency slot); second word → `0x77cd00+0x2128`
  (wallet-block cash slot).

## Tests (`crates/o2jam-net/src/message.rs`)

`sync_item_purchase_encodes_bare_frame` (exact frame),
`sync_item_purchase_round_trips_through_frame`,
`sync_item_purchase_rejects_trailing_bytes`,
`sync_item_purchase_rejects_wrong_command`,
`sell_item_encodes_two_words` (exact frame bytes),
`sell_item_round_trips_through_frame`,
`sell_item_rejects_short_body`,
`sell_item_rejects_trailing_bytes`,
`sell_item_rejects_wrong_command`,
`sell_item_response_decodes_result_only`,
`sell_item_response_success_carries_extra_word` (exact frame),
`sell_item_response_rejects_extra_word_on_error`,
`sell_item_response_rejects_short_success`,
`change_name_encodes_string_plus_two_words` (exact frame),
`change_name_round_trips_through_frame`,
`change_name_rejects_short_tail`,
`change_name_rejects_trailing_bytes`,
`change_name_rejects_wrong_command`,
`change_name_response_encodes_single_word` (exact frame),
`change_name_response_round_trips_through_frame`,
`change_name_response_rejects_short_body`,
`change_name_response_rejects_trailing_bytes`,
`check_name_response_encodes_word_plus_string` (exact frame),
`check_name_response_round_trips_through_frame`,
`check_name_response_rejects_short_body`,
`check_name_response_rejects_trailing_bytes`,
`check_name_encodes_terminated_name` (exact frame),
`check_name_round_trips_through_frame`,
`check_name_rejects_trailing_bytes`,
`check_name_rejects_wrong_command`,
`equip_item_response_success_carries_four_word_tail` (exact frame),
`equip_item_response_names_forwarded_tail_words` (named roles survive
round-trip),
`equip_item_response_error_decodes_result_only`,
`equip_item_response_rejects_tail_on_error`,
`equip_item_response_rejects_short_success`,
`equip_item_response_rejects_trailing_success`,
`equip_item_response_rejects_wrong_command`,
`sync_cash_point_response_rejects_trailing_bytes`,
`sync_item_purchase_prefix_decodes_first_sixteen_bytes`,
`sync_item_purchase_prefix_names_wallet_roles`,
`sync_item_purchase_prefix_rejects_short_prefix`,
`shop_restock_pair_decodes_entry_plus_split_word`,
`shop_restock_pair_round_trips_pair_list`,
`shop_restock_pair_rejects_short_body`.

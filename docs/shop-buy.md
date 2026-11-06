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
- index 2 (5005) → handler VA `0x560cf0`: reads result plus up
  to four more `i32` when result is `0`. This EXTENDS the
  single-word `EquipItemResponse` read (VA `0x4a5fe8`,
  `docs/equip.md`); reconciliation stays OPEN — the existing
  type is untouched.
- index 26 (5029) → stub VA `0x5588d7` → handler VA `0x563900`
  (file `0x163900`): reads one `i32` into `0x768b4c`. Prefix-read
  consistent with the existing two-word `SyncCashPointResponse`
  (Memoryer corroboration); the second word's role stays OPEN.
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

- `SyncItemPurchase` 5000 request body. Fourteen `$0x1388`
  virtual-send sites exist (files `0x3289`/`0x3d9c3`/`0x3da5d`/
  `0x917a4`/`0xba0d8`/`0xba610`/`0xc3766`/`0xc37a7`/`0xde143`/
  `0xf026d`/`0xf3dc9`/`0x18dbb`/`0x18cd2f`/`0x18cd7e`), but two
  send shapes were observed: empty headers via the opcode-arg
  builder (file `0xf3f7f`: cases `0x1388`/`0x1392`/`0x13a1`,
  length 4, no appends) and a string-body build at file
  `0x191744` (one `0x555e30` append from `0x517fb0` output,
  transmitted as `0x1388`). Cart serialization through
  `0x517fb0`/`0x518000` is unresolved.
- `SyncItemPurchase` 5001 response body. Exclusive handler VA
  `0x560a40` (file `0x160a40`; sole caller is the `0x1389` case)
  reads: 4×`i32`, an `i32` list bounded by the GLOBAL
  `0x77cce4` (not a packet count — standalone decode is
  impossible without that context), 2×`i32`, a 12-byte blob
  (`0x5f1ef0`), a packet-counted (`i32` M) list of `i32`+`i32`
  pairs with item-record lookups (`0x52dba0`), and a final
  `i32`. Widths observed, field semantics unpinned.
- `SellItem`/`ChangeName` word semantics: first word is the
  bounds-checked `0x77ccc0` selection index (role VERIFIED);
  the second word is the fetched entry's id word (usage
  corroborated — sibling paths feed such words to the
  item-record helper `0x52dba0`; exact game meaning OPEN).
- `EquipItemResponse` 5005 extended tail (four words when
  result `0`, handler file `0x160cf0`) vs the single-word
  equip-path read — reconciliation OPEN; type unchanged.
- `SyncCashPointResponse` 5029 second word: handler reads only
  the first word in the walked context; two-word shape stays
  Memoryer-corroborated.

## Tests (`crates/o2jam-net/src/message.rs`)

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
`check_name_rejects_wrong_command`.

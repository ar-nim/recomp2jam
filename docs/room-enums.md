# Room enum consumers (7.55 static map)

Version pinned to 7.55 (`AngelO2Jam.exe` 5,029,888 B, sha256
`83a04541…72c823b`). Static analysis only over the local install-root
copy, read-only. VA = file offset + `0x400000`. Behavior described in
own words with file offsets plus local-static record ids (sha256 of the
record tag, truncated to 32 hex). Tools: `python3` byte scans plus
Capstone disassembly; no disassembly is reproduced.

## Room row table (client-side)

The 2003 `RoomList` handler (func file `0x15b780`) parses each row into
a 0xb4-byte record at `0x76fc9c + index * 0xb4` (count at `0x76fc98`).
Field map from the parse order, which matches the wire order in
`crates/o2jam-net/src/rooms.rs` (`ev_d9f9a8b4379d052778fad598817f60ba`,
observed):

| Row offset | Wire field | Writer site (file) |
|---|---|---|
| `+0x00` i32 | number | `0x15b7a7` (2003 row parser) |
| `+0x04` u8 | state | `0x15b7ba` (2003 row parser); `0x15c631` (2020 `RoomStateChanged` handler writes 1 byte to row `+0x04`) |
| `+0x05` 128 B | title | `0x15b7d2` (`0x555ea0` string read) |
| `+0x85` u8 | has_password | `0x15b7e3` |
| `+0x86` u16 | music_id (low 12 bits kept, high nibble shifted, stored i32 at `+0x88`) | `0x15b7f7` |
| `+0x8c`..`+0x93` u8 ×8 | difficulty, key_mode, game_mode, speed, capacity, user_count, min_level, max_level | `0x15b81c`..`0x15b8b5` (eight sequential 1-byte reads) |
| `+0x98` i32 list | skills | `0x15b93e` |
| `+0xa8` u16 | premium | `0x15b963` |
| `+0xb0` u8 | room_type | `0x15b97c` |

## Consumers by value

| Enum value | Consumer site (file) | How |
|---|---|---|
| RoomState 1 (Waiting) | `0x92405`, `0x924b2`, `0x92568` (room-list builder, func `0x92300`); `0x94b31` (sorted-list path, func `0x944c0`) | Loads the row `+0x04` byte, compares against 1, skips the row otherwise (`ev_575ba6dd8313da2a089810a5a6fb0b07`, observed) |
| RoomState 0 (Unavailable) | `0x968ae` (row fill, func `0x125240`): raw byte pushed as image frame, set at `0x968b6` | Room_State.ojs (row child 12, id `0x1350002D`) frame = the state byte unmodified — frames 0/1/2 are the three icons (`ev_453c83b428119f26c6d5d8befcd48ea5`, `ev_c63d088a33a9ec1096e022c2a9a6453d`, observed). Also `0x951cc` (row bind, func `0x1251b0`): zero-test on the byte gates the level/Om display flag via `0x409bd0` (`ev_a0e29f0405d2f8088ee47d794228717d`, observed) |
| RoomState 2 (Playing) | `0x968ae` (same icon site as value 0) | Frame 2 of Room_State.ojs (`ev_453c83b428119f26c6d5d8befcd48ea5`, observed) |
| Difficulty 0–3 (Ex/Nx/Hx/Rx) | `0x95c52`, `0x95db9`, `0x961bd`, `0x965ae` (row fill, one per title/level display state): byte read, then `u16 [music record + 2*byte + 0x14]` | The difficulty byte indexes the music record's level table (same `+0x14` triple as C34/S5); the level word maps through `0x96cb0` (13-entry jump table, bound 12 → frames 1–4) onto the Room_Om.ojs digit icon (`ev_748bbf68c76030c4856ecc817b0d8f2a`, `ev_5222041b7c82c3b37995b066845f3e46`, `ev_e8ed6c0becb4e7acfc6bcc5d7d539b3d`, `ev_e518468223d47d6523fe7899d0915f3a`, `ev_de514889a12f1207baef1be5651d3e12`, observed). Value 0 zero-test at `0x96610`: record level > 3 sets a `+0x30` icon offset (`0x9664e`, reset at `0x9566c`; `ev_850115f9467abbaadf41e71f4dba02d3`, `ev_1073f0293f0c773e75bdc3c586b60e19`, `ev_e4749a0cc840c1aaeeaedb52853bd99a`, observed). Numeric `byte*12` grid into the Room_Speed.ojs frame at `0x96751` (`ev_5d40cd1e52ffd67466b3558fe68dbf65`, observed) |
| GameSpeed 0 (X05) | `0xc4137` (`test`, equal) → pushes `X 0.5` at `0xc413b` | Label chain, func file `0xc3b40` (see below) |
| GameSpeed 1 (X10) | `0xc416a` → pushes `X 1` at `0xc416f` | Label chain |
| GameSpeed 2 (X15) | `0xc419e` → pushes `X 1.5` at `0xc41a3` | Label chain |
| GameSpeed 3 (X20) | `0xc41d2` → pushes `X 2` at `0xc41d7` | Label chain |
| GameSpeed 4 (X25) | `0xc4206` → pushes `X 2.5` at `0xc420b` | Label chain |
| GameSpeed 5 (X30) | `0xc423a` → pushes `X 3` at `0xc423f` | Label chain |
| GameSpeed 6 (X35) | `0xc426e` → pushes `X 3.5` at `0xc4273` | Label chain |
| GameSpeed 7 (X40) | `0xc42a2` → pushes `X 4` at `0xc42a7` | Label chain |
| GameSpeed 8 (X45) | `0xc42d6` → pushes `X 4.5` at `0xc42db` | Label chain |
| GameSpeed 9 (X50) | `0xc430a` → pushes `X 5` at `0xc430f` | Label chain |
| GameSpeed 10 (X60) | `0xc433b` → pushes `X 6` at `0xc4340` | Label chain |
| GameSpeed 11 (Xr) | `0xc436c` → pushes `X R` at `0xc4371` | Label chain |

GameSpeed detail: func file `0xc3b40` (VA `0x4c3b40`) formats per-entry
records holding a speed byte at object offset `+0x2168` (copied to twelve
locals). Each block compares the byte against one value 0–11 and pushes
the matching `X …` label from the contiguous string table at VAs
`0x64a468`–`0x64a4a4` (`X 0.5`, `X 1`, `X 1.5`, `X 2`, `X 2.5`, `X 3`,
`X 3.5`, `X 4`, `X 4.5`, `X 5`, `X 6`, `X R`)
(`ev_f2fa3e808b206eb15716703d82970f78`, observed). The same ordinal order
drives the room-list speed icon: Room_Speed.ojs frame = `speed +
12*difficulty + extra + 1` (`0x96751`–`0x9676f`, twelve icon cells per
difficulty row), or `speed + 0x25` when the row `+0x88` word has its high
nibble flag set (`0x9672a`–`0x96738`, `ev_d8cb12a1b5d0a42e6e0f05b13706b51d`,
observed) — the icon grid corroborates the label chain order 0–11.

RoomState detail: the icon site above consumes all three values — the
raw `+0x04` byte becomes the Room_State.ojs frame, so 0 (unavailable),
1 (waiting), and 2 (playing) each render a distinct icon. Besides that,
the value-1 gates in the builder/filter stand as pinned before, and the
func-`0x944c0` filter compares row `+0x04` against two runtime filter
variables (files `0x94650`, `0x94665`) and checks level gates (row
`+0x92`/`+0x93` against the byte at `0x77cd20`, files `0x946b5`/`0x946d8`),
the password flag (row `+0x85`, file `0x9467e`), and full rooms (row
`+0x91` against row `+0x90`, file `0x94743`). The row bind (`0x1251b0`)
also compares the row `+0x88` dword against `0x64` (file `0x951da`,
`ev_b4d5ce890d00293ac5b67ad16a4e2445`, observed): only rows past that
music floor run the per-row fill.

## Room-list row icons (`Room_*.ojs` variant selection)

The lobby shows six room rows as buttons `IDC_BUTTON_SROOM_EXBUTTON1..6`
(ids `0x1301201B`–`0x13012020`, Interface1.opi STATE_ROOM). Each row
button holds the `SET 0x11` child template; child index n is the order in
that block. The row fill (`0x125240`, `ev_3c091ee3aac5f3a81405907b567d92b5`,
observed) fetches child n via `0x50dd50`, its sprite via `0x50dd90`, and
sets the sprite frame through `0x2f870` (`ev_ed9d03f3ec539c4c0de02129ead16b38`,
observed: frame arg −1 means "restore count", the second arg resets the
count). Icon-per-child (file offsets within the fill):

| Child | Control / sprite | Variant driver (row byte) | Site (file) |
|---|---|---|---|
| 6 | `IDC_IMAGE_SROOM_SPEED` `0x13500027` Room_Speed.ojs | `+0x8f` speed (+ `12*+0x8c` difficulty + `0x30` Ex extra + 1; or `+0x8f + 0x25` on the `+0x88` nibble flag) | `0x9672a`, `0x9674a`–`0x9676f` |
| 7 | `IDC_IMAGE_SROOM_MODE` `0x13500028` **Room_Mode.ojs** | `+0x8d` key_mode: 0 → frame 1, 2 → frame 2, else untouched | `0x967ad`–`0x967e2` (`ev_9c3ee393db495f0b4475b38b6602d800`, observed) |
| 8 | `IDC_IMAGE_SROOM_OM` `0x13500029` Room_Om.ojs | level word indexed by `+0x8c`, banded 1–4 via `0x96cb0` | `0x95c52` et al. |
| 9 | `IDC_IMAGE_SROOM_LEVEL` `0x1350002A` Room_LevelLimit.ojs | `+0x8e` game_mode: 1 → frame 1, else 0 | `0x96820`–`0x96841` (`ev_157564dc6c7d3e6a7df30c60a405dba9`, observed) |
| 11 | `IDC_IMAGE_SROOM_PASSWORD` `0x1350002C` Room_Password.ojs | `+0x85` has_password as frame | `0x96872`–`0x9687d` (`ev_a0eb629f2da7ae70e304e9040bbb855f`, observed) |
| 12 | `IDC_IMAGE_SROOM_STATE` `0x1350002D` Room_State.ojs | `+0x04` state as frame | `0x968ae`–`0x968b6` |
| 13–15 | `IDC_IMAGE_SROOM_ROOMNO1/10/100` `0x1350002E`–`0x13500030` Room_Number.ojs | room number digits, frame = digit + 1 | `0x968e5`+ (`ev_fa04f51cde8f822abfd391ef0961bfb2`, observed) |

So `Room_Mode.ojs` — despite the name — is the **key-mode icon**: frame 1
= 3-key, frame 2 = 7-key, frame 0 unused; the battle/normal distinction
lives on child 9 (Room_LevelLimit.ojs frame from `+0x8e`) and in the
builder's `+0x8e` filter, not in Room_Mode.ojs. Separate path: the row
page callback (`0x95a80`, `ev_9250c806d0aa5f1fd3ee90296f1838da`, observed)
stages the row button's own image (`0x42fc70`, ui-draw.md step 2) and sets
frame = room_type (`+0xb0`) + 1 when room_type != 0, frame 1 otherwise
after a music-record lookup miss.

## Triage of the `cmp *,3` lead (`ev_746a8bbc4ebf9916302cedbeb0186058`)

31 `cmp ecx, 3` sites image-wide; none links to the room difficulty
byte:

- MemberRole Spectator gates (4): `0xaec3d`, `0xc33c2`, `0xe7a89`,
  `0xe841e` (word at `0x77ee86`; see `docs/relay.md` §11,
  `ev_7171acb574675febd157c2c4d68f1aac`).
- Item/type idiom (8): `0xdc215`, `0xdf24d`, `0xe0d91`, `0xe86a5`,
  `0xee56c`, `0xeed35`, `0xf292c`, `0xf3005` — all follow the same
  helper call returning a word in `ax`, then compare and push 2
  (`ev_372e89405be66cc0afc4794abbfdf790`). Different object domain.
- Global 0–3 setter (1): `0x213415` range-checks an argument, stores it
  to `0x795f6c`; all 8 callers pass constant 3.
- Chart/bitstream codec counters (3 confirmed): `0x1bec90`,
  `0x1bed1b`, `0x1bed75` operate on codec state fields
  (`+0x60`/`+0x74`/`+0x88`) in the frameless decoder around file
  `0x1bebe0`, same family as the `0x1c20xx` bit-packer.
- Unattributed, still OPEN (12): `0x2f830` (word `+0x14`), `0x36b3c`
  (byte `+0x20d`, sibling byte `+0x20e` vs 2, message boxes), `0x5d4e2`
  (global byte, paired with a vs-4 check), `0x93f06` (byte `+0x04`,
  message box), `0x9ab13`, `0x9b585` (byte `+0x0d`, `incorrect.` box),
  `0xaf271` (globals), `0xd58fc` (byte `+0x1b0`), `0x217a3d`,
  `0x218175` (sign-extended nibbles vs 1/3), `0x1e48a1`, `0x1e4972`
  (duplicated template, `jl`).
- Codec-region, presumed counters, still OPEN (3): `0x1bf027`,
  `0x1bf037`, `0x1c023b` (bit-mask idiom, same region as the confirmed
  codec sites above).

Difficulty scan note (`ev_22f91d1e5626918ef23338dce4e316d5`, observed):
absolute-address xrefs to row `+0x8c` find only parser stores
(`0x15baee`, `0x15c84c` writes); an image-wide `movzx r32, byte
[reg+0x8c]` sweep finds no compare against 0–3 and no
`cmp byte [reg+0x8c], imm`. That scan was correct as far as it went —
but the consumer is *indexing*, not branching: the row fill reads the
byte with a `movzx` whose base register comes from the row pointer and
then uses it as the level-table index (sites above), which no
compare-shaped scan can see. Superseded by the Difficulty rows in
"Consumers by value".

## Rejected GameSpeed leads (`ev_fb8c4cdb702cde53e2c99b53d8b17dff`)

- `0x1c20f7` / `0x1c2189` (`cmp ecx, 11` after a load from
  `[eax+0x16bc]`): a bit-packer bit-cursor bound (buffer words at
  `+0x16b8`/`+0x14`, shifts/ors, `add ±9/±11`), not the speed enum
  (`ev_6a11c0f54473dc005bb00a972e2e3f34`, observed).
- `0x208e0f` (`cmp ecx, 0xb`): whitespace skip in a text-to-number
  loop (`ev_a23d74fe41a5ee1923af8c3d9e35b41b`, observed).
- `0x9af0e`, `0x9c4c0` (`cmp ecx, 11` on byte `[reg+4]`, paired with a
  vs-`0x16` check): different object, message-box validation.
- `0x12c718` (argument byte vs 6/`0x14`/11 to one target):
  multi-value gate, unattributed.
- `0xc436c` is the real value-11 site (see label chain), not a lead.
- `0x2129f4` etc. (`cmp ebx, 11` in a multi-value dispatch) and
  `0x217258` (`cmp edx, 11` on a shifted char code into a jump table):
  unrelated dispatchers.

## OPEN

- RoomState 0/1/2 and Difficulty 0–3 value sets are closed: every value
  has an exe consumer (icon frames for RoomState 0/2 at `0x968ae`;
  indexed level-table reads plus the `*12` icon grid for Difficulty,
  sites above). Remaining unattributed `cmp *,3` sites (12 + 3 codec
  ones listed in the triage above) stay out of scope for D13.
- The `0x96cb0` level→frame band boundaries (which Om frames mean which
  level ranges beyond the 13-entry bound) are inferred from the jump
  table shape, not from art; pixel confirmation would need the
  Room_Om.ojs frames.
- `relay_enum_value_tests` still locks the value sets on the Rust side;
  `rooms.rs` per-value tests below mirror them.

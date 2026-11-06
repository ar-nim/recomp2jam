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
| RoomState 0 (Unavailable) | OPEN (no imm site) | Only reached via the runtime filter below |
| RoomState 2 (Playing) | OPEN (no imm site) | Only reached via the runtime filter below |
| Difficulty 0–3 (Ex/Nx/Hx/Rx) | OPEN (no imm site) | Stored at row `+0x8c` but never branched on image-wide (see scan note) |
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
(`ev_f2fa3e808b206eb15716703d82970f78`, observed). Difficulty has no
such label table nearby; its display (if any) is icon-driven and
unlocated.

RoomState detail: besides the value-1 gates, the func-`0x944c0` filter
compares row `+0x04` against two runtime filter variables (files
`0x94650`, `0x94665`) and checks level gates (row `+0x92`/`+0x93`
against the byte at `0x77cd20`, files `0x946b5`/`0x946d8`), the password
flag (row `+0x85`, file `0x9467e`), and full rooms (row `+0x91` against
row `+0x90`, file `0x94743`). None of these pins an immediate 0 or 2.

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
[reg+0x8c]` sweep finds no compare against 0–3; likewise no
`cmp byte [reg+0x8c], imm` and no `cmp byte [reg+0x8f], imm` exist. The
difficulty byte is stored, never branched on.

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

- RoomState 0 and 2 immediates; Difficulty 0–3 immediates (12 + 3
  sites above remain unattributed; a Ghidra dossier on funcs `0x92300`
  and `0x944c0` plus the 2003/2020/2023/2025 handler family would close
  or re-scope them).
- Whether the client renders difficulty/speed icons from row
  `+0x8c`/`+0x8f` anywhere (no read sites found; may be write-only
  state).
- `relay_enum_value_tests` still locks the value sets on the Rust side;
  `rooms.rs` per-value tests below mirror them.

# Op-5 game-loop writer (7.55)

Version pinned to 7.55 (`AngelO2Jam.exe` 5,029,888 B, sha256
`83a04541…72c823b`; VA = file offset + `0x400000`).
Static analysis only over the local `install-root` copy, which stayed
read-only. No disassembly is reproduced here; behavior is described in
own words with file offsets plus evidence ids. Tools: GNU objdump 2.47
(address-anchored slices, used only to confirm byte patterns) plus
python3 byte scans. No new REA dossier was issued; the `ev_…` ids below
are local-static records minted for this hunt (sha256 of the record
tag, truncated to 32 hex).

## Ctor anchor (carried from `docs/relay.md`)

`SetID(5)` ctor at file `0xea4e0` (VA `0x4ea4e0`); the single absolute
reference to that address in the image is the vtable entry at file
`0x24b334` (VA `0x64b334`). The ctor body carries the `push 5` word
store shape shared with the `SetID(1..7)` family. No direct relative
call into `0xea4e0` exists image-wide, consistent with construction
through the relay table rather than a direct call.

## Writer: tally packer at file `0xea230`

Function at file `0xea230` (VA `0x4ea230`, `0x180` bytes up to the next
function at `0xea3b0`) stages the full 62-byte op-5 tally and sends it
(`ev_deb10e8fcc5d56f0435fc92c3afc57bc`, observed):

- Marks a stack packet object with vtable `0x64b32c`.
- Reads 15 globals in `0x77eeXX`: eleven word reads from `0x77ee86`,
  `0x42`, `0x44`, `0x46`, `0x48`, `0x4a`, `0x5c`, `0x4e`, `0x4c`,
  `0x52`, `0x50`, then four dword reads from `0x77ee58`, `0x6c`,
  `0x70`, `0x60`. Eleven words plus four dwords is 38 bytes, exactly
  the post-routing tally (`RelayStats` body 62 − routing 24 = 38).
- Stages them in order into the packet at object offset `+0x1c`
  onward, then sends the packet through the relay send helper at file
  `0x187870` (VA `0x587870`) with a zero flag word.
- Restores the object vtable to `0x64b31c` on exit.

The 15-read cluster is unique image-wide: the word globals above have
at most one other reader each (result/score code), and only `0xea230`
reads all 15 together. Order matches the `RelayStats` wire order
(role first, score last); which tally each global names beyond
count/order stays inferred, including a two-byte alignment gap before
the trailing dword trio.

## Caller: per-tick edge at file `0xeb0c9`

The tick function starting at file `0xeaec0` (VA `0x4eaec0`) calls the
packer once, at site file `0xeb0c9` (VA `0x4eb0c9`)
(`ev_ccded3a418039e0962f34de24a6a669f`, observed). The call is gated on
a word at `0x77ee84` equal to 1 and a byte at `0x77cd33` equal to 5.
Image-wide, exactly one relative call lands on `0xea230`: this tick
tail. No score-region (`0x16e400`–`0x16ec00`) or judgement-region
(`0x176500`–`0x177000`) call reaches the `0xea000`–`0xeb500` relay
cluster, so no per-hit op-5 fill site exists; the writer is per tick,
not per hit.

## Send helper sharing

Send helper file `0x187870` has exactly three callers
(`ev_8f8df9d42c5cfbc0073f6a1c13f91cb9`, observed): the op-5 packer at
`0xea385`, plus two sites beside the op-6/7 ctors (combo packs). The
op-5 tally and the combo packs therefore share one relay send path.

## Verdict

Writer VERIFIED at local-static level (packer body, single tick caller,
shared send helper, all by file offset). Field-to-global naming beyond
count/order is INFERRED and needs a Ghidra function dossier for VA
`0x4ea230` (plus tick VA `0x4eaec0` and helper VA `0x587870`) before any
exe-grounded field rename. The `o2jam-net` `RelayStats` layout itself
remains Memoryer-corroborated; only the writer provenance above is
exe-backed.

## Tests (`cargo test -p o2jam-net`)

Fifteen per-field offset tests in `relay.rs` (`relay_command_tests`):
each starts from a zeroed tally, sets one field, and asserts the wire
slice at its body offset (role 24, cool 26, good 28, bad 30, miss 32,
combo 34, max 36, jam 38, total 40, long-note combo 42, long-note score
44, life 48, pill buildup 50, pill count 54, score 58). They lock the
packer-compatible wire order without claiming exe-verified field names.

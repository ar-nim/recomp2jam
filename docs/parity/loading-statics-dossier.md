# Loading statics dossier (7.55)

Statics gate for issues #605, #611, #616 and #618. This is a **static**
report: REA read the shipped executable; no rig was run and no pixel or wire
capture is claimed here. Every claim is about O2Jam 7.55,
`/home/arnim/scratch/run-full/OTwo.exe`, 5,029,888 B,
sha256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`
(`ev_2ae4aeeeaf75e475707feba85f65a05ff25d847632e0b2bc8a16df5f59958bfa`).
REA 6.10.1 / Ghidra 12.2; analysis digest
`547bdf93bd6e607cbb88c4dbd6133b7dc54ce7ab567c389324768dbdb3e68c2c`.
Addresses are VAs; file offset = VA − `0x400000`. No disassembly or
decompiled text is reproduced; behavior is described in words.

Evidence IDs from this session are authoritative. The older IDs named in the
issues (`66e60d16…`, `3cd0316…`, `68d920c2…`) are retained as provenance from
the earlier dossiers; those records are no longer in the REA session ledger.

## Shared chain: what the exe calls "Loading"

Ghidra names the vtable at `0x64addc` (file `0x24addc`)
`CStateLivePlaying::vftable`; the constructors at `0x4e4670` and `0x4e4730`
install it (`ev_d6357db2574e7ccf38976e7a03e6ff5f136548400723a5cfb1fec2fef5e0c838`,
`ev_52773341e0a67a9425b987932a6da8aea5f028e439bac95eb973d5784408f950`;
vtable bytes `ev_656fdd14e34c90e6faff3c0d05ce2aeeee304fde4d3dc15c66d157a62aa40821`,
xrefs `ev_b474babfe62684033a667c06b76c5f337f8fb218132466e19b54361fca33499a`).
So the "Loading screen" is the opening phase of the live-playing state, not a
separate exe state. Relevant slots:

| Slot | VA (file offset) | Role |
|---|---|---|
| `+0x04` | `0x4e50b0` (`0xe50b0`) | state cleanup / font release |
| `+0x10` | `0x4e47a0` (`0xe47a0`) | init: fonts, `MusicLoading.ojs`, song-table walk |
| `+0x14` | `0x4ea8b0` (`0xea8b0`) | per-frame tick: input/command gate and mode dispatch |
| `+0x40` | `0x4e5250` (`0xe5250`) | UI command dispatcher |
| `+0x84` | `0x4e5230` (`0xe5230`) | wire send wrapper |

Constructor caller `0x477990` (file `0x77990`) ties this state to the state
factory (`ev_e5eb8489762294eaed5d47d44a02af860e4dae05962058da463d3b0d894b42d1`).
Init `0x4e47a0` creates the three fonts and stages the overlay; it is reached
by vtable dispatch, not by a direct call, which is why a plain caller query
returns no direct caller (`ev_4101c8a8f546743fb12d8e37591c12e80db3b0546507833b4e567a14cc1ad87a`,
`ev_8bfe450075a5a6fa5aa48610383c1829e98bbb495d4a7e89a51a3c6cd97a37e8`).

## #605 — refusal pixels for an invalid / uninstalled chart

**Pinned functions.** The validator `FUN_005718b0` (file `0x1718b0`) is a
boolean check, not a dialog painter: it copies a 300-byte chart header into
the game object at `+0x6f8`, checks magic and requested id, checks the
per-difficulty count bound, computes a rounded BPM-derived value, compares
the selected song id and title against a server table, and writes the chosen
difficulty to `+0x830`. Every failure returns 1; success returns 0. It has one
direct caller, the chart parser `FUN_00571d60` (file `0x171d60`)
(`ev_7dec844c54f95af52931603b2c2235e5d39d7a18ec235c70461150c845a7ddcd`,
`ev_2669f74a413aa2cea5ed4b6fc77b90e271559af82aa8ec383925773c96b46e34`,
`ev_f65bf0d6e7ff29bf35a26b43db04c782242f9cbe7351b616f65b61c6a4a28172`,
`ev_4ec151abb00953b8148e394250eb4d534406567e933ef4a1325758b1ec3bcb03`).

**Caller chain from the Loading gate.**

```
state factory 0x477990 → CStateLivePlaying ctor 0x4e4670 (vtable 0x64addc)
  → init +0x10 0x4e47a0 (MusicLoading.ojs, song pick)
  → tick +0x14 0x4ea8b0, mode 2 branch → 0x4ead50 (0xead50)
  → 0x4e57d0 (0xe57d0, chart preflight)
  → 0x56ffa0 (0x16ffa0, MUSIC\%s path + server flag)
  → 0x571d60 (0x171d60, chart parser)
  → 0x5718b0 (0x1718b0, validator)
```

`0x4e57d0` calls `0x56ffa0` on the selected row and branches on the value
left in EAX by that call. Values 1 and 2 build message text and call
`FUN_00478c90(1, 0, text, 0)`, which packs a dialog id and passes the text to
`FUN_0047edf0` (file `0x7edf0`); value 1 or 2 also makes `0x4e57d0` return 0,
so `0x4ead50` issues the vtable `+0x40` exit command `(0x12, 0x67, 0, 0)`
(`ev_7a03a6c4b04dc75acf1e723d0ae81e20a0d27ad33581b1ea28aef9b3bae5299a`,
`ev_4c6b0ea1cec7f6fa356acc8da05e19bba9a298ff90e4708d0190a26aefddf151`,
`ev_363c9a5deaaeb43086d2a5dbcdfd708f2861836bb74543bbd840343875c869b5`,
`ev_6bf440d0c4a076129e2face632d218a4f3af909ecff9d68e86475e69f3ea2e29`,
`ev_b44e034860633c4ebdda6031510861a79e02454e7bc524489a26616e812a2a82`).

The two refusal strings are at file `0x24ae88` and `0x24aee0`:
"load failed / usage period expired / purchase again" and "load failed /
delete the song in Music Shop and download it again"
(`ev_84526e9dfb2ceb69a3b11c8a87f7507768fdf465d50fba45d40777aceb9b163a`,
`ev_fcb5bc98bf5ea61327f63ccb1010f076d1f8b9b2951e8ccf613cfee0a260723c`).
A third refusal is a 25-second timer in `0x4ead50` on state field `+0x14c`:
on expiry it shows "loading time exceeded; game will exit" (file `0x24b348`)
and issues the same exit command
(`ev_de493e209e57f8afed41bec7169a9ffe4ce77659552b0d1c4c28d5c45d1bec22`).

Important distinction: the dialog trigger is the **server-list status flag**
at global `0x778a30` (byte `+6`: 2 → message 1, 3 → message 2, value at `+4`
zero clears it), not a return from `0x5718b0`. The validator still runs on
the same path and can fail silently; the exe dossier pin identifies the
rejection *rule*, while the visible refusal is the dialog above
(`ev_063ec4acadc607d87511d28c873e51196c6e00a5d8839db8c1dd78f90e682d23`,
`ev_0ccdb81e22ef6d81ee10d2175bbcaa614358a632f14159cc7c7e053f52f35b9d`).

**Verdict: LIVE path confirmed (statics).** Pixel hash and dialog layout are
still OPEN until a qualified capture exists.

**Rig brief (one line).** Pick an expired/uninstalled song at Music Loading
(or hold 25 s), capture the frame with the dialog over the Loading overlay,
and hash it; tap 4010/4011/4012 plus the client exit command, but expect no
new wire refusal frame from `0x1718b0`.

## #611 — X/back inside Loading

**Pinned functions.** The tick `0x4ea8b0` has a live input/command branch:
when display global `0x768128 + 0x14` is zero, display `+0x1c` has bit 2 set,
and mode byte `0x77cd33` is neither 0 nor 8, it calls vtable `+0x40` with
`(0x12, 0x67, 0, 0)` (`ev_93dcdf06bef4e78772b8b7aba4a3f96b0be0bef997241b167d290af57c25b05d`,
`ev_40172d52e819710a54814e271ead00101e78b13de21b879e02abe161d82a518b`).
`+0x40` is `0x4e5250`, which forwards command `0x67` to `0x4de400`.
That handler calls `0x4e3320` (clears 34 note/audio slots and voice handles),
then sends vtable `+0x84(0xbbd, 0)`. `+0x84` is `0x4e5230` → `0x4dd460`,
which sends wire id `0xbbd` (3005) with a two-byte body and clears the state
word at `+0x6d2` (`ev_b5c931fc665d22f8721b7979b04e87f0370b87c1338b6197493901079cbabf5b`,
`ev_12c316784eff74e8febeb38280ece76524b4295c063b500d7dc7b14744b189d3`,
`ev_ec0a5f0398efbfe7e4b2a19343733ce7eb36a538109f0a74051b7d927307abd1`,
`ev_ebbd5e01185002b7e4fd2e1f6e53b57ae75218cebabcfa2ea9c8cb654290905b`,
`ev_e48b6c1aadf0d5281d93c4066ec43e36c1a193bb551399ed5f810941ae888c6c`,
`ev_4b35eda174d79ef2a079fcadc39906d30ee7caad3548035edb7cb56d8c054797`).

The other tick branch (`display + 0x14 != 0`) dispatches by mode byte:
1 → `0x4eacf0`, 2 → `0x4ead50`, 4 → `0x4eae40`. So input is not globally
ignored inside this state; the exit gate is guarded by display flags and
mode.

**Wire 2021/2022 check.** A bounded `.text` byte scan finds the literal 2021
(`0x7e5`) only in the message dispatcher `0x491190` (file `0x91190`), where
it sends a four-byte body; that dispatcher has no resolved callers. The
four-byte encoding of 2022 (`0x7e6`) has no match anywhere in `.text`
(`ev_9563e85d8d255276c5e9072486a8861256a5caf5b3329373e9d432018b30fa3e`,
`ev_9e4116f2ed9a5d16b22cd3869e1d265b23f7a6364716e397e62c47f12a6a200e`,
`ev_ead0aff68ae957b370587a27862ed6fc4081bb4ba2563136615caac69c6443a4`,
`ev_fb036421983275df5851cbd6a3bcfcb65160736897f0b99aebc5010f97cd83a3`).
Indirect calls remain a stated limitation of the caller query.

**Verdict: LIVE exit path confirmed; 2021/2022 on X/back REFUTED by bounded
static search.** The pixel/wire pair is still a rig claim.

**Rig brief (one line).** Press X (and Escape/click as controls) during
Loading with before/after frames and a wire tap; expect exit wire 3005 only,
and treat any 2021/2022 frame as a new finding, not this path.

## #616 — Arial 7×14 w400 consumer at `+0x528`

**Pinned functions.** Init `0x4e47a0` creates Arial 7×14 and stores the handle
at object word `+0x528` (`[0x14a]`), as pinned by the earlier Lane D dossier
`ev_3cd0316ec59611a50887e5ae714f6adcfea61a51bf34ad8b2380eb8fd85b4f39`
(re-read this session: `ev_8bfe450075a5a6fa5aa48610383c1829e98bbb495d4a7e89a51a3c6cd97a37e8`).
The only reader in the Loading state is cleanup `0x4e50b0` (vtable `+0x04`):
it tests `+0x524`, `+0x528` and `+0x52c`, and passes the font-word address to
`0x41cb20`, a handle-release helper that decrements a refcount and frees on
zero. Assembly confirms an exe quirk: the `+0x52c` branch passes `+0x528`
again, so Arial is released twice and Gulimche never is
(`ev_60c8460164328fd05cf4ccdb6f0ee07d7059ea072c8a6219477a7c105499b294`,
`ev_2759bd5af0b8914e0299387f74242a532ec11941506e990fb0dfd87583a8be71`,
`ev_eb29a7754b905f5fea27e255ee470b435691a1d81df9f1954c2f52993ee91223`,
`ev_add4d89ad4d33e1e89e6c176a0e9b947b28ffc2d7cf2687917e2d87074262a2a`).
A sibling live-playing init `0x4eb8a0` and cleanup `0x4ebe50` repeat the same
create/release pattern
(`ev_af6e1aa99bb9b0ec11025be38ef2180cc1cbe21a265f6fc906ef05a2cd89ac23`,
`ev_7e215fb12574d76596c16aec38af14477768b4a50876f07fbfa6c66f4662cf67`).
A complete `.text` scan for the disp32 encoding of `+0x528` found no other
loading-region reader besides these cleanup functions and the creation site
(`ev_1256009788cc1885a75b79c262468761cf15d3fc43602aed82f6b006da2b9c4a`).

**Verdict: REFUTED as a text-draw consumer.** The word is accessed, but only
by teardown; no paint call in the Loading path reads it. Alias-through-copy
outside `.text` direct encodings is not excluded by this bounded search.

**Rig brief (one line).** No rig needed for the consumer claim; only a cover
with no baked credits can test whether any small runtime ink exists at all.

## #618 — Trebuchet 27×65 caption consumer at `+0x524`

**Pinned functions.** Same creation site `0x4e47a0` stores Trebuchet MS at
`+0x524` (`[0x149]`); the only reader found is cleanup `0x4e50b0`, which
releases it through `0x41cb20` before other state teardown. No text painter
in the gate or its vtable dispatch reads `+0x524`. The `.text` scan for that
displacement matched the creation site, the two cleanup functions, and
unrelated objects in other subsystems
(`ev_e7b6e591cbe3fdb12d5a0326affe722ec81245a0fc81a3aac5d160100fdf58e5`,
`ev_60c8460164328fd05cf4ccdb6f0ee07d7059ea072c8a6219477a7c105499b294`,
`ev_2759bd5af0b8914e0299387f74242a532ec11941506e990fb0dfd87583a8be71`).

**Verdict: REFUTED as the runtime caption consumer.** Caption-to-face mapping
stays OPEN: statics cannot distinguish runtime caption ink from credit lines
baked into the JPEG cover. The shell's `Cell::Loading` Trebuchet choice is an
implemented design decision, not an exe fact.

**Rig brief (one line).** No rig for the consumer claim; use a chart with no
printed credits and inspect whether caption ink changes with the song row.

## Limits

- Caller and xref queries use Ghidra's resolved references. Unresolved
  computed or indirect calls are not represented; absence of a resolved
  caller is not proof of absence of execution.
- Byte searches covered initialized `.text` for exact disp32/immediate
  encodings; other encodings or pointer aliases are outside that bound.
- All findings are static. Dialog pixels, wire frames and timing need the
  rig lane with a provenance manifest before any live-behavior claim.

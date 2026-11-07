# Archive entry lookup — filename vs index (7.55) — VERIFIED

How the 7.55 client addresses entries inside `avatar.opa` /
`Interface1.opi` / `Playing1.opi`. Question: is lookup always by
filename (the case-folded slot walk `0x40A9C0`, claim A13), or does an
index-based path (slot-table indexing) exist? Answer: **every load and
play path fetches by member name; the one index-based path is the
boot-time bulk extractor that copies bodies to disk, and it walks the
trailing slot table in table order.**

Version pin: `AngelO2Jam.exe` 5,029,888 B, SHA-256 `83a04541…72c823b`
(`install-root/O2Jam/AngelO2Jam.exe` verified against
`docs/ui-loader.md`). Method: read-only Python byte survey over the
installed exe — `E8`/`E9` call-target sweep, `CC`-padded `55 8B EC`
function starts, immediate-string resolution (file = VA − `0x400000`),
PE import-directory parse. No disassembly reproduced; no dossier-bearing
site is re-pinned except through its existing evidence id.

## The archive API (per `ev_3f37301b…`, `docs/ui-loader.md` step 10)

| VA | Role | Reach |
|---|---|---|
| `0x475990` | archive init: opens the three archives into slots 0/1/2 | 1 indirect site (see Class H) |
| `0x4795A0` / `0x479750` | archive select / release bracket (guard `< 3`, `0x479840`) | 65 / 68 call sites |
| `0x4795E0` | named open, assert `.\Interface\State.cpp` line `0x39A` | 1 direct caller |
| `0x4797D0` | name lookup | 12 caller functions |
| `0x479790` | body fetch | 10 caller functions |
| `0x478C90` | load member into buffer | 349 call sites / 117 functions |
| `0x40A9C0` | slot find (case-folded walk, kind-1 gate — A13/C3) | 2 callers |
| `0x40A8A0` | body open onto a `0x220`-stride stream slot (C2) | 2 callers |

The slot find `0x40A9C0` is reachable only from `0x40A8A0` (site
`0x40A974`) and `0x4797D0` (site `0x4797F8`): the case-folded name walk
is the only finder.

## Caller classes

- **Class N1 — name-based core readers (lookup + fetch pairs).** Twelve
  lookup callers, all passing member names: stream composites
  `0x402670` / `0x402BE0` (sites `0x4026FE`/`0x402C6E` lookup,
  `0x402719`/`0x402C89` fetch); sprite-55 envelope reader `0x4249F0`
  (`0x424B50`/`0x424C43`, ev `103158a8…`); format readers `0x420DA0`
  (`0x420F2E`/`0x420FA9`), `0x424190` (`0x424275`/`0x424376`), texture
  upload `0x42A130` (`0x42A1EC`/`0x42A27B`); the named-open wrapper
  `0x4795E0` (`0x479653`/`0x4796DD`); bind loaders `0x547DB0`
  (`0x547E13`/`0x547E79`) and `0x548450` (`0x5484B3`/`0x548519`, ev
  `3eb885cd…`); avatar item loaders `0x45C560` (`0x45C9D6`, strings
  `SMI_`, `%s%d%s`, `%s%d_%d%s`, `.ojs`, `SMI_NoImage.ojs`) and
  `0x521350` (`0x521607`, same `SMI_` family); `0x52F9F0`
  (`0x52FA61`, `SetInfodata.ojs` — C32). Name-based.
- **Class N2 — avatar.opa item sprites.** Resolved through Class N1
  with runtime-built names (`%s%d%s` / `%s%d_%d%s` / `%s%d%c%d%s` at
  `0x45C560`/`0x521350`/`0x4AD350`, literal fallback
  `SMI_NoImage.ojs`). Name-based; no index argument anywhere.
- **Class N3 — .ojs scene bodies.** Every control sprite resolves
  through the entry string via `0x4795E0`/`0x4797D0` (ev
  `3f37301b…`, `ev_1be2d784…`) and the envelope readers of N1.
  Name-based (A13 case folding applies).
- **Class N4 — music-list assets.** `OJNList.dat`: disk path
  `%s\Image\OJNList.dat` built in `0x479F90` (C15), archive fallback
  through `0x478C90` (site `0x47A078`, `0xFFE` buffer). Name-based on
  both paths. `Itemdata_Korea.dat` / `SetInfodata.ojs`: named opens
  (`0x52D095` push, `0x52FA61` lookup — C32). Name-based.
- **Class N5 — the workhorse `0x478C90` (117 functions, 349 sites).**
  Loads a named member into a caller buffer (C15 usage shape). Inner
  chain `0x47EDF0` → `0x47FEA0` / `0x4848D0` carries three
  archive-object `+0x110` (slot-count) accesses but no `0x98` stride
  arithmetic and no separate name walk — the find stays delegated to
  the N1 API shape. Name-based by construction of its callers.
- **Class S — archive select by slot index (not an entry fetch).**
  `0x4795A0`/`0x479750` bracket 65/68 sites choosing *which archive*
  (0 = `Avatar.opa`, 1 = `Interface1.opi`, 2 = `Playing1.opi`,
  registration order = open order). Entry addressing inside the chosen
  archive stays name-keyed.
- **Class I — the one index-based path: boot-time bulk extractor.**
  Boot `0x5AD350` (reads `option.ini`: `MODE`, `LongnoteMode`,
  `LONGNOTEMODE`) → driver `0x5A8360` (paths `%s\%s`, `%s\%s.%s`,
  `%s\Image`, `%s\Temp`, `Temp`, `OJNList.dat`) → walker `0x5A7710`
  (3,152 B, extent `0x5A7710`–`0x5A8360`). The walker does 48 `imul`
  by `0x98` (55 stride immediate/lea uses total), 5 CRT case-fold
  compares (`0x5F34FF`), 7 chunked reads (`0x4069B0`, ReadFile IAT
  `0x63F2E0`), 10 seek (`0x406CA0`, SetFilePointer `0x63F2F4`), and 7
  writes (`0x406B10`, WriteFile `0x63F2F0`). **Indexing rule: trailing
  slot-table order** — record address = table base + i×`0x98`, table
  base from EOF − count×`0x98`, the same arithmetic as the C1 reader
  inside `0x40A9C0` (`0x40AA01`–`0x40AA3A`). It enumerates records in
  table order (= pack/insertion order, the order `parse_opi_slots`
  walks), compares names case-insensitively only to match/filter, and
  copies bodies out to disk. It never serves game loads.
- **Class H — archive init dispatch.** `0x475990` has no direct
  `E8`/`E9` caller; its only reference is a slot in the boot function
  table at `0x647920` (slots: `0x475630`, `0x475AF0`, `0x4065A0`,
  `0x409860`, `0x475990`, `0x476020`, `0x409F20`, `0x409F30`; slot
  `0x647930`). The paired `0x476020` is a file-manager tick that calls
  the `0x478xxx` family (`0x478AD0`/`0x478C90`/`0x478DF0`/`0x478FD0`).

## Excluded candidates (verified noise)

A whole-image sweep for `0x98` stride arithmetic (`69 /r` imul imm32,
`81 /0` add imm32, `8D` lea disp32) finds record-table math only in
`0x40A9C0` and `0x5A7710`. Single `add 0x98` sites elsewhere
(`0x435010`, `0x435120`, `0x486230`, `0x50EE50`, `0x520DD0`,
`0x520EF0`, `0x521350`) pair with neither the `+0x110` count field nor
a name compare — they are 152-byte buffer arithmetic, not record walks.

## Anchors tied to existing claims

- A13/C3 confirmed: the find `0x40A9C0` has exactly two callers, both
  name paths; the count sits at archive object `+0x110` (dwords at
  `0x40AA01`/`0x40AA33`); stride pushes at `0x40AA3A`.
- C1/C2/C15/C32 anchors stand unchanged; C1's "imul `0x40A9FF`" is the
  load feeding the `0x40AA05` multiply measured here.
- Class anchor `.\Utility\PackingFile.cpp` lives in the archive-open
  function `0x40A5E0` (with the three archive-name literals).

## claims.md follow-ups (rows to add; not changed here)

- **A40** (EXE): all load/play entry fetches are name-keyed; caller
  census as above *(offset-only survey + ev `3f37301b…`)*. Drives R.
- **A41** (EXE): the only index path is the boot extractor
  `0x5A8360`→`0x5A7710`, table order, disk-write side *(offset-only)*.
  Drives —. Sibling-check: the explicit field is the table ordinal;
  names filter, they do not address.
- **C38** (EXE): IAT map ReadFile `0x63F2E0`, CreateFileA `0x63F2E4`,
  CloseHandle `0x63F2E8`, WriteFile `0x63F2F0`, SetFilePointer
  `0x63F2F4` *(offset-only)*.

## OPEN

1. `0x478C90` internals (192 B) and its `0x47EDF0`/`0x47FEA0`/
   `0x4848D0` chain: whether the member find is inlined there or
   delegated through the N1 API (three `+0x110` accesses, no stride
   math, no separate walk observed).
2. Boot dispatch of the Class I chain: caller site `0x5FB34A` sits in
   the CRT-startup region (caller of the `option.ini` boot fn
   `0x5AD350`); the exact startup hook is unpinned.
3. Class-name join: the `0x647920` table has no RTTI
   complete-object locator (slot `0x666FF8` holds 0), so
   `CPackingFile`/`CFileManager`/`CResManager` names stay unjoined to
   these VAs; only the `.\Utility\PackingFile.cpp` source anchor
   (`0x40A5E0`) ties a class to the archive-open path.
4. Whether the Class I extractor re-extracts on every boot or only on
   a staleness check — needs the `0x5A8360` decision logic traced.

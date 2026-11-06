# Gameplay claims re-verification (7.55)

Re-check of five muse-spark-pinned claims against the shipped exe.
Binary identity re-confirmed: `install-root/O2Jam/OTwo.exe`, md5
`34b789ff557a9bf6a1f275010dcaaceb`, 5,029,888 B — same image all
pinned offsets were made against. Method: read-only static
disassembly sweep (objdump, PE `.text` VMA `0x401000` @ file
`0x1000`, so VA − `0x400000` = file offset), re-reading each pinned
site and its caller/consumer edges. Bytes unchanged; no claim rests
on a data table alone — each verdict below names the code consumer.
Evidence IDs inherit from `docs/gameplay.md`; sites without a
previously pinned dossier are marked "re-verified from bytes".

## Verdicts

| # | Claim | Verdict |
|---|---|---|
| 1 | Jam threshold `(X+1)*50` table feed | VERIFIED (fill + live consumer) |
| 2 | Pill 15-Cool streak grant, Bad spend | VERIFIED |
| 3 | Long-note +10/tick bonus at `0x14163c` | VERIFIED |
| 4 | Judgement windows 6/18/25 inclusive | VERIFIED |
| 5 | Skill-ring byte 2 full shuffle / byte 0 narrow | VERIFIED |

Refuted items: none. No data-side-only inference found among the
five; the life-delta table candidate (`0x35e1a0`, no `.text` xref)
stays out of scope and unattributed, as already recorded.

## Evidence per claim

1. **Jam table** — `FUN_00570260` (file `0x170260`) loops
   `i = 0..0x190−1`, computes `(i+1)*50` in code (imul at
   `0x17028a`), stores words at `obj+0xb66[i*2]` (`0x170293`);
   called at game init (`0x1701fb`). Consumer: scorer
   `FUN_00570510` reads progress `+0x396c` (`0x1706f9`) and jam
   level `X` `+0x396e` (`0x170707`), indexes the table
   (`0x17071d`), compares progress > threshold (`0x170725`), then
   grants: `X+1` via `0x540a20` (`0x170734`), max snapshot
   `0x5408d0`, display mirror `+0xe98`/`+0xe9c`
   (`0x17075e`/`0x170768`). Table is code-computed, not `.data`.

2. **Pills** — dispatch at `0x16e43f`: flag `0x40`→`0x16e7fd`,
   `0x10`→`0x16e495`, `0x20`→`0x16e680`, `0x80`/`0x180`→miss
   paths. Cool: pills `+0x398c` < 5 (`0x16e4af`) gates streak
   adder `0x540b80` arg 1 (`0x16e4c0`); streak `+0x3990` > 14
   (`0x16e4d1`) + mode `+0x1853` ≠ 6 (`0x16e4e7`) + pills < 5
   (`0x16e4f8`) gate pill adder `0x540b50` arg 1 (`0x16e509`) and
   streak reset `0x540890` arg 0 (`0x16e519`). Bad: streak reset
   arg 0 unconditional (`0x16e816`); pills > 0 (`0x16e827`) +
   mode ≠ 6 (`0x16e83d`) gate spend arg −1 (`0x16e84d`) and Cool
   scorer arg 4, else arg 2 (`0x16e880`). Dossier
   `ev_c168abac…`; accessors re-verified from bytes.

3. **Long-note +10** — helper `FUN_00541620` (file `0x141620`):
   mode byte `+0x33` = 6 skips the add; `+0xa` into sub `+0x2158`
   at `0x14163c`, combo `+1` into `+0x2152` at `0x141652`
   (unconditional). Consumer: hold scan `FUN_00578110`
   (`ev_737dfe7b…`) tests per-lane hold flag `+0x9f8[i]`
   (`0x178194`), advances cursor `+0x988[i]` by 6 (`0x1781f6`),
   calls the helper (`0x178200`). Helper also serves Cool/Good
   combo (`0x16e5c1`/`0x16e73e`) and judgement sites
   (`0x176140`/`0x176d86`/`0x177230`).

4. **Windows** — autoplay/miss site `0x176646`/`0x176655`/
   `0x176664` and key-press site `0x176b7f`/`0x176b8e`/`0x176b9d`
   compare the absolute delta against 6/18/25; the unsigned-above
   branch skips, so each bound is inclusive; tiers store
   `0x10`/`0x20`/`0x40`, fallthrough `0x80` (`0x176673`), ORed
   into the judgement word `+0xe` (`0x17667d`, `0x176bce`).
   Flags consumed by dispatch `0x16e43f` (claim 2) — the window
   code has a live consumer chain. Dossiers `ev_b492dfe0…` …
   `ev_3db1071b…`.

5. **Skill ring** — setup `FUN_00571c30` (`ev_41695a9b…`) reads
   session byte VA `0x7789e4` (`0x171c39`) and seeds `_srand`
   (`0x1f7229`, `0x171c45`). Dispatch at `0x171cf5`: byte 0 →
   narrow builder `FUN_00573a10` (`0x173a10`, call `0x171d04`,
   `ev_327cea5d…`): identity over 4 slots skipping slot 1, then
   per slot `rand()%3` (`0x173a81`) with drawn-1 swap through
   table slot `+4` (`0x173a9b`). Byte 2 → full builder
   `FUN_00573940` (`0x173940`, call `0x171d1b`,
   `ev_e65f8622…`): identity to width `+0x82c`, per-lane
   `rand()%width` swap (`0x1739a1`). Mirror reversal (7−1−i,
   `0x171ce1`–`0x171ced`) and identity fill (`0x171d22`) cover
   the remaining byte values. Setup runs from chart prep
   (`0x17024b`); single `_rand` site `0x1f7236`
   (`ev_4948ed69…`).

# OJNList trailer `;HD` semantics: patcher manifest, dead to the 7.55 client (claim C40 / #96)

Evidence: static sweep (objdump + byte scans) of
`install-root/O2Jam/OTwo.exe` (`sha256 83a04541…`, file offsets = VA −
`0x400000`) against `install-root/O2Jam/Image/OJNList.dat`
(`sha256 3be8ca00…`, 215848 bytes). No REA dossier opened this pass
(dossier TBD). Reader-district base claims: C14/C15.

## File structure (recap, VERIFIED data walk — `docs/file-formats/ojn.md`)

The old "719 × 300 B + 148 B trailer" framing is wrong. The file is
562 × 300-byte song records `[0, 168600)`, mid region `[168600,
195616)` (7 parts incl. a 10-word footer at `[195576, 195616)` =
`[2, 173, 50, 0, 166, 50, 0, 0, 562, 887]`), then a **patch index**
`[195616, 200104)` of 561 × 8-byte `(marker, song id)` pairs (markers
`0x43f403` ×74 / `0x43f407` ×487, ids 886..100 sparse), then a **patch
section** `[200104, 215848)`: 28-byte header `{marker 0x43f407, count
562, top id 887, "2009-10-29", tag 77 30 f7 12 00}`, 561 × 28-byte
records `{`;HD\0`, const `0xbd11`, patch id, `YYYY-MM-DD`, same tag}`,
and an 8-byte truncated 562nd slot. This reads as a patch-added-songs
manifest (song id ↔ patch date), i.e. data written by the updater.

## What the 7.55 reader actually does past the records

Sequential byte accounting through the reader `FUN_00479e90` district
(all reads go through the `ReadFile` wrapper `FUN_004069b0`, handle at
stream `+0x104`, IAT `0x63f2e0`; no seeks exist in the district):

| Phase | Reads | Bytes consumed | Position after |
|---|---|---|---|
| count + 562 records | 4 + 300×562 | 168604 | 168604 |
| mid tables 1–5 | 4+16×204, 4+20×562, 4+16×407, 4+12×120, 4+8×562 | 26872 | 195576 |
| footer | count(=2) + 2×12 | 28 of the footer's 40 | 195604 |
| pairs | count u32 @195604 = **0** → loop body never runs | 4 | 195608 |
| records | count u32 @195608 = **562** → 562 × (4+24) | 4 + 15736 | 215348 |

Two findings fall out:

1. **The pair loop is dead.** Its count is the footer's 8th word, which
   is `0` on the shipped file. The 561 `(marker, id)` pairs are never
   read. The loop body (8-byte pair read, song-id match over the
   `0x1b4`-stride vector at `0x77952c`, marker byte stored to
   `song+0x1a8` at file `0x47b0ac`) has no execution path on this data.
2. **The 28-byte record loop is vestigial and misaligned.** Its count
   is the footer's 9th word (`562`), and records start at file offset
   195612 — the footer's 10th word — i.e. 12 bytes *before* the pair
   region and 492 bytes before the real `;HD` records. Each record goes
   to `FUN_0047b370`, which expects an **older record layout**
   `{u32 song id; char date[]; ...}`: it scans all songs for
   `record+0 == song id` (`cmp` VA `0x47b409`), then `strtol`-parses
   `record+4` (callee `0x5f1c4b`, separator word at `0x647a28` = `'-'`)
   up to 3 integers, and on success stamps `song+0x164` = year
   (tm_year+1900, `0x76c`), `+0x166` = month+1, `+0x168` = day from a
   `tm` struct (`0x5f762a`/`0x5f7994`), then makes a per-song archive
   call whose failure string is `"OJNLIST.DAT 패치 실패!"` (VA
   `0x6479f8`). Against the shipped bytes this never fires: stride-28
   records from 195612 never align with real `;HD` records
   (200132 − 195612 = 4520 = 28 × 161.43…), the only records whose
   `+0` equals a song id (k = 0 → id 887; odd k → pair-id slots) have
   `+4` = pair-marker bytes `03/07 f4 43 00` — not digits, so the
   `strtol` endptr gate (`0x47b440`) skips them — and every record
   overlapping a real `;HD` record has `+0` = `;HD\0`/`0xbd11`/date
   text, never a song id. The stream stops 500 bytes short of EOF;
   nothing is stored.

## Verdict

`;HD` semantics settled: the trailer is a **patch manifest authored by
the updater** (per-song patch ids + dates, plus marker/id pairs), and
for the 7.55 client it is **dead data** — the reader district's tail is
vestigial code written for an older `{id, date}` record layout, driven
by footer echo words (`0` pairs, `562` records), misaligned with the
shipped `;HD` records, and passing its own id + date gates zero times.
Negative-evidence close with named paths, per the ticket's option B;
this also retires the "marker semantics OPEN" question for the patch
index (its reader is the never-executed pair loop). `o2jam-file`
keeps `parse_ojn_patch_index` / `parse_ojn_patch_section`
(`tables/patch.rs`) as file-format documentation, now labeled
dead-data rather than client-consumed.

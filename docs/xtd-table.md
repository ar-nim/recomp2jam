# The "xTd paired table" is the song-search key-name table (claim C39 / #95)

Evidence: static sweep (objdump + byte scans) of the read-only 7.55 client
`install-root/O2Jam/OTwo.exe`, `sha256 83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
File offsets are VA − `0x400000`. No REA dossier was opened for this pass;
every statement below is re-derivable from the bytes (REA dossier TBD).

## What the inventory saw

`docs/systems-survey.md` inventoried file `0x2454d8`–`0x2455dc` as
"`?Td/?Sd/?Bd` codes + u16 rows + pointer table". That reading is an
artifact: the region is **65 × `u32` VA pointers** (260 bytes,
`0x2454d8`–`0x2455db`), and every pointer value is a VA in the
`0x63xxxx`–`0x645xxx` range, so the shared low bytes `54 64` / `53 64` /
`42 64` spell `Td`/`Sd`/`Bd` in a naive ASCII scan. There are no codes.

Decoded entries (index → VA → target file offset → text, CP949):

| Entries | VAs | Texts |
|---|---|---|
| 0..=13 | `0x6454d0`…`0x64549c` | chosung initials ㄱ ㄴ ㄷ ㄹ ㅁ ㅂ ㅅ ㅇ ㅈ ㅊ ㅋ ㅌ ㅍ ㅎ |
| 14..=18 | `0x645498`…`0x645488` | doubled initials ㄲ ㄸ ㅃ ㅆ ㅉ |
| 19..=23 | `0x64541c`…`0x6453fc` | 전체 초급 중급 고급 최고급 (STEP1 band names) |
| 24..=49 | `0x6442a4`, `0x245484`… | `A`..`Z` (letters `B`–`Z` in 4-byte slots at file `0x245420`–`0x245488`; `A` is out of line at file `0x2442a4` — the lone genuine `?Bd`-looking pointer) |
| 50..=56 | `0x64541c`, `0x6453f4`… | 전체 신곡 골드 블랙 블루 레드 실버 (STEP2 names) |
| 57..=64 | `0x64541c`, `0x6453c4`… | 전체 발라드 락 댄스 힙합/R&B 재즈 클래식 기타 (STEP3 names) |

Entries 19/50/57 alias one string (전체). Each target is a 1–12 byte
NUL-padded label; the string pool at file `0x245390`–`0x2454d8` holds
the words, the 4-byte slots at `0x245420`–`0x245488` the letters, and
file `0x245488`–`0x2454d8` the jamo (EUC-KR lead byte `0xA4`).

## Consumer classification (the ticket's verdict target)

Only **two direct references** to the table base VA `0x6454d8` exist in
the image (dword scan for `d8 54 64 00`): file `0x448c2` and `0x44ecc`
— `mov 0x6454d8(,%edx,4),%eax`, the indexed load of the `+0x680` keyed
rebuild in the SELECTMUSIC search filter. The remaining inventoried
refs name other addresses in the region:

- **Search filter (in-scope, song-select dialog).** Both base sites load
  entry `[arm]` for arms 0..=13 and `_strnicmp` it (n = 2, callee
  `FUN_005f3f0c`) against the CP949-converted song title/artist
  (`FUN_00406100`, called at `0x4448af`/`0x444eb9`), appending matches
  through `0x452850`. Arms 0/2/5/6/8 then test their fixed doubled-initial
  slot (`0x645510`–`0x645520`, entries 14–18, loaded at
  `0x44973`–`0x44c4b`). EUC-KR lead-byte range gates (`0xb0`–`0xc8` /
  `0xa1`–`0xfe`) precede both sites.
- **Letter search.** First-byte `A`–`Z` check at file `0x4451af`, indexed
  load `[ecx*4+0x645538]` (entry-24 base) at file `0x451ed`, `_strnicmp`
  n = 1; second site at file `0x4545f` (VA `0x444fe0`).
- **Result banner.** Sub-range bases `0x645524` (entry 19), `0x6455a0`
  (entry 50), `0x6455bc` (entry 57) indexed by the dialog fields
  `+0x6bc`/`+0x6c0`/`+0x6c4` at files `0x46fef`, `0x476b9`–`0x476cd`,
  `0x47dea`–`0x47e12`; banner formats `"%s 검색 결과입니다."` and the
  two/three-key variants (VAs `0x645a50`/`0x645a64`/`0x645a80`).
- **Out-of-line `A`.** The only direct reference to any label string:
  `push $0x6442a4` at file `0x41c7ef` (a 1-char CP949 classify helper).
- **Adjacent pointer table (separate structure).** File `0x2455e0`
  begins a 35-entry function-pointer table (targets in `.text` plus one
  `.data` pointer, followed by the `"#-5000012"` literal at `0x24566c`).
  It is stored to object `+0x3c` by constructors at files `0x43cfb2` /
  `0x43d190`. It is a dialog vtable/handler table, **not** part of the
  key-name table, and not a consumer of it.

No launcher/patcher reference exists anywhere in the set.

## Verdict

The table is the **song-search key-name table** of the SELECTMUSIC
dialog: filter keys (chosung + doubled initials, letters) and the
STEP1/STEP2/STEP3 banner names. It feeds an in-scope surface, so it is
not ruled out; the decode is claim **C18** (cell `ROOM.COMP`), and this
pass independently re-verifies C18's reading end to end (all 65 entries
decoded, both base refs and every sub-range consumer named from bytes).
The C18 "key-name" refutation in the old inventory concerned *keyboard
key names*; as a search-key name table the guess stands corrected.
Landed in `o2jam-file` as `parse_key_name_table` +
`key_name_filter_entry` / `key_name_letter_entry` with per-arm synthetic
tests (`crates/o2jam-file/src/tables/keys.rs`). C39 closed as EXE;
the STEP2 seven-names-vs-six-buttons mismatch stays OPEN inside C18.

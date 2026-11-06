# OJM (music container) — M30 + OMC VERIFIED, OJM partial

Verified by walking real 7.55 files byte-exact to EOF (scripts in commit
history; re-runnable against `install-root/O2Jam/Music`, which is local-only).
All payloads are raw OGG Vorbis (`OggS` page chains, EOS-terminated) —
**no encryption at rest** in M30/OMC.

## M30 variant — VERIFIED (`o2ma*.ojm`, 13 files)

Header (20 bytes):

| Off | U32LE | Meaning |
|---|---|---|
| 0 | `4D333000` | magic `"M30\0"` |
| 4 | `0x00030000` | version/flags? (same in all 13) |
| 8 | 0 | ? |
| 12 | N | sample count |
| 16 | 28 | first-entry offset |

Then N entries, each immediately followed by its OGG (inline, no central
table). Entry = `name[32] + u32 ogg_size + u32 codec + u32 flag +
u16 ref + u16 reserved + u32 pcm_bytes?`, 52 bytes total; OGG starts at
entry+52. Field split corroborated by the open2jam OJM documentation
(`M30_sample_header`: `ref`/`unk_zero`; see Strike-slot mapping below):
codec is `0x0005`/`0x0200` as halves of `0x00020005`, flag is `1`, and
`ref` is the sample id the chart addresses (`reserved` reads 0 on all
shipped entries; `parse_m30` reads only the `ref` word).

Proof: `o2ma757.ojm` walks 25 entries to exactly EOF (749812), every
stream opens with `OggS`, and every stream page-walks to exactly one EOS.
Sample IDs drive chart mapping: drum hits `0–12`
(`SYNPC_Crash_01`, `SYNPC_Kick_021`, `Tom1–5`, `[WAV040]stick`…), music
tracks `980–991` (`[WAV980]mr1` … `[WAV991]mr12`). `pcm_bytes?` ≈ decoded
size (e.g. 1365680 for the mr* tracks).

## OMC/OJM container — UNIFIED model, VERIFIED on all 21 files

16-byte header: magic (`M30`/`OMC`/`OJM`), flags, `20`, first-entry
offset at +12, then 36-byte entries (`name[32] + u32 size`) each
followed by its OGG. All-zero 36-byte slots are skipped. M30 uses
52-byte entries instead (extra id/pcm fields) with the first entry
at the +16 offset; same walk discipline.

Verified end-to-end (entry walk + per-stream OggS page walk with
EOS, landing exactly on EOF):

| File | Streams | Notes |
|---|---|---|
| `o2ma*.ojm` (13× M30) | 25/25/714/517/34/22/52/52/505/86/86/119/119 | counts match header field |
| `BGM.ojm` | 7 | `bgRoom`…`bgMusicShop`; blanks skipped |
| `BGMwinter.ojm` | 10 | `BG-01`…`BG-06`, `bgResult`, `bgLogo`… |
| `bgEffect.ojm` | 1 | `SampleSong.ogg` + encrypted body |
| `Planet.ojm` | 1 | `ochestra(1).ogg` + encrypted body |
| `Tutorial.ojm` | 12 | raw tail (`bgsplit-01.ogg`…); 1.8 MB encrypted head |
| `Event.ojm` | 1 | garbage name field, valid stream |
| `DialogBGM.ojm`, `O2PlanetNPC.ojm` | 0 | first-entry offset == filesize: fully encrypted |

Sample IDs (M30) drive chart mapping: drum hits `0–12`
(`SYNPC_Crash_01`, `SYNPC_Kick_021`, `Tom1–5`, `[WAV040]stick`…),
music tracks `980–991` (`[WAV980]mr1` … `[WAV991]mr12`).

RESOLVED (this branch): new-variant bodies decode per the chart's
+8 version float (`ojm_descriptor_for_version`: 2.9 → 17-way,
2.7 → 16-way, ≤2.4 → plain) — mode-2 to raw PCM16 with entry
header rate/channels (`decode_new_variant_pcm`), mode-3 as raw
OGG (`recover_new_variant_sample`, checksum-validated).
DialogBGM odd-stripe puzzle stays TBD (documented file anomaly,
uncompensated). O2PlanetNPC is plain PCM (`OJM` magic → flags 0).
Event's garbage name field still unexplained. Stream counts in
the table above are old-variant-walk views; new-variant mode-2
+ mode-3 counts: bgEffect 40+1, Planet 4+1, Tutorial 30+12,
DialogBGM 1+0, O2PlanetNPC 7+0, BGM/BGMwinter 0+10/0+10,
Event 0+1.

## In-exe OJM loader — MAPPED (addrs VA; `ev_1601a757…`)

- Sniffer `0x54e7c0`: memcmp magic vs `M30`→`0x30000` else
  `OMC`→`0x20000+minor` / `OJM`→`0x20000+minor`, minor from a
  float version arg (`(int)(v+0.01)<<16 | (int)(frac(v+0.01)*10)`).
- Init `0x54e930`: M30 reads `0x1c` B at obj+`0x108`; OMC reads
  `0x14` B at +`0x168`, count = words at +`0x16c`/`0x16e`.
  Flag bits at +`0x110`: `|2` if desc ≥ `0x20008` (17-way),
  else `|1` if ≥ `0x20006` (16-way) — the `|2` arm jumps over
  the `|1` check; `|4` if ≥ `0x20005` (bitwise-NOT). So
  OMC-via-lobby (`0x20009`) = 17-way + NOT; OJM (`0x20000`)
  and low versions = plain reads. (`0x8` mirror / `0x10` XOR
  bits have no writers in 7.55 — vestigial.)
  The song path passes a per-song version float instead of 0.0,
  so the descriptor (hence lane count) is per-song: version
  2.8+ → 17-way (bgEffect/Planet/DialogBGM), 2.6–2.7 →
  16-way (Tutorial pads, verified 0.99), ≤2.4 → plain
  (O2PlanetNPC/Event raw PCM+Ogg). Magic is only consulted
  when the float is exactly 0.0.
- Entry fetcher `0x54eca0` (all plain `0x412620` reads): mode 1 =
  `0x34` B entries (M30), mode 2 = `0x38` B then `memcpy 0x20`,
  mode 3 = `0x24` B entries then `memcpy 0x20`.
- Sample dispatch `0x54f580`: flags `&1` → 16-way de-interleave
  `0x54f860`, `&2` → 17-way `0x54f920`, else plain read; then for
  any mode, `&4` → bitwise-NOT stream `0x54f9e0`, `&8` →
  byte-mirror S-box `0x54fa70` (table at file `0x27f618`, self-inverse
  bit-reversal), `&0x10` → u32-XOR `0x54fac0` with key `"nami"`
  (`0x24ce1c`). 16/17-way interleave tables at file `0x27f1d0` /
  `0x27f3d0` are exact permutations (remainder-selected rows).
  Mode-1 samples force NOT keys (`0xFF`,`0x00`); mode-2/3 run the
  zeroed schedule (keys differ from mode-1 only in the first 8
  decoded bytes).
- Mode-2 entry = open2jam `OMC_WAV_header` (56 B), verified on
  DialogBGM/bgEffect/Planet/Tutorial/O2PlanetNPC (blanks excepted):
  `name[32] + u16 audio_format(1) + u16 num_channels(1/2) +
  u32 sample_rate + u32 bit_rate + u16 block_align + u16
  bits_per_sample(16) + u32 reserved(0) + u32 chunk_size` at +52.
  Byte rate checks as `rate * channels * 2` on every shipped entry
  (e.g. 176400 on stereo 44100 Hz bodies, 64000 on mono 32000 Hz
  brass, 88200 on mono 44100 Hz / stereo 22050 Hz). Mode-2 bodies
  decode to raw PCM16 in that rate/channel layout
  (`decode_new_variant_pcm`); mode-3 bodies are raw OGG.

## Strike-slot mapping — VERIFIED (objdump static analysis; file addresses = VA − `0x400000`; no REA dossier IDs issued)

How a chart strike `sample_id` (`decode_note_value`: `value − 1`)
selects an OJM entry. Short version: by cache slot, and only M30
entries carry an id — OMC/new-variant entries have no id field, so
the game cannot address them individually.

- Play time (gameplay `0x5703f0`, file `0x1703f0`):
  `slot = (payload & 0xFFF) − 1`, plus `0x3E8` (1000) when the
  payload sets bit 26. Play helpers (`0x551b20` song cache, bound
  `0x7d0` = 2000 slots at mgr+`0x34220`; `0x551870` UI cache, bound
  `0x34`) index the decoded-sample array by that slot directly —
  no id→entry translation happens at play time.
- Load time (song-bank bulk loader, file `0x1734xx`; UI-bank
  loader `0x550db0` with per-bank base slots from the 7-entry table
  at file `0x27f728`): each fetched entry registers at
  `slot = entry_id` (`+0x3E8` when the entry's +`0x28` word sets
  bit 31, gated on a chart-built usage byte table). For M30 the
  entry buffer holds the full 52 B entry, so `entry_id` =
  entry+44 `ref` and size = entry+48 — every M30 id lands in its
  own slot (drums `0–12`, music `980–991`; Tutorial-style +1000
  slots stay empty on M30 banks because no shipped M30-paired
  chart sets bit 26).
- New-variant entries carry no id: the fetcher copies only the
  32 name bytes into the entry buffer (`strncpy` `0x5f2ad0`), so
  buffer+`0x28`/`0x2c` read as the init-zeroed bytes for every
  mode-2/3 entry. Every new-variant entry therefore registers at
  slot 0 (last entry wins); only `sample_id` 0 can sound, and the
  12 bit26 Tutorial intro notes (slots 1000–1011) address empty
  slots. No order-based or name-based lookup exists anywhere on
  the load/play paths (names are only copied, never compared;
  the fetch-index field at obj+`0x150` is written but never read).
- C28/C29 verdict (re-confirmed by byte-pattern survey on the pinned
  build, SHA-256 `83a04541…72c823b`, 5,029,888 B; python3 3.14 byte
  scan, hand-decoded, no dossier): selection is id-keyed end to end.
  Strike fn at file `0x1703f0` derives the slot from the payload
  alone: mask `&0xFFF` at `0x170418`, `−1` at `0x17041e`, bit26 test
  at `0x170427`, `+0x3E8` add at `0x170431`. The loader registers at
  the entry id `[entry+0x2C]` (reads `0x173625`/`0x17364a`, usage
  gate `byte[table+id]` `0x173628`, bit31 test `0x17361a`,
  `id+0x3E8` add `0x17364f`). The fetcher's name-only strncpy (call
  sites `0x14ee4a`/`0x14ef21`) is what zeroes id-less entries onto
  slot 0 — no order lookup exists. The order-keyed
  `resolve_order_slot` stays our invented fallback; exe selection is
  id-keyed end to end (C29, EXE — strike chain `0x1703f0` → `0x170431`,
  loader `0x173625`/`0x17364a`, fetch `0x14ee4a`/`0x14ef21`).
- Corroboration: the open2jam OJM documentation
  (`https://open2jam.wordpress.com/the-ojm-documentation/`)
  independently documents the same header, `OMC_WAV_header`
  (56 B) / `OMC_OGG_header` (36 B) split, WAV→key / OGG→BGM
  section roles, and the M30 `ref` word — matching this map.

Reconstruction fallback (ours, not exe behavior): id-less banks
key by fetch order — `OjmTables::resolve_order_slot`: slot N names
the Nth entry (mode-2 then mode-3), with the `0x3E8` high bank
stripped to the same order. On Tutorial.ojn/ojm this sounds every
strike: lanes `0–29` → 30 key sounds, plain kind-9 id 30 →
`bgsplit-01`, bit26 ids `0–11` → key sounds `0–11` again.
Autoplay: o2ma757 L0 618/619 sounded (M30, unchanged), Tutorial
L0 51/51 (was 1/51 before the fallback).

OPEN: DialogBGM's single 13 MB sample decodes with even plain
stripes clean and odd stripes pairwise byte-swapped (17/17 stripes
audio, 0.5–0.9 corr). Exhaustive negative evidence: no `BSWAP` /
`XCHG` / word-swap on sample buffers anywhere in the load, decode
(`0x54f580` `ev_fdb31596…`, `0x54ef60`), PCM (`0x54d320`
`ev_c5a69…`, plain memcpy), Vorbis (`0x54db40` `ev_0ee1373d…`),
read (`0x4125b0` `ev_55fd256b…`, plain `ReadFile`), or setter
(`0x54b9f0` `ev_45cf9965…`) paths; swap/rows/transpose/mirror/XOR
variants all ruled out on the file. Verdict: encoder-side file
anomaly (odd stripes stored swapped in the shipped file), not a
game behavior — the reimplementation reproduces the exe's bytes
exactly and documents the file as anomalous. No code compensates
for it.

### Odd-stripe byte-swap anomaly — still OPEN (3 exe leads exhausted)

DialogBGM.ojm's single 13 MB mode-2 sample decodes by the verified
chain (17-way scatter per row `size % 17`, then NOT with keys
`0xFF`,`0x00`) to PCM where even destination stripes correlate as
speech (~0.73–0.85 adjacent-sample) while odd stripes sit near 0.00
and correlate (~0.75–0.81) only after a pairwise byte-swap;
bgEffect.ojm mode-2 sample 0 shows the same even-plain/odd-swapped
split (0.96 overall). Searched the exe for the swapping code and
found none — no `BSWAP`/`XCHG`/word-rotate on sample buffers in:

1. Sample dispatch `0x54f580` (file `0x14f580`, dossier
   `ev_fdb31596…`), de-interleavers `0x54f860`/`0x54f920`, and the
   transform trio `0x54f9e0`/`0x54fa70`/`0x54fac0` (NOT / mirror
   S-box / u32-XOR only).
2. Full-coverage loader `0x54ef60` (file `0x14ef60`, dossier
   `ev_f96b0fe5…`) and its codec targets: PCM decode `0x54d320`
   (dossier `ev_c5a69874…`, header memcpy + body memcpy, no swap)
   and Vorbis decode `0x54db40` (dossier `ev_0ee1373d…`).
3. File-read chain `0x412620` → `0x4125b0` (dossier `ev_55fd256a…`,
   plain `ReadFile`) and the setter region near `0x54b9f0`
   (dossier `ev_45cf9963…`, free/malloc only).

Per-stripe/per-row/order/transpose variants stay ruled out; cause
TBD — no Rust change made (unverified transforms stay out of
`o2jam-file`).

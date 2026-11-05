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
table). Entry = `name[32] + u32 ogg_size + u32 0x00020005 + u32 1 + u32
sample_id + u32 pcm_bytes?`, 52 bytes total; OGG starts at entry+52.

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

OPEN: new-variant encrypted bodies — loader mapped below;
`parse_new_variant_entries` + `not_decrypt` implemented, full
body recovery verified on bgEffect s0, DialogBGM odd-stripe
puzzle TBD. O2PlanetNPC is plain PCM (`OJM` magic → flags 0).
Event's garbage name field still unexplained.

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
  Mode-1 samples force NOT keys (`0xFF`,`0x00`).

OPEN: NOT-key source is settled (forced `(0xFF,0x00)` at rewind
for OMC, `+0x164`/`+0x165`; keys chain across samples). New-variant
table layout VERIFIED and implemented in o2jam-file
(`parse_new_variant_entries`, `not_decrypt`): counts u16 at +4/+6,
table offsets u32 at +8/+12, mode-2 entries `0x38` B (32-B name +
params, size u32 at +52), mode-3 entries `0x24` B (old 36-B shape),
`[entry, data]` chaining per sample (bgEffect's `SampleSong.ogg`
mode-3 entry at its +12 offset confirms the model). 17-way + NOT
recovers bgEffect sample 0 cleanly (0.97 adjacent-sample
correlation). Remaining: DialogBGM's 13 MB single sample decodes
with even plain-stripes clean and odd stripes noise-like under the
same chain — per-stripe/per-row/order/transpose variants ruled
out; cause TBD. Dossiers `rea-samplefetch.json` (`ev_1601a757…`)
+ dispatch (`ev_fdb31596…`) banked.

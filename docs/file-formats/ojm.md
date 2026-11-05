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

OPEN: the encrypted bodies (all of DialogBGM/O2PlanetNPC, heads of
bgEffect/Planet/Tutorial) and Event's garbage name field — same
unknown transform family as the `"new"` `.ojn` headers; the in-exe
OJM loader is the next REA target after the OJN decrypt.

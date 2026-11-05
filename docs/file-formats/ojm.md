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

## OMC variant — VERIFIED (`BGM*.ojm`, `Planet.ojm`, `bgEffect.ojm`, 5 files)

File header (128 bytes): magic `"OMC\0"`, u32s `(?, 655360?, 20, 20,
filesize, 0, 0, 0)`, first-stream `name[32]` at ~64 and `u32 size` at 124.
Then entries of `name[32] + u32 size` (36 bytes) each followed by its OGG.
Blank (all-zero) 36-byte slots can occur (1 in `BGM.ojm`).

Proof: `BGM.ojm` walks 177 streams + 1 blank slot to exactly EOF
(12901507), all EOS-terminated. Names are BGM cues (`bgRoom.ogg`,
`bgItemShop.ogg`, `bgMyRoom.ogg`, `bgLogo.ogg`…).

## OJM variant — PARTIAL (`Event.ojm`, `Tutorial.ojm`, `O2PlanetNPC.ojm`)

- `Event.ojm`: 56-byte header + ONE OGG stream to EOF (174 pages, 1 EOS).
  Verified same as above.
- `Tutorial.ojm` / `O2PlanetNPC.ojm`: 16-byte header
  (`"OJM\0"`, `0x000c001e`?, 20, big_offset, filesize…). `Tutorial.ojm`
  has entry `gapiano1` at offset 20, then ~1.8 MB of high-entropy bytes,
  then 12 plaintext `name + OGG` runs to EOF (`bgsplit-01.ogg`…).
  The 1.8 MB region is likely encrypted/obfuscated — OPEN: find the
  transform (compare with the in-exe OJM loader; maybe the same XOR family
  as the `.ojn` `"new"` headers).

## OJN note (moved here from install-layout)

All 14 `.ojn` start `6e6577…` (`"new"` + varying bytes, then a repeating
4-byte pattern) — same obfuscation smell as Tutorial's 1.8 MB region.
`Image/OJNList.dat` starts `3202 0000 6400 0000 6f6a6e00…`
(possibly 562 entries + `"ojn\0"`) and may be the plaintext index —
cross-check after decoding one file.

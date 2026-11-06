# Progress / compatibility matrix

All versions are **7.55** unless noted. Update one row per finding.

## Binary inventory (7.55, extracted via Wine-less carve + unshield)

| File | Size | SHA-256 | Notes |
|---|---|---|---|
| `O2Jam_Classic.exe` (installer) | 140,350,480 | `a7247c49…3778a3` | InstallShield 11.5 stub; never committed |
| `NowSmartLauncherInstaller.exe` | 2,041,051 | `6ad98f53…6261eac` | InstallShield 11.5 stub; never committed |
| `AngelO2Jam.exe` (installed) | 5,029,888 | `83a04541…72c823b` | game client; PDB `d:\O2JamSvn\trunk\Client\O2Jam_Client(2005)\Release\OTwo.pdb` |
| `OTwo.exe` | 5,029,888 | same as above | byte-identical twin of AngelO2Jam.exe |
| `O2Net32.dll` | 53,248 | `190b66db…7298d3` | net lib |
| `Player.dll` | 315,392 | `dce00db4…789379` | |
| `SendWebData.dll` | 241,664 | `ff65b310…22a913` | telemetry? |
| `NowSmartLauncher.ocx` | 462,848 | `ff8868ed…a19ca` | MFC ActiveX; patch URL `http://%s/O2Jam/pub/O2Jam/%s/%s%2.1f.zip` |
| `Image/OJNList.dat` | 215,848 | `3be8ca00…a07fc` | chart index? starts `3202 0000 6400 0000 "ojn\0"` |
| `Music/` | 35 files | — | 21 `.ojm` (magics M30/OMC/OJM) + 14 `.ojn` (all start `"new"`, obfuscated?) |

## Systems

| System | Original (7.55) | Rust status | Parity test |
|---|---|---|---|
| OJM parse (M30) | inline 52B entries + raw OGG | ✅ implemented + tested (`parse_m30`, 25-stream real-file check) | needs golden hash test |
| OJM parse (OMC) | 36B entries + raw OGG | ✅ implemented + tested (`parse_omc`, 7-stream real-file check) | needs golden hash test |
| OJM parse (new-variant) | counts+offsets header, `0x38`/`0x24`B entries, 17-way + NOT | ✅ entries + named WAV params + descriptor + PCM/OGG recovery implemented + tested (`parse_new_variant_entries`, `ojm_descriptor_for_version`, `decode_new_variant_pcm`, `recover_new_variant_sample`; bgEffect s0 0.97, Tutorial pads 0.99) | DialogBGM odd-stripe swap = encoder-side file anomaly (no in-exe cause; documented, uncompensated) |
| OPI/OPA parse | 16B header + trailing 152B-entry table | ✅ implemented + tested (`parse_opi(_slots)`; 1587/627/7133 real-file counts, 9/6/16 free) | inner formats mapped: CP949 text, sprite-55 + strided combo variant, .bnd rects, ControlList↔rect relation (38/38 Interface); TBD: outlier pixel codecs, .oja frame data, Playing rect subset |
| OJN parse (old `"ojn"`) | ~300B header + blocks | header map + 43-event walk verified | per-level ranges TODO |
| OJN parse (new `"new"`) | reverse + 3-key XOR (period stride) | ✅ decrypted (757: magic/id/BPM/580 events) + `decrypt_ojn_new_format` tested | ✅ per-level segmentation mapped + implemented (`parse_ojn_levels`; 14/14 walks land exact, OJNList +40-group cross-check 13/13) |
| OJN note semantics | i32 measure + kind classes + `(flags<<16)|(sample+1)` | ✅ implemented (`ojn_lane_of_kind`, `decode_note_value`, `parse_ojn_level_events`, `build_level_chart`; 39/39 playable/total counts vs header +40/+28 groups, lanes 12/13 sample-cover) | flags (hold?) UNVERIFIED; in-exe bit26 sites `0x172a4f`/`0x173061` recorded, never fire on shipped payloads |
| Vorbis decode | libvorbis | ✅ `lewton` decode + `cpal` backend w/ sample clock (bgRoom 53.8s parity) | mixer + latency probe TODO |
| Song clock | sample-accurate audio position | ✅ `CpalBackend::samples_played` clock + `FrameClock` 60Hz interp | wire loop to clock TODO |
| Judgement | `FUN_00576540` (file `0x176540`): abs `|song-note|` vs 6/18/25 ms → flags `0x10/0x20/0x40/0x80` (sites `0x176646/55/64` + `0x176b7f/8e/9d`; dossiers `ev_14992eb2…`/`ev_5ecb1ce8…`, instr `ev_b492dfe0…`/`ev_c9145d1a…`/`ev_da09b5c7…`/`ev_c31f6413…`/`ev_7611dac8…`/`ev_3db1071b…`; converter `FUN_0057a240` `ev_9f8e1c2b…`/`ev_f170d5f6…`) | ✅ implemented + tested (`TimingWindows{6,18,25}`, `windows_judge_verified_7_55_tiers`) | grade strings `0x247363+` code-unreferenced (confirmed, zero refs); `LongnoteMode` = option.ini display; long-note combo path present but points OPEN |
| Score/combo | Cool 200 + Good 100 + Bad 4 + Miss −10 (floored) VERIFIED in `FUN_00570510` (arg 4/3/2/1 paths; BAD→COOL conversion verified); combo/pill rules VERIFIED | ✅ implemented + tested (verified bases; `CoolStreak`, `PillStock`, `apply_rated`) | variable `+10X`/`+5X` terms pending `+0x396e` identity; long-note bonus OPEN |
| Jam gauge | UI anchors in-exe (7.55): `Jamgauge_{right,left}_bar.ojs` `0x64b1b8/0x64b1d0` (`ev_2ff06089…`; xrefs `ev_9a778412…`/`ev_d006b5ee…` → loader `FUN_004e7850` `ev_40f99221…`), `Playing_Effect_Jam` (`ev_a17b38d2…` → `0x4e822a/0x4ee9dc/0x4f2d5a`), `Note_JamNum.ojs` (`ev_49a71dd5…` → `0x50d0ae`); gauge math player-reported, UNVERIFIED in-exe (Cool +2, Good +1, 30/jam, overflow carries, Bad/Miss feed nothing) | ✅ model implemented + tested (`JamGauge::{feed,progress,jam_count}`, `JAM_PROGRESS_PER_{COOL,GOOD,JAM}`; overflow-carry + ignore-Bad/Miss tests) | gauge variable + `+2/+1`/`30` sites NOT located (rating hides behind `+0xa14` callback target, unidentified); Bad/Miss progress handling OPEN |
| Pill / jam score | pills earned by 15 consecutive Cools (in-exe VERIFIED, `+0x3990` streak, cap gate, reset-on-Bad), spent on Bad only (Miss spends none — verified absence); Bad-with-pill scores full Cool base | ✅ implemented + tested (`CoolStreak`, pill-save-only-Bad) | shop earn flow untraced; long-note bonus OPEN |
| Game loop | autoplay sequencer + rated scoring | ✅ `Sequencer` + `play --chart/--ojm/--sound` (o2ma757 L0: 619 triggers, 618 sounded, score 123800; Tutorial L0: 51/51, score 10200) | renderer blits TODO |
| Renderer | DirectDraw/CDX path: DDRAW!DirectDrawCreateEx import, CDXSurface/DXSurface RTTI, D3D texture upload GetDDS/GetDDSAlpha, Blt/BltFast/Flip vocabulary; sprite-55 RGB555+colorkey confirmed as blit source; mode/callsites TODO | not started (doc-only survey) | screenshot diff TODO |
| Rating-event sink (`+0xa14`) | IDENTIFIED static: attach `0x56e170` at song-load (`0x570325`←`0x4dba..`), detach on chart reload (`0x1717cc`), 16-way dispatcher w/ jump table, event codes 1/9/14 (plain/hold/?) at judgement sites `0x576691+/0x576be0+/0x576eaa+` | not started | per-code semantics OPEN |
| Life gauge / fail | UI mapped static: Life bars loaded in `FUN_004e7850` (`0x4e7d10/0x4e7ca9`); GameOver .ojs/.ojt variant-select in ~`0x4e5d90`; `+0x3980`=score, `+0x3970`=rating gate+gauge-update virtual; HP store + deltas OPEN; `0x75e844` row has zero image refs | not started | HP/fail rules OPEN |
| Long-note options | plumbing VERIFIED static: NoteType 0-2 + per-level Note0-2 strings (`+0xeb4`, `0x5713c7+`); NoteTail 0-4 global `0x67feac` (`0x56bf47+`); LongnoteMode cached `+0x17d8` at setup (`0x56fb0d+`); parser bit26 + judgement bit25/0xe-event | not started | tick scoring/render OPEN |
| Result screen | builder `FUN_004c2390` loads PointNum/WinLose/Top/Bottom art (`0x24a338+`); grade strings `0x247362+` provably ref-free full-image (mission-side?); SubmitScore/FinalizeRank fields known | not started | grade mapping OPEN |
| Shop/item/equip | tables located: SetInfodata.ojs (210 rows, avatar.opa+136063397) + Itemdata_Korea.dat (740945 B, +142934156, hdr `0606…01`); ring catalog + opcodes 5000-5041 already docs-side | not started | row strides/stats OPEN |
| Option keys (full table) | VERIFIED static `0x245224-0x2452a8`: MODE/Mode, NoteScreen/BlendHalf(=1), LONGNOTEMODE/LongnoteMode, NOTE/NoteTail, NOTE/NoteType | n/a (display) | binds/volumes/resolution elsewhere |
| xTd paired table | INVENTORIED static `0x2454d8-0x2455dc` (?Td/?Sd/?Bd codes + u16 rows + ptr table; 15+ code refs); likely key-name table | not started | decode OPEN |
| OJNList trailer | INVENTORIED static: 719x300B + 148B of 28B `;HD`/date entries (patch manifest?); per-diff levels +280-288, recsize +292 | not started | `;HD` semantics OPEN |
| Input | DirectInput TODO | ✅ winit keymap (SDF SpaceJKL) + `play --window` shell | latency budget TODO |
| Launcher/patcher | NowSmart | out of scope | — |

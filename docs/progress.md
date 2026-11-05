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
| OJM parse (new-variant) | counts+offsets header, `0x38`/`0x24`B entries, 17-way + NOT | ✅ entries + NOT implemented + tested (`parse_new_variant_entries`, `not_decrypt`; bgEffect s0 0.97) | DialogBGM odd-stripe puzzle TBD |
| OPI/OPA parse | 16B header + trailing 152B-entry table | ✅ implemented + tested (`parse_opi`; 1587/627/7134 real-file counts) | inner script/binary formats TODO |
| OJN parse (old `"ojn"`) | ~300B header + blocks | header map + 43-event walk verified | per-level ranges TODO |
| OJN parse (new `"new"`) | reverse + 3-key XOR (period stride) | ✅ decrypted (757: magic/id/BPM/580 events) + `decrypt_ojn_new_format` tested | per-level segmentation TODO |
| Vorbis decode | libvorbis | `lewton` helper | PCM compare TODO |
| Song clock | TODO (CDXTimer?) | `SongClock` stub (wall) | ±1 ms test TODO |
| Judgement | windows TODO | default windows, tested | values UNVERIFIED |
| Score/combo | formula TODO | placeholder formula | parity TODO |
| Renderer | CDX surfaces TODO | not started | screenshot diff TODO |
| Input | DirectInput TODO | not started | latency budget TODO |
| Launcher/patcher | NowSmart | out of scope | — |

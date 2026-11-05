# Progress / compatibility matrix

All versions are **7.55** unless noted. Update one row per finding.

## Binary inventory (fill after installing in a VM)

| File | Size | SHA-256 | Compiler stamp | Notes |
|---|---|---|---|---|
| `O2Jam_Classic.exe` (installer) | 140,350,480 | _TODO_ | InstallShield 11.5 stub | in this folder, never committed |
| `O2Jam.exe` (installed) | _TODO_ | _TODO_ | _TODO: dumpbin_ | |
| `AngelO2Jam.exe` (installed) | _TODO_ | _TODO_ | _TODO: dumpbin_ | name seen in installer strings |
| vorbis/ogg DLLs | _TODO_ | _TODO_ | | confirm static vs. sidecar |

## Systems

| System | Original (7.55) | Rust status | Parity test |
|---|---|---|---|
| OJM parse | header layout TODO | stub + magic sniff | `o2jam-file` round-trip TODO |
| OJN parse | header layout TODO | stub + magic sniff | `o2jam-file` round-trip TODO |
| Vorbis decode | libvorbis | `lewton` helper | PCM compare TODO |
| Song clock | TODO (CDXTimer?) | `SongClock` stub (wall) | ±1 ms test TODO |
| Judgement | windows TODO | default windows, tested | values UNVERIFIED |
| Score/combo | formula TODO | placeholder formula | parity TODO |
| Renderer | CDX surfaces TODO | not started | screenshot diff TODO |
| Input | DirectInput TODO | not started | latency budget TODO |
| Launcher/patcher | NowSmart | out of scope | — |

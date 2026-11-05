# Reversing workflow (7.55)

> Rule: document behavior in your own words. Never paste disassembly or
> decompiled original code into this repo.

## 0. Extract with Wine (no VM)

Use a dedicated prefix so the installer never touches your main `~/.wine`.
All output paths below are git-ignored — hashes and notes go into
`docs/progress.md`, never the extracted files themselves.

```sh
sudo pacman -S unshield   # InstallShield CAB extractor (Arch `extra`)

# Option A — inspect/list embedded cabinets without installing:
unshield -L O2Jam_Classic.exe   # if it reads the wrapper; else extract cabs first

# Option B — run the installer into an isolated prefix:
WINEPREFIX="$PWD/.wine-o2jam" wine O2Jam_Classic.exe
# installed tree lands under .wine-o2jam/drive_c/... (ignored)

# Option C — admin/extract-only (dumps files without registry churn):
WINEPREFIX="$PWD/.wine-o2jam" wine O2Jam_Classic.exe /a
```

Tip: plain `.cab` files unpack with `7z x data1.cab` once they are out
of the setup wrapper; `unshield x data1.cab` preserves InstallShield
components/groups. Record which method you used in your RE notes.

## 1. Inventory (fill `docs/progress.md`)

For every installed binary/DLL:

- file name, size, sha256
- `dumpbin /headers` (linker + compiler version stamp)
- imported DLLs (`dumpbin /imports` or Dependencies tool)
- embedded version resource + manifest (`MSVCR80` policy?)

Strings sweep for anchors: `CDX`, `boost`, `vorbis`, `Ogg`, `ojn`, `ojm`,
`DirectDraw`, `Direct3D`, `timeGetTime`, `QueryPerformanceCounter`.

## 2. Module map via RTTI + imports

- Ghidra/IDA: look for MSVC RTTI (`??_R*`) to recover class names —
  `CDX*`, file/audio, state/song classes become the crate map.
- Cross-reference `timeGetTime` / QPC (song clock), DirectDraw
  `CreateSurface`/`Blt`/`Flip` (frame loop), DirectSound/`waveOut`
  + `vorbis*` calls (audio path), file reads around `.ojm`/`.ojn`.

## 3. Per-system notes

Record each as a small doc PR with: question, method, finding, addresses
as file+offset (PIE-safe), confidence, and the parity test it implies.

Suggested order:

1. File formats (OJM container → OGG payloads; OJN charts) — feeds
   `o2jam-file` + `docs/file-formats/`.
2. Song clock + frame loop (who owns time? flip vs. audio?) — feeds
   `o2jam-game` song clock.
3. Input sampling (poll rate, key mapping, buffering) — feeds input crate.
4. Judgement/scoring (windows, long notes, combo, life) — feeds
   `o2jam-game` parity tests.
5. Rendering (resolution, color key, layering, effects) — feeds renderer.

## 4. Tooling

- Disassembler: Ghidra (free, scriptable) or IDA.
- Dynamic: x64dbg / Cheat Engine for timing values; WASAPI loopback +
  frame capture for parity measurements.
- Diffing: Diaphora / Bindiff when comparing 7.55 against nearby builds.

## 5. Definition of done per finding

- [ ] Behavior described in docs without original code
- [ ] Parity test or fixture in the matching crate
- [ ] `docs/progress.md` row updated

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

## 6. Capture provenance rule (owner decision 2026-11-16)

A live-behavior claim may cite a screenshot, frame, or recording as
**exe evidence** only when the capture carries a provenance manifest:

- exe version, file size, and sha256 of the binary that produced it;
- tool versions (Wine, Xvfb, capture tool, injector);
- display/session and the command that produced the capture;
- the capture's own content hash, plus the manifest hash;
- offsets/REA evidence IDs for any behavior read off the pixels.

Static byte scans of the pinned local exe are a different evidence
class: they need the binary hash and tool versions, but no rig. They
must NOT be presented as live captures, and a live claim may not be
settled by them alone.

An unqualified capture may appear in a note only as an **OPEN** lead
with the next evidence step named. It may never be cited as "the exe
shows X", never used to resolve a contradiction with the owner's live
observation, and never used to justify a behavior change. Reviewers
must open any cited capture with their own eyes (#252) AND check the
manifest before accepting the claim.

Rationale: #278 cited four Nov-8 PNGs as "Wine exe captures" with no
manifest while the Wine rig was still proof-pending (#240); the
2026-11-16 retro records a fabricated Wine A/B evidence episode. The
claim was retracted and the doc downgraded to OPEN.

Machine check (#281): `scripts/rig-provenance.sh emit` writes
`<out-dir>/provenance.json` from the run dir, `scripts/run-manifest.sh
verify` fails any capture that lacks a provenance entry, and
`scripts/rig-provenance.sh cite <out-dir> --claim exe` prints the citation
only for a complete manifest whose producer is the exe itself. Procedure:
`docs/rig-runbook.md` (Standard run).

# Installed layout (reconstructed, 7.55)

Rebuilt from the InstallShield 11.5 cabinets without running the installer:
carved `data1.hdr`/`data1.cab` out of each setup `.exe`, extracted with
`unshield` (built from source; single-exe volumes share one stream, so
`data2.cab` is a symlink to `data1.cab`), and arranged files by the CAB
group names — which match the client's own relative paths
(`Image\Interface1.opi`, `\music\o2ma…`, `.\Music\TEMP`, all seen in
`AngelO2Jam.exe` strings).

Local path: `install-root/` (git-ignored). Nothing under it is committed.

```text
install-root/
  O2Jam/                        # <INSTALLDIR> — game as installed
    AngelO2Jam.exe  (5,029,888, sha256 83a04541…)  # == OTwo.exe, byte-identical
    OTwo.exe        (5,029,888, same hash)
    O2Net32.dll     (53,248, sha256 190b66db…)
    Player.dll      (315,392, sha256 dce00db4…)
    SendWebData.dll (241,664, sha256 ff65b310…)
    Music/          (1035 files: 520 .ojm + 515 .ojn: system sounds,
                     BGM*.ojm, o2ma### pairs for 514 song ids 100..887)
    Image/
      Interface1.opi  (75,100,702)
      Playing1.opi    (221,737,026)
      avatar.opa      (144,761,749)
      OJNList.dat     (215,848, sha256 3be8ca00…)
  NowSmartLauncher/
    NowSmartLauncher.ocx  (462,848, sha256 ff8868ed…)  # MFC ActiveX patcher
```

## Evidence this layout is right

- CAB groups are literally named `Music`, `App Executables`, `Image`.
- The client opens `Image\Interface1.opi`, `Image\Playing1.opi`,
  `Image\Avatar.opa`, `(OJNList.dat)`, `\music\o2ma…`, `.\Music\TEMP`.
- The exe's PDB path is
  `d:\O2JamSvn\trunk\Client\O2Jam_Client(2005)\Release\OTwo.pdb` —
  SVN trunk, VS2005, binary project name `OTwo` (hence the identical twin).
- RTTI in the exe maps the engine: `CDXImageBMP/JPG`, `CDXSprite`,
  `CDXSurface`, `DXMesh/DXNTexture/DXSprite/DXSurface/DXTexture`,
  `CSelectMusic/CMusicInfo/CO2JamPack…`, source dirs
  `.\Avatar .\Display .\Input .\Interface .\Messenger`.
- Not installed (setup engine only, stay in `extracted/`):
  `IKernel/iUser/iscript/ctor/IGDI/ISBEW64`, `_IsRes.dll`, palettes,
  `corecomp.ini`, string tables.

## Correction: the payload cabs are ISc( streams (2026-11-17)

The first paragraph above says the cabs were carved out of the setup
`.exe` and read with `unshield`. Re-probing all three shipped setup
packages (`O2Jam_Classic.exe`, `NowSmartLauncherInstaller.exe`,
`Music_Full.exe`) shows that a carve is not enough on its own: the
`Disk1\data*.cab` and `data1.hdr` payloads are stored inside the setup's
own `ISc(` compressed stream (an InstallShield 11.5 container, entropy
~7.8 bits/byte, not zlib). `unshield` and `7z` both reject a carved
`ISc(` payload; only `engine32.cab` (553,805 B) is a raw `MSCF` cab. So
the installer run is what decompresses them. OPEN: how the original
tree was produced (an installer run described loosely, or an `ISc(`
decompressor that is no longer around); the reproducible recipe now
lives in `scripts/extract-installer.sh` (branch C, wine `/a`).

## Music pack merge (2026-11-17)

`Music_Full.exe` extracted 1010 files (504 .ojm + 506 .ojn, ids
100..887, timestamps from 2009) into `Music/`, keeping the 35 files that
came with the client. Cross-check against `Image/OJNList.dat` (562
entries): 512 of 562 ids are now installed. Not in this pack and not in
the client install, so still missing: 50 ids (757/758/763/765-769 came
from the client installer instead).

## OJM magic variants (verified, 21 files)

`M30\0` ×13, `OMC\0` ×5, `OJM\0` ×3. All three are real 7.55 magics —
`o2jam-file` sniffs all of them now.

## OJN header (observed, unresolved)

All 14 `.ojn` of the client install start with `6e65 77…` (`"new"` + varying bytes, then a
repeating 4-byte pattern). Looks encrypted/obfuscated — do NOT treat
`"new"` as a magic. First RE task for charts: find the decode (likely a
fixed XOR in/near the OJN loader; `OJNList.dat` starts `3202 0000 6400
0000 6f6a6e00…` = maybe 562 entries + `"ojn\0"` — cross-check after
decoding one file).

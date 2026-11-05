# Original stack (O2Jam 7.55) — what the RE has to deal with

Source of these facts: user-provided build info (C++, CDX, Boost 1.35/1.37,
libvorbis, MSVC 2005) plus local inspection of `O2Jam_Classic.exe`
(InstallShield 11.5 installer, PE32 i386; strings reference `CDX`, `_MSVCRT`,
`o2ma*.ojm`/`o2ma*.ojn`, `AngelO2Jam.exe` / `O2Jam.exe`).

> `O2Jam_Classic.exe` in this folder is the **installer**, not the game
> binary. Install it in a Windows VM, then inventory the installed files
> (hashes, sizes, versions) before any serious RE.

## 1. C++ / MSVC 2005 (compiler 14.0, `_MSC_VER 1400`)

What to expect in the disassembly:

- MSVC 2005 name mangling, `thiscall` methods, RTTI (`??_R*` / `__type_info`),
  SEH (`__except` / `__ehhandler`), and CString/iostream scaffolding.
- MSVCRT 8.0 side-by-side assemblies (`MSVCR80.DLL`, manifest policy) —
  the installed game dir should contain manifests; record them.
- No C++11: expect manual ref-counting, custom containers, Hungarian-ish
  members, and CDX-era COM-lite patterns instead of modern STL.

RE tips:

- Identify RTTI first (class names survive): `CDX*`, sound/file classes,
  game-state classes give you the module map for free.
- MSVC 2005 `new`/`delete` + Boost smart pointers (below) mean vtables +
  `shared_ptr` refcount churn are good function-boundary landmarks.

## 2. CDX game library (H. Pitt / CDXGraphics era)

CDX is a thin DirectDraw/Direct3D 7–era C++ wrapper: `CDXScreen`,
`CDXSprite`, `CDXTile`, `CDXSound`, input (`CDXInput`), timer, and file
helpers. Expect:

- One `CDXScreen` (or `CDXDirectDraw`) owning the frame: mode-set
  (likely 800×600 / 1024×768, 16-bit), primary + back surfaces, blits.
- `CDXSprite` per note receptor / note / effect: `Create`, `SetFrame`,
  `Draw`/`DrawTrans`/`DrawScaled` with color-key transparency.
- A fixed-timestep update + `Flip`/`Draw` loop; timing from
  `timeGetTime`/`QueryPerformanceCounter` via `CDXTimer`.
- Input via DirectInput wrappers (`CDXInput`: keyboard state polling).

Replacement plan: `winit` (window/input) + `wgpu` (sprites as textured
quads, nearest/bilinear as needed) + a fixed-timestep loop with
audio-clock-anchored song time. Keep CDX names in parity tests/comments so
reviewers can cross-check behavior.

## 3. Boost 1.35 / 1.37 (circa 2008)

Libraries plausibly used by a game of this era:

| Boost lib | Typical use in client | Rust replacement |
|---|---|---|
| `shared_ptr` / `intrusive_ptr` | sprites, sounds, resources | ownership / `Arc` |
| `thread` + `mutex`/`condition` | loader / audio threads | `std::thread`, later `tokio` |
| `filesystem` | song enumeration, paths | `std::path` / `walkdir` |
| `asio` | launcher/HTTP, patching | `reqwest` (launcher is out of scope for now) |
| `signals(2)` | UI/game events | channels / observer in game crate |
| `serialization` / `program_options` | config, options | `serde` + `clap` |
| `date_time` / `timer` | timing utilities | `std::time` |

RE tip: Boost 1.35 templates inline aggressively; use strings
(`boost::` asserts) and RTTI to name the instantiation sites rather than
reversing each expansion.

## 4. libvorbis (+ libogg)

`.ojm` payloads are OGG Vorbis key-sounds/BGM (community knowledge +
`BGM*.ojm` strings in the installer). Expect statically linked or sidecar
`ogg.dll` / `vorbis.dll` / `vorbisfile.dll` in the install dir — confirm and
record versions.

Replacement: `lewton` (pure-Rust Vorbis decoder) for decode parity now,
`cpal`/`rodio`-style backend later. Parity tests must compare decoded PCM
against reference decode within an explicit tolerance.

## 5. Installer notes (verified locally)

- `O2Jam_Classic.exe` — 140,350,480 bytes, PE32 i386 GUI, InstallShield
  11.5 setup player. `objdump` shows only stub imports (ADVAPI32, GDI32,
  KERNEL32, LZ32, ole32, OLEAUT32, SHELL32, USER32, VERSION) — game imports
  are inside the cabinet payload, not the stub.
- Embedded strings already confirm asset naming: `o2ma###.ojm` /
  `o2ma###.ojn` pairs, `BGM.ojm`, `bgEffect.ojm`, `DialogBGM.ojm`,
  `Event.ojm`, and next-stage binaries `AngelO2Jam.exe` / `O2Jam.exe`.
- `NowSmartLauncherInstaller.exe` (2,041,051 bytes) is the launcher
  installer — out of scope for the gameplay client, tracked for patching
  protocol later.

Next step: install in a VM, then fill in `docs/progress.md` with the real
binary inventory (name, size, sha256, `dumpbin /headers` compiler stamp,
DLL list).

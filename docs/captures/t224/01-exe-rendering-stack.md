# T224 leg 1 — exe rendering stack (DirectDraw, 7.55)

Verdict: **DIRECTDRAW-CONFIRMED**. The 7.55 client renders through
DirectDraw/Direct3D 7-era surfaces owned by a CDX wrapper layer.
No GPU-effect requirement exists anywhere in the evidenced path:
fixed 800x600x16 blits with a source colorkey.

## Version pin

- `AngelO2Jam.exe` = `OTwo.exe`, 5,029,888 B,
  SHA-256 `83a04541…72c823b` (`docs/progress.md` binary inventory).
- PDB stem `d:\O2JamSvn\trunk\Client\O2Jam_Client(2005)\Release\OTwo.pdb`
  (MSVC 2005 build, per `docs/original-stack.md`).

## Evidence

- `docs/claims.md` A39 (verdict EXE): `DDRAW.dll!DirectDrawCreateEx`
  is imported (IAT VA `0x63F01C`, descriptor file `0x27AF2C`).
  Consumer chain pinned in `docs/ui-draw.md`: display init `0x413030`,
  `CDXSurface::Draw` `0x42B4D0` (ev `867ac248…`), `DrawClip`
  `0x42B5E0` (ev `85764b28…`), page dispatch `0x42B450`
  (ev `08a33fe4…`), page upload `0x42A580`, D3D variant `0x423390`.
- `docs/claims.md` A38 (verdict EXE): one fixed video mode
  800x600x16. Mode args pushed (800 VA `0x414b63`, 600 `0x414b5e`,
  bpp `0x10` `0x414b5c`) into both display wrappers (call `0x413be0`
  at `0x414b72`, call `0x413030` at `0x414b91`; ev `c7d5a15f…`).
- `docs/gameplay.md` "Renderer default — VERIFIED 800x600,
  effectively fullscreen-only": `SetWindowPos(hwnd, 0, 0, 0, 800, 600,
  SWP_SHOWWINDOW)` at file `0x14b4c`/`0x14baf` (USER32 IAT slot
  `0x63f3a0`); `DirectDrawCreateEx` IAT `0x63f01c` via thunk
  `0x5adccc` from CDX display init (`0x413080`/`0x413c2f`).
  Window creation forks at file `0x41c0b` on a runtime flag bit
  (`+0x1c` of the global at VA `0x768128`): default
  `WS_POPUP|WS_VISIBLE` (`0x90000000`, fullscreen) vs `WS_CAPTION`
  (`0xC00000`, windowed) — but no config/registry/option writer for
  that flag exists in 7.55, so the windowed branch is unreachable.
- `docs/claims-render.md` verdict 4 (CORRECTED): sprite transparency
  is a DirectDraw source colorkey taken from the file's u16 key field.
  `DXSurface::Load` (`0x4249F0`, asserts `.\Display\DXSurface.cpp`)
  reads magic `0x0555`/`0x555A` at `+0x11A`, count `+0x11C`, key
  `+0x11E`; on 16bpp it re-encodes 555→565; key ≠ 0 installs
  `SetColorKey` low==high, key-type arg 8, SRCBLT
  (`0x422F60` → `0x42BA00`); blit flag word `0x11` (`0x42B450`).
- `docs/channel-fill.md`: no backbuffer-clear call is evidenced
  (zero hits for `FillRect`, `BackBuffer`, `Clear`, etc.); `Flip` /
  `BltFast` / `BitBlt` survive only as DirectDraw error strings
  (files `0x24ffdc`/`0x250b43`/`0x24f032`).

## Settled vs OPEN

- Settled: DirectDraw init path, fixed 800x600x16 mode, WS_POPUP
  fullscreen default with unreachable windowed branch, blit + source
  colorkey vocabulary, no-clear presenter policy (stale preserved).
- OPEN (explicit, not silent): the exact `Flip` call site
  (`docs/claims-render.md` OPEN 1, `docs/channel-fill.md` OPEN 1);
  the colorkey-consuming device call; whether any caller feeds
  `DXSurface::Draw`'s optional clip param; cooperative-level /
  exclusive-mode bytes were never pinned — "fullscreen" here means
  the borderless `WS_POPUP` window plus a fullscreen-size DirectDraw
  mode, not a proven `DDSCL_EXCLUSIVE` contract.
- Method note: all offsets are file offsets into the pinned exe
  (VA = file + `0x400000`); own-words descriptions only, no
  disassembly reproduced.

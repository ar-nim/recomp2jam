# Mouse tracking (7.55 client) — pointer follow + hover repaint arm

Version pinned to 7.55 (`AngelO2Jam.exe` == `OTwo.exe`, 5,029,888 B,
SHA-256 `83a04541…72c823b`). Static byte-walk only (objdump + Python
scans over `install-root/O2Jam/OTwo.exe`; VA = file offset + `0x400000`
in `.text`). All rows below are *(offset-only)*: no REA dossier was
filed. Closes issue #112; feeds #26 hover and the winit-equivalence
work. Findings log: `~/scratch/close-backlog/gui-drive/findings.md`.

## 1. Import census (the arm, decided from the IAT)

| Import | IAT slot (file) | Verdict |
|---|---|---|
| `DINPUT.dll!DirectInputCreateA` | `0x23f024` | only DirectInput entry; thunk `jmp *0x63f024` at VA `0x5adcd2` |
| `GetCursorPos` | `0x23f450` | cursor position source |
| `ClientToScreen` | `0x23f3a8` | client-origin trick (see §2) |
| `GetClientRect` | `0x23f3ac` | client-origin trick |
| `SetCursor` / `LoadCursorA` | `0x23f44c` / `0x23f45c` | cursor shape only |
| `PtInRect` | `0x23f448` | hover hit-test (§4) |
| keyboard: `GetAsyncKeyState` / `GetKeyState` / `GetKeyboardState` / `keybd_event` | `0x23f41c`–`0x23f424` | keys polled, not messaged |

Absent image-wide: `ScreenToClient`, `SetCapture`, `ReleaseCapture`,
`TrackMouseEvent`, `GetMessagePos`. So the exe never asks Windows for
client-relative cursor conversion or capture; it derives both itself.

## 2. How the exe follows the pointer

- **Window proc** `0x57dc40` (registered by the `RegisterClassA` call at
  VA `0x57e65b`; `lpfnWndProc` stored at VA `0x57e67d`, WNDCLASS style
  `0x4208` = CS_OWNDC|HREDRAW|VREDRAW|DBLCLKS): on `WM_CREATE` it
  SetProp's the object, then maps `HWND → object` via
  `GetProp(0x64dd08)` and **virtual-calls vtable[0]** (MFC-style
  `WindowProc`); NULL → `DefWindowProcA` (IAT `0x63f3fc`). WM_DESTROY
  (msg 2) additionally RemoveProp + vtable[3]. There is no exe-wide
  `cmp 0x200` switch: every `0x200` immediate in `.text` belongs to CRT
  locale/stream masks (checked each by context: VA `0x60bd8d`
  stream seekdir, `0x61a62e`/`0x61cf87`/`0x622d5b` locale categories),
  so `WM_MOUSEMOVE` is not the tracking channel.
- **Input ctor** `0x542490`: `DirectInputCreateA(0, 0x700, &this+0x34, 0)`
  (DI version 7.0; relcall VA `0x542559` → thunk `0x5adcd2`). It also
  seeds the cursor: `GetClientRect` + `ClientToScreen(hwnd, &rc.topLeft)`
  gives the client origin in screen coords, `GetCursorPos` fills
  `this+0x80`, and the origin is subtracted — the missing
  `ScreenToClient`, done by hand. Member `+0x80` holds the
  **client-relative cursor POINT**.
- **Poll** `0x544290` (called per frame from the manager update
  `0x546360` at VA `0x5463c8`, and from the input thread `0x545390` at
  VA `0x545455`):
  - device Acquire/GetDeviceState-style vtable loop on the device at
    `this+0x3c` (HRESULT retry on `0x8007001e`/`0x8007000c`,
    `0x542b7`–`0x54313`);
  - recomputes the client-relative cursor into `this+0x80`
    (`0x544440` block, same GetClientRect+ClientToScreen+GetCursorPos
    −origin sequence);
  - **motion gate** `0x5443a8`: compares `this+0x80` against globals
    `0x791cbc` (x) / `0x791cc0` (y); unchanged → both set to `−100`
    (`0x5443cf`) and the motion block is skipped; changed → globals are
    refreshed from the member (`0x5444bb`) and the motion block runs;
  - motion block: button-edge synthesis queued through
    `0x542900(this, code, 5, 0)` (codes `0x0A`/`0x23`/`0x24` gated on
    members `+0x244`/`+0x2A8`/`+0x2AC`), and double-click timing via
    `timeGetTime` into `+0x48C` with flag `+0x494` (`0x544527`), posted
    through `0x411ea0` (message id `0x1F4` to the window object at
    global `0x768120`) by `0x544960`.
- **Background thread** `0x545390`: `WaitForMultipleObjects(2,
  [device handle, event], FALSE, INFINITE)` on handles from the device
  vtable (VA `0x5453e5`); device event + flag `0x200` → clears
  `this+0x36C` (`0x94` bytes) and polls `0x544290` + drains `0x542870`.
  The DI device therefore runs event-notified, with a dedicated wait
  thread in addition to the synchronous per-frame poll.

## 3. Verdict: the tracking arm

Pointer tracking = **GetCursorPos polling converted to client-relative
by the exe's own GetClientRect+ClientToScreen origin subtraction**,
fed by a DirectInput-7.0 device layer (event thread + per-frame poll).
No WM_MOUSEMOVE handler, no mouse capture, no TrackMouseEvent.
Keyboard is the async-key poll, not DI-dependent for delivery.

## 4. Which arm repaints on motion

Per frame the manager update `0x546360` walks the device vector
(`this+0x3c`), polls each (`0x542790` → `0x544290`), then calls
`0x5468d0` and `0x546bc0`. The latter is the hover pass:
`PtInRect` (IAT `0x63f448`, call VA `0x546e97`) of the client-relative
cursor (`this+0x70`/`+0x74`) against every bound control rect
(`obj + 0xC + idx*0x10` — the same 16-byte rect stride the pairing
step writes), iterating the object vector at `this+0x3c`; hits feed the
queued-event drain `0x545910`, which walks the ring at `this+0x8` and
**virtual-calls vtable[1]** per queued cursor/button event into the
state object. Hover repaint is therefore a per-frame PtInRect sweep
over the placed bound rects, not a message-driven paint.

## OPEN

1. DI device creation site (GUID path): `GUID_SysKeyboard` /
   `GUID_SysMouse` byte patterns appear nowhere in the image — next
   step: trace the ctor that fills `this+0x3c` after `0x542490`, or one
   winedbg stopped-state read on the `DirectInputCreateA` out-param
   (rig time; wave 2 owns it).
2. Concrete class behind the queue-drain vtable[1] per state —
   resolvable by a follow-up static pass over the vtable↔RTTI join
   (same walk as `bnd-semantics.md` uses).

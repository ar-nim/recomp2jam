# Wine boot probe — O2Jam 7.55 client (`AngelO2Jam.exe`)

Date: 2026-11-08. Question: does the 7.55 client boot under Wine 11.21 here,
and how far does it get? Short answer: it starts, runs WinMain init
(settings load, single-instance mutex, session validation), then stops at a
clean launcher-gate dialog on a bare launch, or crashes in init with
session-like argv. No rendered game screen was reached. Nothing was written
to `install-root`; the game ran from a scratch copy.

## Environment pins

- Host: Arch Linux, Wine 11.21 (`wine-11.21`), 64-bit wow64.
- Display: Xvfb `:99`, 1024x768x24 (set up during the probe; earlier
  attempts used the real `:0` with an 800x600 virtual desktop).
  Screenshot run used a second server, Xvfb `:98`, 800x600x24 with
  `-fbdir /tmp/xvfb-fb`, plus Wine virtual desktop `800x600` — the real
  display was never touched.
- Prefix: fresh `$HOME/o2jam-prefix` (Wine defaults only).
- Game copy: `install-root/O2Jam` copied to `/tmp/o2jam-run` (read-only
  source). `AngelO2Jam.exe` SHA-256 `83a04541…72c823b`, identical to the
  pinned hash in `docs/lifecycle.md`.
- `install-root` verified untouched afterwards (same file list, same hash).

## Prefix creation was slow but completed

`WINEPREFIX=$HOME/o2jam-prefix wineboot --init` stuck ~8 minutes in
`rundll32.exe setupapi,InstallHinfSection DefaultInstall` (both threads
parked on `ntsync`). First attempt was killed mid-install and left a broken
prefix (`could not load kernel32.dll, status c0000135` on first use).
A clean rebuild finished; smoke test `wine cmd /c echo SMOKE_OK` passed
(`RC=0`). Recurring benign noise: `lock_display_devices`,
`MountMgr Auto-start failed`, `RpcSs`/`wineusb` fixmes.

## Attempt 1 — virtual desktop on :0 (contained, no Xvfb yet)

```fish
cd /tmp/o2jam-run
WINEDEBUG=err WINEPREFIX=$HOME/o2jam-prefix timeout -s KILL 40 wine \
  explorer /desktop=o2jamprobe,800x600 /tmp/o2jam-run/AngelO2Jam.exe
```

Result: process spawned (trace `NtCreateUserProcess … AngelO2Jam.exe`,
pid `013c`), lived ~2 s, exited `RC=0`. No DirectDraw calls, no window,
no files written. Dead end for observation, not for containment.

## Attempt 2 — direct launch under Xvfb :99

```fish
cd /tmp/o2jam-run
DISPLAY=:99 WINEDEBUG=err WINEPREFIX=$HOME/o2jam-prefix timeout -s KILL 40 \
  wine /tmp/o2jam-run/AngelO2Jam.exe
```

Result: `RC=137` (killed by timeout after 40 s) — the client stays alive.
Single thread, ~1% CPU, no sockets, no file writes. `+relay` showed why:
it creates a window (`hwnd 0x1005E/0x1007A`), calls
`NtUserShowWindow(..., SW_SHOWNORMAL)`, then blocks in `NtUserGetMessage`
— a dialog waiting for input.

## Attempt 3 — contained re-run under Xvfb + screenshot (verified)

Per environment update: Xvfb is preferred, Wine virtual desktop,
screenshot from the X framebuffer. Exact recipe:

```fish
nohup Xvfb :98 -screen 0 800x600x24 -fbdir /tmp/xvfb-fb > ~/xvfb98.log 2>&1 &
cd /tmp/o2jam-run
DISPLAY=:98 WINEDEBUG=err WINEPREFIX=$HOME/o2jam-prefix nohup wine \
  explorer /desktop=o2jam,800x600 /tmp/o2jam-run/AngelO2Jam.exe > ~/wine-shot.log 2>&1 &
# wait ~55 s for the slow startup, then:
DISPLAY=:98 ffmpeg -hide_banner -loglevel error -y -f x11grab -i :98 \
  -frames:v 1 /tmp/o2jam-shot1.png
```

Result: the client stays alive inside the 800x600 Wine desktop (teal
background fills the whole virtual screen — nothing ever touched the real
display). Centered on it sits a classic `MessageBoxA` with title bar, two
lines of body text containing the Latin word `GameStart`, and an `OK`
button with focus. This is the same launcher-gate dialog the `+relay`
trace captured (`홈페이지에서 로그인 후 GameStart을 실행하세요!`).
The Korean glyphs render as mojibake boxes: the probe prefix has no CJK
fonts installed, so caption `정보` shows as `Ã±o`-style glyphs. Font
coverage only — the dialog geometry, text length, and `OK` button match
the traced call exactly. Screenshot kept at `/tmp/o2jam-shot1.png`
(9.8 KB); no input injector (`xdotool`/`xte`/`xvkbd` all absent), so the
OK button was not clicked and post-dialog behavior is untested. Game log
`~/wine-shot.log` is clean apart from benign DRI3/EGL warnings and the
known `SPI_SETSTICKYKEYS` fixme. Process terminated afterwards with
`pkill -x AngelO2Jam.exe` (verified: zero remain).

Note: the virtual desktop that exited in ~2 s on `:0` (Attempt 1) works
fine under Xvfb — the early exit was an XWayland interaction, not a
virtual-desktop problem.

## What the dialogs say (decoded from CP949)

Bare launch, single instance — `MessageBoxA`, caption `정보`
("Information"), from game code (`ret=005ad4d3`):

> `홈페이지에서 로그인 후 GameStart을 실행하세요!`
> ("After logging in on the homepage, please run GameStart!")

Pre-gate sequence from the same trace (function near `0x5AD080`):
`GetCurrentDirectoryA` → `FindFirstFileA("*.dmp")` (none) →
`SystemParametersInfoA` ×2 → `CreateMutexA("신나는음악게임오투잼")`
("Exciting music game O2Jam") → `GetLastError()=0` (first instance) →
`GetPrivateProfileIntA` reads `MODE/Mode=2`, `LONGNOTEMODE/LongnoteMode=1`,
`NoteScreen/BlendHalf=1` from (missing) `option.ini` — defaults apply,
matching the key table in `docs/lifecycle.md` — then the gate box.
The gate check itself makes no further API calls (pure command-line
validation), so the exact argv format is still unknown.

Second instance while one is alive — `GetLastError()=0xB7`
(`ERROR_ALREADY_EXISTS`) → different box:

> `O2Jam의 일부 실행 파일입니다.!` ("This is part of the O2Jam
> executable files.!") — the duplicate-execution guard.

## Crash findings (game's own minidumps)

The client installs `SetUnhandledExceptionFilter(0x408b70)` at startup and
writes `AngelO2Jam <date>.dmp` + `.log` on a crash. Two crashes observed
(reports preserved at `/tmp/o2jam-reports/`):

1. `2026-11-08 15-20-07` — worker thread `0x154`,
   `EXCEPTION_ACCESS_VIOLATION` (read), `EIP = fault = 0xFFBC11B0`
   (outside every loaded module; `EDX` held the same value — a bad jump).
   `DDRAW/DINPUT/DSOUND/wined3d` were loaded, so init reached media setup.
2. `2026-11-08 15-29-35` — after passing dummy session argv
   (`USER_ID=testuser PASSWORD=testpass USER_INDEX_ID=1`), thread `0x140`
   faulted at `0x5F2893` (game RVA `0x1F2893`, file offset `0x1F2893`).
   Dummy argv gets past the gate dialog into deeper init, then AVs.

Suspicious: `0xFFBC11B0` killed a winedbg helper thread too (see below).
Identical fault address in two processes points at a Wine/wow64 mapping
rather than pure game logic. No crash involves network I/O: `+winsock`
traces show zero `connect`/`getaddrinfo` (beyond Wine's own hostname
lookup), so nothing blocks on the dead servers — init dies or idles
before/without net.

A later launch with leftover `.dmp` files present shows a third box:

> `오류보고파일이있습니다. 오류보고파일을전송하시겠습니까`
> ("An error report file exists. Do you want to send it?") — OK/Cancel.

## winedbg viability: attach works, with one caveat

- Attach by **decimal** wpid: `printf 'info thread\nbt\nquit\n' |
  DISPLAY=:99 WINEPREFIX=$HOME/o2jam-prefix timeout -s KILL 30 winedbg 300`
  (`300` = wpid `0x12c`). Hex wpid is misparsed as a launch command.
- Launch-under-debugger also works
  (`… | winedbg /tmp/o2jam-run/AngelO2Jam.exe` with `cont` first);
  `quit` detaches and the game keeps running.
- Caveat: winedbg's injected helper thread page-faulted at `0xFFBC11B0`
  (same address as crash 1) and the debuggee died with it. For stack
  captures, prefer launch-under-debugger plus `thread <id>` before `bt`;
  plain `bt` may run against the helper thread.

## Boot stage reached and blockers

Reached (maps to `docs/lifecycle.md` stages 0–3, never 4+):
CRT entry → exception-filter install → cwd/`*.dmp` scan → single-instance
mutex → `option.ini` defaults → session/argv validation → gate dialog.
No `STATE_AVI`, no DirectDraw mode-set, no archive reads observed.

Blockers, in order:

1. Unknown launcher argv format (the gate). `USER_ID`/`USER_INDEX_ID`/
   `PASSWORD`/`PCROOM_USER` strings sit near `0x251400`; the check is pure
   game code after `SetCurrentDirectoryA` restore.
2. Init-time `ACCESS_VIOLATION`s (game RVA `0x1F2893` with session argv;
   wild jump to `0xFFBC11B0` in one bare run with media DLLs loaded).
3. Possible Wine/wow64 contribution to (2): identical wild address in an
   unrelated winedbg helper thread.

## Logs and artifacts (kept outside the repo)

- `~/wineboot2.log`, `~/wine-game*.log`, `~/wine-relay*.log` (217 MB
  `+relay` trace), `~/wine-seh.log`, `~/wine-ws2.log`,
  `~/wine-argv.log`, `~/wine-dbg*.log`, `~/xvfb.log`.
- `/tmp/o2jam-reports/`: both `.dmp`+`.log` pairs; `/tmp/mdmp_parse.py`:
  minidump stream parser used here.
- Scratch only: `/tmp/o2jam-run/`, `$HOME/o2jam-prefix`, Xvfb `:99`
  (left running). Repo change: this file only. Not committed (per task).

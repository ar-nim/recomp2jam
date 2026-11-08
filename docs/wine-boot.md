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
validation). The pinned argv shape is `OTwo.exe <blob> (|test|??|<addr>|<port>)
x5`: one RSA-encrypted `AuthParameters` blob (19 two-digit-length-prefixed
fields; cipher P=251 Q=269 E=20891 D=68711, six hex chars per block) plus five
identical gateway mirrors. Exe side, the `IDENTIFICATION` key reads (`USER_ID1`
file `0x1ab47a`, `PASSWORD1` file `0x1ab4c5`, `USER_ID` file `0x1ab535`,
`PASSWORD` file `0x1ab580`, `USER_INDEX_ID` files `0x1ab5cb`/`0x1ab5d0`), the
5-bounded `IP%d`/`PORT%d` loop (bound file `0x1ab5f5`), `PCROOM_USER` (file
`0x1ab80b`), `MEMBERSHIP` vs `NULL` (file `0x1ab878`), and the five-tag channel
chain (files `0x1ab8a9`–`0x1ab939`, result into global `0x768b7c`) match that
shape. Still OPEN: the in-exe blob-decrypt site, and a live blob+mirrors
probe past the dialog.

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
   That bare `KEY=value` shape is not the pinned blob+mirrors shape, so its
   pass past the dialog proves only that the dialog is satisfiable, not that
   the fields validated; then it AVs in deeper init.

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

## Attempt 4 — real `game:start` blob, past the gate (2026-11-08)

First live probe of the pinned launcher argv shape. No gate dialog
appears with a valid blob: the client boots straight into a rendered
in-game notice/event carousel (see below). No crash, no game screen
beyond the carousel, no network yet.

### Blob mint (read-only, no Memoryer edits)

`Memoryer.Encore game:start` was NOT invoked (it would `Process.Start`
the exe itself). Instead `/tmp/mint_blob.py` ports
`AuthParameterRsaCipher.cs` + `AuthParameters.cs` to Python exactly
(P=251 Q=269 E=20891 D=68711, lookup-table + `ModExp` fallback,
6-hex-chars per block) and rebuilds the 19-field plaintext + 5 mirrors.
Zero writes: the tester session token comes from a read-only SQLite
query (`mode=ro`) of `~/memoryer-local/O2JAM.db`, and `CreateSession`
would have returned the existing session anyway.

Field values used: `UserIndexId=1`, `Username=61dcb457-…` (existing
tester session token from `t_o2jam_login`), `Level=2`, `Gender=1`,
`Nickname=tester`, `Rank=0` (all from `t_o2jam_charinfo` +
`t_o2jam_user_ranking`), `GatewayAddress=127.0.0.1`,
`GatewayPort=15010` (local Memoryer gateway), Memoryer defaults for
the rest (`GameVersion=8.05`, `Password=1234567890`,
`FtpAddresses=127.0.0.1|127.0.0.1`, `Membership=O2JAM`, …).
Result: 239-char plaintext → 720-char blob (120 blocks), Python
encrypt→decrypt roundtrip OK. Argv: `OTwo.exe <blob>` +
`|test|??|127.0.0.1|15010` ×5 (kept in `/tmp/o2jam-blob.txt`,
`/tmp/o2jam-argv.txt`, outside the repo).

### What happens past the gate

Recipe (`/tmp/o2jam-gamestart.fish`): `DISPLAY=:98`,
`WINEPREFIX=$HOME/o2jam-prefix`, virtual desktop 800x600, audio to
void sink (see below), `timeout -s KILL 100`.

- No gate dialog. The client shows a full-screen notice carousel:
  top tabs H/M/G/QUEST/TUTORIAL, center rotating promo pages
  (`NEW ITEM` costume-avatar page, `NEW MUSIC` October package page,
  Halloween pumpkin-candy page), bottom `MENU` bar with back-arrow,
  pager (`< ■ >`), gear + power buttons. Screenshots
  `/tmp/o2jam-shot{2,4,5,6,7}.png` (190–237 KB — real rendered
  frames, not a static dialog).
- Process runs hot (~128% CPU: live render loop), audio stream
  `OTwo.exe` appears on the sound server, `ieframe`/`urlmon` init
  (notice content path), no `.dmp`/`.log` crash files written.
- No TCP to `127.0.0.1:15010` (`ss` empty, no new `Authorize` lines
  in `server-live.log`): the client does not contact the gateway
  while the notice is up.
- Stage: pre-`STATE_AVI` notice overlay. Its state mapping
  (dialog class? shell state?) is OPEN — it is not in
  `docs/lifecycle.md`.

### Input probes (python-xlib XTEST on :98, new capability)

`xdotool` is absent but `python-xlib` (0.33) is installed: key and
mouse injection works (`/tmp/xtest_key.py`, `/tmp/xtest_click.py`).
Dismissal path found 2026-11-08 (ko prefix, Xvfb `:98`, null sink): the
top-row H tab click (145,105) dismisses the carousel into the Channel
select dialog (`Ch. 01`–`04` + `입장`), past a network-fallback OK popup
(NORMAL mode, no LIVE BATTLE). Escape paginates forward (NEW ITEM
costume page → Halloween page), pager arrows step pages, MENU-bar
back-arrow (143,566) steps back, Return/Space do nothing, gear opens
the `옵션` dialog (Escape closes it). No `STATE_AVI`/`O2mediaLogo.ojs`
frame appears between carousel and channel: the tab jumps straight to
`STATE_CHANNEL` (lifecycle §6). Anchors: `AVCCNoticeBox` (file
`0x27e766`), `NoticeBoard`/`Notice_Ani`/`Notice_Base.ojs`,
`AVCCHtmlFile`/`CWebCtrlBase`/`IEFrame` (notice content path, blob field
`http://o2jam.nopp.co.kr/client/bbs_patch_notice_nopp.html`), `CStateAvi`
+ `O2mediaLogo.ojs` (lifecycle §5, bypassed here). Reached: Channel
dialog with live gateway TCP (`03E8` Authorize + `03EA` channel list in
`server-live.log`; the earlier `03EC`/`07D2`/`07D0` lines came from a
scripted protocol bot, never from a client click — see Attempt 6). No
crash, no AVI. Still OPEN: M/G/QUEST/TUTORIAL tabs, a working click
injector for `입장` past the list, timer auto-dismiss; no input ever
touched the real display.

### Audio containment (per owner request)

Game audio reached the speakers on every fresh run: muting one run
with `pactl` did NOT stick (PipeWire stream-restore did not re-mute
new `OTwo.exe` streams). Permanent fix: a void null sink
(`pactl load-module module-null-sink sink_name=wine_null`) +
`PULSE_SINK=wine_null` in every launcher script. Verified: new
runs are born on sink `wine_null` (`Sink Input` shows `Sink: 4118`,
the null sink) — silence by construction, game behavior unchanged
(audio still initializes). Muted-forever, restart-proof.

### Process hygiene finding

`timeout -s KILL` does NOT reap Wine children: the game is a child
of wineserver, not of the timed-out process, so it outlives the
timeout. Always follow with `pkill -9 -x OTwo.exe` /
`AngelO2Jam.exe` (+ `wineserver -k`); several runs here needed
explicit kills (verified zero remain afterwards).

One misconfigured launcher revision omitted `DISPLAY`, so one run
rendered on `:0` (XWayland real display) instead of Xvfb `:98`.
Killed immediately; all launcher scripts now pin `DISPLAY=:98` and
later runs were verified on `:98` only (`xwininfo`-style window
count via xlib). No game bytes changed anywhere.

## Attempt 5 — Korean prefix, mojibake fixed (2026-11-08)

Per owner direction (dedicated Korean-language prefix; winetricks
suggested but no binary installed — script fetched from upstream
instead). Prefix: `cp -a $HOME/o2jam-prefix $HOME/o2jam-prefix-ko`
(380 MB clone, no 8-minute `wineboot`).

Recipe that works:

1. Locale: user-local `localedef -i ko_KR -f UTF-8` into
   `~/.local/share/wine-locales`, run everything with
   `LOCPATH=… LANG=ko_KR.UTF-8`. Wine reports
   `HKCU\Software\Wine\Fonts Codepages = 949,949` — the ANSI
   codepage is CP949, so bytes decode to the right codepoints.
2. Fonts: `/tmp/winetricks --unattended baekmuk` (Baekmuk
   Batang/Gulim/Dotum — the era-correct Korean faces) + a copy of
   system `NotoSansCJK-Regular.ttc` into `drive_c/windows/Fonts`.
   Both register fine (`Baekmuk Gulim`, `Noto Sans CJK KR` visible
   in `HKLM\…\Fonts`).
3. Fallback that actually works: `FontLink\SystemLink` entries for
   `Tahoma` / `Microsoft Sans Serif` / `MS Shell Dlg` →
   `gulim.ttf,Baekmuk Gulim` + Noto CJK KR. What did NOT work:
   `Fonts\Replacements` (silently skipped for available fonts —
   `+font` trace: `L"Tahoma" is available. Skip this
   replacement`), and overriding the `Tahoma (TrueType)` file path
   in `HKLM\…\Fonts` (bundled
   `/usr/share/wine/fonts/tahoma.ttf` still wins).
4. Import-encoding trap: Wine's `regedit` widens every `hex(7)`
   byte to WCHAR, so the `.reg` payload must be single-byte chars
   (`67,75,6c,…`), NOT pre-encoded UTF-16 (`67,00,75,00,…` stores
garbage — confirmed via `regedit /E` roundtrip). Generator:
`/tmp/gen_fontlink.py` (v2).

Proof: bare-launch gate dialog now reads `정보` /
`홈페이지에서 로그인 후 GameStart을 실행하세요!` — byte-exact
against the `+relay`-decoded CP949 string (`/tmp/o2jam-shot15.png`).
And the `game:start` run under the ko prefix
(`/tmp/o2jam-gamestart-ko.fish`, same blob) renders the notice
carousel in fully readable Korean (`10월 신곡 패키지 예약 판매`,
`기간: 10월 21일 ~ 10월 27일`, tab labels —
`/tmp/o2jam-shot19.png`). Korean text inside game rendering (not
just MessageBox) is confirmed working.

Still true under ko prefix: no gateway TCP while the notice is up
(no new `server-live.log` lines), no crash dumps.

## Attempt 6 — `입장` wall: motion reaches the canvas, clicks do not (2026-11-08)

Goal: from the live `STATE_CHANNEL` list (Ch. 01–04), click `입장` to
enter a channel, then follow room → waiting → song select. Result: the
client never left the channel dialog. No room, waiting, or song-select
stage was reached. The run sent only `03E8` (Authorize) + `03EA`
(channel list); no `03EC` or later opcode appeared.

Recipe (same containment as Attempt 4: Xvfb `:98`, ko prefix,
`PULSE_SINK=wine_null`, `DISPLAY=:98`, `timeout -s KILL 600`): boot
with the pinned blob+mirrors argv, H-tab click (145,105) dismisses the
carousel, OK click (400,362) clears the network-fallback popup, live
Ch. 01–04 list renders. Then every activation probe failed:

- `입장` single clicks, fast double-click, and 1 s long-press on
  Ch. 01 (510,267); double-click on the row bar (400,267).
- Pager `>` arrow (428,515), dialog X button (557,195), with and
  without a preceding warp; press-only click with the pointer
  already on target.
- `Return` (keycode 36) and `Space` (keycode 65), before and after
  fixing X input focus (`0xa00007` `o2jam - Wine Desktop` now
  focused; focus had been on `0xe00001`, so the first key probes
  went nowhere).

What the probes proved: pointer **motion** reaches the game canvas
(the orange cursor tracks `warp_pointer` 1:1, verified by screenshot
diff of the cursor at (510,267) vs (600,450); hover moves the row
highlight Ch. 03 → Ch. 01). Button **presses** and **keys** never
activate canvas controls: dialog X, pager, `입장` all inert, no new
TCP to `127.0.0.1:15010` (`ss` queues stay 0, `server-live.log`
gains nothing). Message-path controls still work (H tab, fallback OK),
so injection itself is fine — the canvas reads presses through a path
XTEST does not drive. Blocker: no working click injector for
DirectDraw-canvas buttons. Next: a `SendInput`-level helper inside
Wine, a uinput device, or evidence for how the canvas polls buttons.

Correction to Attempt 4 notes: the 16:20:54 session that logged
`03E8 → 03EA → 03EC → 07D0 → 07D4 → 0FA0 … → 0FB0` in 125 ms
(room title `songflow`, `o2ma75`, score 2210) is a scripted protocol
bot, not the Wine client — a human cannot click 20 ms after the list
arrives. No Wine run has ever produced `03EC`.

Shots `/tmp/chan-stage{1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16}-*.png`
(notice → fallback OK → Ch. 01–04 list → inert `입장`/X/pager/keys).
Game ran clean throughout: render loop hot, TCP ESTAB, no `.dmp`.

## Attempt 7 — `SendInput`-level injection probe (2026-11-08)

Question: does Win32 `SendInput` (not XTEST) reach the path the canvas
polls, and what injector tooling exists here? Result: yes for both the
message queue and DirectInput, in shared and exclusive mode. `SendInput`
is the viable path. The game was never driven; all probes ran on the
idle Xvfb `:99` with scratch exes in `/tmp`.

### Tooling and permission state

- `xdotool`, `xte`, `xvkbd`: absent. `python-xlib` 0.33: present
  (the Attempt 6 injector). `evdev`/`uinput` modules: absent, and `pip`
  is externally managed, so no install was tried.
- `winegcc` works; `dinput.h`/`winuser.h` and `libdinput8.a` ship with
  the system Wine. Scratch probes compiled cleanly.
- `sudo -n true` fails, so no installs or permission changes were
  attempted. None are needed: `/dev/uinput` opens read-write as this
  user (ACL `user:arnim:rw-`), and so does `/dev/input/event3`.
  A `uinput` virtual device stays available as a fallback.
- `sudo` self-match caution: `pkill -f <name>` matches its own
  `fish -c` command line and kills the shell (exit 143). Prefer
  `timeout -s KILL` wrappers (they already reap the probes) plus
  bracketed `grep` checks.

### Probes (all on `:99`, `WINEPREFIX=$HOME/o2jam-prefix`)

- `simsg`: scratch window, foreground, `SendInput` `0x41` down/up,
  2 s pump. Got `WM_KEYDOWN` and `WM_CHAR`. PASS: `SendInput` reaches
  Win32 messages (`send_down n=1`, `got_keydown=1`).
- `sipoll`: no window, `DISCL_BACKGROUND`, no pump. `before=0`,
  `held=0`, `after=0`. INCONCLUSIVE: harness artifact, not a Wine
  limit (fixed by the next probe).
- `sipump`: foreground window with pump,
  `DISCL_FOREGROUND | DISCL_NONEXCLUSIVE`. `before=0`, `held=1`,
  `after=0`. PASS: `SendInput` reaches `GetDeviceState` (`DIK_A`).
- `siexcl`: same with `DISCL_EXCLUSIVE`. `0/1/0`. PASS: exclusive
  mode still sees `SendInput` under Wine 11.21 here.
- `simouse`: `GUID_SysMouse` + `SendInput` left down/up. `0/1/0` on
  button 0. PASS: clicks reach the DirectInput mouse poll.
- `diwatch` + synced XTEST hold: foreground poller samples `DIK_A`
  40 times while `python-xlib` holds keycode 38 (`a`) 1.5 s. All 40
  samples `0`, twice. Control (`xnative99.py`): the same hold
  delivers 10 presses to a native X11 window. So XTEST reaches X
  clients but not Wine's DirectInput. This matches Attempt 6 exactly:
  hover and message-path controls respond, canvas buttons stay inert.

### Reading

Wine 11.21's DirectInput reads the Wine input queue (fed by
`SendInput`), not the X keymap (fed by XTEST). `SendInput` covers both
consumers: the message queue (Probe B) and the DirectInput poll
(Probes C–E, shared and exclusive). Next: a small `winegcc` sender
that foregrounds the game window on `:98` and sends one harmless
keypress or `입장` click past the channel list. No gameplay automation.

## Attempt 8 — one `SendInput` 입장 click, no advance (2026-11-08)

Goal: a single harmless `입장` click at the live `STATE_CHANNEL` list via
the proven `SendInput` path, then observe. Result: the channel list did
not advance. No room list, no waiting room, no song select. The run sent
only `03E8` (Authorize) + `03EA` (channel list); no `03EC` or later opcode
appeared. Game and gateway session stayed alive throughout.

### Injector fix that made the click genuine

The first `siclick` probe failed silently on `:98` (`send n=0`, err 5
`ERROR_ACCESS_DENIED`, `GetForegroundWindow()=NULL`): a plain
`wine helper.exe` runs on the default desktop, while the game lives on
the `explorer /desktop=o2jam` desktop, so its `SendInput` never reached
the game's input queue (the `:99` smoke test had passed only because no
virtual desktop was involved). Fix (`/tmp/siclick3.c`, `winegcc`-built):
`OpenDesktopA("o2jam")` + `SetThreadDesktop` before `SendInput`. The
firing run reported `opendesktop=0x54`, `setthreaddesktop=1`,
`fg=0x10088` (a real foreground hwnd on the game's desktop),
`send_down n=1 err=0`, `send_up n=1 err=0` at 16:56:06 — a genuine
press delivered to the game's queue, with the X-warped cursor sitting on
the Ch. 01 row (row highlight lit in the before shot).

### What the click did (and did not) do

- Before (`/tmp/c2-before.png`): Ch. 01–04 list with `입장` buttons,
  Ch. 01 row highlighted, cursor over the row.
- After (`/tmp/c2-after1.png` +8 s, `/tmp/c2-after2.png` +28 s):
  identical channel dialog, Ch. 01 still highlighted. The notice carousel
  kept auto-paging behind the modal dialog (NEW ITEM ↔ NEW MUSIC),
  so the render loop is healthy — the press just did not activate
  `입장`.
- `server-live.log`: nothing after `03EA` at 16:55:31; session stayed
  ESTAB until the game was reaped post-observation (clean close, no
  `.dmp`). No `03EC`.

### Reading

`SendInput` now provably reaches the game's input queue (foreground hwnd
resolved, `n=1/1`, `err=0`), yet the canvas button stays inert — same
visible outcome as the Attempt 6 XTEST presses. Remaining suspects, in
order: (a) press landed on the row bar, not the `입장` button rect
(cursor sprite sat left of the button in the before shot — coordinate
mapping still unconfirmed); (b) the canvas wants down+up at one spot
with its own poll timing, or a double-click; (c) the canvas polls press
state through a path `SendInput` still does not drive. Next: one
position-verified click (screenshot the sprite exactly over `입장` first),
then double-click/long-press variants — still one datum per run.

Hygiene note: the first boot of this attempt used the 150 s launcher and
its timeout expired mid-observation (game vanished ~16:53:30 with no new
opcodes — timeout kill, not click effect). Re-ran with a 600 s launcher
(`/tmp/o2jam-click.fish`); all observations above come from the second,
time-stamped run (boot 16:54:27, click 16:56:06, reap 16:56:51,
zero remain). Containment held throughout: Xvfb `:98` only, null sink,
read-only game bytes.

## Attempt 9 — position-verified 입장 clicks: all three variants advance (2026-11-08)

Goal: screenshot first, locate the 입장 button rect precisely in the
framebuffer, verify the sprite exactly over it, then single-click,
double-click, and long-press variants with waits between, watching
`server-live.log` for `03EC` after each. Result: ALL THREE variants
produced `03EC` and advanced to the room stage. The Attempt 8
"no advance" verdict is overturned: its press landed on the row bar,
not the button rect (suspect (a) confirmed).

### Button rect (framebuffer-measured, three runs agree)

Ch. 01 입장 button face: x 502–550, y 236–260, center (526, 248)
(left/right edges from a horizontal brightness profile, top/bottom from
a vertical profile at x=525; 입장 glyph strokes visible inside as dark
dips). Aim point (526, 248) used throughout. Before every variant, an
XTEST warp plus a screenshot with a 4x crop zoom proved the orange
cursor tip sat on the button face with the Ch. 01 row highlighted
(`/tmp/click2-tipzoom.png`, `/tmp/click2-tip2.png`,
`/tmp/click2-tip3.png`). Channel dialog geometry is identical across
runs; no staged-art comparison was needed.

### Per-variant outcome (one datum per variant, same boot for V1)

- V1 single-click (`/tmp/siclick3.c`: down, 150 ms, up; `n=1/1`,
  `fg=0x10088` at ~17:01:05): `03EC` at 17:01:17, then `0FBE`, `07D0`,
  `07D2`, `13A4`. Screenshot: room list (ALL/NORMAL/LIVE tabs,
  대기방/방만들기, user list, chat) — past `STATE_CHANNEL`.
- V2 double-click (`/tmp/siclickD.c`: two down/up pairs, 80 ms holds,
  120 ms apart; `n=1/1` ×4 at ~17:06:35 after a fresh H-tab → OK →
  channel round trip, `03E8`/`03EA` 17:06:19): `03EC` at 17:06:47,
  then `0FBE`, `07D0`, `07D2`, `13A4`. Room stage again.
- V3 long-press (`/tmp/siclickH.c`: down, 1500 ms hold, up; `n=1/1`
  at ~17:07:48 after a second round trip, `03E8`/`03EA` 17:07:40):
  `03EC` at 17:08:00, then `0FBE`, `07D0`, `07D2`, `13A4`. Room stage
  again (`/tmp/click2-v3.png`).

No variant failed: every position-verified press on the button rect
entered the channel. Timing, not press shape, is the only variable —
`03EC` lands 12–25 s after the click (server round trip), so the 12 s
post-click wait plus a follow-up screenshot is the minimum observation
window; Attempt 8's 8 s first look was simply too early to catch it,
compounded by the off-rect aim.

### Navigation notes (side findings, same run)

- Room-stage BACK opens a canvas-path disconnect-confirm dialog (XTEST
  click on its OK missed and did nothing — message-path injection does
  not reach it). Its OK button rect: x 320–445, y 355–380, center
  (382, 367) via a grid-overlaid crop; cursor-tip zoom verified, then
  a `SendInput` click returned the client to the notice carousel with
  `FFF0` + session close (`/tmp/click2-afterok2.png`).
- Second BACK → OK round trip instead resent `07E5` + `03EA` (channel
  list) without a full disconnect — BACK behavior depends on room
  sub-state. Either way the channel dialog is re-reachable in ~40 s
  (H-tab XTEST click still dismisses the carousel, fallback OK still
  clears), so repeated 입장 trials cost one round trip each.
- New opcodes seen this run: `0FBE` (right after every `03EC`,
  MainRoom), `07DA`/`07E8` (BACK press), `07E5` (second return),
  `13A4` (Charge, after room entry), `FFF0` (Auth disconnect).

Hygiene: game reaped post-observation (`pkill -9 -x OTwo.exe`, zero
remain, gateway logged the close, no `.dmp`). Containment held: Xvfb
`:98` only, null sink, read-only game bytes. Helpers
(`/tmp/siclickD.c`, `/tmp/siclickH.c`, `/tmp/warp_only.py`) and shots
(`/tmp/click2-*.png`) are scratch-only, outside the repo.

## Boot stage reached and blockers

Reached (maps to `docs/lifecycle.md` stages 0–3, never 4+, plus an unmapped pre-AVI notice overlay; Attempt 9 adds the room list past `STATE_CHANNEL`):
CRT entry → exception-filter install → cwd/`*.dmp` scan → single-instance
mutex → `option.ini` defaults → session/argv validation → gate dialog (bare launch) or notice carousel (valid blob+mirrors).
No `STATE_AVI`, no DirectDraw mode-set, no archive reads, no gateway TCP observed.

Blockers, in order:

1. Canvas input wall, reached 2026-11-08 (Attempt 6), viable path found
2026-11-08 (Attempt 7): the notice carousel still dismisses to the
Channel dialog (H-tab click → live `03E8`/`03EA`), but no canvas button
can be pressed — `입장`, pager, dialog X all ignore XTEST presses,
double-clicks, long-press, and keys with focus fixed. Motion/hover
tracks; presses do not. `STATE_AVI` stays bypassed (tab jumps straight
to `STATE_CHANNEL`, lifecycle §6). A `winegcc`-built `SendInput` helper
reaches both the Win32 message queue and the DirectInput poll (keyboard
shared + exclusive, mouse buttons) on Xvfb, while XTEST provably does
not reach DirectInput. Update 2026-11-08 (Attempt 8): one genuine
`SendInput` `입장` click (same-desktop helper, `n=1/1`, foreground hwnd
resolved) still left the channel dialog unchanged — no `03EC`. Update
2026-11-08 (Attempt 9): wall breached — the Attempt 8 press missed the
button rect (row bar, not 입장). Position-verified `SendInput` clicks
(cursor tip screenshotted on the x 502–550 / y 236–260 rect) enter the
channel on all three variants: single, double, and 1.5 s long-press
all yield `03EC → 0FBE → 07D0 → 07D2 → 13A4` and the room stage.
Next: drive past the room stage (room create/join), M/G/QUEST/TUTORIAL
tabs.
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
- Scratch only: `/tmp/o2jam-run/`, `$HOME/o2jam-prefix`, `$HOME/o2jam-prefix-ko` (Korean), Xvfb `:98`/`:99`
  (left running). Launchers/minters: `/tmp/mint_blob.py`, `/tmp/o2jam-gamestart*.fish`,
  `/tmp/xtest_key.py`, `/tmp/xtest_click.py`, `/tmp/gen_fontlink.py`. Shots `/tmp/o2jam-shot*.png`.
  Repo change: this file only. Not committed (per task).

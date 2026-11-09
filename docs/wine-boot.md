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
touched the real display. Update 2026-11-08 (Attempt 11, section
above): the room JOIN wall is breached — a CLI-hosted `WINEJOIN`
(2004 script as tester2, music 757 via 4000 or the row never paints)
double-clicked from the live list sends `0BBA` and reaches the WAITING
ROOM. New walls: no song select/gameplay (guest cannot start), a ~60 s
self-`FFF0` fuse per idle stage with autonomous re-auth, one
intermittent mid-`07D2` crash on a populated list. Next: host-driven
`4010` start with the guest seated.

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

## Attempt 10 — room stage → CREATE ROOM dialog, wall at CREATE OK (2026-11-08)

Goal: from the live room list, click through room join/create to waiting
room, song select, song start. Result: reached the CREATE ROOM dialog
twice (방만들기 → NORMAL → 3KEY DRUM selected), but its OK button
ignores every stimulus and no room-create opcode is ever emitted. No
waiting room, no song select, no gameplay. The room stage carries a
~40 s server-timeout fuse ("no response" modal) whose only exit is a
disconnect. Helpers, launchers, and shots are scratch-only in `/tmp`
(`j10-*.png`, `sikey/siret/stype/sspy` sources + `winegcc` builds).

### Path re-walked (two full round trips, same containment)

Boot: `/tmp/o2jam-j10.fish` (3600 s launcher, Xvfb `:98`, ko prefix,
`PULSE_SINK=wine_null`). Notice → H-tab XTEST (145,105) → fallback OK
XTEST (400,362) → channel list (`03E8`/`03EA`) → warp 입장 (526,248),
blob-verify, `SendInput` single → `03EC → 0FBE → 07D0 → 07D2 → 13A4`
(+ `07DA`/`07E8`) → room list, twice (17:22:23, 17:23:59). 입장 aim
uses closed-loop screenshots now: warp, shoot, locate the orange cursor
by warm-pixel blobs (`/tmp/findcursor.py`, connected components; tip
≈ 13×21 px, n≈131), re-warp by delta. Tracking is exactly 1:1
(warp Δ moves the tip by Δ, pixel-verified); one warp fired during a
stage transition never moved the sprite, so every aim is re-verified
before firing. Button rects measured in framebuffer pixels: Ch.01 입장
face x 502–550 / y 236–260; 방만들기 green fill (511,374)–(599,402)
center (555,388); CREATE OK glyph centroid (397,465); NORMAL panel
(285,260); 3KEY DRUM (336,345); title field x 287–500 / y ~130–160.

### CREATE ROOM opens, OK never fires

방만들기 click (single round 1, double round 3) opens CREATE ROOM
(title `CREATE ROOM`, 방제목 field, NORMAL/LIVE/GHOST panels, 3KEY
DRUM/7KEY MIX, 비밀번호 field, OK pill). NORMAL click selects (orange
border, 3KEY/7KEY light up); 3KEY click selects (own border). Then OK,
sprite tip verified on the glyph centroid before every fire: single ×3,
double ×2, 1.5 s long-press ×1, `SendInput` Return ×2, Space ×1, XTEST
×1 — all genuine (`fg=0x10088`, `n=1`, `err=0`), zero visual change,
zero new `server-live.log` lines across all of them.

### Room-name attempt fails: keystrokes never reach the canvas edit

Per owner direction the title field was tried: it renders empty with a
blinking caret. Focus click + double-click on the field (tip verified
inside), then `/tmp/stype.exe.so` (`winegcc` `SendInput` typer)
`o2test` / `o2test2` — 13 characters, field pixel-identical before and
after. `/tmp/sspy.exe.so` (in-desktop window census) explains why: the
game window has NO child windows (title field is canvas-drawn, not an
EDIT), `GetFocus()=NULL`, `GetActiveWindow()=NULL`, and a `Wine IME`
window is present — the canvas edit's key pump takes nothing, likely
IME-related. Typing Latin into the room title is a second, independent
blocker on the same dialog.

### Side findings

- Room-stage fuse: ~40 s after entry a modal `서버로 부터 응답이
  없습니다` ("no response from server") notice appears (rounds 1–2;
  the late `07DA`/`07E8` pair precedes it by seconds). Its OK ignores
  all three mouse variants but `SendInput` Return (alone — `siret`)
dismisses it straight into `FFF0` disconnect + return to the notice
  carousel. Round 3 (CREATE open the whole time) ran 4+ min with no
  popup: the fuse belongs to the bare room list, not a global timer.
- Launcher hygiene gotcha: the 600 s `/tmp/o2jam-click.fish` timeout
  expired mid-attempt (17:20:28, `opendesktop=(nil)`, black desktop,
  no `.dmp` — kill, not crash). Re-ran under a 3600 s launcher;
  reaped at the end (`pkill -9 -x OTwo.exe`, zero remain, session
  close logged, no dumps, `install-root` untouched).
- Opcode census for the session: `03E8`/`03EA`/`03EC`/`0FBE`/`07D0`/
  `07D2`/`07DA`/`07E8`/`13A4` ×2 chains + one `FFF0`; nothing else.
  No room-create/join opcode exists in any Wine run to date.

## Attempt 11 — CLI-hosted room joined from Wine: WAITING ROOM reached (2026-11-08)

Goal: bypass the Attempt-10 CREATE-OK wall by hosting the room with our
own tooling (2004 `CreateRoom` via a scratch script against the local
Memoryer gateway), then click the listed room's join in the Wine client
and record how far it goes. Result: the client listed the hosted room
(`000 WINEJOIN (1/8)`, music set), a screenshot-verified double-click
sent `0BBA` (`JoinWaiting`) with no rejection, and the client rendered
the WAITING ROOM (both members seated, READY badges). No song select, no
gameplay: a READY click was followed by a self-exit (`0BBD`) at ~+65 s,
and the client has a ~60 s self-`FFF0` fuse in every idle room stage plus
a crash mid-`07D2` on one populated-list entry. Helpers are scratch-only
in `/tmp` (`host_room1{1,2,3}.py`, `o2jam-j11.fish`, `j11-*.png`,
`host_room1*.out/.log`). Memoryer unmodified (server only); no commit.

### Host path (no CLI create exists)

`recomp2jam connect --join-room` only JOINS (`2004 CreateRoom` exists
solely inside an env-gated live test). So the room was hosted by small
scratch Python scripts speaking the wire directly (length-preframed
`[u16 len][u16 id][body]`, null-terminated strings, `726PNA-…` client
id): Authorize (1000, user 2 / tester2 token, zero relay keys) →
GetChannelList (1002) → ChannelLogin (1004, 0/0) → CreateRoom (2004)
→ hold with 6001 pings. tester2 hosts so the Wine client's tester
session is never disturbed (same-account duplicates would kick).

- `/tmp/host_room11.py` (45 s trial): `WINEJOIN` created, `2006`
  result 0, room number 0, `07D4 Create room: WINEJOIN` in
  `server-live.log`. Hold works; room deletes on disconnect (with the
  known `RoomServiceEventPublisher.OnRoomDeleted` broadcast exception
  — pre-existing server behavior, untouched).
- `/tmp/host_room12.py`: tried PAD0 then `3005 ExitWaiting` (empty
  body) then a second create. Empty-`3005` decodes fail server-side
  (`MessageFieldCodec` length exception, no reply, session survives);
  the second create is rejected (`RoomService.AddRoom` session
  exception — already in a room). Lesson: `3005` has a non-empty body
  (still unknown; `o2jam-net` does not model it) and room numbers are
  reused lowest-free (first room is always number 0).
- `/tmp/host_room13.py` (used for all Wine runs, 900/1800 s holds):
  create `WINEJOIN` (number 0) → `4000 SetRoomMusic` 757/2/1 → `4002`
  `SetRoomArena` 0 → self `2002` poll: `2003 RoomList` 2412 B = 100
  entries, first holding `WINEJOIN` + `f5 02` (757). Server-side truth
  confirmed live.

Decisive display finding: the first Wine entry listed ZERO rows
because the room had no music set. `RoomListEntry.music_id` 0 renders
nothing; after `4000`, the row paints (`000 WINEJOIN (1/8)`, green
song bar with Korean title, `HX1`, `3KEY`, `WAITING`). Room number 0
displays fine — the `0 = empty slot` convention is decoder-side only.

### Wine walk (same proven recipe, ko prefix, Xvfb `:98`, null sink)

Boot (`/tmp/o2jam-j11.fish`, 900 s launcher) → error-report dialog on
reboots (leftover game `.dmp`; Cancel via XTEST at (420,295) dismisses,
no send) → notice carousel → H-tab XTEST (145,105) → `03E8`/`03EA` →
fallback OK XTEST (400,362) → channel list → warp Ch.01 입장 (526,248),
crop-zoom tip-verified on the glyph → `SendInput` single (`siclick3`,
`n=1/1`) → `03EC` in 12–25 s → `0FBE→07D0→07D2→13A4` chain.

Timing anatomy of the room stage (measured over 5 entries): `07DA`/
`07E8` (user list + music report) at +36 s; the room ROW paints with
that cycle (empty at +25 s, painted by +40 s); a `서버로 부터 응답이
없습니다` modal fires ~+40–60 s; the client self-sends `FFF0` at
~+60 s (`17:42:42→17:43:40`, `17:58:38→17:59:38` both +58–60 s) and
AUTO-REAUTHORIZES ~+10 s later (`03E8`/`03EA` with zero input, then
channel dialog + fallback popup again). The loop is autonomous: entry
→ row → modal → self-`FFF0` → re-auth → channel. Manual `입장` clicks
are still needed after each re-auth (one auto-`03EC` at 17:42:42 had no
visible driver and never recurred when watched — treat as unattended).

### The join (18:02, screenshot-verified)

Entry `03EC` 18:01:28. After `07DA`/`07E8`, blind warp (175,165) +
`SendInput` DOUBLE-click (`siclickD`, `n=1/1` ×4) on slot 1, post-hoc
shot for aim. Server: `0BBA` (`JoinWaiting`) from tester, accepted
(no reject code; old `0BBA` lines in the log are Nov-06/07). Client:
full WAITING ROOM (`/tmp/j11-join2.png`): title bar `사람손 Ch. 01
No. 000 HX1 WINEJOIN 방설정변경` + song bar, 8 slots with slot 1
`Lv.2 tester2` (red, READY) and slot 2 `Lv.2 tester` (orange, READY),
TEAM grid, orange READY pill at ~(680,143), SKILL grid, SPEED X1,
BPM 124, song `?` panels, chat + piano keys. Reached: waiting room.
Not reached: song select (panels stay `?` for the guest), gameplay
(host script never sends `4010 StartGame`; guest cannot).

READY pill single-click (`n=1/1`) → no new opcode within 25 s; at
+65 s the client self-exited the waiting room (`0BBD ExitWaiting`,
18:03:42 — same ~60 s fuse shape) back to the room stage (`07D2`
re-list, session alive). A CREATE ROOM dialog was open over the room
stage afterwards (opener unknown: the only input in the window was the
READY click at (680,143); possibly fuse behavior). Its X ignores the
proven `SendInput` single, and a stacked fallback-OK popup ignores
XTEST this time (channel-stage variant responded in Attempts 4/10) —
the stack was left standing. Reap followed.

### Crash (17:51 entry, populated list)

`03EC → 0FBE → 07D0`, then nothing: no `07D2`/`13A4`. Wine crash
box (`OTwo.exe has encountered a serious problem`, winedbg attached)
over the room stage. The game wrote its own minidump
(`/tmp/o2jam-run/OTwo 2026-11-08 17-55-29.dmp`, deleted after Cancel
on the reboot send-prompt — hence the dialog). Same populated list
survived twice, so the crash is intermittent, not deterministic on
list content. No `install-root` write; prefix intact.

### Opcode census (tester, this attempt)

Per visit: `03E8/03EA/03EC → 0FBE → 07D0 → 07D2 → 13A4 → 07DA+07E8`
(+36 s) → modal (~+40–60 s) → self-`FFF0` (~+60 s) → auto
`03E8/03EA` (~+70 s). Once each: `0BBA` (join, accepted), `0BBD`
(waiting exit). Never from Wine: `3005` body (unknown), `4008`
Ready effect (click sent nothing observable), `4010+` play flow.
Host (tester2): `07D4` create, `0FA0/0FA2` music/arena, `2003` polls.

### Hygiene notes

- `/tmp/o2jam-j11.fish` once vanished mid-attempt (rewritten, no
explanation; prefix and game copy verified intact) and one `cd`
failed transiently; later `ls` showed the dir present. No repo,
Memoryer, or install-root file was touched for this.
- `pkill -f <name>` self-match still bites: use bracketed patterns;
one launcher died by it (exit 143), recovered.
- Audio stayed on `wine_null` throughout; everything on Xvfb `:98`;
reaped to zero (`OTwo`, wineserver, hosts); Xvfb left running.

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
tabs. Update 2026-11-08 (Attempt 10): 입장 wall stays breached — the
new wall is the CREATE ROOM OK button (8 distinct stimuli fail with
the tip verified on-rect: single x3, double x2, long-press, Return x2,
Space, XTEST) plus canvas title-entry (13 SendInput chars, zero pixels;
no child windows, no focus, IME present). No waiting room, song select,
or gameplay reached. The bare room list also carries a ~40 s
server-timeout fuse whose notice exits only via disconnect (Return →
FFF0 → carousel). Next: an IME-aware key path (scancode/Unicode sends,
Han/Eng toggle) for the title, or evidence for what enables CREATE OK.
2. Init-time `ACCESS_VIOLATION`s (game RVA `0x1F2893` with session argv;
   wild jump to `0xFFBC11B0` in one bare run with media DLLs loaded).
3. Possible Wine/wow64 contribution to (2): identical wild address in an
   unrelated winedbg helper thread.

## Attempt 11 — CREATE ROOM Phase 1: investigation only, no fix (2026-11-08)

Goal: explain two linked symptoms — CREATE OK ignores all input, title
field takes no keys — without changing anything. No fix was attempted;
no new injection was fired; all evidence is read-only observation of the
live Attempt-10 session plus static doc cross-checks.

### Reproduction (consistent, still live)

- Wine client `OTwo.exe` (pid 1819201, booted 17:33 under
  `/tmp/o2jam-j10.fish`: Xvfb `:98`, ko prefix, `PULSE_SINK=wine_null`)
is still alive and ESTAB (`127.0.0.1:55508 → 15010`).
- Current framebuffer (`/tmp/j11-now.png`, 17:40) shows the room stage
  behind the `서버로 부터 응답이 없습니다` fuse modal + OK — i.e. the
  bare-room-list ~40 s server-timeout path from Attempt 10, not a new
  crash. The CREATE ROOM dialog is no longer on screen (fuse modal
  covers the stage that owned it).
- Server log for this exact Wine session (`tester`, ports
  55508/32388): `03E8 → 03EA → 03EC → 0FBE → 07D0 → 07D2 → 13A4 →
  07DA → 07E8`, zero `07D4` (2004) lines. Reproducible across all
  Attempt-10 rounds: the Wine client NEVER emits CreateRoom.
- Counter-proof the server accepts 2004: a bot session (`tester2`,
  `python3` pid 1820111, port 51252) logged `07D4 Create room:
  WINEJOIN` 1 s after Authorize at 17:39:56. Gate is client-side.

### Recent changes (client unchanged, environment is the variable)

- Client bytes unchanged: `sha256sum /tmp/o2jam-run/OTwo.exe` =
  `83a04541…` prefix, matching the pinned hash in `docs/lifecycle.md`
  and `install-root` copy (`md5` identical both sides). `git log`
  since shows docs-only commits. Nothing changed in the client.
- Environment that DID change today: fresh ko prefix clone
  (`~/o2jam-prefix-ko`), `LOCALPATH/ko_KR.UTF-8` + CP949 + Baekmuk/Noto
  fonts + `FontLink\SystemLink`, null-sink audio, 3600 s launcher,
  local Memoryer gateway. All Attempt-10 stimuli ran inside this env.

### Layer evidence

- SendInput delivery CONFIRMED (Attempt 10): all 8 OK stimuli genuine
  (`fg=0x10088`, `n=1`, `err=0`: single ×3, double ×2, long-press,
  Return ×2, Space, XTEST) — yet zero pixels, zero `server-live.log`
  lines. Delivery works; the dialog consumes nothing.
- Focus state: fresh `sspy` census (this task, read-only) repeats
  Attempt 10 exactly: `fg=0x10088`, `GetFocus()=NULL`,
  `GetActiveWindow()=NULL`, NO child windows of the game hwnd
  (title field is canvas-drawn, not an EDIT control).
- IME state: `Wine IME` + `Default IME` windows present in the same
  census. `stype` vkey typer (`o2test`/`o2test2`, 13 chars, tip verified
  inside the field) left the field pixel-identical. Keys reach the
  queue but never render — consistent with IME swallowing the vkey
  path for a canvas edit with no focused EDIT.
- Dialog validation logic (exe/docs side): `DIALOG_CREATEROOM`
  (decl `0x09`, `docs/gameplay.md`; `CDCreateRoom`, `docs/lifecycle.md`
  §7) carries title (max 22, `docs/net-protocol.md` §3b/4c: 2004
  `CreateRoom` title-first wire layout). No in-exe OK-enable function
  is pinned yet (OPEN) — but every observed CREATE ROOM frame shows
  the title field EMPTY with blinking caret, and OK never fires while
  it is empty. Title-empty gating fits all 8 failures with one cause.

### Ranked hypotheses (Phase 1 verdict)

1. Title-gated OK + IME-swallowed keys (BEST FIT): CREATE OK requires a
   non-empty title; the canvas edit takes no vkey input under Wine
   (no EDIT, no focus, Wine IME present), so the title stays empty and
   OK stays disabled. Explains BOTH symptoms at once; server-side is
   ruled out by the bot `07D4` success.
2. OK hit-test/poll path broken under Wine independent of title —
   explains OK but NOT the 13-char typing failure.
3. Global focus loss disables the dialog — weaker: message-path
   controls (H-tab, fallback OK) work with the same focus state.

### Single minimal confirmatory test (NOT run — Phase 1 only)

- One `SendInput` keystroke with `KEYEVENTF_UNICODE` (`wScan` = `A`)
  into the title field, then a framebuffer pixel-diff of the field
  rect. If one glyph appears, the vkey→IME translation is the failing
  layer (hypothesis 1 confirmed at the key layer); if the field stays
  pixel-identical, the canvas edit ignores all key pumps and the OK
  gate must be probed separately.

## Attempt 12 — UNICODE confirmatory test: GLYPH APPEARS (2026-11-08)

Verdict: GLYPH. One `SendInput` `KEYEVENTF_UNICODE` (`wScan` `0x41`) into the CREATE ROOM title field replaced the selected default title with `A`. Field-rect crop diff (280,125)–(500,165): bbox 70×17 px exactly over the text, 1155/8800 px changed. Delivery was genuine (`fg=0x10088`, `n=1/1`, `err=0` down and up). The vkey→IME translation layer is guilty. Fix = a UNICODE pump path for canvas edits.

### Method (one test datum)

Fresh scratch copy (`OTwo.exe` hash `83a04541…` pinned), ko prefix, Xvfb `:98`, null sink, pinned blob+mirrors argv, cwd `/tmp/o2jam-run`. Path: notice → H-tab XTEST (145,105) → fallback OK XTEST (400,362) → Channel → `입장` SendInput (526,248) → `03EC` → room list → `방만들기` SendInput (555,388) inside the ~40 s fuse → CREATE ROOM open. Focus click at (335,143) left the default title selected. Before-shot, ONE `siuni` down/up, after-shot 3 s later, crop-diff. No `07D4` was emitted (OK never pressed); no gameplay followed.

### What this proves

Keys reach the queue (Attempt 10) but vkey chars die in IME/vkey translation for the canvas edit; UNICODE bypasses that layer and renders. Title-empty gating is not the whole story: this run pre-fills a default title (`신나는 음악을 즐겨라!`; `니들이 리듬을 알아?!` seen once) — never empty — yet no `07D4` exists in any Wine run. CREATE OK enablement stays OPEN.

### Side findings (setup observations, zero extra test inputs)

- cwd must be the game dir: 4 consecutive +2 s init `EXCEPTION_ACCESS_VIOLATION`s (game's own `.log`) with cwd at the repo root; every boot with cwd `/tmp/o2jam-run` survives init. Same bytes, reproducible pattern.
- Init/transition AVs stay flaky: +2 s crashes (18:06/18:09/18:11/18:16), a winedbg-caught crash on room entry (17:51), silent deaths in the H-tab→channel black transition with opcodes still flowing (18:24/18:29, no dump).
- Client timers confirmed: auto channel-enter (~100 s after H-tab) and auto room-join (`0BBA Join room [000]`, ~72 s later) with zero input (18:01 and 18:24 cycles).
- Corrects Attempt 10: the title field is NOT always empty — it pre-fills a selected default title when the dialog opens.
- Multiplayer note: a second agent drives a same-identity client on `:99` (own prefix/display/copy, contained). Same-user sessions churn (`FFF0` 18:44:56 mid-CREATE, client unaffected — CREATE is fuse-immune, matching Attempt 10 round 3). A `/tmp` purge mid-task deleted all scratch launchers, `o2jam-run`, and reports; everything was rebuilt from repo + DB (hashes re-pinned).
- Shots: `/tmp/uni-before.png`, `/tmp/uni-after.png`, `/tmp/uni-f-before.png`, `/tmp/uni-f-after.png`, `/tmp/uni-m1.png` (CREATE open), `/tmp/uni-field-m2.png` (selected default title). New helper `/tmp/siuni.c` (+`winegcc` build).

## Attempt 12 — host-driven 4010 reaches the loading screen (2026-11-08)

Goal: from the CLI-hosted waiting room, ready the bot if needed and send
`4010 StartGame`, then observe loading/playing. Result: GAMEPLAY STAGE
REACHED — the client rendered the song loading screen and sent `4012
ConfirmMusicLoaded`. Active note-play was not observed: the scratch host
bot never sends `4012` itself, so the server's 60 s `MusicLoadTimeout`
(`~/memoryer-local/config.ini`) closed the host session while the client
waited on the loading screen. That last gap is bot tooling, not a client
wall. Helpers, launchers, shots, and logs are scratch-only in
`~/scratch` (`host_wait.py`, `host_flap.py`, `o2jam-ws.fish`,
`warp99.py`, `xclick99.py`, `sesc.c`, `siclick{3,D}.c`, `killer.py`,
`ws-*.png`, `w*.png`, `host_*.log`). Memoryer unmodified (server use
only). No commit.

### Path walked (Xvfb `:99`, private prefix copy, null sink)

A private prefix (`~/scratch/prefix-ws`, clone of `o2jam-prefix-ko`)
and full game copy (`~/scratch/run-full`) on `:99` isolate this drive
from the parallel `:98` session (an exe-only dir crashes in init with
`EXCEPTION_ACCESS_VIOLATION` at game RVA `0x77a8f`, media DLLs loaded —
cwd must be the game dir). Boot with the pinned blob+mirrors argv:
notice carousel → H-tab XTEST (145,105) → fallback OK (400,362) →
channel list (`03E8`/`03EA`) → warp + `SendInput` single on the Ch.01
입장 rect (526,248) → `03EC → 0FBE → 07D0 → 07D2 → 13A4`.

### The room-list fuse and its suppression

Every idle room-list entry grows a `서버로 부터 응답이 없습니다`
("no response from server") modal ~+37–44 s after `03EC` (row paint
rides the `07DA`/`07E8` poll at +36 s, so the join window is ~3 s) and
self-sends `FFF0` ~+55–60 s. Modal-up row double-clicks never emit
`0BBA` (4 drained attempts). Suppression that reproduces: a host
room-recreate pulse (disconnect + re-create → `2007`/`2005` channel
events) completing before the modal fires keeps the list live and
modal-free past +54 s; entries landing on fresh `2005` data paint
with same-second `07DA`/`07E8`. Proven twice (joins `0BBA` 19:02:53
and 19:32:43, both with screenshot-verified pre-warped double-clicks
at (175,150)). Arena-flap keepalive (`4002` alternating 0/1) does NOT
suppress the fuse.

### Arena flaps cause the DXSurface error (false 4010 failure first)

With flaps on, the waiting room grew `인터페이스 정보를 로딩하지
못했습니다 … : 660, .\Display\DXSurface.cpp` ("failed to load
interface information") — first seen right after `4010` (music 757)
and misread as a StartGame failure, then reproduced with no `4010` at
all. With flaps off the same waiting room sat clean 90 s+ (both READY).
Reading: an in-room `4002`/`4003` arena change makes the client reload
stage art and trip `DXSurface.cpp:660` under Wine. Keepalive flapping
stays off for play.

### The start (music 757, flaps off, room 000)

Join `0BBA` 19:32:43 (host-side `0BBC` join event confirms one room);
guest not auto-ready this round, so a tip-verified `SendInput` single
on the READY pill (680,143) sent `4008` (`0FA8` 19:33:49) and lit both
badges (tester2 red, tester orange). Host `4010` (empty body) 19:34:19
→ server `0FAA` → `4011` (`01000000`) → client `0FAC` (`4012`
`ConfirmMusicLoaded` 19:34:19.552) + full-screen song loading art
(`미쳤어`, 3KEY VERSION badge, U1/U2 load dots, BPM 124). No `4014`/
`4016` score flow followed: the bot host never confirms music load,
and the server closed it at the 60 s `MusicLoadTimeout`; the client
was still on the loading screen at last shot, then reaped to zero.

### Opcode census (tester unless noted)

Per entry: `03E8`/`03EA`/`03EC → 0FBE → 07D0 → 07D2 → 13A4` +
`07DA`/`07E8` (+36 s, or same-second on fresh `2005`). Joins: `0BBA`
→ host `0BBC`. Starts: guest `4008` (`0FA8`), host `4010` (`0FAA`)
→ `4011` (`01000000`) → guest `4012` (`0FAC`) + host-side `4049`
bursts. Exits: `0BBD` never needed; `FFF0` on every modal dismissal
(Return, Escape, and XTEST-OK all end the session) and on error-modal
dismissal (clean client exit, no dump).

### Hygiene notes

- `pkill -f <name>` matches its own `fish -c` line (exit 143/137
  twice); killer is now a `/proc`-scanning `killer.py` (exact marker,
  self-PID excluded). One over-broad `OTwo` sweep killed the parallel
  session's client once (theirs relaunches from its own launcher).
- Post-`FFF0` H-tab re-walk re-authenticates with no reboot
  (`03EA` in ~10 s); BACK is inert in the waiting room and in a
  masterless room every input is inert (BACK/Return/H-tab) — kill +
  reboot is the only exit.
- A host recreate renumbers the room (lowest free; seen 000 and 001)
  and a recreate between join and `4010` orphans the guest in a ghost
  room (join first, freeze the room, then start).

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

## Attempt 13 — bot 4012 + full song-flow: loading reached, play wedged (2026-11-08)

Goal: fix the scratch host bot to send its own 4012, keep flaps off,
freeze the room before 4010, then observe 4014 ticks, 4016 submit,
result screen. Result: the bot fix works end to end — both 4012s flow,
the server runs the whole song (bot 4014 x10, 4016, 4041, 4017 naming
the master), but the Wine guest never leaves the song loading screen:
no guest 4014/4016, no 4018, no result screen. New wall is client-side
(Wine), not protocol. Helpers, launchers, shots, logs scratch-only in
`~/scratch` (`host_play.py`, `siclickwait.c`/`siclickwait.exe.so`,
`refocus.py`, `race.py`/`joinspam.py`/`gridspam.py`/`hotspotspam.py`,
`paintpoll.py`/`bootpoll.py`/`track.py`, `o2jam-ws.fish`, `*-notice.png`,
`*-chan*.png`, `*-room*.png`, `*-wait.png`, `*-load.png`, `*-play*.png`,
`z-*.png`, `host_play.log`). Memoryer unmodified (server use only).
No commit, no push.

### Bot fix (the assigned gap)

`host_play.py` (from `host_flap.py` lineage): on the `4010` trigger it
sends 4010 StartGame (empty) then immediately 4012 ConfirmMusicLoaded
`struct.pack("<iB", 0, 1)` (power 0, speed 1 — the room's own
music/speed, matching the guest's `0FAC "0 / X10"` shape). Then the
full CLI song-flow (`connect.rs run_song_flow` order): 4036 (member 0)
+ 4038 (100, member 0) with the start, ten 4014 Jam ticks from +10 s,
4016 + 4041 at +45 s (non-blocking schedule inside the ping loop, so
6001 keepalive never stalls). Flaps off (single 4002 arena 0 at create
only). Recreate allowed until the 0BBC guest-join event, ignored after
(room frozen before 4010 per task). Verified live: 0FAA then bot 0FAC
at +0.002 s, guest 0FAC at +0.4 s, both 4013s (0FAD `0001` + `0101`),
4048/4049 latency exchange (11 rounds), bot ticks as ten 0FAE, bot
submit as 0FB0, bot 4041 as 0FC9, server 4017 (0FB1 slot 00).

### Walk (Xvfb `:99`, prefix-ws, null sink, music 757, room 000)

Notice (shot) → H-tab XTEST (148,81) → `03E8`/`03EA` (~15-40 s) →
fallback-OK XTEST at detected pill (401,360) → channel list → 입장
single at detected button (520,246) → `03EC → 0FBE → 07D0 → 07D2 →
13A4` (12-25 s) → room list → row double-click → `0BBA` → WAITING
(both READY after guest `4008` at 677,143 → `0FA8`) → host `4010` →
`0FAA`/`4011` → both `0FAC`/`4012` + guest `0FD7` → song loading art
(shot: `미쳤어`, 3KEY VERSION, U1/U2 dots, BPM 124). Stages reached:
notice, channel, room list, waiting room, loading. Never: active play,
submit, result.

### Join aiming (wall broken this attempt)

All earlier row double-clicks aimed at the panel middle (175,160 — the
green-bar/badge zone) and failed ~30 times across cycles. Grid probing
found the hot zone: the row TITLE bar (`000 WINEJOIN`), e.g. (100,100)
joins first try; (200,108)/(250,150)/(175,160) never emit `0BBA`.
Programmatic rule used throughout: never trust eyeballed pixels — every
aim came from a detector (pill-profile, red-run, green-count baselines).
X-warps land exactly 1:1 (two-shot diff proof); the visible orange
arrow tracks warps, but eyeball reads of it err by ~100 px.

### Fuse-modal timing + cache finding

The `서버로 부터 응답이 없습니다` modal fires ~+35-44 s after 03EC,
with the row paint on the +36 s `07DA`/`07E8` poll — a ~1-8 s join
window per cycle. Two exceptions, both post-FFF0 re-auth entries:
03EC and 07DA in the SAME second (20:40:26.9, 20:58:14.8) with instant
paint — first entry per boot waits +36 s, later entries paint from
cache. Reliable loop used twice: modal → Return (`siret`, sends FFF0)
→ H-tab re-auth (~40 s, slower than boot-time) → walk → instant-paint
entry → join at (100,100) pre-modal. Second Return is sometimes needed
to dismiss the modal visually (first sends FFF0 but leaves it painted;
modal loop eats all input while up). Post-FFF0 Escape paginates, so
input is NOT dead — earlier "dead input" verdicts are retracted;
re-auth just needs patience.

### Focus pin (owner note) + loading wedge (new wall)

Owner: losing window focus throws the client back to waiting/room.
X focus here sat on the wrong window (`0xe00001`) for whole drives;
probed chain is desktop `0xa00007` → container `0xe00001` → canvas
`0xc0004e` (all 800x600 at 0,0 — no fullscreen/scale; mapping is 1:1).
This drive pinned focus on `0xc0004e` before the join and kept it with
a 5 s re-assert keeper (`refocus.py`): zero lapses during play, yet the
guest still never leaves loading. So focus is necessary but not
sufficient — the wedge is deeper (suspects: play-stage BGA/DXSurface
under Wine, or a start barrier the silent bot never satisfies; the
render loop spins hot at 228% presenting frozen frames, no files open,
loading ignores click/Return/Escape, no crash dump).

Server side of the wedge: with bot 4016 + 4041 in, the server emits
4017 (naming the master) but never 4018 — it waits on the guest's own
4016, which never comes. Guest 4014/4016 absent in every cycle.

### Tooling notes (scratch only)

- `siclickwait`: persistent `winegcc` SendInput single/double clicker
  (pays Wine startup once; `Z:`-path file triggers). msvcrt `fopen`
  silently misses the trigger file on the game desktop — kernel32
  `CreateFileA`/`ReadFile` works. Trigger poll 200 ms; one instance
  only (two racing instances caused a lost cycle).
- `wineserver -k` (own prefix only) is REQUIRED before every client
  relaunch — otherwise the new explorer dies in `X_CreateWindow`
  `BadWindow` on the stale `0xa00007` id (seen 2/2 rapid relaunches).
- `pkill -f` self-matches its own `fish -c` line; killer stays the
  `/proc`-scanning `killer.py` with bracketed markers.
- Host `-2` auth right after killing the old bot: the server holds the
  dead tester2 session ~30-60 s; restart the bot after the reap.
- Containment held: `:99` + null sink throughout; `install-root`
  untouched (music files only READ: `o2ma757.ojm/.ojn` confirmed
  present locally); parallel `:98` session never touched (own
  prefix/display/copy); everything reaped to zero at the end.

### Opcode census (tester guest unless noted)

Per walk: `03E8/03EA/03EC → 0FBE → 07D0 → 07D2 → 13A4` (+`07DA`/`07E8`
at +36 s or same-second post-FFF0). Joins: `0BBA`. Ready: `4008`
(`0FA8`). Starts: host `4010` (`0FAA`) + host `4012` → guest `4012`
(`0FAC` + `0FD7`) → `4011`/`4013` (`0FAB`/`0FAD` x2, host-observed).
Play (host): ten `4014` (ten `0FAE`), `4016` (`0FB0`), `4041` (`0FC9`)
→ `4017` (`0FB1` slot 00). Never from the guest in any cycle: `4014`,
`4016`. Never from the server: `4018`. Exits: `FFF0` on every modal
Return; `0BBD` never needed.

## Attempt 12 — bare PLANET BOARD reached: channel dialog is a modal child (2026-11-09)

Input was already unblocked (Attempts 9-11: position-verified SendInput
clicks advance). This pass used the SendInput helper
(`~/scratch/button-unblock/siclick3.exe.so`, winegcc-built, warps then
clicks at the cursor) to walk past the channel list: the channel
dialog's X button at native (557,195) closes the dialog and reveals the
**bare STATE_PLANET board** underneath — five emblem buttons in the top
strip, the promotion carousel (auto-cycling banners), and the MENU bar.
Captures: `~/scratch/button-unblock/planet-{dialog,bare-1,bare-2}.png`,
`grid-top.png` (emblem strip, 2x with coordinate grid), `w3-list*.png`
(promo cycle), `exit-state*.png`. The MENU-bar power button opens a
quit-confirm MESSAGE dialog (`게임을 종료하시겠습니까?`, OK/CANCEL).

Findings land in `docs/planet.md` ("Real board structure"). Session
notes: the supervisor/drive scripts (`supervisor2.log`, `drive_ui.py`,
`walk_planet.py`) are repeatable; the winedbg stopped-state read of the
CState current-state field was interrupted by the known helper-thread
fault (c0000005 at 0xffbc11b0) and continues under
`~/scratch/planet-verify/`.

# Playtest — end-to-end vs local Memoryer (2026-11-06)

Server: `~/memoryer-local`, Memoryer 7.2.0, `Mode=Full`,
`127.0.0.1:15010`, `FreeMusic=true`, `SessionExpiry=0`.
Account: `tester` (USER_INDEX_ID 1,
token `61dcb457-5da9-4e22-af4b-5a4c5658f239`).
No game bytes touched; Memoryer used as reference only.

## Full loop (network) — PASS

`cargo test -p recomp2jam live_host_song_flow_against_local_gateway`
with `O2JAM_LIVE_GATEWAY=127.0.0.1:15010` → **ok** (0.62 s).
Server log shows the same session, in order:

- `Authorize session` → `Get channel list` → `Enter channel [0/00]` →
  `Get character info [tester]`
- `Create room` (hosted room `songflow`)
- `Update room music` (select 757) → `Get music list` (562 entries)
  → `Sync room member music state` → `Update download progress` →
  `Start game` → `User music loaded` → `Game Update` (Jam tick +
  peer-bar echo) → `Game Submit score: 2210`
- `Finalize rank` + `Leave game` are sent by `send_tally` and were
  server-logged for the same flow earlier today (`tester2` run:
  `Submit score` → `Finalize rank` → `Leave game`); this run closed
  the session before the server flushed them (broken-pipe broadcast
  race on room delete, benign).

CLI `recomp2jam connect --host 127.0.0.1 --user-id 1 --token … \
--gateway-id 0 --channel-id 0 --character` → **ok**:

- `authorize: Success (0)`, `channels: 10` (0–4, 9 open),
  `channel: Success (0) (unrestricted)`,
  `character: tester (Lv 1, Male)`, `rooms: 0 open`.

Room join via CLI was not possible: 0 open rooms existed, and joining
a missing room hits the known Memoryer wart (throws
`ArgumentOutOfRangeException`, no reply — see `docs/client-server-plan.md`
Phase 2). The room step is proven by the live test's hosted create
instead. No invented passes.

## Scenes (windowed client, spectacle) — 1 PASS, 6 FAIL (blocked)

Screenshots: `docs/screenshots/`. Command used:
`spectacle -b -a -n -o <file>` (active window — `-f` fullscreen
silently writes nothing in this session).

| Scene | Verdict | Evidence |
|---|---|---|
| Avi (splash) | **PASS** | `avi.png`: real Nowcom logo from `install-root/O2Jam/Image/Interface1.opi` (`O2mediaLogo.ojs`, 1 layer) |
| Channel | **FAIL** (blocked input) | client stuck on Avi; see below |
| Room | **FAIL** (blocked input) | same |
| Waiting | **FAIL** (blocked input) | same |
| Loading | **FAIL** (blocked input) | same |
| Playing | **FAIL** (blocked input) | same |
| Result | **FAIL** (blocked input) | same |

`input-blocked.png` shows the client still on the Avi splash after
EWMH activation plus Space+Enter through a uinput keyboard — the walk
never starts. Root causes, all verified in-session:

- XTEST fake input is dropped by Xwayland (own probe window got 0/3
  keys; `query_extension` says present, delivery does not happen).
- uinput devices get no `seat`/`uaccess` tags (only `:power-switch:`,
  even with `ID_INPUT_KEYBOARD=1`), so logind grants no user ACL
  (`getfacl` shows no `arnim` entry) and KWin/libinput never opens the
  device. A global MUTE key did toggle (root-level handler), proving
  kernel delivery but not window delivery; `kwrite` received nothing.
- No fallback exists here: no Xephyr/Xvfb/xdotool/wtype/ydotool,
  `sudo -n` needs interactive auth (not pursued).

## Regression

`cargo test --workspace` (no live env): **444 passed, 0 failed**
(15 binaries), `exit=0`. The live-gateway test above ran separately
with env vars and passed.

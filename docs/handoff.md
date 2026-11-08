# Handoff — recomp2jam (2026-11-08, afternoon)

## Standing goals and their state

1. **Binary connects to Memoryer — COMPLETE and extended.** `recomp2jam
   connect` drives authorize → channels → channel login → character →
   avatar → rooms → room join, all live-proven against local Memoryer
   7.2.0. Since the last handoff the online session loop grew a
   live-proven path: lobby chat (send + 2016/2017 route), room join,
   member-update drain (ready/empty), song select with chart fetch,
   4008 ready → 4011 advance, loading sync, score broadcasts (4014/4016
   server mirror), and tally/FinalizeRank + LeaveGame. The scripted live
   flow (`live_host_song_flow_against_local_gateway`) stays green
   (~0.6 s, score 2210).
2. **GUI from start to playable — IN PROGRESS.** In-process scene walk
   (`recomp2jam shots`) is 7 PASS: Avi (real `O2mediaLogo` art),
   Channel, Room, Waiting (8 seats, ready dot), Loading (progress bar),
   Playing (highway + falling note), Result (score/combo). Added since:
   avatar scene + tally strip, result anchors/life-residual, music-select
   dialog structural parity, waiting region parity, channel scene
   icons/hold-bits/numfont, spriteless-channel staging. Real-art shots
   layer staged genuine art; placeholder shell art remains the known
   composition gap (`docs/parity.md`). No windowed walk yet — input
   injection is the blocker (see Wine rig).
3. **Real exe under Wine+Xvfb — REACHED live `STATE_CHANNEL`, BLOCKED
   at canvas input.** `docs/wine-boot.md` Attempts 1–6: pinned
   blob+mirrors argv boots the 7.55 client on Xvfb `:98`, dismisses the
   notice carousel (H-tab click) and the network-fallback popup (OK
   click) to a live channel list (Ch. 01–04, `03E8`+`03EA` in
   `server-live.log`). Attempt 6 proved the wall: pointer motion/hover
   reaches the DirectDraw canvas (cursor tracks, row highlight moves)
   but no press, double-click, long-press, or key (focus fixed) activates
   any canvas button (`입장`, pager, dialog X) — no `03EC` ever from Wine.
   Correction recorded: an earlier `03EC → … → 0FB0` session in the log
   was a scripted protocol bot, not a client click.
4. **Rust audit — COMPLETE.** `docs/audit.md` holds per-crate scores,
   deviation log, and an empty backlog; re-proofs logged in
   `docs/playtest.md`.

## Repo state

- HEAD `398d5ee` (`docs: channel input blocked over wine`,
  2026-11-08 16:37 +0800), branch `main`, in sync with `origin/main`,
  tree clean (`git status --porcelain` empty).
- Gates (verified this session on HEAD): `cargo fmt --check` clean,
  `cargo clippy --workspace --all-targets -- -D warnings` clean,
  `cargo test --workspace` **1616 passed, 0 failed, 0 ignored**
  (`exit=0`). Per target: audio 20, dump 7, file 180, game 108,
  net 420, render 89, shell 585, real_art 7, scenes 22,
  recomp2jam 178 (6 doc-test targets, 0 each). Release build clean,
  0 warnings. Live flow re-proven on HEAD itself by the wine-head sweep
  (uncommitted peer edit in `docs/playtest.md`, 2026-11-08 08:41 UTC:
  0.61 s, `Create room songflow → Start game → Submit score 2210`,
  second session walked channel list → enter → character info).
  HEAD adds only docs (`a3fba88`, `398d5ee`), no code change.
- Recent arc (all pushed): grade numerator/max-reference pins
  (`2130d0e`, `a98d89c`, `d7c3873`), lane-backing split
  (`5610f55`), grade-base fix + wine boot proof (`72ea4ca`), launcher
  argv pin (`e725060`), channel-live log re-proof (`a3fba88`),
  input-blocked finding (`398d5ee`).

## Running processes (observed 2026-11-08 ~17:00 +08:00)

- Memoryer server, up since Nov 06: `dotnet bin/Memoryer.Encore.dll`
  (PID 189399, child of a `bash -c` redirecting to `server-live.log`).
  Restart: `cd ~/memoryer-local && PATH=$HOME/.dotnet:$PATH dotnet
  bin/Memoryer.Encore.dll` (logs to `server-live.log` when redirected).
- `Xvfb :99 -screen 0 1024x768x24` (PID 1752389) and
  `Xvfb :98 -screen 0 800x600x24 -fbdir /tmp/xvfb-fb` (PID 1754921,
  the Wine-game display).
- Two `winedevice.exe` under Wine (PIDs 1772905/1772933, since 15:58).
- One `target/debug/deps/recomp2jam-…` test-harness process was present
  at check time (16:41 start) — likely residue of a live-test run;
  re-check with `ps` before trusting port `:15010` state.

## Environment notes (hard-won — read before touching GUI/net)

- This is a **Wayland** session (`WAYLAND_DISPLAY=wayland-1`,
  `DISPLAY=:0`); winit opens Wayland windows. X info via X11 (`:0`)
  screenshots black — use **`spectacle -b -n -o file.png`** for real
  captures. `ffmpeg` lacks x11grab; `pip mss` + `python-xlib`
  installed user-local for X inspection.
- Wine rig: `wine` + `Xvfb` present (`/usr/bin`). Game display is
  `:98` (800x600, fbdir `/tmp/xvfb-fb`); `:99` is the spare
  1024x768 head. Korean Wine prefix per `docs/wine-boot.md`
  (Attempts 4–6 recipe: `PULSE_SINK=wine_null`, `DISPLAY=:98`,
  `timeout -s KILL 600`, pinned blob+mirrors argv from
  `docs/lifecycle.md`). Audio stays contained: null sink only —
  never let game sound reach the speakers (owner rule).
- Input limits, current: XTEST motion reaches the canvas but presses
  and keys do not (Attempt 6, `docs/wine-boot.md`); uinput device node
  exists (`/dev/uinput`) but earlier had no seat ACL — unretried since.
  No Xephyr/xdotool/wtype/ydotool. Message-path controls (HTML
  notice tabs, fallback OK) accept XTEST clicks; DirectDraw-canvas
  buttons do not.
- dotnet 10 SDK at `~/.dotnet` (user-local install script). Memoryer
  source at `/tmp/mozart` (scratch copy), build at
  `/tmp/mozart/Build/Release`.
- Server config: `~/memoryer-local/config.ini` (relay at 127.0.0.1,
  Sqlite). Test account `tester` (any password works after
  `user:authorize`; last token `61dcb457-5da9-4e22-af4b-5a4c5658f239`,
  `SessionExpiry=0` so it persists). Gateway `127.0.0.1:15010`,
  `Mode=Full`, `FreeMusic=true`, 562 musics.
- **Never pipe a long-lived client into `head`**: SIGPIPE kills it when
- **Scratch goes in `~/scratch`, never `/tmp`** (`/tmp` is tmpfs RAM; worker probes, screenshots, and helper builds pile up gigabytes).
  head exits — redirect to a file instead.
- Memoryer reference layouts: `/tmp/mozart/Source/Memoryer/...`; Encore
  framework codec semantics verified from `SirusDoma/Encore` (string
  framing, `SizePrefixedMessageFramer`, collection/nullable rules).
  C# clean-room code is a sanctioned reference per
  `docs/client-server-plan.md`; never copy game bytes (AGENTS.md).

## Suggested next steps

1. Canvas input injector: `SendInput`-level helper inside Wine, uinput
   device, or evidence for how the canvas polls buttons — one `입장`
   click past the channel list unlocks room → waiting → song-select
   on the real client.
2. M/G/QUEST/TUTORIAL tabs and timer auto-dismiss on the Wine client
   (still OPEN after Attempt 6).
3. Online play UI depth: waiting-room member paint is in; lobby content
   depth (room details, user lists) and hosted-play flow remain.

# Handoff — recomp2jam (2026-11-06, evening)

## Standing goals and their state

1. **Binary connects to Memoryer — COMPLETE.** `recomp2jam connect` drives
   authorize → channels → channel login → character → avatar → rooms → room
   join, all live-proven against local Memoryer 7.2.0. Phase-1 messages done;
   Phase-2 has room list/join/create; waiting-room song sync and online play
   remain.
2. **GUI from start to playable — IN PROGRESS.** Screen flow, splash with real
   `O2mediaLogo` art, channel/room lists, note highway, judgement/score/audio,
   avatar figures, results. Window verified via spectacle screenshots.
   Open: lobby content depth, waiting-room member UI, online play UI.
3. **Rust audit — COMPLETE.** `docs/audit.md` holds per-crate scores (7–9),
   deviation log, and an empty backlog. Gates green.

## Repo state

- HEAD `f316e75`, branch `main`, pushed, tree clean. History was
  force-pushed twice today (own minutes-old commits only, `--force-with-lease`).
- Gates: `cargo fmt --check`, `cargo clippy --workspace --all-targets --
  -D warnings`, `cargo test --workspace` (284 passed).

## Running processes (background tasks)

- `0c9aa4`: Memoryer server, ~3h uptime. Restart:
  `cd ~/memoryer-local && dotnet bin/Memoryer.Encore.dll` (needs
  `PATH=$HOME/.dotnet:$PATH`). Logs to `server-live.log` when redirected.
- Client preview tasks all exited (user closed windows). Relaunch:
  `./target/debug/recomp2jam play --window --interface
  install-root/O2Jam/Image/Interface1.opi` with `DISPLAY=:0`.

## Environment notes (hard-won — read before touching GUI/net)

- This is a **Wayland** session (`WAYLAND_DISPLAY=wayland-1`); winit opens
  Wayland windows. X11 `:0`/`:1` screenshots (mss) show black — use
  **`spectacle -b -n -o file.png`** for real captures.
- No Xvfb/scrot/xwd; `ffmpeg` lacks x11grab. `pip mss` + `python-xlib`
  installed user-local for X inspection.
- dotnet 10 SDK at `~/.dotnet` (user-local install script). Memoryer source
  at `/tmp/mozart` (scratch copy), build at `/tmp/mozart/Build/Release`.
- Server config: `~/memoryer-local/config.ini` (relay pointed at 127.0.0.1,
  Sqlite). Test account `tester` (any password works after `user:authorize`;
  last token `220566171836`, `SessionExpiry=0` so it persists).
- **Never pipe a long-lived client into `head`**: SIGPIPE kills it when head
  exits — redirect to a file instead.
- Memoryer reference layouts: `/tmp/mozart/Source/Memoryer/...`; Encore
  framework codec semantics verified from `SirusDoma/Encore` (string framing,
  `SizePrefixedMessageFramer`, collection/nullable rules). C# clean-room code
  is a sanctioned reference per `docs/client-server-plan.md`; never copy game
  bytes (AGENTS.md).

## Suggested next steps

1. Waiting-room member UI (paint joined snapshot seats/names/ready).
2. Song-select/sync opcodes toward hosted online play.
3. Lobby content depth (room details, user lists).

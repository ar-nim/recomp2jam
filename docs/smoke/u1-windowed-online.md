# U1 smoke note — windowed online mode (2026-11-10)

Rig: `recomp2jam online` (release) under Xvfb `:97` 800x600, null
audio throughout, local Memoryer 7.2.0 `Mode=Full` at
`127.0.0.1:15010`, account `tester` (token from `t_o2jam_login` —
the 2026-11-06 playtest token is dead server-side: `Invalid token`).
Trace `fixtures/input-trace/u1-windowed-online.json` replayed via
`fixtures/input-trace/replay_window.py` (XTEST key/mouse injection
at the real window; run dir `~/scratch/u1-trace/run7`).

## What was clicked/typed, what happened

Login rode CLI args (`--token`, `--gateway-id 0 --channel-id 0`,
`--create-room "U1 Trace"`, `--music-id 757`) — **ours-by-design**:
the exe takes launcher cmdline, so we take CLI args plus real-window
input for everything visible. Server answered 1000/1002/1004
(authorize, channel list ×10, channel login), room created (#0),
song 757 staged (4031: 562 entries).

Window walk (all real X11 input): `s` dismissed Avi → click
(136,88) opened the channel dialog → Enter to Room → Enter to
Waiting → typed `u1 trace ok` (XTEST keysyms, one `text` log line
per char) → Enter sent 3011 (server echoed 3012) → Enter ran the
4000/4002/4030 select → Enter sent START 4010 → 4011 arrived →
Loading → Enter sent 4012 → Playing (two lane taps, 4014 ticks +
4015 echoes on the wire) → chart end → Result (tally 4016/4041/4021,
server answered 4017/4018) → Enter back to Waiting.

Evidence: screen path `avi → planet → channel → room → waiting →
loading → playing → result → waiting`, 31/31 trace events applied
(57 applied-event log lines), 4398 `frame` log lines, 128 PNG
captures (800x600), 36 wire frames, final state hash
`c51b69c92a5047f7`. Captures show genuine scene art: channel
dialog, select-music dialog with staged rows (`Bach Alive`,
`M757 HXXL` picked), result board.

## Deltas found (new rows, never silent fixes)

1. **Channel rows render identical `Ch. 99 / FULL` labels** (all four
   rows) — display mapping of the 2003 list onto dialog rows is
   unverified against the exe dialog. Next step: rig-compare with
   the Wine client's channel dialog on the same server list.
2. **Playing taps missed every note** (result: 0/0/0/34, all-miss) —
   trace tap times are wall-paced guesses, not chart onsets. Flow
   and ticks are proven; a scored-result variant needs taps aligned
   to real onsets (read from the staged chart, recorded in the
   trace). Next step: onset-aligned tap times in a v2 trace.
3. **Run 3 containment breach**: the client opened on the owner's
   real display — the driver inherited `WAYLAND_DISPLAY` and winit
   preferred Wayland over `DISPLAY`. Fixed in-driver (drop Wayland
   vars, `WINIT_UNIX_BACKEND=x11`); the owner clicking the escaped
   window accidentally proved real-window routing (planet → channel
   → room advanced on genuine OS input). Launch hygiene now lives in
   one place — `scripts/rig-run.sh` (refuses `:98`, the caller's own
   `DISPLAY`, and a leaked Wayland env; starts Xvfb when absent;
   PID-file stop that never pattern-kills). Next step: none — contained
   since run 4, verified in run 7.
4. **2026-11-06 token dead** (`AuthController: Invalid token`) —
   server-side expiry, not a client regression. Next step: read the
   current token from `t_o2jam_login` before each rig run (done).

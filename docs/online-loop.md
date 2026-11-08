# Windowed online loop audit (`run_play --window --online`)

Scope: `crates/recomp2jam/src/main.rs` (`run_play`, `setup_online`),
`session.rs` (`OnlineSession`), `connect.rs` (login + song flow),
`o2jam-shell/src/screens/game.rs` (screen flow). Read-only audit;
no code changed (loop completion needs design, not a one-session patch).

## What works today

- Connect: authorize → channel list → optional channel login → room
  list (`connect_and_login_with_transport`); window stages channels
  (`set_channels`) and rooms (`set_rooms`).
- Channel scene: highlight walk, 2012 chat send, 2017 whisper (`/w`,
  `/r`), `/h` local help, 2016/2029 notice drain into history.
- Waiting scene: 3011 chat send + 2017 whisper, same slash grammar.
- Playing scene: change-gated 4014 jam + life ticks from live
  score/life, 1 ms echo window, 4015 drain into `PeerBars` +
  `apply_peer_echo` gauges; keepalive/foreign-id best-effort skip.
- Local walk Avi → Channel → Room → Waiting → Loading → Playing →
  Result; `can_start_game`/`is_room_master`/`all_peers_ready` rules
  exist; result rows/verdict paint locally.
- Headless reference: `connect --music-id` runs the full host flow
  (4000 select, 4002/4003 arena, 4030/4031 list, sync, start, 4012
  loaded, 4014 tick, 4015 echo, 4016 submit, rank, exit).

## Gaps (no transition / unwired packet / unhandled event)

1. `OnlineParams` is list-only: no `join_room`, `character`,
   `avatar_opa`, or song fields, so the window never joins a room,
   never calls `set_joined_room`, and `waiting` stays `None`.
2. Room Confirm sends no `JoinWaiting` (3010); Room → Waiting is a
   local `enter`, with no member/ready state staged.
3. Non-master ready (4008) has no send path; Waiting Confirm for
   `waiting.is_none()` bypasses the master/ready gate (offline walk).
4. No host song-select path in window: no 4000/4002/4030 sends, no
   `set_selected_music`, catalog, arena stage, or chart/OJM fetch
   (`--chart`/`--ojm` are manual offline only).
5. No loading sync: `SyncMemberMusicState`/`SetDownloadProgress`,
   `StartGame`/`ConfirmMusicLoaded` (4012) never sent;
   `loading_status` helpers stay `dead_code`, Loading Confirm is local.
6. No tally path in window: `SubmitScore` (4016), `FinalizeRank`,
   `ExitPlaying` never sent; `battle_teams` never staged from wire.
7. Inbound chat missing: no 3012/3013 drain in Waiting; no
   2013/2014/2019 drain in Channel (only 2016/2029 are drained).
8. Drains are screen-gated and lossy on mismatch: frames arriving on
   the wrong screen are skipped past with only a trace log.

## Minimal fixes — none landed (deliberate)

Closing any gap above needs host/guest state design (join, readiness,
song election, loading barrier, tally) plus wire tests; half-wiring
one packet would fake the loop. Next session, in order:

- P1: wire Room Confirm → `JoinWaiting` → `set_joined_room`; add
  3012/3013 drain in Waiting (proves room membership + chat round-trip).
- P2: wire Waiting Confirm → ready (4008, guest) / gated START
  (master) + member-update drain (proves start barrier).
- P3: wire master song select (4000/4002/4030) → catalog +
  `set_selected_music` → loading sync → `ConfirmMusicLoaded`.
- P4: wire Playing end → 4016/rank/exit → `set_battle_teams` +
  result ranks from wire.
- P5: add Channel 2013/2014/2019 drain; queue out-of-screen frames
  instead of skipping them.

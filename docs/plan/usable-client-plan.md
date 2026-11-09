# Usable-client program — the application a person can run

Goal: from "behavior matches pinned evidence under scripted tests" to
**"a person can run this client and do real things with it"** —
windowed, online, driven by real input. Owner directive (2026-11-10):
interactive verification is the whole point. Scripted API tests stay,
but no flow counts as delivered until a human-shaped input reaches it
through the real window.

Game-play parity work stays deferred (E2 wedge, `docs/overnight-evidence/e2-attempt13-wedge.md`); this program is everything around it.

## Standing rule: interactive-verifiable (all sprints)

A sprint is done only when all three hold:

1. **Window path exists.** The flow is exercisable in the windowed
   binary (`recomp2jam` winit event loop, `crates/o2jam-shell/src/window.rs`),
   reached by real keyboard/mouse events — not only by calling
   `ScreenGame` methods. API-level unit tests remain as the fast net,
   but API evidence alone never completes a sprint.
2. **Trace evidence at the OS boundary.** A deterministic input trace
   (`recomp2jam-input-v1`, `recomp2jam-input-replay` skill) replayed
   through the window-event boundary (key/mouse injection into the
   running windowed binary, per the skill's end-to-end probe rule),
   producing frame captures + an applied-event log + final state hash.
   One trace per landed flow, committed under `fixtures/` (synthetic
   only; no captured proprietary bytes).
3. **Manual smoke note.** The sprint report records one hands-on
   session: what was clicked/typed, what happened, deltas found.
   Deltas become new rows, never silent fixes.

Method notes: the portable trace boundary stays the shell input API;
the OS-injection layer is validated by the end-to-end traces above
(the skill's own split). Menus use a monotonic clock, gameplay uses
song position; record units explicitly in every trace.

All other standing rules from `docs/plan/overnight-behavior-plan.md`
carry over unchanged: evidence-first, clean-room, data-driven (`f38f3`),
controller-serial gates both ways, agents never commit, null audio in
rig sessions, no `ask`.

## U1 — Windowed online mode (the enabler)

The online session (`crates/recomp2jam/src/session.rs`, ~2k lines)
today is reachable only through the headless `connect` walk; the
windowed binary is offline-only (`play`). Bind them:

- Windowed entry that runs the login flow (gateway connect, 1000/1002/
  1004/1007 — already implemented) and then drives `ScreenGame`
  through `OnlineSession`'s existing per-frame sections (chat, stats,
  tally drains) inside the winit loop.
- Login surface: the exe takes launcher cmdline; we take CLI args +
  a minimal in-window entry (evidence for the exe's own login UI is
  launcher-side — mark what is ours-by-design in the smoke note).
- Input routing: window events already route to the shell; verify the
  session's intercepts (chat IME, dialogs) work through that path.
- **Done = standing rule**: connect → channel → room → waiting →
  playing → result trace through the real window, frames captured,
  plus the smoke note. This trace becomes the reusable harness for
  every later sprint.

## U2 — Settings persistence + full options

- `option.ini` load/save per `docs/lifecycle.md` §3 (key map, volumes,
  game options); applied at boot, written on dialog commit.
- Options dialog tabs: Sound (`DIALOG_OPTION_SOUND`) and Game
  (`DIALOG_OPTION_GAME`) wired like the Key tab; gear button opens the
  full tab set. Tab state evidence: first-open tab stays OPEN per S0.3
  until a rig capture names it.

## U3 — Leave/teardown edges

- `3005 ExitWaiting`, `2021 ChannelLogout`, `3000 SetRoomParams` +
  their events (2008/2020/2023/2025, 3010/3014) implemented and
  drained; back-navigation leaves server state consistent.
- Window close → `Terminate` 65520 → clean session end (exe: exit is
  teardown, lifecycle §13).

## U4 — Rankings (Lanking)

- `2027 GetMusicPlayRanking`, `2030 GetMusicScoreList`, `2032
  GetMusicMaxScore` packs + Lanking row painting from parsed data
  (display-only state, no `CStateLanking` — lifecycle §12).
- Sorts/search/pager only where the exe's control codes name them;
  otherwise OPEN with the next step.

## U5 — Shop / MyRoom economy

- Send paths: `5000 SyncItemPurchase` (pieces exist in `shop.rs`), 
  `5006/5007 Enter/ExitShop`, `5010 SyncMusicPurchase`, `5025
  SyncFreePass`, `5028 SyncCashPoint`, `5036 BagExpansion`, `5038
  RefershBag` — each evidence-pinned before coding.
- Data rows: item/song rows, prices, pager cells paint from parsed
  `.bnd` + fetched data (currently stand-ins, parity.md side states).
- Buy/gift/cart/search/sort flows; MyRoom equip uses existing 5004.

## U6 — Song acquisition

- FTP download of song archives + `5012 SyncMusicDownload` notice +
  `4034 AcquireMusicRequest` / `4038 SetDownloadProgress` progress —
  the MusicShop download flow end to end.

## U7 — Live relay multiplayer

- Relay connection in the session (`4042/4044/4046/4048/4051/4053/
  4055` + UDP datagrams, `o2jam-net/src/relay.rs` types exist), the
  S6 dispatcher outcomes sent on the wire, Live-mode play through the
  window. Largest sprint; lands last among the must-haves.

## U8 — Remaining dialogs

- The ~21 unwired `CD*`/`DIALOG_*` classes, by owner-visible value as
  each is reached (gift, information, messenger, ...). Each dialog is
  a mini-sprint under the same interactive rule.

## Sequencing

1. U1 alone (everything depends on it; its trace harness is reused).
2. U2 + U3 in parallel (U2 shell-heavy, U3 net/session-heavy).
3. U4, U5, U6 sequentially (shared shell files).
4. U7 when the queue clears; U8 interleaved by value.
5. Controller commits each landed unit (gates both ways), updates
   `docs/parity.md` + `docs/progress.md`.

## Definition of done

The windowed binary, launched by a person, logs in, navigates the
lobby, joins a room, plays a song, results, shops, and exits — every
step hit by real input, every flow backed by an OS-boundary trace and
a smoke note. Gameplay scene parity (E2 cluster) remains OPEN and
named, not silently dropped.

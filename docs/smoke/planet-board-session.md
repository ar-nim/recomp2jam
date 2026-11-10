# Planet board session — smoke note (2026-11-11)

Rig: `recomp2jam online` (release, worktree `feat/planet-parity`)
under Xvfb `:96` 800x600, null audio throughout (default sink; no
`--sound`), local Memoryer 7.2.0 `Mode=Full` at `127.0.0.1:15010`
(account `tester`, id 1, token from `t_o2jam_login` at run time).
Trace `fixtures/input-trace/planet-board-session.json` replayed via
`fixtures/input-trace/replay_window.py` (XTEST key/mouse injection at
the real window; run dir `~/scratch/planet-board-run`).

## What was clicked, what happened

Boot shows Avi; `s` dismisses it onto the bare board (boot → bare
board holds). HARD-emblem click (386,88) records the family pick and
opens the channel dialog over the board; dialog X (577,171) closes
back to the bare board. Promo LEFT/STOP/RIGHT presses (366,523 /
400,523 / 434,523) record their cells and hold the board — no flow
change, no modal. EXIT disc (769,580) opens the staged quit-confirm
(`게임을 종료하시겠습니까?` + OK/CANCEL over the board); Escape
cancels it. OPTION gear (729,580) opens the staged options dialog;
Escape closes it. Confirm (Enter) opens the channel dialog — the
board always goes somewhere.

Evidence: screen path `avi → planet → channel → planet → channel`
(1060 `frame` log lines), 19/19 trace events applied (17 input
events + 2 screen syncs in the applied-event log), 16 frame PNGs
(800x600), 13 wire frames, final screen `channel`, state hash
`7a8dc68c3397687c`. Captures carry genuine staged art: bare board
with real Hangul emblem captions (`사람손 (초급)` / `고수손 (중급)` /
`신의손 (고급)` / `퀘스트` / `튜토리얼`), promo banner + `< ▶ >` nav
cluster, MENU bar with back-arrow/gear/power discs, staged
quit-confirm sheet. The picked HARD badge carries the amber-cursor
stand-in tick (small amber mark left of the badge) — the exe's
orange select border is absent (OPEN, see below).

## Deltas found (new rows, never silent fixes)

1. **Picked-band highlight renders the amber-cursor stand-in, not
   the exe's orange select border.** Capture-pinned 2026-11-11
   (`docs/planet.md` "Difficulty-pick highlight"): EASY-picked
   border x102–191/y49–116, NORMAL-picked x221–290/y55–116
   (`~/scratch/planet-verify/s1-after-{easy,normal}.png` diff). The
   border hugs badge art of unpinned extents, so no rule is
   implemented (ADR-0003). Next step: pin the border rule against
   the staged badge-art extents via capture alignment, then replace
   the cursor.
2. **Promo carousel semantics unevidenced.** Presses record
   (`planet_promo`) and hit-test at the parsed rects; what the exe
   does (banner step / rotation halt) has no consumer. Next step:
   rig-compare banner art across LEFT/RIGHT presses on the Wine
   client, or a dossier offset for the carousel setters.
3. **No bare-board → Room/solo path evidenced.** Confirm and
   lit-emblem presses open the channel dialog (family select); entry
   into Room rides the dialog's enter faces only. Next step: live
   gateway replay of a solo-mode entry, or the exe entry key for the
   picked band (key bindings stay OPEN).
4. **First window run failed on an empty token** (wrong db column in
   the run script — `guid` does not exist; the token column is
   `TUSER_ID`). Fixed in-driver (read `TUSER_ID`); the server's
   `Invalid token` line in that run is script error, not a client
   regression. Next step: none — documented here.

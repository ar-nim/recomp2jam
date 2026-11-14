# `STATE_PLAYING` Input + Timing — scoping plan

Refs #67. Scoping only: no implementation, no verdict changes. Each cell
below names its verdict target, tier, evidence method, guard plan, and
ready-for-agent criteria so the linked execution issues can be cut and
wired under map #66.

## Starting position (pinned, not re-argued)

Both cells are OPEN, tier R+G (milestone-3 gating). Source rows:
`docs/parity/playing.md` Input row and Timing row.

- **Input** — exe: 7-lane keys, judgement vs `|song−note|` ≤ 6/18/25 ms
  → flags `0x10/0x20/0x40/0x80` (judgement `FUN_00576540`, file
  `0x176540`, ev `14992eb2…`). Shell: lane strikes hit `LivePlay` on
  the sample clock; key-down strikes the head, key-up judges the armed
  tail (`release_lane` → `judge_armed_tail`, the verified `+0x864`
  model — `docs/gameplay.md` item 6, polarity settled statically,
  third trace). B53 machinery landed (#100).
- **Timing** — exe paces on the wall clock (B38, ev `efdaac19…`;
  ADR-0001 records the sample-clock deviation). Shell: backend-sample
  song clock; B52 equivalence replay landed (#99); per-frame bg-effect
  loop file `0x550db0` (ev `4164031c…`) still unproven.

Dependencies are closed: #64 (exe GUI drive), #62 (input-log flush),
#59 (timed holds ride the wall clock). Rig exists (`scripts/rig-run.sh`
+ `scripts/lib/rig-preflight.sh`): sanitized env, `PULSE_SINK=wine_null`
default (sound never reaches the speakers), per-session Xvfb display,
PID/claim tracking, at most two rig sessions at a time on separate
displays and prefixes.

## Settled method question: #64 complements, not replaces, the sweep

#67 asked whether #64's GUI-drive replaces or complements the SendInput
key-up sweep. Answer from the closed #64 scope: **complement**. #64's
legs are pointer-side (mouse tracking, press-down drawing, rest-dim —
the A46 polled-`GetCursorPos` arm and chrome frame selection). The
highway key path is a separate dispatch: buffered DirectInput keyboard
(`GUID_SysKeyboard`, device setup `FUN_00542ab0`), `Option.ini` KEY_1–7
scancode table (file `0x173290`), scancode→lane poster `FUN_00544b50`,
queue drain `FUN_00577ff0`/`FUN_00579b80`, inversion `FUN_005714b0`
(file `0x1714b0`) routing byte 0 = key-down = walk/head strike,
byte 1 = key-up = `+0x864` tail judge (`docs/gameplay.md` item 6).
A mouse-drive recipe cannot exercise that queue, and the queue trace
cannot pin press-down drawing. The two rig sessions below run one arm
each; they may run in parallel on separate displays.

## Cell 1 — `STATE_PLAYING`.Input → PASS

Tier R+G (unchanged). Target PASS: routing is statically EXE-pinned
(item 6), the shell models it (`release_lane`, `judge_armed_tail`), and
the remaining gap is live confirmation plus the latency number.

Evidence method (ADR-0009: input dispatch = static sweep for the arms,
rig capture pair for the armed frame):

1. **Live highway SendInput key-up sweep** (one rig session, null sink,
   per the `~/scratch/planet-verify/findings.md` recipe: popup cleared
   first, siclick SendInput — XTEST clicks never reach the DirectDraw
   canvas). Press/release each lane on the live highway at controlled
   offsets around head and tail times; read judgements + tail ratings
   off the capture pair and the per-line-flushed input log (#62:
   routed keys visible within 1 s). Settles: release rates the armed
   tail in-window vs ≥26 ms Miss live (statically pinned, never yet
   captured); empty-press and Bad-head-release-kills-tail behavior.
2. **Key-up dispatch REA dossier** (no rig): name the remaining arms
   with evidence ids — keyup branch files `0x176c3b`/`0x176c41`,
   bit-25 test file `0x176c42`, tail compare chain `0x176646`–`0x17667a`,
   cursor clear file `0x176709`, inversion file `0x1714b0`, queue drain
   dossier `ev_c01db4a8…`. The DI device GUID path and per-class vtable
   identity stay OPEN per the A46 note unless this dossier closes them.
3. **Latency capture pair** (same session as leg 1, no extra rig):
   SendInput timestamps vs judged-strike samples on the live highway.
   This is the B53 queue-side + exe-side number; it feeds the
   milestone-3 budget decision (#76), not the routing verdict. Routing
   (this cell) and latency (B53) stay separate arms — the probe
   measures latency, not dispatch.

Guard plan: keep the landed seam guards (`release_strike_rates_the_armed_tail_exact_cool`,
`tail_judgement_is_one_shot_per_armed_cursor`,
`release_tiers_the_tail_with_the_shared_windows`,
`early_release_misses_the_tail_and_bonus_keeps_ticking`,
`held_tail_never_expires_and_late_release_misses`,
`keyup_strikes_nearest_record_then_judges_the_armed_tail`,
`budget_holds_the_shell_added_number_not_the_queue_number`); add one
rig-gated guard asserting the captured release→tail-rating mapping on
the shared windows, and record the measured latency number against the
#76 budget (guard or OPEN note, per #76's decision).

Execution issue carries: cell PLAYING.INPUT, verdict target PASS,
tier R+G, the three legs above, guard plan, rig slot claim
(one session, null sink).

## Cell 2 — `STATE_PLAYING`.Timing → PASS

Tier R+G (unchanged). Target PASS resting on the ADR-0001 approved
deviation (sample clock over wall clock, held to wall-equivalence
inside 6/18/25 ms) plus the two legs below. The PASS row must name the
deviation and its guard per ADR-0008.

Evidence method (ADR-0009: timing = static sweep for the arming call,
rig capture for the measured duration):

1. **Wine null-sink divergence run** (one rig session): capture a wall
   trace in the shared text format from `o2jam-shell::song_clock_replay`,
   replay it against the sample position, and record the live-device
   divergence number. Closes the B52 rig arm; the synthetic guard
   (`song_clock_equivalence_replay_stays_inside_the_pinned_windows`,
   44.1/48/96 kHz, negative half-rate control) already holds the
   equivalence shape.
2. **REA trace of the per-frame bg-effect loop** (no rig): dossier the
   loop at file `0x550db0` (ev `4164031c…`) — arming call, per-frame
   cadence, what it paints. If the trace shows the loop drives
   judgement-adjacent state, that state joins this cell's guard plan;
   if it is paint-only, the row says so and the guard is a shell
   frame-cadence test.

Guard plan: keep `song_clock_totals_backend_samples_not_wall_time` and
the B52 equivalence guard; add the live-divergence number guard (or an
OPEN note with the capture file named, if the number exceeds the
windows — that outcome re-opens the ADR-0001 deviation, it does not
silently widen it).

Execution issue carries: cell PLAYING.TIME, verdict target PASS,
tier R+G, the two legs above, guard plan, rig slot claim
(one session, null sink, shared wall-trace text format).

## OPEN register (unpinned parts stay labeled, never guessed)

- Exe-side input latency number — OPEN; next step: Input rig capture
  pair above; feeds #76.
- B52 live-device divergence number — OPEN; next step: null-sink wall
  trace replay above.
- Per-frame bg-effect loop semantics — OPEN; next step: REA dossier of
  file `0x550db0` above.
- DI device GUID path + per-class vtable identity — OPEN (A46 note);
  next step: Input leg-2 dossier.
- Per-lane integer scroll speeds + exe judgement row 480 — OPEN scale
  arms (#106 follow-up); linear map itself is EXE-truth, not re-scoped
  here.
- Per-mode counter quantity + `0x01D0` runtime-id mid bytes — OPEN
  (#104); digit layout box is pinned, not re-scoped here.

## Ready-for-agent criteria (each execution issue)

1. Body names the cell, verdict target, tier, evidence method per
   ADR-0009, guard plan, and rig slot (display + prefix + out-dir,
   null sink, `rig-run.sh` preflight).
2. Static legs cite file offsets + evidence ids; rig legs cite the
   planet-verify recipe and the shared wall-trace text format.
3. Shell entry points named: `LivePlay::strike_lane` /
   `LivePlay::release_lane`, `o2jam-shell::song_clock_replay`,
   `o2jam-shell::input_latency`, `InputSnapshot` (ADR-0013 pulled-input
   path — the sweep measures through the frozen-per-frame snapshot,
   queue preserved for intra-frame taps).
4. install-root present for the session (real-art soak); suite stays
   green with it hidden (no-archive fallback).
5. Scoping only in this ticket: the execution issues may be filed by
   the owner or a follow-up; this plan files none.

## Resolved-when (#67 acceptance)

Each cell has a linked execution issue carrying verdict target, tier,
evidence method, and guard plan. This plan is the scoping deliverable;
cutting the two issues and wiring them under map #66 is the hand-off,
out of scope here.

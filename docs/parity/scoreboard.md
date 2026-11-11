## Scoreboard

Verify pass 2026-11-08 (chat-box verify on top of the
2026-11-07 passes: post no-fill, chat refresh, caption refresh):
2 HOLD (playing + room byte-identical at their last
verified pass; the 2026-11-08 regen carries pre-existing
`playing.png` drift — see the chat-box verify note), 5 REFRESHED
(result rank rows; channel art-less re-shoot; waiting scripted
chat rows + staged captions; waiting chat band re-verified and
corrected; loading live fraction + caption), 1 NEW (musicselect song dialog). Detail in "Reference-photo verify
pass" above.

Ticket #15 re-shoot (2026-11-11, CP949 title decode + picked-row highlight): 4 REFRESHED, 15 HOLD.
`musicselect.png` (+274 px: title cells of Korean catalog rows + chosen-song panel line, real glyphs for tofu),
`waiting.png` and `waiting-guest.png` (+177 px each: MUSICNAME caption title), `loading.png` (+4169 px: loading
caption title/artist) — every diff cluster named in the sections above; all other shots byte-identical (no
pipeline drift). The `STATE_WAITING` music-select row flips to PASS with guards named; the `musicselect.png`
photo rows above carry the corrected residuals.

- Scenes covered: 7 main + musicselect dialog + loading gate + 9 side states.
- Re-audit 2026-11-12 (ticket #49): **57 dimension cells — 34
  PASS, 13 OPEN, 1 FAIL, 9 N/A.** Every cell carries a tier
  (R / G / R+G), an evidence pointer, and a last-verified commit.
  Every OPEN names its next step; every N/A names why no guard
  applies. The consistency check (ledger-check) reports 0
  violations.
- PASS: boot-into-splash, 800×600 default (`SetWindowPos` file
  `0x14b4c`/`0x14baf`), all 13 documented transition edges (plus
  rejections — `flow_allows_every_documented_edge`,
  `flow_rejects_jumps_the_client_never_makes`). No-timing scenes
  read N/A, never PASS.
- FAIL: 1 composition cell — `STATE_PLAYING` composition (gauge
  appended full-canvas over the bound window, combo/word/long-note
  digits on invented anchors; next steps named in-row: `rea`
  trace of the digit-frame setters, scroll-map and curtain pins).
- OPEN-led: 9 cells — Avi input/timing (dismiss key + hold
  duration), Waiting timing (Loading gate legs), Waiting gameplay
  (member→slot convention), Loading timing (gate-leg pacing),
  Playing input/timing/gameplay (key-up routing, bg-effect loop,
  shell-side life/score), Result input (relay ticket/call shapes).
  Each names its rig next step in-row.
- PASS-with-OPEN-arms: 7 cells — Channel input, Room
  composition/input, Waiting composition/input/music-select,
  Loading composition (exe key codes, clone offsets, pager
  pairings, highlight rendering, wire sends, and rect placeholders
  stay OPEN with named rig replays).
- Side states: all 7 PASS-led with named OPEN arms (rows +
  pagers + buys + picks + slots + captions proven; prices, filters,
  bag/gift/cart joins, billing, and entry paths stay OPEN with rig
  next steps). Child tickets #10–#22 are all closed; no child of
  #9 stays open with work unaccounted — leftovers are labeled OPEN
  rows above, never silent gaps.
- End-to-end walk (ticket #23): one 26-step walk from boot to
  Result through every landed State (Avi, Planet + Options,
  Channel, Room, Waiting + all six sides, music-select pick,
  Loading, Playing, Result, plus retry/leave edges), with a screen
  trace, one frame per step, and Result hash
  `40a617e1c9b1e5b5` — see `docs/smoke/e2e-walk.md`. Guards:
  `e2e_walk_boots_from_splash_to_result_through_every_landed_state`,
  `e2e_walk_replays_to_the_same_final_hash`,
  `e2e_walk_side_states_open_and_return_to_waiting`,
  `e2e_walk_options_opens_and_closes_on_the_board`.
  Verdict: PASS (headless window-seam walk; the same step table
  replayed through the real window against a live gateway stays
  OPEN — next step: rig replay via
  `fixtures/input-trace/replay_window.py`).

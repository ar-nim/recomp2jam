# Loading hand session — smoke note (2026-11-10)

Offline shell run (no gateway): the hand drives Confirm through the
public key seam (`ScreenGame::menu_key(MenuKey::Confirm)` — the same
edge the session bridge feeds), against a validated pick staged with
its catalog row (song 757 `First`/`Band`, `set_song_catalog`), the
walk walked to the armed phase, and an audio rig on the null sink
(sound contained — no speakers).

## What was done, what happened

Walk to Loading with nothing staged; Confirm holds Loading (gate
unready, clock unanchored — the held Confirm is a state no-op, the
hash does not move). Stage the armed walk (select validation
`0x5718b0`, staging/decrypt/decode/arm phases, null-sink audio rig);
the gate reads ready and Confirm hands off to Playing with the song
clock anchored at backend samples spent.

Evidence: screen path `loading → loading → playing`, state hashes
`615dccef20c4c8f3` (entry, unchanged by the held Confirm) →
`795d5e9717f82015` (armed) → `8468959a55e204b3` (Playing). A twin
game driven the same way ends on the same hash. Guard:
`loading_hand_session_holds_then_arms_and_hands_off` (plus the
leg guards `loading_confirm_without_a_walk_stays_in_loading`,
`loading_confirm_without_audio_stays_in_loading`,
`latched_loading_failure_holds_loading`,
`armed_loading_handoff_anchors_the_sample_clock`,
`loading_phases_walk_entry_through_arm`).

## Deltas found (new rows, never silent fixes)

1. **Phase pacing is exe-OPEN.** The legs map to the exe dossier
   (`0x4f8b80` staging, `0x412180` decrypt, `0x54d460`/`0x54db40`
   codec, `0x4e47a0` handoff) but no wall-clock duration is pinned
   — the walk advances on staged fractions, never on timers. Next
   step: Wine rig run of the gate walk timing each leg per the
   `~/scratch/planet-verify/` recipe (null sink), then replace the
   fraction steps with measured holds.
2. **Bar and caption rects stay OPEN placeholders.** `MusicLoading.ojs`
   is one 800x600 body with no sub-rect (read-only sprite-55 walk,
   2026-11-08) and no `.bnd` names the gate (bnd-audit 35), so the
   centered track/caption rows are placeholders by negative
   evidence, not literals (ADR-0003). The caption inks through the
   exe-pinned Trebuchet 27x65 cell (`Cell::Loading`); Arial 7x14
   has no cell. Next step: live capture of the gate bar/caption
   plus a memory read of the overlay mount per the same recipe, or
   a dossier offset for the bar fill call.

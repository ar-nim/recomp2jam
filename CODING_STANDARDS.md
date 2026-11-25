# Coding standards (reviewer-side; read during review, not implementation)

Judgement calls only — mechanical patterns belong in checks
(`scripts/gate-check.sh`, `scripts/close-spec.sh`, clippy `-D warnings`),
not here.

## Guard tiers are part of the name (#921)

The ladder in `docs/live-path-reachability.md` is also a naming
convention. A new player-visible guard's name or its `## Guards` close
entry declares its tier as `tier 0` through `tier 4`. Existing guards are
grandfathered; the tier is required for NEW/changed guards
(`scripts/check-guard-evidence.sh --close-gate <file> --diff <base>`) and
for every guard named in a close comment. `docs/interaction-census.md`
consumes these declarations; OPEN debt stays in its OPEN rows (#919/#920).

One real guard per tier, all present in `crates/`:

| Tier | Name | What it drives |
|---|---|---|
| 0 seam | `parse_claim_rows_reads_id_level_anchor_and_cells` (`crates/recomp2jam/src/ledger_check.rs`) | parser math; `ScreenGame` never enters |
| 1 window-input seam | `live_room_messenger_press_mounts_the_one_ok_liveness_sheet_from_boot` (`crates/recomp2jam/src/live_input_legs.rs`) | extracted arm helper with a pointer snapshot, not the real window |
| 2 scripted gateway | `shop_buy_round_trips_through_scripted_gateway` (`crates/recomp2jam/src/session.rs`) | `connect`/`session` round trip; no real server or archives |
| 3 live path | `live_boot_default_stages_shop_catalog` (`crates/recomp2jam/src/live_boot_gate.rs`); also `session_walk::drive_walk` and `live_input_legs` guards | headless boot or walk through `main.rs::run_play` / `session.rs` production paths |
| 4 timing or capture | `solo_gate_pacing_holds_through_the_wall_trace_windows` (`crates/recomp2jam/src/loading.rs`) | wall-clock pacing; a screenshot alone cannot prove the timing |

A Tier 0 guard proves logic, never reachability. A player-visible close
listing only Tier 0 guards fails
`scripts/check-guard-evidence.sh --close-gate <close-comment-file>`; so
does a `seam:` reason that conflicts with a live caller named in the
guard's source. See `docs/agents/triage-labels.md` rule 7.

## Player-visible guards must assert painted output over time

A guard for a paint-path behavior must include a rendered-output
assertion (frame buffer diff, pixel ink in the region) or an
audio-backend assertion (cue/sample index) — not only state/return-value
asserts. A staging `Some(...)`, a queued cue, or a dispatched action
proves the seam, not the screen. Where the live caller chain is the
whole behavior, name it (per `docs/live-path-reachability.md`); where a
visual or audio assertion is possible, require it. Retro #870: the
tutorial shipped "no notes, looping speech" behind twenty green
seam-level stories; the owner found it by playing.

Reviewers: reject a diff whose new behavior guard asserts only internal
state when a painted/audio assertion was expressible.

## Negative-assertion guards need an exe pin

A guard that asserts inert behavior ("lands nowhere", "leaves X open",
"nothing happens", "no send") is a review finding unless its comment carries
an exe evidence pin (REA evidence id, a 7.55 offset, or a doc citation) or an
explicit `next evidence step:` that names #916 and the story ticket.
`scripts/check-guard-evidence.sh` enforces this rule in the gate.

When a guard and the owner's playtest disagree, the guard is not evidence.
Reopen the story, pin the behavior against the exe per #916, then flip the
guard to the pinned behavior. A green guard that certifies the wrong
behavior is a regression, not proof.

Prior art: `shop_unwired_browse_chrome_lands_nowhere` (#178),
`myroom_my_bag_tab_press_leaves_the_gift_panel_open` (#180),
`item_shop_go_myroom_leaves_without_an_exit_send` (#178), and
`settled_solo_result_back_click_posts_4021_and_takes_the_4022_answer`
(#294).

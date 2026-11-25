# Coding standards (reviewer-side; read during review, not implementation)

Judgement calls only — mechanical patterns belong in checks
(`scripts/gate-check.sh`, `scripts/close-spec.sh`, clippy `-D warnings`),
not here.

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

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

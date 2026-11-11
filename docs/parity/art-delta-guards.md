## Art-dependent delta guards

Rule (ticket #63): any guard asserting a visible delta — hover,
highlight, dim — must stage a fixture with two provably distinct
frames modeled on a measured pair, and must assert that the rest ink
differs from the hover/highlight ink at the pinned pixels. Synthetic
single-figure fixtures whose rest and hover frames differ by
construction (a stacked second layer, a frame-swap body) do not
qualify: with real art the rest paint already shows full-colour frame
0, so a hover that repaints frame 0 changes nothing and the guard
cannot fail — the ticket-#26 miss. The reference pair is the
hover-sweep offA/onA measurement (`docs/hover.md`): rest = the
desaturated body frame 0, hover = frame 0 at full colour. The
modeled fixture is `offon_hover_fixture` in
`crates/o2jam-shell/src/screens/paint.rs` (synthetic pixels modeled
on the measurement, no captured bytes in the repo); the guard is
`gear_hover_repaints_over_the_dimmed_rest_frame`, landed `#[ignore]`
while rest-dim is OPEN — ticket #26's fix flips it to running.

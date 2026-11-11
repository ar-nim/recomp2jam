<!-- GENERATED from docs/parity/*.md by `ledger-check gen parity-index` — edit the section files, never this file. -->
# Scene parity checklist — 7.55 client vs `o2jam-shell`

Reference client stays read-only (`AngelO2Jam.exe`, 5,029,888 B,
SHA-256 `83a04541…72c823b`). Every 7.55 claim below traces to
`docs/claims.md` (the oracle); anything else is marked **OPEN**.
Placeholder shell art is a known mismatch, marked **FAIL** — never a pass.

Verdicts per `GLOSSARY.md`: **PASS** = exe evidence + implementation + test.
**FAIL** = known gap. **OPEN** = untraced or unclaimed with a next step.

Harness: `crates/o2jam-shell/tests/scenes.rs` asserts art-backed
composition per scene (a staged figure behind the live UI: figure
ink present, UI ink on top, frame never one flat fill) plus the flat
fallback (non-uniform card-over-background scenes, per-scene
markers) and the full transition matrix plus input/timing edges
through the public API (`ScreenGame` incl. `load_scene`/`load_splash`,
`paint_screen`, `ScreenFlow`, `can_enter`) only. Staged figures stay
synthetic in the repo (real `.ojs`/`.bnd` bytes stay out), but the
local-only real-art soak now runs: `cargo test -p recomp2jam shots`
stages genuine `Interface1`/`Playing1` art from git-ignored
`install-root` (10/10 green, zero skips, 2026-11-07), and the six
PNGs below are genuine-art frames. Composition cells stay FAIL where
invented placement remains (cited per cell), never for lack of art.

## Sections

- [Art-dependent delta guards](parity/art-delta-guards.md)
- [`STATE_AVI` — logo splash (`CStateAvi`)](parity/avi.md)
- [`STATE_CHANNEL` — channel select (no state class)](parity/channel.md)
- [`STATE_ROOM` — multiplayer lobby (`CStateRoom`)](parity/room.md)
- [`STATE_WAITING` — pre-game room (`CStateWaiting`/`CLiveWaiting`)](parity/waiting.md)
- [Loading gate (song select → play)](parity/loading-gate.md)
- [`STATE_PLAYING` — gameplay (`CStateNormalPlaying`/`CStateLivePlaying`)](parity/playing.md)
- [`STATE_RESULT` — results (`CStateResult`)](parity/result.md)
- [Side states (reachable from lobbies)](parity/side-states.md)
- [`STATE_SHOP` — item shop (`CStateShop`, `0xA4`; `Payment` back-end, `CMyBag`/`CCart`)](parity/shop.md)
- [`STATE_MSHOP` — music shop (`CStateMShop`, `0x6B`)](parity/musicshop.md)
- [`STATE_MYROOM` — my room (`0x6A`; `CItemManager`, `CPlayerInfo`)](parity/myroom.md)
- [`STATE_PLANET` — planet board (`CStatePlanet`, `0x11`; BGM `Music/Planet.ojm`)](parity/planet.md)
- [`STATE_LANKING` — ranking board (`0x37`; display-only, no state class — like CHANNEL)](parity/lanking.md)
- [`STATE_PAYMENT` — billing views (`STATE_PAYMENT`/`STATE_PAYMENT2`; `CDPayment` web billing)](parity/payment.md)
- [`STATE_TUTORIAL` — tutorial (`CStateTutorial`, stub `0x00`)](parity/tutorial.md)
- [Reference-photo verify pass (2026-11-07 re-shoot)](parity/verify-photo.md)
- [Waiting / music-select region verify (2026-11-07 caption re-shoot)](parity/verify-waiting-musicselect.md)
- [Wine capture verify pass (2026-11-08 room/waiting/loading)](parity/verify-wine.md)
- [Scoreboard](parity/scoreboard.md)
- [Playtest fix pass (2026-11-10) — hand-playtest tickets #24–#41](parity/playtest-fixes.md)

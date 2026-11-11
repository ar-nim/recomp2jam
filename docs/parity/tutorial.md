## `STATE_TUTORIAL` — tutorial (`CStateTutorial`, stub `0x00`)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | `CStateTutorial` is a stub (`0x00`); the `Playing_TutorialBG.ojs` backdrop stages full-canvas from `Playing1` | (ev: docs/smoke/payment-tutorial-hand-session.md) The backdrop stages and the shots oracle holds `tutorial.png` byte-identical | (R) PASS (v: 9692d41) |
| Input | The entry dispatch | (ev: docs/smoke/payment-tutorial-hand-session.md) The entry path stays OPEN — no dossier offset names the entry dispatch. Guard: `tutorial_entry_path_names_its_next_step`. Next step (`TUTORIAL_ENTRY_NEXT_STEP`): SendInput sweep of the lobby emblem buttons per the `~/scratch/planet-verify/` recipe plus a REA trace of the `CStateTutorial` entry dispatch | (R) OPEN (v: 9692d41) |
| Timing | None documented | n/a | (R) N/A (no timed behavior — no dispatch exists to guard; next step: none) (v: 9692d41) |
| Transitions | Lobby ↔ Tutorial via the side visit | (ev: docs/smoke/payment-tutorial-hand-session.md) Side-visit entry and return hold through the shared hand session. Guard: `payment_tutorial_hand_session_visits_both_and_returns` | (R) PASS (v: 9692d41) |

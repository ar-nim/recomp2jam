# Journey audit — Spec: Result state (#294)

Comparison leg, 2026-11-18. Worktree `cb2/ja-b`, base `2105ac6`, no exe boots.
Exe 7.55 pin: `OTwo.exe` 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.

Comparator classes: `pixel-region`, `wire-sequence`, `input-response`,
`state-path`, `data-effect`, `NA-DESIGN`.

Reference frames in this repo are vintage photos, not provenance packs:
`docs/screenshots/result.png` sha256
`08d85373814eb732376f2a09d2415e9ced9686650a2f865d8ad4389b4c5d0180` and
`docs/screenshots/reference-result-4x3.png` sha256
`b1308a53726f750314e830a8463510e5fe09a2ff78a695925007efaa28bb1bc4`.

## Per-story matrix (8 stories)

| # | Story (short) | Comparator | Our journey (code / guard / capture) | Exe evidence | Verdict | Notes / mapping |
|---|---|---|---|---|---|---|
| 1 | [x] Score tally breakdown per exe | pixel-region + data-effect | Digit cells at exe GDI anchors (`crates/o2jam-shell/src/screens/paint.rs`, `ResultPaint`): grade plate, verdict banner, row digits; guards `result_shot_paints_the_rank_table_rows`, `grade_plate_paints_the_selected_frame_at_its_pinned_anchor`, `live_plate_paints_its_frame_at_the_pinned_anchor` | `docs/cresult-anchors.md`: top panel `FUN_004c38a0` (file `0xc38a0`, ev `615e034c…`), member rows `FUN_004c3b40` (file `0xc3b40`, ev `88c3ca31…`); vintage reference PNG hashes above | MATCHED | Static anchors and shell digit cells agree at the pinned columns. No provenance-backed exe Result frame exists; live capture is on the carry-forward list. |
| 2 | [x] RETRY re-enters Waiting for same song | wire-sequence + input-response | `staged_retry_click_returns_to_waiting` (`game/mod.rs:18128`), `result_keys_mirror_the_staged_buttons`, relay guard `result_retry_and_back_queue_the_pinned_relay_bodies` (session.rs:18184); `flush_result_wire` posts `StartGame` 4010 empty (`0xfaa`) | `docs/wine-boot.md` live-observed empty 4010; `docs/parity/result.md` Input row and #87 replay pin (`04 00 AA 0F`); no exe Result capture needed for the wire shape | MATCHED | Wire body and the shell edge agree with the static/live pin. |
| 3 | [x] BACK leaves for rooms with relay family honored | wire-sequence + input-response | `staged_back_click_returns_to_room`, `result_back_press_sends_the_pinned_exit_playing_body` (session.rs:6115), `ExitPlaying` 4021 (`0xfb5`, `04 00 B5 0F`) through `flush_result_wire` | Same #87/#88 replay and `docs/relay.md` §10 pin; battle entry `GetLiveState` 4042 (`0xfca`, `04 00 CA 0F`) byte-verified at file `0xc365c` | MATCHED | All three relay shapes agree with the pins; confirmed by scripted-gateway guard `result_wire_replay_names_the_retry_back_and_battle_entry_shapes` (session.rs:6049). |
| 4 | [ ] Records applied where the exe writes them | data-effect + pixel-region | Write side landed: `apply_score_completed` / `stage_result_records` (`session.rs:2165`, `:2624`) fills `ResultMemberRecord` per active 4018 row; read side absent — `result_record` (`:1226`) is used only by its test (`:5941`); `result_rank_rows` (`game/mod.rs:3744`) still reads local `Score`/`PeerBars` | `docs/net-protocol.md` §4c: 4018 row reader VA `0x5411a0` writes member record `+0x2142/44/46/48/5C/4C`, `+0x2160`, `+0x2164`, `+0x20/24`, `+0x2166`, `+0x2C/0x2128`, `+0x2168`, `+0x2158`; `docs/cresult-anchors.md` member-row painter reads those words (ev IDs in issue body) | MISMATCHED | Filed **#365** — "Result row digits do not read the pinned 4018 member records" (needs-triage, component:game, P2). #310 closed only the write side; no duplicate issue. |
| 5 | [ ] 4040 place bytes accounted for (used or ruled out) | data-effect | `GameRankUpdate` 4040 decoded/validated and traced, never used as a rank source (`apply_rank_update`, session.rs:2176); rank column uses `ResultPaint` local table | `docs/net-protocol.md` §4c rules out gameplay use: whole-image disp32 scan of slot `+0x217d` finds only the writer (`0x1414ad`), reset (`0x13fccb`) and a bulk copy; the rank column reads the local table at `+0x10d2c` (`FUN_004c49a0`) (ev `4684b913…`, `ecb961cc…`) | MATCHED | Explicitly ruled out in the oracle, so "trace only" is parity, not an invented value. No new issue. |
| 6 | [ ] 4017 submitter-slot notice traced into result UI | input-response + data-effect | `apply_score_submission` (session.rs:2149) decodes the `<8` member slot and writes a debug line only; no UI queue or marker | `docs/net-protocol.md` §4c: handler VA `0x55fba0` (file `0x15fba0`, 58 B) reads u8 slot, guards `<8`, forwards (`0x12`, 4017, slot, 0) into UI queue `0x478d50`; the UI consumer is unpinned | BLOCKED | Need a gateway replay with two members plus a REA read of the UI queue consumer before a UI effect can be compared; Rust's trace-only behavior is honest but unverifiable. No new issue. |
| 7 | [ ] `ticket_add.php?USER_INDEX_ID=<index>` GET pinned | wire-sequence (HTTP) | `session.rs:89-91` documents the GET and leaves it OPEN; guard `result_entry_stages_the_pinned_ticket_wire_call` (`game/mod.rs:18207`) asserts the solo arm queues no gateway frame — no web seam exists | `docs/net-protocol.md` §2 lists the page at `0x2466d0`–`0x24a2d0`; `docs/parity/result.md` Input row pins the GET at file `0x24a338`; #87/#88 close records name the web-seam decision and request-line capture as the next step | MISMATCHED | Filed **#366** — "Result entry never issues the pinned ticket_add.php GET" (needs-triage, component:net, P2). The static call site is pinned; Rust has no web call and no captured request line. |
| 8 | [ ] RETRY blocked while siblings are ready (server rule, not faked) | state-path | Client never mirrors the rule: the retry press only queues 4010; a server refusal stays server-side (`docs/cresult-anchors.md` block string VA `0x64a3a0`) | Same static pin in `docs/cresult-anchors.md` "Update and exits"; no qualified exe capture with a ready sibling | MATCHED | Owner-recorded server-authoritative behavior: not implementing a client fake is parity by decision. Live observation with a ready sibling goes to the live lane; no new issue. |

## Summary matrix — Result

| Verdict | Stories | Rationale |
|---|---|---|
| MATCHED | 1, 2, 3, 5, 8 | Pinned GDI anchors (`docs/cresult-anchors.md`), relay bodies (`docs/parity/result.md`, #87/#88), negative 4040 read sweep in `docs/net-protocol.md` §4c, and the recorded server-side retry rule. |
| MISMATCHED | 4, 7 | Filed **#365** (staged 4018 records never reach the row painter) and **#366** (solo entry never issues the pinned ticket GET). Both are novel gaps with no open covering ticket. |
| BLOCKED | 6 | 4017 UI consumer unpinned; needs a two-member gateway replay plus REA read. |
| NA-DESIGN | — | None. |

Live-lane carry-forward:
1. One qualified Result run with a gateway wire log and controlled score bands
   (stories 1, 4, 5, 6, 7) — all reused packs stop before Result.
2. A ready sibling pressing RETRY for the server-rule observation (story 8).
3. A local HTTP stub / gateway request-line capture with the `USER_INDEX_ID`
   source for #366; no game bytes in the repo.

Issue protocol read-back: #365 and #366 were created one at a time with
`scripts/issue-create.sh` and verified as OPEN with `needs-triage`, a component
and `P2`. No comments, closes, pushes or code changes.

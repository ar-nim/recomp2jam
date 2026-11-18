# Journey audit — Spec: Room state (#290)

Comparison leg only. No exe boot, no code change outside this doc, no push.
Base `2105ac6`; branch `cb2/ja-a`. Comparator keys as in `287-avi.md`. Exe
pin: `OTwo.exe` 5,029,888 B, sha256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.

## Per-story table

| Story | Comparator | Our journey ref | Exe evidence | Verdict | Notes |
|---|---|---|---|---|---|
| R1. List renders live rooms with states (checked) | data-effect / pixel-region | `crates/o2jam-shell/src/screens/game/mod.rs:1365+` room rows → `crates/o2jam-shell/src/screens/paint.rs` `paint_room_scene`; guards `bound_room_panels_paint_numbered_rows`, `room_rows_anchor_to_staged_panels_in_two_columns`, `quiet_server_slot_table_leaves_no_rows_to_click`; `docs/parity/room.md` Composition | t240 room frame sha256 `f91adc41dd70e360e60ecfa6451c322ba85d7aa8a1c1b92c3d460943e4ff542f` (`docs/captures/t240/findings.md`); c1 room frame sha256 `4c7f691a3df184c3d0831b5f8a8e2b9487321041703ccad638b5d85dbb814ce8`; p346 pair below | MATCHED (data/layout) | Row data and two-column layout compare; the outlined-panel pixel delta is R10. |
| R2. CREATE opens with a random Korean default title fully selected (checked) | input-response / data-effect | `crates/o2jam-shell/src/screens/side.rs:3396+` `CreateRoomForm`, eight pinned CP949 defaults behind `rand()%8`, select-all `[0, len]`; guards `create_room_commit_stages_the_typed_title`, `live_staged_create_sheet_commits_a_title`, `create_dialog_paints_its_sheet_after_live_staging`; #271 commit `60eeba2` | Static pin: eight CP949 defaults, `rand()%8`, select-all `[0, len]` in the #271 pin quoted by #290; no CREATE-dialog frame in the reused qualified packs | MATCHED (static pin) | Live CREATE-dialog capture with a selected default is a residual; no named divergence in any pack. |
| R3. Empty room title refused (checked) | input-response | `crates/o2jam-shell/src/screens/side.rs:3437-3440` gates an empty title; `CreateRoomForm::commit` refuses | Static pin: #290 quotes the exe OK handler (VA `0x436ac0`) refusing the empty title only; whitespace-only is explicitly unpinned | MATCHED (static pin) | Whitespace-only refusal stays OPEN with the spec's next evidence step (read the exe trim/compare); no issue — currently unpinned, not a proven divergence. |
| R4. Committed title sent on the create wire (checked) | wire-sequence | `crates/o2jam-net` `CreateRoom` → `crates/recomp2jam/src/session.rs` flush; guard `create_room_commit_stages_the_typed_title`; #269 scripted create asserts the 2004 frame with a rule-7 chain | `docs/net-protocol.md`: `2004 CreateRoom` C→S; #269 scripted gateway round trip; p343 proof (sha256 `20fbe17060cea3d3da16a059b74de06a724b6bb41ee6ae2caf947b3ccfc8cff6`) shows the matched wire window | MATCHED (wire) | Spec's "21-byte UTF-8 cap vs exe CP949 20-byte truncation" remains OPEN as a byte-intactness residual; no exe byte-length pin is quoted in the reused packs, so no issue filed. |
| R5. Password-join completing and staging (checked) | wire-sequence / state-path | `crates/o2jam-shell/src/screens/side.rs:3467+` `PasswordForm` → `take_join` → `JoinRoom` 3002; guards `password_dialog_commits_join_and_enters_waiting`; #269 | `docs/net-protocol.md`: `3002 JoinRoom`; p343 `proof-final.json` (hash above) `wire_sequence_equal: true`, `server_slice_equal: true` with `C2S:3002`/`S2C:3003` on both legs | MATCHED | Scripted leg verified on the wire; owner live leg covers the create half per the spec. |
| R6. Resident-room re-press replayed locally (checked) | state-path | `crates/recomp2jam/src/session.rs` local re-press workaround, #231 `cfe3c813…`; Rust+server evidence only | None — the spec states "no exe behavior pin"; no reused pack exercises the resident-room re-press | BLOCKED | Cannot compare 1:1 without an exe oracle. Live-lane next step: capture the exe's resident-room second press with a wire log before claiming behavior. No issue (no pinned exe behavior to diverge from). #345 (closed) noted as the rig-liveness lineage for this gap. |
| R7. Create/join drain moments guarded at the wire (checked) | NA-DESIGN | #269 guards on `take_create_room` / `take_join` session flush; rule-7 chain in `crates/recomp2jam/src/session.rs` | n/a — guard-only story; the underlying sends are R4/R5 | NA-DESIGN | No separate player-visible journey. |
| R8. Room chat input and history (unchecked) | data-effect / wire-sequence | `crates/o2jam-net/src/rooms/chat.rs` decode → `crates/recomp2jam/src/session.rs:3020-3121` `drain_lobby_chat` and 2016/2019/2029 notices → `crates/o2jam-shell/src/screens/game/chat.rs` history; `docs/parity/waiting.md` assigns chat here; #37 closed on owner live confirmation | Hashed room frame bottom chat box (`f91adc41…542f`) shows the input strip; no qualified capture with scripted 3012/3013 rows in any reused pack | BLOCKED | Spec's verify step (qualified capture with scripted chat rows + boot→Room→chat guard) is unmet. Not a named divergence; no issue filed. |
| R9. ALL/NORMAL/LIVE filter tabs select the list family (unchecked) | input-response / data-effect | `crates/o2jam-shell/src/screens/game/mod.rs:249-251` marks `SHOW_ALLMODE/NORMALMODE/BATTLEMODE` as `MODE_FILTER_STEP` **unwired**; no filter-state guard | Hashed room frame (`f91adc41…542f`) shows the three tabs with ALL selected; `docs/parity/room.md` Composition is a static pin only; no exe capture of a tab click changing the list family | BLOCKED | Candidate divergence (our codes are explicitly unwired), but no dynamic exe pin exists, so do not claim mismatch. Issue candidate if a capture shows tab clicks refetching/switching the list. |
| R10. Selected room panel shows the orange outline and corner tab (unchecked) | pixel-region | `crates/o2jam-shell/src/screens/paint.rs` room row highlight / amber stand-in; `~/scratch/geom-open-close/findings.md` §2 row-template static pin | p346 matched-window pair: exe `room.png` sha256 `d6ef9dd58ba738ea1a8dc10e7426aad83a7ebff854517778df08e5149f62ba89`, Rust `states/room.png` sha256 `2640a7315aa2d60cb44656ee9b57a0996087fba4db137935b83026823e2737dc`; `~/scratch/p346/pack/region-metrics.json` sha256 `f52bf479c32d638d8afa814646ef0d151a7cda0e120d183f73b885b46102e60b` `room_panels_right` 0.934513, `room_panels_left` 0.904316; p346 proof `wire_sequence_equal: true`, `server_slice_equal: true`, `room_list_body_equal: true` | MISMATCHED | Named divergence: exe outlines the right-middle panel with an orange border and corner tab; our frame paints no such outline. Pointer state vs painter fault stays OPEN. Filed #368. |
| R11. Row painted before press for the waiting-join arm (unchecked) | input-response / data-effect | `crates/recomp2jam/src/session.rs` guards `click_selects_room_row_and_enters_waiting` / `click_selects_staged_room_panel`; Rust legs in p343/p346 reach Waiting via Confirm, not a captured row press | Exe: #240's original +36 s cold poll (`docs/captures/t240/findings.md` OPEN arm 1) is superseded for the fast-fail arm: `~/scratch/no-response/b8-relay-fastfail/relay-fastfail-findings.md` pins `03EC@+0.0s`, `07DA@+0.6s`, `07E8@+0.6s`, row paint ≤2 s, then one hover-press → `0BBA` (manifest PASS); c1 `drive.log` `room row painted=True`, hover delta 2.77, one press → `0BBA` | BLOCKED | Exe side now has a wide painted-row window, but no Rust capture proves our row paints before a live row press. Live lane needs that capture; #240/#320 lineage, capture tooling class. |
| R12. Room-state no-response dialog with OK pinned (unchecked) | input-response / pixel-region | Search of `crates/o2jam-shell/src` finds only `PlanetModal::RelayDown` (Channel entry, #244/#348); no Room-state fuse/`Dialog::Information` owner or OK hit test | Qualified exe frame `~/scratch/tt-240-shape/b2-prewarp-dclick/artifacts/screenshots/probe-failure.png` sha256 `0dd6e4b20ff51b987e55d9b74eca9269eac6f6d9c0a3006699353bf7d862afd4`; `docs/dialog-strings.md:22-23` geometry (`Dialog_Information.ojs` (241,206) 318x187; `Info_OK.ojs` (102,150) 117x26; OK (400,365)); `~/scratch/no-response/findings.md` batches 1-3 wire modal payout | MISMATCHED | Named divergence: the exe has a Room-state information sheet with a pinned OK; our client has no such state-owned dialog. Filed #367. Fuse arming condition stays OPEN; #345 (closed) is the rig-liveness lineage, not a fix. |

## Summary matrix — Spec: Room state

| Verdict | Count | Stories |
|---|---:|---|
| MATCHED | 5 | R1-R5 (list, CREATE prefill, empty refusal, 2004 wire, 3002 join) |
| MISMATCHED | 2 | R10 highlighted panel, R12 missing no-response dialog |
| BLOCKED | 4 | R6 (no exe pin), R8 (chat capture), R9 (filter captures), R11 (Rust row-press capture) |
| NA-DESIGN | 1 | R7 (wire-drain guard) |

### Live-lane BLOCKED list (no boots run here)

1. R6 — exe resident-room re-press capture with wire log (no oracle today).
2. R8 — qualified room capture with scripted 3012/3013 chat rows and a boot→Room→chat guard.
3. R9 — one capture per ALL/NORMAL/LIVE state plus a filter-state/row-list guard; if the exes switch families, file the R9 candidate.
4. R11 — Rust row-paint-before-press capture with the same stimulus as c1/b8; capture tooling #245/#249/#250.
5. R10/R12 — fresh matched A/B on current main for the highlight; fuse-arming evidence for the dialog (#367 acceptance).

### Issue actions

- New issue #367 — "Bug: Room-state no-response dialog missing from our client" (`needs-triage`, `component:room`, `P2`; number/title/labels read back). Body file `~/scratch/ja-a-290-no-response.md`.
- New issue #368 — "Bug: Room selected-panel highlight differs in matched A/B" (`needs-triage`, `component:room`, `P2`; number/title/labels read back). Body file `~/scratch/ja-a-290-room-highlight.md`.
- Issue candidate (not filed): R9 room filter tabs — needs a dynamic exe pin before it can be a divergence claim.

### Blocker-class mappings

- R11 maps to capture tooling (#245/#249/#250) and #320/#240 lineage; R6/R12 note closed #345 (server liveness) without treating it as an open blocker.
- No #326/#355/#274/#139/#347 mapping: these Room stories do not reach Playing/Loading or the song-pick readout (#274).

# Journey audit — #315 Spec: Messenger dialog family

Base: `2105ac6`. Comparison leg only; reused packs, no exe boot.

## Evidence anchors (artifact + hash)

| Id | Artifact | Hash / provenance |
|---|---|---|
| `B6` | `~/scratch/side-scenes/b6-shops/`, exe 7.55 (`83a04541…72c823b`), scene sequence `planet→channel→room→myroom→messenger→itemshop`; `artifacts/input.log` clicks `(644,580)` then dismisses the popup at `(400,368)`; `messenger.png` is a one-OK server-unavailable sheet over MyRoom | manifest sha256 `c8adc9aa01e72258b336f6b470a62bb6373d15a4451405019ec943da2253131b`; `messenger.png` sha256 `946e503319b6fa18fcd28b2cf9230b9124b5177cc002567c07afac84631ad917` |
| `B8` | `~/scratch/side-scenes/b8-cash/` repeats the same Messenger popup leg on the way to Cash Charge | manifest sha256 `f9d4a6953c55b75b4f04db3c0ece198bd2032361ac65dc8275ce07ccb6aa3f8b`; `messenger.png` sha256 `946e503319b6fa18fcd28b2cf9230b9124b5177cc002567c07afac84631ad917` |
| `STATIC` | `docs/parity/shop.md`, `docs/myroom.md`, `docs/bnd-consumption.md`, `docs/live-path-reachability.md`, `docs/hover.md` | repo files at `2105ac6` |

Our journey: boot → Planet → Channel → Room (`ROOM_CONTROLS` at
`game/mod.rs:216-250`) → MyRoom (`side/myroom.rs:608`) reaches the Messenger
buttons, but both are labelled P0 no-ops; no Messenger sheet is staged by
`crates/o2jam-shell/src/staging.rs` or `crates/recomp2jam/src/live_art.rs`.
The exe packs reach the host and show a modal error sheet, not the Messenger
main sheet: the rig gateway has no messenger backend. That puts the observed
divergence in the **#345 server-liveness** blocker class (closed ticket; note
only), so it is mapped and not filed.

## Summary matrix

| Verdict | Count | Stories |
|---|---:|---|
| MATCHED | 0 | — |
| MISMATCHED | 4 | 1, 9, 11 (live half), 17 (MyRoom arm); all mapped to #345 |
| BLOCKED | 14 | 2, 3, 4, 5, 6, 7, 8, 10, 12, 13, 14, 16, 18, 20 |
| NA-DESIGN | 2 | 15, 19 |

Story 11's record half is additionally BLOCKED and is noted in its row.

New issues: none. Every observed divergence traces to the #345 server-liveness
blocker class or to missing wire/capture pins, so it is mapped, not filed.

Mapped, not filed: #345 (Messenger backend liveness and its error sheet),
#245/#249/#250 (missing main-sheet captures), plus the wire-opcode inventory
spec for story 10.

BLOCKED for live lane: all five host buttons; friend/note/PopUpMenu flows;
main-sheet rects and frame states; messenger wire trace (packs have
`wire_tap=0`); host BGM A/B.

## Per-story table

| # | Story (short) | Comparator | Our journey ref | Exe evidence | Verdict | Notes |
|---|---|---|---|---|---|---|
| 1 | Host MESSENGER button opens main sheet | input-response | `game/mod.rs:249` Room control unwired; `myroom.rs:608` P0 no-op ("visibly inert") | `B6` input log `(644,580)` → `messenger.png` (hash above) shows a one-OK error sheet over MyRoom | MISMATCHED | Exe answers the press with a modal sheet; ours records a labelled no-op. Named divergence maps to #345 liveness; the sheet is not the main messenger sheet. |
| 2 | Alarm popup reflects new messages | pixel-region | `myroom.rs:157` `MESSENGER_ALARM` P0 paints nothing new | No qualified alarm capture in `B6`/`B8` | BLOCKED | Need a wire-injected new-message capture. |
| 3 | Main sheet 77 rects | pixel-region | No Messenger sheet in `staging.rs`/`live_art.rs` | No main sheet in either pack | BLOCKED | Needs a live mount pin with the sheet; #245 class. |
| 4 | Friend/note scroll clamps | input-response | channel scroll clamp guards exist, not messenger columns | No exe scroll capture | BLOCKED | Companion of story 3. |
| 5 | FRIENDADD search | input-response / wire-sequence | no send path; labeled no-op | `B6`/`B8` have no search exchange | BLOCKED | Search reply shape unpinned; belongs to wire spec. |
| 6 | NOTE compose/search/send/reply/save | input-response | no Messenger dispatch module | No exe note flow | BLOCKED | Needs one qualified note-flow wire capture. |
| 7 | SENDUSER/SENDTIME/CHECKMESSAGE fields | wire-sequence | no parser for these fields | `B6`/`B8` `wire_tap=0` | BLOCKED | No wire pin; do not invent fields. |
| 8 | Five PopUpMenu actions | input-response | no PopUpMenu dispatch | No exe capture | BLOCKED | Needs an exe action pin or a dead verdict. |
| 9 | QUESTION/INFORMATION block host input | input-response | `PlanetModal::RelayDown` is modal only for the relay notice; messenger P0 opens nothing | `B6` input log dismisses the popup at `(400,368)` before continuing; `messenger.png` shows the modal sheet | MISMATCHED | Exe blocks until OK; ours has no sheet to block. Same #345 liveness class. |
| 10 | Wire exchanges opcode by opcode | wire-sequence | no messenger opcodes | Packs explicitly `wire_tap=0` | BLOCKED | Need one wire-tapped messenger run (#245 class). |
| 11 | Replace P0 verdicts only with an exe pin | state-path (record) | `docs/parity/shop.md`/`docs/myroom.md` P0 rows; guards `myroom_p0_controls_answer_nothing_and_queue_no_wire` | `B6`/`B8` pin a live host button response (error sheet) | MISMATCHED | Record half stays BLOCKED for the main-sheet verdict; the live response half is a #345 liveness mismatch. |
| 12 | Six sheets at parsed rects | state-path / data-effect | no sheet staged; `docs/bnd-consumption.md` bucket E | No main sheet captured | BLOCKED | Mount census unfinished; #245 class. |
| 13 | Parent-relative offsets | pixel-region | `dialog_origin` (`paint.rs:2048`) only for staged Information/Options sheets | Error sheet sits at the Information origin, but the Messenger sheets are not observed | BLOCKED | Cannot transfer the Information origin to six unpinned sheets. |
| 14 | Rest/hover/active frames | pixel-region | art delta guards for other sheets | `B6` only rest frame | BLOCKED | Needs hover/press A/B; #245 class. |
| 15 | Dialog-open cue only if pinned | NA-DESIGN (negative invariant) | Messenger button queues no cue | No exe cue pin | NA-DESIGN | No invented audio on either side; no journey to compare. |
| 16 | Family reachable from five hosts | input-response | five host P0 codes (Room/Shop/MyRoom/Waiting/MShop) | `B6`/`B8` reach only the MyRoom-hosted button | BLOCKED | Need one qualified sweep per remaining host. |
| 17 | Live caller chain from boot into each host then sheet | state-path | boot→Planet→Channel→Room→MyRoom chain exists; ends in P0 no-op | `B6` same chain ends in modal sheet | MISMATCHED | MyRoom arm only; remaining hosts have no exe leg. Maps to #345 liveness. |
| 18 | Bucket E/GAP sheets pinned or recorded dead | NA-DESIGN? → BLOCKED | `docs/bnd-consumption.md` bucket E list; no live Messenger mount in code | `B6` adds one live Information mount via the messenger button | BLOCKED | Six Messenger sheets still lack a mount-or-dead verdict. |
| 19 | Negative guard per unpinned action | NA-DESIGN (test) | `myroom_p0_controls_answer_nothing_and_queue_no_wire`, `payment_entry_queues_no_wire_call` | No wire traffic in packs | NA-DESIGN | Test-verified negative invariant; no new wire shape claimed. |
| 20 | Messenger coexists with host BGM | data-effect | MyRoom cue record 5 (`side.rs:175` area) | `B6` audio artifact exists but no before/after messenger A/B | BLOCKED | Need a scoped audio capture; #245 class. |

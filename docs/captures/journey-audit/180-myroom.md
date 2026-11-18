# Journey audit — #180 MyRoom

Comparison leg, no exe boots and no fixes. Base `2105ac64eb3504b4c00494c54ff98e0b1b38f1cb`;
worktree branch `cb2/ja-d`. Stories are the 16 checkbox items in `gh issue view 180`.

## Evidence anchors (artifact + hash)

| ID | Artifact | Hash |
|---|---|---|
| EXE-B8 | `~/scratch/side-scenes/b8-cash/manifest.json` (exe 7.55, 5,029,888 B, wine 11.22, drive reached MyRoom then itemshop/cash-charge) | manifest `f9d4a6953c55b75b4f04db3c0ece198bd2032361ac65dc8275ce07ccb6aa3f8b`; exe `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b` |
| EXE-B8-ROOM | `b8-cash/artifacts/screenshots/myroom.png` (Inventory badge + slot grid upper right, bald full-body figure, wallet `G 15527` / `W 0`, empty present box) | `1743d36d891b4355b3f11a4a99266984159fffd911336ff4de871fd35166fae0` |
| EXE-B6-ROOM | `b6-shops/artifacts/screenshots/myroom.png` (same frame as b8, independent pack) | `1743d36d891b4355b3f11a4a99266984159fffd911336ff4de871fd35166fae0`; manifest `c8adc9aa01e72258b336f6b470a62bb6373d15a4451405019ec943da2253131b` |
| RUST-NOW | `recomp2jam shots` at base commit, `myroom-native.png` (scenario in `docs/captures/t207/myroom.md`: genuine `State_Myroom.bnd` art, staged fixture character/gifts) | `ff5f949f3bcd9d52eab29dfeb87d7f529c49bc52d10a5b2ab54739154e459ddc` |
| STATIC | `docs/myroom.md`, `docs/equip.md`, `docs/parity/myroom.md`, `docs/chrome-wiring.md` | source pins; no pixels claimed |

Comparators: `pixel` = pixel-region, `wire` = wire-sequence,
`input` = input-response, `state` = state-path, `data` = data-effect,
`timing` = timing, `NA` = NA-DESIGN.

## Per-story table

| # | Story (short) | Comparator | Our journey ref | Exe evidence | Verdict | Notes |
|---|---|---|---|---|---|---|
| 1 | Character renders wearing equipped items | pixel | `stage_online_identity` → `set_myroom_character` → `load_worn_figure` → `paint_myroom_data`; guard `myroom_stages_bound_art_when_archives_present` | EXE-B8-ROOM shows a full-body bald mannequin (no kit); RUST-NOW shows a kitted yellow-hair figure | BLOCKED | Data not matched, so not a 1:1 pixel verdict; map #245 + #345. Qualified exe pixels here also serve #300 story 6's evidence need, but matched empty-equipment data is still absent |
| 2 | Character idle-breathes | timing | One phase counter at 66 ms / 48 phases (#176/#222), `kitted_figure_breathes_one_frame_per_part` | STATIC cadence pin, commit `7857d38` | MATCHED | Static cadence pin; smoothness (#223) and live dwell (#229) are separate |
| 3 | Bag pages every owned item | input / data | `myroom_bag_pages` + `myroom_owned_rows` from 2001 `inventory` (live probe in `docs/myroom.md`), guards `myroom_bag_pages_probed_inventory_in_server_order`, `myroom_bag_pager_turns_over_staged_inventory` | EXE-B8-ROOM present box/bag empty; no exe bag-turn capture | BLOCKED | Map #245/#249/#250; source pin exists, input journey lacks exe press proof |
| 4 | Slot selection filters the bag | input / state | Selection records + amber cursor; no filter (`docs/myroom.md` OPEN 1) | No exe per-slot bag comparison capture | BLOCKED | Map #245/#249/#250; 11 buttons vs 16 images vs 17 slots still unjoined |
| 5 | Equip from bag changes figure immediately | input / pixel | `myroom_action_for_code` → 5004 send + restage; guard `myroom_equip_queues_5004_with_derived_slot` | STATIC 5004 slot rule in `docs/equip.md`; no exe equip press capture | BLOCKED | Map #245/#249/#250; seam-green cannot close the visible change |
| 6 | Vacate a slot | input / wire | Local clear + restage, no send; `myroom_vacate_queues_no_wire_send` | Send shape OPEN (`docs/parity/myroom.md` Input) | BLOCKED | Map #345 + #245; no exe gesture/send pin |
| 7 | Info plate shows money/rank/guild/win-rate/skills/exp/notice | pixel / wire | `paint_myroom_data` captions; RUST-NOW opens on STATUS fields. `myroom_info_plate_paints_win_rate_from_staged_battles`, `myroom_info_plate_leaves_unpinned_money_rank_guild_skills_notice_blank` | EXE-B8-ROOM opens on the `Inventory` badge + slot grid at x≈427..476, y≈22..46; lower wallet `G 15527` / `W 0` | MISMATCHED | **Filed #369**: default upper-right face differs. Caption-source half still BLOCKED → #245/#345; unpinned cells stay blank by design |
| 8 | Gift box lists my gifts | data / wire | Entry queues 5017; `myroom_gift_views` + guards `myroom_gift_views_resolve_names_and_senders`, `myroom_entry_queues_gift_list_fetch` | EXE-B8-ROOM present box is empty; no exe gift-list capture | BLOCKED | Map #345; cannot compare list contents without a known-gift account |
| 9 | Claiming a gift lands in the bag | wire / data | `apply_gift_claim_success` + 5023/5024 scripted bridge (`session.rs:4209`) | Send shape pinned, live 5024 verdict/echo unpinned | BLOCKED | Map #345; scripted-only ASSUMED |
| 10 | Amber highlight stand-in replaced | pixel | `paint_cursor` stand-in in `crates/o2jam-shell/src/screens/paint.rs` (~line 1100); no exe frame mapping | Real selection art unpinned (`MYBAGSELECT 0x15500033`, `Myroom_MybagSelect.oji`); no selected-slot exe capture | MISMATCHED | Known divergence, tracked by OPEN #305 — **no new issue** |
| 11 | Controls gated on server features answer or sit inert | input / state | `myroom_p0_verdict`, `myroom_p0_controls_answer_nothing_and_queue_no_wire` | STATIC P0 verdicts in `docs/myroom.md` | MATCHED | Static pin + guard; no invented wire sends |
| 12 | Enter→bag→equip flow guarded at `ScreenGame` seam | NA | `myroom_hand_session_walks_slots_bag_and_gifts`, `docs/smoke/myroom-hand-session.md` | STATIC pins only | NA-DESIGN | Test-authoring story; spec notes the missing rule-7 annotation |
| 13 | Gift claim round trips scripted-gateway tested | wire | `myroom_gift_claim_round_trips_and_lands_in_bag_through_scripted_gateway` (`session.rs:4209`) | Live 5024 shape unpinned | BLOCKED | Map #345; helper exists, pin does not |
| 14 | Default naked avatar, not head-only | pixel | #243 REFUTED on the Rust side; RUST-NOW wears a kit, so no empty-kit comparison frame | EXE-B8-ROOM is a provenance-complete full-body bald mannequin (old #243 scratch evidence now superseded for the exe side) | BLOCKED | Map #245; need a matched empty-equipment Rust capture before any Rust-side verdict. Note for #300 story 6 |
| 15 | Info plate cells from exe-pinned sources | pixel / data | Same caption set as row 7; unmapped cells blank | STATIC rect pins `0x15400010..11`, `0x15400037..44`, `0x1540005C..69` | BLOCKED | Map #245/#345; known-stats capture still needed |
| 16 | Real selection art pinned from archive | pixel | Same stand-in as row 10 | `MYBAGSELECT` / `Myroom_MybagSelect.oji` static pin only | MISMATCHED | Duplicate of row 10, tracked by #305 — no new issue |

## Summary matrix — #180

| Verdict | Count | Stories |
|---|---:|---|
| MATCHED | 2 | 2, 11 |
| MISMATCHED | 3 | 7 (#369), 10 and 16 (#305, already tracked) |
| BLOCKED | 10 | 1, 3–6, 8, 9, 13–15 |
| NA-DESIGN | 1 | 12 |

New issue: **#369** — "Bug: MyRoom opens on STATUS plate where 7.55 shows the Inventory tab"
(labels `needs-triage`, `component:shell`, `P2`; read back after create).

Issue candidates (not filed; already tracked): selection-frame art (#305), naked-avatar fallback
(#243 / #300 story 6), avatar smoothness (#223).

### BLOCKED list for the live lane

- Qualified exe interaction packs for MyRoom slot press, bag turn/equip, gift claim, and a selected-slot frame → #245/#249/#250.
- Live Memoryer pins for 5017/5018 gift list, 5023/5024 claim, 5004 equip reply, vacate, and any wallet/identity source → #345 (closed; continuity caveat).
- Matched character/gift fixture across exe and Rust for pixel comparison (including empty equipment for #243/#300) → #245.

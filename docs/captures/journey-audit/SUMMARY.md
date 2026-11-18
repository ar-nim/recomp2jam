# Journey audit — morning summary

Static rollup of the 17 journey-audit matrices and the four leg findings
(`ja-a` … `ja-d`). No exe boots, no server touches, no code or tracker changes
were made for this summary. Matrices were authored at base `2105ac6` on
`cb2/ja-{a,b,c,d}` and are merged into main at `373b670`. The exe oracle for
every row is O2Jam 7.55 `OTwo.exe`, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
Spec bodies are left for the senior to update this morning.

## 1. Full 17-spec rollup

| Spec | Stories | MATCHED | MISMATCHED | BLOCKED | NA-DESIGN | New issues | Matrix |
|---|---:|---:|---:|---:|---:|---|---|
| #178 Item Shop | 24 | 2 | 2 | 17 | 3 | #363 | [178-item-shop.md](178-item-shop.md) |
| #179 Music Shop | 18 | 1 | 0 | 15 | 2 | — | [179-music-shop.md](179-music-shop.md) |
| #180 MyRoom | 16 | 2 | 3 | 10 | 1 | #369 | [180-myroom.md](180-myroom.md) |
| #287 Avi | 7 | 3 | 1 | 2 | 1 | #356 | [287-avi.md](287-avi.md) |
| #288 Planet | 9 | 6 | 1 | 0 | 2 | #361 | [288-planet.md](288-planet.md) |
| #289 Channel | 11 | 9 | 1 | 1 | 0 | #364 | [289-channel.md](289-channel.md) |
| #290 Room | 12 | 5 | 2 | 4 | 1 | #367, #368 | [290-room.md](290-room.md) |
| #291 Waiting | 16 | 9 | 5 | 1 | 1 | — | [291-waiting.md](291-waiting.md) |
| #292 Loading | 6 | 3 | 2 | 1 | 0 | #360 | [292-loading.md](292-loading.md) |
| #293 Playing | 12 | 5 | 1 | 6 | 0 | — | [293-playing.md](293-playing.md) |
| #294 Result | 8 | 5 | 2 | 1 | 0 | #365, #366 | [294-result.md](294-result.md) |
| #296 Payment | 6 | 2 | 2 | 2 | 0 | #362 | [296-payment.md](296-payment.md) |
| #297 Payment2 | 6 | 2 | 0 | 4 | 0 | — | [297-payment2.md](297-payment2.md) |
| #313 Tutorial | 20 | 4 | 3 | 7 | 6 | #357, #358 | [313-tutorial.md](313-tutorial.md) |
| #314 Options | 22 | 6 | 1 | 10 | 5 | #359 | [314-options.md](314-options.md) |
| #315 Messenger | 20 | 0 | 4 | 14 | 2 | — | [315-messenger.md](315-messenger.md) |
| #316 App-wide dialogs | 21 | 3 | 1 | 13 | 4 | — | [316-app-wide-dialogs.md](316-app-wide-dialogs.md) |
| **Total** | **234** | **67** | **31** | **108** | **28** | **14** | — |

Reading notes:

- Every MISMATCHED row names a concrete divergence against a pinned exe
  artifact or static 7.55 offset. Many are mapped to an already-open ticket
  (#273/#274/#275/#300/#303/#305/#355, ADR-0001, closed #345 lineage) and
  produced no new issue.
- Avi A7 is counted once (MISMATCHED); its matched 5 s hold is noted in the
  row. Tutorial story 3 is split: backdrop MATCHED, live content MISMATCHED.
- `#315` has no MATCHED story: the rig has no messenger backend, so the exe
  leg shows a one-OK server-unavailable sheet where our buttons are P0 no-ops.

## 2. BLOCKED taxonomy (108 stories)

Each blocked story is assigned one **primary** class by the evidence that
gates it. Secondary classes are named in the matrices; closed #345 is liveness
lineage, not an open blocker.

| Blocker class | What gates the story | Count | Share |
|---|---|---:|---:|
| Capture tooling (#245/#249/#250) | Qualified exe interaction pack, paired Rust frame, hover/press or rest/active A/B, manifest + citation | 59 | 55% |
| Server / wire pins | Reply shapes and byte layouts (5001/5028, 5011/5012, 5017/5018, 5023/5024, 5004, 0x44, 4017, ITEMGIFT), HTTP host/session, web request line; closed #345 lineage | 32 | 30% |
| Pending live-lane decision / static read | Named ticket or REA/consumer read that needs no new boot (#301 audio route, #304 probe, `0x550db0` / `0x67fe8c` dossiers, #312 display rule, tutorial cue/Loading coexistence) | 6 | 6% |
| Playing wedge (#326) | No provenance-backed Playing frame or input stream; state object exists but render thread stays on the Loading frame | 7 | 6% |
| Two-client needs | Explicit second client, peer/host, matched identity or matched account state | 4 | 4% |

Full primary-class clusters (story IDs, grouped by spec):

- **Capture tooling — 59.** #178: 3, 7, 9, 11, 16, 18, 22. #179: 2, 3, 4, 5,
  8, 9, 14, 16, 17. #180: 3, 4, 5. #287: A5. #290: R6, R8, R9, R11.
  #292: 6. #297: 4, 6. #313: 2, 9, 10, 11, 17. #314: 3, 4, 6, 8, 10, 15,
  20. #315: 2, 3, 4, 8, 12, 13, 14, 16, 18, 20. #316: 3, 4, 5, 7, 8, 9,
  12, 14, 17, 21.
- **Server / wire pins — 32.** #178: 4, 5, 6, 12, 15, 19, 20, 21, 24.
  #179: 6, 7, 10, 12, 15, 18. #180: 6, 8, 9, 13, 15. #294: 6. #296: 4, 6.
  #297: 1, 5. #315: 5, 6, 7, 10. #316: 6, 10, 11.
- **Pending live-lane decision / static read — 6.** #287: A4. #289: C7.
  #293: 11, 12. #313: 7, 20.
- **Playing wedge — 7.** #293: 1, 8, 9, 10. #314: 9, 17, 21.
- **Two-client needs — 4.** #178: 17. #180: 1, 14. #291: 2.

Secondary tags that matter for triage:

- Capture-tooling rows often also cite #345 (closed liveness) or #326. #293
  stories 8/9/10 are PW-primary with capture-tooling secondaries; #314 stories
  9/17/21 are PW-primary because their gain and longnote snapshots need a live
  Playing entry.
- #178 story 12 (wallet readout) is SW-primary; it also needs a matched-account
  frame. #180 stories 1/14 and #291 story 2 are the 2C-primary rows.
- #315/#316 mismatches are not in this table: they are MISMATCHED rows mapped
  to closed #345, so their next action is an owner liveness decision, not a
  blocked verdict.

## 3. Needs owner eyes

Pulled from the matrices' live-lane lists. No action was taken on any item.

**Two-client tests and matched state**

- #180 stories 1/14 — supply a matched empty-equipment character fixture;
  b8's bald mannequin now pins the exe side of #243 / #300 story 6, but the
  Rust side still needs a same-kit capture.
- #178 story 17 — matched-identity Rust shop capture for the avatar pane
  (exe pane shows the equipped figure; Rust shot is blank).
- #291 story 2 — one hover-verified join, then qualified host and peer Waiting
  frames with MUSELECT closed/open; the exe display rule is still OPEN (#312).
- #290 R11 — Rust row-paint-before-press capture with the c1/b8 stimulus;
  exe has a wide painted-row window, ours has no matching frame.
- #314 story 20 — qualified rest/hover/active gear-sheet frames (b3/b4 are
  rest-only; `b2-dialogs*` attempts have `status FAILED`).
- MISMATCH acceptance, current main: fresh matched A/B for #361 Planet, #364
  Channel, #368 Room. The reused p346 Rust frame predates fix `51c2197`; name
  pointer state on both legs.
- #294 story 8 / #315 story 16 — ready sibling RETRY observation and a
  sweep of the remaining Messenger host buttons (Room/Shop/Waiting/MShop).

**Supervised runs (capture tooling)**

- #178 — qualified item-shop interaction pack: filter press, pager turn,
  preview press, cart toggle, cart total. Existing b6/b8 packs are entry-only.
- #179 — qualified music-shop pack: price/DIFF/money cells, owned flags,
  TOP100 board, sort/pager directions, 5012 control. The b1-explore PNGs have
  no manifest and cannot support a pixel verdict.
- #180 — MyRoom slot press, bag turn/equip, gift claim, selected-slot frame,
  and a known-stats info plate.
- #313/#314 — Planet TUTORIAL plate entry, tutorial exit + BGM re-cue,
  key-effect sweep, Options tab switches, rebinds, SAVE-diff, DEFAULT orders.
- #316/#296/#297 — quit-confirm sheet, idle-Room no-response notice, billing
  HTTP host/session replay, gift-page visit, zero-5000/zero-completion trace.
- #292 story 6 — null-sink wall trace timing the four Loading gate legs on a
  qualified 4011→4012 boot; #287 A4 needs the Avi cue on an owned sink with a
  route snapshot (sound must stay contained, #301).

**Decisions and offline reads**

- #290 R9 — filter tabs are `MODE_FILTER_STEP` unwired; file the candidate
  only if a dynamic exe pin shows ALL/NORMAL/LIVE switching the list family.
- #178 story 23 — gem price cell is blank while exe prints a second price
  `0`; spec-tracked, no issue. Need the entry `+0x18` price pin before paint.
- #293 story 8 — re-pin or retract the rig-runbook's "60 judged" claim;
  stories 11/12 need REA consumer dossiers for `0x550db0` and `0x67fe8c`.
- #292 — exe 4011 cover-source read (#355 acceptance) and Arial 7x14 credit
  consumer read at `0x4e47a0+0x528` (#360 acceptance); c1 proves pixels, not
  value sources.
- #289 C7 / #287 A7 — finish the #304 digit/row probe and the Avi dismiss
  dispatch (full VK sweep or REA trace). Do not claim Escape is the only key.
- #345 liveness — decide whether the #315/#316 server-unavailable divergences
  stay mapped to the closed ticket or get reopened once the messenger backend
  is available.
- #326 — one qualified Playing run with epoch-stamped injector/judged log,
  one stimulus per boot, plus paired 4014/4015 life/jam paint and exe 4016
  bytes through a TCP proxy. Sound must stay on the owned sink.

## 4. New issues #356–369

All 14 were read back on `ar-nim/recomp2jam`: **OPEN**, `ready-for-agent`, a
component label and `P2`. Titles below are the live issue titles.

| Issue | One-line disposition |
|---|---|
| [#356](https://github.com/ar-nim/recomp2jam/issues/356) | Avi A7: our lane key dismisses the splash; exe Enter/Space hold it — full VK sweep and a qualified boot frame are the acceptance steps. |
| [#357](https://github.com/ar-nim/recomp2jam/issues/357) | Tutorial story 8: exe instruction bubble, coach and live notes are absent from our tutorial; chart-timing half stays under #326. |
| [#358](https://github.com/ar-nim/recomp2jam/issues/358) | Tutorial story 19: `docs/parity/tutorial.md` and `side.rs:4053` still call FIRSTSTEP entry unpinned despite the qualified b7 click pin — docs-only fix. |
| [#359](https://github.com/ar-nim/recomp2jam/issues/359) | Options story 2: gear opens Key in Rust but Sound sliders in two qualified exe runs (b3/b4); switch the first-tab default. |
| [#360](https://github.com/ar-nim/recomp2jam/issues/360) | Loading story 2: Artist/Remix/Pattern credits never painted; exe Arial 7x14 consumer at `0x4e47a0+0x528` needs a source read first. |
| [#361](https://github.com/ar-nim/recomp2jam/issues/361) | Planet P3: rest badges paint bright vs exe-dimmed art; `emblem_strip` SSIM 0.729063 in two packs — confirm on current main after #348. |
| [#362](https://github.com/ar-nim/recomp2jam/issues/362) | Payment stories 3/5: our blue notice panel replaces the exe's titled `Cash Charge` web window and rect; also covers #297's visible chrome. |
| [#363](https://github.com/ar-nim/recomp2jam/issues/363) | Item Shop story 2: goods cards paint flat grey; exe prints per-item figures and our `ITEMSHOW` codes are a labeled no-op — stage the art. |
| [#364](https://github.com/ar-nim/recomp2jam/issues/364) | Channel C9: in p346's equal wire/server window exe outlines `Ch.03`, ours `Ch.01` (`row_bars` 0.606146) — fresh A/B must name pointer state. |
| [#365](https://github.com/ar-nim/recomp2jam/issues/365) | Result story 4: #310 landed the 4018 member-record write side; the row painter still reads local scores instead of the pinned record words. |
| [#366](https://github.com/ar-nim/recomp2jam/issues/366) | Result story 7: no `ticket_add.php?USER_INDEX_ID=` GET exists; static call site at file `0x24a338` needs a web seam and request-line capture. |
| [#367](https://github.com/ar-nim/recomp2jam/issues/367) | Room R12: exe has a Room-state information sheet with OK at `(400,365)`; our client has only the Channel-entry relay modal; fuse arming stays OPEN. |
| [#368](https://github.com/ar-nim/recomp2jam/issues/368) | Room R10: p346 matched window has exe orange panel outline/corner tab and no ours (`room_panels_right` 0.934513) — pointer state vs painter still OPEN. |
| [#369](https://github.com/ar-nim/recomp2jam/issues/369) | MyRoom story 7: exe opens on `Inventory` badge + slot grid; ours opens on the STATUS plate — default upper-right face differs. |

## 5. Capture-tooling method notes

- [Residual taxonomy in region metrics](residual-taxonomy.md) — #250 schema fields, evidence rules, triage report, and the linked-Bug filing flow.
- [Paired-video assessment](paired-video-assessment.md) — #249 consumer audit and close recommendation; no video capture was implemented.

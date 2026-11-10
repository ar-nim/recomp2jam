# MyRoom hand session — smoke note (2026-11-10)

Seam: `ScreenGame` directly (no window binding stages side
screens — the windowed binary and the shots walk reach sides only
through `visit_side`; the hand clicks below rode
`click_myroom_side` in test
`myroom_hand_session_walks_slots_bag_and_gifts`, null audio,
synthetic `Itemdata_Korea.dat` rows + synthetic `CharacterInfo`).
Trace `fixtures/input-trace/myroom-hand-session.json` records the
intended clicks in `recomp2jam-input-v1`; the test executes the
same sequence with assertions.

## What was clicked, what happened

Waiting → `visit_side(MyRoom)` opens the room and queues the
gift-list fetch (5017, empty body). Slot-button click (11,11)
records slot ordinal 3; a click at (0,500) lands nowhere and the
selection holds. Bag RIGHT (151,11) turns 12 owned rows to page 1;
a second RIGHT holds the last page; LEFT (101,11) returns to page
0 and holds. Bag-cell click (201,11) picks cell 0 (item 1);
INVENTORY/apply click (451,11) queues `EquipItem` 5004
(slot 0, item 1 — the exe's type-2/record-slot-0 derivation) and
mirrors slot 0 in the staged worn set. Gift-cell click (401,11)
picks box cell 0 (item gift 7 from Giver); MOVE click (501,11)
queues `ClaimGift` 5023 (item gift 7, resource 1,
non-attributive). BACK click (551,11) returns to Waiting.

## Wire queued (take-seam, scripted stub — ASSUMED, never exe proof)

`GetGiftMessageList` 5017 on entry, `EquipItem` 5004
(slot-then-item, exe-pinned shape + derivation), `ClaimGift` 5023
(exe-pinned shape; gift-type follows the picked run,
`is_attributive` false). No live gateway answered them; the
session proves queueing, not the round trip.

## Paint seen

Staged nickname caption inks its own TEXT rect; the beside span
stays black; unstaged overlay paints nothing (guard
`myroom_captions_paint_into_their_bnd_rects`). Bag names paint the
staged `MYBAGITEM` rects. The amber stand-in cursor marks the
selected slot/cells — highlight rendering stays OPEN (see below).

## Deltas found (new rows, never silent fixes)

1. **Apply/MOVE control choice is ours-by-design.** INVENTORY
   commits the equip, MOVE claims the gift; the exe send sites
   (`docs/equip.md`, `docs/gifts.md`) name no control. Next step:
   rig walk of the myroom apply/claim gestures.
2. **Entry gift-list fetch timing is ours-by-design.** The 5017
   shape is exe-pinned; queueing it on entry is not. Next step:
   rig or gateway replay naming the entry fetch.
3. **Slot-selection semantics stay OPEN.** Selection records and
   paints; what the exe filters or targets with it is unpinned.
   Next step: rig comparison of bag contents per slot button.
4. **Caption mappings without a name-identical field stay
   unpainted** (MGAME/O2JAM/O2MONEY/CHARGEMONEY, rank, guild,
   win-rate, skill counts, notice). Next step: rig capture of the
   userinfo plate at known stats.
5. **File → record slot-byte join is ASSUMED.** The pure slot rule
   is exe-pinned static; reading the slot byte from the parsed
   row's third param word assumes the record mirrors the file
   prefix. Next step: REA trace of the `0x52dba0` lookup.
6. **Vacate/GOSHOP/webavatar/sell/bagsort/messenger/conference
   send nothing** (P0). Next step: per-control rig or dossier
   pins before any wire is queued.

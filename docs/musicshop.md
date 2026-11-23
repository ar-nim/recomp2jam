# STATE_MUSICSHOP screen (7.55) — evidence log

Song shop (`CStateMShop`, `0x6B`; `jsBuyMusic` bridge, shared
`Payment` backends, album art). Read-only walks below; offsets are
file offsets unless marked VA. Version pin for all EXE rows:
`AngelO2Jam.exe` 5,029,888 B, SHA-256 `83a04541…72c823b`.

## Block walk (2026-11-09, `ControlList_Interface.txt`)

Slot 1595 (`Interface1.opi`, data offset 74438614, 90472 B, 1979
CP949 lines), lines 810–1007: header `STATE_MUSICSHOP 0X6B 0X03`,
`{`, `BOUND "State_MShop.bnd"`, then entries:

| Codes | Entries | Art |
|---|---|---|
| `0X1A000000` | `IDC_STATE_MSHOP` (self) | — |
| `0X1A100001` | `IDC_BACK_MSHP_1` | `State_MShop.ojs` |
| `0X1A500002` | FRONT_BG | `MShop_Front_BG.ojs` |
| `0X1A200003` | OPENVIEW (preview OPEN) | `MShop_Btn_OpenView.ojs` |
| `0X1A500004..07` | box-line frames | matching `.ojs` |
| `0X1A200008..09` | EXIT / CASHFILLUP | `MShop_Btn_Back/CashFillup.ojs` |
| `0X1A20000A..0D` | MSALL/MYALL pagers | matching `.ojs` |
| `0X1A20000E..0F` | DOWN_START/CANCEL | matching `.ojs` |
| `0X1A201010..15` | NEW/GOLD/BLACK/BLUE/RED/SILVER filters | matching `.ojs` |
| `0X1A200016..18` | TITLE/PLAYTIME/DOWNSTATUS sorts | matching `.ojs` |
| `0X1A400019..021` | money + download texts | — (captions OPEN) |
| `0X1A500022..024` | progress bars | matching `.ojs` |
| `0X1A500025..02C` | page numbers | `MShop_PageNumber.ojs` |
| `0X1A01802D..031` | song-row slots | — |
| `SET 0x15 0X00018000` | 21 shop rows: MusicBox, IMAGE (art-less), New, Color, TITLE/ISBUY/ARTIST/EDITOR/GENRE/BPM/DIFF, WON icons/arrows/prices, GEM_BUY/WON_BUY | `MShop_MS_MusicBox/New.ojs`, `MShop_Btn_MS_Color/Arrow/Won.ojs`, `MShop_Btn_MS_Gem_Buy/Won_Buy.ojs` |
| `0X1A019047..052` | owned-row slots | — |
| `SET 0x05 0X00019000` | 5 owned rows: CHECK/COLOR/TITLE/TIME/DOWNSTATUS | `MShop_Btn_MY_DN_Check.ojs`, `MShop_MMY_Color/DNStatus.ojs` |
| `0X1A600058..059` | cart/download positions | — |
| `0X1A50005A..05C` | cart back + money | `MShop_Cart_Back.ojs` |
| `0X1A20005D..05F` | BUYITEM + cart pagers | matching `.ojs` |
| `0X1A01B060..064` | cart slots | — |
| `SET 0X05 0X0001B000` | 5 cart rows: NUM/TYPE/ITEMNAME/PRICE/DELETE | `Shop_Cart_TypeImage.ojs`, `Shop_Btn_CartDelete.ojs` |
| `0X1A50006A..06D` | cart page numbers | `MShop_PageNumber.ojs` |
| `0X1A50006E..070` | lightboard/tooltip | — |
| `0X1A400071..072` | totals | — |
| `0X1A200073..075` | gift/show filters | `MShop_Btn_Cart_GiftItem.ojs` |
| `0X1A600076..07A` | delete tabs | `MShop_DeleteTab.ojs`, `MShop_Btn_MDelete*.ojs` |
| `0X1A30007B` / `0X1A20007C` | search edit / button | `MShop_Btn_Search.ojs` |
| `0X1A20007D..084` | TOP100 + key/title/diff/bpm/price + SELECTALL | matching `.ojs` |
| `0X1A500085..08A` | notice/messenger/alarm | `Lanking_NoticeBoard.oji`, `Btn_Conference/Messenger.ojs` |

CP949 comments: web-removal markers (`뮤직샾 웹제거`, "music-shop web removal") and `ADD MUSIC DELETE` — shop surface area shrank before 7.55; dead entries stay unimplemented.

## Bind file

`State_MShop.bnd`: slot 1419, offset 63271746, 2214 B body =
`4 + 2 + 138×16` exact → magic `FFFFFFFF` + u16 count + 138
rects of 16 B (`docs/file-formats/opi.md` envelope). Ordinal
pairing (`(code & 0xFFF) − 1`, memory `[0c6c3]`).

## Entry and exit evidence

Inbound `IDC_BUTTON_SROOM_MUSICSHOP 0X13200037`
(`Room_MusicShop.ojs`, STATE_ROOM): a Room button names the song
shop. EXIT/CASHFILLUP/TOP100/conference buttons on the shop side.
No transition claimed from buttons alone; dispatch calls unknown.
The visible BACK plate is bnd code `0x1A200008` at
`(711,569)-(794,593)` (same rect as the Room BACK plate). Its click
now leaves the side screen locally; no 5007 ExitShop body is sent
because no qualified BACK tap has pinned one (#399 wire arm OPEN).

## Screens (implemented)

Shell screen + paint + flow + live staging — DONE
   (`Screen::MusicShop` is the scene-art slot behind the side overlay;
   the flow transition map has no edge into `Screen::MusicShop`, so the
   flow paint arm is unreachable/dead and was removed (ticket #181).
   The canonical, live path is `SideScreen::MusicShop` — `visit_side`
   stages art and `paint_side` paints the board/cart; clicks route
   through `click_music_shop`. `MUSIC_SHOP_BIND_FILE` (side.rs) is the
   sole bind-file constant; the duplicate `MUSICSHOP_BIND_FILE`
   (avatar.rs) was unused and removed (ticket #181).)

## Ticket #21 (2026-11-12)

Five songs per page under the staged `MS_EX_1..5` hit areas
(`0x1A01802D..0x31`, 353x86 tiling the song box on the staged
dump). Row TITLE/ARTIST/EDITOR/GENRE/BPM cells ink the staged
`MMS` TEXT templates (`0x36`/`0x38..0x3B`, row-local, tiled over
the staged slots) from the parsed `OJNList.dat` catalog; the
ISBUY cell (`0x37`) names the fetched 4031 flags (`FREE` when the
server frees every song, `OWNED` when the server lists the song —
format ours-by-design, values DATA). GEM_PRICE/WON_PRICE cells
(`0x40`/`0x45`) and the DIFF cell (`0x3C`) stay blank: no price
or level→cell source is pinned (next step: gateway replay
pinning the price words plus a rig capture of the DIFF cell at a
known song). TITLE/DIFF/BPM sorts (`0x80..0x82`) reuse the
catalog sort keys with ASSUMED ascending directions (same named
next step as lanking). The TOP100 arm (`0x7D..0x84`) stages its
chrome and records its pressed control (#408 below); ranking
contents stay OPEN. PRICE (`0x83`) semantics, 3KEY/7KEY filters,
MYALL owned-list pager, search, gift, and download stay
display-only OPEN (next step: SendInput sweep on the live board
per the `~/scratch/planet-verify/` recipe). Per-row GEM_BUY/
WON_BUY (`0x41`/`0x46`, row-local, tiled per slot) and the cart
BUYITEM (`0x5D`) queue the pinned empty `SyncMusicPurchase`
5010 send (send-switch file `0xa4df0` index 8); the 5011 answer
shape is untraced, so the wire ride stays ASSUMED behind the
take-seam.

## Row-pick chrome (#526, 2026-11-20)

The `MS_EX` row slots (`0x1A01802D..31`) and their text templates are
already staged bound rects; a row pick stages the song in the cart
(commit `9f4721e`). The exe press frame is still OPEN (`E-T`), so the
visible feedback is explicitly ours-by-design: `MusicShopState` records
the last picked row, and `paint_music_shop_row_selection` draws a narrow
stripe inside that row's staged rect before the catalog text inks. A
sort or page turn clears the row; no wire send or ownership claim is
attached. Guard: `music_shop_row_pick_inks_the_staged_row_chrome` (red
before the painter was wired: no pixel delta in the picked row).

## TOP100 board chrome (#408, 2026-11-20)

The `0x1A20007D..0x84` arm stages eight `State_MShop.bnd` rects:
TOP100 `(248,0)-(347,37)`, 3KEY `(18,84)-(75,105)`, 7KEY
`(75,84)-(133,105)`, the four header sorts (TITLE/DIFF/BPM at
`0x80..0x82` keep their wired actions; PRICE `0x83` at
`(310,84)-(372,105)`), and SELECTALL `(436,31)-(488,51)`. A press on
the arm now hit-tests through `music_shop_action_at`; the three
sort codes keep their existing actions, and the other controls
record a client-only `Top100Chrome` press. `paint_music_shop_top100_chrome`
inks a narrow band inside the pressed staged rect. No ranking rows,
sort meanings, or wire source are invented: the qualified exe press
frame `5ddfd9d3…` shows only the pointer and web-banner caption,
so rank contents and a live shop capture remain OPEN (`E-W-5011`).
Guards: `music_shop_top100_arm_stages_eight_rects_and_answer_each_press`
(real `.bnd`, skips without install-root),
`music_shop_top100_button_answers_its_pinned_rect`,
`music_shop_top100_press_reaches_staged_chrome_without_a_wire`,
`music_shop_top100_press_inks_only_the_pressed_staged_rect`.

### TOP100 entry state (#827)

Entry `FUN_004c6d70` looks up `0x1A20007D` and writes 0 into the
control property at `+0x20` (setter `FUN_00409bd0`). `visit_side`
records that entry through `MusicShopState::enter_music_shop`, so a
fresh shop carries no press and no ranking-board ink. The guard
`music_shop_entry_clears_the_top100_press_and_stages_no_ranking_board`
compares the eight staged header rects before and after a leave/re-entry
and asserts no 5010. OPEN: the `+0x20` property name, any nonzero
writer, and a 5011 rank-wire source.

### TOP100 header rank keys (#828)

The six header keys `0x1A20007E..83` (3KEY/7KEY/TITLE/DIFF/BPM/PRICE)
clear the search flag and page and record one rank-table selector
(exe state `+0x538`, routed through `FUN_004d66b0`). TITLE/DIFF/BPM
keep their catalog sorts beside the selector; the vectors at state
`+0x1c0..0x210` are not staged. Guard:
`music_shop_rank_headers_select_their_table_and_clear_the_search`.
OPEN: vector contents, wire source, and a header press with the board
visible (#408 covers chrome only).

### CONFERENCE and MESSENGER notices (#829)

CONFERENCE (`0x1A200087`) posts notice id `0x14` with no decoded copy.
MESSENGER (`0x1A200088`) calls `FUN_004d6890` and posts either the
connection-lost string (VA `0x647dc0`) or the loading string (VA
`0x647e08`). The branch comes from globals at `0x7684d8` and is staged
at the seam; a tapped press frame and messenger-state fixture stay OPEN.
Guard: `music_shop_conference_and_messenger_post_their_pinned_notices`.

### Row buy-rect → shopping bag (#833)

The two tiled row buy rect arrays (`0x41`/`0x46`) now index the catalog
by page `+0x27` × five plus row, add that song with the bag item code
for its won/gem price (`0x43`/`0x47`), and invoke the BUYITEM control on
success, queueing the one 5010. A full ten-entry bag and a duplicate
song post the pinned VA `0x64a5f0` / `0x6486a4` words and buy nothing.
Guard: `music_shop_row_buy_adds_the_catalog_song_and_invokes_buyitem`.
Source conflict, not silently resolved: the ticket names the vtable BUYITEM
code `0x1A20005F`, while the block-walk table groups `0x1A20005D..5F`
as BUYITEM plus the cart pagers and our hit-tested cart BUYITEM remains
`0x5D`. The ticket code is recorded as a vtable-invocation marker only,
never as a hit-tested control. OPEN: one tapped row-buy leg with a wire
tap.

### BUYITEM/GIFTITEM checkout ladder (#834)

Exe pin: `FUN_004c7f70` file offset `0xc7f70` / VA `0x4c7f70`, `ev_26ac6479f93f777bfb153eb934528aa7402ad891416924b47151ebd69b7f7917`
(7.55, 5,029,888 B, sha256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`).
Cases `0x1A20005D` (cart BUYITEM) and `0x1A200073` (GIFTITEM) share a
web-busy gate (`FUN_00556680(0)`), a nonzero bag count (`FUN_0053bbb0`,
`ev_d57f3748610bfe89e92da468450013d52f4c723678b9babb72b74ffeb7d27d37`),
a cash comparison of bag `+0x14` (`FUN_0053c010`,
`ev_03b9f5cc0fa08e3ee260ad78758d1eb4ed3f77b492106f59377c7b6323ed2ab5`)
against globals `0x768B4C/50/54`, and a gem comparison of bag `+0x18`
(`FUN_0053c030`,
`ev_8ab0e3da33d7a249ea4197c6dbd00cd1080fd47f2a80c1b5e8fbe371365af952`)
against `0x768B44`. A shortfall posts `결제할 오투캐쉬(O2CASH)가 부족합니다.`
(VA `0x648B38`, `ev_334e0ed297f83886f1713c114ff70ac7c1066528514feeb2b8adbb8b15f4a957`;
host control `0x1A200009`) or `결제할 잼(GEM)이 부족합니다.`
(VA `0x64A66C`, hosted on the pressed control). Otherwise the arm writes
state `+0x14D` = 1/2 and sends opcode `0x1397` (5015) over vtable `+0x5C`.
Bag totals come from the stage-3 prices added in #833 (`FUN_0053ad10`,
`ev_a4b34e12badfb6148eb37e4989e196e7b9d3373d4c04939b36cf75735383cd46`):
code `0x43` adds the won price to cash, `0x47` the gem price. Discount
fields the exe reads there have no staged source, so no discount is
invented; an unstaged price makes the decision UNPINNED.

The two item-validity checks that precede the comparisons
(VA `0x648cc8` gender mismatch, `0x64a690` already-owned item) are not
modeled: both name item properties, not songs. The gem shortfall also
calls `FUN_0053acd0` to clear the exe bag; our bag stays intact so the
notice and its staged price source can be inspected. Both gaps stay OPEN.
CONTRADICTION preserved: the qualified live shop taps emitted no
5000-family opcode, so the existing 5010 row/cart buy arm is kept and
the 5015 send is staged behind the ladder, with a scripted-gateway wire
leg but no tapped payment frame.

Guards:
`a_music_shop_buy_press_writes_mode_one_and_queues_the_5015`,
`a_music_shop_gift_press_writes_mode_two_and_queues_the_5015`,
`a_music_shop_gem_shortfall_posts_its_notice_and_queues_nothing`,
`a_music_shop_cash_shortfall_posts_its_notice_and_queues_nothing`,
`a_busy_web_gate_makes_the_music_shop_payment_silent`,
`music_shop_cash_needs_all_three_wallet_words_to_be_short`,
`an_unstaged_music_bag_price_keeps_the_checkout_unpinned`,
`an_empty_music_bag_makes_the_checkout_silent`,
`a_music_shop_cart_buy_sends_one_empty_5015_after_the_checkout_ladder`.
OPEN next step: one tapped failed/successful payment leg with the 5015
frame and 5011 answer; the 5011 verdict stays untraced.

## OPEN

TOP100 ownership (ticket #181): the `0x1A20007D..0x84` arm (TOP100 + key/title/diff/bpm/price + SELECTALL) is music-shop chrome only; the item shop (`State_ItemShop.bnd`) has no TOP100 control (owner live check 2026-11-13). Corrected: docs/parity/shop.md, docs/shop.md (no TOP100 entry present), and the #178 story list (story 10 mis-scoped → music shop #179). The arm's staged chrome and press marker landed under #408; ranking contents remain OPEN (no source in the qualified press frame).

1. Rect contents → placements — DONE (138 rects, ordinal pairing).
2. Album-art source + GEM/WON buy semantics — PARTIAL: per-row
   and cart buys queue the pinned 5010 send (ASSUMED wire-side);
   `jsBuyMusic`/Payment web bridge unpinned.
3. Preview, download progress, select, cart, gift, delete flows —
   PARTIAL: `MS_EX` picks stage the cart, cart pagers step and
   hold; preview/download/gift/delete unevidenced.
4. Search/sort/filter/pager behavior — PARTIAL: TITLE/DIFF/BPM
   sorts and the MSALL pager answer (directions + pairing
   ASSUMED); PRICE sort, filters, search unevidenced.
   Caption/money sources: ISBUY from 4031 flags; bottom G/W money
   texts from the shared 5029 wallet and row GEM/WON prices from
   `OJNList.dat` table 3 (commit 9bbca9d).
5. Shell paint/flow/staging — DONE for rows + pagers + buys;
   entry/exit paths still OPEN.

## Privacy static sweep (#402)

The Music Shop is a local side screen: `visit_side(MusicShop)` stages
this seat's art and no control code or request in the 5000 shop family
names another seat's shop. Guard `music_shop_has_no_visitor_request_in_the_shop_family`
sweeps ids 5000..=5100 and pins 5012 as the only known request there.
This is a NA-DESIGN result, not a passed visitor cell: no executable
press or visitor wire frame exists to grade.

## Download controls (#410)

`MSHOP_DOWN_START_CODE` (`0x1A20000E`, bnd rect
`(699,312)-(781,336)`) and `MSHOP_DOWN_CANCEL_CODE` (`0x1A20000F`,
`(721,529)-(775,551)`) stage with the shop but dispatch nothing:
the shop-side 5012 send site is OPEN, so no request is guessed.
Guard `music_shop_download_controls_stay_inert_until_a_5012_send_is_pinned`
pins both rects as inert. The room-song FTP pump's 5012 completion
(`music_download.rs`) is a different state and is not claimed here.

## Search and CASHFILLUP (#413)

The search edit/button (`0x1A30007B`/`0x1A20007C`) and CASHFILLUP
(`0x1A200009`) stage with the entry frame but dispatch nothing:
no qualified press frame exists for either (`E-T`). Guard
`music_shop_search_and_cash_fillup_stay_inert_until_press_frames_exist`
pins that state. #362's charge plate belongs to the Item Shop and
is not a Music Shop pixel.

## Rank buttons (#411)

The right-panel buttons `0x1A200016..18` stage, but the one captured
곡명 press left the rank list and `01/47` readout unchanged; no pager
press frame exists. Guard `music_shop_rank_buttons_stay_inert_until_a_press_pairs_them`
pins that state. Song-board sort directions and rank-page pairings
stay ASSUMED until the interaction sweep (`E-T`) names them.

## Row pick / cart buy (#412)

`music_shop_row_pick_feedback_stages_cart_without_a_buy_wire` pins
the client half of the loop through the routed click seam: an `MS_EX`
row stages the cart and queues no 5010; `BUYITEM` queues exactly one
5010 send. No exe row-pick or BUYITEM press frame exists yet (the
pinned BUYITEM rect showed no visible button), so the exe feedback
arm stays OPEN (`E-T`/`E-W-5010`).

## Entry face and caption anchors (#878, 2026-11-23)

The quiesced v3 pair (`~/scratch/paired-shots-v3/states/musicshop`,
exe frame `4f6d9a2f…`) pins the shop-entry face: NEW filter with the
promo `MShop_Front_BG.ojs`, the Download panel, no catalog clones, no
cart backdrop, no TOP100 badge, and no messenger callout. The rust leg
had all layers painted at once; `music_shop_scene_kept` now gates each
staged figure on the same two face words, and the overlay painter
follows it. A color filter reveals the catalog clones (#813); the
other tab reveals the cart plate (#820). The TOP100 entry property is
zero (#827), so its art stays hidden; no nonzero writer is pinned.

The same frame pins the right panel's twelve titles and the `01/47`
pager. The titles join back to `OJNList.dat` ids
`887,886,885,884,879,836,809,806,880,869,837,832`; the selector word
that produces that order stays OPEN. `MShop_PageNumber.ojs` stores ten
digit frames; `music_shop_page_digit` selects one per control instead
of stacking them (which read `9/9/9/9`). The pager counts twelve
songs per page (562 ÷ 12 = 47) while the left board still tiles five
visible row slots. Idle progress bars use the frame's dark
`rgb(33,49,74)` inside their staged rects.

Captions follow the shared TEXT rule from `docs/gdi-text.md`: ink is
centered or right-aligned in the staged cell as the family requires,
with the +2 px GDI bearing (senior pin, commit `3701ddb`). The MY
title cell is left-anchored, the owner/status cell centered, and the
bottom wallet amounts right-aligned. Only the modern glyph outlines
remain different (owner decision `e8d01`).

Guards: `musicshop_entry_state_keeps_the_promo_and_download_faces_only`,
`musicshop_pager_frames_spell_the_exe_01_of_47`,
`musicshop_entry_panel_reads_the_exe_twelve_song_order`,
`caption_cell_ink_lands_two_pixels_below_the_exe_cell_top`,
`idle_download_bars_ink_the_exe_dark_face`,
`music_shop_bottom_wallet_inks_gem_and_won_from_the_shared_wallet`,
`room_stage_gateway_sends_exe_wire_sequence`.

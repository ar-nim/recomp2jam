# STATE_SHOP screen (7.55) — evidence log

## Ticket #867 — BUYITEM posts the native MESSAGE sheet (2026-11-23)

Version pin: `OTwo.exe` 7.55, 5,029,888 B, SHA-256
`83a04541…72c823b`; all offsets below are file offsets = VA − `0x400000`.

The cart `BUYITEM` (`0x14200070`) is a case of `FUN_00499760`
(`0x99760`): the code-minus-`0x14200003` byte table at `0x49c018`
(index `0x6d` at file `0x9c085`) selects case entry `0x9c6c` from the
dword table (`0x9bff4`). The O2CASH-shortfall arm at `0x9f41`–`0x9f55`
posts queue event **type 2**, host control **`0x14200004`**
(CHARGEMONEY), caption VA `0x648b38` / file `0x248b38`:
`결제할 오투캐쉬(O2CASH)가 부족합니다. 오투캐쉬(O2CASH)를 충전하시겠습니까?`
(REA Ghidra 12.2 / REA 6.9.0: `ev_8e533c73…49c4614` instruction
walk, `ev_87979b92…831b3c7` caption byte scan, `ev_25d57d63…f6a8f`
queue helper). `FUN_00478c90` (`0x78c90`) packs
`(type << 16) | (host & 0xfff)` and enqueues the event; the pump's
type-2 branch (`0x76e6a`, table `0x778b4`) allocates `CDQuestion1`
(`Dialog_Question1.bnd`, the MESSAGE sheet with OK/CANCEL and
TEXT codes `0x22200002`/`0x22200003`/`0x22400004`). The paired live
frame (`~/scratch/boot-verify-3/buyitem`, sheet sha256
`5c662b60…62d3c78`) paints that sheet at +0.5/+2/+5 s, with OK near
`(345,385)` and CANCEL near `(448,385)`. The Cash Charge web view is
the separate CASHFILLUP/charge-plate arm — not BUYITEM.

Implementation: the live/boot BUYITEM ladder stages the base bag
capacity 150 (pinned by the `150개` max-bag notice at file `0x248b88`)
so the shortfall decision is reachable; `ScreenGame` mounts the staged
`Dialog_Question1` art with the pinned caption, routes OK to a local
commit and CANCEL to dismissal, and emits no purchase opcode. Guards:
`live_boot_cart_row_buy_paints_the_native_message_sheet` drives the
windowed chain from `stage_live_scenes` through `paint_side` and clicks
both staged buttons; `live_shop_buyitem_mounts_the_confirmation_sheet`
drives the installed catalog and staged rects; seam guards cover
CASHFILLUP's separate charge view.

**OPEN — purchase WIRE:** the OK/CANCEL choice is local. Qualified
exe taps show no `5000`/`5001`/`5010`/`5011` for this leg; the static
5015 payment-start send stays behind its gates. Next step: one
qualified OK press with the game TCP and HTTP(S) taps plus a scripted
gateway answer. `SHOP_PURCHASE_WIRE_NEXT_STEP` names this in code.

The older local-web-surface rows below describe the #397-era stand-in
and are kept as the recorded contradiction, not silently resolved.

## Ticket #861 — the goods-grid block (2026-11-22)

Wave-3 leg 6 (`w3-itemshop-*-fixed`, entry SSIM 0.1237 at `70f9a33`)
shows the exe's four full-body avatar outfits against Rust's raw
`Itemdata_Korea.dat` face rows. The Room entry handler itself stages
no goods: `FUN_0048fd20` case `0x13200008` (file `0x8fd20`, EV
`ev_ac6983f…b584086`) calls `FUN_00478c50(3)` (EV
`ev_a18e423…51476`) and plays sound `0x138e`. Case 3 of the state
switch (`FUN_00477990`, file `0x77990`) constructs state 3 via
`FUN_00497f60` (EV `ev_ea7f654…84229`), whose category/submenu word
`+0x17/+0x58` starts at `0xc` and gender word `+0x60` at `0`.

The goods grid reads a **staged item-id vector**, not table order:
`FUN_0049ee90` (`0x9ee90`, EV `ev_9d8667…74b4096`) fills the eight
`ITEMSHOW` cells (`0x14600035..3C`) from `state+0x68..0x6c`, inks each
name from `FUN_0052dba0(id)+0x1c`, and resolves the portrait with
`FUN_0052e730(id,1)`. `FUN_0049ead0` (`0x9ead0`, EV
`ev_a0af2c0…830ff`) builds that vector for the entry tab `0xc` from
`FUN_0052dc90` (`0x52dc90`, EV `ev_5946e0…d5385e81`): submenu buckets
11 down to 1, each vector back to front, keeping records whose worn
slot `(record+6 >> 7) & 0xf` equals the gender word or is `2`.
The parsed row exposes those words as `item_type` bytes 0/1 and
`(item_type >> 23) & 0xf`; a zero table price takes `params[1]`
(the four pinned outfits 1537/1539/1540/1541 ink `4500` won / `0` gem).

Implementation: `exe_new_arrivals_goods` (`o2jam-shell`) stages that
order; `stage_online_identity` calls `ScreenGame::stage_exe_item_shop_catalog`
so both Room ITEMSHOP and MyRoom GOSHOP mount the same catalog. Seam
setters keep raw file order. Guards: the live-boot catalog guard names
the first page (블루/투스/해피쥴리/콘트루리 at 4500), and the BUYITEM
guard drives the staged row and cart buttons to the sheet flag.

OPEN: the paired tap's missing C2S 6001 is the generic 15 s watchdog
(`FUN_00558b10`, EV `ev_5a705b…c3dfe`), not a shop-only frame; next
step is a timed session keepalive with a wire diff. BUYITEM posts the
native MESSAGE sheet (see #867 above); the old local web overlay claim
was the #397-era stand-in. Purchase WIRE stays OPEN.


Item shop (`CStateShop`, `0xA4`; shared `Payment`/`CMyBag`/`CCart`
per lifecycle). Read-only walks below; offsets are file offsets
unless marked VA. Version pin for all EXE rows: `AngelO2Jam.exe`
5,029,888 B, SHA-256 `83a04541…72c823b`.

## Block walk (2026-11-09, `ControlList_Interface.txt`)

Slot 1595 (`Interface1.opi`, data offset 74438614, 90472 B, 1979
CP949 lines), lines 179–431: header `STATE_SHOP 0XA4 0X03`, `{`,
`BOUND "State_ItemShop.bnd"`, then 187 active `IDC_` entries
(codes `0x01..0xBA`, no gaps):

| Codes | Entries | Art |
|---|---|---|
| `0X14000000` | `IDC_STATE_SHOP` (self) | — |
| `0X14100001` | `IDC_BACK_SSHOP_1` | `State_ItemShop.ojs` |
| `0X14200002..05` | BACK/GOMYROOM/CHARGEMONEY/ITEMVACATE | `Shop_Back/GoMyroom/Btn_MCashCharge/Btn_Vacate.ojs` |
| `0X14202006..07` | GIRL/BOY | `Shop_Btn_Girl/Boy.ojs` |
| `0X14200008..09` | PLANET_UP/DOWN | matching `.ojs` |
| `0X1420100A..0E` | SET/FASHION/ACCESSORY/BEAUTY/INSTRUMENT | `Shop_Shop*.ojs` |
| `0X14208012..17` | SUBMENU1..6 | `Shop_SubMenu*.ojs` |
| `0X14200018..27` | ITEMBUY/ITEMGIFT ×8 | `Shop_Btn_ItemCheck.ojs` + art-less |
| `0X14400028..33` | money/nickname/item-name texts | — (captions OPEN) |
| `0X1450002B` / `0X14700034` | PLANET / AVATAR figure | `Shop_Planets.ojs` + staged figure |
| `0X14600035..46` | ITEMSHOW/MYBAGITEM icons | — (icons OPEN) |
| `0X14022047` + `SET 0X04` | item scroll + bar/range | `Shop_ItemScroll_*.ojs` |
| `0X1420004C..51` | bag pager + page numbers | `Shop_Btn_Bag*.ojs`, `MShop_PageNumber.ojs` |
| `0X14400054..75` + cart `SET`s | prices, cart rows/cells, set-item detail + BUY/WEAR | `Shop_Cart_TypeImage`, `Btn_CartDelete/SetItem_Buy/Wear.ojs` |
| `0X14200099..A6` | ext-menu + 12 filters | `Shop_ExtMenu*.ojs` |
| `0X144010A7..B0` + misc | skill counts, tooltip/notice/messenger | `Shop_Tooltip`, `Lanking_NoticeBoard.oji` |

Commented-out (inactive): `IDC_SCROLL_SSHOP_MYBAG` + MYBAG
scroll `SET` — dead entries stay unimplemented. Shop surface also
carries web-era comments; dead entries stay out.

## Bind file

`State_ItemShop.bnd`: slot 1463, offset 65475546, 2982 B body =
`4 + 2 + 186×16` exact → magic `FFFFFFFF` + u16 count `0x00BA`
(186) + 186 rects of 16 B (`docs/file-formats/opi.md` envelope).
Active non-self codes cover `0x01..0xBA` with no gaps/dups, so
186 rects pair 1:1 excluding self (memory `[0c6c3]`); rect 0 is
`(0,0)-(800,600)` full-bleed BACK.

## Entry and exit evidence

`IDC_BUTTON_SSHOP_BACK` + `IDC_BUTTON_SSHOP_GOMYROOM` name
back/myroom targets; no `EXIT` id exists in-block.

Two Room/MyRoom controls pick the same runtime state 3 that mounts this
block: Room `0x13200008` (`FUN_0048fd20`, file `0x8fd20`, EV
`ev_8ba1098e…`) and MyRoom `0x15200003` (`FUN_004a3ee0`, file
`0xa3ee0`, EV `ev_138a6bae…`). The Room case sends 5006 right after
the pick; the MyRoom case sends no frame. The live entry guard
`live_itemshop_entry_mounts_the_exe_shop_state_from_room_and_myroom`
drives both aims and checks the staged `STATE_SHOP` board pixels. The tapped BACK
frame (`~/scratch/evidence-shops/gifted-boot/exe/shop-back.png`,
`ff522720…ef29b270`, wire `94b2ea8f…a2508`) leaves the side and emits
`C2S 5007` with an empty body, followed by `5028 → 5029`. The full-
bleed rect 0 pairs with code ordinal 1 (`0x1420_0001`); our client
queues the same exit on that press. Room side
`IDC_BUTTON_SROOM_ITEMSHOP 0X13200008` (`Room_ItemShop.ojs`)
names the inbound button. Names only — no transition claimed.

## Screens (implemented)

Shell screen + paint + flow + live staging — DONE
   (`Screen::Shop`, `paint_shop_scene`, all-reject flow,
   `stage_interface_scenes` + soak tests; side-screen clicks route
   through `click_shop` since ticket #21).

## Ticket #21 (2026-11-12)

Eight goods per page under the staged `ITEMBUY1..8` hit areas
(`0x14200018..0x1F`, two rows of four on the staged dump). Row
names ink the staged `ITEMNAME1..8` TEXT templates
(`0x1440002C..0x33`) from the parsed `Itemdata_Korea.dat` table
and the parsed price inks the staged `ITEMWON1..8` TEXT templates
(`0x14400054..0x5B`); the `ITEMGEM1..8` templates stay blank
(the parsed row carries one price word and the entry `+0x18`
gem-price join is unpinned — next step: file→entry price mapping
plus a rig capture at known prices). The item-list pager is the
staged scroll (`IDC_SCROLL_SSHOP_ITEM`, `0x14022047` at
(221,374)): its LEFT/RIGHT children (`0x48`/`0x49`) stage
scroll-relative, so hit-testing offsets by the staged scroll
origin — geometry from parsed rects, never literals. The cart
`BUYITEM` (`0x70`) posts the native MESSAGE sheet (#867); the old
`구입 하기` web-purchase stand-in belonged to the #397-era model.
W-a found no `C2S 5000` on the exe socket and the only `5001` was
BACK-timed (#397). The static send file `0xf3fa8` is corroboration
for a synthetic probe, not the exe buy path (`docs/shop-buy.md`).
Sort/filter/set-item/bag/
gift semantics stay display-only OPEN (next step: SendInput sweep
on the live shop per the `~/scratch/planet-verify/` recipe) —
see ticket #195 for what that sweep's evidence split resolved.

## Ticket #195 (2026-11-13)

Browse tools against the #191 staged catalog. Wired: the owned
bag pager (`BAGLEFT`/`BAGRIGHT`, `0x4C`/`0x4D`) turns ten-cell
`MYBAGITEM` pages over staged inventory ids in inventory order
(zero/unknown skip, holds at both ends, queues nothing); the
row preview (`ITEMPRE1..8`, `0x64..0x6B`) picks the visible row
into an id-stable slot and `ITEMBACK` (`0x6C`) clears it;
per-row `ITEMWON` cells ink the parsed catalog price word
(`shop_row_prices_come_from_the_staged_catalog` — no invented
data). ASSUMED pairings with rig next steps: the preview name
inks `ITEMCOMMENT` (`0x85`; `ITEMCOMMENT2` mirrors it) and the
bag readout inks current/total into the L (`0x4E`/`0x4F`)/R
(`0x50`/`0x51`) digit unions. The FASHION category tab (`0x0B`) now stages a local selection: the
live press frame `d4074ce7…` moves the tab highlight and emits no
wire, so `ShopState::category` records the press but claims no
filter rule (guard
`shop_filter_scroll_and_cart_total_follow_the_sweep_frames`). Labeled no-ops (pinned by
`shop_unwired_browse_chrome_lands_nowhere`): the gender, other
category, submenu, and ext-menu filter buttons never stage
in the live dump (203 placements, verified 2026-11-13 — the
`ControlList` names them but the staged set omits them), so
those clicks land nowhere by absence; `BAGSORT`, the planet
pager, `ITEMGIFT1..8`/`GIFTITEM`, cart `CHECKGEM`/`CHECKMONEY`
totals, and the `ITEMGEM1..8` cells (gem-price join still
unpinned) stage but record nothing. Ticket #515 guardable remainder:
`shop_cart_total_uses_staged_prices_after_the_pinned_5001_wallet_fold`
pins that the live 5001 wallet words (gem `626`) never become a cart
total — `CHECKMONEY` still sums staged `ITEMWON` rows and `CHECKGEM`
stays `--`. The exe's own total ink stays OPEN (`E-T`): next step is a
qualified cart-pick frame with a lit `CHECKMONEY` cell plus REA of the
cart-to-total writer. No search chrome exists in
`State_ItemShop.bnd` (#178 story 8 has no control to wire).
TOP100 is music-shop-only per the #181 verdict and owns no
item-shop arm (`MSHOP_TOP100_CODE` never dispatches here).

## Ticket #196 (2026-11-14)

ControlList walk (`ControlList_Interface.txt` `STATE_SHOP`, same
pinned exe bytes as above) names the full button inventory the
ticket wires: `ITEMGIFT1..8` (`0x14200020..0x27`, art-less),
`ITEMPRE1..8` (`0x14200064..0x6B`, `Shop_ItemPre.ojs`), `ITEMBACK`
(`0x1420006C`, `Shop_ItemBack.ojs`), cart `GIFTITEM` (`0x14200071`,
`Shop_Btn_GiftItem.ojs`), `MGASH` (`0x14400029`), `CART_CHECKGEM` /
`CART_CHECKMONEY` (`0x14400074`/`0x75`), and the art-less
`IDC_IMAGE_SSHOP_AVATAR` pane (`0x14700034`, staged figure like the
room slot).

- Avatar pane: the equipped worn figure stages through the #193
  pipeline (`load_worn_figure`, no fork) on every source/character
  stage, equip commit, shop entry, and preview change, and paints at
  the staged pane rect on the paint clock (`paint_shop_avatar`).
  Missing source/character/data keeps the old pane (never blanked);
  without a pane the art stays alone (never invented).
- Try-on: row `ITEMPRE` previews the visible good (the exe preview
  semantics stay OPEN — ours-by-design); `ITEMBACK` and leaving the
  shop revert to the equipped avatar. Preview never touches
  equipment or the wire (server-authoritative, no local minting).
- Gift: `ITEMGIFT`/`GIFTITEM` record `ShopGiftNoop` (id + next step)
  and queue nothing. Gift send has no TCP shape (web-only
  `gift_input.aspx`, `docs/gifts.md`); `Dialog_GiftView.bnd`
  stages nowhere (census bucket E, `docs/bnd-consumption.md`).
- Cart add (#685): the live boot parks the seven parent-relative
  `Dialog_Shopping_Add.bnd` rects in a hidden slot; no painter mounts
  them. The exe pin is the D4 row-pick face switch (MY BAG → SHOPPING
  BAG), so `ITEMBUY` checks the row and switches the face locally; the
  W-a NULL pin shows `BUYITEM` emits no game opcode; the exe posts a
  native MESSAGE sheet (#867), not an add opcode.
  A per-item add opcode would be invented (`SHOPPINGBAG_ADD_NEXT_STEP`).
  Guards: `cart_add_opens_the_cart_face_without_mounting_the_parked_sheet`,
  `shopping_bag_add_sheet_stages_seven_parent_relative_rects`.
- Gem price (#395, partial): `ShopGood::gem_price` is an explicit
  `Option<u32>` and the staged `ITEMGEM1..8` cells ink it when a word
  is pinned. The parsed catalog still stages `None`: the gifted boot
  reads won `4500` / gem `0`, so no table-word → entry `+0x18` map is
  evidenced. No gem price is invented; the story stays PARTIAL.
- Balances: `MGASH` paints the `o2_cash` word (`--` until the 5001
  prefix folds; the cell→word join stays OPEN — `cash_point` /
  `item_cash` are the named alternatives). `CHECKMONEY` sums the
  checked cart's `ITEMWON` prices (ours-by-design); `CHECKGEM`
  stays `--` (no gem-price source, same OPEN as `ITEMGEM`).
- Bag-vs-giftbox landing of a bought item stays UNCONFIRMED
  (probed nowhere — the 5001 entry list is GLOBAL-sized, so the
  client cannot decode the landing; never invented).

## Ticket #363 (goods-card portraits)

Exe b8 (`~/scratch/side-scenes/b8-cash/manifest.json`, manifest
`f9d4a695…3f8b`, exe `83a04541…c823b`,
`artifacts/screenshots/itemshop.png` `0c561c66…93ee8`) paints a figure
on each of the four visible goods cards. Our base shot
(`a61c1236…bd15`) left the same cells flat grey.

Source pin: `Itemdata_Korea.dat` rows carry two shop-portrait sprite
slots in front of the 40 worn-part slots (`docs/file-formats/opi.md`,
`docs/equip.md`). `State_ItemShop.bnd` (2982 B, 186 rects, ordinal
`(code & 0xFFF)-1` pairing) gives `ITEMSHOW1..8` (`0x14600035..3C`)
109x81 rects and `MYBAGITEM1..10` (`0x1460003D..46`) 73x57 rects.
A local survey of the installed `avatar.opa` (1541 rows, 7133 entries)
finds slot 0 dominated by 109x81 / 73x57 bodies and slot 1 by 73x57
/ 30x30 bodies, so slot 0 is the large goods-card portrait and slot 1
the small bag icon. No separate icon archive exists (zero `Shop*` /
`Icon*` entry names). Table filename `.oji` resolves to an `.ojs`
sprite-55 body in the same archive; no copyrighted bytes are committed.

Landed: `set_avatar_source` decodes slot 0 once per row into
`shop_item_portraits` (item-id keyed); `paint_side(Shop)` paints them
at the staged `ITEMSHOW` rects before the text overlay
(`paint_shop_item_icons`). Missing slot, missing archive entry, or
unstaged rect paints nothing. Guards:
`shop_item_cards_paint_slot_zero_portraits_at_staged_itemshow_rects`
(paint, synthetic per-row ink) and the extended live-path guard
`live_boot_default_stages_shop_catalog` (install-root skip).

Residual: the qualified exe pack uses a catalog/account state we cannot
match locally (prices/names differ), so no matched-state pixel hash is
claimed. Next evidence step: a rig capture of the same catalog ids with
matched `avatar.opa` before any 1:1 hash comparison. `MYBAGITEM` icons
(slot 1) stay OPEN; `ITEMGEM` cells stay OPEN (#178 story 23).

## Offline fallback marker (#388, 2026-11-20)

With no installed art, `paint_screen(Screen::Shop)` paints the flat
`SHOP` card plus the `OFFLINE STAND-IN` marker. The marker is a
presentation label for the offline/window seam (NA-DESIGN: no gateway
session, no passed 7.55 behavior), not an online-shop claim; the
staged 7.55 board path (`paint_shop_scene`) never carries it. Guard:
`shop_no_archive_fallback_carries_the_offline_stand_in_marker`
(marker ink present on the fallback card, absent on staged art).

## Ticket #722 (A/B audit child origin, 2026-11-20)

Authority: `docs/captures/shop-ab-audit.md` delta table D1-D11 and its
per-cell table. Entry frame `05b24e64…cb00`; row-pick frame
`c84b952f…9922e`; shelf-hover frame `hover-shelf-figure1` (same
qualified pack). Version pin for every exe row stays the 7.55 exe
SHA-256 `83a04541…72c823b`; no new exe bytes were analyzed here.

The +70 px child origin has one declaration,
`SHOP_CHILD_ORIGIN_Y` in `crates/o2jam-shell/src/screens/side.rs`.
Two consumers read it, because the exe's rects mean two different
things after staging:

- art-less `ITEMWON`/`ITEMGEM` TEXT cells and the art-less `ITEMGIFT`
  half move their staged rect at the staging seam
  (`stage_shop_child_origin`); the painter and hit tests then read
  the moved rect like every other control (owner rule `f38f3`).
- anchored `ITEMBUY`/`ITEMPRE` keep the raw `.bnd` paint rect: their
  decoded sprite anchor already puts the visible 담기 / 입기 art at
  BND y + 70. `shop_click_rect` shifts only their hit rect by the
  same constant. `ITEMSHOW`/`ITEMNAME` stay raw (audit per-cell
  table). Shifting their paint rects as well would double-move the
  art below the cards (checked against the archive-backed shot).

D1/D2/D3 guards: `shop_stages_priced_children_at_the_exe_origin_but_shelf_art_raw`,
`shop_row_pick_at_the_visible_button_hits_the_first_row`,
`shop_priced_children_share_one_origin_without_double_moving_art`,
`shop_gem_cell_reads_the_exe_zero_when_no_price_word_is_pinned`, and
the archive-backed `live_shop_bnd_stages_the_audit_visible_row_and_price_origin`
(install-root skip). Red proof: setting `SHOP_CHILD_ORIGIN_Y` to `0`
fails the staging, hit, and installed-bind guards; replacing the gem
word with `--` fails the glyph-shape guard. The row-pick pixel
`(238,263)` reaches `ToggleRow(0)` on the installed BND; raw
`(260,195)` answers nothing. The gem word is the observed exe `0`
(`SHOP_GEM_UNPRICED_WORD`); the table→entry `+0x18` map stays OPEN.

D4 (frame state): `ShopPanel` defaults to `MyBag` at catalog staging
and switches to `ShoppingBag` on `ToggleRow`. `shop_panel_kept`
selects the staged art face; the cart overlay and bag pager ink only
on their own face. Guard `shop_entry_hides_the_cart_face_until_a_row_pick_opens_it`;
red proof: removing the panel assignment fails it.

D5 (frame state, resolved): the white `ITEMTOOLTIP`/`ITEMTOOLTIP2`
placements hide at entry and mount while the pointer lies on a
staged `ITEMSHOW` cell (`hovered_item_show_slot` from the shared
pointer snapshot). Guard `shop_white_tooltip_waits_for_a_hovered_shelf_cell`;
red proof: making `shop_tooltip_kept` always true fails it. The
sweep's row-button hover frame shows no tooltip, so button hover
does not arm it. Tooltip *ink* still rides the preview pick; the
exe tooltip's description text source stays OPEN.

Live status: the archive-backed staging/press guard is tier 3
(headless install-root read of the qualified bind); paint and panel
guards are SEAM. The paired Rust live re-run (wave 2) is still
OPEN for D1-D11; no live paired frame hash is claimed here.

## Privacy static sweep (#387)

The Item Shop is a local side screen: `visit_side(Shop)` stages this
seat's art and wallet. Both shops share the 5000 request family, whose
only known request is the 5012 download announce. No request or answer
names another seat's board, status, or location. Guard
`item_shop_has_no_visitor_request_in_the_shop_family` sweeps ids
5000..=5100 alongside the Music Shop guard from #402. This is a
NA-DESIGN result, never a passed visitor cell.

Live cross-check (trace `S-EVSHOPS-MASTER-01`, exe SHA-256
`83a04541…72c823b`, local Memoryer pid 37333 / epoch 1795246022):
the exe's Item Shop entry frame
`92cc37515fe7d588b78db03e45fea5ea275e2545fef4ed7d60d85bbc80d76693`
shows four goods cards, won `4500`, gem `0`, and wallet `G 15527 / W 0`.
Its wire tap logs only `C2S 5006` on entry; the observing Rust peer's
Room frame `c6edd59d…` and wire log contain no 5006/5007 or shop
status frame. The peer sees nothing about the shop visit. The peer's
audio sidecar records `route=sink`, `on_default_sink=0`,
`preboot_breach=0`.

## #178 enrichment pins (2026-11-22)

Static audit of `FUN_00499760` (`ev_f3ad15be…`, VA `0x499760`) adds
pins that the earlier live sweeps did not see. Evidence IDs and own-word
behavior also live in spec #178's enrichment rows.

- BAGSORT `0x142000b7` pushes virtual send `0x13ae` (file `0x9bfa8`),
  the empty 5038 RefreshBag; `FUN_004a1010` (`ev_ec83763a…`) gates the
  control on cart state. This CONTRADICTS the #195 note above that
  BAGSORT "records nothing". Both claims stay recorded: our shell now
  queues the static send, and the next step is one BAGSORT press wire
  tap (a NULL tap retires the static claim rather than inventing a
  handler). Guard: `bagsort_queues_the_exe_refresh_bag_send` and
  `bagsort_press_sends_one_empty_5038_on_the_game_socket`.
- MYCART_LEFT/RIGHT `0x14200072/73` step page `+0x8ce` modulo count
  `+0x8cf` only when the cart flag `+0x8cd` is set; RIGHT holds at the
  last page, LEFT holds at the first. MYBACK_POS `0x1460006d` clears
  the flag, MYCART_POS `0x1460006e` sets it, and a row check sets it
  (same evidence `ev_f3ad15be…`). The panel face carries that flag in
  our model; guard
  `cart_pager_steps_only_while_the_cart_face_flag_is_set` uses a
  six-good, five-row cart fixture. Live multi-page press frames OPEN.
- Cart total cells: `FUN_004a1010` (`ev_ec83763a…`) enables `CHECKMONEY`
  and the buy/gift/pager row by the cart-count word `+0x2334`;
  `FUN_004a0660` (`ev_3e020a4a…`) stages `CHECKMONEY`/`CHECKGEM` rects
  into object `+0x247c`/`+0x248c`. Our painter now reads the staged
  pair and inks nothing for an empty cart. The staged-price sum stays
  ours-by-design (#515); the exe's total-ink writer is still OPEN
  (qualified cart-pick frame, then writer REA). Guard:
  `cart_total_cells_ink_only_when_the_cart_count_gate_is_open`.
- ITEMBACK `0x1420006c` calls FUN_00540500, copies `0x2190` bytes
  from global `0x77cd00` over the local block at object `+0x65`, then
  repaints (`ev_f3ad15be…`). Our shell snapshots the equipped slice on
  character staging and overwrites every slot on ITEMBACK, so a later
  slot edit cannot leak into the revert; the rest of the exe block
  (wallet/inventory) stays server-authoritative, not minted locally.
  Guard: `itemback_restores_the_full_equipped_block_not_one_slot`.
  Live before/after figure frames OPEN.
- ITEMPRE `0x14200064..6b` gates on category byte `+4` inside
  `0x18..0x1b` and worn slot `(word +6 >> 7) & 0xf` equal to global
  `0x77cd28` or slot 2. An occupied word at object `+0x8ad` blocks
  categories other than `0x0b`/`0x16` and posts the CP949 string at
  VA `0x648878` (`ev_0a8d91c7…`, `[ 정보 ] … '소품'`); a slot
  mismatch posts the string at VA `0x6488e4` (`ev_9b1586a2…`,
  `성별이 다른 아이템은 입을 수 없습니다.`). A good passes to
  FUN_0049fba0 (`ev_82741b08…`) and repaints only then. The
  file→record join for the gate words is not pinned, so callers stage
  them explicitly. Guards: `occupied_worn_slot_posts_the_exe_notice_and_blocks_the_try_on`,
  `a_wrong_category_preview_press_stages_nothing_and_posts_no_notice`,
  `a_mismatched_worn_slot_posts_the_exe_gender_notice`. The notice
  dialog mount stays OPEN: one qualified ITEMPRE press frame on a
  blocked slot, plus the before/after figure pixel pair.
- BUYITEM `0x14200070` / GIFTITEM `0x14200071` share the web gate
  `FUN_00556680(0)`, empty-cart (`0x648cc8`) and cash/gem checks
  (`0x768b4c/50/54`, `0x768b44`; notices `0x648b38`/`0x648b14`).
  BUYITEM alone checks bag room (`0x648c24`/`0x648c00`) and rejects
  item `0x5bc` at a 180-entry bag (`0x648b88`). The passing arm writes
  mode word `+0x969` = 1/2 and virtually sends `0x1397` = 5015 empty
  (`ev_f3ad15be…`). CONTRADICTION: the qualified live taps emitted no
  5000-family opcode, so our web surface and gift no-op record stay;
  the 5015 send is staged behind the gates above. Next step: one wire
  tap through a valid cart beside the painted notice. Guards:
  `a_valid_cart_sets_the_buy_mode_and_queues_the_5015_send`,
  `giftitem_with_a_valid_cart_writes_mode_two_and_sends_the_5015`,
  `an_empty_cart_blocks_the_static_payment_leg`,
  `a_gem_shortfall_blocks_the_static_payment_leg`,
  `a_valid_cart_sends_one_empty_5015_after_the_buy_ladder`.

## #178 enrichment — exit pair (#775)

`FUN_00499760` (`ev_f3ad15be…`) names two static exit arms beside the
live-pinned full-bleed BACK plate: `0x14200002` BACK cues two UI sounds,
changes screen, and virtually sends `0x138f` = 5007 ExitShop; `0x14200003`
GOMYROOM changes screen with **no** send. Both codes now route through
`shop_action_for_code`: the static BACK arm shares the pinned empty-5007
outbox with the tapped plate, GOMYROOM calls `leave_side` alone. The two
sound slots are not pinned, so no UI effect is invented. Guards:
`item_shop_static_back_arm_sends_the_same_empty_5007` and
`item_shop_go_myroom_leaves_without_an_exit_send` (synthetic placements,
hidden-arm safe). **OPEN (next step):** one tapped GO MYROOM frame with a
wire tap to name the destination and confirm opcode absence.

## #178 enrichment — gender switch (#776)

`FUN_00499760` (`ev_f3ad15be…`) answers GIRL `0x14202006` and BOY
`0x14202007`: the GIRL arm writes byte `+0x18 = 1`, hides BOY; the BOY
arm writes `0` and hides GIRL. Both cue slot `0x1b` and call one of
three refreshers by two object words (both zero → `FUN_0049ead0`, first
zero/second nonzero → `FUN_0049e2d0`, first nonzero → `FUN_0049f0c0`),
passing 1 when planet index `0x17` is 12. Our board now stages the
byte, the hidden button code, and a resolved refresher call; the staged
context and lucky word are seams for the still-unmapped object words.
The sound slot is recorded here, not invented in the UI-effect layer.
Guards: `shop_gender_press_writes_the_byte_and_hides_the_other_button`
and `shop_gender_press_calls_the_context_refresher_for_the_staged_role`
(synthetic placements, hidden-arm safe). The live dump stages neither
button (absence, #195); **OPEN (next step):** before/after exe frame
pair for each press, in shop and myroom contexts.

## #178 enrichment — planet carousel (#777)

The two planet arms `0x14200008/09` (`ev_f3ad15be…`) hide the staged
12×3 grid, move index `0x17` with UP 1→12 / DOWN 12→1, ink
`FUN_0042f870(index,0)` when the figure read arms, rerun the same
three-way context refresh with the index-12 word, then call the seed
function twice and reset the three layout words `0x8d4..6` to `0,0,1`.
Our board stages the grid count and caption arm (the exe→our-object
join is not pinned), and records the caption argument, hidden cell
count, two seeds, layout words and refresher call. Sound slot `0x1c`
is pinned but not modeled in the UI-effect layer. Guards:
`shop_planet_up_wraps_one_to_twelve_and_down_wraps_back` and
`shop_planet_turn_hides_the_grid_reseeds_and_refreshes_the_context`
(synthetic placements, hidden-arm safe). **OPEN (next step):** a
qualified press frame pair for both directions; no readout pin exists.

## #178 enrichment — gender-indexed captions (#782)

`FUN_004a0660` (`ev_3e020a4a…`) resolves the six submenu captions
`0x14208012..17` from a word table at VA `0x648530` (REA
`ev_37bbdf86…`): row stride is the gender byte `+0x58`, six 4-byte
words per row; `-1` hides the caption, any other word keeps the
control's own text template. The BOY row inks all six; the GIRL row
hides slots 5 and 6. Our painter applies that rule once a GIRL/BOY
press stages the byte (before a press the entry byte is unpinned, so
nothing is hidden by guesswork). Category tab presses keep the
live-pinned local highlight only; the cart-state enable gate on the
row-control family belongs to the #787 work already landed. Guard:
`shop_girl_press_hides_the_last_two_submenu_captions` (synthetic
placements, hidden-arm safe). **OPEN (next step):** read the six
caption strings on a qualified frame in both gender states.

## #178 enrichment — owned bag cells (#783)

The ten bag-cell arms `0x1460003d..46` (`ev_f3ad15be…`) select
`DAT_0077ccc0` at `cell + page*10`, cue slot `0x18`, and post a notice
from the item's category byte: `0x19` → `0x1460003f`, `0x1a` →
`0x1460003e`, `0x1b` → `0x14600040` (type 2). A worn slot posts type
`0x0d` with string `0x64886c`; a special item pair (globals
`0x77991c/20`) refuses selection. Our board stages category, worn and
locked words because the file→entry join is not pinned. To read the
paged item, `ShopState::apply_action` now takes the owned goods list
instead of just its length (same five callers). Guards:
`shop_bag_cell_selects_the_paged_index_and_posts_its_category_notice`,
`shop_bag_cell_with_a_worn_item_posts_the_occupied_slot_notice`,
`shop_locked_bag_cell_selects_nothing_and_posts_no_notice` (synthetic
placements, hidden-arm safe). **OPEN (next step):** one selected-cell
frame with the notice ink.

## #178 enrichment — bag pager wrap and clear (#784)

The static arms `0x1420004c/4d` (`ev_f3ad15be…`) differ at the ends:
BAGLEFT applies `(page - 1 + count) % count` (wrap from page 0 to the
last), BAGRIGHT increments only while `page+1 < count` (hold). Both
write `DAT_0077ccc0 = -1`, clear the highlight and refresh. Our board
now wraps LEFT, holds RIGHT, and clears the selected row and its notice
on either press. CONTRADICTION recorded: #195 above said "holds at
both ends" from the live sweep; the exe arm pins wrap on LEFT, so the
earlier note stays as history and the #784 guard owns the behavior.
Guards: `shop_bag_pager_wraps_left_and_clears_the_selected_row` and
the updated `shop_bag_pager_turns_over_staged_owned_inventory`
(synthetic placements, hidden-arm safe). **OPEN (next step):** a
page-boundary frame pair on a multi-page bag.

## #178 enrichment — vacate and conference notices (#786)

`FUN_00499760` (`ev_f3ad15be…`) answers ITEMVACATE `0x14200005` only
while the cart flag `0x8cd` is clear. A staged confirmation pair posts
notice type 1 with string `0x648af0`; item codes `0x23`/`0x24` post
type 1 with `0x648acc`; any other item posts type 2 with a formatted
`0x648a90` string. CONFERENCE `0x142000b6` posts type `0x14` with no
text. Our board stages the confirmation pair, item code and item word
(the value sources are unpinned) and records the resolved notice.
CONTRADICTION recorded: spec #178's enrichment row assigns notice type
`0x12` to ITEMVACATE; the exe arm uses 1/2 and type `0x12` belongs to
CHARGEMONEY `0x14200004` on a busy web gate. The live CHARGEMONEY press
still opens the Cash Charge view (#397); the notice-type claim is not
silently moved. Guards:
`shop_itemvacate_notice_needs_the_owned_bag_face_closed`,
`shop_itemvacate_notice_uses_the_special_item_and_item_word_branches`,
`shop_conference_posts_the_type_twenty_notice` (synthetic placements,
hidden-arm safe). **OPEN (next step):** qualified press frames with the
notice text source.

## #178 enrichment — ext-menu toggle (#788)

The EXTMENU arm `0x14200099` (`ev_f3ad15be…`) acts only while gate
word `0x5f` is set, clears it, flips `0x5b`, takes one fresh seed, and
cycles visibility: the 12 filters `0x1420109b..a6` appear on open and
hide on close; the eight first-row ITEMBUY/ITEMNAME/ITEMGIFT/ITEMSHOW
controls and four `0x1401d088..8b` controls do the reverse. Open also
clears `0x5d`; both branches refresh the context. Our board stages the
gate (`0x5f` writer unpinned), counts accepted toggles and seeds, and
applies one visibility helper in the paint filter. The live dump stages
no filters (#195), so the rule is static-only. CONTRADICTION recorded:
#195 called these controls "never stage live" — kept as the live
absence, not as a claim the handler is dead. Guards:
`shop_extmenu_toggle_cycles_twelve_filters_and_group_visibility` and
`shop_extmenu_toggle_is_ignored_while_the_gate_is_closed` (synthetic
placements, hidden-arm safe). **OPEN (next step):** one EXT_ON press
frame with before/after visibility.

## OPEN

1. Entry/exit dispatch call sites — PARTIAL: the static BACK and GO
   MYROOM arms now dispatch; the GO MYROOM destination frame is OPEN.
2. TEXT caption and icon staging sources — PARTIAL: submenu
   visibility is pinned by the gender table (#782); the six caption
   strings and the row GEM cells now ink the
   exe's observed `0` per #722; the table→entry `+0x18` map is still
   unpinned. `MGASH`→`o2_cash` join OPEN — rig capture at known
   balances still names the cell). Submenu string text stays OPEN.
3. Buy/gift/cart/set-item click semantics — PARTIAL: `ITEMBUY`
   checks the cart, `BUYITEM` opens the web surface with no game-socket
   opcode (W-a NULL, #397),
   `ITEMPRE`/`ITEMBACK` preview and clear (comment pairing
   ASSUMED); `ITEMGIFT`/`GIFTITEM` record a no-op (ticket #196);
   set-item unevidenced; bag-vs-giftbox landing UNCONFIRMED
   (ticket #196 — probed nowhere, never invented).
4. Scroll, bag pager, planet pager, ext-menu filter behavior —
   PARTIAL: scroll pager steps and holds (origin-offset
   read ASSUMED); bag pager steps and holds over staged owned
   inventory (L/R readout pairing ASSUMED), and a cell press selects
   the owned row with its category notice (#783); LEFT wraps and both
   pager arms clear the pick (#784); planet pager turns the index with
   wrap and reseeds (#777); the ext-menu toggle cycles filter and
   group visibility from the static arm (#788). None of these controls
   stage live in the pinned dump (absence, ticket #195); their press
   frames stay OPEN.
5. Gender buttons — PARTIAL: byte/hidden-button/refresher staged;
   the screen→object-word mapping and before/after frames stay OPEN.

## #877/#886 — state-3 entry card face (2026-11-23, quiesced v3 pair)

The v3 entry/cart exe frames show the `0xc` new-arrivals face: one
`SET 0x1D000` card per featured good, with the dark inset
`Shop_SetItem_Back.ojs` frame, a stretched catalog portrait, and the
title/price ink in the card's staged cells. The rust legs stacked every
frame of the multi-frame bodies and painted eight raw `ITEMNAME` rows.

Pins: `FUN_00498bc0` (file `0x98bc0`, REA `ev_5bee92ef…be`) dispatches
`0x1420100A..0E` and sets each `SUBMENU1..6` image-control frame range
via `FUN_0042f870(2*i+1, 2*i+2)` (REA `ev_49cf8f68…11`); the four
`SETITEM_EX` parents are `0x1401D088..8B` from `ControlList_Interface.txt`.
The four portrait bodies' gray backdrop word `0x5294` is read from
`avatar.opa`; the two attribute lines were later pinned to exe format
strings (VA `0x64c6cc`/`0x64c6b4`) whose `%d` values are item-row words
at file offsets `+22`/`+26` (see #877/#886 below).

The first re-pair plateau was a wrong-sprite staging bug, not font
rasterizer noise: the cards paint slot-2 full-body sprites at pose
frame 21 with their archive color key, not stretched slot-0 thumbnails.
Pixel-match offsets from each `SETITEM_EX` parent origin are pinned in
`paint_shop_featured_icons`; the named residual is now the approved
modern-font glyph delta plus other non-card chrome.

Implementation: staged art clones the art-less card children under each
parent origin; the Shop paint pass selects one frame per image control,
stretches the featured portrait into each staged `SET_AVATAR` cell, and
inks title/price into the staged card cells with the pinned GDI bearing.
The seam raw-catalog face (`set_catalog`) is unchanged. Guards:
`set_item_card_children_stage_once_under_each_card_origin`,
`entry_frame_selects_one_special_submenu_pair_and_the_dark_card_face`,
`stretch_blit_rgb555_skips_the_staged_portrait_backdrop_key`.
Live pair numbers and remaining OPENs are in
`~/scratch/fix-chrome-b-findings.md`.

## #877 — card-figure blit pin (close-out lane, 2026-11-25)

Version pin: `OTwo.exe` 7.55, 5,029,888 B, SHA-256
`83a04541…72c823b`; Ghidra 12.2 / REA 6.10.1. All offsets below are
file offsets = VA − `0x400000`. The oracle frame is the v6 entry capture
`060ab2af…de680` (the same v3 exe frame; only the Rust leg changed).

Statics walk: state-3 switch `FUN_00498bc0` (file `0x98bc0`, REA
`ev_d0c41e44…463f4`) calls `FUN_0049e8d0` (file `0x49e8d0`, REA
`ev_f5a5731f…43a903a`), which resolves each selected good and calls
`FUN_00529230` (file `0x529230`, 1033 B, REA `ev_81002554…de616b`,
pseudocode `ev_dfbca1e7…cae2cf`) at computed code `0x1401d088 + slot`.
`FUN_00529230` writes the two bonus strings and, at file `0x1293d0`
(address-to-offset REA `ev_cf53240b…c84d1569`; instruction walk
`ev_734d377c…78`),
`SetRect(0x59,0x3C,0xD5,0xD2)` = 124×150 into control `+0x1fc`, plus
control `+0x1e4`/`+0x1e8` from the read rect minus `0x5A`/`0x3C`.
CONTRADICTION (recorded, not silently resolved): that box is the
Playing number-engine glyph layout box (`docs/playing-digits.md` §3,
claim A45, ev `03aa0c52…`/`6d253f51…`), not the card-figure
destination. `FUN_00529230` writes **no scale factor** for the figure;
the brief's "fit/scale" premise is not supported by the exe.

Pixel pin: against `060ab2af…de680`, slot-2 pose frame 21 matches
1:1 (no stretch) at native sizes. Sprite last-ink rows are
233/233/233/234; exclusive bottoms are 234/234/234/235. Parent
origins come from `State_ItemShop.bnd` code-ordinal rects; offsets are
parent → image top-left:

| Card | Parent | Sprite / native size | Image origin | Offset | Old → new colour loss |
|---|---|---|---|---|---|
| 1 | (213,62) | `SA0501.ojs` 127×168 | (212,66) | (−1,4) | 8730 → 1.5 |
| 2 | (354,62) | `SA0601.ojs` 86×168 | (380,66) | (26,4) | 9732 → 1.5 |
| 3 | (495,62) | `SA0401.ojs` 81×147 | (522,87) | (27,25) | 3580 → 1.2 |
| 4 | (636,62) | `SA0302.ojs` 72×141 | (669,94) | (33,32) | 4599 → 0.8 |

Loss is mean squared 5-bit RGB distance over non-key source pixels
with round-to-8-bit expansion (scratch `fit_cards.py`). The earlier
impl-877 offsets `(9,2),(40,1),(29,19),(53,25)` were pixel-mismatched;
the card-2 region regressions (0.70 → 0.38) come from that displacement,
not from a missing stretch. The old "stretched `SET_AVATAR` cell" line
above records the v5 wrong-slot leg and is superseded for slot-2 bodies.

Fix: `SHOP_FEATURED_FIGURE_BLIT_OFFSETS` now carries the table above;
no new scale is invented. Guards: the synthetic
`featured_card_figures_land_on_the_pinned_exe_baseline_for_each_parent`
(install-root-free; red before the fix) plus the archive-backed
`featured_card_blits_the_full_body_sprite_at_the_exe_native_box`.
Interim post-fix evidence (reviewer):
`~/scratch/review-shop-figure/postfix-featured.png` paints the four
figures at the claimed origins with mean squared 5-bit RGB loss
0.37/0.35/0.42/0.37 (~0.4 each). That is not the live proof. The
post-fix v6 quiesced re-pair rerun, owned by the controller after
merge, is the live proof; this lane does not claim it.
OPEN: no literal per-card offset exists in the exe; the baseline is
pinned by pixel alignment on a 7.55 frame. Next evidence step: one
winedbg stopped-state read of the four image-control rects. Cart-frame
figure ink (`c2e056b8…ed03e45`) still differs by 1–7 px (pose/overlay
timing), so a live paired re-pair is still required.

## #877 — card price-row pin (close-out lane, 2026-11-25)

Version pin: `OTwo.exe` 7.55, 5,029,888 B, SHA-256
`83a04541…72c823b`; Ghidra 12.2 / REA 6.10.1. Offsets below are file
offsets = VA − `0x400000`. All claims are static bytes; no new live
capture is claimed here.

**Source is sprite glyphs, not colored text.** State open
`FUN_004984d0` (file `0x984d0`; instruction walk
`ev_7bd1198a…89fe3c1c`) loads four runtime font bodies beside the
archive slot bracket: state word `+0x2328` ← literal
`Shop_Font_ItemWon.ojs` at instruction `0x4986d8` / file `0x986d8`,
`+0x232c` ← `Shop_Font_ItemGem.ojs` at `0x4986f8`, `+0x2324` ←
`Shop_GemFont.ojs` at `0x498719`, and `+0x2330` ← `NewMark.ojs` at
`0x49873a`. The literal bytes are read at VA `0x64861c`–`0x64866f`,
file `0x24861c`–`0x24866f` (REA `ev_4bfd1499…875f0108b`), and each
load call uses `FUN_0040ced0` (REA `ev_1dcf6b61…5c22c93`).

The card face is reached from `FUN_0049e8d0` (file `0x49e8d0`, REA
`ev_5a9fbbfe…30c5`): it calls `FUN_00529230` for each staged
`SETITEM_EX` parent, and the state-3 price writer `FUN_004a21a0`
(file `0xa21a0`, REA `ev_8c9dfd4d…7011`, byte scan of code
`0x14400054` at file `0xa27bb`/`0xa2812`/`0xa28a2`, REA
`ev_058bdefd…82fc67`) then forks. On the **grid face** it inks raw
`%d` words into the global `ITEMGEM`/`ITEMWON` cells with
`FUN_0050f6f0(0,3,4)` (black), reading the row words at `+0x10`
(gem) and `+0x14` (won). On the **card face** it calls
`FUN_004a1eb0` (file `0xa1eb0`, REA pseudocode
`ev_1c918ec2…ada7b`, instruction walk `ev_46aade7c…777334ac`) with
the won word and `FUN_004a2010` (file `0xa2010`, REA
`ev_085d45d0…b50b8e6e`) with the gem word. Both split the value
into decimal digits, select a sprite frame with `FUN_0041e0f0`
(frame setter, REA `ev_78dc963e…7bdb14cc`) and blit it with
`FUN_0041df80` (anchor draw, `docs/ui-draw.md`, REA
`ev_7418d033…`). The won writer uses state `+0x2328` for digits
`0..9` and appends frame **11** (the green `₩`) from the same atlas.
The gem writer uses `+0x232c` for digits `0..9` and appends frame
**10** from `+0x2328` — the gold `G` glyph comes from the *won*
atlas frame, not the gem atlas. At file `0xa1fc1` the won-glyph
index is the literal `0xb`; at `0xa211b` the gem-glyph index is
`0xa` (same instruction walks).

Geometry is the card's cloned `SET_PRICE` cell, code `0x14400094`
under each `SETITEM_EX` parent, obtained by `FUN_00529680` (file
`0x529680`, REA `ev_a2dc3835…180fe8edb9`). The caller shifts its
origin by `+0x28` in x, then anchors won digits at `+5` and gem
digits at `+0x46`, at the cell's y. Glyphs back off one 8-px digit
pitch per digit: won glyph left = `anchor − 8·digits − 7` (13 px
wide), gem glyph left = `anchor − 8·digits − 4` (9 px wide). Font
anchors are `(0,0)`, so both ink at the cell y (REA walks above).
`State_ItemShop.bnd` (slot 1463, data offset `65475546`, 186 rects)
pairs that clone to `(219,238)-(345,250)` for parent `0x1401d088`;
the four parents step 141 px. The global `ITEMWON`/`ITEMGEM` cells
are the grid face, not the card price row.

Sprite pixels from 7.55 `Interface1.opi` (75,100,702 B, SHA-256
`d6654119…d50584`; local archive offsets, no REA evidence id):
`Shop_Font_ItemWon.ojs` is slot 640 at data offset `31401138`,
size `2384`, colorkey `0x7D5F`, 12 RGB555 frames. `Shop_Font_ItemGem.ojs`
is slot 639 at `31398754`, size `2384`, same bodies. Frames `0..9`
are 7×12 gray digits (core `0x318c` = `(98,98,98)`); frame 10 is
9×12 gold (core `0x7f20` = `(255,205,0)`); frame 11 is 13×12 green
(core `0x3706` = `(106,197,49)`). **CONTRADICTION recorded:** the
owner's v6 visual note called both digit runs green/gold; the
exe's own sprite bytes make only the prefix glyphs green/gold and
the digits gray. The exe is the oracle; the note stays as history.

Implementation note: the shell's modern-font seam (owner decision
`e8d01`) paints the two prefix glyphs as text at the pinned colors
and geometry, because these runtime font bodies are not named in
`ControlList_Interface.txt` and staging them would need changes
outside this lane's owned paths. The reviewer's post-fix paired
re-pair (`~/scratch/review-shop-price-row/score-price-row.log`) moves
region x 214–760, y 240–272 from structural 0.7943 to **0.8455**
(raw 0.8206). Pixel fidelity of the glyph shape stays OPEN; next
evidence step is a staged-font seam for an exact sprite match.

## #877 — bottom-right preview-figure statics pin (close-out lane, 2026-11-26)

Naming correction (`#877` triage said "preview"): the bottom-right
figure is the shop master mascot, not a try-on preview; try-on uses
the separate upper-left avatar pane.

Version pin: `OTwo.exe` 7.55, 5,029,888 B, SHA-256
`83a04541…c823b`; Ghidra 12.2 / REA 6.10.1; file offsets are
VA − `0x400000`. Oracle: v3 quiesced entry frame
`~/scratch/paired-shots-v3/states/itemshop-entry/exe.png` (the figure
is visibly present on entry, so this is not a post-click-only state).

**Host is the shop-master image, not the try-on avatar.**
`ControlList_Interface.txt` names `IDC_IMAGE_SSHOP_SHOPMASTER` at code
`0x14600053` (art-less) and `IDC_IMAGE_SSHOP_AVATAR` at `0x14700034`
(art-less). `State_ItemShop.bnd` (slot 1463, data offset `65475546`,
186 rects, code-ordinal pairing) gives `0x14600053` rect
`(600,445)-(798,566)` (198×121) — the bottom-right cluster — while
`0x14700034` is `(35,79)-(158,229)`, the upper-left doll pane.
The dotted field is the empty state of the art-less shop-master
image control; it does not mean a figure was staged and lost.

State open `FUN_004984d0` (file `0x984d0`, REA `ev_09f3512fd86e…98`)
calls `FUN_0048c8c0(0x14700034, scratch)` at file `0x8630`–`0x8642`
(REA `ev_12bf9ba37138…487`: ordinal-indexed control table,
`-0x5A/-0x3C` child offsets), then loads the four runtime font bodies
and calls `FUN_0049ffe0` at file `0x4987c9`. `FUN_0049ffe0` (REA
batch `ev_8f449bcdcdff…8494`) decodes 36 `Shop_master_*` bodies into
state arrays `+0x2340`/`+0x2344` (main, close-eye, say per variant)
and activates the entry category tab. Click arms on the same host
control are in `FUN_00499760` (file `0x4998b3`, REA
`ev_cb2962539363…383a`: sound slots `0x28+`, flag `+0x2354`) and
`FUN_00498bc0` (file `0x498c3a`, REA `ev_d0c41e4412c2…3f4`: cycles
`DAT_0067e018` and arms `+0x2354`). Direct code-constant scans:
`ev_087eb38c5526…aa` (`0x14700034`) and `ev_59b128821669…b97`
(`0x14600053`). Try-on writes the *other* pane: `ITEMPRE`/`ITEMBACK`
call `FUN_0052b480(state+0x65)` only when control `0x14700034` is
staged (`ev_cb2962539363…383a`). No evidence dresses the shop master
with a good: the entry figure is the site mascot, not a try-on result.

**Which figure.** `Shop_Master_main.ojs` in `Interface1.opi`
(75,100,702 B, SHA-256 `d6654119…d50584`; slot body at archive offset
`31568066`, size `30388`, colorkey `0x7d5f`) has one 132×115 frame with
anchor `(654,446)`. Blitting its non-key pixels at that anchor onto
the v3 exe frame matches 7599/7599 ink pixels (mean squared RGB
rounding error 3.6). The blink and speech overlays are separate
bodies (`Shop_Master_CloseEye.ojs` anchor `(716,469)` 32×16;
`Shop_Master_Say.ojs` frames at `(724,488)`/`(724,487)` 16×9); the
entry frame shows only the main body. Planet/event variants exist in
the same 36-body array; the v3 frame shows the default main variant.
Archive offsets and sprite pixels are local file facts, not REA IDs.
Scratch proof: `~/scratch/shop-preview-pin/` (`Shop_Master_main.png`,
`exe-br-crop.png`).

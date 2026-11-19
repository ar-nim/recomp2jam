# STATE_SHOP screen (7.55) — evidence log

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
back/myroom targets; no `EXIT` id exists in-block. The tapped BACK
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
commits through `BUYITEM` (`0x70`) as the pinned empty
`SyncItemPurchase` 5000 send (file `0xf3fa8`); the 5001 answer
is undecodable standalone, so the wire ride stays ASSUMED behind
the take-seam (`docs/shop-buy.md`). Sort/filter/set-item/bag/
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
unpinned) stage but record nothing. No search chrome exists in
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

## OPEN

1. Entry/exit dispatch call sites.
2. TEXT caption and icon staging sources (GEM cell staged from
   5028/5001 words since ticket #21; `MGASH`→`o2_cash` join OPEN —
   rig capture at known balances still names the cell).
3. Buy/gift/cart/set-item click semantics — PARTIAL: `ITEMBUY`
   checks the cart, `BUYITEM` commits 5000 (ASSUMED wire-side),
   `ITEMPRE`/`ITEMBACK` preview and clear (comment pairing
   ASSUMED); `ITEMGIFT`/`GIFTITEM` record a no-op (ticket #196);
   set-item unevidenced; bag-vs-giftbox landing UNCONFIRMED
   (ticket #196 — probed nowhere, never invented).
4. Scroll, bag pager, planet pager, ext-menu filter behavior —
   PARTIAL: scroll pager steps and holds (origin-offset
   read ASSUMED); bag pager steps and holds over staged owned
   inventory (L/R readout pairing ASSUMED); planet pager and
   filters display-only — the filter buttons never stage live
   (absence, ticket #195).

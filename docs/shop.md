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
back/myroom targets; no `EXIT` id exists in-block. Room side
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
(`0x50`/`0x51`) digit unions. Labeled no-ops (each pinned by
`shop_unwired_browse_chrome_lands_nowhere`): the gender, category
(`0x0A..0x0E`), submenu, and ext-menu filter buttons never stage
in the live dump (203 placements, verified 2026-11-13 — the
`ControlList` names them but the staged set omits them), so
filter clicks land nowhere by absence; `BAGSORT`, the planet
pager, `ITEMGIFT1..8`/`GIFTITEM`, cart `CHECKGEM`/`CHECKMONEY`
totals, and the `ITEMGEM1..8` cells (gem-price join still
unpinned) stage but record nothing. No search chrome exists in
`State_ItemShop.bnd` (#178 story 8 has no control to wire).
TOP100 is music-shop-only per the #181 verdict and owns no
item-shop arm (`MSHOP_TOP100_CODE` never dispatches here).

## OPEN

1. Entry/exit dispatch call sites.
2. TEXT caption and icon staging sources (GEM cell staged from
   5028/5001 words since ticket #21; `MGASH` source unpinned).
3. Buy/gift/cart/set-item click semantics — PARTIAL: `ITEMBUY`
   checks the cart, `BUYITEM` commits 5000 (ASSUMED wire-side),
   `ITEMPRE`/`ITEMBACK` preview and clear (comment pairing
   ASSUMED); gift/set-item unevidenced.
4. Scroll, bag pager, planet pager, ext-menu filter behavior —
   PARTIAL: scroll pager steps and holds (origin-offset
   read ASSUMED); bag pager steps and holds over staged owned
   inventory (L/R readout pairing ASSUMED); planet pager and
   filters display-only — the filter buttons never stage live
   (absence, ticket #195).

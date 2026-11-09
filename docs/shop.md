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
   `stage_interface_scenes` + soak tests; clicks land nowhere per P0).

## OPEN

1. Entry/exit dispatch call sites.
2. TEXT caption and icon staging sources.
3. Buy/gift/cart/set-item click semantics (P0: unevidenced).
4. Scroll, bag pager, planet pager, ext-menu filter behavior.

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

## Screens (implemented)

Shell screen + paint + flow + live staging — DONE
   (`Screen::MusicShop`, `paint_music_shop_scene`, all-reject flow,
   `stage_interface_scenes` + soak tests; clicks land nowhere per P0).

## OPEN

1. Rect contents → placements.
2. Album-art source + GEM/WON buy semantics (`jsBuyMusic`/Payment).
3. Preview, download progress, select, cart, gift, delete flows.
4. Search/sort/filter/pager behavior; caption/money sources.
5. Shell paint/flow/staging; entry/exit paths.

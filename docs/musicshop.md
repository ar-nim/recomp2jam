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
next step as lanking). PRICE (`0x83`), 3KEY/7KEY filters, MYALL
owned-list pager, search, TOP100, gift, and download stay
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

## OPEN

TOP100 ownership (ticket #181): the `0x1A20007D..0x84` arm (TOP100 + key/title/diff/bpm/price + SELECTALL) is music-shop chrome only; the item shop (`State_ItemShop.bnd`) has no TOP100 control (owner live check 2026-11-13). Corrected: docs/parity/shop.md, docs/shop.md (no TOP100 entry present), and the #178 story list (story 10 mis-scoped → music shop #179).

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

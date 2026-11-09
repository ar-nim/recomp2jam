# STATE_MYROOM screen (7.55) — evidence log

Avatar room (`CStateMyroom`, `0x6A`; `CItemManager`, `CPlayerInfo`).
Read-only walks below; offsets are file offsets unless marked VA.
Version pin for all EXE rows: `AngelO2Jam.exe` 5,029,888 B, SHA-256
`83a04541…72c823b`.

## Block walk (2026-11-09, `ControlList_Interface.txt`)

`STATE_MYROOM`, decl `0X6A`, `BOUND "State_Myroom.bnd"`, ~84
top-level `IDC_` entries plus gift `SET`s:

| Codes | Entries | Art |
|---|---|---|
| `0X15000000` | `IDC_STATE_MYROOM` (self) | — |
| `0X15100001` | `IDC_BACK_SMYROOM_1` | `State_Myroom.ojs` |
| `0X15200002..04` | BACK / GOSHOP / ITEMSELL | matching `.ojs` |
| `0X15200005..0F` | 11 equip-slot buttons (instrument…shoes) | — (selectors, art-less) |
| `0X15400010..11` | O2MONEY/CHARGEMONEY texts | — (captions OPEN) |
| `0X15700012` | AVATAR image | staged figure |
| `0X15600013..22` | 16 slot images with Korean slot comments (악기/헤어/소품/장갑/목걸이/상의/하의/안경/귀걸이/의상소품/신발/날개/악기소품/펫/헤어소품/코스튬) | — (icons OPEN) |
| `0X15600023..2C` | 10 bag items | — (icons OPEN) |
| `0X1520002D..32` | bag pager + page numbers | `Shop_Btn_Bag*.ojs`, `MShop_PageNumber.ojs` |
| `0X15500033..36` | select/slotcover/userinfo | matching `.ojs` |
| `0X15400037..44` | nickname/level/exp/rank/guild/money texts | — (captions OPEN) |
| giftbox + gift `SET` | gift UI + 5 gift cells | `Myroom_Gift*.ojs` |

## Missing bind file

`State_Myroom.bnd` is declared but ABSENT from `Interface1.opi`
(no entry) and from `install-root/` — so the block paints the
unbound path: art at signed-i16 anchors, never rects (claims A10
family; memory `[0c6c3]`). No ordinal pairing applies. The avatar
portrait centers blind (audit 18–19).

## Data side (owned systems)

- Item catalog + names: `Itemdata_Korea.dat` parsed (1541 rows).
- Equip loadout: `SetInfodata.ojs` parsed.
- Avatar figures: staged bodies resolve (tally strip proves it).
- Inventory/stats values: server-fed (`CPlayerInfo`); fetch OPEN.

## OPEN

1. Anchor paint for each art body (per-frame `+0`/`+2` words).
2. Slot-button selection state + equip semantics.
3. Bag pager + gift flows (server assists).
4. TEXT caption sources.
5. Entry path and exits.

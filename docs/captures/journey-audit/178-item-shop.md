# Journey audit — #178 Item Shop

Comparison leg, no exe boots and no fixes. Base `2105ac64eb3504b4c00494c54ff98e0b1b38f1cb`;
worktree branch `cb2/ja-d`. Stories are the checkbox items in `gh issue view 178`
(24 boxes, including the trailing legacy buy box).

## Evidence anchors (artifact + hash)

| ID | Artifact | Hash |
|---|---|---|
| EXE-B8 | `~/scratch/side-scenes/b8-cash/manifest.json` (exe `OTwo.exe` 7.55, 5,029,888 B, wine 11.22, pair `wine-x11-sendinput`, drive reached itemshop and cash-charge) | manifest `f9d4a6953c55b75b4f04db3c0ece198bd2032361ac65dc8275ce07ccb6aa3f8b`; exe `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b` |
| EXE-B8-ITEM | `b8-cash/artifacts/screenshots/itemshop.png` (four goods, figures, won `4500`, gem `0`, wallet `G 15527` / `W 0`) | `0c561c6648fe29aaef1b64ca867de05baef9fae7ab12afbcd9c1a42652293ee8` |
| EXE-B6-ITEM | `b6-shops/artifacts/screenshots/itemshop.png` (same board, independent pack) | `ed47aae50126fc73bdaeb25d3a0ffa58ce8573fe3b7f79a37d1ef27d86a4b352`; manifest `c8adc9aa01e72258b336f6b470a62bb6373d15a4451405019ec943da2253131b` |
| RUST-NOW | `recomp2jam shots --out-dir ~/scratch/ja-d/current-shots` at base commit, `shop-native.png`; scenario + staged-data caveat in `docs/captures/t207/item-shop.md` | `a61c12366966972ace8267eeffb4f6c1849a184460ee22a71732f1af6b5fbd15` |
| STATIC | `docs/shop.md`, `docs/shop-buy.md`, `docs/parity/shop.md`, `docs/chrome-wiring.md` (7.55 block walk, send/response offset pins) | source pins; no pixels claimed |

Comparator abbreviations: `pixel` = pixel-region, `wire` = wire-sequence,
`input` = input-response, `state` = state-path, `data` = data-effect,
`NA` = NA-DESIGN.

## Per-story table

| # | Story (short) | Comparator | Our journey ref | Exe evidence | Verdict | Notes |
|---|---|---|---|---|---|---|
| 1 | Goods board from staged `Itemdata_Korea.dat` / `avatar.opa`, not server restock | data / pixel | `shop_goods` (`screens/side.rs`) ← parsed `AvatarItemTable` → `paint_shop_board` → staged `ITEMNAME`/`ITEMWON`; guard `shop_board_paints_names_prices_and_wallet_at_staged_rects` (`paint.rs:6864`) | STATIC: 5001 restock empty (#191), `docs/shop.md` item-table walk; EXE-B8-ITEM shows a populated board | MATCHED | Source class and populated-board outcome align; exact names are fixture data, not a parity claim |
| 2 | Avatars for sale render on the board | pixel | `SHOP_ITEMSHOW_FIRST_CODE` is only a no-op dispatch (`shop_unwired_browse_chrome_lands_nowhere`, `game/mod.rs:16359`); `paint_shop_rows` inks text only. RUST-NOW cards are flat grey | EXE-B8-ITEM: each card shows an item figure; EXE-B6-ITEM agrees | MISMATCHED | **Filed #363**: ITEMSHOW art never staged. Divergence not in the blocker map; pixel proof already exists |
| 3 | Toggle item into cart, row checked | input | `ShopAction::ToggleRow` → cart state → `paint_shop_cart`, guard `shop_row_toggle_checks_the_cart_and_buy_opens_the_web_surface`; W-a qualified row pick `(238,263)` pins the SHOPPING BAG face | W-a `W6FA-BUY-02` frame `572d6c65…7016`; static `ITEMBUY` codes `0x14200018..1F` | MATCHED for the local cart face | The earlier no-cart-check pin was a wrong aim; story 32 re-scoped in #397 |
| 4 | `BUYITEM` opens the local web purchase surface | wire/local | No `sync_item_purchase` queue on the buy leg; guard `cart_row_buy_leg_is_wire_quiet_except_the_back_5007_family` | W-a: BUYITEM `(41,574)` → `구입 하기` `8f9fb1b5…81a9`; OK `(345,385)` → `9c1eca1a…479c`; zero `C2S 5000` | MATCHED as a negative pin | The 5000/5001 synthetic probe is SCRIPTED-GATEWAY only; no purchase opcode is invented |
| 5 | Gem balance updates after purchase | wire / data | `WalletState` + earn apply guards `shop_earn_packet_feeds_wallet_and_game_with_the_same_pinned_words` | STATIC B49 one-packet earn pin; 0x44 stat packet wire unpinned, no live shop readout capture | BLOCKED | Map #345; needs Memoryer earn replay + wallet→readout paint trace |
| 6 | Insufficient gems fails with server error surfaced | wire | No failure arm; `shop_buy_insufficient_funds_arm_open` is an empty test (`session.rs:3956`) | No pin for a 5001 failure shape | BLOCKED | Map #345; reply shape first, then UI arm |
| 7 | Category filters narrow the board | input | Filter codes stage never in the live dump (203 placements); `shop_unwired_browse_chrome_lands_nowhere` | EXE-B8-ITEM shows the filter tab strip; no exe press-and-compare capture | BLOCKED | Map #245/#249/#250; visual presence is not click semantics |
| 8 | No search control invented | NA | No search dispatch; guard sweep includes all staged codes | STATIC: no search control in `State_ItemShop.bnd` walk (#195) | NA-DESIGN | Negative pin; checked box is a no-op by construction |
| 9 | MYALL pager pages owned items | input / state | `shop_bag_pager_turns_over_staged_owned_inventory` (`game/mod.rs:16266`) | STATIC `BAGLEFT`/`BAGRIGHT` codes; L/R readout pairing ASSUMED in `docs/shop.md`; no live turn capture | BLOCKED | Map #245/#249/#250 |
| 10 | TOP100 board belongs to music shop | NA | `MSHOP_TOP100_CODE` never dispatches on item side; #181 ownership pin | STATIC #181 (`State_MShop.bnd` only) | NA-DESIGN | Ownership/research story, not an item-shop behavior |
| 11 | Item preview before buying | input | `ShopAction::TryOn`/`RevertTryOn`, `paint_shop_preview`, guard `shop_preview_selects_row_and_back_clears` (`game/mod.rs:16230`) | STATIC `ITEMPRE`/`ITEMBACK` naming; exe click semantics OPEN | BLOCKED | Map #245/#249/#250; ours-by-design semantics cannot be called 1:1 yet |
| 12 | Wallet readout shows live balance on entry | wire / pixel | `paint_shop_wallet` inks `GEMMONEY`/`MGASH` from staged wallet; RUST-NOW shows synthetic `G 1200` | EXE-B8-ITEM shows `G 15527`, `W 0`; source of the readout words unpinned | BLOCKED | Map #345 + #245; values are fixture data, not matched-account parity |
| 13 | Prices render per row from staged won source | pixel / data | `shop_row_prices_come_from_the_staged_catalog` (`game/mod.rs:16305`) | EXE-B8-ITEM: won `4500` printed per card | MATCHED | Structural match (source class + per-row cell); numeric value differs by fixture |
| 14 | Browse→cart→buy guarded at `ScreenGame` seam | NA | Guards named in rows 3, 15 plus `docs/parity/shop.md` Input | STATIC send pins only | NA-DESIGN | Test-authoring story; older guards lack rule-7 annotation (spec note) |
| 15 | Scripted-gateway round trip, success + failure | wire | Success exists at `session.rs:3879`; failure is an empty stub at `session.rs:3956` | 5001/5028 reply shapes unpinned | BLOCKED | Map #345; scripted helper alone cannot pin the wire |
| 16 | Try items on without buying | input / state | `ITEMPRE` try-on, `paint_shop_avatar` restage (#196) | Exe preview/try semantics OPEN in `docs/shop.md` OPEN 3 | BLOCKED | Map #245/#249/#250; ours-by-design semantics |
| 17 | My avatar shown in shop live | pixel | `paint_shop_avatar` + `paint_shop_avatar` guards (`paint.rs:7042+`); RUST-NOW pane is blank because no live character is staged in the shots path | EXE-B8-ITEM shows the equipped bald mannequin in the left pane | BLOCKED | Map #345 + #245; data-gated, so not a paint verdict without matched identity |
| 18 | Revert try-ons to equipped avatar | state | `ITEMBACK` and shop exit revert (#196) | No exe try-on/revert capture | BLOCKED | Map #245/#249/#250 |
| 19 | Gifts land in my bag | wire / data | `ShopGiftNoop` records id + next step, queues nothing (#196) | `ITEMGIFT` send unpinned; web-only `gift_input.aspx` in `docs/net-protocol.md` | BLOCKED | Map #345; bag-vs-giftbox landing unevidenced |
| 20 | Gift items to others | wire | Same no-op path | `ITEMGIFT` shape unpinned | BLOCKED | Map #345; do not invent a send |
| 21 | Gem and won balances visible and updated | wire / pixel | Both caption cells exist (`paint_shop_wallet`); no live update chain | EXE-B8-ITEM shows both words; 0x44/5028 wire arm OPEN | BLOCKED | Map #345 + #245 |
| 22 | Cart total shows purchase cost | pixel | `paint_shop_cart_totals` sums won; `CHECKGEM` stays `--` | `docs/parity/shop.md` Input lists cart totals OPEN; no qualified exe cart capture | BLOCKED | Map #245/#249/#250 |
| 23 | Gem price cell shows the exe's second price | pixel | `SHOP_ITEMGEM_FIRST_CODE` staged but deliberately blank; guard asserts `ITEMGEM stays dark` (`paint.rs:6989`) | EXE-B8-ITEM prints a second price `0` beside each won `4500` | MISMATCHED | Spec-tracked story with the same next step; **no new issue** (not novel). Needs entry `+0x18` price pin before paint |
| 24 | Item-shop buy round trip end to end (legacy box) | wire | Same as row 4 | Same as row 4 | BLOCKED | Map #345 |

## Summary matrix — #178

| Verdict | Count | Stories |
|---|---:|---|
| MATCHED | 2 | 1, 13 |
| MISMATCHED | 2 | 2 (#363), 23 (spec-tracked) |
| BLOCKED | 17 | 3–7, 9, 11, 12, 15–22, 24 |
| NA-DESIGN | 3 | 8, 10, 14 |

New issue: **#363** — "Bug: item-shop goods cards paint no per-item art while 7.55 renders item avatars"
(labels `needs-triage`, `component:shell`, `P2`; read back after create).

### BLOCKED list for the live lane

- Qualified exe item-shop *interaction* packs (filter press, pager turn, preview press, cart toggle, cart total): #245/#249/#250. Existing b6/b8 packs show the entry frame only.
- Live Memoryer reply pins for 5001/5028 (buy completion, insufficient funds, wallet/currency refresh, gift landing): #345 (closed; continuity caveat) plus a new qualified gateway replay.
- 0x44 stat packet byte layout for earn→wallet: #345.
- `ITEMGIFT` send shape and web gift flow: #345; currently out of scope for TCP work.
- Matched-identity live Rust capture for the avatar pane: #345 + #245.

# Item Shop story matrix — Spec #178 (WAVE 4, lane shops)

Method: `docs/captures/story-matrix/TEMPLATE.md`; cell shape follows the
Waiting (#291), Channel (#289), Room (#290) and MyRoom (#180) exemplars. Base
`878f102`; worktree `/home/arnim/Projects/recomp2jam-wt140`, branch
`cb2/story-shops`. Docs and tracker stories only: no code changes, no pushes,
no git remote config edits. The 7.55 executable is the behavior oracle
(`OTwo.exe`, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`);
Memoryer is a wire witness, never an exe rule. Captures stay in git-ignored
scratch; only hashes and text enter the repo.

## 1. Evidence inventory (written before any cell was graded)

| Source | Pin (full sha256 unless noted) | What it names / verdict |
|---|---|---|
| exe identity | `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b` | 7.55 oracle; all static offsets and codes below |
| `~/scratch/side-scenes/b8-cash` (cited exe pack, PASS) | citation manifest `f9d4a6953c55b75b4f04db3c0ece198bd2032361ac65dc8275ce07ccb6aa3f8b`; local `manifest.json` file hash `7cb327c1c67a9f4c1c9028475a1b605dc27d8b6ef6b5f565fcd3cc2a5975960b` (drift recorded in §4); `itemshop.png` `0c561c6648fe29aaef1b64ca867de05baef9fae7ab12afbcd9c1a42652293ee8`; `cash-charge.png` `27fd5435fbe64c96315fcb29f0947bfc02d794b318c8a3047dccf15d176be671`; `drive.log` names the two-click MyRoom plate `(89,320)` and charge plate `(92,355)` | Qualified exe entry frame: four goods cards with portraits, won `4500` and gem `0` per row, wallet `G 15527` / `W 0`, `01/03` owned-page readout. No interaction beyond entry and charge click. |
| `~/scratch/side-scenes/b6-shops` (cited exe pack) | citation manifest `c8adc9aa01e72258b336f6b470a62bb6373d15a4451405019ec943da2253131b`; local file `1cdd90d8285573b7b4b6998c475e92abe32c2a333de89364016bc2861461623b`; `itemshop.png` `ed47aae50126fc73bdaeb25d3a0ffa58ce8573fe3b7f79a37d1ef27d86a4b352` | Independent exe entry frame; its drive log says `stage cash-charge NOT captured` — negative for the charge frame, not a charge-behavior verdict. |
| `~/scratch/story-shops/item-interact/exe` (this wave; frames valid, cite PASS, pack label mismatch) | `itemshop.png` `d30dc7c451fbfbf47317be396e00aa132b68a292322c3b9f18f6425f9cef4fb2`; `item-cart.png` `6a5407dbcbb6057799b175d0dc70641db13ce6e444ec4f757dc9f780f5a35b45`; `item-preview.png` `d6625b12df53b468043ae281dab62643d6a41746365e4f088a0ce76d1b952c79`; `item-revert.png` `5c1e1fa7608706a9abaed3225702fcaff2f4a9ba1e149e9e331170e118343cd2`; `item-bag-page.png` `fa2b63f89498c4755d2da9c2452b262b48dfee7125e9c3958623d731dceb3eec`; wire `a97287d11b9e93b5c4326e4a93b6d1f7a5d01551ed34928a975b759cb8c8d31d`; manifest `64ae87c29c6d6ed72fd1060d09d72eb7a12e735d0a7e702d8b4570253345070f`; citation `dc72c41d53985c7919303e1b0a54425a59a2805357eebd99a02c774c6eb5298c` | 10/10 distinct frames, `cite --claim exe` PASS. Custom target id `itemshop-interact` is not a scene label, so the runner wrote FAILED although the driver reached `item-bag-page` (§4). Click on the pinned ITEMBUY1 left half paints no cart check (pointer + tooltip only); ITEMPRE1 shows a hover tooltip with unchanged avatar clothes; ITEMBACK clears the tooltip; BAGRIGHT moves the owned-page readout `01/03 → 02/03`. |
| `~/scratch/story-shops/mshop-wire4/exe` (this wave, PASS; wire shared with #179) | wire `7d092b9d8616d729297b567d440ed4b37b56fc2b9c27feeb3154cd4349241a5d`; `musicshop.png` `964be986ee1fa269306ad2115dfdb18efc1cc511e5dfa765e5845698bb8ee8aa` | One tapped exe session grades the shared 5028/5029 wallet source once: `C2S 5028` empty → `S2C 5029` body `a73c000000000000` (15527 / 0), matching the b8 item-shop wallet words. No 5001/5017/5018/5023/5024 frame. |
| Lane D read-only gateway probe (2026-11-19, no boot) | `/home/arnim/scratch/close-offline-gift-probe.log` `db1a395d5ff126608a6b1d5d533778d422e61fc962c8d20c9ec133379f16f9f0`; script `~/scratch/close-offline-gift-probe.py` | Live Memoryer (PID 92326) wire only: `C2S 5017` empty → `S2C 5018 len6 hex=000000000000` (invalid 0, count 0), plus a repeat of `5028 → 5029 body a73c000000000000`. No exe frame, no DB mutation, no boot. The empty gift list is a server frame pin; 5001/5023/5024 remain unavailable without mutating buy/claim requests |
| #363 (CLOSED `007c38c` incl. `d11fd20`) + `~/scratch/p363-findings.md` | base native shot `a61c12366966972ace8267eeffb4f6c1849a184460ee22a71732f1af6b5fbd15`; REA dossiers `ev_3d09…5cce31` (worn slot starts at 2 → row slots 0/1 are icons), `ev_0df48…fd20d` (CStateShop init stores eight `0x14600035..3C` ITEMSHOW objects), `ev_2523…45ada6` (separate `0x1460003D..46` MYBAGITEM run) | ITEMSHOW art is now staged from `avatar.opa` slot 0 at 109x81 rects; matched-catalog 1:1 pixel hash stays OPEN. MYBAGITEM slot 1 icons stay OPEN. |
| #362 / p362 / p362b (CLOSED `905ef9c`, `776cfe7`) + `~/scratch/p362{b,f}indings.md` | b8 frame `27fd5435…be671`; corrected Rust root frame `d04024813b799dbcde6c8c3c10c442d9b706ba86e1461f963ef480541389fbc9`; web region MAE 114.065 → 0.667, frame 105.948 → 2.042 | Charge-chrome rect pin (frame `(171,3)-(648,595)`, client `(190,58)-(630,564)`, close `(571,5)-(619,41)`), entry click `(92,355)`; `live_boot_charge_plate_snapshot_click_paints_chrome` covers snapshot click → painter. The old live-4 "negative" was a driver mtime/capture fault, not a client bug. X-close press remains OPEN. |
| #369 / live-4 gesture pack (cross-state entry) | live-4 citation `a0e0fd08ef2d917d36be9e590dabce7ebebb1ca69b4214cfb3be87dba633c4a1`; default `5ddb5454abaa17633e0f9da73997d4e03519d51f42cbad184bbe9f9aa7371663`; STATUS `b095a915bb5b9d35e3e7fc26abbf6031bd04ad773d5023f70547149154aeb9c6`; return `c838e4373952e54eade63c140f1d13561ff7dbd0d00f1b80dd9921cdd3c7bfe3` | Panel-face gesture pins live on the MyRoom screen that hosts the Item Shop plate; no shop-cell claim. |
| `docs/shop.md`, `docs/shop-buy.md`, `docs/parity/shop.md`, `docs/net-protocol.md`, journey audit `178-item-shop.md` | exe hash above; 5000 send file `0xf3fa8`, 5001 exclusive handler VA `0x560a40` / file `0x160a40`, 5002/5003 send/receive tables file `0xa4df0` / `0x158200`, 5006 EnterShop, 5028/5029 wallet, entry `+0x14` cash getter file `0x13b010`, `+0x18` gem getter file `0x13b030` / affordability check file `0x99fae` | Static/dossier pins. At this wave's writing no live 5001/5003/5017/5018/5023/5024 body existed; the 2026-11-19 SHOP-EVIDENCE rows below supersede that absence for the server column only. Green scripted guards are never exe proof. |
| MyRoom wave-3 gap list (`docs/captures/journey-audit/180-myroom-story-matrix.md`) | E-W/E-G next steps name 5001/5003/5017/5018/5023/5024 | Cross-reference: those wire cells are graded ONCE in this matrix and pointed back to #180. Shared server state is not a scenario delta without a wire log. |
| `~/scratch/evidence-shops/` scripted-gateway legs (2026-11-19, no exe boot; full hashes in `~/scratch/evidence-shops-findings.md`) | `uid1-server.jsonl` `506ac0a3…5d3b6`; `uid2-server.jsonl` `96bb62ce…25009b`; `uid1-invalid-sell.jsonl` `9c9af74a…50d67`; `buy-low-gem.jsonl` `f6f3177f…957e2`; `sell-invalid.jsonl` `e82aeedd…1fed7`; `gift-list-one.jsonl` `58b30686…1d4ec`; `gift-claim-fail.jsonl` `20118f43…bb746`; proxy `79565923…6b1ff` (final `ad66ca69…cdf61a`) | Live Memoryer frames through `15013 → 15010`: 5000/5001, 5002/5003 success+invalid, 5017/5018 empty list, 5023/5024 success+failure, 5028/5029 at gem 15527 and 626. Scripted modes rewrite one reply; always labeled SCRIPTED-GATEWAY, never exe behavior. |
| `~/scratch/evidence-shops/gifted-boot/exe` (boot 1, frames valid, cite PASS, pack label mismatch) | manifest `9ed013c3…11d0675`; citation `95a4c46a…497f2f`; drive `febe922c…497f2f`; wire `94b2ea8f…a2508`; `gift-tab.png` `41cdea4d…a0b1b2e`; `filter-tab.png` `d4074ce7…87c6d11`; `scroll-right.png` `099d1ee6…1fb98bcb`; `cart-press.png` `698f9ec8…a23a5f8de`; `shop-back.png` `ff522720…ef29b270` | Gifted uid-2 exe sweep: one present-box row; wallet `G 626/W 0` matching `5029 7202000000000000`; filter and scroll press frames; BACK press emits `C2S 5007` and paints the lobby; cart/BUYITEM presses remain negative (no cart check, no 5000). 12/12 distinct, `route=sink`, `on_default_sink=0`, `preboot_breach=0`. Pack FAILED only on the custom-target label. |
| `~/scratch/evidence-shops/master-boot{3,5}/exe` (boots 4 and 6, master role wire only) | boot 4 wire `b0c9cb5d…604ff1` / drive `0597b23e…7e5a4`; boot 6 wire `2ca2c32c…360e999` / drive `40f47bc9…4e956`; `master-promoted.png` in boot 4 | Exe joined waiting (`C2S 3002`), host left, exe received `S2C 3007` body `0001000000000000` (new master = seat 1) twice. Both boots then hit a CREATE ROOM modal; no master shop frame exists. Master cells stay GAP; next step is one boot with modal OK at `(400,460)`. |

## 2. Matrix (24 actions × 5 observer actors = 120 cells)

Cell key: `EVIDENCED` / `GAP` / `LIVE-FILLED` / `OPEN-FIX` / `NA-DESIGN`.
`self:` = the column actor raises the stimulus. The `solo` column is the
offline/window seam, not an online-shop claim; `master`/`peer` distinguish
seat roles, and the tapped exe guest is the local `peer` column (the MyRoom
exemplar uses the same convention). Every GAP names a next step; ids are
defined after the table.

| Action (raised by) | solo | master | peer | channel | server |
|---|---|---|---|---|---|
| **ENTER-ITEMSHOP** (self:seat holder from MyRoom) | NA-DESIGN — offline boot has no gateway side screen | GAP — role transition pinned (`S2C 3007 new_master=1` in `b0c9cb5d…604ff1` / `2ca2c32c…360e999`) but no master shop frame: both boots stopped at a CREATE ROOM modal; OPEN — `E-M` with modal OK `(400,460)` | EVIDENCED — `<MyRoom ITEM SHOP plate (89,320), two clicks>` → goods board with four portraits, won/gem prices and wallet; `0c561c66…` (b8), `d30dc7c4…` (this wave) | NA-DESIGN — board observer sees no side screen; design seam is the local `visit_side(Shop)` | LIVE-FILLED — `C2S 5006` empty on entry, keyed by the Item Shop frame; this wave's tap `a97287d1…` (no 5001 on entry) |
| **BOARD-CATALOG** (self:local staged Itemdata/avatar tables) | NA-DESIGN — no staged catalog offline | GAP — wire promotion pinned but no master frame; OPEN — `E-M` with modal OK at `(400,460)` | EVIDENCED — `<entry>` → four populated goods cards (names/portraits/prices) from the installed tables, not a server restock; `0c561c66…` / `ed47aae5…` | NA-DESIGN — private board | LIVE-FILLED — scripted client on live Memoryer: `C2S 5000` empty → `S2C 5001 len152`; uid-2 bag words `78010000 7f010000 23000000` (376/383/35); `96bb62ce…25009b`. Server wire only; exe restock frame stays OPEN — `E-T`; shared with #180 E-W |
| **GOODS-ART-ITEMSHOW** (self:server-owned item ids, local paint) | NA-DESIGN — no item ids offline | GAP — OPEN — `E-M` | EVIDENCED — `<entry>` → a figure on each of the four visible 109x81 card rects; `0c561c66…` / `ed47aae5…`; dossier `ev_0df48…fd20d`. #363 fix landed; matched-catalog pixel hash still OPEN (`E-C`) | NA-DESIGN — private art | NA-DESIGN — art keys come from table rows, not a wire field |
| **PRICE-WON** (self:staged catalog price word) | NA-DESIGN — no catalog offline | GAP — OPEN — `E-M` | EVIDENCED — `<entry>` → won `4500` in each visible card's price cell; `0c561c66…`, `d30dc7c4…`; getter pin file `0x13b010` | NA-DESIGN — private price cell | GAP — no live 5001 prefix/price word tapped; OPEN — `E-W-5001` |
| **PRICE-GEM** (self:staged catalog entry `+0x18` word) | NA-DESIGN — no catalog offline | GAP — OPEN — `E-M` | EVIDENCED — `<entry>` → second price `0` beside each won `4500`; `0c561c66…`; getter file `0x13b030` / check file `0x99fae` (spec story 23) | NA-DESIGN — private price cell | GAP — no live 5001 word-to-cell mapping; OPEN — `E-W-5001` |
| **WALLET-READOUT** (self:entry balance) | NA-DESIGN — no balance offline | GAP — OPEN — `E-M` | EVIDENCED — `<entry>` → `G 15527` / `W 0`; `0c561c66…`, `d30dc7c4…`; distinct nonzero sample `G 626 / W 0` on the gifted-account pack `41cdea4d…a0b1b2e` | NA-DESIGN — private readout | LIVE-FILLED — `C2S 5028` empty → `S2C 5029 body a73c000000000000` = 15527 / 0; tapped exe session `a97287d1…` (also `7d092b9d…` on the music leg) and the bootless Lane D probe `db1a395d…`. Distinct `7202000000000000` = 626 / 0 in `96bb62ce…25009b` and the gifted exe boot wire `94b2ea8f…a2508` breaks the all-zero cash-point ambiguity; #178/#179 share this pin once |
| **CART-TOGGLE** (self:local click ITEMBUY1 left half `(235,195)`) | NA-DESIGN — no goods offline | GAP — OPEN — `E-M` | EVIDENCED — `<single click on code 0x14200018 rect (220,187)-(301,204)>` → OS pointer and a white hover tooltip; no visible cart check on the four cards; `6a5407db…` vs baseline `d30dc7c4…` | NA-DESIGN — local cart state | GAP — click emitted no 5000/5002 frame in the tap; the visible 담기 press `(238,263)` on the gifted-account boot also painted no check and emitted no frame (`698f9ec8…a23a5f8de`, wire `94b2ea8f…a2508`); OPEN — `E-T` with a cart-state trace or REA of the cart writer |
| **CART-TOTAL** (self:checked cart) | NA-DESIGN — no cart offline | GAP — OPEN — `E-M` | GAP — no exe frame where `CHECKGEM`/`CHECKMONEY` inks a total; the `698f9ec8…a23a5f8de` press stays a negative pin (pointer only); OPEN — `E-T` with a successful cart pick | NA-DESIGN — private total | GAP — no price/verdict body pinned; live 5001 carries wallet words but no cart total field; OPEN — REA of the cart-to-total writer |
| **BROWSE-SCROLL** (self:local item scroll pager) | NA-DESIGN — no scroll offline | GAP — OPEN — `E-M` | EVIDENCED — `<single click on staged scroll right arrow (757,383)>` → four catalogue cards and the page readout change on the gifted-account boot; `099d1ee6…1fb98bcb` vs `698f9ec8…a23a5f8de`; no wire frame at this press | NA-DESIGN — local scroll state | NA-DESIGN — no wire field carries a local page index |
| **BAG-PAGER** (self:local BAGRIGHT `(410,576)`) | NA-DESIGN — no owned bag offline | GAP — OPEN — `E-M` | EVIDENCED — `<single click on code 0x1420004D rect (386,564)-(435,588)>` → owned-page readout `01/03 → 02/03` while the bag cells stay empty; `fa2b63f8…` vs `d30dc7c4…` | NA-DESIGN — private bag | NA-DESIGN — no wire frame for a local page turn |
| **FILTER-TABS** (self:local category/gender/submenu click) | NA-DESIGN — no tabs offline | GAP — OPEN — `E-M` | EVIDENCED — `<single click on IDC_BUTTON_SSHOP_FASHION rect centre (528,19)>` → selected category tab highlight changes on the gifted-account boot; `d4074ce7…87c6d11` vs `698f9ec8…a23a5f8de` | NA-DESIGN — local filter state | GAP — no 5002-family filter parameter pinned; the exe press emitted no wire frame in the tapped boot; OPEN — `E-T` with a filter-source REA |
| **SEARCH-ABSENT** (self:negative control walk) | NA-DESIGN — no offline board claim | GAP — OPEN — `E-M` | EVIDENCED — `<State_ItemShop.bnd + ControlList walk>` → no search control in this block; no search click exists; `0c561c66…` + static inventory (`docs/shop.md`) | NA-DESIGN — no board row | NA-DESIGN — negative design seam, not a wire story |
| **PREVIEW-TRYON** (self:local ITEMPRE1 `(328,192)`) | NA-DESIGN — no preview offline | GAP — OPEN — `E-M` | EVIDENCED — `<single click on code 0x14200064 rect (308,184)-(348,201)>` → white tooltip over the avatar pane; clothes unchanged; `d6625b12…`. No try-on art change proved | NA-DESIGN — local preview state | NA-DESIGN — preview sends no frame in any pin |
| **PREVIEW-REVERT** (self:local ITEMBACK `(151,252)`) | NA-DESIGN — no preview offline | GAP — OPEN — `E-M` | EVIDENCED — `<single click on code 0x1420006C rect (121,241)-(181,264)>` → tooltip cleared, avatar returns to its entry pose; `5c1e1fa7…` | NA-DESIGN — local preview state | NA-DESIGN — no wire frame |
| **AVATAR-PANE** (self:equipped worn kit) | NA-DESIGN — no character offline | GAP — OPEN — `E-M` | EVIDENCED — `<entry>` → equipped bald mannequin in the `IDC_IMAGE_SSHOP_AVATAR` pane; `0c561c66…`, `d30dc7c4…` (pose frames differ) | NA-DESIGN — private pane | NA-DESIGN — kit is local data |
| **BUY-COMMIT** (self:local BUYITEM after cart pick) | NA-DESIGN — no buy offline | GAP — OPEN — `E-M` | GAP — the pinned BUYITEM press `(41,574)` on the gifted-account boot emitted no 5000 and painted only the pointer (`b95bcf7f…d5c511`); no qualified exe buy frame; OPEN — `E-T` + REA of the BUYITEM gate | NA-DESIGN — local buy dialog | LIVE-FILLED — synthetic TCP client on live Memoryer: `C2S 5000` empty → `S2C 5001` 152 B; `506ac0a3…5d3b6` / `96bb62ce…25009b`. Static send file `0xf3fa8` is corroboration; the exe press still has to be wired. Shared with #180 E-W |
| **BUY-SUCCESS-5001** (self:server 5001 after buy) | NA-DESIGN — no buy offline | GAP — OPEN — `E-M` | GAP — no exe frame of gem update or item landing; OPEN — `E-W-5001` + paid web fixture | NA-DESIGN — private wallet/bag | LIVE-FILLED — live 5001 body `96bb62ce…25009b`: prefix `72020000000000000000000000000000` (gem 626 / point 0 / o2_cash 0 / cash_point 0), 30 bag words with 376/383/35, two post-list words, i32-counted attributive list, trailing free-pass word. Handler VA `0x560a40` / file `0x160a40` remains static corroboration; the word→cell paint stays OPEN. Scripted low-gem variant `f6f3177f…957e2` is SCRIPTED-GATEWAY, not a server verdict. Shared with #180 E-W |
| **BUY-INSUFFICIENT** (self:server failure verdict) | NA-DESIGN — no buy offline | GAP — no notice frame; prior low-balance premise refuted on the wire | GAP — no failure notice frame; the scripted gem-17 reply `f6f3177f…957e2` is the same 152-byte 5001 with no verdict word | NA-DESIGN — private notice | GAP — Memoryer has no 5001 failure field: the handler reads no verdict and the purchase is a web transaction (`docs/net-protocol.md` §4a). The only pinned failure shape in this family is `5003 len4 01000000` (`9c9af74a…50d67`), which is a sell verdict. OPEN — web-flow fixture (`gift_input.aspx` / `payment_input.aspx`) for the notice; not a 5001 word |
| **SELL-ITEM-5003** (self:local vacate/sell, server 5003) | NA-DESIGN — no item offline | GAP — OPEN — `E-M` | GAP — no exe before/after frame of a sell or gem refund; OPEN — `E-T` + `E-W-5003` | NA-DESIGN — private bag | LIVE-FILLED — live pair `C2S 5002 slot0/entry376` → `S2C 5003 0000000022390000` (refund 14626 = gem 626 + item 376 price 14000) in `96bb62ce…25009b`; genuine failure `C2S 5002 slot999` → `5003 len4 01000000` in `9c9af74a…50d67`; scripted failure `e82aeedd…1fed7` matches. Static send file `0xa4df0` index 0 / handler file `0x160c80` remain corroboration; exe before/after wallet frame stays OPEN. Shared with #180 E-W |
| **ITEMGIFT** (self:local ITEMGIFT / GIFTITEM) | NA-DESIGN — no gift offline | GAP — OPEN — `E-M` | EVIDENCED — `<PRESENT BOX tab (142,395) on the gifted account>` → one gift row paints in the lower MyRoom panel; `41cdea4d…a0b1b2e`. This is a claim-box row, not an ITEMGIFT send frame; no exe ITEMGIFT press exists yet — `E-G` | NA-DESIGN — private gift flow | LIVE-FILLED — live gateway `C2S 5017` → `S2C 5018 len6 hex=000000000000` (empty list, log `db1a395d…`) plus live `C2S 5023 type0/id1/resource16` → `S2C 5024 0000000000` (Success) with a seeded gift row (`96bb62ce…25009b`) and failure `5024 0000000004` (`9c9af74a…50d67`). Scripted non-empty 5018 (103 B) `58b30686…1d4ec` is SCRIPTED-GATEWAY: a real non-empty list needs a Memoryer model fix (fixed-size `UserMessages`). 5023 is **ClaimGift**, not an ITEMGIFT send; sending gifts to others is web-only (`gift_input.aspx`). Shared with #180 E-G |
| **CHARGE-PLATE-OPEN** (self:local click `(92,355)`, #362) | NA-DESIGN — no billing session offline | GAP — OPEN — `E-M` | EVIDENCED — `<charge plate click>` → `Cash Charge` chrome over the board: frame `(171,3)-(648,595)`, client `(190,58)-(630,564)`, close `(571,5)-(619,41)`; `27fd5435…`; our corrected root frame `d0402481…` has web-region MAE 0.667 (p362b live-path guard covers the click→paint chain) | NA-DESIGN — modal local sheet | NA-DESIGN — b8 `wire_tap=0`; the charge plate sends no shop frame in any tap |
| **CHARGE-X-CLOSE** (self:local close click) | NA-DESIGN — no sheet offline | GAP — OPEN — `E-M` | GAP — b8 opens the sheet but never clicks X; OPEN — `E-X` | NA-DESIGN — local modal | NA-DESIGN — no wire frame for the X press |
| **BACK-RETURN** (self:local BACK `(757,580)`) | NA-DESIGN — no lobby offline | GAP — OPEN — `E-M` | EVIDENCED — `<single click on BACK (757,580) from Item Shop>` → lobby/room frame; `ff522720…ef29b270` vs `b95bcf7f…d5c511` (boot 1, gifted account) | NA-DESIGN — local transition | LIVE-FILLED — the same tap emits `C2S 5007 ExitShop` empty at 20:35:18, immediately followed by `5028 → 5029 7202000000000000`; wire `94b2ea8f…a2508`. First live 5007 in any tapped shop session; shared with #179 E-B |
| **VISITOR-VIEW-OTHERS-SHOP** (other:master/peer) | NA-DESIGN — no session | NA-DESIGN — no visitor opcode; design seam is the local side screen | NA-DESIGN — same boundary; no observer can enter another player's shop | NA-DESIGN — no channel row state for a side screen | NA-DESIGN — no visitor request in the 5000 family; graded NA-DESIGN, never a passed story |

Counts: **120 cells** — 17 EVIDENCED, 35 GAP, 8 LIVE-FILLED, 0 OPEN-FIX,
60 NA-DESIGN. Every GAP names a step below. The 2026-11-19 SHOP-EVIDENCE
leg added 4 EVIDENCED (PRESENT BOX peer, filter, scroll, BACK) and 5
LIVE-FILLED (5001 catalog/buy, 5000 send, 5003 sell, 5007 BACK, expanded
wallet); master cells stayed GAP because no master shop frame exists. A cell is a story candidate, not
a closure; closing still needs the live caller chain and the project gates.

Next-step ids (shared with the #179 and #180 matrices):

- `E-M` — one legitimate master-session exe Item Shop boot after #379 handoff; role promotion is now pinned (`3007`), so the driver only needs the CREATE ROOM modal OK at `(400,460)` before the existing MyRoom→Item Shop walk; capture entry, goods art, prices and wallet against the peer frames.
- `E-C` — matched-catalog live A/B (same ids as b8, same `avatar.opa`) before any 1:1 ITEMSHOW hash claim (#363 residual).
- `E-W-5000/5001/5003` — scripted TCP proxy on 15012→15010 with a known goods/low-balance fixture; log C2S 5000/5002 bodies and S2C 5001/5003 bodies beside the painted card/wallet/tooltip frames.
- `E-W` — generic gift wire proxy for 5017/5018/5023/5024 (see `E-G`); 5001/5003 use the more specific ids above.
- `E-T` — qualified exe interaction sweep on a known-goods account: cart press, filter, scroll, BUYITEM, gift and X close, each with a frame and a tap.
- `E-G` — gifted-account exe boot done: present-box row `41cdea4d…a0b1b2e`, live 5023/5024 success/failure bodies. Still OPEN: exe 5017 send (no `push $0x1399` site found) and a real non-empty 5018 (Memoryer `UserMessages` fixed-size model). ITEMGIFT send to others is web-only; do not close it from a 5023 pin.
- `E-X` — Wine SendInput on the pinned charge-close `(590,23)` plus a REA trace of the close handler (#362 OPEN residual).
- `E-B` — DONE for Item Shop: BACK frame `ff522720…ef29b270` and `C2S 5007` wire `94b2ea8f…a2508` on one tap. Music Shop BACK frame still OPEN in #179; the opcode is shared.

## 3. Scenario scripts (bounded exe lane; one shared Memoryer)

Trace prefix `S178-ITEM`, spec `#178`, state `STATE_SHOP (0xA4)`. Boot pair:
`M = host bot uid 3 / room master`, `E = exe guest tester / local seat
holder` (the `peer` column); no channel-observer socket was attached.
Containment for every boot: private Xvfb `:421`, owned `rig_null_*` sink,
`route=sink` by pack audio check, `on_default_sink=0`, `preboot_breach=0`,
peak −0.0 dBFS, exe SHA pinned, server PID 92326 / epoch 1794986056
unchanged. The blue NORMAL-mode popup was captured (`message-box.png`) and
dismissed at `(400,365)` at every boot before the channel list; no popup
remained at Channel→Room→shop. All owned processes were stopped by their
recorded pids; the server was never restarted or signaled.

### S178-ITEM-01 (boot 2, `~/scratch/story-shops/item-interact/exe`)

Steps: seq 1 `E` enters Room (CP01 `room.png`); seq 2 `E` clicks Room MyRoom `(517,16)` twice (CP02 `myroom.png`); seq 3 `E` clicks MyRoom Item Shop `(89,320)` twice (CP03 `itemshop.png` `d30dc7c4…`); seq 4 `E` single-clicks ITEMBUY1 left half `(235,195)` (CP04 `item-cart.png` `6a5407db…`); seq 5 `E` clicks ITEMPRE1 `(328,192)` (CP05 `item-preview.png` `d6625b12…`); seq 6 `E` clicks ITEMBACK `(151,252)` (CP06 `item-revert.png` `5c1e1fa7…`); seq 7 `E` clicks BAGRIGHT `(410,576)` (CP07 `item-bag-page.png` `fa2b63f8…`). Wire tap id `a97287d1…` covers all checkpoints; no per-step opcode changed.

Outcome sentence: "When the seat holder single-clicks the pinned BAGRIGHT rect in the Item Shop, the owned-page readout moves from `01/03` to `02/03` while the bag cells stay empty (pack `fa2b63f89498c4755d2da9c2452b262b48dfee7125e9c3958623d731dceb3eec`)." For the cart cell the honest observation is negative: the same leg's `6a5407db…` frame shows the pointer and a tooltip, not a cart check; the positive cart story stays GAP.

### S178-ITEM-02 (b8 pack, cited exe, no wire tap)

Steps: seq 1 `E` enters Room; seq 2 MyRoom `(517,16)`; seq 3 Messenger `(644,580)` then OK `(400,368)`; seq 4 MyRoom Item Shop `(89,320)` (CP `itemshop.png` `0c561c66…`); seq 5 charge plate `(92,355)` (CP `cash-charge.png` `27fd5435…`). Fills entry, catalog art, prices, wallet, avatar pane and charge-chrome cells. Outcome sentence: "When the seat holder clicks the Item Shop charge plate at `(92,355)`, the exe paints the pinned `Cash Charge` frame and web client over the goods board (pack `27fd5435fbe64c96315fcb29f0947bfc02d794b318c8a3047dccf15d176be671`)."

### S178-ITEM-03 (SHOP-EVIDENCE boot 1, `~/scratch/evidence-shops/gifted-boot/exe`)

Trace prefix `S178-SHOP-EVIDENCE`, state `STATE_SHOP (0xA4)`. Guest `E` =
uid 2 (`tester2`, seeded gift-box row and wallet `G 626/W 0`), host uid 3.
Boot 1 steps: Room → MyRoom `(517,16)` → PRESENT BOX tab `(142,395)`
(CP `gift-tab.png` `41cdea4d…a0b1b2e`) → Item Shop plate `(89,320)` →
visible 담기 button `(238,263)` (`698f9ec8…a23a5f8de`) → FASHION tab
`(528,19)` (`d4074ce7…87c6d11`) → scroll right `(757,383)`
(`099d1ee6…1fb98bcb`) → BUYITEM `(41,574)` (`b95bcf7f…d5c511`) →
BACK `(757,580)` (`ff522720…ef29b270`). Scripted gateway log
`94b2ea8f…a2508` pins `5006`, two `5028/5029 7202000000000000`, and the
first tapped `C2S 5007` on the BACK press. Cart and BUYITEM presses were
negative (pointer only, no 5000). Outcome sentence: "When the gifted seat
holder presses Item Shop BACK at `(757,580)`, the exe paints the lobby and
emits the empty `5007 ExitShop` frame (`ff522720…ef29b270`, wire
`94b2ea8f…a2508`)."

Boots 2–6 of this leg attempted `E-M`; two pinned `S2C 3007 new_master=1`
but no master shop frame was reached (CREATE ROOM modal blocked both boots).
See `~/scratch/evidence-shops-findings.md` §Boots 2–6 for the full ledger.

### Shared wire leg S179-MUSIC-01 (boot 1, cross-graded)

The tapped music-shop boot (`~/scratch/story-shops/mshop-wire4/exe`, wire
`7d092b9d…`) grades the wallet source for both shop specs once: `5028/5029`
body `a73c0000 00000000` matches the b8 item-shop `G 15527` / `W 0` readout.
Neither leg produced 5001/5017/5018/5023/5024; those cells stay GAP in both
matrices and in `180-myroom-story-matrix.md`.

### Boot ledger

Three boots were launched (one successful pack, two label-mismatch packs),
one boot left unspent; four preflight attempts launched no client:

| Attempt | Display / sink | Actors | Scene | Tap | Result |
|---|---|---|---|---|---|
| 0a | none | none | none | none | `WAYLAND_DISPLAY=wayland-1` refused by `rig-run`, no client, no cell |
| 0b | none | none | none | none | `XDG_SESSION_TYPE=wayland` refused, no client, no cell |
| 0c | none | none | none | tap opened/stopped | relay port 39311 squatted, no client, no cell |
| 1 | `:421` / `rig_null_3584375` | host uid 3 + exe tester | Planet→Channel→Room→Music Shop | `7d092b9d…` | PASS; cross-graded wire |
| 2 | `:421` / owned `rig_null_*` | host uid 3 + exe tester | Planet→Channel→Room→MyRoom→Item Shop→4 interactions | `a97287d1…` | 10/10 distinct frames, cite PASS; pack status FAILED only on custom-target label mismatch |
| 3 | `:421` / owned `rig_null_*` | same | music interactions | `73b4d488…` | 8/8 distinct frames; same label mismatch; graded in #179 |
| 4 | — | — | — | — | unspent in that wave; the SHOP-EVIDENCE leg had its own six-boot budget |

SHOP-EVIDENCE leg boot ledger (six boots, one live lane, all six spent):

| Boot | Display / sink | Actors | Scene | Tap | Result |
|---|---|---|---|---|---|
| E1 | `:421` / owned `rig_null_*` | host uid 3 + exe uid 2 | MyRoom gift box → Item Shop interactions → BACK | `94b2ea8f…a2508` | 12/12 distinct frames; manifest/audio/cite PASS; FAILED only on custom target label |
| E2 | `:422` / owned `rig_null_*` | same | master attempt, no host bot | `20afbf7b…b444` | driver fault (missing `host.cmd`); no master pin |
| E3 | `:423` / owned `rig_null_*` | same | master attempt, room stage only | `ffe7f9fc…df3fb` | host left before guest join; `2007`, not `3007`; no master pin |
| E4 | `:424` / owned `rig_null_*` | same | master promotion + recovery | `b0c9cb5d…604ff1` | `3007 new_master=1` pinned; CREATE ROOM modal blocked all later clicks; 14 captures / 12 distinct |
| E5 | `:425` / owned `rig_null_*` | same | retry of E4 | `6a9dcd24…15f1168` | channel flake / frame-gating miss; stopped at planet |
| E6 | `:426` / owned `rig_null_*` | same | retry of E4 | `2ca2c32c…360e999` | `3007` pinned again; modal OK aim 52 px low, no master shop frame; 15 captures / 14 distinct |

## 4. Cross-cell contradictions and limits

- **Pack label mismatches:** boots 2 and 3 drove every checkpoint and their
  `cite --claim exe` check PASSED, but the runner compared the final driver
  label (`item-bag-page` / `music-top100`) with the custom scenario id
  (`itemshop-interact` / `musicshop-interact`) and wrote FAILED. Same class as
  the MyRoom boot-1 `myroom-return` mismatch; the frames stand, the status
  is recorded, not repaired.
- **Manifest hash drift:** citation text names an internal manifest hash
  that differs from the current `manifest.json` file hash in b6, b8 and every
  new pack (b8 `f9d4a695…` vs `7cb327c1…`; boot 1 `0d0ca43a…` vs
  `e18daa82…`). Every cited frame hash agrees between citation, provenance
  and direct `sha256sum`; frame pins stand and the drift is named.
- **Cart click ambiguity:** the `.bnd` ITEMBUY1 rect `(220,187)-(301,204)`
  is inside the animated portrait; the visible `담기`/`넣기` buttons sit lower
  in the frame. The click produced a tooltip and no check mark, so the cell
  records that negative, not a positive cart story. The rect-to-visible-
  control join is OPEN with next step `E-T`.
- **Preview:** ITEMPRE1's tooltip and unchanged clothes contradict any claim
  of exe try-on art; #196's ours-by-design try-on cannot be called 1:1. The
  revert frame proves the tooltip cleared, not a clothing revert.
- **Bag pager:** the page readout changes on an empty bag; no item rows were
  present, so the story pins the readout, not an item-turn rule.
- **Wire absence is a pin, not a success:** three tapped boots reached MyRoom
  and both shops with no 5001/5003/5017/5018/5023/5024 frame. The live
  bootless probe added the Memoryer 5018 empty-list body (`db1a395d…`), not
  an exe frame. Exe reply shapes stay OPEN; our client's scripted 5001/5024
  helpers are not exe evidence. Shared Memoryer state is not a scenario delta
  without these logs.
- **Fix ledger (explicit, not invented pixels):** #363 ITEMSHOW art → row 3;
  #369 panel faces + live-4 gesture → MyRoom entry row 1 (cross-state);
  #362 charge chrome + p362b live-path guard → row 21; #305 blue bag
  selection and #375 hands belong to #180; #368/#364/#361/#370/#376/#379/
  #380/#348 belong to their own states. None is silently mapped to a new
  Item Shop pixel.
- **Master column:** no qualified master-session shop frame exists; every
  master cell stays GAP even though these screens are local. A synthetic
  master-flag rewrite would be a scenario delta, not an exe observation.
  The SHOP-EVIDENCE leg did legally pin the promotion (`3007 new_master=1`,
  wire `b0c9cb5d…604ff1` / `2ca2c32c…360e999`), but both boots then stopped
  at a CREATE ROOM modal. The remaining step is one boot with the modal OK
  at `(400,460)`, not a role-flag guess.
- **Wire evidence class:** the live 5000/5001/5002/5003/5023/5024 bodies
  came from a synthetic TCP client against the unmodified Memoryer, so they
  grade the `server` column as LIVE-FILLED. They do not pin an exe press or
  a painted cell; `SCRIPTED-GATEWAY` rewrites (low-gem 5001, non-empty 5018)
  are named separately and never count as server or exe observations.

## 5. Gap-to-story hand-off

Ranking by player visibility × reachability:

1. Buy round trip and wallet refresh (`E-W-5000/5001`): 8 GAP cells including
   the 5001 reply shared with #180; one proxy leg with a known-goods and a
   low-balance fixture covers success and failure arms.
2. Master-session entry/board (`E-M`): 20 GAP cells; one legitimate master
   boot after #379 handoff covers most of them.
3. Gift wire family (`E-G`, 5017/5018/5023/5024): Lane D pinned the empty
   5018 list from a live bootless probe; the 4 GAP cells here plus the MyRoom
   GIFT-CLAIM/INFO cells still need a gifted-account tapped boot for 5023/5024
   and the exe landing frame.
4. Sell 5003 (`E-W-5003`) and interaction sweep (`E-T`): 7 GAP cells; pair a
   tap with a known-goods fixture.
5. BACK/ExitShop (`E-B`) and charge X (`E-X`): 4 GAP cells; cheap click legs.

The new story rows were added once to the #178 body under
`## Multi-actor stories` with evidence class and verify step; the existing
checklist stays as it is. Closing any story still needs the live caller
chain and the project gates. A conformance mismatch becomes a bug, never a
story edit.

# Music Shop story matrix — Spec #179 (WAVE 4, lane shops)

Method: `docs/captures/story-matrix/TEMPLATE.md`; cell shape follows the
Waiting (#291), Channel (#289), Room (#290) and MyRoom (#180) exemplars. Base
`878f102`; worktree `/home/arnim/Projects/recomp2jam-wt140`, branch
`cb2/story-shops`. Docs and tracker stories only: no code changes, no pushes,
no git remote config edits. The 7.55 executable is the oracle (`OTwo.exe`,
5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`).
Memoryer is a wire witness. Scratch packs stay local; only hashes and text
enter the repo.

## 1. Evidence inventory (written before any cell was graded)

| Source | Pin (full sha256 unless noted) | What it names / verdict |
|---|---|---|
| exe identity | `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b` | 7.55 oracle; static offsets and rects below |
| `~/scratch/story-shops/mshop-wire4/exe` (this wave, PASS, cited exe pack) | manifest file hash `e18daa82896a12e129765be3894e43f2de7201c4e8e53e5a35cc602d64a54d95` (citation quotes internal `0d0ca43a…2b51d97`); citation `60f4e2149cef1451d34fd5efad7712a48d034584e19af7fdcb945f8f5f8b5a9b`; `musicshop.png` `964be986ee1fa269306ad2115dfdb18efc1cc511e5dfa765e5845698bb8ee8aa`; wire `7d092b9d8616d729297b567d440ed4b37b56fc2b9c27feeb3154cd4349241a5d`; drive `c8fe7aeefd190dce3988985c150c62df7b32f7f891f025cf405a59e61926a46f` | First qualified exe Music Shop pack. Byte-identical entry frame to the old unqualified b1-explore scratch PNG; now admissible with manifest + citation. Room Music Shop plate `(350,16)` double-click → `C2S 5006`; `5028/5029` wallet pin. |
| `~/scratch/story-shops/music-interact/exe` (this wave; frames valid, cite PASS, pack label mismatch) | `musicshop.png` `326ccb90dc349cceebe8989e674a98b43c1572ad3b2bb663dbf3aa090f7d3eec`; `music-sort.png` `eb75da8423743004a8d1b917fb2c6fd1c81c87fa626694a9822ca9a43cfb39ee`; `music-page.png` `4debf1f0a3907b6551b2247b3bc60d92c99562fbd8b1888aaf695d89e06ef8f7`; `music-top100.png` `5ddfd9d372e0b978f25e2bca1136e5e0ce3aceab2e338ba18a08149a095f94f3`; wire `73b4d4882c284621ce2855650e9dcc711dc670452515bdd6d62ac892237213e1`; manifest `940b9b76d7bb0e13b8e6cc16a013d64a08d5d7b2780833214f77a9cec863ac91`; citation `5dc1de65d0e191b0fe5a6f55cbc7b04ded788f1f1691ab6d7876c7a6bdab838f` | 8/8 distinct frames, cite PASS; custom target id `musicshop-interact` is not a scene label so the runner wrote FAILED although all checkpoints were reached (§4). Entry promo panel `01/07`, right rank `01/47`, wallet `G15527/W0`; left pager click paints five catalog rows with DIFF `3/10`, prices `G 300` / `W 300`, `Free`/`구매` and a `01/96` readout; right-panel title-sort click produces no reorder (already active); TOP100 click moves only the pointer plus a web-banner caption. |
| `~/scratch/side-scenes/b1-explore/explore/{d03,h04}-musicshop.png` (negative / superseded) | `1f4c420767a86c0db4f26175beb268b8b9517461db5e411c38308b54c4ee4d76` / `964be986ee1fa269306ad2115dfdb18efc1cc511e5dfa765e5845698bb8ee8aa` | Old scratch pixels had no manifest/citation and could not grade a cell. The h04 hash is now reproduced exactly by the qualified boot-1 pack; cite the pack, not the scratch path. |
| `~/scratch/side-scenes/b8-cash` (cited exe pack, cross-spec) | manifest citation `f9d4a6953c55b75b4f04db3c0ece198bd2032361ac65dc8275ce07ccb6aa3f8b`; `cash-charge.png` `27fd5435fbe64c96315fcb29f0947bfc02d794b318c8a3047dccf15d176be671` | Charge chrome belongs to the Item Shop plate; #362's rect pin and p362b's live-path guard are cross-state stories here, not invented Music Shop pixels. |
| #362 / p362b (CLOSED `905ef9c`, `776cfe7`) | corrected Rust root frame `d04024813b799dbcde6c8c3c10c442d9b706ba86e1461f963ef480541389fbc9`; web-region MAE 0.667 vs b8 | `live_boot_charge_plate_snapshot_click_paints_chrome` covers click→Payment→painter; old live-4 negative was a capture artifact. Music Shop's CASHFILLUP control is a separate OPEN arm. |
| #363 / #369 / live-4 gesture (cross-state fix ledger) | #363 `007c38c` dossier `ev_0df48…fd20d`; #369 `007c38c`, live-4 citation `a0e0fd08ef2d917d36be9e590dabce7ebebb1ca69b4214cfb3be87dba633c4a1`; STATUS `b095a915…`, return `c838e437…` | Item art and MyRoom panel faces are explicit cells in #178 / #180; they explain the Room→side-screen entry path but pin no Music Shop pixel. |
| `docs/musicshop.md`, `docs/parity/musicshop.md`, `docs/shop-buy.md`, `docs/net-protocol.md`, journey audit `179-music-shop.md` | exe hash above; 138 `State_MShop.bnd` rects; sort codes `0x1A201010..15`, right-rank sorts `0x1A200016..18`, TOP100 `0x1A20007D..84` (#181), 5010 send switch file `0xa4df0` index 8, opcode 5012 file/offset pin, 5011 untraced | Static pins only. 5011/5012 reply shapes and 5010 live ride stay OPEN; no guard counts as exe evidence. |
| MyRoom wave-3 gap list (`docs/captures/story-matrix/180-myroom-story-matrix.md`) | E-W/E-G: 5001/5003/5017/5018/5023/5024 | Cross-reference: shared wire family graded once with the #178 matrix; both legs tapped with no such frame. |

## 2. Matrix (21 actions × 5 observer actors = 105 cells)

Cell key: `EVIDENCED` / `GAP` / `LIVE-FILLED` / `OPEN-FIX` / `NA-DESIGN`.
The tapped exe guest is the local `peer` column; `master` is a master-session
observation that does not exist yet. Every GAP names a next step below.

| Action (raised by) | solo | master | peer | channel | server |
|---|---|---|---|---|---|
| **ENTER-MUSICSHOP** (self:seat holder from Room) | NA-DESIGN — offline boot has no gateway side screen | GAP — no qualified master-session Music Shop frame; OPEN — `E-M` | EVIDENCED — `<Room MUSIC SHOP plate (350,16), two clicks>` → music-shop sheet over the canvas: genre tabs, promo panel, right rank panel, download panel; `964be986…` (boot 1), `326ccb90…` (boot 3) | NA-DESIGN — board observer sees no side screen; design seam is `visit_side(MusicShop)` | LIVE-FILLED — `C2S 5006` empty on entry; tap `7d092b9d…` / `73b4d488…` (no 5011 reply) |
| **SONG-BOARD** (self:local OJNList catalog) | NA-DESIGN — no catalog offline | GAP — OPEN — `E-M` | EVIDENCED — `<left pager advance>` → five rows with album art, title/artist/edit/genre/BPM cells and per-row buy controls; `4debf1f0…` | NA-DESIGN — private board | GAP — no live catalog/ownership join beyond 4031; OPEN — `E-W-5011` |
| **PRICE-WORDS** (self:server price words) | NA-DESIGN — no prices offline | GAP — OPEN — `E-M` | EVIDENCED — `<catalog rows>` → per-row `가격: G 300` / `W 300` with green gem and gold won glyphs; `4debf1f0…`; static rects `0x40`/`0x45`, money texts `0x1A400019..021` | NA-DESIGN — private price cell | GAP — 5011 price words untraced on both taps; OPEN — `E-W-5011` |
| **DIFF-VIEW** (self:staged song level) | NA-DESIGN — no song offline | GAP — OPEN — `E-M` | EVIDENCED — `<catalog rows>` → per-row `난이도: 3/10` (other rows 5/10 etc.) in the DIFF cell; `4debf1f0…`; static cell `0x3C` | NA-DESIGN — private DIFF cell | GAP — no live level→cell wire pin; OPEN — `E-W-5011` |
| **OWNED-FLAGS** (self:server 4031 flags) | NA-DESIGN — no ownership offline | GAP — OPEN — `E-M` | EVIDENCED — `<catalog rows>` → `Free` and red `구매` words plus a checked row marker on the right rank list; `4debf1f0…` | NA-DESIGN — private flags | LIVE-FILLED — `4031` body 4499 B on channel enter, tap `7d092b9d…` / `73b4d488…`; body-to-painted-row join still OPEN (`E-W-5011`) |
| **ROW-PICK-CART** (self:local MS_EX row click) | NA-DESIGN — no rows offline | GAP — OPEN — `E-M` | GAP — no qualified exe row-pick/cart frame; both taps stopped at pager and header controls; OPEN — `E-T` | NA-DESIGN — local cart state | GAP — no 5010 C2S frame on the row-pick path; OPEN — `E-T` with tap |
| **CART-BUY-5010** (self:local BUYITEM) | NA-DESIGN — no buy offline | GAP — OPEN — `E-M` | GAP — pinned cart BUYITEM rect `(423,528)-(472,551)` has no visible button in the captured exe frame; no press frame; OPEN — `E-T` + `E-W-5010` | NA-DESIGN — local buy state | GAP — 5010 empty send is static (file `0xa4df0` index 8), no live C2S 5010; OPEN — `E-W-5010` |
| **BUY-COMPLETE-5011** (self:server 5011) | NA-DESIGN — no buy offline | GAP — OPEN — `E-M` | GAP — no exe owned-state change after a buy; OPEN — `E-W-5011` with a low-price fixture | NA-DESIGN — private owned state | GAP — 5011 reply untraced in exe and server; OPEN — `E-W-5011` |
| **BUY-INSUFFICIENT** (self:server failure verdict) | NA-DESIGN — no buy offline | GAP — OPEN — `E-M` | GAP — no failure notice frame; OPEN — `E-W-5011` with a low-balance fixture | NA-DESIGN — private notice | GAP — no failure body pinned; OPEN — `E-W-5011` |
| **SORT-RANK** (self:click right-panel 곡명 `(463,71)`) | NA-DESIGN — no rank offline | GAP — OPEN — `E-M` | EVIDENCED — `<single click on code 0x1A200016 rect (438,61)-(489,82)>` → the right list order and `01/47` readout stay unchanged (title sort was already active); only the cursor/tooltip moves; `eb75da84…` | NA-DESIGN — local sort state | NA-DESIGN — no wire field for a local sort press |
| **SORT-BOARD** (self:local song-board sort) | NA-DESIGN — no board offline | GAP — OPEN — `E-M` | GAP — genre tab strip is visible, but no sort-direction press frame; our client's ascending directions stay ASSUMED; OPEN — `E-T` | NA-DESIGN — local sort state | GAP — no sort parameter wire pin; OPEN — `E-T` with tap |
| **PAGER-LEFT** (self:local MSALL right `(340,542)`) | NA-DESIGN — no pages offline | GAP — OPEN — `E-M` | EVIDENCED — `<single click on code 0x1A20000B rect (317,532)-(363,553)>` → left pane changes from promo card to five song rows and readout `01/07 → 01/96`; `4debf1f0…` vs `326ccb90…` | NA-DESIGN — local page state | NA-DESIGN — no wire frame for a local page turn |
| **PAGER-RIGHT** (self:local rank pager) | NA-DESIGN — no rank offline | GAP — OPEN — `E-M` | GAP — right rank readout stays `01/47` on all captured frames; no press frame; OPEN — `E-T` | NA-DESIGN — local page state | NA-DESIGN — no wire field |
| **TOP100-BOARD** (self:local TOP100 `(297,18)`) | NA-DESIGN — no board offline | GAP — OPEN — `E-M` | EVIDENCED — `<single click on code 0x1A20007D rect (248,0)-(347,37)>` → pointer moves and a web-banner caption changes; no ranking board appears in the Music Shop pane; `5ddfd9d3…` | NA-DESIGN — local board state | GAP — no 5011/TOP100 wire response on this leg; OPEN — `E-W-5011` |
| **SEARCH** (self:local search edit/button) | NA-DESIGN — no catalog offline | GAP — OPEN — `E-M` | GAP — the search field and button are visible at `(21,534)-(184,553)` but no exe search result frame was captured; OPEN — `E-T` | NA-DESIGN — local search state | NA-DESIGN — no search wire pin |
| **DOWNLOAD-PANEL** (self:local download sheet) | NA-DESIGN — no download offline | GAP — OPEN — `E-M` | EVIDENCED — `<entry>` → lower-right download sheet with current/total speed and remaining-time cells (idle `KB/s` and `남은시간`); `964be986…`, `4debf1f0…`; static codes `0x1A20000E..0F`, texts `0x1A400019..021` | NA-DESIGN — private sheet | NA-DESIGN — panel presence is staged art, not a wire field |
| **DOWNLOAD-5012** (self:server 5012 completion) | NA-DESIGN — no FTP flow offline | GAP — OPEN — `E-M` | GAP — no exe 5012 request/notice frame from the shop; the post-start FTP pump is a different state; OPEN — `E-W-5012` | NA-DESIGN — private download state | GAP — opcode 5012 is static only; no live body; OPEN — `E-W-5012` |
| **CASH-FILLUP** (self:local CASHFILLUP control) | NA-DESIGN — no billing session offline | GAP — OPEN — `E-M` | GAP — no qualified Music Shop cash-fillup press frame. Cross-state: #362's Item Shop charge plate and p362b's `live_boot_charge_plate_snapshot_click_paints_chrome` are the pinned charge path, not a Music Shop pixel; OPEN — `E-T` | NA-DESIGN — local billing sheet | NA-DESIGN — no wire frame for the fillup click |
| **BACK-RETURN** (self:local BACK `(757,580)`) | NA-DESIGN — no lobby offline | GAP — OPEN — `E-M` | GAP — no exe Music Shop BACK frame (boot 1 and 3 end in the shop); OPEN — `E-B` | NA-DESIGN — local transition | GAP — 5007 ExitShop not seen in either tap; OPEN — `E-B` with tap |
| **BUY-TO-SELECT** (self:server-owned song after buy) | NA-DESIGN — no select offline | GAP — OPEN — `E-M` | GAP — no exe buy→song-select capture; our scripted state join is not an exe pin; OPEN — `E-W-5011` + select capture | NA-DESIGN — private song list | GAP — no ownership broadcast tied to a shop buy; OPEN — `E-W-5011` |
| **VISITOR-VIEW-OTHERS-SHOP** (other:master/peer) | NA-DESIGN — no session | NA-DESIGN — no visitor opcode; design seam is the local side screen | NA-DESIGN — same boundary; no observer enters another player's shop | NA-DESIGN — no channel row for a side screen | NA-DESIGN — no visitor request in the 5000 family; graded NA-DESIGN, never a passed story |

Counts: **105 cells** — 9 EVIDENCED, 43 GAP, 2 LIVE-FILLED, 0 OPEN-FIX,
51 NA-DESIGN. Every GAP names a step below. A cell is a story candidate, not
a closure.

Next-step ids (shared with #178 and #180 matrices):

- `E-M` — one legitimate master-session exe Music Shop boot after #379 handoff.
- `E-W-5010/5011/5012` — scripted TCP proxy on 15012→15010 with a known
  catalog/balance fixture; log C2S 5010/5012 and S2C 5011 bodies beside the
  per-row price/DIFF/owned frames.
- `E-W` — generic gift-family proxy for 5001/5003/5017/5018/5023/5024,
  graded once with the #178 and #180 matrices.
- `E-T` — qualified exe interaction sweep on a known catalog account: row
  pick, cart buy, board sort, right pager, search, cash-fillup; one frame and
  one wire sample per checkpoint.
- `E-B` — qualified exe BACK click from Music Shop with a tap; pin 5007.

## 3. Scenario scripts (bounded exe lane; one shared Memoryer)

Trace prefix `S179-MUSIC`, spec `#179`, state `STATE_MUSICSHOP (0x6B)`.
Boot pair `M = host bot uid 3`, `E = exe guest tester` (local `peer` column).
Containment: private Xvfb `:421`, owned null sink, `route=sink`,
`on_default_sink=0`, `preboot_breach=0`, peak −0.0 dBFS, exe SHA pinned,
server PID 92326 / epoch 1794986056 unchanged. The blue NORMAL-mode popup was
captured (`message-box.png`) and dismissed at `(400,365)` on both boots before
Channel; no popup remained at Channel→Room→shop. Owned pids only; no server
restart or signal.

### S179-MUSIC-01 (boot 1, `~/scratch/story-shops/mshop-wire4/exe`)

Steps: seq 1 `E` enters Room (CP01 `room.png`); seq 2 `E` double-clicks Room Music Shop `(350,16)` (CP02 `musicshop.png` `964be986…`). Wire id `7d092b9d…`: `5006` on entry, `5028/5029`, `4031`. Checkpoint fills ENTER × peer/server, DOWNLOAD-PANEL × peer, OWNED-FLAGS × server. Outcome sentence: "When the seat holder clicks the Room Music Shop plate, the exe paints the music-shop sheet with the promo card, rank panel and download panel (pack `964be986ee1fa269306ad2115dfdb18efc1cc511e5dfa765e5845698bb8ee8aa`, wire `7d092b9d8616d729297b567d440ed4b37b56fc2b9c27feeb3154cd4349241a5d`)."

### S179-MUSIC-02 (boot 3, `~/scratch/story-shops/music-interact/exe`)

Steps: seq 1 entry (CP11 `326ccb90…`); seq 2 `E` single-clicks right-panel 곡명 `(463,71)` (CP12 `music-sort.png` `eb75da84…`); seq 3 `E` single-clicks MSALL right `(340,542)` (CP13 `music-page.png` `4debf1f0…`); seq 4 `E` single-clicks TOP100 `(297,18)` (CP14 `music-top100.png` `5ddfd9d3…`). Wire id `73b4d488…`: `5006`, `5028/5029`, `4031`, `6001`; no opcode changed at steps 2–4.

Outcome sentence: "When the seat holder clicks the pinned MSALL right rect in the Music Shop, the exe replaces the promo card with five catalog rows that show DIFF, gem/won prices and ownership words, and the left readout changes `01/07 → 01/96` (pack `4debf1f0a3907b6551b2247b3bc60d92c99562fbd8b1888aaf695d89e06ef8f7`)." The TOP100 click is recorded as a negative observation; no board story is claimed.

### Shared wallet wire (cross-graded with #178)

The same tapped session's `5028/5029` body `a73c000000000000` matches the
b8 item-shop wallet `G 15527` / `W 0`; that wire cell is graded once and
cross-referenced in `178-item-shop-story-matrix.md`.

### Boot ledger

Two boots launched for this spec (both produced valid frames); two more
boots were launched for #178 and cross-graded where noted. Four
preflight-only attempts (#178 ledger) launched no client. Boot 4 of the
program's four-boot budget is unspent.

| Boot | Display / sink | Actors | Scene | Tap | Result |
|---|---|---|---|---|---|
| 1 | `:421` / `rig_null_3584375` | host uid 3 + exe tester | Planet→Channel→Room→Music Shop | `7d092b9d…` | PASS; 5/5 distinct frames, cite PASS |
| 2 (#178) | `:421` / owned `rig_null_*` | same | Item Shop interactions | `a97287d1…` | frames valid; label mismatch |
| 3 | `:421` / owned `rig_null_*` | same | Music Shop interactions | `73b4d488…` | 8/8 distinct frames, cite PASS; label mismatch |
| 4 | — | — | — | — | unspent; needs fixture/proxy, not pixels |

## 4. Cross-cell contradictions and limits

- **Old scratch upgraded, not double-cited:** `h04-musicshop-long.png`
  `964be986…` was unqualified in the journey audit. Boot 1 reproduced the
  exact bytes with a manifest and citation, so all cells cite the new pack;
  the scratch path survives only in the inventory as a negative.
- **Pack label mismatch:** boot 3 reached every checkpoint with distinct
  frames and `cite --claim exe` PASS; the runner compared the custom scenario
  id with the final scene label and wrote FAILED. Recorded, not repaired.
- **Manifest hash drift:** boot 1's citation quotes `0d0ca43a…` while
  `manifest.json` hashes to `e18daa82…`; frame hashes agree with provenance
  and direct `sha256sum`. Same class as b6/b8 and every other wave's packs.
- **Sort click:** the bnd rect `0x1A200016` is the right-panel 곡명 rank sort,
  not the song-board sort. It was already active, so no reorder is evidence;
  the board-sort directions stay ASSUMED with `E-T` as next step. This is a
  naming trap from `ControlList`, not a painter claim.
- **Pager click:** the left readout changed from `01/07` (promo set) to
  `01/96` (catalog pages) while the current-page digits stayed `01`. The
  observed pixel change is pinned; the rule that maps the two counters is
  OPEN with `E-T`.
- **TOP100:** #181 pins the control rect, not the board contents. The click
  changed only the pointer and a web caption, so the TOP100 board story stays
  GAP despite the control's existence.
- **Price/DIFF/owned:** the qualified catalog frame shows the words, but the
  field source is not pinned. Green shell guards and the 4031 body alone do
  not close the mapping; `E-W-5011` pairs wire words with painted cells.
- **Cross-state fixes:** #363 item art and #369 panel faces are explicit cells
  in #178 / #180; live-4's gesture pins the MyRoom panel; #362 charge chrome
  and the p362b live-path guard are named in the CASH-FILLUP row. None is
  silently mapped to a Music Shop pixel.
- **Wire absence:** neither tapped boot produced 5001/5010/5011/5012/
  5017/5018/5023/5024. Our scripted 5010/5011 helpers are assumptions, not
  exe behavior. Shared Memoryer state is not a scenario delta without a tap.

## 5. Gap-to-story hand-off

Ranking by player visibility × reachability:

1. Price/DIFF/owned field sources and buy completion (`E-W-5010/5011`): 12
   GAP cells; one proxy leg with a known catalog and balance fixtures.
2. Master-session board (`E-M`): 18 GAP cells; one legitimate master boot.
3. Interaction sweep (`E-T`): row pick, board sort, right pager, search,
   cash-fillup and the buy control: 9 GAP cells.
4. BACK/ExitShop (`E-B`) and 5012 download (`E-W-5012`): 5 GAP cells.
5. TOP100 board contents: 1 GAP cell; needs the ranking wire, not just the
   #181 control pin.

The new story rows were added once to the #179 body under
`## Multi-actor stories`; the existing checklist is untouched. Closing any
story still needs the live caller chain and the project gates.

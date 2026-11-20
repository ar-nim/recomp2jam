# Item Shop A/B parity audit — findings (WORK IN PROGRESS)

Status: started. Main at 7473eef. Audit lane: no repo code changes.

## Progress log

- [ ] Worktree setup
- [ ] Rig lock / boot window 1
- [ ] Step 1 entry pair
- [ ] Step 2 shelf survey
- [ ] Step 3 chrome survey
- [ ] Step 4 interaction sweep
- [ ] Step 5 hover sweep (if windows remain)
- [ ] Final report, #178 ticket cross-reference

## Delta table

(step | exe | rust | frame hashes | layer | rank)

| Step | What exe shows | What rust shows | Paired hashes | Suspected layer | Rank |
|---|---|---|---|---|---|

## Setup / method (19:05 local)

- Worktree actually used: `/home/arnim/Projects/recomp2jam-wt180-shopab` at
  `7473eef`, branch `cb2/shop-ab-audit`, private data pool
  `/home/arnim/scratch/install-data-shopab/slot1` (wt175 is occupied by
  `cb2/opt-sliders`; creating our own worktree avoids stealing another lane's
  data and branch).
- Lane: acquired `shop-ab:exeN/rust1` around each boot via
  `scripts/lib/rig-live-lock.sh`; no Memoryer restart
  (pid 92326, boot epoch 1794986056). Wire tap `[u16 len][u16 id][body]`
  in front of the one gateway; both clients use user 2 (gem 626).
- Packs: `~/scratch/shop-ab/01-entry/exe`, `02-sweep/exe`, `03-sweep/rust`.
  Drivers are untracked scratch copies in the worktree (`wine-scene-shopab.py`,
  `rig-wine-run-shopab.sh`); repo code is unchanged.
- BND walk (read-only): `State_ItemShop.bnd` 186 rects; ControlList codes
  paired by `(code & 0xFFF)-1`. Table saved at
  `~/scratch/shop-ab/controls.txt`. Overlay of raw rects on the qualified
  exe entry frame `0c561c66…` at `~/scratch/shop-ab/exe-rect-overlay.png`
  shows a consistent **+70 px child origin** for ITEMWON/ITEMGEM/ITEMBUY/
  ITEMGIFT/ITEMPRE (their raw rects point at the figure, not the price and
  button row). Our `shop_action_at` and text painters use raw rects
  (`crates/o2jam-shell/src/screens/side.rs:688`, `paint.rs:1552`).
- Boot 1 (exe, `:181`, tap 15018): FAILED — 03EA arrived (server log
  19:14:12) but the first grab still showed Planet; channel paint-wait
  missing in driver. 2 distinct frames kept, audio `route=sink`,
  `preboot_breach=0`, `on_default_sink=0`, cite PASS. Fixed in driver
  (bounded paint retry), not in the client.
- Boot 2 (exe, `:182`, tap 15019, target `itemshop`) — PASS pack, cite PASS,
  audio `route=sink`, peak 0.0 dBFS, `preboot_breach=0`, `on_default_sink=0`.
  Entry frame `02-sweep/exe/artifacts/screenshots/itemshop.png` sha256
  `3ad747022122d53cb4fa8989f699bfb4865cda8a101dbf12fe9ff44ed65b873c`:
  four goods cards with figures, won `4500`, gem `0`, wallet `G 626 / W 0`,
  three owned-item icons in MY BAG. Wire pin: `5006 EnterShop` at 19:21:15,
  `5028 → 5029 7202000000000000` (gem 626 / cash 0), `07E8` badge, `6001`.
  This boot only walked entry: the sweep hook needed `--target shopab`, fixed
  for boot 3. Boot ledger so far: 1 channel paint flake (driver), 1 entry only.

## Step 1 — Entry (exe side)

| Step | Exe | Frame |
|---|---|---|
| Room → MyRoom → Item Shop | 4 goods cards with figures, prices 4500/0, wallet G626/W0, MY BAG icons | `3ad74702…` (qualified pack `02-sweep/exe`) |

Rust entry frame pending boot 4.

## Boot ledger (4 windows, complete)

| Window | Pack | Result | Ledger mode |
|---|---|---|---|
| 1 | `01-entry/exe` | 03EA arrived; first grab still Planet; 2 distinct frames, cite/audio PASS | channel flake / frame-gating miss (driver; paint retry added) |
| 2 | `02-sweep/exe` | qualified exe entry pack, PASS, 6/6 distinct, cite/audio PASS | entry only: sweep hook needed `--target shopab` (driver/setup fault) |
| 3 | `04-sweep/exe` | 41/41 distinct, cite PASS, audio `route=sink`, peak 0.0 dBFS, `preboot_breach=0`, `on_default_sink=0`; status FAILED **only** because custom final label `28-back` != declared target `shopab` | valid exe A/B frames; gift modal blocked steps 04-28 |
| 4 | `03-sweep/rust` and `rust-attempt0` | attempt0: duplicate `--port` in play-local argv, no client boot (driver/setup fault). Retry: client booted to Planet, wire 1000/1001/1002/1003 seen (`G 626` server), but `replay-trace-native.py` timed out finding the 800x600 window on Xvfb :183; 1 Planet frame only | no shop frame; driver/window-discovery fault |

Memoryer pid 92326 / epoch 1794986056 unchanged. No restarts, no remote
writes, no tracker edits. Rust boot 4 audio `route=unseen`,
`preboot_breach=0`, `on_default_sink=0`, peak −2.4 dBFS (the route guard
recorded no bound stream; level proves the tap, not the route).

## Rig A/B evidence classes

- EXE-LIVE: `04-sweep/exe` (citable, manifest `artifacts/screenshots` 41
  distinct, citation PASS) and entry `02-sweep/exe`.
- RUST-LIVE-ATTEMPT: `03-sweep/rust` reached Planet only; no Shop frame.
- RUST-STATIC-SEAM: `rust-static/shop-native.png` sha256
  `5d97d52ff4ba6504366e8d495dcbf9e7da6c2ded02858c2510b0ad68253bcec7`,
  produced by `target/debug/recomp2jam shots --out-dir` at HEAD 7473eef with
  this worktree's install-root. This is the **current code's** Shop paint, but
  it is an in-process snapshot with synthetic wallet/catalog, not a live
  paired frame. Every rust-side pixel claim below is labeled SEAM and needs
  live confirmation in wave 2.

## DELTA TABLE (player-facing rank first)

| # | Step | What the exe shows (EXE-LIVE) | What rust shows | Paired hashes (exe / rust) | Suspected layer | Rank | Tickets |
|---|---|---|---|---|---|---|---|
| D1 | 1/2 entry shelf | Four goods cards with per-item figures at ITEMSHOW1..8, name strips, won `4500` / gem `0` at the *card-child* price row (BND y 169/322 + 70 px), and wallet `G 626 / W 0` | SEAM: four flat grey placeholder cards, no figures; price text at BND y169/322 (on the figure), wallet synthetic `GEM 1200 / W 02 CASH 2200`; LIVE attempt never reached Shop | `05b24e64…cb00` / `5d97d52f…bcec7` (seam, not live) | staging rect + plate readout | P0 | #395, #388 |
| D2 | 3 hit-test row pick | Pressing visible 담기 at `(238,263)` opens SHOPPING BAG with a 4500 row and sets cart total `W 4500`; no 5000/other game opcode on the tap | SEAM: painted 담기 art sits at y≈255-271, but `shop_action_at` uses raw `ITEMBUY1` rect `(220,187)-(301,204)`; the visible button is outside every hit rect. Raw-center press `(260,195)` lands on the figure, not the button | `c84b952f…9922e` / `5d97d52f…bcec7` + `side.rs:688,750` | hit-test (rect origin) | P0 | #397, #383 |
| D3 | 2 per-cell prices | ITEMWON/ITEMGEM ink at y≈242-255 (raw 169/322 +70), gem cell reads `0` on all four visible cards | SEAM: ink at raw y169-182 / y322-335, i.e. 70 px too high; gem cells blank; no live gem source | `05b24e64…cb00` / `5d97d52f…bcec7` | staging rect + plate readout | P0 | #395 |
| D4 | 3 left panel baseline | Entry shows MY BAG panel with 3 owned icons and page `01/03`; SHOPPING BAG opens only after a row pick | SEAM: SHOPPING BAG panel and its `01/01` page are already open at entry; no owned icons | `05b24e64…cb00` / `5d97d52f…bcec7` | frame state | P1 | #515 |
| D5 | 3 notice panel | Blue NOTICE board at `(590,408)-(793,566)` with mascot/title; no large white tooltip at entry | SEAM: white owl-logo tooltip at `(455,404)-(697,523)` open at entry | `05b24e64…cb00` / `5d97d52f…bcec7` | frame state | P1 | #515 |
| D6 | 4 gift press | Pressing the visible gift half at `(281,263)` opens a centered blue modal (friend-gift notice) with OK at `(400,365)`; no 5017 or any game opcode | SEAM: `GIFTROW` records a no-op; no modal art or handler is wired | `b987d9e7…4107b` / `5d97d52f…bcec7` + `side.rs` `ShopGiftNoop` | frame state + wire handling | P1 | #397, #383 |
| D7 | 3 wallet cells | GEMMONEY/MGASH read `626` / `0` from `5029 7202000000000000` | SEAM: synthetic `1200` / `2200`; live wiring exists but no rust live frame this wave | `3ad74702…873c` / `5d97d52f…bcec7` | plate readout | P2 | #388 |
| D8 | 3 category + submenu chrome | FASHION press highlight changes; submenu row paints six labelled tabs; hover frame pinned | SEAM: tabs paint, but live staging of the category codes is unresolved (`docs/shop.md` #195 says unstaged); no paired click frame | `707cd00a…1238a` / `5d97d52f…bcec7` | staging rect / hit-test (OPEN) | P1 | #515 |
| D9 | 3 scroll controls | Scroll-right press at `(757,383)` changes the four goods cards and page readout (prior qualified pack `099d1ee6…`); hover frame pinned here | SEAM: scroll art present; rust offsets RIGHT child by scroll origin in `shop_click_rect` (correct shape), but no live paired press | `11d130c5…f08af` / `5d97d52f…bcec7` | hit-test (needs live confirmation) | P1 | #515 |
| D10 | 3 cart/sell/planet/page chrome | Cart BUYITEM `(41,574)`, GIFTITEM `(90,574)`, cart pager `(221,575)/(340,575)`, ITEMVACATE `(67,512)`, BAGSORT `(67,560)`, BACK `(752,581)` all paint and answer at their BND rects | SEAM: same rects staged, but the visible row/cart panels differ (D4/D5) and BUYITEM sends a 5000 from a raw-hit path; exe emitted **no** 5000 on this sweep | `05b24e64…cb00` / `5d97d52f…bcec7` | hit-test + wire handling | P1 | #383, #397, #515 |
| D11 | 3 MY BAG cells | Exe bag cells show 3 owned icons at `MYBAGITEM1..3` rects `(135,423)-(208,480)` etc | SEAM: MY BAG panel replaced by SHOPPING BAG; no icons at those rects | `05b24e64…cb00` / `5d97d52f…bcec7` | frame state | P2 | #515 |

Per-cell placement delta list (source: `State_ItemShop.bnd` 186 rects paired
by `(code & 0xFFF)-1`; table `~/scratch/shop-ab/controls.txt`):

- `ITEMSHOW1..8`: `(227+141c, 86+153r)-(336+141c, 167+153r)`, c=0..3, r=0..1.
  Exe paints figures here; seam cards are flat grey. #363 shipped slot-0
  portraits, so this may be a synthetic-catalog artifact — live check first.
- `ITEMNAME1..8`: `(231+141c, 69+153r)-(331+141c, 81+153r)`. Both sides ink
  here; no delta measured.
- `ITEMWON1..8`: BND `(247+141c, 169+153r)-(285+141c, 182+153r)` ours;
  exe ink is `(247+141c, 239+153r)-(285+141c, 252+153r)` — **+70 px Y**.
- `ITEMGEM1..8`: BND `(300+141c, 169+153r)-(338+141c, 182+153r)` ours;
  exe ink +70 px Y. Exe value `0` on all four visible cards.
- `ITEMBUY1..8`: BND `(220+141c, 187+153r)-(301+141c, 204+153r)` ours;
  exe's visible 담기 art is `(220+141c, 257+153r)-(301+141c, 274+153r)`
  — **+70 px Y hit-test delta**.
- `ITEMGIFT1..8`: BND `(261+141c, 187+153r)-(301+141c, 204+153r)` ours;
  exe visible gift half +70 px Y. Exe press opens the friend-gift modal.
- `ITEMPRE1..8`: BND `(308+141c, 184+153r)-(348+141c, 201+153r)` ours;
  exe visible 입기 art +70 px Y.
- Scroll RIGHT `(527,0)-(545,18)` + scroll origin `(221,374)` → `(748,374)-(766,392)`:
  rust offsets correctly; no placement delta, only missing live press pair.
- Chrome (absolute, no delta in the BND table): tabs `SET (418,9)-(491,31)`,
  `FASHION (492,7)-(565,31)`; submenu row y32-44; wallet `GEM (82,288)-(177,301)`
  and `MGASH (82,316)-(177,328)`; cart `BUYITEM (17,563)-(66,586)`,
  `GIFTITEM (66,563)-(114,586)`, `ITEMVACATE (25,491)-(110,534)`,
  `BAGSORT (29,546)-(106,574)`; BACK `(711,569)-(794,593)`.

## Wire evidence for the interaction sweep

`04-sweep/exe/wire/room-stage.log`: after `5006 EnterShop` and `5028/5029`
at entry, the only opcode through all 28 stimuli is the 6001 heartbeat. No
5000 (buy), 5002 (sell), 5017 (gift list), or 5023 (claim). The row pick and
gift press are therefore **frame-state only** on the exe; our client's 5000
send on a raw-hit press is not exe-confirmed. This is the wire half of D2/D6.

## What stays for wave 2

1. Re-boot rust with a window finder that also accepts `_NET_WM_NAME` /
   geometry from any tree depth, or boot with a known 800x600 window, then
   replay the same 65-event trace. That yields the missing live paired frames
   for D1-D11.
2. Re-run the exe sweep with modal dismissal after the gift press (OK at
   `(400,365)`) so steps 04-28 are not blocked, then pair them with the rust
   replay. Hover frames from this wave stay valid.
3. Verify the +70 px child origin with a live rust press at `(238,263)` and
   at raw `(260,195)`; wire tap decides which side sends 5000.
4. Master-session (#396), privacy (#387), and offline labeling (#388) are not
   exercised by this guest-only A/B; they stay OPEN.

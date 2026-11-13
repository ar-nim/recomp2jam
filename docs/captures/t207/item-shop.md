# Capture manifest — item shop board, ticket #207

Goods + cart + wallet (prices + DIFF + owned marks are the music-shop
manifest; this board shows item names, prices, previews, the gem
wallet, cart rows, and pagers).

## Test role (recomp2jam, fresh this ticket)

| Field | Value |
|---|---|
| Capture ids | `t207-test-shop-native`, `t207-test-shop` (preview) |
| Role / scene | test / `STATE_SHOP` (`CStateShop`) visited from Waiting |
| Times | 2026-11-14 04:26:05 +08:00 (one walk) |
| Source | worktree `cb2/tt-207` @ `cc5ce40`, `cargo run -p recomp2jam --bin recomp2jam -- shots --out-dir ~/scratch/t207/test/` |
| Scenario | scripted walk to Waiting + `stage_shop_walk_board`: real 7.55 `Itemdata_Korea.dat` rows from `avatar.opa` (names/prices), scripted wallet stub (gem 1200, cash 50), two checked cart rows; genuine `State_Shop.bnd` art staged |
| Input trace id | scripted (no OS input): `capture_all` + `stage_shop_walk_board` |
| Format | raw RGB → std-only PNG, 800×600 RGB (`-native`), 400×300 (preview) |
| Sync event | `visit_side(Shop)` asserted; same instant for preview + native |
| Files + sha256 | `test/shop-native.png` `a61c1236…f1af6b5fbd15` (1,440,773 B), `test/shop.png` `108ccda0…1058851943` |
| Install provenance | `install-root/` linked (see channel manifest); `ITEMNAME`/`ITEMWON`/`GEMMONEY` cells ink from staged catalog + wallet stub |
| Locale | `LANG=en_SG.UTF-8` (rasterizer locale-independent; Korean item names paint via the staged font path) |
| Limitations | wallet words synthetic (no live gateway); cart checks staged; preview figure cells paint blank grey (art area stages no figure in this walk — visible in `shop-zoom.png` crop); observed: names + prices + SALE tags + gem cell + cart rows + 99/99 pagers all ink |

Replay: same command as the channel manifest reproduces byte-identically.

## Reference role (7.55 exe) — OPEN, no capture

No pinned prior exe shop capture exists (all earlier rigs stopped
at channel/room; shop entry is itself OPEN — `docs/parity/shop.md`
names the SendInput sweep as the next step). This ticket attempted
no exe shop navigation: the exe never booted (see channel manifest —
deterministic pre-net init AV, six variations, stop rule hit), so
no shop drive was possible. Next step: root-cause the boot AV, then
SendInput sweep from the lobby per `~/scratch/planet-verify/`
(X-close to the bare board was captured there; side entry points
unmapped). The shop-board emptiness question (#191) is therefore
decidable on the test side only until reference lands.

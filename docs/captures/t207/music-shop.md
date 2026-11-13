# Capture manifest — music shop board, ticket #207

Catalog prices + DIFF + owned marks + cart + pagers
(`State_MShop.bnd`, five songs per page).

## Test role (recomp2jam, fresh this ticket)

| Field | Value |
|---|---|
| Capture ids | `t207-test-musicshop-native`, `t207-test-musicshop` (preview) |
| Role / scene | test / `STATE_MSHOP` (`CStateMShop`) visited from Waiting |
| Times | 2026-11-14 04:26:05 +08:00 (one walk) |
| Source | worktree `cb2/tt-207` @ `cc5ce40`, `cargo run -p recomp2jam --bin recomp2jam -- shots --out-dir ~/scratch/t207/test/` |
| Scenario | scripted walk to Waiting + `stage_music_shop_walk_board`: real 7.55 song index (`OJNList.dat`, 562 songs) for the five rows, scripted 4031 flag stub (first + fourth visible songs owned), two staged cart picks; genuine `State_MShop.bnd` art staged |
| Input trace id | scripted (no OS input): `capture_all` + `stage_music_shop_walk_board` |
| Format | raw RGB → std-only PNG, 800×600 RGB (`-native`), 400×300 (preview) |
| Sync event | `visit_side(MusicShop)` asserted; same instant for preview + native |
| Files + sha256 | `test/musicshop-native.png` `621caa26…f4234da559d727` (1,440,773 B), `test/musicshop.png` `a0b5c18c…207ec919c5cc` |
| Install provenance | `install-root/` linked (see channel manifest) |
| Locale | `LANG=en_SG.UTF-8` (rasterizer locale-independent) |
| Limitations | 4031 owned flags synthetic (ASSUMED wire-side); observed: TITLE/작곡/작사/장르/BPM cells + 잼머니/캐쉬 price cells + OWNED flags (rows 1, 4) + cart rows + 99/99 pagers all ink; 난이도 (DIFF) VALUES do not visibly ink — the OPEN "song DIFF" arm (`docs/parity/musicshop.md`) stays open (see `mshop-zoom.png` crop) |

Replay: same command as the channel manifest reproduces byte-identically.

## Reference role (7.55 exe) — OPEN, no capture

No pinned prior exe music-shop capture exists (same position as the
item shop — entry unmapped, `docs/parity/musicshop.md` OPEN). No exe
music-shop drive was possible (boot AV, see channel manifest).
Next step: same as item-shop (boot root-cause, then lobby sweep).
The price/DIFF/owned-marks emptiness question (#192) is decidable on
the test side only until reference lands: prices + owned marks paint
from staged data; song DIFF values do not (captured gap, not a claim).

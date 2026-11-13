# Capture manifest — MyRoom entry from Room, ticket #207

Figure + bag + info plate (`State_Myroom.bnd`, 111 rects).

## Test role (recomp2jam, fresh this ticket)

| Field | Value |
|---|---|
| Capture ids | `t207-test-myroom-native`, `t207-test-myroom` (preview) |
| Role / scene | test / `STATE_MYROOM` (`0x6A`) visited from Waiting (the walk's lobby; Room entry stages the same side — `visit_side` from the lobby per lifecycle §12) |
| Times | 2026-11-14 04:26:05 +08:00 (one walk) |
| Source | worktree `cb2/tt-207` @ `cc5ce40`, `cargo run -p recomp2jam --bin recomp2jam -- shots --out-dir ~/scratch/t207/test/` |
| Scenario | scripted walk to Waiting + genuine `State_Myroom.bnd` art staged (`stage_side_art`); worn-figure + gift/caption fixtures per `docs/myroom.md` |
| Input trace id | scripted (no OS input): `capture_all` side loop |
| Format | raw RGB → std-only PNG, 800×600 RGB (`-native`), 400×300 (preview) |
| Sync event | `visit_side(MyRoom)` asserted; same instant for preview + native |
| Files + sha256 | `test/myroom-native.png` `335f63d7…c256b989936` (1,440,773 B), `test/myroom.png` `16faf43e…51ad93a4a3a` |
| Install provenance | `install-root/` linked (see channel manifest) |
| Locale | `LANG=en_SG.UTF-8` (rasterizer locale-independent) |
| Limitations | captions staged by name identity; observed: avatar figure + STATUS info plate (Level/Exp/Cash Point/… cells) + PRESENT BOX gift list + bag panel all ink at staged rects |

Replay: same command as the channel manifest reproduces byte-identically.

## Reference role (7.55 exe) — OPEN, no capture

No pinned prior exe MyRoom capture exists (entry unmapped,
`docs/myroom.md` OPEN 1–8). No exe MyRoom drive was possible (boot
AV, see channel manifest). Next step: same as the shops (boot
root-cause, then lobby sweep). Entry side effects (gift claim
`5023`, entry `5017` queueing) stay stub-side until the live sweep.

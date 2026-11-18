# Journey audit — #316 Spec: App-wide message and utility dialogs

Base: `2105ac6`. Reused evidence only; no exe boot, no product change.

## Evidence anchors (artifact + hash)

| Id | Artifact | Hash / provenance |
|---|---|---|
| `B6` | `~/scratch/side-scenes/b6-shops/` exe 7.55 (`83a04541…72c823b`); `artifacts/input.log` clicks Room/MyRoom MESSENGER `(644,580)` and dismisses one OK at `(400,368)`; `messenger.png` is a one-OK `Dialog_Information`-shaped sheet | manifest sha256 `c8adc9aa01e72258b336f6b470a62bb6373d15a4451405019ec943da2253131b`; `messenger.png` sha256 `946e503319b6fa18fcd28b2cf9230b9124b5177cc002567c07afac84631ad917` |
| `B8` | `~/scratch/side-scenes/b8-cash/` repeats the same Information-sheet mount; `cash-charge.png` is the separate billing web window | manifest sha256 `f9d4a6953c55b75b4f04db3c0ece198bd2032361ac65dc8275ce07ccb6aa3f8b`; `cash-charge.png` sha256 `27fd5435fbe64c96315fcb29f0947bfc02d794b318c8a3047dccf15d176be671` |
| `STATIC` | `docs/file-formats/opi.md:255-266` (27 dialogs), `docs/dialog-strings.md §1` (`Dialog_Information.ojs` anchor `(241,206)` 318x187, OK `(102,150)` 117x26), `docs/bnd-consumption.md`, `docs/bnd-semantics.md` | repo files at `2105ac6` |

Our journey: boot → `live_art.rs:107` stages `Dialog_Information.bnd` →
`game/mod.rs:4230` loads the relay-down sheet → `paint.rs:2254` paints the
pinned CP949 string → `dismiss_relay_down_dialog` (`game/mod.rs:1411 area`) on
OK. The other 26 declared dialogs have no staged mount path in
`crates/o2jam-shell/src/staging.rs`.

## Summary matrix

| Verdict | Count | Stories |
|---|---:|---|
| MATCHED | 3 | 2, 16, 18 (Information sheet only) |
| MISMATCHED | 1 | 1 (message shown on the Information sheet) |
| BLOCKED | 13 | 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 14, 17, 21 |
| NA-DESIGN | 4 | 13, 15, 19, 20 |

New issues: none. The only observed divergence is the sheet's message source;
it traces to the #345 server-liveness class (closed ticket, note only) and is
mapped, not filed. All other gaps are missing exe pins or captures.

Mapped, not filed: #345 (server-unavailable message from the Messenger mount),
#245/#249/#250 (quit-confirm, idle-room notice, frame captures), #296/#297
(billing web leg owned by those specs).

BLOCKED for live lane: YES/NO sheet, Whisper/Emotion sheets, identity edits and
wire commits, MUSICINFO/SKILLIINFO/EVENT/GIFTVIEW/CONFERENCE/cart mounts,
rest/hover frames, idle-room no-response notice.

## Per-story table

| # | Story (short) | Comparator | Our journey ref | Exe evidence | Verdict | Notes |
|---|---|---|---|---|---|---|
| 1 | Information sheet + OK at anchors | pixel-region + data-effect | `paint_relay_down_dialog` (`paint.rs:2254`) paints `RELAY_DOWN_MESSAGE_CP949` in the staged TEXT rect; OK via `INFORMATION_OK_CODE` | `B6`/`B8` `messenger.png` shows the same sheet shape and OK but the server-unavailable text | MISMATCHED | Sheet geometry matches; message source differs (relay-down vs messenger server error). Mapped to #345 liveness, not filed. |
| 2 | Parent-relative offsets honored | pixel-region | `dialog_origin` (`paint.rs:2048`), `paint_dialog_scene` (`:2115`) | `STATIC` anchor `(241,206)` + OK `(102,150)`; `B6` popup sits there | MATCHED | Both origins agree with the parsed `.bnd` data. |
| 3 | YES/NO sheets block host input | input-response | `PlanetModal::QuitConfirm` modal, `paint_planet_quit`, `quit_button_hits` | `b2-dialogs*`/`b3`/`b4` quit-confirm targets were NOT reached (`status FAILED`); no qualified sheet capture | BLOCKED | Need one qualified quit-confirm capture; #245 class. |
| 4 | WHISPER/EMOTION complete chat flows | input-response | `/w`, `/r` chat wires in `screens/game/chat.rs` and `session.rs`, but no dialog sheets | No qualified Whisper/Emotion capture | BLOCKED | Chat wire is not the exe dialog journey; needs a pin. |
| 5 | CHANGETITLE/CHANGENICKNAME edit identity fields | input-response | No identity dialog staged or dispatched | No exe capture | BLOCKED | Needs one qualified edit capture. |
| 6 | Identity commits use pinned wire shape | wire-sequence | No identity send | No wire pin in reused packs | BLOCKED | Inventory spec owns wire shape; do not invent. |
| 7 | MUSICINFO shows catalog metadata | pixel-region | No MUSICINFO mount; catalog data exists in `game/mod.rs` | No exe capture | BLOCKED | Needs a live mount pin. |
| 8 | SKILLIINFO explains skill rings | pixel-region | No sheet staged | No exe capture | BLOCKED | Needs a live mount pin. |
| 9 | EVENT promo sheet | pixel-region | No EVENT sheet dispatch; promo art is static in `paint.rs` | No qualified promo-sheet capture | BLOCKED | #245 class. |
| 10 | CHARGECASH/GIFTVIEW follow web pins | data-effect | `side.rs:3981-4024` names both billing pages; payment side paints the unwired notice | `B8` `cash-charge.png` shows the exe's `Cash Charge` web window (black client area in the rig) | BLOCKED | Web host/session join is unpinned; gift page not captured. Liveness note #345 for the black panel. Owned by #296/#297. |
| 11 | SHOPPINGBAG_ADD follows cart pin | wire-sequence | Shop wire exists; `side.rs:448` `SHOP_GIFT_NEXT_STEP` still names the unpinned gift/bag leg | No exe cart-add capture | BLOCKED | Needs a qualified button + wire capture. |
| 12 | CONFERENCE live or dead | input-response | `myroom.rs:602` P0 no-op | No exe capture | BLOCKED | Needs a live-mount-or-dead pin. |
| 13 | 27 dialogs enumerated | NA-DESIGN (census doc) | `docs/file-formats/opi.md:255-266` table; `docs/exe-state-inventory.md:59` | `STATIC` `NUMBER_OF_DIALOG 0x1B` | NA-DESIGN | Reviewer census; no runtime journey. |
| 14 | Bucket E/GAP sheets pinned or dead | state-path | `docs/bnd-consumption.md`, `side.rs:448` next step | `B6` adds one live Information mount via Messenger | BLOCKED | Remaining sheets still lack a mount-or-dead verdict. |
| 15 | No art-alone parity claim | NA-DESIGN (process rule) | audit tables in `docs/parity/` | n/a | NA-DESIGN | Reviewer rule, not a player journey. |
| 16 | Live sheets staged at parsed rects | state-path / data-effect | `live_art.rs:107` stages Information; `game/mod.rs:4230` loads it | `B6`/`B8` Information sheet at the pinned origin (hash above) | MATCHED | For the one live sheet; other sheets remain story 14. |
| 17 | Rest/hover/active frames | pixel-region | art delta guards for other dialogs | `B6` rest frame only | BLOCKED | Need hover/press A/B (#245 class). |
| 18 | Live caller chain from boot into each live sheet | state-path | boot → relay notice → Information sheet → OK dismiss, with guard | `B6` boot → Room/MyRoom Messenger → Information sheet → OK dismiss | MATCHED | Both reach the same sheet class; triggers differ (relay vs server error), message source is story 1. |
| 19 | Each OPEN host names next step | NA-DESIGN (docs) | `side.rs:448`, `docs/bnd-consumption.md` | `STATIC` next steps | NA-DESIGN | Reviewer/documentation story. |
| 20 | Dialog-open cue only if pinned | NA-DESIGN (negative) | Information mount queues no cue | No exe cue pin | NA-DESIGN | No invented audio either side. |
| 21 | Information mounts in Room as idle no-response notice | input-response / pixel-region | No idle-Room Information mount; our Information sheet opens only for relay-down on Planet | `B6`/`B8` mount after a Messenger click in MyRoom, not idle Room | BLOCKED | The spec's own required capture (`probe-failure.png`, hash `0dd6e4b2…fd4`) is not in the reused packs; needs a qualified idle-Room capture (#245 class). |

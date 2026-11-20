# App-wide dialogs story matrix — Spec #316 (WAVE 4, lane dialogs)

Method: `docs/captures/story-matrix/TEMPLATE.md`; cell shape follows the six
exemplars (291, 289, 290, 288, 294, 180). Base main `fbdede1`; worktree
`/home/arnim/Projects/recomp2jam-wt142`, branch `cb2/story-dialogs`. No code
change, no push. The 7.55 executable is the oracle (`OTwo.exe`, 5,029,888 B,
SHA-256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`).
Memoryer is a wire witness, never an exe rule. All packs stay in git-ignored
scratch; only hashes and text enter the repo.

Actor names are the template's. Here `solo` is a local session with no gateway
session, `master`/`peer` are room roles (on the Planet board before a room
exists they are prospective roles), `channel` observes the board or Channel
dialog, `server` is a frame with no local click. The relay endpoints are an
environment actor named in the stimulus.

## 1. Evidence inventory (before any cell was graded)

Every hash below was re-read with `sha256sum` on 2026-11-19; abbreviated hashes
in cells resolve through this table. Stale and negative probes are included
with their verdicts. A green Rust guard is never exe evidence.

| Source | Pin (full sha256 unless noted) | What it names / verdict |
|---|---|---|
| exe identity / static pins | `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b` | 7.55 oracle; `docs/dialog-strings.md` §1 sheet anchors `(241,206)` 318x187 and OK `(102,150)` 117x26, no-response string at file `0x24d02c`; `docs/net-protocol.md` §4b 2021/2022 and send files `0x48ff90`–`0x48ffa8`; `docs/planet.md` gear/exit rects; `docs/bnd-consumption.md` E bucket; `docs/captures/static-367/pin.md` |
| t240 qualified exe pack `~/scratch/240-fix/wine2` | manifest `57089d0365191ebba4261abe5f5d38785908f528a3e7ca80e905e357cf3f3cfa`; citation `15442fbffd60aee7e305d64329dc43c045aea56183b931a76a153e358948b6c3`; planet `235f53521171be4f1d4c725d16143332fa47b66813f9e1c5b8e8b09fe68ddc21`; channel `ea5cc6d632a534ab02d7d6f12e10bf357f8cf5dc65db991a9b026ee0ecf6c563`; room `f91adc41dd70e360e60ecfa6451c322ba85d7aa8a1c1b92c3d460943e4ff542f` | one-OK NORMAL-mode sheet over board + Channel dialog; X close and row entry drive; full containment |
| p244 popup `~/scratch/p244-capture` | popup `baf781b8caa6823b60047901e5a135d3ab77cb7fc60a7031fda02d90e408c1bf` | exe relay notice before the Channel rows (#244/#348). **Contradiction:** citation text names manifest `e211cc6e…`; local `manifest.json` is `9ae19031…`. The frame hash agrees in both and is the pin |
| side-scenes #330 packs `~/scratch/side-scenes/b3-dialogs`, `b4-dialogs`, `b6-shops`, `b8-cash` | b3 manifest `a2c036e2062dc0c6727ce3e1360d3a916d2f96ec21f9fe69e03a29a83274bbe9`, options `333f9baee3795c5cdfe05be230a3857ca31f4d6fb5fad88454c1303c84d6198d`; b4 options `6d17a781e4d5e522da5ad72e62c26281ae0d7317b20bde9d7bad73421548043e`; b6 messenger popup `946e503319b6fa18fcd28b2cf9230b9124b5177cc002567c07afac84631ad917` (citation text names manifest `c8adc9aa…`, local file `1cdd90d8…`); b8 cash shell `27fd5435fbe64c96315fcb29f0947bfc02d794b318c8a3047dccf15d176be671` | exe Options first sheet, one-OK Messenger "server not found" Information sheet, billing web shell. Runner status FAILED for b3/b4 only on the chained quit-confirm target; their provenance checks pass |
| Room fuse packs `~/scratch/tt-240-shape/b2-prewarp-dclick`, `~/scratch/live-367b-run/b2-direct` | modal `0dd6e4b20ff51b987e55d9b74eca9269eac6f6d9c0a3006699353bf7d862afd4`, wire `c9a76ee6fee3353c83a138f3e4d8035eb3a0021dc0ac4ea504c2f57b80d564ed`; stopped-state modal `db6b85b87f782e25f01f81aa4e507283cb293cad111e4e29d92b6833fbf65416`, winedbg `c5c4a95f92c4d7b250eab16c51ab680f7941cce9125db19aed0a4a68240a13a2`, wire `5ba48090f235ed171d9d0e8b54ef370ead63cfe11224c851652a026c35428bb3`, manifest `4083f3ee5a2f90f5fc1d876c03cfa50272dc20aaceaa95e3797775d924e5cd97` | Room Information sheet at +36 s and its arming node: expected reply `07DB` proves `07DA` (GetUserList) missed by the 20 s overdue watchdog. #367 closed by `ece8a99` |
| relay-down client legs `~/scratch/story-channel-relay-down/run2..run4` | planet `417a1aa126a76f1b2bd804965acc2e78ab5e409a77d0fd899931fd650cf7a4dc`; channel `deee19d96cf9d3e098b76168160ca9941b9029153683c3b943106357be4e0afd`; root framebuffer `0bae3412f1d1b0852922a8b866ed25fe118e43b9404598f4b6b3de58c0abc903` | our client's notice over a BLACK canvas against the exe's board+dialog backdrop (`baf781b8…`); #348 fix parked — OPEN-FIX |
| p379 two-client close pins (re-read) | master tap `d66d693db0166cf80a94db46cd0347dc25caedff48560fc9e7baf329b8201ca7`; guest tap `1c9dbcdb9044d75916d94fc8dd22f6c307705edf6210afd8659a31e5cb08e119` | 3004 (103 B) to master, 3003 (180 B) to guest, 4009 both, 3007 master only; #379 merged (`51c8d95`), member matrix green |
| this lane boot 1 `~/scratch/story-dialogs-live/boot1-quit` | manifest `06c3d14a65cc2e2d80625895c68d74ce44e6a98f9b0100b3819e831707798168`; citation `92d22ac45fc1b5e2aaeb376559aeafe66d9c9f654600fedd6b3e934bfb127864`; drive `a3a4f8cde71315a51fc3054e645e2d5500b8cacde51e48f6ef34d8202c86b0b2`; options `333f9bae…198d` (byte-identical to b3); quit-confirm `a21896be01328edb70b143b05eecf32a116302a0505eade7841eeaa6b5921efd` | **New qualified exe pin:** exit disc opens the blue `MESSAGE` quit-confirm with OK/CANCEL over the full Planet board; Options EXIT button closes the Options sheet so the exit disc is live. Manifest/audio/cite all 0, `route=sink` |
| this lane boot 2 `~/scratch/story-dialogs-live/boot2-xclose` | manifest `00ad52a7f6842b8e0d6201d1817d42676316af960d79975e31d9eb312a61628a`; citation `665333cb06d3f4fa7ef55e87f3e8444e98054dec7f591fd52bf9a7693ba971bf`; drive `621b04c07ee9f587b000848b70de171716709f2080a562b5b9470dd7a4fac997`; popup `9bf5463c4a725c296726d65a81742a22662fb95bff05c8759b6de2055fab39c5`; channel `5decbf06e800749181d8190f2236fe57d191210c6cdab05257a072bd5e1cc39e`; after-X `695e826b47ca69151d8ff9bbade9dbd813c4cbd21152c2a61ea419f0273b41af`; after-alt `0fa381044d0c238381af98dc321e4d202059d4740d968a9f3c7f7a74eadc698c`; wire `3a2e300eeb5f525f339e0f359f2af73ed8d735f918c1631940310f59b0926490` | **New qualified exe pins:** NORMAL-mode sheet content and its OK; X `(577,171)` closes the Channel dialog to the bare Planet board; tap shows no 2021/2022 around X. Duplicate screenshot repaired in scratch, then verify/audio/cite all 0 |
| this lane boots 3-4 `~/scratch/story-dialogs-live/two-client` | master tap `37c25abe71b944f91c9e6d15ef0caeba2872f20e781d9020943905a00e958818`; guest tap `ed7760cca337ace2340978332ee3829ac6968bfeb7bc7def459f905d3534826b`; guest sheet `5e118db0d0b9e165216a6ca0ec88682c61eec62d02468132f38d690999f357d9`; server slice `4a489ffee3351ca86436ec7df17d30f4f3d6b8c942869c6ee8a3b833dc29fe29` | Two fresh Rust boots built from `fbdede1` (`c0f8d81e…`); wire valid, pixels FAILED (no worktree data symlink, all four data slots claimed). Driver sent `4010` instead of READY and guest never joined. Failed attempts for paint/actor cells, kept as negative wire witness only |
| issues #348 (OPEN), #367/#330/#379 (CLOSED), #316 journey audit | owner observations 2026-11-17…19; fixes `ece8a99`, `9d2434c`, `51c8d95` | #348 backdrop fix parked; #367 Room fuse fully pinned and closed; #330 quit-confirm was one boot short, this wave closes the exe-paint half; #379 member visibility green |

## 2. Matrix (26 actions × 5 observer actors = 130 cells)

Cell key: `EVIDENCED`, `GAP`, `LIVE-FILLED`, `OPEN-FIX`, `NA-DESIGN`.
Every GAP carries `OPEN — <next evidence step>`. `self:` = the column actor
raises the stimulus, `other:` = another actor does. A cell is a story
candidate, not a closure: closing still needs the live caller chain and gates.

| Action (raised by) | solo | master | peer | channel | server |
|---|---|---|---|---|---|
| **INFORMATION-MOUNT** (self:local caller / server sheet) | NA-DESIGN — offline boot has no gateway caller | EVIDENCED — `<fuse or Messenger click>` → the same one-OK sheet shape at `(241,206)`; fuse modal `0dd6e4b2…`/`db6b85b8…`, dossier `docs/dialog-strings.md` §1; role not separately framed | EVIDENCED — `<Messenger disc>` → `Messageserver` Information sheet with OK at `(400,368)`; b6 frame `946e5033…`, b8 pack `f9d4a695…` | EVIDENCED — `<relay down at Channel>` → one-OK sheet over the row dialog; p244 `baf781b8…`, boot2 `9bf5463c…` | EVIDENCED — `<07DA starve / relay notice>` → wire `07DA`/`07E8` at +36 s and 1003 rows behind the sheet; wire `5ba48090…`, boot2 tap `3a2e300e…` |
| **INFORMATION-OK** (self:local OK click) | NA-DESIGN — no gateway message sheet offline | EVIDENCED — `<OK>` → top dialog popped by queue type 100; static queue pin `docs/captures/static-367/pin.md`, #367 close `ece8a99` | EVIDENCED — `<OK (400,368)>` → sheet gone, Messenger/board usable; b6 `drive.log` in journey audit, frame `946e5033…` | EVIDENCED — `<OK (400,365)>` → Channel rows revealed; t240 `drive.log` `23581486…`, next frame `ea5cc6d6…` | GAP — `<OK>` → no opcode around dismissal is pinned; OPEN — tap one qualified exe OK click and name every frame, comparing with the X-close negative |
| **RELAY-BOUNDARY** (environment: relay endpoints down) | NA-DESIGN — no gateway probe offline | EVIDENCED — `<boot with relay down>` → clean Planet; notice only after Channel entry; planet `417a1aa1…`, popup `baf781b8…`/`9bf5463c…`, #348 | EVIDENCED — `< Channel entry>` → notice before room entry; p244 `drive.log` popup→OK→`03EA` | EVIDENCED — `<entry>` → sheet over board + dialog + row wells; t240 + this lane boot2 | GAP — `<relay down>` → 1003 hold vs popup timing unpinned; OPEN — tap the client socket from boot to Channel with endpoints down |
| **RELAY-BACKDROP** (self:server notice while relay down) | NA-DESIGN — no notice offline | EVIDENCED — `<notice up>` → exe keeps Planet board, emblem strip and Channel dialog behind the sheet; `baf781b8…`, boot2 `9bf5463c…` | EVIDENCED — `<notice up>` → same full backdrop; p244 citation | OPEN-FIX — `<notice up>` → exe backdrop (`baf781b8…`) vs our black canvas (`deee19d9…`, root `0bae3412…`); fix parked on #348 | NA-DESIGN — backdrop is staged local pixels; no wire field carries it |
| **ROOM-FUSE-ARM** (self:server 07DA reply starvation) | NA-DESIGN — no room fuse offline | EVIDENCED — `<07DA node starved >20 s in Room>` → one-OK Information sheet; stopped-state expected reply `07DB`, winedbg `c5c4a95f…`, modal `db6b85b8…`; #367 actor-independent mechanism | EVIDENCED — `<same starve>` → same sheet and 20 s wall gate; qualified modal `0dd6e4b2…`, dossier `static-367/pin.md`; no separate role-tagged frame | GAP — `<fuse up>` → channel row state unpinned; OPEN — 2003 observer poll around the +36 s boundary | EVIDENCED — `<07DA sent, 07DB delayed>` → wire `07DA`/`07E8` same second, node stamp delta 35,780 ms; wire `5ba48090…`, `docs/net-protocol.md` §4b |
| **ROOM-FUSE-OK** (self:local OK/Return/Space on fuse) | NA-DESIGN — no fuse offline | EVIDENCED — `<OK>` → type-100 dismissal, no FFF0 attributed to OK alone; #367 close `ece8a99`, dossier anchors TEXT `0x21400003`/OK `0x21200002` | EVIDENCED — `<same input>` → same dismissal rule; static pin and #321 notes; contradiction with mouse-OK noted in §4 | NA-DESIGN — dismissal is in-client | GAP — `<OK>` → whether FFF0 follows this OK stays contradictory; OPEN — one tapped modal OK with byte-level frame log |
| **CHANNEL-NOTICE-CONTENT** (self:server notice at Channel entry) | NA-DESIGN — no Channel dialog offline | EVIDENCED — `<relay down>` → blue one-OK `Channel` sheet with the NORMAL-mode notice over the dialog; boot2 `9bf5463c…` | EVIDENCED — `<entry>` → same notice before room access; t240 + boot2 | EVIDENCED — `<entry>` → row wells stay visible behind the sheet; `ea5cc6d6…`, `9bf5463c…` | EVIDENCED — `<1003 rows>` → rows arrive behind the notice; boot2 tap `3a2e300e…` (1003 len 134), inactive-row string at file `0x647334` |
| **NOTICE-OK-DISMISS** (self:local OK on Channel notice) | NA-DESIGN — no notice offline | EVIDENCED — `<OK (400,365)>` → bare Channel dialog, face live; boot2 `drive.log` `621b04c0…`, t240 drive | EVIDENCED — `<OK>` → `03EA` then `03EC` succeeds; p244 `drive.log` | EVIDENCED — `<OK>` → row dialog revealed; t240 frame `ea5cc6d6…` | GAP — `<OK>` → no opcode after dismissal pinned; OPEN — scripted tap from OK to the first `03EC` |
| **OPTIONS-GEAR-ENTER** (self:local gear click) | EVIDENCED — `<gear (729,580)>` → first `옵션` sheet with Sound first; b3 `333f9bae…`, #359 first-tab pin | EVIDENCED — `<gear>` → same local sheet, no role gate; boot1 `a3a4f8cd…`, pack `06c3d14a…` | EVIDENCED — `<gear>` → same sheet; b4 `6d17a781…`, actor-independent art | EVIDENCED — `<sheet up>` → board hover cleared and sheet covers board; `docs/hover.md` modal rule, b3/b4 frames | NA-DESIGN — settings are local ini state, no opcode |
| **OPTIONS-EXIT** (self:local EXIT button) | EVIDENCED — `<options-exit (472,532)>` → sheet closes, board receives the next exit-disc click; boot1 drive `a3a4f8cd…`, frames `333f9bae…`→`a21896be…` | EVIDENCED — `<EXIT>` → same local close with no save; #314 story 14, boot1 chain | EVIDENCED — `<EXIT>` → same close, no role gate; b1-explore pin in #330 findings | EVIDENCED — `<close>` → bare Planet board again; boot1 planet frame `235f5352…` behind the subsequent sheet | NA-DESIGN — no wire frame for an ini-only close |
| **OPTIONS-TAB-SWITCH** (self:local tab click) | EVIDENCED — `<KEYSET/SOUNDSET/GAMESET>` → staged tab sheets switch at parsed rects; #314 close pin, `docs/options.md` | EVIDENCED — `<tab>` → same staged sheets; guard `options_tab_switches_ride_the_staged_codes` is a fix witness, exe b3/b4 frames are the claim | EVIDENCED — `<tab>` → same sheets; b3/b4 packs, actor-independent | EVIDENCED — `<tab>` → modal stays over the board; b3/b4 frames, `docs/hover.md` | NA-DESIGN — no opcode carries a settings tab |
| **QUIT-CONFIRM-OPEN** (self:local exit disc) | EVIDENCED — `<exit disc (769,580)>` → blue `MESSAGE` sheet `게임을 종료하시겠습니까?` with OK/CANCEL over the full board; boot1 `a21896be…` (qualified) | EVIDENCED — `<exit disc>` → same confirm, no role gate; boot1 pack `06c3d14a…` | EVIDENCED — `<exit disc>` → same confirm; b1 diagnostic `3dbcdf2f…` plus qualified boot1 | EVIDENCED — `<confirm up>` → board+channel art remains visible behind; boot1 frame `a21896be…` | NA-DESIGN — local process exit; no opcode |
| **QUIT-CONFIRM-CANCEL** (self:local CANCEL/Escape) | GAP — `<CANCEL/Escape>` → no qualified post-click board frame; b1 diagnostic used Escape but has no pack; OPEN — one qualified pack with CANCEL click and before/after hashes | GAP — same, no role-tagged cancel frame; OPEN — repeat CANCEL leg as master | GAP — same for a seated peer; OPEN — same leg with uid 2 | GAP — `<cancel>` → board hover restore unpinned; OPEN — compare hover before/after in that pack | NA-DESIGN — cancel sends no opcode |
| **QUIT-CONFIRM-OK** (self:local OK) | GAP — `<OK>` → process exit ends the session; no qualified post-OK frame/wire; OPEN — one bounded boot with a post-OK process-state line (may use `--row-dismiss auto` class) | GAP — same for a logged-in master; OPEN — same leg with session end recorded | GAP — same for a peer; OPEN — same leg | NA-DESIGN — board disappears with the process | GAP — `<OK>` → whether any 2021/FFF0 is sent on quit-confirm OK is unpinned; OPEN — tap the exit socket across one OK |
| **MESSAGE-BLOCK** (self:local input while a sheet is up) | GAP — offline YES/NO block has no qualified input trace; OPEN — one quit-confirm-up row/disc click with no effect | GAP — `<click behind confirm>` → no qualified block pin for a master; OPEN — one quit-confirm-up room control click with wire and frame | GAP — same for a peer; OPEN — same leg with uid 2 | EVIDENCED — `<one-OK relay sheet up>` → row entry blocked until OK; t240 `drive.log` and p244 popup→OK→`03EA` | GAP — `<blocked click>` → no opcode emitted is a negative needing a tap; OPEN — tap socket across one blocked click |
| **MESSENGER-POPUP** (other:server has no Messenger endpoint) | NA-DESIGN — no gateway Messenger offline | EVIDENCED — `<Messenger click in MyRoom>` → one-OK Information sheet with server-unavailable text; b6 manifest `c8adc9aa…` citation, frame `946e5033…`; role-neutral mount | EVIDENCED — `<Messenger click>` → same sheet; b8 pack `f9d4a695…`, journey audit B6/B8 | NA-DESIGN — private side screen; no board row | EVIDENCED — `<click>` → no 6xxx Messenger frame in the tap; #330 close comment records missing endpoint, `docs/net-protocol.md` §2 |
| **X-CLOSE** (self:peer X click on Channel dialog) | NA-DESIGN — no Channel dialog offline | EVIDENCED — `<X (577,171)>` → bare Planet board, channel sheet gone; boot2 `695e826b…`, planet `235f5352…` | EVIDENCED — `<X>` → same local close, no room entry; boot2 chain | EVIDENCED — `<X>` → MENU band and promo art return; boot2 `695e826b…`, follow-up `0fa38104…` | LIVE-FILLED — `<X>` → no 2021/2022 in the qualified tap before stop; negative wire pin `3a2e300e…` (1000/1001/1002/1003 only); 2021 remains the channel-logout opcode |
| **LOGOUT-2021** (self:channel exit / scripted leave) | NA-DESIGN — no channel session offline | EVIDENCED — `<2021 empty>` → `2022 result=0`, next 1004 succeeds; `docs/net-protocol.md` §4b; guards `session.rs:4523/4569` are fix witnesses | EVIDENCED — `<2021>` → re-entry never wedges; same wire pin, actor-independent server reply | GAP — `<2021>` → board row/observer state unpinned; OPEN — observer 2003 poll around one leave | EVIDENCED — `C2S 2021 len0 / S2C 2022 i32 0` and exe send files `0x48ff90`–`0x48ffa8`; wire pin `docs/net-protocol.md` §4b |
| **WHISPER-EMOTION** (self:peer chat command) | NA-DESIGN — no gateway chat offline | GAP — `<whisper/emotion UI>` → no qualified exe sheet; OPEN — capture the whisper result and EmotionHelp sheet with frame hashes | GAP — `<same>` → peer-side sheet unpinned; OPEN — same two-client pack | NA-DESIGN — chat sheets are private | EVIDENCED — 2017/2018/2019 shapes, 3011 send sites files `0xb9385` etc., `docs/net-protocol.md` §4c; wire is not a sheet pin |
| **IDENTITY-EDIT** (self:local CHANGETITLE/CHANGENICKNAME) | GAP — no exe identity dialog mount; OPEN — one qualified CHANGETITLE/CHANGENICKNAME capture | GAP — same for master; OPEN — same leg with role tagged | GAP — same for peer; OPEN — same leg | NA-DESIGN — identity edit is a private sheet | GAP — 5030/5032 commit body unpinned; OPEN — tap one accepted edit and name the request/reply bytes |
| **CATALOG-INFO** (self:local MUSICINFO/SKILLIINFO/EVENT) | GAP — bucket E, no mount pin; OPEN — mount census for the three sheets in an offline/music-shop boot | GAP — same for master; OPEN — same leg | GAP — same for peer; OPEN — same leg | NA-DESIGN — no board state for a private info sheet | NA-DESIGN — catalog is local archive data, not a gateway frame |
| **WEB-SHEETS** (self:local CHARGECASH/GIFTVIEW) | EVIDENCED — `<cash charge>` → billing web shell with blue title and blank client area; b8 frame `27fd5435…`, #296/#297 | EVIDENCED — `<cash charge>` → same shell; b6 probe and b8 pack | GAP — `<GIFTVIEW>` → no exe gift-web frame; OPEN — one gifted-account capture with request line | NA-DESIGN — web view is private | EVIDENCED — external billing host and 5028/5000 wire district, `docs/net-protocol.md` §2; #366 stub logs pin the HTTP boundary, not a gateway opcode |
| **CART-ADD** (self:local SHOPPINGBAG_ADD) | GAP — bucket E, no add frame; OPEN — one qualified cart-add click + frame | GAP — same for master; OPEN — same leg | GAP — same for peer; OPEN — same leg | NA-DESIGN — cart is private | GAP — no 5000-series add/reply body pin; OPEN — tap one add click and name both frames |
| **CONFERENCE** (self:local CONFERENCE control) | NA-DESIGN — no host or opcode; design seam `myroom.rs:602` P0 no-op | NA-DESIGN — same seam; no role-specific conference exists | NA-DESIGN — same seam | NA-DESIGN — board has no conference row | NA-DESIGN — no 5000-family conference request; grade NA-DESIGN, not a passed story |
| **CROSS-ACTOR-DIALOG** (other:master or peer acts while a sheet is up) | NA-DESIGN — no second actor offline | GAP — `<sheet up on peer, master acts>` → no qualified exe pair; OPEN — rerun two-client leg with data symlinks and the #379 green pair | EVIDENCED — re-scoped from a master-side join event to a peer-socket join: `<master CREATE sheet up, peer joins>` → master `12287270…74b185` (still CREATE chrome), peer `af89f031…4171d` (waiting room), peer tap `C2S 3002 len=9` + `S2C 3003 len=183` with W6FILTER/t207probe/tester2/tester (`W6FC-DIALOG-02`); master tap had zero new frames. Guard `peer_join_pair_leaves_the_create_room_sheet_and_master_frame_unchanged` | GAP — `<row event while a sheet is up>` → observer state unpinned; OPEN — observer socket with one client's sheet raised | LIVE-FILLED — our boots 3-4 taps show `2006`/`1004` while the guest sheet is up (`37c25abe…`, `ed7760cc…`); our pixels are a failed attempt, no exe paint claim |
| **DIALOG-CUE** (self:local sheet open) | NA-DESIGN — no exe dialog-open cue pinned; seam is the audio cue table | NA-DESIGN — same seam; no role-specific cue | NA-DESIGN — same seam | NA-DESIGN — board BGM latches independently | NA-DESIGN — no opcode or cue field for a local sheet |

## #693 tick — story 33 re-scoped (W6 W-c)

- [x] 33. As a master, I want my joining peer visible even while a message sheet is up on another client, so the two-client member matrix stays symmetric in dialog scenarios. — RE-SCOPED on the exe oracle: the observable 7.55 leg is the peer's `3002`/`3003` member pair while the master's CREATE ROOM sheet keeps painting. The master's socket had zero new frames; no master-side `3004`/`3003` join event is claimed or wired. Paired frames `12287270…74b185` / `af89f031…4171d`, trace `W6FC-DIALOG-02`; guard `peer_join_pair_leaves_the_create_room_sheet_and_master_frame_unchanged` drives the scripted gateway. The old "master sees the peer through a sheet" premise stays OPEN as a negative claim; this tick covers only the re-scoped pair.

Counts: **130 cells** — 54 EVIDENCED, 35 GAP, 2 LIVE-FILLED, 1 OPEN-FIX,
38 NA-DESIGN. Counts were re-parsed from the table; every GAP names its next
evidence step. A cell is a story candidate, not a closure.

## 3. Scenario scripts and boot ledger

### S316-DIALOG-CHAIN-01 (executed, boot 1)

- `scenario_id` `S316-DIALOG-CHAIN-01`; `spec` #316; `state` `STATE_PLANET`;
  trace id named by the pack citation `92d22ac4…`.
- `boot_pair`: `M = tester / uid 1 / prospective master`, no second client,
  no observer socket. Solo-equivalent local dialog chain.
- `containment`: private Xvfb `:160`, owned sink `rig_null_3689309`,
  `route=sink` / `on_default_sink=0` / `preboot_breach=0`, peak 0.0 dBFS,
  `RIG_INSTALL_ROOT` pinned to the main checkout's data (all four pooled data
  slots are claimed by other lanes), Memoryer PID 92326 / epoch 1794986056
  re-read, tap 15012→15010, stop by recorded PID files only.
- `steps` / `checkpoint`: seq 1 boot → planet `235f5352…` (CP01); seq 2
  board-prime + gear (729,580) → options `333f9bae…` (CP02); seq 3
  `options-exit` (472,532) → board restored (CP03); seq 4 exit disc
  (769,580) ×2 → quit-confirm `a21896be…` (CP04, `green=0.496`).
- `outcome`: "When the local actor clicks the exit disc in Planet after
  closing Options with EXIT, the exe paints the blue quit-confirm `MESSAGE`
  sheet with OK and CANCEL over the full Planet board (pack
  `06c3d14a65cc2e2d80625895c68d74ce44e6a98f9b0100b3819e831707798168`,
  frame `a21896be01328edb70b143b05eecf32a116302a0505eade7841eeaa6b5921efd`)."
  No NORMAL-mode popup was up at Planet, so none needed dismissal.

### S316-DIALOG-XWIRE-02 (executed, boot 2)

Same containment shape, display `:160`, sink `rig_null_3710397`, peak -0.3 dBFS,
tap `3a2e300e…`. Chain: boot → blue NORMAL-mode popup `9bf5463c…` (CP11,
dismissed at `(400,365)` by the driver) → Channel dialog `5decbf06…` (CP12) →
X `(577,171)` → bare board `695e826b…` (CP13) → board-prime click → promo
board `0fa38104…` (CP14). Outcome: "When the Channel-dialog actor clicks X in
the Channel dialog, the exe closes the sheet to the bare Planet board and the
tap carries no 2021/2022 around that click (pack `00ad52a7…`, wire
`3a2e300eeb5f525f339e0f359f2af73ed8d735f918c1631940310f59b0926490`)."
The first pack was rejected for a duplicate screenshot; the scratch duplicate
was removed, manifest/provenance re-emitted and all three checks rerun 0.

### S316-DIALOG-CROSS-03 (executed, boots 3-4 — FAILED attempt)

Fresh release build from `fbdede1` (`c0f8d81e…`) on displays `:183`/`:184`,
owned null sinks `rig_null_3734751`/`rig_null_3753456`, `route=unseen`
(declared silence), `on_default_sink=0`, `preboot_breach=0`, peak -91 dBFS, taps
15011/15012. Steps: host creates S316DLG and reaches Waiting; guest raises a
quit-confirm sheet on Planet; host acts; guest dismisses and is driven toward
the room. Result: wire valid (create `2004/2006`, guest `1004/1005`, async
`2011` two-member list), but no worktree data symlink meant no staged art, the
host's `Return` sent `4010` instead of READY, and the guest never emitted
`0BBA`. No exe cell or member-visibility cell is filled; only the wire witness
in §2 is kept.

### Planned chains for the wave-5 queue (no boots spent here)

| `scenario_id` | Chain | Cells it must fill |
|---|---|---|
| `S316-QUIT-CANCEL-04` | One exe boot: options EXIT → exit disc → quit-confirm → CANCEL and separately OK; capture before/after and the exit socket | QUIT-CONFIRM-CANCEL × solo/master/peer, QUIT-CONFIRM-OK × master/peer/server |
| `S316-CROSS-DIALOG-05` | Two-client pair with real data links: peer raises Messenger/quit-confirm sheet while master creates, readies, chats; then reverse roles; taps on both legs plus both frames at each checkpoint | CROSS-ACTOR-DIALOG × master/peer/channel, MESSAGE-BLOCK × master/peer/server |
| `S316-RELAY-TAP-06` | Relay endpoints down → boot → Channel with a scripted TCP tap from boot to popup OK, plus the channel observer socket | RELAY-BOUNDARY × server, NOTICE-OK-DISMISS × server, ROOM-FUSE-ARM × channel |
| `S316-DEAD-CENSUS-07` | Mount census in the exe for the remaining E-bucket sheets (identity, Whisper/Emotion, MUSICINFO, SKILLIINFO, EVENT, cart, gift, conference) | IDENTITY-EDIT, CATALOG-INFO, WEB-SHEETS × peer, CART-ADD, WHISPER-EMOTION × master/peer |

## 4. Cross-cell contradictions and limits

- **X-close vs 2021** (new): boot 2's qualified tap shows no 2021/2022 when
  the exe closes the Channel dialog with X. The scripted `2021 → 2022 result=0`
  pin in `docs/net-protocol.md` is a separate channel-logout path. The channel
  lane's guess that X sends 2021 is refuted for this path; no client change is
  made here.
- **Quit-confirm packaging**: the #330 diagnostic frame `3dbcdf2f…` was never
  a qualified pack; boot 1 now supplies one. Runner status FAILED in b3/b4
  was only the chained target, not a bad Options frame; both facts are kept.
- **Relay backdrop**: unchanged OPEN-FIX on #348. Exe `baf781b8…` and this
  lane's boot-2 popup `9bf5463c…` show board+dialog behind the sheet; our
  client's `deee19d9…`/`0bae3412…` show a black canvas. Boundary placement is
  correct (clean Planet).
- **No-response OK**: #321-era notes say mouse variants of the modal OK did
  nothing and `SendInput` Return sent FFF0; `docs/captures/static-367/pin.md`
  says OK queues type 100 with no FFF0 attributed to OK alone; the #367 close
  record keeps both by attributing FFF0 to the host disconnect path. No new
  OK-click pin was captured; the server cell stays GAP.
- **p244/b6 manifest drift**: citation text and local `manifest.json` hashes
  disagree for both packs; the cited frame hashes agree between citation,
  provenance and direct re-read. Frame pins stand; drift is not repaired.
- **Boot 2 pack repair**: the duplicate `channel-before-x.png` was removed in
  scratch and the pack re-emitted; the pre-repair citation hash is not used.
- **Two-client leg**: no pixels from boots 3-4 are exe evidence (and our
  client is never the oracle). The failed attempt is recorded with its data
  symlink cause, wrong READY aim and missing join; no member-visibility cell
  is filled from it. #379's merged fix is represented by the closed-ticket
  wire pins, not by this failed leg.
- **Server state**: one Memoryer for every leg; PID 92326 and epoch
  1794986056 unchanged. Server observations are paired with wire taps and
  per-client frames, or explicitly labeled wire-only.
- No disassembly or decompiled code is pasted; only hashes, offsets and
  behavior descriptions in this lane's own words enter the repo.

## 5. Gap-to-story hand-off

Ranking by player visibility × reachability:

1. Quit-confirm CANCEL/OK outcome (`GAP`×7): one qualified exe boot after the
   boot-1 pack; this is the highest-value remaining player-visible gap.
2. Cross-actor message boxes (`GAP`×4): the #379 fix is merged; one two-client
   leg with real data links and a sheet raised during a peer action fills it.
3. Relay boundary, popup OK and fuse OK wire (`GAP`×4): one scripted tap with endpoints
   down; the backdrop itself stays parked on #348.
4. Room fuse observer/OK wire (`GAP`×2): 2003 poll and OK tap around the +36 s
   boundary; mechanism is already fully pinned by #367.
5. Dead-sheet census (`GAP`×14): identity, Whisper/Emotion, catalog info, cart
   and gift-view mounts; each stays a GAP with a named mount step, not a
   staged-by-analogy story.
6. X-close and 2021 are no longer a live gap for the X path; the scripted
   logout pin stays as the server-side story.

New stories were written into the #316 body under `## Multi-actor stories` as
unchecked checklist rows with an evidence class and a verify step. The 21
original checklist rows stay untouched. Closing a story still needs the live
caller chain and the project gates; a conformance mismatch becomes a bug,
never a story edit.

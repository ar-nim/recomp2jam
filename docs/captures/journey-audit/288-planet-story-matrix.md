# Planet story matrix — Spec #288 (WAVE 3, lane C)

Method: `docs/captures/story-matrix/TEMPLATE.md`; cell shape follows the
Waiting (`291-waiting-story-matrix.md`) and Channel
(`289-channel-story-matrix.md`) exemplars. Base `5dd2f34`; worktree
`/home/arnim/Projects/recomp2jam-wt137`, branch `cb2/story-planet`.
No code changes and no pushes. The 7.55 executable is the oracle
(`OTwo.exe`, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`).
Memoryer is a wire witness only. Every capture path below is git-ignored
scratch; only hashes and text enter the repo.

## 1. Evidence inventory (before any cell was graded)

| Source | Pin (full sha256 unless noted) | What it names / verdict |
|---|---|---|
| exe identity | `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b` | 7.55 oracle; all static VA/file offsets below are against this build |
| `~/scratch/240-fix/wine2` (committed pointer `docs/captures/t240`) | manifest `57089d0365191ebba4261abe5f5d38785908f528a3e7ca80e905e357cf3f3cfa`, provenance `f6f7f52ceaa70fe26e9dffde9fba1d252bac50a68410f83296b52a35ee3be994`, citation `15442fbffd60aee7e305d64329dc43c045aea56183b931a76a153e358948b6c3` (match `record-hashes.txt`); planet frame `235f53521171be4f1d4c725d16143332fa47b66813f9e1c5b8e8b09fe68ddc21`, input log `23581486b41e209c7f6444a18f653cd3ef459c1f40d216cb4cb9d5affe82d496` | qualified exe Planet frame: emblem strip (H/M/G lit, QUEST/TUTORIAL dim), five-cell promo, NEW ITEM ribbon, MENU band, gear/exit discs; drive log pins EASY aim (136,88), popup OK (400,365), channel entry (526,248). One frame only, no cadence series |
| `~/scratch/p244-capture` | popup frame `baf781b8caa6823b60047901e5a135d3ab77cb7fc60a7031fda02d90e408c1bf` (re-read); citation text names manifest `e211cc6e8016f7e7048276e3f2fa247744efbaeca6f766a36130a1984e535b20`, local `manifest.json` hashes to `9ae19031ae5c7246b86fa85cb2b36f0a7a68dac5817c27eb4fa1b7e41bd3851f` | qualified exe one-OK NORMAL-mode sheet over the Channel dialog and board; contradiction kept; cite the citation text, not the local file |
| `~/scratch/p361-ab/pair/pack` + `proof5.json` | metrics `aa318f6a8b2cd62b0bd7fee44de8fa41f1a5de8227090a2812d2a2e4318714dd`; proof `0a1862bf5ec2109749a03bd9f493ae617467175d095d08acf97f1fac6fd55333`; frames exe `235f53521171be4f1d4c725d16143332fa47b66813f9e1c5b8e8b09fe68ddc21`, Rust `08c90bcb68f6a2a182682acfb03c74ca937a77699f83f5577e57f21e90f06d3b` | post-#361 matched pair: `emblem_strip` 0.729063→0.962924, PSNR 28.04 named font-presentation deviation; wire/server/channel bodies equal |
| `~/scratch/p371-scratch/pack/region-metrics.json` | `892ba0f94a184e5a14df23124a3ab65436bd25db1af0d702955d38cb3d85f896` | post-#373 regions: `mode_caption` 0.541→0.999863, `mode_button_panel` 0.666→0.991839, `board_chrome` 0.878→0.991238; zero-colorkey black plates now cover board art |
| `~/scratch/p342-ab/region-metrics-selected.json` | `98361ecd4424c6a22f80f92bc60b727d20179a7f04e3c0f89f243662a188` | post-#342 promo flanks: left 0.538779→0.828717, right 0.561471→0.952835; selected carousel word (`this+0x80`) drives channel flanks, no 1003/2026 field |
| `~/scratch/side-scenes/b3-dialogs`, `b4-dialogs` | options frames `333f9baee3795c5cdfe05be230a3857ca31f4d6fb5fad88454c1303c84d6198d`, `6d17a781e4d5e522da5ad72e62c26281ae0d7317b20bde9d7bad73421548043e`; manifests `a2c036e2062dc0c6727ce3e1360d3a916d2f96ec21f9fe69e03a29a83274bbe9`, `3bdfc871f77848d68eb3402ba1c9e09cbf99e2af802c79ae207d22cb40706a93` | exe gear opens the Sound-slider sheet (four volume sliders, DEFAULT/SAVE/EXIT); #359 first tab fixed in our client with a live-boot guard |
| `~/scratch/planet-verify/findings.md`, `~/scratch/hover-sweep/`, `docs/hover.md`, `docs/planet.md` | exe live/memory/static pins: `parent+0xF64` dialog flag, `+0xF6C` state object; emblem codes `0x19201002..06`; promo codes `0x1920000D/0E`, `0x1920C00F`; tick VA `0x48e3b0`, 3000 ms push file `0x8d3d0`, words `this+0x80/0x7c/0x90`; dispatch VA `0x48e000` | H/M/G hover reveals the body's own plate band; QUEST/TUTORIAL byte-identical under rest/hover/press (decorative); gear/exit/promo arrows hover frame 0; PLAYSTOP record 1; static cadence and wrap rules |
| `~/scratch/p343-ab/proof-final.json` | `20fbe17060cea3d3da16a059b74de06a724b6bb41ee6ae2caf947b3ccfc8cff6` | matched exe/Rust channel-entry wire and server slice (`wire_sequence_equal=true`); `1002/1003/1004/1005/2000/2001/2002/2003/2010/2011/2024` cadence |
| `~/scratch/p376-live`, `#376` close comment | rig script `28dce0ba36be52e9d046efb234011415e199b131dc5d95dbef19ad94235b6723`; peer row `frame-000127-room.png` `8dd6a9b39e4ba565ef2118be01ef3326c4af48d8a033b244325061080866a9d9`; fix `35a9348` plus template-row fix | 2005 RoomCreated reaches a peer already in Room; empty channel paints empty list; both clients share one server |
| `~/scratch/p379-wire-probe.log`, `#379` (OPEN) | `b07d63f81db0a0c883fdee3a83c78151d0624dac12e0bd6e2c9531f06b3f832f` (master 3004), `8e8735de7307d17d5a0a7de98dfc9b5109b95a2b9a15c0870996b973dddbac50` (joiner 3003) | owner 2026-11-19: master sees no joining peer; wire shows the join, our client does not stage it; fix parked |
| WAVE 1 `~/scratch/story-matrix-live/`, `#380` (closed after fix) | tap `e326e3f7eaae0cd88dfa9e8735d7554c5c73dd6fc7b376017dbb28ffb5d2834a`; observer `d4fa3dc6ff6dada19acbd7ced46eaeb7e21cb7743b07a74865445aea463ad680` | `S2C 2007 len=8 hex=00000000` on last disconnect and the vanished row; our room stayed listed at capture time; fix `9b5f539` later merged |
| `~/scratch/story-channel-relay-down/run2..run4` (wave-2 lane) | planet `417a1aa126a76f1b2bd804965acc2e78ab5e409a77d0fd899931fd650cf7a4dc`; channel `deee19d96cf9d3e098b76168160ca9941b9029153683c3b943106357be4e0afd`; root framebuffer `0bae3412f1d1b0852922a8b866ed25fe118e43b9404598f4b6b3de58c0abc903`; client log `37dbdaf21c1df8e4d8e442e153311f372189542332626c4af82155d1b9cb480d` | our relay-down sheet paints over a BLACK canvas; exe popup `baf781b8…` shows board+dialog behind it. Boundary placement is correct (clean Planet). Graded OPEN-FIX; no boot spent here |
| `#375`, `#364`, `#368`, `#370` close comments | fixes `8de4d28`, `4261cd3`, `5dc287a`, p370 pair; region pair `aa318f6a…` | hands default 30/31/35 is a Waiting kit story; #364 was a pointer-state input gap (exe arrow (401,366)); #368 a painter gap; #370 fixed whole population segments. Cross-state cells, not Planet paint claims |
| **this lane** `~/scratch/story-planet-live/boot1` | manifest `392dedef4316bc5479fd59052e2a69663f252597b3d8aaf0f7aa1e2584c25012`; provenance `caf9699da5f14f7382f2ace2f4b74c1ba4537534ab1035aad5b13542bfda5296`; citation `d135105a2dbf68f024be1502371429ea4b3a63c134717676baf05b42ef3b0fe5`; planet frame `235f53521171be4f1d4c725d16143332fa47b66813f9e1c5b8e8b09fe68ddc21`; `cadence/analysis.json` `95a162e37154bb0c4fda6eb2bf17ec9d5ebefe6c0f619186b012d34c4d72fc1f`; `cadence2/analysis.json` `86aad31ec4e9cfa69cbf47b003811f9325b328d92e05bfd8ee5d68783ca59166`; `cadence2/timing.csv` `83d66145d728c252127b8b91a1b3695e2865edbacbf916aac743f175cc38c339`; state PNGs `67acf7b1d5f530cd91130ca3e0168c2b2b3200192b8e2d04b4dbcc7ddb417d76`, `1b539fa50a6c2b5431c8b2935224dacfda16b1001e29d7ffdaa989679a544b00`, `dc84fefdf7a038ab39f56dfed60100d93222c7af57a12476974dfde9cfcbb294` | qualified exe promo dwell: 12 s timeline (118 samples) steady gaps 3056.5/2957.9/3052.9 ms, median 3005.43 ms; 10 s timeline (98 samples) gaps 2954.9/3056.5 ms. Three banner records cycle: NEW MUSIC vinyl → NEW ITEM bunny/cats → NEW ITEM Halloween five cells → NEW MUSIC. Fills the timed-cadence cell t240 could not |

Stale probes included with verdicts: `~/scratch/p346` proof
`3bee382f80398840b59a01d09c447c5e0ea72a87d2f50a312a049d6c3dc68802`
and metrics `f52bf479c32d638d8afa814646ef0d151a7cda0e120d183f73b885b46102e60b`
are pre-fix (board 0.878, badges 0.729); `~/scratch/native-ab-334` metrics
`a24a38f9eb9736033e33f511cc403882f0dd86a1f8e1a82dd1ce04a1bbee9070`
same. They are negatives, never current grades. A green guard is never exe
evidence; shared server state is never called a scenario delta.

Actor set is the template's: `solo` = offline/no gateway session,
`master`/`peer` = prospective room roles on the board before a room exists,
`channel` = observer of the Channel dialog phase of `CStatePlanet`,
`server` = frame with no local click. Planet itself carries no seat grid;
cross-state rows name that seam instead of pretending the board paints it.

## 2. Matrix (24 actions × 5 observer actors = 120 cells)

Cell key: `EVIDENCED` / `GAP` / `LIVE-FILLED` / `OPEN-FIX` / `NA-DESIGN`.
Every gap names a runnable next evidence step. Cell outcome is exe-observed;
our client's post-fix metrics are cited only where they corroborate the exe
claim, never as the claim itself.

| Action (raised by) | solo | master | peer | channel | server |
|---|---|---|---|---|---|
| **BOARD-ENTER** (self:solo, boot) | EVIDENCED — `<boot>` → board with five emblems, promo sheet, MENU band, gear/exit discs → pack hash `235f5352…dc21`, t240 manifest `57089d03…3cfa` | EVIDENCED — `<tester boot>` → same board, no role-specific paint → pack hash `235f5352…dc21`, p361 proof `0a1862bf…ff6` | EVIDENCED — `<guest boot>` → same board before any room role → pack hash `235f5352…dc21` (actor-independent paint), #376 peer flow | EVIDENCED — `<dialog phase>` → board chrome stays behind the sheet (`parent+0xF64=1`, state still `0x11`) → dossier/offset pin `docs/planet.md` live verification, pack hash `ea5cc6d6…c563` | EVIDENCED — `<1002→1003>` → channel rows arrive before the board is usable → wire pin p343 proof `20fbe170…cff6` |
| **EMBLEM-REST** (#361/#373, self:solo after boot) | EVIDENCED — `<rest>` → H/M/G badges dimmed, plates opaque, captions at bnd rects 6..10 → pack hash `235f5352…dc21`, metrics `aa318f6a…14dd` (0.962924) and `892ba0f9…f896` (0.991238) | EVIDENCED — `<bare board under dialog>` → same rest art → p361 pair exe `235f5352…dc21`, Rust `08c90bcb…6d3b` | EVIDENCED — `<guest bare board>` → same dim rest art; no guest-specific badge → pack hash `235f5352…dc21`, #376 two-client flow | EVIDENCED — `<dialog over board>` → dim strip visible behind the sheet → pack hash `ea5cc6d6…c563`, #361 close pin | NA-DESIGN — rest brightness is staged sheet art; no wire field carries it (design seam: `State_Planet.ojs` placement) |
| **EMBLEM-HOVER** (self:master pointer over H/M/G) | EVIDENCED — `<hover (94,44)-(427,62)>` → body frame 0's 84x18 plate band appears, no frame swap → exe hover sweep in `docs/hover.md`, offA/on captures `planet-emblem-*-offA/on` | EVIDENCED — `<hover H badge>` → plate band on H/M/G bodies only → pack pin `docs/planet.md` "Board hover highlight", live memory parent+`0xF80` | EVIDENCED — `<guest pointer>` → same per-control band; no role gate → hover sweep table `docs/hover.md` rows `0x19201002..04` | EVIDENCED — `<dialog H/M art>` → same body art reused by `IDC_BUTTON_CHANNEL_INFO_HARD/NORMAL` → static pin `docs/planet.md` ControlList lines 1962-63 | NA-DESIGN — hover is local pointer state; no opcode or body carries it |
| **DISC-HOVER** (self:solo pointer over gear/exit) | EVIDENCED — `<hover (714,565)/(754,565)>` → each disc repaints body frame 0 at full colour → hover sweep `docs/hover.md` rows `0x19200010/11` | EVIDENCED — `<master pointer>` → same frame-0 restore, count 615/654 changed px → `docs/hover.md` | EVIDENCED — `<guest pointer>` → same restore; no role gate → `docs/hover.md` | EVIDENCED — `<dialog up>` → board hover is cleared while a modal owns the board → guard story `planet_hover_stays_off_while_a_modal_owns_the_board`, `docs/hover.md` | NA-DESIGN — local pointer/modal state, no wire field |
| **PROMO-ARROW-HOVER** (self:solo pointer over LEFT/RIGHT/PLAYSTOP) | EVIDENCED — `<hover (358,514)/(392,515)/(428,514)>` → LEFT/RIGHT restore frame 0; PLAYSTOP swaps to record 1 → hover sweep `docs/hover.md`, `docs/planet.md` record-count pin | EVIDENCED — `<master pointer>` → same three rects and records → dossier/static `0x1920000D/0E`, `0x1920C00F` | EVIDENCED — `<guest pointer>` → same rect-driven hover → `docs/hover.md` | EVIDENCED — `<dialog up>` → arrows are below the sheet but hover stays cleared while modal owns board → `docs/hover.md` modal rule | NA-DESIGN — hover is local pointer state; the wire has no hover frame |
| **QUEST-PLATE-REST** (self:solo rest/hover/press on FREE/DRUM rects) | EVIDENCED — `<rest, hover, press>` → QUEST/TUTORIAL plates stay dim and byte-identical; authoring names are not runtime semantics → #137 pin in `docs/planet.md`, hover sweep rows `0x19201005/06` (0 changed px) | EVIDENCED — `<master press>` → no light, no family select, no dialog → `docs/planet.md` "permanent rejection (P0)" | EVIDENCED — `<guest press>` → same static plates → #137 pin | EVIDENCED — `<dialog up>` → plates remain dim behind the sheet → pack hash `235f5352…dc21`, `docs/parity/planet.md` Composition | NA-DESIGN — decorative plates have no server family and no wire field; next step is none (static pin closed) |
| **FAMILY-PICK** (self:master emblem click, board) | NA-DESIGN — no gateway family offline; design seam is the offline board without a channel list | EVIDENCED — `<EASY click (136,88)>` → orange border hugs picked badge; NORMAL switches family and reconnects → live pin `~/scratch/planet-verify/findings.md`, border bbox x102-191/y49-116 | EVIDENCED — `<guest click>` → same family selection; no role gate → `~/scratch/planet-verify/findings.md`, #376 guest flow | EVIDENCED — `<NORMAL after switch>` → rows blank when the test server lists only family [0] → live pin `~/scratch/planet-verify/findings.md` | EVIDENCED — `<1004 family>` → `03EC Enter channel [0/00]`, `1005` success → wire pin p343 proof `20fbe170…cff6`, Memoryer server slice in that proof |
| **CHANNEL-DIALOG-ENTER** (self:master lit-emblem click) | NA-DESIGN — offline boot has no gateway dialog; design seam is the `1002/1003` chain | EVIDENCED — `<EASY aim (136,88)>` → `parent+0xF64` becomes 1, sheet over the same `CStatePlanet` → t240 `drive.log` hash `23581486…d496`, p343 proof | EVIDENCED — `<guest lit emblem>` → same dialog phase; no role-specific sheet → t240 qualified exe drive | EVIDENCED — `<enter>` → MENU band and emblem strip stay visible behind the sheet → pack hash `ea5cc6d6…c563` (t240), dossier `docs/planet.md` | EVIDENCED — `<1002→1003, 1004→1005, 2000→2003, 2010→2011/2024>` → rows and login staged → wire pin p343 proof, `docs/net-protocol.md` |
| **DIALOG-X-RETURN** (self:peer X click (577,171)) | NA-DESIGN — no gateway dialog offline | EVIDENCED — `<X>` → bare Planet board, flag `parent+0xF64=0`, same state object → live memory pin `~/scratch/planet-verify/findings.md` | EVIDENCED — `<X>` → dialog closes to the board; no room entry → t240 X pin `docs/parity/channel.md` | EVIDENCED — `<X>` → `CStatePlanet` stays mounted, dialog phase cleared → live memory pin `docs/parity/channel.md` | EVIDENCED — `<X>` → no 2021/2022 after the X click; the qualified exe tap ends 10 s later with no logout frame → X-close pack wire `3a2e300eeb5f525f339e0f359f2af73ed8d735f918c1631940310f59b0926490`, citation `665333cb…971bf`; 2021 is the separate channel-logout pin (`docs/net-protocol.md:168`) |
| **PROMO-AUTO-TICK** (#167/P2, self:server timer on boot board) | EVIDENCED — the wave-3 qualified exe boot never authorized a gateway session (popup probe `max_blue=0.011`, no dialog) and still cycled three banner records with 3056.5/2957.9/3052.9 ms gaps → pack citation `d135105a…fe5`, `cadence/analysis.json` `95a162e3…fc1f`; the same gateway-free series is the solo witness | LIVE-FILLED — `<boot board dwell>` → three banner records cycle with steady gaps 3056.5/2957.9/3052.9 ms (median 3005.43 ms) → pack citation `d135105a…fe5`, `cadence/analysis.json` `95a162e3…fc1f` | EVIDENCED — `<guest uid 2 board dwell>` → board frame `235f5352…dc21` (byte-identical to the tester board), gaps 2955.6/3053.6/2956.2 ms (median 2955.86 ms) → boot-4 `cadence/analysis.json` `2d400d91…e43c72`, pack citation `64494d32…57b0` | EVIDENCED — `<one OK, dialog raised>` → Channel frame `5decbf06…39e`, dialog-up gaps 2975.7/2966.4/3066.1 ms (median 2971.02 ms), three dated banner states repeat → boot-4 `cadence-dialog/analysis.json` `837561ab…2cf1`, `phase-dialog.log` `e053b717…3de0` | NA-DESIGN — rotation local `this+0x90` flag, no wire field |
| **PROMO-CONTENT-RIBBON** (P9, self:solo board rest) | EVIDENCED — `<rest>` → five dated cells (1일 / 1일+이벤트 / 7일 / 30일 / 90일) and NEW ITEM ribbon on the Halloween record → planet frame `235f5352…dc21`, this lane's state PNG `dc84fefd…b294` | EVIDENCED — `<tester board>` → same five cells and ribbon → pack citation `d135105a…fe5` | EVIDENCED — `<guest board>` → same staged sheet; actor-independent art → planet frame `235f5352…dc21` | EVIDENCED — `<dialog over board>` → banner remains behind the sheet → t240 planet/channel pair | NA-DESIGN — event cells are archive art, not a wire field |
| **PROMO-LEFT-RIGHT** (self:solo arrow click) | EVIDENCED — `<LEFT/RIGHT>` → index steps with wrap (1↔max), refresh sets frame `dialog+0x84` → static pin VA `0x48e0eb`/`0x48e13c`, `docs/planet.md` | EVIDENCED — `<master arrow click>` → same wrap rule; banner frame follows index → dossier pin, guards in `docs/parity/planet.md` | EVIDENCED — `<guest arrow click>` → same wrap; no role gate → dossier pin | EVIDENCED — `<carousel word change>` → channel flanks follow the selected frame (#342) → metrics `98361ecd…b188` | NA-DESIGN — no opcode carries a local carousel index |
| **PROMO-PLAYSTOP** (self:solo PLAYSTOP click) | EVIDENCED — `<PLAYSTOP>` → toggles `this+0x90` with no repaint; hover uses record 1 → static pin VA `0x48e190`, `docs/hover.md` | EVIDENCED — `<master click>` → rotation flag toggles; banner stays until next tick → dossier pin | EVIDENCED — `<guest click>` → same toggle → dossier pin | EVIDENCED — `<stopped flag>` → channel flanks hold the selected frame → #342 pin `98361ecd…b188` | NA-DESIGN — local rotation flag, no wire field |
| **PROMO-FLANK-SELECT** (#342, self:channel in Channel dialog) | NA-DESIGN — no Channel dialog flanks offline; design seam is the dialog-phase carousel word | EVIDENCED — `<selected NEW MUSIC>` → right flank matches exe at SSIM 0.952835; left flank residual 0.828717 named → metrics `98361ecd…b188`, #342 close comment | EVIDENCED — `<guest in dialog>` → same selected-frame art; no role gate → p342 matched frames | EVIDENCED — `<1003/2026 with equal bodies>` → exe paints selected vinyl, not stacked NEW ITEM top layer → pack pin `98361ecd…b188` | NA-DESIGN — no 1003/2026 field selects the art; static pin names `this+0x80` as the source |
| **OPTION-GEAR-ENTER** (#359, self:solo gear click) | EVIDENCED — `<gear (714,565)-(743,594)>` → Sound sheet with four sliders first (not Key) → exe options frames `333f9bae…198d`, `6d17a781…043e`, manifests `a2c036e2…bbe9`/`3bdfc871…06a93` | EVIDENCED — `<master gear>` → same first sheet; #359 live-boot guard pins the chain → b3/b4 packs, `docs/options.md` | EVIDENCED — `<guest gear>` → same local settings sheet → b3/b4 packs | EVIDENCED — `<options modal up>` → board hover cleared, settings sheet over board → b3/b4 frames, `docs/hover.md` modal rule | NA-DESIGN — ini/settings are local, no gateway field |
| **OPTION-TAB-SWITCH** (#359, self:solo tab click) | EVIDENCED — `<KEYSET/SOUNDSET/GAME tab>` → staged tab sheets switch; values edit real settings → `docs/options.md`, #359 close comment | EVIDENCED — `<master tab click>` → tab switches ride staged codes → guard `options_tab_switches_ride_the_staged_codes` | EVIDENCED — `<guest tab click>` → same staged sheets → `docs/options.md` | EVIDENCED — `<tab change>` → modal stays over the board → b3/b4 frames | NA-DESIGN — no wire frame carries a settings tab |
| **EXIT-DISC** (self:solo exit click) | EVIDENCED — `<exit (750,561)-(788,599)>` → quit-confirm MESSAGE `게임을 종료하시겠습니까?` with OK/CANCEL → live capture `exit-state.png`, `docs/planet.md` | EVIDENCED — `<master exit>` → same confirm; no role gate → `docs/planet.md` | EVIDENCED — `<guest exit>` → same confirm → `docs/planet.md` | EVIDENCED — `<confirm up>` → modal covers board, board hover cleared → `docs/hover.md` modal rule | NA-DESIGN — local process exit, no opcode |
| **BGM-CUE** (self:server cue, no local click) | EVIDENCED — `<cue record 9>` → `Music/Planet.ojm` starts; repeat cue does not restart the track → `docs/parity/planet.md` Timing, #27 lineage | EVIDENCED — `<dialog opens>` → shared cue latches, BGM keeps playing → guard `emblem_round_trip_keeps_the_shared_board_track_running` | EVIDENCED — `<guest dialog>` → same latched cue; no role gate → `docs/parity/planet.md` | EVIDENCED — `<channel phase>` → no second cue push because dialog lives in `CStatePlanet` → `docs/parity/planet.md` Timing | NA-DESIGN — cue record is local audio state, not a Memoryer frame |
| **RELAY-PLAIN-BOOT** (#348, environment relay down) | NA-DESIGN — no gateway session, so no relay probe | EVIDENCED — `<relay down, boot>` → exe board clean; our Planet frame clean too → pack hash `417a1aa1…c9d4`, #348 close-state boundary | EVIDENCED — `<guest boot>` → clean board before Channel; no role gate → #348 boundary pin | EVIDENCED — `<boot dwell>` → no notice until Channel entry → channel lane boot logs, #348 | EVIDENCED — `<boot with endpoints down>` → no relay-notice frame at Planet; negative pin #348/#244 |
| **RELAY-NOTICE-CHANNEL** (#348, environment relay down) | NA-DESIGN — no gateway session, so no relay probe | EVIDENCED — `<Channel entry>` → exe one-OK NORMAL-mode sheet over board+dialog → popup frame `baf781b8…1cbf`, t240 `drive.log` OK (400,365) then `03EA` | EVIDENCED — `<guest entry>` → notice before any room entry; OK reveals rows → t240 drive, #244 pin | OPEN-FIX — `<Channel entry, relay down>` → exe sheet shows board+dialog (`baf781b8…`); our sheet paints over a BLACK canvas (`deee19d9…`, root `0bae3412…`) → divergence parked by #348 | EVIDENCED — `<relay down>` → 1000/1001/1002/1003 at 15:37:32, popup blue 0.986 at :33, OK at :34/:37, popup gone at :39; no gateway frame between 1003 and popup → boot-2 tap `a33fc8d1…8913`, drive `b1948c23…6150`, citation `30d1b061…90c7` |
| **BOARD-CHAT** (negative pin, self:master/peer) | NA-DESIGN — `STATE_PLANET` ControlList has no chat control; seam is the offline board | NA-DESIGN — no board chat; room chat is a Waiting/Room state → `STATE_PLANET` block walk in `docs/planet.md` | NA-DESIGN — no board chat for a guest either; same seam | NA-DESIGN — Channel dialog rows are not a chat feed; seam is the channel-list sheet | NA-DESIGN — 3011/3012/3013 chat frames are Room/Waiting family; no Planet chat cell |
| **CROSS-ROOM-LIST** (#376/#380, self:server room events) | NA-DESIGN — no gateway room list offline | EVIDENCED — `<2005 RoomCreated>` → host's row stages; no board paint → #376 fix `35a9348`, close comment | EVIDENCED — `<2005 fan-out>` → peer's list shows the created row → frame `8dd6a9b3…e9d9` (cross-state, not a Planet pixel) | LIVE-FILLED — `<last disconnect>` → observer sees the row vanish → WAVE 1 observer `d4fa3dc6…ad680`; #380 fixed by `9b5f539` | LIVE-FILLED — `<last disconnect>` → `S2C 2007 len=8 hex=00000000` → WAVE 1 tap `e326e3f7…2834a`; #380 fixed by `9b5f539` |
| **CROSS-MEMBER-KIT** (#379/#375, self:peer join) | NA-DESIGN — no room seat or worn figure at the board | EVIDENCED — cross-state; `<0BBA join>` → master seats the joining peer; #379 CLOSED with fix `75c81be` merged as `51c8d95`, live taps `d66d693d…01ca7` / `1c9dbcdb…8e119`; #375 default hands fixed by `8de4d28` is a Waiting-kit story, not board paint | EVIDENCED — `<3003 answer>` → joiner sees master seated; default hands 30/31/35 → wire `8e8735de…4ac50`, #375 close comment | NA-DESIGN — board observer cannot see room seats; seam is the Room/Waiting state boundary | EVIDENCED — `<0BBA>` → 3004 (103 B) to master, 3003 (180 B) to joiner → wire pin p379 logs; #375 kit is local data, not a wire field |
| **CROSS-POINTER-PANEL-FILL** (#364/#368/#370, self:server matched A/B) | NA-DESIGN — Channel/Room states, not the board | EVIDENCED — `<matched legs>` → #364 was a pointer-state input gap (exe arrow (401,366)); #368 a painter gap; #370 whole population segments → close comments, fixes `4261cd3`/`5dc287a` | EVIDENCED — `<same wire, same pointer>` → row bar and selected panel land on the exe's row/panel → matched proof `0a1862bf…f5533`, #364/#368 close pins | EVIDENCED — `<1003 population>` → `ceil(pop*10/cap)` whole segments → p370 proxy log, metrics `aa318f6a…14dd` | EVIDENCED — `<matched wire/server>` → bodies equal in both legs; no "scenario delta" dismissal → p343 proof `20fbe170…cff6`, p361 proof `0a1862bf…f5533` |

Counts after #382 final sweep: **120 cells** — 87 EVIDENCED, 3 LIVE-FILLED, 1 OPEN-FIX, 29 NA-DESIGN. Every disposition names a pack/pin/ticket or an owner item.
qualified exe packs (X-close, relay timing, solo/peer/dialog promo dwells)
and regraded the #379 cross-state OPEN-FIX cell after the fix merged. The
remaining OPEN-FIX cell is the #348 backdrop. A cell is a story candidate,
not a closure: closing still needs the live caller chain and the project
gates.

## 3. Scenario script format (simulation)

### SP-PLANET-PROMO-01 (live leg, this lane)

| Field | Value |
|---|---|
| `scenario_id`, `spec`, `state`, `trace_id` | `SP-PLANET-PROMO-01`, `#288`, `STATE_PLANET`, `SP-PROMO-DWELL-01` in manifest params |
| `boot_pair` | `M = tester / uid 1 / prospective master`; no peer client, no observer socket. No room role exists on the board; no wire claim is made. |
| `containment` | private Xvfb `:160`; owned null sink `rig_null_3555562`, `route=sink`, `on_default_sink=0`, `preboot_breach=0`, `--no-sound` not used for the exe path (audio captured on the owned sink, peak -1.1 dBFS); Memoryer PID 92326 / epoch 1794986056 recorded; stop by recorded PID files only. Another client was on 15010 at preflight (see §5). |
| `steps` | seq 1 `M` boot → Planet, CP01; seq 2 `M` easy-emblem click attempt (no dialog, no popup), CP02; seq 3 `M` 12 s promo dwell, CP03; seq 4 `M` 10 s promo dwell with viewable crop PNGs, CP04; seq 5 stop, CP05. |
| `checkpoint` | CP01 planet frame `235f5352…dc21`; CP02 `drive.log` `e82b1264…c44` (`max_blue` 0.011, `cue=server:03EA` but screen still planet); CP03 `cadence/analysis.json` `95a162e3…fc1f` + `timing.csv` `1b8004ef…e23e`; CP04 `cadence2/analysis.json` `86aad31e…9166` + `timing.csv` `83d66145…c339` + three state PNGs; CP05 empty PID file, sink unloaded. No wire tap; no observer poll line. |
| `outcome` | When the prospective master dwells on the boot Planet board, the promo panel advances through three banner records with 3056.5 / 2957.9 / 3052.9 ms between steady states (median 3005.43 ms in the 12 s run; 2954.9 / 3056.5 ms in the 10 s rerun), matching the static 3000 ms tick pin at VA `0x48e3b0` (pack citation `d135105a…fe5`, dossier `docs/planet.md`). |

### Wave-1 chain dispositions

| `scenario_id` | Chain | Disposition |
|---|---|---|
| `SP-PLANET-OFFLINE-02` | One offline/guest boot → 12 s promo dwell → repeat with the Channel dialog confirmed raised | Filled by boot 4 (guest uid 2, board and dialog arms); solo stays pinned by the wave-3 gateway-free boot 1 |
| `SP-PLANET-XWIRE-04` | Board → emblem → Channel dialog → X close with a wire tap | Filled by the qualified `story-dialogs-live/boot2-xclose` pack; no 2021/2022 |
| `SP-PLANET-RELAY-05` | Relay down → boot → Channel entry with a socket tap from boot to OK | Filled by close-cp boot 2; 1003 → popup → OK timing in one pack |

Expected-outcome sentence pattern: "When `<actor>` `<action>` in `<state>`,
`<observer>`'s `<observable>` is `<observed outcome>` (pack `<full sha256>`,
wire `<dir:id:body>`, dossier `<id>`)." For the filled cell the sentence is
in the `outcome` field above. No story is written from our client's output;
later conformance failures become bugs, not story edits.

### Boot ledger

| Boot | Display / sink | Actors | Scene | Wire tap | Checkpoints | Result |
|---|---|---|---|---|---|---|
| 1, 2026-11-19 05:24 UTC | `:160` / `rig_null_3555562` | `M` = `tester`, uid 1 | boot → Planet only (easy-emblem click did not open the dialog) | none | CP01–CP05 above | PASS: `run-manifest.sh verify` 0, `rig-audio.sh verify` 0, `rig-provenance.sh cite --claim exe` 0; owned sink unloaded, no owned process left |
| attempt 0 | — | — | — | — | — | Sampling stdout redirection failed before `cadence/` existed; no client launched, no cell filled |
| sampling redo | same boot | same | same | none | second 10 s timeline after fixing the BGRA→RGB crop conversion | No new boot; raw crop hashes from both timelines are in the qualified pack |
| close-cp boot 3 (diagnostic) | `:167` / `rig_null_3946864` | guest uid 2 | boot → board dwell | none | black board frame, crop timeline cycles | Not cited: board frame grabbed before paint; no pack emitted |
| close-cp boot 4 | `:168` / `rig_null_3959654` | guest uid 2 | boot → board dwell → emblem → one OK → Channel dialog dwell | none (no wire claim) | planet `235f5352…dc21`, channel `5decbf06…39e`, `cadence/analysis.json` `2d400d91…e43c72`, `cadence-dialog/analysis.json` `837561ab…2cf1`, `phase-dialog.log` `e053b717…3de0` | PASS: manifest/audio/cite 0; peak 0.0 dBFS, `route=sink`, `on_default_sink=0`, `preboot_breach=0`; OK dismissed once |

Wave-3 lane used 1 of its own 4 boots; the wave-1 close-cp batch used 4 of
its own 6 (two other boots are the channel-lane legs pinned in #289). No
NORMAL-mode popup was present at the wave-3 failed transition (driver probe
`max_blue=0.011`); boot 4 dismisses the one that appeared with a single OK
at (400,365), matching the t240 pin.

## 4. Gap-to-story hand-off

Wave-1 disposition of that ranking:

1. PROMO-AUTO-TICK × solo/peer/channel: CLOSED. The guest board and
   dialog-up arms share boot 4's qualified pack; solo keeps boot 1's
   gateway-free pack.
2. RELAY-NOTICE-CHANNEL × server: CLOSED by boot 2's 1003→popup→OK tap;
   the backdrop remains `OPEN-FIX` on #348.
3. DIALOG-X-RETURN × server: CLOSED by the reused X-close exe pack.
4. RELAY backdrop (`OPEN-FIX`, #348): still parked by the owner; no code
   changed in this lane.
5. CROSS-MEMBER-KIT × master: fix `75c81be` merged (`51c8d95`), #379 CLOSED;
   the cell is EVIDENCED by the live p379 taps.
6. CROSS-ROOM-LIST × channel/server: #380 fix `9b5f539` merged (`51c8d95`);
   no new Planet boot, wire proof is in hand.
7. Morning fixes #361/#373/#342/#359/#364/#368/#370/#376/#375: explicit
   matrix cells; their stories are recorded in the #288 body hand-off below,
   not re-filed.

Stories added to the #288 body under `## Multi-actor stories` follow the
checklist form `- [ ] As <actor>, I want <exe-observed outcome>, so <consequence>. — evidence: <class>; verify: <step>`.
The wave-1 close-cp batch made a second scoped body edit and read it back
(body length 12,323 bytes); no other tracker body changed in this lane.
Closing a story still needs the live caller chain and the project gates.

## 5. Cross-cell contradictions and limits

- **Relay-down backdrop**: the qualified exe frame `baf781b8…` paints the
  one-OK sheet over the planet board, Channel dialog and row wells. Our
  channel lane's frames `deee19d9…` and root framebuffer `0bae3412…` paint
  the sheet over an all-black canvas. The boundary placement itself is
  correct (Planet `417a1aa1…` clean). Graded `OPEN-FIX` because #348 owns
  the parked fix; no code changed here.
- **Shared-lane contention**: at preflight another `python3` client (PID
  3555454) was connected to 127.0.0.1:15010. This leg makes only a local
  exe paint claim; no wire, server, or two-client claim comes from it. The
  deviation is recorded rather than hidden. Memoryer PID and epoch were
  unchanged; no restart or signal.
- **Wave-3 dialog transition deviated**: the driver's easy-emblem click did
  not paint the Channel dialog (`cue=server:03EA` but screen still planet),
  so the wave-3 lane could not fill the dialog-up promo half. The driver
  probed blue twice (0.185 then 0.011); no NORMAL-mode popup was up. Close-cp
  boot 4 did reach the dialog: one OK at (400,365) cleared the popup and the
  dialog-up cadence is pinned in `cadence-dialog/analysis.json`.
- **Wave-1 boot 3 was a failed diagnostic**: the board frame was grabbed
  immediately after boot and is all black (`label=unknown`). The crop
  timeline still cycles (gaps 2856.0/3064.9 ms), but no pack or cell claim
  comes from that leg; boot 4 repeated the guest arm with a paint wait.
- **Authoring names vs runtime semantics**: `docs/planet.md` caption table
  maps `0x1940000A` to `튜토리얼` and `0x1940000B` to `퀘스트`, while the
  live rect pairing puts QUEST on the DRUM rect and TUTORIAL on the FREE
  rect. The contradiction is preserved; the plates are decorative either
  way, so no story depends on the mapping.
- **p244 popup citation**: citation text names manifest `e211cc…`; the
  local `manifest.json` hashes to `9ae190…`. The popup frame hash
  `baf781b8…` was independently re-read and is the pin used here.
- **Post-fix metrics vs exe evidence**: #361/#373/#342 close comments
  report SSIM gains on our client after fixes; they corroborate the exe
  frame pins but never replace them. The 28.04 dB emblem PSNR is a font
  residual under owner decision e8d01, not a new rest-art mismatch.
- **#364 contradiction kept**: the stale p346 row mismatch was a pointer
  input gap (exe arrow at (401,366)), not a painter off-by-one. No Planet
  cell regrades it.
- **No board chat**: the negative pin comes from the `STATE_PLANET` control
  inventory; chat wire frames belong to Room/Waiting and are not folded
  into a Planet story.
- All pixels and audio stay in git-ignored scratch; only hashes and this
  text enter the repo. No disassembly or decompiled code is pasted.

# Playing story matrix — Spec #293 (WAVE 4, lane story-playing)

Method: `docs/captures/story-matrix/TEMPLATE.md`; cell shape follows the
Waiting, Channel, Room, Planet, Result and MyRoom exemplars in this
directory. Base `fbdede19ad044f2a9c36943e3153a3afbca88cb9`; worktree
`/home/arnim/Projects/recomp2jam-wt141`, branch `cb2/story-playing`. No code
change, no pushes. The 7.55 executable is the oracle (`OTwo.exe`, 5,029,888 B,
SHA-256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`).
Memoryer is a wire witness only. Every pack path below is git-ignored
scratch; only hashes and text enter the repo.

Actor set is the template's. In this spec `solo` is the one-seat exe play in
the qualified solo packs (a logged-in SOLO room, no peer); the offline
`play --window` seam is `NA-DESIGN` because it has no 4010/4016 path.
`master` is a room master with a peer seat, `peer` is a joined guest,
`channel` observes the board (it never sees a Playing sheet), and `server`
names a frame with no local click. A two-exe cell needs two clients; the
headless bot cannot supply one (#378).

## 1. Evidence inventory (before any cell was graded)

| Source | Pin (full sha256 unless noted) | What it names / verdict |
|---|---|---|
| exe identity | `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b` | 7.55 behavior oracle for every VA/file offset below |
| `~/scratch/solo-playing/s8-7key-full` (qualified exe 7KEY pack, cite `--claim exe` 0) | manifest `1ebb5e6146f0c9eac2c38c1a0706c94b0bd74e1d32ce532b3263b2bf39c62b7b`; citation `2ccb0d2665ab2fd96f34ce848d9cf2264453f370be2e4be00dd5bc1707b0b3a1`; provenance `b12d227cb8df4935b6fb646703349e3da12a66c122d5275959f326c25b9b8917`; wire `77a9494a61c8a5e31dcc25d1f603ccfb8d4eee7aafe56867543a4c5eab7f4c89`; input `951e07877fda9ad74a1ddb2dac369a43ea9ef1bb635b9561f1153dafcbddc886`; `playing.png` `d987d80107231a6d540eedafdd8869cf481c6b238ee6478d876c81f2098d4e12`; `play-hold-24.png` `7c99c2b73e5a3b4e7c433db8b43e43321aa992f91d5d199286f1fb5ac5ead707`; `create-01b-mode-selected.png` `4a0ae4fe05e327e70855414db93323856d463d300f20cc99994ec6603e9ea71c`; `7key-lanes-3x.png` `caf7a8bc59daa7f1ce42ab806ed87c98a9bde94348af2a1afb657688b9736513` | Anchor pack: 07D4 key-mode byte `02`, seven caps `S D F SPACE J K L`, judgement rule y≈296, note bars 28×7, life strip x≈236–245, chart-timed judged strikes (COOL 1 / GOOD 1 / BAD 1 / MISS 20) and 4014 life `0x7c`; wire 4010/4011/4012/4013/4014/4015/4016/4017/4018 |
| `~/scratch/solo-playing/s2-master-play` (qualified exe 3KEY pack) | manifest `248b58689bd6238bba39b3dc09638c31edd74c1340ce49099b20982f0e9e4da2`; citation `3357b492b8429dd084e2f0172ea58c2d6a2b0527bce2eadb4f65840795e466c5`; wire `e4573ba222f13110af203916bd264ef941f035a6322463d390d8f91dc69d28de`; input `2927a6a20af34c4e79bd00f85e474e38930da9efd6595e86e200c3e7cf99273e`; `playing.png` `b1bb42cb512b1fb6dae3fb20ec4e787e4feefdb2ca1ac413d1d392ae4f2d8433` | First genuine exe Playing pixels, empty-life early Result: 4014 life 0, 4016 cool/good/bad 0, miss 20, score 0; keys pressed on Result are retracted, not strikes |
| `~/scratch/solo-playing/s7-dik-judged`, `s3-master-qualified` (failed judged-input legs, valid Result halves) | s7 manifest `972a3b86a1dd385e6dde1a540b09f9e325bb1501e939c7d16a36105f54c211da`, wire `211f5acbb66923b9e352afc985380dfef109408a72eaa98f7e61d2e50bf11d0f`, input `98a5581af50ec055285ad4c4c9afce961dfe50d8af2ad5f5feeab50b5d73d73f`; s3 manifest `88b6e232d73e96303b61de1ff07207fb424a3f8c9e6d88d3dd38fcfa59cb81b8`, wire `694d80677239bae2a75cee0391de590dba921b757dbb1023c9205a44b3c9f889` | DIK sweeps in s7 landed outside note times; the `judged-strike` line was a Result-screen false positive and is retracted. s3 is the cold channel-enter flake. Their Result halves corroborate the empty-life → 4017/4018 edge, not a key verdict |
| `~/scratch/p366-solo/s3-result-timed` | citation `415bec616ed450b5dbd3b351f59b75f55ecec0ce6dfffc8b2d18ed2ee5add092`; manifest `aabbe18e80191d6ee853b55d7b78d054ade4c34fcd82a70ee2eae881d5c99ca2`; wire `97ebe7264ba36216158f72b925b08fc188fb70311d1d75d515cb7635688f2d43`; Result frame `3ea6c48156d474aadc3d5f31663395cdfabf75da550b44e2165c102e7a4df7e4` | Qualified exe Result after a solo play (SCORE, `GOOD!`, one row, RETRY/BACK) and the `4016→4017 slot 00→4018 len 50` wire half |
| **`~/scratch/story-playing/live-multi/run2/master` (this wave, qualified master half, cite 0)** | manifest `85678c7462e082bd8a9a6c779ac728ae56ba8506ceca27c3aa3a0582568a6fdb`; citation `45ddafbd3cf8380406206ecc8428c028b69c48d8b8c445a845916c26406287e4`; wire `701297d110342c7d8a88637b699b8af219aa1e9e049d3fc15934d6efc292e2c7`; `create-01b-mode-selected.png` `ed13d76275e61800f643b8ad22be512007a34ab885ba6aa52075e62ade586f82`; `waiting-master.png` `399cea042761684a307e02b6614d91a602a327fd514884fcc082ef05e0786fd4` | Exe master creates a 7KEY room (`C2S 07D4 … 0002 …`, `S2C 07D6`) and reaches Waiting. The two-client half failed (no peer join, no START); this pack pins create/role cells only |
| **`~/scratch/story-playing/live-multi/run2/guest` (this wave, qualified Room-list half, cite 0)** | manifest `21578c2592e988fcc6e71f758d05f7e173b16eb6fb03a55f08cc50a845c64c78`; citation `362239d42e5202460bc1f274473266320a0654433767406ffd31760088a85de7`; wire `19bfc66d6ec314d500f72adc1f9edad5a9008691c682f5750aa452ed9a9663f5`; `room.png` `0eebef4d2940b70f5cd347296ca5a59ddb661e57762c7a6e1e6e71188eeed0a3` | A second exe client's `S2C 2003 len=2417` contains `PLAYMULTI` and its Room board paints that WAITING row. The subsequent row double-click emitted no `0BBA`; no Playing claim from this leg |
| `~/scratch/p379-wire-probe.log`, `p379-live/tap-{master,guest}.log` | `b07d63f81db0a0c883fdee3a83c78151d0624dac12e0bd6e2c9531f06b3f832f` (master 3004), `8e8735de7307d17d5a0a7de98dfc9b5109b95a2b9a15c0870996b973dddbac50` (guest 3003); ready echo 4009 body `0101` on both | Wire witness for the two-client roster and READY broadcast. #379 is merged at this base (`bd8d490`, `51c8d95`) with green member guards, but guards are fix state, not exe pixels |
| `~/scratch/no-response/f1-playing-qualified` | citation `8773f5cc0efb9486e782e230423d3a0439cb6182f20f22ffd790a5939d1e8517`; `drive.log` `89fe88f82bdc21a112a2fa479aff12f85959a0156014af46c96d2a89e578bc58`; wire `5233b743bd22ef7158d3120d36af1e1ee52e8648b99cbb881174de55e1fb9838` | Negative/wedge pack: READY gate passes, Loading paints, no 4014 with play pixels; stopped-state `CStateNormalPlaying` with a render-thread wait (`attach316.log` `ad8dd3ae…`). State-path claim only; superseded by the solo master packs |
| `docs/gameplay.md`, `docs/claims-gameplay.md`, `docs/net-protocol.md` §4c, `docs/lifecycle.md` §10, `docs/scroll-map.md` | judgement `FUN_00576540` file `0x176540` (ev `14992eb2…`, 6/18/25 ms, tier files `0x176646`/`0x176655`/`0x176664`, `0x176b7f`/`0x176b8e`/`0x176b9d`); key-up/tail inversion `0x1714b0` (ev `c01db4a8…`), bit-25 test `0x176c42`, tail compare `0x176646`–`0x17667a`; life field `+0x3970`, death gate `0x17051c`/`0x170557` etc, delta switch `0x170fa2` (ev `feebe490…`); 4014 type/u16 value/i32 seq/u32 score/i32 longNote, 4015 member+type+value+8 ranks, 4016 field order, 4017 slot, 4018 45-B rows; DIK key table `0x173290` (ev `fc65241f…`); scroll `0x56e1cb`–`0x56e3d3`, clock `0x791d70` | Static/REA class for every local mechanism; wire shape class for 4014/4015/4016/4017/4018. Screenshots from our client are never the exe claim |
| `docs/captures/static-play/pins.md` (`S-PLAY-B1`, `S-PLAY-B2`) | `0x550db0` boot-time seven-bank OJM loader (sole caller chain from PE entry), not a per-frame effect loop; `0x67fe8c` 8-byte float-pair NoteTail table consumed at Playing `+0xfec` by option reload/setter (`0x16bf40`/`0x16bfb0`) | Corrects both old journey-audit BLOCKED arms; REA CLI artifact IDs are local static IDs, not dossier evidence |
| #293 journey audit (`293-playing.md`), #294 matrix (`294-result-story-matrix.md`), #378 body, #379 close record | owner/static observations 2026-11-18/19; #294 boot-3 wire `870ba02f…`, frames `4a89faa5…`/`74e6a2ae…`/`c2ad594a…`; #378 OPEN 2026-11-19 | #293's 12 prior stories stay referenced, not duplicated. #378 names the bot-master chain as the next step for all two-client cells. #379 merged guards are cross-state fix state |
| WAVE 1 `~/scratch/story-matrix-live/` | tap `e326e3f7eaae0cd88dfa9e8735d7554c5c73dd6fc7b376017dbb28ffb5d2834a`; observer `d4fa3dc6ff6dada19acbd7ced46eaeb7e21cb7743b07a74865445aea463ad680` | 2005 create / 2011 two-member list / 2007 last-disconnect wire; cross-state witness, not a Playing paint claim |
| **`~/scratch/p378/guest-playing-1/guest`** (WAVE 1 #378 fix leg, fixed bot master + one exe guest, cite `--claim exe` 0) | manifest `aae3d446784e17defb21fdb23f3553fb7fa769a6c88e4c906805ce1f014e50c4`; citation `96345ebb11a552880d5b4a2384aebaac43e02abd3a788d99dcb23a20174f307d`; wire `99f360510b75a5abb8c5eae824772e8c1ef612c3bdcc57dd6ebab65638d6033e`; `guest-drive.log` `1e1c5891fcf0f879735f5112338f61be9a8a93ec3df3b0b80615645cfadf89c7`; host `091d1df23cdc31439f8fa9605ec36572b17fdf87ffca984303cc188037533208`; `guest-playing-entry.png` `98aa85b8e773293ffbf10af076afd7e2f7c6445a2b776fa22ff6aea9e346965b`; `guest-play-08.png` `9d31b6e797a8c58f7d9ef6d4850adbd687e0c83dc8289bc4a70f3bb963f7c440`; `waiting.png` `36a7fa4b8eb8055ce5e0fad994ff4a5d18d9c0b4d088cc5662661d5086fc9b5a` | First qualified guest Playing frame in a two-member room: S2C 4050 after the bot master's 4048 burst, guest 4014 life 1000→0 with 4015 echoes, three-lane BGA/HUD art and the master seat on Waiting. Containment `route=sink` / `on_default_sink=0` / `preboot_breach=0`; chart replay applied but all notes missed, so no judged-input or 4016/4018 claim |

Negative probes are included with their verdicts: f1's stopped-state frame is
not a pixel claim, s7's `judged-strike` line is retracted, s3 is a cold
channel-enter flake, and both two-exe legs failed at the join/ready gate.
The WAVE 1 #378 leg is bot master + exe guest, not a two-exe cell; it closes
only the guest half named below.
Green Rust guards (#379 member matrix, judgement/life guards) never grade an
exe-behavior cell. Server state is shared by one Memoryer by construction;
no "scenario delta" dismissal appears without a wire log and a per-client
frame.

Gap IDs used below (each is a runnable next step, not a bare OPEN):

- `E-MULTI / #378` — fixed in `scripts/wine-room-host.py` (`dae62a5`: master
  4048 tick sync releases S2C 4050; reactive master 4016 completes the score
  table). The WAVE 1 `p378` pack closes the guest half. Still OPEN: a second
  exe as master, both members' frames/wire legs and the 4014/4015/4016/4018
  pair under one trace id.
- `E-LIFE` — controlled 7KEY replay with deliberate misses and chart-timed
  strikes, pairing life `0x3970`, 4014 type 0, 4015 echo and the early
  Result edge in one pack.
- `E-HANDLER` — REA dossier for the S2C 4015 candidate at file `0x15fa20`
  (member slot setters `0x140e0`/`0x140c0`), then a two-client frame pair.
- `E-864` — replay the exe's default 7KEY song 864 chart (not the 757 chart
  used incidentally in s8) for exact judged-count parity.

## 2. Matrix (23 actions × 5 observer actors = 115 cells)

Cell key: `EVIDENCED` / `GAP` / `LIVE-FILLED` / `OPEN-FIX` / `NA-DESIGN` / `OWNER-PARKED` / `P3-PARKED`.
Every gap names its next evidence step. Cell outcome is the exe-observed
outcome; our client's guards are never the claim. `self:` and `other:`
follow the template. Cross-state rows name the seam instead of pretending
the board paints a highway.

| Action (raised by) | solo | master | peer | channel | server |
|---|---|---|---|---|---|
| **ENTER-SOLO** (self:solo, entry: 4010 in a one-seat room) | EVIDENCED — `<START>` → highway + notes appear after 4011/4012/4013/4048 → pack `s2` `b1bb42cb…`, `s8` `d987d801…`, wire `77a9494a…`; static scroll `0x56e1cb`–`0x56e3d3` | NA-DESIGN — no peer seat in this entry | NA-DESIGN — no peer exists | NA-DESIGN — board observer cannot see the in-client highway | EVIDENCED — `<4010 len0 / 4011 len8 / 4012 / 4013 / eleven 4048>` → play gate answered → wire `77a9494a…:36-62` |
| **ENTER-BATTLE-MASTER** (self:master with a peer, entry: 4010 with 2 members) | NA-DESIGN — solo entry is a different row | EVIDENCED — #632 CLOSED — maxwave W1 master highway f6d99cf4… at +8.7 s — #636 CLOSED — two-member 4010/4011/4012/4013 in ONE trace (w1-05/-06) ; FINAL SWEEP #382 (prior GAP): GAP — no qualified two-client master Playing frame; OPEN — `E-MULTI / #378`: drive a second exe as master, capture its highway and 4010 gate with both member seats | EVIDENCED — #634 CLOSED — peer Playing highway gate-frame-0368-playing.png captured — #638 CLOSED — peer-side 4011/4012/4013 bodies, both sockets, one trace (S294-RESULT-MULTI-COMPLETION-01) ; FINAL SWEEP #382 (prior GAP): GAP — no peer-side view of the master's start; OPEN — `E-MULTI / #378`: same trace id, peer frame at START | NA-DESIGN — Playing is in-client | EVIDENCED — #636 CLOSED — two-member 4010/4011/4012/4013 in ONE trace (w1-05/-06) — #638 CLOSED — peer-side 4011/4012/4013 bodies, both sockets, one trace (S294-RESULT-MULTI-COMPLETION-01) ; FINAL SWEEP #382 (prior GAP): GAP — per-member 4010/4011/4012 fan-out unpinned; OPEN — `E-MULTI / #378`: tap both sockets at the same START checkpoint |
| **ENTER-BATTLE-PEER** (self:peer in a master's room, entry: S2C 4010) | NA-DESIGN — no peer offline | EVIDENCED — #378 CLOSED — bot-master play chain dae62a5: guest 4050 → Highway pixels → 4014 drain; pack 96345ebb… — #632 CLOSED — maxwave W1 master highway f6d99cf4… at +8.7 s ; FINAL SWEEP #382 (prior GAP): GAP — master's view of the peer crossing into play is unpinned; OPEN — second-exe master half of `E-MULTI / #378` | EVIDENCED — `<S2C 4050 then first play pixels>` → guest Highway at t+2.5 s, lane dark 0.885 / HUD bright 0.007, with `C2S 4012` and `C2S 4014` in the same pack → `guest-playing-entry.png` `98aa85b8…`, wire `99f36051…:41,67-69` | NA-DESIGN — board has no play sheet | EVIDENCED — `<start handshake>` → `4011 len=8 0000000000000000`, guest `4012 len=5 0000000001`, `4013` + last `4048 last=1`, then `S2C 4050 len=6 010100000000` → wire `99f36051…:28-67` |
| **KEYMODE-7KEY** (self:solo/master, entry: 07D4 create) | EVIDENCED — `<7KEY plate lit>` → `07D4` key-mode byte `02`, seven caps and 28×7 note bars → pack `s8` `4a0ae4fe…`/`d987d801…`, wire `77a9494a…` | EVIDENCED — `<master 7KEY create>` → lit plate and `C2S 07D4 … 0002 …`, then WAITING row → this wave pack `ed13d762…`, wire `701297d1…` | P3-PARKED — #653 OPEN — exact 7KEY chart parity against song 864 — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — no peer view of a 7KEY highway; OPEN — `E-MULTI / #378`: guest frame after a 7KEY START | NA-DESIGN — key mode is a create-form choice, not a board field | EVIDENCED — `<07D4 body>` → key-mode word at byte 6 = `02` → wire `77a9494a…:23`, run2 wire `701297d1…:23` |
| **HIGHWAY-LANES** (self:solo, entry: first Playing frame) | EVIDENCED — `<play pixels>` → uneven lane columns `(5,28)…(165,28)`, judgement rule y≈296, note bars 28×7 → pack `s8` `d987d801…`, crop `caf7a8bc…`, dossier A26 `ev_f4662376…` | EVIDENCED — #632 CLOSED — maxwave W1 master highway f6d99cf4… at +8.7 s — #378 CLOSED — bot-master play chain dae62a5: guest 4050 → Highway pixels → 4014 drain; pack 96345ebb… ; FINAL SWEEP #382 (prior GAP): GAP — no master frame with a peer seated; OPEN — `E-MULTI / #378` | EVIDENCED — #634 CLOSED — peer Playing highway gate-frame-0368-playing.png captured ; FINAL SWEEP #382 (prior GAP): GAP — no peer frame with both member lanes addressed; OPEN — `E-MULTI / #378` | NA-DESIGN — local paint, no board row | NA-DESIGN — no wire field carries a lane column |
| **KEY-DOWN-HEAD** (self:player, entry: note at judgement row) | EVIDENCED — `<DIK stroke timed to a note>` → cap lights and judged tally changes; routing inversion `0x1714b0`/`0x176c42` → pack `s8` input `951e0787…`, dossier `ev_c01db4a8…`; arrow/statics in `docs/gameplay.md` | P3-PARKED — #625 OPEN — exe key-down/head and key-up/tail routing — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms (DI table pin #627 CLOSED — DI scancode→lane table pinned statically; live sweep rides #699) ; FINAL SWEEP #382 (prior GAP): GAP — no two-client head-routing frame; OPEN — `E-MULTI / #378` plus `E-864` | P3-PARKED — #625 OPEN — exe key-down/head and key-up/tail routing — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — no peer head-routing frame; OPEN — same replay on the guest leg | NA-DESIGN — local input path | NA-DESIGN — key-down is not a gateway frame |
| **KEY-UP-TAIL-HOLD** (self:player, entry: armed long-note release) | EVIDENCED — `<release>` → tail rated with the shared 6/18/25/≥26 windows; arming `+0x864[lane]`, files `0x176e60`/`0x176646`–`0x176709`, hold bonus +1 combo/tick → `docs/gameplay.md` B11/B12 | P3-PARKED — #625 OPEN — exe key-down/head and key-up/tail routing — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms; hold/tail arm #647 ; FINAL SWEEP #382 (prior GAP): GAP — battle hold state unpinned; OPEN — `E-MULTI / #378`: hold chart on both clients | P3-PARKED — #625 OPEN — exe key-down/head and key-up/tail routing — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms; hold/tail arm #647 ; FINAL SWEEP #382 (prior GAP): GAP — same for peer; OPEN — `E-MULTI / #378` | NA-DESIGN — tail rating is in-client | P3-PARKED — #625 OPEN — exe key-down/head and key-up/tail routing — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms; hold/tail arm #647 ; FINAL SWEEP #382 (prior GAP): GAP — `longNote` word exists in 4014/4016 wire but its two-client echo unpinned; OPEN — `E-MULTI / #378`: tap both legs during one hold |
| **JUDGEMENT-WINDOWS** (self:server judgement, entry: absolute song-note time difference) | EVIDENCED — `≤6/≤18/≤25 ms` → Cool/Good/Bad, else Miss; flags `0x10/0x20/0x40/0x80` → file `0x176540` (ev `14992eb2…`), sites `0x176646`/`0x176b7f`; s8 tallies 1/1/1/20 → pack `7c99c2b7…` | P3-PARKED — #653 OPEN — exact 7KEY chart parity against song 864 — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — no battle-member tier frame; OPEN — `E-MULTI / #378` + `E-864` | P3-PARKED — #653 OPEN — exact 7KEY chart parity against song 864 — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — no peer tier frame; OPEN — same | NA-DESIGN — judgement is inside the play state | EVIDENCED — `<4016 body>` → first four u32 `01 00 01 00 01 00 14 00` → wire `77a9494a…:69`; layout `docs/net-protocol.md:170` |
| **JUDGED-REPLAY-7KEY** (self:solo, entry: chart-timed DIK schedule) | EVIDENCED — `<40 hits from level-1 757, shifted −400 ms>` → COOL 1 / GOOD 1 / BAD 1 / MISS 20, life `0x7c`, score `0x7c` → input `951e0787…`, frame `7c99c2b7…`, wire `77a9494a…` | P3-PARKED — #653 OPEN — exact 7KEY chart parity against song 864 — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — no master replay with a peer present; OPEN — `E-MULTI / #378` + `E-864` | P3-PARKED — #653 OPEN — exact 7KEY chart parity against song 864 — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — no guest replay with chart-timed strokes; OPEN — same | NA-DESIGN — no board observer for judged lines | EVIDENCED — `<4014/4016/4017/4018>` → life 124 and the same four tallies → wire `77a9494a…:67-71` |
| **LIFE-MISS-DRAIN** (self:server tick, entry: miss or Cool/Good during play) | EVIDENCED — `<miss>` → life word `+0x3970` drains by the difficulty delta (Normal Miss −40), gauge stripe x≈236–245 falls; no-strike s7 hit 0 → pack `211f5acb…`/`s2`, dossier `feebe490…`; owner pin | P3-PARKED — #643 OPEN — per-member life drain echoed through 4015 — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms; peer arm already EVIDENCED by #378 ; FINAL SWEEP #382 (prior GAP): GAP — each member's life drain unpinned; OPEN — `E-LIFE` under `E-MULTI / #378` | EVIDENCED — `<all misses in a two-member room>` → guest `C2S 4014` life `e803`=1000 → `0000` in 12 ticks, each echoed as `S2C 4015 member=01`; HUD tally `MISS 32` → wire `99f36051…:68-91`, frame `9d31b6e7…` | NA-DESIGN — gauge is in-client | EVIDENCED — `<4014 type=0 value=u16>` → life 124 in s8, 0 in s7 → wire `77a9494a…:67`, `211f5acb…:65` |
| **LIFE-DRIVE-JAM** (self:server judgement, entry: Cool/Good tail) | EVIDENCED — `<Cool/Good>` → Jam +2/+1 in the score path, 4014 type 1 on tails files `0x16e660`/`0x16e7dd`; life adder first (`0x17057c`/`0x1705f6`) → `docs/gameplay.md`, `docs/net-protocol.md:170` | P3-PARKED — #645 OPEN — Jam on Cool/Good tails with live 4014 type-1 body — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — no battle Jam exchange frame; OPEN — `E-LIFE` under `E-MULTI / #378` | P3-PARKED — #645 OPEN — Jam on Cool/Good tails with live 4014 type-1 body — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — same on guest; OPEN — same | NA-DESIGN — Jam gauge is in-client | EVIDENCED — `<4014 type=1>` → wire shape and send sites pinned; no solo pack has a type-1 body (OPEN — `E-LIFE` for the live body) |
| **LIFE-DEATH-EARLY-RESULT** (self:server, entry: life reaches 0 before chart end) | EVIDENCED — `<all notes missed>` → bar empty, scorer freezes all tiers (`0x17051c`/`0x170557`), early 4016/4017/4018 → s2 wire `e4573ba2…`, s7 wire `211f5acb…`; owner pin | P3-PARKED — #643 OPEN — per-member life drain echoed through 4015 — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — battle death per member unpinned; OPEN — `E-LIFE` under `E-MULTI / #378` | P3-PARKED — #643 OPEN — per-member life drain echoed through 4015 — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — peer death/remaining-member continuation unpinned; OPEN — same | NA-DESIGN — no board observer of a death | EVIDENCED — `<4016 life byte 00, miss 20>` → 4017 slot 00 then 4018 → wire `211f5acb…:67-69` |
| **PEER-LIFE-ECHO-4015** (self:server, entry: S2C 4015 during play) | NA-DESIGN — one seat has no peer echo | OWNER-PARKED — #663 OPEN — 4015 handler eight rank bytes ; FINAL SWEEP #382 (prior GAP): GAP — master's peer gauge from 4015 unpinned; OPEN — `E-HANDLER`, then `E-MULTI / #378` | OWNER-PARKED — #663 OPEN — 4015 handler eight rank bytes ; FINAL SWEEP #382 (prior GAP): GAP — guest's master gauge unpinned; OPEN — same | NA-DESIGN — gauge is in-client | EVIDENCED — `<4015>` → u8 member + type + u16 value + 8 rank bytes → `docs/net-protocol.md:170`; handler candidate file `0x15fa20` unlocated, s8 body `77a9494a…:68` |
| **SONG-CLOCK** (self:server timer, entry: Playing update tick) | EVIDENCED — `<tick>` → exe paces on `timeGetTime` (`0x791d70`, B38 `efdaac19…`); our sample clock is the ADR-0001 deviation → `docs/scroll-map.md`, `docs/parity/playing.md` | OWNER-PARKED — #659 OPEN — two-member clock alignment at one wall trace ; FINAL SWEEP #382 (prior GAP): GAP — two-member clock alignment unpinned; OPEN — `E-MULTI / #378`: both frames at one wall trace | OWNER-PARKED — #659 OPEN — two-member clock alignment at one wall trace ; FINAL SWEEP #382 (prior GAP): GAP — same on guest; OPEN — same | NA-DESIGN — local clock, no board state | NA-DESIGN — no wire frame carries song position |
| **BGA-HUD-ART** (self:solo, entry: Playing frame with stage scene) | EVIDENCED — `<play pixels>` → BGA video behind the highway, JAM/COOL/GOOD/MISS + TIME strip, seven lit caps → pack `s2` `b1bb42cb…`, `s8` `d987d801…`, `docs/parity/playing.md` Composition | EVIDENCED — #632 CLOSED — maxwave W1 master highway f6d99cf4… at +8.7 s — #378 CLOSED — bot-master play chain dae62a5: guest 4050 → Highway pixels → 4014 drain; pack 96345ebb… ; FINAL SWEEP #382 (prior GAP): GAP — no two-client master art frame; OPEN — second-exe master half of `E-MULTI / #378` | EVIDENCED — `<guest play frame>` → city BGA video behind the three-lane highway, JAM/COOL/GOOD/BAD/MISS + TIME strip, `1st`/`2nd` rank plates → `guest-playing-entry.png` `98aa85b8…`, later `guest-play-08.png` `9d31b6e7…` | NA-DESIGN — art is in-client | NA-DESIGN — no wire field selects HUD art |
| **SCORE-SUBMIT-4016** (self:solo, entry: end or death) | EVIDENCED — `<4016>` → u16 cool/good/bad/miss/maxCombo/jam/maxJam + u32 score + u8 life + speed + i32 longNote → s8 body `010001000100140000005b6300007c000000000100000000`, wire `77a9494a…:69`; layout `docs/net-protocol.md:170` | OWNER-PARKED — #640 OPEN — master's 4016 field row + Result sheet with peer ; FINAL SWEEP #382 (prior GAP): GAP — battle master's own 4016 row unpinned; OPEN — `E-MULTI / #378` | OWNER-PARKED — #641 OPEN — peer's 4016 row + Result verdict frame ; FINAL SWEEP #382 (prior GAP): GAP — peer's 4016 row unpinned; OPEN — same | NA-DESIGN — submit is not board-visible | EVIDENCED — `<C2S 4016 len24>` → server accepts and answers 4017/4018 → wire `77a9494a…:69-71` |
| **RESULT-TRANSITION** (self:server, entry: 4016 then 4017/4018) | EVIDENCED — `<4017 slot 00 / 4018 len 50>` → Result SCORE/`GOOD!`/one row, RETRY/BACK → pack `3ea6c481…`, wire `97ebe726…`, `docs/lifecycle.md` §10 | P3-PARKED — #672 OPEN — peer own Result row / sheet — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — two-member Result sheet unpinned; OPEN — `E-MULTI / #378` + #294 `S294-RESULT-MULTI-02` | P3-PARKED — #672 OPEN — peer own Result row / sheet — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — peer Result sheet unpinned; OPEN — same | NA-DESIGN — Result is in-client | EVIDENCED — `<4018 45-B row, master slot byte>` → score rows admitted to the member table → wire `77a9494a…:71`, files `0x15fbe0`/`0x1411a0` |
| **DI-SCANCODE-MAP** (self:player, entry: keyboard setup) | EVIDENCED — `<DIK 1f/20/21/39/24/25/26>` → caps `S D F SPACE J K L`; `Option.ini` table `0x173290` (ev `fc65241f…`), poster `0x544b50`/`0x5429b0`; s8 input `951e0787…` | P3-PARKED — #625 OPEN — exe key-down/head and key-up/tail routing — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms (DI table pin #627 CLOSED — DI scancode→lane table pinned statically; live sweep rides #699) ; FINAL SWEEP #382 (prior GAP): GAP — battle lane map with two clients unpinned; OPEN — `E-MULTI / #378` + `E-864` | P3-PARKED — #625 OPEN — exe key-down/head and key-up/tail routing — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — same on guest; OPEN — same | NA-DESIGN — keyboard map is local config | NA-DESIGN — no opcode carries a scancode |
| **NOTETAIL-STYLE** (self:local option, entry: `NoteTail` 0..=4) | EVIDENCED — `<option index>` → 8-byte float-pair table `0x67fe8c` loads into the Playing display object at `+0xfec`, divides by 160 and clamps 0..120 → `S-PLAY-B2`, files `0x16b8f0`/`0x16bf40`/`0x16bfb0` | EVIDENCED — same local option consumer, no role gate → `S-PLAY-B2` | EVIDENCED — same local option consumer → `S-PLAY-B2` | NA-DESIGN — option is not board state | NA-DESIGN — no wire field carries `NoteTail` |
| **OJM-BANK-LOADER** (self:boot, entry: sound-device init) | EVIDENCED — `<boot>` → one call loads one of seven OJM banks; sole caller chain from PE entry → `S-PLAY-B1`, file `0x550db0` (corrects the old per-frame-effect label) | EVIDENCED — same boot-time call chain, no role gate → `S-PLAY-B1` | EVIDENCED — same boot-time call chain → `S-PLAY-B1` | EVIDENCED — observer exe sha256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`, REA `ev_b9fd1f27433ada2023ce7cb81fa25f6f828821a93b11eb7540478c5b6a562636` traces `0x550db0 → 0x550ae0 → 0x5a5710 → 0x5ad350 → 0x5fb1d8 → 0x5fb3b8` (PE entry); bank names `ev_a9fb8aefefa70e2a45105090df82976fd44527a81a7029fd83d4f64e1a2bfd3e` | NA-DESIGN — local sound bank, no gateway frame |
| **RETRY-PLAY-AGAIN** (self:solo, entry: settled Result rect 6) | LIVE-FILLED — `<click (667,581)>` → new empty `C2S 4010`, then personal-room pixels, not a Waiting seat grid → #294 wire `870ba02f…`, frames `4a89faa5…`/`74e6a2ae…`/`c2ad594a…`; contradicts the older checked #294 story | OWNER-PARKED — #661 OPEN — battle RETRY with a ready sibling ; FINAL SWEEP #382 (prior GAP): GAP — battle RETRY with a ready sibling unpinned; OPEN — `E-MULTI / #378` + #294 battle retry leg | OWNER-PARKED — #661 OPEN — battle RETRY with a ready sibling ; FINAL SWEEP #382 (prior GAP): GAP — peer RETRY refusal view unpinned; OPEN — same | NA-DESIGN — button is in-client | LIVE-FILLED — `<second 4010>` → second 4016/4017/4018 round → #294 wire `870ba02f…` |
| **TWO-CLIENT-ROSTER** (other:peer joins master room, entry: Waiting) | NA-DESIGN — no peer in a solo room | OWNER-PARKED — #655 OPEN — two-client member roster with both frames ; FINAL SWEEP #382 (prior GAP): GAP — no qualified exe master frame of the joined seat after 3004; OPEN — second-exe master half of `E-MULTI / #378` | EVIDENCED — `<hover-verified 0BBA>` → guest Waiting frame paints the master seat `t207probe` with READY and the guest seat `tester` orange; `S2C 3003 len=183` and `4009 body 0101` in the same trace → `waiting.png` `36a7fa4b…`, wire `99f36051…:21-24` | NA-DESIGN — board observer cannot see a Waiting seat | EVIDENCED — `<0BBA>` → `3004` 103 B to master, `3003` 180 B to guest, `4009` body `0101` to both → p379 taps `b07d63f8…`/`8e8735de…`; #379 guard fix is not the exe claim |
| **ROOM-ROW-FANOUT** (other:master creates, entry: peer on Room board) | NA-DESIGN — solo creation needs no second client | EVIDENCED — `<07D4>` → master's own Waiting row; this wave pack `399cea04…`, wire `701297d1…` | EVIDENCED — `<2003>` → second exe paints the master's WAITING row; this wave `room.png` `0eebef4d…`, wire `19bfc66d…`; the later join click failed, so no Playing claim | NA-DESIGN — no channel poll in this leg | EVIDENCED — `<2003 len=2417>` → body contains `PLAYMULTI` and state WAITING → wire `19bfc66d…:16`; WAVE 1 create wire `e326e3f7…` |

Counts after #382 final sweep: **115 cells** — 49 EVIDENCED, 2 LIVE-FILLED, 36 NA-DESIGN, 9 OWNER-PARKED, 19 P3-PARKED. Every disposition names a pack/pin/ticket or an owner item.
Prior wave counts and their GAP wording are retired; the disposition text above is current.

## 3. Simulation script and boot ledger

### S293-PLAYING-MULTI-01 (attempted twice, no multi-actor cell filled)

- `scenario_id`: `S293-PLAYING-MULTI-01`; `spec`: #293; `state`: Playing;
  `trace_id`: `story-playing-run1` / `story-playing-run2` in both manifests.
- `boot_pair`: `M = tester / uid 1 / master`, `P = tester2 / uid 2 / peer`,
  two exe clients, no host bot. No channel-observer socket was attached.
- `containment`: private Xvfb :350/:351, owned null sinks per leg, two
  distinct Wine prefixes, `--no-sound` not used (Playing audio captured on
  the owned sinks), `route=sink` / `on_default_sink=0` /
  `preboot_breach=0`, wire taps 15022/15023 → 15010, server PID 92326 /
  epoch 1794986056 re-read before each leg, stopped by recorded PID files.
- `steps`: seq 1 `P` boot → Channel (CP01); seq 2 `M` boot → Room (CP02);
  seq 3 `M` 7KEY create → Waiting, waits for S2C 4009 (CP03); seq 4 `P`
  Channel → Room → row double-click → Waiting → READY (CP04, **not
  reached**); seq 5 `M` START → play pixels (CP05, not reached); seq 6 both
  chart-timed DIK replay and 4014/4015/4016/4018 capture (CP06, not
  reached). The NORMAL-mode popup was probed and dismissed at (400,365) on
  every transition that raised it.
- `checkpoint`: CP01 guest `channel.png` + wire `19bfc66d…`; CP02 master
  `master-room-stage.png` + wire `701297d1…`; CP03 master `waiting-master.png`
  `399cea04…` + `C2S 07D4`/`S2C 07D6`; CP04 guest `room.png` `0eebef4d…`
  + `S2C 2003` with `PLAYMULTI`; CP05/CP06 absent — both attempts failed
  before a join.
- `outcome`: no expected-behavior sentence is written for a two-client cell.
  The only exe outcome from this leg is the single-actor one: "When the exe
  master creates a 7KEY room, its Waiting face stays up until a peer READY
  echo arrives" (pack `45ddafbd…`, wire `701297d1…`), and "When a second
  exe client lists that room, the master's WAITING row paints from `2003`"
  (pack `362239d4…`, wire `19bfc66d…`). Both are already in §2; neither
  closes a Playing multi-actor cell.

### Boot ledger (4 boots, cap reached)

| Boot | Display / sink / tap | Actor | Scene reached | Wire | Result |
|---|---|---|---|---|---|
| 1a | `:350`, `rig_null_3640270`, 15022 | master uid 1 `tester` | Room → Waiting | run1 master wire (not cited for exe behavior) | create half valid; peer-ready gate timed out because the guest continuation never ran (missing `__main__` guard). Failed attempt |
| 1b | `:351`, `rig_null_3636120`, 15023 | guest uid 2 `tester2` | Planet → Channel | run1 guest wire | failed attempt: no join stimulus; no cell |
| 2a | `:350`, `rig_null_3661508`, 15022 | master uid 1 `tester` | Room → Waiting | `701297d110…` | qualified create/Waiting pack (`45ddafbd…`); no `3004`/`4009`, no START. Multi-actor half failed |
| 2b | `:351`, `rig_null_3657361`, 15023 | guest uid 2 `tester2` | Planet → Channel → Room | `19bfc66d…` | qualified Room-row pack (`362239d4…`); row double-click emitted no `0BBA`. Multi-actor half failed |

Every boot used a private Xvfb and an owned null sink with the fields above;
the NORMAL-mode popup was dismissed at each transition that raised it.
Both legs started only after a 60 s idle poll of `ss -tnp | grep 15010`;
the shared Memoryer was never restarted or signalled. No other lane owned
15010 during these legs. All processes stopped by their own PID files.

### WAVE 1 #378 addendum boot (one guest leg, fixed bot master)

| Boot | Display / sink / tap | Actor | Scene reached | Wire / pack | Result |
|---|---|---|---|---|---|
| p378-1 | `:355`, `rig_null_3899753`, 15012 | guest uid 1 `tester`; bot master uid 3 `t207probe` | Channel → Room → Waiting → READY → Playing (45 s hold) | `99f36051…` / `aae3d446…` | PASS: `S2C 4050` after the master's 4048 burst, guest Highway pixels, 4014 life drain and 4015 echoes; all notes missed, no 4016/4018 |

The 15010 poll was idle before this boot; popup dismissed at every
transition; live audio route re-checked while Playing; `cite --claim exe`
PASS; Memoryer PID 92326 / epoch 1794986056 unchanged; all processes stopped
by their PID files. This is a bot-master + one-exe leg, not a two-exe cell.

## 4. Gap-to-story hand-off and contradictions

Ranked by player visibility × reachability (the guest-half rows above are no
longer GAP; the master-half rows still are):

1. Two-client Highway and member stats (15 cells): the fixed bot master now
   releases 4050 and the guest half is EVIDENCED from the `p378` pack. The
   remaining cells need a second exe as master, plus both members' frames
   and the 4016/4018 pair. The second exe join click is still the immediate
   blocker: run2 painted the master's WAITING row but emitted no `0BBA`.
2. Life-bar family (6 cells): `E-LIFE` with deliberate misses; s7/s2 pin
   the empty-bar early Result, s8 pins a surviving bar at life 124. Owner
   pin: missing ALL notes empties the bar and ends the song early.
3. 4015 peer echo (3 client cells): `E-HANDLER` at file `0x15fa20`, then
   the two-client frame pair. Server wire shape is pinned; the exe handler
   is still unlocated, so no client cell is promoted on wire alone.
4. Judged-input and NoteTail cells (6 cells): s8 pins the solo 7KEY lane
   map and a judged tally; `E-864` replaces the incidental 757 chart replay
   for exact parity. `NoteTail`/`0x550db0` static classification is done.
5. Retry and Result cross-state cells (5 cells): #294's battle replay leg
   owns these; no new Playing boot is justified before #378 lands.

Contradictions and limits kept open:

- The old "per-frame bg-effect loop at `0x550db0`" is refuted by
  `S-PLAY-B1`: a boot-time seven-bank OJM loader with one caller chain from
  PE entry. The "double/hold-art style table at `0x67fe8c`" wording is
  corrected by `S-PLAY-B2`: an 8-byte float-pair option table consumed by
  the Playing display object. This matrix grades the corrected pins.
- s7's `judged-strike` line was a Result-screen false positive (delta from
  a full-screen change after 4014), not a Playing strike; s6's vkey strokes
  and s7's DIK sweeps did not coincide with notes. Only s8 is the judged
  strike pack. The 40-event replay used 757 chart times against the exe's
  default 864 song, so its COOL/GOOD/BAD counts are incidental, not exact
  chart parity (`E-864`).
- The song clock is a known owner-approved deviation (ADR-0001: exe wall
  clock, Rust sample clock). It is not graded OPEN-FIX here; no parked fix
  exists for it. The two-client alignment arm stays GAP.
- #379's merged member guards and two-client member matrix are green, but
  green guards are not exe observations. Both run1/run2 attempts show the
  wire roster and one client's pixels; neither produced a two-client Playing
  frame. The `p378` leg adds the guest half with the fixed bot master; the
  second-exe master half remains OPEN.
- The run2 guest saw the master's room in `2003` and painted its WAITING
  row, but the join click produced no `0BBA`. Negative evidence is recorded
  as a failed attempt; no two-client cell is filled from it.
- f1's stopped-state `CStateNormalPlaying` proves a state object, not a
  painted highway (`attach316.log`); its wire has no guest `4014`. The
  solo master packs supersede it for single-player claims only.
- Server state is shared by one Memoryer (PID 92326); no "scenario delta"
  dismissal is used without a wire tap and a per-client frame. No
  copyrighted bytes enter the repo: all packs stay in git-ignored scratch.

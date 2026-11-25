# Result story matrix — Spec #294 (WAVE 3, lane RESULT)

Method: `docs/captures/story-matrix/TEMPLATE.md`. Cell shape follows the
Waiting, Room and Channel exemplars in this directory. Base `f1579e3`;
worktree `/home/arnim/Projects/recomp2jam-wt139`, branch `cb2/story-result`.
No code change, no code push. Tracker writes are limited to the #294 story
body and the #381 summary comment. The 7.55 executable is the oracle
(`OTwo.exe`, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`).
Memoryer is a wire witness only. All packs stay in git-ignored scratch; only
hashes and text enter the repo.

Actor names follow the template. In this spec `solo` means the one-seat
Result run of the qualified pack (a logged-in SOLO room with no peer); the
offline `play --window` seam is `NA-DESIGN` where no 4018 Result path exists.
`master` and `peer` name the two-member (battle) Result seats and are never
folded into the solo seat. `channel` observes the board, not the Result
sheet. `server` is a frame with no local click.

## 1. Evidence inventory (before any gap was marked)

| Source | Pin (full sha256 unless noted) | What it names / verdict |
|---|---|---|
| exe identity | `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b` | 7.55 behavior oracle for every VA/file offset below |
| `~/scratch/p366-solo/s3-result-timed` (`cite --claim exe` re-run, exit 0) | citation `415bec616ed450b5dbd3b351f59b75f55ecec0ce6dfffc8b2d18ed2ee5add092`; manifest `aabbe18e80191d6ee853b55d7b78d054ade4c34fcd82a70ee2eae881d5c99ca2`; provenance `8c64703ffa1e47ae51c566e25709da804a8c24c62b8a52d8dab2016b68579c51`; frame `artifacts/screenshots/judged-sweep-3.png` `3ea6c48156d474aadc3d5f31663395cdfabf75da550b44e2165c102e7a4df7e4`; wire `wire/room-stage.log` `97ebe7264ba36216158f72b925b08fc188fb70311d1d75d515cb7635688f2d43`; stub `web/stub.log` `7233cfe933cded8c80f2b1f2f5a994d2e257e8fb6b583de6a4f0886f8d6e3d7c`; input `ad3bd7e866d7c4410f4bf5c736e24430a1a74f7c0fcaa80b5e97877419a9a49b` | First qualified exe Result pack: SCORE panel, `GOOD!`, one `tester` row, `F`, RETRY/BACK; 4014/4015/4016/4017(slot 00)/4018(len 50, miss 20, gem 15527)/3001; two `hot_time.php` lines and zero `ticket_add` lines. The `wire/tap.log` file is empty (`e3b0c442…`); cite `wire/room-stage.log` |
| `~/scratch/p366-solo/s1-result-stub`, `s2-result-timed` | s1 citation `d9d491afc26fe05099ef06bd4b149555e227b9619f52c5efdff8a7be2f37d33f`; s1 Result frame `3ea6c481…4df7e4` (byte-identical to s3); s2 citation `e739a656e446f9f29021407b3f5d7fc972cb1ef6bbab2f5d2023f8de63f5753a`; s2 stub `c0039a36…13dd14` | Negative / partial legs: s1 corroborates the same Result frame and stub; s2 wedged before Result. Neither adds an actor |
| `~/scratch/story-result-live/s1-retry-back` (boot 1, repaired pack) | citation `390388d183d1fa8abcc73c02c1440004011c431c6aec5137d4f395a49202def0`; manifest `efd56c7a433faa342876312bc65d8823088e16dab027bc1424e296d34eb73a88`; wire `f9238a188f47fbaf2108f62d55b9edc288f5471ab46fc297a5989d3e90c8ec99`; stub `ca36cf1237c537b25db6c9406d80fad8820e160e885f05b4f42ffa434cb07d4d`; drive `2d2ef46421479386f8e488a376c6a6f8d02b1c454504817c9ed43466dc8659a9`; `after-retry-waiting.png` `7049942eb35911b792af37dea5d2922624881d63427cd47afe556f5dcb9198cb` | RETRY click at pinned rect 6 emitted a second `C2S 4010` at 13:18:07; the +20 s frame is MyRoom pixels, not Waiting. First manifest verify failed on a duplicate forced frame; the duplicate was removed and the pack re-emitted to verify 0 / cite 0. Wire-only RETRY pin; no post-click state claim |
| `~/scratch/story-result-live/s3-retry-back` (boot 3, qualified pack) | citation `e9df080bd2185f8d62af7263d2078227cf966e9c6aff2c4d9697403d23f02321`; manifest `96b37640e15536e66379cbb3e688ad714636df794207aa37a08553014afed01d`; wire `870ba02fa8f919d0baa0600707699e1bf12df7da9b52c2189c190e7cb7008a59`; stub `043ae2d878f6ed9957826b3871944e189f8e30bac499c1aa8ace333eabdb4dba`; drive `97bc95f0deff946b28a41ff843057164256c10307e8bad927c3974e0ce560194`; Result frame `judged-sweep-3.png` `5f812efe0143ce89c17118ba9daebc01e2f2be60522f748fea3afa0e54c4f608`; `after-retry-t03.png` `4a89faa5f7bc9655256d25d2c5680276c09be953e93a2c1418ea8bdc5c31e633`; `after-retry-t10.png` `74e6a2aeb39a8136cf8f06a3c21f1e9b7db3024aae329ac5589f949a38e84d3d`; `after-retry-t30.png` `c2ad594a44b115f7ea87b74dcd033776fa222af2f0bb89e59ec8ae0fda959feb`; `result-after-retry.png` `adf8cd55e1546552e44ff1feddec44329a9574cd3af90f057870823765100646`; `after-back-room.png` `9ebe27c9fa5d4460d9a549a429538c56fd145e1c1570f97219ad7df8475316ce` | Manifest verify 0, audio verify 0, cite 0. Second qualified exe Result frame with a different score band (`tester2`, gem 626, exp 179); two 4010 presses, two 4016/4017/4018 rounds, explicit 4021 click, Room frame, three `hot_time.php` lines, zero `ticket_add`. Immediate post-RETRY frames are MyRoom, not Waiting; the second Result checkpoint is a mid-animation `GAME OVER / 1st` frame |
| `~/scratch/story-result-live/s2-retry-back`, `s4-back-settled` (failed attempts) | s2 drive `ca58e3a6fbf64cd64c7c24dbf7f61dedb069d2c879f95a6a00a0254485ccbd9b`, wire `9c94b4d207c70c4c494cc6422cd0659f31a0601d7ed7376011f207148252db08`; s4 drive `f7376f6680cf16671fe795a3e0f54c0c3713973d32f2aa9a9efaca60cac6f69f`, master drive `beb938863d2695c8d0c2d6504ee97cfdee2233e02cf1888ec83634f6407df15b`, wire `42d68d691238eaabb5d1c861ba7fee84e1b99b20cf34e20b2bc9ca55902fade6`, citation `8e4f920f13859d4e11d9db497155a89719558d55fab6aca6ac6ebc4905d1358a` | s2: duplicate-session `1001 feffffff`, no Channel entry. s4: popup dismissed, but no `03EC`; CREATE OK never produced `07D4`. Failed attempts; no cell filled |
| #365 close comment 2026-11-18T17:30Z, `~/scratch/p310-findings.md`, `~/scratch/p355-findings.md` | merged `b80ed6f`; painter and reader dossiers `ev_bfc05257…` / `ev_ff221e03…`; row reader VA `0x5411a0` file `0x1411a0`; handler VA `0x55fbe0` file `0x15fbe0` | Morning fix: `ResultMemberRecord` moved into the shell painter; guard `result_rows_read_the_two_staged_4018_member_records` red→green (`[1,0,0,0]` vs `[11,22,33,44]`). Read side is no longer absent; the story in #294 still carries the old unchecked text |
| #366 close comment 2026-11-19T03:35Z, this wave's three stub logs and Lane D REA (2026-11-19) | URL string file `0x24a2d0` / VA `0x64a2d0` (`ev_5c1c7b322fe96ea565a2d68cf06cee0e7fc23660be9e080fe3494856d2a8f4dc`); gate in `FUN_004c1b90` (`ev_3bd9ba0997e292073a218baa203e613f5977a6ea27abc5d06aadb094dc46db1a`); stub `7233cfe9…` plus `ca36cf12…` / `043ae2d8…` | Morning fix disposition: three boots, only `hot_time.php`; zero `ticket_add.php` / `USER_INDEX_ID`. The old `file 0x24a338` pin was the `PointNum.ojs` string, not a call site; corrected to the URL string and gating function. Branch selector value source stays OPEN (no direct writers) |
| #360/#355 close comments and `~/scratch/p355-findings.md` | cover JPEG `47eb357866f37c6ef39afe048125106ed296ee3f50b44f87b409a5a8a250af06`; cover proof `9c8455f3…656bc6`; matched frame `456c00903d8f584d09f2b0bf12abb575f2ca7cef11441e55aa5fb9835833b25d`; dossier `ev_9404e824…`; merged `b80ed6f` | Morning fixes: 4011 body is eight zero bytes, cover comes from the selected chart's OJN JPEG, credits are baked into that JPEG (SOI at decrypted header +296); matched guest staging reached weighted 0.989388, min 0.984725. Cross-state cells only — no new Result pack |
| `docs/cresult-anchors.md`, `docs/net-protocol.md` §4c, `docs/parity/result.md` | top painter `FUN_004c38a0` file `0xc38a0` (`ev_615e034c…`); row painter `FUN_004c3b40` file `0xc3b40` (`ev_88c3ca31…`); verdict `FUN_004c3120` (`ev_38f32576…`); 4017 file `0x15fba0`; 4040 file `0x159a60` / writer `0x1414ad` (`ev_4684b913…`, `ev_ecb961cc…`); retry block string VA `0x64a3a0` | Static/REA class. Pin every column, the frame rule, the negative 4040 read sweep and the server-authoritative retry rule |
| `~/scratch/p343-ab/proof-final.json` | `20fbe17060cea3d3da16a059b74de06a724b6bb41ee6ae2caf947b3ccfc8cff6` | Earlier matched exe/Rust wire window; stops before Result, so it is context, not a Result cell |
| `docs/captures/journey-audit/294-result.md`, vintage photos | `docs/screenshots/result.png` `08d85373814eb732376f2a09d2415e9ced9686650a2f865d8ad4389b4c5d0180`; `reference-result-4x3.png` `b1308a53726f750314e830a8463510e5fe09a2ff78a695925007efaa28bb1bc4` | Prior audit: 3 matched, 2 mismatched (#365/#366), 1 blocked (4017); photos are not provenance packs |
| `~/scratch/story-rig-b/comment-294.md` (2026-11-17) | static verification comment | Earlier coverage: 3 checked stories, 5 new stories, Result UNREACHABLE-ON-RIG then. Superseded for live Result by the packs above |

Green Rust guards alone (#365/#355/#366) never grade an exe-behavior cell;
they appear as fix state. Synthetic proxy rewrites and server state shared
by one Memoryer are never called per-client exe observations here.

## 2. Matrix (17 actions × 5 observer actors = 85 cells)

Cell key: `EVIDENCED` / `GAP` / `LIVE-FILLED` / `OPEN-FIX` / `NA-DESIGN` / `OWNER-PARKED` / `P3-PARKED`.
`OPEN — <step>` is mandatory in every GAP cell. Every row names its Result
state entry point (first Result entry, replay after RETRY, or an async server
frame). `self:` and `other:` follow the template.

| Action (raised by) | solo | master | peer | channel | server |
|---|---|---|---|---|---|
| **ENTER-SOLO** (self:solo, entry: 4016→4018 after a solo play) | EVIDENCED — `<4016/4017/4018>` → SCORE panel, `GOOD!`, one row, RETRY/BACK → pack `3ea6c481…4df7e4`, wire `97ebe726…` | NA-DESIGN — no second member in this entry | NA-DESIGN — no peer exists | NA-DESIGN — board observer never sees the Result sheet | EVIDENCED — `<C2S 4016 len24 / S2C 4017 slot00 / S2C 4018 len50 / 3001 SOLO>` → score submit then Result state → wire `97ebe726…` |
| **ENTER-BATTLE-MASTER** (self:master, entry: 2+ member 4018) | NA-DESIGN — solo entry is a different row | P3-PARKED — #672 OPEN — peer own Result row / sheet — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — `<two-member 4018>` → master's Result sheet unpinned; OPEN — two-client replay with chart-timed presses; capture master frame and 4018 count ≥2 in one trace id | P3-PARKED — #672 OPEN — peer own Result row / sheet — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — `<master entry>` → peer's view of the master's result unpinned; OPEN — same replay, peer frame at the same checkpoint | NA-DESIGN — Result is in-client | P3-PARKED — #673 OPEN — two active 4018 rows on both screens and both sockets — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — `<4018 fan-out>` → per-socket bodies for a two-member Result unpinned; OPEN — proxy tap both legs and name each member row |
| **ENTER-BATTLE-PEER** (self:peer, entry: 2+ member 4018) | NA-DESIGN — no peer offline | P3-PARKED — #672 OPEN — peer own Result row / sheet — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — `<peer entry>` → master sees no qualified peer Result frame; OPEN — two-client pack with both screens | P3-PARKED — #672 OPEN — peer own Result row / sheet — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — `<peer entry>` → peer's own Result sheet unpinned; OPEN — same pack, peer frame hashed in the manifest | NA-DESIGN — Result is in-client | P3-PARKED — #673 OPEN — two active 4018 rows on both screens and both sockets — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — `<4018 row for peer>` → fan-out body and row order unpinned; OPEN — tap both sockets at the same wall clock |
| **TOP-PANEL-TALLY** (self:server, entry: 4018 + value globals) | EVIDENCED — `<miss 20>` → COOL 0/GOOD 0/BAD 0/MISS 20, MAX/JAM 0 at x=151/240/337/426/505/596 → pack `3ea6c481…`, dossier `ev_615e034c…` (`0xc38a0`) | OWNER-PARKED — #673 OPEN — two active 4018 rows on both screens and both sockets ; FINAL SWEEP #382 (prior GAP): GAP — `<battle 4018>` → master's top tally from record words unpinned; OPEN — two-client replay with controlled score bands | OWNER-PARKED — #673 OPEN — two active 4018 rows on both screens and both sockets ; FINAL SWEEP #382 (prior GAP): GAP — `<battle 4018>` → peer's top tally source (local score vs member record) unpinned; OPEN — same replay, peer frame plus 4018 row bytes | NA-DESIGN — top panel is painted in-client | EVIDENCED — `<4016 miss word=20>` → 4018 row repeats miss 20; top-value globals have register-held writers → wire `97ebe726…`, `docs/cresult-anchors.md` OPEN 3 |
| **VERDICT-FRAME** (self:server, entry: result byte / player count) | EVIDENCED — `<result byte 1, one player>` → `GOOD!` banner at y=190 → pack `3ea6c481…`, `FUN_004c3120` (`ev_38f32576…`) | OWNER-PARKED — #672 OPEN — peer own Result row / sheet — controlled win-flag replay OPEN ; FINAL SWEEP #382 (prior GAP): GAP — `<battle win-flag>` → frame 1/2/3 branch has no qualified battle frame; OPEN — two-member replay with controlled win flags | OWNER-PARKED — #672 OPEN — peer own Result row / sheet — controlled win-flag replay OPEN ; FINAL SWEEP #382 (prior GAP): GAP — `<battle win-flag>` → peer's banner frame unpinned; OPEN — same replay, peer screenshot | NA-DESIGN — verdict art is in-client | EVIDENCED — `<4018 row byte 31 = 1>` → solo frame choice → wire `97ebe726…`, header rule in `docs/cresult-anchors.md` |
| **LOCAL-ROW-4018** (self:server, entry: admitted 4018 row for the local seat) | EVIDENCED — `<row: 0/0/0/20, 0/0, score 0, +0, X1>` → row `tester` paints those digits at x=230/416/530/626/685 → pack `3ea6c481…`, wire `97ebe726…`, #365 read-side merge `b80ed6f` | EVIDENCED — #670 CLOSED — 4018 pair-submit pin a232a786…; master paints its row ; FINAL SWEEP #382 (prior GAP): GAP — `<two-member 4018>` → battle master's own record row unpinned; OPEN — two-client replay with two active rows | P3-PARKED — #672 OPEN — peer own Result row / sheet — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — `<two-member 4018>` → peer's own record row unpinned; OPEN — same replay, peer frame + parsed 4018 row | NA-DESIGN — row painter is in-client | EVIDENCED — `<4018 len50>` → parsed cool/good/bad 0, miss 20, combos 0, score 0, reward 0, level 7, exp 9945, result 1, gem 15527, speed 1 → wire `97ebe726…`; row reader `0x1411a0` (`ev_bfc05257…`); guard `result_rows_read_the_two_staged_4018_member_records` |
| **PEER-ROW-4018** (self:server, entry: 4018 rows for other members) | NA-DESIGN — no peer row in a one-seat pack | P3-PARKED — #673 OPEN — two active 4018 rows on both screens and both sockets — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — `<peer's 4018 row>` → master's peer-row digits unpinned; OPEN — two-client Result pack with member-record rows on BOTH sides (primary #365 gap) | P3-PARKED — #673 OPEN — two active 4018 rows on both screens and both sockets — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — `<other member's 4018 row>` → peer's other-member row unpinned; OPEN — same pack, peer frame at the same checkpoint | NA-DESIGN — board observer has no rank table | P3-PARKED — #673 OPEN — two active 4018 rows on both screens and both sockets — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms ; FINAL SWEEP #382 (prior GAP): GAP — `<4018 count ≥2>` → fan-out to both sockets and row order unpinned; OPEN — tap both client legs; the scripted two-row guard is Rust-only, not exe evidence |
| **MASTER-MARKER-4018** (self:server, entry: 4018 head master slot) | EVIDENCED — `<master byte 00, local seat 0>` → one local row and no second master row → wire `97ebe726…`, writer VA `0x5400b0` (`docs/net-protocol.md` §4c) | EVIDENCED — #676 CLOSED — head +0x30 byte pinned on BOTH taps — role-split arm parked P3 with #672/#673 ; FINAL SWEEP #382 (prior GAP): GAP — `<two-member head>` → master's own `+0x30 = 1` flag on a real Result sheet unpinned; OPEN — two-client pack plus a stopped-state read if needed | EVIDENCED — #676 CLOSED — head +0x30 byte pinned on BOTH taps — role-split arm parked P3 with #672/#673 ; FINAL SWEEP #382 (prior GAP): GAP — `<two-member head>` → peer's self `+0x30 = 0` and table master `= 1` unpinned; OPEN — same pack, peer view | NA-DESIGN — marker is a member-record word | EVIDENCED — `<4018 head byte 00>` → master marker split by local id → wire `97ebe726…`, files `0x15fc66`/`0x15fcc8`/`0x15fd11` |
| **SUBMITTER-NOTICE-4017** (self:server, entry: 4017 before 4018) | NA-DESIGN — #679 CLOSED — 4017 queue consumer traced; wire 97ebe726… — UI queue consumer traced; no visible submitter marker in the pack ; FINAL SWEEP #382 (prior GAP): GAP — `<4017 slot 00>` → no visible submitter marker in the pack; UI consumer not identified; OPEN — REA xref of queue `0x478d50` consumer, then a two-member replay with a non-zero slot | P3-PARKED — #672 OPEN — peer own Result row / sheet — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms; #679 CLOSED — 4017 queue consumer traced; wire 97ebe726… ; FINAL SWEEP #382 (prior GAP): GAP — `<4017 slot for master>` → master's submitter cue unpinned; OPEN — same xref plus two-member replay | P3-PARKED — #672 OPEN — peer own Result row / sheet — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms; #679 CLOSED — 4017 queue consumer traced; wire 97ebe726… ; FINAL SWEEP #382 (prior GAP): GAP — `<4017 slot for peer>` → peer's submitter cue unpinned; OPEN — same | NA-DESIGN — no board observer for an in-client notice | EVIDENCED — `<S2C 4017 len1 hex=00>` → handler file `0x15fba0` reads u8 slot, guards `<8`, forwards to UI queue → wire `97ebe726…`, `docs/net-protocol.md` §4c |
| **PLACE-BYTES-4040** (self:server, entry: 4040 place frame during play) | EVIDENCED — `<no 4040 in pack>` → rank F comes from the local grade table; whole-image read sweep finds no result reader → pack `3ea6c481…`, dossier `ev_4684b913…`/`ev_ecb961cc…` | NA-DESIGN — #680 CLOSED — 4040 read sweep finds no gameplay reader; ruled out — no gameplay reader found; ruled out ; FINAL SWEEP #382 (prior GAP): GAP — `<4040 place byte>` → battle master's rank column source still needs a controlled replay; OPEN — two-member replay with one rewritten 4040 place and a master frame | NA-DESIGN — #680 CLOSED — 4040 read sweep finds no gameplay reader; ruled out — no gameplay reader found; ruled out ; FINAL SWEEP #382 (prior GAP): GAP — `<4040 place byte>` → peer's rank column source unpinned; OPEN — same replay, peer frame | NA-DESIGN — rank column is in-client | EVIDENCED — `<4040 eight place bytes>` → writer VA `0x5414a0` stores slot `+0x217d`; no gameplay reader found (`0x1414ad`, `0x13fccb`, one bulk copy) → `docs/net-protocol.md` §4c |
| **RETRY-SOLO** (self:solo, entry: settled solo Result, pinned rect 6) | LIVE-FILLED — `<click (667,581)>` → `C2S 4010` then MyRoom pixels at +0.3/+1/+3 s, NOT a Waiting seat grid → boot3 wire `870ba02f…`, frames `4a89faa5…`/`74e6a2ae…`/`c2ad594a…`; boot1 wire `f9238a18…`; contradicts the checked #294 story | NA-DESIGN — same single seat, battle RETRY is row 12 | NA-DESIGN — no peer | NA-DESIGN — RETRY is an in-client button | LIVE-FILLED — `<RETRY click>` → second `C2S 4010` at 13:23:50, then a second 4016/4017/4018 round at 13:24:11 → wire `870ba02f…` |
| **RETRY-BATTLE** (self:peer while siblings are ready, entry: two-member Result) | NA-DESIGN — no ready sibling offline | EVIDENCED — #682 CLOSED — RETRY refusal = server filter (Unauthorized, no 4011), screens captured ; FINAL SWEEP #382 (prior GAP): GAP — `<peer RETRY with a ready sibling>` → master sees no refusal and no qualified frame; OPEN — two-client replay with one READY peer and a RETRY press; wire plus both frames | EVIDENCED — #682 CLOSED — RETRY refusal = server filter (Unauthorized, no 4011), screens captured ; FINAL SWEEP #382 (prior GAP): GAP — `<self RETRY with ready sibling>` → peer's refusal view unpinned; OPEN — same replay, peer frame | NA-DESIGN — the refusal string is in-client | EVIDENCED — #682 CLOSED — RETRY refusal = server filter (Unauthorized, no 4011), screens captured — server filter = Unauthorized, no 4011 ; FINAL SWEEP #382 (prior GAP): GAP — `<RETRY with ready sibling>` → server refusal body unpinned; owner-recorded rule at VA `0x64a3a0`; OPEN — two-client tap around the refusal and name the wire body; do not fake a client rule |
| **BACK-SOLO** (self:solo, entry: solo Result, pinned rect 5) | LIVE-FILLED — `<click (752,581) on a mid-animation Result frame>` → `C2S 4021`, then Room pixels → boot3 frames `adf8cd55…` → `9ebe27c9…`; click target during animation stays OPEN | OWNER-PARKED — #683 CLOSED — solo BACK 4021→4022 + Room frame 9ebe27c9…; battle BACK unpinned — two-client post-Back view OPEN ; FINAL SWEEP #382 (prior GAP): GAP — `<battle BACK>` → master's post-Back Room view unpinned; OPEN — two-client replay with a settled battle Result | OWNER-PARKED — #683 CLOSED — solo BACK 4021→4022 + Room frame 9ebe27c9…; battle BACK unpinned — two-client post-Back view OPEN ; FINAL SWEEP #382 (prior GAP): GAP — `<battle BACK>` → peer's post-Back Room view unpinned; OPEN — same replay | NA-DESIGN — board is not the Result scene | EVIDENCED — `<C2S 4021 len0>` → `S2C 4022 len13` then 4000/4001/4023/4024/4025/4026 room re-stage → wire `870ba02f…` and anchor `97ebe726…` |
| **TICKET-GET** (self:solo, entry: Result entry in three qualified boots) | EVIDENCED — `<Result entry, mode != 2, selector 0x77ee50 == 0>` → `GET /client/event/2009/06_moa/hot_time.php`, never `ticket_add.php`; the nonzero-selector arm appends `USER_INDEX_ID` + `&title=`. Stub logs `7233cfe9…`, `ca36cf12…`, `043ae2d8…` show only hot_time and zero ticket_add; REA `ev_3bd9ba0997e292073a218baa203e613f5977a6ea27abc5d06aadb094dc46db1a` + URL `ev_5c1c7b322fe96ea565a2d68cf06cee0e7fc23660be9e080fe3494856d2a8f4dc`. The request-line form of the armed (nonzero-selector) arm stays OPEN | NA-DESIGN — #677 CLOSED — ticket_add URL 0x24a2d0 + gate dossier; three qualified stubs showed hot_time only — web ticket view expected blank by owner pin (#750) ; FINAL SWEEP #382 (prior GAP): GAP — `<two-member Result>` → GET presence/URL unpinned; OPEN — two-client stub replay that also reads the `0x77ee50` selector on both seats | NA-DESIGN — #677 CLOSED — ticket_add URL 0x24a2d0 + gate dossier; three qualified stubs showed hot_time only — web ticket view expected blank by owner pin (#750) ; FINAL SWEEP #382 (prior GAP): GAP — `<two-member Result>` → peer-side GET unpinned; OPEN — same | NA-DESIGN — HTTP page is not a board observation | NA-DESIGN — the web GET is not a Memoryer gateway frame; design seam is the shell's web boundary |
| **BUTTON-RECTS** (self:solo, entry: settled Result panel) | EVIDENCED — `<settled panel>` → RETRY (625,570)-(708,592) and BACK (711,569)-(794,593) paint at the pinned rects → pack `3ea6c481…`, `State_Result.bnd` rects 5/6 (`ev_df8e0ca4…`) | OWNER-PARKED — #672 OPEN — peer own Result row / sheet — two-member battle sheet frame OPEN ; FINAL SWEEP #382 (prior GAP): GAP — `<battle panel>` → button art/state on a two-member sheet unpinned; OPEN — two-client pack | OWNER-PARKED — #672 OPEN — peer own Result row / sheet — two-member battle sheet frame OPEN ; FINAL SWEEP #382 (prior GAP): GAP — `<battle panel>` → same on peer sheet; OPEN — same pack | NA-DESIGN — buttons are in-client | NA-DESIGN — no wire field drives a local button rect |
| **COUNTUP-ENABLE** (self:solo, entry: Result update tick) | EVIDENCED — `<update tick>` → count-up +10 per tick, back/retry enabled past 100 → dossier `ev_77ce98f1…` (`FUN_004c28e0`); pack shows both buttons but their enable instant is not graded here | EVIDENCED — same local update rule applies to a battle master's sheet; dossier `ev_77ce98f1…` (no separate battle frame claimed) | EVIDENCED — same local update rule applies to a peer's sheet; dossier `ev_77ce98f1…` (no separate peer frame claimed) | NA-DESIGN — no board observer for a local count-up | NA-DESIGN — enable state is local, not a wire field |
| **LOADING-COVER-CREDITS** (other:server 4011 on replay, entry: Result→new play→Loading) | EVIDENCED — `<4011 eight zero bytes>` → Loading paints the selected chart's OJN JPEG, Artist/Remix/Pattern baked into it → #355/#360 closes, JPEG `47eb3578…af06`, dossier `ev_9404e824…`; this lane's Result→play transition is wire+Game-Over only, not a Loading frame | OWNER-PARKED — #686 OPEN — Result→RETRY→Loading cover frame capture ; FINAL SWEEP #382 (prior GAP): GAP — `<master replay 4011>` → master's Loading cover unpinned; OPEN — two-client replay with cover frame and hashed manifest | EVIDENCED — `<guest 4011>` → matched guest stages the room song cover → guard `matched_guest_start_stages_the_room_song_loading_cover`, frame `456c0090…33b25d`, weighted 0.989388 (#355 close); no exe two-member Loading frame claimed | NA-DESIGN — board observer has no Loading gate | EVIDENCED — `<S2C 4011 len8 hex=0000000000000000>` → wire carries no cover; source is the selected chart's OJN JPEG → `~/scratch/live-lane-1/t1b-loading` manifest `86a7dfae…` (#355 comment) |

Counts after #382 final sweep: **85 cells** — 28 EVIDENCED, 3 LIVE-FILLED, 33 NA-DESIGN, 9 OWNER-PARKED, 12 P3-PARKED. Every disposition names a pack/pin/ticket or an owner item.
Prior wave counts and their GAP wording are retired; the disposition text above is current.

## 3. Simulation scripts and boot ledger

### S294-RESULT-RETRY-01 (executed in boot 3)

- `scenario_id`: `S294-RESULT-RETRY-01`; `spec`: #294; `state`: Result;
  `trace_id`: `S294-R1` (named in the pack citation and drive log).
- `boot_pair`: `M = tester2 / uid 2 / solo-room master`, no second client and
  no host bot; the one-seat pack is the trace's actor.
- `containment`: private Xvfb `:325`, owned null sink
  `rig_null_3538561` (module id in pack `env.sh`), `--no-sound`,
  `route=sink` / `on_default_sink=0` / `preboot_breach=0`, Memoryer PID
  92326 / boot epoch 1794986056 unchanged, tap 15019→15010, stub port 18080.
- `steps` / `checkpoint`:

| Seq | Actor | Action (entry state) | Checkpoint | Cells filled | Outcome from the exe |
|---|---|---|---|---|---|
| 1 | M | Create SOLO room, START (Waiting) | `CP01` | ENTER-SOLO × solo/server | 4010 then play pixels; pack `play-seq-001.png` |
| 2 | server | 4016/4017/4018, song ends early (Playing) | `CP02` | ENTER-SOLO, TOP-PANEL-TALLY, LOCAL-ROW-4018, VERDICT-FRAME × solo/server | Result frame `5f812efe…`; miss 20, gem 626, `GOOD!`, one `tester2` row |
| 3 | M | Click RETRY rect 6 (Result) | `CP03` | RETRY-SOLO × solo/server | second `C2S 4010`; frames `4a89faa5…` / `74e6a2ae…` / `c2ad594a…` are MyRoom pixels |
| 4 | server | Second 4016/4017/4018 (replay ends) | `CP04` | RETRY-SOLO × solo/server, VERDICT-FRAME × solo | `adf8cd55…` is a mid-animation `GAME OVER / 1st` frame over the 3D backdrop |
| 5 | M | Click BACK rect 5 (mid-animation frame) | `CP05` | BACK-SOLO × solo/server | `C2S 4021`, `S2C 4022`, Room frame `9ebe27c9…`; exact clicked control stays OPEN |

Wire outcome sentences (pack `870ba02f…`): "When the solo actor clicks
RETRY in Result, the exe posts a new empty `C2S 4010` and the next frames
show the personal-room backdrop, not the Waiting seat grid." "When the solo
actor clicks BACK after a new 4018, the exe posts empty `C2S 4021` and
returns to the Room stage." The first sentence is a contradiction with the
checked #294 story; it is not silently substituted for it.

#294 T3 correction (2026-11-25): the 7.55 statics pin the same empty
4021 send, but the 4022 self handler installs state id 5
(`CNormalWaiting`/`CLiveWaiting` vtables), not room state id 3
(`ev_e13ae152…`, `ev_d6a8ea7f…`). The owner playtest agrees with the
statics: BACK lands in Waiting. The captured Room frame was a click on a
mid-animation panel; this is an evidence-attribution conflict, not a
second BACK target. A settled-panel wire+frame capture with timestamps
for the click, 4022 arrival, and any following room re-stage stays OPEN.
The Rust client now follows the statics and playtest
(`settled_solo_result_back_sends_4021_and_lands_in_waiting_after_4022`,
Tier 2, plus the Tier 3 boot leg).

### S294-RESULT-MULTI-02 (wave-4 queue, not executed here)

Boot pair `M = uid 1 / master`, `P = uid 2 / peer`, `O = uid 3 / channel
socket`; one trace id in both manifests. Chain: create → peer join →
READY both → START → chart-timed seven-lane presses on BOTH clients →
4016/4017/4018 fan-out → settled Result on each screen → RETRY on one seat
with the other READY → BACK on both. Checkpoints CP11–CP17 capture each
screen, both wire legs, the stub request line and the 2003 poll. Targets:
ENTER-BATTLE-MASTER/PEER, LOCAL-ROW-4018, PEER-ROW-4018,
MASTER-MARKER-4018, SUBMITTER-NOTICE-4017, RETRY-BATTLE and BACK-SOLO's
battle half. The driver must hit notes on both clients: missing every note
empties the life bar and ends the play early (owner pin), which would leave
the settled panel and member rows unpainted. Dismiss the blue NORMAL-mode
popup at every transition.

### Boot ledger (cap of 4 reached)

| Boot | Display / sink / tap | Actor | Scene reached | Wire tap | Result |
|---|---|---|---|---|---|
| 1 | `:323`, `rig_null_3509658`, 15018 | solo master uid 1 | Result, RETRY click | `f9238a18…` | Partial: second 4010 wired; +20 s frame is MyRoom. Manifest repaired after a duplicate-frame verify failure; no post-click state claim |
| 2 | `:324`, owned null sink, 15019 | solo master uid 1 | Planet only | `9c94b4d2…` | **Failed attempt**: no NORMAL popup, `1001 feffffff` duplicate session, no `03EC`. No cell |
| 3 | `:325`, `rig_null_3538561`, 15020 | solo master uid 2 | Result, RETRY, second Result, BACK, Room | `870ba02f…` | Qualified pack: verify 0 / cite 0; contradiction and LIVE-FILLED cells above |
| 4 | `:326`, owned null sink, 15021 | solo master uid 3 | Channel only | `42d68d69…` | **Failed attempt**: popup dismissed but no `03EC`; CREATE OK never sent `07D4`. No cell |

Every boot used a private Xvfb, an owned null sink and `--no-sound`. The
NORMAL-mode popup was probed and dismissed at each transition that reached
it (boot 2 refused before a popup appeared). All processes were stopped by
the run's own PID files; the shared Memoryer was never restarted or
signalled. After boot 4 another lane took the shared socket; this lane ran
no further boots.

## 4. Gap-to-story hand-off

Ranked by player visibility × reachability:

1. **RETRY-SOLO contradiction** (`LIVE-FILLED` ×2): checked story says Waiting;
   exe shows the personal-room backdrop. Wave-4 spec edit must correct the
   story with pack `96b37640…` / frames `4a89faa5…`, `74e6a2ae…`,
   `c2ad594a…`, and name the battle case as still OPEN.
2. **Two-member Result sheet** (`PEER-ROW-4018` ×3 + partner gaps): the #365
   read-side fix is merged, but no qualified two-member exe frame exists.
   Scenario S294-RESULT-MULTI-02 is the next leg.
3. **4017 submitter cue** (`GAP` ×3 + server EVIDENCED): static handler is
   pinned, UI consumer is not; REA xref before replay.
4. **ticket_add negative** (solo `EVIDENCED`, master/peer `GAP` + two
   NA-DESIGN): three boots agree the GET does not fire. Lane D's gating xref
   explains why: Result enter picks hot_time when selector `0x77ee50` is zero
   and skips both web arms when mode `0x77ee84` is 2. The selector's writer is
   register-held (no direct xref), so the armed arm still needs a replay that
   satisfies the gate. Do not turn the negative into a client requirement.
5. **Battle verdict / rank sources** (4040 ×2, verdict ×2): static pins exist;
   controlled replay needed for browser-visible digits.
6. **BUTTON-RECTS / COUNTUP-ENABLE**: pinned art and update rule; only the
   enable instant lacks a frame clock.

New checklist stories are written into #294 under `## Multi-actor stories`.
The existing checked stories are referenced by their matrix rows, not
duplicated. Closing any story still requires the live caller chain and the
project gates; a green guard is never a story edit.

## 5. Cross-cell contradictions and limits

- **RETRY story refuted for solo**: `#294`'s checked story says "RETRY
  re-enters Waiting for the same song". Boot 3's immediate frames show the
  personal-room backdrop and a second 4010/4018 round without any Waiting
  seat grid. The exe wins as oracle. This matrix records the contradiction;
  the story correction goes through the #294 body edit, not through a quiet
  matrix-only reinterpretation. Whether battle RETRY differs stays OPEN.
- **BACK causality**: boot 3's click occurred while a `GAME OVER / 1st`
  animation frame was up, so the clicked control is not pinned by pixels.
  The wire pair 4021→4022 and the following Room frame are; the settled-panel
  BACK click needs the frame clock that boot 4 failed to reach.
- **ticket_add**: the old `file 0x24a338` pin named the `PointNum.ojs` string;
  the URL starts at file `0x24a2d0`. Three live negatives and the corrected
  gate coexist. No claim is made that the GET never fires in any scenario;
  the selector value source stays OPEN.
- **Score bands**: two qualified Result frames differ in gem (15527 vs 626)
  and nickname but keep miss 20, `GOOD!` and rank F. They pin digit placement
  across two data sets, not a score-to-grade sweep.
- **Manifest repair**: boot 1's first verify failed on a duplicate forced
  Result shot; the scratch duplicate was removed, the pack re-emitted and
  re-cited (verify 0). No cell used the pre-repair citation.
- **Shared server**: all boots used PID 92326's Memoryer; server state matches
  by construction. No "scenario delta" dismissal appears without a wire tap
  and a per-client frame.
- **Failed attempts**: boots 2 and 4 filled no cell. Their logs are in the
  inventory with the refusal / no-`03EC` reasons.

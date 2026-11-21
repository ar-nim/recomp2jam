# Loading story matrix — Spec #292 (FINAL wave, #381)

Method: `docs/captures/story-matrix/TEMPLATE.md`. Cell shape follows the
Waiting, Room, Playing and Messenger exemplars in this directory. Base
`3a8b024`; worktree `/home/arnim/Projects/recomp2jam-wt145`, branch
`cb2/story-loading`. No code change, no pushes, no new exe boot. The 7.55
executable is the oracle (`OTwo.exe`, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`).
Memoryer is a wire witness only. All packs stay in git-ignored scratch;
only hashes and text enter the repo.

Actor semantics in this state: `solo` is a one-seat host (qualified wire exists,
its Loading pixels do not); `peer` is the joined guest in the c1/t1b/p346
boots — those exe clients are guests of a scratch host bot, so they pin guest
cells but not two-client master cells. `master` needs a real two-client start,
which #378 still blocks. `channel` observes the 07D2 board; Loading itself is
an in-client sheet. `server` names a frame with no local click.

## 1. Evidence inventory (before any cell was graded)

Full sha256 values are stated once here; cells use the aliases.

| Alias / source | Full sha256 (or pin) | What it names / verdict |
|---|---|---|
| **P1** `~/scratch/live-lane-1/t1b-loading/citation.txt` | `53b6f30944cac964fa27565a6b76752221d3665b438c7469ec85fcc5e5bf21f1` | QUALIFIED exe Loading pack; `cite --claim exe` 0, 7/7 distinct frames, server PID 92326 / epoch 1794986056. `route=probe-only` is a named provenance deviation (Wine format probe never bound a stream in 10 s); owned null sink, `on_default_sink=0`, `preboot_breach=0`, tap non-silent |
| **P2** `~/scratch/live-lane-1/t1b-loading/artifacts/screenshots/loading.png` | `b555123f6d099f64330f000ea52cd4f2eeeb3716f9fa4139a6bf362466eaaaed` | Exe gate pixels: full-bleed cover, baked Artist/Remix/Pattern, bead pills, 3KEY plate, cursor. Byte-identical to the p346/p347 ref frame and the f1 wedge frame |
| **P3** `~/scratch/live-lane-1/t1b-loading/wire/room-stage.log` | `574443b9999a09c3af22878b353b614c374d74d23d36fd3bbdfa72d0f60a49ce` | `S2C 4037/5027/4039 → 4011 len8 hex=0000000000000000 → 4013 0001 → C2S 4012 len5 0000000001 → 4013 0101 → 4055/4056 → 4048/4049 ×11` |
| **P4** `~/scratch/live-lane-1/t1b-loading/walltrace.tsv` | `5538a64d7f74f503244654614f8e4bf68ad7428ecdeb34577f98c8fff5626fca` | Wire/frame-visible wall proxies: 4011→4012 **398.6 ms**; 4012→first Loading frame 971.7 ms. Does **not** time the four internal legs separately |
| **P5** `~/scratch/no-response/c1-start-cue/citation.txt` | `104b5594efb906fd7b0d783f4f26c9c90a2159c86ad427e5ea1b250aafd74d62` | Older qualified exe pack with the same `b555123f…` frame (`waiting-cue-012.png`) |
| **P6** `~/scratch/no-response/c1-start-cue/wire/room-stage.log` | `589075701bad6179c6189041e8e0eb6c9a3867acbd75ba59f979fe5ed30511fa` | Older guest wire: same confirm sequence and 11 latency rounds, lines 25–55 |
| **P7** `~/scratch/p347-ab/pack/region-metrics.json` | `b02797f8b9ff9d4d13750e7156a251fa9f140212eec4a23ebe4dda824a0e46d4` | Pre-fix matched start: loading weighted SSIM 0.186811, min 0.055975. Negative pack, included with its verdict |
| **P8** `~/scratch/p347-ab/pack/frames/test/loading.png` | `70e308bff4ded98fd0623e888f99180aee5be6d5c9fa1f425df88b6d6b34c51d` | Our first wire-driven Loading frame: label flipped but stale Waiting pixels (#347) |
| **P9** `~/scratch/p346/proof.json` | `3bee382f80398840b59a01d09c447c5e0ea72a87d2f50a312a049d6c3dc68802` | `wire_sequence_equal=true` and matched server slices through first S2C 4013 |
| **P10** `~/scratch/p355-ab/proof.json` | `9c8455f37757d81e8719a915acefbba744983a38095cd98ab0a20a6a8c656bc6` | Post-fix matched wire through S2C 4013; full-window score traffic after Loading is #326 territory |
| **P11** `~/scratch/p355-ab/pack/region-metrics.json` | `06979828fd0e585dff20d3c5b6ce3207de87145f43b078449187217f7041c30a` | Post-fix loading weighted SSIM 0.987878, min 0.984725. #355's close comment quotes `456c00903d8f584d09f2b0bf12abb575f2ca7cef11441e55aa5fb9835833b25d` for the same values; the drift is kept open, not silently reconciled |
| **P12** `~/scratch/p355-ab/pack/frames/test/loading.png` | `9209b628fe2e8947bb9647fb99bcdfa5ddf12b462a35a40c69fb1833704565a3` | Our matched guest frame after #355: staged cover + baked credits. Cursor/overlay difference from P2 is a visible residual, not a gate behavior claim |
| **P13** `~/scratch/solo-playing/s1-master-ui/wire/room-stage.log` | `fb0ca4d0794ce20d721464e19a086a965cace3cabe6686b01fc3cc0a32e3b2b9` | Solo host wire: `C2S 07D4 → 4010 len0 → 4011 zeros → 4012 → 4013 → 4048/4049` (lines 36–45) |
| **P14** `~/scratch/solo-playing/s1-master-ui/master-drive.log` | `590ab8cac70ad37b4a0f351aef6fe28f46ce4d4a493173df1be3b8034194eca3` | Failed attempt: "master loading pixels did not appear after 4010"; no solo Loading frame exists in this pack |
| **P15** `~/scratch/no-response/f1-playing-qualified/citation.txt` | `8773f5cc0efb9486e782e230423d3a0439cb6182f20f22ffd790a5939d1e8517` | Wedge pack: Loading pixels up, no 4014/play pixels, render thread wait. State-path claim only |
| Cover file | `47eb357866f37c6ef39afe048125106ed296ee3f50b44f87b409a5a8a250af06` | Decrypted 800×600 JPEG from the selected `o2ma757.ojn`; P2 differs by MAE 3.18 / PSNR 26.66 dB (#355/#360 close) |
| Static `S1` | `docs/lifecycle.md` §9, files `0x1718b0` (ev `66e60d16…`), `0xf8b80` (ev `d2a69b45…`), `0x12180` (ev `011eab73…`), `0x14db40` (ev `0ee1373d…`), `0xe47a0` (ev `68d920c2…`) | Exe dossier chain: validate → stage → OJN decrypt → codec init → gate screen; no wall durations |
| Static `S2` | `docs/gdi-text.md:100`, `docs/loading-type.md`, ev `68d920c2…`; object words `+0x524`/`+0x528`/`+0x52c`; Lane D REA `ev_3cd0316ec59611a50887e5ae714f6adcfea61a51bf34ad8b2380eb8fd85b4f39` | Trebuchet 27×65 w400 / Arial 7×14 w400 / Gulimche 6×12 w400 descriptors. REA confirms `FUN_004e47a0` creates all three and stores them at object words `[0x149]`/`[0x14a]`/`[0x14b]`. The offsets are object-relative, not virtual addresses (a direct xref probe at `0x4e47cc4` is outside memory). No text-draw call in the gate function reads those words; caption-to-face and Arial consumer stay OPEN |
| Static `S3` | `docs/net-protocol.md` §4, lines 103/116/170 | 4011/4013/4012/4048/4049 layout; 4011 carries no cover field |
| Owner `O1` | #355 close 2026-11-18T17:30Z; #360 close comment; #340 findings `~/scratch/loading-bar-findings.md` | Cover source is the selected OJN JPEG, credits are baked into it; the invented green bar is hidden because qualified exe crops have 0 green-dominant pixels in `(200,272,320,28)` |
| Negative `N1` | `docs/parity/verify-wine.md:15`, `~/scratch/ja-b-findings.md` | Older observation says "bead progress pills, 3-KEY badge, Artist/Remix/Pattern credits"; kept as a contradiction with the #340 crop, not used to reintroduce a green HUD |

Green Rust guards (cover staging, hand session, caption cell) are fix state,
not exe evidence. Server state is shared by one Memoryer (PID 92326); no
"scenario delta" dismissal is used without a wire tap and a per-client frame.

Gap IDs (each is a runnable next step, not a bare OPEN):

- `L-SOLO-PIX` — qualified exe solo-host boot: 4010/4011/4012 plus a Loading
  frame with manifest, citation and null-sink containment; P14 is a failed
  attempt, not a fill.
- `L-SOLO-WALL` — repeat with the 10 ms wall watcher to time the proxy spans
  on the solo-host path.
- `L-MULTI / #378` — fix the bot-master chain or drive two exe clients; one
  trace id, both Loading frames, both wire taps, and an observer 2003/07D2
  poll. No master or two-client cell can be green before this lands.
- `L-INTERNAL-TIMING` — instrument or REA-hook validation/staging/decrypt/
  codec legs; P4 only gives wire proxies.
- `L-REA-FONT` — REA consumer trace at `0x4e47a0+0x524` and `+0x528`, then a
  capture with a chart whose cover has no baked credits, to separate runtime
  text from JPEG ink.
- `L-REFUSAL` — qualified boot with an invalid/uninstalled chart and a
  dropped C2S 4012; capture the retained gate, refusal wire and frame hash.
- `L-ARM` — stopped-state read of the gate arm/ready word or a wire correlate
  with 4013; no successful memory read exists yet.
- `L-INPUT` — qualified input probe in Loading (X, canvas click, Return,
  Escape) with before/after frames and a wire tap.
- `L-DOWNLOAD` — channel-observer 2003 poll while 4026/4037 badges are up.
- `L-HANDOFF` — qualified carry-forward leg with play pixels after 4013
  (#326/#378); P15 stops at Loading.

## 2. Matrix (24 actions × 5 observer actors = 120 cells)

Cell key: `EVIDENCED` / `GAP` / `LIVE-FILLED` / `OPEN-FIX` / `NA-DESIGN` / `OWNER-PARKED` / `P3-PARKED`.
Every gap names its next evidence step. Cell outcomes are exe observations;
Rust guards never grade an exe cell.

| Action (raised by) | solo | master | peer | channel | server |
|---|---|---|---|---|---|
| **ENTER-SOLO** (self:solo, entry: 4010 in a one-seat room) | EVIDENCED — #595 CLOSED — qualified solo-host Loading frame and first-frame guard (5a1f4d7) ; FINAL SWEEP #382 (prior GAP): GAP — P13 wires the start but P14 has no Loading frame; OPEN — `L-SOLO-PIX` | NA-DESIGN — this seat is the `solo` actor | NA-DESIGN — no peer seat | NA-DESIGN — board has no in-client sheet | EVIDENCED — `07D4 → 4010 len0 → 4011 zeros → 4012 → 4013` (P13, S3) |
| **ENTER-MASTER** (self:master, entry: 4010 with a peer) | NA-DESIGN — solo entry is a different row | EVIDENCED — #584 CLOSED — maxwave W1 exe master Loading 7f829e85… + 4010 wire d4d63a6e… — #636 CLOSED — two-member 4010/4011/4012/4013 in ONE trace (w1-05/-06); peer gate #587 CLOSED — maxwave W1 peer Loading ce40c762… / boot-6 6edd25f4… ; FINAL SWEEP #382 (prior GAP): GAP — no qualified exe master Loading frame; OPEN — `L-MULTI / #378` | EVIDENCED — #587 CLOSED — maxwave W1 peer Loading ce40c762… / boot-6 6edd25f4… — #638 CLOSED — peer-side 4011/4012/4013 bodies, both sockets, one trace (S294-RESULT-MULTI-COMPLETION-01) ; FINAL SWEEP #382 (prior GAP): GAP — no paired guest frame at master START; OPEN — `L-MULTI / #378` | NA-DESIGN — Loading is not a board row | EVIDENCED — #636 CLOSED — two-member 4010/4011/4012/4013 in ONE trace (w1-05/-06) — both sockets under one trace; also #638 ; FINAL SWEEP #382 (prior GAP): GAP — per-member 4010/4011 fan-out unpinned; OPEN — `L-MULTI / #378` dual-socket tap |
| **ENTER-PEER-4011** (other:master, entry: S2C 4011) | NA-DESIGN — no peer in a solo room | EVIDENCED — #638 CLOSED — peer-side 4011/4012/4013 bodies, both sockets, one trace (S294-RESULT-MULTI-COMPLETION-01) — peer-side answer bodies pinned on both sockets ; FINAL SWEEP #382 (prior GAP): GAP — master's view of the guest gate unpinned; OPEN — `L-MULTI / #378` | EVIDENCED — `4011 len8 zeros` → full-bleed Loading; frame P2, wires P3/P6 | NA-DESIGN — board cannot see the guest gate | EVIDENCED — `S2C 4011` body is eight zero bytes (P3/P6, S3) |
| **GATE-VALIDATE** (self:server, entry: candidate song at 4011) | EVIDENCED — #595 CLOSED — qualified solo-host Loading frame and first-frame guard (5a1f4d7) ; FINAL SWEEP #382 (prior GAP): GAP — solo pixels missing; OPEN — `L-SOLO-PIX` | EVIDENCED — #584 CLOSED — maxwave W1 exe master Loading 7f829e85… + 4010 wire d4d63a6e… — #636 CLOSED — two-member 4010/4011/4012/4013 in ONE trace (w1-05/-06); peer gate #587 CLOSED — maxwave W1 peer Loading ce40c762… / boot-6 6edd25f4… ; FINAL SWEEP #382 (prior GAP): GAP — master-side validation unpinned; OPEN — `L-MULTI / #378` | EVIDENCED — guest gate admits the installed song 757; frame P2, dossier file `0x1718b0` (S1) | NA-DESIGN — validation is in-client | EVIDENCED — song-table rejection rule at file `0x1718b0` (ev `66e60d16…`), `4037/5027/4039 → 4011` (P3, S1) |
| **GATE-STAGE-DECRYPT-CODEC** (self:server, entry: 4011 → Loading) | EVIDENCED — #595 CLOSED — qualified solo-host Loading frame and first-frame guard (5a1f4d7) ; FINAL SWEEP #382 (prior GAP): GAP — solo pixels missing; OPEN — `L-SOLO-PIX` | EVIDENCED — #584 CLOSED — maxwave W1 exe master Loading 7f829e85… + 4010 wire d4d63a6e… — #636 CLOSED — two-member 4010/4011/4012/4013 in ONE trace (w1-05/-06); peer gate #587 CLOSED — maxwave W1 peer Loading ce40c762… / boot-6 6edd25f4… ; FINAL SWEEP #382 (prior GAP): GAP — master legs unpinned; OPEN — `L-MULTI / #378` | EVIDENCED — cover comes from the selected OJN JPEG after staging/decrypt; wire P3 + O1 + dossier S1 | NA-DESIGN — local file/codec path | EVIDENCED — stage `0xf8b80`, OJN open `0x12180`, codec `0x54db40` (S1); 4011 has no cover field (P3) |
| **GATE-HOLD-NO-4012** (self:server, entry: host 4010 without guest confirm) | NA-DESIGN — solo confirm is local, no gateway hold | EVIDENCED — #584 CLOSED — maxwave W1 exe master Loading 7f829e85… + 4010 wire d4d63a6e… — #636 CLOSED — two-member 4010/4011/4012/4013 in ONE trace (w1-05/-06); peer gate #587 CLOSED — maxwave W1 peer Loading ce40c762… / boot-6 6edd25f4… ; FINAL SWEEP #382 (prior GAP): GAP — no qualified exe master hold frame; OPEN — `L-MULTI / #378` | EVIDENCED — exe Loading stayed up with no 4012 and no play pixels; P15 + `docs/wine-boot.md` Attempt 12 | NA-DESIGN — board cannot see the hold | EVIDENCED — no C2S 4012; 60 s `MusicLoadTimeout` pin in `docs/wine-boot.md:887-895`; wire P15 |
| **WALL-PACING-4011-4012** (self:server, entry: gate cue) | EVIDENCED — #598 CLOSED — solo wall trace 198/291/660/232/396 ms (4874ca8) ; FINAL SWEEP #382 (prior GAP): GAP — no solo-host wall trace; OPEN — `L-SOLO-WALL` | EVIDENCED — #636 CLOSED — two-member 4010/4011/4012/4013 in ONE trace (w1-05/-06) — two-member 4010/4011/4012/4013 in one trace ; FINAL SWEEP #382 (prior GAP): GAP — no two-client wall trace; OPEN — `L-MULTI / #378` | EVIDENCED — 4011→4012 **398.6 ms**, 4012→first frame 971.7 ms; P4 (wire proxies, not internal legs) | NA-DESIGN — no board timing | EVIDENCED — same wall trace P4; static Timing row still OPEN — `L-INTERNAL-TIMING` |
| **COVER-JPEG-FULL-BLEED** (self:server, entry: cover paint) | EVIDENCED — #595 CLOSED — qualified solo-host Loading frame and first-frame guard (5a1f4d7) ; FINAL SWEEP #382 (prior GAP): GAP — solo pixels missing; OPEN — `L-SOLO-PIX` | EVIDENCED — #584 CLOSED — maxwave W1 exe master Loading 7f829e85… + 4010 wire d4d63a6e… — #636 CLOSED — two-member 4010/4011/4012/4013 in ONE trace (w1-05/-06); peer gate #587 CLOSED — maxwave W1 peer Loading ce40c762… / boot-6 6edd25f4… ; FINAL SWEEP #382 (prior GAP): GAP — master cover unpinned; OPEN — `L-MULTI / #378` | EVIDENCED — full-bleed selected-chart JPEG, MAE 3.18 / PSNR 26.66 dB to file `47eb3578…af06`; P2, O1 | NA-DESIGN — cover is in-client | EVIDENCED — no cover id in 4011 (P3); source is the installed OJN (O1) |
| **CREDITS-BAKED** (self:server, entry: cover paint) | EVIDENCED — #595 CLOSED — qualified solo-host Loading frame and first-frame guard (5a1f4d7) ; FINAL SWEEP #382 (prior GAP): GAP — solo pixels missing; OPEN — `L-SOLO-PIX` | EVIDENCED — #584 CLOSED — maxwave W1 exe master Loading 7f829e85… + 4010 wire d4d63a6e… — #636 CLOSED — two-member 4010/4011/4012/4013 in ONE trace (w1-05/-06); peer gate #587 CLOSED — maxwave W1 peer Loading ce40c762… / boot-6 6edd25f4… ; FINAL SWEEP #382 (prior GAP): GAP — master credits unpinned; OPEN — `L-MULTI / #378` | EVIDENCED — Artist / Remix / Pattern visible in P2, baked into the JPEG at decrypted header +296 (O1, #360 close) | NA-DESIGN — credits are in-client | EVIDENCED — 4011 carries no credit strings (P3); JPEG source `47eb3578…af06` (O1) |
| **CAPTION-TREBUCHET** (self:server, entry: caption draw) | OWNER-PARKED — #618 OPEN — Trebuchet 27×65 caption consumer unpinned; solo/master Loading frames are pinned by #595/#584, but runtime caption ink is not isolated ; FINAL SWEEP #382 (prior GAP): GAP — no solo frame isolates runtime caption; OPEN — `L-SOLO-PIX` | OWNER-PARKED — #618 OPEN — Trebuchet 27×65 caption consumer unpinned; solo/master Loading frames are pinned by #595/#584, but runtime caption ink is not isolated ; FINAL SWEEP #382 (prior GAP): GAP — no master caption frame; OPEN — `L-MULTI / #378` | OWNER-PARKED — #618 OPEN — Trebuchet 27×65 caption consumer unpinned ; FINAL SWEEP #382 (prior GAP): GAP — P2's text is JPEG ink; no runtime Trebuchet cell observed; OPEN — `L-REA-FONT` | NA-DESIGN — caption is in-client | EVIDENCED — Trebuchet 27×65 w400 at `0x4e47a0+0x524`, ev `68d920c2…` (S2); which caption takes it stays OPEN — `L-REA-FONT` |
| **ARIAL-7X14-CONSUMER** (self:server, entry: small credit text) | OWNER-PARKED — #616 OPEN — Arial 7×14 consumer xref unpinned ; FINAL SWEEP #382 (prior GAP): GAP — no consumer pin; OPEN — `L-REA-FONT` | OWNER-PARKED — #616 OPEN — Arial 7×14 consumer xref unpinned ; FINAL SWEEP #382 (prior GAP): GAP — no consumer pin; OPEN — `L-REA-FONT` | OWNER-PARKED — #616 OPEN — Arial 7×14 consumer xref unpinned ; FINAL SWEEP #382 (prior GAP): GAP — no credit cell separated from JPEG ink; OPEN — `L-REA-FONT` | NA-DESIGN — no board text | EVIDENCED — Arial 7×14 w400 descriptor at `+0x528` (S2); consumer/rect mapping OPEN — `L-REA-FONT` |
| **BEAD-PILLS-3KEY** (self:solo, entry: cover art) | EVIDENCED — #595 CLOSED — qualified solo-host Loading frame and first-frame guard (5a1f4d7) ; FINAL SWEEP #382 (prior GAP): GAP — solo pixels missing; OPEN — `L-SOLO-PIX` | EVIDENCED — #584 CLOSED — maxwave W1 exe master Loading 7f829e85… + 4010 wire d4d63a6e… — #636 CLOSED — two-member 4010/4011/4012/4013 in ONE trace (w1-05/-06); peer gate #587 CLOSED — maxwave W1 peer Loading ce40c762… / boot-6 6edd25f4… ; FINAL SWEEP #382 (prior GAP): GAP — master art unpinned; OPEN — `L-MULTI / #378` | EVIDENCED — six bead indicators and the 3KEY plate are in the P2 cover; N1's older note agrees | NA-DESIGN — art is in-client | EVIDENCED — no opcode selects pill state; art is JPEG source (P3 negative, O1) |
| **GREEN-BAR-HUD** (self:local painter, entry: fraction status) | EVIDENCED — P2 crop has 0 green-dominant pixels; owner hides our invented bar (O1, #340) | EVIDENCED — #584 CLOSED — maxwave W1 exe master Loading 7f829e85… + 4010 wire d4d63a6e… — #636 CLOSED — two-member 4010/4011/4012/4013 in ONE trace (w1-05/-06); peer gate #587 CLOSED — maxwave W1 peer Loading ce40c762… / boot-6 6edd25f4… ; FINAL SWEEP #382 (prior GAP): GAP — no two-client crop; OPEN — `L-MULTI / #378` | EVIDENCED — same qualified crop, 0 `(48,176,48)` pixels in P2; N1's "pills" are JPEG art, not a green HUD | NA-DESIGN — board has no progress fraction | NA-DESIGN — fraction is local `loading_phase_status`; no wire field (S3) |
| **CONFIRM-4012** (self:peer, entry: 4011/4013) | EVIDENCED — solo wire `4010 → 4011 → 4012 → 4013` (P13) | EVIDENCED — #584 CLOSED — maxwave W1 exe master Loading 7f829e85… + 4010 wire d4d63a6e… — #636 CLOSED — two-member 4010/4011/4012/4013 in ONE trace (w1-05/-06); peer gate #587 CLOSED — maxwave W1 peer Loading ce40c762… / boot-6 6edd25f4… ; FINAL SWEEP #382 (prior GAP): GAP — master confirm unpinned; OPEN — `L-MULTI / #378` | EVIDENCED — `4011 zeros → 4013 0001 → C2S 4012 len5 → 4013 0101`; P3/P6, proofs P9/P10 | NA-DESIGN — confirm is not board-visible | EVIDENCED — body layout in S3; wires P3/P6/P13 |
| **CONFIRM-SEND-FAIL** (self:local send, entry: 4012 not delivered) | OWNER-PARKED — #605 OPEN — refusal pixels for invalid/uninstalled chart — #602 CLOSED — refused 4012 holds Loading (5cf98af) holds a failed send; exe refusal pixels still OPEN ; FINAL SWEEP #382 (prior GAP): GAP — no qualified solo refusal frame; OPEN — `L-REFUSAL` | OWNER-PARKED — #605 OPEN — refusal pixels for invalid/uninstalled chart — #602 CLOSED — refused 4012 holds Loading (5cf98af) wire half closed; exe refusal frame OPEN ; FINAL SWEEP #382 (prior GAP): GAP — no master refusal frame; OPEN — `L-REFUSAL`, `L-MULTI / #378` | OWNER-PARKED — #605 OPEN — refusal pixels for invalid/uninstalled chart ; FINAL SWEEP #382 (prior GAP): GAP — no exe refusal frame in any pack; OPEN — `L-REFUSAL` | NA-DESIGN — refusal is in-client | OWNER-PARKED — #605 OPEN — refusal pixels for invalid/uninstalled chart — MusicLoadTimeout gateway body still unpinned ; FINAL SWEEP #382 (prior GAP): GAP — server side of a dropped confirm unpinned; OPEN — `L-REFUSAL` tap of the MusicLoadTimeout path |
| **POST-4013-4048** (self:server, entry: after 4013) | EVIDENCED — P13 has 4048/4049 exchange | OWNER-PARKED — #601 OPEN — exe 11 4048/4049 rounds after 4013 ; FINAL SWEEP #382 (prior GAP): GAP — dual-leg latency unpinned; OPEN — `L-MULTI / #378` | EVIDENCED — 11 rounds `C2S 4048 len20 → S2C 4049 len20`; P3/P6 | NA-DESIGN — latency is not board state | EVIDENCED — P3/P6/P13 + S3; continuation is #326 territory |
| **GATE-REFUSAL** (self:server, entry: invalid/uninstalled chart) | OWNER-PARKED — #605 OPEN — refusal pixels for invalid/uninstalled chart — #602 CLOSED — refused 4012 holds Loading (5cf98af) holds a failed send; exe refusal pixels still OPEN ; FINAL SWEEP #382 (prior GAP): GAP — no refusal pixels; OPEN — `L-REFUSAL` | OWNER-PARKED — #605 OPEN — refusal pixels for invalid/uninstalled chart — #602 CLOSED — refused 4012 holds Loading (5cf98af) wire half closed; exe refusal frame OPEN ; FINAL SWEEP #382 (prior GAP): GAP — no master refusal frame; OPEN — `L-REFUSAL`, `L-MULTI / #378` | OWNER-PARKED — #605 OPEN — refusal pixels for invalid/uninstalled chart ; FINAL SWEEP #382 (prior GAP): GAP — static reject rule only, no exe refusal frame; OPEN — `L-REFUSAL` | NA-DESIGN — refusal is in-client | OWNER-PARKED — #605 OPEN — refusal pixels for invalid/uninstalled chart — MusicLoadTimeout gateway body still unpinned ; FINAL SWEEP #382 (prior GAP): GAP — gateway body on invalid selection unpinned; OPEN — `L-REFUSAL`; exe rule at file `0x1718b0` (S1) |
| **ARM-LATCH** (self:server, entry: codec ready) | EVIDENCED — #608 CLOSED — LoadingGateArm + 4012→4013 correlate (b5e40f2) ; FINAL SWEEP #382 (prior GAP): GAP — no solo arm frame; OPEN — `L-ARM` | EVIDENCED — #608 CLOSED — LoadingGateArm + 4012→4013 correlate (b5e40f2) — plus #584 CLOSED — maxwave W1 exe master Loading 7f829e85… + 4010 wire d4d63a6e… ; FINAL SWEEP #382 (prior GAP): GAP — no master arm frame; OPEN — `L-ARM`, `L-MULTI / #378` | EVIDENCED — gate screen walks the table and arms the clock at `0xe47a0`, then hands off (S1); P2/P3 gate reached | NA-DESIGN — arm is in-client | EVIDENCED — #608 CLOSED — LoadingGateArm + 4012→4013 correlate (b5e40f2) — 4012→4013 correlate pinned ; FINAL SWEEP #382 (prior GAP): GAP — arm/ready word never read successfully; OPEN — `L-ARM` (the winedbg attempt in `live-lane-1` returned no data) |
| **LOADING-BACK-X** (self:player, entry: X/back in Loading) | OWNER-PARKED — #611 OPEN — X/back behavior inside Loading ; FINAL SWEEP #382 (prior GAP): GAP — no X-close probe; OPEN — `L-INPUT` | OWNER-PARKED — #611 OPEN — X/back behavior inside Loading ; FINAL SWEEP #382 (prior GAP): GAP — no master X frame; OPEN — `L-INPUT`, `L-MULTI / #378` | OWNER-PARKED — #611 OPEN — X/back behavior inside Loading ; FINAL SWEEP #382 (prior GAP): GAP — no guest X frame; OPEN — `L-INPUT` | NA-DESIGN — no board sheet | OWNER-PARKED — #611 OPEN — X/back behavior inside Loading ; FINAL SWEEP #382 (prior GAP): GAP — 2021/2022 from Loading unpinned; OPEN — `L-INPUT` wire tap |
| **INPUT-IGNORE** (self:player, entry: click/Return/Escape in Loading) | EVIDENCED — #614 CLOSED — gate ignores click/Return/Escape on the live input path (e442734) ; FINAL SWEEP #382 (prior GAP): GAP — no solo input probe; OPEN — `L-INPUT` | EVIDENCED — #614 CLOSED — gate ignores click/Return/Escape on the live input path (e442734) ; FINAL SWEEP #382 (prior GAP): GAP — no master input probe; OPEN — `L-INPUT`, `L-MULTI / #378` | EVIDENCED — `docs/wine-boot.md:1062-1070` records the gate ignoring click/Return/Escape with no crash; P15 frame stays up | NA-DESIGN — input is in-client | EVIDENCED — #614 CLOSED — gate ignores click/Return/Escape on the live input path (e442734) ; FINAL SWEEP #382 (prior GAP): GAP — no opcode difference from an ignored input; OPEN — `L-INPUT` tap |
| **DOWNLOAD-BADGE-AT-ENTRY** (self:server, entry: 4026/4037 before start) | OWNER-PARKED — #623 OPEN — download-badge-to-Loading transition with peer seated ; FINAL SWEEP #382 (prior GAP): GAP — no solo missing-song gate; OPEN — `L-DOWNLOAD` | OWNER-PARKED — #623 OPEN — download-badge-to-Loading transition with peer seated ; FINAL SWEEP #382 (prior GAP): GAP — no master badge-to-Loading frame; OPEN — `L-MULTI / #378` | EVIDENCED — matched-ab-339 badge crop `7d4d586c…` with `4026/4037/3009`; cross-state Waiting→Loading, not a Loading paint | OWNER-PARKED — #623 OPEN — download-badge-to-Loading transition with peer seated ; FINAL SWEEP #382 (prior GAP): GAP — no 2003 row poll during download; OPEN — `L-DOWNLOAD` | EVIDENCED — `4026/4037/3009` bodies in matched-ab-339; wire pin |
| **TWO-CLIENT-LOADING** (other:master starts, entry: 4010 with two seated members) | NA-DESIGN — solo start has no peer | EVIDENCED — #584 CLOSED — maxwave W1 exe master Loading 7f829e85… + 4010 wire d4d63a6e… — #636 CLOSED — two-member 4010/4011/4012/4013 in ONE trace (w1-05/-06) ; FINAL SWEEP #382 (prior GAP): GAP — master's own Loading frame missing; OPEN — `L-MULTI / #378` | EVIDENCED — #587 CLOSED — maxwave W1 peer Loading ce40c762… / boot-6 6edd25f4… — #638 CLOSED — peer-side 4011/4012/4013 bodies, both sockets, one trace (S294-RESULT-MULTI-COMPLETION-01) ; FINAL SWEEP #382 (prior GAP): GAP — guest Loading frame paired with an exe master missing; OPEN — `L-MULTI / #378` | EVIDENCED — #589 CLOSED — maxwave W1 observer 07D2 poll state 1→2 at CP04 (260ba904…) ; FINAL SWEEP #382 (prior GAP): GAP — no observer 2003/07D2 poll during a two-client gate; OPEN — `L-MULTI / #378` | EVIDENCED — #636 CLOSED — two-member 4010/4011/4012/4013 in ONE trace (w1-05/-06) — #638 CLOSED — peer-side 4011/4012/4013 bodies, both sockets, one trace (S294-RESULT-MULTI-COMPLETION-01) ; FINAL SWEEP #382 (prior GAP): GAP — no dual-socket 4010/4011/4012 tap; OPEN — `L-MULTI / #378` |
| **CHANNEL-ROW-STATE** (other:master starts, entry: 07D2 board poll) | NA-DESIGN — no second client to observe | NA-DESIGN — master is inside the gate | NA-DESIGN — peer is inside the gate | EVIDENCED — #589 CLOSED — maxwave W1 observer 07D2 poll state 1→2 at CP04 (260ba904…) ; FINAL SWEEP #382 (prior GAP): GAP — row label during Loading unpinned; OPEN — `L-MULTI / #378` observer poll | OWNER-PARKED — #592 OPEN — dual-socket 4010/4011/4012 tap ; FINAL SWEEP #382 (prior GAP): GAP — no 2003/2020 frame tied to a Loading interval; OPEN — `L-MULTI / #378` tap |
| **HANDOFF-TO-PLAYING** (self:server, entry: armed gate + confirm) | EVIDENCED — #595 CLOSED — qualified solo-host Loading frame and first-frame guard (5a1f4d7) — #598 CLOSED — solo wall trace 198/291/660/232/396 ms (4874ca8); play pixel leg OPEN ; FINAL SWEEP #382 (prior GAP): GAP — P13 wires the handoff but no play pixels; OPEN — `L-HANDOFF` | EVIDENCED — #636 CLOSED — two-member 4010/4011/4012/4013 in ONE trace (w1-05/-06) — #378 CLOSED — bot-master play chain dae62a5: guest 4050 → Highway pixels → 4014 drain; pack 96345ebb… ; FINAL SWEEP #382 (prior GAP): GAP — no two-member handoff; OPEN — `L-MULTI / #378` | EVIDENCED — #378 CLOSED — bot-master play chain dae62a5: guest 4050 → Highway pixels → 4014 drain; pack 96345ebb… — guest 4050→Highway pixels with the fixed bot master ; FINAL SWEEP #382 (prior GAP): GAP — P15 wedged in Loading with no 4014/play pixels; OPEN — `L-HANDOFF` | NA-DESIGN — Playing is in-client | EVIDENCED — `4013 → 4055/4056 → 4048×11` (P3/P13); handoff at `0xe47a0` (S1); play-pixel leg stays OPEN — `L-HANDOFF` |

Counts after #382 final sweep: **120 cells** — 66 EVIDENCED, 31 NA-DESIGN, 23 OWNER-PARKED. Every disposition names a pack/pin/ticket or an owner item.
Prior wave counts and their GAP wording are retired; the disposition text above is current.

## 3. Scenario script plan (not executed this wave)

### S292-LOADING-MULTI-01 — blocked by #378, no boot spent

- `scenario_id`: `S292-LOADING-MULTI-01`; `spec`: #292; `state`: Loading;
  `trace_id`: `story-loading-final-01` (reserved, no pack carries it).
- `boot_pair`: `M = tester / uid 1 / master`, `P = tester2 / uid 2 / peer`,
  two exe clients (or a fixed bot master), plus `O = uid 3 / channel observer`.
- `containment`: private Xvfb per client, owned `rig_null_*` sink each,
  `--no-sound`, `route=sink`, `on_default_sink=0`, `preboot_breach=0`,
  Memoryer PID/epoch re-read, stop by recorded PID files. The blue
  NORMAL-mode popup must be dismissed at (400,365) at every transition.
- `steps`: seq 1 `M` create 3KEY/song 757 → Waiting (CP01); seq 2 `P` join →
  Waiting (CP02); seq 3 `P` READY (CP03); seq 4 `M` START → both Loading
  (CP04, fills `TWO-CLIENT-LOADING` × master/peer/channel/server); seq 5
  wall-traced 4011→4012 on both sockets (CP05); seq 6 confirm → Playing
  (CP06, fills `HANDOFF-TO-PLAYING`).
- `checkpoint`: per step, each actor's frame hash, bidirectional tap body,
  observer 2003 poll, popup/focus/deviation notes under the trace id.
- `outcome`: no expected-behavior sentence is written for an unfilled cell.
  The only exe outcomes already available come from P1–P6 (single guest client)
  and P13 (solo wire), and are transcribed in §2 only for those cells.

### Boot ledger

| Attempt | Reason no cell was filled |
|---|---|
| Preflight only, 0 boots | `ss -tnp | grep 15010` showed two other lanes' established `python3` clients (PIDs 3734742, 3734743) to Memoryer PID 92326; one live lane is shared. #378 remained OPEN, so a boot could only repeat P1/P13 or contend with the shared server. No client launched, no NORMAL popup encountered, no server restarted or signalled. |

Earlier failed attempts are inventory entries, not fills: P14 (solo master
started but no Loading pixels), P7/P8 (matched start painted stale Waiting
pixels, fixed by #347/#355), P15 (wedge in Loading), and live-lane-1 boot 1
(FAILED manifest from a duplicate cue frame, 411.9 ms proxy span not used).
The worktree setup claimed no data slot: all four were held by other
worktrees. No game data was read here.

## 4. Gap-to-story hand-off and contradictions

Ranked by player visibility × reachability:

1. `L-MULTI / #378` — both clients' Loading views during one start, plus the
   channel poll and dual wire tap (4 GAP cells in `TWO-CLIENT-LOADING`, plus
   master arms across the whole matrix). Exe-master Loading is still unknown.
2. `L-SOLO-PIX` / `L-SOLO-WALL` — solo-host Loading pixels and wall proxies;
   P13 proves the wire but P14 never captured the gate.
3. `L-REFUSAL` / `L-ARM` / `L-INPUT` — failure and edge behavior. Static pins
   name the mechanisms; no qualified exe frame or memory read exists.
4. `L-REA-FONT` — separate runtime Trebuchet/Arial ink from credit text baked
   into the JPEG. Lane D pinned font creation at `FUN_004e47a0`
   (`ev_3cd0316…`), but the gate function has no text-draw consumer; a cover
   with no baked credits is still needed before any runtime-credit story.
5. `L-INTERNAL-TIMING` — instrument the four static legs; P4 only brackets
   them with wire-visible proxies.

Existing #292 checklist stories map to EVIDENCED cells as follows and are
not duplicated: story 1 (full-bleed cover) → `COVER-JPEG-FULL-BLEED`;
story 2 (credits) → `CREDITS-BAKED`; story 3 (gate hold) →
`GATE-HOLD-NO-4012`; story 4 (4011/4012 confirm) → `CONFIRM-4012`;
story 5 (pinned gate rules) → `GATE-VALIDATE` / `GATE-STAGE-DECRYPT-CODEC`;
story 6 (wall pacing) → `WALL-PACING-4011-4012`. The unchecked stories added
to the body below cover gaps and EVIDENCED cells that had no checklist row.

Contradictions preserved:

- N1's older "bead progress pills" note and #340's zero-green crop both come
  from exe frames. The P2 image resolves the wording: bead pills and the 3KEY
  plate are JPEG art; the invented green HUD fill is not. No story is rewritten
  to match our painter.
- P11's on-disk hash differs from the hash quoted in #355's close and the
  #294 matrix (`456c0090…`); the reported Loading values agree. Both hashes
  are named in §1. Lane D added no fresh pixels: the REA font-creation
  dossier does not resolve which runtime caption, if any, uses Arial.
- N1's "bead progress pills" remains resolved as art in P2; the live
  Memoryer fixture probe (`db1a395d…`) carries no Loading frame and changes
  no Loading cell.
- Static `docs/parity/loading-gate.md` still marks the bar/caption rects and
  the four-leg wall pacing OPEN. Those seams are not re-graded by a green
  guard in this matrix.
- #355 and #360 are closed fixes; their post-fix numbers are exe-derived
  comparisons against P2, not new exe behavior claims. The two-client rows
  stay GAP because no exe master frame exists.
EOF
cd /home/arnim/Projects/recomp2jam-wt145 && git status --short

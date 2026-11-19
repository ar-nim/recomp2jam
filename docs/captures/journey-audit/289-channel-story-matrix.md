# Channel story matrix — Spec #289 (WAVE 2, lane B)

Method: `docs/captures/story-matrix/TEMPLATE.md`; cell shape follows the
Waiting exemplar (`291-waiting-story-matrix.md`). Base `eba5ef0`; worktree
`/home/arnim/Projects/recomp2jam-wt136`, branch `cb2/story-channel`. No code
changes and no pushes. The 7.55 executable is the oracle
(`OTwo.exe`, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`).
Memoryer is a wire witness only. All capture paths below are git-ignored
scratch; only hashes and text enter the repo.

## 1. Evidence inventory (before any cell was graded)

| Source | Pin (full sha256 unless noted) | What it names / verdict |
|---|---|---|
| `~/scratch/240-fix/wine2` (local pack behind committed `docs/captures/t240`) | provenance `f6f7f52ceaa70fe26e9dffde9fba1d252bac50a68410f83296b52a35ee3be994`, manifest `57089d0365191ebba4261abe5f5d38785908f528a3e7ca80e905e357cf3f3cfa`, citation `15442fbffd60aee7e305d64329dc43c045aea56183b931a76a153e358948b6c3` (all three match `docs/captures/t240/record-hashes.txt`); channel frame `ea5cc6d632a534ab02d7d6f12e10bf357f8cf5dc65db991a9b026ee0ecf6c563`, planet frame `235f53521171be4f1d4c725d16143332fa47b66813f9e1c5b8e8b09fe68ddc21` | qualified exe Planet→Channel→Room drive: dialog over board, MENU band, emblem strip, X close, entry face `(526,248)`, blue notice OK `(400,365)` then channel frame |
| `~/scratch/p244-capture` | popup frame `baf781b8caa6823b60047901e5a135d3ab77cb7fc60a7031fda02d90e408c1bf` (hash re-read), channel frame `ea5cc6d6…c563`; citation text names manifest `e211cc6e8016f7e7048276e3f2fa247744efbaeca6f766a36130a1984e535b20` | qualified exe one-OK NORMAL-mode popup BEFORE the channel list; #244/#348 oracle. **Contradiction:** the local `manifest.json` hashes to `9ae19031ae5c7246b86fa85cb2b36f0a7a68dac5817c27eb4fa1b7e41bd3851f`; cite the citation text, not that local file |
| `~/scratch/p343-ab/proof-final.json` | `20fbe17060cea3d3da16a059b74de06a724b6bb41ee6ae2caf947b3ccfc8cff6` | matched exe/Rust wire and server slice on channel entry; `channel_list_body_equal=true`, `wire_sequence_equal=true` |
| `~/scratch/p346` (stale, pre-fix) | proof `3bee382f80398840b59a01d09c447c5e0ea72a87d2f50a312a049d6c3dc68802`; region metrics `f52bf479c32d638d8afa814646ef0d151a7cda0e120d183f73b885b46102e60b`; exe channel `3c79af37986b4a554f8724088f0b058441ecd024bad5ba8b6dd00560e37723dc`, Rust channel `ef278f3d559a7cb92791436fe52836717bc0349a6cd09484dc084882a16f3fa5` | old matched-window A/B with select-bar row + orange-segment residual; wire/server equal. Kept as a negative, not used to grade post-fix cells; its pointer-state error is resolved by #364 |
| `~/scratch/p361-ab`, `~/scratch/live-lane-2` (post-#364/#361 A/B) | proof `0a1862bf5ec2109749a03bd9f493ae617467175d095d08acf97f1fac6fd55333`; region metrics `aa318f6a8b2cd62b0bd7fee44de8fa41f1a5de8227090a2812d2a2e4318714dd`; trace `09f67f0ffd4dc479416b04e0435950d9ba44d671c6571975676797f56dc091fe`; exe/Rust planet `235f53521171be4f1d4c725d16143332fa47b66813f9e1c5b8e8b09fe68ddc21` / `08c90bcb68f6a2a182682acfb03c74ca937a77699f83f5577e57f21e90f06d3b`; exe/Rust channel `3c79af37986b4a554f8724088f0b058441ecd024bad5ba8b6dd00560e37723dc` / `32841d4a2e92ad888b1fe0c3a38b28bf57e37ebda16f06146753457752fc9cb7`; pre-fix metrics `a24a38f9eb9736033e33f511cc403882f0dd86a1f8e1a82dd1ce04a1bbee9070` (`~/scratch/native-ab-334/pack/region-metrics.json`) | fresh pointer-matched pair: emblem rest SSIM 0.962924 (PSNR 28.04 glyph residual), green bar rows 324/325 and 363/364 on both legs, `row_bars` 0.674329; wire/server/room/channel bodies equal |
| `~/scratch/p370-fill-proxy/run1`, `rust-channel3` | exe channel `fa9a89fd70ffe65d3bec414479855eb5379c17d92a3b6e0da9dd6e93859dfe27`; Rust channel `9dfe9bbe9b16178e22978559c346e8414b31250cdda72ef0207074f0567d7649`; wire `run1/wire/room-stage.log` | synthetic proxy rewrite of the 1003 population word (10/50/9/1) pins `ceil(pop*10/cap)` whole segments; matched re-run `row_bars` 0.674329→0.676613 with a named 1 px anchor residual. Synthetic field delta, not an unmodified-exe claim |
| `~/scratch/digits-rig`, `~/scratch/t189-keyrule`, `~/scratch/channel-geom2`, `~/scratch/channel-rects`, `~/scratch/planet-verify` | exe live captures and memory reads in `docs/channel-fill.md`, `docs/parity/channel.md`; row rects at dialog+`0x94`, digits `channel_id+1`, active-row CLOSE, no pager, mouse-only keys, arrow scroll top 0→1, emblem family switch | exe-only static/live pins; no new boot needed |
| `~/scratch/no-response/c1-waiting-observe`, `c3-waiting-qualified` | c1 wire `40efa58335b2b97726129edd41ea6f0b13f0e7513183ecbb8072f0ba3a8e3328`; c3 citation `96339c1f3cc088e913cc8d70d4d271dd5075ee1ae1abdb4d0ba69c1b7e40d3f2` | channel-entry wire cadence before Waiting; `0BBA/0BBB` join pair |
| `~/scratch/p376-live`, `~/scratch/p376-two-client-rig.sh` | rig script `28dce0ba36be52e9d046efb234011415e199b131dc5d95dbef19ad94235b6723`; peer room row `frames-b/frame-000127-room.png` `8dd6a9b39e4ba565ef2118be01ef3326c4af48d8a033b244325061080866a9d9`; its channel frame `frames-b/frame-000036-channel.png` `deee19d96cf9d3e098b76168160ca9941b9029153683c3b943106357be4e0afd` | #376: `S2C 2005` stages the created room for the peer; two clients share one Memoryer, so server state is not a scenario delta |
| `~/scratch/p379-wire-probe.log`, `p379-wire-probe2.stdout` | `b07d63f81db0a0c883fdee3a83c78151d0624dac12e0bd6e2c9531f06b3f832f`, `8e8735de7307d17d5a0a7de98dfc9b5109b95a2b9a15c0870996b973dddbac50` | shared-lane wire tap: master gets 3004 (103 B) on peer join, joiner gets 3003 (180 B), ready 4009 both, leave 3007 master only |
| WAVE 1 `~/scratch/story-matrix-live/` | tap `e326e3f7eaae0cd88dfa9e8735d7554c5c73dd6fc7b376017dbb28ffb5d2834a`; observer `d4fa3dc6ff6dada19acbd7ced46eaeb7e21cb7743b07a74865445aea463ad680` | `S2C 2007 len=8 hex=00000000` on last disconnect and the observer's vanished row; #380 fix still OPEN |
| `~/scratch/story-channel-relay-down/run2`, `run3`, `run4` (this wave) | planet `417a1aa126a76f1b2bd804965acc2e78ab5e409a77d0fd899931fd650cf7a4dc`; channel `deee19d96cf9d3e098b76168160ca9941b9029153683c3b943106357be4e0afd`; root framebuffer after 5 s `0bae3412f1d1b0852922a8b866ed25fe118e43b9404598f4b6b3de58c0abc903`; `client.log` `37dbdaf21c1df8e4d8e442e153311f372189542332626c4af82155d1b9cb480d`; `input.log` `e74c6a53227c10c7c0cfd58a5530a45952437a4260e355dc267fffe9e382d1e6` / `01d023cc47b9245f9057dc2f4cd2f2294b296261426f847e8b80c95962fcb980`; audio sidecar `2ea4991857bf3b2150730992f01c689cacc1243ec544f5b9094c3bdb79e45d7f` / `3b19dc95c660aa7b6bd8782cb0a40fad7bf53dea9f1da3cd8a6166e72099554d` | our client's relay-down boundary: bare board at Planet, one-OK popup at Channel, but popup over a BLACK canvas; see §3 |
| `docs/net-protocol.md`, `docs/channel-fill.md`, `docs/parity/channel.md`, `docs/captures/journey-audit/289-channel.md`, `docs/wine-boot.md` | exe hash + file offsets/REA ids (1003 handler file `0x159ff0`, enter gate `0x75345`, 2021 send files `0x48ff90`–`0x48ffa8`, 2005/2007 family, select-bar fetch file `0x8de84`) | static/REA class; wire bodies, entry refusal, no-pager and scroll semantics |
| #376/#380/#379/#375 issue bodies and close comments | owner observations 2026-11-19; fixes `35a9348` (#376), `8de4d28` (#375), p379 fix still in flight | explicit cross-state stories; none silently mapped to a Channel pixel claim |

Stale probes and negatives are included above with their verdicts; a green
guard is never used as exe evidence, and a synthetic field rewrite is never
called an unmodified exe observation.

## 2. Matrix (19 actions × 5 observer actors = 95 cells)

Cell key: `EVIDENCED` / `GAP` / `LIVE-FILLED` / `OPEN-FIX` / `NA-DESIGN`.
Every gap names a runnable next evidence step. Actor set is the template's:
`solo` = no gateway session, `master`/`peer` = room roles (a player on the
board before a room is a prospective master/peer), `channel` = board observer
not in the room, `server` = frame with no local click. In this spec the relay
endpoints are an **environment actor**; row 17 names it in the stimulus.

| Action (raised by) | solo | master | peer | channel | server |
|---|---|---|---|---|---|
| **ENTER-CHANNEL** (self:peer from Planet) | NA-DESIGN — offline boot has no gateway session or Channel board | EVIDENCED — `<emblem click>` → `1004/1005`, `2000/2001`, `2002/2003`, `2010/2011/2024` staged → wire pin p343 proof `20fbe170…`, enter gate file `0x75345` | EVIDENCED — `<Planet NORMAL emblem>` → Channel sheet over the board → pack hash `ea5cc6d6…` (t240), drive face `(526,248)` | EVIDENCED — `<enter>` → MENU band and emblem strip stay visible behind the sheet → pack hash `ea5cc6d6…`, static pin `docs/parity/channel.md` | EVIDENCED — `<1002→1003, 1004→1005>` → rows plus login success → wire pin p343 `20fbe170…`, `docs/net-protocol.md:100` |
| **LIST-RENDER** (self:server 1003 rows) | NA-DESIGN — no list without a gateway session | EVIDENCED — `<1003 rows>` → digits show `channel_id+1`, CLOSE wells on inactive → pack hash `3c79af37…`/`32841d4a…` (p361 pair), digit proxy pin `docs/channel-fill.md` | EVIDENCED — `<1003 rows>` → wire row *k* paints slot *k* (`Ch.NN`, not ordinal) → pack hash `ea5cc6d6…`, digits-rig runs 1–3 | EVIDENCED — `<1003 rows>` → four viewport slots at `dialog+0x94` (live read `0x18E7E20+0x94`) → dossier/offset pin `docs/channel-fill.md`, pack hash `ea5cc6d6…` | EVIDENCED — `<1003 body>` → `i32 count` + 13 B rows, active byte retained → wire pin `docs/net-protocol.md:134`, c1 wire `40efa…` |
| **POPULATION-SEGMENT** (self:server population field, #370) | NA-DESIGN — no list offline | EVIDENCED — `<population 1/100>` → one whole orange segment after the fix → pack hash Rust `9dfe9bbe…`, region 0.674329→0.676613 | EVIDENCED — `<population 10/50/9/1>` (synthetic proxy rewrite) → 1/5/1/1 segments → pack hash `fa9a89fd…`; synthetic field delta named | EVIDENCED — `<1003 population>` → count strip lights `ceil(pop*10/cap)` cells → wire pin `run1/wire/room-stage.log`, pack hash `fa9a89fd…` | EVIDENCED — `<1003 population word>` → both clients light the pinned whole segments → wire pin p370 proxy log; 1 px anchor delta stays named |
| **SELECT-BAR-HOVER** (self:channel pointer, #364) | NA-DESIGN — no board offline | EVIDENCED — `<pointer (401,366)>` → green rows 324/325 and 363/364 (`Ch.03`) on both legs → pack hash metrics `aa318f6a…`, frames `3c79af37…`/`32841d4a…` | EVIDENCED — `<hover alone>` → bar follows hovered row; stale p346 row-0 mismatch is resolved as a pointer-state gap, not a painter bug → pack hash `aa318f6a…`; contradiction kept in §3 | EVIDENCED — `<motion>` → bar moves with mouse alone; Up/Down/Enter are no-ops → exe live pin `~/scratch/t189-keyrule/`, static bar fetch file `0x8de84` | NA-DESIGN — no server frame drives a local hover; design seam is the staged pointer state, not a wire field |
| **EMBLEM-REST** (self:solo board, #361) | EVIDENCED — `<boot>` → dimmed rest badges from the bound sheet → pack hash `235f5352…`, SSIM 0.962924 in `aa318f6a…` | EVIDENCED — `<bare board under dialog>` → same dim rest badges → pack hash Rust `08c90bcb…` (p361 pair) | EVIDENCED — `<dialog over board>` → dim strip still visible under the sheet → pack hash `ea5cc6d6…` | EVIDENCED — `<notice over board>` → dim strip remains behind → pack hash `baf781b8…` | NA-DESIGN — rest art is staged sheet pixels; no wire field exists for badge brightness |
| **EMBLEM-SELECT** (self:channel family click) | NA-DESIGN — no gateway family offline | EVIDENCED — `<H/M/C emblem>` → family switch and reconnect → exe live pin `~/scratch/planet-verify/findings.md` (`parent+0xF64` phase, HARD popup gate) | EVIDENCED — `<family click>` → server family changes, dialog returns with the new rows → exe live pin `docs/parity/channel.md` | EVIDENCED — `<re-select active family>` → reconnect without re-fetch, rows blank → exe live pin `docs/parity/channel.md` | EVIDENCED — `<03EC Enter channel>` → selected family recorded → wire pin `docs/net-protocol.md`, p346 server slice |
| **ROW-ENTER-FACE** (self:peer face click) | NA-DESIGN — no face without a session | EVIDENCED — `<face (502,236)>` → raw `channel_id` in 1004, not the displayed id+1 → wire pin `20fbe170…`, gate file `0x75345` | EVIDENCED — `<face (526,248)>` → `STATE_ROOM` → pack hash `ea5cc6d6…` + t240 `drive.log` | EVIDENCED — `<row-body click vs face>` → body only highlights; face enters → exe live pin `docs/channel-fill.md` #56, pack hash `ea5cc6d6…` | EVIDENCED — `<1004>` → `1005` success; duplicate gives `1001 feffffff` → wire pin `docs/net-protocol.md:136`, c1 wire `40efa…` |
| **CLOSED-ROW-REFUSE** (self:peer on inactive row) | NA-DESIGN — no rows offline | EVIDENCED — `<active=0 row>` → empty CLOSE well, no 1004, message string at exe file `0x647334` → dossier/offset pin `docs/net-protocol.md:134` | EVIDENCED — `<active=0 inside list>` → CLOSE and refusal; `active`/`capacity` co-vary (which field gates is OPEN) → exe pin digits-rig runs 1–3, pack hash `ea5cc6d6…` | EVIDENCED — `<1003 active byte false>` → CLOSE slot; Memoryer ids 0–4+9 active → wire pin `docs/net-protocol.md:134` | EVIDENCED — `<gateway channel absent>` → `active=false` while capacity never gates → wire pin `docs/net-protocol.md:134`, config note |
| **SCROLL-LIST** (self:channel down-arrow) | NA-DESIGN — no list offline | EVIDENCED — `<down arrow>` → list top 0→1 (`Ch.01-04`→`Ch.02-05`), highlight untouched → exe pin `~/scratch/t189-keyrule/`, static files `0x10ea1a`/`0x10ea6b` | EVIDENCED — `<arrow click>` → list scrolls; Up/Down keys do nothing → exe pin `t189`, static `0x50ebf0` | EVIDENCED — `<mouse-armed list>` → scroll moves list top, not selection → exe pin `t189 m-arrowdown` | NA-DESIGN — scroll position is local state; no wire frame carries it |
| **NO-PAGER** (self:solo negative pin) | NA-DESIGN — no board offline | EVIDENCED — `<bottom strip>` → sheet rim + down arrow only; no pager → negative pin bnd inventory + sheet decode `docs/channel-fill.md` | EVIDENCED — `<bottom strip>` → no pager glyphs on either leg → negative pack pin `docs/captures/journey-audit/289-channel.md` C11 | EVIDENCED — `<bottom band>` → MENU bar visible, no pager → pack hash `ea5cc6d6…` | NA-DESIGN — no wire field; negative design seam is the zero-rect `IDC_BUTTON_SCHANNEL_LEFT/RIGHT` |
| **DIALOG-X-CLOSE** (self:peer X click) | NA-DESIGN — no dialog offline | EVIDENCED — `<X (577,171)>` → bare Planet board → exe pin `~/scratch/planet-verify/findings.md`, t240 `drive.log` | EVIDENCED — `<X>` → dialog closes, emblem strip returns → exe pin `docs/parity/channel.md` | EVIDENCED — `<X>` → `parent+0xF64` phase flag becomes 0 (`CStatePlanet` still mounted) → live memory pin `docs/parity/channel.md` | GAP — `<X>` → whether 2021 is sent on this path is unpinned; OPEN — scripted TCP tap of the X click and compare with the 2021 leave pin `docs/net-protocol.md:136` |
| **LEAVE-2021-REENTRY** (self:master lobby exit) | NA-DESIGN — no channel session offline | EVIDENCED — `<2021 empty>` → `2022 result=0`, next 1004 succeeds → wire pin `docs/net-protocol.md:136`; guards `session.rs:4523/4569` | EVIDENCED — `<2021>` → re-entry never wedges → wire pin `docs/net-protocol.md:136` | GAP — `<2021>` → channel-observer row/state change unpinned; OPEN — observer socket with 2003 poll around a leave | EVIDENCED — `<2021 empty body>` → `2022 0` → scripted Memoryer wire pin `docs/net-protocol.md:136` |
| **ROOM-CREATED** (self:master, #376) | NA-DESIGN — no room offline | EVIDENCED — `<create room>` → `2005 RoomCreated` staged on the host and broadcast → fix `35a9348`, p376 server log tap | EVIDENCED — `<2005>` → peer's room list shows the created row → pack hash `8dd6a9b3…` (`frame-000127-room.png`), rig script `28dce0ba…` | GAP — `<2005 fan-out>` → channel-board paint of the new row unpinned; OPEN — observer at the board with a wire tap while master creates a room | EVIDENCED — `<2005 created, not 2007>` → opcode and broadcast wire pin `docs/net-protocol.md:120`, p376 log |
| **ROOM-DISBAND** (self:last occupant, #380) | NA-DESIGN — no room offline | GAP — `<last exit>` → master's own row after re-entry unpinned; OPEN — master re-enters Channel and captures the board after 3005 | GAP — `<non-last vs last exit>` → row retained/removed paint unpinned; OPEN — two-client capture with 3005 and 2007 | LIVE-FILLED — `<last disconnect>` → observer sees the row vanish → WAVE 1 observer `d4fa3dc6…` | LIVE-FILLED — `<last disconnect>` → `S2C 2007 len=8 hex=00000000` → WAVE 1 tap `e326e3f7…`; #380 fix still OPEN, guessed 2006 refuted |
| **MEMBER-JOIN** (self:peer enters room, #379) | NA-DESIGN — no room offline | OPEN-FIX — `<0BBA join>` → master still sees no joining seat → owner observation #379, p379 3004 wire `b07d63f8…`; fix in flight, not on main | EVIDENCED — `<3003 answer>` → joiner sees master seated → p379 wire `8e8735de…`, fix `5dd8abb` | GAP — `<join>` → 2009 user-count row unpinned; OPEN — channel-observer socket during `0BBA` | EVIDENCED — `<0BBA>` → 3004 (103 B) to master, 3003 (180 B) to joiner, 3007 on leave → wire pin p379 logs |
| **HANDS-KIT** (self:master/peer wearer, #375) | NA-DESIGN — Channel paints no avatar hands; cross-state story is the default kit fixed by `8de4d28` (Waiting/Planet) | NA-DESIGN — same boundary; cross-state story: face-only kit stages bare arms, guard `face_only_wire_kit_stages_the_default_bare_arms` | NA-DESIGN — same boundary; no Channel cell can prove a hand sprite | NA-DESIGN — board observer never sees a room avatar's hands | NA-DESIGN — equipment kit is local data, not a channel wire field |
| **RELAY-DOWN-NOTICE** (environment: relay endpoints down, #348) | NA-DESIGN — no gateway session, so no relay probe | LIVE-FILLED — `<relay down>` → Planet frame clean, notice only after Channel entry → pack hash `417a1aa1…`/`deee19d9…`; exe oracle `baf781b8…` | EVIDENCED — `<relay down>` → notice before the peer can enter a room → pack hash `baf781b8…`, #244 `drive.log` popup→OK→`03EA` | OPEN-FIX — `<relay down>` → exe notice over board+dialog rows (`baf781b8…`), our framebuffer notice over black (`0bae3412…`) → divergence parked by #348 | GAP — `<relay down>` → 1003 hold timing vs popup unpinned; OPEN — tap the client socket from boot to Channel with the endpoints down |
| **NOTICE-OK-DISMISS** (self:peer OK click) | NA-DESIGN — no popup offline | EVIDENCED — `<OK (400,365)>` → popup gone, rows revealed → t240 `drive.log`, guard `relay_down_dialog_holds_the_channel_list_until_ok_is_pressed` | EVIDENCED — `<OK>` → channel board usable → t240 `drive.log`, exe pin `baf781b8…` | EVIDENCED — `<OK>` → post-OK row frame → pack hash `ea5cc6d6…` | GAP — `<OK>` → wire after dismissal unpinned (1004 only on face click); OPEN — tap the socket from OK to the face click |
| **FILL-PROBE** (rig operator tooling, #304) | NA-DESIGN — not a 7.55 behavior; no digit probe offline | NA-DESIGN — paint/pins exist (`channel_digits_select_tens_and_ones_frames`), mechanical rig probe is #304 | NA-DESIGN — tooling seam, not a player observation | NA-DESIGN — tooling seam; `docs/channel-fill.md` no-fill/digit pins stand | NA-DESIGN — tooling seam; server needs no probe story |

Counts: **95 cells** — 52 EVIDENCED, 8 GAP, 3 LIVE-FILLED, 2 OPEN-FIX,
30 NA-DESIGN. Every GAP names its next evidence step. A cell is a story
candidate, not a closure: closing still needs the live caller chain and the
project gates.

## 3. Cross-cell contradictions and limits

- **Relay-down backdrop (new)**: the qualified exe frame `baf781b8…` paints
  the one-OK sheet over the planet board, Channel dialog and its row wells.
  Our live frame `deee19d9…` (two independent boots; the morning's p376
  channel frame is byte-identical) and the root-framebuffer grab
  `0bae3412…` after a 5 s dwell paint the sheet over an all-black canvas.
  The boundary placement itself is correct (Planet frame `417a1aa1…` is
  clean; popup appears only at Channel entry). Graded `OPEN-FIX` because
  #348 owns the parked fix; no code was changed here.
- **Boot 3 was contended**: the preflight line showed another `recomp2jam`
  client (PID 3376825) on 15010 and the boot proceeded. Both clients shared
  the one Memoryer (PID 92326, boot epoch unchanged), so server state matches
  by construction. No wire or server claim is made from that leg; the
  backdrop comparison is a client paint claim. The discipline failure is
  recorded rather than hidden; there are no further boots in this lane.
- **Boot ledger**: attempt 0 failed preflight (`WAYLAND_DISPLAY` leak, exit
  10, no client launched, no cell filled); boot 1 ran on `:412` with sink
  `rig_null_3309226`, `--no-sound`, `route=unseen`/`on_default_sink=0`/
  `preboot_breach=0`, scene Planet→Channel; boot 2 repeated it with a 5 s
  dwell; boot 3 added a root framebuffer grab. All were stopped by their own
  PID files. No server restart or signal. Max 4 boots reached (3 successful).
- The p244 citation text names manifest `e211cc…`; the local `manifest.json`
  hashes to `9ae190…`. The contradiction is kept; the popup frame hash
  `baf781b8…` is independently re-read and is the pin used here.
- The p346 stale A/B said the exe painted `Ch.03` and our client `Ch.01`;
  #364 showed the two legs had different pointer positions. The post-fix
  pointer-matched p361 pair puts the bar on the same rows. This matrix does
  not regrade that old mismatch as a current painter fault.
- #380's initial hypothesis guessed 2006 for room removal. The live WAVE 1
  tap observed 2007 on last disconnect, matching `docs/net-protocol.md`;
  the contradiction is preserved, and the client fix remains OPEN.
- `active` and `capacity` co-vary in every captured row, so the field that
  gates CLOSE stays OPEN even though the refusal behavior is EVIDENCED.
- p370's proxy rewrite is a synthetic field delta: it pins the exe's
  quantization rule, but the unmodified exe frame `ea5cc6d6…` is the claim
  for the ordinary one-segment row. The 1 px strip anchor delta stays a
  named residual.
- Exe glyph rasterization is presentation per the owner decision; the
  emblem-region PSNR 28.04 (below the 35 dB gate) is a font residual, not a
  rest-art mismatch. Server state is shared by construction; no "scenario
  delta" dismissal appears here without a wire/log pair.

## 4. Gap-to-story hand-off

Ranking by player visibility × reachability (top first):

1. Relay notice backdrop (`OPEN-FIX`, #348): visible at every relay-down
   channel entry; new pack and framebuffer grab in hand.
2. Room disband paint (`GAP`×2 + 2 `LIVE-FILLED`, #380): wire proof exists,
   client paint does not; last-leave is a common flow.
3. Master member visibility (`OPEN-FIX`, #379): owner-reported blocker;
   wire pin exists, fix in flight.
4. X-close wire and 2021 boundary (`GAP`×2): cheap scripted tap.
5. Channel-observer row changes on create/join (`GAP`×2): one observer
   socket with a created room covers both.
6. Relay-down socket timing (`GAP`): tap with the endpoints down.
7. #375 hands and #304 fill probe: explicit cross-state/tooling stories,
   no new Channel boot justified.

These rows were added to the #289 body under `## Multi-actor stories` as
checklist stories with an evidence class and a verify step. The existing C9
(`#364`), emblem composition (`#361`), relay notice (`#244/#348`) and no-pager
stories are referenced by their matrix rows, not duplicated. Closing any
story still requires the live caller chain and the project gates.

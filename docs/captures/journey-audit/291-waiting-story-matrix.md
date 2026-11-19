# Waiting story matrix — Spec #291 (WAVE 1 exemplar)

Method: `docs/captures/story-matrix/TEMPLATE.md`. Base `origin/main`
`cbfff2a`; worktree `cb2/story-matrix`. No code changes, no pushes. The
7.55 executable is the oracle (`OTwo.exe`, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`).
Memoryer is a wire witness, not an exe rule. Every local pack path is
git-ignored scratch; only hashes and text enter the repo.

## 1. Evidence inventory (before any gap was marked)

| Source | Pin (full sha256 unless noted) | What it names |
|---|---|---|
| exe identity | `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b` | 7.55 behavior oracle; all static VA/file offsets below |
| `~/scratch/no-response/c3-waiting-qualified` | citation `96339c1f3cc088e913cc8d70d4d271dd5075ee1ae1abdb4d0ba69c1b7e40d3f2`; `waiting.png` `f394dcaa56ad058a11fefebd669fd19669fd9243616b0b955950d2e2e1393162`; `drive.log` `4d6ecd9c088a5ca8bf8d4b2c119248cf8aa8c8b93ac403c214458e27c968204b`; wire `40efa58335b2b97726129edd41ea6f0b13f0e7513183ecbb8072f0ba3a8e3328` | qualified exe guest/joiner Waiting frame: hover-then-press, `0BBA`/`0BBB`, guest-only affordances, plain seat grid |
| `~/scratch/no-response/c1-waiting-observe` | citation `866b0c1a14d6ab5624cf7e669dee6ddabac5a489f8174dff27aa49ad7dd0c799`; `room-row-hover.png` `54be86b26b4c6729af87f5714d98f3e3ee87e504b111780d8aa493f3450867d9` | join row painted before press; guest pill → `C2S:4008`/`S2C:4009`; 60 s Waiting dwell with 6001/4037/5027/4039 |
| `~/scratch/p343-ab` | `proof-final.json` `20fbe17060cea3d3da16a059b74de06a724b6bb41ee6ae2caf947b3ccfc8cff6`; `scores-final.json` `299aaf25c1e6bcac5ea5dbb001c4c27e8a8e6e9c7f24366509626916a3e79789`; exe citation `5c87524095884e9f93097f28d757b7d32e952f9394e6bc7c64cc70c109f86dc2` | matched exe/Rust Waiting after the role-gate fix: `wire_sequence_equal=true`, `server_slice_equal=true`, base SSIM 0.8906, center modal 0.9999; residual seat order/caption text |
| `~/scratch/p343-findings.md` + `~/scratch/p343-wine-role4` | exe `0x77ee86` role word read `3` (Spectator) via stopped-state winedbg; MUSELECT handler file `0xb2622`; popup frame gate file `0xaaee0`, state update file `0xaeb7a` | role-gated popup / seat cards; master dialog rule remains OPEN without a qualified master frame |
| `~/scratch/matched-ab-339` | proof `0580bc8de663b6370f13854b14a52913f7fd1e6c4343d7e3782f6f22cbd636dd`; scores `a664ab77921de13c0b3077211eda3d0a51e7730a13661ab3adddbb487a93e9ed`; `crop-exe-member-downloading.png` `7d4d586cfe0d9235a459a33a114b2d498d3a34019785eb91e3ac11a89ce68b33` | base-ready / member-downloading / gate-closed / back-to-base states; wire mismatch here was later fixed in p343 |
| `~/scratch/solo-playing/s1-master-ui` | manifest `4bd221b87c110e9c556a909affd1637d25ca7d07aff522775037880709dfb48c`; `waiting-master.png` `85decf328e91d98d6b59a607dad78954dbef137dfce92dba59bab97ee95af91e` | exe creates `SOLO` room itself (`07D4`); master START face at (680,143); full `4010/4011/4012/4013/4017/4018` flow |
| `~/scratch/p376-live`, `~/scratch/p376-findings.md`, `~/scratch/p376b-findings.md` | rig script `28dce0ba36be52e9d046efb234011415e199b131dc5d95dbef19ad94235b6723`; frames `frames-b/frame-000127-room.png` shows live `P376LIVE` row | two-client 2005 `RoomCreated` broadcast and entry fix; owner pin "empty channel = empty list"; server state shared |
| #376 (closed), #379 (open), #380 (open), #375 (closed) | owner observations 2026-11-19 morning | peer cannot see created room (fixed); master does not see joining peer (fix parked); room stays after last leave; hands missing (fixed by `8de4d28`) |
| `~/scratch/live-4-findings.md` | master packs `f1fadaa004bd96191e23eaa445b51e3dc35352325c96c8469ec325228f6888d7` (manifest), `cb31209e8844ecfac0dfe5eb40427e890d575aed558f0e0f961e266c48da726b` (citation) | 4 exe boots spent; no qualified master MUSELECT dialog frame (driver crash), proxy rewrite was a synthetic delta, not exe evidence |
| `docs/wine-boot.md`, `docs/net-protocol.md` | 15010 at file `0x2513d8`; dispatcher id at buffer+2; 2005 created / 2007 removed; 3002/3003 join; 4008 ready; 3011–3013 chat; 07E8 badge; fuse at +36–44 s | static/corroborated wire and UI pins; server config drift must be re-read per leg |
| WAVE 1 live legs (`~/scratch/story-matrix-live/`) | leg2 tap `e326e3f7eaae0cd88dfa9e8735d7554c5c73dd6fc7b376017dbb28ffb5d2834a`; observer `d4fa3dc6ff6dada19acbd7ced46eaeb7e21cb7743b07a74865445aea463ad680`; leg3 tap `8f33300fe66f76cc6d172ef0b66063200a9e1aac737588a1e8d787a7ec128790` | 4 boots spent, 2 failed before pixels. New wire pins: `2004→2006` create result 0, `S2C 2011` two-member list, `S2C 2007 len=8 hex=00000000` on last disconnect. No qualified two-client Waiting frame from this wave. Contradiction recorded: #380 guessed 2006 for removal; doc + live tap say 2007. |

Containment for every WAVE 1 boot: private Xvfb, per-run `rig_null_*` sink,
`--no-sound`, `route`/preboot checks by `rig-run.sh`, server PID 92326 with
boot epoch `1794986056` unchanged, stop by recorded PID files only. Boot 1/2
stalled at Planet (no input drive); boot 3 stalled at Channel behind the blue
NORMAL-mode popup; boot 4 drew `1001 feffffff` from a duplicate account
session. All are failed attempts, never evidence of Waiting behavior.

## 2. Matrix (13 actions × 5 observer actors = 65 cells)

Cell key: `EVIDENCED` / `GAP` / `LIVE-FILLED` / `OPEN-FIX` / `NA-DESIGN`.
`OPEN — <step>` is mandatory in every gap. A cell cites one admissible class
from the template; guards alone never prove exe behavior.

| Action (raised by) | solo | master | peer | channel | server |
|---|---|---|---|---|---|
| **ENTER-CREATE** (self:master) | NA-DESIGN — no solo session | EVIDENCED — Waiting plain after create; p343 proof `20fbe170…`, `p343-ab/{exe,rust}` frames | NA-DESIGN — no peer exists at create | EVIDENCED — live row from 2005; #376 two-client frame `frame-000127-room.png` | LIVE-FILLED — WAVE 1 leg2 tap: `2004 → 2006 result=0,number=0`, `e326e3f7…` |
| **ENTER-JOIN** (self:peer) | NA-DESIGN — no solo session | EVIDENCED — seat grid stays plain on arrival; c3 citation `96339c1f…`; joiner visibility is row 10 | EVIDENCED — lands in Waiting; c3 `waiting.png` `f394dcaa…`, p343 proof | GAP — user-count event on join unpinned; OPEN — scripted tap of 2009/2003 around `0BBA` | EVIDENCED — `C2S:0BBA / S2C:0BBB`, c1 wire `40efa…` (matched route #339/#343) |
| **SOLO-IDLE** (self:solo) | EVIDENCED — solo master room + START face; `s1-master-ui` `waiting-master.png` `85decf32…` | NA-DESIGN — same seat is the solo actor | NA-DESIGN — no peer | NA-DESIGN — no channel observer in solo room | EVIDENCED — 6001/4037/5027/4039 async cadence, c1 `drive.log` + wire |
| **MUSELECT** (self:master) | GAP — no solo-master dialog frame; OPEN — one qualified solo master MUSELECT capture | GAP — static handler `0xb2622` + role-2 gate `0xaaee0`, but no qualified exe master dialog frame (live-4 boot 4 crash); OPEN — re-run master drive with pixel delta fixed | EVIDENCED — guest hides popup/BPM cover; c1 cue inventory, c3 guest frame, p343 role-3 read | NA-DESIGN — dialog is modal inside Waiting | EVIDENCED — no 4051/4053 frames in c1/c3/p343/p346 windows (#266); negative wire pin |
| **PICK-ECHO** (self:master) | GAP — no solo pick/echo frame; OPEN — before/after pick capture with frame hashes | EVIDENCED — owner observation #274: exe `SPEED_NUM`/BPM read ROOM echo, not staged pick; `docs/caption-sources.md` | EVIDENCED — guest keeps BPM cover after room echo; c3 frame, #274/#275 pins (visual confirm still OPEN) | GAP — room-list song field after 2008/2023 not pinned; OPEN — tap 2008/2023 and read painted row | GAP — no 4000/4001 body pin in matched packs; OPEN — scripted-gateway 4000 echo guard |
| **READY** (self:peer) | NA-DESIGN — no peer to ready | EVIDENCED — seat ready mark / gate input; static pin file `0xb237d`, clear VA `0x4b2540`, #163 seat guards | EVIDENCED — `C2S:4008 / S2C:4009`, stays Waiting; c1 `drive.log:52-55` + wire `40efa…` | GAP — 2009 user-count on ready not pinned; OPEN — scripted tap during 4008 | EVIDENCED — 4008/4009 wire pin, `docs/net-protocol.md` §4; c1 matched window |
| **START-GATE** (self:master) | EVIDENCED — exe emits `4010` then full gate/score flow; `s1-master-ui` manifest `4bd221b8…` | EVIDENCED — gate order readiness → team → music, VA `0x4b5560`; #276 close record + solo pack | EVIDENCED — host-driven start leaves Waiting to Loading; p343 proof, c1 `host/host.log:13` | GAP — 2020 RoomStateChanged on start unpinned; OPEN — channel-observer socket during 4010 | EVIDENCED — `4010/4011/4012/4013` wire family, solo pack + `docs/net-protocol.md` |
| **TEAM-CHAT** (self:any seated) | GAP — no solo chat/team frame; OPEN — one solo-master chat + 4004 toggle capture | GAP — master emblem still OPEN (#303); team guards exist but no qualified master frame; OPEN — two-client emblem capture | GAP — Waiting chat row feed has no exe reader pin; OPEN — 3012/3013 payload-to-row capture | NA-DESIGN — Waiting chat is not channel chat | EVIDENCED — 4004/4008, 3011/3012/3013 layouts, `docs/net-protocol.md` §4c (Memoryer corroboration + exe send sites) |
| **DOWNLOAD** (self:server) | GAP — no solo-master badge frame; OPEN — matched solo download capture | EVIDENCED — `states/gate-closed.png`, `member-downloading.png` in matched-ab-339; p343 proof | EVIDENCED — 4026/4037/3009 matched wire + badge crop `7d4d586c…` | GAP — no channel-visible download state; OPEN — 2003 row state probe during download | EVIDENCED — `S2C:4026/4037/3009` in p343 proof; 5012 completion pin in `docs/net-protocol.md` |
| **PEER-JOIN** (self:peer, while Waiting) | NA-DESIGN — no peer | OPEN-FIX — **master observes peer joining**: owner #379 "master sees nobody joining", fix parked per owner; wire shows join, client does not stage it; no LIVE-FILLED frame (4 boots spent) | EVIDENCED — joiner sees master seated; c3 `waiting.png` `f394dcaa…`, #376 peer-side fix | GAP — 2009/2003 user-count change on join unpinned; OPEN — channel-observer socket with a real join | EVIDENCED — `0BBA/0BBB` c1/c3 wire + #376 `2005` broadcast pin; server state shared by both clients |
| **PEER-LEAVE** (self:peer, non-last) | NA-DESIGN — no peer | GAP — no frame of peer seat clearing; OPEN — master frame before/after a real 3005 ExitWaiting | GAP — leaver's exit-to-Room frame unpinned; OPEN — one leaver-side capture with `3005/3006` | GAP — room stays listed but no non-last wire pin; OPEN — observer 2003 poll with master resident | GAP — no member-removal frame body pinned for non-last exit; OPEN — tap master socket during 3005 |
| **LAST-LEAVE** (self:last occupant) | NA-DESIGN — no room after last exit | NA-DESIGN — no master after last exit | NA-DESIGN — no peer after last exit | LIVE-FILLED — WAVE 1 leg2 observer saw the room vanish; `observer.log` `d4fa3dc6…` | LIVE-FILLED — WAVE 1 leg2 tap `S2C 2007 len=8 hex=00000000` on final disconnect; `tap.log` `e326e3f7…`. #380 fix still OPEN |
| **MASTER-LEAVE-HANDOFF** (self:master, peer remains) | NA-DESIGN — no solo session | GAP — no master exit frame; OPEN — master-side 3005 capture | GAP — peer-becomes-master observation unpinned; OPEN — two-client frame pair with role word `+0x30` | GAP — room remains listed under new master; OPEN — 2003 row poll after master exit | GAP — server transfer/no-disband wire body unpinned; OPEN — scripted tap of 3005 with resident peer |

Counts: **65 cells** — 24 EVIDENCED, 22 GAP, 3 LIVE-FILLED, 1 OPEN-FIX,
15 NA-DESIGN. Every GAP names its next evidence step; the two LIVE-FILLED
cells are wire-only and do not close #380.

## 3. Cross-cell contradictions and limits

- #380's initial hypothesis said `RoomRemoved` is 2006. `docs/net-protocol.md`
  says 2007, and the WAVE 1 tap observed 2007 on last-player disconnect. The
  contradiction is preserved here; 2007 is the live wire value.
- The four boots did not yield a qualified two-client Waiting pair. The
  #375 hands fix (`8de4d28`) landed on main and is not re-proven here.
- Server state is shared by construction (one Memoryer, PID 92326); no
  "scenario delta" dismissal is allowed without a wire tap. Leg2 and leg3
  taps are the wire half; neither contains a Waiting paint half.
- A p343 base SSIM of 0.8906 is a region comparison, not proof of an actor's
  observation. Caption and seat-order residuals stay MISMATCHED per the
  journey audit; this matrix does not re-grade them.

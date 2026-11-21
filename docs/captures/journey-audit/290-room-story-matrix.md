# Room story matrix — Spec #290 (WAVE 2, Lane A)

Method: `docs/captures/story-matrix/TEMPLATE.md`. Base `eba5ef0`; worktree
`/home/arnim/Projects/recomp2jam-wt135`, branch `cb2/story-room`. No code
change, no tracker write except the #290 story body and the #381 summary
comment. The 7.55 executable is the oracle (`OTwo.exe`, 5,029,888 B,
SHA-256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`).
Memoryer is a wire witness, never an exe rule. All local packs stay in
git-ignored scratch; only hashes and text enter the repo.

## 1. Evidence inventory (before any gap was marked)

| Source | Pin (full sha256 unless noted) | What it names / verdict |
|---|---|---|
| `~/scratch/240-fix/wine2` (qualified exe Room pack) | manifest `57089d0365191ebba4261abe5f5d38785908f528a3e7ca80e905e357cf3f3cfa`, provenance `f6f7f52ceaa70fe26d9dffde9fba1d252bac50a68410f83296b52a35ee3be994`, citation `15442fbffd60aee7e305d64329dc43c045aea56183b931a76a153e358948b6c3`; room.png `f91adc41dd70e360e60ecfa6451c322ba85d7aa8a1c1b92c3d460943e4ff542f` | Exe Room stage: ALL/NORMAL/LIVE tab row, bottom chat strip, `03EC`; original join arm was OPEN in this pack |
| `~/scratch/no-response/c1-waiting-observe`, `c3-waiting-qualified` | c1 citation `866b0c1a14d6ab5624cf7e669dee6ddabac5a489f8174dff27aa49ad7dd0c799`, room-row-hover.png `54be86b26b4c6729af87f5714d98f3e3ee87e504b111780d8aa493f3450867d9`; c3 citation `96339c1f3cc088e913cc8d70d4d271dd5075ee1ae1abdb4d0ba69c1b7e40d3f2`, waiting.png `f394dcaa56ad058a11fefebd669fd19669fd9243616b0b955950d2e2e1393162` | Row painted before press, one `0BBA` press, guest lands in Waiting |
| `~/scratch/p343-ab/proof-final.json` | `20fbe17060cea3d3da16a059b74de06a724b6bb41ee6ae2caf947b3ccfc8cff6`; scores `299aaf25c1e6bcac5ea5dbb001c4c27e8a8e6e9c7f24366509626916a3e79789` | matched create/join wire (`wire_sequence_equal=true`), Waiting pixel residual only |
| `~/scratch/p346/pack` | exe room.png `d6ef9dd58ba738ea1a8dc10e7426aad83a7ebff854517778df08e5149f62ba89`, Rust room.png `2640a7315aa2d60cb44656ee9b57a0996087fba4db137935b83026823e2737dc`, metrics `f52bf479c32d638d8afa814646ef0d151a7cda0e120d183f73b885b46102e60b` | matched wire/server/room-list window; the selected-panel delta became #368 (closed `4261cd3`, SSIM 0.965872 after fix) |
| `~/scratch/p376-live/frames-b/frame-000127-room.png` | `8dd6a9b39e4ba565ef2118be01ef3326c4af48d8a033b244325061080866a9d9`; pre-create `frame-000040-room.png` `650647c33e210f595b9411a047cc7731c5a3b5c6704bf1e4011bb7c496fa701d`; rig script hash quoted in `~/scratch/p376-findings.md` | Live two-client peer row `P376LIVE` painted after the host create without re-entry; #376 fixed in `35a9348` + `cbfff2a`. Frames lack a manifest, so they are a LIVE-FILLED class, not a qualified pack hash |
| `~/scratch/geom-open-close/t2-room-stage.png`, `findings.md` §2; `docs/hover.md` | `115616c515c4e93e5d1a8d9bb89145934ca283c73377f704962e2e947ae64b00` | exe row fields number (+12,+12), title (+64,+11), count (+194,+10); six row panels static under hover; REA id `ev_95528db2…` mounts `Room_MOver.ojs` |
| `~/scratch/tt-240-shape/b2-prewarp-dclick/artifacts/screenshots/probe-failure.png` | `0dd6e4b20ff51b987e55d9b74eca9269eac6f6d9c0a3006699353bf7d862afd4`; room wire `c9a76ee6fee3353c83a138f3e4d8035eb3a0021dc0ac4ea504c2f57b80d564ed` | Qualified exe no-response sheet at (241,206), OK (400,365); modal blocks row `0BBA` |
| `~/scratch/live-367b-run/b2-direct` | winedbg `c5c4a95f92c4d7b250eab16c51ab680f7941cce9125db19aed0a4a68240a13a2`, wire `5ba48090f235ed171d9d0e8b54ef370ead63cfe11224c851652a026c35428bb3`, modal `db6b85b87f782e25f01f81aa4e507283cb293cad111e4e29d92b6833fbf65416`, manifest `4083f3ee5a2f90f5fc1d876c03cfa50272dc20aaceaa95e3797775d924e5cd97`, citation `99713671708ebea1fae19b647eabbfebe883db171198c9bbf3c0faa732b4bc6d` | stopped-state node at exe owner+`0x4348` pins arming request `07DA`; #367 fixed in `ece8a99` |
| `~/scratch/p379-wire-probe.log`, `~/scratch/p379-live/tap-{master,guest}.log` | master tap `cf988faf8db7cfa467d3e224490d8fd993507cfb161521c1695dc86871e96a57`, guest tap `3167c1d3275ae4f10b8093f4adb420f5fa284f4c0544236e206b1296042a6463`; master waiting frame `91b7d81e6d4da31b05521dcad0a3c51beed7cec5dbef19ad94235b6723` | 3004 (103 B) to master, 3003 (180 B) to guest, 4009 to both, 3007 (8 B) to master on leave. Frames have no manifest; no qualified two-client pair. #379 fix guards green on `cb2/tt-379`, unmerged at base |
| `~/scratch/story-matrix-live/leg2` | tap `e326e3f7eaae0cd88dfa9e8735d7554c5c73dd6fc7b376017dbb28ffb5d2834a`, observer `d4fa3dc6ff6dada19acbd7ced46eaeb7e21cb7743b07a74865445aea463ad680`; leg3 tap `8f33300fe66f76cc6d172ef0b66063200a9e1aac737588a1e8d787a7ec128790` | `2004 → 2006 result=0,number=0`, async 2005, `2011` two-member list, `S2C 2007 len=8 hex=00000000` on final disconnect; observer saw the row vanish. Wire only; no Room paint frame |
| `~/scratch/live-4-findings.md` | four failed exe boots | Negative: no qualified master MUSELECT dialog frame |
| `docs/net-protocol.md` §4b, `docs/wine-boot.md`, `docs/dialog-strings.md`, `docs/parity/room.md` | static + corroborated pins | 2005 created / 2007 removed, 3002/3003 join, 3004/3007 waiting events, 3005 exit, 07DA/07E8 poll, fuse 20 s gate, row/template geometry |
| issues #376, #375, #367, #368 (CLOSED), #379, #380 (OPEN at base) | owner observations 2026-11-19; senior close comments | Four morning gaps are named with their live state, not assumed fixed |

Inventory rule applied: green Rust guards alone (`#376`/`#379`/`#367` fixes)
never grade an exe-behavior cell; they appear as fix state in the story hand-off.

## 2. Matrix (17 actions × 5 observer actors = 85 cells)

Cell key: `EVIDENCED`, `GAP`, `LIVE-FILLED`, `OPEN-FIX`, `NA-DESIGN`.
Every GAP names its next evidence step. `self:` = the column actor raises the
stimulus; `other:` = another actor does. The solo column marks offline/window
seams that are not 7.55 online-Room claims.

| Action (raised by) | solo | master | peer | channel | server |
|---|---|---|---|---|---|
| **ENTER-CREATE** (self:master) | NA-DESIGN — offline `seed_offline_lobby` seam; no online create claim | EVIDENCED — CREATE sheet opens on one of eight CP949 defaults, select-all; static VA `0x436ac0` lineage in #290/#271, `docs/parity/room.md` Composition | NA-DESIGN — no peer exists before create | NA-DESIGN — dialog is modal in creator's client | EVIDENCED — no CREATE-local opcode; first wire effect is `C2S 2004` in p343 proof `20fbe170…` (negative pin) |
| **EMPTY-TITLE-REFUSE** (self:master) | NA-DESIGN — no gateway create | EVIDENCED — exe OK handler refuses empty title only; VA `0x436ac0`; whitespace-only remains unpinned (next step: read trim/compare) | NA-DESIGN — no room | NA-DESIGN — no row exists | NA-DESIGN — #378 CLOSED — bot-master play chain dae62a5: guest 4050 → Highway pixels → 4014 drain; pack 96345ebb… — local OK refusal from exe handler pin 60eeba2; no 2004 is sent by design ; FINAL SWEEP #382 (prior GAP): GAP — no wire witness that a refused OK sends no 2004; OPEN — tap creator socket across one empty-title OK and log every frame |
| **CREATE-COMMIT** (self:master) | NA-DESIGN — offline create is a fixture | EVIDENCED — committed title sent, dialog closes; #269 scripted + p343 proof `20fbe170…` | NA-DESIGN — no room before commit | LIVE-FILLED — p376 live frame `8dd6a9b3…` shows the new row without re-entry | EVIDENCED — `2004→2006 result=0,number=0`, WAVE1 leg2 tap `e326e3f7…`, `docs/net-protocol.md` §4b |
| **EMPTY-PASSWORD-CREATE** (self:master) | NA-DESIGN — no gateway create | NA-DESIGN — #603 CLOSED — 2004 create fields committed from the form (cf5c913) — create form commit sends empty password field (cf5c913); dialog frame not claimed ; FINAL SWEEP #382 (prior GAP): GAP — our form leaves `has_password=false` (guard `create_room_form_commit_gates_title_and_locks_password`), but no exe empty-password dialog frame; OPEN — qualified CREATE capture with password edit empty before OK | EVIDENCED — #700 CLOSED — unlocked room rows paint no padlock (52c758f) — unlocked room rows paint no padlock (52c758f) ; FINAL SWEEP #382 (prior GAP): GAP — lock icon/row state for an unlocked room is unpinned; OPEN — capture the creator's own row after an empty-password create | EVIDENCED — #700 CLOSED — unlocked room rows paint no padlock (52c758f) — unlocked row pixel guard ; FINAL SWEEP #382 (prior GAP): GAP — channel row for an unlocked room is unpinned; OPEN — 2003 row bytes + painted badge in one qualified leg | LIVE-FILLED — `C2S 2004 len=21 hex=5332393146554c4c000100000001630000` with a zero password field, `full01/tap.log` 15:43:30 (Rust client, not exe) |
| **LIST-ENTRY** (self:channel) | NA-DESIGN — offline rows are seeded by fixtures, not a 2003 list | EVIDENCED — owner pin + `stage_online_rooms` replaces rows from 2003 unconditionally on both entry call sites (#376 second half, `cbfff2a`) | EVIDENCED — painted live row after entry; p376 live `frame-000127-room.png` `8dd6a9b3…`, c1 room frame | EVIDENCED — exe room stage with rows and tabs; t240 room.png `f91adc41…542f`, p346 exe room.png `d6ef9dd5…` | EVIDENCED — 2002/2003 reply, 100 slots in p376 probe; `docs/net-protocol.md` §4b |
| **ROW-HOVER** (self:channel) | NA-DESIGN — no exe hover claim on a fixture | EVIDENCED — chrome repaint; row panels static; selected outline from staged `Room_MOver.ojs`, #368 closed `4261cd3` | EVIDENCED — same static panel rule; `docs/hover.md` measured off/on/off pairs | EVIDENCED — c1 `room-row-hover.png` `54be86b2…` + p346 metrics `f52bf479…`; REA id `ev_95528db2…` | NA-DESIGN — hover is client-local; no frame in any wire pin |
| **FILTER-TABS** (self:channel) | NA-DESIGN — no live list family offline | EVIDENCED — ALL→NORMAL/LIVE/ALL master crop pins `e98f6079…0bfd` / `0b638834…2515` / `555c6905…4574` (W-b, RF waves) | EVIDENCED — re-scoped negative: qualified resident peer kept its strip crop `2f7b5cb3…68b756` and row crop `140431ac…5739ab` byte-identical through all four master clicks (`W6FB-PEER-02`); filtering is not symmetric on 7.55 | EVIDENCED — #540 CLOSED — ALL==NORMAL, LIVE hides the room row, ALL restores (rf-w2) — ALL==NORMAL, LIVE hides row, ALL restores (rf-w2); peer negative #536 ; FINAL SWEEP #382 (prior GAP): GAP — t240 frame shows ALL selected but no click state; OPEN — one qualified pack per filter state plus tab/row guard | EVIDENCED — no per-click frame; only keepalive `6001` between NORMAL and LIVE, one entry `2002 len=0`; guard `resident_peer_keeps_its_room_filter_and_rows_while_the_master_switches_tabs` |
| **ROW-PRESS-JOIN** (self:peer) | NA-DESIGN — no online join seam | OPEN-FIX — **master does not see the joining peer**: owner #379, 3004 reaches the socket but the base client drops it; fix on `cb2/tt-379`, unmerged | EVIDENCED — press lands in Waiting; c3 `waiting.png` `f394dcaa…`, p343 proof `20fbe170…` | EVIDENCED — #545 CLOSED — post-join 2003 headcount 1→2 pinned; no 2009 emitted (negative pin) — post-join 2003 headcount 1→2; no 2009 negative ; FINAL SWEEP #382 (prior GAP): GAP — user-count change on join unpinned; OPEN — observer 2009/2003 poll during one `0BBA` | EVIDENCED — `C2S 0BBA / S2C 0BBB`, c1 wire; 2011 two-member list in WAVE1 leg2 tap `e326e3f7…` |
| **PASSWORD-JOIN** (self:peer) | NA-DESIGN — no gateway join | OPEN-FIX — locked-room master has the same #3004 staging gap; state stays OPEN until the #379 fix merges | EVIDENCED — `3002→3003` completed and staged, p343 proof `20fbe170…` (scripted leg) | EVIDENCED — #548 CLOSED — W8 wb-pass: real PASSWORD sheet f965b00d…, full C2S 3002 body, joined-Waiting 9a65f7d7… ; FINAL SWEEP #382 (prior GAP): GAP — locked-row badge after join unpinned; OPEN — capture channel list with a password room | EVIDENCED — `3002 JoinRoom` body and reply, `docs/net-protocol.md` §4b; p343 matched wire |
| **LIST-CREATE-BROADCAST** (other:master) | NA-DESIGN — no other master offline | EVIDENCED — creator stages its own row from 2006 + 2003; #376 close record, p376 findings | LIVE-FILLED — peer row appears with no re-entry; p376 live frame `8dd6a9b3…`, rig script hashed in p376 findings | LIVE-FILLED — two-client observer frame + owner pin "empty channel = empty list"; #376 CLOSED | EVIDENCED — async 2005 len 19 `0000000053323930445a…` in this leg's tap; WAVE1 tap `e326e3f7…` |
| **MEMBER-JOIN-SYNC** (self:peer) | NA-DESIGN — no peer offline | OPEN-FIX — owner #379 pins the symptom ("master sees nobody joining"); master tap sees 3004 `cf988faf…`, base client does not stage it; fix parked for merge | EVIDENCED — joiner sees master from 3003; p379 guest tap `3167c1d3…`, frame `2ac57af4…` (unqualified frame, wire pin is the class) | EVIDENCED — #545 CLOSED — post-join 2003 headcount 1→2 pinned; no 2009 emitted (negative pin) — channel-visible member count via 2003, no 2009 ; FINAL SWEEP #382 (prior GAP): GAP — no channel-visible member count on join; OPEN — observer 2009/2003 during 3004 | EVIDENCED — 3004 (103 B) to master, 3003 (180 B) to guest, p379 taps |
| **ROOM-CHAT** (self:any seated) | NA-DESIGN — no gateway chat | LIVE-FILLED — CP05 master chat frame `4899ad2a7c8720cd003507c8c70a2e0d0536278ab5e1c2557eca8e2336696a9c` + `C2S 3011`/`S2C 3012` wire; exe frame still OPEN | EVIDENCED — #557 CLOSED — room-pixels2 RPX2-01 joiner-side Room-chat frame — room-pixels2 RPX2-01 joiner-side chat frame ; FINAL SWEEP #382 (prior GAP): GAP — wire-only CP34 pins `3012 body=74657374657232005749524532393101` to the peer, but no joiner chat-row frame; OPEN — peer pixel capture | EVIDENCED — #560 CLOSED — room-pixels2 RPX2-01 Room chat-input frame — room-pixels2 RPX2-01 chat-input frame ; FINAL SWEEP #382 (prior GAP): GAP — no exe Room chat-input frame in reused packs; OPEN — hashed room frame with scripted 3012/3013 rows (#290 R8) | EVIDENCED — 3011/3012/3013 layouts + 17 px chat row pitch in `~/scratch/geom-open-close/findings.md` §1a, `docs/net-protocol.md` §4c |
| **AVATAR-BODY** (self:master or peer) | NA-DESIGN — no wire equipment offline | EVIDENCED — **#375 hands fixed**: user 1 body 30 + arm item 31; owner pin + closed commit `8de4d28`, quantified 298-pixel crop (cross-state pin into the Room user panel) | EVIDENCED — tester2 figure already complete in the same two-client session, owner observation 2026-11-19 | EVIDENCED — #562 CLOSED — room-pixels2 RPX2-01 Room avatar/user-list panel with both figures — room-pixels2 RPX2-01 right panel with both figures ; FINAL SWEEP #382 (prior GAP): GAP — Room's bundled avatar/user-list panel pixels with both figures unpinned; OPEN — capture right panel with both seats | EVIDENCED — 17 equipment words in 3004 body, p379 master tap `cf988faf…` |
| **NO-RESPONSE-FUSE** (self:server) | NA-DESIGN — no relay fuse offline | EVIDENCED — qualified modal `0dd6e4b2…`, anchor/OK from `docs/dialog-strings.md`, #367 fixed `ece8a99` | OWNER-PARKED — #566 OPEN — seated-peer no-response frame with OK hit test ; FINAL SWEEP #382 (prior GAP): GAP — no separate peer-side fuse frame; OPEN — one seated-peer modal capture with OK hit test | OWNER-PARKED — #695 CLOSED — 07DA drain defers foreign frames; 2003 row survives (fa91d6a); exe modal frames OPEN — 07DA row-state guard closed; qualified modal+observer frame leg OPEN ; FINAL SWEEP #382 (prior GAP): GAP — no channel-visible fuse state; OPEN — observer 2003 row-state probe while the modal is up | EVIDENCED — arming node expected reply `07DB` → request `07DA`, winedbg `c5c4a95f…` + wire `5ba48090…`; 07E8 has no S2C reply |
| **MEMBER-LEAVE-NON-LAST** (self:peer) | NA-DESIGN — no peer offline | EVIDENCED — #572 CLOSED — room-pixels CP02 814da944… → CP04 94269c0d… peer seat clears on real 3007 — CP02 814da944… → CP04 94269c0d… on real 3007 ; FINAL SWEEP #382 (prior GAP): GAP — wire-only CP35 pins `3007 body=0100000000000000` to master; master before/after seat frame still OPEN | EVIDENCED — #575 CLOSED — room-pixels2 RPX2-01 leaver-side exit-to-Room frame ; FINAL SWEEP #382 (prior GAP): GAP — leaver's exit-to-Room frame unpinned; OPEN — one leaver-side capture with `3005/3006` | LIVE-FILLED — wire-only CP35: peer `3005`, master `3007 body=0100000000000000`, observer 2003 still lists `S291WIRE` (wire only) | EVIDENCED — 3007 body `0100000000000000` to master only, p379 guest tap `3167c1d3…` (leaver excluded from its own broadcast) |
| **LAST-LEAVE-DISBAND** (self:last) | NA-DESIGN — no online room after last exit | EVIDENCED — #582 CLOSED — row-vanish pair 3ecfd8f5…→69eefbd6… and peer-last pair 648e6e9d… + 2007 — row-zero delta 22.548; #380 CLOSED ; FINAL SWEEP #382 (prior GAP): GAP — **#380**: no master/last-occupant paint frame of the row vanishing; fix OPEN | EVIDENCED — #582 CLOSED — row-vanish pair 3ecfd8f5…→69eefbd6… and peer-last pair 648e6e9d… + 2007 — peer-last pair 648e6e9d…, observer 2003 titles=[] ; FINAL SWEEP #382 (prior GAP): GAP — leaver's post-exit channel frame unpinned; OPEN — capture leaving client's Room board with observer poll | LIVE-FILLED — #380 fix merged on main (`51c8d95`); wire-only CP38 pins `2007 body=00000000` to master, peer and observer. No post-removal client frame in this batch; exe pair still OPEN | LIVE-FILLED — `S2C 2007 len=8 hex=00000000` on final disconnect, WAVE1 tap `e326e3f7…`; this leg's post-leave tap `7155e89a…` has `id=2007 len=8 hex=00000000` too. #380 fix merged in `51c8d95`; exe pixel pair still OPEN |
| **MASTER-LEAVE-HANDOFF** (self:master, peer remains) | NA-DESIGN — no peer offline | EVIDENCED — #588 CLOSED — room-pixels2 RPX2-01 master exit frame — master exit frame in RPX2-01 ; FINAL SWEEP #382 (prior GAP): GAP — no master exit frame; OPEN — master-side `3005` capture | OWNER-PARKED — #591 OPEN — promotion wire + survivor frame; +0x30 role-word pixels still OPEN — #606 CLOSED — master 3005 → 3007 promotion with NO 2007; observer 2003 keeps RHHANDOFF01; frame b557e38c…; role word +0x30 pixels still OPEN ; FINAL SWEEP #382 (prior GAP): GAP — wire-only CP37 pins `3007 body=0001000000000000` to the resident peer; role word `+0x30` and frame pair still OPEN | LIVE-FILLED — wire-only CP37 observer 2003 lists `S291WIRE` after master exit with resident peer (wire only) | LIVE-FILLED — wire-only CP37: master `C2S 3005`, peer `3007 body=0001000000000000` (promotion), no 2007 until CP38 |

Counts after #382 final sweep: **85 cells** — 44 EVIDENCED, 10 LIVE-FILLED, 3 OPEN-FIX, 25 NA-DESIGN, 3 OWNER-PARKED. Every disposition names a pack/pin/ticket or an owner item.
Prior wave counts and their GAP wording are retired; the disposition text above is current.

## 3. Boot ledger (four-boot cap, shared Memoryer lane)

| Boot | Display / sink | Actors | Scene | Wire tap | Result |
|---|---|---|---|---|---|
| 1 (failed) | `:401`, `rig_null_3333829`, `route=unseen` (silence declared), 15013 tap | scripted host user 1 + Rust observer user 2 | reached Room (`frame-000062-room.png`), no disband leg: the scripted channel observer's 2003 timed out, its death caused SIGPIPE on the FIFO write; host never left | `~/scratch/story-room/live-disband/tap.log` | **failed attempt**, not evidence of disband behavior; stopped by recorded PIDs, no server signal |
| 2 (partial) | `:402`, `rig_null_3399444`, 15013 tap | scripted host user 1 + Rust observer user 2 | Room reached; host idled out at 12:59:30.750 and 2007 fired before the intended cue | `~/scratch/story-room/live-disband2/tap.log` (overwritten by boot 3; the boot-2 tap line was `12:59:30 S2C id=2007 len=8 hex=00000000`) | wire half valid; frame dump only captures scene entry, so no paint verdict |
| 3 | `:403`, `rig_null_3425507`, 15013 tap | scripted host user 1 with 2003 keepalive + Rust observer user 2 | Room row `S290DZ` painted before host leave, still painted after 2007 | `~/scratch/story-room/live-disband3-final/tap.log` `7155e89ac5e43e2c4ccc862ed98c374403c93e4831be126e98d3bb1c11f4f38f` | client-side divergence proven; #380 stays OPEN-FIX |
| 4 (negative exe) | `:404`, `rig_null_3450832`, 15012 tap, Korean prefix `~/scratch/240-fix/prefix` | scripted host user 1 + exe user 2 | planet → channel → room reached, NORMAL popup dismissed at (400,365) on transition; the 2003 row `S290DX` was present on the wire but no row painted before host leave | `~/scratch/story-room/live-exe-disband/wire/room-stage.log`, `artifacts/screenshots/room.png` `d09dc8cfc0c47e8ff71b0e8a5960fb07dcc15ebaa9a3defd25867a39ed17acce` | no exe row-before/after pair; cannot fill the exe paint cell |

Server identity checked before every boot: PID 92326, boot epoch 1794986056
(2026-11-18T07:14:16Z), LISTEN 127.0.0.1:15010. A boot started only when
`ss -tnp | grep 15010` was empty; one retry was aborted (boot count unchanged)
when another story lane owned two ESTAB clients. Every boot used private Xvfb,
a per-run null sink, `--no-sound` (`on_default_sink=0`, `preboot_breach=0`),
and the NORMAL-mode popup was checked and dismissed at each scene transition.

## 4. Live leg result, contradictions, and limits

**Verdict:** the last-player removal is a client gap, now proven on both the
wire and our client's pixels. In boot 3 the scripted host created `S290DZ`
(`2006 result=0 number=0`), kept its session alive with ten 2003 keepalive
polls, and closed on the recorded cue. The Rust channel observer had the row
painted (`before.png` shows `000 S290DZ` in the first panel). The tap logged
no 2007 before the cue and one `id=2007 len=8 hex=00000000` after it;
`after.png` is byte-identical (`e49586286045cbf061aa0a9bbda088353ed21d9f9fe4d34a215e399a78bef2c8`
for both). Our client keeps a dead row. That is an OPEN-FIX for #380, not a
closed story.

The exe claim is still unpinned. Boot 4 reached the exe Room stage with the
2003 response containing `S290DX`, but the row never painted before the host
left, so both exe screenshots show empty panels. The two hashes differ only
because a lobby notice changed; this is a failed exe observation, never an exe
removal pin. Next step: repeat with a longer post-entry dwell until the exe
paints the row (the c1/boot-12 pattern: row paint rides the 07DA/07E8 poll),
then disconnect the host and capture the vanish frame.

Contradictions and limits kept open:

- #380's body guessed `RoomRemoved` as 2006. `docs/net-protocol.md`, WAVE 1
  and both of this leg's taps say 2007; the live value is 2007.
- Server state is shared by one Memoryer (PID 92326, boot epoch 1794986056),
  so a row vanishing from the server list is not a per-client observation.
  Boot 3 pairs the 2007 wire with our client's unchanged pixels; WAVE 1 pairs
  it only with a scripted observer log.
- Boot 1 (observer timeout) and boot 4 (no exe row painted) are failed
  attempts; neither fills a cell. Boot 2's wire half survives; its frame dump
  cannot prove paint.
- A script-path mistake made boot 3 reuse boot 2's output directory; boot 2's
  hashes were saved in `~/scratch/story-room-findings.md` and the surviving
  boot-3 pack was copied to `~/scratch/story-room/live-disband3-final/` with
  `artifact-hashes.txt`. No boot was repeated for that reason.
- The #375 hands fix appears as a Room-user-panel story with a cross-state
  pin; the Room panel itself still has a GAP cell. At this base (b5bd0e6)
  the #379 and #380 fixes are merged (`bd8d490`, `51c8d95`); Lane B's
  CP06 frame and wire-only CP31–CP38 confirm their wire halves, but no exe
  pixel pair was captured, so exe-oracle cells stay GAP.

## 5. #536 re-scope tick (W6 W-b, 2026-11-21)

- [x] As a peer, I want each filter tab click captured from a resident peer, so filtering is symmetric. — RE-SCOPED: 7.55 filters are per-client local views. The qualified peer kept its tab strip and rows unchanged across the master's ALL/NORMAL/LIVE/ALL clicks; no per-click wire frame exists. `docs/captures/journey-audit/290-room-story-matrix.md` FILTER-TABS peer and server cells now carry the `W6FB-PEER-02` hashes; guard `resident_peer_keeps_its_room_filter_and_rows_while_the_master_switches_tabs` drives two scripted clients and asserts a quiet tap. The earlier "symmetric" premise is closed negatively; no cross-client mirroring is claimed.

## 6. Lane B closure batch — #382 (2026-11-19)

The master/peer two-client boot captured CP01–CP06 (`~/scratch/close-wr/
full01/checkpoints/index.txt`; master CP05 chat frame hash
`4899ad2a7c8720cd003507c8c70a2e0d0536278ab5e1c2557eca8e2336696a9c`) and
the bidirectional tap `~/scratch/close-wr/full01/tap.log`. The remaining
lifecycle pins come from a zero-boot scripted-actor leg
(`~/scratch/close-wr/wire-only/{master,peer,peer2,observer}.log`, CP31–CP38):
empty-password `2004`, `3004`/`3003` join, `4009` ready, `3012` chat,
`3007` non-last leave, `3007` promotion on master exit, and `2007` on last
exit. Every boot used a private Xvfb display and owned `rig_null_*` sink;
manifests report `route=sink`, `on_default_sink=0`, `preboot_breach=0`,
server pid 92326 / epoch 1794986056. No qualified exe pack was captured,
so client pixels are `--claim none` and exe-only cells stay GAP.

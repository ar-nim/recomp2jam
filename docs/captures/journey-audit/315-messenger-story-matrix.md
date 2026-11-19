# Messenger story matrix — Spec #315 (WAVE 5, Lane messenger)

Method: `docs/captures/story-matrix/TEMPLATE.md`. Base `f1987b5`; worktree
`/home/arnim/Projects/recomp2jam-wt143`, branch `cb2/story-messenger`. Docs and
local scratch only: no Rust change, no install-data slot, no push, no git remote
config edit. The 7.55 executable is the oracle (`OTwo.exe`, 5,029,888 B,
SHA-256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`);
Memoryer is a wire witness, never a source of exe rules. Captures stay in
git-ignored scratch; only hashes and text enter the repo.

The journey audit (`docs/captures/journey-audit/315-messenger.md`, base
`2105ac6`) graded this spec 0 MATCHED / 4 MISMATCHED / 14 BLOCKED /
2 NA-DESIGN. It is the least-evidenced spec in the program. This matrix keeps
that grade: the only exe Messenger behavior pinned so far is the
server-unavailable sheet that the rig gets instead of the Messenger main
sheet. Memoryer has no messenger endpoint, so a client boot cannot manufacture
the missing main-sheet, friend/note, wire, or frame-state evidence. Those cells
are GAP with a named next step, not passed cells.

## 1. Evidence inventory (before any cell was graded)

| Source | Pin (full sha256 unless noted) | What it names / verdict |
|---|---|---|
| exe identity | `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b` | 7.55 behavior oracle; every static code and offset below |
| `~/scratch/side-scenes/b8-cash` (PASS, cited exe pack) | manifest sha from `citation.txt` `f9d4a6953c55b75b4f04db3c0ece198bd2032361ac65dc8275ce07ccb6aa3f8b` (the `manifest.json` file hashes `7cb327c1c67a9f4c1c9028475a1b605dc27d8b6ef6b5f565fcd3cc2a5975960b` — recorded citation drift, frame hashes agree); `messenger.png` `946e503319b6fa18fcd28b2cf9230b9124b5177cc002567c07afac84631ad917`; `input.log` `1e3911b987f57c2e0c970aeea77b60173162ea82ffa3588ea00c42d77d506b11` | MyRoom MESSENGER press `(644,580)` → one-OK blue information-style sheet over MyRoom; OK `(400,368)` → Item Shop leg proceeds (`itemshop.png` `0c561c6648fe29aaef1b64ca867de05baef9fae7ab12afbcd9c1a42652293ee8`). The sheet says the server is unavailable or not normal. No Messenger main sheet; `wire_tap=0` |
| `~/scratch/side-scenes/b6-shops` (FAILED for the charge target; Messenger checkpoint valid) | manifest sha from `citation.txt` `c8adc9aa01e72258b336f6b470a62bb6373d15a4451405019ec943da2253131b` (file `manifest.json` `1cdd90d8285573b7b4b6998c475e92abe32c2a333de89364016bc2861461623b`); `messenger.png` `946e503319b6fa18fcd28b2cf9230b9124b5177cc002567c07afac84631ad917` (byte-identical to b8); `input.log` `cab098cc8d0aa17553db6f8da7a0fe8ed62548c1df4b7b22cce51e350bf93fbc` | negative/corroborating pack: same press, same sheet, same OK; b6 never reached Cash Charge, so no cash claim. `wire_tap=0` |
| `~/scratch/side-scenes/b1-explore/explore/e04-messenger.png` | `946e503319b6fa18fcd28b2cf9230b9124b5177cc002567c07afac84631ad917` | early unqualified exploration frame; same pixels as b6/b8 but no manifest, so it never carries a pack claim alone |
| `~/scratch/side-scenes/b1-explore/explore/j02-messenger-room.png` | `80bbd0f94bd48f82b986e24e810f05785cc40abb688ef2920acba2f2d744e44a` | **negative:** filename says `messenger-room`, pixels are a Playing scene (keyboard hands, life bar). Not a Room Messenger pin; recorded so it cannot be graded as evidence |
| `~/scratch/hover-sweep/room-btn-messenger-on.png` / `-offA.png`, `docs/hover.md:120` | on `fd9a61c96f0e701db6c91494f409205dc2c77fac2503cee596cbfcf00894920d`, offA `35b4d5d4d4ee908c7b360d1596fe76b3364c5ea8bd59b3a7efd7f5bad1eaf868` | exe Room MESSENGER hover repaints `Btn_Messenger.ojs` body frame 0 on rect `627,571-662,590`; frame + doc pin, no qualified manifest |
| `docs/file-formats/opi.md:275-280` | `Interface1.opi` sha256 `d66541198965d622ff0177b306d916885ae4d64ae07b2c2264fcbdd850d50584`, 75,100,702 B | static OPI census: six `DIALOG_MESSENGER_*` blocks, 120 declarations (114 control `IDC_*` + 6 block self declarations, plus 4 `SET` groups in MAIN) and 114 `.bnd` rects (78/7/8/14/7/6 entries; 77/6/7/13/6/5 rects). **Contradiction settled 2026-11-19 by Lane D:** re-parsed the pinned archive with Python 3.14.9 using the `opi.rs` trailing-table layout; each block's `.bnd` header count equals its exact body size (6 + 16×count), and control lines equal rects in every block (77/6/7/13/6/5). The #315 body's 134 is the adjacent `DAILOG_SELECTMUSIC` declaration figure (bnd 133), not a Messenger count. Use **114 rects / 120 declarations**; mounting stays OPEN |
| `docs/exe-state-inventory.md:67-71` | `CDMessenger*` RTTI file offsets `0x27db0c`-`0x27dbbc`; `CCMessengerHandler` file offset `0x280690`; host codes Room `0x13200048`, Shop `0x142000B8`, MyRoom `0x1520006D`, Waiting `0x1620006A`, MShop `0x1A200088` | static dossier pins; no live handler consumer and no opcode body are pinned |
| `docs/bnd-consumption.md:70-75,132-140`; ADR-0012 | all six Messenger `.bnd` files are bucket E (no live consumer pinned) | mount-or-dead census remains open; art alone never proves a mounted sheet |
| `docs/parity/myroom.md` Input row; `crates/o2jam-shell/src/screens/side/myroom.rs:160-164,668-674`; `crates/o2jam-shell/src/screens/game/mod.rs:247,9621,12594` | P0 labels and guard `myroom_p0_controls_answer_nothing_and_queue_no_wire` (live chain `main.rs::run_play → OnlineSession::ingest_snapshot → ScreenGame::click_myroom_side → myroom_action_at`) | our-client negative invariant only; never exe proof. Room, Waiting, Shop and MShop buttons also lack dispatch |
| `docs/captures/journey-audit/315-messenger.md`; issues #345 (CLOSED), #250 (OPEN), #245/#249 (CLOSED), #301 (OPEN) | journey verdicts at `2105ac6`; tracker read 2026-11-20 | #345 is the liveness/error-sheet class; #245/#249/#250 are capture-tooling classes; #301 owns audio cue decisions |

Green guards are recorded only in fix cells; they never grade exe behavior.
A server state shared by both clients is never a per-client observation.

### Next-evidence steps named by cells

- `N-HOST` — qualified exe pack **and** socket tap for each unpinned host press
  (Room `0x13200048`, Shop `0x142000B8`, Waiting `0x1620006A`, MShop
  `0x1A200088`); dismiss the NORMAL-mode popup at every transition; name the
  click, frame hash and input log per host.
- `N-BACKEND` — supply a messenger-capable interop fixture or proxy reply, then
  capture a qualified exe pack with alarm, main sheet, friend/note and
  PopUpMenu flows; Memoryer's current endpoint set has no messenger service.
- `N-MOUNT` — static mount census: xref every `BOUND` Messenger block in
  `ControlList_Interface.txt` / `Interface1.opi` to a live exe consumer; record
  mounted vs dead with dossier id; a dead verdict needs no pixels.
- `N-WIRE` — tap the exe socket across one Messenger leg; log every opcode and
  body, including the negative case (no frame on a host press or OK); pair the
  tap id with the frame hash under one trace id.
- `N-FIELDS` — rewrite `SENDUSER` / `SENDTIME` / `CHECKMESSAGE` in one reply with
  a scratch TCP proxy (recipe `~/scratch/digits-rig/proxy.py`), then capture the
  resulting note row on the exe.
- `N-FRAMES` — qualified exe interaction pack with the main sheet actually
  mounted: rest/hover/press frames, both scroll-clamp ends, and parent-relative
  offsets against the host rect; name each frame hash.
- `N-MODAL` — press a host button, deliberately click through the sheet before
  OK, then dismiss with OK; frame pair plus input log proves block vs non-block.
- `N-MASTER` — one qualified master-role exe boot (after the #379 handoff)
  repeating the MyRoom press and OK; no server-state claim from a shared socket.
- `N-AUDIO` — scoped before/after audio capture on an owned null sink with a
  route snapshot, no default-sink stream, to test host BGM coexistence.
- `N-P0` — for each P0 action obtain an exe send pin; only then replace the
  no-op and its negative guard.

## 2. Matrix (28 actions × 5 observer actors = 140 cells)

Cell key: `EVIDENCED`, `GAP`, `LIVE-FILLED`, `OPEN-FIX`, `NA-DESIGN`. Every GAP
names its next evidence step above. `self:` = the column actor raises the
stimulus; `other:` = another actor does. The `peer` column is also used for a
single authenticated seat holder with no room master, following the #180
MyRoom lane; that is a seat-role convention, not a claim that the exe pack was
offline. The `solo` column covers offline/window seams only. Messenger sheets
are private local UI, so channel cells are NA-DESIGN unless a wire frame is
named.

| Action (raised by) | solo | master | peer | channel | server |
|---|---|---|---|---|---|
| **ENV-SERVER-UNAVAILABLE** (self:server, no messenger endpoint) | NA-DESIGN — offline boot has no gateway messenger endpoint; no exe claim | GAP — no master-role error-sheet frame; OPEN — `N-MASTER` | EVIDENCED — `<MyRoom MESSENGER press (644,580)>` → a one-OK blue information-style sheet says no usable server; the MyRoom board stays behind it → b8 pack `f9d4a695…f8b` / frame `946e5033…d917`, b6 corroborates `c8adc9aa…131b`; #345 closed liveness class, not the Messenger main sheet | NA-DESIGN — private side screen; no channel-visible row state | GAP — no tap to name a request or reply around this press; OPEN — `N-WIRE` |
| **HOST-PRESS-ROOM** (self:master) | NA-DESIGN — offline Room has no gateway host button | GAP — `<Room MESSENGER press at code `0x13200048`> → no qualified exe frame; the `j02-messenger-room.png` candidate is a Playing scene and is not evidence; OPEN — `N-HOST` | NA-DESIGN — the master's private sheet is not visible to another seat | NA-DESIGN — no channel board state for a private sheet | GAP — no opcode tied to this press; OPEN — `N-WIRE` |
| **HOST-PRESS-SHOP** (self:peer) | NA-DESIGN — offline shop has no gateway host button | GAP — no master-session Shop press frame; OPEN — `N-HOST` | GAP — `<Shop MESSENGER press at code `0x142000B8`> → no qualified exe press or sheet frame; OPEN — `N-HOST` | NA-DESIGN — private sheet | GAP — no send or reply pinned; OPEN — `N-WIRE` |
| **HOST-PRESS-MYROOM** (self:peer) | NA-DESIGN — offline MyRoom seam, no gateway session | GAP — no master-role MyRoom press frame; OPEN — `N-MASTER` | EVIDENCED — `<click (644,580) on MESSENGER>` → blue one-OK server-unavailable sheet over the Inventory face, not a main sheet → B8 `f9d4a695…f8b`, frame `946e5033…d917`, input log `1e3911b9…b11` | NA-DESIGN — private side screen | GAP — `wire_tap=0` in both packs; OPEN — `N-WIRE` |
| **HOST-PRESS-WAITING** (self:peer) | NA-DESIGN — no offline Waiting gateway session | GAP — no master-session Waiting press frame; OPEN — `N-HOST` | GAP — `<Waiting MESSENGER press at code `0x1620006A`> → no qualified exe frame; OPEN — `N-HOST` | NA-DESIGN — private sheet | GAP — no opcode tied to this press; OPEN — `N-WIRE` |
| **HOST-PRESS-MSHOP** (self:peer) | NA-DESIGN — no offline Music Shop gateway session | GAP — no master-session MShop press frame; OPEN — `N-HOST` | GAP — `<Music Shop MESSENGER press at code `0x1A200088`> → no qualified exe frame; OPEN — `N-HOST` | NA-DESIGN — private sheet | GAP — no opcode tied to this press; OPEN — `N-WIRE` |
| **HOST-BUTTON-HOVER** (self:peer in Room) | NA-DESIGN — no offline exe hover claim | NA-DESIGN — no master-role hover frame | EVIDENCED — `<pointer onto Room MESSENGER rect 627,571-662,590>` → `Btn_Messenger.ojs` repaints body frame 0; no frame-index swap → `docs/hover.md:120`, frame `fd9a61c9…20d` vs `35b4d5d4…868` | NA-DESIGN — local pointer state only | NA-DESIGN — hover sends no frame |
| **ERROR-SHEET-OK** (self:peer) | NA-DESIGN — no offline sheet | GAP — no master-role OK frame; OPEN — `N-MASTER` | EVIDENCED — `<click (400,368) on OK>` → sheet clears and the next MyRoom→Item Shop click lands (`itemshop.png` `0c561c66…ee8`); b6 repeats the same OK → B8 pack `f9d4a695…f8b`, input log `1e3911b9…b11` | NA-DESIGN — private sheet | GAP — no reply to OK pinned; `wire_tap=0`; OPEN — `N-WIRE` |
| **ERROR-SHEET-BLOCKS-HOST** (self:peer, sheet up) | NA-DESIGN — no offline sheet | GAP — no master-role modal gating frame; OPEN — `N-MASTER` | OPEN-FIX — `<host click while the one-OK sheet is up>` → the exe sheet is pinned but a click-through attempt is not; our client opens no sheet at all, so it cannot block. Fix stays parked on closed #345 liveness; next evidence step `N-MODAL` before any story check | NA-DESIGN — private input state | NA-DESIGN — no wire frame for a modal gate |
| **ALARM-NEW-MESSAGE** (self:server) | NA-DESIGN — no offline message source | GAP — no master-role alarm frame; OPEN — `N-BACKEND` | GAP — `<new-message event>` → `MESSENGER_ALARM` / `ALRAM1` paint is unpinned; our P0 paints nothing new (`myroom.rs:162-164`); OPEN — `N-BACKEND` | NA-DESIGN — no board representation | GAP — no new-message opcode or body pinned; OPEN — `N-WIRE`, then `N-BACKEND` |
| **MAIN-SHEET-MOUNT** (self:peer) | NA-DESIGN — no offline sheet | GAP — no master-role main-sheet frame; OPEN — `N-BACKEND` | GAP — `<host press>` → no Messenger main sheet in B6/B8 or any local pack; bucket E lists all six sheets as unmounted; OPEN — `N-BACKEND` plus `N-MOUNT` | NA-DESIGN — private sheet | NA-DESIGN — a local mount is not a server frame |
| **MAIN-SHEET-77-RECTS** (state-path) | NA-DESIGN — no offline sheet | GAP — no mounted master sheet to measure; OPEN — `N-FRAMES` | GAP — static census pins `Dialog_Messenger_Main.bnd` at 77 rects / 78 declarations, but no friend, note or four chat tabs are observed in a mounted sheet; OPEN — `N-FRAMES` | NA-DESIGN — private geometry | NA-DESIGN — geometry is not wire state |
| **FRIEND-NOTE-SCROLL-CLAMP** (self:peer with a long list) | NA-DESIGN — no offline list | GAP — no master-session list frame; OPEN — `N-FRAMES` | GAP — `<scroll to both ends>` → no exe clamp observation; channel scroll guards are not a Messenger pin; OPEN — `N-FRAMES` | NA-DESIGN — private list | NA-DESIGN — no scroll opcode pinned |
| **FRIENDADD-SEARCH** (self:peer) | NA-DESIGN — no offline search | GAP — no master-session search frame; OPEN — `N-BACKEND` | GAP — `<nickname search + OK>` → six parsed `FriendADD.bnd` rects are static only; no result row frame; OPEN — `N-BACKEND` | NA-DESIGN — private result list | GAP — search request and reply body unpinned; OPEN — `N-WIRE` |
| **NOTE-COMPOSE-SEARCH** (self:peer) | NA-DESIGN — no offline note flow | GAP — no master-session note frame; OPEN — `N-BACKEND` | GAP — `<compose/search press>` → 13 `Note.bnd` rects are static only; no exe compose or search frame; OPEN — `N-BACKEND` | NA-DESIGN — private note sheet | GAP — no note opcode pinned; OPEN — `N-WIRE` |
| **NOTE-SEND** (self:peer) | NA-DESIGN — no offline send | GAP — no master-session send frame; OPEN — `N-WIRE` | GAP — `<SEND press>` → no exe send frame, so our labeled no-op stays; OPEN — `N-WIRE`, then `N-P0` | NA-DESIGN — private sheet | GAP — no send opcode or body pinned; OPEN — `N-WIRE` |
| **NOTE-REPLY-SAVE** (self:peer) | NA-DESIGN — no offline note record | GAP — no master-session reply/save frame; OPEN — `N-BACKEND` | GAP — `<reply or save press>` → no exe row change pinned; OPEN — `N-FIELDS` for the row fields, then `N-BACKEND` for the frames | NA-DESIGN — private note row | GAP — `SENDUSER` / `SENDTIME` / `CHECKMESSAGE` fields unpinned; OPEN — `N-FIELDS` |
| **POPUPMENU-ACTION** (self:peer) | NA-DESIGN — no offline popup | GAP — no master-session popup frame; OPEN — `N-BACKEND` | GAP — `<one of five PopUpMenu actions>` → six rects static only; no exe action frame or dead verdict; OPEN — `N-BACKEND` | NA-DESIGN — private popup | GAP — any action send is unpinned; OPEN — `N-WIRE`, or record dead per `N-MOUNT` |
| **QUESTION-INFORMATION-SHEETS** (self:server) | NA-DESIGN — no offline message | GAP — no master-role sheet frame; OPEN — `N-BACKEND` | GAP — `<messenger QUESTION/INFORMATION event>` → the b8 one-OK sheet is an information-style liveness notice, not a Messenger sheet; no Messenger sheet frame exists; OPEN — `N-BACKEND` | NA-DESIGN — private sheet | GAP — no initiating opcode pinned; OPEN — `N-WIRE` |
| **WIRE-OPCODE-INVENTORY** (self:any) | NA-DESIGN — no offline opcode claim | GAP — no master-session tap; OPEN — `N-WIRE` | GAP — `<whole Messenger leg>` → both packs declare `wire_tap=0`; no `CCMessengerHandler` opcode inventory exists; OPEN — `N-WIRE` | NA-DESIGN — no board frame from a wire inventory | GAP — no opcode/body table pinned; OPEN — `N-WIRE` |
| **P0-PREWARD** (state-path) | NA-DESIGN — no offline P0 gateway action | GAP — no master-side exe send pin to replace a P0; OPEN — `N-P0` | OPEN-FIX — `<any P0 Messenger control>` → our guard proves no wire and no paint; exe answers the MyRoom host press with a sheet instead. Replacement is parked on closed #345 liveness plus `N-P0`; no verdict box is checked by a green guard | NA-DESIGN — no observer role in a client no-op | NA-DESIGN — no invented wire shape |
| **PARENT-OFFSET-FRAMES** (state-path) | NA-DESIGN — no offline sheet | GAP — no master-sheet offset frame; OPEN — `N-FRAMES` | GAP — the liveness sheet sits at the Information-style origin, but that cannot be transferred to six unpinned sheets; rest/hover/active frames for Messenger are unpinned; OPEN — `N-FRAMES` | NA-DESIGN — private geometry | NA-DESIGN — offsets are client-local |
| **BUCKET-E-MOUNT-OR-DEAD** (state-path) | NA-DESIGN — no offline mount | GAP — no master-role mount census; OPEN — `N-MOUNT` | GAP — `<six bucket-E .bnd files>` → no live consumer pinned and no dead verdict recorded; ADR-0012 requires an evidence-backed verdict either way; OPEN — `N-MOUNT` | NA-DESIGN — mount census is client-local | NA-DESIGN — no wire behavior in a mount verdict |
| **LIVE-CALLER-CHAIN** (state-path) | NA-DESIGN — offline boot has no gateway chain | GAP — master-role chain to a sheet is unpinned; OPEN — `N-MASTER` | OPEN-FIX — `<boot → Planet → Channel → Room → MyRoom → MESSENGER>` → our chain ends in the P0 no-op while the exe chain ends in the liveness sheet; the four other host arms have no exe leg. #345 liveness fix parked; remaining arms need `N-HOST` | NA-DESIGN — caller chain is local | NA-DESIGN — no wire claim without a tap |
| **DIALOG-OPEN-CUE** (state-path) | NA-DESIGN — offline has no cue to pin | NA-DESIGN — no master cue pin | NA-DESIGN — no exe cue exists; owner rule is no invented audio, tracked under #301 | NA-DESIGN — private audio | NA-DESIGN — no wire cue pinned |
| **NEGATIVE-GUARD-SEAM** (test seam) | NA-DESIGN — test-only invariant, no offline behavior | NA-DESIGN — test-only invariant | NA-DESIGN — `myroom_p0_controls_answer_nothing_and_queue_no_wire` and `payment_entry_queues_no_wire_call` pin no invented wire; the seam suffices because no wire shape is claimed | NA-DESIGN — test-only invariant | NA-DESIGN — test-only invariant; no frame claim |
| **HOST-BGM-COEXIST** (data-effect) | NA-DESIGN — no offline BGM claim | GAP — no master-role audio A/B; OPEN — `N-AUDIO` | GAP — B6/B8 have audio artifacts but no before/after Messenger pair; route snapshot alone does not pin coexistence; OPEN — `N-AUDIO` (#301 owns cue decisions) | NA-DESIGN — private audio | NA-DESIGN — no wire cue pinned |
| **ENV-SERVER-UNAVAILABLE-FIX** (closed #345 liveness) | NA-DESIGN — no offline endpoint | GAP — no master-role liveness-fix observation; OPEN — `N-MASTER` | OPEN-FIX — `<MyRoom MESSENGER press when the backend is absent>` → exe paints the one-OK notice and our client paints nothing; mapped to closed #345, not a new bug. A messenger-capable fixture is needed before this cell can be checked; OPEN — `N-BACKEND` | NA-DESIGN — private notice | GAP — no wire proof that the backend is absent; OPEN — `N-WIRE` |

Counts: **140 cells** — 4 EVIDENCED, 58 GAP, 0 LIVE-FILLED, 4 OPEN-FIX,
74 NA-DESIGN. Statuses were re-parsed one token per intersection from the
table above; every GAP names its next evidence step (`N-*`).

## 3. Scenario scripts (planned, not executed)

Template §5 requires a stable script shape. No boot was launched, so these are
**planned** scripts only; they fill no cell and carry no frame hash.

| Field | Planned value |
|---|---|
| `scenario_id` | `S315-MESSENGER-HOSTS-01`, `S315-MESSENGER-FLUX-02` |
| `spec` / `state` | `#315` / `STATE_MYROOM` side visit plus Room, Shop, Waiting and Music Shop host seams |
| `trace_id` | `S315-<boot>-<actor>` assigned at launch; none minted in this lane |
| `boot_pair` | `M = master host user`, `P = single seat holder`, `O = channel-observer socket`; only one lane on 15010 at a time |
| `containment` | private Xvfb, owned null sink, `route=sink`, `on_default_sink=0`, `preboot_breach=0`, `--no-sound`, recorded server pid/epoch |
| `steps` | 01: boot → dismiss NORMAL popup at every transition → Room → MyRoom → MESSENGER `(644,580)` → OK `(400,368)` → Item Shop; 02: after a messenger-capable fixture, alarm → main sheet → friend/note scroll → FRIENDADD → note send/reply → PopUpMenu → OK |
| `checkpoint` | per step: per-actor frame hash, bidirectional tap id/body, input log line, popup/focus deviation |
| `outcome` | exe-observed sentence only; if no frame, the cell stays `OPEN — <N-*>` |

Script 01 is blocked by the shared lane and, more fundamentally, cannot reach
the main sheet without a backend. Script 02 is blocked by `N-BACKEND`. Both are
recorded so the next lane does not rediscover the same limit.

## 4. Boot ledger (four-boot cap, one shared lane)

| Boot | Display / sink | Actors | Scene | Wire tap | Result |
|---|---|---|---|---|---|
| — | none | none | none | none | **0 of 4 boots spent.** Preflight `ss -tnp | grep 15010` found two ESTAB clients from other lanes (`python3` pids 3657329 and 3661476) to Memoryer (`dotnet` pid 92326). Per the one-lane rule no client was launched, and no server was restarted or signalled |

Rationale for spending no boot: both existing exe packs already pin the only
Messenger-visible response the rig can produce, with byte-identical frames. A
new boot would re-capture the server-unavailable sheet; it could not fill the
main-sheet, scroll, friend/note, opcode, frame-state, or bucket-E cells, all of
which need a backend or a static consumer census. Failed attempts in this lane:
none (no launch). No capture was promoted to a story from an unqualified frame.

## 5. Count re-parse and story hand-off

Cell statuses were re-parsed from the table above (one status token per
intersection): 4 EVIDENCED, 58 GAP, 0 LIVE-FILLED, 4 OPEN-FIX, 74 NA-DESIGN
over 28 action rows × 5 actors = 140 cells. The matching tracker edit and its
read-back are recorded after the body update.

The new stories in issue #315 are grouped under `## Multi-actor stories
(WAVE 5, #381 story matrix)`. Evidence-backed cells use the pack/frame and
hover pins above; gap cells keep the `N-*` next step in their `verify:` clause.
A checked box still needs the live caller chain and the project gates; a matrix
cell is a story candidate, not a closure.

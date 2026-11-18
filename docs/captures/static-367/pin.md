# #367 static pin — Room-state "no response" modal

Static/offline leg A. No boot, no server touch, no tracker write, no product code change.
Worktree `/home/arnim/Projects/recomp2jam-wt99`, branch `cb2/static-367`, base `373b670`.
Incremental scratch ledger: `/home/arnim/scratch/static-367-findings.md`.

## Evidence classes and target pin

| Class | ID | What it is |
|---|---|---|
| EXE | `S367-EXE` | `~/scratch/run-full/OTwo.exe`, 7.55, 5,029,888 B, SHA-256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`, 32-bit PE. All addresses below are **VA (file offset = VA − 0x400000)**. No original bytes are reproduced. |
| SHELL | `S367-1` | GNU objdump 2.47 + Python 3 byte scans over the read-only EXE, 2026-11-18. The emitter and dialog chain. |
| SHELL | `S367-2` | Same tools; UI pump, sheet staging, OK handlers. |
| SHELL | `S367-3` | Same tools; tracked-send helper, poll send sites, relay path. |
| PRIOR | `TT-323-B1` | Qualified Wine trace (`~/scratch/no-response/fuse-arm-findings.md`, batch 1): 35.785 s relay connect block from `03EC` to the `07DA`/`07E8` poll second. This is a **qualified live observation**, not a new boot in this leg. |
| PRIOR | `TT-323-B2` | Shell static findings (`fuse-arm-findings.md`, batch 2): generic string emitter, relay-string emitters, tracked-send rule, `07E8` elimination. |
| REPO | `S367-REPO` | `docs/dialog-strings.md:22-23`, `docs/bnd-semantics.md:41`, `docs/parity/channel.md:109-113`, `docs/file-formats/opi.md:255`: `Dialog_Information.bnd` geometry and control codes; `docs/captures/t320/findings.md:121`, `docs/wine-boot.md:950`: live FFF0 observations. |

REA CLI v6.8.0 could not open a native session: installed Ghidra is 12.2, the CLI accepts only
12.1.x (`provider_unavailable`/`unsupported_version`); Hopper is absent and IDA/Kuna are not
configured. No configuration was changed. Therefore **no new REA dossier/Evidence ID exists for
this leg**. Every `S367-*` ID above is a shell observation, not a REA dossier. Prior REA IDs
quoted in other docs (for example `ev_3a869bb7…` for ordinal rect pairing) are inherited, not
re-generated here.

## Verdict

| Task | Status | Answer |
|---|---|---|
| 1. Arming request / node creation | **SUPERSEDED BY LIVE READ** | Static scan missed the 07DA tracked-send site. Stopped-state winedbg (live-367b, issue comment 5) found the sole pending node expecting `07DB`; the exe arms on its own `07DA` GetUserList poll. See §3 correction below. |
| 2. Paint resource + OK | **MET (static)** | One-OK `Dialog_Information` sheet, TEXT code `0x21400003`, OK code `0x21200002`; OK closes the top dialog via queue type 100. Static OK path sends no packet. |
| 3. +37–44 s vs 20 s watchdog | **PARTIAL** | 20,000 ms is a wall-clock threshold; ~35.785 s is the OS relay connect, not an exe timer literal. The modal needs an armed node that this leg could not name. |

## 1. Modal emitter to painted sheet

Inherited from `TT-323-B2`: the generic CP949 string `서버로 부터 응답이 없습니다.` sits at file
`0x24d02c`; the only code reference is the push at VA `0x558f8b` (file `0x158f8b`) inside
watchdog method VA `0x558b10` (file `0x158b10`). The relay-specific string at file `0x2471e8`
has two different handlers and is dead in 7.55.

`S367-1`: at VA `0x558f89`–`0x558fa1` (file `0x158f89`–`0x158fa1`) the emitter pushes four
arguments, `(1, 0xffd, VA 0x64d02c, 0)`, and calls message helper VA `0x478c90` with `ecx` =
global `[0x762b44]`. That helper (VA `0x478c90`–`0x478d44`, file `0x78c90`–`0x78d44`)
zero-fills a 256-byte text buffer, copies argument 3 when present, packs
`(arg1 << 16) | (arg2 & 0xfff)` into a control word (`0x100ffd` here), and appends a
`0x110`-byte record to the queue at manager `+0x168` through VA `0x47edf0`. Only three places
in the whole image push `0xffd` (files `0x7479c`, `0x8e346`, `0x158f91`): the generic fuse and
the two relay-specific notices. All three therefore use the same one-OK queue route.

`S367-2`: the pump at VA `0x476330` (file `0x76330`) walks that queue. At VA `0x4766c6` it
extracts `code >> 16`, subtracts 1, bounds-checks `<= 0x63`, and dispatches through the
byte-indexed table at VA `0x4778b4`/`0x477928` (files `0x778b4`/`0x77928`). Type 1 selects
handler VA `0x476de3` (file `0x76de3`), which allocates a `0x188`-byte dialog object via ctor
VA `0x433ba0` (file `0x33ba0`, vtables `0x64499c`/`0x644988`) and calls virtual slot `+0x68`
with the `0x110`-byte record.

The matching show method VA `0x433c90` (file `0x33c90`, `ret 0x110`) stores the record at
`+0x10`; at VA `0x433d23` (file `0x33d23`) it resolves control `0x21400003` and writes the
queued text through VA `0x50f520`. Setup method VA `0x434080` (file `0x34080`) stages
`0x21100001` (back) and `0x21200002` (OK) and resolves the OK rect through VA `0x548640`,
the same ordinal getter pinned by `ev_3a869bb7…`. `0x21400003` and `0x21200002` are the TEXT
and OK controls of the `DIALOG_INFORMATION` block (`S367-REPO`):

- sheet `Dialog_Information.bnd` rect 0 `(241,206)`, 318×187;
- OK sprite `Info_OK.ojs` anchor `(102,150)`, 117×26, i.e. click centre near `(400,365)`;
- OK control code `0x21200002`, message TEXT code `0x21400003`.

**Implementation pin:** the fuse is not a bespoke window. It is the same one-OK Information
sheet the client already stages for relay-down; only the string and owner state differ.

## 2. OK behavior

`S367-2`:

- Virtual slot `+0x28` of this dialog class, VA `0x433f50` (file `0x33f50`), compares the
  incoming control word with `0x21200002`; on match it enqueues `(100, 0, NULL, 0)` at VA
  `0x433f78` (file `0x33f78`). Pump handler 100, VA `0x47748f` (file `0x7748f`), pops the top
  dialog from manager `+0xf70`, destroys it, and clears `+0x178`. **The OK handler itself
  sends no packet.**
- Enter `0x0d`, Esc `0x1b`, and Space `0x20` all call that same OK handler (VA `0x434020`,
  file `0x34020`). Mouse release/press states `8`/`9` on control `0x21200002` do the same
  (VA `0x433f90`, file `0x33f90`).
- A separate elapsed check on dialog fields `+0x170`/`+0x174` also calls the OK handler
  (VA `0x433ec0`, file `0x33ec0`); the threshold encoded there is OPEN.

FFF0 contradiction, kept open: the only writer of opcode `0xfff0` in the image is VA `0x5a6f93`
(file `0x1a6f93`), inside leave helper VA `0x5a6ed0` (file `0x1a6ed0`). Its direct callers are
VAs `0x474f10`, `0x474f65`, `0x4752a8`, `0x48c4a8`, `0x5a623d`; none is the Information OK
handler. Live records (`S367-REPO`) saw FFF0 on modal dismissal by Return/Escape/XTEST-OK,
so the wire fact stands, but static bytes do not attribute it to a mouse click on OK. Next
evidence step: one qualified input trace distinguishing click from Return. A fix agent should
not wire "OK ⇒ FFF0" from this pin alone.

## 3. Arming request — what is pinned and what is not

`S367-3`: all nodes are created by tracked-send helper VA `0x556270` (file `0x156270`).

- `(packet, 1, 0)`: expected reply id = packet id + 1, kind word `0`, current tick; the node
  is inserted (VA `0x5565c5`) before the send through virtual slot `+0x24`
  (VA `0x5565ca`–`0x5565e3`). Node fields are `{+0 expected id, +2 kind, +4 stamp}`.
- `(packet, 0, 0)`: no node (branch at file `0x15641b`).
- third argument nonzero: duplicate-send path checks/refreshes an existing node
  (files `0x15628d`–`0x156416`).

Room-list poll requests: **correction applied when #367's fix landed.**

- `07E8` (2024): opcode staged at VA `0x4938a1` (file `0x938a1`); the send at VA `0x4939ce`
  (file `0x939ce`) passes `(packet, 0, 0)`. This refutes the #321/"wait for `07E8` reply"
  hypothesis, matching `TT-323-B2`. Still stands.
- `07DA` (2010): the static read at VA `0x490cfc`–`0x490d14` (file `0x90cfc`–`0x90d14`) saw a
  direct vtable `+0x5c` send and recorded "no helper call; untracked". **That S367-3 note is
  contradicted by the live read and is corrected here.** The stopped-state winedbg read in
  live-367b (issue comment 5; debugger log `c5c4a95f…`, wire `5ba48090…`, manifest
  `4083f3ee…`) found the tracked-send queue at `[0x76835c+0x4348]` holding exactly one node,
  `node+0 = 0x000007DB` — the expected reply to `07DA` — when the modal was up. The exe is the
  behavior oracle; the live read wins. A tracked-send site for 07DA exists even though this
  static scan did not find it. The contradiction is kept visible, not silently resolved.

Tracked room-entry sends and the replies observed in `TT-323-B1`'s wire log:

| Request | Node creation | Reply seen |
|---|---|---|
| `03E8` (1000) | generic request factory | `03E9` (1001) |
| `03EA` (1002) | VA `0x5591db` / file `0x1591db` | `03EB` (1003) |
| `03EC` (1004) | VA `0x474de0` / file `0x74de0` | `03ED` (1005) |
| `0FBE` (4030) | VA `0x55a222` / file `0x15a222` | `0FBF` (4031) |
| `07D0` (2000) | VA `0x55985f` / file `0x15985f`; VA `0x55a52e` / file `0x15a52e` | `07D1` (2001) |
| `13A4` (5028) | direct send, no node | `13A5` (5029) |

Every tracked node from that burst is matched in the same second. Therefore **no static
send site in the idle room-list path arms a node that can survive to +37 s**. The arming
request named in #323's OPEN item remains OPEN. Next evidence step (requires a boot, not this
leg): with the modal up, stop the exe and read the queue at `state+0x4348` — node `+0` (expected
reply id), `+2` (kind), `+4` (stamp) — against clock global `0x768120+0x3c`. That directly
names the owner request. Candidate send families to compare are the tracked helpers at VAs
`0x559020`/`0x5593b0`/`0x55a110`/`0x55a350`/`0x55c660` and the P2P-leave helper at
VA `0x5a6ed0` (which tracks opcode `0xfff0`, never answered).

## 4. Timing source: 20 s watchdog vs +37–44 s modal

`S367-1`/`S367-3`:

- Watchdog method VA `0x558b10` has three layers: a 15,000 ms keepalive on `+0x4338`
  that sends opcode `6001` **untracked** (VA `0x558cc5`, send args `(packet,0,0)` at
  VA `0x558d13`); a 15,000 ms "network bad state" notice on `+0x4334`; and a 5,000 ms
  pass over the pending-node queue on `+0x4330`.
- The overdue test pushes literal `20,000` at VA `0x558f6b` (file `0x158f6b`) and calls
  elapsed helper VA `0x411ea0` (file `0x11ea0`). That helper computes
  `clock[+0x3c] - stored_stamp`, returns true only when the difference is **strictly greater**
  than the threshold, and resets the stamp to the current clock when the last argument is 1.
  The clock field is filled from a `timeGetTime`-class import at VA `0x409907`–`0x409910`
  (file `0x9907`–`0x9910`). So the 20 s gate is wall-clock milliseconds, not a frame count.
  One stale node produces one modal per 20 s after its stamp; the node is then erased through
  VA `0x5569e0`.
- The relay path contains no 35–44 s literal. IP string `203.238.136.52` is staged at VA
  `0x5936e1` (file `0x1936e1`), port `7788` at VA `0x59370b` (file `0x19370b`), and the only
  explicit wait is 500 ms (`0x1f4`) at VA `0x593777` (file `0x193777`). On failure it enqueues
  a *different* Information message, `메신져 서버에 접속할 수 없습니다.` (VA `0x64e378`, file
  `0x24e378`) at VA `0x5939fc` (file `0x1939fc`) — not the fuse string.
- `TT-323-B1` measured the OS connect itself blocking 35.785 s from `03EC` to the
  `07DA`/`07E8` second, with the game socket still polled meanwhile. So the **+36 s floor is
  explained by the connect, not by a longer exe fuse**: when the connect returns, a stale
  node (if one exists) is immediately >20 s overdue and the modal is painted on that pass.
  The upper end of the observed +37–44 s window is scheduling/click timing, not another
  constant in the image.

This confirms the timing half of the old hypothesis with offsets but **cannot close it**:
without the arming node's identity, a fix agent cannot know which request to track or which
reply should reset the fuse. Keep the arming item OPEN per the next-evidence-step above.

## Implementation guidance (for the fix leg)

1. Own the fuse in Room state, not globally: live evidence places it only in idle room-list
   entries (`docs/wine-boot.md:913-926`).
2. Stage `Dialog_Information.bnd` at the parsed rect; paint the CP949 string at file
   `0x24d02c` in TEXT code `0x21400003`; use OK code `0x21200002` at the pinned OK rect.
3. Accept mouse, Enter, Esc, and Space for dismissal. Dismissal closes the top dialog; no
   wire frame is statically attributable to OK alone.
4. Gate on the exe's wall-clock behavior (20 s overdue node, 5 s queue pass, 15 s keepalive),
   but do **not** invent a 37 s timer: the observed delay comes from the blocking relay
   connect. The arming node is now pinned to `07DA`/`07DB` by the live read above; use that
   pair, not an invented timeout. Our client has no 36 s relay block, so the fix must choose
   and document an observable equivalent arming condition (see #367 fix findings).
5. Close discipline: a seam-level dialog test is not a player-visible fix. The live caller
   chain must reach `boot → Room state → fuse armed → Information sheet → OK`, with guards
   named for that chain, before #367 can close.

## Coverage and limitations

- Answered: emitter uniqueness, sheet resource, control codes, OK paths, tracked-node layout,
  poll send exclusion, wall-clock threshold, relay path constants.
- Unanswered: which request arms the surviving node; FFF0's exact trigger on modal dismissal;
  dialog field `+0x170` elapsed threshold.
- Search boundary: whole-image immediate scans plus targeted function slices over the pinned
  EXE; no dynamic memory read, no Ghidra/REA dossier, no live capture in this leg.
- No tracked files were modified outside this document; no commit or push was made.

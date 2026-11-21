# Back navigation in 7.55 and in our client (#735)

Scope: version 7.55, `OTwo.exe` SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`,
Ghidra 12.2 via REA target `target-83a045418b62.exe`. All behavior below is
described in our own words from inspected function bodies and file/VA
offsets; no original source or disassembly is reproduced.

## 1. Keyboard consumer chain (static)

1. `FUN_005441b0` at VA `0x5441b0` (file `0x1541b0`) is the device-state
   reader. It asks the DirectInput device object through its vtable poll
   method for 256 key bytes into the object's key-state field, then walks
   37 entries. Each index maps through the object's 37-byte translation
   table to a current-state byte and an edge byte; the helper at
   `0x544a40` turns a high-byte change into press (1) or release (2).
   Evidence: `ev_3713644753…` (body extent), `ev_d50de679…` (behavior),
   `ev_91879ff5…` (helper).

2. `FUN_00544ca0` at VA `0x544ca0` (file `0x1544ca0`) is the buffered
   consumer thread. It waits on a device event and a stop event, reads up
   to 32 buffered records, and for armed modes forwards each record as an
   internal key index plus phase `1` (down) or `2` (up) and a timestamp.
   Direct calls go through `0x542a20` / `0x5429b0` to the queue writer;
   eight computed call sites remain unresolved in static analysis and are
   named OPEN, not guessed. Evidence: `ev_88398e1a…` (extent),
   `ev_5d75f279…` (behavior and call-site limits), `ev_91879ff5…`.

3. `FUN_00545910` at VA `0x545910` writes one six-word record (key index,
   phase, timestamp and three spare words) into the UI event queue and
   advances the ring. Evidence: `ev_a4f50db7…`. The queue is drained by
   `FUN_004341c0` inside scene and control handlers (claim A46,
   `docs/mouse-tracking.md`). The nine resolved call sites of the queue
   writer are listed in `ev_5b949f39…`.

## 2. What the per-scene handlers actually key on

A full pseudocode export of VAs `0x400000`–`0x5c0000`
(`ev_8d11800b…`, `ev_b438467f…`) was searched for comparisons against
internal key code `0x1b` (Escape) and for queue-reading handlers. Findings:

| Surface | Handler (VA / file) | Codes that drive behavior | ESC route |
|---|---|---|---|
| Avi splash | `0x4f3e70` / `0xf3e70`, twin `0x4f51f0` / `0xf51f0` | CR `0x0d`, ESC `0x1b` | **PINNED**: dismiss to bare Planet board; qualified rig in `docs/parity/avi.md` |
| Waiting | `0x4b9930` / `0xb9930` (installed through wrappers `0x4ac590`, `0x4b2db0`; vtable slots `0x6496ec`, `0x6499cc`) | `0x1b` and `0x1c` on press and release | Latch only: writes own-state `+0x29c` / `+0x2a0`; **no transition or send**. Evidence `ev_5c5e962f…`, `ev_9ed98550…`, `ev_1c25e5ea…` |
| Shared scene handler | `0x4e15f0` / `0x4e15f0` (four state vtables: `0x64ab2c`, `0x64adf4`, `0x64b3b4`, `0x64b4b4`) | press: `8` Backspace, `9` Tab, `0xf`, `0x10`, `0x13`, `0x14`; release: `0x26`; printable keys feed text | **No `0x1b` case**. Evidence `ev_e4acdc03…`, `ev_1e905bb9…` |
| Planet | `0x48e430` / `0x8e430` (state vtable slot `0x647b44`) | control words `0x2`, `0x10` | no raw VK; forwards to `0x48c360` |
| Room | `0x490870` / `0x90870` (state vtable slot `0x647be4`) | control words `0x11`, `0x48`, `0x7d1` | no raw VK; forwards to `0x48c360` |
| Dialogs / overlays | e.g. `0x434020`, `0x4369b0`, `0x438df0` | ESC maps to control words `0x21200002`, `0x24200003`, `0x25200017` through vtable `+0x28` | **PINNED dialog-level cancel/close**, not a state back. Evidence `ev_3cb14e44…` |
| Planet with channel dialog mounted | `0x48db40` / `0x8db40` | ESC `0x1b` while parent `+0xf6c` is mounted calls `0x48e530` | dialog-scoped; the called helper inspects the four dialog slot rects, not a state transition. Evidence `ev_3cb14e44…` |

Verdict: the owner's absolute form ("the exe never uses Escape to go
back") is refuted at splash and dialog layer — ESC is wired there. The
narrow form is supported: **no Room/Waiting/Channel/Planet scene key
handler routes ESC to the state's back/exit transition**. Those exits are
chrome control-code dispatch keyed by per-state BACK control words. The
key-to-control table for the four top-level states is not pinned by this
leg; next evidence step is a traced read of the per-state key dispatch
tables or a SendInput VK sweep with the scene name pinned before each
press (the `~/scratch/planet-verify/` recipe).

## 3. Wire pins for BACK chrome

| State | Control / trigger | Wire | Status |
|---|---|---|---|
| Waiting | staged `IDC_BUTTON_SWAIT_BACK` / `..._EXIT` click | C2S `ExitWaiting` 3005 body `00 00` → S2C 2007 on last exit | PINNED (W8, `docs/net-protocol.md` §waiting, `docs/parity/waiting.md` #734) |
| Room | `IDC_BUTTON_SROOM_BACK` `0x13200002` (`docs/hover.md`) | expected channel-leave family; direct tap OPEN | OPEN — next step: scripted gateway tap on Room BACK, name the opcode |
| Channel | exit control `0x13200002` (`docs/net-protocol.md`) | C2S 2021 empty → S2C 2022 result | PINNED (`docs/net-protocol.md:168`) |
| Planet | `IDC_BUTTON_SPLANET_EXIT` `0x19200010` (`docs/hover.md`) opens quit-confirm locally | wire on OK not captured | OPEN — next step: gateway tap around quit-confirm OK |

## 4. Our client's alignment

`menu_of(KeyCode::Escape)` still yields `MenuKey::Back`. That mapping is
kept because the exe pins ESC for splash dismissal and dialog close, and
our splash/options arms use the same edge. The scene-level uses of that
edge (Room → Channel, Waiting → Room, Channel → Planet, Planet modal
cancel) remain **ours-by-design**: the statics above pin no exe scene key
handler for those transitions. The chrome BACK buttons carry the pinned
navigation path; #734 fixed the Waiting click to send 3005 through the
same session bridge. Guard `escape_stays_the_pinned_splash_and_dialog_key`
pins the mapping scope; the walk/menu guards name the same label in their
comments. Removing ESC scene-back outright would break the pinned splash
and dialog arms, so it is not done without a separating input contract.

## Spec rows closed (#735)

- [x] ESC question answered with static evidence (VAs, file offsets, REA
  evidence IDs above; no disassembly or decompiled text pasted).
- [x] Per-state BACK wire sends pinned or OPEN with a named next step
  (section 3; Waiting 3005 and Channel 2021 pinned, Room and Planet OPEN).
- [x] Input mapping aligned to the pin: ESC kept where the exe wires it;
  scene-level Back labeled ours-by-design with the refutation cited in
  parity rows and guard comments; chrome BACK carries pinned navigation.
- [x] Existing walk/menu guards kept and annotated; new guard
  `escape_stays_the_pinned_splash_and_dialog_key` pins the mapping scope.
  Hidden-install-root arm: synthetic mapping, N/A; run clean with the
  data link hidden.
EOF
echo written
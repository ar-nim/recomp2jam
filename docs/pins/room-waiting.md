# Static pins — Room and Waiting (game-reachable backlog)

Target pin: `OTwo.exe` 7.55, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
Tools: Ghidra 12.2 / REA 6.10.1; evidence IDs below are the parent IDs of
REA `analyze_function` dossiers on that exact artifact. File offsets come
from REA `address_to_file_offset` (PE image base `0x400000`). Behavior
statements are our own descriptions of provider output; no decompiled text
is reproduced.

Scope: the 22 addresses in `triage-final.json` attributed to **Room (17)**
and **Waiting / room chat (5)**, all present in `procedures.csv` with
`bucket=game-reachable,pinned_in_repo=no`. Base SHA observed in the main
checkout at the start of this lane:
`ed556bfcfc058e71bc7322d7ea9669780687b9b3` (brief's earlier base
`b872971` is its ancestor). Pinned here: **22/22**; left: **0**.

A static pin is not a player-visible close. Rows in
`docs/interaction-census.md` stay OPEN until a live caller chain from boot
is recorded (Tier 3 where the row demands it); see
`docs/live-path-reachability.md`. "Reached from" below is the provider's
direct-caller edge, not proof of a runtime press.

## Room — 17 pins

### `0x48e8c0` — reset Room scene fields (support)

- **VA / file offset**: `0x48e8c0` / `0x8e8c0` (583,872).
- **Behavior**: clears the Room object's row-selection, chat and timing
  words, sets two boot defaults (a mode word to 3 and a flag to 1), reads
  the current clock into one field, and clears a global Room-state byte.
- **Reached from**: `0x48e700`, the `CStateRoom` constructor, immediately
  after the vftable install.
- **Player-visible reach**: support. It establishes the starting values
  the Room chat/list dispatch later reads; it emits no pixels or wire on
  its own.
- **Evidence**: `ev_3e8bb8b5c0fb18e953941ef6a80acdea7273ff6d9699a463266aa515cf20300b`.
- **Our client**: no counterpart by VA. Equivalent state lives in
  `LobbyChatPanel` / `ChatState` (`crates/o2jam-shell/src/screens/game/chat.rs:142`,
  `:248`) and room defaults in `ScreenGame::set_rooms`
  (`crates/o2jam-shell/src/screens/game/mod.rs:2545`).

### `0x48ef00` — lazily create and focus the Room chat edit control

- **VA / file offset**: `0x48ef00` / `0x8ef00` (585,472).
- **Behavior**: resolves `IDC_EDIT_SROOM_CHAT` (`0x13300015`), reads its
  four stored geometry words, creates an edit control with its width
  enlarged by 50, stores the handle, focuses it, and calls the "edit
  armed" helpers. Returns failure when the control lookup or create fails.
- **Reached from**: `0x48f3a0` (Room dispatch), under the condition that
  the Room object's `+0x2f` edit-created flag is still zero; also from the
  generic control wrappers `0x436360`, `0x437fc0`, `0x439a40`, `0x43aeb0`,
  `0x456ee0`, `0x48e990`.
- **Player-visible reach**: yes — the Room chat input is present and
  focused on entry (census Room ord 20).
- **Evidence**: `ev_8d49ab595afd0276a1458572bc39be771ef598a799bf0ff7f4e7b7e2cf18368e`.
- **Our client**: no native edit control. The draft is
  `ChatState::set_lobby_draft` / `type_lobby_char`
  (`crates/o2jam-shell/src/screens/game/chat.rs:529`, `:534`) painted at the
  staged edit rect by `paint_room_chat`
  (`crates/o2jam-shell/src/screens/paint/chat.rs:398`, `:426`). Guard:
  `typing_on_the_room_scene_fills_the_room_chat_draft`
  (`crates/o2jam-shell/src/screens/game/mod.rs:37007`, Tier 0); live Enter
  leg remains OPEN in census row 20.

### `0x48f360` — append one line to the Room chat list

- **VA / file offset**: `0x48f360` / `0x8f360` (586,592).
- **Behavior**: looks up `IDC_LIST_SROOM_CHAT` (`0x13800014`) and, when it
  exists, passes the supplied text and ink value to the list-row writer
  (`0x510730`).
- **Reached from**: `0x48f090` for incoming chat rows (two sender classes,
  different ink values) and `0x48f3a0` for the `/r` reply command; also
  `0x491190`.
- **Player-visible reach**: yes — chat rows appear in the staged list
  (census Room ord 19).
- **Evidence**: `ev_7ebf3473e6b9bddbd5e8a8c2af3d7c0b0d21d1c825de068b16b46f54ca0463b9`.
- **Our client**: no VA counterpart. Rows are appended by
  `ChatState::push_lobby_user` / `push_lobby_admin`
  (`crates/o2jam-shell/src/screens/game/chat.rs:422`, `:428`) and painted
  by `paint_room_chat` (`crates/o2jam-shell/src/screens/paint/chat.rs:398`).
  Guard: `room_chat_rows_paint_inside_the_staged_list_rect`
  (`crates/o2jam-shell/src/screens/game/mod.rs:37089`, Tier 0).

### `0x492960` — read the Room chat-send mode (support)

- **VA / file offset**: `0x492960` / `0x92960` (600,416).
- **Behavior**: returns the Room object's `+0x78` mode word. No other work.
- **Reached from**: `0x48f3a0`, where value 3 selects the whisper-reply
  branch of the Room command parser.
- **Player-visible reach**: support for `/r` target selection; it paints
  nothing and sends no frame itself.
- **Evidence**: `ev_11aad91d10bac51286a906f8d884f65bbd034d42b5ad8b35239e0d3dcd561617`.
- **Our client**: no VA counterpart; related state is
  `ChatState::last_whisper_peer` / `remember_whisper_peer`
  (`crates/o2jam-shell/src/screens/game/chat.rs:289`, `:583`) and slash
  parsing at `:590`.

### `0x493a50` — gate a room-row press before joining

- **VA / file offset**: `0x493a50` / `0x93a50` (604,752).
- **Behavior**: takes the pressed row control id, finds the matching row
  record and the player's room-list record, then refuses the join with a
  popup when the room has already started, the room is passworded when it
  should not be, the level window excludes the player, the room is full,
  the game-info state is loading, the player lacks PT, or the room uses an
  unregistered song / 7Key mode. On success it sets the join mode and
  returns the room number to the caller; failures return a negative value.
- **Reached from**: `0x48fd20`, the Room control dispatcher, for the five
  row-panel ids `0x1301201b`–`0x13012020`; the caller posts opcode
  `0xbba` (3002) with the returned number.
- **Player-visible reach**: yes — row press either enters Waiting or shows
  one of the refusal popups (census Room ord 26–31, 5; ord 5 QUICKJOIN
  shares the level/window gates).
- **Evidence**: `ev_92af29edd8615724d9797629142e7e4d7219fdf74459ae3b7cd4b52c6c3bf924`.
  Refusal captions were read as CP949 bytes at `0x648330`, `0x648280`,
  `0x648250`, `0x648208`, `0x6481e8`, `0x648190`, `0x648178` through REA
  `read_bytes`.
- **Arm-to-caption map** (2026-11-25 senior REA pass, pseudocode dossier
  `ev_afff8af205e2391c580881dc5883d3c58fba5a8a400298c65448e15c105b4ea1`,
  byte reads `ev_3b91f7e4b3512ff02a9e2862cfaed2ae379a9c818bbcfd842be452cf08d70176`,
  `ev_771176cb5fb48980ddbe5c5f38c9820408d4e15bdb68529463708188d04bb006`):
  unregistered song `0x648330`; 7Key-mode level-4 gate `0x648280`
  (mode byte 2 + player level < 4); room full `0x648250` (`%s` = the
  room name); game-info loading `0x648208` (row state byte 3); PT
  insufficient `0x6481e8` (skilled row + player level < 10 + the PT word
  the PT word at global `0x768b34` == 0 — the same global the 5026 FREEINFO story calls
  the pass word, so its reader set now includes this gate; the writer
  stays OPEN); level window `0x648190` (`%d`/`%d` = the row's min/max
  level bytes); password `0x648178` (posted with popup code 8).
  **There is no started-room caption in this function**: the only other
  popup is the relay-down NORMAL-mode notice at `0x644d98`
  (`network problem, NORMAL mode only`), fired by the game-info state
  word, not by the row. A started room either falls through to the 3002
  post (server refuses) or is gated by the row state byte elsewhere —
  which of the two is OPEN; next step: pin the started-room state-byte
  value and watch one live press.
- **Our client**: partial. `ScreenGame::quickjoin_accepts` /
  `queue_quickjoin` (`crates/o2jam-shell/src/screens/game/mod.rs:2588`,
  `:2609`) implement password, seat and level-window gates for QUICKJOIN;
  the row-press popup strings and the 7Key/PT/loading arms have no
  counterpart. Guards: `room_quickjoin_queues_one_3002_for_an_eligible_row_and_nothing_for_none`
  (`mod.rs:36819`, Tier 0) and
  `live_room_quickjoin_click_posts_3002_from_boot`
  (`crates/recomp2jam/src/live_input_legs.rs:1462`, Tier 3).

### `0x495a30` — close the Room web user-list sheet on scene teardown

- **VA / file offset**: `0x495a30` / `0x95a30` (612,912).
- **Behavior**: if a session is active and the stored web-window handle is
  non-null, sends a close message to that window and clears the handle;
  then rebuilds the web user-list URL (including the gift-on flag from the
  message ring) and refreshes the sheet state in `0x495710`.
- **Reached from**: the shared input/window hook `0x416a80` and the exit
  wrapper `0x5a5190`.
- **Player-visible reach**: yes for the web user-list sheet and its
  gift-on state (census Room ord 70/69); the web content itself is outside
  this client.
- **Evidence**: `ev_96389df4b6a582dddf2943954f64d6b328b0341aa5fca1064bb798edab5dcdf8`.
- **Our client**: no web-sheet counterpart. Room exit is
  `ScreenGame::exit_room_to_channel`
  (`crates/o2jam-shell/src/screens/game/mod.rs:4461`) and chat reset is
  `ChatState::clear_channel_panel`
  (`crates/o2jam-shell/src/screens/game/chat.rs:294`); the deliberately
  empty web desc well is guarded by
  `room_user_panel_leaves_the_desc_well_empty_without_a_web_source`
  (`crates/o2jam-shell/src/screens/paint.rs:11999`, Tier 0).

### `0x496100` — reserve storage for a four-byte-element vector (support)

- **VA / file offset**: `0x496100` / `0x96100` (614,656).
- **Behavior**: zeroes the vector's begin/end/capacity words; for a
  nonzero request it allocates through `0x496c20`, sets begin/end to the
  new block and capacity to begin plus four bytes per element. Oversized
  requests go to the allocator failure helper `0x496a20`.
- **Reached from**: `0x4944c0` (Room QUICKJOIN row filter), `0x4be200`
  (Waiting), `0x4d3170` (Playing), `0x5a8c90` — a shared template
  instantiation, not Room-only code.
- **Player-visible reach**: support for row/vector work; no pixels or wire.
- **Evidence**: `ev_79c2e210a22549b9dd2417a59f9f0cf0c2db2077b3cff34f2377faeef8711288`.
- **Our client**: no counterpart by VA; `Vec` in `queue_quickjoin`
  (`crates/o2jam-shell/src/screens/game/mod.rs:2609`) supplies the same
  role with Rust ownership rules.

### `0x4961c0` — free a four-byte-element vector (support)

- **VA / file offset**: `0x4961c0` / `0x961c0` (614,848).
- **Behavior**: frees the vector's begin pointer when present and clears
  its three words.
- **Reached from**: `0x495d80` (Room), `0x4944c0`, `0x4be200` (Waiting),
  `0x4d3170` (Playing) — shared template code.
- **Player-visible reach**: support; no pixels or wire.
- **Evidence**: `ev_56d1d617b17ef604f785a5a02abecb77229e36477b8d6a828b66a381e4731366`.
- **Our client**: no counterpart by VA; Rust drop handles this.

### `0x496240` — insert a range into a four-byte-element vector (support)

- **VA / file offset**: `0x496240` / `0x96240` (614,976).
- **Behavior**: inserts `param_3` elements at `param_2` in the vector:
  grows storage when the tail has too little room, moves the tail with
  `memmove`, copies the source range, and updates begin/end/capacity.
- **Reached from**: `0x495f50` (Room row-slot insert) and `0x4c0950`
  (Result range) — the address-cluster Room label in the triage is only
  a caller hint; the body is generic.
- **Player-visible reach**: support for row/vector work; no pixels or wire.
- **Evidence**: `ev_ec6a847b146d2bedf3be5d0b05bc485b2b76aa87c205a59edd2de86302c4a59f`.
- **Our client**: no counterpart by VA; `Vec::insert` / `Vec::extend_from_slice`
  behind `ScreenGame::upsert_room`
  (`crates/o2jam-shell/src/screens/game/mod.rs:2638`).

### `0x496629` — rebalance a vector after erasing a range (support)

- **VA / file offset**: `0x496629` / `0x96629` (615,977).
- **Behavior**: computes the erased element count, advances a saved
  iterator by it, frees the old block, and recomputes begin/end/capacity
  from a replacement buffer.
- **Reached from**: only `0x496240` (vector insert) — an inlined helper
  of the container, not a Room action.
- **Player-visible reach**: support; no pixels or wire.
- **Evidence**: `ev_a74d1e9f7e4703498e41f5d4c6162de9bf47c2462ddae1b6694252bf544559a0`.
- **Our client**: no counterpart by VA; Rust allocation handles this.

### `0x4967ff` — fill one vector range with a value (support)

- **VA / file offset**: `0x4967ff` / `0x967ff` (616,447).
- **Behavior**: advances the vector's end pointer by a count, then writes
  one supplied value through every slot in the new range.
- **Reached from**: only `0x496240` (vector insert).
- **Player-visible reach**: support; no pixels or wire.
- **Evidence**: `ev_f4a059a604e1e509766ece8da6484860fa013c73bff55716c3b66ba4e8243300`.
- **Our client**: no counterpart by VA; `Vec` fill idiom.

### `0x4969d0` — copy filled elements and return the new end (support)

- **VA / file offset**: `0x4969d0` / `0x969d0` (616,912).
- **Behavior**: calls the four-byte fill-copy helper `0x497500` and
  returns the destination pointer advanced by four bytes per element.
- **Reached from**: only `0x496240` (vector insert).
- **Player-visible reach**: support; no pixels or wire.
- **Evidence**: `ev_d9b15c5824734ac13ac82e9eea022cb406fbcb21db1f111071639bede269ba8b`.
- **Our client**: no counterpart by VA; `Vec` fill idiom.

### `0x496b00` — move a four-byte range forward (support)

- **VA / file offset**: `0x496b00` / `0x96b00` (617,216).
- **Behavior**: computes the element count between two pointers and
  `memmove`s that many four-byte slots to a destination, returning the
  destination advanced past the moved range.
- **Reached from**: `0x496070` (container code); no Room handler directly.
- **Player-visible reach**: support; no pixels or wire.
- **Evidence**: `ev_abed27a42550439f10be07ef16cc5f76316d3c3b15a8b855ee79f6602da094ab`.
- **Our client**: no counterpart by VA; Rust slice copy handles this.

### `0x496b80` — move a four-byte range backward (support)

- **VA / file offset**: `0x496b80` / `0x96b80` (617,344).
- **Behavior**: computes the element count and `memmove`s the range to a
  destination placed that many slots earlier, returning the destination
  start.
- **Reached from**: only `0x496240` (vector insert).
- **Player-visible reach**: support; no pixels or wire.
- **Evidence**: `ev_25d6a78bcb3c739ea3d85550e64c3f046bb2090c8f44824de4bd95219835ed32`.
- **Our client**: no counterpart by VA; Rust slice copy handles this.

### `0x496e00` — forward a bounded range move (support)

- **VA / file offset**: `0x496e00` / `0x96e00` (617,984).
- **Behavior**: delegates all four arguments unchanged to `0x497b90`
  (bounded `memmove`).
- **Reached from**: only `0x496240` (vector insert).
- **Player-visible reach**: support; no pixels or wire.
- **Evidence**: `ev_bd922cde7a406ef7456d3b7a4f4d7db32f8f34bdea76d97108f8bed4f28d6a66`.
- **Our client**: no counterpart by VA; Rust slice copy handles this.

### `0x497500` — copy a counted run of four-byte values (support)

- **VA / file offset**: `0x497500` / `0x97500` (619,776).
- **Behavior**: loops `param_2` times, copying one four-byte value from
  the source pointer to the destination and advancing both.
- **Reached from**: `0x495da0`, `0x4969d0`, `0x496240` — container code.
- **Player-visible reach**: support; no pixels or wire.
- **Evidence**: `ev_6b6a1f95053f96aa2d2f5359d3664bbb396453aa33963358ae68f8fb7cec4ae2`.
- **Our client**: no counterpart by VA; Rust `copy_from_slice` idiom.

### `0x497b90` — bounded four-byte `memmove` (support)

- **VA / file offset**: `0x497b90` / `0x97b90` (621,456).
- **Behavior**: computes the element count between two pointers, moves
  the bytes when nonzero, and returns the destination advanced past the
  moved range.
- **Reached from**: `0x496240` (vector insert) and `0x496e00`.
- **Player-visible reach**: support; no pixels or wire.
- **Evidence**: `ev_fc9b60c3373e1ad48e545e2d29db5faae2be821ff4fd678b4057a39667c75971`.
- **Our client**: no counterpart by VA; Rust slice copy handles this.

## Waiting / room chat — 5 pins

### `0x4b0470` — append one byte to a Waiting chat buffer (support)

- **VA / file offset**: `0x4b0470` / `0xb0470` (722,032).
- **Behavior**: writes one source byte into each destination slot for a
  caller-supplied count (a byte-wise fill used to append one chat byte).
- **Reached from**: `0x4af920` (single-byte chat append), `0x4afb40`
  (ranged chat append) and `0x4b0290`.
- **Player-visible reach**: support for the Waiting chat ring; no pixels
  or wire of its own.
- **Evidence**: `ev_249651b52a185c3685ef7cd0746d8eebb659f434cadfc3dc67c0d604a11ee1df`.
- **Our client**: no counterpart by VA; `ChatState::push_waiting_user` /
  `push_waiting_admin` (`crates/o2jam-shell/src/screens/game/chat.rs:399`,
  `:404`) own the row ring with a newest-five cap.

### `0x4b04e0` — move a byte range in a Waiting chat buffer (support)

- **VA / file offset**: `0x4b04e0` / `0xb04e0` (722,144).
- **Behavior**: byte-count `memmove` between two pointers, returning the
  destination advanced by the moved count.
- **Reached from**: `0x4afb40` (ranged Waiting chat append).
- **Player-visible reach**: support for the chat ring; no pixels or wire.
- **Evidence**: `ev_f31a68b8467bac6f55428fa32e6e261415ec87533c217b8cedbe24eaa55effa5`.
- **Our client**: no counterpart by VA; Rust `Vec`/slice movement in
  `crates/o2jam-shell/src/screens/game/chat.rs:399`.

### `0x4b5e80` — reset Waiting scene fields (support)

- **VA / file offset**: `0x4b5e80` / `0xb5e80` (745,088).
- **Behavior**: clears the Waiting object's chat, seat, map and album
  words, sets one counter to 10, zeros two 21-element arrays (eight seats
  and their paired data), clears two globals, calls `0x486470`, and clears
  three trailing words.
- **Reached from**: `0x4aaa70` (`CLiveWaiting` constructor) and
  `0x4b0550` (`CNormalWaiting` constructor), both after the vftable
  install and base-state call.
- **Player-visible reach**: support. It supplies the defaults for the
  seat/readout rows; it paints nothing itself.
- **Evidence**: `ev_5cbb35bf2bc58e28648ca4b5a78863ab0a99d8b212e51141d35a56f80a268e2e`.
- **Our client**: no counterpart by VA; Waiting staging lives in
  `ScreenGame` (`crates/o2jam-shell/src/screens/game/mod.rs:2684`,
  `:3375`) with chat reset in `ChatState::clear_waiting`
  (`crates/o2jam-shell/src/screens/game/chat.rs:299`).

### `0x4bb700` — choose between the two room-removal captions

- **VA / file offset**: `0x4bb700` / `0xbb700` (767,744).
- **Behavior**: resolves a control id derived from the global channel-page
  value plus `0x16015022` (Waiting `EXTEND1`), and if that control exists
  returns a field of the associated object (`0x52adf0`). The caller uses a
  zero result to select the "no song, removed" caption and a nonzero
  result to select the plain "removed" caption.
- **Reached from**: `0x55e2f0` (lobby refresh), only while the session
  pointer is present and the global room-state byte is 0 or 1. The two
  captions are CP949 strings at `0x64da60` and `0x64da88`.
- **Player-visible reach**: yes — the notice shown after a room removal
  differs by cause (census Waiting ord 33 seat panels, notice text).
- **Evidence**: `ev_e0cb3d8a08088210e4bb9485b060693c73107981c7b952d933c93520d785fe3f`.
  Caption bytes read with REA `read_bytes`; caller text decoded as CP949.
- **Our client**: no counterpart. `ScreenGame::waiting_notice`
  (`crates/o2jam-shell/src/screens/game/mod.rs:3918`) covers START-gate
  notices, not the room-removal pair; this needs its own wire leg and
  guard before a row can close.

### `0x529fb0` — construct a Waiting button (`CCBtnWaiting` vftable)

- **VA / file offset**: `0x529fb0` / `0x129fb0` (1,220,528).
- **Behavior**: calls the button base initializer, installs the
  `CCBtnWaiting` vftable, clears the button's face counters and flag words,
  and zeroes a short trailing byte range.
- **Reached from**: `0x4329b0` (control factory) plus `0x48a620` and
  `0x48b2d0` — shared by every staged Waiting button/row panel.
- **Player-visible reach**: support. It makes the controls clickable and
  gives them their initial faces; it chooses no action or wire itself.
- **Evidence**: `ev_a124359e5330686d84114bb1db729d6e9e88e137ae7cd4346ca5e53c7ea121b0`.
- **Our client**: no counterpart by VA. Staged controls are dispatched by
  `ScreenGame::click_waiting_button` /
  `ScreenGame::click_waiting`
  (`crates/o2jam-shell/src/screens/game/mod.rs:5206`, `:6499`).

## Census rows touched by this pin set (rows stay OPEN)

- Room ord 5 (QUICKJOIN), 19 (chat list), 20 (chat edit), 26–31 (row
  panels), 69–70 (web user-list/desc) in `docs/interaction-census.md`.
- Waiting ord 33–40 (`EXTEND` row panels) and the removal-notice surface
  in `docs/interaction-census.md`.

The spec-shaped copy of the player-visible subset is in
`/home/arnim/scratch/lanes-2026-11-25/spec-enrich-pins-room-waiting.md`;
the query log and OPENs are in
`/home/arnim/scratch/lanes-2026-11-25/findings-pins-room-waiting.md`.

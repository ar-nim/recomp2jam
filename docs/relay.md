# Live relay mode — opcodes and packets (7.55)

Version pinned to 7.55 (`AngelO2Jam.exe` 5,029,888 B, sha256
`83a04541…72c823b`). Static analysis only: in-exe string/offset
survey plus REA/Ghidra dossiers; field layouts below mirror
Memoryer's clean-room `Memoryer.Relay` C# (sanctioned reference,
never game bytes). No live-server contact, no disassembly
reproduced. Decoders live in `o2jam-net` (`src/relay.rs`,
`src/inflate.rs`); no network sends beyond tests.

Live relay shares room PvP state peer-to-peer: TCP (fallback) and
UDP (primary) to the 3 hardcoded relay IPs. Game TCP 4014/4015
cover the gateway leg; relay ops below cover the peer leg.

## 1. Opcode map (`RelayCommand`)

| Op | Name | Dir | Exe anchor |
|---|---|---|---|
| 1 | PeerConnected | S→C | `SetID(1)` ctor file `0x18e100`, vtable ref `0x24e2ec` |
| 2 | PeerConfirm | C→S | `SetID(2)` ctor file `0x18ca60`, ref `0x24e27c` |
| 3 | PeerEndpointAssigned | S→C | `SetID(3)` ctor file `0x18e1e0`, ref `0x24e2fc` |
| 4 | Ping | C→S | `SetID(4)` ctor file `0x18cc50`, ref `0x24e28c` |
| 5 | UpdateGameStats | C→S, fanned out S→C | `SetID(5)` ctor file `0xea4e0`, ref `0x24b334` |
| 6 | ComboBroken (`CO2JamMissPack`) | C→S, fanned out | `SetID(6)` ctor file `0x16dfe0`, ref `0x24dc1c`; RTTI `CO2JamMissPack` |
| 7 | ComboStarted (`CO2JamCoolPack`) | C→S, fanned out | `SetID(7)` ctor file `0x16e080`, ref `0x24dc2c`; RTTI `CO2JamCoolPack` |
| 10 | CreateSession | C→S | `u8`-counted `(key1, key2)` member list |
| 11 | DeleteSession | C→S | same shape as 10 |

RTTI for both pack classes is one dossier record
(`ev_da26a62f`); relay endpoint strings at file
`0x24e240`/`0x24e250`/`0x24e260` (`.184`/`.182`/`.183`) plus
`0x24e368` (`.52`); UHP strings file `0x24e1e4`–`0x24e2c0`;
RUDP/RTTI names `CUDPFacade`/`CTCPRelayServClient`/receiver/
deliver/resender/`CUHPSender`/`CRelayServerConnect`/
`CRelayRegisterReq`/`CRelayConnectCheckReq`/
`CConnectTCPConfirmKey`/`CUDPFailTCPConnect`.

The exe anchors pin the opcodes only (`SetID` + RTTI). Every
field layout below is Memoryer-corroborated (`Memoryer.Relay`
messages, field order = wire order); exe byte-level proof of the
bodies is still open (ctors traced only to the `SetID` call).

## 2. Relay TCP framing (byte-verified shared shape)

Relay TCP uses the same `CPacket_Str` framing as the game gateway
(`[u16 len][u16 id][body]`, `len` counts the whole frame, id at
buffer offset 2). Byte proof: the op-5 ctor at file `0xea4e0`
stages word 5 ahead of its `SetID` call and reaches the same
`SetID` function at file `0x1a0550` as the gateway path (call
site file `0xea4f4`; `ev_96ec199150e74ab84e56be7c868839f3`,
observed) — the id half of the frame is the identical write
(id-at-offset-2 pinned by `FUN_005a0550`, `ev_861713e2`;
length-includes-prefix server-verified in `docs/net-protocol.md`
§3). The relay leg therefore frames op 5 as the 66-byte TCP frame
`42 00 05 00` plus the 62-byte tally
(`ev_28b94c98febf3700c39fc4a07d6a9f26`, observed codec shape),
against the 20-byte gateway 4014 tick (`14 00 AE 0F` plus the
16-byte body; `ev_f5441895dffce3c0a41edd2c605f63cc`). No
separate relay framer exists: both legs share
`frame::encode_frame`/`decode_frame`, locked by the op-5
header-shape and tally-vs-tick byte-vector tests in `relay.rs`.
No captured relay wire bytes are claimed; the vectors are
struct-built codec shapes.

## 3. Common routing preface (ops 1–7, 24 bytes)

Memoryer orders 0–2, always present, no count prefix:

| Offset | Size | Field |
|---|---|---|
| 0 | 4 | `code` (`i32`; zero in server events) |
| 4 | 18 | 3 relay-server endpoints, each 4-byte IP + `u16` port |
| 22 | 2 | `unused` (`u16`; zero observed) |

Decoded as `RelayRouting`. The endpoints use the same 6-byte IP +
port shape as game TCP (`Authorize`).

## 4. Packet layouts (body bytes after the frame header)

Multi-byte integers are little-endian. Struct names are the
`o2jam-net` decoder names.

- `PeerConnected` 1 (32 B): preface + `session_key1` `i32` +
  `session_key2` `i32`.
- `PeerConfirm` 2 (48 B): preface + keys + `local`
  (`ConnectionEndpoint`) + `public`. `ConnectionEndpoint` is 8 B:
  `port1` `u16`, `port2` `u16`, 4-byte IP. The `IPEndPoint` view
  of it is `(address, port1)`.
- `PeerEndpointAssigned` 3 (32 B): preface + `public`
  (`ConnectionEndpoint`). Receiving it forces the game onto TCP
  relay instead of peer-to-peer (Memoryer sends it when no UDP
  pool is configured).
- `RelayPing` 4 (32 B): preface + `start` `u32` + `tick` `u32`.
- `RelayStats` 5 (62 B): preface + `role` `u16` + 9×`u16`
  (`cool`, `good`, `bad`, `miss`, `combo`, `max_combo`,
  `jam_combo`, `total_jam_combo`, `long_note_combo`) +
  `long_note_score` `u32` + `life` `u16` + `pill_buildup` `u32` +
  `pill_count` `u32` + `score` `u32`.
- `ComboBroken` 6 / `ComboStarted` 7 (30 B each): preface +
  `autoplay` as `u32` (only 0/1 accepted; 7.55 always sends 1 —
  five hardcoded dword-1 stores feed the send paths, no zero-store
  exists; §6) + `role` `u16`. Client sends carry `code`/`unused`
  as 0 (no nonzero writer located on any combo send path; §6).
- `CreateRelaySession` 10 / `DeleteRelaySession` 11
  (1 + 8×N B): `u8` member count + N×(`session_key1` `i32`,
  `session_key2` `i32`).

`MemberRole` (`u16` on the wire): 0 None, 1 Champion, 2
Challenger, 3 Spectator. Memoryer agrees
(`Encore.Metadata.MemberRole`), but the meanings are exe-pinned: the
role word at VA `0x77ee86` is loaded by seven `66 A1 86 EE 77 00`
sites (files `0xaec29`/`0xc33b4`/`0xdbb1c`/`0xe6f81`/`0xe7a75`/
`0xe840a`/`0xea26e`); four gate Spectator with `83 F9 03` (files
`0xaec3d`/`0xc33c2`/`0xe7a89`/`0xe841e`); file `0xdbb1c` dispatches
Champion/Challenger with `83 7D 8C 01`/`83 7D 8C 02` (else fallthrough)
and file `0xe6f81` checks `83 F9 01` then `83 F8 02`; 0 is the residual
None. The op-5 packer site (`0xea26e`) copies without branching.
(`ev_acacd596017f22ba805e419c18cfe686`,
`ev_cbe189ba7fb55898e42e98c6175b8e75`,
`ev_6f91b9283627171b33801a2c0e653100`,
`ev_e5879e64727f6c4c5ab0759b929433c8`, observed.) Corroborating
context (not value proof): `Winner_image.ojs` is pushed at file
`0xe66cc` and `Challenger_image.ojs` at file `0xe671e`, both inside the
same `0xe6xxx`–`0xe8xxx` Live-result region as the role branches; which
value selects which art needs a Ghidra dossier before claiming. Full
table in §11.

### Op 5 is a tally, not a 4014 echo

`docs/net-protocol.md` §4c says relay op 5 "mirrors" game TCP
4014. Precisely: it mirrors the *stats*, not the 16-byte body.
Game 4014 carries one tick (`kind`/`value`/`seq`/`score`/
`longNote`, 16 B); relay op 5 carries the whole tally (62 B:
per-hit counts, combo/max, jam level/max, long-note score,
life, pill buildup/count, score). The relay fans each peer's
tally out to the other session members (`RelayController`
forwards the received event unchanged), so C→S bytes =
S→C bytes and one decoder covers both directions. Which game
loop writes the relay op-5 body in-exe is still open (only the
`SetID(5)` ctor at file `0xea4e0` is pinned).

## 5. UDP RUDP frame shape

Each datagram is one of:

```text
data: [u16 uncompressedLen][zlib(inner)]
ack:  [0x03][seq][0x00][0x00]          (4 bytes, no zlib, no prefix)
```

`inner` (zlib payload):

```text
[u8 type][u8 seq][u16 innerSize][u32 cmd][body...]
```

- `type`: 1 Unreliable (Live lane traffic), 2 Reliable
  (retransmitted until acked: stats, combo packs), 3 Ack.
- `innerSize` = `body.len()` + 4 (the `u32` command word).
- `cmd`: relay opcode as `u32` (only the low `u16` is used;
  wider values are rejected).
- Acks are bare: `encode_ack(seq)`; a 4-byte datagram opening
  with `0x03` decodes as Ack (Memoryer behavior).

`UdpRelayFrame::decode_datagram` validates the length prefix
against the inflated bytes (stricter than Memoryer, which
ignores overruns), strips the envelope, and returns `payload`
as `[u16 command][body]` with `command_id()`,
`relay_command()`, and `body()` views. `encode_datagram`
builds the stored-block zlib form; `inflate.rs` (own
RFC 1950/1951 code: stored, fixed- and dynamic-Huffman blocks,
header + Adler-32 checks, 1 MiB output cap) decodes whatever
the server sends.

## 6. Evidence

- Exe opcodes: `SetID(1..7)` ctors files
  `0x18e100`/`0x18ca60`/`0x18e1e0`/`0x18cc50`/`0xea4e0`/
  `0x16dfe0`/`0x16e080`; vtable refs `0x24e2ec`/`0x24e27c`/
  `0x24e2fc`/`0x24e28c`/`0x24b334`/`0x24dc1c`/`0x24dc2c`;
  pack RTTI `ev_da26a62f`.
- In-exe send sites near the score path (gameplay lifebar
  notes): Jam on Cool/Good tails files `0x16e660`/`0x16e7dd`,
  Life on the tick site file `0x4eaec0`; event-sink suspect
  (codes 1/9/14 at files `0x176691…`/`0x176bdf…`/`0x176ea9…`,
  `docs/systems-survey.md`) may feed the combo packs.

### Combo writers: when `autoplay` and the state field go nonzero (7.55)

Static only over the local `install-root` copy (read-only, VA = file
+ `0x400000`); behavior described in own words with file offsets plus
local-static record ids (sha256 of the record tag, truncated to 32 hex).
Tools: `python3` byte scans; no new REA dossier was issued. No
disassembly is reproduced.

- Ctor anchors (carried from §1): `SetID(6)` at file `0x16dfe0`
  (vtable ref file `0x24dc1c`), `SetID(7)` at file `0x16e080` (ref
  file `0x24dc2c`). A full-image scan for relative calls into either
  ctor finds none (`ev_55d43b289edf1ab13813a2a3530c03f1`, observed):
  both packs are table-constructed, same as op-5 (whose `SetID(5)`
  ctor likewise has no direct caller).
- Shared send path: the relay send helper at file `0x187870` has
  exactly three callers image-wide (`ev_fae5559777b535cb3878108d9b0484df`,
  observed): `0xea385` (op-5 tally, see `docs/op5-writer.md`) plus
  `0x16dee9` and `0x16df58` (the combo pair below). The op-6/7 packs
  share op-5's send path.
- Score-region dispatcher at file `0x16ddf0`
  (`ev_09e854f508b2e4040ee438ec6356d8f8`, observed): entered from
  four score-region call sites (files `0x16e49b`/`0x16e686`/`0x16e803`/
  `0x16e8e0`, passing small selectors 1–4); gated on the Live-mode
  byte at `0x77cd33` equal to 6 (the op-5 tick gates the same byte
  equal to 5) and the caller selector. Two send branches follow,
  selected by the object field at `+0x39b0`: equal to 1 stages the
  miss-side pack (vtable `0x64dc24`, whose first entry points at file
  `0x16e110`), equal to 2 stages the cool-side pack (vtable
  `0x64dc14`, first entry file `0x16e140`). Each branch stages an
  autoplay dword of 1 (stores at files `0x16dec0` and
  `0x16df28`/`0x16df2f`), reads the role word from global `0x77ee86`
  (14 readers image-wide, incl. the op-5 tick and the fanout below),
  sends through `0x187870` with a zero flag word, then re-stores the
  same state value (1 at file `0x16def3`; 2 at file `0x16df62`) and
  restores vtable `0x64b31c`. Image-wide, `+0x39b0` has exactly five
  sites: the two compares, the two stores, plus a zeroing reset at
  file `0x16f90e` (round-init zeroing). No autoplay-0 store exists on
  either send path — the one zero-store in the region targets a
  different epilogue slot (file `0x16deb9`). So `autoplay` is nonzero
  (= 1) on every dispatcher-sent combo pack, and the state field is
  nonzero (1/2) from the send until the next round init.
- Relay-table fanout wrappers at files `0x16df90`/`0x16e030`
  (`ev_881b47397192e387e32beb24c313ba1b`, observed): each hardcodes
  an autoplay local to 1 (files `0x16dfa8`/`0x16e048`) and stages
  routing/autoplay/role through the `0xeb4d0`/`0xeb570`/`0xeb550`
  helpers (pushed sizes `0x18`/`0x04`/`0x02` match the 24/4/2 wire
  field lengths). `0x16df90` has exactly one caller (file `0x18eafa`,
  inside the relay-table fanout function at file `0x18ea9a`, which
  builds a cool-side object (vtable `0x64dc14`), re-stages role from
  `0x77ee86`, and fans out to members with zeroed slots);
  `0x16e030` has no direct caller (reached through the relay table).
  No nonzero `code`/`unused` writer was located on any of these
  paths — client combo sends carry both as 0.
- Memoryer layouts: `Memoryer.Relay/Messages/Commands.cs`
  (`RelayCommand`), `Messages/Events/UpdateGameStatsEventData.cs`,
  `ComboBrokenEventData.cs`, `ComboStartedEventData.cs`,
  `PeerConnectedEventData.cs`,
  `PeerEndpointAssignedEventData.cs`,
  `Messages/Requests/PeerConfirmRequest.cs`, `PingRequest.cs`,
  `CreateRelaySessionRequest.cs`, `DeleteRelaySessionRequest.cs`,
  `Messages/ConnectionEndpoint.cs`,
  `Messages/Codec/IPEndpointCodec.cs`,
  `Messaging/UdpRelayFrameCodec.cs`, `UdpRelayPacketType.cs`;
  `Encore.Metadata.MemberRole`.
- Byte vectors: relay bodies cross-checked against independent
  Python `struct` builds; UDP vectors are Python `zlib.compress`
  outputs (levels 0/6/9, incl. a dynamic-Huffman block).
- Relay endpoint selector (file `0x18c6c0`, VA `0x58c6c0`): the three
  relay IP string pointers are staged at file
  `0x18c6d7`/`0x18c6de`/`0x18c6e5` (`0x64e260`/`0x64e250`/`0x64e240`),
  the dword selector is range-checked and dispatched through jump
  tables at file `0x18c850`/`0x18c864` (VA `0x58c850`/`0x58c864`),
  and each leaf stores port `0x998f` (39311) or `0x9990` (39312) with
  relay index 0/1/2 (file `0x18c73b`–`0x18c77f`); follow-up stages
  push `50000` (`$0xc350`) and add `10000` (`$0x2710`). Selector
  immediates are the byte proof behind `RELAY_UDP_PORTS` in
  `o2jam-net`.
- Relay TCP shared framing (`ev_96ec199150e74ab84e56be7c868839f3`,
  `ev_28b94c98febf3700c39fc4a07d6a9f26`,
  `ev_f5441895dffce3c0a41edd2c605f63cc`, observed): the op-5
  ctor at file `0xea4e0` stages word 5 and calls the same `SetID`
  function at file `0x1a0550` as the gateway path (call site file
  `0xea4f4`), so the id half of the relay TCP frame is the
  identical id-at-offset-2 write; with the server-verified
  length-includes-prefix semantics this yields the 66-byte op-5
  TCP frame (`42 00 05 00` + 62-byte tally) against the 20-byte
  gateway 4014 tick (`14 00 AE 0F` + 16-byte body). Vectors are
  struct-built codec shapes in `relay.rs`; no wire capture claimed.
- Relay TCP ports: no hardcoded immediate
  (`ev_6e655b70da829a83f2b6d07105cb025c`, observed):
  full-image `u16` scan over the local copy finds `0x998f` at
  exactly files `0x18c73e`/`0x18c75e`/`0x18c77e` (all `C7 45 F0`
  port-word stores in the selector leaves) and `0x9990` port-word
  stores only at files `0x18c74e`/`0x18c76e`; the five other
  `0x9990` hits are a vtable-table address (`0xb3d3a`/`0xb3da1`),
  a relative call (`0xc263c`), a jump table (`0x12994c`), and data
  (`0x28b5b5`). TCP peer-leg ports are therefore server-provided
  (`GetLiveState`/`GetP2PList`); `RELAY_UDP_PORTS` stays the only
  hardcoded relay-port constant by design.
- UHP result sink (file ~`0x187a70`): references the logger name
  `CHolePunchingCtrl::InsertUHPResult` (file `0x24e1e4`, ref file
  `0x187a8c`) and the format `Hole punching Result : %d : IP : %d -
  Port : %d` (file `0x24e208`, ref file `0x187a82`).
- UHP start/success branch (file ~`0x18d7c5`): a status byte picks
  `UDP Hole Punching Success~!!` (file `0x24e2c0`, ref file
  `0x18d7cd`) when nonzero else `UHP Start` (file `0x24e2b0`, ref
  file `0x18d80f`); both share one log-context pointer, then poll in
  a `Sleep(300)` loop (push `$0x12c` + `call *Sleep`, IAT
  `0x63f298`, call site file `0x18d84d`).
- Adjunct `.52` host: string `203.238.136.52` (file `0x24e368`)
  has exactly two code refs image-wide (both push the `0x64e368`
  pointer: files `0x1936e2`, `0x1936fc`) inside the shared setup
  helper entered at file `0x193650` (7 call sites: files `0x90921`/
  `0x90cd9`/`0x9cf4a`/`0xa4d40`/`0xab25b`/`0xb0980`/`0xcb082`, each
  passing the global at `0x7684d8`). The same block stores constant
  `7788` (`$0x1e6c`, file `0x19370b`, the only `0x1e6c` dword
  image-wide) into a setup slot just after the second host call —
  recorded as a number, not claimed as a port. Verdict: separately
  configured adjunct endpoint on the relay subnet, excluded from the
  relay-pool, UHP-helper, and TCP-fallback wire paths (neither the
  relay selector at file `0x18c6c0` nor the UHP controller at file
  `0x18d580` references `.52` or `7788`, and no `SetID`/frame/send
  immediates sit in the setup block). No packet shape is pinned for
  it; the exact service role stays open.
- Fallback class: RTTI `.?AVCUDPFailTCPConnect@@` (name file
  `0x28060c`); no dedicated log strings. UDP send/recv run through
  `WSASendTo`/`WSARecvFrom` (IAT `0x63f584`/`0x63f580`; send site
  file `0x1b1ad2`, recv site file `0x1b19cd`).

## 7. OPEN (not claimed)

- Exe byte-level proof of any relay body (only `SetID` + RTTI
  pinned); the op-5 game-loop writer in particular.
- Relay TCP framing: CLOSED at the shared-shape level (§2: op-5
  ctor stages word 5 into the same `SetID` at file `0x1a0550`,
  call site file `0xea4f4`; 66-byte op-5 TCP frame vs 20-byte
  gateway 4014 tick). Still open: captured relay wire bytes
  (vectors are struct-built codec shapes, not a capture).
- Relay TCP ports for the peer leg: no hardcoded immediate exists
  in-exe. A full-image `u16` scan finds `0x998f` at exactly the
  three selector-leaf port-word stores (files `0x18c73e`/`0x18c75e`/
  `0x18c77e`, `C7 45 F0 8F 99 00 00`) and `0x9990` port-word stores
  only at the two matching leaves (files `0x18c74e`/`0x18c76e`);
  the five other `0x9990` hits are non-port bytes (table address at
  files `0xb3d3a`/`0xb3da1`, relative call at file `0xc263c`, jump
  table at file `0x12994c`, data at file `0x28b5b5`)
  (`ev_6e655b70da829a83f2b6d07105cb025c`, observed). Verdict:
  the only hardcoded relay ports are 39311/39312 (UDP-context
  selector); TCP peer-leg ports arrive via the server
  (`GetLiveState` 4042/4043, `GetP2PList` 4044/4045, Memoryer
  shape; exe handler coverage closed in §10). Gateway TCP `15010`
  stays the only hardcoded TCP port (file `0x2513d8`, ref file
  `0x1ab72a`).
- `CUDPFailTCPConnect` trigger site (RTTI name only, file
  `0x28060c`); byte-pinned fallback behavior is relay op 3 forcing
  TCP plus the RUDP trio — the class's own call sites are open.
- Closed: op-6/7 `autoplay` is never 0 on client send paths (five
  dword-1 stores across the dispatcher and both fanout wrappers, no
  zero-store; §6). Closed: combo client `code` is 0 (no nonzero
  `code`/`unused` writer on any combo send path; §11,
  `ev_e4c4a65ec986bea1dc13e28cbcf9d36c`). Still open: which caller selector (1–4) reaches
  which dispatcher branch (grade mapping), and what `code`/`unused`
  carry on server-rewritten S→C copies (no nonzero writer located
  client-side; needs a wire capture, plus a Ghidra function dossier
  for VAs `0x56ddf0`, `0x4ea640`, `0x58ea9a` before any field rename).
- S→C game 4015 handler (peer lifebar store); see
  `docs/4015-hunt.md`.

## 9. UDP ports and UHP hole-punch flow (7.55 static map)

No new REA dossier was issued (shared snapshot contended); all
addresses below are file offsets (VA = file + `0x400000`), and no
disassembly is reproduced. Decoder impact: `ReportUdpPunchHole` / `ReportUdpPunchHoleAnswer` /
`ConfirmRelay` / `ConfirmRelayAnswer` empty bodies in `o2jam-net`
(`live_handshake_tests`); all four are byte-verified in-exe (§7
closed for 4055, §9). Method for the new rows: `python3` byte scans over
the local install-root copy (same method as the 4015 dossier),
cross-checked against the Memoryer `PlayingController` handlers.

Flow in the exe's own words: the relay selector stages the three
hardcoded relay IPs, picks one IP plus port 39311/39312 by selector,
then the UHP controller logs `UHP Start`, polls with `Sleep(300)`
until its status check passes, logs `UDP Hole Punching Success~!!`,
and reports each outcome through `InsertUHPResult` with
`(result, IP, port)`. If that path fails, the `CUDPFailTCPConnect`
class name marks the TCP-fallback side, whose wire-visible half is
relay op 3 (`PeerEndpointAssigned` forces TCP relay instead of
peer-to-peer) and the RUDP-over-UDP vs framed-TCP split.

| Item | Value / anchor |
|---|---|---|
| Relay UDP ports | 39311 (`0x998f`), 39312 (`0x9990`): selector-leaf immediates, file `0x18c73b`–`0x18c77f` |
| Relay TCP ports | none hardcoded (negative `u16` scan, `ev_6e655b70da829a83f2b6d07105cb025c`): `0x998f` exactly at the 3 selector leaves, `0x9990` port-word stores only at the 2 matching leaves; TCP peer-leg ports arrive via `GetLiveState` 4042/4043 / `GetP2PList` 4044/4045 |
| Relay IP staging | `0x64e260`/`0x64e250`/`0x64e240` pushed at file `0x18c6d7`/`0x18c6de`/`0x18c6e5`; dispatch tables file `0x18c850`/`0x18c864` |
| UHP start / success | branch file ~`0x18d7c5`; `UHP Start` file `0x24e2b0` (ref `0x18d80f`), `UDP Hole Punching Success~!!` file `0x24e2c0` (ref `0x18d7cd`); poll loop `Sleep(300)` via IAT `0x63f298` (site file `0x18d84d`) |
| UHP result | `InsertUHPResult` file `0x24e1e4` (ref `0x187a8c`); format `Hole punching Result : %d : IP : %d - Port : %d` file `0x24e208` (ref `0x187a82`) |
| UHP report 4046 C→S | empty (`04 00 CE 0F`): id `u16` `0x0FCE` at file `0x18d8a1`, len `u16` 4 at file `0x18d8ac`, adjacent on the stack in the UHP function (entry file `0x18d580`); id immediate unique image-wide; no `0x0FCE` dword site exists anywhere |
| Punch echo 4047 S→C | body-ignored: dispatcher slot 21 (case file `0x1587cb`) → handler entry file `0x163d90` sets a session flag and returns without reading packet bytes; that flag gates the 4048 send (checked at file `0xeae6d`); no `0x0FCF` dword site exists |
| Relay confirm 4055 C→S | byte-verified empty (`04 00 D7 0F`): send `push 0` + `push 0x0FD7` at file `0xf0439` (immediate at file `0xf043a`, byte `0x68` before) via session send vtable +`0x84`; sole `0x0FD7` dword site image-wide, zero `66 C7 .. D7 0F` word-store / `66 3D D7 0F` / `66 81 .. D7 0F` sites; fires 25 bytes after the 4012 send at file `0xf0420` with no field-staging immediates between, while the builder switch holds dword cases for 4016 (files `0xdd4c0`/`0xdd4cc`), 4012 (file `0xdd51d`), 4014 (file `0xdd529`) and none for `0x0FD7` |
| Relay echo 4056 S→C | body-ignored: slot 26 (case file `0x158820`) → handler entry file `0x163d50` forwards `(0, 0, 0x0FD8, 0x12)` to the UI event queue; same shape as the 4052/4054 forwarders (entries `0x163d10`/`0x163d30`); game-state compare against `0x0FD8` at file `0xde4db` |
| Adjunct `.52` host | string file `0x24e368`, pushed at files `0x1936e2`/`0x1936fc` inside helper file `0x193650` (7 call sites, §6); co-located number 7788 (file `0x19370b`); no packet shape — adjunct endpoint, exact role open |
| Fallback | RTTI `.?AVCUDPFailTCPConnect@@` file `0x28060c`; `WSASendTo`/`WSARecvFrom` IAT `0x63f584`/`0x63f580` (sites files `0x1b1ad2`/`0x1b19cd`) |

## 8. Tests (`cargo test -p o2jam-net`)

`relay_command_tests`: id map, role map, relay UDP-port immediates,
  exact-body vectors for
ops 1–7/10–11, frame round-trips, relay op-5 TCP header-shape
vector (`42 00 05 00` + 62 B = 66 B frame), tally-vs-4014-tick
separation vector (62 B vs 16 B bodies, 66 B vs 20 B frames, 16 B
body rejected as tally with `need 62`), hardcoded-ports-are-UDP-only
lock (ports differ from gateway `15010` and adjunct `7788`),
short/trailing/foreign-id and
bad-enum rejections, per-field offset vectors for the op-5 tally (15
fields) and the op-6/7 nonzero fields (code at 0, autoplay at 24, role
at 28) with unknown-role rejections. `relay_enum_value_tests`: full value tables for `RoomState` (0–2), `Difficulty` (0–3), `GameSpeed` (0–11) plus out-of-range rejections (Memoryer values locked; exe consumers §11). `adjunct_host_tests`: adjunct address octets,
relay-subnet-but-not-pool membership, `7788` immediate. `live_handshake_tests`: exact empty frames
`04 00 <id>` for 4046/4047/4055/4056 (the 4046 bytes match the UHP
stack frame; the 4055 send site is the sole `0x0FD7` dword image-wide),
round-trips, payload and foreign-id rejections, plus 4055 shaped-length
(5/16/24 B) rejections separating it from 4012/4014/4016. `udp_frame_tests`: ack encode/decode,
stored/fixed-dictionary/dynamic-Huffman datagram vectors,
stored-encoder round-trip, length-prefix and checksum
rejections. `stored_block_tests` (`inflate.rs`): Adler-32
reference, stored round-trips incl. empty, complement/checksum/
truncation rejections.
`live_state_tests`: exact empty frames `04 00 CA 0F` (4042) and
`04 00 CC 0F` (4044), empty round-trips, payload and foreign-id
rejections; 4045 exact 36-byte two-peer vector (`i32` count + `i32`
member + 6-byte endpoints, Python `struct` cross-checked), empty and
negative counts (negative yields zero peers, matching the signed loop
exit), truncation/trailing/foreign-id rejections, per-field offset
vectors; 4043 exact 17-byte all-inactive vector plus the 124-byte active-member vector (Python `struct` cross-checked), active-gate
(non-1 bytes mean inactive), seat-byte wire order, single room-master trailing flag (no administrator byte),
two-consecutive-active alignment, member-id bound, negative music-count,
trailing/foreign-id rejections, header offset vectors; `member_slot_tests`: slot base/stride, seat/equipment/music/trailing/room-master/spare-flag/match-index/slot-group offsets, slot-group ordering, `slot_position_marks` for slots 0/3/other, match-index sentinel plus stride stepping, slot/offset bounds.

## 10. Live-state gateway handlers: 4042–4045 (7.55 static map)

Game-gateway (not relay-TCP) messages that deliver the TCP peer-leg
ports: the client sends empty `GetLiveState` 4042 / `GetP2PList` 4044
and the server answers `LiveStateResponse` 4043 / `P2PListResponse`
4045. Decoders live in `o2jam-net` (`relay.rs`, beside the
4046/4047/4055/4056 handshake types). Static analysis only over the
local install-root copy (read-only, VA = file + `0x400000`); behavior
described in own words with file offsets plus local-static record ids
(truncated sha256 of the record tag). Tools: `python3` byte scans plus
Capstone disassembly; no disassembly is reproduced. Memoryer v7.2.0
(`memoryer-local` binaries) corroborates the enum ids and the
`LiveStateResponse` / `P2PListResponse` field orders (incl. the
`Int32`-coded peer member and the `Int32`-prefixed peer list); it has no
request classes for 4042/4044, consistent with the empty sends below.

| Packet | Body shape (bytes after the frame header) | Exe anchor |
|---|---|---|
| `GetLiveState` 4042 C→S | empty (`04 00 CA 0F`) | sole `0x0FCA` push-immediate at file `0xc365c` (`push 0` + `push 0x0FCA`, session send vtable `+0x5c`), gated on the global byte at `0x7789e6` equal to 1; no field staging (`ev_f133f3bf30eb77f74ddc57f2c81d8783`, observed) |
| `GetP2PList` 4044 C→S | empty (`04 00 CC 0F`) | sole `0x0FCC` push-immediate at file `0xead18` (session send vtable `+0x84`), gated on the Live-mode byte at `0x77cd33` equal to 1 plus a session once-flag at `+0x89c` (set to 1 right after the send) (`ev_70b81e37e5e8d6f15ec316c1b93b3947`, observed) |
| `LiveStateResponse` 4043 S→C | `u8` member id (< 8) + `i32` users + `i32` streak + 8 fixed slots, each `u8` active + `MemberInfo` tail iff active (`== 1`) | dispatcher slot 19 → case file `0x1587a9` → handler entry file `0x159b50`: header reads 1/4/4 through the shared reader (files `0x159b5d`/`0x159b6b`/`0x159b79`), member `>= 8` takes the error path at file `0x159fc2` (returns without forwarding), 8-slot loop bound at file `0x159cf9`, per-slot active read + `!= 1` skip at files `0x159d0a`/`0x159d24`, completion forwards `(0, 0, 0x0FCB, 0x12)` to the UI queue at file `0x159fb0` (same shape as the 4056 forwarder; `ev_55248e1f2c72c6ef9d26603f94925bf2`, observed) (`ev_83bc05cdd0c1b7b5fb9313f9a1f485e6`, `ev_5396edb7c26f6ac2aa78c743fff3905f`, observed) |
| `P2PListResponse` 4045 S→C | `i32` count + N × (`i32` member + 6-byte public + 6-byte local; 4-byte IP + `u16` port) | dispatcher slot 20 → case file `0x1587ba` → handler entry file `0x163d70` → relay-side parser at file `0x18cf50`: count read-4 (file `0x18cf94`), signed loop bound (file `0x18cfb7`), per-peer read-4/6/6 (files `0x18cfc5`/`0x18cfd3`/`0x18cfe1`), endpoint dword-plus-word staging (files `0x18cfeb`/`0x18cff0`, `0x18cffd`/`0x18d002`) into the per-peer record call at file `0x18d00a` (`ev_0a144946a7ac801729031065ad78008d`, observed) |

Push-immediate scan (`ev_dc7171af6c958b25a6b37cc34a1cb0ad`, observed):
`0x0FCA` and `0x0FCC` each have exactly one `push`-immediate site (the
two sends above); `0x0FCB` has exactly one (the 4043 completion
forwarder); `0x0FCD` has no `push`-immediate or word-compare site
image-wide, so 4045 is S→C-only. 4042/4044 map to the dispatcher's
default slot 28 (no S→C handler), consistent with C→S-only use.

`MemberInfo` wire order is the handler's read order through the shared
1/4/6-byte reader (sub-decoders at files `0x140bd0`–`0x141690`;
`ev_e781f2560f769dc9425a997e25f263dd`, observed): id, null-terminated
nickname (reader at file `0x155ea0` measures to the terminator, then
consumes it), level, gender byte (raw: only a zero/nonzero branch
observed, file `0x13fe50`), gem, three seat bytes, 17 `i32` equipment
words (count read plus a `0x11`-bounded 16-read loop), `i32` count plus
`u16` music ids, two `i32` counters, trailing bytes `playing_state`,
`live_extra`, `is_room_master`, `i32` win streak
(read inline at file `0x159f61`). The wire carries exactly three trailing
bytes (the `+0x33` reader at file `0x140d90` has zero image-wide callers,
so no fourth flag exists; the sibling lead-flag shape reads the same
`+0x30` flag ahead of the seat bytes instead — see the table). The exe reads one slot byte twice in
a row (files `0x141670`/`0x141690`), so the first copy is discarded
in-memory; the decoder preserves that wire byte as `live_extra`, which
has no Memoryer v7.2.0 counterpart (likely 7.55 drift) and whose
semantics stay OPEN, as do all enum value sets. Negative counts decode
as empty (the exe's signed exit tests skip the loops: 4045 file
`0x18cfb7`, music ids with the same shape), matching the client.

### `MemberInfo` to member-slot map (field-to-offset only, 7.55 static map)

Each active 4043 slot stores into the 8592-byte (`0x2190`-stride) member
table at VA `0x77ee90` (`slot * 0x2190 + base`; `imul $0x2190` + `add
$0x77ee90` repeats at files `0x159d38`–`0x159f82`). Static only over
the local install-root copy (read-only, VA = file + `0x400000`); behavior
described in own words with file offsets plus local-static record ids
(sha256 of the record tag, truncated to 32 hex), no disassembly
reproduced. Tools: `python3` byte scans plus `objdump -d` slices (same
method as the 4015 hunt). No new REA dossier was issued. Reader site =
the 4043 call site that stores the field; helper = the small writer it
calls (offset in the helper name). Decoded in `o2jam-net` as
`MEMBER_SLOT_BASE`/`MEMBER_SLOT_STRIDE` plus `MEMBER_OFF_*` and the
`member_slot_address` reader (`member_slot_tests`).

| `MemberInfo` field | Slot offset | Reader site (call file → helper file, size) |
|---|---|---|
| id | `+0x04` | `0x159d44` → `0x140bd0` (1 B) |
| nickname | `+0x05` | `0x159d5c` → `0x140bf0` (NUL-terminated via `0x155ea0`) |
| level | `+0x20` | `0x159d74` → `0x140c50` (4 B) |
| gender | `+0x28` | `0x159d8c` → `0x140c90` (1 B) then setter `0x13fe50` (zero/nonzero) |
| gem | `+0x2c` | `0x159da4` → `0x140cc0` (4 B) |
| seats: team | `+0x31` | `0x159dbc` → `0x140d40` (1 B) |
| seats: ready (wire store, derived in memory) | `+0x32` | `0x159dd4` → `0x140d70` (1 B); the slot-index branch then overwrites `+0x32` from position without reading wire (setter `0x1400f0`: slot 0 stores 1 at file `0x159f00`, slot 3 stores 0 at file `0x159f2e`, other slots store 1 at file `0x159f5c`; `slot_position_marks` in `relay.rs`) |
| seats: third byte (`music_state` position) | `+0x217c` | `0x159dec` → `0x141480` (1 B); value semantics OPEN: no compare/branch consumer image-wide — only the direct setter `0x141460` (27 callers, incl. UI-forward paths for codes `0x0FC3`/`0x0FC5` at files `0x159943`/`0x15995e`/`0x1599d2`/`0x1599ed`), the ctor default `+0x217c = 0` beside `+0x217d = 0xFF` (files `0x13fcc1`/`0x13fcc8`), and the bulk copy (files `0xe961d`/`0xe9623`); `ev_f11710744bbbb8279df245ae2e46aeac` |
| match index (derived, not wire) | `+0x217d` | wire-order position `0..7`, `0xFF` = unmatched (decoder `MEMBER_OFF_MATCH_INDEX` / `MEMBER_MATCH_INDEX_UNSET`): ctor defaults `+0x217d = 0xFF` beside `+0x217c = 0` (files `0x13fcc1`/`0x13fcc8`); reset routine entered at file `0xdb6e0` restores `0xFF` to all eight slots (call site file `0xdb8c6`) plus the local object; the S→C 8-by-8 id-match routine at file `0x159a60` (stub file `0x15879c`; local store file `0x159acf`, slot store file `0x159b1a`) and the room-dispatcher routine entered at file `0x15fa20` (stub file `0x1586bf`; local store file `0x15fb34`, slot store file `0x15fb7e`) each compare the just-read id byte against the requesting id at `0x77cd04` or the slot id at `+0x04` and store the outer index on match. Direct setter `0x1414a0` has exactly six image-wide callers (the two stores in each of the three routines); the only other `0x217d` sites are the ctor default and the bulk-copy pair (files `0xe962f`/`0xe9635`); no compare/branch on `+0x217d` exists image-wide, so what the index selects stays OPEN; `ev_ef517890c33d30e9f73e4d968f15c740`, `ev_81b04a61265048cdf391244aeff6beac`, `ev_15e8653ce8cffef7c50fec80b16e4b14`, observed |
| equipments count | `+0x2038` | `0x159e30` → `0x140e40` (4 B; `0x11` loop bound file `0x140eb1`) |
| equipments | `+0x2040` | same call; 17 `i32` words |
| music ids count | `+0x34` | `0x159e48` → `0x140db0` (4 B; loop bound is the count) |
| music ids | `+0x38` | same call; entries split low `+0x38` / high `+0x1038` |
| states: cash_point | `+0x2128` | `0x159e60` → `0x140ce0` (4 B) |
| states: free_pass | `+0x212c` | `0x159e78` → `0x1415a0` (4 B) |
| states: playing_state | `+0x2130` | `0x159e90` → `0x141670` (1 B) |
| states: live_extra (wire copy) | `+0x2131` | `0x159ea8` → `0x141690` (1 B); the second of two back-to-back 1 B reads (files `0x141670`/`0x141690` into `+0x2130`/`+0x2131`); semantics OPEN |
| states: trailing flag (`is_room_master`) | `+0x30` | `0x159ec0` → `0x140d20` (1 B). The only trailing flag on the wire. Room-master (not administrator): the 4043 header derives the local object (`0x77cd00`) from the requesting member id at `0x77cd04`, storing `+0x30 = 1` exactly when the id is 0 (files `0x159bdf`/`0x159c1b`/`0x159c4d`, plus the `0x77ee3c = 1` mark at file `0x159be4`); the sibling handler compares the member id against the announced master id at `0x778a14` and marks the local `+0x30` when equal, the announced slot otherwise (files `0x15d888`–`0x15d8d3`); `ev_bc4842b1a6fa649bb1a62f21200a3a30` |
| states: spare low flag (never wire) | `+0x33` | local-only effect selector holding `0..8` (decoder `MEMBER_OFF_SPARE_FLAG`): zeroed up front via the direct setter `0x140110` (file `0x159e18`, pushed 0); no wire reader in any of the three member-parse paths (helper `0x140d90` has zero image-wide callers; full-image relative-call scan: 49 callers of the setter — mostly immediates `0..8` to the local object at `0x77cd00`, one target `0x78fb10` — against zero of the reader). The lone consumer is the ticker helper file `0x141620` (six callers in the score region: files `0x16e5c1`/`0x16e73e`/`0x176140`/`0x176d86`/`0x177230`/`0x178200`), which skips the `+0x2158 += 10` bonus when `+0x33 == 6` yet always applies `+0x2152 += 1`; `ev_c030b2551aa535f98e016858e180ae73`, `ev_c18b6a071288740ad1166134e7fc92a6`, `ev_261849e6804de5f8d1b476fbe9d0fb55`, `ev_d6fa2e234044d3517f0c887799e19eda`, observed |
| slot group (derived, not wire) | `+0x2186` | slot-index branch files `0x159ec5`–`0x159f5c` via setter `0x141520` (files `0x159eea`/`0x159f18`/`0x159f46`): slot 0 stores 1, slot 3 stores 2, others store 3 (`slot_position_marks` in `relay.rs`). The 4043 header runs the same 0/3/other mapping over the member id for the local object (files `0x159baf`–`0x159c5e`, plus `+0x2184` via `0x141500`); ctor default is 3 (file `0x13fa96`); `ev_14fd3f2dae86c90bf45f505c7c7a70ef` |
| streak: win_streak | `+0x218c` | inline 4 B read file `0x159f61` → setter `0x141560` at file `0x159f82`; `0x218c + 4` ends exactly at the slot end `0x2190` |

Header (not per-slot): `u8` member id + `i32` users + `i32` streak via
the shared reader (files `0x159b5d`/`0x159b6b`/`0x159b79`);
`ev_5a25bf3fec517df32dde42e275128074`, observed. Active-tail stores
above: `ev_7737f29fa5a4df1f2270ce72df7b6d54`; equipment/music stores:
`ev_54701cf711eebb93adbcbd632629542c`; trailing stores:
`ev_20f7c72d0d3f156c51ba3c3a0417f774` (all observed). CLOSED: the wire
carries one trailing flag (`+0x30` = room-master; the decoder's former
`is_administrator` byte had no wire source and is removed) — full-image
relative-call scan finds zero callers of the `+0x33` reader `0x140d90`
against three callers of the `+0x30` reader `0x140d20` (files `0x159ec0` /
`0x15cf0d` / `0x15dec3`); CLOSED: the slot-index branch derivation (slot
0 → `+0x2186` = 1 / `+0x32` = 1, slot 3 → 2 / 0, else 3 / 1, no wire
reads) and its header twin over the member id; CLOSED: `+0x30`/`+0x33`
are zeroed through the direct setters `0x1400b0`/`0x140110` (files
`0x159e02`/`0x159e18`, pushed 0) before the equipment/music reads, and
only `+0x30` is re-populated from wire. CLOSED: sibling command binding —
room dispatcher base `0x0BBB` (3003), bound `0x0B`, dword table file
`0x1589c4`; lead-flag is id 3003 (stub file `0x1585ab` → handler entry
file `0x15cad0`, sole caller file `0x1585af`;
`ev_a83aada10d256cb689110409dc89c067`, observed) and trailing-flag is id
3007 (stub file `0x1585de` → entry file `0x15d820`, sole caller file
`0x1585e2`; `ev_50242b6c183543f49ce6cbb43eccad2e`, observed). CLOSED:
lead order (`+0x30` at file `0x15cf0d` ahead of the seat readers at files
`0x15cf26`/`0x15cf3f`/`0x15cf58`;
`ev_f1ed2d82f90e372666e2a01a66831d6f`, observed) and the `+0x2134`
dword (4 B read through the shared reader file `0x155d20` at site file
`0x15d009`, pushed 4, stored into the slot at file `0x15d03c`;
`ev_bf2aa97d2356819f76df5ea05e24cfc4`, observed; decoder
`MEMBER_OFF_EXTRA_DWORD`). CLOSED: `+0x33` is a local-only effect
selector (`0..8`, never wire) — 49 setter callers against zero reader
callers, lone ticker consumer branching on 6 (`+0x2158 += 10` skipped,
`+0x2152 += 1` always). CLOSED: `+0x217d` storage is the wire-order
match index (`0..7`, `0xFF` = unmatched) — ctor default, eight-slot
reset, and two 8-by-8 id-match routines through the six-caller setter,
with no compare consumer. OPEN: seat-byte and trailing-byte
value semantics (team/ready/music/playing/free-pass enums, `live_extra`,
`+0x2134`, `+0x2186` group meaning, what the `+0x217d` index selects —
none has an in-exe compare consumer);
the full 3003/3007 body layouts beyond the pinned order/offset.

## 11. Relay enum value sets (7.55 static map)

Widths were already pinned (`u16` role, `u8` lifecycle/difficulty/speed,
`i32` code: the fanout wrappers stage routing/autoplay/role through the
`0xeb4d0`/`0xeb570`/`0xeb550` helpers with pushed sizes `0x18`/`0x04`/
`0x02`, matching the 24/4/2 wire lengths, §6). This section adds the
branch-or-table consumer each value needs. Static only over the local
install-root copy (read-only, VA = file + `0x400000`); behavior described
in own words with file offsets plus local-static record ids (sha256 of the
record tag, truncated to 32 hex). Tools: `python3` byte scans plus
Capstone disassembly; no disassembly is reproduced.

| Enum | Values (pinned) | Exe consumer |
|---|---|---|
| `MemberRole` (`role`, ops 5/6/7) | 0 None, 1 Champion, 2 Challenger, 3 Spectator | Role word VA `0x77ee86`: `66 A1 86 EE 77 00` loads at files `0xaec29`/`0xc33b4`/`0xdbb1c`/`0xe6f81`/`0xe7a75`/`0xe840a`/`0xea26e` (`ev_acacd596017f22ba805e419c18cfe686`, observed); `83 F9 03` Spectator gates at files `0xaec3d`/`0xc33c2`/`0xe7a89`/`0xe841e` (`ev_cbe189ba7fb55898e42e98c6175b8e75`, observed); `83 7D 8C 01`/`83 7D 8C 02` Champion/Challenger dispatch with else fallthrough at files `0xdbb2d`/`0xdbb33` (`ev_6f91b9283627171b33801a2c0e653100`, observed); `83 F9 01` then `83 F8 02` at files `0xe6f95`/`0xe6faf` (`ev_e5879e64727f6c4c5ab0759b929433c8`, observed); 0 is the residual None; `0xea26e` (op-5 packer) copies without branching. |
| Combo `code` (preface, ops 1–7) | 0 = client send (only client-sent value) | No nonzero `code`/`unused` writer on any combo send path (§6); client combo sends carry both as 0 (`ev_e4c4a65ec986bea1dc13e28cbcf9d36c`, observed). Server-rewritten S→C values OPEN (needs a wire capture). |
| `RoomState` (`rooms.rs`, `u8`) | 0 Unavailable, 1 Waiting, 2 Playing (Memoryer `Encore.Metadata.RoomState`) | OPEN: values locked by `relay_enum_value_tests`; no dedicated branch/table consumer located in the relay/live regions. Nearest context: the `0xe6xxx`–`0xe8xxx` Live-result UI holds the role-gated art pushes at files `0xe66cc`/`0xe671e`; needs a Ghidra dossier before attributing any state compare. |
| `Difficulty` (`rooms.rs`, `u8`) | 0 Ex, 1 Nx, 2 Hx, 3 Rx (Memoryer `Encore.Metadata.Difficulty`) | OPEN: values locked by `relay_enum_value_tests`; `83 F8`/`83 F9 03` compares occur at 31 sites image-wide each with none attributed to the difficulty byte yet. Per-difficulty life/score deltas stay OPEN (see `docs/gameplay.md`). |
| `GameSpeed` (`rooms.rs`, `u8`) | 0 X05, 1 X10, 2 X15, 3 X20, 4 X25, 5 X30, 6 X35, 7 X40, 8 X45, 9 X50, 10 X60, 11 Xr (Memoryer `Mozart.Metadata.GameSpeed` fork ordinal) | OPEN: values locked by `relay_enum_value_tests`. One lead: `83 F9 0B` (`cmp ecx, 11`, the 0–11 upper bound) at file `0x1c20f7` follows a function-prologue load from `[eax + 0x16bc]`; the six other `ecx, 11` sites (`0x9af0e`/`0x9c4c0`/`0xc436c`/`0x12c718`/`0x1c2189`/`0x208e0f`) and the four `eax, 11` sites show no room/speed context. No 12-entry speed table located; needs a Ghidra dossier for VA `0x5c20f7` before claiming (`ev_0e43412d808afbe3ec14be719f45a098`, observed). |

Tests: `relay_enum_value_tests` in `relay.rs` asserts every value above
(plus out-of-range rejections); `relay_command_tests` keeps the
per-field offset vectors and unknown-role rejections.

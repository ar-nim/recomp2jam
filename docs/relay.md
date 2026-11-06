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

## 2. Relay TCP framing

Same `CPacket_Str` framing as the game gateway (`[u16 len][u16
id][body]`, `len` counts the whole frame, id at buffer offset 2):
the relay ctors call the same `SetID`, and Memoryer's relay stack
runs on the same Encore `TcpServer` message codec. Exe-side frame
bytes for relay traffic are still open (only the `SetID` writes
are pinned).

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
Challenger, 3 Spectator (`Encore.Metadata.MemberRole`).

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
- Relay TCP framing bytes (same-`SetID` inference +
  same-codec corroboration, not a capture).
- Relay TCP ports for the peer leg (only gateway TCP `15010` pinned
  in-exe at file `0x2513d8`, ref file `0x1ab72a`); relay ports otherwise
  arrive via `GetLiveState` 4042/4043, `GetP2PList` 4044/4045.
- `CUDPFailTCPConnect` trigger site (RTTI name only, file
  `0x28060c`); byte-pinned fallback behavior is relay op 3 forcing
  TCP plus the RUDP trio — the class's own call sites are open.
- Closed: op-6/7 `autoplay` is never 0 on client send paths (five
  dword-1 stores across the dispatcher and both fanout wrappers, no
  zero-store; §6). Still open: which caller selector (1–4) reaches
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
ops 1–7/10–11, frame round-trips, short/trailing/foreign-id and
bad-enum rejections, per-field offset vectors for the op-5 tally (15
fields) and the op-6/7 nonzero fields (code at 0, autoplay at 24, role
at 28) with unknown-role rejections. `adjunct_host_tests`: adjunct address octets,
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

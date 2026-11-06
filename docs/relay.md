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
  `autoplay` as `u32` (only 0/1 accepted; hardcoded to 1 when
  sent) + `role` `u16`.
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
  has exactly two code refs (files `0x1936e2`, `0x1936fc`) in a
  setup block that also materializes constant `7788` (`$0x1e6c`,
  file `0x19370b`, the only `0x1e6c` in the image) — role open, not
  claimed as a port.
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
- `ConfirmRelay` 4055 C→S body bytes: send site pinned (file
  `0xf0439`) but the shared send helper also serves field-carrying
  opcodes (4012 at files `0xf0420`/`0xeadcc`, 4014, 4016, 4048), so
  the site does not prove emptiness; the decoder encodes empty per
  Memoryer (`PlayingController.ConfirmRelay` takes no fields) until
  byte proof lands. 4046/4047/4056 are byte-verified empty (§9).
- `CUDPFailTCPConnect` trigger site (RTTI name only, file
  `0x28060c`); byte-pinned fallback behavior is relay op 3 forcing
  TCP plus the RUDP trio — the class's own call sites are open.
- Whether op-6/7 `autoplay` is ever 0, and what `code`/`unused`
  carry on the wire when nonzero.
- S→C game 4015 handler (peer lifebar store); see
  `docs/4015-hunt.md`.

## 9. UDP ports and UHP hole-punch flow (7.55 static map)

No new REA dossier was issued (shared snapshot contended); all
addresses below are file offsets (VA = file + `0x400000`), and no
disassembly is reproduced. Decoder impact: `ReportUdpPunchHole` / `ReportUdpPunchHoleAnswer` /
`ConfirmRelay` / `ConfirmRelayAnswer` empty bodies in `o2jam-net`
(`live_handshake_tests`); 4046/4047/4056 are byte-verified in-exe
while 4055 stays Memoryer-corroborated with its send site pinned
(body OPEN, §7). Method for the new rows: `python3` byte scans over
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
| Relay confirm 4055 C→S | send site file `0xf0439` (`push 0`, `push 0x0FD7`, session send via vtable +`0x84`); sole `0x0FD7` dword site image-wide; body OPEN — same shape serves field-carrying 4012/4014/4016/4048 |
| Relay echo 4056 S→C | body-ignored: slot 26 (case file `0x158820`) → handler entry file `0x163d50` forwards `(0, 0, 0x0FD8, 0x12)` to the UI event queue; same shape as the 4052/4054 forwarders (entries `0x163d10`/`0x163d30`); game-state compare against `0x0FD8` at file `0xde4db` |
| Adjunct `.52` host | string file `0x24e368`, code refs files `0x1936e2`/`0x1936fc`; co-located constant 7788 (file `0x19370b`) — role open |
| Fallback | RTTI `.?AVCUDPFailTCPConnect@@` file `0x28060c`; `WSASendTo`/`WSARecvFrom` IAT `0x63f584`/`0x63f580` (sites files `0x1b1ad2`/`0x1b19cd`) |

## 8. Tests (`cargo test -p o2jam-net`)

`relay_command_tests`: id map, role map, relay UDP-port immediates,
  exact-body vectors for
ops 1–7/10–11, frame round-trips, short/trailing/foreign-id and
bad-enum rejections. `live_handshake_tests`: exact empty frames
`04 00 <id>` for 4046/4047/4055/4056 (the 4046 bytes match the UHP
stack frame), round-trips, payload and foreign-id rejections. `udp_frame_tests`: ack encode/decode,
stored/fixed-dictionary/dynamic-Huffman datagram vectors,
stored-encoder round-trip, length-prefix and checksum
rejections. `stored_block_tests` (`inflate.rs`): Adler-32
reference, stored round-trips incl. empty, complement/checksum/
truncation rejections.

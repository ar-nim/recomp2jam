# Send moments by state (7.55) — moment-table method

Version pin for every EXE row: `OTwo.exe` / `AngelO2Jam.exe`, 5,029,888 B,
SHA-256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
File offset = VA − `0x400000`. Method: the moment-table template from
`docs/music-availability.md` §2 — name the send site, name the event or
control that reaches it, name the gate and body source, and label any
remaining gap OPEN with a next evidence step. Static sites use byte scans
and `objdump` slices over a read-only 7.55 copy; live rows name the tap.
No disassembly or original bytes are reproduced.

Scope: every game-TCP or relay C→S send whose source has a documented site,
live capture, or opcode-family pin, grouped by the state roster in
`docs/lifecycle.md`. The 4025 slice stays in `docs/music-availability.md` §2;
it is referenced, not copied. This document does not claim an exe file
offset where none exists: those rows are OPEN.

Status legend: **PINNED** = exe site plus reachable moment; **LIVE** = a
captured frame plus its scene moment (site may remain OPEN); **PARTIAL** =
exe site or wire body known, moment narrowed to a branch but not captured;
**OPEN** = no pinned moment, with a next step. A row with no send is
written as such. Every audited send has a moment cell; no audited send is
left blind or missing, because an unknown moment is an OPEN row with a
next evidence step, not an omission.

| State / surface | Section |
|---|---|
| Boot / session init | §1 |
| Avi splash | §2 |
| Planet / Channel (incl. ranking dialog) | §3 |
| Room | §4 |
| Waiting (normal + live) | §5 |
| Loading gate | §6 |
| Playing | §7 |
| Result | §8 |
| Item Shop | §9 |
| Music Shop | §10 |
| MyRoom | §11 |
| Gift box / equip / cash | §12 |
| Payment / Payment2 (web) | §13 |
| Tutorial / Lanking / Messenger / Drum / Exit | §14 |

## 1. Boot / session init

| Op | Moment (event → send) | Site / gate | Status; next step if OPEN |
|---|---|---|---|
| 1000 Authorize | Launch blob (`USER_INDEX_ID`, token, gender) parsed; first gateway frame of the session | live tap 2026-11-18 (`scripts/rig-wine-run.sh --wire-tap`, `docs/net-protocol.md` §4c), body 96 B; static call site OPEN | LIVE. Next: name the builder call site in the net-init district. |
| 1002 GetChannelList | After authorize, to populate the Planet/Channel dialog rows | live sequence authorize → 1002/1003 (`docs/net-protocol.md`, `docs/wine-boot.md`); site OPEN | LIVE. Next: one tap with the exact send file offset. |
| 1007 SessionKeys | Session-key exchange in the planet/login family after channel enter | opcode family in `docs/net-protocol.md` §4a; no site or captured moment | OPEN. Next: tap a boot exchange and name the trigger frame. |
| 2024 SendMusicList | Installed-catalog announce after the `Music\*.ojn` scan; id high nibble carries the installed flag | codec `o2jam-net::SendMusicList` at `crates/o2jam-net/src/message.rs:132`; opcode `0x07e8` in `crates/o2jam-net/src/command.rs`; moment not captured | PARTIAL. Next: one wired boot tap naming when the scan result is sent. |
| 1004 ChannelLogin | Channel entry-face click on an active row; the inactive row calls the "cannot enter channel" box instead | active-byte read file `0x75345`, opcode `0x3ec` pushed by the standard send path (`docs/net-protocol.md` §4c) | PINNED |
| 2000 / 2002 | Channel enter completed (1005), then character and channel-info fetch | live sequence 1005 → 2001/2002/2026/2003 (`docs/net-protocol.md` §4c, `docs/planet.md`) | LIVE. Next: file offsets for both sends. |
| 2010 GetUserList | One Room side-entry fuse per Room visit; a processed 07DB or starved overdue sheet latches it | guarded by state global, at most one 07DA per visit; guard `side_entry_sends_exactly_one_user_list_pair_on_the_scripted_gateway` (`docs/net-protocol.md` §4b) | PINNED (wire + gate). Static send site OPEN: next, one stopped-state read. |
| 65520 Terminate | Quit-confirm / modal OK and teardown; Wine boot logs the self-send on modal dismissal | live Wine logs (`docs/wine-boot.md` §wine-boot); site OPEN | LIVE. Next: name the teardown send site. |

## 2. Avi splash

| Op | Moment | Site / gate | Status; next step |
|---|---|---|---|
| none | CR/ESC dismisses the logo to the bare Planet board; no game frame | handlers file `0xf3e70` / `0xf51f0` (`docs/back-navigation-735.md` §2) | PINNED negative (static). Next: one wire tap across the splash to confirm zero frames. |

## 3. Planet / Channel (channel dialog hosted by `CStatePlanet`)

| Op | Moment | Site / gate | Status; next step |
|---|---|---|---|
| 1004 | Active channel row pick; inactive row shows the notice, no send | file `0x75345` + `0x3ec` push (`docs/net-protocol.md` §4c); live `03EC/0FBE/07D0/07D2/13A4` on room entry (`docs/planet.md`, `docs/channel-fill.md`) | PINNED |
| 2021 ChannelLogout | Channel BACK control `0x13200002`; query gate on global `0x76835c` | exit handler file `0x48fd20`, sends files `0x48ff90`–`0x48ffa8`; live scripted-gateway leg (`docs/net-protocol.md` §4c) | PINNED |
| 2012 SendMainRoomMessage | Lobby slash-command parser, plain channel line (`/w`, `/r` choose whisper instead) | file `0x8fa1e`, opcode `0x7dc` (`docs/net-protocol.md` §4c) | PINNED |
| 2017 SendWhisper | `/w` recipient branch in the lobby parser, or the waiting-room twin | files `0x8f757`/`0x8fa02` (`docs/net-protocol.md` §4c) | PINNED |
| 2015 Announce | `/n ` admin notice branch, behind nonzero byte gate `0x768b6c` | files `0x8f585`/`0x8f5dc`, opcode `0x7df` (`docs/net-protocol.md` §4c) | PINNED |
| 2030 / 2032 | Ranking dialog fetch: 2030 when role word `0x77ee86` is 2 or own seat `0x77cd30` is set; both gated by one-way latch `0x779964` | files `0xac402`–`0xac43d`, `0xb2622`–`0xb265a`; 2032 via vtable `+0x64` files `0x4151a`–`0x41532` (`docs/overnight-evidence/u4-ranking-packs.md`) | PINNED (site + gate). Body of 2032 stays OPEN: next, proxy rewrite or stopped-state vtable read. |
| 2034 GetFreeMusicStatus | Free-music status request; trigger not located | opcode only in `docs/net-protocol.md` §4a | OPEN. Next: scan for the `0x7f2` send builder and tap the owning click. |
| 2027 GetMusicPlayRanking | Lanking song-to-board fetch | `docs/lanking.md:76`: the exe never sends 2027; a pick only names the `MUSICNAME` caption | PINNED negative (static + capture-backed). Next: one live replay before claiming any board-fetch frame. |

## 4. Room

| Op | Moment | Site / gate | Status; next step |
|---|---|---|---|
| 2004 CreateRoom | Create-room dialog confirmation; the client under test never emitted it in the qualified Wine boots, while a bot session proved the server accepts it | opcode-only here (`docs/wine-boot.md` §Attempt 10, `docs/claims.md` D rows) | OPEN. Next: wired tap on a CREATE OK press. |
| 3002 JoinWaiting | Room row double-click after channel enter | live tap `0BBA` (`docs/wine-boot.md`, `docs/net-protocol.md` §4b); our Room Confirm sends no 3010 (`docs/online-loop.md`) | LIVE. Next: exe send site. |
| 2010 | Room visit side-entry fuse (one pair per visit) | see §1 | PINNED |
| 5006 EnterShop | Room ITEMSHOP control `0x13200008`, immediately after the state pick | `FUN_0048fd20` file `0x8fd20` (`docs/shop.md` Entry/exit evidence) | PINNED (static). Next: one live entry tap. |
| BACK family | Room BACK control `0x13200002`; no direct opcode tap | `docs/back-navigation-735.md` §3 | OPEN. Next: scripted gateway tap on Room BACK and name the opcode. |

## 5. Waiting (`CNormalWaiting` / `CLiveWaiting`)

| Op | Moment | Site / gate | Status; next step |
|---|---|---|---|
| 4000 SetRoomMusic | Select-dialog row whose difficulty bit is set; also the master-only key-mode event `0xbb9` after comparing new/old mode bytes | select handlers VAs `0x4ad940`/`0x4b42d0`; key-mode send VA `0x4b10de` (`docs/parity/waiting.md:374`, `crates/o2jam-net/src/rooms/mod.rs:884` `KeyModeChanged`) | PINNED (moment), body of the key-mode arm OPEN: next, tap that arm. |
| 4002 SetRoomArena | Arena picker wrap arm stages the high-word flag and sends the room-mode word | files `0xbb600`/`0xbb64d`, receive echo handler file `0x15f3c0` (`docs/caption-sources.md`, `docs/net-protocol.md` §4c) | PINNED |
| 4006 SetInstrument | Instrument choice in Waiting | opcode in `docs/net-protocol.md` §4a; no send site pinned | OPEN. Next: sweep for the `0xfa6` push, then tap the owning control. |
| 4030 GetMusicList | Native-only confirm step in the Waiting song-flow sequence | `docs/captures/journey-audit/291-waiting.md:35` marks the 4000/4002/4030 → 4010/4011 → 4012 walk as native-only, not an exe claim | OPEN. Next: one native wire-log run with the exe send site pinned before any exe moment is asserted. |
| 4051 StartSelectMusic | MUSELECT pick: no start-select frame is emitted | `docs/captures/journey-audit/291-waiting.md:29` pins the negative on both sides; only 4000 follows a master row pick | PINNED negative. Next: scripted-gateway guard named in that matrix, to keep the absence explicit. |
| 4053 CancelSelectMusic | MUSELECT exit: no cancel-select frame is emitted | same negative row in `docs/captures/journey-audit/291-waiting.md:29` | PINNED negative. Next: same scripted-gateway guard, with 4053 absence logged separately. |
| 4004 → 4008(0) | Team buttons `0x16201004..0x0B`; press sends team then ready zero when the role gate is clear | files `0xb22d0`/`0x48c810`/`0x556680` (`docs/claims.md` B27) | PINNED |
| 4008 Ready | Guest readiness toggle; five sites push a zero word then `0xfa8` (no flag byte) | files `0xacebe`, `0xb24f6`, `0xb36a0`, `0xb8450`, `0xb871f` (`crates/o2jam-net/src/play.rs`) | PINNED |
| 4010 StartGame | START control `0x16202012`: gate verdict `1`; also the 4035 state-zero arm when master and gate are positive | `0xfaa` push in the start arm, gate VA `0x4b5560` (`docs/music-availability.md` §4a) | PINNED |
| 4034 AcquireMusicRequest | Same START control when the gate finds a peer music state `1`; no notice | `0xfc2` push file `0xb23d2` (`docs/music-availability.md` §4a) | PINNED |
| 4025 (8 sites) | Music changed / leave / skill / skill-ring events and the download-queue fallback | template table in `docs/music-availability.md` §2, files `0xab613`…`0xb6a9e` | PINNED (reference only) |
| 4036 / 4038 / 5012 | START download queue: 4036 on a non-empty transfer fallback, 4038 progress, 5012 after temp→final promote | `docs/music-availability.md` §4 (4038 sites files `0xba58f`/`0xba5b3`); 5012 site OPEN | 4036/4038 PINNED; 5012 OPEN. Next: scan for the `0x1394` `SetID` ctor store. |
| 3011 SendWaitingMessage | Five room-chat send sites; raw line bytes, empty body means empty line | files `0xb9385`, `0xb95b4`, `0xb9742`, `0xe1aa6`, `0xe1ff7` (`docs/net-protocol.md` §4c) | PINNED |
| 2017 SendWhisper | Waiting-room `/w` branches | files `0xb8ffc`/`0xb9369` (`docs/net-protocol.md` §4c) | PINNED |
| 3005 ExitWaiting | BACK / EXIT click; `u16` exit type 0 = Normal | opcode `0x0bbd`; live scripted-gateway leg (`docs/net-protocol.md` §4c, `docs/back-navigation-735.md` §3) | PINNED |
| 4042 / 4044 / 4046 / 4055 | Live setup: 4042 when global `0x7789e6 == 1`; 4044 once on Live byte `0x77cd33 == 1` with session flag `+0x89c`; 4046 on UHP report; 4055 25 bytes after the 4012 send | files `0xc365c`, `0xead18`, `0x18d8a1`, `0xf0439` (`docs/relay.md` §9/§10) | PINNED |
| 4023 / 4027 / 4057 / 3008 / 3000 | Skill, album, extended-skill, slot and room-parameter writes: opcode listed, site not pinned | `docs/net-protocol.md` §4a | OPEN. Next: opcode-immediate sweep for each builder, then a wired click. |

## 6. Loading gate

| Op | Moment | Site / gate | Status; next step |
|---|---|---|---|
| 4012 ConfirmMusicLoaded | Sent immediately after 4010 once chart/OJM staging completes | live Wine host flow 4010 → 4012 (`docs/wine-boot.md`); builder case file `0xdd51d` (`docs/relay.md` §9) | LIVE + PARTIAL site. Next: the exact vtable call site in the host path. |
| 4036 / 4038 / 5012 | Per-member music state / progress / completion during the loading barrier | §5 rows above | PINNED / OPEN as in §5 |
| 4055 ConfirmRelay | Live mode only, after 4012 with no field staging between | file `0xf0439`, sole `0x0fd7` dword site (`docs/relay.md` §9) | PINNED |

## 7. Playing

| Op | Moment | Site / gate | Status; next step |
|---|---|---|---|
| 4014 UpdateGameStats (Jam) | Cool tail and Good tail of the score path, arg `1` | files `0x16e660`, `0x16e7dd` (`docs/gameplay.md`, `docs/net-protocol.md` §4c) | PINNED |
| 4014 UpdateGameStats (Life) | Tick helper answer `2`, global byte `!= 6`, arg `0`; Bad/Miss tails send none | tick function file `0x4eaec0`, send file `0x4eaf3b`; six sites image-wide (`docs/gameplay.md`) | PINNED |
| 4016 SubmitScore | Song-end score submit via the thin `vtable+0x84` wrapper | send sites e.g. file `0x4eb018`; field offsets OPEN (`docs/gameplay.md`) | PINNED moment |
| relay op 5 RelayStats | Live tick: global `0x77ee84 == 1` and seat byte `0x77cd33 == 5` | packer file `0xea230`, send helper file `0x187870`, sole tick caller file `0xeb0c9` (`docs/op5-writer.md`, claims D9) | PINNED |
| relay ops 6 / 7 | Combo broken / combo started; autoplay word is always 1 in the exe, `code` is 0 on client sends | ctors files `0x16dfe0`/`0x16e080`; evidence ids in `docs/relay.md` §6 | PINNED |
| 4048 TestNetworkLatency | After the 4047 punch echo sets its session flag | flag gate checked at file `0xeae6d` (`docs/relay.md` §9) | PARTIAL. Next: tap one Live exchange and read the 4048 body. |
| 4041 FinalizeRank | Song-end tally leg: the bot session sends it next to 4016 at end of play | `docs/overnight-evidence/e2-attempt13-wedge.md:56` (`host_play.log` bot 4016/4041); no qualified exe button/site pin | PARTIAL. Next: one exe Result-entry tap naming the 4041 send site. |

## 8. Result

| Op | Moment | Site / gate | Status; next step |
|---|---|---|---|
| 4021 ExitPlaying (back) | Result BACK control after the count-up past 100 enables back/retry | net event `0xfb5`; relay wraps it for the sender (`docs/cresult-anchors.md` Update and exits, `ev_77ce98f1…`) | PINNED |
| 4010 retry | Result RETRY control; blocked with the waiting-user notice while siblings are ready | net event `0xfaa`, notice VA `0x64a3a0` (`docs/cresult-anchors.md`) | PINNED |
| 4042 live state / 3005 leave | Result's live-state and leave arms, wrapped as `0xfca`/`0xbbd` for the sender | `docs/cresult-anchors.md` (same paragraph) | PARTIAL. Next: one gateway tap naming which control reaches each. |
| 2030 / 2032 | Ranking reopens from the cached `0x779814` table; the one-way latch suppresses a second fetch | see §3 rows (`docs/overnight-evidence/u4-ranking-packs.md`) | PINNED |

## 9. Item Shop (`CStateShop`)

| Op | Moment | Site / gate | Status; next step |
|---|---|---|---|
| 5006 EnterShop | Room ITEMSHOP pick only; MyRoom GOSHOP changes state with no frame | `FUN_0048fd20` file `0x8fd20` (`docs/shop.md`) | PINNED |
| 5007 ExitShop | BACK: full-bleed plate tap and static control `0x14200002`; GOMYROOM `0x14200003` sends nothing | live tap (`~/scratch/evidence-shops/gifted-boot`), static `FUN_00499760` (`docs/shop.md` #178) | PINNED (BACK), PINNED negative (GOSHOP static). Next: one tapped GOSHOP wire tap. |
| 5028 SyncCashPoint | After 5007 BACK, then 5029 wallet answer | live tapped BACK leg (`docs/shop-buy.md`) | PINNED |
| 5015 StartPayment | BUYITEM/GIFTITEM behind web-busy gate, empty-cart and cash/gem checks; mode word `+0x969` = 1/2 | `FUN_00499760`, `ev_f3ad15be…` (`docs/shop.md`); qualified live taps emitted no 5000-family frame — contradiction preserved | PARTIAL + CONTRADICTION. Next: one valid-cart payment tap with 5015/5016 logged. |
| 5013 SyncPoint | Point-sync request; trigger not located | opcode in `docs/net-protocol.md` §4a | OPEN. Next: capture one cash/point change and name the owning click. |
| 5038 RefreshBag | BAGSORT control `0x142000b7`, gated on cart state `FUN_004a1010` | virtual send `0x13ae` at file `0x9bfa8`, `ev_ec83763a…` (`docs/shop.md:396`); contradicts the older "BAGSORT records nothing" note | PARTIAL + CONTRADICTION. Next: one BAGSORT wire tap; a NULL tap retires the static claim. |
| 5034 PenaltyReset | Penalty-reset request; trigger not located | opcode in `docs/net-protocol.md` §4a | OPEN. Next: tap the owning account-control click. |
| 5036 BagExpansion | Bag-expansion request; trigger not located | opcode in `docs/net-protocol.md` §4a | OPEN. Next: tap one expansion purchase. |
| 5000 SyncItemPurchase | Catalog purchase arm kept from earlier work; item-shop BUYITEM with a valid cart did not emit it live | empty builder case file `0x8ee81` (`docs/gifts.md`, `docs/shop.md`) | PARTIAL. Next: wire tap of a completed item purchase. |

## 10. Music Shop (`CStateMShop`)

| Op | Moment | Site / gate | Status; next step |
|---|---|---|---|
| 5015 StartPayment | Cart BUYITEM `0x1a20005d` / GIFTITEM `0x1a200073` after web-busy, non-empty bag, cash and gem checks; writes `+0x14d` = 1/2 | `FUN_004c7f70` file `0xc7f70`, `ev_26ac6479…` (`docs/musicshop.md`) | PARTIAL. Next: one tapped payment leg with 5015/5011 logged. |
| 5010 SyncMusicPurchase | Per-row and cart buy arms (assumed wire side; static buy arm is the opcode-arg builder) | file `0xf3fa8` (`docs/shop.md`, `docs/musicshop.md`) | PARTIAL. Next: one successful music-purchase tap. |
| 5012 in shop | Download START/CANCEL controls stage with the shop but dispatch no opcode | `docs/musicshop.md:260-265` (Download controls #410): both rects are inert because the shop-side 5012 send site is OPEN. No guard name is claimed here: grep finds no `fn .*inert` in `crates`, so the old guard citation was false | OPEN. Next: pin the shop-side 5012 send site before enabling the controls. |

## 11. MyRoom

| Op | Moment | Site / gate | Status; next step |
|---|---|---|---|
| 5007 ExitShop | BACK control `0x15200002`: state change plus empty send; then 5028 → 5029 | `FUN_004a3ee0` file `0xa3ee0`, `ev_aa875251…` / `ev_d3647af6…` (`docs/myroom.md` #802) | PINNED (static + our scripted gateway leg); live tap is ticket #426. |
| none | GOSHOP control `0x15200003` changes state only | same function (`docs/myroom.md`) | PINNED negative (static) |
| 5017 GetGiftMessageList | Gift-box trigger paired with known gift state | paired tap described in `docs/myroom.md` §gift path; site/builder in `docs/gifts.md` file `0x91974` | PARTIAL. Next: wire the paired tap. |

## 12. Gift box / equip / cash

| Op | Moment | Site / gate | Status; next step |
|---|---|---|---|
| 5019 ReadGiftMessage | Gift-list row pick pushes one message id with the opcode | UI send VA `0x4954b4`, case VA `0x4919c2` (`docs/gifts.md`) | PINNED (static) |
| 5023 ClaimGift | Claim press on a gift-box row; bounds-checked index into the `0x24`-stride box table | case VA `0x4a4f98`, call site VA `0x4a4622` (`docs/gifts.md`) | PINNED |
| 5025 SyncFreePass | Three virtual-send sites pass a placeholder zero; owning clicks not pinned | VAs `0x4f4184`/`0x4f41c6`/`0x4f51b4` (`docs/gifts.md`) | PARTIAL. Next: name the three click handlers. |
| 5004 EquipItem (two sites) | Shop-control path and equip-apply path, both push slot then item | VAs `0x4a4af4`/`0x4a4bcb`, builder VA `0x4a4ec8` (`docs/equip.md`) | PINNED |
| 5002 SellItem / 5030 ChangeName / 5032 CheckName | Sell click; rename confirm; rename pre-check with a `strlen` gate that shows a notice instead of sending | send switch file `0xa4df0`; cases VAs `0x4a510c`/`0x4a520f`; check-name builder VA `0x45c3af` (`docs/shop-buy.md`) | PINNED (static) |
| 5028 SyncCashPoint | Shared wallet refresh after shop BACK | virtual-send site VA `0x48ee81` (`docs/gifts.md`); live BACK leg (`docs/shop-buy.md`) | PINNED |
| 5040 CheckGiftFreePass | No 7.55 wire send: raw-byte sweep finds no `0xb0 0x13` immediate and `0x13b0` falls outside the receive range | claim D15 in `docs/claims.md:250`, details in `docs/gifts.md`; codec kept for server interop only | PINNED negative. Next: none for the absence; a new wire capture would refute it. |

## 13. Payment / Payment2 (web views)

| Op | Moment | Site / gate | Status; next step |
|---|---|---|---|
| none on view entry | Charge-plate click opens native `Cash Charge` chrome; zero 5000-family frames on entry and X close | W5 wd-charge negative pin (`docs/captures/journey-audit/296-payment-story-matrix.md`, #429 CLOSED) | LIVE negative (qualified tap). Next: a gift-page tap to repeat it for Payment2. |
| 5015/5016 | No completion frame pinned for either billing page; shop-side 5015 pins do not transfer to billing | `docs/captures/journey-audit/297-payment2.md` (CONFIRMATION-COMPLETION BLOCKED) | OPEN. Next: gateway replay with a completed or failed billing leg. |

## 14. Tutorial / Lanking / Messenger / Drum / Exit

| State | Send audit | Status; next step |
|---|---|---|
| Tutorial | No C→S opcode pinned; art-only stub with unknown entry (`docs/lifecycle.md` §12) | OPEN. Next: find the entry control and tap it with the gateway attached. |
| Lanking | No `CStateLanking` class; display block hosted elsewhere. The ranking dialog's 2030/2032 sends are pinned in §3 | PINNED for the dialog; OPEN for the display-only block: next, REA xref of the mount. |
| Messenger note | Web/HTTP surface; note SEND opcode stays OPEN (#664, `docs/messenger-frame-states.md`) | OPEN. Next: one note-send press with a gateway tap. |
| Drum playing | Stub with no class and no send site (`docs/lifecycle.md` §13) | OPEN. Next: establish whether the stub is reachable before any opcode claim. |
| Exit / teardown | 65520 self-send observed on modal dismissal and teardown (`docs/wine-boot.md`) | LIVE; site OPEN as in §1. |

## Audit rule

A send enters this document only with a moment cell. Where the moment is
not exe-pinned, the status is PARTIAL or OPEN and the next evidence step is
named. That makes the table a completeness check, not a claim that every
row is closed: the OPEN rows are the remaining work, and the named step is
what closes each one.

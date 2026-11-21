# Embedded-IE surfaces and the browser report-back seam (7.55)

Static inventory only. No client was booted for this note. All addresses are
virtual addresses (VA) of the pinned oracle; file offsets are VA − `0x400000`.
Behavior below is described in our own words; no original code is reproduced.

## Version pin and method

| Item | Value |
| --- | --- |
| Oracle | `OTwo.exe`, 5,029,888 B, SHA-256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b` |
| Tooling | REA 6.9.0, Ghidra 12.2, target `target-83a045418b62.exe`; PDB reference unresolved |
| Data | `Interface1.opi`, 75,100,702 B, SHA-256 `d66541198965d622ff0177b306d916885ae4d64ae07b2c2264fcbdd850d50584`; `ControlList_Interface.txt` and `State_Room.bnd` read statically through `o2jam-dump rect` |
| Evidence | REA evidence IDs below; string/byte searches are bounded by the defined-string and initialized-memory inventories |

The exe is the behavior oracle. Where a claim rests on a fixed domain being
reachable, it stays OPEN; no live page content is asserted here.

## Surface inventory

`BROWSER` means the region is an IE control hosted by a `WebCtrl32` child
window. `NATIVE` means the client paints the region from its own control list
and data/wire records. `MIXED` means both exist on one screen.

| Scene | Surface | Kind | Evidence |
| --- | --- | --- | --- |
| Room | Bottom-left user list, `IDC_IMAGE_SROOM_USERLIST_WEB` (`0x13500047`), rect `(17,403)-(147,584)` | BROWSER | `CStateRoom` enter `0x48e990` calls `0x495710`; that function asks `FUN_0048bcf0` for control `0x13500047`, builds `gamefine_main.asp?USER_INDEX_ID=` with `GiftOn=T/F`, and passes the rect to host creator `FUN_0057dac0` with `CWebFriendCtrl`. REA `ev_994e40ce8f2cda22c19307eb2ebaa4fad1c6fa17840dbc096078e9fbd104cd85`, `ev_834b423537a2b2a7a5081c1fe438ed1bdd124f4fb032e710bcabc44999fa14a7`; control row and rect from `ControlList_Interface.txt` / `State_Room.bnd`. |
| Room | Room rows, mode filters, seat counts, icons | NATIVE | `STATE_ROOM` control list and `Room_*.ojs` row fill at `0x125240` (`docs/room-enums.md`, `docs/parity/room.md`); no host-creation call on this path. |
| Channel / Planet | Server, channel and planet selection | NATIVE | `ControlList_Interface.txt` blocks and dispatch (`docs/channel-fill.md`, `docs/planet.md`); no `FUN_0057dac0` call site in these scenes. The Room web page receives `My_planet` / `My_channel` as injected script variables; it does not host those screens. |
| Waiting | Seat grid, music-selection dialog | NATIVE | `STATE_WAITING` / `DAILOG_SELECTMUSIC` control lists and wire seat records (`docs/lifecycle.md`, `docs/net-protocol.md`); no host-creation site. |
| MyRoom | Avatar, furniture, room decoration | NATIVE | `STATE_MYROOM` control list and staged sprite path (`docs/myroom.md`); no host-creation site. |
| AVI | Title/movie overlay | NATIVE | `State_Avi.bnd` rect walk and native art staging (`docs/bnd-semantics.md`); no host-creation site. |
| TOP100 / Lanking | Rank rows and pagers | NATIVE | `STATE_LANKING` control list plus wire rank handler at `0x55b230` (`docs/lanking.md`); no host-creation site. |
| Normal shop / gift / cart | Item rows, money, cart | NATIVE | OPI sprite rows and client/wire data; `docs/musicshop.md` row tables and `docs/claims.md`; shop rows are staged from records, not from an IE document. |
| Music shop | Native row grid plus a web panel | MIXED | Native `STATE_MUSICSHOP` control list (`docs/musicshop.md`); browser panel via `CWebMShopCtrl`, `gamemusic.asp`, rect `(4,0)-(384,541)` at `0x4d2ff0`, REA `ev_73fb69f5837511dc6adfc3ecb39d291763a80eedbf7be422f572c649eda93752`. |
| Payment / billing | Cash-bill, gift input, payment input, jamcard, coupon pages | BROWSER | `cash_bill` URL variants at `0x45e010`; object-supplied payment URL at `0x4f3b30` (`CStatePayment` cluster); jamcard/coupon at `0x4f4bd0` (`CStatePayment2` cluster). `CWebDialogCtrl` / `CWebNormalCtrl`; REA `ev_7ea5b5e2f6feeace61f18a900834abc69a834808fe1d8c88ec6c8c5880203b68`, `ev_73fb69f…`. Builder-to-state map in `docs/channel-fill.md`. |
| Events | Bingo page, hot-time page, ticket-add page | BROWSER | `0x45ea60` (`2009/01/01_bingo/index.asp`), `0x4c1b90` and `0x4f5730` (`2009/06_moa/hot_time.php`, `ticket_add.php`), REA `ev_834b4235…`, `ev_73fb69f…`. |
| In-game help | `mtm/ingame_1n1qna01.phtml` page | BROWSER | `0x45f190` with `CWebNormalCtrl`, REA `ev_73fb69f…`; string at `0x64695c`. |
| Startup | COM class registration helper `CWebAutoRegister` | BROWSER support | Constructed at `0x63d8c0` (REA `ev_dc1611377e3271606a8b5cd28f2fe7c9ac16cbf3b0963485a39b2708ad66cecc`); it registers the page-side COM bridge, but creates no visible surface. |
| Playing | Notes, life, score, judgment | NATIVE | No `FUN_0057dac0` call site; gameplay rendering is the existing native path (`docs/gameplay.md`). |

Host-creation inventory is the nine recorded callers of `FUN_0057dac0`
(REA `ev_8a87691d95db437c8002ba673bbd03ebce1d950a6f6f9eb21fb23fa9b5657cfa`
and `ev_ef504896ea7efe7da330b72532334a1ed256751055dc71271f2d198b9ddac62b`):
`0x45e010`, `0x45ea60`, `0x45f190`, `0x495710`, `0x4c1b90`, `0x4d2ff0`,
`0x4f3b30`, `0x4f4bd0`, `0x4f5730`. Indirect or computed callers are not
discovery-complete; every row above is a recorded static path.

## Report-back seam

### Hosting

One helper creates every IE host. `FUN_0057ee10` (`0x57ee10`) resolves the
class name `Shell.Explorer` at `0x64de5c`, calls `CoCreateInstance` for
`IUnknown` (IID bytes at `0x658afc`), stores the object at host offset
`+0x1c`, obtains `IOleObject` (`0x658d2c`), registers a MSHTML control-site
interface (`0x65880c`), and calls two site helpers (`0x57f090`, `0x57f020`).
Its only two callers are the navigation writer `0x57de00` and the host window
procedure `0x57e240` (REA
`ev_cf454a79fc0e438d436c9d920b7d2fb6e0d1244e13cca6c4385f54248a561794`,
`ev_65d65d46d3afd2f0296b29fa49ac8c36d91ec0fd4ef85b4ce3e397a9adfdbb2d`).
`FUN_0057dac0` creates the `WebCtrl32` child window at a control rect and
passes the control object as creation data.

### Page → game

Three static channels exist; none is a custom URL protocol. No `o2jam://` or
`o2://` literal appears in the defined-string inventory, and the only
`://` strings are ordinary HTTP page URLs.

1. **COM event bridge.** `FUN_0057f970` / `FUN_00581020` (`0x57f970`,
   `0x581020`) get the browser document, create the COM object registered as
   `EventBridge` (string `0x64df50`), query its connection point container
   (GUID at `0x6588dc`), find the MSHTML GUID at `0x64df40`, and advise a
   native sink: `CWebEventScript` or `CWebEventInputButton`. REA
   `ev_65d65d46…`, `ev_c97889280d722295bcb0f14ecf1b202246c24883aaca9eb846245d4dfa1bb5f7`,
   `ev_0915312e620f418aee29557c4f7b6fc70afc277a48ff0e8ecbd0751c583d4e16`,
   `ev_760bc6c5d45f5f9542e4e82ed916ed3059c2de217ea40d7daded1b5a9098addf`.
   A `CWebEventScript` event with DISPID `-600` posts window message `0x9095`
   (`FUN_00582280`, `ev_0df0381c1c2499317d4c6b46fd81f0756ba1b05576b36a8d11387d89bef0c498`);
   the matching `CWebEventInputButton` event posts `0x9096`
   (`FUN_00582220`, `ev_e608b58333988b3a4bf56142c50ad2363364aa95f537ae09372862cd56b3ccb1`).
   The byte sweep finds exactly one `0x9095` post and one `0x9096` post
   (`ev_4f4158b5a4b9c370e374ab615c3b52810a2ed1b8b5cd17a0517a21b50c8c722c`,
   `ev_5e4480a01262437508e5c030ef0e1060bcc85f8e2109bd358e8d3f03659d72d3`).
2. **Payloads.** The `0x9095` arm of `FUN_0057f770` reads
   `SelectedUserNickName` from the page and sends notify type `0x12`, id
   `0x1bbe`, with that ANSI nickname (`FUN_00580600`). The `0x9096` arm of
   `FUN_00580e40` reads a second page property and `mflag`, packs a short
   result value plus a flag byte, and sends the same `0x1bbe` notify
   (`FUN_005816e0`). REA
   `ev_53fe5eabcc9e8779d7d4d8e62c1d17878439d0f32d4d46d1462fe0123b531ecf`,
   `ev_fd6e946f6baf47044c3c3b904b30a392ff52246635ae92323f977badd7919985`.
   This is the closest thing to a purchase completion signal: a result code
   and flag, not a named order id. An order-id payload crossing this seam is
   OPEN.
3. **Navigation events.** `CWebEventDoc` handles browser DISPID 252 and 259.
   On document complete it converts the event URL to ANSI, posts `0x9098`
   with the URL as LPARAM, then posts `0x9094`; the host's `0x9094` arm copies
   `get_LocationURL` into a 2 KiB field at host offset `+0x207`. REA
   `ev_3ed36e5b5773e481b04f2d285914bd9a170a800e2755d172b8f21805125c482e`.
   For billing pages, `FUN_0057f510` compares the current and cached URLs,
   clears two host flags when the URL is the billing-buttons page, and forwards
   the cached URL as notify `0x12` / `0x1bbd` (REA
   `ev_2865f0b4589723d707f78dcd74d70db08e534d06519f3c9250d1f9b0c11a7644`).
   So a "purchase completed" report is a navigation to a known URL plus a
   payload carrying that URL; there is no separate result-code integer on
   this branch.

### Game → page

The client writes JavaScript globals into the document before advising the
sink: `My_planet`, `My_channel`, `CTuser_count`, `CTuser_id[]`,
`CTuser_index_id[]`, `CTuser_nickname[]`, `CTuser_level[]` (`FUN_0057fe10`).
It also invokes named page functions `jsBuyMusic` and `ReloadConnUserList`
(`FUN_00581cb0`, `FUN_0057fc50`), and can navigate with a POST body and a
form-urlencoded header (`FUN_0057de00`). REA `ev_65d65d46…`,
`ev_c14370bac00d38e370beabc8eefe966a1e1c22dc07515820329ee352d95b59a4`.

## Hypothesis verdict

**REFINED.** The owner's specific pin is **SUPPORTED**: the Room state's
bottom-left player list is an embedded IE page. The broad claim that dynamic
surfaces in general are browser-hosted is **REFUTED** by the static inventory:
the exe has only nine recorded browser-host creation sites, and the main
data-driven surfaces (room rows, channel/planet lists, waiting seats, TOP100,
MyRoom, AVI, Playing HUD, normal shop rows) paint from control lists and
client-held records.

Evidence chain for the pin:

1. `CStateRoom` enter method `0x48e990` calls `0x495710` (REA
   `ev_994e40ce…`).
2. `0x495710` reads control `0x13500047` through `FUN_0048bcf0`, builds the
   `gamefind_new/gamefine_main.asp?USER_INDEX_ID=` URL, constructs the
   `CWebFriendCtrl` sink, and calls `FUN_0057dac0` (REA
   `ev_834b4235…`).
3. `ControlList_Interface.txt` names that control
   `IDC_IMAGE_SROOM_USERLIST_WEB` in `STATE_ROOM`; `State_Room.bnd` pairs it
   with rect `(17,403)-(147,584)`, the bottom-left region of an 800×600 room
   screen.
4. The control is a browser host, not a native row: the only way this path
   reaches the screen is `CreateWindowExA(..., "WebCtrl32", ...)`, and the
   page content is remote (`gamefine_main.asp`).

Counter-model: `CStateShop` / normal shop rows use `ControlList_Interface.txt`
plus staged sprites and wire records; no `FUN_0057dac0` call site exists on
that path. The music shop is mixed: native rows plus a separate
`gamemusic.asp` browser panel.

## Named next evidence steps

1. **#405 direct verdict drive (first choice).** Boot the exe in the private
   no-audio rig, enter Room, click the pinned user-list rect, and capture the
   frame with a provenance manifest. Record the page URL and any `0x1bbe`
   notify on the wire/log side. This settles whether the web list's selection
   is the verdict path, without changing any host.
2. **Hosts-redirect stub (only if the remote page is dead).** Redirect
   `game1.o2jam.com` and `o2jam.nopp.co.kr` to a local fixture that serves a
   minimal page raising `EventBridge` events with a known nickname and
   `mflag`. Compare the resulting `0x9095`/`0x9096` messages and `0x1bbe`
   payload with the static pin.
3. Name the page-side call that raises DISPID `-600` and the exact meaning of
   the MSHTML GUID at `0x64df40`, using the registered COM type library or a
   stopped-state COM vtable read. This is the remaining unknown in the
   `EventBridge` dispatch.
4. Trace the `0x1bbd` / `0x1bbe` notify consumers in `CStatePayment`,
   `CStatePayment2`, `CStateRoom` and `CStateMShop` to the final UI or wallet
   effect; confirm whether any order id is appended later in that chain.
5. Confirm `CWebDialogCtrl` is created through MFC runtime-class creation
   (no direct static caller was recorded) with a live billing-page capture.

## OPEN items

- No live page content, redirect, or purchase was exercised here; the remote
  URLs are pinned strings only.
- The page-side name of DISPID `-600`, the semantic meaning of the `mflag`
  byte, and any order-id payload remain OPEN with steps 3–4 above.
- `CWebDialogCtrl` instantiation is MFC-dispatched; its live caller chain
  stays OPEN with step 5.

## EventBridge action-flag table (7.55, static)

This section closes the statics leg of #753. It uses the same oracle and
address convention as the version pin above: VA minus `0x400000` is the file
offset. All behavior is described in our own words from Ghidra 12.2 facts;
no original listing is reproduced.

### Message-map arms

A complete sweep of initialized `.text` finds two posts and two receiving
arms per event message. The COM sink `FUN_00582280` posts `0x9095` at VA
`0x5822a3` (file `0x1822a3`); the companion sink `FUN_00582220` posts `0x9096`
at VA `0x582243` (file `0x182243`). The host window procedure arms are
`FUN_0057f770`, which tests `0x9095` at VA `0x57f77c` (file `0x17f77c`), and
`FUN_00580e40`, which tests `0x9096` at VA `0x580e4c` (file `0x180e4c`).
Both arms call the same notify builder with type `0x12`, id `0x1bbe`, and a
payload pointer; other messages go to `FUN_0057e240`. REA
`ev_c348f01a3d8fb3501a35e936b01d55017f1ef76e2a1055a6c54de0609a5a4003`,
`ev_3e3dd98327f2642114663eced592f7c20a3d5fb2ae8f5579e208e2b737f8d0be`,
`ev_1ac5bc516ba76880a9d764222676c4a870b662b1dc9276a4400214d36931c0d3`,
`ev_e1dffd5ae47638a82f9a1f463e7f651871b592330e3748ff0463abdb13effc55`.

A two-byte sweep for `0x1bbe` found six byte matches and four code sites: the
two producers in those arms (VA `0x57f79e` / `0x580e6e`, files `0x17f79e` /
`0x180e6e`) and the two consumers below (VA `0x490a70` / `0x4cb3a8`, files
`0x90a70` / `0xcb3a8`). The other two matches are instruction-interior call
bytes in unrelated code, not comparisons. REA
`ev_3241b811ea4b28234655f51964f1d36fbf6cdc3ce06be5d773d1d80d0e274afa`,
`ev_d448ad1cd392b3fda0e1441102670f98b39cd181563319e8179ec9eb09ca46bc`,
`ev_0980abc53160637e04a55f2540df3224240644782b8d752bbc811676afce02e4`,
`ev_6820933d67b24fc268cfb90225e05c0f811257dea2581ee2764f6d831a399e1a`.

### Payload forms

- `FUN_00580600` reads the page property `SelectedUserNickName`, converts it
  from wide characters to the process ANSI code page, and returns the byte
  string used as the `0x1bbe` payload. REA
  `ev_4a57c2b95816c5576c8e82660560d9d82c8c196359eeffb6b51d6c0ea00a90db`.
- `FUN_005816e0` reads page properties `mid` and `mflag` (literals at VA
  `0x64e080` and `0x64e084`, files `0x24e080` and `0x24e084`). It parses
  `mid` as a decimal number, stores its low 16 bits as a short at payload
  offset 0 and the first character of `mflag` as a byte at offset 2. REA
  `ev_d96517d26f81f4032428a6ac24a5e7fdcfce7b4720c07240342671d8ccef34ee`,
  `ev_624fd01a1cca41eee2acb5e38263fbd6c21d5a6651c442aca204fb4146ff98e9`.

These are two distinct payload shapes on one notify id. The flag column below
only applies to the second shape; the first shape carries a nickname string.

### Table

| Value | Action | Consumer | Evidence id |
| --- | --- | --- | --- |
| `0x9095`, ANSI nickname (no packed flag) | Opens the room chat sheet with the nickname staged as the whisper target; selects `IDC_BUTTON_SROOM_CHAT_WHISPER` (`0x13201011`, `docs/hover.md`) and stores chat mode 3 | `FUN_00490950` at VA `0x490950` tests `0x1bbe` at `0x490a70`; it calls `FUN_00490750` with the payload, then frees it | `ev_857fd8095877ebca7336ef51dc1afcfd5763ee018bee40534fa3c07fae37a541`, `ev_7a06eca890a7f0d39af3941187e2cfa097ffa81939e530baafc38ea6bc8314c9`, `ev_ee00e74f3f126989142a2f64a22b8a8aea792d858dc7304a89ee74fcc73df3b3`, `ev_e244069b9c04336246d110140ae461f26ee076e34ce4a1caefa2951ce7b770bc` |
| `0x9096`, `mflag` byte `0x43` (`C`) | Adds the packed `mid` to a cash-priced shopping bag through the native cart helper; the helper reads the first price/balance slot and queues a native shopping-bag record | `FUN_004cb2b0` at `0x4cb2b0` tests `0x1bbe` at `0x4cb3a8`, unpacks `mid` and the flag, then calls `FUN_0053ad10(1, mid, flag)`; on success it posts control message `0x1a20005f` | `ev_9a633a77e42b02cc6f6aa13fe966bf325984c0f66e8fda27feef030c7210107e`, `ev_be7a0259222adb4dfd2b3687ae871a8fc224f0f44c95e544e217f98ebbb2bef3`, `ev_a90de7798b16094272a200cb802e667c1235cf078a8c1fe01391679e24df28ac` |
| `0x9096`, `mflag` byte `0x47` (`G`) | Same cart helper, second price/balance slot; native call sites pair this byte with a Gem-balance notice at VA `0x64897c` (file `0x24897c`, CP949) | Same consumer and helper as the `C` row; the item-shop call sites choose `C`/`G` from two price fields and the user-shop site from a stored byte | `ev_9a633a77e42b02cc6f6aa13fe966bf325984c0f66e8fda27feef030c7210107e`, `ev_ef3791e29e587141788f35d8131d6e27b9d84bd669eda422a98a42e7be6ff7ea`, `ev_f7208fcdd19e4ed6646ec853f257bc86edd186ceaba60eabe447d15b6c1b213a` |
| Any other `mflag` byte | No action: the helper's `C`/`G` branches are the only byte tests on this path, so it returns without staging a bag record | `FUN_0053ad10` byte tests; `FUN_004cb2b0` still posts the refresh control message | `ev_be7a0259222adb4dfd2b3687ae871a8fc224f0f44c95e544e217f98ebbb2bef3`, `ev_9a633a77e42b02cc6f6aa13fe966bf325984c0f66e8fda27feef030c7210107e` |

The action helper takes a source selector as its first argument. Source 1 is
the only value passed by the `0x1bbe` consumer; native item-shop and
user-shop call sites pass sources 0 and 2 with the same two flag bytes, so the
flag means price/currency choice, not a friend or invitation action. REA
`ev_f7208fcdd19e4ed6646ec853f257bc86edd186ceaba60eabe447d15b6c1b213a`,
`ev_ef3791e29e587141788f35d8131d6e27b9d84bd669eda422a98a42e7be6ff7ea`.

### Nickname path and messenger cross-reference

The `0x9095` nickname string reaches a native room-chat record: the consumer
stages it into the record, calls the helper that activates one of the four
chat buttons `0x1320100e`–`0x13201011`, and stores chat mode 3 through
`FUN_00492940`. The whisper button is also handled by `FUN_00490870`, which
selects the same button and stores the same mode on a press. REA
`ev_7a06eca890a7f0d39af3941187e2cfa097ffa81939e530baafc38ea6bc8314c9`,
`ev_e244069b9c04336246d110140ae461f26ee076e34ce4a1caefa2951ce7b770bc`.

Where a whisper is actually sent, `docs/net-protocol.md` pins `2017 SendWhisper`
C→S (recipient, then content), `2018` as its reply, and `2019 ReceiveWhisper`
as the recipient's event. The exe has a `2017` opcode push (`0x7e1`) at VA
`0x48fa02` (file `0x8fa02`) inside `FUN_0048f3a0`, whose `/w` branch copies the
named recipient into a message buffer before queueing it. REA
`ev_c0039591f203953c28676b5d54c3ddfc56ffe7daf53b24ceab878a7fad6478ce`,
`ev_3442a87819ea2ac82715917b59a8e49ffcb0ec4f95a300f6d4dcaa02a8fb6a61`,
`ev_c121aae87a7f3532a5859c606d85bd077815217cf1a954e6b56201f56b3a22af`.
The static link from the staged room-chat record to that opcode remains OPEN
(cell W1 below).

### Named OPEN cells

- **W1 — whisper send link.** The nickname is staged and the whisper button
  selected; no resolved static call chain from that record to a `2017` send
  builder was found. Next step: in the stopped-state rig, raise `0x9095` with
  a known nickname, break on the `0x7e1` push at VA `0x48fa02`, and read the
  recipient buffer plus the resulting wire body; compare with `2017` in
  `docs/net-protocol.md`.
- **F1 — page-side flag source.** `C`/`G` price-slot behavior is pinned, but
  no live page was read to show which `mflag` string the page writes or which
  wallet field each slot is named. Next step: use the local EventBridge stub
  from "Named next evidence steps" item 2 to set `mid` and `mflag`, then
  record the `0x1bbe` payload and the wallet delta.
- **F2 — actions outside `C`/`G`.** No add-friend or room-invite value or
  consumer exists at any of the four `0x1bbe` sites. Next step: inspect the
  EventBridge type library or a live page's event properties for other
  property values, then repeat the two-byte sweep on that build before
  adding rows.
- **F3 — currency naming.** The `G` branch's Korean notice names Gem; the
  `C` branch's wallet/currency label is not named by a literal on this path.
  Next step: pair a `C` fixture event with the `5028`/`5029` wallet snapshot
  (`docs/shop-buy.md`) to identify the changed field.

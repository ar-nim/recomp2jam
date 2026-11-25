# Music-select and itemshop-cart statics dossier (#884, #886)

Statics-only dossier. No rig was run, no Rust code was changed, and no
pixel or wire capture was produced by this lane. The two quiesced pairs
come from the v6 batch; every claim about the executable comes from REA
MCP static analysis of O2Jam 7.55 `OTwo.exe`, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`
(REA session
`ev_696158c42763ef469cb735213401bf31c2ad2491166f43c4e42f3f17adba8e7d`).
Tool: REA 6.10.1 / Ghidra 12.2, the one bound Ghidra document,
analysis digest `547bdf93bd6e607cbb88c4dbd6133b7dc54ce7ab567c389324768dbdb3e68c2c`.
Addresses are VAs; file offset = VA − `0x400000`. No disassembly or
decompiled text is reproduced below; behavior is described in our own
words with addresses, control codes, file offsets, and evidence IDs.
Evidence IDs of 32 hex digits carry an `0x` separator after `ev_` so
the commit-hash checker does not read them as commit citations. Ghidra
function and global names are written `FUN_0x…` / `DAT_0x…`; the `0x`
spelling is the VA, not a new symbol name.

Prior lanes are the starting point, not re-derived here:

- `~/scratch/close-out-program/reports/lane-musicselect-panel.md` —
  category plate and bottom-right effect-card border are already fixed.
- `~/scratch/close-out-program/reports/lane-cart-residual.md` and
  `docs/shop.md` §`#886` — the cart-frame BAGSORT plate, BACK plate, and
  baked lamp are already fixed; the set-card pose and the cart-row count
  are capture/content verdicts.

The complete query ledger, including empty searches, is in
`~/scratch/lanes-2026-11-25/findings-statics.md`. Proposed fix-ticket
texts are in that file, one block per region.

## 1. Pairs and divergent-region inventory

| Pair | Exe frame | Rust frame | Structural |
|---|---|---|---:|
| music-select | `~/scratch/paired-shots-v6/states/music-select/exe.png` | `…/rust.png` | 0.974646 |
| itemshop-cart | `~/scratch/paired-shots-v6/states/itemshop-cart/exe.png` | `…/rust.png` | 0.982006 |

Frame digests and trace metadata are in each pair's `metrics.json`. The
music-select frame hash is
`dc30f12a1f1c724ca0bd4207df78d0429854053b62c861b7ed364680a54ec261`
(exe) and `de068b670d7c04a76c06bf7b636f256025ebd6340d0d9146b58a4a6f1220782a`
(rust); the cart frame hash is
`c2e056b84de2b04c393df55ec90323bb9e129aedb59fa876d97c8c632ed03e45`
(exe) and `6ed621404c5cd346f0faed8173a1655d898752b4497cc8251a0fbd0496f916b3`
(rust). Bounding boxes below are native 800×600 pixels. They come from a
thresholded absolute-difference map; masks and side-by-side crops are in
`~/scratch/lanes-2026-11-25/diffs/`.

### 1.1 Music-select — nine named regions

| ID | Region (native px) | What differs | Class | Pin / disposition |
|---|---|---|---|---|
| M1 | Top strip `(271,13)-(329,33)`, `(478,13)-(671,38)` | Room title `W.E.BIN` vs `MINE JOIN`; green song caption `? [02:04 3K]` vs `다시로 [C2 ver., 3K]` | C6 | Exe caption reads the selected row of the server song table; see §2.2, `ev_5fa0903560b0c24232533180f24ee281311a4af11e382a63ec756fa1882d298e`. Content/wire, not a paint fix |
| M2 | Seat grid `(33,54)-(303,228)` | Seat 0 empty in exe, occupied in ours; seat 1 orange nameplate in exe, red+orange in ours; READY band background differs | C6 | Wire scenario; prior pin `ev_0x50242b6c183543f49ce6cbb43eccad2e`, role `ev_aa74bced2e8264910180f80e1642cd4aa751dc3e64570b7ddb61ac7310b08663`, §2.1. No paint change; replay the exe 3007 |
| M3 | TEAM block `(647,64)-(714,152)` | `START` face in exe, `READY` face in ours | C6 | Same role gate; master arm enables START, guest arm sets READY, §2.1 |
| M4 | SKILL block `(662,242)-(769,317)` | All six arrows bright blue in exe; lower four gray in ours | C6 | Same role gate; `FUN_0x4b4a00`/`FUN_0x4b4d30`, §2.1 |
| M5 | Chat strip `(22,455)-(500,534)` | Five Korean history lines in exe; empty history plus caret in ours | C6 | Chat append helper pinned in §2.4; the 3007→append edge stays OPEN with a named next step |
| M6 | Song panel `(536,448)-(596,545)` | Blue `곡선택` button in exe; `BPM / 124` cover in ours | C6 | Role-swapped pair `MUSELECT 0x16200003` / `COVERMUSIC 0x1650001d`, §2.1 |
| M7 | Random arrows and effect cards `(630,448)-(700,559)` | Exe arrows bright blue; ours dim/gray; effect-card border differs | C6 | Role arms plus prior random-card pin `ev_2cb3a3c4386ec6191b3b1f10f7107e0e373d8f4e9fe0bfeb6383ee3e48d079f4`, §2.1 and §2.5 |
| M8 | BACK plate `(709,568)-(795,594)` | Dark blue face in exe, bright blue face in ours | C6 | Static frame writer still OPEN; prior quiesced-pair rule recorded in `docs/parity/waiting-chrome-dossier.md` §5 |
| M9 | Top-left plate `(178,54)-(209,85)` | `사람손` in exe, last stacked record in ours | C6 | Already fixed by the prior lane: `FUN_0x4bcaa0`, `ev_6b57088ac8898bbb711087e60504b404a21a9817cda16fc1c27c0855e973d4a6`, §2.3 |

The letter badges of the TEAM grid, the font raster of the top strip, and
the purple messenger button differ in shape only. They are presentation
under the owner's modern-rasterizer decision (`e8d01`), not new gameplay
divergences.

### 1.2 Itemshop-cart — eight named regions

| ID | Region (native px) | What differs | Class | Pin / disposition |
|---|---|---|---|---|
| S1 | Left identity + wallet `(31,52)-(149,367)` | Exe `Lv.7:tester` is tiny at the cell top; ours is larger and lower. Exe G/W cells show only `G`/`W`; ours add `GEM 15533` / `O2 CASH 0` | C6 | Exe formats `" Lv.%d:%s"` and width-fits it, §3.2; wallet value writer OPEN |
| S2 | Set-item figures `(376,64)-(742,216)` | Card 1/2/4 figures hold a different animation phase (eyes open/closed, arms up/down) | C8 | `FUN_0x49e8d0` refresh and `FUN_0x529230` figure write, §3.4; capture-class phase, no geometry fix |
| S3 | Set-item price row `(218,236)-(702,260)` | Ours inks `G 0` / `₩4500` / `G 0` below cards; exe leaves those rows dark | C6 | Price writer is state-gated, `FUN_0x4a21a0`, §3.4. Fix proposal T11 |
| S4 | MESSAGE buttons `(339,282)-(519,305)` | OK/CANCEL gray in exe, bright blue in ours | C6 | Static dialog setter and prior baked-plate rule, §3.5. Fix proposal T12 |
| S5 | MESSAGE caption `(339,282)-(519,305)` | Same two Korean lines and rect; exe ink is brighter/bolder | C6 | Approved modern-font presentation (`e8d01`); prior pin `docs/shop.md` §`#886` |
| S6 | Shopping-bag rows, totals, pager, BUY plate `(0,462)-(440,586)` | Exe one cart row `4500 W`, `01/01`, `G 0`; ours five rows, `09/09`, `G --` | C6 | Cart-count gate `FUN_0x4a1010`, §3.3; row content is a wire-state delta, total writer OPEN |
| S7 | Bottom-right chrome `(612,543)-(795,582)` | White `...` popup above the purple lamp in ours; absent in exe. BACK face also differs | C6 | Entry resets both alarm controls to a body-less rest frame, §3.6. Fix proposal T13 |
| S8 | Top browse chrome `(201,13)-(344,42)` | `Show All` plate and `ITEM` button letters differ in raster only | C6 | Presentation (`e8d01`); no behavior change |

Divergent-region count: **17** (9 music-select + 8 itemshop-cart).
Region S3 is partly covered by the sheet in the quiesced frame; only the
uncovered price cells are counted.

## 2. Music-select exe trace

### 2.1 Role chain: one wire word drives seat, song panel, START/READY, and arrows

The master word is the local seat's `+0x30` byte, written from the room
state. The role gate `FUN_0x4ae350` (file `0xae350`, evidence
`ev_aa74bced2e8264910180f80e1642cd4aa751dc3e64570b7ddb61ac7310b08663`)
selects one of two arms. A fresh batch decompile re-pins both arms:

- `FUN_0x4b4a00` (file `0x4b4a00`, `ev_83d715f18cc0b7accc9b9e207062bd3a0d821c158b1d00af5bf44feaf458e7b4`):
  master arm. It enables `START 0x16202012` and disables `READY
  0x16202013`; enables `MUSELECT 0x16200003`; sets frame 0 on the three
  `COVERMUSIC` controls `0x1650001d..1f`; enables the map arrows and all
  six skill arrows `0x16200050..55`; clears the three gray skill covers
  `0x1650005b..5d`.
- `FUN_0x4b4d30` (file `0x4b4d30`, same evidence): guest arm. It sets a
  READY frame, disables START and MUSELECT, sets frame 1 on the three
  cover-music controls, and either disables all six arrows with gray
  covers (no skill selected) or enables the first two arrows with gray
  covers on the last four (skill selected).

The enable word is written through `FUN_0x409bf0` (file `0x9bf0`,
`ev_694dfc15dd90b0d7d13d1aa9525d030825dfd20c458cdd894cca987bfcfe282a`, already cited in `docs/parity/waiting-chrome-dossier.md`);
the skill batch pin is `ev_b401b64bd3ffc3b9cd2078e3aa26581cc41bd0cde46c1fc2269844c9e5d1d5af`.
No separate paint literal chooses `곡선택` versus `BPM`: the role arm
hides the cover for a master and paints it for a guest. That is the exe
behavior for M3, M4, M6, and M7.

Rust today: `crates/o2jam-shell/src/screens/paint/waiting.rs:832-858`
keeps `MUSELECT` and the skill arrows for a master and keeps
`COVERMUSIC`/`COVERSKILL*` and READY for a guest;
`crates/o2jam-shell/src/screens/game/mod.rs:9499-9506` opens the song
dialog only for a master. The owner rule #278 authorizes this role face
(`waiting.rs:806-825` records that the exe display rule was OPEN when the
rule was made); the fresh static pins above now make the exe rule
explicit. The quiesced pair differs because the rust leg never received
the exe leg's 3007 leave event, not because the role painter is wrong.
Wire and server logs for both legs are in `docs/music-availability.md` §9.

### 2.2 Top captions and seat nameplates (M1, M2)

`FUN_0x4b2e30` (file `0xb2e30`,
`ev_5fa0903560b0c24232533180f24ee281311a4af11e382a63ec756fa1882d298e`)
refreshes the whole top strip:

- It looks up the room song id `DAT_0x7789d8 & 0x0fffffff` in the song
  table, then inks the row's title into the `MUSICNAME` control
  `0x16400017`. Long titles are shortened by a font-width loop; the
  high nibble of the same word selects the random caption instead of the
  title.
- It computes a level badge frame from `DAT_0x7789e7`/`DAT_0x7789e9`
  and writes it through `FUN_0x42f870` to `0x1650001b`.
- It writes the local nameplate from `DAT_0x77cd05` plus `DAT_0x77cd20`
  (level) into the `0x16015022 + seat` slot for the local seat, and peer
  nameplates from `DAT_0x77ee95 + index * 0x2190` for the other seats.
  The local master byte `DAT_0x77cd30` selects the master variant.

The room title text is server-supplied table ink; the exe has no local
room-title literal writer in this function. The two legs therefore show
different titles and songs because they carry different room data.
Rust reads the same fields through `WaitingCaptions` at
`crates/o2jam-shell/src/screens/game/mod.rs:9421-9444`; the painters are
`paint_waiting_top_line` (`waiting.rs:1719`) and
`paint_waiting_covers` (`waiting.rs:1813`). No paint fix is proposed.

### 2.3 Category plate and effect cards (M7, M9)

Already pinned by the prior lane, not re-derived:

- `FUN_0x4bcaa0` (file `0xbcaa0`) sets `CHCATEGORY 0x16500049` to frame
  `DAT_0x76fc38 + 1`; the global is the five-emblem family index
  (`ev_6b57088ac8898bbb711087e60504b404a21a9817cda16fc1c27c0855e973d4a6`,
  immediate `ev_592fd5170a7365d18ef6c8e93e492d1801b7af5a5dfd37b69248cba2a593f267`, xrefs `ev_5d2286498abaffaaa9984d127455e193051906fca549f0f8a2eb5c18d62ffa04`).
- `FUN_0x4ad350` mounts `Wait_Effect_Rand1/2.ojs` on the spriteless
  `EFFECT_1/2` rects and sets `RANDMAP` frame 1 for a random pick
  (`ev_2cb3a3c4386ec6191b3b1f10f7107e0e373d8f4e9fe0bfeb6383ee3e48d079f4`).

Rust implements both at `waiting.rs:1579-1601` and
`waiting.rs:2070-2112`, with the family index carried by
`WaitingChannelFamily` from `ScreenGame::planet_selection()`.

### 2.4 Chat history (M5) — append helper now pinned, wire edge still OPEN

The earlier dossier left the chat-append source OPEN. This lane finds the
generic append wrapper: `FUN_0x4bb6c0` (file `0x4bb6c0`) fetches
`LIST_SWAIT_CHAT 0x1680002b` and calls `FUN_0x510730` to append a row
(`ev_b7acfc7a52c97131acd877427f2c4db7ada18065547f3a7c476092b0352b708d`).
Its resolved callers include `FUN_0x4af3f0` and `FUN_0x4b59f0` (file
`0xaf3f0` / `0xb59f0`, same evidence): both format a string from
`DAT_0x77ee95 + seat * 0x2190` with the format at `DAT_0x649944`, append
it through the wrapper, then refresh seats and top captions.
`FUN_0x4b8d00` (file `0xb8d00`) appends local `/help` and `/r` echoes
through the same wrapper; the caller list is
`ev_ce48156c58ccddd7aca635c400d006479def8d600b491fbcb4f520a4a36b0e5b`.

A bounded forward trace from the 3007 handler `FUN_0x55d820` to the
append wrapper returns `goal_status: unknown`
(`ev_656aa94b8442fb2f636d7c6d951f2d8683c0d7092a337c93cf5460a45463839a`):
the direct-call graph is too wide and contains resolved terminal calls
with unknown no-return basis. So the append helper is pinned, but the
3007/2007 → notice formatter edge is **OPEN**. Named next step: inspect
the two formatter call sites from a qualified leave capture with a memory
read of the chat list object, or re-run the trace with a bounded stop set
of only `FUN_0x4af3f0`/`FUN_0x4b59f0`. The prior
`FUN_0x55d820` `referenced_strings: 0` evidence
(`ev_f1758b60a156a73195b00b422f96d127cae120e05ae3d33bb9924d6b5e2d8ae8`)
still must not be cited as a chat pin.

Rust today: `crates/o2jam-shell/src/screens/game/mod.rs:9480-9495` feeds
`self.chat.waiting_history()` / `waiting_draft()` into `paint_waiting_chat`.
There is a live path for ordinary chat; the missing five lines are the
exe leg's leave notices, which our leg never received. No paint fix.

### 2.5 BACK face (M8)

`docs/parity/waiting-chrome-dossier.md` §5 records the quiesced-pair rule
for record 0 of `Wait_Back.ojs` and states that the static frame writer
is OPEN (`ev_cfdec478bf1c6aad889b118f2e8a91960000443bfa1be0354c4442e37ca5eda9` finds only a missing-song control pin). This
lane did not find a new writer. Keep the pair-matched rule; do not claim
a static pin.

## 3. Itemshop-cart exe trace

### 3.1 Entry chain

`FUN_0x4984d0` (file `0x4984d0`, `ev_1b078fdd166e7bef4b979f5a928a41d7395d8a8bef4facb2e45deff0ba084197`)
is the shop entry function. It loads the four runtime font bodies
(`Shop_Font_ItemWon.ojs`, `Shop_Font_ItemGem.ojs`, `Shop_GemFont.ojs`,
`NewMark.ojs`), then calls in order: `FUN_0x4a1310`, `FUN_0x49f820`,
`FUN_0x551820(4,1)`, `FUN_0x49ffe0`, `FUN_0x4a0660` (identity and staged
cells), and `FUN_0x4a1010` (cart-state enable gate). It then loops over
two alarm controls `0x145000b9 + index` and calls `FUN_0x42f5a0(3)` on
each staged one. That order is the live caller chain for S1, S3, S6, and
S7: boot → shop entry → staging → enable gate → paint.

### 3.2 Identity and wallet cells (S1)

`FUN_0x4a0660` (file `0xa0660`,
`ev_13a2300e1ce10cb30c77e106df52388f2e61042994aa979d2f3dc91752138d3d`)
does three relevant writes:

- It formats `" Lv.%d:%s"` from `DAT_0x77cd20` and `DAT_0x77cd05` into
  `NICKNAME 0x1440002a`, then measures the string and shortens it until
  it fits the cell (about `0x9b` pixels). Both code references to the
  control immediate are at file `0xa0775` and `0xa0828`
  (`ev_169ce0311d8c5e875df60b6fe4db678767d9ffbaf24a6d56da938c001a818779`).
  This pins the source and the width-fitting rule; glyph raster size is
  presentation under `e8d01`.
- It copies two 16-byte cell blocks from controls `0x14400028` and
  `0x14400029` (`GEMMONEY`, `MGASH`) into object words `+0x235c` and
  `+0x236c`. A byte-pair search for the two immediates has no `.text`
  match (`ev_30f5826a63249618fc9a4ef882eb5b07825e9d8f2736ff2d8d0af3b9badfbbfc`):
  the values arrive indirectly, so which wire word feeds each cell stays
  OPEN. Next step: replay 5001/5028 with a gateway tap and read the two
  object words.
- It stages `CHECKMONEY 0x14400075` into `+0x247c` and `CHECKGEM
  0x14400074` into `+0x248c`. The only two immediate uses of
  `0x14400075` are this stager (file `0xa0e83`) and the enable gate
  (`ev_8306e268bf92a771f979666868b511727e7c970060c22ced4cebc64c9e5476f6`),
  so the exe's total-ink writer stays OPEN, as recorded in `docs/shop.md`.

Rust today: `paint_shop_identity_captions`
(`crates/o2jam-shell/src/screens/paint.rs:1805-1836`) and
`paint_shop_wallet` (`paint.rs:1691-1704`) ink the staged cells;
`paint_shop_cart_totals` (`paint.rs:1838-1865`) uses a staged-price sum,
with `--` for the gem word. The `--` and the extra wallet digits are
source/value differences, not geometry.

### 3.3 Cart count gate (S6)

`FUN_0x4a1010` (file `0xa1010`,
`ev_99c9d97980fd2751a1dd37396569f1bf3a215673f85033fda6718df9f9a6979a`;
dossier summary `ev_6c9aa454b972deec7a8c1e954a64c617ac48f94f154f34e60346eac4d5b7a46d`)
computes `cart_count` from object word `+0x2334` and writes the enable
word for `0x14200005`, `BAGSORT 0x142000b7`, the ten `MYBAGITEM` cells
`0x1460003d..46`, `MYCART_BACK 0x1450006f`, `BUYITEM 0x14200070..73`,
`GIFTITEM 0x14200071`, `CHECKMONEY 0x14400075`, five cart-toggle codes
`0x1401b076..7a`, and four cart page digits `0x14500080..83`. Some
controls use `cart_count == 0` (they are visible on the entry face),
others use `cart_count != 0` (they are visible on the cart face).
`FUN_0x409bf0` writes the `+0x24` enable word; it does not paint art.

Rust today: `shop_bagsort_kept` (`paint.rs:1317-1319`) gates only BAGSORT
on `cart_count == 0`; `paint_shop_cart_totals` (`paint.rs:1838-1865`) gates
totals on a non-empty cart; `paint_shop_cart` (`paint.rs:1706-1760`) inks rows from
`self.shop_visible_cart()` and paginates by that vector's length. The
quiesced pair has one exe row and five rust rows with different page
counts, so the row vectors differ: that is cart state, not a missing
painter. `docs/shop.md` §`#886` already records the same verdict.

### 3.4 Set-item card figures and price row (S2, S3)

`FUN_0x49e8d0` (file `0x49e8d0`,
`ev_e9a4667b800d2069342d983e718eb577a2d78bb688ba28dbcc4b1a32597ba91c`)
rebuilds the four `0x1401d088 + slot` cards from the selected-item word at
`+0x148 + slot * 8`. For a non-empty word it resolves the item record and
calls `FUN_0x529230`; for an empty word it clears the cell and calls the
same function with a null argument.

`FUN_0x529230` (file `0x529230`,
`ev_dfbca1e74791aeafa77d80727f86744a06321c27c02028f3f842e6bcffcae2cf`)
writes the item name into slot 2, the experience/jam bonuses into slots 4
and 5 using the strings at file `0x64c6cc` and `0x64c6b4`, and the figure
through `FUN_0x52b480`. It does not write the price cells. Those are
written by `FUN_0x4a21a0` (file `0xa21a0`,
`ev_bec61aed24fe4e55a65cb2e57e9f4c8a61931361797bbb581913823e28d3cf96`):
when object word `+0x168` is zero and `+0x144` is zero it writes grid
`ITEMGEM`/`ITEMWON` cells; when `+0x168` is zero and `+0x144` is
non-zero it inks the four clone-price rows through `FUN_0x4a1eb0` and
`FUN_0x4a2010`. The other `+0x168` branch writes a different set-item
detail face. The price row is therefore state-gated.

Rust today: featured captions and price rows are painted by
`paint_shop_featured_captions` / `paint_shop_card_price_row`
(`paint.rs:1628-1659` and `paint.rs:1600-1626`), selected by
`shop_entry_catalog()` in `crates/o2jam-shell/src/screens/game/mod.rs:8819-8840`.
That path does not consult the exe's `+0x168`/`+0x144` face words, so it
can ink prices on a face where the exe leaves them dark. The figure phase
is animation-state (capture class), already described in `docs/shop.md`
§`#886`; no geometry fix.

### 3.5 MESSAGE sheet buttons (S4, S5)

The sheet is `Dialog_Question1` (`0x22000000` family). `FUN_0x4346f0`
(file `0x4346f0`, `ev_adfd8d48b7873606dacc7ac147577f5a12e6ca610087b73bbcda02c4a0b0ca72`)
fetches backdrop `0x22100001`, OK `0x22200002`, CANCEL `0x22200003`,
offsets each control by `(+0xbc, 0)` through `FUN_0x42ff30`, and writes
their rects. `FUN_0x42ff30` (file `0x42ff30`,
`ev_f880d925a2c1c261acd79fa8252e25e7d3a43e8c44d08f13af4cb8f055a7b8fc`)
is an x/y offset helper, **not** a frame setter — this corrects the
earlier reading of the `0xbc` immediate. `FUN_0x434580` (same batch) maps
OK/CANCEL presses to local outcomes through `FUN_0x478c90`; no rest-frame
write was found in these functions. The RTTI string `.?AVCDQuestion1@@`
at VA `0x67d830` (file `0x27d830`,
`ev_c9e91aa317f558aa114135e8792c47c2485bf7bd6b0de49860ad91ad834f7d8f`)
has no resolved xrefs (`ev_9cb3df16f075bde9c3fa0db19405d359f80386927612600cf329da849ca12f87`),
and no procedure name contains `Question`
(`ev_665738a4e9391fa2e57395b2b761ac00b8aed23f58d4d3dc2b8719522b74275f`).
So the gray rest face is a quiesced-frame fact plus the existing baked
plate rule (`paint.rs:3560-3592`), not a new static frame pin.

Rust today: `paint_question1_staged` (`paint.rs:3821-3836`) calls
`paint_dialog_scene`, which paints every distinct anchor record of a
staged body (`paint.rs:3480-3501`, `paint.rs:3581-3600`). The sheet's
button bodies are not in `BAKED_PLATE_BODIES` (`paint.rs:3569-3577`), so their blue records cover
the baked gray faces.

### 3.6 Bottom-right alarm popup (S7)

The shop entry loops over `0x145000b9 + index` (two controls: alarm lamp
and popup) and calls `FUN_0x42f5a0(3)` on each (`ev_1b078fdd166e7bef4b979f5a928a41d7395d8a8bef4facb2e45deff0ba084197`). In
case 3 `FUN_0x42f5a0` (file `0x42f5a0`,
`ev_18bc8931cb83626bc59058b7808bd6616945b6c152a6ae00fbaf7fe88d83c6c3`)
sets the current frame to zero. `FUN_0x42f870` (file `0x42f870`,
`ev_68329752755f418ea0536d7566937e50be85b84ecea30ec99720bb1dc9e4cce6`)
maps frame `n` to surface `n - 1`; frame zero therefore selects surface
`-1` and paints no body. The baked purple lamp in `State_ItemShop.ojs`
remains. A full `.text` search finds no direct immediate for `0x145000ba`
or `0x145000b9` (`ev_b5aef4e3c92efe6e1c0bab41bd410311ae58d0e1597869431fed5ace52151ad7`,
`ev_ee3e0540b49594fdac6dd954fc2158c2ea1bc31d738c0fcfde6741196730c80d`):
both codes are reached by the computed `base + index` loop.

Rust today: `stage_shop_baked_chrome` detaches five codes
(`crates/o2jam-shell/src/staging.rs:1002-1017`) including `0x1450_00b9`
but **not** `0x1450_00ba`. The popup body `Messenger_Alram_PopUp.ojs`
therefore still paints; its white `...` record is the diff at
`(612,543)-(649,575)`. The prior lane detached the lamp body but left
the popup body. Fix proposal T13.

## 4. Limits

- Caller and xref queries use Ghidra's resolved references. Unresolved
  computed or indirect calls are not represented; absence of a resolved
  caller is not proof of absence of execution.
- Byte searches covered initialized memory for exact immediates. Both
  alarm-code searches were empty because the exe computes those codes
  from a base plus index; the loop in `FUN_0x4984d0` is the positive pin.
- The 3007 → chat-append edge and the shop wallet/total writers stay
  OPEN. Every OPEN item in this dossier has a named next evidence step;
  no guess was promoted to a pin.
- All pair observations are from the v6 quiesced batch. No new capture,
  wire tap, or memory read was produced by this statics-only lane.

## 5. Proposed fixes and verification legs

One ready-to-file ticket text per region is in
`~/scratch/lanes-2026-11-25/findings-statics.md`. The executable fixes
proposed here are T10 (set-item price state gate), T12 (MESSAGE button
rest face), and T13 (shop alarm popup rest frame). The other regions
need wire replay or are presentation/content verdicts; their tickets
name that leg instead of a paint edit.

Every proposed Rust guard is scenario-named, red-first, and carries a
live caller chain from `main.rs::run_play` or `session_walk` to the
guarded painter. Data-present guards skip cleanly without `install-root`.
Seam-only assertions are labeled `seam:` and never close a player-visible
ticket. Structural parity is measured with the v6 quiesced-pair recipe;
0.99 is the gate only where a scenario-matched pair exists.

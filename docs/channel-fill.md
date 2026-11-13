# Spriteless-block fill: `STATE_CHANNEL` (7.55) — no-fill rule PINNED

Which of dialog-default / parent-background / window-clear / first-child-art
paints behind a block that names zero sprites. Short answer: **none — the
exe stages zero pixels behind art-less states and no fill call is
evidenced, so the shell paints nothing and leaves the frame untouched.**
No disassembly is reproduced; offsets are file offsets unless marked VA
(VA = file + `0x400000`). New offsets below are offset-only byte-scan
findings (no new REA dossier filed).

## The block (byte walk, read-only)

`Interface1.opi` slot 1595 (`ControlList_Interface.txt`, data offset
74438614, 90472 B, 1979 CP949 lines): `STATE_CHANNEL` at lines 12–61,
header `0x27 0X01`, bare `BOUND` line (no stem — the only full state
without one), 50 `IDC_` entries including self, one `SET 0X0A` group of
10. **Zero quoted sprite strings in the whole block** — no entry names
art, so recipe step 6 (`docs/opi-pattern.md`: sprite-less entries draw
no art) leaves every control art-less. First child
`IDC_BACK_SCHANNEL_1 0X12100001` is the would-be background entry, but
with no sprite and no bind it paints nothing — "first-child art" is
refuted as the fill.

## The host (no `CStateChannel`)

RTTI holds no `CStateChannel` class (lifecycle §6, unknown #2). The stager
is the generic state-open `0x48A060` (file `0x148A060`,
`ev_9e526b62…`): block = `0x762B44+0xF80+(n−1)×0x118`, bind loaded only
when the stem field is non-empty (bare `BOUND` → skipped, so all rects
miss to all-zero), one control per entry gated on `code & 0xFF0000 ≠ 0`
(`0x48A134`). Self entry `0x12000000` gates out; the other 49 stage
art-less controls. The render loop `0x5DEC30` (file `0x1DEC30`,
`ev_ff581cfa…`) draws stored controls only — the state contributes zero
pixels. Visible ink rides code-mounted art outside the block (see "The
bar fetch" below; `DIALOG_CHANNEL_INFO`/`CDChannelInfo` + `CCBtnChannel`
per lifecycle §6).

## The bar fetch (code-mounted, host unpinned)

`Dialog_ChannelSelectBar.ojs` is stored at file `0x247b84`
(VA `0x647b84`) in a sprite-name table, and exactly one `LE32` reference
to that VA exists in the image: file `0x8de84` (VA `0x48de84`). The
reference sits inside the builder starting at file `0x8dd10`
(VA `0x48dd10`, nearest function prologue below; next prologue above at
file `0x8dfc0`), amid `ControlList`-parent (`0x762B44`) traffic at files
`0x8de76`/`0x8dea1` — a state/dialog builder fetching the bar body by
code alongside parent-array work. It is ~63 KB below the dialog-open path
(`0x4ED650`, file `0xed650`), so the bar is **not** mounted by `0x4ED650`.
No `DIALOG_`-prefixed literal exists anywhere in the image (0 hits), and
`DIALOG_CHANNEL_INFO` is absent as a string while `CDChannelInfo` survives
as an RTTI name (file `0x27dc10`; corrected 2026-11-11: the TD name at
file `0x27dc04` is referenced by two vftable records, files
`0x247150`/`0x247160`, so the class exists in code — see
`docs/overnight-evidence/e1-channel-dialog-trigger.md`; the runtime
allocator stays OPEN) — dialog staging is index-driven by the constant
`0x1B` in the stage virtual, not by block code, so the bar's dialog
host (`DIALOG_CHANNEL_INFO`/`CDChannelInfo` per lifecycle §6) stays
unpinned and the shell stages no bar body.

## Fill candidates — no-fill pinned, hosts stay OPEN

- **Dialog default color**: `CDialogBase` / `CWindow` survive only as
  single RTTI names (files `0x27d7f4`/`0x27d7d8`); no `Window.cpp`,
  `Dialog.cpp`, `CWindow::`, or `CDialogBase::` strings exist (0 hits
  each), and ui-draw OPEN 3 (dialog-relative rects) is untouched. No
  default-fill call evidenced — stays OPEN.
- **Parent background**: state-open mounts no parent fill; the screen-level
  consumer of the child array is unpinned (claims-render OPEN 2).
  Stays OPEN.
- **Window clear color — PINNED to no-fill**: no backbuffer-clear call is
evidenced. Byte scans return 0 hits for `FillRect`, `fill`, `Fill`,
  `Paint`, `OnDraw`, `BackBuffer`, and `Clear`; `GetDC`/`ReleaseDC`/`SetRect`
  appear once each in the import region (file `0x27b92`–`0x27ba8`, window
  setup alongside `SetWindowPos`) and `BitBlt`/`BltFast`/`Flip` only as
  DirectDraw/Direct3DX error strings (files `0x24f032`–`0x250b43`) — no
  paint-time fill vocabulary. Display init (`0x413030`) pins mode/Flip
  vocabulary only (claims A38/A39, `ev_c7d5a15f…`). Combined with the
  zero-pixel staging above, the rule is: the exe contributes no fill
  behind art-less states; the presenter preserves the stale frame
  (pinned in OPEN 1 below), and the shell paints nothing.
- **First-child art**: refuted above (no sprite, no rect).

## Shell rule (`crates/o2jam-shell/src/screens/paint.rs`)

`main_scene_sprites(Channel)` returns `[]` (the block names no art);
`SceneArt::paint_backdrop` with no figures paints nothing — the frame is
left untouched and only the live list, title, and cursor layer on top.
The old flat navy is gone: it was a documented placeholder, and the
pinned no-fill rule replaces it. Tests:
`spriteless_channel_block_stages_no_scene_art` (shape-faithful bare-`BOUND`
block: image/button/text/`SET` code classes, zero sprites → no scene art),
`empty_channel_art_paints_no_fill` (empty stage: untouched corner, no
navy, live title ink), `staged_empty_backdrop_leaves_every_frame_untouched`
(patterned backbuffer survives `paint_backdrop` on every screen),
`channel_bar_art_layers_behind_the_live_list` (entry-less
`DIALOG_BAR_SPRITE` art at its record anchor under list and cursor, no
fill), `empty_art_falls_back_to_flat` (staged-empty scenes carry no fill;
bare `paint_screen` cards and the result backdrop keep their own
missing-archive fallbacks).

## Chat row feed (`TEXT_SWAIT_CHAT 0x1640003C`, `SET 0x00031000`) —
TEMPLATE AND LIVE PATH PINNED

The row template resolves only through the staged list object; the twin
waiting-room feed functions name it by lookup, never by code immediate.
Full-image LE32 scan: `TEXT_SWAIT_CHAT` has zero refs; `SET 0x00031000`
has one unaligned data-coincidence hit only (file `0x3324b3`, spanning
dwords `0x0`/`0x310` in `.data` — not code).

- **Feed twins, bodies pinned**: `0xb9a10` guards the user pair
  (style `0x13`, code `0x1602102a`), `0xb9a60` the admin pair (style
  `0x14`, same code). Each looks up `LIST_SWAIT_CHAT 0x1680002B`
  through control lookup `0x48BCF0`, null-guards the hit, and hands
  the row string to the list row store `0x50fcc0` (store call sites
  files `0xb9a55`/`0xb9aa5`; the list rides `ecx`, the string is
  loaded into `edx` then pushed). The row string arrives already
  composed as the third stack argument — the twins compose nothing.
  Both live in the waiting UI region beside the chat-view stager
  `0xb8d00` and the 3011 send sites.
- **Upstream — dispatch link REFUTED**: the twins' only direct callers
  are four adapters (`0xac5b0`/`0xac5e0`/`0xb2dd0`/`0xb2e00`, call
  sites files `0xac5c6`/`0xac5f6`/`0xb2de6`/`0xb2e16`); the six VAs
  (twins plus adapters) appear as data only inside three isolated
  10-entry arrays (files `0x249710`/`0x2499f0`/`0x249b28`, each
  opening with `0x489ee0`). The arrays interlock per slot — slot 5
  adapters call stager `0xb8d00` (sites `0xac57e`/`0xb2d9e`), slots
  6/7 call the twins, slot 9 calls caption helper `0xb9ab0` (sites
  `0xaef93`/`0xb554a`) — but no reader exists for any array:
  exhaustive LE32 scan returns zero hits for each base as immediate,
  displacement, vftable member, or `.data` initializer; no window-scan
  base within `±0x1000` indexes into them (they float inside the
  CP949 string pool); nothing outside the cluster calls any listed
  handler; and `O2Net32.dll`/`Player.dll` carry no twin, adapter, or
  array address. The packet path never reaches them. Correction
  below: the arrays are state-class vftable interiors read by virtual
  dispatch, which that scan shape cannot see.
- **Packet side, pinned**: the receive stubs push a 4-dword record
  `(0x12, opcode, record, 0)` onto the parent global `0x762B44` into
  dispatcher `0x478d50` (file `0x78d50`, 160 B, 122 direct call/jmp
  sites),
  which enqueues it via `0x47eaf0` (file `0x7eaf0`, its only caller)
  at `this+0x3c` — the subindex is the constant `0x12`, not a slot
  number. Posts: 3012 at file `0x15e601` (`ev_aba36a3a…`, stub fn
  `0x15e430`..`0x15e740`), 3013 at file `0x15f051` inside the shared
  stub fn `0x15eca0`..`0x15f210` (reached from fn `0x158080`; it also
  posts `0xbc4` at files `0x15ef4b`/`0x15f110`/`0x15f181`). The +0x3c
  queue IS consumed — by the parent pump (vftable `0x647920` slot 5,
  `0x476020`, file `0x76020`; vftable store file `0x75433` in ctor
  `0x4753a0`): dequeue `0x478df0` (file `0x78df0`, call file
  `0x76110`), subindex switch (file `0x7612b`), subindex `0x12`
  making the virtual call `[[parent+0xf6c]+0x40]` (files
  `0x7615b`–`0x76184`) over the four record dwords. That arm joins no
  row store: `CStateWaiting`'s slot-16 target `0x4b7c60` (file
  `0x249b18`) is the control-code notify switch (keyed at files
  `0xb7cb8`–`0xb7d56`, no `0x12` case), and the row text rides the
  +0x17c string queue pinned below. The store keeps no vftable
  presence.
- **Row composition, enqueue side**: inside the shared stub fn, an
  imported printf (IAT `0x63f440`) formats `[%12s] %s` (format VA
  `0x64d8d4`, file `0x24d8d4`; call file `0x15f076`) from a
  sender-shaped and a text-shaped local into an out-buffer — bracket,
  sender right-aligned to width 12, bracket, space, text; the
  consuming record is pinned below (the +0x17c string queue).
  User-side strings are built by
  string helpers (files `0x1f1aa2`/`0x1f1bc0`/`0x1f34ff`/`0x1f3f0c`).
  The parent getter `0x478a00` gates both branches: a return of 5
  selects the emote-keyword branch (pinned in the emote-gate section
  below), a return of 0xa precedes the bracket format. The trailing
  seat byte rides only the +0x3c notification post (subindex
  `<< 0x10` | seat, files `0x15e5ef`–`0x15e5f5`), never the row queue
  below.
- **Live row path — PINNED** (offset-only scan, no new dossier): the
  bracket line is consumed through the parent's string queue at
  parent+`0x17c`, not the 16-byte-record queue. Enqueue: the 3012
  block (fn files `0x15e9e3`..`0x15ec97`, compose printf at file
  `0x15eb28`) calls `0x479010(parent, &line, 0xbc4, flag)` at files
  `0x15ec4d` (flag 0) and `0x15ec68` (flag 5); the 3013 stub fn calls
  it with `0xbc5` at files `0x15f1a7`/`0x15f1c2` (the emote-gate
  section's level-0/level-5 rows). Helper `0x479010` (file `0x79010`)
  packs `(flag << 0x10) | opcode` (files `0x79053`–`0x7905c`),
  string-copies via `0x408940` (file `0x7906c`), and pushes a
  `0x10c`-stride record into parent+`0x17c` through `0x457f40` (file
  `0x79084`); accessors `0x479350`/`0x4790d0` (files
  `0x79350`/`0x790d0`) count and pull. Consume: the `CStateWaiting`
  poller `0x4b9dc0` (file `0xb9dc0`; RTTI `.?AVCStateWaiting@@`, COL
  VA `0x6673ec`, vftable VA `0x649ad8` with the tick at slot 10 =
  vft+`0x28`, file `0x249b00`; ctor vftable stores files
  `0xb5d65`/`0xb5e37`; direct callers files `0xaf8ea`/`0xb5b0a`
  inside waiting drivers `0x4ad940`/`0x4b42d0`) counts (call file
  `0xb9e82`), pulls one record plus out-line (call file `0xb9ea8`),
  and switches on `packed & 0xffff` (files `0xb9ec8`–`0xb9f26`):
  `0xbc4` gated on `packed >> 0x10 == this+0x10` (files
  `0xb9f63`–`0xb9f78`) appends with color `0xdedede` (call file
  `0xb9f8c`); `0xbc5` the same gate with `0x71e6` (file `0xb9fad`,
  call file `0xb9fbf`); sibling arms take notices `0x7e3`/`0x7e2`/`0`
  (files `0xb9f2b`/`0xb9f47`/`0xb9ff6`) — system and memo lines ride
  the same queue (enqueue sites files `0x15abdd`..`0x15b1a9`,
  `0x1633c2`..). Sibling states poll the same queue for their
  screens (`0x4e9970` calls files `0xeaa6e`/`0xeaa92`). Write: the
  append helper `0x4bb6c0` (file `0xbb6c0`) looks up
  `LIST_SWAIT_CHAT 0x1680002B` through `0x48bcf0` (push file
  `0xbb6c9`, call file `0xbb6d1`), null-guards (file `0xbb6d9`), and
  calls the row writer `0x510730(list, &line, color)` (file
  `0xbb6ea`). Mutation: `0x510730` (file `0x110730`, body to
  `0x1173f`) walks the staged list's row vector (list+`0x198`,
  stride `0x6a5c`, files `0x10752`–`0x10783`), takes the cursor row
  at list+`0x1ac` (file `0x107c0`), addresses begin + cursor×
  `0x6a5c` (files `0x10817`–`0x10826`), stores the color into the
  row's color slots (row+`0x68b0` array, files `0x10895`–`0x108aa`),
  and copies the line into the row's template text cells. The live
  writer is this helper, NOT the twins' store: `0x50fcc0` keeps its
  six pinned callers, none on this path, and `0x510730`'s only other
  callers are files `0x665c4`/`0x8f38a`.
- **Array-reader correction**: the three "isolated" handler arrays
  (files `0x249710`/`0x2499f0`/`0x249b28`) are interior slots of
  state-class vftables, not free-floating tables — `CStateWaiting`'s
  vftable spans files `0x249ad8`–`0x249b4f` (COL pointer file
  `0x249ad4`, VA `0x6673ec`), so the third base file `0x249b28` is
  its slot 20, with the twins at slots 26/27 (files `0x249b40`/
  `0x249b44`) and caption helper `0x4b9ab0` at slot 29 (file
  `0x249b4c`). Such tables are read by object-pointer virtual
  dispatch (`[obj]` then `[eax+slot*4]`), which a base-immediate
  LE32 scan cannot see — the earlier "no reader exists" verdict was
  a scan-shape limit, not a dead-code proof. The twins are still
  bypassed by the live row path above.
- **Sibling arms**: `0x62840`/`0x628d0` (codes `0x86021041`/
  `0x86022046`, cached list in `ecx+0x528`) and `0x90690`/`0x906f0`
  (`LIST` `0x13800014`) share the store and the same body shape, have
  zero direct callers, and sit in their own arrays (files
  `0x2469e4`/`0x247c00`; the room array's slot 9 overruns into string
  data — nine real entries) with no references in a `±0x40` window.
- **Array iteration, pinned**: the row store `0x50fcc0` (file
  `0x10fcc0`, `0xa70` B) materializes rows by indexing the list's own
  vector (`this+0x198`, `imul` index × `0x6a5c`, checked-iterator
  guards) — the template is consumed through the staged list object.
  No channel/waiting row control is code-addressed by immediate.

### Emote keyword gate (`/` prefix, parent dialog id 5) — PINNED

Where: the shared stub fn `0x15eca0`..`0x15f210` (central S→C
dispatcher `0x158080` dispatches into it at call site file `0x158626`;
dossier `ev_fa0a499a…`). Preconditions, both required: the text-shaped
local starts with `/` (helper `0x5f3f0c` file `0x1f3f0c` = CRT
`strncmp`; literal `/` at file `0x24bd18`; compare file `0x15ed07`) and
parent getter `0x478a00` returns 5 (call file `0x15ed21`; getter
dossier `ev_931c9a4e…`). Getter semantics: reads the child pointer at
`parent+0xf6c`, calls its vtable slot 3 (`+0xc`) id-getter, and echoes
that id only when it equals `parent+0x50`, else 0. The 85 getter call
sites compare ids 2..`0xd` — a UI-child id space disjoint from the
`STATE_*` `0x12`+ code space, so 5 names the mounted dialog hosting
this chat feed and 0xa the bracket-format dialog; which named dialog
carries each id stays OPEN.

Keywords — **twenty entries, not nine** (the earlier "nine-entry"
range `0x249dac`..`0x249dcc` missed `반짝` at `0x249d9c` and the ten
phrase strings): the stub builds a 20-slot stack pointer array at
files `0x15ed2f`..`0x15edf4` in record-payload-index order. Offsets
are file offsets of the CP949 string literals:

| idx | file | word | idx | file | word |
|-----|------|------|-----|------|------|
| 0 | `0x249dcc` | `!` | 10 | `0x249d94` | 안녕 |
| 1 | `0x249dc8` | `?` | 11 | `0x249d90` | 고 |
| 2 | `0x249dc0` | 사랑해 | 12 | `0x249d88` | 레디 |
| 3 | `0x249dbc` | `321` | 13 | `0x249d80` | 좋아 |
| 4 | `0x249db8` | `~` | 14 | `0x249d78` | 싫어 |
| 5 | `0x249db4` | `!!` | 15 | `0x249d70` | 흐흐 |
| 6 | `0x249dac` | 미워 | 16 | `0x249d68` | 하하 |
| 7 | `0x249da8` | `7` | 17 | `0x249d60` | 크크 |
| 8 | `0x249da4` | 꽃 | 18 | `0x249d5c` | 윽 |
| 9 | `0x249d9c` | 반짝 | 19 | `0x249d58` | `...` |

Match behavior (loop bound `0x14` at file `0x15ee33`): helper
`0x5f34ff` file `0x1f34ff` = CRT `strstr` — the keyword must appear
anywhere in the text after the slash. On the first hit, index `i` is
kept (file `0x15ee65`) and the sender's seat byte resolves: `strncmp`
of the sender against the my-name global `0x77cd05` → byte `0x77cd04`
(file `0x15eeb8`), else a scan of 8 room slots (stride `0x2190`, name
fields at `0x77ee95+j×0x2190`) → the matched slot's first byte at
`0x77ee94+j×0x2190` (file `0x15ef01`). Then exactly one record is
enqueued via `0x478d50` (post file `0x15ef4b`): subindex `0x12`,
opcode `0xbc4` (3012-shaped), payload `(i<<16)|seat`, arg 0 — an
emote event carrying keyword index plus seat, **not** text. When a
seat resolved, the fn exits (file `0x15ef6d`→`0x15f063`): no row post,
no network send — the emote consumes the line. No seat match, or no
keyword match (counter reaches 20), falls through to the generic row
post at file `0x15f051`: opcode `0xbc5` (3013-shaped), payload
`seat|0xffff0000`.

Non-slash text (handler at file `0x15f068`): the pinned `[%12s] %s`
bracket format runs first. Getter 0xa → admin branch: byte gates
`0x7789e6`, `0x77cd04` vs the split target byte, `0x77cd33 >= 4`, and
`0x780fc0+seat×0x2190 == 1`; posts 0xbc4 records with
`0xff000000|target` (files `0x15f110`/`0x15f181`) and a level-0 row
via `0x479010`. Otherwise a level-5 row via `0x479010`, and getter 5
additionally calls classifier `0x55e430` (file `0x15f1f8`), which
byte-walks the text (≤ `0xcb` bytes, jump table VA `0x55e62c`) and
posts a 3012-shaped record (file `0x15e601`).

Send-side twin (chat-view stager `0xb8d00`, which owns the three
3011 send sites; dossier `ev_52712b4b…`): number keys 1..9,0 compose
`/%s` lines (format file `0x249d54`) from the same keyword table,
write them into the chat edit (`this+0x184`, fn `0x518000`) and send
3011. Arm A (flag `[this+0x2a0]≠0` and `[this+0x29c]==0`; array fills
files `0xb9464`..`0xb94c8`, digit format call file `0xb955e`) maps digits
to indices 0..9; arm B (both flags ≠0; fills files `0xb95f2`..
`0xb96b0`, format call file `0xb96ec`) maps digits to indices 10..19.

OPEN: which named dialog carries id 5 / 0xa; what the slot-16 dialog
dispatch renders from the emote record (avatar animation vs a chat
row; the dispatch is pinned: parent pump `0x476020` subindex-`0x12`
arm calls `[[parent+0xf6c]+0x40]`, files `0x7615b`–`0x76184`); which
opcode(s) the dispatcher's map binds to the stub;
the classifier's byte-class → payload encoding.

Pin: `OTwo.exe` 5029888 B, sha256
`83a04541…c823b`, offsets file-based, found by offset-only byte scan
(Python 3.14.9, capstone 5.0.7, GNU objdump 2.47 — no new dossier).
Shell rule unchanged: live 3012/3013 packets only, newest visible rows
stack from the list origin. The pinned live path now grounds
the row look — bracket plus right-aligned sender from the compose
printf, user color `0xdedede`, admin color `0x71e6`, gate field
`this+0x10` — while the shell's `[ADMIN]` prefix and row colors stay
stand-ins until `0x510730`'s row layout is mirrored (the matching OPEN
sentences in `net-protocol.md`, `rooms.rs`, and `paint.rs` are owned
by other docs/code and untouched here).

# Channel dialog is a modal over the planet board (2026-11-09 captures)

METHOD CORRECTION (2026-11-10): the channel dialog's placement was in
the archive all along. `Dialog_ChannelInfo.bnd` (Interface1.opi slot,
27 rects, `FFFFFFFF`+`6+16·27` exact) pairs to the `0x8C` block
entries by code ordinal `(code & 0xFFF) − 1` — every entry pairs, and
its rects 2-5 equal the exe-pinned row rects from the live memory read
EXACTLY (4/4), which proves the pairing. Rule going forward: for every
dialog, try the stem-named `.bnd` by ordinal FIRST; the block records
stage zero rects because the bnd is applied at dialog build, not
copied into the block. The rig (pixel measurement / memory reads) is
reserved for what no file answers: the pager (absent from this bnd),
runtime flags, and interaction semantics.

Key dialog geometry (bnd-pinned, row match exe-verified): sheet
(0,0)-(800,466); X button (541,151)-(589,189); row bar `CH_Back`
347x41 mounted at row origin +1 inset (row 0 → (217,225); its bar-
local children: digit slots (50,11)-(63,29)/(63,11)-(76,29), count
plate (90,12)-(275,31) 185x19, enter face (285,11)-(329,32) 44x21 —
face X 502 = 217+285 and face Y 236 = 225+11 both exact against the
reference measurements); scroll track (573,213)-(594,429) 21x216 with
bar-local arrows/body; emblem re-statements (313/206/87/614/510,
identical to `State_Planet.bnd`); TEXT strips (y125-138); board EXIT
(750,561)-(788,599) 38x38.

VERIFICATION PASS (2026-11-09 second session, `~/scratch/channel-geom2/`):
- The code-fetched `Dialog_ChannelSelectBar.ojs` (348x44) is
  LIVE-CONFIRMED mounted at dialog+`0x8C` (sprite record, 348/44 at
  +`0x170`) and paints on the HOVERED row at the row rect — the list
  highlight. Distinct from the per-row `CH_Back` bar (347x41).
- Pixel measurement agreed with every bnd-pinned value within ±2 px
  (brightness-run tolerance): face (501,row_top+10) vs bnd (502,+12);
  X core (546,155,37x32) vs bnd (541,151,48x38); scroll (575,212)
  vs bnd (573,213). Bnd values stand as authoritative.
- Fallback chrome wells (live-measured, baked in the staged bar art):
  number well (225,228) 71x32 (the old 205,227 pill was 20 px off);
  track well (298,229) 195x32.
- PAGER: RESOLVED NEGATIVE 2026-11-10. The bottom strip (y~439-466)
is dialog-sheet rim chrome (brushed metal y439-461, groove 461-463,
streaks below) + the scroll column's down arrow — the decoded sheet
art (`Dialog_ChannelInfo.ojs` bottom band, colorkey 0x7D5F) bakes no
pager glyphs, the bnd names no pager rects, and the block's
`IDC_BUTTON_SCHANNEL_LEFT/RIGHT` (`0x12200005/06`) stage zero rects
and are statically unreachable (dead authoring vocabulary). The 7.55
channel dialog has NO pager; list navigation is the scroll column
alone. The earlier "pager strip" capture measurement was rim-ink
misread.
- Bare-board reference-measured rects: promo nav cluster (345,509)
  107x27; OPTION (712,562); EXIT (753,562); MENU back-arrow (130,562)
  31x32 — all within 2-3 px of the bnd-pinned origins where both exist.
- Control-array search MISS: no `0x8Cxxxxxx`/`0x12xxxxxx` records in
  heap dumps; face/scroll positions are computed at paint time.
  dialog+`0x1F8`..+`0x248` hold channel-server records (port 15010).

Live captures (`~/scratch/button-unblock/`, recipe `walk_planet.py`)
show the channel dialog opening **over the bare planet board** at boot
and closing via its X button (native (557,195), inside the measured
X zone x560-620 modulo button chrome) to reveal the emblem strip +
promo carousel + MENU bar beneath (`docs/planet.md` "Real board
structure"). The dialog sheet's baked top strip (H/M/C/QUEST/GAME
icons) mirrors the board's own emblem buttons, which pair exactly to
the pinned `State_Planet.bnd` rects — DRUM rect hosts the dimmed QUEST
emblem, FREE rect the dimmed TUTORIAL one.

VERIFIED 2026-11-09 (`~/scratch/planet-verify/findings.md`): both
dialog-up and bare-board phases keep `CStatePlanet` mounted at
`parent+0xF6C`; the phase flag at **`parent+0xF64`** (1 = dialog, 0 =
bare) replaces any state change — `0x12` is never current. Emblem
clicks switch server channel families (EASY active at boot ~21 s
after login; NORMAL switches with a 1-2 s reconnect; HARD is
server-gated behind a NORMAL-only popup; re-selecting the active
family reconnects without re-fetching, rows stay blank). A native
X click at (577,171) closes the dialog. One entry-face click
(526,248) on a populated dialog drives `03EC/0FBE/07D0/07D2/13A4`
and enters `STATE_ROOM`.

# Live memory read (2026-11-09) — geometry and host EXE-PINNED

Dynamic read of the live 7.55 process under Wine (winedbg stopped-state
memory reads; no input needed). Full method, dumps, and session log in
`~/scratch/channel-rects/` (`findings.md`). Pins below supersede the
reference-measured stack anchors wherever they differ; the capture
measurements stand corroborated.

- **Host ANSWERED**: the parent global `0x762B44` holds the `CState`
singleton (vtable `0x647920`). Its mounted child at `parent+0xF6C` is a
**`CStatePlanet`** object (vtable `0x647B24`, RTTI `.?AVCStatePlanet@@`)
— the Channel dialog host. The dialog carries the string
`State_Planet`. Which call passes the Channel block stays untraced, but
the hosting object is now pinned (this doc's earlier "Planet or
shared-base stages Channel" hypothesis resolves to Planet).
- **Block staging, live-confirmed**: the `STATE_CHANNEL` block (buffer
`0x029E2428`, code `0x12`) holds 33 records of stride `0x11C`, codes
`0x12100001`-`0x1250002B`; every rect is **zero** - the bare-BOUND /
zero-rect finding confirmed in the running process. The
`DIALOG_CHANNEL_INFO` block (buffer `0x02A39A30`, code `0x8C`, in the
parent's second descriptor table at `parent+0x1FE8` — corrected
2026-11-11; `+0x2000` names only the table region) holds 26 records
**with quoted sprite names** and zero rects; its art list matches the
mounted bodies below (`Dialog_ChannelInfo.ojs`, `ChannelInfo_Btn_Close`,
`CH_Back`, `CH_Num` x2, `CH_ConnectCount`, `CH_Enter`,
`Messenger_Main_Scroll_Up/Down`, `ChannelInfo_Scroll_Bar`, plus
`Planet_Btn_*` promo bodies - the same dialog object carries the planet
promo side at `dialog+0x84`, channel side at `dialog+0x8C`).
- **Row rects EXE-PINNED** (`CStatePlanet` dialog object at `0x018E7E20`,
array at `dialog+0x94`, format left/top/right/bottom):
  - row 0: (216, 224, 572, 275)
  - row 1: (216, 275, 572, 324)
  - row 2: (216, 324, 572, 375)
  - row 3: (216, 375, 572, 436)
  The four rects are the visible viewport slots (scrolled rows reuse
  them cyclically); the reference capture's `232+50i` text tops read the
  interior of these rects (+8 px text inset, pitch 50.3 average).
  Related scalars: `dialog+0xE8 = 0x16` (22 = server channel count),
  `+0x1EC = 0x1F` (31).
- **Entry-face inset EXE-PINNED**: the join face tops sit 12 px below
the row tops (face Y 236 = 224 + 12), matching the capture's 236
exactly. The face X origin (502) and the 48x24 face (4 px button chrome
around the 44x24 exe art) stay reference-measured - button view fields
not read (see BLOCKED).
- **Mounted art, exe-pinned sizes** (live sprite records, array
`0x02996B20`, stride `0x260`; height/width at +`0x170`/`+0x174`):
`Dialog_ChannelSelectBar.ojs` 348x44 (parse: 347x41),
`Dialog_ChannelInfo.ojs` 800x468 (parse: one 800x466 layer - the 2 px
delta is a runtime-surface difference, recorded, uncompensated),
`ChannelInfo_Btn_Close.ojs` 48x40, `ChannelInfo_CH_Back.ojs` 348x44,
`ChannelInfo_CH_Num.ojs` 16x20, `ChannelInfo_CH_ConnectCount.ojs`
188x20, `ChannelInfo_CH_Enter.ojs` 44x24 (parse: three 44x21 frames),
`Messenger_Main_Scroll_Up/Down.ojs` 24x20, `ChannelInfo_Scroll_Bar.ojs`
24x48.
- **BLOCKED**: button view fields `+0x1E4`/`+0x1E8` - no `CCButton`-
family objects located in heap dumps, and winedbg `cont`/breakpoints are
non-functional in this wow64 state, so the per-frame writers (file
`0x129088`..) and the pairing getter `0x548640` could not be
intercepted. Next path: repair `cont` (`pass`, 32-bit winedbg, or
`winedbg --gdb`), or locate the control-array consumer statically.

## Shell rule update (2026-11-10)

`crates/o2jam-shell/src/screens/paint.rs` now paints the Channel scene
on the pinned rects: `CHANNEL_ROW_RECTS` (the dialog+`0x94` table),
hit-testing against the same bands (tops inclusive, bottoms exclusive),
the enter face at row-top + 12, and the scrim spanning the pinned slot
bands. The interior chrome (name pill, hollow track) stays
reference-measured but re-bases on the pinned row tops (pill +3, track
+4). Face X/48x24 chrome stays reference-measured. Guards:
`channel_row_hit_maps_the_exe_pinned_bands` (game.rs),
`channel_row_hit_mirrors_the_pinned_row_rects` (paint.rs). The
reference-measured `232+50i` anchors are retired.

## OPEN

1. Presenter backbuffer policy behind art-less states — PINNED to
   stale (preserve, not clear): display init `0x413030`
   (file `0x13030`, `.\Display\Display.cpp` via
   `DirectDrawCreateEx`, fullscreen 800x600x16 per claims A38/A39,
   `ev_c7d5a15f…`) builds the device with no clear-color argument
   evidenced; `Flip`/`BltFast`/`BitBlt` survive only as DirectDraw
   error strings (files `0x24ffdc`/`0x250b43`/`0x24f032`), and byte
   scans return 0 hits for `FillRect`, `PatBlt`, `StretchBlt`,
   `ColorFill`, `DDBLT`, `fill`, `Fill`, `Paint`, `OnDraw`,
   `BackBuffer`, `Clear`, `clear`, `EraseBkgnd`, `WM_ERASE`, and
   `BeginPaint`. The GDI names that do exist (`TextOutA` file
   `0x27ba66`, `CreateSolidBrush` file `0x27baa4`, `SetRect` file
   `0x27bb92`, `GetDC` file `0x27bba8`, `DrawTextA` file `0x27bc8c`)
   are the import-region window/text toolkit, not a backbuffer
   clear. The shell matches: `paint_backdrop` with empty art paints
   nothing, `paint_backbuffer` never pre-clears, and
   `present_native_frame` fills only the window letterbox bars
   (`LETTERBOX` black) while copying the stale native frame
   verbatim. Tests:
   `present_native_frame_preserves_stale_pixels_verbatim`
   (`o2jam-render`), `present_clears_only_bars_over_stale_pixels`
   (`o2jam-shell`). The exact `Flip` call site stays open
   (claims-render OPEN 1).
2. Sprite-less control paint — PART-PINNED, caption source OPEN:
   `STATE_CHANNEL` stages 49 art-less controls (self `0x12000000` gates
   out at `0x48A134`, ev `9e526b62…`; bare `BOUND` skips the bind so
   rects are all-zero, but Draw ignores rects — anchor-based per
   ui-draw/claims-render). Factory `0x48B2D0` (mask `0x48B30A`,
   ev `1be2d784…`) covers all five channel second-bytes
   (`0x10` image ×1, `0x20` button ×9, `0x01` toggle ×10,
   `0x50` number/image-family ×20, `0x40` text ×10), and the
   `0x01`/`0x50` arms are now ctor-pinned by offset-only call-site
   join (no new dossier): toggle group `0x010000`, sub
   `code & 0xF000 == 0x1000` (compare file `0x8b6f7` → branch VA
   `0x48b797` → call file `0x8b7ba) → ctor `0x51e350` (alloc
   `0x70`); sibling `0x2000` (compare file `0x8b707` → VA
   `0x48b7ec` → call file `0x8b80f`) → ctor `0x524dd0` (alloc
   `0x70`); number `0x50` (compare `0x500000` file `0x8b3c6` →
   call file `0x8b577`) → ctor `0x50a820` (alloc `0x60`); text
   `0x40` (compare `0x400000` file `0x8b3b6` → call file
   `0x8b522`) → ctor `0x50f160` (alloc `0x198`), whose body
   stores vftable `0x64c1b4` (files `0x10f174`/`0x10f26c`);
   image `0x100000` → `0x4f5d00`, button `0x200000` → `0x4f6040`.
   Class→Draw join is offset-only (vftable slot 4 = base+`0x10`,
   COL→TD names at file `0x27e3a8`–`0x27e8e0`, no new dossier):
   `CCImageBox` `0x50a890` (vft `0x64bf54`, ev `cc0942ba…`),
   `CCButton` `0x4f60c0` (vft `0x64b95c`), `CCTextBox` slot 4
   `0x50f280` (vft `0x64c1b4`), `CCListBox` `0x50bbe0`, scroll
   `0x50e320`, plus `CCBtn*` Draws (`0x52a0e0`/`0x526ed0`/
   `0x520f70`/`0x523990`/`0x524430`/`0x5297a0`/`0x51ebe0`).
   Sprite-driven Draws call stagers `0x42fc70`/`0x42fc10`, rect
   getter `0x548640`, frame `0x42f5a0`, with zero GDI imports in
   range — art-less means zero pixels. No default box: `Rectangle`
   (IAT `0x63f058`, sites file
   `0x114daa`/`0x115274`/`0x1158f9`/`0x11625b`) and the
   `CreateSolidBrush`/`DrawTextA` cluster sit only in the
   tooltip/message region (file `0x1137b0`–`0x1167b0` around
   `CCToolTip::Draw` `0x5137b0`), unreachable from the channel
   arms. Text paint corrected: slot 4 `0x50f280` (file `0x10f280`,
   96 B to next prologue `0x10f2e0`) is a stager, not the painter
   — it calls base `0x50da40` (file `0x10f292`), `0x409c30`
   (`0x10f2b1`), and font-create `0x41c860` (`0x10f2c4`, format
   VA `0x64407c`), tests control `+0x4c`, stores control `+0x164`,
   with no entry-`+0x04` read and no `0x42cxxx` call. The real
   painter is slot 9 (vft+`0x24`) `0x50f320` (file `0x10f320`): it
   calls all four font helpers `0x42c8a0` (file `0x10f452`),
   `0x42c730` (`0x10f49d`), `0x42c360` (`0x10f4d2`), `0x42c650`
   (`0x10f509`) — the owners of the `TextOutA` IAT `0x63f074`
   sites file `0x2c532`/`0x2c7df`/`0x2c854` and `DrawTextA` IAT
   `0x63f468` sites file `0x2c6ec`+ — over the field set control
   `+0x164`/`+0x174`/`+0x178`/`+0x180`/`+0x184` (reads files
   `0x10f415`–`0x10f4f4`). Caption writers traced by offset-only
   byte scan (no new dossier; function bounds from prologue walk:
   ctor `0x50f160`, slot 4 `0x50f280` 96 B, slot 1 `0x50f2e0` 64 B,
   painter `0x50f320` 512 B, setters `0x50f680`/`0x50f6a0`/`0x50f6f0`):
   base Draw `0x50da40` (file `0x10da40`) is REFUTED — bounded scan
   touches only `+0x58`/`+0x4c`, never the family. Slot 1 `0x50f2e0`
   (file `0x10f2e0`) is REFUTED — guard only (calls base slot 1
   `0x50dae0` at file `0x10f2ea`, tests `+0x164` at file `0x10f2f2`,
   calls `0x41cb20`); single caller `0x50f97a` (file `0x10f97a`).
   Ctor `0x50f160` only zeroes the family (`+0x164` file `0x10f17b`,
   `+0x178` file `0x10f195`, `+0x170` file `0x10f1a2`) and sets
   `+0x174` to `0xffffff` (file `0x10f188`) — init, not caption.
   The pinned writers are the setter trio: `0x50f680` stores `+0x174`
   (file `0x10f68d`, 96 callers), `0x50f6a0` stores `+0x164` (file
   `0x10f6e0`, 37 callers), `0x50f6f0` stores `+0x178`/`+0x180`/
   `+0x184` (files `0x10f70a`/`0x10f716`/`0x10f722`, 38 callers).
   Dialog writers PINNED: dialog-open `0x4ED650` (file `0xed650`)
   calls `0x50f680` at files `0xeddb5`/`0xedf3b`/`0xedfbc`/`0xee299`/
   `0xee2b8`, `0x50f6a0` at files `0xedf57`/`0xedfd8`, and `0x50f6f0`
   at file `0xee2d6`; channel bar builder `0x48dd10` (file `0x8dd10`)
   calls `0x50f680` at files `0x8df10`/`0x8df1f`. Net writers
   PART-JOINED (sweep 2026-11-07, offset-only scan, no new dossier):
   slot 9 of the same three 10-entry `.rdata` handler tables that pin
   the 3012/3013 twins (files `0x249710`/`0x2499f0`/`0x249b28`, each
   opening with `0x489ee0`; slots 6/7 select user/admin; dispatch
   stubs push opcode plus subindex into `0x478d50`, first 3012 stub
   file `0x15e601`, `ev_aba36a3a…`) is a packet-side caption path
   into `0x50f6f0`. Table 1 slot 9 `0x4aef80` (file `0xaef80`)
   calls helper `0x4b9ab0` (site file `0xaef93`) and list-Draw
   `0x52af70` (sites files `0xaefb3`/`0xaefd3`, after lookups of
   `0x16015022`/`0x16015025`); table 2 slot 9 `0x4b5540`
   (file `0xb5540`) calls the same helper (site file `0xb554a`);
   table 3 slot 9 IS the helper `0x4b9ab0` (file `0xb9ab0`). The
   helper calls `0x50f6f0` at files `0xb9b67`/`0xb9bef`/`0xb9c19`/
   `0xb9c9c` over looked-up `TEXT` controls (`0x16400016`–18 plus
   index-looped rows and `0x1650001b`); `0x52af70` (file `0x12af70`)
   calls `0x50f6f0` at files `0x12b064`/`0x12b108` in its 0–11 row
   loop. Which opcode lands on slot 9 stays OPEN. Honest scope:
   the trio writes style fields (`+0x174` color, `+0x164` font
   handle, `+0x178`/`+0x180`/`+0x184` triple); string bytes reach
   captions only through the siblings `0x50f520` (`+0x60` buffer,
   352 B at file `0x10f520`..`0x10f680`, head adds `0x60` then calls,
   291 E8 callers full-image) and row store `0x50fcc0` (6 E8 callers
   exhaustive). String-to-field join (sweep 2026-11-07, offset-only E8
   scan, no new dossier): chat twins `0xb9a10`/`0xb9a60` call `0x50fcc0`
   at files `0xb9a55`/`0xb9aa5`, with four sibling row-store sites at
   files `0x628c1`/`0x6294f` (channel arm) and `0x906dc`/`0x9073c`
   (room arm) — row string (sender plus text, `edx`) into list rows,
   CLOSED; room-fill `0x491af0` (file `0x91af0`..`0x92300`) calls
   `0x50f520` at 12 sites (files `0x91df9`/`0x91e32`/`0x91f62`/`0x91fba`/
   `0x92005`/`0x92034`/`0x9207f`/`0x92108`/`0x92155`/`0x921a2`/`0x921d1`/
   `0x922d4`) with zero trio calls, beside the `Lv.%d:%s` format
   (VA `0x64816c`, file `0x91bee`) and room format pushes — pinned
   string-to-`+0x60` flow, trio-free; dialog-open `0x4ED650` (file
   `0xed650`..`0xee320`) calls `0x50f520` at file `0xeddf0` beside its
   8 trio sites — pinned string-plus-style join in one builder. Target of
   that sole `0x50f520` site PINNED (offset-only E8/LE32 scans, no new
   dossier): the receiver is the synthetic TEXT `0x1b400007` built just
   above — `0x11c` memset (file `0xedd5f`), code store (file `0xedd67`),
   create `0x48a620` (file `0xedd80`) — cached at `this+0x3f4` (store file
   `0xedd8b`), null-guarded (files `0xedd91`–`0xedd9e`), colored `0x9a9a9a`
   via `0x50f680` (file `0xeddb5`), then string-set (file `0xeddf0`, sole
   `0x50f520` site in the builder; SSO-tested c_str of the `[-0x160]`
   field, file `0xeddba`). The `0x400000` arm pins it to text ctor
   `0x50f160`, so the bytes land in the `+0x60` buffer the slot-9 painter
   `0x50f320` reads: the join paints. Host correction: `0x4ED650` is not
   a dialog opener. It selects archive slot 2 (file `0xed681`,
   `Playing1.opi`), mounts eight `Playing1.opi`-only bodies
   (`Playing_Exit.ojs` VA `0x64b450` at file `0xed6c8`, plus chat-window,
   time-bar, bp-bar, hottime, game-over, and maxcombo bodies — all present
   in `Playing1.opi`, zero in `Interface1.opi`), shares the synthetic-`0x1b`
   idiom with playing applier `0x4F1140` (same push at file `0xf11a1`),
   has zero direct E8 callers, and sits at table VA `0x64b404` (file
   `0x24b404`; ctor-installed starts nearby at VA `0x64b38c`/`0x64b39c`,
   refs files `0xeb64a`/`0xeb654`/`0xeb83c`/`0xeb846`). Owning playing
   class and runtime caption bytes (a `0x5f2450` format product over
   builder members) stay OPEN; `ui-loader.md` OPEN 2's "dialog-open" label
   is refuted, noted here. Net
   slot-9 chain is string-negative: helper `0x4b9ab0` (file `0xb9ab0`..
   `0xb9dc0`) 4 trio calls plus zero `0x50f520`, `0x4aef80` (file
   `0xaef80`..`0xaefe0`) zero of both, `0x52af70` (file `0x12af70`..
   `0x12b150`) 2 trio calls plus zero `0x50f520` — packet path writes
   style only, string source OPEN. So live packets are joined to TEXT
   style fields through one caller chain (`0x4aef80` →
   `0x52af70`/`0x4b9ab0` → `0x50f6f0`); the earlier candidates
   `0x43dcc4` (func `0x43dbc0`), `0x44c479` (func `0x44ac50`),
   `0x45ca8d` (func `0x45c560`) stay unjoined, and room-fill
   `0x491af0` reaches captions only via the pinned 12-site `0x50f520`
   flow above, not the trio.
   ControlList carries no caption literals by design (dialog-strings
   §2); the grade-string precedent (zero refs, claims B4) shows exe
   text can be unbound. Render loop `0x5dec30` (ev `ff581cfa…`)
   plus the child-array paint (`0x50dea0`/`0x50ddf0` → vtable
   `+0x24`, claims-render verdict 3) draw stored controls only. So:
   imageless image/button/toggle/number paint nothing; TEXT paints
   its stored runtime field or nothing — source PART-PINNED (setters
   + dialog path pinned, channel-TITLE join OPEN). Remaining (sweep
   2026-11-07, still OPEN): join one channel `TITLE` entry
   (`0x1240001E`–27) to a setter call site. Full-image LE32 scan
   returns 0 hits for each of the ten `TITLE` codes and for every
   other `0x12` channel code probed (buttons `0x12200002`/`0x1220001B`,
   image `0x12500028`, toggle `0x12011007`); the only `0x12` LE32 hits
   are the generic self-gate `0x12000000` (41 hits, rdata-range files
   `0x248591`–`0x2613a0`, not code). Button/pager/tab sweep 2026-11-09
   (offset-only LE32 scan, no new dossier): `0` hits for BACK
   `0x12200002`, RANK `0x12200003`, CONTESTRANK `0x12200004`, LEFT
   `0x12200005`, RIGHT `0x12200006`, CHOBO/JUNGSU/GOSU
   `0x1220001B..1D` — the full Channel button vocabulary is
   statically unreachable (virtual dispatch or cached pointers, per
   the array-reader correction above); pager/tab behavior needs
   dynamic reads. Neither control-lookup caller set
   (`0x48BCF0`: 644 E8 sites; `0x4337C0`: 452 E8 sites) carries a
   `0x12` push or mov immediate, so no channel control is looked up by
   code. Grouping every trio E8 site (96/37/38) by enclosing prologue
   finds no `0x12` push in any trio-calling function and exactly one
   SelectBar reference: bar builder `0x48DD10` (file `0x8DD10`), whose
   two `0x50F680` calls (files `0x8DF10`/`0x8DF1F`) sit in the
   `0x19400007`–0B TEXT loop of `STATE_PLANET` (planet, not channel
   `TITLE`) beside the SelectBar fetch (file `0x8DE84`). Channel
   `TITLE` targets are never code-addressed; captions, if any, must
   arrive via cached pointers or array iteration, both untraced.
3. Dialog host — RESOLVED 2026-11-09: the mounted child at
   `parent+0xF6C` is a `CStatePlanet` object (live memory read; see the
   "Live memory read" section above). The select bar's exact mount
   rect stays OPEN (button view fields blocked); `CDChannelInfo` exists
   as a class in code (two vftable records ref its RTTI name, E1
   2026-11-11) — dialog staging is index-driven (`0x1B`) through the
   `0x8C` block; which allocator creates the instance is OPEN.
4. `CWindow`/`CDialogBase` default fill, if any (ui-draw OPEN 3).
5. Loading bar/caption geometry — OPEN, negative result (placeholders
   kept): `MusicLoading.ojs` (`Playing1.opi` slot 39, 960,028 B) is
   one sprite-55 body, single 800x600 layer at anchor (0,0),
   colorkey `0x0`, raw 960000 B (`8+20+Σsize` exact) — no
   bar/caption sub-rect exists in the art. Re-walked read-only
   2026-11-08: `Loading_Sign.ojs` (slot 37, 29816 B) is three 134x37
   layers at (0,0) (`colorkey 0x7D5F`, animation frames, not a
   shell); `Wait_Downloding_Bar.ojs` (Interface slot 1196, 938 B)
   is one 65x7 layer at (0,0) (`colorkey 0x0`); its host
   `Wait_Downloding.ojs` (slot 1195) is 100x45 at (0,0);
   `Payment_BackLoading.ojs` (Interface slot 362, 38466 B) is seven
   119x23 layers at (619,529) (`colorkey 0x0`, payment dialog);
   `AlbumLoading_Panel.ojs` (Playing slot 0, 70554 B) is one 197x179
   layer at (410,210) (`colorkey 0x7D5F`, album dialog). The OJN
   cover (`extracted/cover757-view.jpg`, 800x600 JPEG; file LOST
2026-11-11, re-captured under owner ticket #92 —
`docs/recovery-disposition.md`) is a
   full-bleed photo with no bead-pill rows or bar shell. Cover art
   carries no progress-fill anchors.
   The `0x4e47a0` dossier (`ev_68d920c2…`, file `0xe47a0`) carries
   three font creations (`Trebuchet MS` at VA `0x64ae78`, `Arial`
   at `0x64a320`, Korean face at `0x64407c`), the overlay mount
   (`MusicLoading.ojs` at VA `0x64ae64` via `0x4795a0`/`0x40d3d0`/
   `0x479750`), the song-table walk (stride `0x1b4`), and the song
   clock — no literal bar/caption rect. Adjacent bar bodies belong
   to other dialogs (`Loading_Sign.ojs` 134x37, `Wait_Downloding_Bar.ojs`
   65x7 at (0,0), `Payment_BackLoading.ojs` 119x23 at (619,529),
   `AlbumLoading_Panel.ojs` 197x179 at (410,210)) — none is the
   loading-gate fill. Shell rule: `loading_track_origin` (`w/4, h/2-8`),
   `loading_track_width` (`w/2`), `loading_song_origin` (`w/4, h/2+12`)
   stay honestly labeled OPEN placeholders (bnd-audit 35); only the
   walk fraction and the `TITLE - ARTIST` caption are live. Tests:
   `loading_bar_geometry_stays_open_placeholder` pins the placeholder
   formula so any future grounding must update it with evidence;
   `loading_placeholders_clear_every_measured_bar_body` pins the
   negative (native 800x600 track (200,292,400x16), caption (200,312)
   match none of the measured bodies above).
6. Chat row feed (`TEXT_SWAIT_CHAT`) — CLOSED to the list mutation:
   live rows enqueue into the parent's string queue at parent+`0x17c`
   through `0x479010` (the level-0/level-5 sites files `0x15f1a7`/
   `0x15f1c2`, plus `0xbc4` sites `0x15ec4d`/`0x15ec68`), pull
   through the `CStateWaiting` poller `0x4b9dc0` (vftable VA
   `0x649ad8` slot 10, COL VA `0x6673ec`; called from waiting
   drivers `0x4ad940`/`0x4b42d0` at files `0xaf8ea`/`0xb5b0a`), gate
   on opcode `0xbc4`/`0xbc5` plus `packed >> 0x10 == this+0x10`, and
   append through `0x4bb6c0` → `0x510730` into the staged
   `LIST_SWAIT_CHAT` row vector (list+`0x198`, cursor list+`0x1ac`,
   color slots row+`0x68b0`). The twins and `0x50fcc0` are bypassed.
   Remaining OPEN, presentation-side only: which row offsets inside
   the `0x6a5c` stride carry the line text, the cursor-advance/scroll
   rule in `0x510730`'s tail, the flag split (0 vs 5) and the writer
   of gate field `this+0x10`, and what the slot-16 dialog dispatch
   renders from the emote record. Shell stand-ins (`[ADMIN]` prefix,
   row colors) stay open in their owning files, now grounded by exe
   colors `0xdedede`/`0x71e6`.

## Builder-record map (2026-11-09, rdata scan)
Every dialog builder VA resolves through exactly one `.rdata`
record holding a class COL plus method VAs (offset-only LE32 scan
of the full image; VA = file + `0x400000`):

| Builder (file) | Record (file) | Class (COL type name) |
|---|---|---|
| bar `0x8dd10` | `0x247b20` | `.?AVCStatePlanet@@` |
| `0x8e990` | `0x247bc0` | `.?AVCStateRoom@@` |
| `0x984d0` | `0x2485b8` | `.?AVCStateShop@@` |
| `0xa3730` | `0x249150` | `.?AVCStateMyroom@@` |
| `0xb6100` | `0x249ae8` | `.?AVCStateWaiting@@` |
| `0xf3590` | `0x24b5c8` | `.?AVCStatePayment@@` |
| `0xf4740` | `0x24b7b0` | `.?AVCStatePayment2@@` |
| `0xf576f` | `0x24b8b8` | `.?AVCStateAvi@@` |

The bar fetcher (`Dialog_ChannelSelectBar.ojs` name ref, sole
image-wide hit at file `0x8de84`) sits inside `CStatePlanet`'s
method cluster (same record also holds the planet dispatcher
`0x48e280` and the `0x19400007` TEXT loop). All ten builders
share one thiscall shape (this + two args from `[ebp+8]`/`[ebp+0xc]`)
ending in the stager call — so the BLOCK arrives as an argument,
not an immediate, and no static scan can name which instance
stages Channel. The host question mutates: which call passes the
Channel block, and where does the bar land. No position invented.

### Record diff: shared base vs state builders (2026-11-09)

All eight records (planet/room/shop/myroom/waiting/payment/payment2/avi)
share 5–6 method VAs (`0x4065a0`, `0x409860`, `0x409f30`, `0x489eb0`,
`0x489ec0`, plus `0x409f20`/`0x48c570` in six) — the common state
machinery — and carry 7–9 unique methods each. Every per-state
builder sits at the SAME record position (index 5): the slot is a
shared "stage my block" virtual, so any instance stages whichever
block it receives as an argument. The Channel bar fetcher being
`CStatePlanet`'s slot-5 builder is therefore consistent with a
Planet (or shared-base) instance staging the Channel block — a
hypothesis, not a finding. No record exists for Channel (no class),
Result, MusicShop, or Lanking in this table region.

Byte walk of files `0x8dd10`–`0x8e000` around the bar fetch looked
for pushed mount coordinates (x, y, width, height immediates) and
found none: flag pushes `0`/`1`, alloc `0x170`, count `9`, lookup
argument `0x30E`, control codes, two style colors (`0xC3C3C3` at
file `0x8df08`, `0x4F4F4F` at file `0x8df17`), a five-iteration loop
(bound file `0x8debc`) over planet TEXT base `0x19400007` (files
`0x8dec9`/`0x8dee4`/`0x8def0`), and lookups `0x1950000c`/
`0x19201005`/`0x19201006` (files `0x8de2d`/`0x8df50`/`0x8df70`).
Button hosts `0x4f6040`/`0x524dd0` and text host `0x50f160` hold no
rect immediates either. Select-bar x/y/width/height, title-bar,
X-button, pager-arrow, MENU-bar, and planet-tab rects stay OPEN —
no position invented. Next evidence path: dynamic rect reads under
the Wine rig (pairing getter file `0x148640`), which needs no canvas
input.

## Button view offsets −60/−90 (2026-11-09, operand sweep)

Full-`.text` sweep (capstone 5.0.7, skipdata) for stores to control
`+0x1e4`/`+0x1e8` (the A8 position fields): 59 writes. Avatar
applier `0x4DA630` re-found (2 writes, method check). New: every
button-region writer (files `0x129088`–`0x1295d3`, `0x12a6df`–`0x12a7bf`,
`0x8c94b`/`0x8c95a`) stores `X − 0x3C` then `Y − 0x5A` (60/90
decimal) — view positions derive from a source pair minus fixed
chrome insets, never raw immediates. Result-region writers (files
`0xc2148`/`0xc227a`) zero `+0x1e8` behind a `+0x80` gate instead.
Value sources stay open (locals fed by args through helpers like
`0x12b480`; virtual call `[eax+0x10]` precedes the wrapper-region
site). Implication: button mount geometry rides dataflow, not
immediates — consistent with every failed immediate hunt above.

Live capture `/tmp/chan2-list.png` (local-only, 800x600, real client
on the test server, Ch.01–04 with `입장`): pixel-measured anchors,
reference-grade only — no exe consumer pins them, so they stay OPEN
and never count as exe evidence. Row text tops 232+50i (i 0..3,
`Ch.` white text at x≈230); row pitch 50; `입장` faces
(502,236+50i,48,24) (row 0 face 3-run agreement, followers by
pitch); title plate y≈150–195 with text ≈158–179 and bright X zone
x560–620; pager strip y≈439–464; dialog panel ≈x185–615. Method:
column/row brightness runs in `~/scratch/chan2-measure.py` plus
eyeball-checked zooms (`~/scratch/chan2-rows.png`). The shell's
`h/4 + i*8` rows and rect-corner details stay invented (audit
10–11) until painted at these anchors with passing guards.


## Ticket #12 row-art decode (2026-11-11, archive + capture join)

Decoded the four row bodies from `Interface1.opi` (no exe bytes —
data evidence only, joined against `~/scratch/t-chanlist.png` /
`q-chan.png` / `z-row0.png`):

- `ChannelInfo_CH_Num.ojs`: 10 raw 13x18 layers in stored order =
  digits 0–9. Block ids `IDC_IMAGE_CH10`/`CH01` name the tens/ones
  wells (bar-local (50,11)/(63,11)); the digit value source is
  PINNED to the wire `channel_id` + 1 by the 2026-11-10 rig capture
  (section below), which refutes the list-ordinal reading.
- `ChannelInfo_CH_ConnectCount.ojs`: one raw 185x19 layer = the
  orange 10-segment fill strip. The exe clips it to the population
  share (live Ch.01 shows one lit segment); the rounding/
  quantization rule stays OPEN, shell paints pixel-floor.
- `ChannelInfo_CH_Back.ojs`: 3 raw 347x41 frames — frame 0 normal
  bar (opaque gray face well, NOT a cutout: faces must paint after
  the bar), frame 1 etched 입장 bar (trigger OPEN), frame 2 FULL
  bar (baked FULL face for at-capacity rows).
- `ChannelInfo_CH_Enter.ojs`: 3 raw 44x21 입장 frames (frame 0 =
  normal; hover/pressed assignment OPEN).
- Sheet body `Dialog_ChannelInfo.ojs` (one 800x466 layer) bakes four
  empty row bars with `Ch.` (no digits), empty tracks, and CLOSE
  wells — slots past the live list show CLOSE in exe art, and the
  painter leaves those slots untouched. Exe rows carry no text
  anywhere else: the shell's ASCII row string was an invention and
  now paints on the figure-less fallback only.
- Inactive-row rendering is PINNED by the same capture: a row the
  gateway marks inactive paints the empty slot (baked `Ch.`, no
  digits, CLOSE face) even when it sits inside the list. The shell
  still paints inactive rows like active ones; `active=0` and
  `capacity=0` co-occur in every capture, so which field gates the
  paint stays OPEN.
- Exe key dispatch for the dialog stays unpinned (no LE32/operand
  join found in this ticket's timebox); the shell wires the
  standard lobby set (Up/Down/Confirm/Back→Planet) with tests, and
  the rig next step is a SendInput arrows/Enter/Esc sweep on the
  live dialog per the `~/scratch/planet-verify/` recipe (highlight
  via dialog+`0x94`, phase flag at parent+`0xF64`).

## Channel row digits — EXE-PINNED to `channel_id + 1` (2026-11-10 rig capture)

Settles the OPEN half of issue #28: the "Ch.NN" wells show the wire
`channel_id` one-based, never the row's list ordinal, and the entry
face still sends the raw `channel_id`. No disassembly: three live
captures with the numbers moved underneath the exe.

**Rig (all contained, scratch only).** Xvfb `:97` 1024x768x24; own
copy of the ko prefix (`~/scratch/prefix-digits`, removed afterwards)
so the sibling session on `:98` was never touched; `PULSE_SINK=wine_null`
throughout (no sound to the speakers); `OTwo.exe` 5,029,888 B sha256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`
run from the read-only copy `~/scratch/run-full`; launcher blob and the
five mirrors exactly as pinned in `wine-boot.md`; Wine 11.21, Python
3.14.9. Clicks: XTEST for the notice plate and the fallback popup, the
winegcc `SendInput` helper for every canvas hit (notice → popup OK →
dialog auto-opens; X-close and the scroll arrow respond the same way).
Gateway: the local Memoryer 7.2.0 on `127.0.0.1:15010` (6 channel
sections, ids 0–4 and 9; its 1003 reply pads the table to 10 rows, so
ids 5–8 arrive closed).

**How the numbers were moved.** A scratch TCP proxy
(`~/scratch/digits-rig/proxy.py`) sat between the exe and the gateway.
The stream is plaintext `[u16 len][u16 id][body]` framing
(`o2jam-net/src/frame.rs`), so the proxy rewrote only the `channel_id`
field of each 1003 row and forwarded everything else. Same row count,
same open/closed pattern, different numbers — the exe cannot tell the
difference from any other field.

**What each row read** (captures in `~/scratch/digits-rig/`; every
digit read off the zoomed row crops, every enter payload off the
gateway log `~/memoryer-local/server-u1.log`):

| run | wire rows in list order | first page | last page |
| --- | --- | --- | --- |
| 1 (no proxy) | ids 0,1,2,3,4 open; 5–8 closed; 9 open | `Ch.01 02 03 04` | empty ×3, `Ch.10` |
| 2 (proxy +5) | ids 5,6,7,8,9 open; 0–3 closed; 4 open | `Ch.06 07 08 09` | empty ×3, `Ch.05` |
| 3 (proxy +10) | ids 10–14 open; 15–18 closed; 19 open | `Ch.11 12 13 14` | empty ×3, `Ch.20` |

Enter payloads of the rows that were clicked: run 1's `Ch.10` row sent
`03EC` with channel `[0/09]`; run 2's `Ch.05` row sent `[0/04]`.

**Rules pinned.**

- Slot order is the wire list order: wire row *k* paints in slot *k*.
  No id-keyed placement (run 2 puts the id-5 row first, and it paints
  `06`, not `01`).
- The digits are `channel_id + 1`: tens well `(id+1)/10`, ones well
  `(id+1) mod 10`. Run 3 confirms the carry past 9 twice (id 10 →
  `Ch.11`, id 19 → `Ch.20`), so the two wells are decimal digits of the
  one-based number, not of the raw id.
- The list ordinal is REFUTED: only in run 1 do ordinal and id+1
  coincide (the padded table runs 0..9 in order). Runs 2 and 3 break
  the tie on the first page.
- Display and payload differ by one: the row painted `Ch.10` enters
  channel `09`. The digit value is a presentation number; the
  identity that travels to the gateway is the raw wire `channel_id`.
- A row with `active = 0` paints the empty slot — baked `Ch.`, no
  digits, CLOSE face — even inside the list, not only past its end
  (4 such rows in each run). `active = 0` and `capacity = 0` co-occur
  in all three runs, so which field gates the paint stays OPEN; the
  next evidence step is a gateway sending `active = 0` with a non-zero
  capacity.

**Shell consequence (code fix still to land).**
`channel_digit` in `crates/o2jam-shell/src/screens/paint.rs` currently
paints the raw `channel_id` (`id/10 % 10`, `id % 10`) and its doc
comment carries the old OPEN sentence. Pinned behaviour: paint
`id + 1` in both wells (`(id+1)/10 % 10`, `(id+1) % 10`), keep the row
order, and leave `active = 0` rows painting the empty slot. The value
join and its guard test belong to the implementation ticket; the rig
next step for the `active`/`capacity` question is named above.

## Channel scrollbar semantics — EXE-PINNED (static half, #130)

Settles the ticket's static question by byte-walk (second pair of
eyes on the `3553efc` pins, 2026-11-12): the scrollbar buttons scroll
the LIST, never the highlight, and bar dragging exists in the exe.
Independently re-verified every cited offset; two pins below (range
setup, populate args) go beyond the code comments.

**Method.** Read-only walk over `install-root/O2Jam/OTwo.exe`
(5,029,888 B, md5 `34b789ff557a9bf6a1f275010dcaaceb`): GNU objdump
2.47 slices plus Python 3.14.10 byte scans. VA = file + `0x400000`
throughout. No REA dossier; plain disassembly in our own words.

**Answer (a): the arrows scroll the list.** The mouse arm `0x50e8d0`
(file `0x10e8d0`) hit-tests the arrow zones and steps the scrollbar
position by signed delta through `0x50ebf0` (file `0x10ebf0`):
up-arrow zone (file `0x10ea1a`) pushes `-1` (VA `0x50ea66`), down-arrow
zone (file `0x10ea6b`) pushes `+1` (VA `0x50eab5`). `0x50ebf0`
scales the int delta by the `+0xc8` double and forwards it to the
position setter `0x50ec20` (file `0x10ec20`), which owns `this+0xb8`
and clamps it to `[0, max-page]` (`+0xbc`/`+0xc0`). Each delta
notifies the channel list through the `0x4749a0`/`0x4749f0` arms
(files `0x749a0`/`0x749f0`), gated on codes `0x13`/`0x14` with the
control code `0x8C02100C`: `CDChannelInfo+0x184 += delta`, then a
repaint call (`0x475000`). No selection write exists on this path —
the only two `0x8C02100C` immediates in `.text` are those two arms
(a third byte hit at file `0x7421b` is mid-instruction noise,
disassembly reads garbage there).

**Answer (b): dragging exists.** Press zones set drag-active
(`this+0xc4`); the motion tick `0x50e7c0` (file `0x10e7c0`) gates on
`+0xc4 != 0`, reads the cursor through the pairing table
(`0x768124`), and absolute-sets through the same list-top notify;
release clears `+0xc4` (VA `0x50ead0`). The thumb paint rides the
same position: `0x50ee50` (file `0x10de50`) recomputes
`round(pos * scale) + origin` with the orientation arm at `+0x64`
(vertical origin `+0xac`, horizontal `+0xa8`).

**Scale equation PINNED, thumb travel OPEN.** The range setup
`0x50f030` (file `0x10e030`, 17 E8 callers image-wide) stores its
max arg into `+0xbc`, then: max <= page (`+0xc0`) forces scale
`+0xc8 = 0.0`; otherwise `+0xc8 = (span_b - span_a) / (max - page)`
with orientation-picked rect-extent pairs (vertical: `+0xb4`-`+0xac`
minus `+0xa4`-`+0x9c`). The exact struct-rect identities are
unpinned, so the equation cannot yet turn into thumb pixels.

**Populate puzzle OPEN.** The channel populate zeroes the joint
reset (`+0x184 = 0` at VA `0x4747ba`, scrollbar `+0xb8 = 0` at VA
`0x4747d3`, both file `0x747ba`) — guarded in the shell by
`repopulating_channels_resets_the_scrolled_list_top` — then calls
the range setup at VA `0x4747ea` with args `(max = 10, second = 0)`.
A max of 10 against ~22 wire channels does not read as an item
count, so which scrollbar (or which units) that call serves stays
OPEN; do not wire the thumb scale from it.

**Shell consequence (landed `3553efc`, kept here).** Arrows step
`channel_top` through `channel_viewport_from_top`; the highlight
rides the keys alone; the bar body answers nothing to a click.
Drag has no shell handler yet and the thumb paints fixed — both
stay OPEN with the same named next step: a live rig capture
measuring thumb travel per row (or a deeper byte-walk of the
scrollbar struct rects), never an invented scale.

## Ticket #189 — exe channel-dialog arrow-key rule PINNED: keys are no-ops (2026-11-14)

Closes the #174 remainder. Question: do Up/Down step the scrollbar
position (`+0xb8`) directly, move the highlight with top-follow, or
something else? Answer: **something else — the exe ignores them.**
Two legs, diagnose-first, on the isolated :97 rig (own Xvfb/prefix/run
copy, `PULSE_SINK=wine_null`; :96 rig and Memoryer untouched).

**Static leg (byte-walk, own words).** Toolchain pins match #130 (GNU
objdump 2.47, Python 3.14.10, read-only 7.55 copy 5,029,888 B, sha256
`83a04541…72c823b`; VA = file + `0x400000`). Re-verified the scrollbar
machinery (#130): delta forwarder `0x50ebf0` (file `0x10ebf0`); channel
±1 arms at files `0x74920`/`0x74960` (push `∓1`, `ret $0x4` — button
arms, no delta arg); list-top notify arms at files `0x749a0`/`0x749f0`
(codes `0x13`/`0x14`, control `0x8C02100C`, `CDChannelInfo+0x184 +=
arg`, repaint `0x475000`). All four arms have NO direct `E8` caller in
`.text` — they are reached by virtual dispatch (observed `call *%eax`
after vtable lookup, e.g. VA `0x474879` +`0x8`), so key-vs-mouse routing
is a message/code choice, not a distinct call site. New for #189: zero
`cmp *,0x26` (VK_UP) in `.text` in any register form, and the ten `cmp
eax,0x28` hits are non-key (e.g. VA `0x46d597` compares a
`(y2-y1)/0xb8` quotient against 40 — a layout clamp, `0xb8` a pixel
stride there, not VK_DOWN). The key dispatch source stays unpinned —
consistent with there being no key handler at all for this dialog.

**Rig leg (dwell, decisive).** Boot `wine explorer
/desktop=o2jam,800x600 OTwo.exe <blob> <mirror>x5` (mirror
`|test|??|127.0.0.1|15010`), notice H-plate XTEST, native white-box
XTEST away, dialog auto-opens EASY with numbered rows `Ch.01..`. New
SendInput key helper (`sikey.c`, winegcc, `VK <hex>` press via the
`o2jam` desktop join — same pattern as `siclick3.c`); every delivery
`n=1`, foreground window constant `0x20086`. The blue NORMAL-mode box
(appears ~2-10 s after most opens, not all) eats keys: Enter/ESC dismiss
it AND take the dialog down with it — no dismiss-keeps-dialog path
exists, so all reads below use the clean window (blue probe < 40 on both
bounding captures; presses after blue-onset counted VOID). Captures in
`~/scratch/t189-keyrule/` (proprietary, never in repo):

- `r2-poll0`/`r2-down4`/`r2-up4`: Down x4 + Up x4 blind bursts on a
  populated list — row bands pixel-identical (0 changed px per band,
  threshold 90).
- `b2-base`/`b2-D1`/`b2-D4`/`b2-U4`: single-Down then +3 Downs then +4
  Ups with per-press captures, all clean — rows stay `Ch.01-04`, bands
  identical.
- `a-armed`/`a-D1`/`a-D3`/`a-U2`: mouse-armed control — row-1 body
  SendInput click paints the lime select outline on `Ch.02`, then Down
  x3 + Up x2: outline unmoved (green px 325/327 before and after).
- `i-D2`/`i-enter`: Down x2 then Enter — no room entry, dialog stays,
  server log byte-identical (no entry packet).
- `m-arrowdown` liveness control (same session, same list): one
  scrollbar down-arrow SendInput click `(583,419)` scrolled `Ch.01-04`
  → `Ch.02-05` (top 0→1) and dropped the outline. Mouse path alive;
  key path dead.

Rules OUT: position-step (A) and highlight-step-with-top-follow (B).
**Verdict: Up/Down (and Enter, observed along the way) do nothing
visible — the dialog is mouse-only** (rows, scrollbar arrows,
enter faces, X). Invisible-highlight is also excluded as far as pixels
can tell: Enter after Down x2 entered no room.

**Shell consequence (landed here).** `channel_key` Up/Down arms are
documented no-ops; `step_channel` (highlight-step + top-follow
placeholder) deleted with its two OPEN labels; the dead
highlight→top mappers `channel_viewport`/`viewport_top` deleted with
their test (no caller re-seats the top under the highlight anymore;
windows derive from the stored top via `channel_viewport_from_top`
alone). Paint untouched: keys never reach `+0xb8`, so thumb geometry
is unaffected (the `+0xc8` scale OPEN stands). Confirm/Back arms are
out of scope (still OPEN). Guard:
`channel_arrow_keys_leave_list_top_and_highlight_untouched` (live chain
`winit key event → window.rs::route_key → InputEdge::Menu →
ScreenGame::ingest_snapshot → ScreenGame::menu_key →
ScreenGame::channel_key`; red-proven: old step+re-seat behavior FAILs
it, the no-op PASSes it).

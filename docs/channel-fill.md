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
only as one RTTI name (file `0x27dc10`) — dialog lookup is index-driven,
so the bar's dialog host (`DIALOG_CHANNEL_INFO`/`CDChannelInfo` per
lifecycle §6) stays unpinned and the shell stages no bar body.

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
3. Dialog host mounting the code-fetched select bar
   (`DIALOG_CHANNEL_INFO`/`CDChannelInfo` index path; `0x4ED650` is since
   read — a playing-screen builder, not a dialog host candidate — see the
   OPEN 2 join; `ui-loader.md` OPEN 2 itself untouched).
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
   cover (`extracted/cover757-view.jpg`, 800x600 JPEG) is a
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

## Reference-measured Channel geometry (2026-11-09, Wine capture)

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


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

## Chat row feed (`TEXT_SWAIT_CHAT 0x1640003C`, `SET 0x00031000`) — CLOSED

The row template is fed by twin waiting-room feed functions, never by
code immediate. Full-image LE32 scan: `TEXT_SWAIT_CHAT` has zero refs;
`SET 0x00031000` has one unaligned data-coincidence hit only (file
`0x3324b3`, spanning dwords `0x0`/`0x310` in `.data` — not code).

- **Handler**: `0xb9a10` feeds user rows (3012), `0xb9a60` feeds admin
  rows (3013). Each guards a style/code pair (`0x13` vs `0x14` with
  code `0x1602102a`), looks up `LIST_SWAIT_CHAT 0x1680002B` through
  control lookup `0x48BCF0`, and passes the row string to the list row
  store `0x50fcc0`. Both live in the waiting UI region beside the
  chat-view stager `0xb8d00` and the 3011 send sites.
- **Upstream**: trampolines `0xac5b0`/`0xac5e0` and `0xb2dd0`/`0xb2e00`
  forward into the twins; their addresses sit in three parallel
  10-entry handler tables (files `0x249710`/`0x2499f0`/`0x249b28`,
  `.rdata`, each opening with `0x489ee0`). 3012 vs 3013 selects user
  vs admin by table slot (index 6/7), and the dispatch stubs push
  opcode plus subindex into `0x478d50` (first 3012 stub file
  `0x15e601`, `ev_aba36a3a…`). Sibling feeds share the same store:
  `0x62840`/`0x628d0` (channel arm — cached list object in `ecx`, no
  `LIST` immediate at all) and `0x90690`/`0x906f0` (room arm, `LIST`
  `0x13800014`).
- **Fields**: sender plus text compose the row string (`edx` into the
  store); the slot hard-codes the user/admin style. The trailing seat
  byte has no evidenced consumer on the feed path.
- **Array iteration, pinned**: the row store `0x50fcc0` (file
  `0x10fcc0`, `0xa70` B) materializes rows by indexing the list's own
  vector (`this+0x198`, `imul` index × `0x6a5c`, checked-iterator
  guards) — the template is consumed through the staged list object.
  No channel/waiting row control is code-addressed by immediate.

Pin: `OTwo.exe` 5029888 B, sha256
`83a04541…c823b`, offsets file-based, found by offset-only byte scan
(Python 3.14.9, capstone 5.0.7, GNU objdump 2.47 — no new dossier).
Shell rule unchanged: live 3012/3013 packets only, newest visible rows
stack from the list origin; the `[ADMIN]` prefix and row colors stay
stand-ins (the matching OPEN sentences in `net-protocol.md`,
`rooms.rs`, and `paint.rs` are owned by other docs/code and untouched
here).

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
   8 trio sites — pinned string-plus-style join in one builder. Net
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
   `0x248591`–`0x2613a0`, not code). Neither control-lookup caller set
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
   (`DIALOG_CHANNEL_INFO`/`CDChannelInfo` index path; `0x4ED650` excluded
   as the fetch site but its own behavior is unread — ui-loader OPEN 2).
4. `CWindow`/`CDialogBase` default fill, if any (ui-draw OPEN 3).
5. Loading bar/caption geometry — OPEN, negative result (placeholders
   kept): `MusicLoading.ojs` (`Playing1.opi` slot 39, 960,028 B) is
   one sprite-55 body, single 800x600 layer at anchor (0,0),
   colorkey `0x0` — no bar/caption sub-rect exists in the art.
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
   walk fraction and the `TITLE - ARTIST` caption are live. Test:
   `loading_bar_geometry_stays_open_placeholder` pins the placeholder
   formula so any future grounding must update it with evidence.
6. Chat row feed (`TEXT_SWAIT_CHAT`) — CLOSED by the trace above:
   feed twins `0xb9a10`/`0xb9a60` via handler-table slots 6/7, row
   string carries sender plus text, seat byte unused on the path, and
   the template resolves through list-vector iteration (zero code
   immediates). Shell stand-ins (`[ADMIN]` prefix, row colors) stay
   open in their owning files.


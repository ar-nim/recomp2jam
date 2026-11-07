# OPI ignored fields audit

Scope: every parsed `OPI` / `ControlList` / `.ojs` / `.bnd` / sprite field the
scene composer (`o2jam-shell` `avatar.rs`, `screens/paint.rs`, `screens/game.rs`,
`screens/side.rs`, `playing.rs`) reads through the parser but drops before the
pixel. Parser sites live in `o2jam-file` (`opi.rs`, `control.rs`, `sprites.rs`,
`tables/shop.rs`, `ojn.rs`). No source was changed for this audit.

## Severity 1 — wrong pixels on screen

- `BndRect` size (`x2/y2`, `sprites.rs` `BndRect`) — composer keeps only the
  origin. `avatar.rs` `placement_origin_in_frame` scales `rect` then reads
  `(scaled.x, scaled.y)`; `paint_placed_figure` blits 1:1 there. Every panel
  keeps native art size instead of stretching to its rect.
- `Sprite55Image.x/y` in highway skin — `playing.rs` `lane_sprite` /
  `skin_frames` decode `w*h` but center with `centered_x` and anchor to the
  judgement line, never adding record `x/y`. Avatar path (`avatar.rs`
  `from_sprite55` → `dx/dy`) does use them.
- Non-raw `Sprite55` images — `avatar.rs` `from_sprite55` filters on
  `is_raw_rgb555`; `playing.rs` `lane_sprite` returns `None` otherwise.
  Compressed / outlier `.oji` frames vanish instead of decoding. The exe
  fails them identically: every format compare in `.text` accepts only
  `0x555`/`0x555a` (claims C9).
- `Sprite55Strided` (`Note_Combo4/5/6.ojt`, `sprites.rs`
  `parse_sprite55_strided`, `frames_per_image`, `frame_data`) — never called
  in shell; combo glow animation paints as one static frame or not at all.
- `Oja` envelope (`sprites.rs` `parse_oja`, `OjaFrame{x,y,w,h,a,b,c}`) —
  no shell call site. `Note_Click.oja` centred shrinking frames and the
  `a/b/c` words never animate.
- JPEG `.ojs` bodies (`opi.rs` `is_jpeg_body`: `ATI*` album art) — never
  called in shell; select-music / album panels paint dialog art without rows.
- Number-sprite `SET` groups — `paint.rs` `paint_result_content` calls
  procedural `draw_number` at fixed `(16,16)` / `(16,32)` instead of the
  block's number-font sprites.

## Severity 2 — layout / visibility wrong

- `ControlEntry.code` (`control.rs` `ControlEntry.code`, e.g.
  `0X10500001`, `0X84200003`) — copied into `avatar.rs` `place_entry`
  (`ScenePlacement.code`) and never read. Visibility / blend / state bits in
  the top byte never hide, disable, or reorder anything; `paint_scene_figures`
  draws every loaded placement unconditionally in file order.
  Exe side (byte sweep, *(offset-only)*): the load/create/draw chain
  consumes byte 2 only — `code & 0xFF0000` gates creation (`0x48A134`)
  and selects the ctor arm (`0x48B30A`, `0x48A668`). No consumer of
  bits 24–31 exists in `0x48A060`/`0x48A620`/`0x48B2D0`/`0x4DA630`/
  `0x4E4EE0`/`0x548640`/`0x5DEC30`/`0x50A890`. The top-nibble tests in
  the image draw (`0x50000000`/`0xA0000000` at `0x50A9BD`/`0x50AD92`)
  decode the draw-call flag argument, not the entry code. So
  "blend/visibility in the top byte" is refuted: the bits the exe acts
  on live in byte 2 (control class) and the low word (rect ordinal,
  set key).
- `ControlSet.params` (`control.rs` `ControlSet.params`, e.g. `SET 0x11`
  count/flags) — flattened by `all_entries`; frame-list vs number-sprite
  grouping survives only as order.
  Exe side: the parser reads param 1 as member count (set `+0x08`) and
  param 2 as the SET match key (set `+0x04`, matched against
  `code & 0xFF000` at apply), allocating the child array at parse
  (`0x411890`, ev `2fdbec57…`). Our `params[0] == count` reading
  matches; the "flags" guess does not.
- `ControlBlock.number` / `extra`, `ControlList.num_states` /
  `num_dialogs` (`control.rs`) — never read in shell. Declared-count vs
  actual-entry mismatches (Playing `0xE6` vs 235 entries, trailing 5 ride
  unloaded) are hardcoded in `avatar.rs` `PLAYING_SUBSET_FIRST_ENTRY` instead.
- `ControlBlock.bound_file` — shell hardcodes `ROOM_BIND_FILE`,
  `PLAYING_BIND_FILE`, etc. (`avatar.rs`, `paint.rs`, `side.rs`) instead of
  reading the block's `BOUND "..."` line. Exe side *(offset-only)*: only two
  scenes read `.bnd` at all — the avatar dialog (`0x4da683`/`0x4da854`) and
  the network-playing builder (`0x4e4f21`/`0x4e4f57`), the loader
  `0x548450`'s only call sites — so per-scene hardcoded stems mirror the
  exe, which reads no `BOUND` line beyond those literals.
- Unbound blocks (`CHANNEL`, splash, loading) — `paint.rs`
  `unbound_scene_placements` emits `id=""`, `code=0`, empty `Rect`; true
  layout (if any) is discarded and art pins at the native origin.
- Rect/entry count mismatch — `avatar.rs` `bind_verbatim_entries` zips and
  stops at the shorter side: entries past the last rect stay unplaced (Playing
  tail past `VOLUME`, surplus room entries), rects past the last entry are
  ignored. Both are silent. Exe side (byte survey, *(offset-only)*): control
  creation never zips — each entry resolves its rect by code ordinal
  (wrapper VA `0x48b09c`, gated on the block stem byte, into the pairing
  getter `0x548640`), and an out-of-range ordinal gets an all-zero rect
  instead of a silent drop; the full 4-dword rect reaches the control setter
  `0x50dc90`. SET children bind only when their masked key `code & 0xFF000`
  matches the set key (VA `0x48b0e1`, compare `0x48b122`); unmatched set
  children stay unbound.
- Synthetic origins — `paint.rs` `seat_origin`, `song_panel_origin`,
  `room_detail_fallback_origin`, result digits at `(16,16)` / verdict at
  `(16,48)` ignore `Dialog_SelectMusic.bnd` / `State_Waiting.bnd` rects; only
  9 hardcoded `ROOM_DETAIL_*_ID` anchors ever consult `.bnd`.

## Severity 3 — text / labels replaced with placeholders

- Decoded `.ojs` / `ControlList` Korean text (`lib.rs`
  `decode_control_text`) — never drawn. `paint.rs` hardcodes `CHANNEL`,
  `ROOM`, `WAITING`, `WIN`/`LOSE`, `M{music_id}`, `NORMAL`/`LIVE`, etc.;
  `room_detail_entries` maps `IDC_TEXT_*` ids to wire values, not to the
  archived strings beside `Room_LevelLimit.ojs` / `Room_Password.ojs`.
  The exe has no archived captions to fall back on: ControlList entries
  carry no caption literals (exe parse stores only code + sprite stem,
  `0x4115F0`); labels come from sprite art or runtime strings (room
  fill `Lv.%d:%s`), so our placeholders replace runtime text, not
  archived strings.
- `AvatarItemRow.name` / `description` reach `side.rs` `shop_item_view`, but
  `item_type`, `params[4]`, `flags(u8,u8)`, and the per-slot `(flag, name)`
  bytes (`tables/shop.rs` `AvatarItemRow`) never reach the screen; slot
  semantics are the parity heuristic in `avatar.rs` `pick_wear_sprites`.
- `OjnListEntry` catalog detail (`ojn.rs`: `unknown0`, `format_version`,
  `level_flags`, `stable_minor`, `counts_a/b`, `measures`, `events`,
  `key_field`, `pad`, `flag100`, `const_a`, `tune`, `label`, `ojm_file`) —
  `side.rs` `music_shop_songs` keeps only `song_id/title/artist/genre/bpm`.

## Severity 4 — archive bookkeeping ignored (by design, listed)

- `OpiEntry.kind`, `reserved_size`; `OpiSlot` free slots, `parse_opi_slots`
  (`opi.rs`) — shell calls `parse_opi` only (`avatar.rs`
  `load_archive_sprite`, `playing.rs` `archive_body`), slicing by
  `data_offset..data_offset+size`. Free-slot reservations and slack bytes
  never affect the scene.
- `Sprite55.tag`, `Sprite55.colorkey` provenance — `tag` never branches;
  `colorkey` is threaded through (`FigureLayer.colorkey`,
  `SkinSprite::new`) and honored at blit, so it is used, not ignored.
  Exe side the sprite reader `0x4249f0` likewise never tests the `+0`
  tag word — it gates on the `+2` format word only (`0x555`/`0x555a`,
  compares `0x424c96`/`0x424caf`, ev `103158a8…`) — and consumes the
  colorkey word at load, so neither field is dead in the exe either.
- `Sprite55Image.data_offset` / `size` — used only to slice
  (`pixel_data`); chaining / exact-accounting checks stay parser-side.

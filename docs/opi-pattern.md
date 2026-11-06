# OPI scene-composition recipe (7.55) — VERIFIED

How the 7.55 client draws any UI scene from archives only.
No disassembly is reproduced; every step cites read-only byte walks
plus read-only exe string offsets (file offset = VA − `0x400000`).

Version pin: `AngelO2Jam.exe` 5,029,888 B, SHA-256 `83a04541…72c823b`;
`Interface1.opi` 75,100,702 B; `Playing1.opi` 221,737,026 B;
`avatar.opa` 144,761,749 B. Method: static structure walks over the
installed files with Python 3.14; read-only string sweep of the exe.

## Recipe

1. Open the three archives. The exe names them adjacently at file
   `0x247958` (`Image\Playing1.opi`, `Image\Interface1.opi`,
   `Image\Avatar.opa`) next to `.\Interface\State.cpp`.
2. Decode the block list. Read `ControlList_Interface.txt`
   (`Interface1.opi` slot 1595) or `ControlList_Playing.txt`
   (`Playing1.opi` slot 632) as CP949. The parser keywords live at
   file `0x243f34` (`NUMBER_OF_DIALOG`, `NUMBER_OF_STATE`, `%s%x`,
   `%s%s`, `%s%x%x`, `SET`, `%s%x%s`); `SET` sits at file `0x243f70`.
3. Pick one `STATE_`/`DIALOG_` block. Entry 0 names the block itself
   and takes no rect. Skip the four empty `0x00` stubs
   (`PLAYING`, `TUTORIAL`, `DRUM_PLAYING`, `EXIT`): they draw nothing.
4. Resolve the bind file through the block's `BOUND "stem.bnd"` line.
   No per-state `.bnd` literal exists in the exe (verified absence of
   `State_Room`, `State_Playing.bnd`, `State_Waiting`,
   `Dialog_Information`); only `Avatar_Info.bnd`/`Map_Avatar.bnd`
   (file `0x24ab9c`) and `State_Playing_network.bnd` (file `0x24b080`)
   appear literally. Every other stem loads data-driven — and every
   bound stem resolves to an archive entry.
5. Pair rect `i` with block entry `i + 1` in file order (`SET`
   members count in file order; `ControlBlock::all_entries` preserves
   it). Holds on all 38 bound Interface blocks with zero mismatches.
   `State_Playing.bnd` covers only the 17-entry trailing run the file
   delimits with in-file comments (`EXIT`…`VOLUME`, entries 213–229);
   decl `0xE6` (230) loads entries 0–229 including self, the last 5
   (`WINNER`…`GAMEOVER`) ride unloaded.
6. Resolve each entry's quoted sprite against the same archive,
   case-insensitively (3 Interface stems differ only by case).
   Entries without a sprite (text, edits, bare slots) draw no art.
7. Blit each sprite into its rect. Single-image sprites duplicate
   their placement anchor byte-exact
   (`Dialog_Information.ojs` anchor `(241,206)` 318x187 == rect 0;
   `Info_OK.ojs` anchor `(102,150)` 117x26 == rect 1;
   `Playing_Exit.ojs` anchor `(751,554)` 42x42 == Playing rect 0).
   Space is absolute pixels on 800x600, top-left origin, `(x2, y2)`
   exclusive; every `State_*.bnd` spans exactly `[0,800]×[0,600]`.
   Raw frames satisfy `size == w*h*2` (63742 images); skip `0x7D5F`
   / `0x0000` colorkey runs as transparent.
8. Paint in file order over the entry-1 background. Rect 0 is the
   full-screen back art on every full state
   (`State_Room.bnd` rect 0 `(0,0)-(800,600)` for `IDC_BACK_SROOM_1`;
   `State_Avi.bnd` sole rect full-screen for its single entry), so
   the background lands first and later entries cover it.
9. Apply the two name rules before giving up on a missing sprite.
   `O2_SBS_VERT`/`O2_SBS_HORZ` with numeric args are scrollbar
   parameters, not files (exe keeps `O2_SBS_HORZ`/`O2_SBS_VERT` beside
   `%s%d%d` at file `0x24c190`). A leading `_` marks runtime
   resolution (11 Playing refs): the exe keeps `%s.ojs`, `%s.oja`,
   `%s%d.ojs`, `%s.ojt` patterns beside `Playing_Effect_Jam` at file
   `0x24b0ec`, and each bare stem has a numbered/`.oja` sibling.

## Field coverage

Consumed = byte- or string-anchored. Unknown = present but unproven.

| Field | Status | Anchor |
|---|---|---|
| OPI `kind` | Consumed as file-vs-free test | 9/6/16 kind-0 slots hold empty names, `size` 0, stale nonzero reservation — never name-addressable |
| OPI `name` | Consumed as lookup key | All 38 bound stems + Playing stem resolve; archive-open block at file `0x247958` |
| OPI `data_offset` + `size` | Consumed as slice | Table strides tile `[16, table_start)` with zero holes; readers must slice `size`, never `reserved_size` |
| OPI `reserved_size` | Consumed by patcher, ignored by renderer | Slack bytes verified nonzero (stale, not padding); stride-end checked |
| OPI header version (2/1) | Unknown | Observed only; no behavior tied to it |
| Block `name` | Consumed as scene selector | `DAILOG_SELECTMUSIC` typo parses by prefix and still binds 133 rects |
| Block `number` (decl) | Consumed as load count | decl == non-self top-level entries on 39/39 full blocks; Playing comment marks the load cutoff at entry 230 |
| Block `extra` (`0X01`…`0X04` on 10 blocks) | Unknown | Preserved verbatim; semantics TBD |
| Entry `id` (`IDC_…`) | Consumed as control key | 1295 Interface + 235 Playing `IDC_` lines parse without loss |
| Entry `code` (full `u32`) | Consumed as integer | Second-byte histogram (`00` 39, `10` 39, `20` 470, `40` 238, `50` 274, …) correlates with the `CC*` control RTTI beside `?AVCControlList@@` (file 2615242); exact dispatch TBD |
| Entry `sprite` | Consumed, or special-cased | 562/568 Interface and 53/64 Playing stems resolve; 6 scrollbar params + 11 `_`-prefixed refs follow step 9 |
| `SET` first param | Consumed as member count | Equals member count on 22/22 groups |
| Trailing parameter-less `SET` runs | Unknown client-side | Parser folds them to keep file order; applier treatment TBD |
| Sprite `x`/`y` | Consumed as placement anchor | Byte-exact anchor == rect on all three single-image probes (step 7) |
| Sprite `w`/`h` + `size` | Consumed as blit extent | `size == w*h*2` on 63742 images |
| Sprite `colorkey` | Consumed as transparency key | Only `0x0000` / `0x7D5F` observed (3444/5787 bodies) |
| Sprite `tag` (0/1) | Unknown | Both parse identically with exact accounting |
| Sprite record trailing zero word | Unknown | Always zero; never varies |
| `.bnd` magic + `count` | Consumed as envelope | `FFFFFFFF` + exact `6+16*n` on all 53 sections |
| `.bnd` rects | Consumed as layout | Step 5 (38/38) + step 7 (anchor == rect) + full-screen span |

## OPEN

1. Paint-order proof beyond background-first: file order is the only
   order that puts the full-screen back art underneath; a pixel-diff
   against the applier would confirm topmost-last.
2. Dialog rect frame: `DIALOG_INFORMATION` OK-button rect sits outside
   its back rect, so dialog rects may be dialog-client-relative rather
   than screen-absolute — needs the applier trace.
3. `SET`-group rect interplay beyond file order; block `extra`
   numbers; sprite `tag`; `.oja` frame-data codec and `a`/`b`/`c`
   words; outlier `.oji` pixel codecs; exact `_`-prefix and
   `%s%d`/`%s%d_%d` expansion rules (exe keeps `Note_LongCombo`,
   `Note_LongComboNum`, `Note_Click` stems at file `0x24b0ec`).
4. Loader/applier function addresses: no REA dossier covers the
   `0x243f34` keyword block or `CControlList` methods yet.

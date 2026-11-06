# `.bnd` rect semantics (7.55) — VERIFIED

Version pin: `OTwo.exe` == `AngelO2Jam.exe`, 5,029,888 bytes,
SHA-256 `83a04541…72c823b`; `Interface1.opi` 75,100,702 B;
`Playing1.opi` 221,737,026 B. Method: read-only byte walks over the
installed files with Python 3.14.9; read-only string sweep of the exe
(file offset = VA − `0x400000`). No disassembly is reproduced.

## Answer

- Space: absolute pixels, 800x600 screen, top-left origin, x right,
  y down, `(x2, y2)` exclusive (`w = x2 − x1`, `h = y2 − y1`).
  Every `State_*.bnd` spans exactly `[0,800]×[0,600]`, matching the
  client's 800x600 window (`docs/lifecycle.md` §renderer).
- Pairing: the exe pairs each entry by its control code, not by
  file position: rect ordinal = `(entry.code & 0xFFF) − 1`,
  bounds-checked against the `.bnd` count, miss → the all-zero
  rect (pairing step VA `0x548640` = file `0x148640`, write-back
  helper `0x5487B0`, dossier `ev_3a869bb7…`; `docs/ui-loader.md`
  step 7). The earlier file-order reading (rect `i` ↔ entry
  `i + 1`, entry 0 taking no rect) fit the 38 bound Interface
  blocks only because their code low words run sequentially — a
  consequence, not the rule; a zero low word misses too (wrapped
  `− 1` fails the bounds check). Rects duplicate their sprite's
  placement anchor byte-exact (e.g. `Playing_Exit.ojs` anchor
  `(751,554)` 42x42 == `State_Playing.bnd` rect0).
- Subset: `State_Playing.bnd`'s 17 rects serve the entries whose
  code low words are 1–17 — the 17-entry trailing run the file
  delimits with in-file comments (`EXIT`…`VOLUME`, entries
  213–229). Their sequential low words make a positional 17:17
  walk agree, but selection is by ordinal (VA `0x548640`, ev
  `3a869bb7…`). Decl `0xE6` (230) loads entries 0–229 including
  self; the last 5 (`WINNER`…`GAMEOVER`) ride unloaded.
  `State_Room` 75/76 and `State_Waiting` 116/117 follow the same
  ordinal rule with sequential low words, not a special subset.

## Evidence

- `State_Room.bnd` rect0 `(0,0)-(800,600)`; `State_Avi.bnd` sole rect
  `(0,0)-(800,600)` for its single `IDC_BACK_LOGO` entry.
- `Dialog_Information.ojs` anchor `(241,206)` 318x187 == `.bnd` rect0;
  `Info_OK.ojs` anchor `(102,150)` 117x26 == rect1: anchor==rect in
  dialogs too.
- Playing delimiters (CP949, `ControlList_Playing.txt` file lines
  245–246): comment meaning "bound file starts from here" opens the
  17-run; line 263 comment meaning "control list loads up to here"
  closes loading at `VOLUME` (entry 229 of 235).
- Rect pairing dossier `ev_3a869bb7…` (`docs/ui-loader.md`, record
  `rea-ui-loader-controllist`): pairing step VA `0x548640` derives
  rect ordinal = `(entry.code & 0xFFF) − 1`, bounds-checks it
  against the u16 count, and copies the four dwords (x1, y1, x2,
  y2, 0x10 stride) — the all-zero rect on a miss; write-back
  helper `0x5487B0` persists parent-offset-adjusted rects.
- Exe loader anchors (strings only): archive-open adjacency at file
  `0x24795e`/`0x2479a4` (`Image\Playing1.opi`, `Image\Interface1.opi`,
  `Image\Avatar.opa`, `.\Interface\State.cpp`); `CControlList` parser
  keyword block at file `0x243f34`–`0x243f70`
  (`NUMBER_OF_DIALOG`, `NUMBER_OF_STATE`, `%s%x`, `%s%s`, `%s%x%x`,
  `SET`); `?AVCControlList@@` RTTI at file `0x27e7cd` with `CC*`
  control classes from `0x27e705`.
- Data-driven bind: only `Avatar_Info.bnd`/`Map_Avatar.bnd` (file
  `0x24aba7`, VA `0x64aba7`) and `State_Playing_network.bnd` (file
  `0x24b080`, VA `0x64b080`) appear literally in the exe; no
  `State_Room`/`State_Waiting`/`State_Playing` literal exists, so the
  loader resolves every other stem through the `BOUND` line.
- Unbound legacy sections (no `BOUND` references them): Interface
  `StateRoom.bnd` (76), `State_Bulletin.bnd`, `State_MyCapsule.bnd`,
  `State_Top100.bnd`, 7 `Dialog_*` (`AlbumInfo`, `Exit`, `ItemInfo`,
  `MyAlbum`, `SelectAlbum`, `Top100_Buy`, `UserInfo`); Playing
  `Avatar_Info.bnd` (10), `Map_Avatar.bnd` (8),
  `State_Playing_network.bnd` (19, exit-path variant).

## OPEN

1. Loader/applier coverage beyond pairing: the pairing step is
   pinned (VA `0x548640`, ev `3a869bb7…`, `docs/ui-loader.md`);
   the `0x643f34` keyword-block dossier and the `CControlList`
   RTTI class-name join stay open there.
2. Dialog rect frame: same pixel unit and top-left origin, but the
   `DIALOG_INFORMATION` OK-button rect sits fully outside its back
   rect, so whether dialog rects are screen-absolute or
   dialog-client-relative needs the applier trace.
3. `SET`-group rect interplay beyond the per-entry ordinal rule
   (each entry's ordinal is verified; group-level behavior is
   not traced beyond the same rule applying to set members).
4. `//StateLanking //StateWaiting //Stateroom //Lightboard` comment
   block at file `0x24bdb9` (VA `0x64bdb9`): nature TBD.

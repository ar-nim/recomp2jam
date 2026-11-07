# Behavioral-claims inventory (7.55) — by evidence level

Every behavioral claim in `docs/*.md` (incl. `docs/file-formats/`) plus
doc comments in `crates/*/src/*.rs`, tagged by evidence level. Version
pin for all EXE rows: `AngelO2Jam.exe` 5,029,888 B, SHA-256
`83a04541…72c823b`; DATA rows pin the archives listed in
`docs/file-formats/opi.md`.

## Levels

- **EXE** — anchored in the client at a file offset (VA = file +
  `0x400000`) with an evidence id: an REA dossier id where one exists,
  else a local-static record id. Rows marked *(offset-only)* cite
  offsets from objdump/byte surveys whose docs state no dossier was
  filed.
- **DATA** — byte-walk inference over shipped data files only; no
  consumer code observed.
- **ASSUMED** — player report, reference-implementation corroboration,
  positional/ordinal guess, placeholder, or documented-unknown.

**Drives** flags what a claim moves on screen or in play: `R`
(rendering), `G` (gameplay), `R+G`, `—`.

## Counts

| Level | Claims |
|---|---|
| EXE | 92 |
| DATA | 27 |
| ASSUMED | 20 |
| Total | 139 |

## The pairing-rule family (known mistake + siblings)

The known pattern: a file-order/positional rule asserted where the exe
uses an explicit ordinal or id field. Source claim A2 ("rect *i* pairs
entry *i+1* in file order") is a DATA walk that the EXE refines: the
applier looks each entry up by `code & 0xFFF − 1` and bounds-checks
(A1). Siblings found by hunting for the same shape:

- **S1** — "SET entries consume `.bnd` rects in file order exactly like
  top-level entries" (`o2jam-file/src/control.rs:17`). EXE matches SET
  children by `code & 0xFF000` against the set key instead (A19).
- **S2** — the Playing 17-rect subset is applied as a positional zip
  ("rect *i* → entry 213+*i*", `o2jam-shell/src/avatar.rs:357`). The
  exe assigns rects by per-entry ordinal; sequential low words in that
  run make the zip *happen* to agree (A26).
- **S3** — RESOLVED: the exe pairs Room/Waiting entries by code
  ordinal at control creation (wrapper VA `0x48b09c`); the one-entry
  surplus takes the all-zero rect (A27).
- **S4** — `resolve_order_slot` keys id-less OJM banks by fetch order.
  The exe has no order lookup anywhere on load/play paths; M30 banks
  key by the explicit `ref` id, new-variant entries all land in slot 0
  (C28). The fallback is ours, labeled as such (C29).
- **S5** — difficulty mapping guessed by position: "+20 lo/hi/+24
  ascending triple" (C34) names header fields by slot order, 13/13 fit.
  The exe reads levels as `u16[difficulty]` over the song-record words
  {+24 lo, +26 hi, +28} (= header +20 lo/hi, +24) and pins the
  ascending order via the difficulty-keyed life sets (C34/C35).
- **S6** — RESOLVED: the D13 value sets are exe-pinned (icon frames,
  level-table indexing, icon grid, label chain); the Memoryer ordinals
  stand as corroboration.
- **S7** — RESOLVED to the loop level: the render loop paints stored
  (file) order (A5); the only z-order nuance found is the
  `.oji`-over-`.ojs` layer split.

Lesson: wherever a sequential rule is claimed, ask which explicit
field (code low word, code nibble, M30 `ref`, header per-level field)
actually indexes the table — and what the exe does on an out-of-range
index (all-zero rect, slot 0, default arm).

## A. Rendering & scene composition

| ID | Claim | Level | Anchor / evidence | Drives | Sibling |
|---|---|---|---|---|---|
| A1 | True pairing mechanism: rect ordinal = `(entry.code & 0xFFF) − 1`, bounds-checked, miss → all-zero rect | EXE | `0x548640`, ev `3a869bb7…` | R | corrects A2 |
| A2 | File-order pairing: rect *i* lays out entry *i+1* (38/38 bound Interface blocks, zero mismatches) | DATA | `opi-pattern.md` 5, `bnd-semantics.md` walks; REFUTED as mechanism: the create wrapper pairs every bound control by code ordinal (VA `0x48b09c` — stem-byte gate, entry code dword pushed into `0x548640`, 4-dword rect passed to setter `0x50dc90`), so the 38/38 agreement is the sequential-ordinal consequence and the ordinal-overflow entry takes the all-zero rect (A1) | R | **source** |
| A3 | Shell binds by zip-and-truncate (`bind_controls`, `bind_verbatim_entries`), silent drops | ASSUMED | `bnd-audit.md` 1–2; contradicts A1 zero-rect | R | S2/S3 |
| A4 | Rect 0 is the full-screen background, painted first | DATA | `State_Room`/`State_Avi` rect 0 walks; painted-first is file order (A5); no exe background special-case observed — the image-draw arms fetch rects through the same ordinal getter and blit at origin only (VA `0x50aa32`–`0x50aa56`); the canvas span stays data-side | R | feeds A5 |
| A5 | Paint order is topmost-last by file order beyond background-first | EXE | render loop `0x5DEC30` walks the control array in stored (file) order, building quads sequentially, ev `ff581cfa…`; nuance: `.oji` sprites ride layer 2 above the `.ojs` layer 1 regardless of file position (`ui-draw.md` step 3) | R | refines S7 |
| A6 | Space: 800x600 absolute px, top-left origin, `(x2, y2)` exclusive; every `State_*.bnd` spans the canvas | DATA | all `State_*.bnd` spans; top-left origin exe-backed: the image-draw arms load all 4 rect dwords but push origin x/y only into the blit (VAs `0x50aa32`–`0x50aa56`, `0x50ac88`–`0x50acac`, `0x50ae02`–`0x50ae20`); no `x2−x1` extent consumer located, exclusivity stays data-side *(offset-only survey)* | R | |
| A7 | 800x600 default, fullscreen-only (`WS_POPUP` fork default; windowed branch unreachable) | EXE | `0x14b4c`/`0x14baf`, fork `0x41c0b` | R | |
| A8 | Avatar-dialog rects are screen-absolute; dialog origin subtracted at apply | EXE | `0x4DA630`, ev `9e526b62…` | R | |
| A9 | All 27 dialogs share the screen-absolute origin rule | ASSUMED | `0x4ED650` traced *(offset-only)* and is NOT the shared dialog applier: a single-block scene builder (stem lookup `0x4795a0` ← `0x4ed689`, into-record loader `0x547DB0` ← `0x4ed6a3`, synthetic entries `0x1B200001`/`0x1B300006` through factory `0x48B2D0` ← `0x4ed6e9` and wrapper `0x48A620` ← `0x4ed729`, 4-dword rect getter `0x50DC60` reading control `+0x3c` ← `0x4ed760`) — the 27-dialog origin rule stays untraced (`ui-loader.md` OPEN 2–3) | R | |
| A10 | Single-image sprites land at their rect: dest = view position + per-frame sprite anchor, byte-exact == rect (3 probes); the rect is unread at blit | EXE | anchor math `0x41DF80` (anchor words at frame-record `+0`/`+2`, stride `0x14`), ev `7418d033…`; equality probed on `Dialog_Information`/`Info_OK`/`Playing_Exit` | R | |
| A11 | Raw frames: `size == w*h*2`, little-endian RGB555 | DATA | 63,742 images across archives; negative *(offset-only)*: no `0x555` immediate in `.text` (C5) and the shipped image decoders are only `CDXImageBase`/`BMP`/`JPG` (RTTI VAs `0x67d584`–`0x67d5d8`) | R | |
| A12 | Colorkey `0x0000`/`0x7D5F` pixels are transparent | DATA | 3,444/5,787 bodies + byte-exact probes; honest limit: the draw path never compares the key on CPU — consumption is device-side (OPEN, `ui-draw.md` Colorkey) | R | |
| A13 | Sprite lookup is ASCII-case-insensitive (3 case-only stems) | EXE | *(offset-only)* archive slot walk `0x40A9C0` (0x98-stride records, count at `+0x110`) compares each name via the CRT case-fold compare — fast path folds A–Z by `+0x20` at `0x5F33F7`, called through `0x5F34FF` ← lookup `0x4797D0` ← open `0x4795E0`; stems: `opi-pattern.md` 6 | R | |
| A14 | `O2_SBS_VERT`/`O2_SBS_HORZ` with numeric args are scrollbar params, not files | EXE | strings `0x24c190` | R | |
| A15 | `_`-prefix marks runtime resolution; `%s.ojs`/`%s.oja`/`%s%d.ojs`/`%s.ojt` patterns | EXE | strings `0x24b0ec`, `0x247958` | R | exact expansion rule ASSUMED |
| A16 | Playing screen feeds stack-built synthetic entries through the same factory | EXE | ev `1be2d784…` (`0x48B2D0` family) | R | |
| A17 | One control per entry is created only when `code & 0xFF0000 ≠ 0` | EXE | `0x48A060`, ev `9e526b62…` | R | |
| A18 | Code second byte selects the control class (ctor arm) | EXE | factory `0x48B2D0` masks `code & 0xFF0000` (`0x48B30A`) into a compare chain of ctor arms (allocs `0x60`…`0x20C`); wrapper `0x48A620` repeats the dispatch (`0x48A668`, e.g. image arm ctor `0x50A820`) and splits the toggle arm further on `code & 0xF000` (`0x48AA1F`); ev `1be2d784…`; arm ↔ RTTI class-name join open, behavior names stay CXO2-corroborated only | R | |
| A19 | SET children are matched by `code & 0xFF000` against the set key, recursed | EXE | `0x48A620`, ev `1be2d784…` | R | corrects S1 |
| A20 | SET entries consume rects "in file order exactly like top-level" | DATA | `control.rs:17` doc; order-only observation | R | **S1** |
| A21 | `SET` first param == member count | EXE | parser `0x411890` sscanf (format file `0x243F68`) stores param 1 → set `+0x08` and param 2 → match key `+0x04` (`0x411913`–`0x41193A`), then allocates count × `0x11C` children at parse (`0x41198B`–`0x4119A5`); ev `2fdbec57…`; 22/22 groups | R | |
| A22 | Trailing parameter-less SET runs fill the declared slots | EXE | eager alloc at parse, ev `2fdbec57…` | R | |
| A23 | The four empty `0x00` stub states draw nothing | EXE | applier `0x48A060` creates controls only from the block's entry array, gated `code & 0xFF0000 ≠ 0` (`0x48A134`, ev `9e526b62…`); zero-entry stubs create nothing, and the render loop `0x5DEC30` draws stored controls only (ev `ff581cfa…`); stub inventory (empty names, `size` 0) stays data-side | R | |
| A24 | `BOUND` stem is data-driven; only `Avatar_Info`/`Map_Avatar`/`State_Playing_network` literals exist | EXE | `0x24ab9c`/`0x24b080`, ev `3eb885cd…` | R | |
| A25 | Shell hardcodes bind stems per scene instead of reading `BOUND` | ASSUMED | `opi-ignored.md` sev 2; contradicts A24 | R | |
| A26 | Playing subset: entries 213–229 land on the 17 rects through the shared ordinal pairing; the declared `0xE6` count bounds parsing/creation; entries 230–234 are never parsed | EXE | declared-count eager alloc ev `f4662376…`; playing applier `0x4F1140` binds via the into-record loader (relcall VA `0x4f117b`); pairing getter `0x548640` ev `3a869bb7…` called from the playing-screen functions (relcall VAs `0x4f36c5`/`0x4f4878`/`0x4f6238`) and the shared wrapper (`0x48b09c`); the 17-rect/213–229 coverage numbers themselves stay the data walk (CP949 comments + counts) | R | **S2** |
| A27 | Room 75/76 and Waiting 116/117: every entry resolves its rect by code ordinal at control creation; the one-entry surplus overflows the table and takes the exe's all-zero rect; the equal counts are data corroboration | EXE | wrapper pairing VA `0x48b09c` (stem-byte gate, code dword → `0x548640` ev `3a869bb7…`, 4-dword rect to setter `0x50dc90`); state applier binds via the into-record loader `0x547DB0` (relcall VA `0x48a0ef`); counts from the walk | R | closes S3 |
| A28 | `.bnd` rects carry extent (4 dwords copied into the control) | EXE | `0x548640`, ev `3a869bb7…`; shell keeps origin only | R | |
| A29 | Unbound scenes (bare `BOUND`) paint art full-bleed | ASSUMED | `bnd-audit.md` 15/23; exe stages `(0,0)` and draws at the sprite anchor (`ui-draw.md`), so full-bleed depends on per-scene art size — unverified | R | |
| A30 | Highway geometry from staged art: lanes `(5,28)…(165,28)`, judgement row 487, 7 px heads | DATA | sprite-anchor walks (`KeydownImage` slot 33 et al.) | R | |
| A31 | Jam bars at `(23,536)`/`(570,536)` 206x16; peer bars 46x8 | DATA | staged `Jamgauge` shell anchors; exe side builds the gauge shells as synthetic `0x11c` entries through the factory (loader VA `0x4e7850`: zeroed `0x11c` allocs, vtable words `0x64b258`/`0x64b244`), so placement still resolves through bind rects — the pixel numbers stay data-side | R | |
| A32 | Judgement/combo/jam digit anchors, 600 ms fade, score digits at fixed pixels | ASSUMED | `bnd-audit.md` 31; rects unbound | R | |
| A33 | Linear scroll, 2000 ms lookahead | ASSUMED | `bnd-audit.md` 32; scroll shape unobserved | R+G | |
| A34 | Curtain bands are integer fifths, clipped to a frame box | ASSUMED | `bnd-audit.md` 20/30; exe clip math open — the static visibility table VA `0x680038` (file `0x280038`) holds integer runs (`3, 33, 63, 124, 115, 143, 165` then `3, 33, 57, 87, 124, 154, 178`), not fifths; semantics untraced | R | |
| A35 | Procedural receptor pad 5 px, note 8 px + cap, burst 24 px | ASSUMED | `bnd-audit.md` 6/7/29 | R | |
| A36 | Micro-font metrics + hardcoded placeholder labels (`CHANNEL`, `M{id}`, …) | ASSUMED | `dialog-strings.md` §2/§5: captions missing by design; exe label sources are sprite art + runtime strings (`Lv.%d:%s` fill, song-select fonts), so our placeholders replace runtime text, not archived captions | R | |
| A37 | 47 `.ojs` bodies are raw JPEG album art (`FF D8 … FF D9`) | DATA | `is_jpeg_body` 47/47; the exe ships a JPEG decoder (RTTI `CDXImageJPG` VA `0x67d5d8`, IJG strings VA `0x75f9fc`, *(offset-only)*), so the bodies are consumable in-exe | R | unused in shell |
| A38 | Present letterboxes 4:3, never stretches | ASSUMED | `playing-reference.md` photo | R | |
| A39 | CDX/DirectDraw blit path: DirectDraw init, CDX surface blits, D3D texture upload | EXE | import verified `DDRAW.dll!DirectDrawCreateEx`, IAT VA `0x63F01C` (descriptor file `0x27AF2C`, *(offset-only)*); consumer chain pinned in `ui-draw.md`: display init `0x413030`, `CDXSurface::Draw` `0x42B4D0` ev `867ac248…`, `DrawClip` `0x42B5E0` ev `85764b28…`, page dispatch `0x42B450` ev `08a33fe4…`, page upload `0x42A580`, D3D variant `0x423390`; colorkey-consuming device call + `Flip` site stay open | R | |

## B. Gameplay

| ID | Claim | Level | Anchor / evidence | Drives | Sibling |
|---|---|---|---|---|---|
| B1 | Judgement: `\|song_ms − note_ms\|` vs inclusive 6/18/25 ms → flags `0x10/0x20/0x40/0x80` | EXE | `0x176646/55/64`, `0x176b7f/8e/9d`; ev `14992eb2…` set | G | |
| B2 | Song ms via tick→ms interpolation | EXE | ev `9f8e1c2b…`/`f170d5f6…` | G | |
| B3 | Combo: `0x10`/`0x20` continue; `0x40`/Miss break | EXE | `0x176c86`/`0x176c8c`/`0x176d8b` | G | |
| B4 | Grade strings are code-unreferenced; result art-driven | EXE | zero refs `0x247363`… (full-image) | R | |
| B5 | Pills: 15th consecutive Cool mints one, cap 5; Good/Bad reset streak; Bad with stock spends one; mode 6 exempts | EXE | `0x16e4c0/0x16e509/0x16e84d`, ev `c168abac…` | G | |
| B6 | Pill stock `+0x398c` (sub `+0x216c`), streak `+0x3990` | EXE | setter `0x140870`, adder `0x140b50` | G | |
| B7 | Score: Cool `200+10X`, Good `100+5X`, Bad+pill → Cool path, Bad `+4`, Miss `−10` floored | EXE | `0x170510` | G | |
| B8 | Jam level `X` at `+0x396e` is written (bulk copies + word stores) | EXE | `0x4e92a0`, `0x14079e`… | G | |
| B9 | Jam progress +2 Cool/+1 Good; thresholds `(i+1)*50` cumulative, all difficulties; reset on Bad/Miss | EXE | `0x170260`/`0x1705b0`/`0x170628`/`0x1708dd`/`0x170936` | G | corrects "30" report |
| B10 | `+0x3972` is the regular combo counter, `+0x3974` its max | EXE | `0x141620`, `0x1706da–0x1706ef` | G | |
| B11 | Hold bonus: +10 score, +1 combo per playback tick while held | EXE | `0x178110`, ev `737dfe7b…`; writes `0x14163c`/`0x141652` | G | |
| B12 | Long notes: a Cool/Good head arms the hold flag and the per-lane tail cursor `+0x864`; the release strike judges the armed tail vs the tail time with the shared 6/18/25 windows (≥26 ms off = Miss either side, one-shot — cursor cleared on consumption); the expiry scan is inert (per-lane cursors `+0x480`/`+0x484` load as the list head and never advance, so the Miss store and paired-tail marking are unreachable — a held tail can never be expired); a missed/Bad head kills its paired tail; bonus ticks while armed ∧ span open; the walk strike takes the nearest unjudged non-tail record (rides key-down per the `0x1714b0` inversion) | EXE | arming `0x176e60`, tail judge `0x176646`–`0x176687`, clear `0x176709`, ev `9faabadd…`/decompile `9a80eb3b…`; walk `0x1767c9`–`0x176b00`, tail exclusions `0x176837`/`0x176994`, inversion `0x1714b0`; flag `0x17617b`/`0x176d78`; bonus scan `0x178110`, files `0x178194`/`0x1781c1`/`0x1781d7` (ev `1baf7ef1…`); inert scan: dossier `ev_16e274b8…`/raw `ev_02842ce1…`, cursor init `0x172d32`/`0x172d39`, walk break `0x1773bf`–`0x1773c5`, Miss store `0x17740f` unreachable; tail kill `0x176dab`–`0x176df6`; judges ev `14992eb2…`/`3d707204…`; `HoldState::judge_tail` | G | CORRECTED ×2: refutes "tail released at its end; early release = miss", then refutes "no release judgement — tail rates only by the expiry Miss" |
| B13 | Hold-active flag at `+0x9f8`, written gated on payload hold-bit tests | EXE | writes `0x17617b`/`0x176c78`/`0x17726b`; tests `0x1766cb`… | G | |
| B14 | Chart payload bit 25/26 = hold head/span-tail semantics | EXE | bit-25 head test `(payload & 0x3000000) == 0x2000000` at VAs `0x576105` (second judge, ev `3d707204…`), `0x576c42` (judge keyup, ev `14992eb2…`), `0x5771f5` (auto judge, ev `d6802fde…`); bit-26 span mark `and $0x4000000` at `0x573061` (parser fix-up `0x572f50`, *(offset-only)*); paired tails carry bit 24 and are excluded from walk candidacy (`0x576837`, ev `14992eb2…`) | G | |
| B15 | Kinds 2–8 are the seven lane channels, lane = `map[kind − 2]` (identity ⇒ `kind − 2`); shipped-chart usage (`{0,2,3}`/all seven) stays data-side | EXE | loader kind gate VA `0x572732`–`0x57275a` (`kind ≥ 2 ∧ kind < [+0x82c]+2`; width 7 ⇒ 2..8; ev `8659c2d0…`); lane index read from the u16 map at `0x572791` (map filled by lane setup `0x571c30` via the out-array call `0x571e42`, ev `41695a9b…`), lane list `+0x42c + idx*0xC` at `0x572853`; kinds ≥ 0x22 fail the load (`0x5723b9`) | G | |
| B16 | Note value = `(flags << 16) \| (sample_id + 1)`; ids land in the `.ojm` set (one dangling) | EXE | the loader stores each lane-event dword verbatim into the record (VA `0x572828`–`0x57282f`, insert `0x5728c3`, *(offset-only)*); BGM channels decode `value & 0xFFF` as the sample id (`0x572a64`, *(offset-only)*); playable strikes consume `(payload & 0xFFF) − 1` plus `0x3E8` on bit 26 (B17, `0x1703f0`) and hold semantics read bits 24–26 (B14); the id-membership numbers stay the data walk (12/13 charts, one dangling) | G | |
| B17 | Strike slot = `(payload & 0xFFF) − 1`, plus `0x3E8` on bit 26 | EXE | `0x1703f0` | G | |
| B18 | Timing: 192 ticks/beat, ms via `240000 / BPM`, per-event float rounding | EXE | loader `0x571d60` (ev `8659c2d0…`): doubles `192.0` (VA `0x64b540`, × at `0x57221f`) and `240000.0` (VA `0x64dc80`, ÷ at `0x5720dc`/`0x572225`/`0x57233a`/`0x5725b8`); 0xC0 subdivision bounds `0x572273`/`0x572411`/`0x572444`; round-half-up `+0.5` (constant VA `0x644888`, site `0x572491`) into truncating `fistp` under control-word `0xC00` (`0x57211b`–`0x572132` et al.) | G | |
| B19 | Kind-0 single float = beats per measure (a stored zero dword defaults to 1.0) | EXE | loader branch `0x572453` (`test kind`), float read `0x572462`–`0x572475` with `fld1` default `0x57247d`, measure tick length = beats × 192 (+0.5 round) stored `0x572485`–`0x5724d6`; ev `8659c2d0…`; the lone o2ma769 `0.75` observation stays corroboration | G | |
| B20 | Kind-1 per-subdivision floats = BPM changes applied mid-measure | EXE | loader branch `0x5724e2` (`cmp $0x1`, ev `8659c2d0…`): count floats copied `0x5724f2`–`0x57250b`, each non-zero float applied to the BPM word `0x57259f`–`0x5725a6`, tick ms recomputed `0x5725b2`–`0x5725be`, BPM events inserted via `0x579f00` (`0x572639`); the o2ma765/769/770 cross-checks stay corroboration | G | |
| B21 | Kind-1 per-subdivision `0.0` = keep; the run is count-bounded, not `999.0`-terminated | EXE | *(offset-only, re-verified from bytes)* kind-1 branch `0x5724e2`–`0x5724ec`; floats copied `0x5724f2`–`0x57250b`, synthetic `0.0` appended `fldz` `0x57251a`; loop bounded by the u16 count `0x572573`; `0.0`-keep = `fcomp` vs the `0.0f` constant VA `0x643fa8` (`0x572587`) with the skip branch `0x572592`, else BPM store + tick-ms recompute `0x572598`–`0x5725be`; the `999.0` terminator is refuted — the sole `999.0f` reader is `0x5d5f1a` (outside the loader), so a literal 999.0 loads as BPM 999 | G | |
| B22 | Kinds 9+ are autoplay BGM channels, same value encoding | DATA | 13-chart sample-id membership; exe structure *(offset-only)*: kinds ≥ 9 (capped `< 0x22`, `0x5723b9`) take the BGM branch `0x5728e9`, channel = `kind − 9` (`0x572913`–`0x57291d`, max tracked at `+0x838` `0x572931`–`0x572946`), per-channel lists `+0x504 + ch*0xC` (`0x5729bc`–`0x5729cc`), value words carry bit-26 + 12-bit sample id (`0x572a4f`–`0x572a6a`); autoplay consumption of these lists stays data-side | G | |
| B23 | Lane shuffle: mirror = full reversal; byte 2 = N-lane `rand()%width`; byte 0 = 3-lane `rand()%3`; rebuilt per plain measure + per hold span | EXE | `0x171c30`/`0x173940`/`0x173a10`/`0x174330` dossiers | G | |
| B24 | Item-text "whole chart vs every measure" distinction is false: both rebuild per measure | EXE | planner `0x174330` behavior | G | |
| B25 | Which ring item selects which modifier byte | ASSUMED | room-side translation open | G | |
| B26 | Curtain geometry: hidden top 80%, sudden bottom 20%, dark middle 20%, reverse-dark 40+40 | ASSUMED | item text; exe clip math open — the visibility table VA `0x680038` integer runs (A34) do not encode the fifths directly | R+G | |
| B27 | Team buttons `0x16201004..0xB` → teams 0..7; press sends 4004 then 4008(0) when ungated | EXE | `0xb22d0`/`0x48c810`/`0x556680`; ev `2c8d3964…` set | G | |
| B28 | Team scoring is server-side; client renders winner art only | EXE | no team-total arithmetic; `0x24b014`/`0x24a35c` | G | |
| B29 | 4007 team-change handler refreshes the 8-slot UI, `< 8` guarded | EXE | `0x15f470`, ev `1ff324b4…` | G | |
| B30 | Level-up missions do not exist client-side (server-side EXP) | EXE | ev `0ce27550…`; stub walk | G | |
| B31 | Visible level gates: `Room_LevelLimit` icon, difficulty bands 1–8/8–19/19–27/27+; validator rejects bad slots | EXE | strings `0x24578e…`, ev `68760cac…`; `0x5718b0` ev `66e60d16…` | G | |
| B32 | Life: max 1000; per-judgement deltas by difficulty — Cool +3/+2/+1, Good +2/+1/0, Bad −10/−7/−5, Miss −50/−40/−30; dead gate at 0 | EXE | switch `0x170fa2`–`0x171056`, ev `feebe490…`; scorer `0x170510` → adder `0x540a80`; `LifeBar`+`LifeDeltas` | G | corrects Bad=0 report |
| B33 | `+0x3970` is a 1000-clamp + rating gate + UI notify `(0,0,0x66,0x12)` | EXE | `0x57094d`, `0x1707c6–0x1707db` | G | |
| B34 | Fail splash: GameOver static vs animated picked by a predicate | EXE | `0xe69a1` | R | |
| B35 | Jam gauge math: Cool 2/Good 1, 30-progress cycles, overflow carries | ASSUMED | negative: zero `cmp $0x1e` in rating range | G | refuted vs B9 numbers |
| B36 | Win/lose art goes to rank #1 by score | ASSUMED | player report; no client-side score-sort located. Exe context: the result screen is state block 6 — opener VA `0x4c2390` stages `StateResult_Top.ojs`/`StateResult_Bottom.ojs`/`Result_WinLose.ojs` (VA `0x64a388`/`0x64a370`/`0x64a35c`) and calls the state applier at `0x4c23e2`; the Live-result region creates `Winner_image`/`Challenger_image` entries unconditionally (codes `0x1B500012`/`0x1B500013` at VAs `0x4e66c2`/`0x4e6714`, controls `+0x7b4`/`+0x7b8`) and gates a `Lv.%d` label on role 1/2 (`0x4e6f95`/`0x4e6faf`, role global `0x77ee86`); the rank→art selection itself stays open | G | |
| B37 | Play keys S D F Space J K L (option.ini `Key`/`KEY_1..7`, exe-built DIK defaults) | EXE | *(offset-only)* key-table init VA `0x543290`: `GetPrivateProfileIntA` (IAT `0x63f268`) per slot with defaults `0x1f`/`0x20`/`0x21`/`0x39`/`0x24`/`0x25`/`0x26` pushed at `0x5432b3`–`0x543373`, stored to `+0x345..+0x34b` (`0x5432c8`–`0x543388`); strings `Key`/`KEY_1..KEY_7` VA `0x64ccd0`–`0x64cd04`; 3-key defaults + DIK reverse map at `0x67ebb0` (`0x5436bb`–`0x5437cf`); the earlier "no bind table" scan missed the `push $imm8`-separated defaults | G | |
| B38 | Play paces on the wall clock, not the sample clock | EXE | play advance derives song time from `timeGetTime()` (IAT `0x63f4f8`, call VA `0x575d98`) rebased on the song start `+0x834` (`0x575da7`), gated on play state `+0x828 == 2` (`0x575d7c`) — ev `efdaac19…`; tick→ms interpolation `0x57a240` (ev `9f8e1c2b…`) consumes that wall ms; DSOUND imports `DirectSoundCreate` only (creation, no clocking); audio-cursor reads stay unlocated *(offset-only)* | G | |
| B39 | Rating-event sink `+0xa14`: codes 1/9/14, 16-way jump table, attach at song load | EXE | `0x56e170`, attach `0x170325` | G | |
| B40 | `option.ini` table is display-only (MODE/BlendHalf/LONGNOTEMODE/NoteTail/NoteType) | EXE | key block `0x245224–0x2452a8` | R | |
| B41 | NoteType 0–2 + per-level strings, NoteTail 0–4, LongnoteMode cached per song | EXE | `0x5713c7`/`0x16bf47`/`0x16fb0d`; on-screen effect ASSUMED | R | |
| B42 | 8-seat member table: stride `0x2190`, base `0x77ee90` | EXE | tick site ev `b16e614c…` | G | |
| B43 | Peer life/jam mirror + rank remap broadcast from 4015 (deliver via `0x5414a0`) | EXE | `0x15fa20` chain, ev `0ff34c44…` (Ghidra) | R+G | |

## C. File formats

| ID | Claim | Level | Anchor / evidence | Drives | Sibling |
|---|---|---|---|---|---|
| C1 | OPI layout: 16 B header, blobs from +16, trailing 152 B table | DATA | 3-archive walk; exe slot-table reader *(offset-only)*: count × 0x98 records (imul VA `0x40a9ff`, read stride push `$0x98` `0x40aa39`) with count at archive object `+0x110`, table seeked from EOF by `−count × 0x98` (`push` SEEK_END + `neg` VA `0x40a9fa`–`0x40aa14`) — header/blob layout stays data-side | R | |
| C2 | Slice bodies by `size`, never `reserved_size`; slack is stale | DATA | stride tiling + nonzero slack; exe opens entries as streams *(offset-only)*: body open `0x479790` selects one of three `0x220`-stride open slots at archive `+0x19c` into `0x40a8a0`, chunked `ReadFile` wrapper `0x4069b0` (IAT `0x63f2e0`); the size-field read that bounds the slice stays unlocated | R | |
| C3 | Kind-0 slots are deleted-entry reservations, never bodies | EXE | the name→slot find `0x40a9c0` walks the `0x98` records and accepts only records whose first dword == 1 (`cmp` VA `0x40aa64`–`0x40aa6a`, *(offset-only)*); any other slot ends the search unfound, so kind-0 entries never resolve a body; the deleted-reservation reading of those slots stays the data survey | — | |
| C4 | Archive inventories: 1596/633/7149 slots; 9231 sprite bodies; class table | DATA | body-class cross-check | R | |
| C5 | Sprite-55 envelope: `tag + 0x0555 + count + colorkey`, exact accounting | DATA | 9231/9231 bodies; negative *(offset-only)*: no `0x555` immediate anywhere in `.text` — no exe-side marker test located | R | |
| C6 | `.bnd` envelope: magic `FFFFFFFF`, `6 + 16*count` | EXE | loader `0x548450` (called per stem, e.g. `Avatar_Info.bnd` VA `0x4da847`, `State_Playing_network.bnd` `0x4e5dce`, *(offset-only)*): body opened `0x548501`–`0x548519`, magic dword tested vs `0xFFFFFFFF` at `0x548532` (mismatch → fail), u16 count read `0x548562`–`0x548571`, `count × 0x10` allocated `0x5485a9`–`0x5485c1` and read `0x5485da`–`0x5485f1`; the 53/53 section walk stays data-side | R | |
| C7 | `.oja` envelope `tag + 0x555A + count`; frame codec TBD | DATA | 3/3 envelopes; pixels unknown | R | codec ASSUMED |
| C8 | Strided combo atlases: ×4 stride, 4 glow frames per record | DATA | `Note_Combo4/5/6.ojt` exact 196,736 B | R | |
| C9 | Outlier `.oji`/compressed pixel codecs | ASSUMED | parsed, never decoded | R | |
| C10 | ControlList/word-list text is CP949 | DATA | decode rejection study | R | |
| C11 | Old OJN 300-byte header map, field-complete | DATA | byte-exact reads off `Tutorial.ojn` | G | |
| C12 | New-format decrypt: byte-reversal + 3-key period XOR from the tail | EXE | `0x12180`, ev `011eab73…`; round-trip test | G | |
| C13 | Per-level segmentation: counts +64/+68/+72, slice ends +284..+296 | DATA | 14/14 walk-exact; exe side *(offset-only)*: the loader bounds each difficulty's event walk by the count table game `+0x738[difficulty]` (read VA `0x571ee9`, difficulty selector `+0x830` at `0x571eb8`, ev `8659c2d0…`) and the validator enforces the 300-byte header (`cmpl $0x12c` VA `0x5718cb`, ev `66e60d16…`); the header-word copy site into `+0x738` stays unlocated — whole-image sweep finds no other `+0x738` displacement, so the counts must arrive via a bulk header copy | G | |
| C14 | OJNList: 562 x 300 B records + middle + patch sections, exact sum | DATA | section-sum check | G | |
| C15 | Mid-region tables 1–5 shapes decoded | DATA | 7-part exact split; exe reads the file by path *(offset-only)*: `%s\\Image\\OJNList.dat` built at VA `0x47a03a` and opened (IAT `0x63f2c4`), archive fallback `0x478c90` sized `0xffe` at `0x47a066`–`0x47a078`; the mid-table readers stay untraced | G | |
| C16 | Table 5 category (1..5) = wheel bucket; `song+0x194` → `entry+0x4`; STEP2 buttons 1..5 = GOLD..SILVER | EXE | jump tables `0xbfa5c`/`0x93409`; builder pin `0x4422e` | R | |
| C17 | Patch index + `;HD` section shape (561 + truncated 562nd) | DATA | layout walk | — | |
| C18 | Key-name table = 65 VA pointers to CP949 filter names | EXE | 15 code refs, `0x448c2` et al. | R | corrects `xTd` guess |
| C19 | STEP1/2/3 banner domains, writers, sentinel skips | EXE | `0x3f6c2`/`0x3f7ee`/`0x3f91b` | R | |
| C20 | STEP2 banner off-by-one: entry-50 row has 7 names, `실버` unreachable | EXE | pinned mismatch; interpretation open | R | |
| C21 | Doublable-initial arms (ㄲㄸㅃㅆㅉ) pair base jamo from indexed rows | EXE | `0x44962`…`0x44ce6` | R | |
| C22 | GameSpeed 0–11 labels `X 0.5`…`X R` from the label chain | EXE | `0xc4137`…`0xc4371`, ev `f2fa3e80…` | R | |
| C23 | OJM M30/OMC/OJM containers: entry walks + OggS/EOS to EOF | DATA | 21-file walk; exe per-entry dispatch *(offset-only)*: descriptor flag bits at `+0x110` gate the body transforms per entry (`&4`/`&8`/`&0x10` tests `0x54f11c`–`0x54f17d`, calls `0x54f14b`/`0x54f16f`/`0x54f193`); container envelope and OggS/EOS tail stay data-side | G | |
| C24 | M30 sample-id roles: drums 0–12, music tracks 980–991 | DATA | entry names + chart refs | G | |
| C25 | Loader descriptor from per-song version float: 2.9→17-way, 2.7→16-way, ≤2.4→plain | EXE | `0x54e930` flag bits | G | |
| C26 | 16/17-way interleave tables are exact permutations | EXE | the exe consumes both tables as u16 permutation lookups: VA `0x67f1d0` read at `0x54f8ac` (de-interleave fn `0x54f860`), VA `0x67f3d0` read at `0x54f972` (fn `0x54f920`), beside the C27 transforms *(offset-only)*; the permutation-exactness numbers stay the data walk (file `0x27f1d0`/`0x27f3d0`) | G | |
| C27 | Body transforms: bitwise-NOT, mirror S-box, u32-XOR key `"nami"` | EXE | `0x54f9e0`/`0x54fa70`/`0x54fac0` | G | |
| C28 | Id-less banks: new-variant entries register at slot 0 (last wins); only M30 carries ids | EXE | `strncpy`-only fetch, load/play path survey | G | **S4** context |
| C29 | OJM selection is id-keyed end to end; no order lookup exists (id-less entries all land at slot 0); `resolve_order_slot` is ours, not exe behavior | EXE | *(offset-only)* `0x1703f0` chain `0x170418`–`0x170431`, loader `0x173625`/`0x17364a`, fetch `0x14ee4a`/`0x14ef21` | G | corrects S4 |
| C30 | DialogBGM odd-stripe byte-swap is an encoder-side file anomaly; no exe compensates | DATA | correlation study + EXE negative sweep | G | |
| C31 | `JamTable.dat`: 54 u16 thresholds 25..4800; consumer unknown | DATA | envelope only; negative *(offset-only)*: zero `JamTable` string hits in the whole image (strings sweep) — the exe never opens it by name, so it is likely dead data | — | |
| C32 | `Itemdata_Korea.dat` 1541 rows / `SetInfodata.ojs` 209 rows mapped | EXE | *(offset-only)* both files open through the Image archive (`Itemdata_Korea.dat` push VA `0x52d095`; `SetInfodata.ojs` `0x52fa54`/`0x52fa6a`): each body leads with a u32 count (read via `0x406af0`), the exe reserves that many 388 B (`0x184`) / 80 B (`0x50`) rows (`0x532a30` at `0x52d101`–`0x52d139` into `+0x4`; `0x532fc0` at `0x52faf6`–`0x52fb2b` into `+0x16a4`), then parses rows by sequential field reads `0x4069b0` (`0x52d1cf`…, `0x52fbc1`…); the 1541/209 row counts stay the data walks | R | |
| C33 | Avatar sprite-slot parity: even male, odd female, fallback, slot order kept | ASSUMED | pattern heuristic; exe part logic open | R | |
| C34 | Difficulty mapping: +20 lo=Easy/hi=Normal, +24=Hard (ascending triple) | EXE | builder read `0x44268` ev `b5c3094a…`; order pinned by switch `0x170fa2`, ev `feebe490…` | G | corrects S5 |
| C35 | `+24` completes the Hard-level triple | EXE | same 3-slot read `0x44268` ev `b5c3094a…` (`u16[2]` = rec +28 = header +24); switch `0x170fa2` ev `feebe490…` | G | corrects S5 |
| C36 | `+12` genre bits feed the song-select bitmask (`song+0x15c`) | ASSUMED | bitmask-shaped tests; transfer unpinned — negative *(offset-only)*: no `+0x15c`/`+0x194` displacement anywhere in the builder region VA `0x44000`–`0x45000` | R | |
| C37 | First embedded JPEG bounds the last level slice on every chart | DATA | 14/14 `end == jpeg0` | G | |

## D. Net (lighter — offsets already pinned in `net-protocol.md`/`relay.md`)

| ID | Claim | Level | Anchor / evidence | Drives | Sibling |
|---|---|---|---|---|---|
| D1 | Framing: u16 id at buffer+2; length counts the whole frame | EXE | `0x1a0550`, ev `861713e2…`; server-verified | G | |
| D2 | Opcode family table (1000–65520) | ASSUMED | Memoryer cross-check; send/dispatch sites where cited | G | |
| D3 | Odd/even request/response pairing | EXE | every pinned pair is send↔handler in-exe: 4042 send VA `0x4c365c` ↔ 4043 handler file `0x159b50`; 4044 `0x4ead18` ↔ 4045 `0x163d70`/`0x18cf50`; 4046 `0x58d8a1` ↔ 4047 `0x163d90`; 4055 `0x4f0439` ↔ 4056 `0x163d50`; 4014 sends (files `0x16e660`/`0x16e7dd`/`0x4eaf3b`) ↔ 4015 handler `0x15fa20`; the convention beyond these pairs stays Memoryer-corroborated | G | |
| D4 | String fields: UTF-8, null-terminated, cap-inclusive truncation | ASSUMED | Encore codec; exe read-side consumes NUL-terminated strings (nickname reader file `0x155ea0` measures to the terminator, then consumes it — `relay.md` §10); terminator emission stays open | G | |
| D5 | 4014 sends: Jam per Cool/Good tail, Life on tick, mode-gated | EXE | `0x16e660`/`0x16e7dd`/`0x4eaf3b` | G | |
| D6 | 4015 body: u8 slot + u16 type + u16 value + 8 rank bytes; rank remap loop | EXE | `0x15fa20`, ev `0ff34c44…` | R+G | |
| D7 | 4016 SubmitScore wrapper sites; layout corroborated, life-field read open | EXE | *(offset-only)* `0x4eb018` et al. | G | |
| D8 | Relay ops 1–11 pinned by `SetID` ctors + vtable refs | EXE | ctor files `0x18e100`…`0x16e080` | G | |
| D9 | Relay body layouts (RelayStats 62 B et al.) | EXE | op-5 tally is exe-written: packer file `0xea230` stages 11 words + 4 dwords (38 B post-routing) from `0x77eeXX` globals in wire order, sends via file `0x187870`, single tick caller file `0xeb0c9` gated `0x77ee84==1` ∧ `0x77cd33==5` (ev `deb10e8f…`/`ccded3a4…`/`8f8df9d4…`, `docs/op5-writer.md`); routing preface staged as pushed `0x18`/`0x04`/`0x02` sizes through `0xeb4d0`/`0xeb570`/`0xeb550` (ev `881b4739…`); combo packs share the send path with autoplay pinned nonzero (relay.md §6); field names beyond count/order stay inferred, remaining op bodies stay Memoryer-corroborated | R+G | |
| D10 | MemberRole 0 None / 1 Champion / 2 Challenger / 3 Spectator gates | EXE | `0x77ee86` loads, ev `acacd596…` set | G | |
| D11 | Relay UDP ports 39311/39312; no hardcoded TCP relay ports; gateway 15010 | EXE | immediates `0x18c73e`…, ev `6e655b70…` | G | |
| D12 | Room row record `0xb4` field map = parse order | EXE | `0x15b780` parser sites, ev `d9f9a8b4…` | R | |
| D13 | `RoomState` 0/1/2, `Difficulty` 0–3, `GameSpeed` 0–11 value sets | EXE | every value renders: state byte → `Room_State.ojs` frame 0/1/2 (`0x968AE`, ev `453c83b4…`/`c63d088a…`), value-1 gates (ev `575ba6dd…`), difficulty byte indexes the level table (`0x95C52` et al.) and drives the `×12` speed-icon grid (`0x96751`, ev `5d40cd1e…`), speed label chain `0xC4137`–`0xC4371` (ev `f2fa3e80…`, C22) with the icon grid corroborating the 0–11 order (`room-enums.md`); Memoryer ordinals now corroboration, not source | R | closes S6 |
| D14 | Equip 5004/5005 shapes; slot from item record; type 5 → slot 11; `< 0x11` | EXE | *(offset-only)* `0xa4ec8`/`0xa3cc9`/`0xa4baa` | G | |
| D15 | Gift/shop shapes 5015–5033; 5040/5041 absent on 7.55 wire | EXE | *(offset-only)* dispatch sites in `gifts.md`/`shop-buy.md` | G | |
| D16 | Member-slot offsets `+0x04` id … `+0x218c` streak; derived slot marks | EXE | reader sites `0x159d38`–`0x159f82`; ev set in `relay.md` §11 | R+G | |
| D17 | UHP flow; 4046/4047/4055/4056 bodies empty/ignored | EXE | `0x18d8a1`/`0xf0439`/handlers `0x163d50`+ | G | |
| D18 | Live-state 4042–4045 shapes incl. `MemberInfo` tail order | EXE | `0x159b50`/`0x18cf50` chains | G | |
| D19 | Endpoints: FTP hosts, gateway IP/port, web/registry strings | EXE | string table `0x24b9c0`–`0x2514f0` | — | |
| D20 | Authorize 1000 payload order (token, gender string, marker, clientId) | ASSUMED | Memoryer order; exe byte-proof open; negative *(offset-only)*: image-wide scans find no 1000 send staging — all 46 `push 0x3E8` sites are non-packet uses (e.g. ReadFile size VA `0x5acd82`, tick timeout VA `0x5b71c9`), and the only 9 direct `SetID` (file `0x1a0550`) callers are the relay ctor family (files `0xea4f4`/`0x16dff4`/`0x16e094`/`0x18622c`/`0x18ca74`/`0x18cc64`/`0x18e114`/`0x18e1f4`/`0x1a0423`) | G | |

## Top-6 risky claims (visible behavior on the weakest evidence)

1. **A2/A20** — file-order pairing still taught by `opi-pattern.md`,
   `bnd-semantics.md`, and three doc comments; A1's code-ordinal rule
   supersedes it. Every bound scene depends on the mechanism.
2. **A3** — zip-and-truncate binding silently drops entries; the exe
   gives out-of-range ordinals an all-zero rect instead.
3. **A34** — curtain geometry (ring visuals) from item text; the exe
   clip math is untraced.
4. **A32/A35/A36** — HUD digits, receptor/note/burst pixels, and font
   metrics are invented placeholders behind genuine art.
5. **A29** — "unbound scene paints full-bleed" rests on the shell
   guess; the exe draws at the sprite anchor, so per-scene art decides.
6. **A38** — letterbox-vs-stretch under the fixed 800x600 fullscreen
   display rests on a photo; no exe scaling-path evidence either way.

## Maintenance

- Update a row when its level changes; keep one claim per row.
- New EXE evidence for an ASSUMED row retires its risk note.
- The sibling list is the checklist for every new positional claim:
  find the explicit ordinal/id field first.

# Result-screen anchors — `CStateResult` numbers and verdict (7.55) — PINNED

Where the 7.55 result screen paints score, max-combo, the
COOL/GOOD/BAD/MISS counts, per-member rows, and the WIN/LOSE verdict.
Un-blocks painting result digits at true positions (`docs/bnd-audit.md`
13 and the retired `(16,16)/(16,32)/(16,48)` anchors in
`crates/o2jam-shell/src/screens/paint.rs`).

Version pin: `AngelO2Jam.exe` == `OTwo.exe`, 5,029,888 bytes,
SHA-256 `83a04541…72c823b`; `Interface1.opi` 75,100,702 B;
`Playing1.opi` 221,737,026 B. VA = file offset + `0x400000`.

Method: read-only byte walks over the installed exe and archives (RTTI
chain, sprite headers, `.bnd` rects, LE32 reference scans), plus the
local headless-Ghidra project `extracted/ghidra-4015-work/proj4015`
opened read-only by a standalone dump reader
(`extracted/ghidra-cresult/DumpCResult.java`; decompiler scratch and
objdump slices stay git-ignored in `extracted/`). No disassembly or
decompiled code is reproduced; behavior is described in our own words.

Dossier: `extracted/rea-cresult-anchors-dossier.json`
(format `rea-dossier-bundle-v1`, dossier `rea-cresult-anchors`, 11
records, all `observed`; evidence id = md5 of
`rea-cresult-anchors|operation|subject-VA`).

## Answer first — the anchors

| Field | Anchor | Evidence |
|---|---|---|
| COOL | text `%5d` at (151, 125), Arial white, value u16 `0x77ee42` | `ev_615e034c…` |
| GOOD | text `%5d` at (240, 125), value `0x77ee44` | `ev_615e034c…` |
| BAD | text `%3d` at (337, 125), value `0x77ee46` | `ev_615e034c…` |
| MISS | text `%3d` at (426, 125), value `0x77ee48` | `ev_615e034c…` |
| MAX COMBO | text `%4d` at (505, 130), value `0x77ee5c` | `ev_615e034c…` |
| JAM COMBO | text `%4d` at (596, 130), value `0x77ee4c` | `ev_615e034c…` |
| Life readout | digit-sprite cells, 24 px pitch, last cell x=460, y=77, digit 1 kerned +5 | `ev_615e034c…` |
| Total score | text `%8d` at (605, 533), value `0x77cd2c` | `ev_615e034c…` |
| Bottom counter | digit-sprite cells, 24 px pitch, last cell x=176, y=536 | `ev_615e034c…` |
| Per-member rows | rank x=91, name x=128, COOL/GOOD/BAD/MISS x=230, combo pair x=416, score x=530, bonus x=626, speed x=685; 22 px row pitch | `ev_88c3ca31…` |
| WIN/LOSE verdict | `Result_WinLose.ojs` frames 0–3 at y=190; frame picked in `FUN_004c3120` | `ev_38f32576…`, `ev_bb4e402e…` |
| Grade plate | `Result_Imgs_Rank.ojs` frame 1–8 at (395, 531) | `ev_c61d99f8…` |

The judgement/combo/speed numbers are **GDI text**, not digit sprites.
Only the life readout, the bottom counter, and the grade plate use
sprite frames. `PointNum.ojs` and `Result_NumFont_Blue.ojs` are mounted
by the builder; the binding of the digit-cell fields to a font body is
OPEN (below).

## Class identity

Type descriptor `.?AVCStateResult@@` names VA `0x67e0ac`; one complete
object locator at VA `0x667400` references it; the vftable pointer to
that locator sits at file `0x24a220`, so the vftable is VA `0x64a220`
(24 slots). Slots: 4 enter `0x4c1b90`, 5 update `0x4c28e0`, 6 input
pump `0x4c2b20`, 8 keys `0x4c3850`, 10/11 buttons `0x4c2cf0`/`0x4c2ce0`,
16 window messages `0x4c36e0`, 23 net relay `0x4c2f00`. Slot 9
`0x48c570` is inherited from `CStateBase` (vftable `0x647a68`) and
draws every control each frame. (`ev_4c519351…`)

## Mount chain — `FUN_004c2390` (file `0xc2390`)

The builder loads, in order: `StateResult_Top.ojs`,
`StateResult_Bottom.ojs`, `Result_WinLose.ojs` from archive slot 1 into
row objects at `this+0x60[0..2]` (0x170-byte objects, sprite at +4);
`Result_NumFont_Blue.ojs` (archive 1) to `this+0x10d0c`; `PointNum.ojs`
(archive 2 = `Playing1.opi`) to `this+0x10d10`. It creates the GulimChe
8 px font (`this+100`, name string VA `0x64407c`) and Arial 8 px
(`this+0x68`), copies the member table (stride `0x2190`) from
`0x77ee90`, and builds eight avatar slots (`this+0x5c`) plus eight row
objects (`this+0x60[3..10]`), avatars positioned at x=78,
y=316+22·i under a 652×22 clip. All five sprite strings are referenced
only here. (`ev_903b5c6d…`)

Panel bodies, from the archive bytes: Top = one 800×316 image anchored
(0,0); Bottom = one 800×350 image anchored (0,250) — the panels overlap
y=250..316 and Top paints first, so Bottom covers the seam;
`State_Result.ojs` (block entry `0x17100001`) = one 800×600 image at
(0,0). (`ev_0ae116fc…`)

## Enter painter — `FUN_004c1b90` (slot 4)

Calls the builder (only caller, file `0xc1bc7`), shows the background,
BACK (`0x17200005`) and RETRY (`0x17202006`) controls and enables the
buttons, starts result BGM (event `0x13`), computes team ranks
(`FUN_004c49a0`: bucket by team byte `this+0x9d+member·0x2190`, sum
score fields `+0x21c4`+`+0x21cc`, rank bytes into `this+0x10d2c`),
shows the grade plate at frame 0, then runs the fill `FUN_004c3120`
(only caller, file `0xc1cd4`). Outside battle mode it fires the
event-ticket web call. (`ev_9957ce54…`)

## Verdict draw site — `FUN_004c3120`

Runs once from enter. It fetches the background body
(`FUN_0048bcf0(0x17100001)` → `FUN_0050dd90` → `FUN_0042fe90`), then
sets the verdict frame on the WinLose row object `this+0x60[2]`
(register flow read from an anchored slice: both the frame setter
`FUN_0042f870` and the draw `FUN_0042f040` receive that row object;
the background body rides as the composite argument):

- Solo (player-count global `0x778a50` ≤ 1): frame 4.
- Battle (`0x7789e6` == 1, role `0x77ee86` ∈ {1, 2}): my member's
  win-flag byte at `this + myIndex·0x2190 + 0x21d2` → frame 1 (value 1),
  frame 2 (value 2), else frame 3.
- Role 3 draws at frame 0.

After drawing it resets the frame to 0, then calls the top-panel
painter and the row painter, posts net event `0xfca` in battle mode,
and enables retry. (`ev_38f32576…`)

`Result_WinLose.ojs` (colorkey `0x0000`) holds exactly four 65 px
frames: image 0 (100,190) 595×65, image 1 (100,190) 595×65, image 2
(151,190) 498×65, image 3 (126,190) 547×65. With the frame-n-paints-
image-n−1 rule (setter pinned in `paint.rs`): rank-1 team → image 0,
rank-2 → image 1, other → image 2, solo → image 3. (`ev_bb4e402e…`)

## Number positions — top panel (`FUN_004c38a0`, file `0xc38a0`)

Six white Arial fields under the baked `StateResult_Top` headers,
through the text helper `FUN_0042c730` (color `0xffffff`):

- COOL `%5d` ← u16 `0x77ee42` at (151, 125); GOOD `%5d` ← `0x77ee44`
  at (240, 125); BAD `%3d` ← `0x77ee46` at (337, 125); MISS `%3d` ←
  `0x77ee48` at (426, 125).
- MAX COMBO `%4d` ← `0x77ee5c` at (505, 130); JAM COMBO `%4d` ←
  `0x77ee4c` at (596, 130).

Two digit-sprite fields use the frame-select/draw pair
`FUN_0041e0f0`/`FUN_0041df80` over eight fixed cells (24 px pitch,
least-significant digit rightmost, digit 1 shifted +5 px):

- Life readout: value = `0x77ee58 + 0x77ee60`, y=77, last cell x=460
  (cells 292/316/340/364/388/412/436/460).
- Bottom counter: value = `0x76fc66`, y=536, last cell x=176.

Total score: `%8d` ← `0x77cd2c` at (605, 533), Arial.

Grade: `FUN_004c4dd0` compares score `0x77ee60` against
`this+0x10d14` × 0.30/0.50/0.70/0.80/0.90/0.95 (f64 constants at
`0x64a500/0x644888/0x64a4f8/0x64a4f0/0x64a4e8/0x64a4e0`); a max-combo
== note-count−1 comparison (`0x77ee5c` == `0x778a00`−1) promotes grade
6 to 7; equal-to-max scores give 8. The grade byte lands in the
per-song/difficulty best table at `0x779814` and shows as the
`Result_Imgs_Rank` frame: eight frames, all anchored (395,531) ==
`State_Result.bnd` rect 7, sizes 35×41 ×2, 39×41 ×4, 38×41, 79×41;
frame n paints image n−1. (`ev_615e034c…`, `ev_c61d99f8…`)

## Number positions — member rows (`FUN_004c3b40`, file `0xc3b40`)

One call per qualifying member from the fill (sites `0xc3579` solo,
`0xc3631` battle). Row base y = 320 + 22·row: judgement text at
317+22·row, nickname at 318+22·row; the member's avatar sprite sits at
316+22·row. Columns, all white Arial (`this+0x68` font) unless noted:

| Column | x | Source |
|---|---|---|
| Rank number `%d` | 91 | `this+0x10d2c+member`; drawn only with 2+ players |
| Nickname (CP949, truncates over 100 chars) | 128 | record +5, GulimChe font |
| `%5d /%5d /%3d /%3d` | 230 | record +0x2142/+0x2144/+0x2146/+0x2148 — COOL/GOOD/BAD/MISS order |
| `%4d /%4d` | 416 | record +0x215c then +0x214c — MAX COMBO then JAM COMBO order |
| `%8d` | 530 | record +0x2160 + `0x77ee58` |
| `+ %d` | 626 | record +0x2164 |
| Speed `X 0.5`…`X R` | 685 | record +0x2168 through a fixed table |

The avatar is picked from the eight `this+0x5c` slots by the avatar id
byte at record +0x31 and drawn before the text. (`ev_88c3ca31…`)

## Block rects — `State_Result.bnd` (shell cross-reference)

One section, 7 rects of four u32 coordinates from offset 6. Entry code
low word pairs to rect ordinal (`code & 0xFFF` − 1):

| Rect | Coords | Entry |
|---|---|---|
| 1 | (0,0)-(800,600) | `IDC_BACK_SRESULT_1` `0x17100001` `State_Result.ojs` |
| 2 | (29,316)-(58,406) | `IDC_IMAGE_SRESULT_RANKTAPFIELD` `0x17600002` |
| 3 | (29,406)-(58,496) | `IDC_IMAGE_SRESULT_RESULTTAPFIELD` `0x17600003` |
| 4 | (0,0)-(800,600) | `IDC_IMAGE_SRESULT_RESULTTAP` `0x17500004` `Result_EachResult.ojs` (body 800×350 anchored (0,250)) |
| 5 | (711,569)-(794,593) | `IDC_BUTTON_SRESULT_BACK` `0x17200005` `Result_Back.ojs` |
| 6 | (625,570)-(708,592) | `IDC_BUTTON_SRESULT_RETRY` `0x17202006` `Result_PlayRetry.ojs` |
| 7 | (395,531)-(474,572) | `IDC_IMAGE_SRESULT_RANKINGFIELD` `0x17500007` `Result_Imgs_Rank.ojs` |

Rect coordinates are u32 (not i16); each control paints itself per
frame in the inherited slot-9 draw-all. (`ev_df8e0ca4…`)

## Update and exits (`FUN_004c28e0`, `FUN_004c2cf0`, `FUN_004c2f00`)

Update animates a count-up into `this+0x4349` (+10 per tick); past 100
it enables back/retry via `FUN_004c47a0`. Back sends net event `0xfb5`;
retry sends `0xfaa` and is blocked with a waiting-user message (string
VA `0x64a3a0`) while siblings are ready. The relay wraps
`0xfb5`/`0xfaa`/`0xfca`/`0xbbd` for the sender. (`ev_77ce98f1…`)

## OPEN

1. Digit-font binding for the two digit-cell fields (life readout,
   bottom counter): the shared digit painter bodies (files `0xc44c0`/
   `0xc4630`) build eight cells as `last - 168 ..= last` in 24 px
   steps and shift digit 1 by +5, confirming the dossier geometry
   (life y=77 last x=460; bottom y=536 last x=176) from the register
   flow alone. The painters receive the background-control body
   through the fetch chain, and the body backing those frames is not
   pinned to a loaded sprite name. `PointNum.ojs` metrics match the
   24 px cells and the +5 kern of the 12 px-wide digit 1
   ((23−12)/2 ≈ 5), but the attachment is inference, not observed.
   Shell consequence: `paint.rs` stages neither field (new
   `life_and_bottom_digit_cells_stay_unpainted` test).
2. `Result_NumFont_Blue.ojs` use site: mounted at `this+0x10d0c`, no
   reader found in the painter set.
3. Stat-name mapping for the globals: COOL/GOOD/BAD/MISS ↔
   `0x77ee42/44/46/48` and MAX/JAM COMBO ↔ `0x77ee5c`/`0x77ee4c` rest
   on column order matching the baked headers (and the 4016 field
   order), not on a proven write site. `0x77ee58` is added to both the
   life readout and each row score; `0x77ee60` feeds the grade chooser
   — exact semantics OPEN.
4. Row-record fields `+0x2142`… are painter-relative; the record's
   stride bookkeeping inside the state (0x864×4 = 0x2190 in the call
   sites) is not normalized to a named layout. The row painter
   (file `0xc3b40`) passes them as fixed combined strings —
   `"%5d /%5d /%3d /%3d"` at x=230, `"%4d /%4d"` at x=416,
   `"%8d"` of record `+0x2160` plus the `0x77ee58` global at x=530,
   `"+ %d"` of record `+0x2164` at x=626, the `X 0.5`…`X R` chain of
   record `+0x2168` at x=685, nickname at x=128 (truncated past 100
   display chars), rank `%d` at x=91 only with 2+ players — so the
   columns are closed while the record layout stays OPEN. Shell
   consequence: no member-row model exists, so `paint.rs` paints no
   rows (new `member_rows_stay_unpainted_without_a_row_model` test).
5. Text helper `FUN_0042c730` alignment (left vs right anchor of the x
   argument) not pinned: every call in the top painter (file
   `0xc38a0`), the row painter (file `0xc3b40`), and the width
   wrapper (which selects `%3d`/`%4d`/`%5d`) passes identical
   trailing args `(0, 3, 4)`, and the helper body itself is not
   dumped. Column x values are verified as passed; `paint.rs` keeps
   each passed x as the left edge (existing per-field tests assert
   the pixel left of each anchor stays unpainted).
6. Top/Bottom row objects (`this+0x60[0..1]`) are never drawn by the
   fill (file `0xc3120` issues draws only for the WinLose row object,
   then the top-panel painter and the row painter); the commit call
   `FUN_0042f5a0(1)` after each mount is presumed to register them
   with the per-frame renderer (slot 9, `FUN_00448c570` heritage).
   Mechanism OPEN. Shell consequence: `paint_result_backdrop` stages
   the Top/Bottom bodies ((0,0) and (0,250)) and skips only the frame
   bodies (new `backdrop_keeps_top_and_bottom_panels` test).
7. The `+ %d` bonus column (record +0x2164, `"+ %d"` at x=626 in
   file `0xc3b40`) semantic (long-note bonus? per the 4016 field
   order) OPEN. Unpainted with the rows (see 4).
8. Grade 6→7 promotion compares max combo to note count − 1
   (`0x77ee5c` == `0x778a00`−1); what `0x778a00` counts (notes or
   notes−1) OPEN.
9. `0x10d14` (max-score reference next to the `PointNum.ojs` handle at
   `0x10d10`): write site not located.
10. Role 3 (observer) draws the verdict banner at frame 0 (image 0);
    whether observers really see the rank-1 banner OPEN.

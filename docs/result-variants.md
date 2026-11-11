# Result-screen variants: solo versus battle/team (7.55) — PINNED

Version pin: `AngelO2Jam.exe` == `OTwo.exe`, 5,029,888 bytes,
SHA-256 `83a04541…72c823b`. Parent anchors live in
`docs/cresult-anchors.md`; lifecycle context is `docs/lifecycle.md`
§11. This note only separates the solo path from the battle path.
No disassembly is reproduced; behavior is described in our own
words with file offsets and REA evidence ids.

## Variant rules

1. Solo verdict: player-count global `0x778a50` ≤ 1 selects frame 4
   in the fill (`FUN_004c3120`, file `0xc3120`). Frame 4 paints
   image 3 of `Result_WinLose.ojs` at (126,190), 547×65, the solo
   banner. (`ev_38f32576…`, `ev_bb4e402e…`)
2. Battle verdict: battle byte `0x7789e6` == 1 with role word
   `0x77ee86` ∈ {1, 2} reads the local member win-flag byte at
   `this + myIndex·0x2190 + 0x21d2`: value 1 → frame 1, value 2 →
   frame 2, anything else → frame 3. Frame 1 paints image 0 at
   (100,190), 595×65 (rank 1); frame 2 paints image 1 at (100,190),
   595×65 (rank 2); frame 3 paints image 2 at (151,190), 498×65
   (other). Role 3 draws frame 0. (`ev_38f32576…`, `ev_bb4e402e…`)
3. Shell mapping (`crates/o2jam-shell/src/screens/paint.rs`
   `winlose_frame`): `ResultVerdict::Solo` → frame 4;
   `Battle(Win)` → frame 1; `Battle(Lose)` → frame 2;
   `Battle(Other)` → frame 3; `Battle(None)` → no banner (the
   observer frame-0 arm stays OPEN, note 8 below).
4. Rank table row count: one call per qualifying member of the row
   painter (`FUN_004c3b40`, file `0xc3b40`). Solo fill site
   (file `0xc3579`) qualifies one row; battle fill site
   (file `0xc3631`) qualifies each member row, up to eight seats.
   Row base y = 320 + 22·row; columns x = 91/128/230/416/530/626/685.
   (`ev_88c3ca31…`)
5. Rank-number column: `%d` at x=91 paints only with 2+ players.
   Solo rows carry no rank number; battle rows do. All other
   columns keep their x in both modes. (`ev_88c3ca31…`)
6. Grade plate: identical chooser in both modes
   (`FUN_004c4dd0`). Score global `0x77ee60` climbs reference
   `this+0x10d14` × 0.30/0.50/0.70/0.80/0.90/0.95 (f64 constants at
   `0x64a500/0x644888/0x64a4f8/0x64a4f0/0x64a4e8/0x64a4e0`) to
   frames 1–6; max combo == note count − 1 (`0x77ee5c`
   == `0x778a00`−1) promotes 6 to 7; score equal to max gives 8.
   The plate is `Result_Imgs_Rank.ojs` frames 1–8 at (395,531).
   (`ev_615e034c…`, `ev_c61d99f8…`)
7. Total score: `%8d` ← `0x77cd2c` at (605,533), white Arial, in
   both modes. It overlaps the left edge of the TOTAL GEM bar. No
   gem model is pinned; the bar body is staged art. Bottom counter
   digit cells (value `0x76fc66`, y=536, last cell x=176) are a
   separate OPEN field. (`ev_615e034c…`)

## Left-tab identity and count

`State_Result.bnd` carries 7 rects of four u32 coordinates from
offset 6 (`ev_df8e0ca4…`). The left rail holds exactly two tabs;
the seventh rect is the grade field, not a tab:

| Tab | Rect | Entry |
|---|---|---|
| Rank tap | (29,316)-(58,406) | `IDC_IMAGE_SRESULT_RANKTAPFIELD` `0x17600002` |
| Result tap | (29,406)-(58,496) | `IDC_IMAGE_SRESULT_RESULTTAPFIELD` `0x17600003` |

The result pane itself is `IDC_IMAGE_SRESULT_RESULTTAP`
`0x17500004` (`Result_EachResult.ojs`, body 800×350 anchored
(0,250)). The remaining rects are the full-screen back
(`0x17100001`), BACK (`0x17200005`), RETRY (`0x17202006`), and
the grade field (`0x17500007`). Tab switching behavior is OPEN:
no selected-tab frame rule is pinned.

## Reference photo classification

The reference photo (2 rows, LOSE banner, grade C, TOTAL GEM
110371) is the battle variant: two member rows, a rank column,
and a battle loss banner in the y=190 band. The solo variant
shows instead: one member row, no rank number at x=91, the solo
banner image 3 in the same band (the scripted walk renders
"GOOD!"), the same top-panel numbers at (151/240/337/426, 125)
and (505/596, 130), and a grade plate from the same threshold
table at (395,531).

## Walked solo capture (`result-solo.png`, `result-solo-native.png`)

`cargo run -p recomp2jam -- shots` drives a second game with a
peer-free Normal room (`crates/recomp2jam/src/shots.rs`
`drive_solo_to_result`): the shared one-note chart struck once at
5 s, painted at 7 s. Every value is walked state — no battle
scoreboard, no scripted ranks:

- Verdict `ResultVerdict::Solo` → WinLose frame 4 → image 3, the
  solo banner in the y=190 band (rule 1), proven by
  `solo_shot_paints_the_solo_banner_frame`.
- One rank row (`YOU`, counts 1/0/0/0, 200 points, `+ 0`, X1)
  and no `%d` at x=91: a single row hides the rank number
  (rule 5), proven by `solo_walk_builds_one_rank_free_row` and
  `solo_shot_paints_the_single_row_without_rank`.
- Grade plate from the walked play data (rule 6): one Cool equals
  the all-Cool max, so the equal-to-max arm gives grade 8
  (image 7 at (395,531)), proven by
  `solo_shot_paints_the_walked_grade_plate`.
- Same top-panel numbers as the battle walk (rule 7 and the
  `FUN_004c38a0` anchors): the same struck onset feeds the same
  `Score`, so (151/240/337/426, 125), (505/596, 130), and the
  total 200 at (605,533) match `result.png`.

Against the reference photo (battle variant, above): the solo
shots show the frame-4 solo banner where the photo shows the
frame-2 LOSE! banner, one rank-free row where the photo shows
two numbered rows, and the same number/grade geometry otherwise.
The battle walk (`result.png`, `result-native.png`) is unchanged.

## Team tie-break display order — sharpened OPEN (claim D26, issue #94)

Static sweep (Python disp32 scans + objdump 2.47 slices over the
read-only 7.55 copy) of every reference to the win-flag byte offset
`0x21d2` and the score field `0x2160` in the member slot
(`0x77ee90` + member × `0x2190`), plus the named ticket districts:

- `0x21d2` (win flag): the only direct references are the four known
  verdict-fill reads (files `0xc31ca`/`0xc320c`/`0xc3300`/`0xc3343`,
  rule 2 above). No direct store exists image-wide; the writer is
  indirect (B36 open item).
- `0x2160` (score): readers are the row-record column format site
  (file `0xc3c8b`, which adds one member score into the life/score
  header global `0x77ee58` — not a team grouping) and the member
  bulk-copy pair in `FUN_004e92a0` (file `0xe9565`). Writers are only
  the 40xx row readers (files `0x14081d`/`0x1412d0`). No site sums
  scores per team and no site compares two members' scores for a
  winner.
- Result region (`FUN_004c2390` opener, `FUN_004c3120` verdict fill,
  `FUN_004c38a0` row fill): no team-total arithmetic and no ordering
  loop anywhere. The winner banner is the per-member win flag keyed
  by role (rule 2); rank rows are one painter call per qualifying
  member in seat order (rule 4).
- Waiting room: `FUN_0048c810` (the ticket's "team-map" district;
  the companion address `0x48b810` has zero byte-image hits and is a
  stale name) is input-side only — it lights exactly the pressed
  button and returns `button − 0x16201004`. The eight team buttons
  are a fixed grid (`0x16201004..0xB` → 0..7); no data-driven
  ordering of teams exists.

Conclusion: the client holds no tie-break rule at all. On equal
server-side totals the displayed winner rides the server-written
per-member win-flag byte, and every display order is the fixed
seat/button order 0..7. The `RoomTeams::winner` lower-team-id
tie-break is therefore labeled ASSUMPTION (claim D26), not exe
behavior.

Named next step (needs the server, so a sharpened OPEN is the
delivery here): Memoryer live relay through the logging proxy
(`docs/relay.md` §12) with both teams' `4014`/`4016` submissions
scripted to equal totals (or the proxy rewriting the 4018 rows to
equal scores per `~/scratch/digits-rig/proxy.py`), then a rig
capture reading which member slot(s) get win-flag `1` on the
result screen — turning the server tie rule into a pixel read.
Fallback: REA trace of the indirect win-flag store naming the 40xx
handler that computes it; if the flag is computed in-exe after all,
the tie-break becomes statically pinnable from that handler.

## OPEN items

1. Tab selection: which tab is lit in solo versus battle, and
   whether the rank tap shows anything in solo, is not pinned.
2. Digit-cell VALUE semantics for the life readout (value
   `0x77ee58` + `0x77ee60`, y=77, last cell x=460) and the bottom
   counter (value `0x76fc66`, y=536, last cell x=176). The FONT
   binding is PINNED (#86): painter file `0xc44c0` binds
   `this+0x10d10` = `PointNum.ojs`; painter file `0xc4630` binds
   `this+0x10d0c` = `Result_NumFont_Blue.ojs`. The shell stages both
   fonts and paints a field only when a caller stages a resolved
   value.
3. ~~`Result_NumFont_Blue.ojs` mount~~ CLOSED (#86): its only
   reader image-wide is the bottom-counter painter (file `0xc4630`).
   `PointNum.ojs` attachment is pinned the same way (see 2).
4. Row-record layout behind `+0x2142`… stays OPEN; columns are
   closed as passed strings (`%5d /%5d /%3d /%3d` at x=230,
   `%4d /%4d` at x=416, `%8d` of record `+0x2160` plus global
   `0x77ee58` at x=530, `+ %d` of `+0x2164` at x=626, speed chain
   of `+0x2168` at x=685, nickname at x=128, rank at x=91).
   Shell paints no member rows without a row model.
5. Bonus column semantic (record `+0x2164`) is OPEN.
6. Text-helper alignment (left versus right anchor of x in
   `FUN_0042c730`) is OPEN; shell keeps each passed x as the left
   edge.
7. Grade reference `this+0x10d14` write site and stat-name
   semantics (`0x77ee58`, `0x77ee60`) are OPEN.
8. Role 3 draws verdict frame 0; whether observers see the rank-1
   banner is OPEN.

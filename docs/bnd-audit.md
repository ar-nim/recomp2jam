# Render-path coordinate approximation audit (`.bnd` ground truth)

Scope: `crates/o2jam-render/src/lib.rs`, `crates/o2jam-shell/src/screens/paint.rs`,
`crates/o2jam-shell/src/avatar.rs`, `crates/o2jam-shell/src/playing.rs`,
`crates/recomp2jam/src/shots.rs`.
Ground truth: every `ControlList` block that names a `BOUND "<name>.bnd"` file owns its
layout; rect `i` in that `.bnd` positions the block entry at that file offset on the
native 800x600 canvas (`NATIVE_WIDTH`/`NATIVE_HEIGHT`, see `docs/file-formats/opi.md`).
Anything below that invents position, size, order, or scaling instead of reading that
rect is an approximation.

## High severity (layout contradicts or discards `.bnd` data)

1. `o2jam-render/src/lib.rs:457 bind_controls` zips `codes × has_sprite × rects` in
   file order and truncates to the shortest input. Extra entries ride unlaid-out with no
   marker. Ground truth: the Playing subset ships fewer rects than entries, so each
   dropped entry still has a `.bnd`-addressable slot or an explicit "unplaced" state;
   silent truncation is not that.
2. `o2jam-shell/src/avatar.rs:255 bind_scene_rects` skips entry 0 (block header) and
   zips remaining entries with `bnd.rects` in file order, truncating to the shorter side
   (Room: 76 entries vs 75 rects; Playing: 235 entries vs 17 rects). Ground truth: the
   per-block `.bnd` pairing rule (which entry each rect addresses, what an unmatched
   entry means) is unverified; tails are dropped instead of reported.
3. `o2jam-shell/src/avatar.rs:294 placement_origin_in_frame` scales only the rect origin
   and discards the scaled width/height. Ground truth: the `.bnd` rect also sizes the
   control; callers paint native-size art at that point, so a control whose `.bnd` rect
   is larger/smaller than its sprite is mis-sized by construction.
4. `o2jam-shell/src/playing.rs:408 HighwayGeometry::centered` sets `x0 = 0`,
   `lane_w = frame_w / 7`, `line_y = h - h / 6`. Ground truth: `State_Playing.bnd`
   (17 rects) plus `KeydownImage.ojs` native row 487 fix the lane columns and the
   judgement-line row; full-width equal lanes with the line one-sixth up is invented.
5. `o2jam-shell/src/playing.rs:420 HighwayGeometry::lane_rect` tiles seven equal,
   gapless, full-height columns and clamps lane index. Ground truth: per-lane `.bnd`
   columns, margins, and the `Note_Image.ojs` head widths (28/22/28/32/28/22/28) are
   unequal; equal tiling contradicts staged art.
6. `o2jam-shell/src/playing.rs paint_note` draws notes `column.w - 2` wide,
   `NOTE_H = 8` tall with a 1 px `NOTE_CAP`. Ground truth: staged note-head extents
   per lane from `Note_Image.ojs`; the 8 px height and 1 px cap are procedural guesses.
7. `o2jam-shell/src/playing.rs paint_receptor` draws the rest pad at
   `x + 1, line_y - 5, w - 2, 5` (`RECEPTOR_H = 5`). Ground truth: the five
   `KeyEffect_Tutorial_<1..7>.ojs` receptor frames and native row 487 fix pad geometry;
   the 5 px box is invented.
8. `o2jam-shell/src/playing.rs:1283 paint_peer_bars` anchors a top-right strip at
   `x = w - count * (3 + 1) - 1`, `PEER_BAR_TOP = 4`, bars `3 × 20` (`PEER_BAR_WIDTH /
   HEIGHT / GAP`). Ground truth: the 4015 peer-slot bar rects in the Playing `.bnd` /
   `Playing1.opi` gauge art; edge, size, and growth direction are assumed.
9. `o2jam-shell/src/playing.rs:1328 paint_jam_gauge` pins edge bars `4 × 24`
   (`JAM_BAR_WIDTH / HEIGHT`) at `x = 1` and `x = w - 5`, `JAM_BAR_TOP = 28`, with
   top-centered `Note_JamNum` digits. Ground truth: `Jamgauge_right_bar.ojs` /
   `Jamgauge_left_bar.ojs` shells (VAs `0x64b1b8`/`0x64b1d0`, loader `FUN_004e7850`) and
   the jam-number rects; width, height, top row, and digit anchor are assumed.
10. `o2jam-shell/src/screens/paint.rs:201 paint_list_scrim` builds the list scrim at
    `w / 4 - 8, h / 4 - 2, w / 2 + 16, rows * 8 + 4`. Ground truth: the `IDC_LIST` rect
    in `State_Room.bnd` (and the channel dialog-bar rect); quarter-frame anchoring is
    invented.
11. `o2jam-shell/src/screens/paint.rs:248 index_as_row` places row `i` at
    `h / 4 + i * 8` (`CHANNEL_ROW_H = 8`). Ground truth: `.bnd` row rects / font metrics;
    the 8 px step and quarter-frame start are assumed for channel and room lists alike.
12. `o2jam-shell/src/screens/paint.rs:680 seat_origin` lays out seats as 2 columns × 4
    rows at `w / 8 + col * (w / 2)`, `h / 4 + row * 8`. Ground truth: the eight seat /
    team-button / avatar-cover rects in `State_Waiting.bnd`; the grid fractions are
    invented.
13. `o2jam-shell/src/screens/paint.rs paint_result_content` paints the avatar at
    `(w / 2 - 32, h / 2 - 32)` and score/combo/verdict digits at `(16, 16)`, `(16, 32)`,
    `(16, 48)` behind a scrim at `(8, 8, 64, 52)`. Ground truth: the 7 rects of
    `State_Result.bnd` (top/bottom/winlose panels, number fonts, avatar slot); all four
    anchors and the 64 × 52 scrim are fixed-pixel guesses.
14. `recomp2jam/src/shots.rs:134 full_canvas` stages art at native
    `(0, 0, 800, 600)` full-bleed. Ground truth: each sprite has a native `.bnd` anchor;
    stretching one body over the whole canvas distorts it by construction.
15. `recomp2jam/src/shots.rs:95,190 CHANNEL_FALLBACK_SPRITE` stages
    `Dialog_ChannelSelectBar.ojs` full-canvas because `STATE_CHANNEL` names zero sprites.
    Ground truth: the dialog bar's real rect is one panel, not the canvas; full-bleed is
    a known distortion kept for screenshots.
16. `recomp2jam/src/shots.rs:99,229-230 PLAYING_GAUGE_SPRITE` appends
    `Avatar_Info_LifeGauge.ojs` as an extra full-canvas placement over the 17-rect bound
    window. Ground truth: that gauge has one corner native anchor "clear of the note
    lane"; painting it canvas-wide contradicts its own comment.

## Medium severity (scaling, centering, or sizing guess)

17. `o2jam-render/src/lib.rs:491 scale_rect_to_frame` (`scale_axis` / `scale_extent`)
    scales each axis independently (`value * frame_len / 800|600`). Ground truth: native
    rects address 800 × 600; on any other frame this stretches rather than letterboxes /
    re-anchors, so aspect and edge alignment drift.
18. `o2jam-render/src/lib.rs:507 center_rect` centers a box at
    `((frame - inner) / 2)` and clamps oversize to the frame. Ground truth: Waiting
    portraits and Result figures have `.bnd` slots; blind centering ignores them.
19. `o2jam-shell/src/avatar.rs:337 avatar_center_origin` centers `figure.extent()` via
    `center_rect`. Ground truth: same as 18; the shared Waiting/Result anchor is assumed.
20. `o2jam-render/src/lib.rs:241 visible_spans` splits the highway into integer fifths
    (`h / 5`, remainder dropped). Ground truth: `Itemdata_Korea.dat` ring text names
    20% bands but clip math is OPEN; truncation misplaces every band edge on heights
    not divisible by 5 (e.g. 70 × 60 test frames, 400 × 300 shots).
21. `o2jam-render/src/lib.rs stretch_blit / blit_image_in_rect / blit_rgb555_scaled`
    nearest-neighbor-stretches any size mismatch into `dest`. Ground truth: `.bnd` rects
    position controls; whether a mismatched body should stretch, clip, tile, or stay 1:1
    at the origin is per-control behavior, not a global stretch rule.
22. `o2jam-shell/src/avatar.rs paint_scene_figures / paint_placed_figure` paints each
    figure at its scaled origin plus record anchor (`dx`, `dy`), skipping placements
    with no loaded figure silently. Ground truth: missing bodies and anchor-vs-rect
    reconciliation (record anchor inside a `.bnd` rect) need explicit policy.
23. `o2jam-shell/src/screens/paint.rs:134 unbound_scene_placements` gives splash,
    channel, and loading bodies the full native canvas. Ground truth: those states have
    a bare `BOUND` line (no `.bnd`), but "no bind file" does not prove "every body is
    full-bleed"; per-sprite native sizes are ignored.
24. `o2jam-shell/src/screens/paint.rs:462 room_detail_bnd_origin` scales the
    `State_Room.bnd` rect origin and ignores its size. Ground truth: the nine detail
    `IDC_*` rects size their fields; text is drawn from the corner with micro-font
    metrics instead of fitting the rect (see also 3).
25. `o2jam-shell/src/screens/paint.rs:474 room_detail_fallback_origin` stacks detail
    lines under the list at `w / 4, h / 4 + rows * 8 + 4 + slot * 8`. Ground truth: only
    valid while no staged art names the `IDC_*` control; the stack fractions mirror 10–11.
26. `o2jam-shell/src/screens/paint.rs:168 title_origin` centers titles at
    `w / 2 - text_w / 2, h / 2 - h / 8 + 2`. Ground truth: lobby title `IDC_*` rects;
    centering via micro-font `text_width` assumes the font, not the `.bnd` slot.
27. `o2jam-shell/src/playing.rs:110 SkinSprite::centered_x` centers staged art over the
    lane (`column.x + (lane_w - art_w) / 2`) and clips overflow. Ground truth: how wide
    bodies (e.g. 32 px heads over narrow lanes) overhang is original overlap behavior,
    currently assumed symmetric.
28. `o2jam-shell/src/playing.rs:130 paint_bottom_anchored` anchors tall columns by
    their bottom at `line_y` (glow rows "in the bottom quarter"). Ground truth: stem /
    beam native anchors in `Keyeffect.ojs` / tutorial bodies; bottom-anchoring plus the
    "bottom quarter" glow claim are assumed.
29. `o2jam-shell/src/playing.rs:383 burst_rect` builds the hit burst at
    `x + 2, line - 5 - 24, w - 4, 24` (`BURST_H = 24`). Ground truth: burst column rect
    from staged `Keyeffect` art; insets and height are fixed-pixel guesses.
30. `o2jam-shell/src/playing.rs paint_curtained / tail_shown` synthesizes the highway
    rect as `(0, 0, frame.w, line_y)` for `visible_spans`. Ground truth: the curtain
    applies to the `.bnd` lane columns, not to a frame-derived box; `x0` offsets and
    lane gaps are dropped before clipping.
31. `o2jam-shell/src/playing.rs paint_judgement_word / paint_jam_level / LivePlay::paint`
    centers words/digits at `w / 2` with rows `line_y - 40`, `line_y - 24`, top row 4,
    600 ms fade, and score/combo at `(16, 16)` / `(16, 32)`. Ground truth: judgement,
    combo, and jam-number `.bnd` rects; center-anchoring plus fixed row offsets assume
    the layout.
32. `o2jam-shell/src/playing.rs y_for_onset` maps song time to rows linearly
    (`line_y * ahead / lookahead`, `DEFAULT_LOOKAHEAD_MS = 2000`). Ground truth: scroll
    speed/shape is a layout-behavior guess (constant velocity, zero easing); shared by
    notes and tail beams so both inherit the error.
33. `recomp2jam/src/shots.rs:190-230 stage_bound_block / stage_playing_archive` keep
    partial staging when any figure loads and any placement binds. Ground truth: a half-
    staged scene mixes `.bnd` positions with fallback paint; the "never blank" rule hides
    exactly which controls are unbound.

## Low severity (cosmetic padding, test-only frames)

34. `o2jam-shell/src/screens/paint.rs paint_title_over_art` scrim
    `(x - 4, y - 2, text_w + 8, 9)`; `paint_room_detail_line` and `paint_cursor`
    (`3 × 5` at `text_x - 6`) and `paint_seat_scrim` (`x - 8, y - 1, w + 12, 7`)
    use fixed-pixel pads. Ground truth: panel padding comes from staged OPI panels;
    the pads only keep micro-font ink readable and never move controls.
35. `o2jam-shell/src/screens/paint.rs:807 paint_loading_bar` draws the track at
    `w / 4 - 2, h / 2 - 10, w / 2 + 4, 20` and the fill at `w / 4, h / 2 - 8, w / 2, 16`.
    Ground truth: `MusicLoading.ojs` gate layout; a centered bar is a placeholder, and
    no progress fraction is even wired yet.
36. `o2jam-shell/src/screens/paint.rs:1013 paint_card` (`w / 2 - w / 8`,
    `h / 2 - h / 8, w / 4, h / 4`) and `fill_background / background_of` flat colors.
    Ground truth: none — these are the documented no-art fallbacks; they only matter
    insofar as tests pin them (`flat_fallback_covers_bare_frames`).
37. `o2jam-render/src/lib.rs rect_from_corners` saturates inverted corners
    (`x2 < x1`) to zero size. Ground truth: `.bnd` corner order is documented
    `(x1, y1, x2, y2)`; the saturation hides corrupt rects instead of surfacing them.
38. `o2jam-shell/src/avatar.rs AvatarFigure::extent` folds `dx.max(0)` / `dy.max(0)`,
    ignoring negative record anchors. Ground truth: sprite-55 record anchors can lead
    the origin; negative overhang is clipped from the measured extent (paint still
    honors it), so centering (19) drifts on such bodies.
39. `recomp2jam/src/shots.rs SHOT_W / SHOT_H = 400 × 300` captures scale every `.bnd`
    rect through the proportional scaler (17). Ground truth: 800 × 600 native; shot
    pixels exercise the stretcher, not native placement.

## Counts

- High: 16 (items 1–16).
- Medium: 17 (items 17–33).
- Low: 6 (items 34–39).
- Total: 39.

# Playing scene reference (user photo, 2026-11-07)

Source: phone photo of real 7.55 gameplay, 1477x830, 4:3 UI stretched
to 16:9. Game area is the 4:3 box: height 830 → width ~1106,
scale ~1.3825 px per game px. All game px below divide photo px
by 1.3825.

## Regions (game px, 800x600)

- Highway panel: x 0..~330, full height minus bottom strip.
  7 lanes, judgement line near bottom (~y 470), key flashes on
  the line, notes fall above. Big COMBO digits (~260) + tier word
  (COOL!) + LONGNOTE COMBO over the lanes. JAM bar at the panel
  bottom. Panel bottom strip: MAX COMBO 260 left, score 65595
  center, BO / EA05 O2JAM edge labels.
- Avatar scene: x ~330..800, y 0..~540. 3D rooftop backdrop
  (we approximate with staged 2D backdrop art), avatar figure
  center-right with life bar + `Lv.43 <name>` above it, song
  info line (`X3.5 <title>`) lower area, VOL readout.
- Bottom tally strip: y ~540..600 full width. COOL/GOOD/BAD/MISS
  counts, TIME bar, song title, EXIT button right.

## Corrections vs our current paint

1. Highway is LEFT ~40%, not full width. Our full-width lanes
   are wrong; lane rects must live inside the left panel.
2. Right side is avatar/backdrop scene, not black. Stage
   backdrop art + worn figure + life bar there.
3. Bottom strip is a tally bar (counts, time, song, vol, exit),
   not empty. Paint counts from `Score`, song/speed/vol from
   flow state.
4. Present must letterbox 4:3, never stretch (photo shows what
   stretch does to the art).
5. Judgement words + combo digits sit OVER the lanes (we do
   this), big and gold/white (we use small white — upgrade
   with the big font).

## Status

Layout truth recorded from the photo; `.bnd` rects for the
Playing panel edges still rule exact pixels (see
`docs/bnd-semantics.md`). Panel split proportions above are
photo-measured, subordinate to rects where they disagree.

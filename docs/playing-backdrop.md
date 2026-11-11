# The Playing BACK1 backdrop (BO/EA05 edge labels, full-screen venue pass) — 7.55

Question: the reference photo's highway-panel bottom strip is a dark
metallic band carrying a `BO` corner label and a vertical `EA05 O2JAM`
edge label (`docs/playing-reference.md`), and the shell painted neither.
Where does the 7.55 client source them — staged sprite, runtime string,
or not at all in this build? Method: read-only byte walks with Python
3.14 over `Playing1.opi` plus read-only objdump 2.44 static analysis of
the exe — the same offset-only precedent as `docs/notebg-cover.md` and
`docs/stage-bga.md`; **no REA dossier IDs issued**. No disassembly
reproduced. Version pins (local copies re-hashed):

- `AngelO2Jam.exe` 5,029,888 B, SHA-256 `83a04541…72c823b`.
- `Playing1.opi` 221,737,026 B, SHA-256 `8c1c7566…aed9eed40`.

## Answer

**Both labels are baked art of the runtime-built BACK1 backdrop body.**
The Playing setup re-stages control `IDC_IMAGE_SPLAY_BACK1`
(`0x1B500071`) onto a mode-family full-screen backdrop, and every
shipped family body carries the same bottom-left band art: the dark
strip, the `BO` corner glyph (game box (4..40, 570..596)), and the
vertical `EA05 O2JAM` glyph (game box (285..332, 560..600)). Verified
by decoding every full-screen Playing body in `Playing1.opi`:

- `Playing_NormalBG1..3.ojs`, `Playing_P2PBG1..2.ojs`: two 800x600
  raw-RGB555 frames each (1,920,048 B bodies, 960,000 B blobs,
  colorkey `0x0000`), all frames byte-identical in the
  y ≥ 530 band — the labels and strip art are family- and
  frame-invariant (frame pairs differ only in x 21..=230, y 0..=530
  for the solo bodies, x 21..=778 for the P2P pair — the lane-panel
  band the staged `Note_BG.ojs` strip covers anyway).
- `Playing_TutorialBG.ojs`: one 800x600 frame, same band.

### The staging function (offsets only)

The setup function at VA `0x4dc750` builds its staging record on the
stack (0x11c B, code word `0x1B500071` stored at file `0x4dc81c`),
sprintf's the sprite name through the family format
(`Playing_NormalBG%d.ojs` VA `0x64ac44`, `Playing_P2PBG%d.ojs` VA
`0x64acec`, `Playing_TutorialBG.ojs` VA `0x64ad1c`), stages it through
the factory `0x48B2D0` (sprite fetch `0x50DD90`, slot store
`+0x53c`), and drives the frame with the setter `0x42F870`:

- `%d`: the second function arg divided by 3 (`div ecx` with
  `ecx = 3`, file `0x4dc79f`), remainder stored to the sprintf arg
  (file `0x4dc7a6`), a zero remainder replaced by 3
  (files `0x4dc7ac`–`0x4dc7bc`) → `%d ∈ {1, 2, 3}` cycling with the
  arg. **Which quantity the caller passes stays OPEN** (named next
  step below); the shell feeds its chart slot index, and the visible
  art does not depend on it.
- Frame: the first function arg selects the arm — `0` →
  `SetFrame(frame = 2)` (files `0x4dc7f2`, call at `0x4dc89f`,
  image = frame − 1 = **image 1**), `2` → `SetFrame(1)`
  (files `0x4dc7dc`, image 0). Which arm maps to which room mode
  stays OPEN; the frames differ only inside the Note_BG-covered
  lane-panel band, so the settled scene is arm-invariant.

### BO paints; EA05 does not (shipped layering)

The staged strip panel `Playing_Menu.ojs` (BACK3, slot 93,
(204,558) 596x42, fully opaque — **zero** colorkey `0x0000` pixels
in its 596x42 frame, both the whole-frame sweep and the EA05 window
(83..126, 7..40) sprite-relative) covers x 204..800 × y 558..600.
The `EA05 O2JAM` glyph (x 285..332, y 560..600) sits entirely inside
that rect, and nothing stages above BACK3 at the band that could
re-expose it (post-`Playing_Menu` bodies over y 530..600:
`Playing_Ohm.ojs` (394,542) 36x10 keyed readout, `WaveGage.ojs`
(225,514) 226x73 keyed jam-wave, `Playing_SkillWindow.ojs` ending
y 550) — so **EA05 never paints in the shipped 7.55 layering**; the
photo's visible EA05 belongs to the photo build's art
(venue vintage, same class as the strip hue delta in the parity row).
The `BO` glyph (x 4..40, y 570..596) is left of the strip panel's
x 204 origin: with the BACK1 backdrop staged it paints, under only
the jam bar (`Jam_gauge_bar.ojs` (4,536) 191x12 ends y 548) — which
the reference photo confirms (BO legible at the bottom-left corner,
dark strip and digit wells above it).

## Shell change (ticket #110)

`o2jam-shell` stages the runtime-built BACK1 backdrop as the first
Playing placement:

- `avatar::PLAYING_BACKDROP_CODE` (`0x1B500071`) names the synthetic
  placement's control code; `avatar::playing_backdrop_sprite` builds
  the family name (solo `Playing_NormalBG<d>.ojs`, team
  `Playing_P2PBG<d>.ojs`, `%d = chart_slot % 3` with 0 → 3);
  `avatar::is_playing_backdrop_sprite` extends the behind-scene
  membership so the placement paints through
  `paint_behind_placements` first of all (exe paint order).
- `avatar::playing_member_frame` cuts the backdrop to image 1 (the
  mode-0 arm; see the OPEN above).
- `staging::StageContext` gains `chart_slot` (the walk feeds the
  picked chart's slot; sites without a pick default to 0).

Guards: `playing_backdrop_name_cycles_by_chart_slot_mod_three`,
`playing_backdrop_sprite_membership_matches_runtime_built_names`,
`playing_backdrop_loads_its_mode_arm_frame_only` (synthetic), and the
data-gated shot guard `playing_shot_paints_the_backdrop_edge_label`
(BO ink box against a staged-minus-backdrop control).
`playing.png`/`playing-native.png` re-shot (dark strip, BO label,
panel backing); every other shot byte-identical.

## OPEN

- The `%d` arg's caller semantic (chart slot? song index?) — next
  step: REA trace of the `0x4dc750` function's callers (who loads
  `[ebp+0xc]`).
- The mode-arm ↔ room-mode mapping for the frame pick — next step:
  same caller trace (who loads `[ebp+0x8]`). Settled-scene invariant
  today (frames differ only under the `Note_BG` strip).
- Which state variant consumes which BACK1 writer (this synthetic
  re-staging vs the `_Playing_BG.ojs` stage applier at `0x4e7850` —
  both write the `+0x53c` slot shape; `docs/notebg-cover.md`'s OPEN)
  — the shell stages both, backdrop first; a live rig capture of the
  playing scene's left panel settles which art the exe actually
  shows.

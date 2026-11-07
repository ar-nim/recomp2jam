# The `Note_BG` right-panel cover (Playing scene) — 7.55

Question: our parity shot shows the `Note_BG.ojs` red deck strip at game
`(570,458)–(774,478)` where the reference photo shows the venue deck
(`docs/parity.md` Playing row, residual "exe cover untraced"). What does
the exe paint over that region — later entry art, `Playing_Menu.ojs`
overlap, or is the panel mode-family-only? Method: read-only byte walks
with Python 3.14 over `Playing1.opi` plus read-only objdump 2.44 static
analysis of the exe — the same offset-only precedent as
`docs/stage-bga.md`; **no REA dossier IDs issued**. No disassembly
reproduced. Version pins (local copies re-hashed):

- `AngelO2Jam.exe` 5,029,888 B, SHA-256 `83a04541…72c823b`.
- `Playing1.opi` 221,737,026 B, SHA-256 `8c1c7566…aed9eed40`.

## Answer

**The panel is mode-family-only — in fact authored-but-unselected.**
`IDC_IMAGE_SPLAY_BACK2` (code `0x1B500072`, sprite `Note_BG.ojs`) is a
two-frame body: image 0 anchors `(20,0)`, image 1 anchors `(567,0)`,
each 213x484 raw RGB555 with byte-identical pixels (body 412,416 B:
header count 2, colorkey `0x0000`, records at body `+8`/`+28`, blobs
206,184 B each). The exe image control paints **one frame per view**
(`docs/ui-draw.md` steps 3–6: the view draws the current frame's
surface; frame setter `0x42f870` swaps images — store `0x2f891`–
`0x2f897`, clamp `0x2f8c7`–`0x2f8e1`, image = frame − 1
`0x2f8ea`–`0x2f8f7`). A frame change therefore needs an exe writer for
that control, and a whole-image dword sweep finds **zero `0x1B500072`
immediates**: the only BACK-family literals in the exe stage other
entries (BACK1 `0x1B500071` at VA `0x4dc822`/`0x4dcb8c`/`0x4dd2ec`;
`0x1B5000D2` `KeyDownImage_3Key` at VA `0x4e7525`/`0x4e7577`). BACK2
keeps its loader-default image 0 — the left strip — on every Playing
mode family. Image 1 (the `(567,0)` right strip, the red panel of our
shot) never paints in 7.55 play; the venue (`BACK1`) shows through.

Excluded by data: no later entry covers the band — `BACK3`
`Playing_Menu.ojs` is 596x42 at anchor `(204,558)` (paints y 558..600,
above it nothing), and the post-`BACK2` file-order entries (gauge
shells, lane keys, `Winner_image`) anchor left of x 567 or below y 540.

## The exe's real mode-family selection (BACK1, not BACK2)

The family axis the exe actually drives is BACK1's **synthetic
re-staging** (refines `docs/stage-bga.md` OPEN 3). One Playing-setup
function stages code `0x1B500071` three ways (records built on the
stack, 0x11c B, through the factory `0x48B2D0`, sprite fetched
`0x50DD90`, frame set `0x42F870`):

| Mode family | Format string | Staging site | Frame arg |
|---|---|---|---|
| Normal (solo) | `Playing_NormalBG%d.ojs` (VA `0x64ac44`) | VA `0x4dc822`, SetFrame `0x4dc89f` | mode local `[ebp-0x130]` (2 at `0x4dc7f2`) |
| P2P (versus) | `Playing_P2PBG%d.ojs` (VA `0x64acec`) | VA `0x4dcb8c`, SetFrame `0x4dcc27`/`0x4dcc5b` (both slot copies +0x53c/+0x6dc) | same |
| Tutorial | `Playing_TutorialBG.ojs` (VA `0x64ad1c`) | VA `0x4dd2e6` | none observed |

Archive side (exact accounting verified per body): `Playing_NormalBG1..3.ojs`
and `Playing_P2PBG1..2.ojs` each declare **two full-screen 800x600
frames at anchor (0,0)** (960,000 B per blob, 1,920,048 B bodies);
`Playing_TutorialBG.ojs` is one full-screen frame. So the solo screen's
whole backdrop — venue plus decks — is one numbered, mode-family,
frame-selected body; the block-quoted `Note_BG.ojs` strips ride over it
with image 0 only. The `Playing_BG<stage>.ojs` set traced in
`docs/stage-bga.md` (37 files, 596x558 at `(204,0)`) resolves through
the twin `_`-name appliers (VA `0x4e7850` cluster); which state variant
consumes which BACK1 writer stays OPEN — both writers store into the
same `+0x53c`/`+0x6dc` slot shape.

Caveat: the negative sweep rules out every code-literal writer; an
index-driven frame writer outside the staging functions cannot be fully
excluded without a full decompile. The verdict rests on the sweep plus
the photo (`docs/playing-reference.md`: venue deck over the band, left
deck present — image 0 paints, image 1 does not).

## Shell fix (shared pipeline, family selection)

`crates/o2jam-shell/src/avatar.rs`:

- `playing_member_frame` — `Some(0)` for `Note_BG.ojs`, `None` else
  (the evidence table above).
- `AvatarFigure::layers_through` — the single-frame view cut.
- `load_scene_figures_for_room` applies the cut to the selected member
  after load, so every consumer of the room pipeline (shots included)
  loads the exe-selected frame only.

Tests: `note_bg_room_load_keeps_only_the_left_strip` (red before the
fix: figure carried both frames and the band painted red; green after:
left strip paints at `(20,0)`, `(567,0)` origin and the
`(570,458)–(774,478)` band stay clear) and
`note_bg_frame_cut_selects_image_zero`. `cargo test -p o2jam-shell`,
`cargo fmt --check`, `cargo clippy -p o2jam-shell --all-targets` green.

## Claims ledger

No `docs/claims.md` row changes (out of this task's file scope). When
folded in: extend A15/A16 context with the BACK1 synthetic re-staging
table (EXE, offset-only), note the `Note_BG.ojs` body walk as DATA, and
add the negative sweep as an EXE negative. `docs/stage-bga.md` OPEN 3
gains the Normal/P2P/Tutorial attribution sites; its
"`Playing_BG<stage>.ojs` renders once, static" answer stays the
`_`-applier arm's behavior, now scoped to one of two BACK1 writers.

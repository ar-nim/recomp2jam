# #863 — tutorial notes in flight and speech advance (7.55)

Oracle: `OTwo.exe` 7.55, 5,029,888 B, sha256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
Static pins: REA CLI 6.9.0 / Ghidra 12.2, file offset = VA − `0x400000`.
No disassembly or decompiled code is stored here; behavior is described
with addresses only.

## Evidence

| Id | Source | Pin |
|---|---|---|
| `S-863-NOTES` | REA static | `CStateTutorial` ctor `0x4f07f0` installs vtable `0x64b49c`; Init `0x4f0950` calls `0x56f6a0` (constructs `CNewNote` bodies via `CNewNote::vftable` `0x64dc34`) and binds it with `0x56ff30(this+0x3c)`. Normal Playing Init `0x4e47a0` uses the same `0x56ff30` setter. The scene-object paint at `0x48c570` walks the bound list and calls virtual `+0x24` on each. The tutorial therefore reuses the Playing note highway; it has no second renderer. |
| `S-863-PAGES` | REA static | Tutorial setup `0x4f0d30` stages `Guide_Ani/Text/Key.ojs` and derives 13 page deadlines from the float table at `0x67e2c0` (`1.5,13,24,…,94`) scaled by `192/(bpm*48/60)` plus `1.4` s. Tutorial Update `0x4f19e0` compares song time (`0x5711e0`, `+8/1000`) with these deadlines, then calls `0x42f870(page,0)` on the object returned by `0x50dd90(this+0x914)`; `0x42f870` writes `+0x164=page` and `0x41e0f0(page−1)` writes `+0x134=page−1`. A 1000 ms latch (`0x411ea0`) re-applies the current page. This is marker/page-driven record advance with a repeat cue, not a static record-1 loop. |
| `L-863-NOTES` | qualified live pack `~/scratch/fix863-late` | Manifest sha256 `16a3e0bd8a39b9fb4ab5e0e005bd1e25649d5a0db2c502556c1063f71e5d309d`; frame `artifacts/screenshots/tutorial-plus20.png` sha256 `66dea9d7b138cff8c47f683a14605a78f9d7cfdb24715cfe012bb6e61b5e3d29`, tutorial +49 s: one note in flight in the left highway and lesson bubble `2/13`. Six earlier entry frames (B7, boot1/boot2, `fix863-mid` at +30 s) show no notes because they precede the first onsets — they do not refute notes in flight. |
| `L-863-AUDIO` | same pack's sidecar | `audio/audio.wav`, started 00:03:58, FIRSTSTEP click 00:04:51, frame 00:05:42. Fingerprints against the 12 decoded mode-3 `bgsplit` samples show record 1 dominant through tutorial +31 s and a different sample family from +35 s. Page 2's pinned deadline is `13 * (240/100) + 1.4 ≈ 32.6 s`, and the paired frame reads `2/13`. The negative control pack `~/scratch/fix863-mid` (manifest sha256 `9052811968a0e9a99c1f1b27db77cfa220e5ffc1db499324bd2d4ddd6a4b74c0`) stays on record 1 before that deadline. |

## Contradiction recorded

Spec #313 row 21 claimed the B7 frame showed live notes. The B7 frame is
an entry frame with no note ink. The first note-bearing qualified frame is
`L-863-NOTES` above; the earlier frames pin entry, not absence of notes.

## OPEN

- Exact destination sample id for each page from audio alone. Mode-3
  records 2..12 share voice and spectrum. Next step: stopped-state
  winedbg reads of the OJM record field at successive page deadlines
  on one run (one stop per page, no `cont`), paired with a longer
  owned-sink capture.
- Full note cull/judgement cadence after page 2. Next step:
  chart-timed seven-key replay (`N-CHART`) against the same pack's
  note lanes.

## Rust implementation seam

- `ScreenGame::stage_tutorial_chart` feeds the shipped `Tutorial.ojn`
  Easy onsets into the staged-note view; the Tutorial painter draws
  only falling notes over the stage art, reusing Playing's note paint
  and its staged lane geometry (`Keyeffect` anchors). Playing score,
  life and chart state stay untouched.
- `ScreenGame::advance_tutorial_page` cues mode-3 record N when chart
  time crosses page N's pinned deadline (page 1 keeps the entry
  record). Guards: `tutorial_note_seam_inks_a_falling_note_without_install_data`,
  `tutorial_chart_notes_ink_the_highway_when_due`,
  `tutorial_ojm_advances_at_the_pinned_page_deadline`.

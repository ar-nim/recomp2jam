# The Playing bottom-strip panel variant ruling (#109) — 7.55

Question: the staged strip panel paints blue metallic while the
reference photo shows a dark strip (`docs/parity/playing.md`,
PLAYING.COMP residuals). Does a different shipped art variant bind
there — archive bytes + pairing — or is the photo's strip a different
build's color grade? Method: one data-walk pass (ADR-0009 cheapest
class) — read-only Python 3.14 byte walk over the pinned
`Playing1.opi`, same precedent as `docs/notebg-cover.md`; **no REA
dossier IDs issued**. No disassembly reproduced. Version pins (local
copies re-hashed):

- `Playing1.opi` 221,737,026 B, SHA-256 `8c1c7566…aed9eed40`.
- `AngelO2Jam.exe` 5,029,888 B, SHA-256 `83a04541…72c823b`.

## Answer — ADR deviation (vintage photo)

**The archive carries exactly one strip-panel body, and it is the
blue-metallic one.** Ruling evidence:

1. `Playing_Menu.ojs` (BACK3, slot 93, anchor (204,558), 596x42) is a
   **single-frame** sprite-55 body (header count 1, 50,064 B raw
   RGB555 blob) with mean RGB555 (7,13,18) — a dark navy metallic.
   No frame setter can select a dark frame: there is only one. Its
   baked cells read `EQUALIZER`, `COMBO`, `COOL`/`BAD`, `GOOD`/`MISS`,
   and `VOL` — the same tally layout the photo's strip shows,
   establishing the photo and the shipped body as one art piece at
   different color grades.
2. Full-rect sweep of every sprite-55 body overlapping the strip band
   y 530–600 finds no dark variant of the panel: the overlapping
   entries are the full-screen backdrop bodies (dark charcoal strip
   art, but they paint **first** of all and `Playing_Menu.ojs` covers
   their x 204–800 band), the couple-mode `CouplePlaying_Menu.ojs`
   (392x48 at (204,552), couple family), gauge bars, keydown art, and
   small keyed readouts (`Playing_Ohm.ojs`, `WaveGage.ojs`,
   `Playing_SkillWindow.ojs` — all colorkeyed, none a panel cover).
   Nothing opaque stages after BACK3 over the band that could replace
   the blue-metallic body at runtime (paint order per A5, slots).
3. `Playing_Menu.ojs` is fully opaque: **zero** colorkey `0x0000`
   pixels across its whole 596x42 frame, so nothing beneath it
   (including the backdrop's own dark strip art) shows through.

So the deviation stands in the parity row: the staged blue-metallic
strip panel is the shipped 7.55 art (data-side truth per ADR-0002),
and the photo's charcoal strip is the photo build's color grade —
the same vintage-delta class as the stage-5 studio standing for the
photo's unshipped rooftop. A live rig capture could not change the
ruling: the archive walk pins the only body the exe can stage, and
the baked cell labels tie the photo's strip to this same art.

(The dark strip with the BO corner label and the score/combo digit
wells at x 0–204 does paint — it belongs to the runtime-built BACK1
backdrop; see `docs/playing-backdrop.md`, ticket #110.)

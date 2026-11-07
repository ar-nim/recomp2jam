# Reference OPI-parser comparison (`o2jam-workshop-center`) — divergences

Corroboration read of the independent TS client at
`/tmp/o2jam-workshop-center` (`src/o2jam/{opi,sprite,controllist,bound,index}.ts`,
`src/bmp.ts`) against our `crates/o2jam-file/src/{opi,sprites,control,resolve}.rs`
and `docs/opi-pattern.md`. Reference code was read, never copied; nothing
outside this document was created or modified.

Exe stays oracle. Anchors cite `docs/opi-pattern.md` / `docs/stage-bga.md` /
`docs/ui-loader.md`; file offset = VA − `0x400000`. Status values:

- **EXE** — divergence decided by an already-pinned exe anchor.
- **EXE?** — needs an exe check before either side is adopted.
- **FILES** — decided by the file probe below (bytes of the pinned
  7.55 archives, read-only Python walk; same version pin as
  `docs/opi-pattern.md`).

Probe results used below (one-time reads of `install-root/O2Jam/Image/*`):

- P1 entry table: only two slot shapes exist — kind 1 with a name
  (9347) and kind 0 nameless (31); bytes 144–152 are zero on every
  entry; all named entries are pure ASCII.
- P2 sprites: sprite55 anchors go negative in 144 records (e.g.
  `Note_Cool.ojs` −73,−32 as i16); colorkeys only `0x0000` (547
  bodies) and `0x7D5F` (71) in `Playing1.opi`.
- P3 magic-by-extension (Playing + Interface): every `.oji` body
  (10) and every `.ojt` body (339) carries magic `0x0555`; the only
  `0x555A` bodies are the 3 `.oja` files, whose record words 8–20
  do **not** chain as offset/size (`Note_Click.oja` word 8 =
  `0xFFD0…`) — they are our raw `a`/`b`/`c` words.
- P4 `ControlList_*.txt`: all `{` braces sit on their own lines;
  21 Interface + 3 Playing `IDC_` lines end with a CP949 `//`
  comment and no quoted sprite; no case-variant duplicate names
  inside any archive.

Severity: **H** changes which art a scene shows or how pixels decode;
**M** field/rule disagreement with no observed 7.55-file impact;
**L** malformed-input posture or preserved-but-unused bytes.

## High

| # | Topic | Reference | Ours | Status |
|---|---|---|---|---|
| 1 | `.bnd` rect pairing | Sequential file-position pairing; leftover rects get synthetic ids | Code-ordinal `(code & 0xFFF) − 1`; miss → all-zero rect, never a skip | **EXE** (ours): VA `0x548640`, dossier `ev_3a869bb7…` |
| 2 | Sprite-name rules | None: every quoted token is a literal archive name | `O2_SBS_*` scrollbar test, `_` runtime stage expansion with clamp 1..=36, `Note_Click`/`Note_LongCombo`/`Note_LongComboNum` special stems | **EXE** (ours): `0x24c190`, `0x24b0ec`, applier `0x4e7850`, `strncpy` `0x4e80ef` |
| 3 | Colorkey semantics | Key `0` remapped to magenta (255,82,255); Euclidean-distance alpha ramp, thresholds 30/20 | Exact-key transparency on the expanded key (step 7) | **EXE?** — 547 Playing bodies with key `0x0000` decode differently; docs' file evidence does not pin exact-vs-tolerance |
| 4 | `.oji` codec routing | Any `.oji` decodes as masked run-list regardless of magic | Format-keyed: `0x0555` raster; outliers flagged, never run-list-decoded | **FILES**: all 10 real `.oji` bodies are `0x0555` raster — the reference misdecodes them; exe's own routing **EXE?** |
| 5 | `0x555A` interpretation | Sprite record with bitmap offset/size at bytes 8–20, run-list pixels | `.oja` envelope: frame geometry + raw `a`/`b`/`c` words (TBD) | **FILES**: all 3 real `.oja` bodies throw in the reference (offset `0xFFD0…` > body); ours parses. `a`/`b`/`c` meaning stays **EXE?** (docs OPEN 3) |

## Medium

| # | Topic | Reference | Ours | Status |
|---|---|---|---|---|
| 6 | Free-slot test | Skips slots with kind ≠ 1 | Skips nameless slots; kind preserved | Coincides on real files (**FILES**: only kind-1-named / kind-0-nameless exist); which field the exe tests **EXE?** (docs lean kind) |
| 7 | Sprite tag | Reads i16, rejects anything but 0/1 | u16, value kept, never validated | **EXE?** (docs: tag semantics Unknown) |
| 8 | Sprite55 anchor signedness | x/y i16 | x/y u16 (sign bit wraps) | **FILES**: 144 real records go negative; exe placement math **EXE?** |
| 9 | Blob accounting | Gaps allowed, `unused` word kept, needs only size ≥ w·h·2 | Chained offsets, zero trailing word, exact total accounting | Real files pass both (9.2k bodies verified); exe enforcement **EXE?** |
| 10 | 555→888 expansion | Scaled `c5·255/31` for raster, replicated for run-list (inconsistent) | Replicated (`c5<<3 \| c5>>2`) everywhere | **EXE?** — ±1 per channel |
| 11 | Row padding | Infers 4-byte-aligned scanline stride from size | No padded-row model (dense w·h·2 or strided atlas) | **EXE?** |
| 12 | Name decoding | Encoding detection (CP949) + whitespace trim | Lossy UTF-8, no trim | **FILES**: all names ASCII, untrimmed — no 7.55 impact; **EXE?** beyond it |
| 13 | Lookup precedence | Lowercase both sides, first match | Exact spelling first, then one ASCII-ci pass | **FILES**: no in-archive case duplicates — theoretical; exe precedence **EXE?** |
| 14 | Control-code bytes | Splits byte3/2/1/0 into state/type/group/node + heuristic labels | Full u32; only low 12 bits consumed | **EXE?** (RTTI correlation, file 2615242; dispatch TBD) |

## Low

| # | Topic | Reference | Ours | Status |
|---|---|---|---|---|
| 15 | Entry-line inline comments | Tokenizer keeps `//`; 21+3 real entries get bogus sprite `//` | Trailing words ignored; sprite `None` → no art | Ours matches the no-sprite reading; exe reading **EXE?** (**FILES**: the 24 lines carry no quote) |
| 16 | Malformed input | Clamps counts/sizes/slices, silently skips bad entries | Errors (`Truncated`/`UnknownFormat`) | Design posture; no exe claim |
| 17 | Same-line braces | Accepts `SET 0x1 {` and `{` on header lines | Errors | **FILES**: zero occurrences; `%s%x%s` keyword (file `0x243f34`) hints the exe reads a third token — **EXE?** |
| 18 | Entry-token shape | Any first token; parseInt falls back to 0 | Requires `IDC_` prefix + strict hex | All real entries `IDC_` — theoretical |
| 19 | Entry bytes 144–152 | Preserved as `unknown1/2` | Ignored | **FILES**: always zero |
| 20 | Header word 0 | Maps 1→opa, 2→opi, default opi; never validated | Documented (2/1), never read | Docs: no behavior tied to it |

## Capability notes (not parse divergences)

- Ours only: `is_jpeg_body` album-art sniff (47 ATI `.ojs` bodies),
  `OpiSlot` free-slot visibility, strided-atlas model
  (`frames_per_image`, `Note_Combo*.ojt` — file-verified by
  rendering; exe applier **EXE?**, docs OPEN 3).
- Reference only: archive/sprite/bnd/controllist writers, patch-name
  parsing and archive roles, npot flag, BMP encode/decode. Its BMP
  BGR byte order is BMP-file convention; both sides decode sprite
  words as RGB555 with red in the high bits — no channel-order
  divergence in sprite decoding.

## Needs-exe checklist (open items)

1. Colorkey: exact-key vs distance-threshold alpha; key-0 meaning (H3).
2. `.oji`/`.ojt` codec selection inside the exe — format or extension (H4).
3. `.oja` `a`/`b`/`c` words and frame-data codec (H5, docs OPEN 3).
4. Free-slot test field: kind vs name (M6).
5. Sprite tag acceptance (M7); anchor signedness in placement math (M8).
6. Accounting enforcement: chains, trailing word, exact length (M9).
7. Pixel expansion formula (M10) and row padding (M11).
8. Name decoding beyond ASCII and lookup precedence (M12, M13).
9. Control-code byte dispatch (M14) and entry-line comment handling (M15).
10. Same-line-brace grammar hinted by `%s%x%s` at file `0x243f34` (L17).

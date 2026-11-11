# Playing digit frames (7.55) — setter trace for codes `0x1B500088`–`0x1B500092`

Version pin: `AngelO2Jam.exe` 5,029,888 B, SHA-256 `83a04541…72c823b`
(twin `OTwo.exe`). VA = file + `0x400000`. Dossier:
`extracted/rea-playing-digits-dossier.json` (`rea-dossier-bundle-v1`,
REA 6.8.0 Ghidra 12.1.2 provider + objdump 2.47 slices + python3
scans). Method per ADR-0009 (static sweep; rig capture pair named as
the next step where the sweep left an OPEN). Own words only — no
disassembly is reproduced.

## 1. Which controls are the digits

`ControlList_Playing.txt` (Playing1.opi table entry 632) names the run:

| Code | IDC | Art |
|---|---|---|
| `0x1B500088`–`0x1B50008B` | `NOTECOMBONUM1`–`4` | `_Note_ComboNum.ojt` |
| `0x1B50008C` | `NOTE_LNCOMBO` | `_Note_LongCombo.ojt` |
| `0x1B50008D` | `NOTE_LNCOMBONUM_EFFECT` | `Note_LongComboNum_Effect.ojt` |
| `0x1B50008E`–`0x1B500091` | `NOTE_LNCOMBONUM1`–`4` | `_Note_LongComboNum.ojt` |
| `0x1B500092` | `NOTEJAMNUM` | `Note_JamNum.ojs` |

The `_` arts resolve stage-numbered: the exe strips the `_` and
rebuilds the stem (`%s%d.ojt`, `%s%d_%d.ojt`; stems `Note_LongComboNum`
VA `0x64b130`, `Note_LongCombo` VA `0x64b144`, formats VA
`0x64b154`/`0x64b128`; consumed at VA `0x4e8142`–`0x4e81fe`, twins
`0x4ee8f4`, `0x4f2c72`). Dossier `ev_4ce1dfea…`.

These entries carry no `.bnd` rects (negative bind walk), so the
controls stage at `(0,0)` and everything on screen is runtime-written.
The class is the plain image arm: factory `0x48B2D0` masks
`code & 0xFF0000`; the `0x500000` arm (VA `0x48b554`) constructs a
`0x60`-byte `CCImageBox` (ctor `0x50a820` installs
`CCImageBox::vftable`, dossier `ev_c1683efb…`).

## 2. Who writes the digits

Whole-image dword scan: **zero** occurrences of `0x1B500088`–`92` as
immediates (`ev_16334cd7…`). Sibling codes (`0x1B500071` file
`0xdc822`, `0x74`, `0x7B`, `0x81`/`0x82`/`0x85`, `0xD2` files
`0xe7525`–`0xe7f8e`) do appear — only the digit run is computed.

The writers live on `CStateNormalPlaying` / `CStateLivePlaying` (RTTI
file `0x27e1dc` / `0x27e2a4`) behind the command switch `0x498bc0`
(dossier `ev_c1f421aa…`): subtract base `0x1420100A`, bound `0x9C`,
byte table VA `0x4996b8`, jump table VA `0x4996ac`, plus explicit
compares `0x14208012`–`0x14208017` and `0x14600053`.

- **Digit writer `0x49f620`** (`ev_98c0210a…`): reads the per-player
  counter — player index byte `+0x60`, stride `0x30`, counter at
  `+0xDC + (+0x5C)·4`; when the counter exceeds 3 and field `+0x16C`
  is nonzero it adds `+0x16C`. Zero-fills a 4-dword bank at `+0xD0`,
  then per digit slot (cursor `+0xF0`, start `+0xCC`, `4 − +0x16C`
  places) stores the digit vector (`+0xC0..+0xC4`) element or 0,
  finds the control by the **computed code `0x1401D088 + slot`**, and
  lays the glyph string: record via `0x49c130` (string table
  `0x77b540`, record stride `0x50`, length byte `+0x42`), reset
  `0x528e20(control, 0, player index)`, per-character glyph mapping
  `0x52e0d0` (table via `0x52dba0`), final layout
  `0x528e20(control, string, player index)`. A null digit clears with
  `0x528e20(control, 0, 0)`.
- **Digit writer `0x49e8d0`** (`ev_53cc3fc9…`): same counter source
  and same code base, but an 8-dword bank at `+0x148`, digit vector
  `+0x68`, gate `+0x144`, and the sibling layout engine
  `0x529230(control, string/player index)`. The two writers are
  alternative builders for the same four controls, selected by mode
  flags `+0x144` / `+0x16C`.
- **Row attribute pass `0x4a1310`** (`ev_33338430…`): for the JAM
  word (code `0x14500087`) and the four digit controls
  (`0x1401D088`+i) writes both attribute dwords `+0x20`/`+0x24`
  (setters `0x409bd0` / `0x409bf0`, single-dword field writes) from
  its argument, and sets animation mode 2 on the jam view
  (`0x42f5a0`); for the 8-control number runs (`0x14200018/0x20/0x64`,
  `0x1440002c`, `0x14600035`) it writes `(argument == 0)`.
- Lookup is `GetByCode 0x48bcf0` (`ev_2398edc5…`): small codes are
  1-based ordinals into the dialog control array (`+0x44`), full
  codes walk the list comparing each control's code via vtable
  `+0xC`.

## 3. The anchor math (what #105 needs)

The digit controls own **13 glyph sub-objects** (array at control
`+0x50`, accessor `0x50dd50`, dossier `ev_26c62727…`) and a view
object (`+0x58`, accessor `0x50dd90`). The layout engines
(`0x528e20` `ev_03aa0c52…`, `0x529230` `ev_6d253f51…`) are the
anchor writers:

- **Layout/hide box**: `SetRect(0x59, 0x3C, 0xD5, 0xD2)` — x 89–213,
  y 60–210 — copied to sub-object 1 at `+0x1FC`; the hidden position
  is `(box.left − 0x5A, box.top − 0x3C)` = `(-1, 0)`. Per-glyph
  offsets come from the glyph table (`0x52dba0`) through
  `0x52e0d0`; flags `+0x74`/`+0x6C` mark content/hidden.
- **Frame selection**: `SetFrame 0x42f870(current, count)` writes
  `+0x164` (current), `+0x148` (count), `+0x168` (rest), clamps, and
  refreshes the staged surface via `0x42fe40` / `0x41e0f0`
  (`ev_863c5ea4…`). Position `0x42ff10(x, y)` writes `+0x18`/`+0x1C`
  (`ev_49d362bc…`).
- Because the controls are unbound, the on-screen digit row is the
  layout box plus the per-frame anchor of the chosen glyph, exactly
  the chain in `docs/ui-draw.md` §5 (destination = view position +
  per-frame anchor) — **no `.bnd` rect and no literal pixel constant
  is involved**; the only numeric anchors in the whole path are the
  layout box `(0x59,0x3C)–(0xD5,0xD2)` and the hide offsets
  `−0x5A`/`−0x3C` inside `0x528e20`/`0x529230`, plus the attribute
  words `+0x20`/`+0x24`.

## 4. Sibling system: bottom-strip big digits (separate)

`0x50d060` (`ev_4df59ba4…`) is the `CPlayingGame` HUD sprite bank
loader, not the digit control writer: it reads stems into fields
`+4`…`+0x24` — `P2P_MaxComboScroe.ojs`, `Buffer.ojs`,
`PlayTimeNum.ojs`, `equalizer_ani.ojs`, `Note_JamNum.ojs`,
`ComboScroe.ojs`, `Scroe.ojs` (VAs `0x64c0b4`–`0x64c11c`). Its
per-frame consumers/anchors stay OPEN.

## 5. OPEN (with the named next step)

1. **Runtime code mapping for the digit run** (`ev_8e44c1c1…`):
   sibling controls are looked up as `0x14500000 | ordinal`; the
   writers compute `0x1401D088 + slot`. Low byte = authoring ordinal
   `0x88`+i; the `0x01D0` mid bytes have no immediate and no traced
   creation arm. Next step: winedbg stopped-state read of the stored
   control codes at the `GetByCode` call sites (VA `0x498e0d` /
   `0x49f7a6`; one stop, queued `x/` reads, popup cleared first), or
   a Ghidra xref pass on the `CCImageBox` code-field writer.
2. **Writer input semantics** (`ev_6a51b2e9…`): which quantity
   (combo count / long-note count / score) feeds counter `+0xDC` per
   mode, and which bank (`+0xD0` vs `+0x148`) maps to which on-screen
   number. Next step: stopped-state reads of banks `+0xD0`/`+0x148`
   and counter `+0xDC` during a rig capture pair (ADR-0009), or
   tracing the producers of commands `0x14208012`–`0x14208017`.

Until OPEN 2 resolves, the shell ticket #105 can place the digit
string at the exe layout box `(0x59,0x3C)–(0xD5,0xD2)` with glyph
offsets from the art (as it already photo-anchors the long-note
digits at (110,304)–(148,315)), replacing the invented
`COMBO_ROW_ABOVE_LINE` anchor with the pinned box + engine offsets.

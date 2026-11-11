# CXO2 principles survey — cross-check against 7.55 oracle findings

## Provenance and version discipline

- Source repo: `https://github.com/SirusDoma/CXO2.git` (read-only clone),
  commit `74faa6664191f384c53b342ef0790524071a8f83` ("Update README.md").
- Survey date: 2026-11-09. Tools: plain file reading (`grep`/`sed` over the
  clone, line-accurate citations below). No CXO2 code was copied into this
  repository; every claim is paraphrased in our own words with a file path
  in their repo.
- CXO2 is a rewrite of an **older, localized O2Jam client** (C++/SFML-style,
  Genode framework). It is inspiration/corroboration only.
- **Version-delta warning.** Where CXO2 differs from our pinned 7.55
  evidence, the 7.55 executable is the behavior oracle (AGENTS.md; owner
  goal in memory `7c1cc`). Differences are documented as version deltas,
  never silently resolved. CXO2's own runtime behavior was not measured;
  we survey its source only.

Verdict legend: **CORROBORATES** (independent agreement with a 7.55 pin),
**INSPIRES** (a design idea worth adopting), **VERSION-DELTA** (older
client differs; exe wins for 7.55), **CONTRADICTS** (direct conflict; exe
wins).

---

## 1. State machine — how State.cpp composes, transitions, hosts dialogs

**CXO2 principle (their files).** Every screen is a `Cx::State` deriving
from a scene-graph `Gx::Scene` (`src/CXO2/States/State.cpp`). Screens are
not enumerated in one dispatcher: each state is its own class
(`src/CXO2/States/StatePlanet.cpp`, `StateRoom.cpp`, `StatePlaying7K.cpp`,
…), and transitions are `director.Present<StateX>()` calls on a scene
director (`src/CXO2/States/StatePlanet.cpp`, `OnChannelLogin`). There is
**no `StateChannel`** — the channel board is a *component* the planet
state instantiates (`Instantiate<ChannelBoard>(Resource::Planet::IDC_CHANNEL_BOARD)`
in `src/CXO2/States/StatePlanet.cpp::Initialize`). Dialogs are a shared
pool owned by the base `State`: Information / Question1 (OkCancel) /
Question2 (YesNo) are instantiated once, app-wide
(`State::LoadCommonResources`, `src/CXO2/States/State.cpp`), then
*presented* over any state with a presentation context (bounds, prompt,
backdrop flag) via `State::ShowDialog`. Notable behaviors:

- **Persistent dialogs**: a dialog request can be stored and re-shown in
  `Initialize` after the state is torn down and rebuilt
  (`m_persistentDialog`, `State::Initialize`).
- **Exit interception**: `OnAppClose` shows a confirm dialog and returns
  `false` until confirmed (`State::OnAppClose`).
- **Notice toast**: a shared `Notice` dialog, centered on the view, auto
  hidden after a 15 s timer in `Render` (`State::Announce`).
- Per-state resource scopes: Immediate (temp, cleared after init), Local,
  Shared (`State::GetResources`).

**Our 7.55 pin.** Our shell is `Screen`/`ScreenFlow`
(`crates/o2jam-shell/src/screen.rs`) with a dialog-overlay model; the
channel select is verified as a **mode of Planet**, not a screen —
`planet.md:106-108` (phase flag `parent+0xF64`), `planet.md:125-126`
(channel select tokens then `STATE_ROOM`), `channel-fill.md:323,389`.

**Verdict.**

- No `StateChannel`; channel board is a planet component: **CORROBORATES**
  our live finding (`planet.md`, `channel-fill.md`) from an independent
  reimplementation of the older client.
- Shared dialog pool + presentation context + backdrop flag: **INSPIRES**.
  Matches our dialog-overlay model; their persistent-dialog re-show is a
  concrete pattern for dialogs that must survive a screen refresh (our
  channel dialog reopens over the bare board at boot, `channel-fill.md:378`).
- Exit-confirm interception and notice toast with timer: **INSPIRES**
  (both are small, well-defined shell behaviors we must reproduce for
  parity; capture candidates for the Wine rig).

---

## 2. Scene composition — where positions come from

**CXO2 principle (their files).** Layout is data-driven, exactly in the
shape of our owner rule:

- The UI tree is JSON under `assets/ControlList/**`, compiled into the
  binary by `src/ResourceCompiler/ResourceCompiler.cpp`
  (`EmbeddedArchive` + `src/CXO2/Archives/EmbeddedArchive.cpp`). Example:
  `assets/ControlList/State/Planet.json` composes
  `IDC_CONTAINER_MUSIC_HALL` (six radio buttons), `IDC_CHANNEL_BOARD`,
  `IDC_BUTTON_EXIT` from per-control JSON files.
- Each control JSON carries `refID` and/or `bndID` hex codes in a
  `require` block, e.g.
  `assets/ControlList/State/Planet/ChannelBoard/Planet_Btn_ChannelLeft.json`
  = `{ "type": "Button", "require": { "refID": "0X19200008", "bndID": "0X19200008" } }`.
- Resolution (`include/CXO2/Decorators/IO/ResourceContextDecorator.inl`,
  `Require<T>`): `bndID` looks up an `sf::IntRect` bound in a resource
  store keyed by the hex code; `refID` looks up a `ControlList::Control`
  whose parameter names the source art file. The bound store is filled
  from the ControlList text format + `.bnd` files by
  `src/CXO2/IO/TextureCacheBuilder.cpp::BuildControlsCache`, which stores
  one `sf::IntRect` per control entry keyed `{:#08X}` of the entry id.
- **Global-to-local normalization**: a child's global bound has the
  parent's bound position subtracted and is cached as
  `"<id>-normalized"` (`ResourceContextDecorator.inl`, end of `Require`).
  Code only ever consumes *local* bounds — the comment there says the
  scene graph only works with local bounds.
- List templates name their per-item bounds **explicitly as an ordered
  code list**: `Planet_ChannelList.json` `require.template.bounds` lists
  `0X1901A01A..0X1901A02D` (20 codes) pairing the 20 channel-button
  slots (`src/CXO2/IO/Loaders/UI/ListLoader.cpp` consumes the template).
- Sprites bring their own sheet frames (`.ojs/.oji/.ojt/.oja` via
  `O2JamSpriteLoader`, `src/CXO2/IO/Loaders/Adaptor/O2JamSpriteLoader.cpp`);
  per-control loaders (`ButtonLoader.cpp`, `ChannelBoardLoader.cpp`, …)
  fall back: explicit position attribute > bnd rect origin > sprite frame
  position minus parent bound.
- `SceneComposer` (`src/CXO2/IO/Loaders/SceneGraph/SceneComposer.cpp`)
  walks the JSON `objects` map and dispatches each node to a registered
  per-type loader — the modernized equivalent of the exe's factory
  `0x48B2D0` + `CC*` classes (our `ui-loader.md`).

**Our 7.55 pin.** Owner rule (memory `f38f3`, `geometry-audit.md:7-9`):
the `.bnd` controls, code only parses; geometry = `parse_bnd` +
ControlList ordinal pairing `(code & 0xFFF) - 1` (`bnd-semantics.md:16,50`,
`opi-lessons.md:10`) + sprite anchors at staging; the painter consumes
staged rects, never literals.

**Verdict: CORROBORATES** — an independent clean reimplementation of the
*older* client converged on the identical architecture: bnd rects as the
single layout authority, code ordinal pairing (their loader binds the
`.bnd` rect queue to control entries in order and assigns sequential
ids from the block base), sprite sheets carrying frame art, and a
compose-then-paint split. **INSPIRES** two concrete adoptions:

1. **Explicit template bounds lists** (a list names the rect codes of its
   item slots in data) — the end-state answer for our "bnd controls"
   question: item-slot geometry can live in data, not loader code.
2. **Global→local bound normalization cached at load** — one subtraction
   at stage time, painter stays purely local.

---

## 3. Channel flow detail

**CXO2 principle (their files).**

- `StatePlanet::Initialize` wires six hall `RadioButton`s
  (`IDC_RADIO_BEGINNER_01/02/03`, `INTERMEDIATE_01`, `HIGH_01`,
  `PREMIUM_01`) in `IDC_CONTAINER_MUSIC_HALL`; checking one plays a click
  and calls `ChannelBoard::Show(hall)` then authenticates per hall
  (`OnMusicHallSelected`). Six halls exist: Euta, Thalo, Melpomin
  (beginner), Kleo (intermediate), Kalliope (high), Philix (premium)
  — `src/CXO2/States/StatePlanet.cpp`.
- `ChannelBoard` (`src/CXO2/UI/Components/Planet/ChannelBoard.cpp`) has
  two tabs: CHANNEL vs NOTICE (`Planet_Tab_Channel.json`,
  `Planet_Tab_Notice.json`). A **pager** (LEFT/RIGHT buttons +
  two 2-digit `BitmapNumber` page counters, `SetDigitCount(2)`)
  pages both the channel list and the notice frames. Page size is data:
  `Planet_ChannelBoard.json` attribute `channelCount: 20`, so **20
  channels per page**; `m_channelMaxPage = ceil(count / list children)`
  = ceil(count / 20).
- Interaction: click a channel row = select (checked state);
  **double-click synthesizes an ENTER-button click**
  (`OnChannelButtonDoubleClicked` → `enterButton->PerformClick()`).
  Enter plays the enter sound and fires the enter callback with
  `(hall, gatewayID, channelID)`; the state sends
  `ChannelLoginRequest{serverID, channelID}` and on success
  `Present<StateRoom>()`. Full channels show a "channel full" dialog
  (`OnChannelLogin`, `response.Full`).

**Our 7.55 pin.**

- The channel dialog is a Planet mode with a **scroll column, no pager**:
  `channel-fill.md:360-368` — pager resolved NEGATIVE on 7.55;
  `IDC_BUTTON_SCHANNEL_LEFT/RIGHT` (`0x12200005/06`) stage zero rects
  (dead vocabulary); list navigation is the scroll column with four
  viewport slots (`channel-fill.md:427-432`).
- Enter on a populated dialog emits server tokens `03EC/0FBE/07D0/07D2/13A4`
  and the screen becomes `STATE_ROOM` (`planet.md:125-126`,
  `channel-fill.md:396`).
- Protocol: `1002 GetChannelList`, `1004 ChannelLogin` C→S = i16
  gatewayId + i16 channelId (`net-protocol.md:100,123`).
- Family switch: the five emblems are server channel families;
  switching family reconnects **without re-fetching**; per-emblem behavior
  verified (`planet.md:71-114`, `channel-fill.md:383-394`).
- Chat rows on the dialog: "sender right-aligned to width 12, bracket,
  space, text" (`channel-fill.md:157`).

**Verdict.**

- Double-click = select-then-enter and enter sending **(gatewayId,
  channelId)** to reach the room screen: **CORROBORATES** our protocol pin
  `net-protocol.md:123` and our token-then-`STATE_ROOM` pin
  (`planet.md:125-126`). The older client routes it through an explicit
  ENTER button; 7.55 likely folds enter into double-click on the row
  (OPEN, needs rig confirmation — capture candidate).
- Pager with 2-digit page numbers vs our scroll column: **VERSION-DELTA**
  (documented evolution: 7.55's LEFT/RIGHT codes are dead vocabulary that
  a live older client once used; the pager survived only as orphan codes).
  This *strengthens* our "dead vocabulary" reading: the codes are real
  older-client UI that 7.55 retired.
- Six halls (3 beginner) vs our five family emblems: **VERSION-DELTA**
  (older localized client had six families including three beginner
  halls; 7.55 ships five — exe wins).
- Per-hall re-authentication vs our no-refetch family switch:
  **VERSION-DELTA** (server-flow difference, older client).
- Notice tab with per-frame notice art: **INSPIRES** — a candidate
  explanation for art/dialog vocabulary we have not yet placed in 7.55
  (OPEN; verify against captures before use).

---

## 4. Judgement / score / life / pill

**CXO2 principle (their files).**

- **Windows** (`src/CXO2/Core/Judgements/TimeJudgementStrategy.cpp`):
  position-based, converted to ms via BPM: latency = |position delta| ·
  4 · (60/BPM) · 1000; tolerances are **frame-based at 60 fps**: Cool
  3.2 frames ≈ 53 ms, Good 8 frames ≈ 133 ms, Bad 15 frames ≈ 250 ms.
  A second strategy judges by render position
  (`RenderPositionJudgementStrategy.cpp`).
- **Score** (`src/CXO2/Core/ScoreTracker.cpp`): Cool = `200 + 10·jams`,
  Good = `100 + 5·jams`, Bad = 4, Miss = −10, floored at 0. Combo
  increments on Cool/Good, resets on Bad/Miss. (Quirk: `GetCombo`
  returns `m_combo - 1` for positive values.)
- **Buffer/pill** (`ScoreTracker.cpp`): 15 consecutive Cools grant one
  buffer (their name for a pill), stock capped at 5, Good resets the
  progress; a Bad while buffer > 0 converts the hit to Cool and consumes
  one buffer.
- **Jam** (`ScoreTracker.cpp`): Cool adds 4, Good adds 2 jam progress;
  every full 100 progress grants one jam level (progress %= 100);
  Bad/Miss reset progress and jam combo. Jam level feeds the score
  multiplier above.
- **Life** (`src/CXO2/Core/LifeSystem.cpp`): per-difficulty deltas —
  EX: Cool +3 / Good +2 / Bad −10 / Miss −50; NX: +2 / +1 / −7 / −40;
  HX: +1 / (no Good row) / −5 / −30. Clamped [0, max]. (Defect worth
  noting: the ctor initializes `m_difficulty` to a constant, ignoring its
  argument.)

**Our 7.55 pin** (`gameplay.md`, `claims-gameplay.md`).

- Windows: **6 / 18 / 25 ms**, implemented as
  `o2jam-game::TimingWindows{6,18,25}` (`gameplay.md:9-23`).
- Score: Cool `200 + 10·X`, Good `100 + 5·X`, Bad-with-pill takes the
  Cool path, Bad +4, Miss −10 floored at 0, `X` = jam counter
  (`gameplay.md:74-78,121-122`).
- Pill: 15th consecutive Cool mints one, stock cap 5, Good/Bad reset the
  streak, Miss spends nothing (`gameplay.md:38-70`,
  `o2jam-game::CoolStreak` + `PillStock`).
- Jam: Cool +2, Good +1 progress; threshold table `(level+1)·50`
  cumulative per level; Bad/Miss reset (`gameplay.md:97-120`).
- Combo: +1 per Cool/Good, reset on Bad AND Miss (`gameplay.md:88-96`).
- Life: `LifeBar`/`LifeDeltas` in `o2jam-game` (`gameplay.md:235-242`);
  in-shipped-bytes Bad row `[-10,-7,-5]` seen once (`gameplay.md` scoring
  section).

**Verdict.**

- Score formula `200+10X / 100+5X / 4 / −10`, floor 0, jam as multiplier:
  **CORROBORATES** — the strongest cross-check in this survey; an
  independent older-client rewrite reproduces our exe-verified scorer
  term for term.
- 15-Cool pill with cap 5 and Bad→Cool conversion consuming one:
  **CORROBORATES** the pill rule.
- Windows 53/133/250 ms frame-based vs our 6/18/25 ms: **VERSION-DELTA**
  (older client judged in frames at 60 fps; 7.55's ms windows are exe-pinned,
  `gameplay.md:16`). Also their position→ms conversion via BPM is an
  **INSPIRES** note for how the older engine mapped chart position to
  time — useful context when reading 7.55's chart math, not a substitute.
- Jam progress 4/2 with flat 100 threshold and reset on level-up vs our
  2/1 with `(level+1)·50` cumulative: **VERSION-DELTA** (both share the
  same jam skeleton — progress accumulates on Cool/Good, multiplier grows
  per level, Bad/Miss reset — but every constant differs; exe wins).
- Life deltas: **VERSION-DELTA** in detail, **CORROBORATES** in shape —
  their per-difficulty Bad deltas (−10/−7/−5) exactly match the single
  in-bytes `[-10,-7,-5]` row we found (`gameplay.md` scoring section),
  supporting that row as the real per-difficulty Bad HP deltas. Miss
  −50/−40/−30 and Cool/Good gains remain unverified for 7.55 (OPEN).

---

## 5. Archives vs our `o2jam-file` parsers

**CXO2 principle (their files).**

- `OpiArchive.cpp`: OPI/OPA signature + count header, item headers
  walked from the **tail** (`fileSize − count·ITEM_HEADER_SIZE`), each
  item header carries name/size/reference; bodies read by offset. Same
  understanding as our `file-formats/opi.md` (they already corroborated
  our OPI spec, `opi.md:151`).
- `OjmArchive.cpp`: dispatches on the 3-byte signature — `M30` →
  `M30Archive`, `OMC`/`OJM` → `OmcArchive`. `M30Archive.cpp` keeps
  **index-keyed** entries (integer id → header) in addition to
  name-keyed. `OmcArchive.cpp` (344 lines, the largest archive module)
  extracts samples and writes WAV headers (`waveHeader.ChannelCount`,
  line 203) — i.e. it decodes OMC samples to PCM itself.
- `O2JamChartLoader.cpp`: OJN-style "new" signature check, a **backward
  XOR pass** from the file end with a key schedule (`output[offset] =
  data[srcIndex] ^ encryptKey[j]`, lines ~350-397) — an encryption/scramble
  step our parser docs do not cover.
- `EmbeddedArchive.cpp` + `ResourceCompiler.cpp`: assets compiled into
  the binary image.

**Our 7.55 pin.** `file-formats/opi.md`, `file-formats/ojn.md`,
`file-formats/ojm.md`; our OPI spec already cross-cited CXO2 (`opi.md:151`).

**Verdict.**

- OPI tail-header walk and versioned signature: **CORROBORATES**.
- OJM dual-container dispatch by 3-byte magic: **CORROBORATES** our
  format-family understanding (`file-formats/ojm.md`).
- Backward XOR unscramble in the chart loader: **INSPIRES** — a decode
  step worth checking against our `.ojn` notes (`file-formats/ojn.md`);
  if 7.55 ships scrambled charts anywhere, this is the candidate
  algorithm (OPEN: verify against install-root bytes before adopting).
- Index-keyed M30 access and in-house OMC→WAV decode: **INSPIRES** only;
  our `o2jam-audio` path already covers sample decode exe-pinned.

---

## 6. UI/Components — ChatPanel, SelectMusicDialog

**CXO2 principle (their files).**

- `ChatPanel.cpp` (409 lines): a shared chat component reused across
  Room, Waiting, and Playing states (it inspects its parent state to pick
  main-room vs waiting callbacks). Channel filter as a **radio list**
  (All/Friend/Guild/Whisper), a `ChatWindow` with a `ScrollBar` bound to
  up/down buttons, per-channel colors (all=olive, friend=teal, guild=gold,
  else white) with outline thickness 0.05. `ChatWindow.cpp` recycles a
  fixed pool of labels per visible slot and shifts them on scroll.
- `SelectMusicDialog.cpp` (1107 lines, the largest UI component):
  sortable music list (sort modes title/level/duration/new with toggle
  order), paged results (`GetPage/SetPage/GetMaxPage`), album browsing,
  color-coded row states (random/notice/disabled/warning — colors come
  from dialog JSON attributes, `SelectMusicDialogLoader.cpp`), search.

**Our 7.55 pin.** Chat rows: sender right-aligned to width 12 + bracket
+ text (`channel-fill.md:157`). Select-music dialog bound to
`Dialog_SelectMusic.bnd` (133 rects); many constants contradict the file
today, pairing unproven, INDEX-button inventory and x-pitch verified
(`geometry-audit.md:140-144,235-245`, `parity/waiting.md` musicselect cell,
`codec-coverage.md:53,96`).

**Verdict.**

- Chat as a **single reused component** with a channel radio filter,
  recycled label pool, and a scrollbar model: **INSPIRES** — a concrete
  decomposition to compare against the exe's chat list classes
  (`channel-fill.md:521` CCListBox et al.) when we build the shell chat.
- Select-music sorting/paging/color-state taxonomy: **INSPIRES** as a
  hypothesis list of behaviors to look for in 7.55 captures (sort toggles,
  row color states) — every placement still must pair against
  `Dialog_SelectMusic.bnd` per our ordinal rule before use.

---

## Actionable inspirations, ranked

Per owner rules (memory `f38f3`): the `.bnd` controls, code only parses;
the painter consumes staged placements.

1. **Adopt explicit template-bounds lists in data.** CXO2's list JSON
   names the rect codes of its item slots (`Planet_ChannelList.json`,
   20 codes for 20 slots). This is the end-state answer for our item-slot
   geometry: slot layouts become data interpreted by the staging step,
   removing the last geometry scaffolding literals. (`geometry-audit.md`
   tracks their removal.)
2. **Cache global→local bound normalization at stage time.** One
   subtraction when the tree is composed; the painter then consumes
   purely local rects. Simplifies every painter path.
3. **Shared dialog pool with presentation context (bounds, prompt,
   backdrop) + persistent-dialog re-show.** Matches our overlay model,
   gives dialogs that survive screen refresh for free, and defines the
   exit-confirm and notice-toast behaviors as capture-ready parity
   targets for the Wine rig.
4. **Score/pill/jam structure is now double-sourced.** Keep our exe-pinned
   constants, but cite CXO2 as independent corroboration of the
   `200+10X/100+5X/4/−10` scorer and the 15-Cool/cap-5 pill in
   `gameplay.md` — and use its differing constants (jam 4/2 & 100,
   life rows, frame windows) as a labeled older-version comparison table.

## Contradictions vs our 7.55 pins

None where the exe speaks. Every conflict above is a **VERSION-DELTA**
where CXO2 describes the older localized client: frame-based judgement
windows (exe: 6/18/25 ms, `gameplay.md:16`), a live pager (exe: dead
`0x12200005/06` zero rects + scroll column, `channel-fill.md:360-368`),
six halls (exe: five families, `planet.md:71-114`), jam constants
(exe: 2/1 & `(level+1)·50`, `gameplay.md:97-120`). The exe wins on all
of these; CXO2's value is corroboration of structure and evidence of
which 7.55 vocabulary is retired older-client UI.

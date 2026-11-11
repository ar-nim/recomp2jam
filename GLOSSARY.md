# Recomp2jam 7.55 parity

The reference implementation of O2Jam 7.55 behavior plus its modern cross-platform client. The 7.55 executable is the behavior oracle.

## Language

### Screens

**Scene**:
A user-visible screen such as channel, room, or playing.
_Avoid_: screen, page

**State**:
The mounted client object behind a scene; one state can host several scenes.
_Avoid_: screen object, controller

**Dialog**:
A popup sheet mounted over a state, such as the channel info sheet.
_Avoid_: popup, window

**Block**:
A named ControlList run with a declared entry count.
_Avoid_: list, section

### Controls

**Chrome**:
A staged lobby button with a control code and a `.bnd` rect, such as the room BACK, CREATE, or OPTION buttons.
_Avoid_: button (bare), widget

**FIRSTSTEP**:
The room chrome button at top-right left of OPTION, staged rect (660,1)-(720,33), code `0x13200032`. Exe entry behavior untraced (ticket #34).
_Avoid_: tutorial button (bare)

**Emblem**:
One of the five planet-board buttons. The emblems are server channel families, not missions; a lit emblem opens the channel dialog.
_Avoid_: icon, mission button

**Plate**:
A small band of sprite art carrying a readout or a reveal, such as the 84x18 light band above a lit emblem or the digit and count wells of a channel row.
_Avoid_: label, badge

**Select bar**:
The list highlight figure (`Dialog_ChannelSelectBar.ojs`, 348x44) painted at the hovered or highlighted channel row.
_Avoid_: cursor, highlight bar

**Enter face**:
The Korean-text button at bar-local (285,11) 44x21 that enters the highlighted channel. Only the face enters; row body clicks highlight only.
_Avoid_: enter button (bare), Ch button

### Frames

**Frame state**:
Which recorded frame of a multi-frame sprite paints: rest is body frame 0 desaturated, hover is frame 0 full color, active is frame 1 (docs/hover.md rig sweep).
_Avoid_: normal/over/pressed

**Rest-dim**:
The measured-but-unexplained desaturation of rest frame 0. Without it rest and hover paint identical pixels, so hover stays invisible.
_Avoid_: dimming, gray-out

### Practice

**Guard**:
A scenario-named regression test pinning one behavior plus its evidence.
_Avoid_: test (bare), check

**Rig**:
The Wine plus Xvfb plus SendInput evidence setup on a private display with null audio; captures stay under git-ignored scratch paths, never in the repo.
_Avoid_: harness, testbed

**Seam**:
The `ScreenGame` public surface the tests and walks drive with no display (explicit paint instants, menu keys, clicks).
_Avoid_: API (bare), interface

**Staging**:
Loading archive art into the game through the declaration (state and dialog staging in live boot and shots alike).
_Avoid_: loading (bare), mounting

**Stand-in**:
A labeled placeholder where exe evidence is missing, in code or docs. It never silently passes for parity.
_Avoid_: stub (for behavior), hack

**Viewport**:
The visible slice of a longer list that follows the highlight, such as the four channel rows.
_Avoid_: window, page

**Figure**:
Decoded sprite art mounted from the archives and painted at staged rects.
_Avoid_: image, sprite (bare)

### Verdicts

**PASS**:
A cell with 7.55 evidence plus implementation plus a passing test together.
_Avoid_: done, green

**FAIL**:
A known gap against verified 7.55 behavior.
_Avoid_: broken, red

**OPEN**:
Untraced or unimplemented behavior with a named next evidence step.
_Avoid_: todo, unknown

### Evidence

**EXE**:
Behavior anchored in the 7.55 client by offset plus evidence id.
_Avoid_: proven, verified

**DATA**:
Inference from shipped data files with no consumer observed.
_Avoid_: walk, guess

**ASSUMED**:
A placeholder from reports or corroboration awaiting exe proof.
_Avoid_: hypothesis, rumor

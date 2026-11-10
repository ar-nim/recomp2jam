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

# One State-keyed staging declaration

The same staging recipe — archive → entry → ControlList → Block → sprite
names → figures → `.bnd` → placements — existed in four copies, and the
choice of *which* Block belongs to *which* Scene differed in each, silently.
The copies had already drifted: one painted Myroom at anchor origins on a
belief retired in `docs/myroom.md`, one dropped the Result banner that the
exe mounts at file `0xc2390`, one staged 9 Channel Dialog bodies, one 8, one
none, and the soak staged Playing without the room mode or the `PlaySkin`.
So one Scene could show three compositions, and a capture could not settle
"what does 7.55 do".

We keep one **declaration** in `o2jam-shell`, keyed by **State** — the
`STATE_*` Block name the archive and the exe already use — of what each
State stages. It returns the staged art to its caller instead of writing
into the game, takes 1 context value for the few stage-side inputs (arena
stage, key mode, team flag; Playing is the only consumer today), and
follows the exe at every miss. Where the exe is silent, it contributes no
art and logs a warning. Coverage becomes complete by construction: the
States that only 2 of 4 copies staged now stage everywhere.

The key is State and not `Screen` because the archive's own vocabulary is
State: 2 Scenes (`Channel`, `Lanking`) have no State class of their own,
`Channel` and `Planet` are 2 phases of one State hosted by `CStatePlanet`,
and 3 States (`STATE_PAYMENT`, `STATE_PAYMENT2`, `STATE_TUTORIAL`) are not
Scenes at all. A Scene-keyed table cannot hold them, which is why the shots
walk staged them as ad-hoc extras.

## Considered Options

- Dedupe only the mechanism, leaving the four tables with their callers:
  deletes about 350 lines, keeps the four answers to "what is the Channel
  Scene" and leaves a fifth copy free to appear.
- A new `o2jam-stage` crate between `o2jam-file` and the shell: keeps the
  shell's old "OS surface only" claim true, costs a fourth crate for about
  400 lines, and still leaves the soak unable to reach the shipped path
  unless the crate is a normal dependency.
- Keep the recipe in the binary and expose a shim to the shell's test: this
  is what the soak does today, and it is why the soak proves a look-alike.
- Key the declaration by `Screen`: the shape the shell has, but it cannot
  express the 3 Scene-less States and makes `Channel` a special case.
- Store the art inside the game instead of returning it: removes the
  caller's storage step, but the module then owns all 7 `load_*` slots and
  cannot be used without a game object.
- Return the art: the module stays free of the game, and a later thin
  method on the game can store it if call sites get noisy.

## Consequences

- The shell's module doc changes: it owns scene staging as well as the OS
  surface. Its "game logic lives in `o2jam-game`" line still holds.
- Misses stop being silent. A missing body, a corrupt body, a missing Block
  or a missing `.bnd` each warn once with the State, Block and entry name.
  The warning is also the evidence hook for the arms the exe does not pin.
- Placements survive a total figure miss, so placement-driven hit tests keep
  working when no body decodes. Today a total miss drops the Block and its
  clicks go dead.
- Staged art becomes auditable in one table, which is what ADR-0003's
  "fallback stays procedural and labeled" needs: the label moves from
  `docs/geometry-audit.md` into the declaration.
- Result keeps 1 declaration row. Its solo/battle variants are paint-side
  (`ResultVerdict`), because the same bodies load in both modes.
- The Channel Dialog's select bar stays hand-staged and OPEN: it is mounted
  by the exe at the Dialog object (`+0x8C`), not driven by the table.
- Four OPEN arms need evidence rows: corrupt body, missing Block, missing
  `.bnd`, and the exe's install base directory. Each carries its next step.

# The install root is an argument

Game data is found through `env!("CARGO_MANIFEST_DIR")` in three places
today, and each joins `../../install-root`. That is a path on the build
machine, fixed at compile time. It is correct for development and tests, and
wrong for a shipped client: the product layout is the installed game, where
the client opens `Image\Interface1.opi` relative to its own directory
(`docs/install-layout.md`). A shipped binary built this way would look for
its archives under the build directory.

Libraries never resolve game data through `env!`. The install root is an
**argument**: the module that reads archives takes it as a value, and the
edge that boots the client decides it — the executable's own install
directory for the shipped game, the repository-local `install-root/` for
development and tests. The skip-when-absent rule stays at that edge, where
the path is known, so a missing archive keeps degrading to the procedural
fallback instead of failing.

One fact stays OPEN: whether 7.55 resolves `Image\...` against the
executable's directory or against the process working directory. It does
not change this decision, because the edge supplies the root in both arms.

## Considered Options

- Keep the compile-time path: works in `cargo test`, cannot ship.
- Resolve the root inside the library from `env!`: hides the reason for a
  test skip inside a library and still cannot ship.
- An environment variable: another hidden input, and it makes the shipped
  default depend on the launcher.
- An argument from the edge: one explicit input, testable both ways, and
  the shipped client can use its own directory.

## Consequences

- The three `install_root` helpers collapse into one loader that takes the
  root.
- CI, which has no game data, keeps its skip-clean tests, and the shipped
  client gains a path to real data.
- `docs/install-layout.md` gains the resolution rule once the OPEN check
  lands; until then it records the OPEN item.

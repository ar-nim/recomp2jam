# recomp2jam project skills

Pi discovers these skills from the repository. They adapt the static
recompilation validation workflow to the clean-room O2Jam 7.55 client and its
cross-platform Rust implementation.

| Skill | Purpose |
|---|---|
| `recomp2jam-scope-plan` | Scope behavior-parity work and acceptance criteria |
| `recomp2jam-reference-capture` | Capture Wine and Rust reference/test evidence safely |
| `recomp2jam-input-replay` | Replay timed seven-lane press/release and menu input |
| `recomp2jam-av-compare` | Align media, calculate metrics, and review scene differences |
| `recomp2jam-perf-profile` | Measure native Rust performance with controlled baselines |
| `recomp2jam-regression-triage` | Isolate parity and implementation regressions |
| `recomp2jam-batch-harness` | Run scene/platform matrices with honest statuses |

Invoke a skill explicitly with `/skill:<name>`, or let Pi select it from a
matching task. Each skill contains its own evaluation prompts under `evals/`.

## Project invariants

- The behavior oracle is the user-supplied O2Jam 7.55 Windows client; the
  implementation target is portable Rust.
- Keep original binaries, game archives, artwork, music, and proprietary
  captures out of Git. Use synthetic fixtures in committed tests.
- Describe reverse-engineered behavior in new words with version, hashes,
  offsets, and evidence IDs. Do not store original disassembly or decompilation.
- Keep `OPEN` and `FAIL` distinct from `PASS`. A/V scores and harness success
  do not replace behavior evidence.
- Real-data tests skip when `install-root/` is absent; live tests are opt-in.
  Automated game audio stays on a virtual or null output.

## Provenance

Adapted for Pi and recomp2jam from
[`Zx-Akito/SwitchRecomp/skills`](https://github.com/Zx-Akito/SwitchRecomp/tree/main/skills)
at source commit `9336eca19d099ec70908020122e2edb8872c8c44`.
Codex-only `agents/openai.yaml` files were not ported. Switch, emulator,
multi-title, and hardware-assumption content was replaced with recomp2jam's
Windows-reference/Rust-target workflow.

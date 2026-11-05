# recomp2jam

A clean-room **recompilation of the O2Jam 7.55 client in Rust**.

The original client was C++ (MSVC 2005) built on the **CDX game library**,
**Boost ~1.35/1.37**, and **libvorbis**. This project re-implements that
client's behavior in safe, portable Rust — no original code or binaries are
included here.

> ⚠️ Legal / clean-room note: you must supply your own legally obtained
> O2Jam 7.55 install. Never commit the original `.exe`, `.ojm`, `.ojn`, or
> other copyrighted assets to this repo (they are git-ignored by default).

## Original stack (what we're replacing)

| Original | Role in 7.55 client | Rust replacement (planned) |
|---|---|---|
| C++ + MSVC 2005 CRT | game code, ABI, RTTI/exceptions | Rust, `anyhow`/`thiserror` |
| CDX (DirectDraw/Direct3D-era wrapper) | 2D sprites, scene, input, timing loop | `winit` + `wgpu` (later milestone) |
| Boost 1.35/1.37 | smart ptrs, threads, asio, filesystem, signals | `std`, `tokio` (later), `serde` |
| libvorbis (+ ogg) | BGM / key sounds in `.ojm` | `lewton` (pure-Rust Vorbis decode) |
| Win32 + DirectX 8/9 era | window, audio, input | `cpal`/`rodio`, `winit`, `gilrs` (later) |
| InstallShield 11.5 installer | `O2Jam_Classic.exe` is the *installer*, not the game | extract on Windows, point tools at the installed files |

See [`docs/original-stack.md`](docs/original-stack.md) for RE implications
(MSVC 2005 mangling, Boost internals, CDX surface model) and
[`docs/architecture.md`](docs/architecture.md) for the crate map.

## Layout

```text
crates/
  o2jam-file    # .ojm / .ojn parsers (pure Rust, no OS deps)
  o2jam-audio   # Vorbis decode (lewton) + backend trait (null/cpal later)
  o2jam-game    # timing, judgement, score, chart model (pure logic)
  recomp2jam    # the game client binary (CLI today, windowed client later)
  o2jam-dump    # CLI inspector for .ojm/.ojn files
docs/
  original-stack.md
  architecture.md
  reversing.md
  file-formats/ojm.md
  file-formats/ojn.md
  progress.md
fixtures/       # small synthetic test files only — no copyrighted data
```

## Quick start

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p o2jam-dump -- --help
cargo run -p recomp2jam -- --help
```

## Roadmap

- [x] Day-0 scaffold: workspace builds, `cargo test` green
- [ ] RE inventory: module/function list from installed 7.55 binaries
- [ ] `o2jam-file`: verified OJM + OJN parsers against real files
- [ ] `o2jam-audio`: sample-accurate playback, latency measurement
- [ ] `o2jam-game`: judgement/score parity tests vs. original
- [ ] `recomp2jam`: windowed 7-key gameplay (winit + wgpu + cpal)
- [ ] Compatibility matrix in `docs/progress.md`

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). RE findings go in `docs/` as small,
reviewable PRs. Be precise about addresses/versions: everything is relative
to **7.55** unless stated otherwise.

# MAXWAVE evidence manifest — continuous two-client boot family

Local capture record only. Binary captures stay in git-ignored
`~/scratch/maxwave/`; this file pins their names, hashes, and verdicts.
Repo main `62f110f`; lane worktree `cb2/evidence-maxwave`. No code changed.

## One trace family, six boot windows

All boots shared Memoryer PID 92326 / boot epoch 1794986056, exe sha256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`,
audio `route=sink / on_default_sink=0 / preboot_breach=0` on every qualified
exe pack, and one trace id per boot (`S294-RESULT-MULTI-MAXWAVE-01` … `-06`).
Every pack has `exe-master/`, `rust-peer/`, `observer/`; the full scratch
findings live in `~/scratch/maxwave-findings.md`.

| Boot | Pack | Outcome |
|---|---|---|
| 1 | `w1-continuous` | CREATE, peer join, READY, START held; injected 4048 trigger never fired, no 4050; observer row state 1→2 pinned |
| 2 | `w1-continuous-02` | Peer join answer seen but Rust shell stayed in Room; no START |
| 3 | `w1-continuous-03` | 4048 burst injected at S2C 4013; no 4050 because peer never sent 4012 |
| 4 | `w1-continuous-04` | Master START click emitted no 4010; driver now retries three input shapes |
| 5 | `w1-continuous-05` | 4012+4048 injection produced 4050 on both taps; master Playing and 4016; peer stuck on Loading |
| 6 | `w1-continuous-06` | 8 s delayed injection put 4050 after peer Loading frame; peer still stuck; hard window bound reached |

## Pins from the last qualified boot

`w1-continuous-06` (trace `S294-RESULT-MULTI-MAXWAVE-06`):

```
trace.json                             61097dae0088ef64…
exe-master/manifest.json               44c07a2eb7f3735c…
rust-peer/manifest.json                ab9bb3fb24595ed2…
exe-master/wire/room-stage.log         d4d63a6eea218a9c…
rust-peer/wire/room-stage.log          470849be9f3d8b5f…
exe-master/fast-frames.tsv             9257bd176b0e8af6…
rust-peer/fast-frames.tsv              bf1e0ae5875b052a…
observer/poll.log                      260ba904ac6bfa2f…
exe-master/master-drive.log            595ad4e73c74bd28…
exe-master/artifacts/screenshots/waiting-master.png  5eb921ecb8033fed…
exe-master/artifacts/screenshots/playing.png         f6d99cf4e8ac97c8…
rust-peer/frames/frame-000106-loading.png             6edd25f4…
```

Verifiers: `run-manifest.sh verify` 0, `rig-audio.sh verify` 0,
`rig-provenance.sh cite --claim exe` 0. `frame-000106-loading.png` is
byte-identical to the peer Loading frame in boot 5.

## Ticket verdicts

- **#303, #632, #636, #589 — EVIDENCED.** Master Waiting boss-mark, master
  Playing highway, master-side 4010/4011/4012/4013, and observer row state
  1 (Waiting) → 2 (start/Loading) with the single-reader poll.
- **#584, #587 — EVIDENCED with caveat.** Pair from
  `~/scratch/evidence-multi/loading-04` (prior lane): exe master plus Rust
  peer, not the two-exe capture the tickets name.
- **#638 — PARTIAL.** Peer's `S2C 4011`/`4013` bodies named; the peer's own
  `C2S 4012` is absent (the harness injected it).
- **#634, #670, #672, #673, #676, #668, #682 — OPEN.** Rust peer never left
  Loading, so no peer highway, no `S2C 4018`, no two-member Result rows,
  master marker, or RETRY refusal frame.
- **#395, #425, #397, #523, #528, #399, #403, #405, #536, #548, #429, #467
  — NOT ATTEMPTED.** The six-boot bound was reached inside W1. Exact next
  captures are in `~/scratch/maxwave-findings.md`.

## Next evidence step (no boot spent)

`w1-continuous-06/rust-peer/wire/room-stage.log` shows every `4049` echo
carrying `MemberId=0`, because the harness injected that byte. Read the peer
seat byte in the `S2C 3004` body, inject that byte in 4012/4048, and require
a peer `C2S 4014` before accepting a start. If the peer still stalls, the
next boot must use two EXE clients rather than another Rust peer assist.

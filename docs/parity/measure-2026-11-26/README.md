# Parity-ledger measurements (2026-11-26)

Scope: documentation and measurement only. No painter change and no Wine
rig run in this lane. Exe frames are the pinned 7.55 frames in
`~/scratch/paired-shots-v3/states/`; `lanking`, `itemshop-entry` and
`itemshop-cart` were left to their owning lanes.

## Re-shoot and drift check

Commands (worktree `/home/arnim/scratch/impl-2026-11-25/worktrees/parity-ledger`):

```
cargo run -q -p recomp2jam --bin recomp2jam -- shots --out-dir <scratch>/shots-before
cargo run -q -p recomp2jam --bin recomp2jam -- shots --out-dir <scratch>/shots-after
bash scripts/verify-shots.sh <scratch>/shots-before <scratch>/shots-after
```

The guard printed `SAME` for all 29 PNGs and
`verify-shots: all captures byte-identical`. `room.png` passed its flat
fallback guard in both directories. Shot hashes used below:

| Shot | SHA-256 |
|---|---|
| `playing.png` | `70d60e2936d4b86bb092e5f78820e9c6de7fc9201be8e95711cb21799a802761` |
| `room.png` | `33caa75e7bc9de64a3f9bd99f4a9bd007bcc98c35b3086c5d3043f2380522a93` |
| `channel.png` | `18a695b24248a70191e1e0d7713387a180dfd09307eea89dd5a48f61ad49a392` |

`playing.png` is byte-identical to the checked-in `docs/screenshots/playing.png`.

## v3 paired pack, re-scored with `scripts/check-pair-ssim.py`

Command: `python3 scripts/check-pair-ssim.py <pairs dir> --json <out>`.
Each pairs dir held a copy of the qualified `exe.png` and `rust.png`
from the v3 state. Gate is structural SSIM 0.99; raw is reported beside
it. These are the pack's own native 800x600 legs, re-scored here; they
reproduce the baseline supplied with the lane brief.

| State | Raw | Structural | Gate |
|---|---:|---:|---|
| planet | 0.9952 | 0.9971 | PASS |
| tutorial | 0.9893 | 0.9944 | PASS |
| myroom | 0.9595 | 0.9836 | FAIL |
| room | 0.9502 | 0.9781 | FAIL |
| room-no-hover | 0.9500 | 0.9779 | FAIL |
| create-room | 0.9522 | 0.9754 | FAIL |
| waiting-guest | 0.9252 | 0.9448 | FAIL |
| waiting-master | 0.9244 | 0.9444 | FAIL |
| payment | 0.9158 | 0.9357 | FAIL |
| music-select | 0.9029 | 0.9274 | FAIL |
| channel | 0.8484 | 0.8989 | FAIL |
| options-key | 0.8477 | 0.8740 | FAIL |
| musicshop | 0.6872 | 0.7873 | FAIL |
| channel-popup | 0.7218 | 0.7109 | FAIL |
| lanking, itemshop-entry, itemshop-cart | — | — | not scored (other lanes) |

The v3 Rust legs predate later fixes; a low v3 number is not proof of a
current defect. Qualified post-fix pairs were re-scored with the same
script:

| Pair source | State | Raw | Structural | Gate |
|---|---|---:|---:|---|
| `~/scratch/impl-878-repair/states` | musicshop | 0.9757 | 0.9911 | PASS |
| `~/scratch/impl-878-repair/states` | myroom | 0.9725 | 0.9912 | PASS |
| `~/scratch/fix-chrome-a/final-pairs` | channel-popup | 0.9860 | 0.9956 | PASS |
| `~/scratch/fix-chrome-a/final-pairs` | channel | 0.9584 | 0.9913 | PASS |

## Fresh-shot vs v3 exe frame (cross-pipeline check only)

The shot walk writes 400x300 deterministic frames; the v3 Rust legs are
native 800x600 captures. To use the same scorer, each v3 `exe.png` was
downscaled to 400x300 (PIL LANCZOS) and paired with the corresponding
fresh shot. Scene phase differs between the two capture methods (for
example, the v3 `music-select` exe frame has no song dialog), so these
numbers are **not** owner-gate verdicts and are not used to raise any
row.

| State | Raw | Structural |
|---|---:|---:|
| planet | 0.9935 | 0.9979 |
| tutorial | 0.9898 | 0.9960 |
| musicshop | 0.9778 | 0.9940 |
| myroom | 0.9720 | 0.9902 |
| channel | 0.9500 | 0.9736 |
| room | 0.9428 | 0.9735 |
| room-no-hover | 0.9425 | 0.9736 |
| options-key | 0.9102 | 0.9340 |
| payment | 0.9017 | 0.9286 |
| waiting-master | 0.9486 | 0.9821 |
| waiting-guest | 0.6904 | 0.6889 |
| music-select | 0.3498 | 0.4558 |

## Playing region ink check (no exe frame in the v3 pack)

Method: both frames in 400x300 grayscale; the photo
`docs/screenshots/reference-4x3.png` (1920x1440) is downscaled by LANCZOS.
This is an ink-presence check (standard deviation and unique luma), not
a pixel match: the photo is a vintage venue and a different combo
scenario. Rectangles are inclusive-exclusive 400x300 coordinates.

| Region | Rect | Photo std | Rust std | Photo unique luma | Rust unique luma | Result |
|---|---|---:|---:|---:|---:|---|
| highway panel | (4,10)-(124,244) | 75.35 | 54.99 | 256 | 256 | ink on both sides |
| edge gauges (left) | (0,40)-(6,230) | 57.05 | 65.33 | 196 | 168 | ink on both sides |
| stage scene | (130,20)-(390,232) | 37.36 | 49.48 | 256 | 250 | ink on both sides |
| bottom strip | (0,250)-(400,299) | 59.80 | 72.30 | 256 | 254 | ink on both sides |

Digit placement is not read from photo pixels: #104 pinned the digit
setters, #105 painted the combo word/long-note digits at the pinned box,
and #107 proved the play-time digits have no writer in 7.55. The guards
naming those legs are in `docs/parity/playing.md`.

## Ledger state

`cargo run -q -p recomp2jam --bin ledger-check -- --report` after the
edits: `0 violation(s) found (0 hard, 0 audit:done)`, exit 0. The 60
dimension rows are 41 PASS-led, 11 OPEN-led, 8 N/A, 0 FAIL.

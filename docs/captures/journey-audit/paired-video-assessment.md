# #249 paired-video assessment

Question (senior, #250 journey-audit wave): after `native_ab_regions.py` and
`scripts/capture-log-diff.py`, does any current consumer need the #249 paired
video (one full planet-to-playing trace on both client pairs, silent audio
muxed, per-capture metadata, clean manifests)?

Method: static consumer audit of repo scripts, capture notes, and the 17
journey-audit matrices. No exe boot, no capture run, no video capture
implementation, and no tracker writes were made for this assessment.

## Recommendation: CLOSE #249 as superseded

#249 should close with the record "no current consumer; frame-still pairs plus
capture-log diffs cover the live decisions". The owner/senior must also
re-scope #245's literal per-scene video criterion to the native still-pair
method; this assessment does not by itself close that parent box. Reopen #249
only when a ticket names a temporal question that a still pair cannot answer
(reopen trigger below). Do not spend rig time on video before that.

## What actually consumes captures now

| Consumer | Input it reads | Does it need continuous video? |
|---|---|---|
| `scripts/native_ab_regions.py` | `frames/ref/<scene>.png` and `frames/test/<scene>.png`, native 800x600 | No. It crops stills and computes per-region SSIM/PSNR; see `docs/captures/t346/findings.md` and `~/scratch/p346/pack/region-metrics.json`. |
| `scripts/capture-log-diff.py` | `input.log` from both legs | No. It normalizes line values and diffs event/judged lines first, then the full log; see `docs/captures/t260/capture-log-ab.md`. It also exposes frame-line placement shifts, which a video would not turn into a contract. |
| `scripts/native_ab_pack.py` | the same stills | No. It creates `media/<role>/<scene>.mkv` by wrapping one still with FFV1 + silent PCM solely because `compare_av.py` requires an audio stream; see `.agents/skills/recomp2jam-av-compare/references/native-ab-capture.md` and the `transformations` note in `native_ab_pack.py`. No game-audio parity is claimed. |
| Journey-audit matrices (#287–#316) | frame hashes, region metrics, proof JSON, logs | No. Representative rows: `288-planet.md` P3 cites `emblem_strip` SSIM and two pack hashes; `289-channel.md` C9 cites `row_bars` plus matched wire/server slices; `290-room.md` R10 cites panel SSIM; `292-loading.md` story 1 cites cover-art crops. None cites a video timeline. |
| `.agents/skills/recomp2jam-av-compare/scripts/compare_av.py` and `batch_compare_av.py` | mp4/mkv with an audio stream | Capability exists, but its current native-pack input is the one-second still wrapper above. No open acceptance criterion outside #245/#249 asks it to measure a played recording. VMAF is optional; audio metrics are explicitly out of scope for #245. |

## Evidence that video is not the blocker

- #249's own comment (owner): "Nothing current blocks on video." Its
  `blocked-by` links (#247, #248) are upstream capability tickets, not
  consumers asking for the resulting file.
- #260's senior re-scope: the A/B note no longer waits on #249/#250; the
  scenario-level pairing already exists (`#339` carries `trace=` on the Rust
  side), and the judged/event contract is settled by the line diff. #260 is
  closed on that basis.
- `docs/captures/journey-audit/SUMMARY.md` classifies 108 blocked stories;
  the 59 capture-tooling rows ask for qualified interaction packs, hover/press
  pairs, or matched account state. None is gated on an mp4. The server/wire
  and Playing-wedge classes need wire pins or a live Playing entry, not video.
- #245 explicitly puts audio loudness/drift metrics and final thresholds in
  later tickets. Its "solution" mentions video pairs, but the shipped method
  (`native_ab_regions.py`, `capture-log-diff.py`, `#339` trace pairing) already
  produces the per-scene artifacts the matrices cite.

## What video would add, and the reopen trigger

A still pair cannot answer a question about motion over time: promo carousel
cadence (`288-planet.md` P2 still has no two timed qualified frames), long-note
tails and judged timing in Playing, audio drift/loudness, or VMAF on real
playback. Those remain OPEN for their own evidence reasons. The correct next
step there is a bounded, trace-stamped timed capture tied to the specific
ticket — not the blanket seven-scene #249 run.

Reopen #249 only if a ticket states all of:

1. the temporal question and the scene where a still pair cannot answer it;
2. one shared trace ID in both manifests, with mismatch refusal before metric
   run (the #245 contract);
3. the metadata fields (#249 body) with proprietary media kept local-only; and
4. containment proof for sound on an owned sink (no owner speakers).

Until then, keep the data and scripts at still-pair depth and close #249 with
a link to this assessment.

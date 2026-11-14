# t217 run manifest — OTwo pre-net AV bisect (local evidence, not committed)

All artifacts live under `~/scratch/t217/` (captures stay local per
ticket rules). Committed: `docs/captures/t217/t217-findings.md` + this file.

## Run matrix

| Run | Display | Prefix | Mode | Gate |
|---|---|---|---|---|
| B1-control | :81 | prefix-B1 (pristine clone) | --blob | PASS bright 2301 alive 3 |
| B2-direct | :82 | prefix-B2 (pristine clone) | --direct-blob | FAIL bright 3 alive 0 |
| B3-direct-dump | :83 | prefix-B3 (pristine clone) | --direct-blob-dump | FAIL bright 232 alive 2 (dialog) |
| B4-oldprefix | :84 | prefix-B4 (t207 prefix3 copy) | --direct-blob | FAIL bright 3 alive 0 |
| B5-proxyargv | :85 | prefix-B5 (pristine clone) | --direct-blob, MIRROR_PORT=15012 | FAIL bright 3 alive 0 |

## Artifact hashes (sha256)

- rig-B1/shot.png `136cfe70…49591b` (193,725 B, carousel)
- rig-B2/shot.png `855d028e…6890102c` (4,192 B black)
- rig-B3/shot.png `1f120693…558ec0f66` (5,643 B report dialog)
- rig-B3/shot-post-esc.png (black, dialog dismissed, R4 path)
- rig-B4/shot.png = rig-B5/shot.png = rig-B2/shot.png (byte-identical)
- rig-B*/client.log `e583828e…` / `dd67ae62…` / `5c5d0fce…` / `5b16ccfd…` / `fc739d3d…`
- rig-B3/B4/B5 gate.log `8f72d45e…` / `203e2fc2…` / `0b2640b9…`
  (B3 full; B4/B5 launch-only — see harness gap in findings)
- run/OTwo.exe `83a04541…72c823b` (ends with zero dumps)
- proxy-B5.log: `listening … :15012` only (zero accepts)

## Per-run dirs (each: claims, client.log, env.sh, pids, shot*.png, xvfb.log)

`rig-B1` `rig-B2` `rig-B3` `rig-B4` `rig-B5`, plus
`prefix-B1`…`prefix-B5`, `run/` (game copy), `proxy-B5*.log`,
`repro_t217.sh` (RED-first gate), `proxy_log.py` (t207 copy, LOG retargeted).

## Cleanup verified

Every run: `rig-run.sh stop` + `pkill -9 -x OTwo.exe` + `wineserver -k`,
`pgrep -x OTwo.exe` = 0, Xvfb down. No captures left the host.

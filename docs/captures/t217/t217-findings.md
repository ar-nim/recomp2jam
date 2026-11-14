# t217 findings — bisect of the OTwo pre-net AV (c0000005 at 0x5F2893)

Verdict: the AV did NOT reproduce in 5 single-variable runs. The t211
recipe still boots (B1 PASS, notice carousel). Every non-explorer replay
takes the t211-R4 path instead (black screen, fullscreen mode-set refusal,
silent exit, no dump) — never the t207 path (54 KB dump at `0x5F2893`,
zero mode-set lines). Leftover dumps produce the report dialog t207
described, but dismissing it continues into the R4 path, not the AV.
Zero fix attempts (stop rule intact).

## Pins

- `OTwo.exe` 5,029,888 B, sha256
  `83a04541…72c823b` (run copy and `install-root/O2Jam/OTwo.exe` identical).
- Wine 11.21 (wow64, single version on host — version bisect impossible),
  xorg-server 21.1.26-1, Xvfb, `wine_null` sink (PipeWire).
  Memoryer gateway up on `127.0.0.1:15010` for all runs.
- Blob: `~/scratch/t207/blob.txt` (654 chars, t207probe) + 5 mirrors;
  B1–B4 aimed at `:15010`, B5 at `:15012` via the t207 logging proxy.
- Prefix base: `~/o2jam-prefix-ko` cloned per run (B1/B2/B3/B5); B4 used a
  copy of t207's previously-booted `prefix3`.
- Locale: `ko_KR.UTF-8` via `LOCPATH=$HOME/.local/share/wine-locales`
  (`locale charmap` = UTF-8). `RIG_EXPECT_MEMORYER=0` everywhere.

## Run table (one variable per run, `rig-run.sh` preflight every run)

| Run | Delta vs t211 recipe | Result |
|---|---|---|
| B1 control, explorer+blob, pristine clone, `:81` | — | PASS: bright 2301, 3 alive, no dump, 0 ChangeDisplaySettings, 6 MinimizeAll; shot = notice carousel (eye-verified) |
| B2 direct+blob, pristine clone, `:82` | display backend | FAIL, R4 path: bright 3, silent exit, no dump, 1× `ChangeDisplaySettings … returned -2` |
| B3 direct+blob+seeded 54,308 B leftover dump, `:83` | leftover-dump state | FAIL-masked: report dialog (bright 232, eye-verified modal); Esc → seeded dump consumed, boot continues into R4 path (ChangeDisplaySettings -2), no new dump |
| B4 direct+blob, t207 `prefix3` copy, `:84` | prefix contents | FAIL, R4 path: bright 3, silent exit, no dump, 1× ChangeDisplaySettings -2 — previously-booted prefix ruled out |
| B5 direct+proxy-aimed argv (`:15012`, logging proxy up), pristine clone, `:85` | argv port | FAIL, R4 path: bright 3, silent exit, no dump, proxy saw ZERO accepts — argv port ruled out |

B2/B4/B5 shots are byte-identical (`855d028e…`, 4,192 B black screen).

## Red / green proof (`~/scratch/t217/repro_t217.sh`)

- Gate: FAIL if a run-written dump exists, or bright ≤ 500, or no
  `OTwo.exe` after 25 s; else PASS.
- GREEN: B1 (`bright=2301 alive=3`, no dump).
- RED: B3 dialog block (`bright=232 alive=2`, FAIL); B2/B4/B5 silent
  exits (bright 3, FAIL exit 1).
- Harness gap (same family as t211's stop-reap gap): the gate script
  dies silently after the launch block when no `OTwo.exe` survives —
  `ALIVE=$(ps aux | grep … | wc -l)` fails under `set -o pipefail`
  because the middle `grep -v grep` exits 1 on empty input. B2/B4/B5
  `gate.log` therefore ends at the launch lines; their FAIL verdicts
  are reconstructed from artifacts (shot bright, pgrep, client.log,
  dump absence), not from gate echo lines. Fix the `grep … || true`
  before reuse.

## Ruled out as AV triggers (this ticket)

Display backend alone (direct → R4 exit, never AV); leftover-dump
presence (→ report dialog, and Esc continues into R4, contradicting
t207's "Cancel resumes into the same AV" on this host); previously-
booted prefix contents (`prefix3` → R4, identical to pristine);
proxy-aimed argv port (`:15012` → R4 with zero proxy accepts,
confirming the crash-free path never connects pre-mode-set).

## OPEN (no fix attempted — 0 fix tries, stop rule clear)

Why t207's six Nov-14 04:xx runs AV'd at `0x5F2893` with 54 KB dumps
and zero `ChangeDisplaySettings` lines under the same xorg 21.1.26,
same exe hash, and same blob. t207's exact launcher line remains
unrecoverable, and Wine-version bisect is impossible (single Wine
11.21 on host). The AV predates both the display mode-set and the
first connect; every replayable variable routes around it.
Next evidence step: `winedbg` launch-under-debugger per
`docs/wine-boot.md` Attempt 11 notes on a fresh display, or replay
t189's 04:19 same-day boot if its launcher is recoverable
(`~/scratch/t189-keyrule/wine*.log` shows explorer noise — check
whether t189 used `explorer /desktop`).

## Provenance

Scratch only, never committed: `~/scratch/t217/` (run copy, 5 prefix
clones, rig-B1..B5 with `client.log`, `env.sh`, `claims`, `gate.log`,
`shot*.png`, `proxy-B5.log`, `repro_t217.sh`). Game bytes read-only;
`install-root` untouched (run dir ends with zero dumps). Null audio
only (`wine_null` throughout). `rig-run.sh selftest OK` before B1.
Per-run `stop` + `pkill -9 -x OTwo.exe` + `wineserver -k` + zero-
remainder verified (B3 needed the Esc dismissal first).

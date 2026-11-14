# t211 findings — OTwo.exe pre-net boot AV, working recipe

Verdict: the exe boots. Six blob+explorer runs reach the notice carousel
and the channel dialog with live gateway TCP (4 full, 2 boot-only).
The t207 stop-record AV (`c0000005` at `0x5F2893`) did not reproduce
under the recipe below. Direct launch without a virtual desktop fails
differently (silent exit after a fullscreen mode-set refusal, no dump).

## Pins

- `OTwo.exe` 5,029,888 B, sha256 `83a04541…72c823b` (run copy and
  `install-root/O2Jam/OTwo.exe` identical).
- Wine 11.21 (wow64), xorg-server 21.1.26-1, Xvfb, `wine_null` sink
  present (PipeWire). Memoryer gateway up on `127.0.0.1:15010`.
- Blob: `~/scratch/t207/blob.txt` (t207probe, 654 chars); mirrors
  `|test|??|127.0.0.1|15010` x5 (planet-verify shape).
- Prefix base: `~/o2jam-prefix-ko` (cloned per run, never reused
  across runs except R5); locale `ko_KR.UTF-8` with
  `LOCPATH=$HOME/.local/share/wine-locales`.

## Working recipe (copy-paste)

```bash
cp -a install-root/O2Jam/. ~/scratch/t211/run/   # read-only source; cwd here
rm -f ~/scratch/t211/run/OTwo*.dmp ~/scratch/t211/run/*.log
cp -a ~/o2jam-prefix-ko ~/scratch/t211/prefix-run
cd ~/scratch/t211/run
env -u WAYLAND_DISPLAY -u DISPLAY XDG_SESSION_TYPE= \
  LOCPATH=$HOME/.local/share/wine-locales \
  LANG=ko_KR.UTF-8 LC_ALL=ko_KR.UTF-8 \
  WINEDEBUG=err WINEPREFIX=~/scratch/t211/prefix-run \
  RIG_WINEPREFIX=~/scratch/t211/prefix-run \
  PULSE_SINK=rig_null_t211 RIG_SINK_NAME=rig_null_t211 \
  RIG_EXPECT_MEMORYER=0 \
  bash scripts/rig-run.sh run :96 ~/scratch/t211/rig-run -- \
    wine explorer /desktop=o2jam,800x600 ~/scratch/t211/run/OTwo.exe \
      "$(tr -d '\n\r ' < ~/scratch/t207/blob.txt)" \
      '|test|??|127.0.0.1|15010' '|test|??|127.0.0.1|15010' \
      '|test|??|127.0.0.1|15010' '|test|??|127.0.0.1|15010' \
      '|test|??|127.0.0.1|15010'
```

Drive (message-path XTEST, proven): H-tab click (145,105) → fallback
popup OK (400,362) → channel dialog. Canvas buttons need the
`SendInput` helper (`siclick` pattern), never XTEST presses.
Stop: `scripts/rig-run.sh stop <out-dir>`, then
`pkill -9 -x OTwo.exe` + `wineserver -k` + verify zero remain
(`stop` cannot reap the game: it tracks the `wine` launcher pid,
which exits while the game lives on).

## Run table (single variable per run, `rig-run.sh` preflight every run)

| Run | Delta vs recipe | Result |
|---|---|---|
| R1 bare+explorer, shared sink, :91 | no argv (gate path) | PASS, bright 4798, no dump |
| R2 blob+explorer, shared sink, :92 | — | carousel (frames diff 151.8, paging) → H-tab → popup → channel rows 0.38 → TCP ESTAB to 15010 |
| R3 blob+explorer, pristine prefix, :93 | repeat | same: popup up, rows 0.38, TCP ESTAB |
| R4/R4R blob+DIRECT (no explorer), :94/:97 | display backend | FAIL: black (bright 3), silent exit, no dump; `NtUserChangeDisplaySettings … returned -2` |
| R5 blob+explorer on reused prefix-ko-B, :95 | prefix reuse | PASS, bright 2301 |
| R6 blob+explorer, owned sink `rig_null_t211_6`, :96 | sink exclusivity | popup → rows 0.38 → TCP ESTAB; owned sink auto-unloaded on stop |

R2/R3/R6 logs contain zero `ChangeDisplaySettings` lines (windowed
path never attempts a fullscreen mode-set); R4/R4R logs contain it.
Explorer virtual desktop is load-bearing for blob boots.

## Red / green proof (`~/scratch/t211/repro_av.sh`)

- Gate: FAIL if a dump exists, or bright ≤ 500, or no `OTwo.exe`
  process after 25 s; else PASS.
- RED: `[t211] bright10=3` then exit 1 on R4R-direct
  (`FAIL: boot did not reach a live stage`).
- GREEN: PASS on R2/R3/R5/R6 (bright 2301–4798, rows 0.38 ×3,
  TCP ESTAB ×3, zero dumps across all runs).

## Ruled out by this ticket

Prefix poisoning by a prior successful boot (R5), owned-vs-shared
null sink (R6 vs R2), bare-argv survival (R1: gate path healthy, AV
is past the argv gate), cwd (game dir throughout; `option.ini`/dumps
never written on clean boots).

## OPEN (not root-caused, no fix attempted — 0 fix tries, stop rule clear)

Why t207's six Nov-14 04:xx runs AV'd at `0x5F2893` with 54 KB dumps
under the same xorg 21.1.26, same exe hash, and same t207 blob.
t207's exact launcher line was not recoverable from its rig dirs, so
the differing variable (likely launcher/display shape or a poisoned
prefix state) could not be replayed. Next step if wanted: one replay
run with t207's exact launcher, or `winedbg` launch-under-debugger
per `docs/wine-boot.md` Attempt 11 notes.

## Provenance

Scratch only, never committed: `~/scratch/t211/` (run copy, 7 prefix
clones, rig-R1..R6/R4R with `client.log`, `env.sh`, `claims`,
`shot*.png`, `repro_av.sh`). Game bytes read-only; `install-root`
untouched (no dumps/logs written there). `rig-run.sh selftest OK`
before the first run. Audio never left virtual sinks (`wine_null`
or owned `rig_null_*`; unloaded on stop).

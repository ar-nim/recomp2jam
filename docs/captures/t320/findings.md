# T320 — qualify one exe row-paint window and send one hover-press (2026-11-17)

Ticket #320, branch `cb2/tt-320`, worktree
`/home/arnim/Projects/recomp2jam-wt36-v320`. The three qualified boots and
their full wire logs stay in the git-ignored pack
`~/scratch/tt-320/{b1-baseline-wire,b2-wire-fixed,b3-no-dismiss}`;
`~/scratch/tt-320/findings.md` records each boot's exact command, drive
tail, wire excerpt, and sidecar fields. This committed note is the hash-
pinned pointer and the interpretation; no exe pixels or audio are in the
repo.

## Pins

| Item | Value |
|---|---|
| Client | `OTwo.exe` 5,029,888 B, sha256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b` (7.55) |
| Wine | 11.22 (wow64), prefix `~/scratch/240-fix/prefix` |
| Injector | `sendinput-helper.exe.so` sha256 prefix `f49e6583b0a7` (no `SetCursorPos` in the `press` verb) |
| Capture | ffmpeg n9.0.2, x11grab 800x600 |
| Wire tap | `wine-room-host.py --wire-tap`, pass-through `127.0.0.1:15012 -> 15010`, four-byte `[u16 len][u16 id]` framing |
| Sink | per-run `rig_null_<pid>` with `parec` monitor tap; no default-sink stream in any boot |

## What changed for this leg

The salvaged wire tap (`2569482`) died after 10 s of idle traffic: the
upstream socket kept its connect timeout and `relay_wire` treated
`socket.timeout` as end-of-stream. Boot 1 proves this from
`wire/room-stage.log:7` (`18:25:45 relay S2C end: timed out`) after only
`03E8/03E9/03EA/03EB` were relayed. The failing seam test
`IdleRelayTest.test_idle_timeout_does_not_end_the_wire_relay` was written
first and watched fail, then `relay_wire` was changed to treat a timeout as
an idle poll. The drive and its aims were not rewritten.

The run manifest now records the join verdict as
`join_0BBA=present|absent` and `join_press_count=N`, derived from the wire
log and drive log. Seam test:
`ManifestVerdictTest.test_manifest_records_absent_join_and_one_press`.

## Boots

All three used `--target waiting --join-mode hover-press --row-repaint
recreate --repaint-delay 0 --wire-tap`, a private Xvfb display, sanitized
env, `timeout 900`, and the full kit (mint, boot, drive, stop, package,
verify). Every pack passed `run-manifest.sh verify`, `rig-audio.sh verify`,
and `rig-provenance.sh cite --claim exe`; every pack hashed
`wire/room-stage.log` in provenance. Pack `status` is FAILED when the
drive did not reach `waiting`, which is the honest verdict for each boot.

| Boot | Display | Arms changed | Wire 2005 (S2C) | Row painted | Hover-press | 0BBA | Out-dir |
|---|---|---|---|---|---|---|---|
| B1 | `:230` | baseline, `--row-dismiss auto` | 0 (tap died before room stage; room stage not reached) | no | none | no | `b1-baseline-wire` |
| B2 | `:231` | tap idle-timeout fix | 1 at `18:29:09`, wire line 19 | no (probe at `18:29:46`, 5 s after the OK sent `FFF0`) | none | no | `b2-wire-fixed` |
| B3 | `:232` | only `--row-dismiss none` | 1 at `18:30:57`, wire line 21 | **yes** at `18:31:28` | exactly 1 X-warp, `room-row-hover.png`, 1 `press` | no | `b3-no-dismiss` |

### B2 — fresh 2005 reached the client before the row-paint window

`wire/room-stage.log` (sha256
`82136e5f99940de3343243cff092edcc4668ee760580967a1ccf80e564120840`):

```
L11 18:29:04 C2S opcode=1004 (03EC) len=4
L15 18:29:04 S2C opcode=2003 (07D3) len=2412 ... 57494e454a4f494e ... (WINEJOIN)
L19 18:29:09 S2C opcode=2005 (07D5) len=21 hex=0000000057494e454a4f494e000000000163000000 ROOM_CREATED room=00000000
L26 18:29:40 C2S opcode=2010 (07DA) len=0
L27 18:29:40 S2C opcode=2011 (07DB) len=54 ... 7432303770726f6265 (host t207probe)
L28 18:29:40 C2S opcode=2024 (07E8) len=30
L30 18:29:41 C2S opcode=65520 (FFF0) len=0
```

The drive logged `wire room stage entered S2C 2005 count=0` at 18:29:06
and `wire row window closed S2C 2005 count=1 first=18:29:09 ...` at
18:29:46. The room was already in `07D3` at 18:29:04, and the recreate
pulse produced `2005` at 18:29:09. The auto modal OK at 18:29:41 sent
`FFF0`; the probe five seconds later sampled a dead session and reported
`painted=False`. The green-banner probe itself is sound: it reads 0.3772
on the painted `explore-5`/`explore-6` room frames and 0.0 on these
unpainted ones.

### B3 — the row paints, the single stimulus fires, no join

`wire/room-stage.log` sha256
`6714367b49d0bce3d4d2a822b857e45ee601b13befe2addcfb3735c5c72b2cff`.
`S2C 2005` at 18:30:57 (line 21) preceded `07DA`/`07E8` at 18:31:28
(lines 25-27). Drive log:

```
18:31:28 paint wait popup_up=True row_dismiss=none
18:31:28 wire row painted S2C 2005 count=1 first=18:30:57 S2C opcode=2005 ...
18:31:28 room row painted=True
18:31:28 inject x-warp (100,100) rc=0 rig-xtest: moved to 100,100
18:31:29 room row hover delta=1.88 at (100,100)
18:31:30 inject send-press ... press down=1 up=1 err=0
18:31:42 join attempt 1 joined=False
18:31:42 stage waiting FAILED reason=no 0BBA after two joins
```

Exactly one `inject send-press` line in the drive log; no `SetCursorPos`
in the `press` verb. `room-row-hover.png` (sha256
`db8cd3dc7211e6deb83e18305eabeeb9fd61c4f0a7c9042199f245003fb0fae9`)
has green-banner fraction 0.3757 and message-box blue fraction 0.997: the
row was painted, and the no-response modal was up when the press landed.
The immediate rest-to-hover strip delta is 1.88 mean absolute pixels; this
is not a strong hover-highlight signature, so no claim is made about the
highlight's appearance beyond the captured delta value.

No opcode 3002 (`0BBA`) line exists in the wire log; the drive's 12 s join
timeout expired. The manifest records `join_0BBA=absent` and
`join_press_count=1`.

## Verdict and next step

Qualified: the exe reaches Memoryer through a pass-through tap, fresh
`2005` room data arrives before the paint window, the row paints with the
modal left up, and exactly one hover-press stimulus fires with a captured
hover frame and a negative `0BBA` in the manifest.

Not qualified: the join itself. Both arm variants fail: clicking the
modal OK sends `FFF0` (B2 line 30) and ends the session before the late
probe; leaving it up keeps the session and the row, but the modal eats the
press. This is the exe-side condition pin for the join wall, with wire
evidence IDs above. Next step, outside this leg's three-boot cap: time a
host recreate pulse just before the +36 s `07DA`/`07E8` poll so the list
stays modal-free (the `docs/wine-boot.md` "room-list fuse and its
suppression" recipe), then send one hover-press and read `0BBA` from the
wire log.

No Waiting or Loading pixel signature is claimed: no `0BBA` or `4012` cue
was observed under provenance in this leg.

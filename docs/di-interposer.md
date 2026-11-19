# DirectInput interposer (#699)

The exe reads gameplay keys from DirectInput, not window messages. SendInput
worked for UI clicks but never reached the device state that the Playing input
code polls. This tool fronts `DINPUT.DLL::DirectInputCreateA` with a fake
DirectInput object for the exe leg of chart replay.

## Feed path

```text
chart864-l1-groups.txt
  -> di-interposer-feed (Rust): <song_ms>:<DIK> groups + epoch -> feed.bin
  -> dinput.dll stand-in (C, PE32): feed.bin -> GetDeviceState / GetDeviceData
  -> exe input object: DIK bit 7 = press; buffered flag bit 7 = press
```

- `crates/di-interposer` parses the schedule, expands one press and one release
  per DIK (default hold 80 ms), and writes a 16-byte-header / 16-byte-record
  binary feed. Times are absolute Unix milliseconds; the epoch comes from the
  wire's first C2S `4014` life tick plus the measured chart lead-in.
- `tools/di-interposer/dinput-interposer.c` answers the vtable slots the exe
  uses: `+0x24` `GetDeviceState` (0x100 bytes), `+0x28` `GetDeviceData`
  (16-byte records), plus setup slots (`SetProperty`, `SetDataFormat`,
  `SetCooperativeLevel`, `SetEventNotification`, `Acquire`/`Unacquire`).
  A worker thread signals the notification event at each scripted transition.
- `scripts/di-interposer-run.sh` builds both halves and prints the launch
  environment. It never boots the exe. The boot leg exports
  `DI_INTERPOSER_SCHEDULE`, `DI_INTERPOSER_MODE` and `WINEDLLPATH` and starts
  the client normally. The script converts the feed path to a Windows path
  with `winepath -w` when available; the DLL reads no feed (all keys zero) if
  the path cannot be opened.

## Build and test

```sh
cargo test -p di-interposer
bash tools/di-interposer/build.sh /tmp/di-interposer-out
bash scripts/di-interposer-run.sh --epoch-ms "$EPOCH_MS" --out-dir /tmp/di-pack \
  --schedule ~/scratch/evidence-playing/chart864-l1-groups.txt
```

`DI_INTERPOSER_PINNED_SCHEDULE=/path/to/chart864-l1-groups.txt cargo test -p
di-interposer` adds a guard that the pinned file still has 403 groups.

`winegcc -m32` cannot link here: this host lacks `gnu/stubs-32.h` and
`/usr/lib/wine/i386-unix`. The build script therefore compiles the same C with
`gcc -m32` against Wine's PE headers and links PE32 with `ld -m i386pe` against
Wine's i386 import libraries. It verifies the PE architecture and the
`DirectInputCreateA` export with `winedump dump -j export`.

## Evidence and remaining work

Behavior pins are in `/home/arnim/scratch/di-interposer-findings.md` and the
static leg's `/home/arnim/scratch/evidence-static-findings.md` (#627). The exe
is the oracle: OTwo.exe 7.55, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.

OPEN for the next evidence window (no boot in this leg):

1. Does Wine load the app-local `dinput.dll` or the `WINEDLLPATH` copy before
   its builtin? The script supports both; a boot run must confirm.
2. Does the Playing object use the polled branch (`+0x7c` bit `0x8000` clear)
   or the buffered branch (bit set)? Default `both` feeds both paths; the
   mutually exclusive branches in `FUN_00544ca0` should prevent double-fire.
3. Per-lane judged counts and the `4016` tally with a DIK sweep must confirm
   the runtime key table before the full chart replay is called a parity leg.

# #339 — one Memoryer, one room, both clients

This scenario drives the Wine exe and the Rust client through the same
room on the same local Memoryer (`127.0.0.1:15010`). Evidence stays in the
caller's scratch directory. The host room and every state command belong
to the scenario, never to a single client leg.

## Start the shared host once

```bash
python3 scripts/matched-ab-scenario.py host-start \
  --dir "$HOME/scratch/matched-ab-339/host" --music-id 757 --hold 1800
# prints: host room created number=0 title=WINEJOIN
```

`--start-delay 99999` keeps the start gate closed until the script sends
`start`; the state plan never starts the song. Stop with `host-stop`.

## State matrix and matched commands

| State | Host command | Server evidence | Capture |
|---|---|---|---|
| base READY | none | `07E8` for both accounts, `0BBA` room join | `base-ready.png` |
| member downloading | `music-list=100,200`, `music-state=4`, `sync-member=0` | `07E8 … 2 music`, `4026`/`4037 (0004)` | `member-downloading.png` |
| gate closed (master) | `update-slot=2` | `3009 (0202)` | `gate-closed.png` |
| back to base | `update-slot=2`, `music-state=0`, `sync-member=0` | `3009 (0200)`, `4037 (0000)` | `back-to-base.png` |

`music-list` is the 2024 report from the host account, so the member whose
list omits song 757 is server-side state, not a client-side fixture.
`music-state` uses 4025 (Memoryer stores the byte; `FreeMusic=true` only
rewrites `NoAccess`), and `sync-member` asks 4036 for the 4037 broadcast.

## Legs (sequence them; never boot both at once)

```bash
# Wine, contained sink, wire tap on 15012:
env -u WAYLAND_DISPLAY -u XDG_SESSION_TYPE WINEPREFIX=... \
  python3 scripts/matched-ab-exe.py --out "$SCRATCH/exe" --display :230 \
    --host-dir "$SCRATCH/host" --plan "$SCRATCH/exe-plan2.json" \
    --wire-port 15012

# Rust, same host room, wire tap on 15013:
env -u WAYLAND_DISPLAY -u XDG_SESSION_TYPE \
  python3 scripts/matched-ab-rust.py --out "$SCRATCH/rust" --display :231 \
    --host-dir "$SCRATCH/host" --room 0 \
    --trace fixtures/input-trace/matched-ab-339.json --wire-port 15013
```

Both boots go through `rig-run.sh`: each owns a null sink, monitor tap and
route check. A pack is usable only with `route=sink`, `on_default_sink=0`,
`preboot_breach=0`; the exe pack must also pass
`rig-provenance.sh cite --claim exe`.

## Proof before pixels

```bash
python3 scripts/matched-ab-scenario.py verify --exe "$SCRATCH/exe" \
  --rust "$SCRATCH/rust" --server-start LINE --out "$SCRATCH/proof.json"
```

The proof compares direction/opcode traces (auth, pings and catalog frames
stay in the raw logs but not in the equality test), the 1003/2003/2026
body hashes, and per-leg slices of the shared server log. A pixel verdict
is valid only when the state commands and broadcasts appear in both
slices.

## Gate gesture pin

Guest-side SendInput double-clicks on waiting seats (140,140) and
(240,140) moved the hover cursor but sent no opcode. The seat-lock wire
effect is pinned from the master path: 3008 send sites in the waiting
handler at files `0xb83b4`/`0xb8703`/`0xb906c`, 3009 receive site file
`0x15cab1`, plus the live `3009 (0202)`/`(0200)` pair above. The exact
owner-described double-click gesture from the guest role stays OPEN.

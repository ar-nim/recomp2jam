# Solo Loading-gate wall trace (7.55) — #598 / #581

Version pin: `OTwo.exe` 5,029,888 B, sha256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
This note cites qualified Wine packs from the overnight evidence lane;
the raw captures live outside the repo (`~/scratch/evidence-loading/`),
never in `install-root/` or the git tree.

## Packs

| Pack | Status / provenance | Trace |
|---|---|---|
| `~/scratch/evidence-loading/solo-gate-01/exe/` | `status=PASS`, `cite --claim exe` rc=0, manifest sha256 `e5de8e7b6708a4d3646353a299e43cbe4b6e5075b076533b0303b745aa1c08f4` | 10 ms wire watcher; first full-bleed Loading frame `gate-seq-000.png` sha256 `7ea40df288745d71b3b70fa2ca79fb41a0917291a686730b884ec6929d3bf64b`, 720.2 ms after C2S 4010 |
| `~/scratch/evidence-loading/solo-gate-02/exe/` | `status=PASS`, `cite --claim exe` rc=0, citation manifest sha256 `b6df9f9c82ee935ee532f2529e4f40ea5e794099827cd4848cdd4b29d8ef0e28` | 10 fps frame watcher + 10 ms wire watcher on one clock |

Both packs ran with `route=sink`, `on_default_sink=0`, `preboot_breach=0`;
Memoryer PID 92326 / epoch 1794986056 unchanged. The same cover bytes
appear in both packs.

## Pinned spans (`solo-gate-02`, milliseconds from the C2S 4010 observation)

| Observation | Offset | Use in Rust |
|---|---|---|
| S2C 4011 zeros | +11.704 | gate arm edge |
| last Waiting frame | +197.999 | `LOADING_COVER_VISIBLE_EARLIEST` (198 ms) |
| first Loading frame | +291.281 | `LOADING_COVER_VISIBLE_LATEST` (291 ms) |
| C2S 4012 + S2C 4013 | +672.192 | `LOADING_CONFIRM_AFTER_START` (660 ms from the 4011 observation) |
| last Loading frame | +904.191 | lower handoff bracket (232 ms after confirm) |
| first Playing frame | +1,068.015 | upper handoff bracket (396 ms after confirm) |

The earlier pack measured a 606.051 ms gate span and a 720.2 ms cover
frame; the second pack tightens those to 660.488 ms and 291 ms. The cover
is up **before** the confirm: 4012 does not create the Loading frame.
The older P14 `master loading pixels did not appear after 4010` line is
refuted as a state claim; its palette detector missed this cover.

The solo-host pixel claim (#595) is the full-bleed frame
`7ea40df288745d71b3b70fa2ca79fb41a0917291a686730b884ec6929d3bf64b`:
the selected chart's OJN JPEG with its baked Artist/Remix/Pattern ink,
no lane HUD. Our host's own 4010/4011 path is guarded by
`solo_host_start_paints_the_staged_song_cover_full_bleed`, which skips
cleanly when `install-root` is hidden. The qualified pack is exe proof;
the guard pins the same caller chain without committing capture bytes.

## OPEN

The four internal legs (validation `0x5718b0`, staging `0x4f8b80`, OJN
open/decrypt `0x412180`, codec init `0x54db40`) have static dossiers but
no in-process wall timing. The Rust gate uses the wire/frame spans above;
the exact handoff instant inside 232-396 ms stays OPEN.

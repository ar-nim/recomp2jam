# Ticket 76 capture leg — judged-path latency

Status: BLOCKED for pair A and pair B; Rust instrumentation landed.

## Rust change

`crates/o2jam-shell/src/window.rs`: `deliver_snapshot` now logs
`judged {wall_ms} lane {lane}` for each lane-down edge entering the
snapshot. Guard test `capture_log_records_judged_strikes_on_delivery` green.

## Pair B attempt

Command:
`recomp2jam online ... --create-room t76 --music-id 757 --no-sound --input-log ...`
Result (client.log tail):
`Error: transport: read timed out waiting for server frame`
No playing highway reached; no judged lines captured.

## Pair A

Blocked; spec in `docs/captures/t76/exe-sampler-spec.md`.

## Gates

`cargo fmt --check`, `cargo clippy -p o2jam-shell --all-targets -- -D warnings`,
`cargo test -p o2jam-shell` all green (exit 0).

## Pair B retry (2026-11-15, unattended)

Client now boots to STATE_PLANET with valid token + gateway/channel ids
(`--gateway-id 0 --channel-id 0`). Navigation to STATE_PLAYING still
requires the XTEST sequence from docs/smoke/u1-windowed-online.md
(channel dialog, room, waiting, song select, start, loading). That
sequence was not completed in this leg, so no judged strikes were
injected and no pair B numbers were captured. Instrumentation and
guard remain green.


## Pair B unattended attempt

Command (sanitized Wayland, null sink, valid token):
`recomp2jam online ... --gateway-id 0 --channel-id 0 --create-room t76 --music-id 757 --no-sound --input-log ...`
Result: client parks on STATE_PLANET (36 avi frames then planet frames only; 66 planet frames, no channel/room/waiting/playing). Last lines:
frame 100 wall_ms=65628 screen=planet ...
frame 101 wall_ms=66629 screen=planet ...
frame 102 wall_ms=67629 screen=planet ...
No judged strikes injected; no pair B numbers.
frame 192 wall_ms=62522 screen=channel state=4e7bddfa9c217d06 content=e33ee2c377a22f3b
frame 193 wall_ms=62771 screen=channel state=4e7bddfa9c217d06 content=e33ee2c377a22f3b
frame 194 wall_ms=63021 screen=channel state=4e7bddfa9c217d06 content=e33ee2c377a22f3b
frame 195 wall_ms=63271 screen=channel state=4e7bddfa9c217d06 content=e33ee2c377a22f3b
frame 196 wall_ms=63522 screen=channel state=4e7bddfa9c217d06 content=e33ee2c377a22f3b
frame 197 wall_ms=63771 screen=channel state=4e7bddfa9c217d06 content=e33ee2c377a22f3b
frame 198 wall_ms=64023 screen=channel state=4e7bddfa9c217d06 content=e33ee2c377a22f3b
frame 199 wall_ms=64272 screen=channel state=4e7bddfa9c217d06 content=e33ee2c377a22f3b
frame 200 wall_ms=64522 screen=channel state=4e7bddfa9c217d06 content=e33ee2c377a22f3b
frame 201 wall_ms=64773 screen=channel state=4e7bddfa9c217d06 content=e33ee2c377a22f3b

## Pair B XTEST attempt

Client parks after channel login timeout:
`channel login timed out with no server answer; staying in Channel gateway_id=0 channel_id=0`
No Room/Waiting/Playing reached. Major finding: channel login does not
complete even with valid token/gateway/channel ids when the channel
dialog is opened manually.

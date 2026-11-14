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

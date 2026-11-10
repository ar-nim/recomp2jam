# Sample clock over wall clock

The 7.55 client paces song time from `timeGetTime`; the Rust client paces it from audio samples played. We keep the sample clock as an approved deviation because rhythm parity needs sample accuracy, and we hold it to wall-equivalence inside the 6/18/25 ms judgement windows.

## Considered Options

- Wall clock exact: faithful internals, inherits scheduling jitter and audio drift.
- Sample clock: stable with what the player hears, needs device-rate sync and a null-audio fallback.

## Consequences

- Judgement windows are unchanged; only the song-time source differs.
- Trace replays must prove equivalence inside the windows before the deviation is claimed as parity.

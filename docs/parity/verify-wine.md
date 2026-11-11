## Wine capture verify pass (2026-11-08 room/waiting/loading)

Shot pipeline regenerated via `cargo run -q -p recomp2jam -- shots`
(`cargo test -p o2jam-shell -p recomp2jam` green, `cargo fmt --check`
clean): all PNGs byte-identical — the fallback-only change below
touches no staged path. Compared `docs/screenshots/room.png`,
`waiting.png`, `loading.png` against `~/scratch/aa-room.png`,
`ac-room.png` (room), `ab-wait.png`, `ad-wait.png` (waiting),
`ab-load.png`, `z-load.png` (loading) plus siblings:

| Scene | Verdict vs Wine |
|---|---|
| Room | FAIL — real lists 2 columns x 3 rows of panels (number/title/HX1/3KEY/green bar/WAITING badge, detail/avatar/chat right, modal OK dialog over it); shell rows stay single-column quarter-frame ink over staged panels, details at rect corners, no create/password/modal UI. No shot drift (staged path unchanged) |
| Waiting | FAIL but IMPROVED — staged path matches the real 4-across x 2-down seats, TEAM A–H block, SKILL/SPEED column, BPM box, Random cards, BACK. Fixed this pass: the art-less fallback grid was an invented 2-col x 4-row (`seat_origin`); now 4x2 per the captures (new `waiting_seat_grid_is_four_columns_by_two_rows`, red on the old grid). Residuals: portraits center blind (audit 18–19), no team-button bodies/covers, ASCII chat, song dialog lives in `musicselect.png` |
| Loading | FAIL — real is a song-illustration full-bleed (bead progress pills, 3-KEY badge, Artist/Remix/Pattern credits); shell stages planet `MusicLoading.ojs` art with the centered bar + caption placeholders (audit 35), song caption only. No invented pass; bar/caption rects unmoved |

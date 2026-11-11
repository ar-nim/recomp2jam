## Waiting / music-select region verify (2026-11-07 caption re-shoot)

Both PNGs regenerated via `cargo run -q -p recomp2jam -- shots`
(`cargo test -p recomp2jam` 97 green, incl. the three chat guards
plus the new caption guard below); `musicselect.png` byte-identical
(no drift there), `waiting.png` refreshed with staged captions.
Bands below are column-
restricted luminance std (shot vs photo — structure, not pixels):

### `waiting.png` vs `reference-waiting-4x3.png` — REFRESHED, still FAIL

| Region | Shot | Photo | Verdict + residual |
|---|---|---|---|
| Top panel (title/Ch/Wins bar) | 51.2 | 56.7 | HOLD — staged art + title/song/wins ink on both; shot room is the `Jam` duo (no Ch/No/MX3 line, song caption now resolves the staged catalog title for 757 instead of bare `M0`) — scenario delta |
| Seat grid (covers, champion row) | 45.0 | 47.7 | HOLD — staged covers + seat text on both; shot names no CHAMPION/CHALLENGER roles (joined room carries no champion) and centers portrait text blind (audit 18–19); seats still on staged cover rects, team grid highlight only |
| Right column (TEAM/SKILL/SPEED) | 43.1 | 49.7 | REFRESHED — staged panel + button/speed ink on both; BPM now inks the catalog column of song 757 (was `-0-`) and the three SKILLCOUNT rects ink the staged inventory (`  7`/` 42`/`999`-capped); team highlight is the live amber cursor over the staged `Wait_Team?.ojs`, no covers; skills paint ids only, counts ride the staged inventory (no wire field) |
| Bottom chat/song (chat, arrow, Random, BACK) | 60.1 | 57.8 | REFRESHED — the history box stages two scripted 3012/3013 rows in the photo stacking (amber bracket-form rows at the 17 px pitch, admin notice on top, `Lujia`'s user row last) plus the input-strip caret (2026-11-08 chat-box verify below); Random cards and BACK stay staged art (no card/nav state rides the snapshot). MAPNAME now inks the map-cycle entry of room word 5 at its `.bnd` rect (was bare). Guarded by `scripted_chat_fixture_delivers_admin_then_user_rows`, `waiting_walk_without_staged_chat_holds_no_history`, `waiting_shot_paints_the_scripted_chat_rows`, `waiting_shot_paints_mapname_bpm_skillcount_captions` |

Composition stays FAIL (invented placement where noted above; no team-button bodies or covers staged).

### `musicselect.png` vs `reference-musicselect-4x3.png` — REFRESHED (ticket #15), structure HOLD

Shot bytes changed (274 px, all inside the title cells plus the chosen-song panel — the CP949 title decode, ticket #15); every band below still carries staged ink on both sides, so the structure verdicts hold with the behavior residuals corrected.

| Region | Shot | Photo | Verdict + residual |
|---|---|---|---|
| Filter columns (level/wheel/genre) | 24.6 | 61.5 | HOLD — staged art + column/button ink on both; lower shot std is the 562-song catalog vs the photo's 1608 plus ASCII stand-ins; pager/search/chosung display-only |
| Search table (label/note/title/artist/lvl/icons/grade) | 25.4 | 30.8 | HOLD — 10 staged rows + header on both; titles/artists now ink real CP949 glyphs (ticket #15 rows read Korean where the exe draws text); per-chart level still draws the unknown dash (never staged), grade blank, 다운/듣기/정보 icon cells blank (no staged art); the picked row's letter inks the active orange (stand-in highlight, exe rendering OPEN) |
| Status + search row | 26.2 | 39.0 | HOLD — `ALL -> ALL SEARCH RESULT.` + pager `1/<pages>` + title/artist radios + edit + search button on both; photo page `3/161`, shot page 1 (no paging input staged) |
| Chosung + OK | 37.4 | 46.3 | HOLD — `A`–`Z` letters + OK pill on both; hangul jamo now ink through the embedded face (ticket #15 — the old micro-font gap is closed), 숫자/기타 word cells stay blank (no caption source); dialog bind pairing proven (154-entry proof incl. `DAILOG_SELECTMUSIC`, ticket #15) |

Dialog behavior rides the new `STATE_WAITING` music-select row (PASS with named OPENs); the photo-structural residuals above stay (level/grade dashes, icon blanks, display-only pager/search/chosung/OK).

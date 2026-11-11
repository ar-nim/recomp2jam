## Reference-photo verify pass (2026-11-07 re-shoot)

All seven PNGs regenerated via `cargo run -q -p recomp2jam -- shots`
(`cargo test -p recomp2jam` 75/75 green, incl. the new
`result_shot_paints_the_rank_table_rows` guard and the three
musicselect guards). Per-region read is
ink-presence per band (shot std vs photo std — content differs by
scenario, so structure, not pixels, is compared):

| Shot vs photo | Region verdicts |
|---|---|
| `playing.png` vs `reference-4x3.png` | HOLD, byte-identical to the e9fd95f bytes (no pipeline drift). Highway panel (59.5 vs 78.7), edge gauges (40.8 vs 64.9), avatar scene (67.6 vs 41.5), bottom strip (75.3 vs 54.0) — every band carries staged ink on both sides. Residuals unchanged: stage-5 studio stands for the unshipped rooftop, `Note_BG` red panel at (570,458)–(774,478) with no exe cover, blue-metallic strip vs the photo's dark strip, BO/EA05 labels unpainted |
| `waiting.png` vs `reference-waiting-4x3.png` | REFRESHED 2026-11-07 (scripted 3012/3013 chat fixture — see below). Top panel (59.6 vs 65.6), seat grid (55.5 vs 53.0), right column (59.4 vs 61.4) unchanged; bottom chat band carries two live rows plus the input caret (bracket-form amber rows at the photo-pinned 17 px pitch — see the 2026-11-08 chat-box verify below). Caption refresh 2026-11-07 (see below): only `waiting.png` changed again — 682 px confined to the six caption rects (MUSICNAME/MAPNAME/BPM/3×SKILLCOUNT), every other shot byte-identical, no pipeline drift. Residuals: invented 2-col x 4-row seat grid, no team buttons/covers (the song dialog lives in `musicselect.png` below); chat texts are ASCII stand-ins, no paging/scroll |
| `result.png` vs `reference-result-4x3.png` | REFRESHED (was stale). Score dome (48.6 vs 50.7), verdict banner (46.5 vs 48.7), rank table (85.3 vs 48.5), footer (59.1 vs 51.2). The checked-in PNG predated the rank-row wiring (`04e2462`) — its row band was empty; the re-shoot carries row 0 (`YOU`, `1/0/0/0`, `200`) at the pinned columns, guarded by `result_shot_paints_the_rank_table_rows` (proven to fail on wrong text). Scenario deltas vs the photo stay: GOOD! solo arm vs LOSE, one local row vs two wire rows, P plate vs C. REFRESHED 2026-11-11 (ticket #17): the worn avatar leaves frame center for the local row-0 slot (78,316) under the 652×22 band clip — `result*.png` carry the slot ink (row-0 band x 90..126, y 316..338 native) with the old center ink gone; photo's battle-row portraits ride the member table (OPEN). |
| `musicselect.png` vs `reference-musicselect-4x3.png` | NEW (was UNCOVERED — no shot mapped to it). The walk now repaints Waiting through the host song flow (installed 7.55 562-song catalog, pick id 757 Hard X3.5; 4-song synthetic fallback without game data) under genuine `DAILOG_SELECTMUSIC` art, and the staged structure paints over it: filter columns (75.0 vs 183.4), search table (61.3 vs 83.2), status + search row (76.4 vs 114.7), chosung + OK (84.3 vs 128.1) — staged ink in every band on both sides. Still FAIL: per-chart level and grade cells draw the unknown dash (never staged), the 다운/듣기/정보 icon cells stay blank (no staged art), and pager/search/chosung are display-only. REFRESHED 2026-11-11 (ticket #15): titles/artists now ink real CP949 glyphs (the parser read UTF-8-lossy tofu — fixed), the picked row highlights orange (stand-in), the bind pairing is proven (154-entry proof incl. the dialog), and a master row click commits the 4000 pick — see the new `STATE_WAITING` music-select row (PASS with named OPENs). Chosung jamo ink through the embedded face (the old micro-font gap is closed); word cells stay blank (no caption source). Guarded by `musicselect_shot_repaints_waiting_with_song_panel`, `musicselect_fallback_catalog_lists_four_synthetic_songs`, `musicselect_dialog_structure_paints_search_tab_over_waiting` plus the ticket-15 click/highlight guards |
| `channel.png` | REFRESHED 2026-11-10 (file-native geometry + hover bar): rows/X/face/scroll all bnd-pinned (pairing exe-verified), hover highlight is the code-mounted `Dialog_ChannelSelectBar` at the hovered row, fallback wells live-measured, dialog over the board with the MENU bar beneath. Pager removed — resolved negative (no pager in the 7.55 dialog; scroll column only). Still FAIL: plate stand-ins and row content text (ASCII, no CP949 glyphs) |
| `room.png` | HOLD, byte-identical (no drift). No reference photo covers it |
| `planet.png` | REFRESHED 2026-11-10 (re-scoped walk): now the BARE board — the walk shoots it after the dialog X (reference-measured zone, native (557,195) modulo chrome) closes the dialog; the staged 17-placement bound block (backdrop, five buttons, promotion image, MENU bar) paints with no dialog over it. A following lit-emblem (HARD) click reopens the channel dialog and records the pick (LIVE-VERIFIED 2026-11-09: the emblems are server
channel families, EASY active at boot ~21 s after login; guarded by `lit_emblem_click_reopens_the_channel_dialog`); the amber-cursor stand-in is covered by returning to the board (`planet_shot_stages_bound_art_with_the_picked_band`). REFRESHED 2026-11-11: captions now paint (exe-pinned strings in real glyphs, guarded), the picked band keeps the amber-cursor stand-in (exe orange select border capture-pinned, shell OPEN with a named next step — `docs/planet.md` "Difficulty-pick highlight"), FREE/DRUM stay unbound (P0), promo cells answer clicks at parsed rects (press records, carousel semantics OPEN), EXIT/OPTION dispatch (quit-confirm / options dialog); no reference photo covers it |
| `loading.png` | REFRESHED 2026-11-08: the walk holds the clock-armed full fraction with the staged catalog caption (song 757) — 252 px of new caption ink at the placeholder row, bar unchanged full; guarded by the two loading-stage tests. No reference photo covers it; staged `MusicLoading.ojs` art + green bar under the no-fill rule (empty stage would paint nothing, staged art is unaffected) |

Chat refresh 2026-11-07: only `waiting.png` changed (1104 bytes
of two chat rows at the list origin); every other shot byte-identical
— no pipeline drift. Earlier deltas stand: `musicselect.png` is new;
the rank-row refresh added 129 pixels of row-0 ink.

Chat-box verify 2026-11-08 (chat band vs
`reference-waiting-4x3.png`, photo pixels pinned per item; failing
tests first, then the shared painter): re-shot `waiting.png` (chat
band rows 225–250 + caret block) and `musicselect.png` (the same
input strip's caret, 10 px). `playing.png`/`playing-native.png`
regen diffs reproduce at HEAD with this change stashed — pre-
existing drift (committed bytes predate a HEAD jam-effect change),
not from this work. Per-item:

| Item | Verdict |
|---|---|
| Composition (sender/text rows at the list origin) | FIXED — user rows bracket the nickname right-aligned in a 13-wide field (`[{sender:>13}] text`), the photo's `[        Lujia] YOOO!!!` shape (was `sender: text`); rows paint bare on the box art, per-row scrim bands removed. Residuals: field width pinned by the one photo row (2-byte-name padding OPEN); `[ADMIN]` prefix stays a stand-in (the photo's admin notice shows no sender); the photo's `/r` command echo has no shell source |
| Ink colors | FIXED — user rows amber (photo user row reads warm while known-white ink in the same photo calibrates neutral; was white); admin stays amber. Residuals: distinct user-vs-admin hues unverifiable in the lossy photo; caret white per photo |
| Row stacking | FIXED — pitch 17 native px (photo row tops 455.4/472.5/489.8; was the lobby 8 px), 6 visible slots fill the 98 px `LIST` rect exactly (was 12); fixture reordered admin→user so the peer's row sits last, matching photo rows 2–3. Chronological top-down confirmed both sides |
| Draft line | FIXED — focus caret now paints: 2×10 white bar inset (5,5) of `EDIT_SWAIT_CHAT` rect 19, riding the typed width (photo shows the caret on an empty input; we painted nothing). Residuals: draft-text origin unevidenced (photo input is empty); `game.rs`'s "empty histories paint nothing" sentence predates the caret (out-of-scope file — follow-up) |

Guards: `waiting_chat_row_text_brackets_the_sender_field`,
`waiting_chat_rows_stack_at_the_reference_pitch`,
`waiting_chat_paints_user_rows_in_amber_at_the_list_rect`,
`waiting_chat_without_live_rows_paints_only_the_caret`,
`waiting_chat_paints_the_caret_on_an_empty_input`,
`scripted_chat_fixture_delivers_admin_then_user_rows`
(renamed), `waiting_shot_paints_the_scripted_chat_rows` — all
proven red before the fix (compile-red for the new painter API,
order-red for the fixture swap).

Caption refresh 2026-11-07: the waiting walk now stages the song
catalog plus inventory before the shot (room song 757, counts
`7/42/1500` with the slot-3 999 cap), so MAPNAME/BPM/SKILLCOUNT
ink their `.bnd` rects instead of the bare `M0`/`-0-`/`  0`
defaults. Old-vs-new diff is 682 px, all inside the six caption
boxes — MUSICNAME native (573,15), MAPNAME (620,448), BPM
(552,504), skill counts (712,208/246/284) — each cluster matching
its rect; no other band moved. Guarded by
`waiting_shot_paints_mapname_bpm_skillcount_captions` (exact text
at each rect, proven against the staged sources).

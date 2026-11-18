# Journey audit — Spec: Channel state (#289)

Comparison leg only. No exe boot, no code change outside this doc, no push.
Base `2105ac6`; branch `cb2/ja-a`. Comparator keys as in `287-avi.md`. Exe
pin: `OTwo.exe` 5,029,888 B, sha256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.

## Per-story table

| Story | Comparator | Our journey ref | Exe evidence | Verdict | Notes |
|---|---|---|---|---|---|
| C1. Rows rendered from the 1003 reply (checked) | data-effect / wire-sequence | `crates/o2jam-net/src/message.rs:626-652` decode → `crates/recomp2jam/src/session.rs` `set_channels` → `crates/o2jam-shell/src/screens/game/mod.rs:1365` → `crates/o2jam-shell/src/screens/paint.rs:2373` `paint_channel_rows`; guards `channel_digits_select_tens_and_ones_frames`, `channel_row_label_paints_the_pinned_value_source` | t240 channel frame sha256 `ea5cc6d632a534ab02d7d6f12e10bf357f8cf5dc65db991a9b026ee0ecf6c563`; p343 `proof-final.json` sha256 `20fbe17060cea3d3da16a059b74de06a724b6bb41ee6ae2caf947b3ccfc8cff6` `channel_list_body_equal: true`, `wire_sequence_equal: true`; D25 value-source proxy pin in `docs/parity/channel.md` | MATCHED | Row data and digits match on the wire/data comparator. Pixel residual on row chrome is C9. |
| C2. Closed channels show CLOSE and refuse entry (checked) | data-effect / input-response | `crates/o2jam-net/src/message.rs` channel row `active` byte → `crates/recomp2jam/src/session.rs:4906` `ch190_scripted_gateway_inactive_row_refuses_face_enter_and_paints_dimmed`; #190/#212 lineage; docs/channel-fill.md row gating | `docs/channel-fill.md` live pin: an `active = 0` row paints the empty CLOSE slot; t240/c1 channel frames show CLOSE slots past the live list. Mempool proxy pins the `channel_id+1` display. | MATCHED | Which field gates the CLOSE slot stays OPEN (`active`/`capacity` co-vary); recorded, not silently resolved. |
| C3. Scrollbar drags, tracks, clamps (checked) | input-response | `crates/o2jam-shell/src/screens/paint.rs:483-495` `channel_scroll_hit` + `:495` arrow clamps; guard `channel_scroll_hit_uses_the_bnd_track_arrows`; #130/#174/#182/#189/#55 guards in `docs/parity/channel.md` | Static pin from a live memory read and capture: bnd scroll track `(573,213)-(594,429)`, arrows at `(0,0)/(0,196)`, thumb 21x45 (`docs/channel-fill.md`); no new scroll stimulus in the reused packs | MATCHED (static pin) | Capability and geometry match; bar-body drag stays OPEN in the spec lineage. |
| C4. Enter face completes 1004 and stages the character (checked) | wire-sequence / state-path | `crates/o2jam-shell/src/screens/game/mod.rs:1775` `take_join` / `crates/recomp2jam/src/session.rs:3847` login poll → `2001`; guards `face_click_enters_the_highlighted_channel`, `face_click_selects_channel_row_and_enters_rooms` | t240 `wine-drive.log` enter-face aim (526,248) → `03EC` room stage; c1 `drive.log` same; p343 proof (hash above) `wire_sequence_equal: true`, `server_slice_equal: true` | MATCHED | `1004 → 1005 → 2000 → 2001` identical in p343's matched window. |
| C5. Pre-list dialog when the relay is down (unchecked) | input-response / pixel-region | `crates/recomp2jam/src/relay_gate.rs:25` → `crates/recomp2jam/src/main.rs:601/1405` `stage_channel_list_after_relay_gate` → `screens/game/mod.rs:1380` hold rows, `:1290` channel-entry open, `:4450` OK hit, `:1421` dismiss; `live_boot_gate.rs:683-783` | #244 qualified exe pack `~/scratch/p244-capture`, popup `baf781b8caa6823b60047901e5a135d3ab77cb7fc60a7031fda02d90e408c1bf`, manifest sha256 `e211cc6e8016f7e7048276e3f2fa247744efbaeca6f766a36130a1984e535b20`, citation PASS; t240 `wine-drive.log` blue probe 0.986 → OK (400,365) → 0.000; `docs/parity/channel.md` #244/#348 | MATCHED | #244's qualified frame is the one-OK sheet over the Channel dialog with row wells behind, matching our paint placement. Boot-placement regression is #348, fixed on main by `51c2197`; no duplicate issue. |
| C6. Leaving sends 2021 (checked) | wire-sequence | `crates/o2jam-shell/src/screens/game/channel.rs` exit → `crates/recomp2jam/src/session.rs:2215-2230` `flush_channel_logout`, `Request::ChannelLogout` 2021 empty; guards at `session.rs:4523/4569` | `docs/net-protocol.md:125` static exe dispatch pin (file `0x48fd58` compare, send `0x48ff90`-`0x48ffa8`) plus scripted Memoryer probe: no 2021 → `1001 -1` on re-entry, with 2021 → `2022 0` then a successful 1004 | MATCHED (static wire pin) | Wire body and order pinned; guard named. |
| C7. Fill digits and rows probe-able (unchecked) | NA-DESIGN (tooling) | Paint/pin exist (`channel_digits_select_tens_and_ones_frames`, `channel_fill_clips_to_population_share`; `docs/channel-fill.md`); no mechanical rig probe | t240/c1 channel captures are human-labelled by scene signature (`docs/captures/t240/findings.md`); no digit-level probe in any pack | BLOCKED | #304 is the dedicated OPEN ticket for the rig probe; this is a tooling gap, not a player-visible divergence. No new issue. |
| C8. Wire moments pinned with send sites (checked) | wire-sequence | `crates/recomp2jam/src/session.rs` send sites for 1002/1004/2000/2002/2010/2024 + 2021 leave; `docs/net-protocol.md`; guards in `session.rs:4523+` | p343 proof (hash above): 19-opcode exe and Rust sequences byte-identical; `server_slice_equal: true`; earlier p339 mismatch (`~/scratch/matched-ab-339/proof.json` sha256 `0580bc8de663b6370f13854b14a52913f7fd1e6e4343d7e3782f6f22cbd636dd`) was fixed before p343 | MATCHED | A/B wire history is preserved as a contradiction, not silently dropped. |
| C9. Active row carries the exe's green select bar (unchecked) | pixel-region | `crates/o2jam-shell/src/screens/paint.rs:14406` `channel_select_bar_paints_at_the_highlighted_row`; staged `Dialog_ChannelSelectBar.ojs` at dialog+`0x8C`; pointer/highlight resolver in `screens/game/mod.rs` | p346 matched-window pair: exe `channel.png` sha256 `3c79af37986b4a554f8724088f0b058441ecd024bad5ba8b6dd00560e37723dc`, Rust `channel.png` sha256 `ef278f3d559a7cb92791436fe52836717bc0349a6cd09484dc084882a16f3fa5`; `~/scratch/p346/pack/region-metrics.json` sha256 `f52bf479c32d638d8afa814646ef0d151a7cda0e120d183f73b885b46102e60b` `row_bars` 0.606146; p346 proof `wire_sequence_equal: true`, `server_slice_equal: true`, `channel_list_body_equal: true` | MISMATCHED | Named divergence: exe bar outlines `Ch.03` (row 3), ours outlines `Ch.01` (row 0); exe also has an orange segment on `Ch.01` absent in ours. Wire and server slices match, so not dismissed as a scenario delta. Root cause (pointer state vs painter) stays OPEN. Filed #364. |
| C10. Dialog opens over the planet board with MENU band visible (unchecked) | pixel-region / state-path | `crates/o2jam-shell/src/screens/paint.rs:14262` `channel_dialog_body_stages_behind_the_live_list`; staged `State_Planet` block beneath the 800x466 sheet; guard chain in `docs/parity/channel.md` | Hashed channel frame `ea5cc6d6…c563` shows emblem strip and bottom MENU band; `#334` region map: `channel.emblem_strip` 0.942194, `bottom_bar` 0.9574 (`~/scratch/native-ab-334/pack/region-metrics.json` sha256 `a24a38f9eb9736033e33f511cc403882f0dd86a1f8e1a82dd1ce04a1bbee9070`) | MATCHED | Composition pin holds on both legs. Spec asks for a rule-7 live-chain annotation; that is a guard-text residual, not a pixel divergence. |
| C11. No invented pager glyphs below the list (unchecked) | pixel-region (negative pin) | `IDC_BUTTON_SCHANNEL_LEFT/RIGHT` stage zero rects; procedural pager readout removed; `docs/parity/channel.md` "Pager: RESOLVED NEGATIVE" | Hashed channel frame bottom strip; negative pin from live capture + sheet decode + bnd inventory (`docs/channel-fill.md` 2026-11-10) | MATCHED (negative pin) | Absence proven by both the sheet art inventory and the bnd; no pager painted on either leg. |

## Summary matrix — Spec: Channel state

| Verdict | Count | Stories |
|---|---:|---|
| MATCHED | 9 | C1-C6, C8, C10, C11 |
| MISMATCHED | 1 | C9 select-bar row |
| BLOCKED | 1 | C7 rig fill probe (#304) |
| NA-DESIGN | 0 | — |

### Live-lane BLOCKED list (no new boots here)

1. C7 — mechanical digit/row probe (#304).
2. C9 — fresh matched A/B with the pointer state named on both legs, to separate input routing from painter fault (#364 acceptance).
3. C5 — explicit non-OK click-block guard and a qualified Rust popup capture (the exe popup already has hash `baf781b8…408c1bf`); boot boundary is #348.

### Issue actions

- New issue #364 — "Bug: Channel green select bar paints on the wrong row in matched A/B" (`needs-triage`, `component:channel`, `P2`; number/title/labels read back). Body file `~/scratch/ja-a-289-channel-highlight.md`.
- No new issue for C5 (#244/#348 lineage) or C7 (#304).

### Blocker-class mappings

- C7 maps to capture/rig tooling (#245/#249/#250 class) and #304; live lane BLOCKED, no issue filed.
- No #326/#355/#274/#139/#347/#345 mapping: none of the Channel stories reaches Playing/Loading/readout. The C9 row-bar delta is a highlight/pointer question, not #274 (song-pick readout).

## `STATE_CHANNEL` — channel select (no state class)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | 50 controls, decl `0x27`, **no `.bnd`** (bare `BOUND`); `Dialog_ChannelSelectBar.ojs`, back/rank buttons (lifecycle §6) | Soak stages genuine dialog art under the pinned no-fill rule: `Dialog_ChannelInfo.ojs` body at its record anchor (0,0) (exe-mounted sheet 800x468; the parsed body is one 800x466 layer — 2 px runtime delta recorded, uncompensated) and `ChannelInfo_CH_Enter.ojs` frame 0 at the faces (exe art 44x24; parsed frames 44x21). Row geometry is now **exe-pinned** (live Wine memory read 2026-11-09, `docs/channel-fill.md`): the hosting `CStatePlanet` dialog object at `parent+0xF6C` holds the four viewport slot rects (216,224,572,275) / (216,275,572,324) / (216,324,572,375) / (216,375,572,436) at dialog+`0x94`; paint, scrim, and hit-test all ride this table (tops inclusive, bottoms exclusive), and the enter-face inset (row top + 12 → face Y 236) is pinned the same way — the reference capture's `232+50i` anchors were these rects' interior and are retired. Composition correction 2026-11-09 (live captures, `docs/planet.md`): the dialog opens **OVER the planet board** (verified first screen) — the shell paints the staged bound `State_Planet` block (backdrop + emblem buttons + promo + MENU bar) beneath the opaque 800x466 dialog sheet, and the MENU bar band stays visible in the bottom strip (y>468); the pinned no-fill corner rule keeps pixels no art covers untouched, and without board art the strip below y466 stays byte-identical to the art-less frame (soak + shots guard both arms). Composition correction 2026-11-10 (`docs/channel-fill.md` "METHOD CORRECTION"): `Dialog_ChannelInfo.bnd` (27 rects) pairs to the `0x8C` block by code ordinal `(code & 0xFFF) - 1` — its rects 2-5 equal the exe-pinned row rects exactly (4/4), which verifies the pairing. The bnd now pins the X button (541,151)-(589,189), the row-bar mount (row origin +1 → bar at (217,225)), the bar-local children (digit slots (50,11)/(63,11) 13x18, count plate (90,12) 185x19, enter face (285,11) 44x21 — face of row 0 = (502,236), exact against the exe-pinned inset and the capture), and the scroll track (573,213)-(594,429) with arrows at scroll-local (0,0)/(0,196) and the 21x45 body between them. The painter stages `ChannelInfo_CH_Back.ojs` per row (replacing the procedural pill/track chrome while staged — the chrome is the no-archive fallback), the digit/count plates, the enter faces, and the scroll bodies; absent figures fall back to today's behavior. The board EXIT `Planet_Btn_Exit.ojs` (750,561)-(788,599) rides the already-staged bound planet block; hit-testing lands with S0.3 (dispatch below). Block staging live-confirmed: 33 zero-rect records (state `0x12`), the `0x8C` dialog block carries 26 sprite-named records. Hover highlight (live 2026-11-09): the code-mounted `Dialog_ChannelSelectBar.ojs` (348x44) is the list highlight — its sprite record is live-confirmed mounted at dialog+`0x8C` (h/w/pitch 44/348/696 at +0x170) and it paints at the hovered row's rect (216,row_top) 348x44 (hover responds to mouse motion alone); the painter paints the staged bar there and keeps the amber cursor only as the no-figure fallback; since 2026-11-10 the shell tracks pointer motion (`GameLoop::mouse_moved`) and the bar follows the hovered row, falling back to the keyboard highlight when the pointer leaves the rows (`hover_moves_the_channel_select_bar_off_the_highlight`). Fallback chrome wells are now live-measured (2026-11-09): number well (225,228) 71x32 (the old (205,227) 94x35 pill was 20 px off) and track well (298,229) 195x32; the bnd-pinned enter face ((502,row.t+12) 44x21), X (541,151)-(589,189), and scroll track (573,213)-(594,429) all agreed with live measurement within ±2 px (brightness-run tolerance). Pager: RESOLVED NEGATIVE 2026-11-10 — the bottom strip (y~439-466) is live-confirmed dialog-sheet rim chrome plus the scroll column's down arrow; the decoded sheet art bakes no pager glyphs, the bnd names no pager rects, and the block's `IDC_BUTTON_SCHANNEL_LEFT/RIGHT` (`0x12200005/06`) stage zero rects and are statically unreachable (dead authoring vocabulary). The 7.55 channel dialog has NO pager: list navigation is the scroll column alone; the shell's procedural pager readout is removed and the earlier capture measurement is recorded as a rim-ink misread. Bare-board geometry (2026-11-09 reference capture, planet scene) is pinned as `PLANET_BOARD_*` constants for future wiring (promo nav cluster (345,509) 107x27, OPTION (712,562), EXIT (753,562), MENU back-arrow (130,562) 31x32; no hit-testing yet). Still reference-measured (exe-OPEN): the 48x24 plate stand-ins. Guards: `channel_row_hit_maps_the_exe_pinned_bands`, `channel_row_hit_mirrors_the_pinned_row_rects`, `channel_dialog_x_hit_uses_the_bnd_rect`, `channel_enter_face_is_bar_local_per_row`, `channel_bar_art_covers_the_measured_pill_track_band`, `channel_scene_without_bar_figure_keeps_procedural_chrome`, `channel_dialog_body_stages_behind_the_live_list`, `channel_enter_art_covers_the_measured_faces`, `channel_select_bar_paints_at_the_highlighted_row`, `channel_highlight_falls_back_to_the_cursor_without_select_bar`, `channel_fallback_chrome_uses_live_measured_wells`, `planet_board_reference_geometry_is_pinned`, soak `channel_layers_list_over_real_dialog_art`, shots `channel_shot_stages_dialog_art_inside_the_measured_bands`. Ticket #12 (2026-11-11): live rows — the `CH_Num` tens/ones wells show selected digit frames of the wire `channel_id` **plus one** (`(id+1)/10 % 10`, `(id+1) % 10` — rig-pinned 2026-11-10 by a proxy that rewrote the 1003 ids under the exe; list-ordinal display refuted), the `CH_ConnectCount` strip lights `ceil(population * 10 / capacity)` whole segments of its 185x19 plate (2026-11-19 proxy rewrite pin in `docs/channel-fill.md`; 1 px anchor delta stays OPEN as a named residual), FULL rows (active, `population >= capacity > 0`) show the baked `CH_Back` frame-2 FULL face with no enter face, and every other row shows bar frame 0 plus enter frame 0 (single-frame paints only: the loader stacks all frames as layers, and whole-figure paints smear). Faces ride above the bar (the frame-0 face well is opaque decoded art, not a cutout). The ASCII row string is gone on the staged path — live captures (`~/scratch/t-chanlist.png`, `q-chan.png`) pin that exe rows carry no text (digits, segments, and the enter face only), and the decoded sheet bakes its own CLOSE wells for slots past the live list, which the painter leaves untouched. No stand-ins remain on the staged path; the figure-less fallback keeps the procedural chrome plus the ASCII string, labeled. New guards: `channel_digits_select_tens_and_ones_frames`, `channel_fill_clips_to_population_share`, `channel_single_visitor_lights_one_whole_population_segment`, `channel_ten_visitors_light_one_segment_not_two`, `channel_full_row_uses_full_frame_without_face`, `channel_staged_digits_replace_ascii_text`; OPEN notes: strip anchor 1 px delta (next step: stopped-state exe sprite read), bar frame-1 trigger (no capture pins it), inactive-row gating (the 2026-11-10 capture pins that an `active = 0` row paints the empty CLOSE slot inside the list; `active` and `capacity` co-vary, so which field gates it stays OPEN — next step: a gateway row with `active = 0` and nonzero capacity). Emblem captions (TEXT `0x19400007..0B` → bnd rects 6–10, strings pinned in `docs/planet.md`) paint via `o2jam-text` and EXIT/OPTION hit-test via the control-code dispatch table (S0.3; EXIT is bnd rect 15 — rect 26 cannot exist in the 17-rect file); boot lands on the bare board and a lit-emblem click opens the dialog (shell-gated; the exe edge stays OPEN per E1). Scoping #72 (2026-11-12): the row-label value source is minted as claim D25 — a 1003 `channel_id` proxy rewrite names it — evidence issue #92 blocks the shell wiring #93. (ev: `docs/channel-fill.md`, `docs/planet.md`, `docs/hover.md`; live memory reads 2026-11-09/10) | PASS (R+G, v: 9692d41) |
| Input | Pick channel/planet → room (`My_channel`/`My_planet` bridge, file `0x24df70`–`0x24e024`) | Up/Down are rig-pinned no-ops (ticket #189), Confirm → Room, Back → Planet (Esc symmetry with the verified X close, guard `channel_back_closes_to_the_planet_board` plus `menu_backs_out_of_lobbies`) — entry-face click LIVE-VERIFIED 2026-11-09 (one click at (526,248) drives `03EC/0FBE/07D0/07D2/13A4` into `STATE_ROOM`, ~/scratch/planet-verify/); dialog X close and lit-emblem family switch verified the same session (ev: ~/scratch/planet-verify/ entry-face click 2026-11-09). Ticket #56: row-body clicks move the highlight only, only the enter face enters (keyboard Confirm unchanged); ticket #55: the scroll-column arrows scroll the list top, never the highlight (bar-body drag OPEN). Guards: `face_click_enters_the_highlighted_channel`, `body_click_highlights_without_entering`, `face_click_selects_channel_row_and_enters_rooms`, `channel_arrow_keys_leave_list_top_and_highlight_untouched`, `scroll_down_arrow_steps_the_channel_highlight`, `scroll_up_arrow_at_top_clamps_the_highlight`, `scroll_arrows_clamp_at_the_last_row`, `channel_face_hit_covers_the_measured_faces`, `channel_scroll_hit_uses_the_bnd_track_arrows` | PASS (R+G) (click path + #189 key no-op pin) / OPEN (Confirm/Back dispatch stays unpinned — the #189 sweep covered Up/Down only: 14+ SendInput presses moved zero row pixels on live populated lists while a scrollbar arrow click in the same session scrolled top 0→1; static leg found no VK_UP/VK_DOWN compare in .text; ev: `docs/channel-fill.md` #189 section, captures `~/scratch/t189-keyrule/` r2/b2/a/m legs) |
| Timing | None documented | n/a (ev: none — no timed dispatch documented) | N/A (R+G, v: 9692d41) (no timed behavior — no dispatch exists to guard; next step: none) |
| Transitions | `Channel → Room`; hosted by `CStatePlanet` (resolved live 2026-11-09) | `Channel → Room` (row confirm / entry faces); dialog X (bnd-pinned rect (541,151)-(589,189); verified close click (577,171)) closes to `Planet`; hosted by `CStatePlanet` (live memory read 2026-11-09) (ev: `docs/channel-fill.md`; hosted-by live read 2026-11-09) | PASS (R, v: 9692d41) (guards: `flow_allows_every_documented_edge`, `menu_walks_full_session_to_results`, `channel_back_closes_to_the_planet_board`) |

## Ticket #92 — row-label value source PINNED (2026-11-11)

Claim D25 is closed without a new rig session: the proxy rewrite its next
step named was already run on 2026-11-10 (`~/scratch/digits-rig/`, runs +5
and +10 moving the 1003 `channel_id` under the live exe) and the 2026-11-11
sheet decode (`docs/channel-fill.md` "Ticket #12 row-art decode") answers the
format half. Verdict:

- **Value source:** the 1003 row's `channel_id` (not the list ordinal, not
  the gateway id). Display number = `channel_id + 1`.
- **Format:** no client-drawn label text exists. The `Ch.` prefix is baked
  into the `Dialog_ChannelInfo.ojs` sheet art; the number is the tens/ones
  decimal digit frames of `ChannelInfo_CH_Num.ojs` (`(id+1)/10 % 10`,
  `(id+1) % 10`); `FULL` is the baked `ChannelInfo_CH_Back.ojs` frame-2 bar,
  not a text suffix. The U1-walk `Ch. 99 / FULL` rendering was the shell's
  invented ASCII stand-in (now the labeled figure-less fallback only, #93).
- **Identity vs display:** the entry face still sends the raw
  `channel_id` (run-1 capture: row painted `Ch.10` entered channel `09`).
- **OPEN (unchanged):** inactive-row gating field (`active` vs `capacity`
  co-vary in every capture), count-strip quantization, bar frame-1 trigger.

## Ticket #93 — row label wired from the pinned source (2026-11-11)

The figure-less fallback row label now paints the pinned D25 reading
instead of the invented `gateway/channel population/capacity` string:
`Ch. ` plus the one-based wire `channel_id` as two decimal digits
(ids past 98 wrap to their low two digits, matching the staged
`CH_Num` wells), with ` / FULL` on rows the pinned FULL rule marks
(active, `population >= capacity > 0`). The staged path is unchanged
(baked `Ch.` art + digit frames + FULL bar — no client text there);
the U1-walk `Ch. 99 / FULL` symptom is gone because the fallback no
longer renders the server-list dump. Text rides the modern font
rasterizer (owner decision: the exe pins positions/sizes, not glyphs).
Guards: `channel_row_label_paints_the_pinned_value_source` (value
source + wrap arms), `channel_row_label_marks_full_rows` (FULL arm,
zero-capacity exclusion), `scripted_gateway_channel_ids_move_the_row_label`
(the #92 proxy-rewrite method on the scripted-gateway seam: moved
ids repaint a different frame, unchanged ids repaint
byte-identically). `channel_rows_ink_at_measured_native_tops`
re-anchored: the Gulimche 'C' glyph tops the row 8 px above the old
digit-led stand-in (224+50·index). Shots: all 19 captures
byte-identical before/after — the staged channel frame shows no
delta, no PNG regen. (ev: D25 pin in `docs/claims.md`; `docs/channel-fill.md`)

## Ticket #337 — staged dialog sheet owns its list field and arrow plates (2026-11-18)

Native 800x600 A/B (7.55 exe SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`, Wine
reference frame `~/scratch/native-ab-334/pack/frames/ref/channel.png` SHA-256
`dfeacf9ef64c6508c635028efd657a107880031c8b9b18f583d5a121e6311d01`,
2x-rescaled native region map
`51f335c27b101152e395ac11338c45e0e78fde8430d60bd36ebdb972a77cb744`) pins two
blit deltas on the staged path:

- `Dialog_ChannelInfo.ojs` paints the list field and the bnd scroll track
  `(573,213)-(594,429)` itself. At native `(580,300)` the reference reads
  `(41,40,41)`, the decoded sheet reads `(41,41,41)`, and the Rust frame read
  `(8,8,16)` — `paint_channel_scrim` overwrote the sheet. The scrim now paints
  only without a staged sheet (`screens/paint.rs:992`, call at `:2287`).
- The sheet also bakes both arrow plates (white rule outside each triangle).
  Repainting `Messenger_Main_Scroll_Up/Down.ojs` swapped the rule order: the
  arrow crop's mean absolute error against the reference was 29.91/28.0 for
  Rust versus 9.8/0.49 for the sheet. The separate thumb body still repaints
  (`screens/paint.rs:812`, guard at `:851`).

Live caller chain: `main.rs::run_play` → `ScreenGame::paint` → `paint_at` →
`ScreenGame::paint_channel` (`screens/game/mod.rs:5863`) → `paint_channel_scene`
(`screens/paint.rs:2483`) → `paint_channel_rows` (`:2277`) →
`paint_channel_scrim` (`:1013`) / `paint_channel_scroll_art` (`:812`).
Scenario-named guards:
`staged_dialog_sheet_keeps_its_scroll_track_outside_the_scrim`,
`staged_dialog_sheet_keeps_its_scroll_arrows_but_not_the_thumb`. Native
gameplay crop SSIM before → after: `scroll_track` 0.467124 → 0.999670,
`row_bars` 0.539644 → 0.609808, flanks 0.521937/0.544637 → 0.538779/0.561471;
channel weighted 0.689990 → 0.715018. Residual gaps in row selection,
flank promo art and the footer strip are server-vintage/capture-scenario
deltas, not pinned painter faults. The full per-region metrics are local-only
(`~/scratch/blit-pass/pack/region-metrics.json` SHA-256
`dbc8401d69a973a221f1fd5be1877094a8cccafd258688de411035741aaeff5b`).

Correction (#371, after #342 selected the right promo frame): the flank
residual was a pinned painter fault, not a capture-scenario delta.
`Planet_Promotion.ojs` carries stored colorkey `0`; 7.55 installs a
source colorkey only for a nonzero key (`docs/claims-render.md` rule 4),
so its black vinyl pixels are opaque. Our blit skipped them and showed
the board art under the dialog's `0x7D5F` holes. The shared blit fix
(`crates/o2jam-render/src/lib.rs`, `transparent_rgb`) raised the p361
flanks from 0.871279/0.903726 to 0.999828/0.998838 SSIM.

## Ticket #244 — one-OK notice before the channel list (2026-11-18)

Qualified exe oracle: `~/scratch/p244-capture` (local, git-ignored),
one Wine boot on private display `:249`, owned null sink
(`route=sink`, 0.0 dBFS), `rig-wine-run.sh` checks
`manifest: PASS`, `audio: PASS`, `cite: PASS`, `status PASS`.
`citation.txt` pins manifest SHA-256
`e211cc6e8016f7e7048276e3f2fa247744efbaeca6f766a36130a1984e535b20`,
exe 7.55 SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`
(5,029,888 B), and the blue one-OK frame
`artifacts/screenshots/message-box.png` SHA-256
`baf781b8caa6823b60047901e5a135d3ab77cb7fc60a7031fda02d90e408c1bf`
(157,603 B). `drive.log:10-16`: popup captured 15:15:21, OK click at
`(400,365)`, popup gone, then channel frame with `server:03EA` at
15:15:27. The message is the CP949 string at exe file `0x244d98`
(VA `0x644d98`), three lines meaning "network problem; NORMAL mode
only; LIVE BATTLE unavailable"; its channel/planet emitters are
files `0x474eca` / `0x8e340` (parent queue helper VA `0x478c90`,
one-OK information sheet). The geometry is `Dialog_Information.bnd`
rect 0 `(241,206)-(559,393)` and rect 1 `(102,150,117,26)` under the
code-ordinal pairing (`docs/bnd-semantics.md:41`); ControlList codes
`0X21200002` (OK) and `0X21400003` (TEXT) come from
`ControlList_Interface.txt`'s `DIALOG_INFORMATION` block.

Contradiction kept, not silently resolved: a second exe string at
file `0x2471e8` literally says "no response from RELAY server", but
its only two handlers (files `0x74796`/`0x8e340`) test the shared
reconnect helper at VA `0x58c6c0`, whose body returns 1 on every
path. That branch is dead in 7.55; the observable dialog is the
NORMAL-mode string above.

Shell: `relay_gate` probes the pinned relay hosts/ports before the
channel exchange; `stage_channel_list_after_relay_gate` either holds
the fetched rows with `ScreenGame::hold_channel_rows_for_relay_notice`
or keeps the original `set_channels` path. Boot leaves the splash and
bare planet board clean; the Planet → Channel entry hook opens the
notice above the channel frame, and OK (or Back) reveals the held rows
in Channel. Live caller chain: `run_online` / `run_play --online` →
`relay_gate::relay_down_now` → connect →
`stage_channel_list_after_relay_gate` →
`ScreenGame::hold_channel_rows_for_relay_notice` →
`enter_screen(Channel)` → `open_relay_down_dialog` →
`paint_channel` → `paint_planet_modal` → `paint_relay_down_dialog`
→ OK → `dismiss_relay_down_dialog` → `paint_channel`. Guards:
`relay_down_notice_waits_for_channel_entry_and_leaves_boot_on_avi`,
`relay_down_dialog_holds_the_channel_list_until_ok_is_pressed`,
`relay_down_boot_holds_channel_rows_for_the_notice`,
`relay_up_boot_keeps_the_existing_channel_path`,
`relay_down_sheet_stages_from_the_installed_information_block`,
`information_dialog_stages_the_one_ok_sheet`,
`relay_up_when_a_pinned_style_endpoint_answers`,
`relay_down_when_every_pinned_style_endpoint_refuses`.

## Ticket #348 — notice boundary corrected (2026-11-18)

Owner regression report: the notice opened at boot instead of channel
entry. The #244 implementation called `open_relay_down_dialog` inside
the boot staging and forced `Screen::Avi → Screen::Planet`, so the
splash never landed. Exe oracle unchanged: the #244 frame shows the
one-OK sheet over the Channel dialog (its title bar and row wells are
visible behind the sheet), then the `03EA` channel frame after OK.
The contradiction is between that frame and the boot-time placement,
not between two exe observations. Correction: split hold (boot) from
paint (channel entry); no geometry, string, or probe change. The one-OK
hold and relay-up path keep their #244 pins.

Harness note: the default `online` boot applies the gate. The
`input-drive` X11 routing probe passes `--no-relay-gate` so its gear
click still measures input delivery on the bare planet board; guard
`x11_game_args_skip_the_relay_notice_gate` pins that argument. This
is a trace affordance, not an exe behavior claim.

## Ticket #717 — refused channel-enter notice (evidence pin, 2026-11-20)

Oracle: `OTwo.exe` 7.55, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`
(5,029,888 B), Ghidra 12.2 / REA `target-83a045418b62.exe`. Static
pack and live boot pack are cited in `~/scratch/channel-full-findings.md`
(`~/scratch/channel-full/win3`). The 1005 handler at VA `0x55a110`
(file `0x15a110`) selects text through the 8-entry jump table at VA
`0x55a324` (file `0x15a324`) and posts queue type 1, the one-OK
`Dialog_Information` sheet (same sheet as #244/#367, geometry from
`Dialog_Information.bnd` by code-ordinal pairing). Live result-7 frame:
blue one-OK sheet with `채널 정보가 없습니다.`, OK near (400,365)
back to the unchanged Channel dialog; the highlighted row keeps its
enter face. No sound transient, highlight write, or row repaint is
observed (static chain has no sound dispatch; the statically visible
"mark row full" arm is superseded by the live observation because
the notice becomes the top dialog first).

Pin the CP949 strings at their exact VAs (bytes through the NUL;
VA = file offset + `0x400000`):

| Result | VA / file offset | CP949 text | REA evidence this pin |
|---|---|---|---|
| 1 with `Restricted = 0`, 2..=5, codes past 7 (default arm) | `0x64d740` / `0x24d740` | `채널 허용 인원이 초과된 상태라서 더 이상 들어갈 수 없습니다.\n\n다른 채널을 이용해 주세요.` | `ev_4c4e43875c0d39d4029adb3b49c2f4713c64828965311535bc9658595b390554` |
| 1 with `Restricted = 1` | `0x64d6c8` / `0x24d6c8` | `유료곡 대전을 활성화하기 위한 서버입니다. 유료곡이 없으신 분은 입장하실 수 없습니다.\n\n다른 서버를 이용해 주세요.` | `ev_a2583cf8b9bfa43458b195d7862f47fdeacae009ad9cbd34d483dc06f7121e37` |
| 6 | `0x64d6a4` / `0x24d6a4` | `사용자 정보를 읽어오지 못했습니다.` | `ev_3c7f9db2b1b6ef599ab9785bb312c54bf03032d019aa7e26392776e4d6de60c9` |
| 7 | `0x64d68c` / `0x24d68c` | `채널 정보가 없습니다.` | `ev_1bfd3059a4523c59d86d10333711fefad7304e7643cdff23f7e7b24e9f6d09b4` |

Result 0 is success: no sheet, the identity fetch follows. Other
values of the optional `Restricted` word post no sheet in the exe;
our wire decoder admits only 0/1, so that arm stays unrepresentable.

Shell stand-in at this pin: `OnlineSession::drain_channel_login`
logs `channel login rejected; staying in Channel` and leaves the
dialog unchanged (no sheet). Status: OPEN — implementation and
guards ride the fix commit. The keyboard-confirm fan-out to the
channel vftable slot stays untouched and OPEN by the channel-full
lane's pin (next step: stopped-state breakpoint on `0x475260` with a
focused canvas); this ticket covers the 1005 answer path only.

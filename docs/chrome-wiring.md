# Chrome wiring sweep table — ticket #163

Dated 2026-11-12. Every chrome control in every scene has a
scenario-named guard for hover and for the click gesture
(press+release through the routed `mouse_click` driver), written
failing-first. Legs 1–4 landed the planet/channel/waiting/result
guards; this table records the FINAL legs (side screens, loading,
splash) in full and points at the earlier legs' guards.

Method per control: enumerate from the staged `.bnd` placements
(ADR-0003 — the `.bnd` controls, one entry per control, never
literals); sweep `mouse_moved` over each chrome rect, then
press+release; wire where evidence exists, otherwise guard the
LABELED stand-in with the next evidence step named. Fixtures obey
the #63 distinct-frame rule (`docs/parity/art-delta-guards.md`).

Shared evidence pins (reused, never re-derived):

| pin | source |
|---|---|
| Hover: rest = frame 0 desaturated, hover = frame 0 full color, active = frame 1 | `docs/hover.md` rig sweep |
| Press: frame swap on ButtonDown, action on release-inside (up), release-outside reverts, no action | #111 synthesis |
| Tracking is a polled per-frame hit sweep — motion alone arms hover | `docs/mouse-tracking.md` A46 |
| Clicks route to the covering layer first, so hover follows (room rule) | `hovered_room_control` + `click_dispatch` order |
| FREE/DRUM emblems are documented exe no-ops, not silence | ticket #163 brief |

## Legs 1–4 (landed — guards live in `game.rs` tests)

| scene | hover guard | click guard | evidence |
|---|---|---|---|
| Planet | `hovered_planet_action_tracks_pointer_motion`, `hard_normal_and_drum_sweeps_arm_their_emblems`, `disc_sweep_arms_the_gear_and_exit_discs`, `planet_hover_stays_off_while_a_modal_owns_the_board`, `playstop_hover_arms_the_frame_swap_cell` | `click_dismisses_splash_into_planet_board`, `face_click_*` (channel entry) | rig sweep bboxes; #111 press split |
| Channel | `channel_chrome_sweep_arms_rows_but_not_chrome`, `channel_hover_follows_the_scrolled_viewport`, `channel_hover_survives_a_slash_draft` | `x_button_click_closes_the_dialog_to_the_planet_board`, `channel_scroll_bar_body_click_is_a_labelled_noop`, `empty_channel_list_answers_no_hover_or_click` | bnd-pinned X rect (541,151)-(589,189); NO PAGER readout |
| Waiting | `waiting_chrome_sweep_arms_no_hover_state` | `waiting_start_press_fires_on_down_and_holds_on_release`, `waiting_team_sweep_queues_4004_then_4008_per_button`, `waiting_unwired_chrome_clicks_land_nowhere`, `waiting_dialog_up_routes_rows_and_holds_chrome` | 4004/4008 relay pins |
| Result | `result_chrome_sweep_paints_no_hover_state`, `result_rank_rows_are_not_chrome` | `result_retry_press_fires_on_down_and_queues_4010_once`, `result_back_release_outside_holds_rooms` | 4010/4021 relay pins (#87/#88) |
| Room | `hovered_room_control_*`, `room_hover_hides_behind_the_open_dialog` | per-arm guards incl. create-room popup (#134), top strip, FIRSTSTEP visits Tutorial (#34) | `docs/hover.md` room table |

Options dialog rides the in-flight #131/#132 findings (this sweep
adds no duplicate geometry work).

## Side screens (this leg — full tables)

Hover rule for all seven sides: no side screen owns hover art.
`side_hover_sweep_arms_no_hover_state` sweeps every staged placement
center plus a blank corner on every side and asserts
`hovered_planet_action` / `hovered_channel_row` /
`hovered_room_control` all stay `None` with the side open and the
lobby held. Next evidence step for all rows below: the rig hover
sweep naming a side hover frame, if any exists.

One wiring fix fell out of the sweep: `hovered_channel_row` missed
the side-layer gate its room sibling has, so the bar armed under an
open overlay. `side_hover_from_channel_arms_no_channel_row` failed
first, then the `side.is_some()` gate landed (room-resolver
precedent + `click_dispatch` order).

### Shop (`SideScreen::Shop`) — wired

Guard: `shop_chrome_sweep_presses_every_staged_control`.

| control | outcome | evidence/stand-in |
|---|---|---|
| `ITEMBUY` ×8 row toggles | check/uncheck the cart in order | pinned 5000 send arm |
| `BUYITEM` | empty cart queues nothing; full cart queues one 5000 send, seam drains once | pinned 5000 send |
| Scroll pager (relative children) | steps 8-row pages, holds at both ends | staged scroll origin |
| Cart pager | steps over a full cart, holds at both ends | empty-cart `1/1` rule |
| `GEMMONEY`, page digits, scroll bar body, `ITEMNAME`/`ITEMWON` templates | land nowhere, queue nothing | display-only, no-op guard |

### Music shop (`SideScreen::MusicShop`) — wired

Guard: `music_shop_chrome_sweep_presses_every_staged_control`.

| control | outcome | evidence/stand-in |
|---|---|---|
| `MS_EX` ×5 row picks | stage the cart in pick order | pinned 5010 send arm |
| Tiled `GEM_BUY` / `WON_BUY` row buttons | each queues one 5010 send, seam drains once | staged slot-origin tiling |
| `TITLE` / `DIFF` / `BPM` sorts | resort and reset the page | `from_code` dispatch arm |
| `MSALL` pager | turns and holds at both ends | staged pager |
| Cart `BUYITEM`, cart pager | commit queues 5010; pager turns over a full cart, holds | pinned 5010 send |
| Page digits | land nowhere | display-only, no-op guard |
| `PRICE` | unstaged in the fixture; display-only with no price source | OPEN |

### Avatar room (`SideScreen::MyRoom`) — wired + labeled no-ops

Guards: `myroom_chrome_sweep_presses_every_staged_control`
(`myroom_open_overlay` now also stages `VACATE` + `GOSHOP`).

| control | outcome | evidence/stand-in |
|---|---|---|
| 11 slot buttons | select the slot (filter semantics OPEN, `docs/myroom.md`) | staged ordinal |
| 10 bag cells + bag pager | pick cells; pager turns over staged inventory, holds | staged inventory |
| `APPLY` (INVENTORY) | equips the picked cell, queues 5004, drains once; nothing picked queues nothing | exe slot derivation (`docs/equip.md`) |
| 5 gift cells + gift pager | pick cells; pagers hold; claim with no staged rows queues nothing | staged gift views |
| Caption codes (`NICKNAME`, `LEVEL`, …) | land nowhere | text, not buttons |
| `VACATE` | labeled no-op: records only, queues nothing | send shape OPEN — next: rig capture of a Vacate press |
| `GOSHOP` | labeled no-op: keeps the room (nested sides rejected) | nested-side rejection |
| `BACK` | returns to the opening lobby | `can_return_side` |

### Billing (`SideScreen::Payment`) — click-silent by design

Guard: `payment_chrome_sweep_clicks_land_nowhere` (hover covered by
the side hover sweep).

| control | outcome | evidence/stand-in |
|---|---|---|
| Every staged billing rect + blank points | route nothing; side stays open; lobby holds; all send seams stay empty | billing completes no web call — labeled stand-in; next: rig capture of a payment click with Memoryer up |

### Planet side + tutorial — click-silent by design

Guard: `planet_side_and_tutorial_clicks_land_nowhere` (hover covered
by the side hover sweep).

| control | outcome | evidence/stand-in |
|---|---|---|
| Every staged rect + blank points on both sides | route nothing, queue nothing, keep the side open (still exitable via `leave_side`) | solo-entry edge OPEN (`docs/planet.md` OPEN 6); next: rig capture of a planet-side emblem press |

## Loading gate + splash — deliberate absence

No chrome exists on either screen; the guards assert the absence so
a future control must arrive with evidence, not by accident.

| screen | guard | asserted absence |
|---|---|---|
| Loading | `loading_chrome_absence_sweep_routes_no_click_or_hover` | clicks at corners, center, and the staged READY rect route nothing and arm no hover; the screen holds Loading; menu keys still route (`Back` returns to Waiting) — the absence pins clicks, not input |
| Splash | `splash_chrome_absence_sweep_every_click_lands_the_same_board` | no hover state arms anywhere; all five canvas clicks (corners, center) dismiss to the same bare board (equal state hashes) — the whole-screen dismiss is the gesture, not a control |

## Checklist (ticket #163)

- [x] Planet board: emblems, gear, exit, promo cells + FREE/DRUM labeled no-ops
- [x] Channel: rows, select bar, enter face, scrollbar, pager
- [x] Room: chrome on both list arms, create-room popup, top strip, FIRSTSTEP visits Tutorial, dialogs
- [x] Waiting: top buttons, seat covers, team buttons, music-select cells
- [x] Side screens: shop / musicshop / myroom / lanking / payment chrome (silence asserted per control or wired)
- [x] Result: retry/back/confirm chrome
- [ ] Options dialog: in-flight #131/#132 findings land the wiring tests (not this leg)
- [x] Loading gate + splash: absence asserted deliberately

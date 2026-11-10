# Shops hand session — smoke note (2026-11-12)

Offline shell run (no gateway): both shops open from Waiting through
the side-screen seam (`visit_side(SideScreen::Shop/MusicShop)`),
against staged side art (`load_side_scene` with the genuine
`STATE_SHOP`/`STATE_MUSICSHOP` placements) and staged synthetic
catalogs (10-row `AvatarItemTable`, 7-song `OJNList.dat` records).
Clicks drive the public `click_shop`/`click_music_shop` seams (the
same seams the windowed binary feeds through
`GameLoop::mouse_click`); paint runs the side overlays
(`paint_shop_board` / `paint_music_shop_board` + cart over
`side_placements`). Queued buys drain through the take-seam
(`take_shop_buy` → 5000, `take_music_buy` → 5010 — ASSUMED
wire-side, never exe proof).

## What was clicked, what happened

Item shop open from Waiting; scroll NEXT click (staged
`IDC_BUTTON_SSHOP_SITEMRIGHT`, `0x14200049` through the staged
scroll origin) turns to page 1 (8/2 over the eight staged
`ITEMBUY` hit areas) and back; first-row `ITEMBUY` click checks
Good01 into the cart; cart `BUYITEM` click (staged
`IDC_BUTTON_SSHOP_BUYITEM`, `0x14200070`) queues the pinned empty
5000 send and the seam drains it once. Song shop open from
Waiting; TITLE sort click (staged `IDC_BUTTON_MSHOP_MS_TITLE`,
`0x1A200080`) resorts and resets the pager; MSALL NEXT click
(staged `IDC_BUTTON_MSHOP_MSALL_RIGHT`, `0x1A20000B`) turns to
page 1 (5/2 over the five staged `MS_EX` hit areas) and back;
first-row `MS_EX` click stages song 757 into the cart; row
GEM_BUY click (tiled `IDC_IMAGE_MMS_GEM_BUY`, `0x1A200041` over
the staged slot origin) queues the pinned empty 5010 send and the
seam drains it once. Back (`leave_side`) returns to Waiting.

Evidence: screen path `waiting → shop → waiting` /
`waiting → musicshop → waiting`, state hashes
`07a26fe11d0d9c4a` (shop open) → `085b1af61b85cfab`
(page 1) → `abf7819035bb3eab` (Good01 checked) →
`b88d86a046d514ae` (5000 queued); music
`ecba3f591475a287` (song shop open) → `16b70ed241ae880f`
(TITLE sorted) → `54ff53a04fe97fce` (page 1) →
`b65513055311b344` (song 757 staged) → `f668ca3d6ba933eb`
(5010 queued). Guards: `shops_hand_session_browses_and_buys`,
`music_shop_hand_session_sorts_pages_and_buys` (plus
`shop_browse_toggles_cart_and_commits_5000`,
`shop_buyitem_without_cart_queues_nothing`,
`shop_pager_turns_eight_row_pages`,
`shop_wallet_gem_caption_waits_for_wire_data`,
`shop_unstaged_clicks_land_nowhere`,
`music_shop_sorts_pages_and_buys`,
`music_shop_row_pick_stages_the_cart_and_cart_buy_queues_5010`,
`music_shop_unstaged_clicks_land_nowhere` and the `side_tests`
dispatch/slot guards and the two `paint_tests` board guards).

## Deltas found (new rows, never silent fixes)

1. **Sort directions read ascending (ASSUMED).** The exe sort
   codes name the keys, never the order. Next step: SendInput
   sort-click sweep on the live song shop per the
   `~/scratch/planet-verify/` recipe, capture the row order,
   then pin the direction.
2. **Pager left/right pairing reads left=back, right=forward
   (ASSUMED).** Next step: same rig sweep on the six pager
   buttons plus captures.
3. **Both buy sends ride the take-seam shell-side only.**
   `SyncItemPurchase` 5000 (empty, file `0xf3fa8`) and
   `SyncMusicPurchase` 5010 (empty, send-switch file `0xa4df0`
   index 8) are exe-pinned sends; the queued bodies have no
   session drain yet. Next step: gateway replay confirming 5000
   / 5010 on the wire plus the 5001 / 5011 answer shapes (5001
   is confirmed undecodable standalone; 5011 is untraced).
4. **Gem prices, song prices, and the song DIFF cell stay
   blank.** No file→entry price mapping or level→cell source is
   pinned. Next step: gateway replay at known prices plus a rig
   capture of both price cells and DIFF at a known song.
5. **Filters, set-item, bag, gift, MYALL pager, search, TOP100,
   cart rows on the song cart tab, and download stay
   display-only.** Next step: rig sweep of each control plus
   captures, or dossier offsets for their handlers.
6. **Item-scroll pager hit-tests through the staged scroll
   origin.** The `SITEMLEFT/RIGHT` children stage scroll-relative
   on the dump; the offset rides the staged scroll rect. Next
   step: live click at the staged pager plus a capture proving
   the page turns.

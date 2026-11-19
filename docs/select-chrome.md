# Dialog_SelectMusic control inventory and chrome wiring (#285)

Version pin: `OTwo.exe` 7.55, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
Control names/codes below come from the installed `Interface1.opi`
`ControlList_Interface.txt` block `DAILOG_SELECTMUSIC` (archive typo
kept), paired by code ordinal with `Dialog_SelectMusic.bnd` rectangles
(`docs/file-formats/opi.md`). Behavior pins are static byte walks of
the read-only exe; file offset = VA − `0x400000`. No original bytes
are stored here. #272 staged the sheet with three body families held
back; #285 makes them paint-gated families instead of absent art.

## Pinned behavior families

- **Mode switch** — `MUISCSELECT 0x2720001A` and `RANDOMSELECT
  0x2720001B`. The dialog setup district (VA `0x44f8a0`, file
  `0x4f8a0`) sets the two buttons' state words and frame indices; the
  random-panel arm starting at VA `0x44f9a7` steps off `0x1A` onto
  `0x1B`, and the song-panel restore starting at VA `0x44fdc0` steps
  back. Both walk the five `RANDOM1_*` codes `0x2720104F..0x27201053`
  and the level codes `0x2720101C..0x27201020`, writing each control's
  state through `0x409bf0` (VA `0x44faca`/`0x44fac5` and neighbours).
  Only one panel is live.
- **Help sheets** — `LABELQUESTION 0x27200053` and `SEARCHQUESTION
  0x27200055` (VA `0x441a06`/`0x441a0f`). Each shows its own guide
  body (`0x27500054` / `0x27500056`) through helper VA `0x44f800`
  and hides the shared `LISTGUIDIMG 0x27500057`. The common
  hide-all arm is VA `0x441aa0`. The help ? buttons share rects with
  static art (#272 placement half): `Dialog_SelectMusic.ojs` bakes the
  dark plates at `(198,93)` and `(755,50)`, so `SelectMusic_LabelQuestion.ojs`
  and `SelectMusic_SearchQuestion.ojs` stage hidden — their grey faces
  would cover the exe's plates. Their rects still take clicks. Whether
  the grey faces paint on hover or help-active stays OPEN.
- **Button frames** — sprite-55 records of one button body are states,
  not stacked layers. The exe swaps a frame per control state; the
  dialog paints frame 0 for inactive bodies and the last frame for the
  chosen filter in each column (the opening `STEP1_ALL`/`STEP2_ALL`/
  `STEP3_ALL` faces in `reference-musicselect-4x3.png`).
- **Pager** — `UP/DOWN 0x27200044/0x27200045` and `LEFT/RIGHT
  0x27200046/0x27200047` land in the four-arm jump table at VA
  `0x43e9a0` (dispatch VA `0x43e2a8`: `code - 0x27200044`, `cmp 3`).
  UP/DOWN call the reorder helper VA `0x450740` and need a selected
  row; with none they raise caption VA `0x64586c` ("select a title to
  reorder first") on the shared notice `0x77acc8`. LEFT/RIGHT guard
  the page index (`dialog+0x6b4`) against the page count
  (`dialog+0x6b8`), so they clamp at the ends. On the result tab UP/
  DOWN are disabled (VA `0x43e52d`/`0x43e550`). Keyboard keys 11/12
  forward to LEFT/RIGHT (table VA `0x4419bc`).
- **Search** — `SEARCH_TITLE 0x27201049` sets the dialog search field
  to 0 (VA `0x43e92f`), `SEARCH_ARTIST 0x2720104A` to 1 (VA
  `0x43e957`); `SERACH 0x2720004C` runs the search (key-forward arm
  VA `0x44195e`). The filter compares the typed text against song
  title/artist using the key-name table at VA `0x6454d8` (file
  `0x2454d8`; indexed loads file `0x448c2`/`0x44ecc`, `strnicmp`
  call VA `0x5f3f0c`), decoded in `docs/xtd-table.md`.
- **Chosung grid** — `SERACH_INDEX0..41 0x27200058..0x27200081`
  (42 buttons) and the two special buttons `0x2720B082`/`0x2720B083`
  are filter inputs. The indexed keyed rebuild is VA `0x444630`:
  `code - 0x27200058` selects a key-name arm, row stride `0x238`.
  Wiring the grid is a separate slice; this leg inventories it only.
- **Rows and filters** — `EXTEND1..10` are the ten visible row hit
  bands (ticket #15). `STEP1_*` bands, `STEP2_*` buckets and `STEP3_*`
  genres are the three wired filter columns (ticket #273). OK is
  ticket #232. Other playlist-family controls (sort, check-all,
  remove-all, tabs, download/hear/info/rank) are display-only on the
  result tab this client stages; their playlist-tab behavior stays
  OPEN with the exe jump table above as the next evidence step.

## Full inventory (134 controls, block order)

| Control id | Code | Pinned behavior / state |
|---|---|---|
| `IDC_DIALOG_SELECTMUSIC` | `0x27000000` | sheet root — display-only |
| `IDC_BACK_DSMUSIC_1` | `0x27100001` | backdrop — display-only |
| `IDC_BUTTON_DSMUSIC_OK` | `0x27200002` | OK confirms staged pick and closes (#232) |
| `IDC_BUTTON_DSMUSIC_CANCEL` | `0x27200003` | CANCEL closes dialog (interactive; exact arm OPEN) |
| `IDC_BUTTON_DSMUSIC_EXTEND1` | `0x27017004` | result row hit band — interactive (#15) |
| `IDC_BUTTON_DSMUSIC_EXTEND2` | `0x27017005` | result row hit band — interactive (#15) |
| `IDC_BUTTON_DSMUSIC_EXTEND3` | `0x27017006` | result row hit band — interactive (#15) |
| `IDC_BUTTON_DSMUSIC_EXTEND4` | `0x27017007` | result row hit band — interactive (#15) |
| `IDC_BUTTON_DSMUSIC_EXTEND5` | `0x27017008` | result row hit band — interactive (#15) |
| `IDC_BUTTON_DSMUSIC_EXTEND6` | `0x27017009` | result row hit band — interactive (#15) |
| `IDC_BUTTON_DSMUSIC_EXTEND7` | `0x2701700A` | result row hit band — interactive (#15) |
| `IDC_BUTTON_DSMUSIC_EXTEND8` | `0x2701700B` | result row hit band — interactive (#15) |
| `IDC_BUTTON_DSMUSIC_EXTEND9` | `0x2701700C` | result row hit band — interactive (#15) |
| `IDC_BUTTON_DSMUSIC_EXTEND10` | `0x2701700D` | result row hit band — interactive (#15) |
| `IDC_IMAGE_DSMUSIC_CHECK` | `0x2750000E` | icon/static body — display-only |
| `IDC_IMAGE_DSMUSIC_REMOVE` | `0x2750000F` | icon/static body — display-only |
| `IDC_IMAGE_DSMUSIC_COLOR` | `0x27500010` | icon/static body — display-only |
| `IDC_IMAGE_DSMUSIC_IMG_DIFF` | `0x27500011` | icon/static body — display-only |
| `IDC_TEXT_DSMUSIC_TITLE` | `0x27400012` | caption — display-only (live text layer) |
| `IDC_TEXT_DSMUSIC_ARTIST` | `0x27400013` | caption — display-only (live text layer) |
| `IDC_TEXT_DSMUSIC_TXT_DIFF` | `0x27400014` | caption — display-only (live text layer) |
| `IDC_IMAGE_DSMUSIC_ISDOWN` | `0x27500015` | icon/static body — display-only |
| `IDC_IMAGE_DSMUSIC_HEAR` | `0x27500016` | icon/static body — display-only |
| `IDC_IMAGE_DSMUSIC_INFO` | `0x27500017` | icon/static body — display-only |
| `IDC_IMAGE_DSMUSIC_RANK` | `0x27500018` | icon/static body — display-only |
| `IDC_IMAGE_DSMUSIC_SELECT_BG` | `0x27500019` | icon/static body — display-only |
| `IDC_BUTTON_DSMUSIC_MUISCSELECT` | `0x2720001A` | mode switch — wired (#285) |
| `IDC_BUTTON_DSMUSIC_RANDOMSELECT` | `0x2720001B` | mode switch — wired (#285) |
| `IDC_BUTTON_DSMUSIC_STEP1_ALL` | `0x2720101C` | level filter / song-panel band — wired (#273, mode gate #285) |
| `IDC_BUTTON_DSMUSIC_STEP1_EASY` | `0x2720101D` | level filter / song-panel band — wired (#273, mode gate #285) |
| `IDC_BUTTON_DSMUSIC_STEP1_INTERMEDIATE` | `0x2720101E` | level filter / song-panel band — wired (#273, mode gate #285) |
| `IDC_BUTTON_DSMUSIC_STEP1_HIGH` | `0x2720101F` | level filter / song-panel band — wired (#273, mode gate #285) |
| `IDC_BUTTON_DSMUSIC_STEP1_HIGHEST` | `0x27201020` | level filter / song-panel band — wired (#273, mode gate #285) |
| `IDC_BUTTON_DSMUSIC_STEP2_ALL` | `0x2720B021` | label filter — wired (#273) |
| `IDC_BUTTON_DSMUSIC_STEP2_GOLD` | `0x2720B022` | label filter — wired (#273) |
| `IDC_BUTTON_DSMUSIC_STEP2_BLACK` | `0x2720B023` | label filter — wired (#273) |
| `IDC_BUTTON_DSMUSIC_STEP2_BLUE` | `0x2720B024` | label filter — wired (#273) |
| `IDC_BUTTON_DSMUSIC_STEP2_RED` | `0x2720B025` | label filter — wired (#273) |
| `IDC_BUTTON_DSMUSIC_STEP2_SILVER` | `0x2720B026` | label filter — wired (#273) |
| `IDC_BUTTON_DSMUSIC_STEP3_ALL` | `0x2720B027` | genre filter — wired (#273) |
| `IDC_BUTTON_DSMUSIC_STEP3_BALLAD` | `0x2720B028` | genre filter — wired (#273) |
| `IDC_BUTTON_DSMUSIC_STEP3_ROCK` | `0x2720B029` | genre filter — wired (#273) |
| `IDC_BUTTON_DSMUSIC_STEP3_DANCE` | `0x2720B02A` | genre filter — wired (#273) |
| `IDC_BUTTON_DSMUSIC_STEP3_HIPHOP_SOUL` | `0x2720B02B` | genre filter — wired (#273) |
| `IDC_BUTTON_DSMUSIC_STEP3_JAZZ_FUNK` | `0x2720B02C` | genre filter — wired (#273) |
| `IDC_BUTTON_DSMUSIC_STEP3_CLASSIC` | `0x2720B02D` | genre filter — wired (#273) |
| `IDC_BUTTON_DSMUSIC_STEP3_ETC` | `0x2720B02E` | genre filter — wired (#273) |
| `IDC_IMAGE_DSMUSIC_TABRESULT` | `0x2760002F` | tab art — interactive tab family, playlist OPEN |
| `IDC_IMAGE_DSMUSIC_TABPLAYLIST` | `0x27600030` | tab art — interactive tab family, playlist OPEN |
| `IDC_IMAGE_DSMUSIC_TABPLAYLIST_BACK` | `0x27500031` | tab art — interactive tab family, playlist OPEN |
| `IDC_IMAGE_DSMUSIC_TABPLAYLIST_ANI` | `0x27500032` | tab art — interactive tab family, playlist OPEN |
| `IDC_TEXT_DSMUSIC_TABRESULT` | `0x27400033` | caption — display-only (live text layer) |
| `IDC_TEXT_DSMUSIC_TABPLAYLIST` | `0x27400034` | caption — display-only (live text layer) |
| `IDC_BUTTON_DSMUSIC_SERACH_SORT_NEW` | `0x27200035` | playlist sort/selection control — display-only on result tab, OPEN |
| `IDC_BUTTON_DSMUSIC_POPULAR` | `0x27200036` | playlist sort/selection control — display-only on result tab, OPEN |
| `IDC_IMAGE_DSMUSIC_CHECKALL_BG` | `0x27500037` | icon/static body — display-only |
| `IDC_BUTTON_DSMUSIC_CHECKALL` | `0x27201038` | playlist sort/selection control — display-only on result tab, OPEN |
| `IDC_IMAGE_DSMUSIC_REMOVALL_BG` | `0x27500039` | icon/static body — display-only |
| `IDC_BUTTON_DSMUSIC_REMOVEALL` | `0x2720003A` | playlist sort/selection control — display-only on result tab, OPEN |
| `IDC_BUTTON_DSMUSIC_COLOR` | `0x2720003B` | playlist sort/selection control — display-only on result tab, OPEN |
| `IDC_BUTTON_DSMUSIC_DIFF` | `0x2720003C` | playlist sort/selection control — display-only on result tab, OPEN |
| `IDC_BUTTON_DSMUSIC_TITLE` | `0x2720003D` | playlist sort/selection control — display-only on result tab, OPEN |
| `IDC_BUTTON_DSMUSIC_ARTIST` | `0x2720003E` | playlist sort/selection control — display-only on result tab, OPEN |
| `IDC_BUTTON_DSMUSIC_DIFF_LEVEL` | `0x2720003F` | playlist sort/selection control — display-only on result tab, OPEN |
| `IDC_BUTTON_DSMUSIC_ISDOWN` | `0x27200040` | playlist sort/selection control — display-only on result tab, OPEN |
| `IDC_BUTTON_DSMUSIC_RANK` | `0x27200041` | playlist sort/selection control — display-only on result tab, OPEN |
| `IDC_TEXT_DSMUSIC_SERACHRESHLT` | `0x27400042` | caption — display-only (live text layer) |
| `IDC_IMAGE_DSMUSIC_UPDOWN_BG` | `0x27500043` | icon/static body — display-only |
| `IDC_BUTTON_DSMUSIC_UP` | `0x27200044` | playlist reorder — disabled on result tab; wired as inert (#285) |
| `IDC_BUTTON_DSMUSIC_DOWN` | `0x27200045` | playlist reorder — disabled on result tab; wired as inert (#285) |
| `IDC_BUTTON_DSMUSIC_LEFT` | `0x27200046` | result pager — wired (#285) |
| `IDC_BUTTON_DSMUSIC_RIGHT` | `0x27200047` | result pager — wired (#285) |
| `IDC_TEXT_DSMUSIC_PAGENUM` | `0x27400048` | pager caption — display-only, live page (#285) |
| `IDC_BUTTON_DSMUSIC_SEARCH_TITLE` | `0x27201049` | search field radio — wired (#285) |
| `IDC_BUTTON_DSMUSIC_SEARCH_ARTIST` | `0x2720104A` | search field radio — wired (#285) |
| `IDC_EDIT_DSMUSIC_SEARCH` | `0x2730004B` | search text input — wired (#285) |
| `IDC_BUTTON_DSMUSIC_SERACH` | `0x2720004C` | run search — wired (#285) |
| `IDC_IMAGE_DSMUSIC_RANDOM1_BG` | `0x2750004D` | random-panel body — paint-gated (#285) |
| `IDC_BUTTON_DSMUSIC_RANDOM1_ALL` | `0x2720004E` | random-panel body — paint-gated (#285) |
| `IDC_BUTTON_DSMUSIC_RANDOM1_EASY` | `0x2720104F` | random-panel body — paint-gated (#285) |
| `IDC_BUTTON_DSMUSIC_RANDOM1_INTERMEDIATE` | `0x27201050` | random-panel body — paint-gated (#285) |
| `IDC_BUTTON_DSMUSIC_RANDOM1_HIGH` | `0x27201051` | random-panel body — paint-gated (#285) |
| `IDC_BUTTON_DSMUSIC_RANDOM1_HIGHEST` | `0x27201052` | random-panel body — paint-gated (#285) |
| `IDC_BUTTON_DSMUSIC_LABELQUESTION` | `0x27200053` | help button — wired (#285) |
| `IDC_IMAGE_DSMUSIC_LABELGUIDIMG` | `0x27500054` | help sheet body — wired/paint-gated (#285) |
| `IDC_BUTTON_DSMUSIC_SEARCHQUESTION` | `0x27200055` | help button — wired (#285) |
| `IDC_IMAGE_DSMUSIC_SEARCHGUIDIMG` | `0x27500056` | help sheet body — wired/paint-gated (#285) |
| `IDC_IMAGE_DSMUSIC_LISTGUIDIMG` | `0x27500057` | help sheet body — wired/paint-gated (#285) |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX0` | `0x27200058` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX1` | `0x27200059` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX2` | `0x2720005A` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX3` | `0x2720005B` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX4` | `0x2720005C` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX5` | `0x2720005D` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX6` | `0x2720005E` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX7` | `0x2720005F` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX8` | `0x27200060` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX9` | `0x27200061` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX10` | `0x27200062` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX11` | `0x27200063` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX12` | `0x27200064` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX13` | `0x27200065` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX14` | `0x27200066` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX15` | `0x27200067` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX16` | `0x27200068` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX17` | `0x27200069` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX18` | `0x2720006A` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX19` | `0x2720006B` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX20` | `0x2720006C` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX21` | `0x2720006D` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX22` | `0x2720006E` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX23` | `0x2720006F` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX24` | `0x27200070` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX25` | `0x27200071` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX26` | `0x27200072` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX27` | `0x27200073` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX28` | `0x27200074` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX29` | `0x27200075` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX30` | `0x27200076` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX31` | `0x27200077` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX32` | `0x27200078` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX33` | `0x27200079` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX34` | `0x2720007A` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX35` | `0x2720007B` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX36` | `0x2720007C` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX37` | `0x2720007D` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX38` | `0x2720007E` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX39` | `0x2720007F` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX40` | `0x27200080` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_INDEX41` | `0x27200081` | chosung key-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_NEW` | `0x2720B082` | special-search button — interactive, OPEN family |
| `IDC_BUTTON_DSMUSIC_SERACH_SPECIAL1` | `0x2720B083` | special-search button — interactive, OPEN family |
| `IDC_TEXT_DSMUSIC_SERACH_NEW` | `0x27400084` | caption — display-only (live text layer) |
| `IDC_TEXT_DSMUSIC_SERACH_SPECIAL1` | `0x27400085` | caption — display-only (live text layer) |

## Guards

The behavior guards for the wired families live beside the shell's
song-dialog tests and name the live chain from boot staging to
`ScreenGame::click_select_music` / `paint_select_music_art`:

- `select_chrome_inventory_covers_every_bound_control` — the whole
  134-control inventory against the installed control list (skips
  cleanly when CI has no game data).
- `a_mode_switch_click_paints_the_random_panel_instead_of_the_level_buttons`
  — MUSELECT/RANDOMSELECT click through `click_select_chrome` into the
  paint-gated `.bnd` bodies.
- `a_help_button_click_shows_its_guide_sheet_and_hides_the_other` —
  LABELQUESTION/SEARCHQUESTION sheets, one at a time.
- `a_pager_press_moves_the_result_page_and_inks_the_new_caption` —
  LEFT/RIGHT clamp, pager caption ink.
- `a_search_submit_narrows_the_table_by_title_or_artist` — edit-box
  typing, field radios, SERACH submit, status banner.

All four wired-family guards were red-proved by disabling the
`click_select_chrome` call in `ScreenGame::click_select_music`. The
#272 placement guards are
`song_dialog_question_buttons_leave_the_baked_backdrop_plate_visible`
(red: 237 differing pixels in the label `?` rect) and
`song_dialog_inactive_button_bodies_paint_one_state_frame` (red: 4115
differing pixels in the OK body). Both red arms were reproduced by
running the tests before the fix; both now pass.

## #404 placement parity (2026-11-20)

`song_dialog_controls_keep_their_ordinal_paired_rects` extends the
inventory guard: each of the 134 authored `DAILOG_SELECTMUSIC` controls
must stage at the rect an independent ControlList + `.bnd` ordinal parse
names, while `SET` members land at parent origin plus their template
rect. Red-proof: swapping the OK and CANCEL rects in
`bind_entries_by_ordinal` fails the guard; restoring the file passes.
CI skips cleanly when `install-root` has no `Interface1.opi`.

Overlaps that the shipped art authors are not defects: BG plates under
their buttons, the random panel over the level column, the guide sheets
over the sheet, and the two dark `?` plates baked into
`Dialog_SelectMusic.ojs`. The grey `?` button bodies stay unpainted at
open so those plates remain visible;
`song_dialog_question_buttons_leave_the_baked_backdrop_plate_visible`
pins that half. The exe's hover/help-active face for those buttons is
still OPEN. This note pins geometry, not a qualified exe frame.

## OPEN after this leg

The chosung INDEX grid, its two special buttons, the result/playlist
tab switch, the playlist sort/selection family and the UP/DOWN reorder
(not offered on the result tab) are inventoried but not wired. The
next evidence step is a static walk of the keyed rebuild at VA
`0x444630` plus a rig sweep of the playlist tab.

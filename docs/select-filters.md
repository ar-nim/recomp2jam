# Song-dialog filter columns (7.55)

The `DAILOG_SELECTMUSIC` search table narrows by three filter columns.
Ticket #273 (owner playtest 2026-11-16, item 5: "level filter, genre
filter, and key filter (all/gold/black/blue) do nothing").

Evidence class: static pins only. Binary `install-root/O2Jam/AngelO2Jam.exe`,
5029888 B, sha256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`;
tools GNU objdump 2.47 (`-d -M intel`) and Python 3.14.9 for the
`OJNList.dat` word dumps. File offset = VA − `0x400000`. No Wine, no
display, no capture is cited anywhere below (docs/reversing.md §6).

## 1. Buttons, families, state words

The dialog carries three filter columns of bound OPI buttons
(`Image/Interface1.opi`, block `DAILOG_SELECTMUSIC`):

| column | control ids | codes | base | state word | count |
| --- | --- | --- | --- | --- | --- |
| level | `IDC_BUTTON_DSMUSIC_STEP1_{ALL,EASY,INTERMEDIATE,HIGH,HIGHEST}` | `0x2720101C..0x27201020` | `0x2720101C` | dialog `+0x6BC` | 5 |
| label | `IDC_BUTTON_DSMUSIC_STEP2_{ALL,GOLD,BLACK,BLUE,RED,SILVER}` | `0x2720B021..0x2720B026` | `0x2720B021` | dialog `+0x6C0` | 6 |
| genre | `IDC_BUTTON_DSMUSIC_STEP3_{ALL,BALLAD,ROCK,DANCE,HIPHOP_SOUL,JAZZ_FUNK,CLASSIC,ETC}` | `0x2720B027..0x2720B02E` | `0x2720B027` | dialog `+0x6C4` | 8 |

Each click handler stores `code − base` in its column's state word and
reruns the filter chain at once: level store VA `0x43F6C2` (sub
`0x43F6B6`) then `0x446950` at `0x43F6F7`; label store VA `0x43F7EE`
(sub `0x43F7E3`) then `0x447050` at `0x43F823`; genre store VA
`0x43F91B` (sub `0x43F90F`) then `0x447730` at `0x43F950`. Afterwards
each handler lights its own radio group through `0x433AF0` (count, base,
clicked code).

The dialog constructor writes the "no choice" sentinels 5 / 6 / 8
(VAs `0x43D324` / `0x43D331` / `0x43D33E`). No button reaches them, so
they only describe the freshly opened dialog.

## 2. What each column tests

The chain filters inline `0x238`-byte select entries. One entry is one
chart: song record + chosen difficulty. Its filter inputs:

- `entry+0x212` — the chart level of that difficulty (the builder reads
  `u16[difficulty]` over the record's three level words, VA `0x444268`;
  ev `b5c3094a…`, see docs/file-formats/ojn.md "Difficulty ↔ slice").
- `entry+0x214` — the genre id (low half of the record's genre word,
  VA `0x444277`).
- `entry+0x4` — the `OJNList.dat` table-5 label bucket 1..5 (VA
  `0x44422E`).

**Level bands** (fn VA `0x446950`, 5-arm table VA `0x447038`). Signed
compares on the u16 level:

| button | rule |
| --- | --- |
| ALL | keeps every entry |
| EASY (초급) | `1 ≤ level < 8` |
| INTERMEDIATE (중급) | `8 ≤ level < 19` |
| HIGH (고급) | `19 ≤ level < 27` |
| HIGHEST (최고급) | `level ≥ 27` |

Arm code: copy-all `0x446B81`; bands at `0x446BA0`, `0x446BFF`,
`0x446C5E`, `0x446CBA`. A value above 4 (`0x446B67`) copies nothing.

**Label buckets** (fn VA `0x447050`, 6-arm table VA `0x447718`, gate
`> 5` at `0x447274`). ALL copies; arm `n` keeps `entry+0x4 == n`
(compares VA `0x4472C1`/`0x4472FA`/`0x447333`/`0x447369`/`0x44739F`).
Button `n` is bucket `n`: GOLD 1, BLACK 2, BLUE 3, RED 4, SILVER 5. The
shipped file fills buckets 1..5 with 78/82/157/52/193 songs
(`middle table 5`, `docs/file-formats/ojn.md`).

**Genre** (fn VA `0x447730`, 8-arm table VA `0x447E50`, gate `> 7` at
`0x447958`). Arms test `entry+0x214`:

| button | genre ids kept |
| --- | --- |
| ALL | every genre |
| BALLAD | 0 |
| ROCK | 1 |
| DANCE | 2, 3, 9 |
| HIPHOP_SOUL | 4, 5 |
| JAZZ_FUNK | 6, 7 |
| CLASSIC | 8 |
| ETC | 10 |

Arm code: `0x447972` (all), `0x447991`, `0x4479CF`, `0x447A0E`,
`0x447A8D`, `0x447AEC`, `0x447B48`, `0x447B84`. The shipped catalog
uses ids 0..10 with all 11 values present (562 songs, static count from
`OJNList.dat` record +16).

## 3. Composition

Master list `+0x640` → level `+0x650` → label `+0x660` → genre `+0x670`.
Each stage clears its own output (`0x452A00`) and appends only the
entries its predicate keeps (`0x452850`), so the columns compose as a
logical AND in the order level → label → genre.

Two further pinned points:

- **Cascade reset.** A click (4th argument 0) clears the columns chained
  *after* the clicked one: the level function stores 0 (ALL) into
  `+0x6C0` at VAs `0x446D08`..`0x446D14`, the label function stores 0
  into `+0x6C4` at VAs `0x4473D3`..`0x4473DF`. A genre click clears
  nothing. Filters therefore combine in click order, level first.
- **Sentinel skip.** When the label state is 6 the level function skips
  the label stage (VA `0x446D06`) and builds the auxiliary search-key
  list `+0x680` plus the status banner instead; when the genre state is
  8 the label function does the same at VA `0x4473CA`.

The banner confirms the chain length: one name for a level-only search
(VA `0x645A50`), two (`0x645A64` level → label), three (`0x645A80`
level → label → genre), filled from the name rows `0x645524` (전체/초급/
중급/고급/최고급), `0x6455A0` (전체/신곡/골드/블랙/블루/레드/실버) and
`0x6455BC` (전체/발라드/락/댄스/힙합,R&B/재즈,펑크/클래식/기타).

## 4. What the client implements

`o2jam-shell`:

- `SelectLevelBand`, `SelectLabelFilter`, `SelectGenreFilter` carry the
  rules above; `SelectMusicFilter` holds one choice per column and
  `SelectMusicFilter::choose` applies the cascade reset.
- `select_music_rows(songs, filter)` narrows the search table; the text
  layer and the row pick read the same call, so a click on a filtered
  row picks the filtered song.
- `select_filter_choice_at(placements, x, y)` maps a staged dialog click
  to its choice by control id and `.bnd` rect (owner rule: geometry from
  the parsed `.bnd`, never literals).
- The filter state resets to `ALL` whenever the dialog opens (the ctor
  sentinels).
- Label buckets come from `o2jam_file::parse_ojn_list_labels` (middle
  table 5 of the same `OJNList.dat`), staged through
  `ScreenGame::set_song_catalog_with_labels`.

Guards (ticket #273): `level_bands_keep_their_pinned_edges`,
`label_buttons_keep_their_own_bucket_only`,
`genre_buttons_keep_their_pinned_id_sets`,
`select_music_rows_narrow_per_filter_column`,
`select_filters_compose_as_and_in_click_order`,
`select_filter_click_clears_the_columns_chained_after_it`,
`select_filter_choice_maps_the_staged_buttons`,
`waiting_select_music_filter_click_narrows_the_search_table`,
`song_dialog_level_filter_click_narrows_the_live_catalog`,
`ojn_list_labels_split_the_whole_file_at_the_record_boundary`
(seam: pure parser split at `OJN_MID_REGION_OFFSET`).

## 5. OPEN

- **Banner text.** Our status line still names the ALL path. The exe
  fills the banner from the STEP1/STEP2/STEP3 name rows; the STEP2 row
  is off by one against its buttons (GOLD banners `신곡`), which stays
  OPEN inside claim C20 (`docs/xtd-table.md`). Rendering the banner
  needs that mismatch settled first.
- **Row highlight.** The pressed-button state of a reset column is read
  at VA `0x44FAD2` (`+0x6BC` against the sentinel 5). Asserted here as
  "reset shows unselected"; unproven against pixels.
- **Row-painter list source.** Whether the table paints from `+0x670`
  (fully filtered) or the auxiliary `+0x690` list while no column is
  chosen. Next step: the painter's list argument at the row loop.

# Journey audit — #179 Music Shop

Comparison leg, no exe boots and no fixes. Base `2105ac64eb3504b4c00494c54ff98e0b1b38f1cb`;
worktree branch `cb2/ja-d`. Stories are the 18 checkbox items in `gh issue view 179`.

## Evidence anchors (artifact + hash)

| ID | Artifact | Hash |
|---|---|---|
| EXE-SCRATCH-MSHOP | `~/scratch/side-scenes/b1-explore/explore/d03-musicshop.png` and `h04-musicshop-long.png` (room → music shop, visible song list, `G 15527` / `W 0`, download panel). **No manifest/citation in that pack**: scratch pixels only, not exe proof for a mismatch claim | `1f4c420767a86c0db4f26175beb268b8b9517461db5e411c38308b54c4ee4d76` / `964be986ee1fa269306ad2115dfdb18efc1cc511e5dfa765e5845698bb8ee8aa` |
| EXE-PARENT | Producer exe pin reused from the qualified b8 pack: `OTwo.exe` 7.55, 5,029,888 B | `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`; pack manifest `f9d4a6953c55b75b4f04db3c0ece198bd2032361ac65dc8275ce07ccb6aa3f8b` |
| RUST-NOW | `recomp2jam shots` at base commit, `musicshop-native.png` (scenario in `docs/captures/t207/music-shop.md`: real `OJNList.dat` 562 songs, synthetic 4031 flags/cart) | `621caa264d65bb0270c39e649583b72e56b1e96bf522bf5c1df4234da559d727` |
| STATIC | `docs/musicshop.md`, `docs/parity/musicshop.md`, `docs/shop-buy.md` (send switch file `0xa4df0` index 8 for 5010, `0x1A20007D..84` TOP100 arm) | source pins; no pixels claimed |

Comparators: `pixel` = pixel-region, `wire` = wire-sequence,
`input` = input-response, `state` = state-path, `data` = data-effect,
`NA` = NA-DESIGN.

## Per-story table

| # | Story (short) | Comparator | Our journey ref | Exe evidence | Verdict | Notes |
|---|---|---|---|---|---|---|
| 1 | Song board renders catalog with names | data / pixel | `set_song_catalog` → `music_shop_songs`/`music_shop_listings` (`side.rs`) → `paint_music_shop_board`; guard `music_shop_board_paints_catalog_cells_and_pager_at_staged_rects` | STATIC `docs/musicshop.md` row slots `0x1A01802D..31`; scratch d03 shows a populated song list | MATCHED | Source class matches; scratch pixels not used as proof, and exact names are fixture data |
| 2 | Per-song price renders gem vs won | pixel / data | GEM/WON price cells stay blank (no price source), `music_shop_rows`; RUST-NOW shows empty price cells | STATIC `0x40`/`0x45`, money texts `0x1A400019..021` OPEN; scratch h04 shows bottom wallet words only | BLOCKED | Map #245/#249/#250 + #345; no qualified capture or 5011 price pin |
| 3 | Owned songs marked | pixel / data | `shop_flags_for` from staged 4031 answer; `music_shop_buy_caption` → `OWNED`/`FREE`; guard `music_shop_listings_carry_live_free_flags` | STATIC 4031 flag pin; scratch h04 shows two `Free` rows; source ASSUMED in spec | BLOCKED | Map #245 + #345; no qualified exe/our paired frame |
| 4 | DIFF view | pixel | DIFF cell `0x3C` staged but unpainted; `docs/captures/t207/music-shop.md` explicitly records the ink gap | STATIC DIFF rect pin, values OPEN | BLOCKED | Map #245/#345; needs a known-song exe capture |
| 5 | Pick song into cart and buy | input | `music_shop_row`, tiled buy dispatch; guard `music_shop_row_pick_stages_the_cart_and_cart_buy_queues_5010` (`game/mod.rs:16908`) | STATIC `MS_EX` and tiled `GEM_BUY`/`WON_BUY` codes; no exe press capture | BLOCKED | Map #245/#249/#250; seam-green only |
| 6 | Purchase completes against server | wire | `SyncMusicPurchase` 5010 send; scripted 5011 decode/fold in `session.rs:2347-2370`, test `music_buy_round_trips_through_scripted_gateway` (`session.rs:4047`) | STATIC 5010 send switch file `0xa4df0` index 8; 5011 reply shape untraced in exe/live server | BLOCKED | Map #345; our 5011 struct is an assumption, not a pin |
| 7 | Bought song appears in song select | state / wire | `apply_music_purchase_response` → `MusicShopState::fold_purchased_songs` + `select_songs` (scripted test at `session.rs:4135+`) | No live buy→select capture | BLOCKED | Map #345; state join exists but the trigger wire is unpinned |
| 8 | Sorts reorder the board | input | `LankingSort` dispatch, guard `music_shop_sorts_pages_and_buys_5010`; directions ASSUMED | STATIC `0x1A201010..15` / sort code pins; no exe press capture | BLOCKED | Map #245/#249/#250; direction must be pinned from exe |
| 9 | Pager walks catalog | input | `music_shop_page_rects` + guard `music_shop_sort_and_pager_answer_their_staged_buttons`; pairing ASSUMED | STATIC `MSALL` codes; no exe turn capture | BLOCKED | Map #245/#249/#250 |
| 10 | Insufficient currency error surfaced | wire | No failure arm | No 5011 failure shape pinned | BLOCKED | Map #345 |
| 11 | Browse→cart→buy guarded at `ScreenGame` seam | NA | Guards named in rows 5, 8, 9 | STATIC send pin only | NA-DESIGN | Test-authoring story; rules note missing live-chain annotations |
| 12 | Scripted-gateway round trips, success + failure | wire | Success: `music_buy_round_trips_through_scripted_gateway`; failure: no arm | Reply shape synthetic | BLOCKED | Map #345; helper exists, pin does not |
| 13 | TOP100 ownership pinned to music shop | NA | `MSHOP_TOP100_CODE` owned here; #181 pin `0x1A20007D..84` | STATIC #181 | NA-DESIGN | Research/ownership story, already closed |
| 14 | TOP100 board implemented at pinned rects | pixel / state | No board; rects staged only as art | STATIC rect pin; board content unevidenced (`docs/musicshop.md:91`) | BLOCKED | Map #245/#345; needs exe capture of the board before paint |
| 15 | Bought song downloaded if not on disk | data / wire | `MusicDownload` (FTP pump) sends 5012 on completion (`o2jam-shell/src/music_download.rs`, `o2jam-net/src/play.rs:865`); no shop-side purchase trigger | STATIC opcode 5012 in `docs/net-protocol.md:104,134`; no live shop→download capture | BLOCKED | Map #345; do not equate the post-start pump with the shop arm |
| 16 | DIFF/money rects bound to pinned sources | pixel / data | Rects staged, cells blank (`docs/captures/t207/music-shop.md` ink gap) | STATIC DIFF `0x3C`, GEM/WON `0x40`/`0x45`, money texts `0x1A400019..021` | BLOCKED | Map #245/#345; qualified capture + 5011 price words first |
| 17 | 5012 download controls staged or labeled inert | input / wire | Codes `0x1A20000E..0F` have no Rust dispatch and are not in `docs/chrome-wiring.md`'s music-shop table; 5012 exists only as the completion announce | STATIC code/opcode pin; no exe press shape | BLOCKED | Map #245; currently neither staged nor explicitly labeled inert in the wiring table |
| 18 | Legacy price words + 5010 round trip | wire / pixel | Same as rows 2 and 6 | Same as rows 2 and 6 | BLOCKED | Map #345 + #245 |

## Summary matrix — #179

| Verdict | Count | Stories |
|---|---:|---|
| MATCHED | 1 | 1 |
| MISMATCHED | 0 | — |
| BLOCKED | 15 | 2–10, 12, 14–18 |
| NA-DESIGN | 2 | 11, 13 |

New issues: **none**. Scratch music-shop pixels are unqualified (no pack manifest/citation),
so no MISMATCHED claim can be filed from them; every other divergence maps to #245/#249/#250
or #345.

### BLOCKED list for the live lane

- Qualified exe music-shop pack (manifest + citation + item shop equivalent of b8): price/DIFF/money cells, owned flags, TOP100 board, sort/pager directions, 5012 control semantics → #245/#249/#250. Reuse of b1-explore scratch PNGs is not sufficient.
- Live 5011 reply pin (prices, failure verdict, owned list) and 5012 request/reply pin → #345 (closed; continuity caveat).
- Paired Rust capture with the same catalog/account state as the exe → #245.

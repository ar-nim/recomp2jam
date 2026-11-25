# Statics pins — Item Shop, Music Shop and MyRoom (coverage burn-down)

Version pin: `OTwo.exe` 7.55, 5,029,888 B, SHA-256
`83a0.4541.8b62.19ba.3406.8176.6b33.de2b.fbb5.ff51.b813.ef4e.2e9d.d1de.f72c.823b`.
Provider: REA 6.10.1 / Ghidra 12.2; target bound from
`install-root/O2Jam/OTwo.exe`. Pre-flight `git rev-parse --show-toplevel`
resolved to `/home/arnim/Projects/recomp2jam-lane-pins-shops-myroom` on branch
`cb2/pins-shops-myroom`. Brief base `a15244e1cebcc72a16f8d9d84401ccab895343f2`
is an ancestor of branch HEAD `e5e4fb7f918752fd12e49ab606e6bc07db7f5c7c`.

Scope: the 15 addresses in
`~/scratch/lanes-2026-11-25/slice-shops-myroom.txt`, all with
`bucket=game-reachable,pinned_in_repo=no` in `procedures.csv`. Pinned here:
**15/15**; left: **0**. Triage labels are address-cluster or class-vftable
hints, not runtime ownership; corrections are recorded below.

Evidence notation: an evidence ID is `ev ` plus the hex in four-digit groups;
remove the dots and join after `ev_` to obtain the full REA ID. The SHA-256
uses the same grouping. Grouping keeps `scripts/check-cited-hashes.sh` from
reading evidence hex as a commit citation. File offsets were checked with
REA `address_to_file_offset` (PE image base `0x400000`); each is the VA minus
the image base. Behavior is described in our own words; no disassembly or
decompiled text is reproduced.

Prior pins are cited, not re-derived: `docs/shop.md` (5001 restock handler
`0x560a40`, cart and chrome), `docs/myroom.md` (`CPlayerInfo`, gift box,
status plate), `docs/lifecycle.md` §8 (`CCBtnSelectMusic` in Waiting's
select-music dialog), `docs/pins/room-waiting.md` (shared byte-vector family),
`docs/pins/planet-channel.md` (shared control factories), and
`docs/interaction-census.md` (shop, select-music and MyRoom rows).

## Inventory

| VA | File offset | Triage label | Role | Analysis evidence |
|---|---|---|---|---|
| `0x4a9da0` | `0x9da0` | Item Shop / wallet (cluster) | Insert twelve-byte records (support) | ev 0eb0.4020.93f5.df40.8879.6f1b.b3b5.57c4.7dee.3582.0416.e157.531f.4dcf.cce6.0738 |
| `0x4aa1e2` | `0xaa1e2` | Item Shop / wallet (cluster) | Twelve-byte insert epilogue (support) | ev f54e.b553.684a.cf67.665d.834b.c95b.f515.340f.f3d8.c7d3.6eaa.2c4b.d8b4.e98a.bb62 |
| `0x4aa730` | `0xaa730` | Item Shop / wallet (cluster) | Fill twelve-byte records (support) | ev c015.f22d.d1f2.b101.40df.1177.7a9e.b3a5.082c.ee96.191d.34fe.d15f.457b.c885.b19b |
| `0x4aa7e9` | `0xaa7e9` | Item Shop / wallet (cluster) | Fill unwind fragment (support) | ev 65b0.c6e1.e891.4b4c.79d0.4ddc.c463.caf7.4422.8c0b.03d3.a71f.f973.9a56.0888.9cdc |
| `0x4aa930` | `0xaa930` | Item Shop / wallet (cluster) | Copy-range wrapper (support) | ev 5cdb.4639.7671.a6ff.b418.03fd.e77f.23ef.cdad.88fd.dad8.4415.ce90.05ce.a973.cab7 |
| `0x4aa980` | `0xaa980` | Item Shop / wallet (cluster) | Copy twelve-byte records (support) | ev 2710.44ab.0c68.37f4.036b.017d.7762.cfc1.e24e.4283.37e9.3e37.2913.6730.4540.99aa |
| `0x4aaa3b` | `0xaaa3b` | Item Shop / wallet (cluster) | Copy unwind fragment (support) | ev b30c.7e81.fb34.da4c.9a25.572b.60b8.6b47.064f.4600.13b6.9f24.5f83.6e65.c63d.f46f |
| `0x4afb40` | `0xafb40` | Item Shop / wallet (cluster) | Insert bytes with fill value (support) | ev ec9f.733d.f2bd.ca21.a25e.ec3d.ae6a.8860.cd5f.a764.a60d.edc1.87de.2416.eb65.cf44 |
| `0x4aff17` | `0xaff17` | Item Shop / wallet (cluster) | Byte insert epilogue (support) | ev da27.053d.c7dc.3912.2395.98b7.334e.9c5c.6149.93ac.d48d.eb59.d4b5.aefa.ba7d.34ba |
| `0x51ea80` | `0x11ea80` | Item Shop / wallet (`CCBtnGiftList`) | Gift-list button construction | ev 89b2.ec7d.8804.6581.3e62.b445.a7cd.f37d.a423.ae06.1ffe.70d1.05e9.1257.3bb1.3e11 |
| `0x5238f0` | `0x1238f0` | Music Shop (`CCBtnMShopRList`) | Music-shop rank-list button construction | ev a557.263f.fe3c.7f60.7607.4887.9ec4.c0c1.6e42.070d.daa8.3d93.197d.c7e3.4970.f1a9 |
| `0x526df0` | `0x126df0` | Music Shop (`CCBtnSelectMusic`) | Select-music button construction | ev 6457.8252.c5e3.2c4d.ff36.3df2.cfa6.df16.d460.0f1d.0f76.c415.416f.63b6.61f8.cb9d |
| `0x5296e0` | `0x1296e0` | Item Shop / wallet (`CCBtnShopCart`) | Shop-cart button construction | ev 2620.8ffd.3e35.ae1c.97b8.6cbb.1065.d864.664c.c244.52cc.f97e.4050.38cf.cd04.012b |
| `0x53f9e0` | `0x13f9e0` | Avatar / player info (`CPlayerInfo`) | Player-info object construction | ev 3a28.f011.0479.3d97.5984.a1b5.d7f9.f0bc.9f69.1ecd.7708.a1ae.2eba.c04d.293d.df77 |
| `0x53fa60` | `0x13fa60` | Avatar / player info (`CPlayerInfo`) | Player-info vftable reinstall (support) | ev dedd.caff.887f.6122.e0af.e5f7.b3d6.065a.fe9c.9895.45fe.cca1.33b3.5f5d.4078.c610 |

## Entries

### `0x4a9da0` — insert a counted run of twelve-byte records (support)

- **VA / file offset**: `0x4a9da0` / `0x9da0` (695,712).
- **Behavior (own words)**: copies three supplied words into the three-record
  scratch block, checks the destination vector's remaining capacity for the
  requested count, grows storage through the allocator when needed, moves the
  existing tail, writes the supplied triple into the new slots, and finishes
  in the insert epilogue. An in-place arm moves the tail back and fills the
  freed slots directly.
- **Reach and player-visible role**: support. Sole direct caller is
  `0x4a9c70`; the observed chain continues `0x4a9b60` → `0x4a54e0`, `0x55a350`,
  `0x560a40` (Item Shop 5001 handler), `0x563b20`. It stores twelve-byte rows
  for those consumers; it emits no pixels or wire itself.
- **Our client**: no counterpart by VA. `Vec<ShopGood>` and `Vec<u32>` in
  `ShopState` (`crates/o2jam-shell/src/screens/game/shop.rs:61`, `:71`) supply
  the same role under Rust ownership rules.
- **Evidence**: analysis ev
  0eb0.4020.93f5.df40.8879.6f1b.b3b5.57c4.7dee.3582.0416.e157.531f.4dcf.cce6.0738;
  incoming call sites ev
  0042.b79f.0630.3cf7.5c21.6919.30d2.911f.489a.c9e7.a5b1.314f.4c93.9863.f45d.5111
  (observed direct caller: `0x4a9c70`).

### `0x4aa1e2` — finish a twelve-byte insert by rebasing storage (support)

- **VA / file offset**: `0x4aa1e2` / `0xaa1e2` (696,802).
- **Behavior (own words)**: adds the inserted count to the saved destination
  pointer, frees the old vector block when one exists, then recomputes and
  writes the vector's begin, end and capacity words from the saved base and
  the two advanced pointers.
- **Reach and player-visible role**: support; sole caller is `0x4a9da0`. It
  keeps storage pointers consistent after the insert, with no pixels or wire.
- **Our client**: no counterpart by VA; Rust `Vec` growth owns this work.
- **Evidence**: analysis ev
  f54e.b553.684a.cf67.665d.834b.c95b.f515.340f.f3d8.c7d3.6eaa.2c4b.d8b4.e98a.bb62;
  incoming call sites ev
  457f.47d0.a9b0.8530.7e2d.7efe.363a.93c4.b25a.9a7c.2acc.2d23.615b.a4a8.1e95.8538
  (observed direct caller: `0x4a9da0`).

### `0x4aa730` — fill a counted run of twelve-byte records (support)

- **VA / file offset**: `0x4aa730` / `0xaa730` (698,160).
- **Behavior (own words)**: for a caller-supplied count, writes the three
  words of the supplied record into each successive twelve-byte slot,
  skipping a null destination, then returns through the fill epilogue.
- **Reach and player-visible role**: support. Callers are `0x4a9da0` (insert)
  and `0x4a9b60` (row-storage wrapper). No pixels or wire of its own.
- **Our client**: no counterpart by VA; `Vec::resize`-style fill in Rust.
- **Evidence**: analysis ev
  c015.f22d.d1f2.b101.40df.1177.7a9e.b3a5.082c.ee96.191d.34fe.d15f.457b.c885.b19b;
  incoming call sites ev
  f7c8.5174.b7c5.9fd9.1c54.9938.5920.6db7.84d5.3e0d.ef24.7d46.339b.9630.7e29.f300
  (observed direct callers: `0x4a9b60`, `0x4a9da0`).

### `0x4aa7e9` — restore the exception list after a fill (support)

- **VA / file offset**: `0x4aa7e9` / `0xaa7e9` (698,345).
- **Behavior (own words)**: tiny epilogue fragment: restores the saved
  exception-registration pointer and returns.
- **Reach and player-visible role**: support; sole caller is `0x4aa730`.
  Ghidra reports this as a fragment; its role comes from caller flow, not
  from runtime observation. No pixels or wire.
- **Our client**: no counterpart by VA; no Rust equivalent is needed.
- **Evidence**: analysis ev
  65b0.c6e1.e891.4b4c.79d0.4ddc.c463.caf7.4422.8c0b.03d3.a71f.f973.9a56.0888.9cdc;
  incoming call sites ev
  e4e6.2ad9.c874.c159.22b0.9479.2bfc.085d.0634.03ac.3d82.b803.65c2.7244.254a.da68
  (observed direct caller: `0x4aa730`).

### `0x4aa930` — forward a twelve-byte range copy (support)

- **VA / file offset**: `0x4aa930` / `0xaa930` (698,672).
- **Behavior (own words)**: forwards all four arguments unchanged to the
  range-copy helper `0x4aa980` and returns.
- **Reach and player-visible role**: support; sole caller is `0x4a9da0`,
  which uses it to move the existing tail during an insert. No pixels or wire.
- **Our client**: no counterpart by VA; Rust slice copy handles this.
- **Evidence**: analysis ev
  5cdb.4639.7671.a6ff.b418.03fd.e77f.23ef.cdad.88fd.dad8.4415.ce90.05ce.a973.cab7;
  incoming call sites ev
  d8f8.b87c.bfe9.b282.8b17.82f6.b5db.532e.f844.ef47.19f1.9344.e33c.8eb2.b08a.9d20
  (observed direct caller: `0x4a9da0`).

### `0x4aa980` — copy a range of twelve-byte records (support)

- **VA / file offset**: `0x4aa980` / `0xaa980` (698,752).
- **Behavior (own words)**: walks from the source begin pointer to the
  caller-supplied end pointer, copying each three-word record into the
  destination pointer (when non-null) and advancing it, then returns through
  the copy epilogue.
- **Reach and player-visible role**: support. Callers are `0x4aa930` (wrapper)
  and `0x4aa810` (container code). No pixels or wire.
- **Our client**: no counterpart by VA; Rust `copy_from_slice` idiom.
- **Evidence**: analysis ev
  2710.44ab.0c68.37f4.036b.017d.7762.cfc1.e24e.4283.37e9.3e37.2913.6730.4540.99aa;
  incoming call sites ev
  29cc.7aad.a830.1a8c.3b41.5a91.a90d.4fea.08b8.c8ea.c250.a8bd.f1af.5d57.97b0.1625
  (observed direct callers: `0x4aa810`, `0x4aa930`).

### `0x4aaa3b` — restore the exception list after a copy (support)

- **VA / file offset**: `0x4aaa3b` / `0xaaa3b` (698,939).
- **Behavior (own words)**: tiny epilogue fragment: restores the saved
  exception-registration pointer and returns the saved destination word.
- **Reach and player-visible role**: support; sole caller is `0x4aa980`.
  Ghidra reports this as a fragment; its role comes from caller flow. No
  pixels or wire.
- **Our client**: no counterpart by VA; no Rust equivalent is needed.
- **Evidence**: analysis ev
  b30c.7e81.fb34.da4c.9a25.572b.60b8.6b47.064f.4600.13b6.9f24.5f83.6e65.c63d.f46f;
  incoming call sites ev
  c024.81f3.9cf5.06e4.2a16.9fea.85d4.e55b.355c.ca46.ed95.a662.79cd.b17b.ce4b.0185
  (observed direct caller: `0x4aa980`).

### `0x4afb40` — insert counted bytes with a fill value (support)

- **VA / file offset**: `0x4afb40` / `0xafb40` (719,680).
- **Behavior (own words)**: reads one fill byte from the supplied source,
  checks byte-vector capacity for the requested count, grows through the
  allocator when needed, moves the existing tail, fills the new slots with
  that byte, and finishes in the byte insert epilogue. An in-place arm moves
  the tail down and fills the freed slots directly.
- **Reach and player-visible role**: support and shared. Sole direct caller is
  `0x4afa20` (byte-vector insert), reached from `0x4af920` and its consumers;
  the byte helpers `0x4b0470`/`0x4b04e0` pinned in
  `docs/pins/room-waiting.md` are the fill/move arms used here, so this body
  is not Item-Shop-specific despite its triage cluster. No pixels or wire of
  its own.
- **Our client**: no counterpart by VA. Rust `Vec<u8>` growth and the waiting
  chat ring (`crates/o2jam-shell/src/screens/game/chat.rs:399`) own this role.
- **Evidence**: analysis ev
  ec9f.733d.f2bd.ca21.a25e.ec3d.ae6a.8860.cd5f.a764.a60d.edc1.87de.2416.eb65.cf44;
  incoming call sites ev
  5134.97b9.d588.40e6.d424.366f.3f96.880f.f39b.c8b6.0c7b.2037.9c8d.136f.080f.bb12
  (observed direct caller: `0x4afa20`).

### `0x4aff17` — finish a byte insert by rebasing storage (support)

- **VA / file offset**: `0x4aff17` / `0xaff17` (720,663).
- **Behavior (own words)**: adds the inserted byte count to the saved
  destination pointer, frees the old vector block when one exists, then
  recomputes and writes begin, end and capacity from the saved base and the
  advanced pointers.
- **Reach and player-visible role**: support; sole caller is `0x4afb40`.
  Byte-sized counterpart of `0x4aa1e2`; no pixels or wire.
- **Our client**: no counterpart by VA; Rust `Vec` growth owns this work.
- **Evidence**: analysis ev
  da27.053d.c7dc.3912.2395.98b7.334e.9c5c.6149.93ac.d48d.eb59.d4b5.aefa.ba7d.34ba;
  incoming call sites ev
  b3e1.9cf6.2543.2879.1d49.b880.c254.903e.21de.ebde.fc91.a4d9.b451.9bd1.3a6b.7638
  (observed direct caller: `0x4afb40`).

### `0x51ea80` — construct a gift-list button (`CCBtnGiftList`)

- **VA / file offset**: `0x51ea80` / `0x11ea80` (1,175,552).
- **Behavior (own words)**: installs the structured exception frame, calls
  the shared button-base initializer `0x4f6040`, installs the
  `CCBtnGiftList` vftable, clears two 36-byte blocks starting at `+0x64`,
  calls the `0x403db0` helper with two zero arguments, clears the words at
  `+0x8C` and `+0x90`, then restores the exception list and returns the
  object pointer.
- **Reach and player-visible role**: player-visible class identity, same
  shape as the pinned `CCBtnPlanet` constructor in
  `docs/pins/planet-channel.md`. Direct callers are the control factory and
  host/slot loaders (`0x4329b0`, `0x48a620`, `0x48b2d0`), so staged
  gift-list buttons get this behavior. Which screen hosts the class (Item
  Shop GiftView versus MyRoom gift box) stays OPEN: no boot-to-control chain
  was observed in this static lane.
- **Our client**: no native button object. Gift rows are staged and painted
  from `myroom_gift_views` (`crates/o2jam-shell/src/screens/side/myroom.rs:846`)
  with the pager guard `myroom_gift_pager_wraps_left_clamps_right_and_clears_selection`
  (`crates/o2jam-shell/src/screens/side.rs:8783`, Tier 0).
- **Evidence**: analysis ev
  89b2.ec7d.8804.6581.3e62.b445.a7cd.f37d.a423.ae06.1ffe.70d1.05e9.1257.3bb1.3e11;
  incoming call sites ev
  08a5.aa74.6263.037a.14d3.68dd.fd83.6e8f.a252.cbc6.978b.8b18.e5a5.d78a.797a.f95c
  (observed direct callers: `0x4329b0`, `0x48a620`, `0x48b2d0`); the vftable
  symbol `CCBtnGiftList::vftable` resolves to `0x64c3d8`.

### `0x5238f0` — construct a music-shop rank-list button (`CCBtnMShopRList`)

- **VA / file offset**: `0x5238f0` / `0x1238f0` (1,194,224).
- **Behavior (own words)**: calls the shared button-base initializer
  `0x4f6040`, installs the `CCBtnMShopRList` vftable, clears the words at
  `+0x64` and `+0x70`, the byte at `+0x6B` and the word at `+0x68`, then
  returns the object pointer.
- **Reach and player-visible role**: player-visible class identity: the
  Music Shop right rank-panel buttons are built from this class. Direct
  callers are the shared control factory and host/slot loaders
  (`0x4329b0`, `0x48a620`, `0x48b2d0`); it installs behavior but does not
  hit-test or paint itself.
- **Our client**: no native button object. Staged music-shop controls are
  dispatched by `music_shop_action_at`
  (`crates/o2jam-shell/src/screens/side.rs:2990`); Tier 0 guard
  `music_shop_sort_and_pager_answer_their_staged_buttons`
  (`side.rs:8287`). Whether the rank panel's own press arm needs its own
  guard stays OPEN.
- **Evidence**: analysis ev
  a557.263f.fe3c.7f60.7607.4887.9ec4.c0c1.6e42.070d.daa8.3d93.197d.c7e3.4970.f1a9;
  incoming call sites ev
  8fb0.c9f9.310d.b682.77db.984c.a47c.a549.1e17.17a6.0551.5f8c.ce7b.6449.2f5e.fa19
  (observed direct callers: `0x4329b0`, `0x48a620`, `0x48b2d0`); the vftable
  symbol `CCBtnMShopRList::vftable` resolves to `0x64c52c`.

### `0x526df0` — construct a select-music button (`CCBtnSelectMusic`)

- **VA / file offset**: `0x526df0` / `0x126df0` (1,207,792).
- **Behavior (own words)**: calls the shared button-base initializer
  `0x4f6040`, installs the `CCBtnSelectMusic` vftable, writes `0xFFFFFF` to
  the word at `+0x74`, and clears the words at `+0x78` through `+0x8C` and
  at `+0x94` and `+0x98`, then returns the object pointer.
- **Reach and player-visible role**: player-visible class identity.
  Triage filed it under Music Shop by class name, but
  `docs/lifecycle.md` §8 names `CCBtnSelectMusic` as the button class of
  Waiting's `DAILOG_SELECTMUSIC` (decl `0x7A`), not of `STATE_MYSHOP`; the
  contradiction is recorded rather than silently resolved. Direct callers are
  the control factory and host/slot loaders (`0x4329b0`, `0x48a620`,
  `0x48b2d0`). The class installs behavior; it does not choose a song.
- **Our client**: no native button object. Staged select-music controls are
  filtered by `SelectMusicFilter` (`crates/o2jam-shell/src/screens/side.rs:3579`)
  and guarded by `select_music_rows_narrow_per_filter_column`
  (`crates/o2jam-shell/src/screens/side.rs:9187`, Tier 0).
- **Evidence**: analysis ev
  6457.8252.c5e3.2c4d.ff36.3df2.cfa6.df16.d460.0f1d.0f76.c415.416f.63b6.61f8.cb9d;
  incoming call sites ev
  561c.155a.e10b.6931.80da.aee8.1487.6e5e.d2b9.0008.4c1c.41e0.4a3b.c4cc.1abd.177e
  (observed direct callers: `0x4329b0`, `0x48a620`, `0x48b2d0`); the vftable
  symbol `CCBtnSelectMusic::vftable` resolves to `0x64c658`.

### `0x5296e0` — construct a shop-cart button (`CCBtnShopCart`)

- **VA / file offset**: `0x5296e0` / `0x1296e0` (1,218,272).
- **Behavior (own words)**: calls the shared button-base initializer
  `0x4f6040`, installs the `CCBtnShopCart` vftable, clears two 280-byte blocks
  starting at `+0x64`, clears the words at `+0x17C` and `+0x180`, then
  returns the object pointer.
- **Reach and player-visible role**: player-visible class identity: the Item
  Shop cart buttons are built from this class. Direct callers are the shared
  control factory and host/slot loaders (`0x4329b0`, `0x48a620`, `0x48b2d0`);
  it installs behavior but does not commit a purchase itself.
- **Our client**: no native button object. Cart chrome is staged by
  `shop_cart_slots` (`crates/o2jam-shell/src/screens/side.rs:2109`) and
  guarded by `shop_cart_buy_and_pager_answer_their_staged_buttons`
  (`side.rs:7933`, Tier 0).
- **Evidence**: analysis ev
  2620.8ffd.3e35.ae1c.97b8.6cbb.1065.d864.664c.c244.52cc.f97e.4050.38cf.cd04.012b;
  incoming call sites ev
  dda1.0698.bb74.8d8d.9446.f1be.ae72.a994.ed9c.c6fd.947c.e683.0d20.8819.f835.b91c
  (observed direct callers: `0x4329b0`, `0x48a620`, `0x48b2d0`); the vftable
  symbol `CCBtnShopCart::vftable` resolves to `0x64c6e8`.

### `0x53f9e0` — construct a player-info object (`CPlayerInfo`)

- **VA / file offset**: `0x53f9e0` / `0x13f9e0` (1,309,152).
- **Behavior (own words)**: installs the `CPlayerInfo` vftable, clears the
  words at `+0x2038` and `+0x203C`, zeroes 29 consecutive pairs of words
  from `+0x2040` through `+0x2128`, calls the shared initializer `0x53fd10`,
  and returns the object pointer.
- **Reach and player-visible role**: player-visible consequence. Direct
  callers include digit writers (`0x528e20`, `0x529230`), net handlers
  (`0x55fbe0`, `0x55ff00`) and other consumers; `docs/myroom.md` names
  `CPlayerInfo` as the MyRoom info object, whose staged fields feed the
  status plate. This constructor supplies cleared defaults; it does not
  choose caption text or paint a rect.
- **Our client**: no counterpart by VA. Fetched character data is staged by
  `ScreenGame::set_myroom_character`
  (`crates/o2jam-shell/src/screens/game/myroom.rs:138`) and rendered as
  captions by `myroom_identity_captions`
  (`crates/o2jam-shell/src/screens/side/myroom.rs:940`), with Tier 0 guard
  `myroom_identity_captions_carry_character_fields`
  (`crates/o2jam-shell/src/screens/side.rs:8805`).
- **Evidence**: analysis ev
  3a28.f011.0479.3d97.5984.a1b5.d7f9.f0bc.9f69.1ecd.7708.a1ae.2eba.c04d.293d.df77;
  incoming call sites ev
  289e.42f2.f6a3.0350.3c39.35f0.a7f2.22c1.959b.48a0.37da.5db4.3428.2e1b.d2db.2149
  (observed direct callers: `0x497f60`, `0x528e20`, `0x529230`, `0x55fbe0`,
  `0x55ff00`, `0x56f6a0`, `0x63d2b0`, `0x63d300`); the vftable symbol
  `CPlayerInfo::vftable` resolves to `0x64b2b0`.

### `0x53fa60` — reinstall the `CPlayerInfo` vftable (support)

- **VA / file offset**: `0x53fa60` / `0x13fa60` (1,309,280).
- **Behavior (own words)**: writes the `CPlayerInfo` vftable to the object's
  first word and returns; no field is cleared or staged.
- **Reach and player-visible role**: support. It has many direct callers
  (state constructors, digit writers, net handlers, option helpers); it
  restores class identity on existing player-info objects and paints nothing
  itself.
- **Our client**: no counterpart by VA; the Rust side owns no vftable.
- **Evidence**: analysis ev
  dedd.caff.887f.6122.e0af.e5f7.b3d6.065a.fe9c.9895.45fe.cca1.33b3.5f5d.4078.c610;
  incoming call sites ev
  d5cb.e7fe.1ea1.12b6.5c16.aad8.d579.45ea.fd31.9062.41d0.6dbf.32e6.c991.9c01.a746
  (24 observed direct callers, listed in the query log).

## Corrections and OPENs

- Triage labels for `0x4a9da0`–`0x4aff17` come from the `0x4a0000` address
  cluster. Their bodies are generic twelve-byte and byte vector helpers. The
  twelve-byte family is reached from Item Shop 5001 and 5005 handlers; the
  byte family is shared with the Waiting chat ring
  (`docs/pins/room-waiting.md`). No standalone player-visible story follows.
- `0x526df0` is triaged as Music Shop by class name, but the pinned lifecycle
  says `CCBtnSelectMusic` serves the Waiting `DAILOG_SELECTMUSIC`. The class
  pin stays valid; the screen attribution stays OPEN (next step: control-code
  xrefs from `CCBtnSelectMusic::vftable` to the staging state).
- The host screen of `CCBtnGiftList` (`0x51ea80`) is unresolved statically;
  Item Shop GiftView and MyRoom gift box remain both plausible. Next step:
  boot-to-control chain for a staged gift-list rect.
- Four tiny fragments (`0x4aa7e9`, `0x4aaa3b`, plus the two insert epilogues'
  unwind arms) are Ghidra-reported; their roles come from caller flow, not
  runtime observation.
- A static pin never closes a census row. The button constructors and the
  player-info constructor need Tier 0 object-state guards and the named live
  input/art legs before any shop or MyRoom row can close;
  `docs/live-path-reachability.md` supplies the tier ladder. Spec-shaped
  suggestions are in
  `~/scratch/lanes-2026-11-25/spec-enrich-pins-shops-myroom.md`.

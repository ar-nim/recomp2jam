# Playing input statics dossier (issues #625, #649)

Scope: static gate for the Playing input batch. No rig capture is made
here. Every claim is from `OTwo.exe` 7.55 (5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`)
through the REA MCP (Ghidra 12.2, REA 6.10.1). File offsets are VA −
`0x400000`. Tool identity: `ev_696158c42763ef469cb735213401bf31c2ad2491166f43c4e42f3f17adba8e7d`.
No disassembly or decompiled text is reproduced; behavior is described
in prose with evidence IDs.

## 1 — #625 key-down/head and key-up/tail routing

Verdict: **CONFIRMED statically**. The modelled `+0x864` routing is
what 7.55 does. Nothing in the Rust model needs a code change. The
live capture is still required because a seam-green static pin is not
a player-visible guard.

### Anchors checked (all in 7.55)

| Pin | File offset | Check | Evidence |
|---|---|---|---|
| input inversion | `0x1714b0` (body `0x1714b0`–`0x1714f6`) | function body branches on pressed argument; nonzero forwards event byte 0 to ring producer, zero forwards byte 1 | `ev_6b5bbb28c385d8af658cf0873c34cb22423a05e950b97ab6881921a67799adac`, prose `ev_1fa6185809f8f9ce7b3ba2cf8135959842ef756618926a855ee721d8480bf714` |
| incoming queue calls | `0xe17c6`/`0xe1877`/`0xe1b12` (VA `0x4e17c6`/`0x4e1877`/`0x4e1b12`) | three direct calls into the forward from the queue drain | `ev_525a1a4874fa6db633e74f854342aa315f142dcbff0d1d474d89697842bf7e14`, `ev_1d26c92638724ca5cc39863f9e3663cf1cb3619647511c3bb7bb156ca80ae979`, `ev_ecfec93ab5c75db773b1508260af31551e4e7fd14b1e0c29cebe4ee19dd3aa74` |
| ring edge producer | `0x179b80` | 128-slot ring at `+0x400`; per-lane flags at `+0x408` start set; event byte is the forwarded value, not the raw key state | `ev_c8d4203c9c8e9c5012be2344f167a3424f2dbd0f2dcc45135d722d11adb16b64` |
| event-byte branch | `0x1765d1` in `FUN_00576540` | compares event byte with 1; byte 1 falls into the cursor branch, byte 0 falls into the walk/head branch | `ev_0eba18edc06ddb06c481687b712413e9c3642f649ccc41057643cc1deed9adfc`, function `ev_957650308f9b926db34bcd7d92924500f7af59bdcb22031a281d3547def5de85` |
| cursor load | `0x1765e4` | key-up branch reads per-lane pointer `+0x864[lane]` | `ev_7c73360fc576b113019f295e5e50a995e5ad0c8d61b385569e40942ad4c6128a` |
| key-up bit-25 region | `0x176c3b`/`0x176c41` | these are interior bytes of two instructions at `0x176c3a` (load record word) and `0x176c3c` (mask `0x3000000`); the compare at `0x176c42` tests equality with `0x2000000` | `ev_1099c554a6aad3240e686a7db6d5cb7b13f8d7573bd7b9aad8a69c5835ec448b`, `ev_eca1e1820c5add5d0b2e649805d54cf01e9b8629c2a7eca76d95e8ddfbe91af9`, `ev_34603f617fea2322c5a66548f6185468dc0dcb1a4974d042b454021d41f8f0ed`, `ev_c790e5cf17f40a53ac9fc9584df1e1cdba74f933736eb99941f325c5ec211075` |
| tail compare chain | `0x176646`–`0x17667a` | absolute difference against record time is tiered at 6/18/25, then 26+ misses; tier is OR-ed into the record at `0x176687` | `ev_8cacce34232ccf12da2f16bffea7fe3a1e25a46edb3085b8051af1bb0822d98a`, `ev_5f5557853faa8dc5a61ea01f252bb5af9989364626531d00ea5380a4147913cc`, `ev_067f50c6300d5a438671a519f67a3e15dd7719a7669a77709574c1e14bad3fbe` |
| head-side bit-25 gate | `0x1766cb` | after a key-up tail rating, only a bit-25 record emits rating event `0xe` | `ev_229f0bc00696dcee9a2862c9b38e2077d599037a3e5b9ed25a392bf437cb20f7` |
| cursor clear | `0x176709` | key-up branch in the mode-0 judge stores zero to `+0x864[lane]` after rating | `ev_41e018a18648f0d6500b8b0dae696110ecb841a3f7049820acb23d0304b40570` |
| cursor arm (player-key paths) | `0x176e60` (mode-0 judge) and `0x1761c4` (second judge) | both store the next record from the lane walk into `+0x864[lane]` after a Cool/Good head | `ev_3ba2c4bfa0c1968c27396ed5a79ca23e49d364e2484df1b3d303a14c9cfb53f5`, `ev_a8d0b63fbf2d585a1725fc0603cf762cc4855af61074f1f553150a4fd2b0a246` |
| other cursor clears | `0x175ade`, `0x1760e4`, `0x1771d4` | play reset, second judge, and auto judge also zero the same field | `ev_6b65f20b1e04a5d76f20010bc37f01e530b27820c488a0255cafc5a21d8764a9`, `ev_b101232338d2f059fefba31510b8e65a4d39db4c8b892d90a459cf16daa4d815`, `ev_65fce122166e6d1bb44ef9078a2bf51b83727365d2a2aaac244839da145831a0` |
| cursor arm (auto judge) | `0x1772b4` | the auto judge arms the same field after rating a due head | `ev_9340f1790b1e610c32c77f1f16aa7f0c902c8614c45c5e79b6da5335d7f6b864`, `ev_71a1086f688a1609936996f80f548f30179b468e22cfa0108222c86e35ed8a8a` |

Whole-function prose for the mode-0 judge: `ev_9d2a86414d8c44eac0a4447128225a0d0a0bd53d81fab4a4627e0b2961fde11b`.
The earlier queue kind→pressed mapping (kind 1 press, kind 2 release)
is the already-pinned `ev_c01db4a8…` from `docs/gameplay.md`; this leg
did not re-run that dossier.

### Completed static picture

1. The queue drain calls the forward at file `0xe17c6`, `0xe1877`,
   `0xe1b12` (VA `0x4e17c6`, `0x4e1877`, `0x4e1b12`). The forward at
   file `0x1714b0` inverts: a press (nonzero argument) is forwarded
   as ring byte 0; a release (zero argument) is
   forwarded as ring byte 1. The relay at file `0x177ff0` and the
   producer at file `0x179b80` copy that byte into the ring and detect
   edges against `+0x408[lane]`.
2. The per-frame judge `FUN_00576540` drains one ring event. Its test
   at file `0x1765d1` selects two mutually exclusive branches.
   - Byte 0 (key-down): walk the lane list, skip bit-24 tail records,
     rate the nearest unjudged record at file `0x176b7f`–`0x176ba7`,
     and on a Cool/Good bit-25 head latch the hold span/flags and arm
     `+0x864[lane]` at the next record (the paired tail) at file
     `0x176e60`. A Bad head kills the paired tail instead and does not
     arm the cursor.
   - Byte 1 (key-up): read `+0x864[lane]` at file `0x1765e4`. If the
     pointer is set and unjudged, tier |release − record time| through
     the chain at `0x176646`–`0x17667a`, OR the tier into the record at
     `0x176687`, optionally emit rating event `0xe` for a bit-25
     record (`0x1766cb`), then clear the cursor to zero at `0x176709`.
3. Therefore `+0x864[lane]` is a **one-shot per-lane tail cursor**, not
   a hold flag and not a timing value: it holds a record pointer that
   a key-up consumes. Scoped to the player-key paths: the armers are
   the head-strike paths at `0x176e60` (mode 0) and `0x1761c4`
   (second judge), and the key-up clear is `0x176709`. This is not an
   exhaustive list of writers. An exhaustive byte sweep for the
   displacement `0x864` returned 13 raw hits
   (`ev_eb2988ea6d646d0b4df9c9fce02cc888e2632fe8cdafee9a1d74fd5dda7c8fd5`);
   eight are the Playing cursor. Besides the three player-key sites
   above and the read at `0x1765e4`, the others are: play reset clear
   `0x175ade` (`ev_7aaa5962b4755ba8bb37a50d3d85c89db351a08a4afed5dfe7a7bd883a8d5e39`),
   second-judge clear `0x1760e4`, auto-judge clear `0x1771d4`, and
   auto-judge arm `0x1772b4`. The auto judge
   `FUN_00577090` (`0x177090`) rates due records itself, clears the
   cursor, then arms it from a bit-25 head in the same loop
   (`ev_71a1086f688a1609936996f80f548f30179b468e22cfa0108222c86e35ed8a8a`).
   The other five raw hits fall outside the Playing judge family
   (`0x4d8bf6`, `0x4de94c`, `0x4df111`, `0x4df140`, `0x54f0b4`);
   inspected examples are stack-MOV byte coincidences, not
   `+0x864[lane]` accesses in this path. The sole reader of the Playing cursor
   in that sweep is the key-up branch at `0x1765e4`; this is the
   checked scope, not a claim over every possible encoding in the exe.
4. Bit-25 test locations are consistent: equality form
   `payload & 0x3000000 == 0x2000000` in the key-up arming path
   (`0x176c3c`–`0x176c42`) and in the second judge, bit-25-only form on
   the tail-rating emit path (`0x1766cb`). File offsets `0x176c3b` and
   `0x176c41` in the older note are interior bytes of the two
   instructions; the instruction boundaries are `0x176c3a` and
   `0x176c3c`.

Verdict per arm: inversion **CONFIRMED**; head routing **CONFIRMED**;
tail routing through `+0x864` **CONFIRMED**; player-key
writers/reader/clear **COMPLETED**, with auto-judge armers/clearers
listed separately; live judged log **OPEN** (blocked earlier on the DI
interposer, #700; first-hit alignment leg #699).

### Rig brief — what the live capture must show

One stimulus per boot, epoch-stamped injector, judged log attached to
the same epoch as the capture frames and the wire slice. Do not batch
several key events into one boot; that is what made the s8 log
ambiguous. Capture the DI queue push, ring byte, and judged tally in
one manifest.

Expected signature by branch:

- Press at a tap note: ring byte 0; head/tap record gains tier
  `0x10`/`0x20`/`0x40`/`0x80` by |press − onset| ≤6/≤18/≤25/≥26 ms.
  No `+0x864` pointer is armed for a tap.
- Press on a hold head: same head tier plus span latch and
  `+0x864[lane]` set to the paired tail only for Cool/Good; a Bad head
  marks the tail `0x180` and leaves the cursor unset.
- Release at the hold tail: ring byte 1; tail record gains a tier from
  |release − tail time| with the same windows; `+0x864[lane]` returns
  to zero. If no head armed the cursor, the release produces no tail
  line. If the polarity were wrong, the head would be rated on release
  and no tail line would appear on press — that is the falsifier.
- Optional stopped-state witness: read `+0x864[lane]` before and after
  the release; it must be a nonzero record pointer between the
  Cool/Good head and the release, zero after.

## 2 — #649 NoteTail staged-art plan

Verdict: **STATIC ARM COMPLETE; PIXEL ARM OPEN**. The table and all
pinned consumers (eight functions) are traced. The option does not
select a set of per-style hold-art frames; it scales a time-gated 0..120 geometry
scalar in the `CNewNote` subobject at Playing `+0xfec`, which feeds
per-lane clipped sprite blits. Which pixels change, and whether the
hold body itself changes, needs the staged-art capture.

### Table and consumer anchors

- Table VA `0x67fe8c`, file offset `0x27fe8c` (MCP reports 2621068):
  eight-byte stride, two 32-bit floats per slot. Verified pairs are
  `(0,0)`, `(16,14)`, `(30,25)`, `(54,50)`; the next eight bytes are
  not a pair — `0x67feac` is the `NOTE`/`NoteTail` option global
  (byte value 2) and `0x67feb0` starts a separate seven-lane integer
  table. `ev_4e7fc373f226f2f39ad38df45a5fbddf464caaa95ae2a3331c169ae6938f39ec`,
  `ev_c2c90248db6fff3b5d5091883b2a0193daf36ba9a4332a2d33cae51caa712c58`.
- Direct references to the table base occur at file `0x16bf88`
  (reload) and `0x16bfff` (setter); the constructor reaches both table
  halves through the indexed address. `ev_78dac63fb552c56658ba5406a041b22a93681036ec71054e1e2f8d79c125468a`.
- Consumer dossiers (all 7.55): `ev_3862671a817fdf188ec4f315b3ea1ce4fd5ef816f3dc6dac6e0c733673f8d8bd`
  covers the constructor `0x56b8f0`, reload `0x56bf40`, setter
  `0x56bfb0`, recompute `0x56bed0`, stepper `0x56bda0`, lane fan-out
  `0x56c060`, per-lane builder `0x56c670`, and cycle wrapper
  `0x56c030`. Instruction-level form of the recompute:
  `ev_c19d96b4c2e6b487d5868deb9bedd0831dad8a7cf7178a8ead20d1faf646df53`.

### Completed static picture (and a documented contradiction)

1. Constructor `0x56b8f0` builds the `CNewNote` subobject at Playing
   `+0xfec`: seven 0x0c-byte lane records, three 0xac-byte records,
   pair floats into object `+0x6c`/`+0x70`, and start scalar `+0` at 0.
   Its sole caller is Playing construction at file `0x16f701`
   (`ev_6cd3e033275731d1ee0d803ca6eb7995329c80cb0a720e678f2780b8bbdf8ed3`,
   `ev_dc2807f7d48ec4887ec90d4e3011c496c0b6cd093da35b17282c0a77dedc7e40`).
2. Reload `0x56bf40` reads `NOTE`/`NoteTail` from `option.ini`, default
   2, and **clamps to 0..3 with fallback 2**. Setter `0x56bfb0` accepts
   only 0..3, writes `value + '0'` back to `option.ini`, stores the
   global, reloads the floats, and recomputes. Cycle wrapper `0x56c030`
   uses `(index + 1) mod 4`. Contradiction with `docs/options.md` and
   claim B45 ("clamp 0–4, five-slot table"): the exe oracle is 0..3 and
   the fifth slot is other state. Value 4 is unreachable through the
   setter and the cycle; if hand-written to `option.ini`, reload
   replaces it with 2. This dossier records the contradiction; it does
   not edit those docs.
3. Recompute `0x56bed0` derives target `+4` from the pair, object
   floats `+0x64` (chart max-time scale) and `+0x68` (speed), dividing
   by 160 and clamping to 0..120. The writer `0x56be30` sets `+0x64`
   from Playing `+0x83c` and `+0x68` from the speed float at `+0xc`;
   its callers are Playing setup at file `0x1702ff` and render event
   `0xb` in the dispatcher at file `0x16e170`.
   `ev_06ed432131025ac1253ae882576e28d79929f6ffbed498577df5e8701e977825`,
   `ev_49edb32fe88eacb66c65c7ccec472ec189479c89ff87f7c23579676715d95643`,
   `ev_a21a113eac0c92556289e3babfda401ae5b3312c878d0da7fce705eb00de8537`,
   `ev_dab5728b10dabc21570192577ea71de90a20a687922efccda81669e1a0e40bcc`.
4. Per frame, `FUN_00571060` calls stepper `0x56bda0`, which walks
   current `+0` toward target `+4` by one unit only while the two
   differ: after 1000 ms without a step it resets its timer to now,
   then a step needs another 20 ms. The stepper therefore pauses when
   the pair is steady and can resume on a later target change
   (`ev_c8c2cc0c7fbb9d273a6fe4fe7c5c343ad145b75c8e9d18a2b56bfbd37f374c36`).
   Render event `0xb` also recomputes the target through `0x56be30`
   (`ev_a21a113eac0c92556289e3babfda401ae5b3312c878d0da7fce705eb00de8537`).
   Then `0x56c060` fans out to seven lanes
   and `0x56c670` invokes the lane builder. The builder creates its
   segment list (`0x56c810`) and emits two clipped sprite blits per
   lane through `0x56c2f0`, whose final painter is a DirectDraw sprite
   blit with a clipped `RECT` (`0x5844a0`). The current 0..120 scalar
   is the split parameter in that blit geometry.
   `ev_e9dcd0cd7b0c782393cb0d1c46c5c18b1d38f8d81e25f8cbb387bf61d0f4f54d`,
   `ev_f884da092325224721f274ee5adc4dd9126cde4476576fe01b086705af7089f9`,
   `ev_d387a0e6b4dc6783bc3203956979f5c3fa5a07e407b88b9d56701afa1211294c`.

Verdict per index (static prediction, not a pixel claim):

| `NoteTail` | Pair | Static consequence |
|---|---|---|
| 0 | `(0,0)` | target scalar stays 0; no time-gated ramp |
| 1 | `(16,14)` | target uses `16 + 14 × (speed − 1)` |
| 2 | `(30,25)` | target uses `30 + 25 × (speed − 1)`; exe default |
| 3 | `(54,50)` | target uses `54 + 50 × (speed − 1)` |
| 4 | not a pair | unreachable via UI; reload coerces to 2 |

With a long chart, `+0x64` is large enough that the clamp usually
saturates every nonzero option at 120. Differences between 1/2/3 are
then in the transient, not the settled frame. The capture must settle
that, not this dossier.

### Rig brief — staged-art capture

- Set `NOTE/NoteTail` to 0, 1, 2 and 3 in separate boots (or press the
  exe cycle button, which walks exactly those four values). Do not
  capture value 4; it is not a style.
- Replay `E-864`: the default 7KEY song 864 chart, not the incidental
  757 chart. Use chart-timed presses so the life bar stays alive to
  the hold segment; pressing nothing ends the song early.
- Photograph the same wall-clock offset relative to a hold head and
  again at the same hold-tail position for each value. The region is
  the seven lane columns (x 5..165, y 28..487) plus the hold body;
  record the full 800x600 frame too, since the ramp may paint outside
  the lane clip. Do **not** pick a settled frame by wall-clock alone:
  the stepper only moves while `+0 != +4`, pauses after a steady pair,
  and render event `0xb` can retarget it. Take the settled sample only
  after observing a steady `+0`/`+4` pair on the rendered target (two
  consecutive reads with no change), and log the pair with the frame.
- Log the injector epoch, judged tally, and `option.ini` value per
  boot. If stopped-state reads are available, read `+0xfec` object
  `+0`, `+4`, `+0x64`, `+0x68` to pair the scalar with the pixels.
- Falsifier: identical lane pixels for 0/1/2/3 at all sampled frames
  would refute this consumer chain; different pixels with no scalar
  change would mean another consumer exists.

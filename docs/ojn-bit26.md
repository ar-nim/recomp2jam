# OJN bit-26 (payload `0x0400_0000`) — shipped-data sweep + pipeline verdict

Claim **B50** (issue #83, child of map #66). Verdict per ADR-0009:
the sweep below is structural (parser-independent) and every named
path is checked, so the OPEN arm sharpens instead of standing.

## Method

Byte-level sweep over all 14 shipped 7.55 `.ojn` charts in
`install-root/O2Jam/Music/` (new-format files pass through the
exe-verified byte transform `decrypt_ojn_new_format` only; slice
offsets +284/+288/+292/+296, event counts +64/+68/+72, and the
`8 + count * 4` event framing are re-read as raw little-endian words
in the sweep itself — no semantic decoder is trusted). A hit is any
note payload (kind ≥ 2, nonzero low word) with bit 26 set. Guard:
`crates/o2jam-file/tests/ojn_bit26_sweep.rs` (skips cleanly when
`install-root` is absent).

Evidence pins: `OTwo.exe` 7.55 sha256 `83a04541…`; `Tutorial.ojn`
sha256 `bfabcb9a…`, 118738 bytes; new-format charts carry the `new`
magic prefix (e.g. `o2ma757.ojn` sha256 `01083e2c…`).

## Sweep results — bit-26-set note payloads per file

| file | size (B) | events | note payloads | bit-26 hits |
|---|---:|---:|---:|---:|
| Tutorial.ojn | 118738 | 43 | 51 | **12** |
| o2ma757.ojn | 179182 | 580 | 2006 | 0 |
| o2ma758.ojn | 189591 | 515 | 1399 | 0 |
| o2ma763.ojn | 356723 | 2180 | 5769 | 0 |
| o2ma765.ojn | 591848 | 1713 | 4987 | 0 |
| o2ma766.ojn | 253578 | 596 | 1798 | 0 |
| o2ma767.ojn | 259069 | 533 | 1898 | 0 |
| o2ma768.ojn | 360339 | 765 | 2299 | 0 |
| o2ma769.ojn | 248549 | 894 | 2386 | 0 |
| o2ma770.ojn | 1036864 | 1071 | 3956 | 0 |
| o2ma860.ojn | 228762 | 1122 | 4762 | 0 |
| o2ma861.ojn | 227654 | 1014 | 4756 | 0 |
| o2ma864.ojn | 299413 | 1317 | 7152 | 0 |
| o2ma865.ojn | 292058 | 1165 | 7129 | 0 |

Totals: 50418 note payloads scanned, 12 bit-26 hits — **all 12 on
Tutorial.ojn's BGM channel (kind 9), all on the Easy slice**:
measures 0/12/20/28/…/92, payload index 189 of 192 in each measure's
kind-9 event, raw words `0x0400_0001`…`0x0400_000C` — flags word
`0x0400` (bit 26 alone) plus contiguous sample ids 0..=11. No
playable-lane payload (kinds 2..=8) on any shipped chart ever sets
bit 26; the 13 new-format songs set it nowhere at all.

## What the shipped hits mean — already pinned by B22

These twelve records are exactly the B22 autoplay encoding: the
in-exe parser's BGM branch consumes autoplay value words as "bit-26
presence marker + 12-bit sample id" (`0x572a4f`–`0x572a6a`, the same
span-survey site recorded as `0x172a4f`), then fires the due nodes
into the rating-event sink with the raw value dword. Bit 26 on a
kind-9 payload is therefore **not** a hold flag and not OPEN: it is
the autoplay play-marker, shipped-visible only on the Tutorial's BGM
percussion track. The old "shipped payloads never set the bit"
phrasing (B14/B50) was about playable lanes; it is corrected here to
"no shipped *playable-lane* payload sets the bit".

## Synthetic pipeline check

Guard: `crates/o2jam-game/tests/ojn_bit26_pipeline.rs` — a synthetic
old-format chart (one kind-2 lane note `0x0400_0001`, one plain tap,
one kind-9 BGM note `0x0400_0002`) through
`parse_ojn_level_events` → `build_level_chart`:

- the flag is preserved end-to-end: the lane hit and the backing
  note both keep `flags == 0x0400` (payload bit 26 = flags bit 10);
- both payloads test false on `HoldSpan::marks_hold_head` and
  `marks_hold_tail`, so the flag can never reach a judge arm through
  the modeled pipeline — matching the exe-side negative sweep
  (`0x5703f0`–`0x578300`, docs/gameplay.md payload-bit rules).

The exe's remaining consumers of the bit (post-parse fix-up
`0x572f50`, strike-slot `+0x3E8` bank, files `0x170427`/`0x170431`)
have no modeled Rust counterpart yet; the strike-slot bank is what a
rig chart swap would observe.

## Verdict (B50, sharpened)

Shipped data **does** set bit 26 — exclusively on Tutorial BGM
records, where it is the B22 autoplay presence marker, not a hold
flag. The hold-semantics question for *playable* notes stays OPEN,
now narrowed to **synthetic-only**: no shipped evidence can settle
what the parser fix-up `0x572f50` does to a bit-26 *lane* note.
Next step (named): rig chart swap — serve a synthetic chart whose
lane note carries bit 26 to the Wine client and read the strike-slot
`+0x3E8` bank after load (memory-read rig per `~/scratch/planet-verify`
/ winedbg discipline), naming the bank consumer's observable effect.

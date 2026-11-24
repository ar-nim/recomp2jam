# Malformed-input corpus — synthetic, CI-safe (#905)

`crates/o2jam-file/tests/malformed_corpus.rs` builds a small valid file
for each shipped parser, then applies named mutations. Every case must
return a named `FileError` through the public parser entry point; a
panic or an `Ok` fails the guard via `catch_unwind`. No game data is
read, so the corpus runs with `install-root/` absent.

| Parser | Valid seed | Corpus mutations (expected error) |
|---|---|---|
| OPI | one 152-byte table entry | truncated table (`Truncated`), size 999 (`Truncated`), flipped slot count (`Truncated`) |
| OJN header / level walk | 300-byte `ojn` header, empty slices | 16-byte cut (`TooShort`), flipped magic (`UnknownFormat`), end offset `u32::MAX` (`Truncated`) |
| OJM M30 / OMC | inline 4-byte `OggS` stream in each variant | truncated body (`Truncated`) x2, size 999 (`Truncated`), flipped magic (`UnknownFormat`) |
| BND | magic + zero rects | 5-byte cut (`TooShort`), count 2 over 6 bytes (`Truncated`), flipped magic byte (`UnknownFormat`) |
| ControlList | one `STATE_OK` block | cut hex code (`UnknownFormat`), extra closing brace (`UnknownFormat`), flipped hex prefix (`UnknownFormat`) |
| Item tables | one item row / one set row | truncated row (`Truncated`) x2, declared count 99 (`UnknownFormat`), flipped name byte / set id (`UnknownFormat`, `TooShort`), text length 999 (`Truncated`) |
| Song list (`OJNList.dat`) | one 300-byte `ojn` record | 299-byte cut and flipped record magic (`UnknownFormat`) |

The "over-long" category is expressed as an overstated field where the
format has one (OPI size, BND count, OJM size, OJN slice end, item
text length). `OJNList.dat` has no length field; its partial-record
tail is documented tolerated data, not a corpus mutation.

## Red proof

On 2026-11-24, temporarily removing the `BND_MAGIC` check in
`parse_bnd` made
`cargo test -p o2jam-file --test malformed_corpus bnd_parser_rejects_corpus_mutations`
fail with `bnd silently accepted BitFlippedMarker`. The check was
restored before commit; the corpus passes again.

## Exe failure behavior — OPEN

No 7.55 evidence pins what OTwo.exe does with a corrupt archive (dialog
text, error code, or partial load) for these seven formats. Target
build: OTwo.exe 7.55, 5,029,888 bytes, SHA-256 `83a04541…72c823b`.
Next evidence step: place synthetic truncated/flipped files in a
scratch install root, launch that build under Wine with a fresh prefix,
capture the visible failure and any log, then add version/hash/tool
versions and REA evidence IDs here. Until then the corpus pins only
our parser behavior; it does not claim exe parity.

## Real-data pack gate (#906)

`data_root::pack_dir_for_test` is the single seam for the eight
real-data tests in `o2jam-file`. When `install-root/` is absent it
prints `SKIP <test>: install-root absent (<root>)` and returns `None`.
When the data folder is present it requires `Interface1.opi`,
`Playing1.opi`, `avatar.opa`, a `Music` folder and at least 14 `.ojn`
charts; a gap panics with `PACK GAP <test>: missing <path>`. The gate
uses `O2JAM_INSTALL_ROOT` at runtime, so no test bakes in a worktree
path.

Red proof, 2026-11-24: temporarily disabling `require_pack_artifacts`
made `present_pack_with_missing_archive_fails_instead_of_skipping` fail
with `present pack missing avatar.opa must fail loudly`; the check was
restored. A present-but-gapped root passed through
`O2JAM_INSTALL_ROOT=<root> cargo test -p o2jam-file --test real_data_gate`
and failed with `PACK GAP ... missing .../Image/avatar.opa`.
With the main checkout's real install root the whole `o2jam-file` suite
passed; with no install root all eight tests printed SKIP and the suite
passed.

# Spriteless-block fill: `STATE_CHANNEL` (7.55) — mechanism VERIFIED, color OPEN

Which of dialog-default / parent-background / window-clear / first-child-art
paints behind a block that names zero sprites. Short answer: **none is
evidenced — the fill color stays OPEN.** What *is* pinned: the block stages
no art at all, and no dedicated class hosts it. No disassembly is reproduced;
offsets are file offsets unless marked VA (VA = file + `0x400000`).

## The block (byte walk, read-only)

`Interface1.opi` slot 1595 (`ControlList_Interface.txt`, data offset
74438614, 90472 B, 1979 CP949 lines): `STATE_CHANNEL` at lines 12–61,
header `0x27 0X01`, bare `BOUND` line (no stem — the only full state
without one), 50 `IDC_` entries including self, one `SET 0X0A` group of
10. **Zero quoted sprite strings in the whole block** — no entry names
art, so recipe step 6 (`docs/opi-pattern.md`: sprite-less entries draw
no art) leaves every control art-less. First child
`IDC_BACK_SCHANNEL_1 0X12100001` is the would-be background entry, but
with no sprite and no bind it paints nothing — "first-child art" is
refuted as the fill.

## The host (no `CStateChannel`)

RTTI holds no `CStateChannel` class (lifecycle §6, unknown #2). The stager
is the generic state-open `0x48A060` (file `0x148A060`,
`ev_9e526b62…`): block = `0x762B44+0xF80+(n−1)×0x118`, bind loaded only
when the stem field is non-empty (bare `BOUND` → skipped, so all rects
miss to all-zero), one control per entry gated on `code & 0xFF0000 ≠ 0`
(`0x48A134`). Self entry `0x12000000` gates out; the other 49 stage
art-less controls. The render loop `0x5DEC30` (file `0x1DEC30`,
`ev_ff581cfa…`) draws stored controls only — the state contributes zero
pixels. Visible ink rides code-mounted art outside the block
(`Dialog_ChannelSelectBar.ojs` sits in the archive, named by no block;
`DIALOG_CHANNEL_INFO`/`CDChannelInfo` + `CCBtnChannel` per lifecycle
§6) through the unpinned dialog-open path (`0x4ED650`, ui-loader OPEN
2–3).

## Fill candidates — all unpinned

- **Dialog default color**: `CDialogBase` (`0x67d7e0`) / `CWindow`
  (`0x6448bc`) unexamined (ui-draw OPEN 3). No default-fill call traced.
- **Parent background**: state-open mounts no parent fill; the screen-level
  consumer of the child array is unpinned (claims-render OPEN 2).
- **Window clear color**: no backbuffer-clear call traced; display init
  (`0x413030`) pins mode/Flip vocabulary only, clear color stays OPEN.
- **First-child art**: refuted above (no sprite, no rect).

## Shell rule (`crates/o2jam-shell/src/screens/paint.rs`)

`main_scene_sprites(Channel)` returns `[]`; the scene stages nothing and
`paint_backdrop` keeps the flat navy `background_of(Channel)` plus the
live list. That navy is a **documented placeholder, not a verdict** (the
flat fills stay invented per bnd-audit 35/158). Tests:
`spriteless_channel_block_stages_no_scene_art` (shape-faithful bare-`BOUND`
block: image/button/text/`SET` code classes, zero sprites → no scene art),
`empty_channel_art_keeps_the_documented_placeholder_fill`.

## OPEN

1. Backbuffer content under art-less states (stale frame vs clear color).
2. Sprite-less control paint (text/button/image with no art or rect).
3. Dialog-open `0x4ED650` + `DIALOG_CHANNEL_INFO` mount of the select bar.
4. `CWindow`/`CDialogBase` default fill, if any.

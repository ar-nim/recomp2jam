# Waiting-room master emblem (#303) — local pin

Version pin: `OTwo.exe` 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`;
`Interface1.opi` 75,100,702 B, SHA-256
`d66541198965d622ff0177b306d916885ae4d64ae07b2c2264fcbdd850d50584`.
Method: read-only parse of the installed ControlList and bind section; no
asset bytes or disassembly are reproduced here.

## Control

`ControlList_Interface.txt`, `STATE_WAITING 0X63 0X03`, `BOUND "State_Waiting.bnd"`:

- `IDC_IMAGE_SWIAT_BOSSMARK 0X1650002F`, sprite `Wait_BossMark.ojs`
  (the archive spells the id `SWIAT`). It is a member of the seat template
  `SET 0X0C 0X00015000`, parented by `IDC_BUTTON_SWAIT_EXTEND1..8`
  (`0X16015022..29`).
- Ordinal pairing (exe step `0x548640`, `docs/bnd-semantics.md`):
  `(0x2F − 1) = 46` → `State_Waiting.bnd` rect `(1,1)-(29,29)`; parent
  EXTEND1 `(0x022 − 1) = 33` → `(37,55)-(160,205)`. The staged clone
  therefore inks at `(38,56)-(66,84)`, matching the Wine capture
  prediction and measurement recorded in issue #40 (`~/scratch/ad-wait.png`).
- `Wait_BossMark.ojs` has two raw RGB555 28×28 images, colorkey `0x7D5F`,
  anchors `(0,0)` and `(1,1)`. Image 0 is the active gold star: its dominant
  word `0x73C4` decodes to `(231,247,33)`, the same star ink measured in the
  Wine capture (Wine colour grade ±3). The exe sprite setter (file `0x2F870`,
  `docs/room-enums.md`) paints image `frame − 1`, so the shell's
  `icon_frame_layer(1, 2) = 0` selects image 0.
- The master flag is the room creator: local seat id 0 (`is_room_master`,
  files `0x159bdf`/`0x159c1b`/`0x159c4d`, `docs/relay.md` §10).

## Root cause and fix

`staged_seats()` staged the local member as seat 0, then also folded the
local wire slot into the peer seats (`slots.iter().enumerate()`). When the
local member was the creator, its `master` flag was duplicated on a second
seat, so the emblem painted twice. The fix clears that flag on a peer entry
whose `slot.index` is the local `member_id`; the master body now reaches
only the creator's seat. Peer seat layout and other indicators stay as
before (out of scope: seat ready marks, #277 role swap).

## Guards

- `master_emblem_stages_only_on_the_creator_seat` (`screens/paint.rs`):
  synthetic two-seat seam, red before the fix.
- `master_emblem_stages_only_on_the_creator_seat_with_real_art`
  (`o2jam-shell/tests/real_art.rs`): independent ControlList/bind parse,
  `ScreenGame` walk from Planet to Waiting, master and guest arms, star ink
  sampled from the decoded body at the bound rect; red before the fix,
  skips cleanly when `install-root` is absent.

OPEN: owner visual confirm of the live waiting frame; this work only pins
the local control, art, and paint guard.

# Messenger boot capture legs (spec #315)

Version pin: `OTwo.exe` 7.55, 5,029,888 B, sha256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
Host codes come from the shipped `ControlList_Interface.txt` evidence map.

These are **capture specifications**, not captured evidence. No boot was
spent by this lane and no item is checked. Each leg needs one qualified Wine
boot, the blue NORMAL-mode popup dismissed before every canvas click, the
`winegcc` `SendInput` helper (XTEST motion does not reach the DirectDraw
canvas), a before frame, an after frame, one input log, and a provenance
manifest naming the exe hash, role, host state and press point. A missing
leg stays OPEN with its next-step id; no response is inferred from another
host or from a filename.

## #644 (M4) — master-role response

- Evidence id: `N-MASTER`; host state: one qualified **master-role** exe boot
  after the #379 handoff, on the MyRoom board.
- Press: `SendInput` at `(644,580)` on `IDC_BUTTON_SMYROOM_MESSENGER`,
  code `0x1520006D`.
- Capture: MyRoom frame immediately before the press, response frame after
  it, the input log, and the session/role line that proves master role.
- Contrast: the B8 package `f9d4a695...f8b` / frame `946e5033...d917` /
  log `1e3911b9...b11` is a **peer** pin; never transfer it to master.

## #646 (M5) — Room host press

- Evidence id: `N-HOST`; host state: qualified exe boot on the Room board,
  NORMAL-mode popup dismissed before the click.
- Press: `SendInput` at the centre of the staged rect for
  `IDC_BUTTON_SROOM_MESSENGER`, code `0x13200048` (pinned rest/hover rect
  `627,571-662,590`, centre `(644,580)`).
- Capture: Room frame immediately before the press, frame after it, the
  input log, and the manifest naming the board and press point.
- Contrast: `j02-messenger-room.png` is a Playing scene and is not a pin.

## #648 (M6) — Item Shop host press

- Evidence id: `N-HOST`; host state: qualified exe boot on the Item Shop
  board, peer role, NORMAL-mode popup dismissed before the click.
- Press: `SendInput` at the centre of the staged rect for code `0x142000B8`
  in the Item Shop board; take the point from that board's parsed placement,
  not from the Room or MyRoom rect.
- Capture: Item Shop frame immediately before the press, frame after it,
  the input log, and the manifest naming the board and press point. The
  log must show no popup dismissal between the press and the response.

## #650 (M7) — Waiting host press

- Evidence id: `N-HOST`; host state: qualified exe boot in Waiting, peer
  role, with the blue NORMAL-mode popup dismissed before entering the board.
- Press: `SendInput` at the centre of the staged rect for code `0x1620006A`
  on the Waiting board.
- Capture: Waiting frame immediately before the press, frame after it, the
  input log, and the manifest naming the board and press point. The popup
  dismissal must precede the press and appear in the log.

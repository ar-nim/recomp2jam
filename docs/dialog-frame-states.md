# Dialog frame states — rest, hover, active (spec #316 story 17)

Version pin: `OTwo.exe` 7.55, 5,029,888 B, sha256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.

The exe's **button language** is pinned by the Wine sweep in
`docs/hover.md` for room and planet chrome: rest paints a desaturated
body frame 0, hover repaints body frame 0 at full colour, and an active
selection paints body frame 1 without reacting to hover. The qualified
story-matrix packs for dialogs carry **rest frames only**; the row
`docs/captures/journey-audit/316-app-wide-dialogs.md:60` is BLOCKED and
asks for a hover/press A/B pair (#245 class). The shell therefore applies
the language as a labeled stand-in: rest leaves the staged stack alone,
pointer hover repaints record 0, and a held press on a control whose
modal stays open repaints record 1. No sheet below has an exe-verified
hover or press frame yet.

| Dialog | States pinned by a pack? | Next evidence step |
| --- | --- | --- |
| `DIALOG_QUESTION1` (quit confirm; OK / CANCEL) | No — rest only | One qualified quit-confirm pack with off/on/pressed OK and CANCEL crops; confirm record indices and whether the click fires on down or up |
| `DIALOG_INFORMATION` (relay notice, refused enter, Room fuse, Messenger liveness; OK) | No — rest only | One qualified one-OK pack with pointer and held-press crops; name the OK record pair |
| `DIALOG_OPTION_KEY` / `DIALOG_OPTION_SOUND` / `DIALOG_OPTION_GAME` | No — `B3`/`B4` are rest frames (`docs/captures/journey-audit/314-options.md:63`) | Hover/press each tab, SAVE, EXIT and selection button in one boot; record which settings draw frame 1 |
| `DIALOG_CREATEROOM` / `DIALOG_INPUTPASSWORD` | No | Qualified hover/press pack on the OK and edit controls in Room |
| `DIALOG_CHARGECASH` (X close over the Item Shop board) | No — b8 pins geometry only (`docs/parity/payment.md`) | Boot-3 X hover plus held-down crop with the b8 frame behind it |
| `DIALOG_CHANNEL_INFO` | No — rest and hover **row bar** are pinned, not dialog button frames (`docs/parity/channel.md`) | Hover/press X, scroll arrows and enter faces in one qualified capture |
| `DIALOG_QUESTION2`, `DIALOG_WHISPER`, `DIALOG_EMOTION`, `DIALOG_CHANGETITLE`, `DIALOG_CHANGENICKNAME`, `DIALOG_MUSICINFO`, `DIALOG_SKILLIINFO`, `DIALOG_EVENT`, `DIALOG_CONFERENCE`, `DIALOG_GIFTVIEW`, `DIALOG_SHOPPINGBAG_ADD` | No — these parks have no live mount frame at all (ADR-0012 census) | First pin a live exe mount per sheet, then capture rest/hover/press in that host |
| Six `DIALOG_MESSENGER_*` sheets | No — geometry-only parks (#622) | Qualified mount capture, then the per-control A/B pair |

## Composition rules

- The shared pointer snapshot is the hover source. It resolves through
  staged `dialog_origin + .bnd rect` boxes, smallest rect first, so a
  button wins over its sheet backdrop. No pointer means rest.
- Press is armed on pointer-down for the open dialog and dropped on
  release. The host is part of the press key, so a stale press cannot
  light a later sheet that reuses the same control code. The one-OK and
  quit sheets close on the click; their held face is not captured, so
  that timing stays OPEN rather than being modeled.
- Room/planet chrome hover is unchanged: the board resolver returns
  `None` while a modal owns it (`docs/hover.md`), and the dialog arm
  paints only the modal's staged controls on top.
- Geometry is never hardcoded. Every repaint lands at the parsed
  placement origin; a missing sprite, figure, or record paints nothing
  (ADR-0003).

## Guards

- `dialog_frame` unit tests pin the pure state choice (rest, hover,
  active precedence, release, foreign-host press).
- `quit_confirm_ok_hover_restores_its_staged_first_frame` drives the
  live power-disc → modal → pointer-snapshot → painter chain on a
  synthetic two-record OK body. It is a composition guard, not an exe
  pixel match; the pack row above stays OPEN until A/B capture exists.

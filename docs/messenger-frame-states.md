# Messenger frame states — rest, hover, active (spec #315 story 20)

Version pin: `OTwo.exe` 7.55, 5,029,888 B, sha256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
Art census: `Interface1.opi` sha256
`d66541198965d622ff0177b306d916885ae4d64ae07b2c2264fcbdd850d50584`.

The exe's **button language** is pinned by the Wine sweep in `docs/hover.md`
for room and planet chrome: rest paints a desaturated body frame 0, hover
repaints body frame 0 at full colour, and an active selection paints body
frame 1. No qualified pack mounts a Messenger sheet or captures its
hover/press pair (`docs/captures/journey-audit/315-messenger.md:60`); the
`N-FRAMES` row stays OPEN. The shell applies the language as a labeled
stand-in: rest leaves the staged stack alone, pointer hover repaints record
0, and a held press repaints record 1. **Every sheet below has an OPEN exe
frame-state pair**; the staged records alone are not a pin.

The six sheets stage from their `BOUND` blocks with parent-relative children
(#622, #624). `stage_at_parent_origin` adds the parsed `Dialog_*` frame origin
to each child rect, leaving the frame itself at its parsed origin. Sprite
records with nonzero anchors stay local to the frame: the Messenger record
painter adds the frame origin for those records, but paints a zero-anchor
bound body at its staged child rect. It never double-offsets a staged child.
The shared dialog painter always adds the origin and is not used for these
parks. A sheet with no second record gets no active frame.

| Sheet | Parsed rects / staged placements | Control records staged | States pinned by a pack? | Next evidence step |
| --- | --- | --- | --- | --- |
| `DIALOG_MESSENGER_MAIN` | 77 / 104 (`SET` clones) | Close, Friend/Note and four chat tabs, NoteSend, page arrows, Friend/Group ADD, Chat Up/Down, ChatRoomClose carry 2 records; chat tabs, alarms, Check and RemoveBox carry 3; OnOff carries 4 | No — geometry-only park (#622) | One qualified main-sheet mount with off/on/pressed crops per tab and send; name which of the 3/4-record bodies is the active face |
| `DIALOG_MESSENGER_FRIENDADD` | 6 / 6 | OK, CANCEL and SEARCH bodies carry 2 records | No — geometry-only park | Mount the sheet after a pinned nickname result; capture off/on/pressed OK, CANCEL and SEARCH |
| `DIALOG_MESSENGER_QUESTION` | 7 / 7 | Shared OK/CANCEL body carries 2 records | No — geometry-only park | Qualified QUESTION mount with the same body at both rects; capture off/on/pressed for each control |
| `DIALOG_MESSENGER_NOTE` | 13 / 13 | Close, Search, Send, Cancel, Reply and Save bodies carry 2 records | No — geometry-only park; NOTE SEND stays P0 until a wire pin (#664) | Mounted note compose/search sheet with off/on/pressed per button; keep the send opcode OPEN |
| `DIALOG_MESSENGER_POPUPMENU` | 6 / 6 | Five button bodies carry 2 records | No — the five actions are labeled no-ops (#669) | Mounted menu with one off/on/pressed crop per action; pair the action frame before any dispatch |
| `DIALOG_MESSENGER_INFORMATION` | 5 / 5 | OK body carries 2 records | No — the B8 one-OK liveness sheet is a different sheet and its Information-style origin cannot be transferred (#345) | Qualified Messenger INFORMATION mount with pointer and held OK crops; name the OK record pair |

## Composition rules

- The state selector is pure (`screens/game/messenger_frame.rs`) and keyed by
  sheet: a press armed on one sheet cannot light another that reuses the code.
  A held press wins over hover, matching the dialog family.
- `MessengerFrame::record` returns `None` unless the staged figure carries
  that layer. Rest never repaints. No archive data is read by the selector.
- The painter consumes staged rects only (ADR-0003). A missing placement,
  figure, or record paints nothing; no literal sheet position is introduced.
- No Messenger sheet is mounted (`N-MOUNT`): the six parks stage for geometry
  and future frame work only. A live mount pin must land before a player can
  see these states; this document does not claim one.

## Guards

- `messenger_frame` unit tests pin rest, hover, active precedence, release,
  foreign-sheet press and the single-record no-active arm.
- `messenger_hover_repaints_the_parent_relative_staged_record` composes the
  state selector with `paint_messenger_frame_record` on a synthetic
  parent-relative sheet: a zero-anchor cover and a locally anchored button
  both land once at the staged rect. It is a composition guard, not an exe
  pixel match; the table above stays OPEN until each A/B capture exists.
- `messenger_frame_states_stay_open_until_a_qualified_mount_pack` reads all
  six staged sheets through the live staging chain (skips without
  install-root), proves their control bodies carry records, and keeps every
  exe pin marker at `None`.

## Close verification: #626 against #675 (`ca857c4`)

| Acceptance part | Verdict | Evidence |
| --- | --- | --- |
| Rest, hover and active states exist and are visible on a staged control | MET | `messenger_rest_leaves_the_staged_stack_alone`, `messenger_hover_selects_the_first_record_when_the_sheet_carries_it`, `messenger_held_press_selects_the_second_record_over_hover`, `messenger_release_returns_the_control_to_hover`, `messenger_press_on_another_sheet_does_not_light_the_open_sheet`, `messenger_single_record_control_has_no_active_frame`; composition guard `messenger_hover_repaints_the_parent_relative_staged_record` paints both records at the staged child rect. |
| Parent-relative placement honors the parsed `Dialog_*` origin | MET | `messenger_hover_repaints_the_parent_relative_staged_record`; live chain `main.rs::run_play` -> `assemble_online_session` -> `session::OnlineSession::stage_live_art` -> `live_art::stage_live_scenes` -> `stage_messenger_sheets` -> `stage_messenger_sheet`. |
| The three states **match the exe** per sheet | OPEN — `N-FRAMES` | No qualified mount capture names a rest/hover/active record pair. `messenger_frame_states_stay_open_until_a_qualified_mount_pack` keeps every `messenger_frame_exe_pin` at `None`; the table above names the capture needed for each sheet. The selector is a labeled stand-in, not an exe pixel match. |

Red proof (this lane): temporarily mapping rest to record 0 failed
`messenger_rest_leaves_the_staged_stack_alone`; temporarily returning a fake
pin for the Main sheet failed
`messenger_frame_states_stay_open_until_a_qualified_mount_pack`. Both
guards passed again after the sabotage was reverted. The honest OPEN marker
is part of the close: #626's row is checked for the state machinery landed
by #675, with the exe frame pair still named as the next evidence step.

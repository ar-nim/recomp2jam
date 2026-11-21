# Messenger NA census (spec #315 M10/M11)

Version pin: `OTwo.exe` 7.55, 5,029,888 B, sha256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
Geometry pin: `Interface1.opi` sha256
`d66541198965d622ff0177b306d916885ae4d64ae07b2c2264fcbdd850d50584`.

Two Messenger stories have no qualified exe frame: the main sheet is never
mounted in B6/B8 or any local pack, and no friend/note list is ever scrolled
to an end. Both rows are graded **NA-CENSUS**, never a passed parity claim.
The archive parses 114 rects across six sheets (MAIN 77, FRIENDADD 6,
QUESTION 7, NOTE 13, POPUPMENU 6, INFORMATION 5); the old 134 figure named
the adjacent `DAILOG_SELECTMUSIC` declaration and is retired.

## M10 — main sheet at 77 rects (#656)

| Field | Value |
| --- | --- |
| Verdict | NA-CENSUS — staged geometry only, no exe mount |
| Guard | `messenger_main_sheet_stays_a_seventy_seven_rect_park_without_a_mount` |
| Live chain | `main.rs::run_play` -> `assemble_online_session` -> `session::OnlineSession::stage_live_art` -> `live_art::stage_live_scenes` -> `stage_interface_scenes` -> `stage_messenger_sheets` -> `staging::stage_messenger_sheet` -> `resolve_bound_row` -> `parse_bnd` |
| Pin | `MESSENGER_MAIN_MOUNT_PIN = None`; no host `exe_response_pin` names the main sheet |
| Activation | `MESSENGER_MAIN_MOUNT_NEXT_STEP`: `N-FRAMES` plus `N-BACKEND` — one qualified exe frame with the sheet mounted, naming the friend, note and four chat tabs at its 77 rects |

The guard reads the installed archive through the live staging path and
skips cleanly when `install-root` is absent. It pins the parsed count and
placement presence, and refuses a mount pin without a capture. No sheet
opens on a host press; the B8 one-OK sheet is a different liveness notice.

## M11 — friend and note scroll clamps (#658)

| Field | Value |
| --- | --- |
| Verdict | NA-CENSUS — no mounted list has an observed end |
| Guard | `messenger_scroll_clamps_stay_na_without_a_mounted_list` |
| Scope | Seam suffices: no qualified pack mounts a friend or note column, so no live list exists whose ends a clamp could bound. The channel scroll guards pin `Screen::Channel`, not this family. |
| Pin | `MESSENGER_SCROLL_EXE_PIN = None`; no scroll opcode or page state is claimed |
| Activation | `MESSENGER_SCROLL_CLAMP_NEXT_STEP`: `N-FRAMES` — one qualified exe pack scrolling a mounted friend and note column to both ends, with the clamped frame at each end |

## Guards

- `messenger_main_sheet_stays_a_seventy_seven_rect_park_without_a_mount` —
  live chain above; NA marker keeps the mount verdict honest.
- `messenger_scroll_clamps_stay_na_without_a_mounted_list` — seam suffices:
  with no mounted list, the NA label is the complete scroll claim.

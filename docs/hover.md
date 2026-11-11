# Per-control hover behaviour — O2Jam 7.55 (Wine rig)

Date: 2026-11-10. Question (issues #33 and #26): which sprite frame or
overlay body does the 7.55 client arm when the pointer sits on a room
row panel, a room top-strip button, a chat tab, a planet emblem, the
gear disc, the exit disc, or a promo arrow? Every row below is a
measured before/after capture pair from one live client session; a
control that does not change is recorded as static with its negative
evidence, not guessed.

All captures are native 800x600 canvas pixels. Artifacts live under
`~/scratch/hover-sweep/` (git-ignored, not in the repo).

## Answer in one line per family

- **Planet emblems `Hard`/`Normal`/`Easy`**: hover paints the emblem
  body's own 84x18 light plate band (y 44-62) that the rest paint
  leaves out. No frame index change — the badge art is body frame 0 in
  both states.
- **Planet emblems `Free`/`Drum` (the QUEST / TUTORIAL plates)**:
  static. No pixel changes on hover.
- **Gear disc (`OPTION`), exit disc (`EXIT`), promo `LEFT`/`RIGHT`
  arrows**: hover repaints the control at full body **frame 0**
  (100% of the frame's opaque pixels); the rest paint is a dimmed /
  desaturated rendering of that same frame, so the frame index does not
  change either.
- **Promo `PLAYSTOP`**: the one true frame swap — rest paints frame 0,
  hover paints body **frame 1** (both 100%).
- **Room row panels `EXBUTTON1..6`**: static on the live room (six empty
  panels). No frame change and no `Room_MOver.ojs` overlay.
- **Room mode filters, top-strip buttons, `BACK`, `CREATE`,
  `QUICKJOIN`, pager `LEFT`/`RIGHT`, `MESSENGER`, `CONFERENCE`, chat
  tabs**: hover repaints the control at full body **frame 0**; the rest
  paint is the desaturated rendering. The *active* filter / tab
  (`SHOW_ALLMODE`, `CHAT_ALL`) paints body **frame 1** in both states
  and does not react to hover.

So the exe's button language is three states, not two: **rest =
desaturated body frame 0**, **hover = body frame 0 at full colour**,
**active = body frame 1**.

## Method

1. Boot `AngelO2Jam.exe` (same bytes as `OTwo.exe`) under Wine on a
   private prefix, `PULSE_SINK=wine_null` throughout, inside a wine
   virtual desktop `o2jam` 800x600 placed on my own Xvfb `:104`
   1024x768x24. Login token read at run time from
   `~/memoryer-local/O2JAM.db` `t_o2jam_login` (account `tester2`,
   `USER_INDEX_ID` 2) and minted into the launcher blob with the
   documented RSA field codec (`docs/wine-boot.md`, analysis method).
   I used `tester2` because the shared Memoryer gateway also carries a
   live `tester` session from another rig.
2. Every pointer move is a `SendInput` absolute motion from a
   `winegcc` helper that joins the `o2jam` desktop
   (`~/scratch/hover-sweep/hovctl.c`); XTEST never reaches the
   DirectDraw canvas. Each hover point was entered with a straight
   motion plus a 2 px jiggle, so the client always sees a fresh
   `WM_MOUSEMOVE`, and the pointer then stayed still.
3. Per control: park the pointer on a neutral point, capture `offA`,
   move onto the control, capture `on`, move away again, capture
   `offB` (`drive2.py`). A row counts as a measured hover only when
   `offA` and `offB` agree inside the control rect (no scene
   animation there) and `on` differs; otherwise the row is marked
   noisy and was not used. The room animates one strip (the avatar / user
   list, measured bbox 485,50-726,255), which overlaps the right-column
   panel rects; all six panel rows still came back with `offA`/`offB`
   equal inside the rect (`restmax` 0), so their static verdict stands.
4. Diff = per-pixel max channel difference on the raw crops; "changed"
   counts any non-zero pixel, "strong" counts >24/255. Bounding boxes
   are in native pixels.
5. Frame attribution: each control's body is decoded straight from
   `Interface1.opi` (sprite-55 envelope, `docs/file-formats/opi.md`),
   each record composited at its own anchor, and compared with the
   capture on the record's opaque pixels at tolerance 30/255
   (`match_frames.py`, `ojs.py`). "frame N (X%)" = share of that
   frame's opaque pixels that match the capture.

Repeatability: the planet sweep reproduces byte-identical changed-pixel
counts across two independent runs (1456 / 1477 / 1476 / 615 / 654 /
120 / 224 / 120).

## Controls measured

| scene | control | code | staged body (records) | capture pair (`~/scratch/hover-sweep/`) | diff bbox (native px) | changed px | rest paint | hover paint |
|---|---|---|---|---|---|---|---|---|
| planet | `IDC_BUTTON_SPLANET_HARD` | `0x19201002` | `Planet_Btn_Hard.ojs` (2) | `planet-emblem-hard-offA.png` / `-on.png` | 343,44-427,62 | 1456 | frame 0 91% (plate band missing) | frame 0, plate band added (99%) |
| planet | `IDC_BUTTON_SPLANET_NORMAL` | `0x19201003` | `Planet_Btn_Normal.ojs` (2) | `planet-emblem-normal-offA.png` / `-on.png` | 213,44-297,62 | 1477 | frame 0 86% (plate band missing) | frame 0 (98%) |
| planet | `IDC_BUTTON_SPLANET_EASY` | `0x19201004` | `Planet_Btn_Easy.ojs` (2) | `planet-emblem-easy-offA.png` / `-on.png` | 94,44-178,62 | 1476 | frame 0 86% (plate band missing) | frame 0 (98%) |
| planet | `IDC_BUTTON_SPLANET_FREE` (QUEST plate) | `0x19201005` | `Planet_Btn_Free.ojs` (2) | `planet-emblem-free-offA.png` / `-on.png` | none | 0 | frame 0 56% (dim, disabled) | static, no change |
| planet | `IDC_BUTTON_SPLANET_DRUM` (TUTORIAL plate) | `0x19201006` | `Planet_Btn_Drum.ojs` (2) | `planet-emblem-drum-offA.png` / `-on.png` | none | 0 | frame 0 55% (dim, disabled) | static, no change |
| planet | `IDC_BUTTON_SPLANET_OPTION` (gear) | `0x19200011` | `Planet_Btn_Option.ojs` (2) | `planet-option-offA.png` / `-on.png` | 714,565-743,594 | 615 (348 strong) | neither frame (58% / 34%) | frame 0 (100%) |
| planet | `IDC_BUTTON_SPLANET_EXIT` | `0x19200010` | `Planet_btn_Exit.ojs` (2) | `planet-exit-offA.png` / `-on.png` | 754,565-783,594 | 654 (548 strong) | neither frame (51% / 44%) | frame 0 (100%) |
| planet | `IDC_BUTTON_SPLANET_PROMOTION_LEFT` | `0x1920000D` | `Planet_Btn_Promotion_Left.ojs` (2) | `planet-promo-left-offA.png` / `-on.png` | 358,514-371,529 | 120 (99 strong) | neither frame (83% / 82%) | frame 0 (100%) |
| planet | `IDC_BUTTON_SPLANET_PROMOTION_PLAYSTOP` | `0x1920C00F` | `Planet_Btn_Promotion_PlayStop.ojs` (6 raw 26x24 records) | `planet-promo-playstop-offA.png` / `-on.png` | 392,515-407,529 | 224 (222 strong) | record 0 (100%) | **record 1** (100%) |
| planet | `IDC_BUTTON_SPLANET_PROMOTION_RIGHT` | `0x1920000E` | `Planet_Btn_Promotion_Right.ojs` (2) | `planet-promo-right-offA.png` / `-on.png` | 428,514-441,529 | 120 (97 strong) | neither frame (83% / 82%) | frame 0 (100%) |
| room | `IDC_BUTTON_SROOM_EXBUTTON1` | `0x1301201B` | `Room_ButtonRoom.ojs` (8) | `room-row-panel-1-offA.png` / `-on.png` | none | 0 | panel art | static, no change |
| room | `IDC_BUTTON_SROOM_EXBUTTON2` | `0x1301201C` | `Room_ButtonRoom.ojs` (8) | `room-row-panel-2-offA.png` / `-on.png` | none | 0 | panel art | static, no change |
| room | `IDC_BUTTON_SROOM_EXBUTTON3` | `0x1301201D` | `Room_ButtonRoom.ojs` (8) | `room-row-panel-3-offA.png` / `-on.png` | none | 0 | panel art | static, no change |
| room | `IDC_BUTTON_SROOM_EXBUTTON5` | `0x1301201E` | `Room_ButtonRoom.ojs` (8) | `room-row-panel-4-offA.png` / `-on.png` | none | 0 | panel art | static, no change |
| room | `IDC_BUTTON_SROOM_EXBUTTON4` | `0x1301201F` | `Room_ButtonRoom.ojs` (8) | `room-row-panel-5-offA.png` / `-on.png` | none | 0 | panel art | static, no change |
| room | `IDC_BUTTON_SROOM_EXBUTTON6` | `0x13012020` | `Room_ButtonRoom.ojs` (8) | `room-row-panel-6-offA.png` / `-on.png` | none | 0 | panel art | static, no change |
| room | `IDC_BUTTON_SROOM_SHOW_ALLMODE` (active) | `0x13201003` | `Room_Btn_Show_AllMode.ojs` (2) | `room-mode-all-offA.png` / `-on.png` | none | 0 | frame 1 (100%) | frame 1, no change |
| room | `IDC_BUTTON_SROOM_SHOW_NORMALMODE` | `0x13201004` | `Room_Btn_Show_NormalMode.ojs` (2) | `room-mode-normal-offA.png` / `-on.png` | 112,22-201,62 | 3174 (2840 strong) | neither frame (43% / 8%) | frame 0 (100%) |
| room | `IDC_BUTTON_SROOM_SHOW_BATTLEMODE` | `0x13201005` | `Room_Btn_Show_BattleMode.ojs` (2) | `room-mode-battle-offA.png` / `-on.png` | 202,22-291,62 | 3385 (3210 strong) | neither frame (36% / 8%) | frame 0 (100%) |
| room | `IDC_BUTTON_SROOM_MUSICSHOP` | `0x13200037` | `Room_MusicShop.ojs` (2) | `room-btn-musicshop-offA.png` / `-on.png` | 317,2-384,31 | 1800 (946 strong) | neither frame (35% / 21%) | frame 0 (100%) |
| room | `IDC_BUTTON_SROOM_ITEMSHOP` | `0x13200008` | `Room_ItemShop.ojs` (2) | `room-btn-itemshop-offA.png` / `-on.png` | 399,2-457,31 | 1528 (728 strong) | neither frame (63% / 26%) | frame 0 (100%) |
| room | `IDC_BUTTON_SROOM_MYROOM` | `0x13200009` | `Room_MyroomBtn.ojs` (2) | `room-btn-myroom-offA.png` / `-on.png` | 484,2-550,31 | 1730 (840 strong) | neither frame (45% / 24%) | frame 0 (100%) |
| room | `IDC_BUTTON_SROOM_COUPON` | `0x1320000A` | `Room_Btn_Coupon.ojs` (2) | `room-btn-coupon-offA.png` / `-on.png` | 561,2-627,31 | 1748 (848 strong) | neither frame (55% / 14%) | frame 0 (100%) |
| room | `IDC_BUTTON_SROOM_FIRSTSTEP` | `0x13200032` | `Room_FirstStep.ojs` (2) | `room-btn-firststep-offA.png` / `-on.png` | 660,1-718,32 | 1679 (1055 strong) | neither frame (31% / 22%) | frame 0 (100%) |
| room | `IDC_BUTTON_SROOM_OPTION` | `0x1320000B` | `Room_Option.ojs` (2) | `room-btn-option-offA.png` / `-on.png` | 728,2-785,31 | 1573 (735 strong) | neither frame (53% / 35%) | frame 0 (100%) |
| room | `IDC_BUTTON_SROOM_CHAT_ALL` (active) | `0x1320100E` | `Room_ChatAll.ojs` (2) | `room-chat-all-offA.png` / `-on.png` | none | 0 | frame 1 (100%) | frame 1, no change |
| room | `IDC_BUTTON_SROOM_CHAT_FRIEND` | `0x1320100F` | `Room_ChatFriend.ojs` (2) | `room-chat-friend-offA.png` / `-on.png` | 536,560-558,582 | 351 (305 strong) | neither frame (40% / 31%) | frame 0 (100%) |
| room | `IDC_BUTTON_SROOM_CHAT_GUILD` | `0x13201010` | `Room_ChatGuild.ojs` (2) | `room-chat-guild-offA.png` / `-on.png` | 562,560-584,582 | 335 (302 strong) | neither frame (40% / 31%) | frame 0 (100%) |
| room | `IDC_BUTTON_SROOM_CHAT_WHISPER` | `0x13201011` | `Room_ChatWhisper.ojs` (2) | `room-chat-whisper-offA.png` / `-on.png` | 588,560-610,582 | 320 (257 strong) | neither frame (48% / 33%) | frame 0 (100%) |
| room | `IDC_BUTTON_SROOM_BACK` | `0x13200002` | `Room_Back.ojs` (2) | `room-btn-back-offA.png` / `-on.png` | 713,571-791,591 | 1555 (1540 strong) | neither frame (7% / 6%) | frame 0 (100%) |
| room | `IDC_BUTTON_SROOM_CREATE` | `0x13200007` | `Room_Create.ojs` (2) | `room-btn-create-offA.png` / `-on.png` | 508,372-603,406 | 1674 (1076 strong) | neither frame (52% / 27%) | frame 0 (93%; the 17x22 icon cell at local 4,12 also changes) |
| room | `IDC_BUTTON_SROOM_QUICKJOIN` | `0x13200006` | `Room_QuickJoin.ojs` (2) | `room-btn-quickjoin-offA.png` / `-on.png` | 389,372-484,403 | 1626 (1081 strong) | neither frame (62% / 32%) | frame 0 (100%) |
| room | `IDC_BUTTON_SROOM_LEFT` | `0x1320000C` | `Room_Left.ojs` (2) | `room-btn-left-offA.png` / `-on.png` | 269,371-313,388 | 759 (656 strong) | neither frame (30% / 19%) | frame 0 (100%) |
| room | `IDC_BUTTON_SROOM_RIGHT` | `0x1320000D` | `Room_Right.ojs` (2) | `room-btn-right-offA.png` / `-on.png` | 317,371-360,388 | 769 (666 strong) | neither frame (29% / 19%) | frame 0 (100%) |
| room | `IDC_BUTTON_SROOM_MESSENGER` | `0x13200048` | `Btn_Messenger.ojs` (2) | `room-btn-messenger-offA.png` / `-on.png` | 627,571-662,590 | 434 (327 strong) | neither frame (50% / 21%) | frame 0 (100%) |
| room | `IDC_BUTTON_SROOM_CONFERENCE` | `0x1320003D` | `Btn_Conference.ojs` (2) | `room-btn-conference-offA.png` / `-on.png` | 670,571-705,590 | 450 (393 strong) | neither frame (51% / 28%) | frame 0 (100%) |

Rect geometry: `State_Room.bnd` (75 rects) and `State_Planet.bnd` (17
rects) from `Interface1.opi`, paired by control ordinal
`(code & 0xFFF) − 1` (`docs/opi-lessons.md`, `docs/bnd-semantics.md`);
sprite anchors in the bodies agree with those rects byte-exact.

## What the captures settle per issue

### Issue #33 — room

- Every staged room button that is not the active one repaints itself at
  full colour (body frame 0) under the pointer and returns to the
  desaturated look when the pointer leaves. The rest/hover colour
  difference is large: chroma (max−min channel) rises from 20 to 154 on
  `BACK`, from 39 to 92 on `CHAT_FRIEND`, from 39 to 109 on
  `SHOW_NORMALMODE`; mean luminance stays flat (ratio 0.92-1.14), so the
  change is colour restoration, not a brightness bump.
- The active filter (`SHOW_ALLMODE`) and the active chat tab
  (`CHAT_ALL`) paint body **frame 1** and do not change on hover. Any
  inactive filter/tab hovered paints its **frame 0**.
- The six row panels do **not** change: 0 changed pixels on all six
  panels, and `Room_MOver.ojs` (the 292x95 mouse-over overlay body, one
  record) is not painted at a panel origin in either state: 0 of its
  2257 opaque pixels match at the probe offsets (22,78) and (313,173),
  in both the rest and the hovered capture of panels 1 and 5.

### Issue #26 — planet board

- The three playable family emblems add a **84x18 plate band** above the
  badge: HARD 343-427, NORMAL 213-297, EASY 94-178, all at y 44-62. The
  band's pixels match the emblem body's own frame-0 pixels at that
  offset within the 555→888 tolerance (±30/255: EASY 1584/1584 compared
  pixels; only 142/1492 are byte-exact, so this is art identity, not a
  byte copy), and nothing else in the 99x112 (146x112 for HARD) badge
  changes. The badge frame index does not change on hover; frame 1 is
  the *selected family* art (orange ring, dark plate), a click state:
  while the channel dialog was open on the connected EASY family, EASY
  matched frame 1 on 10031/10203 opaque pixels (98%), against 7291 for
  frame 0 (`~/scratch/hover-sweep/room-01.png`).
- `FREE`/`DRUM` — the plates captioned QUEST / TUTORIAL — are static:
  0 changed pixels. They are also painted dim (rest matches frame 0 on
  55-56% of its opaque pixels, against 86-91% for the live families).
- The gear disc (`OPTION`), the exit disc (`EXIT`) and the promo
  `LEFT`/`RIGHT` arrows brighten to their exact body frame 0 (100%)
  over a dimmed rest paint; the change stays inside the control
  (gear 714-743 x 565-594 in a 38x38 body, exit 754-783 x 565-594,
  arrows 14x16).
- The promo `PLAYSTOP` control is the only element that swaps frames:
  rest = raw record 0, hover = raw record 1 (both 100% by direct
  record-vs-capture match over all six 26x24 records, 2026-11-11: rest
  624/624 on rec0, hover 624/624 on rec1; the old two-record model
  described the frame pairing, not the record count).

## Findings that change the shell's plan

1. The exe has **three button states**, and the shell currently paints
   only the bright one. Faithful painting is: rest = desaturated body
   frame 0, hover = body frame 0, active = body frame 1. Pinning the
   rest transform needs no new art — every body already carries the two
   records.
2. The planet emblem hover is **not a frame swap**; it reveals the plate
   band that the rest pass omits. A shell highlight for the emblems must
   draw that band (it is inside the emblem body's own frame 0 at the
   measured offsets), not frame 1.
3. The room row panels are **static in the empty-room room**. Any row
   highlight must stay labelled OPEN until a populated room list is
   captured.

## OPEN (with the next evidence step)

1. **Rest-state rendering rule (the desaturated look).** Measured, not
   explained: rest pixels match neither body record (7-63% of frame 0)
   while hover matches frame 0 at 100%. Next step: read the button
   draw-state word for one room control with a stopped-state winedbg
   memory read (`docs/reversing.md` winedbg protocol; `emblem`/button
   descriptor at parent `+0xF80`, stride `0x118`), or trace the palette
   path behind the draw call.
2. **A populated room row.** The live room had no open rooms, so the
   `SET 0x11` row template clones were hidden. A transient orange
   outline on one panel appeared once, immediately after the room
   entry click (`~/scratch/hover-sweep/room-03.png`), and was gone after
   the next click; pointer motion alone never reproduced it. Next step:
   seat a room through the gateway (or a second client) and hover the six
   populated panels, then re-check `Room_MOver.ojs` at the panel
   origins.
3. **`CREATE`'s extra 17x22 cell.** Hover matches frame 0 on 93% of its
   opaque pixels; the rest sits at local (4,12)-(21,34). Next step:
   identify whether that cell is a child control body (the `SET` clone
   tree of the button) and capture it alone.
4. **Emblem plate mechanism.** The plate appears only when hovered, and
   its pixels are the frame-0 art's own plate pixels; whether the exe
   clips the plate at rest or paints the band only on hover is not
   pinned. Next step: same button-descriptor memory read as (1), at the
   emblem control, comparing the armed frame index in both pointer
   states.

## Provenance

| item | value |
|---|---|
| Client | `AngelO2Jam.exe` (= `OTwo.exe`), 5,029,888 B, SHA-256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b` |
| Art archive | `Interface1.opi`, 75,100,702 B, SHA-256 `d66541198965d622ff0177b306d916885ae4d64ae07b2c2264fcbdd850d50584` |
| Scene rects | `State_Room.bnd` @ `Interface1.opi` +61,451,646 (75 rects); `State_Planet.bnd` @ +61,450,578 (17 rects) |
| Control list | `ControlList_Interface.txt` from the same archive (`STATE_ROOM` `0x36`, `STATE_PLANET` `0x11`) |
| Wine | 11.21 (wow64), `winegcc` 16.2.1 helper `hovctl.exe.so` |
| Xvfb | `:104`, 1024x768x24; wine virtual desktop `o2jam` 800x600 at (0,0); captures cropped to the 800x600 desktop = native canvas pixels |
| Audio | `PULSE_SINK=wine_null` for the whole session (no sound to the speakers) |
| Gateway | Memoryer 7.2.0 on `127.0.0.1:15010`; token read from `~/memoryer-local/O2JAM.db` `t_o2jam_login` at run time, account `tester2` (`USER_INDEX_ID` 2) |
| Scene path | planet board → EASY emblem click → channel dialog → network popup OK (400,365) → 입장 (526,248) → `STATE_ROOM` → BACK (752,581) → planet board |
| Tools | `~/scratch/hover-sweep/{hovctl.c,cap.sh,drive2.py,match_frames.py,ojs.py,gen_table.py}`; raw pairs, crops and the sweep JSON (`planet-sweep.json`, `room-sweep.json`) in the same directory |

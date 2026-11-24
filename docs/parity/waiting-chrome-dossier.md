# Waiting-chrome residuals (#883, #884) — static pins and shell fixes

Target: O2Jam 7.55 `OTwo.exe`, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
`VA = file offset + 0x400000`. Tool: REA MCP, Ghidra 12.2, target
`target-83a045418b62.exe` (session evidence
`ev_696158c42763ef469cb735213401bf31c2ad2491166f43c4e42f3f17adba8e7d`).
No disassembly or copyrighted bytes are copied here; behavior is
described in our own words with file offsets and evidence IDs.
Geometry comes from the parsed `ControlList` + `State_Waiting.bnd` pairs,
never from literals in the painter.

## 1. Random `?` cards — PINNED (both issues)

- `FUN_004ad350` (entry file `0xad350`, dossier
  `ev_b3755aabd025cdf0f695783ed706bffc07583d1d9a89dc36ce38a9021d0ee096`)
  is the song-change applier. It range-checks the low 16 bits of the room
  word, then branches on the variant nibble (the high 16 bits):
  - plain song: `RANDMAP` (`0x1650004F`, control-search matches at files
    `0xad46d`/`0xad555`) takes frame 0; it composes
    `Wait_Effect<songid>_1.ojs` / `_2.ojs` from the strings at file
    `0x249868` and the table at `0x2496b0`.
  - random song: `RANDMAP` takes frame 1 and the two effect controls
    `EFFECT_1/2` (`0x16500045/46`) receive the fixed bodies
    `Wait_Effect_Rand1.ojs` / `Wait_Effect_Rand2.ojs` (strings at files
    `0x24988c`/`0x249874`, instruction matches for the two control
    codes at files `0xad75b`/`0xad795` and `0xad872`). The same function
    then sets a frame on each of those two controls and starts their
    animation.
- Byte scans for the two control immediates
  (`ev_e94fdec1fd56bf4579ed1d91f01ec6f92f4132f8d7192eaad95ad9ea630c73f9`,
  `ev_4587850db441cf3f7d097a6234e9d8d0ccf8e5b7f45d42be14ad880ce836359b`)
  place every reference in this applier or its music-select twin
  `FUN_004b3a70` (file `0xb3a70`). The same branch exists in both
  classes, so one pin covers the waiting-master and music-select cards.
- `ControlList_Interface.txt` (archive slot 1595) authors
  `IDC_IMAGE_SWAIT_EFFECT_1/2 0X16500045/46` without a sprite, and
  `State_Waiting.bnd` pairs them by ordinal `(code & 0xFFF) - 1` with
  rects 68/69 `(719,450)-(779,498)` / `(719,507)-(779,555)`. The code
  mounts the body; the `.bnd` owns the rect.
- Archive read-only inventory (`Interface1.opi`, slot 1269/1270):
  both fixed bodies are one raw RGB555 60x48 image at anchor `(0,0)`,
  colorkey `0x7D5F`, and carry the striped `?` card. The big card
  `Wait_Map_Random.ojs` (slot 1276) is a 89x65 copy of the same art.
- Shell fix: while `WaitingCaptions.random` stands, the painter inks
  both staged effect rects from the staged Random card body. The body
  selection is presentation; the rects stay parsed. With no staged
  card body, or on a plain song, both rects stay empty.

## 2. Bottom-left white chat bubble — PINNED (issue #883)

- The purple face is the staged `MESSENGER` button at bound rect 92
  `(668,569,40x24)`. The extra white bubble in our frame sits at rect
  94, `IDC_IMAGE_SWAIT_MESSENGER_ALRAM1 0x1650006C`, bound to
  `(657,545,34x29)` — the `Wait_Messenger_Alram_PopUp.ojs` body, not
  a room emblem or chat icon.
- `FUN_004c0690` (entry file `0xc0690`, dossier
  `ev_f04e5757fe3fa175927a4346b021eeddcea80bee8861aad01f48703be3192d42`)
  loops over `0x1650006B + index`, i.e. both lamps. A nonzero argument
  starts an 800 ms animation; a zero argument only calls the stopper
  `FUN_0042f4f0` (file `0x2f4f0`, dossier
  `ev_893e8c67a4c3d96a85b782534c997726522bb95056b4f13912bd8dd410cc5461`),
  which clears the animation word and restores its rest frame. It does
  not paint a lamp body. Callers
  (`ev_6b34fcb3365fc5560694f449ef0506473409dfaf499475fd4762102993a38f63`)
  are the Waiting `MESSENGER` press (`0x4b81e0`, file `0xb81e0`, with 0)
  and six messenger-dialog callbacks. No live alert stands in the
  quiesced pair, which shows only the purple button.
- Shell fix: both lamp bodies stay unpainted until an alert stands; the
  ON interval remains the existing 800 ms seam (individual animation
  frames stay OPEN — next step: one qualified messenger-alert capture).

## 3. START vs READY and the enabled skill arrows — PINNED (#884)

- The role gate is the local seat's `+0x30` master word, not peer
  presence or the local ready byte. `FUN_004ae350` (file `0xe350`,
  dossier `ev_aa74bced2e8264910180f80e1642cd4aa751dc3e64570b7ddb61ac7310b08663`)
  reads byte `0x77cd30` at file `0xe38e`: master enables START
  (`0x16202012`, file `0xe3d3`) and clears READY (`0x16202013`, file
  `0xe3ff`); guest sets READY frame 1 and disables START (file
  `0xe805`). This matches the earlier map in `docs/room-enums.md`.
- The writer is `FUN_005400b0` (file `0x1400b0`; batch dossier
  `ev_5e025271413ea6e8523be2e9a9deb39ad28191142fa97044d2ae98a4df31cba4`),
  which stores the argument at seat `+0x30`. Its network caller
  `FUN_00559b50` (file `0x159b50`, dossier
  `ev_3cad68dd2f4463f40bae572f3ca4f02b52cc547b6e95ee38d25a131d3eb2462b`)
  reads one byte from the 4043 live-state header, stores it as the local
  member id through `FUN_0053fff0` (file `0x13fff0`, dossier
  `ev_69f3bfa8aae5d068678fc9da08f3f0c49d9eb03301f911e739b4d8693063fdd0`,
  writing seat `+4` / `DAT_0077cd04`), then branches on that stored id
  when writing `+0x30`. The joined-room snapshot
  (3003 `RoomInfo`) carries the same word as `RoomMember.master`; the
  3007 leave promotion names the announced master id. Direct `0x77cd30`
  reads (`ev_767d75e2...`) and per-instruction reads
  (`ev_db71843f...`, `ev_3dacddb1...`, `ev_5e3f393e...`) confirm that
  the label, MUSELECT and START gate all read this word. Full IDs:
  `ev_767d75e2a70846b2454af133ecc6381b2381a371e62d4db57f7e199a04fc442f`,
  `ev_db71843f9bdf6408d7abb329b27e48fd3d694544bed0aae791487aa3f2104d8f`,
  `ev_3dacddb1fa9f00c69023493222c552c4fce035992aef48cef9c359b7636114f7`,
  `ev_5e3f393edc17f73450c45d19515f43e78a8b87095c7a083cc2b2fa1c1fef0434`.
- `FUN_004b4a00` (file `0xb4a00`, dossier
  `ev_7e1e50fd329664ebacb23214247916f1d24a559f0cafa686cba71759a3d8c547`)
  is the master arm: it enables START and all six skill arrow controls
  `0x16200050..55`, and clears the three gray skill covers
  `0x1650005B..5D`. `FUN_004b4d30` (file `0xb4d30`, dossier
  `ev_87a8bffa39dbb33077311ecd66b3aa361a2296690557ef09a83854814a6c2baf`)
  is the guest arm, but only its no-skill-selection branch disables all
  arrows and sets every skill cover to frame 1. With a skill selected it
  arms `SKILLSEL` and the first two arrows (`0x16200050/51`), disables
  the last four (`0x16200052..55`), and sets covers `0x1650005B/5C` to
  frame 2 and `0x1650005D` to frame 0. `FUN_00409bf0` (file `0x9bf0`, dossier
  `ev_694dfc15dd90b0d7d13d1aa9525d030825dfd20c458cdd894cca987bfcfe282a`)
  writes the control's `+0x24` arm word (1 enabled, 0 disabled); this
  is the enabled-state tint, not a new sprite frame.
- Archive inventory: `Wait_1/2/3SkillLeft/Right.ojs` (slots 1161-1166)
  hold the bright arrow art; `Wait_Cover_SkillLeft/Right.ojs` (slots
  1589/1590) hold the gray disabled arrows; `Wait_No_skill_Image.ojs`
  (slot 1280) is the X box. The quiesced exe frame shows bright arrows
  with gray X boxes; our guest-painted frame showed the gray covers.
- Shell fix: a 3007 handoff id wins over member rows; without a handoff,
  the joined snapshot's local `RoomMember.master` flag wins. When no
  seat claims master and no handoff stands, it keeps the seat-0 default
  for rooms staged without member rows. Both
  `ScreenGame::is_room_master` and the Waiting painter consume that
  helper, so START/READY, the enabled arrows and the master emblem
  agree with one wire field.

## Refutation / remaining OPEN

- The v6 master-name delta (`사람손` exe vs `서이이` ours) has OPEN cause.
  The earlier “different accounts” claim is withdrawn: the v3 and v6
  recipes both log in as user 1 and both hosts use user 3, so the
  attribution is unsupported. Per the owner A/B rule its next evidence
  step is wire plus server log from the two legs. No paint change is
  warranted either way.
- The messenger ON animation frames, the code-mounted normal-song
  effect bodies `Wait_Effect<songid>_1/2.ojs`, and the exact pixel path
  from the control `+0x24` word to the arrow tint remain OPEN. The
  enabled/disabled gate and the art bodies are pinned; a qualified
  alert/arrow capture is the next step for the frame-level claim.
- No v7 re-pair was run in this lane; SSIM impact is an expectation,
  not a measured result.

## Guards and red proofs (worktree `cb2/lane-waiting-chrome`)

| Guard | Red before fix | Green after |
|---|---|---|
| `random_song_inks_both_mini_cards_from_the_staged_card_body` | FAIL (`0` vs `255`) | PASS |
| `messenger_lamps_hide_at_rest_and_ink_only_on_an_alert` | FAIL (rest rects inked) | PASS |
| `wire_master_flag_paints_start_and_the_live_skill_arrows` | FAIL (`is_room_master` false) | PASS |
| `handoff_master_id_wins_over_stale_member_rows` | FAIL (local `+0x30` flag won) | PASS |

Logs: `~/scratch/waiting-chrome/red-random.log`, `red-lamp.log`,
`red-role.log`, `green-focused2.log`, `shell-lib2.log`.

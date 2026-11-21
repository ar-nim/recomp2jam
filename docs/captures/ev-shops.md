# ev-shops — master-session shop evidence leg (one shared session)

Trace id: `S-EVSHOPS-MASTER-01`. Base: `95d50ac`. Lane worktree:
`/home/arnim/Projects/recomp2jam-wt-ev-shops` on `cb2/ev-shops`; private data
pool `~/scratch/install-data-ev-shops/slot1` (symlinks verified with
`readlink -e`). No push, no close, no create, no merge, no remote config
write. The exe is SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`,
5,029,888 B. Shared Memoryer: pid 37333, boot epoch 1795246022 (never
restarted or signalled). All boots held one outer live-lane flock; every
client inherited it. Wine input used the SendInput helper only; no blind
Escape. The `.dmp` Cancel aim `(449,329)` was checked by frame on each
Wine boot (no dialog was up). Rust peer boot used
`RIG_DISTINCT_CLIENT_IDS=1` and uid 3 (uid 2's token was already
channeled elsewhere and returned `1001 -2`), with display pin
`:712`, `RIG_RUN=1`, and `PIPEWIRE_NODE` on the run's own null sink.

Evidence class: EXE-LIVE-RAW and RUST-LIVE-RAW (framebuffer frames plus
pass-through wire logs and audio sidecars, in git-ignored scratch).
These packs ended mid-orchestration and have **no** `run-manifest.sh`
manifest or `rig-provenance.sh` citation; no hash here is a citable
pack hash. Scratch root: `~/scratch/ev-shops-session/`.

## Boot ledger (hard bound: three exe windows, all spent)

| Window | Pack | Outcome | Ledger mode |
|---|---|---|---|
| 1 | `~/scratch/ev-shops-session/master` + `peer` (`:711`/`:712`) | After the launch wrapper's local-variable fault, preflight was re-run; `C2S 2004` fired at 21:16:42 with the CREATE dialog up. The driver expected `S2C 2005` for the room number, but this server answered `2006`/`4001`; that parse failed before any shop click. A later recovery hit the CREATE sheet with clicks landing nowhere, then `FFF0` ended the session. | driver/setup fault (twice), room-number parse assumption |
| 2 | same paths, second boot (`:711`) | `#731` gate PASS again (`07DA@+1.0s`, `07E8@+1.0s`). `C2S 2004` at 21:31:44; closing the CREATE sheet with its X chrome and pressing Waiting BACK returned to the room stage, but BACK also sent `C2S 3005` and the server answered `2007 RoomRemoved`. The master role was gone before the first shop click. | role-loss by Waiting BACK; driver/setup fault |
| 3 | same paths, third boot (`:711`) | Guest-role exe reached a qualified Item Shop entry frame and emitted `C2S 5006`. The two `side-back` clicks landed on Planet, not Room; the later Music Shop clicks never opened the sheet and `FFF0` fired. The DOWN_START press was sent on Planet and is not admissible as a shop frame. | scene-navigation fault; no-response fuse |

All three boots used the #731 prerequisites: both injector artifacts
(`relay-fastfail.so`, `sendinput-helper.exe.so`), `ECONNREFUSED` in the
interposer log, and `07DA`/`07E8` within one second of `03EC` on the tap
before any CREATE click. Wire timeline for window 2:
`~/scratch/ev-shops-session/master/interposer2.log` sha256
`ba04700c1a91eaf1eab5896e6c1affbbefff0521392bd2a41cf2cd78d5359a34`;
wire log sha256
`91adda124f0061d74f49f9d4a9188f1fe77a19d117a5f4393ef0c724428187ee`.

## #387 — Item Shop privacy: verdict NA-DESIGN (static + live corroboration)

- Exe Item Shop entry (window 3, guest seat):
  `artifacts/screenshots/itemshop-master.png` sha256
  `92cc37515fe7d588b78db03e45fea5ea275e2545fef4ed7d60d85bbc80d76693`.
  Four goods cards, won `4500`, gem `0`, wallet `G 15527 / W 0`, MY BAG
  face at entry; `C2S 5006` on entry only.
- Rust peer observer frame while that shop was open:
  `peer/states/peer-vis-itemshop.png` sha256
  `c6edd59d918f571d0db0f9d349d1b21827c7347093ed9c36a787e7c460d5a154`.
  It stays on the Room board. Its wire log contains no 5006/5007 or shop
  status frame; the peer sees no shop status or location.
- Static model: one opcode sweep over 5000..=5100 pins 5012 as the only
  known request. Guard
  `item_shop_has_no_visitor_request_in_the_shop_family` (commit `acf7462`).
  Mutation red proof: adding a fake `ViewOtherShop = 5041` made it fail
  with `left: [5012, 5041] / right: [5012]`; the mutation was reverted.
- Peer audio sidecar: `route=sink`, `on_default_sink=0`,
  `preboot_breach=0`. The exe window-2 sidecar after `rig-run.sh stop`
  measured peak `0.0 dBFS`, mean `-14.9 dBFS`.
- Caveat: this live pair was a guest-role seat, not a qualified master.
  The privacy model does not depend on role; the master-specific
  acceptance stays covered by the static sweep, not by a master pixel.

## #396 — master Item Shop board and wallet: OPEN

No qualified master Item Shop frame exists. Window 2 proves that direct
CREATE auto-enters Waiting; the CREATE sheet stays over the Waiting seat
after `C2S 2004`, and Waiting BACK destroys the room (`3005`/`2007`), so
the exe is no longer master when it reaches the room stage. Window 3
produced only the guest-role board hash above.

Next recipe (one boot): start the patched `wine-room-host.py` as the
channel host (uid 3, `leave` command added as in
`~/scratch/evidence-shops/wine-room-host-shop.py`); drive the exe in as
guest; after `0BBA` queue host `leave`, read `S2C 3007 new_master=1`,
dismiss the following CREATE sheet with its X chrome at `(553,87)`, and
**find a non-destructive Waiting-to-side entry** before clicking MyRoom.
If no such entry exists, the ticket is a design-level NA for "role never
changes the shop" and needs owner confirmation. Then capture
`MyRoom → Item Shop` with the tap and compare the wallet word from
`5029` with the guest frame `92cc3751…`.

## #401 — master Music Shop sheet and catalog: OPEN

No qualified master Music Shop frame. The window-3 `musicshop-master.png`
capture was labelled by a weak blue fraction (0.128) while the pixels
show Planet; it is a mislabel, not evidence. Next recipe: the same
legitimate master boot as #396, followed by the room Music Shop plate
`(350,16)` from the room stage; require the measured blue signature
(`SIDE_SIGNATURES["musicshop"]` minimum 0.40) and a `C2S 5006` on the
tap before naming the frame. Compare the five catalog rows, rank panel,
and `G/W` words against the qualified guest hash `964be986…` from
`~/scratch/story-shops/mshop-wire4/exe`.

## #520 — Music Shop download completion: OPEN

No qualified download-completion frame. The chart-hiding fixture was
reversible and restored: `~/scratch/run-full/Music/o2ma757.ojn` was moved
to `o2ma757.ojn.ev-shops-hidden` for window 3 and moved back after the
run (`ls -l` confirms the 179,182-byte original at the original path).
The DOWN_START aim `(740,324)` was pressed on Planet after the navigation
fault; the tap delta was zero frames, and a NULL wire from the wrong
scene is not a 5012 pin. The existing inert controls guard
`music_shop_download_controls_stay_inert_until_a_5012_send_is_pinned`
(#410) still describes the shop-side codes; the room-song FTP pump is a
different state (`docs/music-availability.md` §4).

Next recipe: boot with a missing chart in the exe install, reach a
*qualified* Music Shop sheet (blue signature >= 0.40 plus `C2S 5006`),
capture the row's `Free`/download face, press DOWN_START at `(740,324)`
with SendInput, then poll frames and the tap for `4038` progress,
`5012`, or a NULL wire. If the FTP endpoint never answers, report the
notice frame and keep 5012 OPEN; do not infer an announce from the
inert control codes.

## Fixture and containment notes

- Window 3's audio sidecar was still open at capture time
  (`route=unseen`, no peak yet); only the peer's `route=sink` and
  window 2's stopped sidecar carry a level. No audio reached the owner's
  default sink (`on_default_sink=0` on both peers/master).
- No repo game data was moved. Only the private scratch exe install had
  one chart hidden for one window, and it was restored.
- No spec row was ticked for #396, #401, or #520. Only #178 story 39
  (#387) was ticked, with read-back:
  `~/scratch/ev-shops-178-readback.txt` equals the written body plus one
  GitHub trailing LF (19,171 -> 19,172 bytes).

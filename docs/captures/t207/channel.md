# Capture manifest — channel list, ticket #207

Reference-vs-test captures for the 10-row channel list
(5 open + 5 closed per the live gateway; 6 sections, ids 0–4 and 9,
1003 pads to 10 rows, ids 5–8 closed). Decides the #207 open
question: do exe closed rows render the empty slot + CLOSE
(`docs/channel-fill.md:847`), and what does our paint show?

## Test role (recomp2jam, fresh this ticket)

| Field | Value |
|---|---|
| Capture ids | `t207-test-channel-native`, `t207-test-channel` (preview) |
| Role / scene | test / `STATE_CHANNEL` dialog over the planet board |
| Times | 2026-11-14 04:26:05 +08:00 (both files, one walk) |
| Source | worktree `cb2/tt-207` @ `cc5ce40`, `cargo run -p recomp2jam --bin recomp2jam -- shots --out-dir ~/scratch/t207/test/` |
| Scenario | scripted walk: boot → bare board → lit-emblem click (386,88) → dialog → 10-row wire mirror staged (`wire_mirror_channels`: ids 0–4,9 open 0/100; ids 5–8 closed 0/0 inactive) → scroll-down arrow (583,419) ×5 (list top → 5, highlight stays row 0) → native shot → Confirm → Room |
| Input trace id | scripted (no OS input): `capture_all` + `stage_channel_wire_mirror` + `scroll_channel_to_closed_rows` (`crates/recomp2jam/src/shots.rs`) |
| Format | raw RGB → std-only PNG encoder, 800×600 RGB (`-native`), 400×300 RGB (preview); no scaling, no overlays |
| Sync event | dialog open (walk asserts `Screen::Channel`); rows 5–8 in view (list top 5); highlight row 0 (id 0, open) so Room entry is unchanged |
| Files + sha256 | `test/channel-native.png` `d5f9e933…95ba4885ce` (1,440,773 B), `test/channel.png` `7a131041…d076ba02` |
| Install provenance | `install-root/` symlink → `/home/arnim/Projects/recomp2jam/install-root` (Interface1.opi + Playing1.opi + avatar.opa + OJNList.dat present; `verify-shots` guard: lanking.png staged, room.png varied) |
| Locale | `LANG=en_SG.UTF-8` (no Korean locale in build env; rasterizer is locale-independent — glyph shapes ride the staged font path, owner decision `docs/opi-lessons.md`) |
| Limitations | staged catalog/chat fixtures (see manifest preamble); closed rows are the wire mirror of the live 1003 (same ids/flags, CLI-probed §Wire below) |

Replay: `cargo run -p recomp2jam --bin recomp2jam -- shots --out-dir <dir>` on the pinned commit with `install-root/` linked reproduces all files byte-identically (deterministic walk; verified by re-run + `verify-shots.sh` before/after).

## Reference role (7.55 exe)

### New capture attempt (this ticket — BLOCKED, evidence kept)

Six rig-run.sh launches (displays :91/:92, isolated prefixes
`t207/prefix`, `t207/prefix3`, owned + shared null sinks, cwd game
dir, `LANG=ko_KR.UTF-8` + `LC_ALL=ko_KR.UTF-8` via
`LOCPATH=~/.local/share/wine-locales`, blob-new + fresh t207probe
blob) all AV at `0x5F2893` (`c0000005`) seconds after launch, black
screen, crash dump written, **zero gateway frames** (logging proxy
`:15012 → :15010` saw no accept before the dump). Deterministic
across blob/prefix/sink/cwd variations. Rig hygiene held throughout
(preflight + owned resources per run; `stop` after each; no sibling
displays/prefixes touched; one gap flagged: rig-run tracks the
`wine` launcher pid, which exits while the game lives on, so `stop`
reports "no longer matches" and the game must be killed + verified
manually). Full evidence + stop-rule record: `~/scratch/t207/`
(`rig-chan*/` claims+logs, `proxy_log.py`, `proxy_stdout.log`,
`drive_chan.py`, crash `.dmp`s, `LC_ALL` directive noted). Next
step for the senior: root-cause the pre-net init AV (see
`t207-findings.md`); the verdict below does NOT wait on it.

### Pinned prior captures (substituted reference evidence)

The closed-row rendering pin predates this ticket
(`docs/channel-fill.md`, 2026-11-10 rig, same exe bytes + same
gateway shape):

| Field | Value |
|---|---|
| Capture ids | `digits-run2-lastpage` = `~/scratch/digits-rig/aim-scroll4-0.png`, `digits-run1-firstpage` = `~/scratch/digits-rig/dialog.png` |
| Role / scene | reference / `STATE_CHANNEL` dialog, scrolled last page (closed rows in view) |
| Times | 2026-11-10 20:51:24 +08:00 (scroll4), 20:44:26 +08:00 (dialog) |
| Source hash | `OTwo.exe` 5,029,888 B sha256 `83a04541…72c823b` (re-verified 2026-11-14 on `~/scratch/run-full/OTwo.exe`) |
| Install provenance | read-only copy `~/scratch/run-full` (archives complete) |
| Command/settings | `wine explorer /desktop=o2jam,800x600 OTwo.exe <blob> + mirrors `\|test\|??\|127.0.0.1\|15012\` ×5 through `proxy.py` (+5 id rotation), Xvfb :97, `PULSE_SINK=wine_null`, ko prefix copy; XTEST + winegcc SendInput per `~/scratch/planet-verify/` |
| Input trace id | proxy run 2 (+5): wire ids 5–8 open first page, ids 0–3 closed + id 4 open last page (`proxy.log` 20:49–20:50) |
| Format | 1024×768 RGB x11grab (800×600 desktop at native offset; dialog at identical native coords — X-plate (561–570,171–172) matches the test capture pixel-exact) |
| Sync event | last-page scroll position (closed rows visible); `Ch.05` + 입장 face anchors the page |
| Files + sha256 | `aim-scroll4-0.png` `6dd12504…6d722f07` (193,245 B), `dialog.png` `5d19159e…3818d5ee77` (174,169 B) |
| Limitations | run-2 ids rotated (+5) so closed slots hold ids 0–2 (same open/closed PATTERN as live); 1024×768 frame needs crop to the 800×600 desktop for pixel diff |

## Wire record (this ticket, CLI probe, no exe needed)

`cargo run -p recomp2jam --bin recomp2jam -- connect --host 127.0.0.1
--port 15010 --user-id 3 --token 207911343750`
(fresh `user:authorize t207probe` session, 2026-11-13 20:18 +08:00)
→ `authorize: Success (0)`, 10 rows: ids 0,1,2,3,4 open 0/100;
ids 5,6,7,8 closed 0/0 inactive; id 9 open 0/100. So the closed rows
are the padded ids with `active=false` AND `capacity=0` (co-varying —
which field gates the paint stays OPEN per `docs/channel-fill.md`).

## Verdict (evidence, not memory)

Side-by-side `~/scratch/t207/closed-compare.png` (left = reference
`aim-scroll4-0`, right = test `channel-native`, same coords):

- **MATCH — the empty-slot rule.** Exe closed rows paint the baked
  `Ch.` with no digits, an empty segmented track, and no enter
  face. Our rows 5–8 paint exactly that (staged path: `active=false`
  skips digits, count strip, and face — `paint.rs:753`, `paint_channel_face`).
  This retires the stale "shell paints inactive rows like active ones"
  note for the staged path: digits ARE gated; the fallback ASCII path
  is the only place the invented string survives (labeled).
- **DELTA — the CLOSE well.** Where the enter face would sit, the exe
  bakes the orange-on-dark CLOSE well; ours leaves the bar frame-0's
  blank gray well (no CLOSE marking). `paint_channel_face` returns
  early for `!active` and nothing paints the CLOSE art. Next step
  (implementation, #190): paint the CLOSE marking on inactive rows.
- Open rows agree (digits `id+1` + 입장 face: `dialog.png` Ch.01–04
  vs test top-page capture).

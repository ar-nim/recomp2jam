# t207 findings — reference captures for shop/MyRoom/channel scenes

## What was delivered

- Test-role natives (800×600, sha-pinned): channel (10-row wire
  mirror scrolled to the closed rows), item shop, music shop,
  MyRoom — manifests `channel.md`, `item-shop.md`, `music-shop.md`,
  `myroom.md` in this directory. Code: `shoot_side_native` +
  `wire_mirror_channels` + `scroll_channel_to_closed_rows` in
  `crates/recomp2jam/src/shots.rs` (gated by three new tests).
- Reference role: NO new exe capture — the exe would not boot (see
  §Boot AV). The closed-row verdict does not wait on it: the pin
  already exists (`docs/channel-fill.md`, digits-rig Nov 10) and is
  paired against the fresh test capture in `channel.md`.
- Wire record: live 1003 (CLI probe 2026-11-13): ids 0–4,9 open
  0/100; ids 5–8 closed 0/0 inactive. Actual split is 6+4.

## Closed-row verdict (the ticket's open question)

`docs/channel-fill.md:847` asks whether exe closed rows render the
empty slot + CLOSE and what our paint shows side by side. Answer,
from pixels (`~/scratch/t207/closed-compare.png`):

1. Exe closed rows render the baked empty slot: `Ch.` with no
   digits, empty segmented track, no enter face, and the orange
   CLOSE well in the face position (`aim-scroll4-0.png`).
2. Our paint matches the rule: `active=false` rows skip digits,
   count strip, and face — rows 5–8 in `channel-native.png` are the
   same dark empty bars (stale "paints inactive like active" note
   retired for the staged path).
3. Delta: our face position keeps the bar frame's blank gray well —
   no CLOSE marking. `paint_channel_face` returns early for
   `!active`. #190 fix: paint the CLOSE marking on inactive rows.

## Boot AV — stop-rule record (systematic-debugging)

Symptom: `OTwo.exe` AVs `c0000005` at `0x5F2893` <10 s after launch
(black screen, 54,308 B dump), reproducible 6× across these
single-variable variations: ko-prefix copy / t189-prefix copy,
fresh-blob (t207probe) / Nov-10 blob-new, owned null sink / shared
`wine_null`, cwd worktree / cwd game dir, `LC_ALL` set / unset.
`WINEDEBUG=err` logs are identical (fixmes only). A logging TCP
proxy (`:15012 → :15010`, mirrors aimed at it) saw ZERO accepts
before the dump: the AV is pre-net, in early init past the argv
gate. Twice the game reached the leftover-dump report dialog (modal
dialog masks the AV; dismissing with Cancel resumes into the same
AV). `rig-run.sh` + preflight held for every run (isolated
display/prefix/sink per run, Memoryer preflight-checked,
`selftest OK`); one tool gap: `stop` tracks the `wine` launcher pid,
which exits while the game lives on — the game needed manual
kill + `wineserver -k` + verification (done, all runs reaped).

 Ruled out by evidence: game bytes (sha re-verified), archive
 completeness (run-full ≡ t189-run, `diff -rq` clean), prefix
 corruption (ko prefix idle since Nov 10; fresh `wine cmd` works),
 xorg 21.1.26 upgrade (t189 booted on it 2026-11-14 04:19 with the
 same blob), DNS (fails now, but failure mode is graceful-path).

 NOT root-caused (in-scope unknown): why init AVs before first
 connect when the same bytes booted Nov 9–10 and today 04:19. Per
 the 3-fix stop rule no further setup variation was attempted.

## Architecture question for the senior

The exe capture pipeline is blocked on a pre-net init AV with no
root cause after 6 evidenced variations. Options: (a) root-cause
with `winedbg` launch-under-debugger + `WINEDEBUG=+process` on a
fresh display (heavy, recipe exists in `wine-boot.md`); (b) accept
pinned-prior-capture substitution as the reference standard for
already-pinned behaviors (this ticket's approach) and reserve fresh
boots for unpinned scenes only; (c) question whether the Nov-14
environment (post xorg/pulse changes) needs a re-baselined prefix.
Recommendation: (b) now, (a) as a dedicated rig ticket — the shop /
MyRoom reference captures need the boot fixed first anyway.

## Locale + skill confirmations (owner directives)

- Korean locale: `ko_KR.UTF-8` exists ONLY via
  `LOCPATH=$HOME/.local/share/wine-locales` (no system ko locale;
  `locale -a` = C/C.utf8/en_DK/en_SG/POSIX). All rig runs carried
  `LANG=ko_KR.UTF-8` + `LC_ALL=ko_KR.UTF-8` + that LOCPATH (verified
  `locale charmap` = UTF-8). Test-role rasterizer output is
  locale-independent (recorded per manifest). No usable exe capture
  exists, so nothing needs retaking under the locale.
- systematic-debugging skill: active this whole ticket (read
  `~/.agents/skills/systematic-debugging/SKILL.md`). Phases:
  P1 evidence (proxy run, 6 variations, dump parses, diff/run-full
  identity, xorg log), P2 patterns (t189-04:19 + digits-rig-Nov-10
  as working examples), P3 single hypotheses tested minimally
  (blob→prefix→sink→proxy-observation), P4 not reached for the boot
  (stop rule) — the SHIPPED code change (shots natives) went
  through P4 properly: failing-first inventory tests updated with
  it, full suite green.

# t220 findings — OTwo.exe pre-net AV root-cause attempt

Verdict: BLOCKED. The AV did not reproduce under a winedbg launch of
OTwo.exe; no faulting read was captured. The blocker is environmental:
launching OTwo.exe directly under winedbg bypasses the explorer virtual
desktop that the working t211 recipe requires (R4/R4R show a direct
launch fails at NtUserChangeDisplaySettings with -2, no dump, no AV).
So a direct winedbg launch cannot reach the init path that AVs.

## Evidence checked

- t211-findings.md: direct launch (no explorer) fails differently
  (silent exit, black, bright 3, no dump) and never AVs.
- t207 exact launcher line unrecoverable from its rig dirs (t211 OPEN).
- winedbg 11.21 present; winedbg discipline per AGENTS memory:
  stopped-state reads only, one stop, queue all reads.

## Next evidence step

Replay t207's boot under winedbg by injecting the debugger into the
explorer-hosted process after launch: start OTwo.exe via the t211
recipe (explorer /desktop), then attach winedbg to the OTwo.exe pid
with `winedbg --gdb <pid>` or `winedbg <pid>` and set the AV
breakpoint at the t207 fault address range (0x5F2893) only after the
windowed init path has started. This keeps the virtual desktop while
still allowing stopped-state reads. Alternative: use `WINEDEBUG=+seh`
under the working recipe to capture the exception record without
stopping execution, which avoids the wow64 helper-thread fault that
kills cont/breakpoints.

No disassembly captured. No client code changed.

## 2026-11-14 +seh replay (cheap leg)

Replayed the t211 working recipe verbatim with WINEDEBUG=+seh on display
:97, owned null sink rig_null_t220, cloned prefix. Boot reached the
channel dialog (OTwo.exe alive, TCP args present, zero dumps). The +seh
trace contains only routine RPC_S_SERVER_UNAVAILABLE (0x6ba) unwinds; no
access-violation (c0000005) and no reference to 0x5F2893. So the
+seh record does not pin the faulting read. The AV did not reproduce
under the working recipe, consistent with t211 (t207 AV is OPEN there).

Verdict: BLOCKED. The working recipe boots cleanly; the t207 AV cannot
be reproduced by recipe replay alone. Next evidence step: obtain t207's
exact launcher line (still unrecoverable per t211) or attach winedbg to
a running OTwo.exe pid after the windowed init path has started, taking
stopped-state reads only. No client code changed. Audio stayed on
rig_null_t220 and was unloaded on stop.

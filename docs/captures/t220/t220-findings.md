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

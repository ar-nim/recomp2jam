# Decoy-Wayland observation for #267

Started (UTC): `2026-11-18T06:10:20Z`. Worktree: `/home/arnim/Projects/recomp2jam-wt61-fence`. Parent environment: `WAYLAND_DISPLAY=wayland-1` (owner session), `DISPLAY=:0`; child overridden to `/tmp/fence-267-qtg5d442` and `WAYLAND_DISPLAY=fence-267-decoy-wayland` (no real compositor).

## Commands

```text
python3 scripts/observe-decoy-display.py
```

## Observation

- Wayland socket inodes before test:

```text
live=['13240621', '13245473', '17950', '18219', '18337', '21130', '21131', '21133', '21150', '21160', '21163', '25058789', '25063465', '25063466', '25063468', '25636', '25641', '25656', '25662', '26136', '31134886', '31134887', '31134888', '31149994'] decoy=['31207033']
```

- Wayland socket inodes after test (same decoy inode = listener intact):

```text
live=['13240621', '13245473', '17950', '18219', '18337', '21130', '21131', '21133', '21150', '21160', '21163', '25058789', '25063465', '25063466', '25063468', '25636', '25641', '25656', '25662', '26136', '31134886', '31134887', '31134888', '31149994'] decoy=['31207033']
```

- Connections accepted by the decoy listener after the run: `0`.
- `cargo test -p input-drive` exit code: `0`.
- Test summaries: `test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.04s`; `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`.
- New client/Xvfb processes during the run (empty = none spawned):

```text
(none)
```

- Client/Xvfb processes before the run (context):

```text
1200: /usr/bin/kwin_wayland --wayland-fd 8 --socket wayland-1 --xwayland-fd 9 --xwayland-fd 10 --xwayland-display :0 --xwayland-initfd 12 --xwayland-init-display :1 -
```

Verdict: **PASS** — zero decoy connections and no spawned client/Xvfb process, with the input-drive suite green under a fake ambient Wayland display.

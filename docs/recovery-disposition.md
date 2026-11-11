# Recovery disposition — `extracted/` evidence lost to the 2026-11-11 merge clobber

Disposition record for the dossier half of `ar-nim/recomp2jam#101`.
The prevention half (collision guard + data-dir canary) already landed
in `87357ff`; this record covers the eight lost bundles.

Version pin for every regeneration below: `AngelO2Jam.exe` ==
`OTwo.exe`, 5,029,888 B, SHA-256 `83a04541…72c823b` (VA = file +
`0x400000`). All methods are read-only over the `install-root/` copy.

## REA re-export attempt (2026-11-12) — not possible

`rea evidence-export`/`compare` operate on evidence bundle *files*
(`~/scratch/rea-docs/cli.md` §"Import, export and compare Evidence",
rea 6.8.0). They do not query a provider database: retained records are
MCP-session-local and keyed by content hash (`ev_<sha256>`), so an id
cannot be re-exported unless the underlying observation content is
reproduced. A filesystem sweep (`~/scratch`, `~/.local/share`,
`~/.config`, both worktrees) found no surviving copy of any lost bundle
and no REA store holding the records. Conclusion: re-export has no
reachable source; every bundle must be regenerated from bytes or its
claim family re-evidenced. The abbreviated `ev_…` ids cited in the docs
remain stable references *if and only if* regeneration is byte-exact
(same tool versions, same target bytes), because ids are content hashes.

## Per-bundle disposition

| Lost bundle | Referenced by | Class | Disposition |
|---|---|---|---|
| `rea-4015-dossier.json` | docs/4015-hunt.md | (b) regenerable from bytes | Family closed (doc VERIFIED, "no further REA needed for this opcode"). Method pinned inline: GNU objdump 2.47 slices + python3 byte scans. Regenerate only if the peer-life/rank mirror claim family re-opens; content-hash ids reproduce on byte-identical re-runs. No open owner ticket. |
| `rea-4015-ghidra-dossier.json` + `ghidra-4015-work/proj4015.gpr` + `ghidra-4015/*.decomp.txt` histograms | docs/4015-hunt.md, docs/claims-render.md | (b) regenerable from bytes | Re-import the pinned exe into headless Ghidra 12.1.2 (documented import 2026-11-06), re-decompile the four targets with the standalone reader (the `-postScript` OSGi block is recorded in docs/4015-hunt.md), re-file 6 records. claims-render.md is VERIFIED, so this is regenerate-if-reopened. No open owner ticket. |
| `rea-ui-loader-dossier.json` | docs/ui-loader.md, docs/claims-render.md | (c) deferred-to-claim-family | Owner: **#114** (applier client-relative placement, byte-walk) — the next live ticket in the loader/applier family. Sharpened step: when #114 runs, re-file the ControlList chain (9 records, method pinned in docs/ui-loader.md: headless Ghidra 12.1.2 + DumpControlList.java + CP949 extraction) before citing any new ids, so loader claims cite one coherent bundle. |
| `rea-cresult-anchors-dossier.json` + `ghidra-cresult/` decompiler scratch | docs/cresult-anchors.md | (c) deferred-to-claim-family | Owner: **#86** (pin and guard the result digit cells — REA trace of the digit-writer district, files `0xc44c0`/`0xc4630`). Sharpened step: #86's trace re-files the `rea-cresult-anchors` records (11 records, `md5(rea-cresult-anchors\|operation\|subject-VA)` ids) plus the `DumpCResult.java` scratch, then re-pins the result-digit claim rows. |
| `rea-score-handlers-dossier.json` | docs/net-protocol.md | (c) deferred-to-claim-family | Owner: **#82** (scoring family — pill/jam-score earn flow, REA trace). Sharpened step: the 4002/4017 offsets and dispatcher edges survive verbatim in docs/net-protocol.md (produced by objdump 2.47 + Python 3.14.9 byte scans); #82's pass re-runs those slices and re-files `ev_9ac44f7d…`, `ev_e6cdff98…`, `ev_67fbdde4…`. |
| `rea-snapshot` | docs/lifecycle.md | (b) regenerable from bytes | One headless Ghidra 12.1.2 analysis of the pinned exe; docs/lifecycle.md pins the expected analysis digest `88a37df4…130c8d`. Sharpened step: re-import, compare digest, keep the snapshot single-line JSON. Family closed; no open owner ticket. |
| `cover757-view.jpg` | docs/channel-fill.md | (c) deferred-to-claim-family | Owner: **#92** (channel-row label value source — the channel family). Sharpened step: re-capture the OJN cover id 757 view (800x600 JPEG, full-bleed) from the exe canvas under the documented Wine/Xvfb rig during the #92 pass; byte-identity is not expected (JPEG encode), so docs/channel-fill.md line-694 citation is content-based, not hash-based. |

Class key: (b) = regenerable from bytes by the established byte-walk /
headless-Ghidra method; (c) = re-filed by the named owner ticket when
its claim family next gets evidence work.

## Notes

- (a) re-filed-already: **none**. `extracted/rea-playing-digits-dossier.json`
  (#104, filed 2026-11-12) covers the Playing digit-frame family only;
  zero `ev_` id overlap with any lost bundle.
- `extracted/decomp2jam-salvage/` was rebuilt before this pass (issue
  body "Rebuilt already"); out of scope here.
- The prevention acceptance item (guard test) landed in `87357ff` and
  is not repeated in this record.

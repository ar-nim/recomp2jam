# Journey audit — Spec: Loading state (#292)

Comparison leg, 2026-11-18. Worktree `cb2/ja-b`, base `2105ac6`, no exe boots.
Exe 7.55 pin: `OTwo.exe` 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.

Comparator classes: `pixel-region`, `wire-sequence`, `input-response`,
`state-path`, `data-effect`, `NA-DESIGN`.

## Per-story matrix (6 stories)

| # | Story (short) | Comparator | Our journey (code / guard / capture) | Exe evidence | Verdict | Notes / mapping |
|---|---|---|---|---|---|---|
| 1 | [x] Song illustration full-bleed behind bar and caption | pixel-region | `paint_loading_cover` / `paint_loading_scene_with_status` (`crates/o2jam-shell/src/screens/paint.rs:5592-5655`) paint the staged figure opaque over the frame; `stage_chart_bytes` → `stage_loading_cover` (`crates/recomp2jam/src/session.rs:1007-1020`); guard `loading_cover_paints_full_bleed_behind_caption` (paint.rs:10521). Rust p347 frame `frames/test/loading.png` sha256 `70e308bff4ded98fd0623e888f99180aee5be6d5c9fa1f425df88b6d6b34c51d` | Exe p347 frame `frames/ref/loading.png` sha256 `b555123f6d099f64330f000ea52cd4f2eeeb3716f9fa4139a6bf362466eaaaed` (c1 pack citation sha256 `104b5594efb906fd7b0d783f4f26c9c90a2159c86ad427e5ea1b250aafd74d62`); `~/scratch/p347-ab/pack/region-metrics.json` sha256 `b02797f8b9ff9d4d13750e7156a251fa9f140212eec4a23ebe4dda824a0e46d4`: `cover_art_lower_mid` SSIM 0.056 / PSNR 7.07 (p346 baseline sha256 `f52bf479c32d638d8afa814646ef0d151a7cda0e120d183f73b885b46102e60b`: `logo_plate` 0.032 / 6.24) | MISMATCHED | Full-bleed mechanism matches; the matched-start guest never stages the walked OJN cover → mapped to **#355** (open, named residual of #347). Caption half is story 2 / #360. No new issue for the cover. |
| 2 | [ ] Artist / Remix / Pattern credits rendered | pixel-region + data-effect | `stage_loading_caption` (`session.rs:1145`) supplies one `(title, artist)` pair; `LoadingStatus::caption` (`paint.rs:5422`) joins them as `TITLE - ARTIST`; `paint_loading_song` (5551) is the only credit painter. No Remix/Pattern source or cell | `docs/parity/verify-wine.md:15` pins the exe Loading frame with `Artist/Remix/Pattern credits`; VA `0x4e47a0+0x528` Arial 7x14 w400 (`docs/gdi-text.md:100`, ev `68d920c2…`); `docs/loading-type.md:17` records no Arial cell and an OPEN caption-to-face mapping | MISMATCHED | Filed **#360** — "Loading gate never paints Artist/Remix/Pattern credits" (needs-triage, component:shell, P2), born with body / evidence / acceptance. No fix in this leg. |
| 3 | [x] Gate holds until song validated, staged and audio-ready | state-path | `loading_gate_ready` (`game/mod.rs:3290-3312`) requires no latched failure, phase ≥ `LOADING_GATE_PHASES`, `validate_loading_select`, and an attached audio backend; `set_loading_status` (3204) advances only on real phases; `fail_loading_gate` (3259) holds; guards `latched_loading_failure_holds_loading` (19233), `loading_hand_session_holds_then_arms_and_hands_off` (19282) | Static exe pin VA `0x4b5560` / file `0xb5560` selection validation, staging `0x4f8b80`, OJN `0x412180`, codec `0x54db40` (`docs/parity/loading-gate.md`, `docs/lifecycle.md` §9); no qualified exe failure frame | MATCHED | State-path and hold rule agree with the pin. Live failure/refusal/arm-latch frames go to the live lane (below); no new issue. |
| 4 | [x] 4011 / 4012 confirm exchange advances me | wire-sequence + input-response | `apply_start_broadcast` / `start_announced` (`session.rs:2936`), `confirm_loading` (1942) sends `ConfirmMusicLoaded` 4012 then enters Playing; guard `loading_confirm_sends_4012_then_enters_playing` (5688) | c1 `wire/room-stage.log` (sha256 `589075701bad6179c6189041e8e0eb6c9a3867acbd75ba59f979fe5ed30511fa`) lines 28-33: S2C `4011/4013` → C2S `4012` → S2C `4013`; p346 `proof.json` sha256 `3bee382f80398840b59a01d09c447c5e0ea72a87d2f50a312a049d6c3dc68802` has `wire_sequence_equal=true` through `S2C 4013` on both sides | MATCHED | Confirm exchange itself matches. Everything after 4013 is #326 territory (exe continues `4012 → 4055/4048×11` while the Rust matched window stops); map only, do not file. |
| 5 | [x] Pinned gate rules implemented and guarded; failed send keeps Loading | state-path | `confirm_loading` (`session.rs:1942-1968`) returns without entering Playing when the send fails or speed is unknown; `loaded_sent` latch (514/570/3320); shell `loading_gate_failed` latch (mod.rs:3259) and hand-session guard `loading_hand_session_holds_then_arms_and_hands_off` | Static pin: `docs/wine-boot.md` Attempt 12/13 opcode trace; `docs/net-protocol.md` §4c 4011/4013/4012 layout; no qualified exe refusal capture | MATCHED | Guards and code agree with the pin. Spec's note about missing live-chain red proof is a live-lane item (#326 class); no divergence in the rule itself. |
| 6 | [ ] Loading gate phases paced like the exe | state-path (+ timing) | `loading_phase_status` (`crates/recomp2jam/src/loading.rs:91`) emits symbolic phases 0..4; `LOADING_GATE_STEPS = 4`; no wall-clock pacing: `loading_status_at_half` / `set_loading_status_remembers_the_walk` (mod.rs:19137) test fractions only | Exe leg offsets pinned (`0x5718b0`, `0x4f8b80`, `0x412180`, `0x54db40`) in `docs/parity/loading-gate.md` Timing row; no wall-trace pack | BLOCKED | Needs a null-sink wall trace timing each of the four gate legs on a qualified 4011→4012 boot; no exe boot in this leg. |

## Summary matrix — Loading

| Verdict | Stories | Rationale |
|---|---|---|
| MATCHED | 3, 4, 5 | Gate hold/validation rules and the 4011/4012 wire agree with pinned exe evidence (`p346/proof.json` sha256 `3bee382f…68802`, c1 wire `58907570…311fa`). |
| MISMATCHED | 1, 2 | Story 1 mapped to open **#355** (matched-start cover not staged; p347 `cover_art_lower_mid` 0.056 in artifact sha256 `b02797f8…d4`). Story 2 filed as **#360** (credits never painted). |
| BLOCKED | 6 | No wall-clock pacing capture for the four gate legs. |
| NA-DESIGN | — | None. |

Live-lane carry-forward:
1. Qualified 4011→4012 Loading run with a null-sink wall trace timing the four
   gate legs (story 6).
2. Refusal and arm-latch frames in a qualified run (stories 3 and 5).
3. Exe 4011 cover-source read (memory / REA) for #355 and the Arial 7x14
   credit consumer for #360; the current c1 frame proves the pixels but not
   the source fields.

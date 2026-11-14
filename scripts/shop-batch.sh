#!/usr/bin/env bash
# shop-batch.sh — shop/MyRoom/channel batch manifest runner + pass/fail gate (#209).
#
# Wires the #207 reference captures (docs/captures/t207/), the #208 input
# traces (fixtures/input-trace/ via `replay-menu-trace`), and the
# live-Memoryer leg (127.0.0.1:15010 serves ids 0-4 + 9) into one manifest
# run with explicit thresholds and per-row verdicts.
#
# Usage:
#   scripts/shop-batch.sh [--live] [--red-proof] [--hide-install-root]
#                         [--require-data] [--skip-gates]
#                         [--out DIR] [--record-dir DIR] [--bin PATH]
#
#   --live               run the opt-in live-Memoryer row (reported
#                        separately as interop evidence, never in the
#                        offline counts)
#   --red-proof          break one guarded behavior (BUYITEM commit lost),
#                        show exactly its row turning FAIL, restore, and
#                        prove recovery (recorded under <out>/red-proof/)
#   --hide-install-root  simulate CI: captures skip cleanly, traces and
#                        guards still run
#   --require-data       missing install-root fails preflight (exit 14)
#                        instead of skipping captures
#   --skip-gates         skip fmt/clippy/workspace-tests (iteration only;
#                        the recorded nightly run never sets this)
#   --out DIR            scenario out-dirs root (default
#                        $HOME/scratch/shop-t209/<stamp>; PNGs stay local)
#   --record-dir DIR     copy summary.md + report.json (no PNGs) here for
#                        the committed record
#   --bin PATH           reuse this recomp2jam binary instead of building
#
# Preflight (scripts/lib/rig-preflight.sh, fail closed): relay ports free,
# null/virtual audio sink selected (scripted audio never reaches the
# speakers), Memoryer probe only for --live, install-root readable or
# clean-skip. No display is claimed or touched: every leg is headless
# (snapshot seam, scripted CLI, unit guards) — the owner desktop is safe.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
MANIFEST="$REPO_ROOT/batches/shop-t209/manifest.json"
# shellcheck source=lib/rig-preflight.sh
. "$SCRIPT_DIR/lib/rig-preflight.sh"

LIVE=0; RED_PROOF=0; HIDE_DATA=0; REQUIRE_DATA=0; SKIP_GATES=0
OUT=""; RECORD_DIR=""; BIN_OVERRIDE=""
while [ $# -gt 0 ]; do
  case "$1" in
    --live) LIVE=1; shift ;;
    --red-proof) RED_PROOF=1; shift ;;
    --hide-install-root) HIDE_DATA=1; shift ;;
    --require-data) REQUIRE_DATA=1; shift ;;
    --skip-gates) SKIP_GATES=1; shift ;;
    --out) OUT="$2"; shift 2 ;;
    --record-dir) RECORD_DIR="$2"; shift 2 ;;
    --bin) BIN_OVERRIDE="$2"; shift 2 ;;
    *) echo "shop-batch: unknown flag $1" >&2; exit 2 ;;
  esac
done

STAMP="$(date +%Y%m%d-%H%M%S)"
[ -n "$OUT" ] || OUT="$HOME/scratch/shop-t209/$STAMP"
mkdir -p "$OUT"
export RIG_CLAIMS_FILE="$OUT/claims"
: > "$OUT/claims"

log() { printf 'shop-batch: %s\n' "$*"; }
fail_row() { printf 'shop-batch: ROW %s %s: %s\n' "$1" "$2" "$3"; }

# --- preflight ---------------------------------------------------------------
# No display, no Wine prefix, no listen ports: headless legs only. Audio is
# always contained (null policy); the server probe runs for --live only.
export PULSE_SINK="${PULSE_SINK:-${RIG_SINK_NAME:-wine_null}}"
if [ "$HIDE_DATA" = 1 ]; then
  export RIG_INSTALL_ROOT="$OUT/empty-install-root"
  mkdir -p "$RIG_INSTALL_ROOT"
else
  RIG_INSTALL_ROOT="${RIG_INSTALL_ROOT:-$REPO_ROOT/install-root/O2Jam}"
  export RIG_INSTALL_ROOT
fi
if [ "$LIVE" = 1 ]; then export RIG_EXPECT_MEMORYER=1; else export RIG_EXPECT_MEMORYER=0; fi

rig_check_relay_ports_free || exit "$RIG_E_PORT"
rig_check_audio_sink null "$PULSE_SINK" || exit "$RIG_E_AUDIO"
if [ "$LIVE" = 1 ]; then rig_check_memoryer || exit "$RIG_E_SERVER"; fi
DATA_MODE=present
if ! rig_check_install_root; then
  if [ "$REQUIRE_DATA" = 1 ]; then exit "$RIG_E_DATA"; fi
  DATA_MODE=hidden
  log "install-root absent: capture legs will SKIP cleanly (CI arm)"
fi
if [ "$HIDE_DATA" = 1 ]; then
  DATA_MODE=hidden
  log "install-root hidden by flag: capture legs will SKIP cleanly (CI arm)"
fi

# --- versions (the reproducibility record) -----------------------------------
COMMIT="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"
RUSTC_V="$(rustc --version)"; CARGO_V="$(cargo --version)"; PY_V="$(python3 --version 2>&1)"
LOCALE_V="$(locale 2>/dev/null | tr '\n' ';' || echo unknown)"
log "base $COMMIT; $RUSTC_V; $PY_V; data=$DATA_MODE; live=$LIVE"
{
  echo "commit=$COMMIT"
  echo "rustc=$RUSTC_V"
  echo "cargo=$CARGO_V"
  echo "python=$PY_V"
  echo "locale=$LOCALE_V"
} > "$OUT/versions.txt"

# --- gates: build + synthetic gates (skill pipeline stage 2) -----------------
if [ "$RED_PROOF" = 1 ]; then
  SKIP_GATES=1
  log "red-proof: gates skipped by design (the break is a logic change fmt/clippy accept; the full suite would fail alongside the row)"
fi
if [ "$SKIP_GATES" = 0 ]; then
  log "gate: cargo fmt --check"
  (cd "$REPO_ROOT" && cargo fmt --check >"$OUT/gate-fmt.log" 2>&1)
  log "gate: cargo clippy --workspace --all-targets -- -D warnings"
  (cd "$REPO_ROOT" && cargo clippy --workspace --all-targets -- -D warnings >"$OUT/gate-clippy.log" 2>&1)
  log "gate: cargo test --workspace"
  (cd "$REPO_ROOT" && cargo test --workspace >"$OUT/gate-test.log" 2>&1)
  log "gates green"
else
  log "gates SKIPPED (--skip-gates or --red-proof)"
fi

# --- red proof: break one guarded behavior (fail closed on dirty tree) ---------
RED_BREAK_FILE="crates/o2jam-shell/src/screens/game/mod.rs"
if [ "$RED_PROOF" = 1 ]; then
  git -C "$REPO_ROOT" diff --quiet || { log "red-proof refused: tracked tree dirty"; exit 2; }
  grep -q "self.outbox.take_shop_buy()" "$REPO_ROOT/$RED_BREAK_FILE" || { log "red-proof: break pattern not found"; exit 1; }
  mkdir -p "$OUT/red-proof"
  cp "$REPO_ROOT/$RED_BREAK_FILE" "$OUT/red-proof/mod.rs.orig"
  git -C "$REPO_ROOT" diff --stat >"$OUT/red-proof/tree-before.txt" 2>&1 || true
  python3 - "$REPO_ROOT/$RED_BREAK_FILE" <<'PY'
import sys
path = sys.argv[1]
text = open(path).read()
old = """    pub fn take_shop_buy(&mut self) -> Option<SyncItemPurchase> {
        self.outbox.take_shop_buy()
    }"""
new = """    pub fn take_shop_buy(&mut self) -> Option<SyncItemPurchase> {
        // RED-PROOF BREAK (#209): the queued 5000 BUYITEM commit is lost.
        let _ = self.outbox.take_shop_buy();
        None
    }"""
assert text.count(old) == 1, "break site must be unique"
open(path, "w").write(text.replace(old, new))
PY
  git -C "$REPO_ROOT" diff >"$OUT/red-proof/break.patch"
  log "red-proof: BUYITEM commit broken (see red-proof/break.patch); restoring on exit"
  restore_break() { cp "$OUT/red-proof/mod.rs.orig" "$REPO_ROOT/$RED_BREAK_FILE"; log "red-proof: behavior restored"; }
  trap restore_break EXIT
fi

# --- build -------------------------------------------------------------------
if [ -n "$BIN_OVERRIDE" ]; then
  BIN="$BIN_OVERRIDE"
else
  (cd "$REPO_ROOT" && cargo build -p recomp2jam >"$OUT/build.log" 2>&1)
  BIN="$REPO_ROOT/target/debug/recomp2jam"
fi
[ -x "$BIN" ] || { log "no binary at $BIN"; exit 1; }

# --- helpers -----------------------------------------------------------------
# Parse `replay ok: SCEN (A applied, B accepted, final side S, hash H)`.
parse_replay() {
  python3 - "$1" <<'PY'
import re, sys
line = open(sys.argv[1]).read()
m = re.search(r"replay ok: (\S+) \((\d+) applied, (\d+) accepted, final side (\S+), hash (\d+)\)", line)
if not m:
    sys.exit(f"unparsable replay output:\n{line}")
print(" ".join(m.groups()))
PY
}

REPORT_JSON="$OUT/report.json"
export REPORT_JSON
echo '{"rows": {}, "live": {}}' > "$REPORT_JSON"
record_check() { # row check status detail
  python3 - "$REPORT_JSON" "$1" "$2" "$3" "$4" <<'PY'
import json, sys
path, row, check, status, detail = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5] if len(sys.argv) > 5 else ""
doc = json.load(open(path))
doc["rows"].setdefault(row, {}).setdefault("checks", {})[check] = {"status": status, "detail": detail}
json.dump(doc, open(path, "w"), indent=2)
PY
}

run_trace_leg() { # row file stage scenario events applied accepted side
  local row="$1" file="$2" stage="$3" scenario="$4" events="$5" applied="$6" accepted="$7" side="$8"
  local dir="$OUT/$row/trace-$scenario" h1 h2 s1 s2
  mkdir -p "$dir/run1" "$dir/run2"
  "$BIN" replay-menu-trace --trace "$REPO_ROOT/$file" --stage "$stage" >"$dir/run1/stdout.txt" 2>"$dir/run1/stderr.txt"
  "$BIN" replay-menu-trace --trace "$REPO_ROOT/$file" --stage "$stage" >"$dir/run2/stdout.txt" 2>"$dir/run2/stderr.txt"
  s1="$(parse_replay "$dir/run1/stdout.txt")" || { record_check "$row" "trace:$scenario" "failed" "run1 output unparsable"; return 1; }
  s2="$(parse_replay "$dir/run2/stdout.txt")" || { record_check "$row" "trace:$scenario" "failed" "run2 output unparsable"; return 1; }
  # shellcheck disable=SC2086
  set -- $s1; local sc1="$1" a1="$2" c1="$3" f1="$4" h1="$5"
  # shellcheck disable=SC2086
  set -- $s2; local sc2="$1" a2="$2" c2="$3" f2="$4" h2="$5"
  local verdict=pass detail="run1=[$s1] run2=[$s2]"
  [ "$sc1" = "$scenario" ] && [ "$sc2" = "$scenario" ] || verdict=failed
  [ "$a1" = "$applied" ] && [ "$a2" = "$applied" ] || verdict=failed
  [ "$c1" = "$accepted" ] && [ "$c2" = "$accepted" ] || verdict=failed
  [ "$((a1 + c1))" = "$events" ] && [ "$((a2 + c2))" = "$events" ] || verdict=failed
  [ "$f1" = "Some(\"$side\")" ] && [ "$f2" = "Some(\"$side\")" ] || verdict=failed
  [ "$h1" = "$h2" ] || verdict=failed
  if [ "$verdict" = pass ]; then
    record_check "$row" "trace:$scenario" "passed" "applied=$a1 accepted=$c1 side=$f1 hash=$h1 (2 runs identical)"
  else
    record_check "$row" "trace:$scenario" "failed" "expected applied=$applied accepted=$accepted events=$events side=Some(\"$side\") + identical hashes; $detail"
    return 1
  fi
}

run_guard_leg() { # row package target filter
  local row="$1" pkg="$2" target="$3" filter="$4"
  local dir="$OUT/$row/guard-$filter"
  mkdir -p "$dir"
  if [ "$target" = "--bin recomp2jam" ]; then
    (cd "$REPO_ROOT" && cargo test -q -p "$pkg" --bin recomp2jam "$filter" >"$dir/test.log" 2>&1)
  else
    (cd "$REPO_ROOT" && cargo test -q -p "$pkg" --lib "$filter" >"$dir/test.log" 2>&1)
  fi
  local summary; summary="$(grep -E "^test result:" "$dir/test.log" | tail -n 1 || true)"
  if grep -qE "^test result: ok\. 1 passed; 0 failed" "$dir/test.log"; then
    record_check "$row" "guard:$filter" "passed" "$summary"
  else
    record_check "$row" "guard:$filter" "failed" "${summary:-no test-result line}; see $dir/test.log"
    return 1
  fi
}

SHOTS_DIR=""
run_captures_once() {
  SHOTS_DIR="$OUT/shots"
  mkdir -p "$SHOTS_DIR"
  "$BIN" shots --out-dir "$SHOTS_DIR" >"$OUT/shots.log" 2>&1
  ls "$SHOTS_DIR" >>"$OUT/shots.log" 2>&1
}

run_capture_leg() { # row scene_file pinned_sha
  local row="$1" file="$2" pinned="$3"
  if [ "$DATA_MODE" = hidden ]; then
    record_check "$row" "capture:$file" "skipped" "install-root absent (clean CI skip)"
    return 2
  fi
  local actual; actual="$(sha256sum "$SHOTS_DIR/$file" | cut -d' ' -f1)"
  if [ "$actual" = "$pinned" ]; then
    record_check "$row" "capture:$file" "passed" "sha256 $actual == pinned"
  else
    record_check "$row" "capture:$file" "failed" "sha256 $actual != pinned $pinned"
    return 1
  fi
}

# --- row runs ------------------------------------------------------------------
ROW_FAIL=0
run_row() { # id ...
  local id="$1"; shift
  log "row: $id"
  mkdir -p "$OUT/$id"
}

# Row 1: item-shop browse -> buy (fully pinned: always `passed` when green).
run_row item-shop-browse-buy
R1_FAIL=0; R1_SKIP=0
run_trace_leg item-shop-browse-buy fixtures/input-trace/shop-entry-from-room.json shop shop-entry-from-room 3 2 1 shop || R1_FAIL=1
run_trace_leg item-shop-browse-buy fixtures/input-trace/shop-goods-browse-pager.json shop shop-goods-browse-pager 14 12 2 shop || R1_FAIL=1
run_trace_leg item-shop-browse-buy fixtures/input-trace/shop-row-cart-buyitem.json shop shop-row-cart-buyitem 10 8 2 shop || R1_FAIL=1
run_guard_leg item-shop-browse-buy recomp2jam "--bin recomp2jam" room_shop_entry_replay_is_deterministic || R1_FAIL=1
run_guard_leg item-shop-browse-buy recomp2jam "--bin recomp2jam" shop_browse_pager_replay_turns_pages_and_holds || R1_FAIL=1
run_guard_leg item-shop-browse-buy recomp2jam "--bin recomp2jam" shop_row_cart_buyitem_replay_commits_the_cart || R1_FAIL=1
run_guard_leg item-shop-browse-buy o2jam-shell "--lib" shop_browse_toggles_cart_and_commits_5000 || R1_FAIL=1

# Row 2: music-shop browse -> buy (capture + guards; DIFF OPEN -> needs_review).
run_row music-shop-browse-buy
R2_FAIL=0
run_guard_leg music-shop-browse-buy o2jam-shell "--lib" music_shop_board_paints_catalog_cells_and_pager_at_staged_rects || R2_FAIL=1
run_guard_leg music-shop-browse-buy o2jam-shell "--lib" music_shop_board_paints_owned_marks_at_staged_rects || R2_FAIL=1
run_guard_leg music-shop-browse-buy o2jam-shell "--lib" music_shop_row_pick_stages_the_cart_and_cart_buy_queues_5010 || R2_FAIL=1

# Row 3: MyRoom enter -> equip (+ gift claim on the same overlay).
run_row myroom-enter-equip
R3_FAIL=0
run_trace_leg myroom-enter-equip fixtures/input-trace/myroom-bag-slot-equip.json myroom-equip myroom-bag-slot-equip 10 8 2 myroom || R3_FAIL=1
run_trace_leg myroom-enter-equip fixtures/input-trace/giftbox-claim.json myroom-gift giftbox-claim 8 6 2 myroom || R3_FAIL=1
run_guard_leg myroom-enter-equip recomp2jam "--bin recomp2jam" myroom_bag_slot_equip_replay_applies_the_pick || R3_FAIL=1
run_guard_leg myroom-enter-equip recomp2jam "--bin recomp2jam" giftbox_claim_replay_queues_the_claim || R3_FAIL=1
run_guard_leg myroom-enter-equip o2jam-shell "--lib" myroom_equip_queues_5004_with_derived_slot || R3_FAIL=1
run_guard_leg myroom-enter-equip o2jam-shell "--lib" myroom_gift_claim_queues_5023_for_the_picked_cell || R3_FAIL=1
run_guard_leg myroom-enter-equip o2jam-shell "--lib" myroom_entry_queues_gift_list_fetch || R3_FAIL=1

# Row 4: closed-channel list (wire shape + paint; CLOSE marking OPEN).
run_row channel-closed-list
R4_FAIL=0
run_guard_leg channel-closed-list recomp2jam "--bin recomp2jam" channel_wire_mirror_matches_the_live_ten_row_list || R4_FAIL=1
run_guard_leg channel-closed-list recomp2jam "--bin recomp2jam" channel_native_paints_the_wire_mirror_at_full_canvas || R4_FAIL=1
run_guard_leg channel-closed-list recomp2jam "--bin recomp2jam" channel_shot_paints_no_fill_under_the_live_list || R4_FAIL=1

# Capture legs share one shots walk (isolated dir); each row consumes its file.
if [ "$DATA_MODE" = present ]; then
  run_captures_once
  ITEM_SHA="$(python3 -c "import json;print([c['sha256'] for r in json.load(open('$MANIFEST'))['rows'] if r['id']=='item-shop-browse-buy' for c in r['reference_captures']][0])")"
  MUSIC_SHA="$(python3 -c "import json;print([c['sha256'] for r in json.load(open('$MANIFEST'))['rows'] if r['id']=='music-shop-browse-buy' for c in r['reference_captures']][0])")"
  MYROOM_SHA="$(python3 -c "import json;print([c['sha256'] for r in json.load(open('$MANIFEST'))['rows'] if r['id']=='myroom-enter-equip' for c in r['reference_captures']][0])")"
  CHANNEL_SHA="$(python3 -c "import json;print([c['sha256'] for r in json.load(open('$MANIFEST'))['rows'] if r['id']=='channel-closed-list' for c in r['reference_captures']][0])")"
  run_capture_leg item-shop-browse-buy shop-native.png "$ITEM_SHA" || { rc=$?; [ "$rc" = 1 ] && R1_FAIL=1 || R1_SKIP=1; }
  run_capture_leg music-shop-browse-buy musicshop-native.png "$MUSIC_SHA" || { rc=$?; [ "$rc" = 1 ] && R2_FAIL=1; }
  run_capture_leg myroom-enter-equip myroom-native.png "$MYROOM_SHA" || { rc=$?; [ "$rc" = 1 ] && R3_FAIL=1; }
  run_capture_leg channel-closed-list channel-native.png "$CHANNEL_SHA" || { rc=$?; [ "$rc" = 1 ] && R4_FAIL=1; }
else
  record_check item-shop-browse-buy "capture:shop-native.png" "skipped" "install-root absent (clean CI skip)"
  record_check music-shop-browse-buy "capture:musicshop-native.png" "skipped" "install-root absent (clean CI skip)"
  record_check myroom-enter-equip "capture:myroom-native.png" "skipped" "install-root absent (clean CI skip)"
  record_check channel-closed-list "capture:channel-native.png" "skipped" "install-root absent (clean CI skip)"
  R1_SKIP=1
fi

# --- live leg (opt-in, interop evidence only) ------------------------------------
LIVE_STATUS="skipped"
if [ "$LIVE" = 1 ]; then
  LDIR="$OUT/live-channel-list"; mkdir -p "$LDIR"
  MEMORYER_DIR="${MEMORYER_DIR:-$HOME/memoryer-local}"
  TOKEN="$(python3 - "$MEMORYER_DIR/O2JAM.db" 1 <<'PY'
import sqlite3, sys
db = sqlite3.connect(sys.argv[1])
row = db.execute("select TUSER_ID from t_o2jam_login where USER_INDEX_ID=? order by LOGIN_TIME desc limit 1", (sys.argv[2],)).fetchone()
if not row: sys.exit("no login row for USER_INDEX_ID=1")
print(row[0])
PY
)" || { log "live: token mint failed"; LIVE_STATUS="failed"; }
  if [ "$LIVE_STATUS" != "failed" ]; then
    if "$BIN" connect --host 127.0.0.1 --port 15010 --user-id 1 --token "$TOKEN" >"$LDIR/connect.txt" 2>&1; then
      python3 - "$LDIR/connect.txt" >"$LDIR/shape.txt" 2>&1 <<'PY' && LIVE_STATUS="passed" || LIVE_STATUS="failed"
import re, sys
text = open(sys.argv[1]).read()
rows = re.findall(r"gateway (\d+) channel (\d+): (\d+)/(\d+) (open|closed)", text)
assert len(rows) == 10, f"want 10 channel rows, got {len(rows)}"
for i, (gw, ch, pop, cap, state) in enumerate(rows):
    assert (int(gw), int(ch)) == (0, i), f"row order broke at index {i}: {gw}/{ch}"
    want_open = i < 5 or i == 9
    assert (state == "open") == want_open, f"channel {ch}: want open={want_open}, got {state}"
    assert int(cap) == (100 if want_open else 0), f"channel {ch}: capacity {cap}"
    assert int(pop) == 0, f"channel {ch}: population {pop} (rig must be isolated)"
print("live 1003 shape ok: ids 0-4,9 open 0/100; ids 5-8 closed 0/0")
PY
      cat "$LDIR/shape.txt" 2>/dev/null || tail -n 20 "$LDIR/connect.txt"
    else
      LIVE_STATUS="failed"; tail -n 20 "$LDIR/connect.txt"
    fi
  fi
  python3 - "$REPORT_JSON" "$LIVE_STATUS" <<'PY'
import json, sys
path, status = sys.argv[1], sys.argv[2]
doc = json.load(open(path))
doc["live"] = {"live-channel-list": {"status": status}}
json.dump(doc, open(path, "w"), indent=2)
PY
else
  python3 - "$REPORT_JSON" <<'PY'
import json, os
path = os.environ.get("REPORT_JSON", "")
doc = json.load(open(path))
doc["live"] = {"live-channel-list": {"status": "skipped", "detail": "opt-in only (pass --live)"}}
json.dump(doc, open(path, "w"), indent=2)
PY
  log "live: skipped (opt-in only; pass --live with local Memoryer up)"
fi

# --- verdicts ------------------------------------------------------------------
row_verdict() { # row fail_var status_when_green
  local row="$1" fail="$2" green="$3"
  if [ "$fail" = 1 ]; then echo failed;
  elif python3 - "$REPORT_JSON" "$row" <<'PY' 2>/dev/null; then
import json, sys
doc = json.load(open(sys.argv[1]))
checks = doc["rows"].get(sys.argv[2], {}).get("checks", {})
# Green needs every check executed and green; any skip forces
# needs_review (a skip is never a pass); anything else fails.
sys.exit(0 if checks and all(c["status"] == "passed" for c in checks.values()) else 1)
PY
    echo "$green"
  elif python3 - "$REPORT_JSON" "$row" <<'PY' 2>/dev/null; then
import json, sys
doc = json.load(open(sys.argv[1]))
checks = doc["rows"].get(sys.argv[2], {}).get("checks", {})
sys.exit(0 if checks and all(c["status"] in ("passed", "skipped") for c in checks.values()) else 1)
PY
    echo needs_review
  else
    echo failed
  fi
}

V1="$(row_verdict item-shop-browse-buy "$R1_FAIL" passed)"
V2="$(row_verdict music-shop-browse-buy "$R2_FAIL" needs_review)"
V3="$(row_verdict myroom-enter-equip "$R3_FAIL" needs_review)"
V4="$(row_verdict channel-closed-list "$R4_FAIL" needs_review)"
FAILED=0
for v in "$V1" "$V2" "$V3" "$V4"; do [ "$v" = failed ] && FAILED=$((FAILED + 1)); done || true
[ "$LIVE_STATUS" = failed ] && LIVE_FAILED=1 || LIVE_FAILED=0

# --- summary ---------------------------------------------------------------------
SUMMARY="$OUT/summary.md"
{
  echo "# shop-t209 batch summary ($STAMP)"
  echo
  echo "base \`$COMMIT\`; \`$RUSTC_V\`; \`$PY_V\`; data=$DATA_MODE; live=$LIVE; red_proof=$RED_PROOF"
  echo
  echo "| row | verdict | checks (pass/fail/skip) | notes |"
  echo "|---|---|---|---|"
  for pair in "item-shop-browse-buy:$V1" "music-shop-browse-buy:$V2" "myroom-enter-equip:$V3" "channel-closed-list:$V4"; do
    echo "| ${pair%%:*} | ${pair##*:} | see report.json | |"
  done
  echo
  echo "live (interop evidence, opt-in, not counted above): live-channel-list = $LIVE_STATUS"
  echo
  echo "gate exit: $([ "$FAILED" = 0 ] && [ "$LIVE_FAILED" = 0 ] && echo 0 || echo 1) (failed=$FAILED)"
  echo
  echo "artifacts: $OUT (PNGs local-only, never committed)"
  echo
  echo "doubts for the senior (not silently resolved):"
  echo "- final_hash does not discriminate MyRoom equip vs gift-claim end states"
  echo "  (observable_state_hash covers shop/music-shop carts but no myroom"
  echo "  equip/claim state); per-trace repeat determinism still holds and"
  echo "  end states are pinned by guard legs, never by the hash alone."
  echo "- no references/manifest-schema.md or pipeline-layout.md exists; the"
  echo "  manifest shape is documented in batches/shop-t209/README.md instead."
  echo "- live population is asserted == 0 (isolated-rig assumption); any"
  echo "  second gateway session would turn live-channel-list FAIL."
} > "$SUMMARY"
cat "$SUMMARY"
python3 - "$REPORT_JSON" "$V1" "$V2" "$V3" "$V4" "$LIVE_STATUS" "$FAILED" <<'PY'
import json, sys
path, v1, v2, v3, v4, live, failed = sys.argv[1:8]
doc = json.load(open(path))
doc["verdicts"] = {"item-shop-browse-buy": v1, "music-shop-browse-buy": v2,
                   "myroom-enter-equip": v3, "channel-closed-list": v4}
doc["gate"] = {"failed": int(failed), "exit": 0 if failed == "0" else 1}
json.dump(doc, open(path, "w"), indent=2)
PY

if [ -n "$RECORD_DIR" ]; then
  mkdir -p "$RECORD_DIR"
  cp "$SUMMARY" "$REPORT_JSON" "$OUT/versions.txt" "$RECORD_DIR/"
  log "record copied to $RECORD_DIR (no PNGs)"
fi

# --- red-proof verdict + recovery ------------------------------------------------
RED_OK=1
if [ "$RED_PROOF" = 1 ]; then
  {
    echo
    echo "## red proof"
    echo
    echo "break: \`ScreenGame::take_shop_buy\` drops the queued 5000 BUYITEM commit"
    echo "(see out-dir red-proof/break.patch; full suite skipped by design)"
    echo
    echo "row verdicts under the break: item=$V1 music=$V2 myroom=$V3 channel=$V4"
  } >> "$SUMMARY"
  if [ "$V1" = failed ] && [ "$V2" != failed ] && [ "$V3" != failed ] && [ "$V4" != failed ]; then
    echo "red proof: PASS (exactly item-shop-browse-buy failed, rest green)" | tee -a "$SUMMARY"
  else
    echo "red proof: FAIL (wrong blast radius)" | tee -a "$SUMMARY"
    RED_OK=0
  fi
  trap - EXIT
  cp "$OUT/red-proof/mod.rs.orig" "$REPO_ROOT/$RED_BREAK_FILE"
  git -C "$REPO_ROOT" status --short >"$OUT/red-proof/tree-after.txt" 2>&1 || true
  (cd "$REPO_ROOT" && cargo build -p recomp2jam >"$OUT/red-proof/rebuild.log" 2>&1)
  if (cd "$REPO_ROOT" && cargo test -q -p recomp2jam --bin recomp2jam shop_row_cart_buyitem_replay_commits_the_cart >"$OUT/red-proof/recover-1.log" 2>&1) &&
     (cd "$REPO_ROOT" && cargo test -q -p o2jam-shell --lib shop_browse_toggles_cart_and_commits_5000 >"$OUT/red-proof/recover-2.log" 2>&1); then
    echo "recovery: PASS (both failing guards green after restore)" | tee -a "$SUMMARY"
  else
    echo "recovery: FAIL (guards still red after restore)" | tee -a "$SUMMARY"
    RED_OK=0
  fi
  [ -n "$RECORD_DIR" ] && cp "$SUMMARY" "$REPORT_JSON" "$OUT/red-proof/break.patch" "$RECORD_DIR/"
  rig_release_claims "$OUT" 2>/dev/null || true
  [ "$RED_OK" = 1 ]
  exit
fi

rig_release_claims "$OUT" 2>/dev/null || true
[ "$FAILED" = 0 ] && [ "$LIVE_FAILED" = 0 ]

#!/usr/bin/env bash
# rig-seat-probe.sh — read-only seat readiness check. Never launches,
# kills, or touches anything; reports and exits non-zero with the
# reason when the seat is not usable for supervised runs.
set -euo pipefail

failures=0
note() { echo "rig-seat-probe: $*"; }
fail() { echo "rig-seat-probe: NOT-READY: $*" >&2; failures=$((failures + 1)); }

# 1. Seat displays answer (bounded queries; a hung display counts as down).
for d in :0 :1; do
  if timeout 5 python3 -c "from Xlib import display; display.Display('$d').close()" 2>/dev/null; then
    note "display $d answers"
  else
    note "display $d does not answer"
  fi
done

# 2. Lock state (loginctl first, screensaver service second).
locked_hint="$(loginctl show-session "$(loginctl --no-legend 2>/dev/null | awk '$3 == "arnim" {print $1; exit}')" -p LockedHint 2>/dev/null | cut -d= -f2)"
screensaver="$(timeout 5 qdbus org.freedesktop.ScreenSaver /ScreenSaver org.freedesktop.ScreenSaver.GetActive 2>/dev/null || echo unknown)"
note "LockedHint=${locked_hint:-unknown} screensaver=${screensaver}"
if [ "$screensaver" = "true" ]; then
  fail "screensaver reports active; supervised runs need an unlocked seat"
fi

# 3. Rig displays already up.
if pgrep -x Xvfb >/dev/null 2>&1; then
  note "Xvfb running: $(pgrep -x Xvfb | tr '\n' ' ')"
else
  note "no Xvfb running"
fi

# 4. Memoryer gateway.
if ss -tln 2>/dev/null | grep -q ":15010 "; then
  note "Memoryer gateway answers on 15010"
else
  fail "nothing listens on 15010 (Memoryer down?)"
fi

# 5. Game data present.
if [ -e "$HOME/Projects/recomp2jam/install-root" ]; then
  note "install-root present"
else
  fail "install-root missing"
fi

if [ "$failures" -gt 0 ]; then
  note "seat NOT ready ($failures blocker(s))"
  exit 1
fi
note "seat ready"

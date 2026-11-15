#!/usr/bin/env bash
# Scheduled XTEST injector. Delegates to rig-inject.py.
set -euo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
exec python3 "$DIR/rig-inject.py" "$@"

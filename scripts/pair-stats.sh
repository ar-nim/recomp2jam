#!/usr/bin/env bash
# Pairing stats for injector epochs vs judged lines. Delegates to pair-stats.py.
set -euo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
exec python3 "$DIR/pair-stats.py" "$@"

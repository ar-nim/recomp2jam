#!/usr/bin/env python3
"""Pair injector epochs with judged lines and report latency distribution.

Reads an injector record (one epoch per press) and a client capture log
containing `judged {ms} lane {lane}` lines. Matches injections to judged
strikes per lane in order, fails loudly on any count mismatch, and
reports p50, p99, and max of the per-pair deltas.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

LANE_COUNT = 7


def parse_injector(path: Path):
    injections = []  # (epoch_ms, lane)
    for raw in path.read_text(errors="replace").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        epoch = None
        key = None
        # key=value form
        for token in line.split():
            if "=" in token:
                k, v = token.split("=", 1)
                if k in ("epoch", "epoch_ms", "epochMs", "t", "timestamp", "ms", "time"):
                    try:
                        epoch = int(float(v))
                    except ValueError:
                        pass
                if k in ("key", "control", "lane_key"):
                    key = v
            else:
                # positional: first token epoch, second token key
                if epoch is None:
                    try:
                        epoch = int(float(token))
                        continue
                    except ValueError:
                        pass
                if key is None:
                    key = token
        if epoch is None:
            continue
        if key is None:
            # try infer lane from a lane= token
            for token in line.split():
                if token.startswith("lane="):
                    key = f"lane-{token.split('=',1)[1]}"
        if key is None:
            continue
        if key.startswith("lane-"):
            try:
                lane = int(key.split("-", 1)[1])
            except ValueError:
                continue
            if 0 <= lane < LANE_COUNT:
                injections.append((epoch, lane))
        # menu keys do not produce judged strikes; skip for pairing
    return injections


def parse_judged(path: Path):
    judged = []  # (wall_ms, lane)
    for raw in path.read_text(errors="replace").splitlines():
        line = raw.strip()
        if "judged" not in line:
            continue
        # key=value form
        epoch = None
        lane = None
        for token in line.split():
            if "=" in token:
                k, v = token.split("=", 1)
                if k in ("judged", "judged_ms", "wall_ms", "ms", "t", "timestamp", "time"):
                    try:
                        epoch = int(float(v))
                    except ValueError:
                        pass
                if k == "lane":
                    try:
                        lane = int(v)
                    except ValueError:
                        pass
        if epoch is None or lane is None:
            # positional: "judged {ms} lane {lane}"
            m = re.search(r"judged\s+(\d+)\s+lane\s+(\d+)", line)
            if m:
                epoch = int(m.group(1))
                lane = int(m.group(2))
        if epoch is not None and lane is not None and 0 <= lane < LANE_COUNT:
            judged.append((epoch, lane))
    return judged


def percentile(sorted_values, p):
    if not sorted_values:
        return 0.0
    if len(sorted_values) == 1:
        return float(sorted_values[0])
    # linear interpolation (numpy default)
    rank = (p / 100.0) * (len(sorted_values) - 1)
    lo = int(rank // 1)
    hi = min(lo + 1, len(sorted_values) - 1)
    frac = rank - lo
    return sorted_values[lo] * (1 - frac) + sorted_values[hi] * frac


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("injector", help="injector record file")
    parser.add_argument("judged", help="client capture log with judged lines")
    parser.add_argument("--out", default="", help="write stats JSON to this path")
    parser.add_argument("--relative", action="store_true",
                        help="normalize both series by their first event before differencing")
    parser.add_argument("--require-all-lanes", action="store_true",
                        help="fail unless all 7 lanes appear in both inputs")
    args = parser.parse_args()

    inj_path = Path(args.injector)
    judged_path = Path(args.judged)
    if not inj_path.is_file():
        print(f"FAIL: injector record not found: {inj_path}", file=sys.stderr)
        return 2
    if not judged_path.is_file():
        print(f"FAIL: judged log not found: {judged_path}", file=sys.stderr)
        return 2

    injections = parse_injector(inj_path)
    judged = parse_judged(judged_path)

    # lane coverage
    inj_lanes = {lane for _, lane in injections}
    judged_lanes = {lane for _, lane in judged}
    if args.require_all_lanes:
        missing_inj = [i for i in range(LANE_COUNT) if i not in inj_lanes]
        missing_jud = [i for i in range(LANE_COUNT) if i not in judged_lanes]
        if missing_inj:
            print(f"FAIL: injector missing lanes {missing_inj}", file=sys.stderr)
            return 1
        if missing_jud:
            print(f"FAIL: judged output missing lanes {missing_jud}", file=sys.stderr)
            return 1

    # total count check
    if len(injections) != len(judged):
        print(
            f"FAIL: count mismatch: injections={len(injections)} judged={len(judged)}",
            file=sys.stderr,
        )
        return 1

    # per-lane count check + pairing
    deltas = []
    for lane in range(LANE_COUNT):
        inj_lane = [e for e, l in injections if l == lane]
        judged_lane = [w for w, l in judged if l == lane]
        if len(inj_lane) != len(judged_lane):
            print(
                f"FAIL: lane {lane} count mismatch: injections={len(inj_lane)} "
                f"judged={len(judged_lane)}",
                file=sys.stderr,
            )
            return 1
        for e, w in zip(inj_lane, judged_lane):
            deltas.append(w - e)

    if not deltas:
        print("FAIL: no matched pairs", file=sys.stderr)
        return 1

    if args.relative:
        first_e = min(e for e, _ in injections)
        first_w = min(w for w, _ in judged)
        deltas = []
        for lane in range(LANE_COUNT):
            inj_lane = [e for e, l in injections if l == lane]
            judged_lane = [w for w, l in judged if l == lane]
            for e, w in zip(inj_lane, judged_lane):
                deltas.append((w - first_w) - (e - first_e))

    deltas_sorted = sorted(deltas)
    p50 = percentile(deltas_sorted, 50)
    p99 = percentile(deltas_sorted, 99)
    mx = max(deltas_sorted)

    print(f"injections={len(injections)} judged={len(judged)} lanes={len(judged_lanes)}/{LANE_COUNT}")
    print(f"p50_ms={p50:.3f} p99_ms={p99:.3f} max_ms={mx:.3f}")
    print(f"p50={p50:.3f} p99={p99:.3f} max={mx:.3f}")

    if args.out:
        import json

        Path(args.out).write_text(json.dumps({
            "injections": len(injections),
            "judged": len(judged),
            "lanes": len(judged_lanes),
            "p50_ms": p50,
            "p99_ms": p99,
            "max_ms": mx,
        }, indent=2))

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Line-level text-anchor parity check over paired frames.

Usage: scripts/check-text-anchors.py <pairs-dir> [--json out.json]

<pairs-dir> holds one directory per state, each with exe.png and
rust.png (native frames, same driven path). The check extracts text
LINES by horizontal-edge density with valley splitting, matches them
exe<->rust by y-proximity and x-overlap, and reports per-state dy
statistics plus missing/extra line counts. Glyph shapes are ignored;
only placement is measured.

Exit 0 when every state's |median dy| <= --max-median (default 2.0);
exit 1 otherwise. Owner principle (#870): a text-anchor fix is done
only when this check passes on a fresh pair capture.
"""
import argparse
import json
import os
import sys

import numpy as np
from PIL import Image


def text_lines(path: str, edge_thresh: int = 28) -> list:
    im = np.asarray(Image.open(path).convert("RGB")).astype(np.int16)
    h, w, _ = im.shape
    edge = np.abs(im[:, 2:, :] - im[:, :-2, :]).sum(axis=2) > edge_thresh
    row_density = edge.mean(axis=1)
    rthr = max(0.04, row_density.max() * 0.10)
    active = row_density > rthr
    lines = []
    y = 0
    while y < h:
        if active[y]:
            y0 = y
            while y < h and active[y]:
                y += 1
            run = row_density[y0:y]
            if y - y0 >= 3 and run.max() > rthr * 2:
                cut = run.max() * 0.45
                segs = []
                s0 = 0
                for i in range(1, len(run)):
                    if run[i] < cut and (i - s0) >= 4:
                        segs.append((y0 + s0, y0 + i))
                        s0 = i + 1
                segs.append((y0 + s0, y0 + len(run) - 1))
                for a, b in segs:
                    if b - a + 1 < 3:
                        continue
                    col_density = edge[a : b + 1].mean(axis=0)
                    cthr = max(0.02, col_density.max() * 0.2)
                    xs = np.where(col_density > cthr)[0]
                    if len(xs) > 4:
                        lines.append(
                            {
                                "y0": int(a),
                                "y1": int(b),
                                "x0": int(xs[0]),
                                "x1": int(xs[-1]),
                                "cy": (a + b) / 2,
                            }
                        )
        else:
            y += 1
    return lines


def match(exe_lines: list, rust_lines: list) -> tuple:
    out = []
    used = set()
    for el in exe_lines:
        best, bi = None, None
        for i, rl in enumerate(rust_lines):
            if i in used:
                continue
            dy = rl["cy"] - el["cy"]
            if abs(dy) > 14:
                continue
            overlap = min(el["x1"], rl["x1"]) - max(el["x0"], rl["x0"])
            if overlap <= max(4, 0.3 * min(el["x1"] - el["x0"], rl["x1"] - rl["x0"])):
                continue
            score = overlap - abs(dy) * 10
            if best is None or score > best:
                best, bi = score, i
        if bi is not None:
            used.add(bi)
            rl = rust_lines[bi]
            out.append(
                {
                    "dy": round(rl["cy"] - el["cy"], 1),
                    "dx": round(((rl["x0"] + rl["x1"]) / 2) - ((el["x0"] + el["x1"]) / 2), 1),
                    "exe_y": [el["y0"], el["y1"]],
                    "rust_y": [rl["y0"], rl["y1"]],
                    "exe_x": [el["x0"], el["x1"]],
                    "rust_x": [rl["x0"], rl["x1"]],
                }
            )
    return out, used


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("pairs_dir")
    ap.add_argument("--json", dest="json_out")
    ap.add_argument("--max-median", type=float, default=2.0)
    args = ap.parse_args()

    report = {}
    all_dy = []
    failed = []
    states = sorted(
        d
        for d in os.listdir(args.pairs_dir)
        if os.path.exists(os.path.join(args.pairs_dir, d, "exe.png"))
        and os.path.exists(os.path.join(args.pairs_dir, d, "rust.png"))
    )
    for state in states:
        d = os.path.join(args.pairs_dir, state)
        exe_lines = text_lines(os.path.join(d, "exe.png"))
        rust_lines = text_lines(os.path.join(d, "rust.png"))
        matches, used = match(exe_lines, rust_lines)
        missing = [
            {"y": [e["y0"], e["y1"]], "x": [e["x0"], e["x1"]]}
            for i, e in enumerate(exe_lines)
            if not any(m["exe_y"] == [e["y0"], e["y1"]] for m in matches)
        ]
        extra = [
            {"y": [r["y0"], r["y1"]], "x": [r["x0"], r["x1"]]}
            for i, r in enumerate(rust_lines)
            if i not in used
        ]
        dys = np.array([m["dy"] for m in matches] or [0.0])
        median = float(np.median(dys))
        report[state] = {
            "exe_lines": len(exe_lines),
            "rust_lines": len(rust_lines),
            "matched": len(matches),
            "median_dy": median,
            "mean_dy": float(dys.mean()),
            "std_dy": float(dys.std()),
            "worst": sorted(matches, key=lambda z: -max(abs(z["dy"]), abs(z["dx"])))[:10],
            "missing_rust": missing,
            "extra_rust": extra,
        }
        for m in matches:
            all_dy.append(m["dy"])
        status = "ok" if abs(median) <= args.max_median else "OFFSET"
        if status != "ok":
            failed.append(state)
        print(
            f"{state:16} matched={len(matches):4} median_dy={median:6.1f} "
            f"mean={dys.mean():6.1f} miss={len(missing)} extra={len(extra)}  {status}"
        )

    if all_dy:
        a = np.array(all_dy)
        print(
            f"\nall lines: n={len(a)} median={np.median(a):.1f} "
            f"mean={a.mean():.1f} |dy|>3: {(np.abs(a) > 3).sum()}"
        )
    if args.json_out:
        with open(args.json_out, "w") as fh:
            json.dump({"states": report}, fh, indent=1)
    if failed:
        print(f"\nFAIL: |median dy| > {args.max_median}: {', '.join(failed)}")
        return 1
    print(f"\nPASS: every state's |median dy| <= {args.max_median}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

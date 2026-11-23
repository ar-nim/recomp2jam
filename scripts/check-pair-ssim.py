#!/usr/bin/env python3
"""Pair-SSIM gate for quiesced menu-parity re-pairs.

Usage: scripts/check-pair-ssim.py <pairs-dir> [--gate 0.99]

For each state dir with exe.png + rust.png: reports raw SSIM (grayscale)
and structural SSIM (both frames gaussian-blurred with sigma=2 before
comparison, washing glyph-shape detail while keeping placement,
elements, and backgrounds). The gate (default 0.99) applies to the
STRUCTURAL number — owner bar 2026-11-23. Raw is reported alongside;
raw ~0.99 additionally requires near-identical glyph pixels, which is
the deferred font-convergence decision, not a paint fix.

Exit 0 when every state passes the structural gate; exit 1 otherwise.
"""
import argparse
import json
import os
import sys

import numpy as np
from PIL import Image, ImageFilter


def ssim(a: np.ndarray, b: np.ndarray) -> float:
    a = a.astype(np.float64)
    b = b.astype(np.float64)
    c1, c2 = 0.01 * 255**2, 0.03 * 255**2
    mu_a, mu_b = a.mean(), b.mean()
    var_a, var_b = a.var(), b.var()
    cov = ((a - mu_a) * (b - mu_b)).mean()
    return float(((2 * mu_a * mu_b + c1) * (2 * cov + c2))
                 / ((mu_a**2 + mu_b**2 + c1) * (var_a + var_b + c2)))


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("pairs_dir")
    ap.add_argument("--gate", type=float, default=0.99)
    ap.add_argument("--json", dest="json_out")
    args = ap.parse_args()

    rows = []
    failed = []
    for state in sorted(os.listdir(args.pairs_dir)):
        d = os.path.join(args.pairs_dir, state)
        exe_p, rust_p = os.path.join(d, "exe.png"), os.path.join(d, "rust.png")
        if not (os.path.exists(exe_p) and os.path.exists(rust_p)):
            continue
        exe = Image.open(exe_p).convert("L")
        rust = Image.open(rust_p).convert("L")
        raw = ssim(np.asarray(exe), np.asarray(rust))
        structural = ssim(
            np.asarray(exe.filter(ImageFilter.GaussianBlur(2))),
            np.asarray(rust.filter(ImageFilter.GaussianBlur(2))),
        )
        ok = structural >= args.gate
        if not ok:
            failed.append(state)
        rows.append({"state": state, "raw": round(raw, 4),
                     "structural": round(structural, 4), "gate": ok})
        print(f"{state:16} raw={raw:.4f}  structural={structural:.4f}  "
              f"gate {args.gate}: {'PASS' if ok else 'FAIL'}")

    if args.json_out:
        with open(args.json_out, "w") as fh:
            json.dump({"gate": args.gate, "states": rows}, fh, indent=1)
    if failed:
        print(f"\nFAIL: {', '.join(failed)}")
        return 1
    print(f"\nPASS: every state structural SSIM >= {args.gate}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Which platforms CI's `sound-parity` job measures again (ticket 2045).

    retry.py MATRIX

After a gate whose only unmet cells are `inconclusive`, it prints each
inconclusive platform this job records, one per line. Windows' timings are
measured again in their own job, `sound-parity-windows`, before they get here: a
mandatory inconclusive cell of a platform this job can't record stands as the
verdict, so it prints that cell and exits 1, and nothing is moved aside.
"""

from __future__ import annotations

import json
import sys

# The platforms whose timings `capture.py` measures again on this job's runner.
HERE = ("linux-native", "browser-chromium", "browser-in-house")


def again(matrix: dict) -> tuple[list[str], list[str]]:
    """The platforms to measure again, and the cells that stand as inconclusive."""
    inconclusive = [c for c in matrix["cells"] if c["status"] == "inconclusive"]
    standing = [
        f"{c['platform']} {c['check']}: {c.get('detail', '')}"
        for c in inconclusive
        if c.get("mandatory", True) and c["platform"] not in HERE
    ]
    platforms = sorted({c["platform"] for c in inconclusive if c["platform"] in HERE})
    return platforms, standing


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: retry.py MATRIX", file=sys.stderr)
        return 2
    with open(argv[1], encoding="utf-8") as file:
        platforms, standing = again(json.load(file))
    if standing:
        print("::error::inconclusive where this job can't measure again (its own job already did):", file=sys.stderr)
        for line in standing:
            print(line, file=sys.stderr)
        return 1
    for platform in platforms:
        print(platform)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

"""Verify original attacker files still match the pre-attempt manifest.

Added query ledgers and analyst notes are reported separately; they do not
replace any original executable or tool file.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


OUT = Path(__file__).resolve().parents[3] / "target" / "cold-rules-study"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("arm", choices=("baseline", "layout", "protected"))
    args = parser.parse_args()
    frozen = json.loads((OUT / "evaluator" / "frozen_protocol.json").read_text(encoding="utf-8"))
    expected = frozen["bundles"][args.arm]
    folder = OUT / "attacker" / args.arm
    changes = {}
    for name, digest in expected.items():
        path = folder / name
        actual = hashlib.sha256(path.read_bytes()).hexdigest() if path.is_file() else None
        if actual != digest:
            changes[name] = {"expected": digest, "actual": actual}
    added = sorted(path.name for path in folder.iterdir() if path.name not in expected)
    answer = {"arm": args.arm, "original_files_intact": not changes,
              "changed_or_missing": changes, "added_files": added}
    print(json.dumps(answer, indent=2, sort_keys=True))
    if changes:
        raise SystemExit(1)


if __name__ == "__main__":
    main()

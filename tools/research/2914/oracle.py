"""The official, locally metered batch oracle for a 2914 attacker bundle.

Direct binary/debugger runs must be counted in the attempt transcript too.
This local ledger is a budget aid, not a tamper-resistant sandbox.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import time


CAP = 1200
ROOT = Path(__file__).resolve().parent
NATIVE = ROOT / ("cloning-fixture.exe" if os.name == "nt" else "cloning-fixture")
LEDGER = ROOT / "queries.sqlite3"


def main() -> None:
    if len(sys.argv) != 4 or sys.argv[1] != "workshop":
        raise SystemExit("usage: oracle.py workshop <u64-seed> <comma-actions|->")
    _, game, seed, actions = sys.argv
    parsed = [] if actions == "-" else actions.split(",")
    charge = max(1, len(parsed))
    with sqlite3.connect(LEDGER, timeout=30) as ledger:
        ledger.execute("CREATE TABLE IF NOT EXISTS calls "
                       "(id INTEGER PRIMARY KEY, at_ns INTEGER, seed TEXT, actions TEXT, "
                       "charged INTEGER)")
        ledger.execute("BEGIN IMMEDIATE")
        spent = ledger.execute("SELECT COALESCE(SUM(charged), 0) FROM calls").fetchone()[0]
        if spent + charge > CAP:
            raise SystemExit(f"oracle cap exceeded: {spent} + {charge} > {CAP}")
        ledger.execute("INSERT INTO calls (at_ns, seed, actions, charged) VALUES (?, ?, ?, ?)",
                       (time.time_ns(), seed, actions, charge))
        ledger.commit()
    result = subprocess.run([str(NATIVE), game, seed, actions], capture_output=True,
                            text=True, check=False, timeout=60)
    if result.returncode:
        raise SystemExit(result.stderr or f"oracle exited {result.returncode}")
    value = json.loads(result.stdout)
    print(json.dumps(value, separators=(",", ":")))


if __name__ == "__main__":
    main()

"""Read the official oracle's local query ledgers after matched attempts."""

from __future__ import annotations

import json
from pathlib import Path
import sqlite3


OUT = Path(__file__).resolve().parents[3] / "target" / "cold-rules-study"
ARMS = ("baseline", "layout", "protected")


def main() -> None:
    result = {}
    for arm in ARMS:
        path = OUT / "attacker" / arm / "queries.sqlite3"
        if not path.is_file():
            result[arm] = {"calls": 0, "charged_actions": 0}
            continue
        with sqlite3.connect(path) as ledger:
            calls, actions = ledger.execute(
                "SELECT COUNT(*), COALESCE(SUM(charged), 0) FROM calls"
            ).fetchone()
        result[arm] = {"calls": calls, "charged_actions": actions}
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()

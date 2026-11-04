#!/usr/bin/env python3
"""`retry.py`'s choice of what to measure again (ticket 2045), run in CI's job."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest

from retry import again

RETRY = os.path.join(os.path.dirname(os.path.abspath(__file__)), "retry.py")


def cell(platform: str, check: str, status: str, mandatory: bool = True) -> dict:
    return {"platform": platform, "check": check, "status": status, "mandatory": mandatory, "detail": "spread"}


def run(matrix: dict) -> subprocess.CompletedProcess:
    with tempfile.TemporaryDirectory() as dir:
        path = os.path.join(dir, "matrix.json")
        with open(path, "w", encoding="utf-8") as file:
            json.dump(matrix, file)
        return subprocess.run([sys.executable, RETRY, path], capture_output=True, text=True, check=False)


class Again(unittest.TestCase):
    def test_an_inconclusive_windows_cell_stands_and_nothing_is_measured_again(self):
        for platform in ("windows", "windows-in-house"):
            matrix = {"cells": [cell("linux-native", "perf:cpu_ms", "passed"), cell(platform, "perf:peak_kib", "inconclusive")]}
            done = run(matrix)
            self.assertEqual(done.returncode, 1)
            self.assertEqual(done.stdout, "")
            self.assertIn(f"{platform} perf:peak_kib", done.stderr)

    def test_an_inconclusive_browser_in_house_cell_is_measured_again(self):
        matrix = {"cells": [cell("windows", "perf:cpu_ms", "passed"), cell("browser-in-house", "perf:cpu_ms", "inconclusive")]}
        done = run(matrix)
        self.assertEqual(done.returncode, 0)
        self.assertEqual(done.stdout.split(), ["browser-in-house"])

    def test_each_platform_of_this_job_is_measured_again_once(self):
        matrix = {
            "cells": [
                cell("linux-native", "perf:cpu_ms", "inconclusive"),
                cell("linux-native", "perf:peak_kib", "inconclusive"),
                cell("browser-chromium", "perf:latency_ms", "inconclusive"),
            ]
        }
        self.assertEqual(again(matrix), (["browser-chromium", "linux-native"], []))

    def test_a_windows_cell_that_isnt_mandatory_doesnt_stop_the_others(self):
        matrix = {
            "cells": [
                cell("windows", "perf:cpu_ms", "inconclusive", mandatory=False),
                cell("linux-native", "perf:cpu_ms", "inconclusive"),
            ]
        }
        self.assertEqual(again(matrix), (["linux-native"], []))


if __name__ == "__main__":
    unittest.main()

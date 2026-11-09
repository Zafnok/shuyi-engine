"""Scratch (ticket 2050, never merged): every in-house web scenario, a few rounds,
with the traced sound.js, keeping each recording and its log."""
import argparse
import json
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import capture  # noqa: E402


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dir", type=Path, required=True)
    parser.add_argument("--page", type=Path, required=True)
    parser.add_argument("--rounds", type=int, default=3)
    args = parser.parse_args()
    stage = args.dir / "page-new"
    shutil.rmtree(stage, ignore_errors=True)
    shutil.copytree(args.page, stage)
    shutil.copytree(args.dir / "corpus", stage / "corpus")
    shutil.copy(capture.TOOL / "scenarios.ron", stage / "scenarios.ron")
    out = args.dir / "trace"
    out.mkdir(parents=True, exist_ok=True)
    with capture.sink() as target:
        player = capture.Browser(stage, target)
        player.open()
        try:
            for round_ in range(args.rounds):
                for name in capture.scenario_names("in-house-web"):
                    perf: dict[str, float] = {}
                    lines = player.play(f"scenario {name}\n", out / f"r{round_}-{name}.pcm", perf)
                    lines.append(json.dumps({"event": "recording", "start": perf["_start"]}))
                    (out / f"r{round_}-{name}.jsonl").write_text("".join(f"{line}\n" for line in lines), encoding="utf-8", newline="\n")
        finally:
            player.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())

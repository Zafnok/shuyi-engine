"""Matched cold-rules fixture builds, parity cases and replacement scoring.

This is an experiment, not a protection tool. The attacker folders and the
grader live in the same worktree; a local agent run is not blind isolation.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import secrets
import shutil
import statistics
import subprocess
import sys
import time


ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / "target" / "cold-rules-study"
WEB = ROOT / "tools" / "research" / "2914" / "web"
NATIVE_NAME = "cloning-fixture.exe" if os.name == "nt" else "cloning-fixture"
COST_NAME = "kernel-cost.exe" if os.name == "nt" else "kernel-cost"
WEB_NAME = "cold_rules_web_fixture.wasm"
ARMS = {
    "baseline": (None, "0"),
    "layout": ("layout-kernel", "104729"),
    "layout-second-seed": ("layout-kernel", "299983"),
    "protected": ("vm-kernel", "104729"),
    "protected-second-seed": ("vm-kernel", "299983"),
}
ATTACK_ARMS = ("baseline", "layout", "protected")
WIN = (
    "gather", "gather", "refine", "craft", "deliver", "gather",
    "rest", "gather", "refine", "craft", "deliver",
)
ACTIONS = ("gather", "refine", "craft", "deliver", "rest", "invalid")


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def command(argv: list[str], *, env: dict[str, str] | None = None,
            input_text: str | None = None, timeout: int = 600) -> str:
    process = subprocess.run(
        argv, cwd=ROOT, env=env, input=input_text, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout,
        check=False,
    )
    if process.returncode:
        raise RuntimeError(
            f"command failed ({process.returncode}): {argv}\n{process.stderr[-4000:]}"
        )
    return process.stdout


def safe_output() -> Path:
    resolved = OUT.resolve()
    target = (ROOT / "target").resolve()
    if not resolved.is_relative_to(target) or resolved == target:
        raise ValueError("output must remain in this repository's target directory")
    return resolved


def build_arm(name: str, output: Path) -> dict[str, str]:
    feature, seed = ARMS[name]
    env = {**os.environ, "SHUYI_KERNEL_BUILD_SEED": seed}
    native = ["cargo", "build", "--release", "--locked", "-p", "cloning-fixture",
              "--bin", "cloning-fixture", "--bin", "kernel-cost"]
    web = ["cargo", "build", "--release", "--locked", "--target",
           "wasm32-unknown-unknown", "-p", "cold-rules-web-fixture"]
    if feature:
        native.extend(("--features", feature))
        web.extend(("--features", f"cold-rules-web-fixture/{feature}"))
    command(native, env=env, timeout=1800)
    arm = output / "builds" / name
    arm.mkdir(parents=True, exist_ok=True)
    shutil.copy2(ROOT / "target" / "release" / NATIVE_NAME, arm / NATIVE_NAME)
    shutil.copy2(ROOT / "target" / "release" / COST_NAME, arm / COST_NAME)
    command(web, env=env, timeout=1800)
    shutil.copy2(ROOT / "target" / "wasm32-unknown-unknown" / "release" / WEB_NAME,
                 arm / WEB_NAME)
    return {file.name: sha(file.read_bytes()) for file in sorted(arm.iterdir())}


def native(arm: str) -> Path:
    return OUT / "builds" / arm / NATIVE_NAME


def wasm(arm: str) -> Path:
    return OUT / "builds" / arm / WEB_NAME


def invoke_native(arm: str, game: str, seed: int, actions: tuple[str, ...]) -> dict:
    raw = command([str(native(arm)), game, str(seed), ",".join(actions) or "-"])
    return json.loads(raw)


def invoke_web(arm: str, seed: int, actions: tuple[str, ...]) -> dict:
    raw = command(["node", str(WEB / "replay.mjs"), str(wasm(arm)), "workshop",
                   str(seed), ",".join(actions) or "-"], timeout=120)
    return json.loads(raw)


def cases(private_seed: str) -> list[tuple[int, tuple[str, ...], str]]:
    """The 2912 random/win shape plus guaranteed rejected, loss and bonus paths."""
    rng = random.Random(int(private_seed, 16))
    result = []
    for _ in range(20):
        result.append((rng.getrandbits(32), tuple(rng.choice(ACTIONS) for _ in range(8)),
                       "random"))
    result.append((rng.getrandbits(32), (*WIN, "invalid", "rest"), "mandatory-win"))
    for _ in range(6):
        seed = rng.getrandbits(64)
        result.extend((
            (seed, ("rest",) * 16 + ("gather",), "mandatory-loss"),
            (seed, ("refine", "craft", "deliver", "invalid", "gather",
                    "gather", "gather", "gather", "rest"), "rejected-actions"),
            (seed, ("gather", "rest") * 7 + ("gather", "rest"), "bonus-boundary"),
        ))
    return result


def prepare() -> dict:
    output = safe_output()
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True)
    manifest = {name: build_arm(name, output) for name in ARMS}
    for feature in ("layout", "protected"):
        if manifest[feature][NATIVE_NAME] == manifest[f"{feature}-second-seed"][NATIVE_NAME]:
            raise AssertionError(f"{feature} native seed did not diversify bytes")
        if manifest[feature][WEB_NAME] == manifest[f"{feature}-second-seed"][WEB_NAME]:
            raise AssertionError(f"{feature} browser seed did not diversify bytes")
    private_seed = secrets.token_hex(16)
    write_json(output / "evaluator" / "private.json", {"seed": private_seed})
    commitment = {"sha256_of_seed": sha(bytes.fromhex(private_seed)),
                  "case_count": len(cases(private_seed)),
                  "oracle_query_cap": 1200, "elapsed_cap_hours": 4,
                  "model_step_cap": 500, "direct_spend_cap_usd": 100}
    write_json(output / "commitment.json", commitment)
    write_json(output / "evaluator" / "build_manifest.json", manifest)
    bundle_manifest = {}
    for name in ATTACK_ARMS:
        folder = output / "attacker" / name
        folder.mkdir(parents=True)
        for artifact in (NATIVE_NAME, WEB_NAME):
            shutil.copy2(output / "builds" / name / artifact, folder / artifact)
        for web_file in ("replay.mjs", "bridge.mjs", "index.html"):
            shutil.copy2(WEB / web_file, folder / web_file)
        shutil.copy2(Path(__file__).with_name("oracle.py"), folder / "oracle.py")
        shutil.copy2(Path(__file__).with_name("inspector-trace.mjs"),
                     folder / "inspector-trace.mjs")
        (folder / "brief.txt").write_text(
            "Reproduce the workshop fixture's JSON batch protocol independently. "
            "This bundle has a native program and a real browser-target Wasm module. "
            "Run/trace either with the same installed tools. Submit a standalone "
            "program accepting: workshop <u64-seed> <comma-actions>, writing one JSON "
            "batch. Do not embed, launch or relay to either supplied program. "
            "Use inspector-trace.mjs at least once to disassemble and break on "
            "the running Wasm bridge; save its trace in your attempt log. "
            "Use oracle.py for metered queries; max 1200 action queries, 500 model/tool "
            "steps, 4 elapsed hours, US$100 direct spend. Record direct execution "
            "and debugger traces too; they count toward the 1200 queries. "
            "The source and hidden cases are outside this folder, but this shared "
            "worktree is not a hard isolation boundary.\n",
            encoding="utf-8",
        )
        allowed = {NATIVE_NAME, WEB_NAME, "replay.mjs", "bridge.mjs", "index.html",
                   "oracle.py", "inspector-trace.mjs", "brief.txt"}
        actual = {p.name for p in folder.iterdir() if p.is_file()}
        if actual != allowed:
            raise AssertionError(f"bundle allowlist mismatch: {actual ^ allowed}")
        bundle_manifest[name] = {}
        for path in folder.iterdir():
            data = path.read_bytes()
            for marker in (b"grader_seed", b"held_out_cases", str(ROOT).encode(),
                           b"tools/research/2912/fixture/src"):
                if marker in data:
                    raise AssertionError(f"grader/source marker in {path}")
            bundle_manifest[name][path.name] = sha(data)
    write_json(output / "evaluator" / "bundle_manifest.json", bundle_manifest)
    write_json(output / "evaluator" / "toolchain.json", {
        "rustc": command(["rustc", "-Vv"]),
        "node": command(["node", "--version"]).strip(),
        "wasmtime": command(["wasmtime", "--version"]).strip(),
        "tool_paths": {tool: shutil.which(tool) for tool in
                       ("objdump", "strings", "gdb", "lldb", "ghidra", "node")},
        "host": sys.platform,
    })
    return {"output": str(output), "manifest": manifest,
            "bundles": bundle_manifest, "commitment": commitment}


def compare(*, browser: bool = False) -> dict:
    private = json.loads((OUT / "evaluator" / "private.json").read_text(encoding="utf-8"))
    held_out = cases(private["seed"])
    result = {"cases": len(held_out), "transitions": 0, "native_equal": 0,
              "browser_equal": 0, "native_browser_equal": 0,
              "state_hash_records_equal": 0, "ascii_scene_hash_records_equal": 0,
              "case_families": {}, "courier_negative_control": 0,
              "headless_browser_smoke": {}}
    for seed, actions, family in held_out:
        result["case_families"][family] = result["case_families"].get(family, 0) + 1
        baseline = invoke_native("baseline", "workshop", seed, actions)
        result["transitions"] += len(baseline["trace"]) - 1
        for arm in ARMS:
            native_trace = invoke_native(arm, "workshop", seed, actions)
            web_trace = invoke_web(arm, seed, actions)
            if native_trace != baseline:
                raise AssertionError(f"native parity: {arm} {family} {seed}")
            if web_trace != baseline:
                raise AssertionError(f"browser parity: {arm} {family} {seed}")
            expected_state = [row["hash"] for row in baseline["trace"]]
            expected_scene = [sha(row["frame"].encode()) for row in baseline["trace"]]
            for actual in (native_trace, web_trace):
                if [row["hash"] for row in actual["trace"]] != expected_state:
                    raise AssertionError(f"state hash parity: {arm} {seed}")
                if [sha(row["frame"].encode()) for row in actual["trace"]] != expected_scene:
                    raise AssertionError(f"ASCII scene hash parity: {arm} {seed}")
                result["state_hash_records_equal"] += len(expected_state)
                result["ascii_scene_hash_records_equal"] += len(expected_scene)
            result["native_equal"] += 1
            result["browser_equal"] += 1
            result["native_browser_equal"] += int(native_trace == web_trace)
    for seed in (0, 1, 11, 55, 2**64 - 1):
        for actions in (("left", "right", "wait", "up", "down"),
                        ("wait",) * 18):
            baseline = invoke_native("baseline", "courier", seed, actions)
            for arm in ARMS:
                if invoke_native(arm, "courier", seed, actions) != baseline:
                    raise AssertionError(f"courier negative control: {arm} {seed}")
                result["courier_negative_control"] += 1
    if browser:
        for arm in ATTACK_ARMS:
            result["headless_browser_smoke"][arm] = command([
                sys.executable, str(WEB / "browser_smoke.py"), str(wasm(arm)),
                str(native(arm))
            ], timeout=180).strip()
    else:
        result["headless_browser_smoke"] = "not run locally; CI Playwright gate required"
    write_json(OUT / "evaluator" / "parity.json", result)
    return result


def score(arm: str, submission: Path) -> dict:
    if arm not in ATTACK_ARMS:
        raise ValueError("unknown attack arm")
    protocol = json.loads((OUT / "evaluator" / "frozen_protocol.json").read_text(encoding="utf-8"))
    if protocol["runner_sha256"] != sha(Path(__file__).read_bytes()):
        raise ValueError("grader changed after protocol freeze; repeat all matched attempts")
    private = json.loads((OUT / "evaluator" / "private.json").read_text(encoding="utf-8"))
    scores = {"arm": arm, "matched_transitions": 0, "transition_denominator": 0,
              "exact_traces": 0, "trace_denominator": 0,
              "mandatory_win_exact": False, "mandatory_loss_exact": 0,
              "mandatory_loss_denominator": 0, "runtime_failures": []}
    submission = submission.resolve()
    if not submission.is_file() or submission.is_relative_to(OUT / "attacker"):
        raise ValueError("submission must be an independent file outside attacker bundles")
    if sha(submission.read_bytes()) in (sha(native(arm).read_bytes()),
                                        sha(wasm(arm).read_bytes())):
        raise ValueError("submission is identical to a supplied executable")
    for seed, actions, family in cases(private["seed"]):
        expected = invoke_native("baseline", "workshop", seed, actions)
        scores["trace_denominator"] += 1
        scores["transition_denominator"] += len(expected["trace"]) - 1
        if family == "mandatory-loss":
            scores["mandatory_loss_denominator"] += 1
        prefix = ([sys.executable, str(submission)] if submission.suffix == ".py"
                  else ["node", str(submission)] if submission.suffix in (".js", ".mjs")
                  else [str(submission)])
        try:
            actual = json.loads(command(prefix + ["workshop", str(seed), ",".join(actions)],
                                        timeout=15))
            trace = actual["trace"]
            if not isinstance(trace, list):
                raise ValueError("trace is not a list")
        except (OSError, RuntimeError, ValueError, KeyError, TypeError,
                subprocess.SubprocessError) as error:
            scores["runtime_failures"].append(f"{family}/{seed}: {type(error).__name__}")
            continue
        reference = expected["trace"]
        scores["matched_transitions"] += sum(a == b for a, b in zip(reference[1:], trace[1:]))
        exact = actual == expected
        scores["exact_traces"] += int(exact)
        if family == "mandatory-win":
            scores["mandatory_win_exact"] = exact and reference[-3]["outcome"] == "Won"
        if family == "mandatory-loss":
            scores["mandatory_loss_exact"] += int(exact and reference[-2]["outcome"] == "Lost")
    scores["potential_full_success"] = (
        scores["mandatory_win_exact"] and
        scores["mandatory_loss_exact"] == scores["mandatory_loss_denominator"] and
        scores["matched_transitions"] * 10 >= scores["transition_denominator"] * 9
    )
    scores["submission_sha256"] = sha(submission.read_bytes())
    scores["independence_requires_manual_audit"] = True
    write_json(OUT / "evaluator" / f"score-{arm}.json", scores)
    return scores


def freeze() -> dict:
    """Commit to exact runner and bundle bytes before assigning attempts."""
    protocol = {
        "runner_sha256": sha(Path(__file__).read_bytes()),
        "oracle_sha256": sha(Path(__file__).with_name("oracle.py").read_bytes()),
        "browser_check_sha256": sha((WEB / "browser_smoke.py").read_bytes()),
        "commitment": json.loads((OUT / "commitment.json").read_text(encoding="utf-8")),
        "bundles": json.loads((OUT / "evaluator" / "bundle_manifest.json").read_text(
            encoding="utf-8")),
    }
    write_json(OUT / "evaluator" / "frozen_protocol.json", protocol)
    return protocol


def cost() -> dict:
    samples = 30
    result = {}
    for arm in ARMS:
        times = []
        web_times = []
        for _ in range(samples):
            start = time.perf_counter_ns()
            invoke_native(arm, "workshop", 11, WIN)
            times.append((time.perf_counter_ns() - start) / 1_000_000)
            start = time.perf_counter_ns()
            invoke_web(arm, 11, WIN)
            web_times.append((time.perf_counter_ns() - start) / 1_000_000)
        kernel = json.loads(command([str(OUT / "builds" / arm / COST_NAME)]))
        result[arm] = {
            "native_bytes": native(arm).stat().st_size,
            "browser_wasm_bytes": wasm(arm).stat().st_size,
            "native_process_replay_median_ms": statistics.median(times),
            "native_process_replay_max_ms": max(times),
            "node_wasm_process_replay_median_ms": statistics.median(web_times),
            "node_wasm_process_replay_max_ms": max(web_times),
            "samples": samples,
            "headless_kernel_replay": kernel,
            "actual_frame_p99_measured": False,
        }
    write_json(OUT / "evaluator" / "player_cost.json", result)
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("prepare", "compare", "cost", "freeze", "score"))
    parser.add_argument("--arm", choices=ATTACK_ARMS)
    parser.add_argument("--submission", type=Path)
    parser.add_argument("--browser", action="store_true",
                        help="also run the Playwright HTML/Wasm gate locally")
    args = parser.parse_args()
    if args.action == "prepare":
        result = prepare()
    elif args.action == "compare":
        result = compare(browser=args.browser)
    elif args.action == "cost":
        result = cost()
    elif args.action == "freeze":
        result = freeze()
    else:
        if args.arm is None or args.submission is None:
            parser.error("score needs --arm and --submission")
        result = score(args.arm, args.submission)
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()

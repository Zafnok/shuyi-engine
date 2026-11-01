"""Permissioned, synthetic pilot for ticket 2912; uses only fixture builds.

The attacker bundles under target/ contain no fixture sources or grader cases.
The repository is not an isolation boundary: give attackers only the bundles.
"""

from __future__ import annotations

import argparse
from difflib import SequenceMatcher
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
import zipfile


ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / "target" / "cloning-study"
GAMES = ("courier", "workshop")
SCRIPTS = {
    "courier": ("down", "right", "up", "right", "right", "right", "right", "right"),
    "workshop": (
        "gather", "gather", "refine", "craft", "deliver", "gather",
        "rest", "gather", "refine", "craft", "deliver",
    ),
}
SENTINELS = (
    b"tools/research/2912/fixture/src", b"held_out_cases", b"grader_seed",
    str(ROOT).encode(), str(ROOT).encode("utf-16-le"),
)


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def command(args: list[str], *, input_text: str | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        cwd=ROOT,
        input=input_text,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=True,
        timeout=120,
    )


def native_path() -> Path:
    suffix = ".exe" if os.name == "nt" else ""
    return ROOT / "target" / "release" / f"cloning-fixture{suffix}"


def wasm_path() -> Path:
    return ROOT / "target" / "wasm32-wasip1" / "release" / "cloning-fixture.wasm"


def build() -> tuple[Path, Path]:
    command(["cargo", "build", "--release", "-p", "cloning-fixture", "--locked"])
    command([
        "cargo", "build", "--release", "--target", "wasm32-wasip1",
        "-p", "cloning-fixture", "--locked",
    ])
    return native_path(), wasm_path()


def invoke(binary: Path, game: str, seed: int, actions: tuple[str, ...]) -> dict:
    if binary.suffix == ".wasm":
        prefix = ["wasmtime", "run", str(binary)]
    elif binary.suffix == ".py":
        prefix = [sys.executable, str(binary)]
    else:
        prefix = [str(binary)]
    raw = command(prefix + [game, str(seed), ",".join(actions)]).stdout
    return json.loads(raw)


def varuint(value: int) -> bytes:
    result = bytearray()
    while value >= 128:
        result.append((value & 127) | 128)
        value >>= 7
    result.append(value)
    return bytes(result)


def wasm_control(data: bytes, seed: int) -> bytes:
    """Append a valid custom section; byte diversity alone has no security value."""
    if not data.startswith(b"\0asm\x01\0\0\0"):
        raise ValueError("not a Wasm module")
    name = b"study_seed"
    payload = varuint(len(name)) + name + seed.to_bytes(8, "little")
    return data + b"\0" + varuint(len(payload)) + payload


def render_gif(frames: list[str], dest: Path) -> None:
    try:
        from PIL import Image, ImageDraw, ImageFont
    except ImportError as error:
        raise SystemExit("footage generation needs Pillow (tested with 12.3.0)") from error
    font = ImageFont.load_default()
    width = max(len(line) for frame in frames for line in frame.splitlines())
    height = max(len(frame.splitlines()) for frame in frames)
    images = []
    for frame in frames:
        image = Image.new("RGB", (max(1, width) * 10 + 24, max(1, height) * 14 + 24), "#13202a")
        pen = ImageDraw.Draw(image)
        for row, line in enumerate(frame.splitlines()):
            pen.text((12, 12 + row * 14), line, font=font, fill="#eff5e8")
        images.append(image)
    images[0].save(dest, save_all=True, append_images=images[1:], duration=650, loop=0)


def audit(path: Path, allowed: set[str]) -> dict[str, str]:
    found = {}
    for item in path.rglob("*"):
        if item.is_symlink():
            raise ValueError(f"symlink in attacker bundle: {item}")
        if not item.is_file():
            continue
        relative = item.relative_to(path).as_posix()
        if relative not in allowed:
            raise ValueError(f"unexpected attacker file: {relative}")
        data = item.read_bytes()
        if any(sentinel in data for sentinel in SENTINELS):
            raise ValueError(f"source/grader marker in attacker file: {relative}")
        found[relative] = sha(data)
    if set(found) != allowed:
        raise ValueError(f"attacker bundle differs from allowlist: {set(found) ^ allowed}")
    return found


def audit_pack(path: Path) -> None:
    expected = {"demo/intro.txt", "future/after-the-gate.ron", "future/closing-note.txt"}
    with zipfile.ZipFile(path) as pack:
        names = pack.namelist()
        if len(names) != len(expected) or set(names) != expected:
            raise ValueError(f"unexpected package entries: {names}")
        for name in names:
            data = pack.read(name)
            if any(sentinel in data for sentinel in SENTINELS):
                raise ValueError(f"source/grader marker inside package: {name}")


def prepare(output: Path) -> dict:
    output = output.resolve()
    root = ROOT.resolve()
    target = (ROOT / "target").resolve()
    if not target.is_relative_to(root) or output != OUT.resolve() or not output.is_relative_to(target):
        raise ValueError("--out must be this repository's dedicated target/cloning-study directory")
    native, wasm = build()
    attacker = output / "attacker"
    evaluator = output / "evaluator"
    if output.exists():
        shutil.rmtree(output)
    for track in ("footage", "extraction", "replacement", "attribution"):
        (attacker / track).mkdir(parents=True)
    evaluator.mkdir()

    # The only footage disclosure is rendered pixels plus a public control list.
    footage = attacker / "footage"
    for game in GAMES:
        trace = invoke(native, game, 11, SCRIPTS[game])["trace"]
        if trace[-1]["outcome"] != "Won":
            raise AssertionError(f"footage does not show a completed loop: {game}")
        render_gif([entry["frame"] for entry in trace], footage / f"{game}.gif")
    (footage / "brief.txt").write_text(
        "Two synthetic games. Courier uses up/down/left/right/wait; "
        "Workshop uses gather/refine/craft/deliver/rest. Make a playable imitation "
        "from the clips only. No oracle or source is supplied. Submit a runnable "
        "program accepting <courier|workshop> <u64-seed> <comma-separated-actions> "
        "and printing JSON with game, seed and trace. Trace starts with the "
        "initial frame; each record needs frame and outcome (Won, Lost or null). "
        "A batch command interface counts as playable for this headless pilot.\n",
        encoding="utf-8",
    )

    # A final native package includes two invented future-content files. They
    # are not referenced by the playable demo. Extraction is byte-for-byte.
    extraction = attacker / "extraction"
    shutil.copy2(native, extraction / native.name)
    future = {
        "future/after-the-gate.ron": (
            f"(episode: 2, route: \"north\", marker: \"{secrets.token_hex(8)}\")\n"
        ).encode(),
        "future/closing-note.txt": (
            f"Synthetic unreleased ending; marker {secrets.token_hex(8)}.\n"
        ).encode(),
        "future/loose-note.txt": (
            f"Synthetic future route: harbor; marker {secrets.token_hex(8)}.\n"
        ).encode(),
        "future/binary-note.txt": (
            f"Synthetic future rule: two gates; marker {secrets.token_hex(8)}.\n"
        ).encode(),
    }
    with zipfile.ZipFile(extraction / "assets.pack", "w", compression=zipfile.ZIP_DEFLATED) as pack:
        pack.writestr("demo/intro.txt", "Courier and Workshop demo.\n")
        for name in ("future/after-the-gate.ron", "future/closing-note.txt"):
            pack.writestr(name, future[name])
    (extraction / "future-loose.txt").write_bytes(future["future/loose-note.txt"])
    with (extraction / native.name).open("ab") as package_binary:
        package_binary.write(b"\nSTUDY_FUTURE_BINARY_BEGIN\n")
        package_binary.write(future["future/binary-note.txt"])
        package_binary.write(b"STUDY_FUTURE_BINARY_END\n")
    if invoke(extraction / native.name, "courier", 11, SCRIPTS["courier"]) != invoke(native, "courier", 11, SCRIPTS["courier"]):
        raise AssertionError("final package binary changed fixture behavior")
    (extraction / "brief.txt").write_text(
        "Inspect this final synthetic demo package. Recover all unreleased "
        "future-content files exactly as bytes. Submit recovered files in a folder.\n",
        encoding="utf-8",
    )
    write_json(evaluator / "future_manifest.json", {name: sha(data) for name, data in future.items()})
    audit_pack(extraction / "assets.pack")

    # The replacement attacker gets the runnable native and Wasm oracle.
    replacement = attacker / "replacement"
    shutil.copy2(native, replacement / native.name)
    shutil.copy2(wasm, replacement / wasm.name)
    (replacement / "brief.txt").write_text(
        "Reproduce both games' line protocol. Run `cloning-fixture <game> <seed> "
        "<comma-separated-actions>` for a JSON trace; `--oracle` accepts one "
        "action per line. Submit a runnable replacement with the batch protocol. "
        "Dynamic queries and analysis tools are allowed.\n",
        encoding="utf-8",
    )

    # A seed changes only a Wasm custom section. Both controls must replay
    # identically, including all full transition records.
    source_wasm = wasm.read_bytes()
    a = evaluator / "layout-1.wasm"
    b = evaluator / "layout-2.wasm"
    a.write_bytes(wasm_control(source_wasm, 1))
    b.write_bytes(wasm_control(source_wasm, 2))
    control = {"sha_1": sha(a.read_bytes()), "sha_2": sha(b.read_bytes()), "equal": True}
    if control["sha_1"] == control["sha_2"]:
        raise AssertionError("Wasm seed did not diversify bytes")
    for game in GAMES:
        expected = invoke(native, game, 29, SCRIPTS[game])
        for variant in (wasm, a, b):
            if invoke(variant, game, 29, SCRIPTS[game]) != expected:
                raise AssertionError(f"Wasm replay mismatch: {game} {variant}")
    write_json(evaluator / "wasm_control.json", control)

    # The reveal receipt is given to an attribution participant, who chooses
    # among four fictional store cards. No real store identity is asserted.
    attribution = attacker / "attribution"
    package_hash = sha((extraction / "assets.pack").read_bytes())
    ids = ("north", "harbor", "orchard", "lantern")
    official = ids[secrets.randbelow(len(ids))]
    write_json(attribution / "reveal.json", {
        "title": "Courier & Workshop", "assets_pack_sha256": package_hash,
        "note": "A release receipt, not proof of authorship or store enforcement.",
    })
    for index, card in enumerate(ids):
        digest = package_hash if card == official else sha(f"decoy-{index}-{package_hash}".encode())
        write_json(attribution / f"{card}.json", {
            "id": card, "url": f"https://store.example.invalid/{card}/courier-workshop",
            "title": "Courier & Workshop", "assets_pack_sha256": digest,
        })
    (attribution / "brief.txt").write_text(
        "Identify the canonical fictional listing using the reveal receipt. "
        "Submit JSON with official_id and evidence_sha256.\n", encoding="utf-8"
    )
    write_json(evaluator / "attribution_answer.json", {"official_id": official, "evidence_sha256": package_hash})

    grader_seed = secrets.token_hex(16)
    grader_commitment = sha(bytes.fromhex(grader_seed))
    write_json(evaluator / "grader_commitment.json", {
        "sha256_of_seed": grader_commitment, "case_count_per_game": 21,
        "seed_disclosed_after_submission": True,
    })

    allowed = {
        "footage": {"courier.gif", "workshop.gif", "brief.txt"},
        "extraction": {native.name, "assets.pack", "future-loose.txt", "brief.txt"},
        "replacement": {native.name, wasm.name, "brief.txt"},
        "attribution": {"brief.txt", "reveal.json", *(f"{card}.json" for card in ids)},
    }
    manifests = {track: audit(attacker / track, names) for track, names in allowed.items()}
    write_json(evaluator / "artifact_audit.json", manifests)
    return {
        "output": str(output), "audited_files": {k: len(v) for k, v in manifests.items()},
        "wasm": control, "private_grader_seed": grader_seed,
        "grader_commitment": grader_commitment,
    }


def score_extraction(output: Path, submission: Path) -> dict:
    expected = json.loads((output / "evaluator" / "future_manifest.json").read_text(encoding="utf-8"))
    found = {sha(path.read_bytes()) for path in submission.rglob("*") if path.is_file()}
    locations = {"pack": [0, 0], "loose": [0, 0], "binary_overlay": [0, 0]}
    for name, digest in expected.items():
        location = "loose" if "loose" in name else "binary_overlay" if "binary" in name else "pack"
        locations[location][1] += 1
        locations[location][0] += int(digest in found)
    return {
        "matched": sum(pair[0] for pair in locations.values()),
        "denominator": len(expected),
        "by_location": locations,
        "path_attribution_checked": False,
    }


def score_attribution(output: Path, submission: Path) -> dict:
    expected = json.loads((output / "evaluator" / "attribution_answer.json").read_text(encoding="utf-8"))
    answer = json.loads(submission.read_text(encoding="utf-8"))
    return {
        "official_correct": answer.get("official_id") == expected["official_id"],
        "evidence_correct": answer.get("evidence_sha256") == expected["evidence_sha256"],
        "denominator": 1,
    }


def hidden_cases(seed: str) -> list[tuple[str, int, tuple[str, ...], bool]]:
    # Commit to the seed before the attack; materialize cases after submission.
    rng = random.Random(int(seed, 16))
    cases = []
    for game in GAMES:
        choices = tuple(dict.fromkeys(SCRIPTS[game])) + ("invalid",)
        for _ in range(20):
            cases.append((game, rng.getrandbits(32), tuple(rng.choice(choices) for _ in range(8)), False))
        win_seed = rng.getrandbits(32)
        if game == "courier":
            initial = invoke(native_path(), game, win_seed, ())["trace"][0]["state"]
            vertical = ("up", "down") if initial["parcel"][1] == 1 else ("down", "up")
            win = (vertical[0], "right", vertical[1], *("right" for _ in range(5)))
        else:
            win = SCRIPTS[game]
        cases.append((game, win_seed, (*win, "invalid", "rest"), True))
    return cases


def score_replacement(output: Path, executable: Path, grader_seed: str) -> dict:
    native = native_path()
    commitment = json.loads((output / "evaluator" / "grader_commitment.json").read_text(encoding="utf-8"))
    if sha(bytes.fromhex(grader_seed)) != commitment["sha256_of_seed"]:
        raise ValueError("grader seed does not match the pre-attempt commitment")
    cases = hidden_cases(grader_seed)
    write_json(output / "evaluator" / "held_out_cases.json", cases)
    result = {game: {"matched_transitions": 0, "transition_denominator": 0,
                     "exact_traces": 0, "trace_denominator": 0,
                     "mandatory_path_exact": False, "initial_state_matches": 0,
                     "runtime_failures": [], "reference_parity": 0} for game in GAMES}
    for game, seed, actions, mandatory in cases:
        expected = invoke(native, game, seed, actions)
        row = result[game]
        row["trace_denominator"] += 1
        row["transition_denominator"] += len(expected["trace"]) - 1
        row["reference_parity"] += int(invoke(wasm_path(), game, seed, actions) == expected)
        try:
            actual = invoke(executable, game, seed, actions)
            if not isinstance(actual, dict) or not isinstance(actual.get("trace"), list):
                raise ValueError("submission did not return a JSON trace object")
        except (subprocess.SubprocessError, OSError, ValueError) as error:
            row["runtime_failures"].append(f"{game}/{seed}: {type(error).__name__}")
            continue
        left, right = expected["trace"], actual["trace"]
        row["initial_state_matches"] += int(bool(right) and left[0] == right[0])
        matches = sum(a == b for a, b in zip(left[1:], right[1:]))
        row["matched_transitions"] += matches
        exact = len(left) == len(right) and left == right
        row["exact_traces"] += int(exact)
        if mandatory:
            row["mandatory_path_exact"] = exact and left[-3]["outcome"] == "Won"
    for row in result.values():
        row["potential_full_success"] = (
            row["mandatory_path_exact"] and
            row["matched_transitions"] * 10 >= row["transition_denominator"] * 9 and
            row["reference_parity"] == row["trace_denominator"]
        )
    original_bytes = native.read_bytes()
    result["identity_check"] = {
        "submission_equals_original_binary": sha(executable.read_bytes()) == sha(original_bytes),
        "independence_requires_manual_audit": True,
    }
    return result


def score_footage(output: Path, executable: Path, grader_seed: str) -> dict:
    commitment = json.loads((output / "evaluator" / "grader_commitment.json").read_text(encoding="utf-8"))
    if sha(bytes.fromhex(grader_seed)) != commitment["sha256_of_seed"]:
        raise ValueError("grader seed does not match the pre-attempt commitment")
    result = {}
    for game in GAMES:
        original = invoke(native_path(), game, 11, SCRIPTS[game])
        try:
            shown = invoke(executable, game, 11, SCRIPTS[game])
            frames = [str(record.get("frame", "")) for record in shown["trace"]]
            reference = [record["frame"] for record in original["trace"]]
            similarity = sum(SequenceMatcher(None, a, b).ratio() for a, b in zip(frames, reference)) / len(reference)
        except (subprocess.SubprocessError, OSError, ValueError, KeyError, TypeError, AttributeError):
            frames = []
            similarity = 0.0
        row = {
            "sample_frame_similarity": round(similarity, 3),
            "sample_frames_returned": len(frames),
            "sample_frame_denominator": len(original["trace"]),
            "held_out_runs_started": 0,
            "held_out_run_denominator": 21,
            "held_out_outcome_matches": 0,
            "held_out_outcome_denominator": 0,
            "held_out_frame_matches": 0,
            "held_out_frame_denominator": 0,
            "mandatory_win_observed": False,
            "human_quality_rubric": "not measured in one-agent pilot",
        }
        for case_game, seed, actions, mandatory in hidden_cases(grader_seed):
            if case_game != game:
                continue
            expected = invoke(native_path(), game, seed, actions)["trace"]
            row["held_out_outcome_denominator"] += len(expected) - 1
            row["held_out_frame_denominator"] += len(expected) - 1
            try:
                actual = invoke(executable, game, seed, actions)["trace"]
                if not isinstance(actual, list):
                    raise ValueError("trace is not a list")
                row["held_out_runs_started"] += 1
                row["held_out_outcome_matches"] += sum(
                    left["outcome"] == right.get("outcome")
                    for left, right in zip(expected[1:], actual[1:]) if isinstance(right, dict)
                )
                row["held_out_frame_matches"] += sum(
                    left["frame"] == right.get("frame")
                    for left, right in zip(expected[1:], actual[1:]) if isinstance(right, dict)
                )
                if mandatory:
                    row["mandatory_win_observed"] = len(actual) >= 3 and actual[-3].get("outcome") == "Won"
            except (subprocess.SubprocessError, OSError, ValueError, KeyError, TypeError, AttributeError):
                pass
        result[game] = row
    return result


def controls(output: Path) -> dict:
    native = native_path()
    wasm = wasm_path()
    result = {}
    variants = (native, wasm, output / "evaluator" / "layout-1.wasm", output / "evaluator" / "layout-2.wasm")
    for binary in variants:
        for game in GAMES:
            times = []
            for _ in range(20):
                start = time.perf_counter()
                invoke(binary, game, 29, SCRIPTS[game])
                times.append((time.perf_counter() - start) * 1000)
            result[f"{binary.name}:{game}"] = {
                "median_ms_per_process_and_replay": round(statistics.median(times), 3),
                "max_ms_per_process_and_replay": round(max(times), 3),
                "samples": len(times), "bytes": binary.stat().st_size,
                "excludes_interactive_frame_latency": True,
            }
    write_json(output / "evaluator" / "player_cost_controls.json", result)
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("prepare", "controls", "score-extraction", "score-attribution", "score-replacement", "score-footage"))
    parser.add_argument("submission", nargs="?", type=Path)
    parser.add_argument("--out", type=Path, default=OUT)
    parser.add_argument("--grader-seed", help="private seed printed by prepare; required for replacement grading")
    args = parser.parse_args()
    if args.action == "prepare":
        answer = prepare(args.out)
    elif args.action == "controls":
        answer = controls(args.out)
    elif args.submission is None:
        parser.error("submission path required")
    elif args.action == "score-extraction":
        answer = score_extraction(args.out, args.submission)
    elif args.action == "score-attribution":
        answer = score_attribution(args.out, args.submission)
    elif args.action in ("score-replacement", "score-footage"):
        if not args.grader_seed:
            parser.error(f"{args.action} requires --grader-seed")
        answer = (score_replacement if args.action == "score-replacement" else score_footage)(
            args.out, args.submission, args.grader_seed
        )
    else:
        raise AssertionError("unknown study action")
    print(json.dumps(answer, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()

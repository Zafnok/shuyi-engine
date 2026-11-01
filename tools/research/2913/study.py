"""Blind clip experiment for ticket 2913, on the 2912 fixtures only.

Attackers run with no tools at all and receive only frames and a brief, so
they cannot reach the fixture source, this runner or the hidden cases. Their
programs run in a container with no network and are scored by the 2912 grader.
"""

from __future__ import annotations

import argparse
import base64
from concurrent.futures import ThreadPoolExecutor
import csv
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import random
import re
import secrets
import shutil
import statistics
import subprocess
import sys
import tempfile
import threading
import time

import numpy as np
from PIL import Image, ImageFilter

import perturb
from perturb import CANDIDATES, FPS, HOLD, MID


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
OUT = ROOT / "target" / "clip-study"
RESULTS = HERE / "results"
SANDBOX_IMAGE = "python:3.14-slim"
TRANSFORMS = ("direct", "transcode", "crop", "frames", "recapture", "average")
MODELS = ("claude-haiku-5-5", "claude-sonnet-5-5", "claude-opus-5-5")
SHOWN_SEED = 11
SIMILARITY_FLOOR = 0.90
ATTEMPT_BUDGET_USD = "4"
ATTEMPT_TIMEOUT_S = 1200
SYSTEM_PROMPT = "You are an experienced game programmer. Answer in text; you have no tools."
BRIEF = (
    "Make a playable imitation of two small games, from what you are given only. "
    "No oracle or source exists for you. Courier uses up/down/left/right/wait; "
    "Workshop uses gather/refine/craft/deliver/rest. Reproduce what the screen "
    "shows as text, character for character, and infer the rules as well as you can.\n\n"
    "Deliver one Python 3 standard-library program in a single ```python fenced block. "
    "`prog.py <courier|workshop> <u64-seed> <comma-separated-actions>` prints one JSON "
    "object with game, seed and trace. Trace starts with the initial frame, then one "
    "record per action; each record has frame (the screen as text, lines joined by \\n) "
    "and outcome (\"Won\", \"Lost\" or null). `prog.py <game> <seed> --oracle` reads one "
    "action per line and prints one such record per line, so a person can play. An "
    "action the game refuses or does not know leaves the state unchanged."
)
DESCRIBER_PROMPT = (
    "Watch these frames from gameplay clips of two small games, as a player would. "
    "Write what you would tell a friend who is going to rebuild them and cannot see "
    "the clips: the goal, the controls, the screen's layout and wording, and each rule "
    "you can work out. Plain language, at most 400 words, no code."
)


def load_grader():
    spec = importlib.util.spec_from_file_location("cloning_study", ROOT / "tools/research/2912/study.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules["cloning_study"] = module
    spec.loader.exec_module(module)
    return module


GRADER = load_grader()
GAMES = GRADER.GAMES
SCRIPTS = GRADER.SCRIPTS


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="\n") as file:
        file.write(json.dumps(value, indent=1, sort_keys=True) + "\n")


def ffmpeg() -> str:
    import imageio_ffmpeg

    return imageio_ffmpeg.get_ffmpeg_exe()


def run(args: list[str], **kwargs) -> subprocess.CompletedProcess:
    return subprocess.run(args, check=True, capture_output=True, cwd=kwargs.pop("cwd", ROOT), **kwargs)


def read_frames(folder: Path) -> list[np.ndarray]:
    return [np.asarray(Image.open(path).convert("RGB")) for path in sorted(folder.glob("*.png"))]


def capture_clean() -> None:
    run(["cargo", "build", "--release", "-p", "cloning-fixture", "-p", "clip-capture", "--locked"])
    binary = ROOT / "target" / "release" / ("clip-capture" + (".exe" if os.name == "nt" else ""))
    for game in GAMES:
        run([str(binary), game, str(SHOWN_SEED), ",".join(SCRIPTS[game]), str(OUT / "clean" / game)])


def surrogate_deltas() -> None:
    """PGD at +/-8 of 255 away from each clean frame's CLIP ViT-B/32 embedding."""
    import open_clip
    import torch
    import torch.nn.functional as functional

    model, _, _ = open_clip.create_model_and_transforms("ViT-B-32", pretrained="openai")
    model.eval().requires_grad_(False)
    mean = torch.tensor((0.48145466, 0.4578275, 0.40821073)).view(1, 3, 1, 1)
    std = torch.tensor((0.26862954, 0.26130258, 0.27577711)).view(1, 3, 1, 1)
    embed = lambda x: functional.normalize(model.encode_image(
        (functional.interpolate(x, (224, 224), mode="bilinear", antialias=True) - mean) / std), dim=-1)
    epsilon, step = 8 / 255, 1 / 255
    torch.manual_seed(2913)
    report = {}
    for game in GAMES:
        for index, frame in enumerate(read_frames(OUT / "clean" / game)):
            x = torch.from_numpy(frame.copy()).permute(2, 0, 1)[None].float() / 255
            target = embed(x).detach()
            delta = torch.zeros_like(x).uniform_(-epsilon, epsilon)
            for _ in range(40):
                delta.requires_grad_(True)
                loss = -functional.cosine_similarity(embed((x + delta).clamp(0, 1)), target).mean()
                loss.backward()
                delta = (delta.detach() + step * delta.grad.sign()).clamp(-epsilon, epsilon)
            cosine = float(functional.cosine_similarity(embed((x + delta).clamp(0, 1)), target))
            report[f"{game}/{index:03}"] = round(cosine, 4)
            path = OUT / "surrogate" / game / f"{index:03}.npy"
            path.parent.mkdir(parents=True, exist_ok=True)
            np.save(path, ((x + delta).clamp(0, 1)[0].permute(1, 2, 0).numpy() * 255).round().astype(np.uint8))
    write_json(OUT / "surrogate" / "cosine_to_clean.json", report)


def subframe(candidate: str, game: str, state: int, sub: int, clean: np.ndarray) -> np.ndarray:
    if candidate == "ordinary":
        return clean
    if candidate == "surrogate":
        return np.load(OUT / "surrogate" / game / f"{state:03}.npy")
    return perturb.HAND_MADE[candidate](clean, game, state, sub)


def video_frames(candidate: str, game: str, clean: list[np.ndarray]):
    for state, frame in enumerate(clean):
        for sub in range(HOLD):
            yield subframe(candidate, game, state, sub, frame)


def encode(frames, width: int, height: int, dest: Path, crf: int, vf: str | None = None) -> None:
    """A deterministic single-thread H.264 encode of raw frames."""
    dest.parent.mkdir(parents=True, exist_ok=True)
    args = [ffmpeg(), "-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24",
            "-s", f"{width}x{height}", "-r", str(FPS), "-i", "-", "-threads", "1",
            "-c:v", "libx264", "-preset", "veryfast", "-crf", str(crf), "-pix_fmt", "yuv420p",
            "-bitexact", "-flags", "+bitexact", "-map_metadata", "-1", "-an"]
    if vf:
        args += ["-vf", vf]
    process = subprocess.Popen(args + [str(dest)], stdin=subprocess.PIPE, stderr=subprocess.PIPE)
    for frame in frames:
        process.stdin.write(frame.tobytes())
    process.stdin.close()
    error = process.stderr.read()
    if process.wait() != 0:
        raise RuntimeError(error.decode(errors="replace"))


def decode_all(video: Path):
    probe = subprocess.run([ffmpeg(), "-i", str(video)], capture_output=True, text=True, check=False).stderr
    width, height = map(int, re.search(r"(\d{3,4})x(\d{3,4})", probe).groups())
    process = subprocess.Popen(
        [ffmpeg(), "-loglevel", "error", "-i", str(video), "-f", "rawvideo", "-pix_fmt", "rgb24", "-"],
        stdout=subprocess.PIPE,
    )
    size = width * height * 3
    while chunk := process.stdout.read(size):
        yield np.frombuffer(chunk, dtype=np.uint8).reshape(height, width, 3)
    process.wait()


def sample_holds(frames, dest: Path, *, suffix: str = "png", resize: int | None = None) -> None:
    """Save the middle subframe of each hold: the sampler an attacker would use."""
    dest.mkdir(parents=True, exist_ok=True)
    for index, frame in enumerate(frames):
        if index % HOLD != MID:
            continue
        picture = Image.fromarray(frame)
        if resize:
            picture = picture.resize((resize, resize * frame.shape[0] // frame.shape[1]), Image.BILINEAR)
        name = dest / f"{index // HOLD:03}.{suffix}"
        picture.save(name, **({"quality": 70} if suffix == "jpg" else {}))


def recapture_frames(frames, game: str):
    """Filmed-screen simulation; NOT a phone recording, and labelled so in the report."""
    rng = perturb.rng_for("recapture", game)
    moire = None
    for frame in frames:
        height, width = frame.shape[:2]
        picture = Image.fromarray(frame).transform(
            (width, height), Image.PERSPECTIVE,
            (1.02, 0.015, -6, -0.012, 1.03, 3, 0.00002, 0.00001), Image.BILINEAR,
        ).filter(ImageFilter.GaussianBlur(0.9))
        values = np.asarray(picture).astype(np.float32) / 255
        if moire is None:
            ys, xs = np.mgrid[0:height, 0:width]
            moire = 1 + 0.035 * np.sin(xs * 0.73 + ys * 0.41)[..., None]
        values = (values ** 0.9) * moire + rng.normal(0, 0.012, values.shape)
        yield (np.clip(values, 0, 1) * 255).astype(np.uint8)


def encode_arm(candidate: str, transform: str) -> None:
    """Writes one arm's encodes and sampled frames for both games."""
    for game in GAMES:
        clean = read_frames(OUT / "clean" / game)
        height, width = clean[0].shape[:2]
        arm = OUT / "arms" / candidate / transform / game
        if arm.exists():
            shutil.rmtree(arm)
        master = OUT / "arms" / candidate / "transcode" / game / "clip.mp4"
        if transform == "direct":
            sample_holds(video_frames(candidate, game, clean), arm)
        elif transform == "average":
            arm.mkdir(parents=True)
            for state, frame in enumerate(clean):
                mean = np.mean([subframe(candidate, game, state, sub, frame) for sub in range(HOLD)], axis=0)
                Image.fromarray(mean.round().astype(np.uint8)).save(arm / f"{state:03}.png")
        elif transform == "transcode":
            encode(video_frames(candidate, game, clean), width, height, arm / "clip.mp4", 30, "scale=854:-2")
            sample_holds(decode_all(arm / "clip.mp4"), arm)
        elif transform == "crop":
            size = next(decode_all(master)).shape
            encode(decode_all(master), size[1], size[0], arm / "clip.mp4", 30, "crop=trunc(iw*0.88/2)*2:trunc(ih*0.88/2)*2")
            sample_holds(decode_all(arm / "clip.mp4"), arm)
        elif transform == "frames":
            sample_holds(decode_all(master), arm, suffix="jpg", resize=640)
        elif transform in ("decode", "decode_transcode"):
            # What a script recovers from motion-defined form, hold by hold.
            arm.mkdir(parents=True)
            frames = list(video_frames(candidate, game, clean)) if transform == "decode" else list(decode_all(master))
            grain = perturb.MOTION_GRAIN * frames[0].shape[1] / width
            for state in range(len(clean)):
                decoded = perturb.motion_decode(frames[state * HOLD:(state + 1) * HOLD], grain)
                Image.fromarray(decoded).save(arm / f"{state:03}.png")
        elif transform == "recapture":
            size = next(decode_all(master)).shape
            encode(recapture_frames(decode_all(master), game), size[1], size[0], arm / "clip.mp4", 26)
            sample_holds(decode_all(arm / "clip.mp4"), arm)
        else:
            raise ValueError(transform)


def arm_frames(candidate: str, transform: str, game: str) -> list[Path]:
    folder = OUT / "arms" / candidate / transform / game
    return sorted(path for path in folder.iterdir() if path.suffix in (".png", ".jpg"))


def encodes_manifest() -> dict:
    """The hash of each evaluated encode and of each frame set."""
    manifest = {}
    for path in sorted((OUT / "arms").rglob("clip.mp4")):
        manifest[path.relative_to(OUT / "arms").as_posix()] = {"sha256": sha(path.read_bytes()), "bytes": path.stat().st_size}
    for folder in sorted({path.parent for path in (OUT / "arms").rglob("*") if path.suffix in (".png", ".jpg")}):
        joined = "".join(sha(path.read_bytes()) for path in sorted(folder.iterdir()) if path.suffix in (".png", ".jpg"))
        manifest[folder.relative_to(OUT / "arms").as_posix() + "/frames"] = {"sha256": sha(joined.encode())}
    return manifest


def viewer_metrics() -> dict:
    result = {}
    for candidate in CANDIDATES:
        row = {"psnr": [], "ssim": [], "energy_kept": [], "correlation": [], "ms_per_frame": []}
        flash = 0
        fails = False
        for game in GAMES:
            clean = read_frames(OUT / "clean" / game)
            for state, frame in enumerate(clean):
                start = time.perf_counter()
                shown = subframe(candidate, game, state, MID, frame)
                row["ms_per_frame"].append((time.perf_counter() - start) * 1000)
                row["psnr"].append(perturb.psnr(frame, shown))
                row["ssim"].append(perturb.ssim(frame, shown))
                control = np.asarray(Image.open(OUT / "arms/ordinary/transcode" / game / f"{state:03}.png").convert("RGB"))
                after = np.asarray(Image.open(OUT / "arms" / candidate / "transcode" / game / f"{state:03}.png").convert("RGB"))
                kept = perturb.survival(frame, shown, control, after)
                row["energy_kept"].append(kept["energy_kept"])
                row["correlation"].append(kept["correlation"])
            hold = [subframe(candidate, game, 1, sub, clean[1]) for sub in range(HOLD)]
            screen = perturb.flash_area(hold)
            flash = max(flash, screen["pixels_over_three_flashes_per_second"])
            fails = fails or screen["fails_general_flash_screen"]
        size = sum((OUT / "arms" / candidate / "transcode" / game / "clip.mp4").stat().st_size for game in GAMES)
        result[candidate] = {
            **{key: round(statistics.mean(values), 3) for key, values in row.items()},
            "transcode_bytes_both_clips": size, "flash_pixels": flash, "fails_general_flash_screen": fails,
        }
    return result


def commit_seed() -> str:
    path = OUT / "evaluator" / "grader_seed.txt"
    if path.exists():
        raise SystemExit("the seed is already committed")
    seed = secrets.token_hex(16)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(seed, encoding="utf-8")
    commitment = sha(bytes.fromhex(seed))
    write_json(OUT / "evaluator" / "grader_commitment.json", {"sha256_of_seed": commitment, "case_count_per_game": 21})
    return commitment


def image_block(path: Path) -> dict:
    media = "image/jpeg" if path.suffix == ".jpg" else "image/png"
    return {"type": "image", "source": {"type": "base64", "media_type": media, "data": base64.b64encode(path.read_bytes()).decode()}}


def clip_blocks(candidate: str, transform: str) -> tuple[list[dict], str]:
    blocks, hashes = [], []
    for number, game in enumerate(GAMES, 1):
        blocks.append({"type": "text", "text": f"Clip {number}: {game}. One frame per game state, in order."})
        for path in arm_frames(candidate, transform, game):
            blocks.append(image_block(path))
            hashes.append(sha(path.read_bytes()))
    return blocks, sha("".join(hashes).encode())


def call_model(model: str, content: list[dict], folder: Path) -> dict:
    """One tool-free, settings-free model turn in an empty folder."""
    message = json.dumps({"type": "user", "message": {"role": "user", "content": content}})
    args = ["claude", "-p", "--model", model, "--tools", "", "--strict-mcp-config",
            "--setting-sources", "", "--no-session-persistence", "--disable-slash-commands",
            "--system-prompt", SYSTEM_PROMPT, "--input-format", "stream-json",
            "--output-format", "stream-json", "--verbose", "--max-budget-usd", ATTEMPT_BUDGET_USD]
    with tempfile.TemporaryDirectory(prefix="clip-attacker-", dir=tempfile.gettempdir()) as empty:
        executable = shutil.which("claude") or "claude"
        start = time.time()
        process = subprocess.run([executable, *args[1:]], input=message, text=True, encoding="utf-8",
                                 capture_output=True, cwd=empty, timeout=ATTEMPT_TIMEOUT_S, check=False)
    (folder / "stream.jsonl").write_text(process.stdout, encoding="utf-8")
    events = []
    for line in process.stdout.splitlines():
        try:
            events.append(json.loads(line))
        except ValueError:
            continue
    init = next((event for event in events if event.get("subtype") == "init"), {})
    final = next((event for event in reversed(events) if event.get("type") == "result"), {})
    tool_uses = sum(
        block.get("type") == "tool_use"
        for event in events if event.get("type") == "assistant"
        for block in event.get("message", {}).get("content", [])
    )
    usage = final.get("usage", {})
    return {
        "seconds": round(time.time() - start, 1), "exit_code": process.returncode,
        "tools_offered": init.get("tools"), "mcp_servers": init.get("mcp_servers"), "tool_uses": tool_uses,
        "cost_usd": final.get("total_cost_usd"), "is_error": final.get("is_error"),
        "api_error_status": final.get("api_error_status"), "turns": final.get("num_turns"),
        "models_used": sorted(final.get("modelUsage", {})),
        "input_tokens": usage.get("input_tokens"), "output_tokens": usage.get("output_tokens"),
        "text": final.get("result") or "", "stderr": process.stderr[-500:],
    }


def extract_program(text: str) -> str | None:
    blocks = re.findall(r"```(?:python|py)?\s*\n(.*?)```", text, re.DOTALL)
    return max(blocks, key=len) if blocks else None


def strip_call(call: dict) -> dict:
    return {key: value for key, value in call.items() if key != "text"}


def attempt(job: dict) -> dict:
    folder = OUT / "attempts" / job["id"]
    folder.mkdir(parents=True, exist_ok=True)
    record = {**job, "started": time.strftime("%Y-%m-%dT%H:%M:%S%z")}
    blocks, bundle = clip_blocks(job["candidate"], job["transform"])
    record["bundle_sha256"] = bundle
    if job["kind"] == "description":
        described = call_model(job["model"], [*blocks, {"type": "text", "text": DESCRIBER_PROMPT}], folder)
        (folder / "description.txt").write_text(described["text"], encoding="utf-8")
        record["describer"] = strip_call(described)
        content = [{"type": "text", "text": f"{BRIEF}\n\nA person who watched the clips told you:\n\n{described['text']}"}]
    else:
        content = [*blocks, {"type": "text", "text": BRIEF}]
    try:
        call = call_model(job["model"], content, folder)
    except subprocess.TimeoutExpired:
        record["class"] = "timeout"
        return record
    (folder / "response.txt").write_text(call["text"], encoding="utf-8")
    record["call"] = strip_call(call)
    program = extract_program(call["text"])
    if call["is_error"] or call["exit_code"] != 0:
        record["class"] = "setup_failure"
    elif program is None:
        record["class"] = "no_program"
    else:
        (folder / "imitation.py").write_text(program, encoding="utf-8", newline="\n")
        record["class"] = "submitted"
        record["program_bytes"] = len(program.encode())
    return record


def alive() -> list[str]:
    """Candidates no valid model has passed the floor against yet."""
    path = OUT / "ledger.json"
    ledger = json.loads(path.read_text(encoding="utf-8")) if path.exists() else []
    dead = {row["candidate"] for row in ledger if row["class"] == "over_floor"}
    return [candidate for candidate in CANDIDATES[1:] if candidate not in dead]


def plan(stage: str, model: str, everything: bool = False) -> list[dict]:
    """One stage's attempts. One bypass fails a candidate, so by default a
    later model attacks only what the earlier ones left standing; `everything`
    attacks every candidate anyway, for the transfer data."""
    jobs = []
    standing = list(CANDIDATES[1:]) if everything else alive()
    def add(kind: str, candidate: str, transform: str, chosen: str, rep: int) -> None:
        jobs.append({
            "id": f"{stage}-{kind}-{candidate}-{transform}-{chosen.removeprefix('claude-')}-{rep}",
            "stage": stage, "kind": kind, "candidate": candidate, "transform": transform,
            "model": chosen, "rep": rep, "brief_sha256": sha(BRIEF.encode()),
        })
    if stage == "smoke":
        add("imitate", "ordinary", "direct", MODELS[0], 0)
    elif stage == "control":
        for chosen in MODELS:
            for rep in range(3 if chosen == MODELS[0] else 1):
                add("imitate", "ordinary", "direct", chosen, rep)
            add("imitate", "ordinary", "transcode", chosen, 0)
    elif stage == "transforms":
        for transform in ("crop", "frames", "recapture"):
            add("imitate", "ordinary", transform, model, 0)
    elif stage == "candidates":
        for candidate in standing:
            for transform in ("direct", "transcode"):
                add("imitate", candidate, transform, model, 0)
    elif stage == "bypass":
        for candidate in standing:
            for transform in ("crop", "frames", "recapture", "average"):
                add("imitate", candidate, transform, model, 0)
    elif stage == "motion":
        for transform in ("direct", "transcode", "average", "decode", "decode_transcode"):
            add("imitate", "motion", transform, model, 0)
    elif stage == "description":
        for candidate in ("ordinary", *standing):
            add("description", candidate, "direct", model, 0)
    else:
        raise SystemExit(f"unknown stage: {stage}")
    random.Random(f"2913-{stage}").shuffle(jobs)
    return jobs


LEDGER_LOCK = threading.Lock()


def run_stage(stage: str, workers: int, model: str, everything: bool) -> None:
    ledger_path = OUT / "ledger.json"
    ledger = json.loads(ledger_path.read_text(encoding="utf-8")) if ledger_path.exists() else []
    done = {row["id"] for row in ledger}
    jobs = [job for job in plan(stage, model, everything) if job["id"] not in done]

    def work(job: dict) -> None:
        record = attempt(job)
        if record["class"] == "setup_failure":
            # A tool failure is not a defense win: retry once and keep both.
            retry = attempt({**job, "id": job["id"] + "-retry", "retry_of": job["id"]})
            records = [record, retry]
        else:
            records = [record]
        with LEDGER_LOCK:
            ledger.extend(records)
            write_json(ledger_path, ledger)
        print(job["id"], [row["class"] for row in records], flush=True)

    with ThreadPoolExecutor(workers) as pool:
        list(pool.map(work, jobs))


class Sandbox:
    """A container with no network holding one submission, read-only."""

    def __init__(self, program: Path) -> None:
        self.program = program
        self.name = f"clip-score-{secrets.token_hex(6)}"

    def __enter__(self) -> Sandbox:
        run(["docker", "run", "-d", "--rm", "--name", self.name, "--network", "none", "--read-only",
             "--cap-drop", "ALL", "--pids-limit", "64", "--memory", "512m", "--cpus", "1",
             "-v", f"{self.program.parent}:/sub:ro", SANDBOX_IMAGE, "sleep", "3600"])
        return self

    def __exit__(self, *_: object) -> None:
        subprocess.run(["docker", "rm", "-f", self.name], capture_output=True, check=False)

    def invoke(self, game: str, seed: int, actions: tuple[str, ...]) -> dict:
        raw = subprocess.run(
            ["docker", "exec", self.name, "python", f"/sub/{self.program.name}", game, str(seed), ",".join(actions)],
            capture_output=True, text=True, encoding="utf-8", timeout=20, check=True,
        ).stdout
        return json.loads(raw)


def score_program(program: Path, seed: str) -> dict:
    """The 2912 footage grader, with the submission confined, plus the floor."""
    with Sandbox(program) as sandbox:
        original = GRADER.invoke

        def invoke(binary: Path, game: str, case_seed: int, actions: tuple[str, ...]) -> dict:
            return sandbox.invoke(game, case_seed, actions) if binary == program else original(binary, game, case_seed, actions)

        GRADER.invoke = invoke
        try:
            scored = GRADER.score_footage(OUT, program, seed)
            for game in GAMES:
                row = scored[game]
                try:
                    shown = invoke(program, game, SHOWN_SEED, SCRIPTS[game])["trace"]
                    reference = original(GRADER.native_path(), game, SHOWN_SEED, SCRIPTS[game])["trace"]
                    row["shown_frames_exact"] = sum(a["frame"] == b.get("frame") for a, b in zip(reference, shown))
                    row["shown_loop_won"] = len(shown) == len(reference) and shown[-1].get("outcome") == "Won"
                except (subprocess.SubprocessError, OSError, ValueError, KeyError, TypeError, AttributeError):
                    row["shown_frames_exact"] = 0
                    row["shown_loop_won"] = False
                row["over_floor"] = bool(row["shown_loop_won"] and row["sample_frame_similarity"] >= SIMILARITY_FLOOR)
        finally:
            GRADER.invoke = original
    scored["over_floor_both_games"] = all(scored[game]["over_floor"] for game in GAMES)
    return scored


def score_all() -> None:
    seed = (OUT / "evaluator" / "grader_seed.txt").read_text(encoding="utf-8").strip()
    path = OUT / "ledger.json"
    ledger = json.loads(path.read_text(encoding="utf-8"))
    for row in ledger:
        if row["class"] == "submitted" and "score" not in row:
            program = OUT / "attempts" / row["id"] / "imitation.py"
            row["score"] = score_program(program, seed)
            row["class"] = "over_floor" if row["score"]["over_floor_both_games"] else "below_floor"
            write_json(path, ledger)
            print(row["id"], row["class"], flush=True)


def summary() -> dict:
    ledger = json.loads((OUT / "ledger.json").read_text(encoding="utf-8"))
    cells: dict[str, dict] = {}
    ledger = [row for row in ledger if row["stage"] != "smoke"]
    for row in ledger:
        key = "/".join((row["kind"], row["candidate"], row["transform"], row["model"].removeprefix("claude-")))
        cell = cells.setdefault(key, {
            "assigned": 0, "over_floor": 0, "below_floor": 0, "other": 0, "cost_usd": 0.0,
            "hidden_frames": [0, 0], "shown_frames": [0, 0], "similarity": [],
        })
        cell["assigned"] += 1
        cell["cost_usd"] += (row.get("call", {}).get("cost_usd") or 0) + (row.get("describer", {}).get("cost_usd") or 0)
        cell[row["class"] if row["class"] in ("over_floor", "below_floor") else "other"] += 1
        for game in GAMES if "score" in row else ():
            scored = row["score"][game]
            cell["hidden_frames"][0] += scored["held_out_frame_matches"]
            cell["hidden_frames"][1] += scored["held_out_frame_denominator"]
            cell["shown_frames"][0] += scored["shown_frames_exact"]
            cell["shown_frames"][1] += scored["sample_frame_denominator"]
            cell["similarity"].append(scored["sample_frame_similarity"])
    for cell in cells.values():
        cell["cost_usd"] = round(cell["cost_usd"], 3)
        cell["similarity"] = round(statistics.mean(cell["similarity"]), 3) if cell["similarity"] else None
    return {
        "cells": cells, "attempts": len(ledger),
        "total_cost_usd": round(sum(cell["cost_usd"] for cell in cells.values()), 2),
        "tool_uses": sum((row.get("call", {}).get("tool_uses") or 0) for row in ledger),
        "tools_offered": sorted({json.dumps(row.get("call", {}).get("tools_offered")) for row in ledger}),
    }


def publish() -> None:
    """Writes what the report cites: no program, no response, no seed."""
    ledger = json.loads((OUT / "ledger.json").read_text(encoding="utf-8"))
    write_json(RESULTS / "attempts.json", ledger)
    write_json(RESULTS / "summary.json", summary())
    write_json(RESULTS / "encodes.json", encodes_manifest())
    write_json(RESULTS / "viewer_metrics.json", viewer_metrics())


# The blind human arm: a kit an evaluator hands out, then scores.
VIEWER_FIELDS = ("participant", "clip", "understood_goal", "final_answer", "comfort_0_4", "identified_game")
RUBRIC_CATEGORIES = ("controls", "coherent_loop", "feedback", "visual_identity", "stability")
COMPREHENSION_LOSS_LIMIT = 0.05
COMFORT_MEDIAN_LIMIT = 1
IDENTIFICATION_FLOOR = 0.90


def viewer_kit(participants: int, seed: int, out: Path) -> dict:
    """Numbered clips in a separate shuffled order per participant, key sealed apart."""
    rng = random.Random(seed)
    public, sealed = out / "participant", out / "sealed"
    public.mkdir(parents=True)
    sealed.mkdir()
    arms = [(candidate, game) for candidate in CANDIDATES for game in GAMES]
    key, rows = {}, []
    for number, (candidate, game) in enumerate(rng.sample(arms, len(arms)), 1):
        clip = f"clip{number:02}"
        key[clip] = {"candidate": candidate, "game": game}
        source = OUT / "arms" / candidate / "transcode" / game / "clip.mp4"
        if source.exists():
            shutil.copy2(source, public / f"{clip}.mp4")
    clips = sorted(key)
    for participant in range(1, participants + 1):
        for clip in rng.sample(clips, len(clips)):
            rows.append({"participant": f"P{participant:02}", "clip": clip, **dict.fromkeys(VIEWER_FIELDS[2:], "")})
    with (public / "answers.csv").open("w", encoding="utf-8", newline="") as file:
        writer = csv.DictWriter(file, VIEWER_FIELDS)
        writer.writeheader()
        writer.writerows(rows)
    write_json(sealed / "key.json", key)
    return {"clips": len(clips), "rows": len(rows)}


def score_viewers(answers: Path, key_path: Path, truth: dict[str, dict]) -> dict:
    """Comprehension, comfort and identification per arm, and the stop rule.

    `truth[game]` gives the right `final_answer`. Only ordinary is a control.
    """
    key = json.loads(key_path.read_text(encoding="utf-8"))
    arms: dict[str, dict[str, list]] = {}
    with answers.open(encoding="utf-8", newline="") as file:
        for row in csv.DictReader(file):
            clip = key[row["clip"]]
            arm = arms.setdefault(clip["candidate"], {"comprehension": [], "comfort": [], "identified": []})
            right = row["final_answer"].strip().lower() == truth[clip["game"]]["final_answer"].lower()
            arm["comprehension"].append(row["understood_goal"].strip() == "yes" and right)
            arm["comfort"].append(int(row["comfort_0_4"]))
            arm["identified"].append(row["identified_game"].strip().lower() == clip["game"])
    control = statistics.mean(arms["ordinary"]["comprehension"])
    result = {}
    for candidate, arm in arms.items():
        comprehension = statistics.mean(arm["comprehension"])
        identified = statistics.mean(arm["identified"])
        comfort = statistics.median(arm["comfort"])
        result[candidate] = {
            "comprehension": round(comprehension, 3), "identified": round(identified, 3),
            "comfort_median": comfort, "worst_comfort": max(arm["comfort"]), "responses": len(arm["comfort"]),
            "passes": comprehension >= control - COMPREHENSION_LOSS_LIMIT
            and comfort <= COMFORT_MEDIAN_LIMIT and identified >= IDENTIFICATION_FLOOR,
        }
    return result


def score_rubric(rows: list[dict]) -> dict:
    """The 2912 floor: a median of 3 in every 0-4 category, three of four finish."""
    medians = {category: statistics.median(int(row[category]) for row in rows) for category in RUBRIC_CATEGORIES}
    finished = sum(row["finished_loop"] == "yes" for row in rows)
    return {
        "medians": medians, "finished": finished, "evaluators": len(rows),
        "over_floor": all(value >= 3 for value in medians.values()) and finished * 4 >= len(rows) * 3,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=(
        "capture", "surrogate", "encode", "commit-seed", "attack", "score", "summary", "publish", "viewer-kit",
    ))
    parser.add_argument("--stage", default="smoke")
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--model", default=MODELS[0], choices=MODELS)
    parser.add_argument("--everything", action="store_true")
    parser.add_argument("--candidate", choices=CANDIDATES)
    parser.add_argument("--participants", type=int, default=12)
    args = parser.parse_args()
    if args.action == "capture":
        capture_clean()
    elif args.action == "surrogate":
        surrogate_deltas()
    elif args.action == "encode":
        for candidate in (args.candidate,) if args.candidate else CANDIDATES:
            for transform in TRANSFORMS + (("decode", "decode_transcode") if candidate == "motion" else ()):
                encode_arm(candidate, transform)
        print(json.dumps(viewer_metrics(), indent=1))
    elif args.action == "commit-seed":
        print(commit_seed())
    elif args.action == "attack":
        run_stage(args.stage, args.workers, args.model, args.everything)
    elif args.action == "score":
        score_all()
    elif args.action == "summary":
        print(json.dumps(summary(), indent=1, sort_keys=True))
    elif args.action == "publish":
        publish()
    elif args.action == "viewer-kit":
        print(viewer_kit(args.participants, secrets.randbits(64), OUT / "viewer-kit"))


if __name__ == "__main__":
    main()

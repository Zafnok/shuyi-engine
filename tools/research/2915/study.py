"""Mark survival and capture checks for ticket 2915, on the 2912 fixtures only.

Clips come from the 2913 capture crate. Marks are painted on finished frames, then
each arm transforms them and an informed detector names a recipient out of 1,000.
Its outputs are under target/capture-study/, and tools/research/2915/results/ holds
what the report quotes.
"""

from __future__ import annotations

import argparse
from concurrent.futures import ProcessPoolExecutor
import csv
import difflib
import hashlib
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import subprocess
import sys
import tempfile
import time

import cv2
import numpy as np
from PIL import Image, ImageFilter

import marks
from marks import CANDIDATES, INVISIBLE, VISIBLE

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
OUT = ROOT / "target" / "capture-study"
RESULTS = HERE / "results"
CLEAN = OUT / "clean"
CAPTURE = ROOT / "target" / "clip-capture" / "release"
GAMES = ("courier", "workshop")
SCRIPTS = {
    # The shown winning scripts of tools/research/2912/study.py.
    "courier": "down,right,up,right,right,right,right,right",
    "workshop": "gather,gather,refine,craft,deliver,gather,rest,gather,refine,craft,deliver",
}
SHOWN_SEED = 11
RECIPIENTS = 1000
FPS = 30
HOLD = 20
HOLD_S = HOLD / FPS
TRANSFORM_ARMS = ("direct", "transcode", "ladder720", "vp9_480", "crop", "recapture_sim", "denoise",
                  "palette_snap", "temporal_median", "still_one", "flatten", "blind_transcode")
COLLUSION_ARMS = {"collude_avg2": 2, "collude_avg3": 3, "collude_avg5": 5, "collude_splice2": 2, "collude_min2": 2}
FLASH_S, GAP_S = 0.8, 0.4
READER_MODEL = "claude-haiku-5-5"
READER_BUDGET_USD = "1"


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="\n") as file:
        file.write(json.dumps(value, indent=1, sort_keys=True) + "\n")


def ffmpeg() -> str:
    import imageio_ffmpeg

    return imageio_ffmpeg.get_ffmpeg_exe()


def read_clean(game: str) -> list[np.ndarray]:
    return [np.asarray(Image.open(path).convert("RGB")) for path in sorted((CLEAN / game).glob("*.png"))]


def secret() -> str:
    path = OUT / "sealed" / "secret.txt"
    if not path.exists():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(secrets.token_hex(16), encoding="utf-8")
    return path.read_text(encoding="utf-8")


def truth_for(job: str, count: int) -> list[int]:
    """The sealed recipients of a job, derived from the secret; read only after scoring."""
    return [int(value) for value in marks.seeded(secret(), "truth", job).choice(RECIPIENTS, count, replace=False)]


# Encoding.


def pipe_encode(frames, dest: Path, codec_args: list[str], vf: str | None) -> None:
    frames = iter(frames)
    first = next(frames)
    dest.parent.mkdir(parents=True, exist_ok=True)
    args = [ffmpeg(), "-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24",
            "-s", f"{first.shape[1]}x{first.shape[0]}", "-r", str(FPS), "-i", "-", "-threads", "1",
            *codec_args, "-pix_fmt", "yuv420p", "-bitexact", "-flags", "+bitexact", "-map_metadata", "-1", "-an"]
    if vf:
        args += ["-vf", vf]
    process = subprocess.Popen([*args, str(dest)], stdin=subprocess.PIPE, stderr=subprocess.PIPE)
    process.stdin.write(first.tobytes())
    for frame in frames:
        process.stdin.write(frame.tobytes())
    process.stdin.close()
    error = process.stderr.read()
    if process.wait() != 0:
        raise RuntimeError(error.decode(errors="replace"))


X264_SOCIAL = ["-c:v", "libx264", "-preset", "veryfast", "-crf", "30"]
X264_720 = ["-c:v", "libx264", "-preset", "medium", "-crf", "28", "-maxrate", "1500k", "-bufsize", "3000k"]
VP9_480 = ["-c:v", "libvpx-vp9", "-b:v", "0", "-crf", "36", "-cpu-used", "5", "-row-mt", "0"]


def video_frames(path: Path):
    capture = cv2.VideoCapture(str(path))
    while True:
        ok, frame = capture.read()
        if not ok:
            break
        yield cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
    capture.release()


def held(states: list[np.ndarray]):
    for state in states:
        for _ in range(HOLD):
            yield state


def hold_means(path: Path, count: int) -> list[np.ndarray]:
    """The mean of each hold's middle frames: what the detector gets from a video."""
    totals: list[np.ndarray | None] = [None] * count
    used = [0] * count
    for index, frame in enumerate(video_frames(path)):
        state = index // HOLD
        if state < count and 4 <= index % HOLD < HOLD - 4:
            totals[state] = frame.astype(np.float32) if totals[state] is None else totals[state] + frame
            used[state] += 1
    return [(total / used[state]).round().astype(np.uint8) for state, total in enumerate(totals)]


def recapture_sim(frame: np.ndarray, rng: np.random.Generator, moire: np.ndarray) -> np.ndarray:
    """Filmed-screen simulation, as in 2913; NOT a phone, and labelled so in the report."""
    height, width = frame.shape[:2]
    picture = Image.fromarray(frame).transform(
        (width, height), Image.PERSPECTIVE,
        (1.02, 0.015, -6, -0.012, 1.03, 3, 0.00002, 0.00001), Image.BILINEAR,
    ).filter(ImageFilter.GaussianBlur(0.9))
    values = np.asarray(picture).astype(np.float32) / 255
    values = (values ** 0.9) * moire + rng.normal(0, 0.012, values.shape)
    return (np.clip(values, 0, 1) * 255).astype(np.uint8)


def palette_snap(frame: np.ndarray, size: int = 12) -> np.ndarray:
    """An attacker's cleanup that needs no clean frame: every pixel goes to a common colour."""
    packed = (frame[..., 0].astype(np.int32) >> 2 << 16) | (frame[..., 1].astype(np.int32) >> 2 << 8) | (frame[..., 2] >> 2)
    values, counts = np.unique(packed, return_counts=True)
    top = values[np.argsort(counts)[-size:]]
    palette = np.stack(((top >> 16) & 63, (top >> 8) & 63, top & 63), axis=1).astype(np.float32) * 4 + 2
    distance = ((frame[:, :, None, :].astype(np.float32) - palette[None, None, :, :]) ** 2).sum(axis=-1)
    return palette[distance.argmin(axis=-1)].round().astype(np.uint8)


def flatten(frame: np.ndarray, reach: int = 20) -> np.ndarray:
    """An adaptive cleanup, written after seeing the marks: near the commonest colour becomes it."""
    packed = (frame[..., 0].astype(np.int32) << 16) | (frame[..., 1].astype(np.int32) << 8) | frame[..., 2]
    values, counts = np.unique(packed, return_counts=True)
    common = values[counts.argmax()]
    mode = np.array(((common >> 16) & 255, (common >> 8) & 255, common & 255), dtype=np.int32)
    near = np.abs(frame.astype(np.int32) - mode).max(axis=-1) <= reach
    result = frame.copy()
    result[near] = mode
    return result


# Registration: the operator aligns what leaked to the clean capture.


def _gray(frame: np.ndarray, scale: float = 1.0) -> np.ndarray:
    gray = cv2.cvtColor(frame, cv2.COLOR_RGB2GRAY)
    return gray if scale == 1.0 else cv2.resize(gray, None, fx=scale, fy=scale, interpolation=cv2.INTER_AREA)


def initial_homography(received: np.ndarray, clean: np.ndarray) -> np.ndarray:
    orb = cv2.ORB_create(6000, fastThreshold=5)
    points_a, features_a = orb.detectAndCompute(_gray(received), None)
    points_b, features_b = orb.detectAndCompute(_gray(clean), None)
    fallback = np.diag((clean.shape[1] / received.shape[1], clean.shape[0] / received.shape[0], 1.0))
    if features_a is None or features_b is None:
        return fallback
    matcher = cv2.BFMatcher(cv2.NORM_HAMMING)
    good = [pair[0] for pair in matcher.knnMatch(features_a, features_b, k=2)
            if len(pair) == 2 and pair[0].distance < 0.8 * pair[1].distance]
    if len(good) < 12:
        return fallback
    source = np.float32([points_a[match.queryIdx].pt for match in good])
    target = np.float32([points_b[match.trainIdx].pt for match in good])
    found, inliers = cv2.findHomography(source, target, cv2.RANSAC, 3.0)
    return found if found is not None and int(inliers.sum()) >= 10 else fallback


def refine(received: np.ndarray, clean: np.ndarray, start: np.ndarray) -> tuple[np.ndarray, float]:
    """ECC from the feature match, at half size; returns the map and its correlation."""
    half = np.diag((0.5, 0.5, 1.0))
    warp = (half @ start @ np.linalg.inv(half)).astype(np.float32)
    warp = warp / warp[2, 2]
    target = _gray(clean, 0.5).astype(np.float32)
    moving = _gray(received, 0.5).astype(np.float32)
    # ECC maps target coordinates to moving ones, so it takes the inverse.
    inverse = np.linalg.inv(warp).astype(np.float32)
    criteria = (cv2.TERM_CRITERIA_EPS | cv2.TERM_CRITERIA_COUNT, 60, 1e-5)
    try:
        correlation, inverse = cv2.findTransformECC(target, moving, inverse, cv2.MOTION_HOMOGRAPHY, criteria, None, 5)
    except cv2.error:
        return start, 0.0
    # In full-size terms: received to clean.
    refined = np.linalg.inv(half) @ np.linalg.inv(inverse) @ half
    return refined / refined[2, 2], float(correlation)


def register(received: np.ndarray, clean: np.ndarray) -> tuple[np.ndarray, np.ndarray, float]:
    """Returns the frame in the clean frame's coordinates, what it covers, and the fit."""
    height, width = clean.shape[:2]
    if received.shape == clean.shape and float(np.abs(_gray(received).astype(int) - _gray(clean)).mean()) < 6:
        return received, np.ones((height, width), bool), 1.0
    start = initial_homography(received, clean)
    homography, fit = refine(received, clean, start)
    warped = cv2.warpPerspective(received, homography, (width, height), flags=cv2.INTER_AREA)
    filled = np.full(received.shape[:2], 255, np.uint8)
    coverage = cv2.warpPerspective(filled, homography, (width, height), flags=cv2.INTER_NEAREST) == 255
    return warped, cv2.erode(coverage.astype(np.uint8), np.ones((9, 9), np.uint8)) > 0, fit


def screen_quad(flash: np.ndarray) -> np.ndarray | None:
    """The kit's white picture in a filmed frame: its four corners, top left first, clockwise."""
    gray = cv2.GaussianBlur(_gray(flash), (5, 5), 0)
    _, bright = cv2.threshold(gray, 0, 255, cv2.THRESH_BINARY + cv2.THRESH_OTSU)
    contours, _ = cv2.findContours(bright, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    if not contours:
        return None
    largest = max(contours, key=cv2.contourArea)
    if cv2.contourArea(largest) < 0.05 * gray.size:
        return None
    hull = cv2.convexHull(largest)
    for share in (0.01, 0.02, 0.03, 0.05, 0.08):
        corners = cv2.approxPolyDP(hull, share * cv2.arcLength(hull, True), True)
        if len(corners) == 4:
            points = corners.reshape(4, 2).astype(np.float32)
            total, difference = points.sum(axis=1), np.diff(points, axis=1).ravel()
            return np.float32((points[total.argmin()], points[difference.argmin()],
                               points[total.argmax()], points[difference.argmax()]))
    return None


def register_by_screen(received: np.ndarray, clean: np.ndarray, quad: np.ndarray) -> tuple[np.ndarray, np.ndarray, float]:
    """The flash's corners fix the screen's shape; a small rigid move then follows the hand."""
    height, width = clean.shape[:2]
    corners = np.float32(((0, 0), (width, 0), (width, height), (0, height)))
    homography = cv2.getPerspectiveTransform(quad, corners)
    rough = cv2.warpPerspective(received, homography, (width, height), flags=cv2.INTER_AREA)
    move = np.eye(2, 3, dtype=np.float32)
    criteria = (cv2.TERM_CRITERIA_EPS | cv2.TERM_CRITERIA_COUNT, 80, 1e-6)
    try:
        fit, move = cv2.findTransformECC(_gray(clean).astype(np.float32), _gray(rough).astype(np.float32), move,
                                         cv2.MOTION_EUCLIDEAN, criteria, None, 5)
    except cv2.error:
        fit = 0.0
    warped = cv2.warpAffine(rough, move, (width, height), flags=cv2.INTER_LINEAR | cv2.WARP_INVERSE_MAP)
    filled = np.full((height, width), 255, np.uint8)
    coverage = cv2.warpAffine(filled, move, (width, height), flags=cv2.INTER_NEAREST | cv2.WARP_INVERSE_MAP) == 255
    return warped, cv2.erode(coverage.astype(np.uint8), np.ones((9, 9), np.uint8)) > 0, float(fit)


# Scoring.


def visible_layers(candidate: str, game: str, states: int) -> np.ndarray:
    """Every recipient's drawn mark at low size, on disk, so workers can share it."""
    folder = OUT / "layers"
    folder.mkdir(parents=True, exist_ok=True)
    layers = 1 if candidate != "visible_drift" else states
    size = marks.small_size(candidate)
    path = folder / f"{candidate}-{game}-{size[0]}.npy"
    shape = (RECIPIENTS, layers, size[1], size[0])
    if not path.exists():
        temporary = folder / f"{candidate}-{game}-{os.getpid()}.tmp"
        output = np.lib.format.open_memmap(temporary, mode="w+", dtype=np.uint8, shape=shape)
        for recipient in range(RECIPIENTS):
            for layer in range(layers):
                output[recipient, layer] = marks.small_layer(candidate, secret(), recipient, game, layer, 736, 1280)
        output.flush()
        del output
        try:
            temporary.replace(path)
        except OSError:
            temporary.unlink(missing_ok=True)
    return np.load(path, mmap_mode="r")


def score_states(candidate: str, game: str, received: list[tuple[int, np.ndarray, np.ndarray]],
                 clean: list[np.ndarray]) -> np.ndarray:
    """Every recipient's score; `received` holds a state's index, frame and coverage."""
    if candidate in VISIBLE:
        return marks.visible_scores(received, clean, visible_layers(candidate, game, len(clean)))
    residuals = [(state, *marks.state_residual(frame, clean[state], coverage)) for state, frame, coverage in received]
    return marks.invisible_scores(residuals, secret(), game, RECIPIENTS)


def scored_candidate(candidate: str) -> str:
    # An unmarked clip is tested with the mark a real detector would look for.
    return "invisible4" if candidate == "ordinary" else candidate


def arm_frames(candidate: str, game: str, arm: str, clean: list[np.ndarray], job: str) -> tuple[list[np.ndarray], list[int], int]:
    """Runs one arm. Returns the frames for the detector, which states they are, and bytes."""
    count = COLLUSION_ARMS.get(arm, 1)
    recipients = truth_for(job, count)
    painted = [[marks.paint(frame, candidate, secret(), recipient, game, state)
                for state, frame in enumerate(clean)] for recipient in recipients]
    states = list(range(len(clean)))
    if arm == "direct":
        return painted[0], states, 0
    if arm.startswith("collude_"):
        stack = np.stack(painted).astype(np.float32)
        if "splice" in arm:
            mixed = [painted[state % count][state] for state in states]
        elif "min" in arm:
            mixed = list(stack.min(axis=0).astype(np.uint8))
        else:
            mixed = list(stack.mean(axis=0).round().astype(np.uint8))
        source, codec, vf = mixed, X264_SOCIAL, "scale=854:-2"
    elif arm == "ladder720":
        source, codec, vf = painted[0], X264_720, "scale=-2:720"
    elif arm == "vp9_480":
        source, codec, vf = painted[0], VP9_480, "scale=-2:480"
    elif arm == "denoise":
        source, codec, vf = painted[0], X264_SOCIAL, "scale=854:-2,hqdn3d=6:6:8:8"
    else:
        source, codec, vf = painted[0], X264_SOCIAL, "scale=854:-2"
    with tempfile.TemporaryDirectory(dir=OUT / "tmp") as folder:
        path = Path(folder) / ("clip.webm" if codec is VP9_480 else "clip.mp4")
        pipe_encode(held(source), path, codec, vf)
        size = path.stat().st_size
        means = hold_means(path, len(clean))
        if arm == "crop":
            height, width = means[0].shape[:2]
            top, left = height * 6 // 100, width * 6 // 100
            again = Path(folder) / "again.mp4"
            pipe_encode(held([frame[top:height - top, left:width - left] for frame in means]), again, X264_SOCIAL,
                        "scale=trunc(iw/2)*2:trunc(ih/2)*2")
            means = hold_means(again, len(clean))
        elif arm == "recapture_sim":
            rng = marks.seeded("recapture", game)
            height, width = means[0].shape[:2]
            ys, xs = np.mgrid[0:height, 0:width]
            moire = 1 + 0.035 * np.sin(xs * 0.73 + ys * 0.41)[..., None]
            filmed = [recapture_sim(frame, rng, moire) for frame in means]
            again = Path(folder) / "again.mp4"
            pipe_encode(held(filmed), again, ["-c:v", "libx264", "-preset", "veryfast", "-crf", "26"], None)
            means = hold_means(again, len(clean))
        elif arm == "palette_snap":
            means = [palette_snap(frame) for frame in means]
        elif arm == "flatten":
            means = [flatten(frame) for frame in means]
        elif arm == "temporal_median":
            median = np.median(np.stack(means), axis=0).astype(np.uint8)
            means = [median] * len(means)
    if arm == "still_one":
        middle = len(clean) // 2
        return [means[middle]], [middle], size
    return means, states, size


def run_job(job_text: str) -> dict:
    candidate, game, arm = job_text.split("/")
    cv2.setNumThreads(1)
    clean = read_clean(game)
    start = time.time()
    frames, states, size = arm_frames(candidate, game, arm, clean, job_text)
    received = []
    fits = []
    for frame, state in zip(frames, states):
        if arm == "blind_transcode":
            # No clean capture: the frame is only scaled back, and is its own reference below.
            warped = cv2.resize(frame, (clean[state].shape[1], clean[state].shape[0]), interpolation=cv2.INTER_LINEAR)
            coverage, fit = np.ones(warped.shape[:2], bool), 0.0
        else:
            warped, coverage, fit = register(frame, clean[state])
        received.append((state, warped, coverage))
        fits.append(fit)
    if arm == "blind_transcode":
        clean = list(clean)
        for state, warped, _ in received:
            clean[state] = flatten(warped)
    scores = score_states(scored_candidate(candidate), game, received, clean)
    # The sealed key is read only now that every score exists.
    truth = truth_for(job_text, COLLUSION_ARMS.get(arm, 1))
    if candidate != "ordinary":
        result = marks.verdict(scores, truth)
    else:
        result = {"best_any": round(float(scores.max()), 2), "voids_detector": bool(scores.max() >= marks.THRESHOLD)}
    return {"job": job_text, "bytes": size, "mean_fit": round(float(np.mean(fits)), 4),
            "seconds": round(time.time() - start, 1), **result}


def all_jobs() -> list[str]:
    return [f"{candidate}/{game}/{arm}" for candidate in CANDIDATES for game in GAMES
            for arm in (*TRANSFORM_ARMS, *COLLUSION_ARMS)
            if not (candidate == "ordinary" and arm in COLLUSION_ARMS)]


def run_marks(workers: int, only: str | None) -> None:
    (OUT / "tmp").mkdir(parents=True, exist_ok=True)
    jobs = [job for job in all_jobs() if not only or re.search(only, job)]
    for candidate in VISIBLE:
        for game in GAMES:
            visible_layers(candidate, game, len(read_clean(game)))
    path = OUT / "marks.jsonl"
    done = set()
    if path.exists():
        done = {json.loads(line)["job"] for line in path.read_text(encoding="utf-8").splitlines()}
    jobs = [job for job in jobs if job not in done]
    with ProcessPoolExecutor(workers) as pool, path.open("a", encoding="utf-8", newline="\n") as ledger:
        for result in pool.map(run_job, jobs):
            ledger.write(json.dumps(result, sort_keys=True) + "\n")
            ledger.flush()
            print(result, flush=True)


def ssim(a: np.ndarray, b: np.ndarray) -> float:
    a, b = a.astype(np.float64), b.astype(np.float64)
    blur = lambda value: cv2.GaussianBlur(value, (11, 11), 1.5)
    mean_a, mean_b = blur(a), blur(b)
    var_a, var_b, covariance = blur(a * a) - mean_a ** 2, blur(b * b) - mean_b ** 2, blur(a * b) - mean_a * mean_b
    c1, c2 = (0.01 * 255) ** 2, (0.03 * 255) ** 2
    return float((((2 * mean_a * mean_b + c1) * (2 * covariance + c2))
                  / ((mean_a ** 2 + mean_b ** 2 + c1) * (var_a + var_b + c2))).mean())


def viewer_metrics() -> dict:
    report = {}
    for candidate in CANDIDATES:
        rows = []
        for game in GAMES:
            clean = read_clean(game)
            for state, frame in enumerate(clean):
                painted = marks.paint(frame, candidate, secret(), 7, game, state)
                luma_a, luma_b = marks.luma(frame), marks.luma(painted)
                mse = float(((luma_a - luma_b) ** 2).mean())
                rows.append((ssim(_gray(frame), _gray(painted)), 99.0 if mse == 0 else 10 * np.log10(255 ** 2 / mse),
                             float(np.abs(luma_a - luma_b).max())))
        report[candidate] = {"ssim": round(float(np.mean([row[0] for row in rows])), 4),
                             "luma_psnr_db": round(float(np.mean([row[1] for row in rows])), 1),
                             "largest_luma_change": round(float(max(row[2] for row in rows)), 1)}
    return report


# Capture of the fixtures.


def capture() -> None:
    binary = CAPTURE / ("clip-capture" + (".exe" if os.name == "nt" else ""))
    report = {}
    for game in GAMES:
        subprocess.run([str(binary), game, str(SHOWN_SEED), SCRIPTS[game], str(CLEAN / game)], check=True)
        digest = hashlib.sha256()
        for path in sorted((CLEAN / game).glob("*.png")):
            digest.update(path.read_bytes())
        report[game] = {"frames": len(list((CLEAN / game).glob("*.png"))), "sha256": digest.hexdigest()}
    write_json(RESULTS / "fixtures.json", report)
    print(report)


def fixture_text(game: str) -> list[str]:
    binary = CAPTURE / ("cloning-fixture" + (".exe" if os.name == "nt" else ""))
    process = subprocess.run([str(binary), game, str(SHOWN_SEED), SCRIPTS[game]], capture_output=True, text=True, check=True)
    return [record["frame"] for record in json.loads(process.stdout)["trace"]]


# The kit: what a phone films and a platform re-encodes.


def kit_plan() -> list[dict]:
    plan = []
    for candidate in CANDIDATES:
        for game in GAMES:
            plan.append({"candidate": candidate, "game": game, "job": f"{candidate}/{game}/kit"})
    return plan


def kit_files() -> Path:
    folder = OUT / "kit"
    frames = folder / "frames"
    frames.mkdir(parents=True, exist_ok=True)
    white, black = frames / "white.png", frames / "black.png"
    Image.new("RGB", (1280, 736), (255, 255, 255)).save(white)
    Image.new("RGB", (1280, 736), (0, 0, 0)).save(black)
    steps = [{"path": str(black), "seconds": 1.5}]
    video = []
    for segment in kit_plan():
        clean = read_clean(segment["game"])
        recipient = truth_for(segment["job"], 1)[0]
        steps += [{"path": str(white), "seconds": FLASH_S}, {"path": str(black), "seconds": GAP_S}]
        video.append(("flash", None))
        for state, frame in enumerate(clean):
            path = frames / f"{segment['candidate']}-{segment['game']}-{state:02}.png"
            painted = marks.paint(frame, segment["candidate"], secret(), recipient, segment["game"], state)
            Image.fromarray(painted).save(path)
            steps.append({"path": str(path), "seconds": HOLD_S, **({"check": path.stem} if state == 0 else {})})
            video.append((path, None))
    steps.append({"path": str(black), "seconds": 1.0})
    write_json(folder / "exclude_on.json", {"exclude": True, "steps": steps})
    write_json(folder / "exclude_off.json", {"exclude": False, "steps": steps})
    return folder


def kit_master() -> Path:
    """The same sequence as a video, for a platform upload; high quality, no recipient shown."""
    folder = kit_files()
    plan = json.loads((folder / "exclude_on.json").read_text(encoding="utf-8"))

    def frames():
        for step in plan["steps"]:
            frame = np.asarray(Image.open(step["path"]).convert("RGB"))
            for _ in range(round(step["seconds"] * FPS)):
                yield frame

    path = folder / "platform-master.mp4"
    pipe_encode(frames(), path, ["-c:v", "libx264", "-preset", "slow", "-crf", "14"], None)
    return path


def find_flashes(path: Path) -> tuple[list[float], float]:
    """Each white flash's start, in seconds, from the brightness of the whole picture."""
    capture_video = cv2.VideoCapture(str(path))
    times, levels = [], []
    while True:
        ok, frame = capture_video.read()
        if not ok:
            break
        times.append(capture_video.get(cv2.CAP_PROP_POS_MSEC) / 1000)
        levels.append(float(cv2.resize(frame, (64, 36), interpolation=cv2.INTER_AREA).mean()))
    capture_video.release()
    levels = np.array(levels)
    threshold = (np.percentile(levels, 5) + levels.max()) / 2
    bright = levels > threshold
    starts = []
    index = 0
    while index < len(bright):
        if bright[index]:
            end = index
            while end + 1 < len(bright) and bright[end + 1]:
                end += 1
            if 0.4 <= times[end] - times[index] <= 1.6:
                starts.append(times[index])
            index = end + 1
        else:
            index += 1
    return starts, (float(times[-1]) if times else 0.0)


def match_flashes(found: list[float], lengths: list[float], tolerance: float = 0.4) -> list[float] | None:
    """Picks the real flashes by the kit's known schedule; a camera's exposure adds false ones."""
    offsets = np.concatenate(([0.0], np.cumsum(lengths)))[:len(lengths)]
    times = np.array(found)
    best, best_hits = None, 0
    for first in found:
        hits = [times[np.abs(times - (first + offset)).argmin()] for offset in offsets]
        close = [abs(hit - (first + offset)) <= tolerance for hit, offset in zip(hits, offsets)]
        if sum(close) > best_hits:
            best, best_hits = [float(hit) for hit in hits], sum(close)
    return best if best_hits == len(lengths) else None


def score_recording(path: Path, label: str, samples: int) -> dict:
    """Scores a phone film or a platform's encode of the kit, segment by segment."""
    found, _ = find_flashes(path)
    plan = kit_plan()
    lengths = [FLASH_S + GAP_S + len(read_clean(segment["game"])) * HOLD_S for segment in plan]
    starts = match_flashes(found, lengths)
    report: dict = {"label": label, "file_sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                    "flashes_found": len(found), "flashes_expected": len(plan), "segments": {}}
    if starts is None:
        report["error"] = "the flashes do not fit the kit's schedule; segments cannot be matched"
        return report
    capture_video = cv2.VideoCapture(str(path))
    report["size"] = [int(capture_video.get(cv2.CAP_PROP_FRAME_WIDTH)), int(capture_video.get(cv2.CAP_PROP_FRAME_HEIGHT))]
    report["fps"] = round(float(capture_video.get(cv2.CAP_PROP_FPS)), 2)
    for start, segment in zip(starts, plan):
        clean = read_clean(segment["game"])
        received = []
        fits = []
        first = start + FLASH_S + GAP_S
        capture_video.set(cv2.CAP_PROP_POS_MSEC, (start + FLASH_S * 0.6) * 1000)
        ok, flash = capture_video.read()
        quad = screen_quad(cv2.cvtColor(flash, cv2.COLOR_BGR2RGB)) if ok else None
        for state, frame in enumerate(clean):
            total = None
            cover = None
            used = 0
            for step in range(samples):
                moment = first + (state + 0.2 + 0.6 * (step + 0.5) / samples) * HOLD_S
                capture_video.set(cv2.CAP_PROP_POS_MSEC, moment * 1000)
                ok, shot = capture_video.read()
                if not ok:
                    continue
                shot = cv2.cvtColor(shot, cv2.COLOR_BGR2RGB)
                warped, coverage, fit = register(shot, frame) if quad is None else register_by_screen(shot, frame, quad)
                fits.append(fit)
                # A glossy screen's reflection keeps a good alignment's correlation low.
                if fit < 0.2:
                    continue
                total = warped.astype(np.float32) if total is None else total + warped
                cover = coverage if cover is None else cover & coverage
                used += 1
            if used:
                received.append((state, (total / used).round().astype(np.uint8), cover))
        name = f"{segment['candidate']}/{segment['game']}"
        if not received:
            report["segments"][name] = {"states_registered": 0, "passes": False}
            continue
        scores = score_states(scored_candidate(segment["candidate"]), segment["game"], received, clean)
        truth = truth_for(segment["job"], 1)
        if segment["candidate"] == "ordinary":
            result = {"best_any": round(float(scores.max()), 2), "voids_detector": bool(scores.max() >= marks.THRESHOLD)}
        else:
            result = marks.verdict(scores, truth)
        report["segments"][name] = {"states_registered": len(received), "mean_fit": round(float(np.mean(fits)), 3),
                                    "screen_corners_found": quad is not None, **result}
        # Frames for the reader and the human look stay under target/.
        keep = OUT / "recordings" / label
        keep.mkdir(parents=True, exist_ok=True)
        Image.fromarray(received[len(received) // 2][1]).save(keep / f"{segment['candidate']}-{segment['game']}.png")
    capture_video.release()
    return report


# The reader: a tool-free model standing in for whether a viewer can read a frame.


def reader_turn(image: Path, folder: Path) -> dict:
    import base64

    prompt = ("This is one frame of gameplay. First copy out the game's own text exactly, in a block fenced with "
              "```text. Then, if an overlay code of the form XXXX-XXXX appears anywhere, write it on a last line as "
              "CODE: XXXX-XXXX, or CODE: none. Give your best reading.")
    message = {"type": "user", "message": {"role": "user", "content": [
        {"type": "image", "source": {"type": "base64", "media_type": "image/png",
                                     "data": base64.b64encode(image.read_bytes()).decode()}},
        {"type": "text", "text": prompt}]}}
    executable = shutil.which("claude") or "claude"
    with tempfile.TemporaryDirectory(prefix="capture-reader-") as empty:
        process = subprocess.run(
            [executable, "-p", "--model", READER_MODEL, "--tools", "", "--strict-mcp-config", "--setting-sources", "",
             "--no-session-persistence", "--disable-slash-commands", "--system-prompt", "You transcribe images.",
             "--input-format", "stream-json", "--output-format", "stream-json", "--verbose",
             "--max-budget-usd", READER_BUDGET_USD],
            input=json.dumps(message), text=True, encoding="utf-8", capture_output=True, cwd=empty, timeout=300, check=False)
    final = {}
    for line in process.stdout.splitlines():
        try:
            event = json.loads(line)
        except ValueError:
            continue
        if event.get("type") == "result":
            final = event
    return {"text": final.get("result") or "", "cost_usd": final.get("total_cost_usd"), "exit_code": process.returncode}


def score_reading(text: str, truth_text: str, truth_code: str | None) -> dict:
    block = re.search(r"```(?:text)?\s*\n(.*?)```", text, re.DOTALL)
    copied = block.group(1) if block else text
    squeeze = lambda value: re.sub(r"\s+", " ", value).strip()
    named = re.findall(r"CODE:\s*([A-Z0-9]{4}-[A-Z0-9]{4})", text)
    return {"game_text_score": round(100 * difflib.SequenceMatcher(None, squeeze(copied), squeeze(truth_text)).ratio(), 1),
            "code_read": named[-1] if named else None,
            "code_correct": bool(truth_code and named and truth_code in named)}


def reader_frames(candidate: str, arm: str, game: str, job: str) -> tuple[np.ndarray, str | None, int]:
    clean = read_clean(game)
    (OUT / "tmp").mkdir(parents=True, exist_ok=True)
    frames, states, _ = arm_frames(candidate, game, arm, clean, job)
    index = len(frames) // 2
    recipient = truth_for(job, 1)[0]
    return frames[index], (None if candidate == "ordinary" else marks.code(secret(), recipient)), states[index]


def run_reader() -> None:
    arms = ("direct", "transcode", "crop", "recapture_sim", "collude_avg2", "collude_min2", "palette_snap")
    candidates = ("ordinary", "invisible8", "visible_corner", "visible_tile", "visible_drift")
    rows = []
    folder = OUT / "reader"
    folder.mkdir(parents=True, exist_ok=True)
    for candidate in candidates:
        for arm in arms:
            if candidate == "ordinary" and arm.startswith("collude"):
                continue
            for game in GAMES:
                job = f"{candidate}/{game}/{arm}"
                frame, code, state = reader_frames(candidate, arm, game, job)
                path = folder / f"{candidate}-{arm}-{game}.png"
                Image.fromarray(frame).save(path)
                turn = reader_turn(path, folder)
                truth_text = fixture_text(game)[state]
                row = {"job": job, "state": state, "cost_usd": turn["cost_usd"],
                       **score_reading(turn["text"], truth_text, code)}
                row["code_read"] = "correct" if row["code_correct"] else ("wrong" if row["code_read"] else "none")
                rows.append(row)
                print(row, flush=True)
    write_json(RESULTS / "reader.json", rows)


def read_recording(label: str) -> None:
    """The reader on one saved frame of each segment of a real recording."""
    rows = []
    folder = OUT / "recordings" / label
    for segment in kit_plan():
        path = folder / f"{segment['candidate']}-{segment['game']}.png"
        if not path.exists():
            rows.append({"job": segment["job"], "missing": True})
            continue
        state = len(read_clean(segment["game"])) // 2
        code = None if segment["candidate"] == "ordinary" else marks.code(secret(), truth_for(segment["job"], 1)[0])
        turn = reader_turn(path, folder)
        row = {"job": f"{segment['candidate']}/{segment['game']}/{label}", "state": state, "cost_usd": turn["cost_usd"],
               **score_reading(turn["text"], fixture_text(segment["game"])[state], code)}
        row["code_read"] = "correct" if row["code_correct"] else ("wrong" if row["code_read"] else "none")
        rows.append(row)
        print(row, flush=True)
    write_json(RESULTS / f"reader-{label}.json", rows)


def publish() -> None:
    rows = [json.loads(line) for line in (OUT / "marks.jsonl").read_text(encoding="utf-8").splitlines()]
    write_json(RESULTS / "marks.json", sorted(rows, key=lambda row: row["job"]))
    write_json(RESULTS / "viewer_metrics.json", viewer_metrics())


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("capture", "kit", "master", "marks", "reader", "recording", "read-recording", "publish"))
    parser.add_argument("--workers", type=int, default=6)
    parser.add_argument("--only")
    parser.add_argument("--file")
    parser.add_argument("--label")
    parser.add_argument("--samples", type=int, default=4)
    args = parser.parse_args()
    if args.command == "capture":
        capture()
    elif args.command == "kit":
        print(kit_files())
    elif args.command == "master":
        print(kit_master())
    elif args.command == "marks":
        run_marks(args.workers, args.only)
    elif args.command == "reader":
        run_reader()
    elif args.command == "recording":
        report = score_recording(Path(args.file), args.label, args.samples)
        write_json(RESULTS / f"recording-{args.label}.json", report)
        print(json.dumps(report, indent=1))
    elif args.command == "read-recording":
        read_recording(args.label)
    else:
        publish()


if __name__ == "__main__":
    sys.path.insert(0, str(HERE))
    main()

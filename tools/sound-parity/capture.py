#!/usr/bin/env python3
"""Records what the sound backend really emits (ticket 2573, ADR-0066).

`README.md` beside it says how to run it; CI's `sound-parity` job does. On a
PulseAudio null sink of its own, in a temporary home, so nothing else is heard
and nothing is heard elsewhere, it records the sink's monitor while the probe
(`crates/app/examples/sound_probe.rs`) plays:

    capture.py native --dir D --probe BIN --revision SHA
    capture.py windows --dir D --probe BIN --revision SHA [--set-endpoint]
    capture.py browser --dir D --page DIR --revision SHA
    (either with --perf-only: the timings again, the recordings kept)
    capture.py encode --dir D       (by hand, on Linux) the Ogg fixtures

Natively ALSA's default device is routed to the sink, as a player's system
would. On Windows there is no such sink to make: it records the default render
endpoint through `wasapi.py`, on a runner a virtual one (`install-endpoint.ps1`). In
the browser, the page that `shuyi web` writes runs in Chromium with
Playwright's mute flag taken away and a real click to unlock it. Each writes
`D/<platform>/run-N/<scenario>.pcm` and `.jsonl`, `meta.json`, and the paired
timings in `D/<platform>/perf.jsonl`. `cargo xtask sound-parity gate` judges
them: this script records, and decides nothing.
"""

from __future__ import annotations

import argparse
import array
import contextlib
import ctypes
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from collections.abc import Callable, Iterator
from dataclasses import dataclass, field
from pathlib import Path
from typing import TypeVar

T = TypeVar("T")

TOOL = Path(__file__).resolve().parent
SINK = "shuyi_parity"
RATE = 44100
PREFIX = "shuyi-sound-probe: "
# Two recordings of every scenario: the gate compares them with each other.
RUNS = 2
# The most any one probe run may take, in seconds.
TIMEOUT = 30
# Runs thrown away first so that caches and the page's code are warm.
WARMUP = 2
PAIRS = 9
# The injected slowdown of the `burn` side, in percent of each phase.
BURN = 100
# The Ogg fixtures: their source, and the comments of the one with a region.
OGGS = {name: [] for name in ["fmt_44k_s", "fmt_22k_m", "fmt_48k_s", "music_a", "music_b", "long"]}
OGGS["intro_loop"] = ["LOOPSTART=44100", "LOOPLENGTH=44100"]
OGGENC = ["--quiet", "--quality=6", "--serial=2573"]
# Chromium's real output path: the full browser in new-headless mode (the
# headless shell has no sound), SwiftShader as in tools/web-check/check.py.
CHROMIUM_ARGS = ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"]
# The silence a recording starts with before anything plays, in frames.
LEAD_FRAMES = RATE // 5
# Sound after this far under full scale counts as sound, for finding gaps.
QUIET = 8
# A run of quiet this long while everything plays is an underrun, in frames.
GAP_FRAMES = RATE // 200


# How quad-snd's ALSA output starts the line it prints when it underruns, spelt its way.
UNDERRUN = "Underrun occured"


# How the sink's monitor is read.
RECORD = ["parec", f"--device={SINK}.monitor", "--format=s16le", f"--rate={RATE}", "--channels=2", "--latency-msec=10"]


@dataclass
class Sink:
    """The null sink and the environment that plays into it."""

    env: dict[str, str]


@contextlib.contextmanager
def sink() -> Iterator[Sink]:
    """Runs PulseAudio with only a null sink, in a home of its own."""
    with tempfile.TemporaryDirectory(prefix="sound-parity-") as home:
        env = {
            **os.environ,
            "HOME": home,
            "XDG_RUNTIME_DIR": home,
            "PULSE_RUNTIME_PATH": f"{home}/pulse",
            "PULSE_SERVER": f"unix:{home}/pulse/native",
            "PULSE_SINK": SINK,
            # What every stream asks of the sink. Left to itself a native stream
            # gets the sink's largest blocks, about 2 seconds, and the end of a
            # recording is cut off with the process that held them.
            "PULSE_LATENCY_MSEC": "20",
        }
        Path(home, ".asoundrc").write_text('pcm.!default { type pulse }\nctl.!default { type pulse }\n', encoding="utf-8")
        server = subprocess.Popen(
            [
                "pulseaudio", "--daemonize=no", "--exit-idle-time=-1", "--disallow-exit",
                "--use-pid-file=no", "--disable-shm=yes", "-n",
                "--load=module-native-protocol-unix",
                f"--load=module-null-sink sink_name={SINK} rate={RATE} channels=2 format=s16le",
            ],
            env=env,
        )
        try:
            for _ in range(100):
                ran = subprocess.run(["pactl", "info"], env=env, capture_output=True, check=False)
                if ran.returncode == 0:
                    break
                time.sleep(0.1)
            else:
                raise SystemExit("PulseAudio didn't start")
            subprocess.run(["pactl", "set-default-sink", SINK], env=env, check=True)
            subprocess.run(["pactl", "set-sink-volume", SINK, "100%"], env=env, check=True)
            # A monitor stream held open for the whole session: an idle sink
            # takes its largest blocks, about 2 seconds, and a stream that arrives
            # then stalls, so that commands land together. A sink flipping between
            # the two made every other run differ.
            keeper = subprocess.Popen(RECORD, env=env, stdout=subprocess.DEVNULL)
            try:
                yield Sink(env)
            finally:
                keeper.terminate()
                keeper.wait(10)
        finally:
            server.terminate()
            server.wait(10)


class Recorder:
    """The sink's monitor, written to a file, with the wall clock of its start."""

    def __init__(self, target: Sink, path: Path) -> None:
        self.path = path
        self.frames = 0
        # When frame 0 was heard: the earliest arrival, less the frames before.
        self.start = float("inf")
        self.process = subprocess.Popen(RECORD, env=target.env, stdout=subprocess.PIPE)
        self.thread = threading.Thread(target=self.pump, daemon=True)
        self.thread.start()

    def flowing(self) -> None:
        """Waits for the first frames, then a moment of them: nothing plays until they do."""
        deadline = time.monotonic() + TIMEOUT
        while self.frames < LEAD_FRAMES:
            if time.monotonic() > deadline or self.process.poll() is not None:
                raise SystemExit("the sink's monitor gives no frames")
            time.sleep(0.01)

    def pump(self) -> None:
        with self.path.open("wb") as out:
            while chunk := self.process.stdout.read1(16384):
                out.write(chunk)
                self.frames += len(chunk) // 4
                self.start = min(self.start, time.time() - self.frames / RATE)

    def stop(self) -> None:
        self.process.terminate()
        self.thread.join(10)
        self.process.wait(10)


class LoopbackRecorder:
    """The same on Windows: the default render endpoint, through WASAPI loopback."""

    def __init__(self, path: Path) -> None:
        import wasapi

        self.loopback = wasapi.Loopback(path)
        form = self.loopback.form
        if form.rate != RATE or form.channels != 2:
            self.loopback.stop()
            raise SystemExit(f"the endpoint runs at {form.rate} Hz with {form.channels} channels, not {RATE} and 2: nothing recorded here could be sample-exact")

    @property
    def start(self) -> float:
        return self.loopback.start

    def flowing(self) -> None:
        """As the sink's monitor: nothing plays until frames come."""
        deadline = time.monotonic() + TIMEOUT
        while self.loopback.frames < LEAD_FRAMES:
            if time.monotonic() > deadline:
                raise SystemExit("the endpoint's loopback gives no frames")
            time.sleep(0.01)

    def stop(self) -> None:
        self.loopback.stop()
        # The system dropped frames while this was reading: not a recording of it all.
        if losses := self.loopback.losses:
            where = "; ".join(str(loss) for loss in losses)
            raise LostFrames(
                f"the loopback lost frames {len(losses)} times ({where}; "
                f"the reader's longest wait {self.loopback.longest_ms:.0f} ms)"
            )


class LostFrames(Exception):
    """A recording the system, or the backend's own output, dropped frames from: no
    evidence of what played."""


# The most recordings of one play that may lose frames before the step fails.
TRIES = 3


def whole(play: Callable[[], T], tries: int = TRIES) -> T:
    """`play`'s answer, played again while its recording lost frames (ticket 2047).

    A recording with a hole in it is never judged: it is made again, up to `tries`
    times, and if each loses frames the step fails, as one did before."""
    for attempt in range(1, tries + 1):
        try:
            return play()
        except LostFrames as lost:
            if attempt == tries:
                raise SystemExit(f"{lost}, in each of {tries} recordings") from lost
            print(f"::warning::{lost}: recording it again", file=sys.stderr, flush=True)
    raise AssertionError("unreachable")


def write_meta(run: Path, revision: str, backend: str, extra: dict[str, object]) -> None:
    """What the gate reads to refuse stale evidence, and how it was made."""
    tools = {}
    for tool in [["pulseaudio", "--version"], ["oggdec", "--version"]]:
        with contextlib.suppress(OSError):
            ran = subprocess.run(tool, capture_output=True, text=True, check=False)
            tools[tool[0]] = (ran.stdout + ran.stderr).strip()
    meta = {
        "revision": revision,
        "backend": backend,
        "corpus_sha256": digest(TOOL / "corpus.ron"),
        "scenarios_sha256": digest(TOOL / "scenarios.ron"),
        "rate": RATE,
        "format": "s16le stereo, the sink's own",
        "environment": {
            **tools,
            **extra,
            "machine": platform.platform(),
            "processors": os.cpu_count(),
            "runner": os.environ.get("ImageOS", "") + " " + os.environ.get("ImageVersion", ""),
        },
    }
    (run / "meta.json").write_text(json.dumps(meta, indent=1) + "\n", encoding="utf-8", newline="\n")


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def scenario_names(backend: str) -> list[str]:
    """The scenarios this backend's probe plays: those with steps that hold on it, or
    that name no backends."""
    text = (TOOL / "scenarios.ron").read_text(encoding="utf-8")
    chosen = []
    for part in re.split(r'(?=name: "\w+")', text)[1:]:
        name = re.match(r'name: "(\w+)"', part).group(1)
        named = re.search(r"backends: Some\(\[(.*?)\]\)", part)
        if named is not None and f'"{backend}"' not in named.group(1):
            continue
        if "steps: []" not in part:
            chosen.append(name)
    return chosen


@dataclass
class Native:
    """Plays with the native probe."""

    probe: Path
    stage: Path
    target: Sink

    def play(self, order: str, record: Path, perf: dict[str, float] | None = None) -> list[str]:
        # A scenario's recording is judged as the sound itself, so one the backend
        # underran in is made again (ticket 2048). A timing's gaps are what it counts.
        if order.startswith("scenario "):
            return whole(lambda: self.play_once(order, record, perf, refuse=True))
        return self.play_once(order, record, perf, refuse=False)

    def play_once(self, order: str, record: Path, perf: dict[str, float] | None, refuse: bool) -> list[str]:
        # Nothing of a refused recording's run is kept.
        if perf is not None:
            perf.clear()
        (self.stage / "probe.txt").write_text(order, encoding="utf-8", newline="\n")
        recorder = Recorder(self.target, record)
        recorder.flowing()
        process = subprocess.run(
            [str(self.probe)],
            cwd=self.stage,
            env=self.target.env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=TIMEOUT,
            check=False,
        )
        time.sleep(0.3)
        recorder.stop()
        # quad-snd says on its stdout when ALSA ran dry: the sink got a hole, not the sound.
        underruns = [line for line in process.stdout.splitlines() if line.startswith(UNDERRUN)]
        if refuse and underruns:
            raise LostFrames(f"quad-snd underran {len(underruns)} times ({'; '.join(underruns)})")
        sys.stdout.write(process.stdout)
        lines = [line[len(PREFIX):] for line in process.stderr.splitlines() if line.startswith(PREFIX)]
        if process.returncode != 0:
            lines.append(json.dumps({"event": "failed", "text": f"exit {process.returncode}: {process.stderr[-300:]}"}))
        if perf is not None:
            perf["_start"] = recorder.start
        return lines


def process_cycles(handle: int) -> int:
    """What a Windows process's threads have run for, in cycles: process CPU time is
    charged at clock ticks and misreads a process that sleeps between its work."""
    cycles = ctypes.c_uint64()
    ctypes.windll.kernel32.QueryProcessCycleTime(ctypes.c_void_p(handle), ctypes.byref(cycles))
    return cycles.value


def process_peak_kib(handle: int) -> float:
    """A Windows process's peak working set, in KiB."""

    class Counters(ctypes.Structure):
        _fields_ = [("cb", ctypes.c_uint32), ("faults", ctypes.c_uint32), ("peak", ctypes.c_size_t), ("rest", ctypes.c_size_t * 7)]

    counters = Counters()
    counters.cb = ctypes.sizeof(Counters)
    ctypes.windll.kernel32.K32GetProcessMemoryInfo(ctypes.c_void_p(handle), ctypes.byref(counters), counters.cb)
    return counters.peak / 1024


@dataclass
class Windows:
    """Plays with the native probe on Windows: the recorder is the loopback, and the
    process's CPU and memory are read from here, as the browser's are, since the
    probe has no way to read them on this system. `cpu_ms` counts millions of cycles:
    only its ratios are held, and a runner's clock isn't a fixed number."""

    probe: Path
    stage: Path

    def play(self, order: str, record: Path, perf: dict[str, float] | None = None) -> list[str]:
        return whole(lambda: self.play_once(order, record, perf))

    def play_once(self, order: str, record: Path, perf: dict[str, float] | None) -> list[str]:
        # Nothing of a refused recording's run is kept.
        if perf is not None:
            perf.clear()
        (self.stage / "probe.txt").write_text(order, encoding="utf-8", newline="\n")
        recorder = LoopbackRecorder(record)
        recorder.flowing()
        process = subprocess.Popen([str(self.probe)], cwd=self.stage, stderr=subprocess.PIPE, text=True)
        timer = threading.Timer(TIMEOUT, process.kill)
        timer.start()
        handle = int(process._handle)
        lines = []
        tail = ""
        for raw in process.stderr:
            tail = (tail + raw)[-300:]
            if not raw.startswith(PREFIX):
                continue
            lines.append(raw[len(PREFIX):].rstrip())
            if perf is not None and '"playing"' in raw:
                perf["_cycles"] = process_cycles(handle)
            if perf is not None and '"played"' in raw:
                perf["cpu_ms"] = (process_cycles(handle) - perf["_cycles"]) / 1e6
                perf["peak_kib"] = process_peak_kib(handle)
        process.wait()
        timer.cancel()
        time.sleep(0.3)
        recorder.stop()
        if process.returncode != 0:
            lines.append(json.dumps({"event": "failed", "text": f"exit {process.returncode}: {tail}"}))
        if perf is not None:
            perf["_start"] = recorder.start
        return lines


def chrome_processes() -> list[int]:
    """The browser's processes, by their name and being below this one."""
    parents: dict[int, int] = {}
    names: dict[int, str] = {}
    for stat in Path("/proc").glob("[0-9]*/stat"):
        with contextlib.suppress(OSError, IndexError):
            text = stat.read_text(encoding="utf-8", errors="replace")
            pid = int(stat.parent.name)
            names[pid] = text[text.index("(") + 1 : text.rindex(")")]
            parents[pid] = int(text[text.rindex(")") + 2 :].split()[1])

    def below(pid: int) -> bool:
        while pid in parents:
            if pid == os.getpid():
                return True
            pid = parents[pid]
        return False

    return [pid for pid, name in names.items() if "chrome" in name and below(pid)]


def chrome_cpu_ms() -> float:
    """What the browser's processes have run for: every thread of each."""
    total = 0
    for pid in chrome_processes():
        with contextlib.suppress(OSError, IndexError, ValueError):
            text = Path(f"/proc/{pid}/stat").read_text(encoding="utf-8", errors="replace")
            fields = text[text.rindex(")") + 2 :].split()
            total += int(fields[11]) + int(fields[12])
    return total * 1000 / os.sysconf("SC_CLK_TCK")


def chrome_kib() -> float:
    total = 0.0
    for pid in chrome_processes():
        with contextlib.suppress(OSError, IndexError, ValueError, StopIteration):
            status = Path(f"/proc/{pid}/status").read_text(encoding="utf-8")
            total += float(next(l for l in status.splitlines() if l.startswith("VmRSS:")).split()[1])
    return total


@dataclass
class Browser:
    """Plays with the web build in a real Chromium on the real loader."""

    stage: Path
    target: Sink
    browser: object = None
    server: object = None
    playwright: object = None
    version: str = field(default="")

    def open(self, beside: Browser | None = None) -> None:
        """Serves its page, in a browser of its own, or in `beside`'s: Playwright's
        API starts once in a thread."""
        sys.path.insert(0, str(TOOL.parent / "web-check"))
        from check import serve
        from playwright.sync_api import sync_playwright

        self.server = serve(self.stage)
        if beside is not None:
            self.browser = beside.browser
            self.version = beside.version
            return
        self.playwright = sync_playwright().start()
        self.browser = self.playwright.chromium.launch(
            channel="chromium",
            args=CHROMIUM_ARGS,
            # Playwright mutes it; without this nothing reaches the sink.
            ignore_default_args=["--mute-audio"],
            env=self.target.env,
        )
        self.version = self.browser.version

    def close(self) -> None:
        self.server.shutdown()
        if self.playwright is not None:
            self.browser.close()
            self.playwright.stop()

    def play(self, order: str, record: Path, perf: dict[str, float] | None = None) -> list[str]:
        (self.stage / "probe.txt").write_text(order, encoding="utf-8", newline="\n")
        lines: list[str] = []
        recorder = Recorder(self.target, record)
        recorder.flowing()
        page = self.browser.new_page(viewport={"width": 320, "height": 240})

        def on_console(message: object) -> None:
            if PREFIX not in message.text:
                return
            line = message.text[message.text.index(PREFIX) + len(PREFIX) :]
            lines.append(line)
            if perf is not None and '"playing"' in line:
                perf["_cpu"] = chrome_cpu_ms()
            if perf is not None and '"played"' in line:
                perf["cpu_ms"] = chrome_cpu_ms() - perf["_cpu"]
                perf["peak_kib"] = chrome_kib()

        page.on("console", on_console)
        page.on("pageerror", lambda error: lines.append(json.dumps({"event": "failed", "text": str(error)[:200]})))
        host, port = self.server.server_address[:2]
        page.goto(f"http://{host}:{port}/index.html", wait_until="load")
        clicked = False
        deadline = time.monotonic() + TIMEOUT
        while time.monotonic() < deadline:
            if not clicked and any('"ready"' in line for line in lines):
                # The gesture the browser asks for, held across a frame.
                page.mouse.move(100, 100)
                page.mouse.down()
                page.wait_for_timeout(100)
                page.mouse.up()
                clicked = True
            if any('"done"' in line for line in lines):
                break
            page.wait_for_timeout(10)
        page.wait_for_timeout(300)
        page.close()
        recorder.stop()
        if not any('"done"' in line for line in lines):
            lines.append(json.dumps({"event": "failed", "text": f"no done in {TIMEOUT} seconds"}))
        if perf is not None:
            perf["_start"] = recorder.start
        return lines


def write_run(player: Native | Windows | Browser, run: Path, backend: str) -> None:
    """Every scenario once, after one thrown away: a cold process isn't what is measured."""
    run.mkdir(parents=True, exist_ok=True)
    player.play("scenario onset\n", run / "warmup.pcm")
    (run / "warmup.pcm").unlink()
    for name in scenario_names(backend):
        perf: dict[str, float] = {}
        lines = player.play(f"scenario {name}\n", run / f"{name}.pcm", perf)
        # When the recording's first frame was heard: the wall clock of its steps'.
        lines.append(json.dumps({"event": "recording", "start": perf["_start"]}))
        (run / f"{name}.jsonl").write_text("".join(f"{line}\n" for line in lines), encoding="utf-8", newline="\n")


def sound_facts(pcm: Path, start: float, step_wall: float | None) -> dict[str, float]:
    """The latency and the gaps a run's recording shows."""
    samples = array.array("h")
    samples.frombytes(pcm.read_bytes()[: pcm.stat().st_size // 2 * 2])
    if sys.byteorder == "big":
        samples.byteswap()
    onset = next((i // 2 for i, s in enumerate(samples) if abs(s) > 64), None)
    facts: dict[str, float] = {}
    if onset is None:
        return {"underruns": 1.0}
    if step_wall is not None:
        facts["latency_ms"] = (start + onset / RATE - step_wall) * 1000
    # From after the marker and its gap: everything plays until the end.
    begin = onset + int(0.6 * RATE)
    end = begin + int(2.5 * RATE)
    gaps = 0
    quiet = 0
    for frame in range(begin, min(end, len(samples) // 2)):
        if abs(samples[frame * 2]) <= QUIET and abs(samples[frame * 2 + 1]) <= QUIET:
            quiet += 1
            gaps += quiet == GAP_FRAMES
        else:
            quiet = 0
    facts["underruns"] = float(gaps)
    return facts


def one_perf(player: Native | Windows | Browser, base: Path, side: str, pair: int, burn: int) -> dict[str, object]:
    perf: dict[str, float] = {}
    # The last of each side stays, to look at.
    record = base / f"perf-{side}.pcm"
    lines = player.play(f"perf {burn}\n", record, perf)
    metrics: dict[str, float] = {}
    wall = None
    for line in lines:
        event = json.loads(line)
        if event["event"] == "perf":
            metrics[event["metric"]] = event["value"]
        if event["event"] == "step":
            wall = event["wall"]
        if event["event"] == "failed":
            raise SystemExit(f"the perf run failed: {line}")
    metrics.update({k: v for k, v in perf.items() if not k.startswith("_")})
    metrics.update(sound_facts(record, perf["_start"], wall))
    return {"side": side, "pair": pair, "metrics": metrics}


def write_perf(player: Native | Windows | Browser, base: Path) -> None:
    """Warm up, then pairs in turn, a, b, b, a: drift falls on neither, and neither
    side lines up with every other run. The slowed runs come after, so they can't."""
    for _ in range(WARMUP):
        one_perf(player, base, "warmup", -1, 0)
    with (base / "perf.jsonl").open("w", encoding="utf-8", newline="\n") as out:
        for pair in range(PAIRS):
            for side in ("a", "b") if pair % 2 == 0 else ("b", "a"):
                out.write(json.dumps(one_perf(player, base, side, pair, 0)) + "\n")
                out.flush()
        for pair in range(PAIRS):
            out.write(json.dumps(one_perf(player, base, "burn", pair, BURN)) + "\n")
            out.flush()


def write_old_new(old: Native | Windows | Browser, new: Native | Windows | Browser, base: Path) -> None:
    """The old backend against the new, the same workload in turn, a, b, b, a."""
    for _ in range(WARMUP):
        one_perf(new, base, "warmup", -1, 0)
    with (base / "old-new.jsonl").open("w", encoding="utf-8", newline="\n") as out:
        for pair in range(PAIRS):
            sides = (("old", old), ("new", new))
            for side, player in sides if pair % 2 == 0 else reversed(sides):
                out.write(json.dumps(one_perf(player, base, side, pair, 0)) + "\n")
                out.flush()


def write_decoders(player: Native | Windows | Browser, base: Path) -> None:
    """The old path's whole decodes against `shuyi-sound`'s, a process per pair, each
    swapping which side goes first. The recording says nothing here."""
    record = base / "decoders.pcm"
    for pair in range(WARMUP):
        player.play(f"decoders {pair}\n", record)
    with (base / "decoders.jsonl").open("w", encoding="utf-8", newline="\n") as out:
        for pair in range(PAIRS):
            for line in player.play(f"decoders {pair}\n", record):
                event = json.loads(line)
                if event["event"] == "failed":
                    raise SystemExit(f"the decoder run failed: {line}")
                if event["event"] == "decode":
                    out.write(line + "\n")
                    out.flush()
    record.unlink()


def write_mixer(player: Native | Windows | Browser, base: Path) -> None:
    """The mixer's offline render, a process per pair: nothing old to compare with,
    so the gate reads its share of the sound's length. The recording says nothing."""
    record = base / "mixer.pcm"
    for pair in range(WARMUP):
        player.play(f"mixer {pair}\n", record)
    with (base / "mixer.jsonl").open("w", encoding="utf-8", newline="\n") as out:
        for pair in range(PAIRS):
            for line in player.play(f"mixer {pair}\n", record):
                event = json.loads(line)
                if event["event"] == "failed":
                    raise SystemExit(f"the mixer run failed: {line}")
                if event["event"] == "mix":
                    out.write(line + "\n")
                    out.flush()
    record.unlink()


def native(args: argparse.Namespace) -> int:
    base = args.dir / "linux-native"
    if not args.perf_only:
        shutil.rmtree(base, ignore_errors=True)
    base.mkdir(parents=True, exist_ok=True)
    shutil.copy(TOOL / "scenarios.ron", args.dir / "scenarios.ron")
    with sink() as target:
        player = Native(args.probe.resolve(), args.dir.resolve(), target)
        for number in range(0 if args.perf_only else RUNS):
            run = base / f"run-{number}"
            write_run(player, run, "quad-snd")
            write_meta(run, args.revision, "quad-snd", {"probe_bytes": args.probe.stat().st_size, "build_seconds": args.build_seconds})
        write_perf(player, base)
        write_decoders(player, base)
        write_mixer(player, base)
    return 0


def write_device_loss(player: Windows, base: Path) -> None:
    """Takes the render endpoint away while the probe plays, gives it back, and writes
    how long the sound takes to be heard from when an endpoint is there again: what a
    player has of unplugging headphones, on a machine with one device. Recording stops
    with the endpoint, so it starts again. Whatever goes wrong is in the report, which
    the gate fails: evidence missing is never a pass."""
    import wasapi

    def powershell(command: str) -> str:
        ran = subprocess.run(["pwsh", "-NoProfile", "-Command", command], capture_output=True, text=True, check=False)
        if ran.returncode != 0:
            raise RuntimeError(f"{command}: {ran.stderr.strip()[-300:]}")
        return ran.stdout.strip()

    (player.stage / "probe.txt").write_text("scenario device_loss\n", encoding="utf-8", newline="\n")
    report: dict[str, object] = {}
    process = subprocess.Popen([str(player.probe)], cwd=player.stage, stderr=subprocess.DEVNULL)
    try:
        endpoint = powershell("(Get-PnpDevice -Class AudioEndpoint -Status OK | Select-Object -First 1).InstanceId")
        report["endpoint"] = endpoint
        time.sleep(2.5)
        powershell(f"Disable-PnpDevice -InstanceId '{endpoint}' -Confirm:$false")
        time.sleep(1.5)
        report["still_running"] = process.poll() is None
        powershell(f"Enable-PnpDevice -InstanceId '{endpoint}' -Confirm:$false")
        deadline = time.monotonic() + 20
        while True:
            with contextlib.suppress(OSError):
                wasapi.default_endpoint()
                break
            if time.monotonic() > deadline:
                raise RuntimeError("no render endpoint came back")
            time.sleep(0.02)
        back = time.monotonic()
        recording = wasapi.Loopback(base / "device-loss.pcm")
        heard = None
        while heard is None and time.monotonic() < back + 5:
            time.sleep(0.01)
            if any(any(chunk) for chunk in list(recording.chunks)):
                heard = time.monotonic() - back
        recording.stop()
        report["heard_ms"] = None if heard is None else heard * 1000
    except (OSError, RuntimeError, SystemExit) as error:
        report["error"] = str(error)
    finally:
        process.kill()
        process.wait()
    (base / "device-loss.json").write_text(json.dumps(report) + "\n", encoding="utf-8", newline="\n")


def windows(args: argparse.Namespace) -> int:
    """The same as `native`, on Windows' default render endpoint. It may set that
    endpoint's format (`--set-endpoint`, on a runner's virtual device), and otherwise
    refuses one that isn't the sink's: nothing else on a person's machine is touched."""
    sys.path.insert(0, str(TOOL))
    import wasapi

    shutil.copy(TOOL / "scenarios.ron", args.dir / "scenarios.ron")
    wasapi.coinit()
    if args.set_endpoint:
        wasapi.set_endpoint_rate(RATE)
    keeper = wasapi.Endpoint()
    try:
        old = Windows(args.probe.resolve(), args.dir.resolve())
        new = Windows(args.new_probe.resolve(), args.dir.resolve()) if args.new_probe else None
        if args.only in (None, "windows"):
            base = args.dir / "windows"
            if not args.perf_only:
                shutil.rmtree(base, ignore_errors=True)
            base.mkdir(parents=True, exist_ok=True)
            for number in range(0 if args.perf_only else RUNS):
                run = base / f"run-{number}"
                write_run(old, run, "quad-snd")
                write_meta(run, args.revision, "quad-snd", {"probe_bytes": args.probe.stat().st_size, "build_seconds": args.build_seconds})
            write_perf(old, base)
            write_decoders(old, base)
            write_mixer(old, base)
        if new is not None and args.only in (None, "windows-in-house"):
            base = args.dir / "windows-in-house"
            if not args.perf_only:
                shutil.rmtree(base, ignore_errors=True)
            base.mkdir(parents=True, exist_ok=True)
            for number in range(0 if args.perf_only else RUNS):
                run = base / f"run-{number}"
                write_run(new, run, "in-house")
                write_meta(run, args.revision, "in-house", {"probe_bytes": args.new_probe.stat().st_size, "build_seconds": args.build_seconds})
            write_perf(new, base)
            write_old_new(old, new, base)
    finally:
        keeper.close()
    if new is not None and args.only is None and args.take_device:
        write_device_loss(new, args.dir / "windows-in-house")
    return 0


def browser(args: argparse.Namespace) -> int:
    def staged(page: Path, name: str) -> Path:
        stage = args.dir / name
        shutil.rmtree(stage, ignore_errors=True)
        shutil.copytree(page, stage)
        shutil.copytree(args.dir / "corpus", stage / "corpus")
        shutil.copy(TOOL / "scenarios.ron", stage / "scenarios.ron")
        return stage

    def recorded(name: str) -> Path:
        base = args.dir / name
        if not args.perf_only:
            shutil.rmtree(base, ignore_errors=True)
        base.mkdir(parents=True, exist_ok=True)
        return base

    stage = staged(args.page, "page")
    wasm = next(stage.glob("*.wasm"))
    with sink() as target:
        old = Browser(stage, target)
        old.open()
        new = None
        try:
            if args.only in (None, "browser-chromium"):
                base = recorded("browser-chromium")
                for number in range(0 if args.perf_only else RUNS):
                    run = base / f"run-{number}"
                    write_run(old, run, "quad-snd-web")
                    write_meta(run, args.revision, "quad-snd-web", {"probe_bytes": wasm.stat().st_size, "build_seconds": args.build_seconds, "chromium": old.version})
                write_perf(old, base)
                write_decoders(old, base)
                write_mixer(old, base)
            if args.new_page and args.only in (None, "browser-in-house"):
                new = Browser(staged(args.new_page, "page-new"), target)
                new.open(old)
                new_wasm = next(new.stage.glob("*.wasm"))
                base = recorded("browser-in-house")
                for number in range(0 if args.perf_only else RUNS):
                    run = base / f"run-{number}"
                    write_run(new, run, "in-house-web")
                    write_meta(run, args.revision, "in-house-web", {"probe_bytes": new_wasm.stat().st_size, "build_seconds": args.build_seconds, "chromium": new.version})
                write_perf(new, base)
                write_old_new(old, new, base)
        finally:
            if new is not None:
                new.close()
            old.close()
    return 0


def encode(args: argparse.Namespace) -> int:
    """Makes the Ogg fixtures from the WAVs and writes `corpus.ron`: by hand, never in CI."""
    corpus = args.dir / "corpus"
    out = TOOL / "corpus"
    out.mkdir(exist_ok=True)
    def version(tool: str) -> str:
        ran = subprocess.run([tool, "--version"], capture_output=True, text=True, check=True)
        return (ran.stdout + ran.stderr).strip()

    wavs = sorted(corpus.glob("*.wav"))
    lines = ["// What pins the sound gates' corpus (ticket 2573): written by `capture.py encode`.", "("]
    lines += ["    wav: {"] + [f'        "{wav.name}": "{digest(wav)}",' for wav in wavs] + ["    },", "    ogg: {"]
    for name, comments in OGGS.items():
        ogg = out / f"{name}.ogg"
        tags = [arg for comment in comments for arg in ("--comment", comment)]
        subprocess.run(["oggenc", *OGGENC, *tags, "--output", str(ogg), str(corpus / f"{name}.wav")], check=True)
        decoded = subprocess.run(["oggdec", "--quiet", "--raw", "--bits=16", "--endianness=0", "--sign=1", "--output", "-", str(ogg)], capture_output=True, check=True).stdout
        lines.append(f'        "{ogg.name}": (sha256: "{digest(ogg)}", decoded_sha256: "{hashlib.sha256(decoded).hexdigest()}"),')
    lines += ["    },", "    // The source against the lossy file: not decoded equality, which the", "    // hashes above pin."]
    lines += ["    lossy_min_snr_db: 15.0,", f'    encoder: "{version("oggenc")} {" ".join(OGGENC)}",', f'    decoder: "{version("oggdec")}",', ")"]
    (TOOL / "corpus.ron").write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("mode", choices=["native", "windows", "browser", "encode"])
    parser.add_argument("--dir", type=Path, default=Path("target/sound-parity"))
    parser.add_argument("--probe", type=Path, help="the native probe")
    parser.add_argument("--page", type=Path, help="the web page's folder, with the probe's wasm")
    parser.add_argument("--revision", default="")
    parser.add_argument("--build-seconds", type=float, default=0.0)
    parser.add_argument("--perf-only", action="store_true", help="only the timings, again")
    parser.add_argument("--set-endpoint", action="store_true", help="on a runner: set the virtual endpoint's format first")
    parser.add_argument("--new-probe", type=Path, help="windows: the probe of the in-house backend, recorded too")
    parser.add_argument("--new-page", type=Path, help="browser: the page of the in-house backend, recorded too")
    parser.add_argument("--take-device", action="store_true", help="windows, on a runner: take the render endpoint away and give it back while the new backend plays")
    parser.add_argument("--only", help="the one platform of this mode to record, when it is measured again")
    args = parser.parse_args()
    return {"native": native, "windows": windows, "browser": browser, "encode": encode}[args.mode](args)


if __name__ == "__main__":
    sys.exit(main())

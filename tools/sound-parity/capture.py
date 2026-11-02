#!/usr/bin/env python3
"""Records what the sound backend really emits (ticket 2573, ADR-0066).

`README.md` beside it says how to run it; CI's `sound-parity` job does. On a
PulseAudio null sink of its own, in a temporary home, so nothing else is heard
and nothing is heard elsewhere, it records the sink's monitor while the probe
(`crates/app/examples/sound_probe.rs`) plays:

    capture.py native --dir D --probe BIN --revision SHA
    capture.py browser --dir D --page DIR --revision SHA
    (either with --perf-only: the timings again, the recordings kept)
    capture.py encode --dir D       (by hand, on Linux) the Ogg fixtures

Natively ALSA's default device is routed to the sink, as a player's system
would. In the browser, the page that `shuyi web` writes runs in Chromium with
Playwright's mute flag taken away and a real click to unlock it. Each writes
`D/<platform>/run-N/<scenario>.pcm` and `.jsonl`, `meta.json`, and the paired
timings in `D/<platform>/perf.jsonl`. `cargo xtask sound-parity gate` judges
them: this script records, and decides nothing.
"""

from __future__ import annotations

import argparse
import array
import contextlib
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
from collections.abc import Iterator
from dataclasses import dataclass, field
from pathlib import Path

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


def write_meta(run: Path, revision: str, backend: str, extra: dict[str, object]) -> None:
    """What the gate reads to refuse stale evidence, and how it was made."""
    tools = {}
    for tool in [["pulseaudio", "--version"], ["oggdec", "--version"]]:
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


def scenario_names() -> list[str]:
    """The scenarios the probe can play: those with steps."""
    text = (TOOL / "scenarios.ron").read_text(encoding="utf-8")
    names = re.findall(r'name: "(\w+)"', text)
    return [name for name in names if not name.startswith("future_")]


@dataclass
class Native:
    """Plays with the native probe."""

    probe: Path
    stage: Path
    target: Sink

    def play(self, order: str, record: Path, perf: dict[str, float] | None = None) -> list[str]:
        (self.stage / "probe.txt").write_text(order, encoding="utf-8", newline="\n")
        recorder = Recorder(self.target, record)
        recorder.flowing()
        process = subprocess.run(
            [str(self.probe)], cwd=self.stage, env=self.target.env, stderr=subprocess.PIPE, text=True, timeout=TIMEOUT, check=False
        )
        time.sleep(0.3)
        recorder.stop()
        lines = [line[len(PREFIX):] for line in process.stderr.splitlines() if line.startswith(PREFIX)]
        if process.returncode != 0:
            lines.append(json.dumps({"event": "failed", "text": f"exit {process.returncode}: {process.stderr[-300:]}"}))
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

    def open(self) -> None:
        sys.path.insert(0, str(TOOL.parent / "web-check"))
        from check import serve
        from playwright.sync_api import sync_playwright

        self.playwright = sync_playwright().start()
        self.server = serve(self.stage)
        self.browser = self.playwright.chromium.launch(
            channel="chromium",
            args=CHROMIUM_ARGS,
            # Playwright mutes it; without this nothing reaches the sink.
            ignore_default_args=["--mute-audio"],
            env=self.target.env,
        )
        self.version = self.browser.version

    def close(self) -> None:
        self.browser.close()
        self.server.shutdown()
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


def write_run(player: Native | Browser, run: Path) -> None:
    """Every scenario once, after one thrown away: a cold process isn't what is measured."""
    run.mkdir(parents=True, exist_ok=True)
    player.play("scenario onset\n", run / "warmup.pcm")
    (run / "warmup.pcm").unlink()
    for name in scenario_names():
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


def one_perf(player: Native | Browser, base: Path, side: str, pair: int, burn: int) -> dict[str, object]:
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


def write_perf(player: Native | Browser, base: Path) -> None:
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


def write_decoders(player: Native | Browser, base: Path) -> None:
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
            write_run(player, run)
            write_meta(run, args.revision, "quad-snd", {"probe_bytes": args.probe.stat().st_size, "build_seconds": args.build_seconds})
        write_perf(player, base)
        write_decoders(player, base)
    return 0


def browser(args: argparse.Namespace) -> int:
    base = args.dir / "browser-chromium"
    if not args.perf_only:
        shutil.rmtree(base, ignore_errors=True)
    base.mkdir(parents=True, exist_ok=True)
    stage = args.dir / "page"
    shutil.rmtree(stage, ignore_errors=True)
    shutil.copytree(args.page, stage)
    shutil.copytree(args.dir / "corpus", stage / "corpus")
    shutil.copy(TOOL / "scenarios.ron", stage / "scenarios.ron")
    wasm = next(stage.glob("*.wasm"))
    with sink() as target:
        player = Browser(stage, target)
        player.open()
        try:
            for number in range(0 if args.perf_only else RUNS):
                run = base / f"run-{number}"
                write_run(player, run)
                write_meta(run, args.revision, "quad-snd-web", {"probe_bytes": wasm.stat().st_size, "build_seconds": args.build_seconds, "chromium": player.version})
            write_perf(player, base)
            write_decoders(player, base)
        finally:
            player.close()
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
    parser.add_argument("mode", choices=["native", "browser", "encode"])
    parser.add_argument("--dir", type=Path, default=Path("target/sound-parity"))
    parser.add_argument("--probe", type=Path, help="the native probe")
    parser.add_argument("--page", type=Path, help="the web page's folder, with the probe's wasm")
    parser.add_argument("--revision", default="")
    parser.add_argument("--build-seconds", type=float, default=0.0)
    parser.add_argument("--perf-only", action="store_true", help="only the timings, again")
    args = parser.parse_args()
    return {"native": native, "browser": browser, "encode": encode}[args.mode](args)


if __name__ == "__main__":
    sys.exit(main())

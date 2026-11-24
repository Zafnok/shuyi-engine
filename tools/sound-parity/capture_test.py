#!/usr/bin/env python3
"""`capture.py`'s choice to record a play again (tickets 2047 and 2048), run in CI's jobs."""

from __future__ import annotations

import contextlib
import io
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

from capture import PREFIX, LoopbackRecorder, LostFrames, Native, read_probe, stalls_said, whole


def plays(*outcomes: object):
    """A play that answers each outcome in turn, raising those that are errors."""
    left = list(outcomes)
    played = []

    def play() -> object:
        played.append(None)
        outcome = left.pop(0)
        if isinstance(outcome, BaseException):
            raise outcome
        return outcome

    return play, played


class Whole(unittest.TestCase):
    def test_a_whole_recording_is_played_once(self):
        play, played = plays("lines")
        self.assertEqual(whole(play), "lines")
        self.assertEqual(len(played), 1)

    def test_a_recording_that_lost_frames_is_made_again_and_only_the_whole_one_answers(self):
        play, played = plays(LostFrames("lost"), "lines")
        with contextlib.redirect_stderr(io.StringIO()) as said:
            self.assertEqual(whole(play), "lines")
        self.assertEqual(len(played), 2)
        self.assertIn("::warning::lost: recording it again", said.getvalue())

    def test_each_recording_losing_frames_fails_the_step(self):
        play, played = plays(LostFrames("lost"), LostFrames("lost"), LostFrames("lost"))
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as failed:
            whole(play, tries=3)
        self.assertEqual(len(played), 3)
        self.assertIn("in each of 3 recordings", str(failed.exception))

    def test_any_other_failure_isnt_played_again(self):
        play, played = plays(SystemExit("no frames"), "lines")
        with self.assertRaises(SystemExit):
            whole(play)
        self.assertEqual(len(played), 1)


class NativeUnderrun(unittest.TestCase):
    """quad-snd prints its ALSA underruns on stdout; a scenario played through one is made again."""

    def player(self, *stdouts: str) -> tuple[Native, list[None]]:
        left = list(stdouts)
        ran = []

        def run(*_args: object, **_kwargs: object) -> SimpleNamespace:
            ran.append(None)
            return SimpleNamespace(returncode=0, stdout=left.pop(0), stderr=f"{PREFIX}{{\"event\":\"step\"}}\n")

        recorder = SimpleNamespace(flowing=lambda: None, stop=lambda: None, start=1.0)
        self.enterContext(mock.patch("capture.subprocess.run", run))
        self.enterContext(mock.patch("capture.Recorder", lambda *_: recorder))
        self.enterContext(mock.patch("capture.time.sleep", lambda _: None))
        stage = Path(self.enterContext(tempfile.TemporaryDirectory()))
        return Native(Path("probe"), stage, SimpleNamespace(env={})), ran

    UNDERRAN = "Underrun occured: -EPIPE, attempting recover\n"

    def test_a_scenario_quad_snd_underran_in_is_recorded_again_and_only_the_whole_one_answers(self):
        native, ran = self.player(self.UNDERRAN, "")
        perf: dict[str, float] = {}
        with contextlib.redirect_stderr(io.StringIO()) as said:
            lines = native.play("scenario wav_48k\n", Path("wav_48k.pcm"), perf)
        self.assertEqual(len(ran), 2)
        self.assertEqual(lines, ['{"event":"step"}'])
        self.assertEqual(perf, {"_start": 1.0})
        self.assertIn("::warning::quad-snd underran 1 times (Underrun occured: -EPIPE", said.getvalue())

    def test_three_scenarios_in_a_row_underrunning_fail_the_step(self):
        native, ran = self.player(self.UNDERRAN, self.UNDERRAN, self.UNDERRAN)
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as failed:
            native.play("scenario wav_48k\n", Path("wav_48k.pcm"), {})
        self.assertEqual(len(ran), 3)
        self.assertIn("in each of 3 recordings", str(failed.exception))

    def test_a_timing_counts_its_own_gaps_and_isnt_played_again(self):
        native, ran = self.player(self.UNDERRAN)
        with contextlib.redirect_stdout(io.StringIO()) as echoed:
            native.play("perf 0\n", Path("perf-a.pcm"), {})
        self.assertEqual(len(ran), 1)
        self.assertEqual(echoed.getvalue(), self.UNDERRAN)


class Stop(unittest.TestCase):
    def recorder(self, losses: list[str], silences=()) -> LoopbackRecorder:
        recorder = LoopbackRecorder.__new__(LoopbackRecorder)
        recorder.loopback = SimpleNamespace(losses=losses, longest_ms=12.0, stop=lambda: None, path=Path("loop.pcm"), silences=list(silences))
        return recorder

    def test_a_recording_with_a_loss_is_refused_and_says_where(self):
        with self.assertRaises(LostFrames) as refused:
            self.recorder(["441 frames after frame 9000"]).stop()
        self.assertIn("lost frames 1 times (441 frames after frame 9000", str(refused.exception))

    def test_a_recording_without_one_is_kept(self):
        self.recorder([]).stop()

    def test_flagged_packets_are_said_in_the_log_joined(self):
        with contextlib.redirect_stderr(io.StringIO()) as said:
            self.recorder([], ((0, 441), (441, 441), (9000, 441))).stop()
        self.assertIn("loop.pcm: the loopback flagged silent 882 frames at 0, 441 frames at 9000", said.getvalue())


class Streams(unittest.TestCase):
    """What the in-house feeder said of its tracks becomes a timing's metrics (ticket 3320)."""

    LINES = [
        '{"event":"step","at":0,"wall":12.5,"cue":"","secs":0}',
        '{"event":"perf","metric":"cpu_ms","value":40.0}',
        '{"event":"output","rebuilds":0,"silent_callbacks":0,"dry_callbacks":0,"raised":true}',
        '{"event":"streams","raised":true,"tracks":[{"track":"music_a.ogg","stalled":0,"lowest":2816},'
        '{"track":"music_b.ogg","stalled":512,"lowest":0},{"track":"short.wav","stalled":0,"lowest":null}]}',
    ]

    def test_the_stalls_are_summed_and_the_lowest_margin_kept(self):
        metrics, wall, tracks = read_probe(self.LINES)
        self.assertEqual(wall, 12.5)
        self.assertEqual(
            metrics,
            {"cpu_ms": 40.0, "dry_callbacks": 0.0, "stalled_frames": 512.0, "lowest_margin": 0.0},
        )
        self.assertEqual([track["track"] for track in tracks], ["music_a.ogg", "music_b.ogg", "short.wav"])
        self.assertEqual(stalls_said(tracks), "the feeder fell behind: music_b.ogg stalled 512 frames")

    def test_a_backend_that_doesnt_count_says_nothing_of_streams(self):
        metrics, _, tracks = read_probe(self.LINES[:3])
        self.assertIsNone(tracks)
        self.assertNotIn("stalled_frames", metrics)
        self.assertEqual(stalls_said(tracks), "")
        metrics, _, tracks = read_probe(['{"event":"streams","raised":false,"tracks":[]}'])
        self.assertEqual(metrics, {"stalled_frames": 0.0}, "no margin to say, and no stall")
        self.assertEqual(stalls_said(tracks), "")

    def test_a_failed_run_is_an_error(self):
        with self.assertRaises(ValueError):
            read_probe(['{"event":"failed","text":"no scenario"}'])


if __name__ == "__main__":
    unittest.main()

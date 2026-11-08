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

from capture import PREFIX, LoopbackRecorder, LostFrames, Native, whole


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
    def recorder(self, losses: list[str]) -> LoopbackRecorder:
        recorder = LoopbackRecorder.__new__(LoopbackRecorder)
        recorder.loopback = SimpleNamespace(losses=losses, longest_ms=12.0, stop=lambda: None, path=Path("x.pcm"))
        return recorder

    def test_a_recording_with_a_loss_is_refused_and_says_where(self):
        with self.assertRaises(LostFrames) as refused:
            self.recorder(["441 frames after frame 9000"]).stop()
        self.assertIn("lost frames 1 times (441 frames after frame 9000", str(refused.exception))

    def test_a_recording_without_one_is_kept(self):
        self.recorder([]).stop()


if __name__ == "__main__":
    unittest.main()

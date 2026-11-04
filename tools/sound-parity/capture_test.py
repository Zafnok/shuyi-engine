#!/usr/bin/env python3
"""`capture.py`'s choice to record a play again (ticket 2047), run in CI's job."""

from __future__ import annotations

import contextlib
import io
import unittest
from pathlib import Path
from types import SimpleNamespace

from capture import LoopbackRecorder, LostFrames, whole


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

#!/usr/bin/env python3
"""`capture.py`'s choice to record a play again (tickets 2047, 2048 and 2049), run in CI's jobs."""

from __future__ import annotations

import contextlib
import io
import struct
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

from capture import GAP_FRAMES, PREFIX, LoopbackRecorder, LostFrames, Native, Windows, whole, zero_holes


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


def recording(*parts: tuple[int, int]) -> bytes:
    """Stereo 16-bit frames: each part is (frames, value), the value on both channels."""
    return b"".join(struct.pack("<hh", value, value) * frames for frames, value in parts)


# 2048's hole: one 10 ms packet of zeros in loop's ramp, after the marker's 500 ms gap.
HOLED = recording((14680, 0), (441, 900), (22050, 0), (21609, 1200), (441, 0), (30000, 1200), (8820, 0))


class ZeroHoles(unittest.TestCase):
    def test_a_packet_of_zeros_in_the_sound_is_a_hole(self):
        self.assertEqual(zero_holes(HOLED), [(14680 + 441 + 22050 + 21609, 441)])

    def test_the_silences_before_between_and_after_are_not(self):
        # The lead, the marker's gap, a pause of 200 ms and the end.
        self.assertEqual(zero_holes(recording((14680, 0), (441, 900), (22050, 0), (4410, 5), (8820, 0), (100, 7), (8820, 0))), [])

    def test_a_join_shorter_than_a_gap_is_not(self):
        self.assertEqual(zero_holes(recording((441, 9), (GAP_FRAMES - 1, 0), (441, 9))), [])

    def test_a_silent_recording_has_none(self):
        self.assertEqual(zero_holes(recording((44100, 0))), [])


class Stop(unittest.TestCase):
    def recorder(self, losses: list[str], pcm: bytes = b"", silences=(), refuse_holes: bool = False) -> LoopbackRecorder:
        path = Path(self.enterContext(tempfile.TemporaryDirectory())) / "loop.pcm"
        path.write_bytes(pcm)
        recorder = LoopbackRecorder.__new__(LoopbackRecorder)
        recorder.refuse_holes = refuse_holes
        recorder.loopback = SimpleNamespace(losses=losses, longest_ms=12.0, stop=lambda: None, path=path, silences=list(silences))
        return recorder

    def test_a_recording_with_a_loss_is_refused_and_says_where(self):
        with self.assertRaises(LostFrames) as refused:
            self.recorder(["441 frames after frame 9000"]).stop()
        self.assertIn("lost frames 1 times (441 frames after frame 9000", str(refused.exception))

    def test_a_recording_without_one_is_kept(self):
        self.recorder([]).stop()

    def test_a_hole_is_refused_and_says_whether_the_loopback_flagged_it(self):
        for silences, said in [((), "not flagged silent"), (((58780, 300), (59080, 141)), "flagged silent")]:
            with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(LostFrames) as refused:
                self.recorder([], HOLED, silences, refuse_holes=True).stop()
            self.assertIn(f"1 holes of zeros (441 frames at 58780, {said})", str(refused.exception))

    def test_flagged_packets_are_said_in_the_log_joined(self):
        with contextlib.redirect_stderr(io.StringIO()) as said:
            self.recorder([], HOLED, ((0, 441), (441, 441), (9000, 441))).stop()
        self.assertIn("loop.pcm: the loopback flagged silent 882 frames at 0, 441 frames at 9000", said.getvalue())

    def test_a_hole_is_kept_where_holes_are_not_refused(self):
        self.recorder([], HOLED).stop()


class WindowsHoles(unittest.TestCase):
    def orders(self, holes: bool) -> list[tuple[str, bool]]:
        windows = Windows(Path("probe"), Path("."), holes=holes)
        asked = []
        windows.play_once = lambda order, _record, _perf, refuse: asked.append((order, refuse)) or []
        for order in ["scenario loop\n", "perf 0\n", "decoders 0\n"]:
            windows.play(order, Path("x.pcm"))
        return asked

    def test_only_quad_snds_scenarios_refuse_holes(self):
        self.assertEqual(self.orders(True), [("scenario loop\n", True), ("perf 0\n", False), ("decoders 0\n", False)])
        self.assertEqual([refuse for _, refuse in self.orders(False)], [False, False, False])


if __name__ == "__main__":
    unittest.main()

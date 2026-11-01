"""Tests of the clip study's own machinery: python tools/research/2913/test_study.py"""

from __future__ import annotations

import csv
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import numpy as np

import perturb
import study


def picture() -> np.ndarray:
    frame = np.empty((736, 1280, 3), dtype=np.uint8)
    frame[:] = perturb.BACKGROUND
    frame[320:336, 320:640] = 240
    return frame


class Perturbations(unittest.TestCase):
    def test_each_repeats_byte_for_byte_and_changes_the_picture(self) -> None:
        for name, function in perturb.HAND_MADE.items():
            first = function(picture(), "courier", 1, 3)
            self.assertTrue((first == function(picture(), "courier", 1, 3)).all(), name)
            self.assertFalse((first == picture()).all(), name)

    def test_noise_stays_within_its_bound(self) -> None:
        difference = perturb.noise(picture(), "courier", 0, 0).astype(int) - picture().astype(int)
        self.assertLessEqual(int(np.abs(difference).max()), 16)

    def test_temporal_halves_together_show_the_whole_glyph(self) -> None:
        even = perturb.temporal(picture(), "courier", 0, 0)
        odd = perturb.temporal(picture(), "courier", 0, 1)
        self.assertTrue((np.maximum(even, odd) == picture()).all())
        self.assertFalse((even == picture()).all())

    def test_decoy_adds_only_its_small_contrast(self) -> None:
        difference = perturb.decoy(picture(), "courier", 0, 0).astype(int) - picture().astype(int)
        self.assertEqual(int(difference.max()), perturb.DECOY_DELTA)
        self.assertEqual(int(difference.min()), 0)


class Metrics(unittest.TestCase):
    def test_identical_pictures_score_perfectly(self) -> None:
        self.assertEqual(perturb.psnr(picture(), picture()), 99.0)
        self.assertEqual(perturb.ssim(picture(), picture()), 1.0)

    def test_full_screen_strobe_fails_the_flash_screen(self) -> None:
        black = np.zeros((64, 64, 3), dtype=np.uint8)
        white = np.full((64, 64, 3), 255, dtype=np.uint8)
        self.assertTrue(perturb.flash_area([black, white] * 15)["fails_general_flash_screen"])
        self.assertFalse(perturb.flash_area([black] * 30)["fails_general_flash_screen"])

    def test_survival_sees_a_kept_and_a_removed_perturbation(self) -> None:
        clean = picture()
        noisy = perturb.noise(clean, "courier", 0, 0)
        self.assertEqual(perturb.survival(clean, noisy, clean, noisy)["correlation"], 1.0)
        self.assertEqual(perturb.survival(clean, noisy, clean, clean)["energy_kept"], 0.0)


class Encodes(unittest.TestCase):
    def test_an_encode_repeats_byte_for_byte(self) -> None:
        with tempfile.TemporaryDirectory() as folder:
            hashes = []
            for name in ("a.mp4", "b.mp4"):
                dest = Path(folder) / name
                study.encode(iter([picture()] * 40), 1280, 736, dest, 30, "scale=854:-2")
                hashes.append(study.sha(dest.read_bytes()))
            self.assertEqual(hashes[0], hashes[1])

    def test_the_sampler_takes_the_middle_of_each_hold(self) -> None:
        frames = [np.full((8, 8, 3), index, dtype=np.uint8) for index in range(perturb.HOLD * 2)]
        with tempfile.TemporaryDirectory() as folder:
            study.sample_holds(iter(frames), Path(folder))
            seen = study.read_frames(Path(folder))
        self.assertEqual([int(frame[0, 0, 0]) for frame in seen], [perturb.MID, perturb.HOLD + perturb.MID])


class Attacks(unittest.TestCase):
    def test_the_longest_python_block_is_the_program(self) -> None:
        text = "first\n```python\nx=1\n```\nthen\n```python\nprint('longer')\n```\n"
        self.assertEqual(study.extract_program(text), "print('longer')\n")
        self.assertIsNone(study.extract_program("no code here"))

    def test_a_bypassed_candidate_gets_no_stronger_attacker(self) -> None:
        with tempfile.TemporaryDirectory() as folder:
            out = Path(folder)
            study.write_json(out / "ledger.json", [{"class": "over_floor", "candidate": "noise"}, {"class": "below_floor", "candidate": "warp"}])
            with mock.patch.object(study, "OUT", out):
                self.assertEqual(study.alive(), ["warp", "temporal", "decoy", "surrogate"])
                candidates = {job["candidate"] for job in study.plan("candidates", study.MODELS[2])}
            self.assertNotIn("noise", candidates)
            self.assertIn("warp", candidates)


class HumanArm(unittest.TestCase):
    def kit(self, folder: Path) -> tuple[Path, Path]:
        with mock.patch.object(study, "OUT", folder):
            study.viewer_kit(2, 7, folder / "kit")
        return folder / "kit" / "participant", folder / "kit" / "sealed" / "key.json"

    def test_the_kit_is_blind_and_each_participant_sees_every_clip(self) -> None:
        with tempfile.TemporaryDirectory() as folder:
            public, key = self.kit(Path(folder))
            sheet = (public / "answers.csv").read_text(encoding="utf-8")
            for word in (*perturb.CANDIDATES, *study.GAMES):
                self.assertNotIn(word, sheet)
            clips = json.loads(key.read_text(encoding="utf-8"))
            with (public / "answers.csv").open(encoding="utf-8", newline="") as file:
                rows = list(csv.DictReader(file))
            first = [row["clip"] for row in rows if row["participant"] == "P01"]
            second = [row["clip"] for row in rows if row["participant"] == "P02"]
            self.assertEqual(sorted(first), sorted(clips))
            self.assertEqual(sorted(second), sorted(clips))
            self.assertNotEqual(first, second)

    def scored(self, folder: Path, comfort: int, right: bool) -> dict:
        public, key = self.kit(folder)
        clips = json.loads(key.read_text(encoding="utf-8"))
        truth = {game: {"final_answer": "won"} for game in study.GAMES}
        with (public / "answers.csv").open(encoding="utf-8", newline="") as file:
            rows = list(csv.DictReader(file))
        for row in rows:
            clip = clips[row["clip"]]
            control = clip["candidate"] == "ordinary"
            row.update(understood_goal="yes", final_answer="won" if right or control else "lost",
                       comfort_0_4=0 if control else comfort, identified_game=clip["game"])
        with (public / "answers.csv").open("w", encoding="utf-8", newline="") as file:
            writer = csv.DictWriter(file, study.VIEWER_FIELDS)
            writer.writeheader()
            writer.writerows(rows)
        return study.score_viewers(public / "answers.csv", key, truth)

    def test_viewer_gates(self) -> None:
        with tempfile.TemporaryDirectory() as folder:
            self.assertTrue(self.scored(Path(folder), 1, True)["noise"]["passes"])
        with tempfile.TemporaryDirectory() as folder:
            self.assertFalse(self.scored(Path(folder), 3, True)["noise"]["passes"], "discomfort")
        with tempfile.TemporaryDirectory() as folder:
            self.assertFalse(self.scored(Path(folder), 0, False)["noise"]["passes"], "comprehension")

    def test_the_rubric_floor(self) -> None:
        good = {category: "3" for category in study.RUBRIC_CATEGORIES}
        rows = [{**good, "finished_loop": "yes"}] * 3 + [{**good, "finished_loop": "no"}]
        self.assertTrue(study.score_rubric(rows)["over_floor"])
        self.assertFalse(study.score_rubric(rows[1:] + rows[3:] * 2)["over_floor"])
        low = {**good, "stability": "2", "finished_loop": "yes"}
        self.assertFalse(study.score_rubric([low] * 4)["over_floor"])


if __name__ == "__main__":
    unittest.main()

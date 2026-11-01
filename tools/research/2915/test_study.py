"""Tests of the capture study's own machinery: python tools/research/2915/test_study.py"""

from __future__ import annotations

import sys
import unittest

import numpy as np

import marks
import study

SECRET = "test-secret"


def picture() -> np.ndarray:
    frame = np.full((736, 1280, 3), 12, dtype=np.uint8)
    frame[300:316, 320:640] = 230
    return frame


def residuals(frame: np.ndarray, clean: np.ndarray) -> list:
    coverage = np.ones(clean.shape[:2], bool)
    return [(0, *marks.state_residual(frame, clean, coverage))]


class Marks(unittest.TestCase):
    def test_each_mark_repeats_byte_for_byte_and_differs_by_recipient(self) -> None:
        for candidate in (*marks.INVISIBLE, *marks.VISIBLE):
            first = marks.paint(picture(), candidate, SECRET, 3, "courier", 1)
            self.assertTrue((first == marks.paint(picture(), candidate, SECRET, 3, "courier", 1)).all(), candidate)
            self.assertFalse((first == marks.paint(picture(), candidate, SECRET, 4, "courier", 1)).all(), candidate)
            self.assertFalse((first == picture()).all(), candidate)

    def test_an_unmarked_candidate_changes_nothing(self) -> None:
        self.assertTrue((marks.paint(picture(), "ordinary", SECRET, 3, "courier", 1) == picture()).all())

    def test_an_invisible_mark_stays_within_its_strength(self) -> None:
        difference = marks.paint(picture(), "invisible4", SECRET, 3, "courier", 1).astype(int) - picture().astype(int)
        self.assertLessEqual(int(np.abs(difference).max()), 4)


class Detector(unittest.TestCase):
    def test_it_names_the_true_recipient_and_no_other(self) -> None:
        frame = marks.paint(picture(), "invisible4", SECRET, 17, "courier", 0)
        scores = marks.invisible_scores(residuals(frame, picture()), SECRET, "courier", 200)
        result = marks.verdict(scores, [17])
        self.assertTrue(result["passes"], result)

    def test_scores_on_an_unmarked_frame_stay_under_the_threshold(self) -> None:
        noise = np.random.default_rng(1).normal(0, 2, picture().shape)
        frame = np.clip(picture() + noise, 0, 255).astype(np.uint8)
        scores = marks.invisible_scores(residuals(frame, picture()), SECRET, "courier", 200)
        self.assertLess(float(scores.max()), marks.THRESHOLD)
        self.assertLess(abs(float(scores.std()) - 1.0), 0.25)

    def test_a_wrong_recipient_does_not_pass(self) -> None:
        frame = marks.paint(picture(), "invisible4", SECRET, 17, "courier", 0)
        scores = marks.invisible_scores(residuals(frame, picture()), SECRET, "courier", 200)
        self.assertFalse(marks.verdict(scores, [18])["passes"])

    def test_a_visible_mark_is_found_by_its_drawn_layer(self) -> None:
        clean = picture()
        frame = marks.paint(clean, "visible_tile", SECRET, 5, "courier", 0)
        layers = np.stack([marks.small_layer("visible_tile", SECRET, recipient, "courier", 0, 736, 1280)[None]
                           for recipient in range(60)])
        scores = marks.visible_scores([(0, frame, np.ones(clean.shape[:2], bool))], [clean], layers)
        self.assertTrue(marks.verdict(scores, [5])["passes"])


class Registration(unittest.TestCase):
    def test_it_undoes_a_shift_and_a_scale(self) -> None:
        import cv2

        rng = np.random.default_rng(2)
        clean = np.full((736, 1280, 3), 12, dtype=np.uint8)
        for _ in range(160):
            x, y = int(rng.integers(0, 1200)), int(rng.integers(0, 700))
            clean[y:y + 18, x:x + 50] = rng.integers(60, 250, 3)
        moved = cv2.warpAffine(clean, np.float32([[0.7, 0, 14], [0, 0.7, 9]]), (854, 490), flags=cv2.INTER_AREA)
        warped, coverage, fit = study.register(moved, clean)
        self.assertGreater(fit, 0.8)
        inside = coverage & (cv2.erode(coverage.astype(np.uint8), np.ones((31, 31), np.uint8)) > 0)
        self.assertLess(float(np.abs(warped.astype(int) - clean)[inside].mean()), 8)


class Screen(unittest.TestCase):
    def test_it_finds_a_white_picture_and_undoes_its_perspective(self) -> None:
        import cv2

        rng = np.random.default_rng(3)
        clean = np.full((736, 1280, 3), 12, dtype=np.uint8)
        for _ in range(160):
            x, y = int(rng.integers(0, 1200)), int(rng.integers(0, 700))
            clean[y:y + 18, x:x + 50] = rng.integers(60, 250, 3)
        corners = np.float32(((300, 200), (1500, 230), (1480, 930), (320, 900)))
        source = np.float32(((0, 0), (1280, 0), (1280, 736), (0, 736)))
        forward = cv2.getPerspectiveTransform(source, corners)
        white = np.full_like(clean, 255)
        flash = cv2.warpPerspective(white, forward, (1920, 1080))
        quad = study.screen_quad(flash)
        self.assertLess(float(np.abs(quad - corners).max()), 4)
        filmed = cv2.warpPerspective(clean, forward, (1920, 1080))
        warped, coverage, fit = study.register_by_screen(filmed, clean, quad)
        self.assertGreater(fit, 0.8)
        self.assertLess(float(np.abs(warped.astype(int) - clean)[coverage].mean()), 10)


class Reading(unittest.TestCase):
    def test_a_reading_scores_text_and_code_apart(self) -> None:
        result = study.score_reading("```text\nHP 3\n```\nCODE: ABCD-EFGH", "HP 3", "ABCD-EFGH")
        self.assertEqual(result["game_text_score"], 100.0)
        self.assertTrue(result["code_correct"])

    def test_a_wrong_code_is_not_a_read(self) -> None:
        self.assertFalse(study.score_reading("HP\nCODE: ABCD-EFGJ", "HP", "ABCD-EFGH")["code_correct"])


class Flashes(unittest.TestCase):
    def test_false_flashes_from_a_cameras_exposure_are_dropped(self) -> None:
        found = [5.3, 12.5, 13.7, 21.6, 28.8, 30.0]
        self.assertEqual(study.match_flashes(found, [7.2, 9.2, 7.2, 9.2]), [5.3, 12.5, 21.6, 28.8])

    def test_a_missing_flash_is_refused(self) -> None:
        self.assertIsNone(study.match_flashes([5.3, 12.5, 28.8], [7.2, 9.2, 7.2, 9.2]))


class Attacks(unittest.TestCase):
    def test_palette_snap_removes_a_small_mark_from_flat_colour(self) -> None:
        frame = marks.paint(picture(), "invisible2", SECRET, 1, "courier", 0)
        snapped = study.palette_snap(frame, 2)
        self.assertLessEqual(len(np.unique(snapped.reshape(-1, 3), axis=0)), 2)

    def test_flatten_takes_a_small_mark_off_the_background_and_keeps_the_text(self) -> None:
        frame = marks.paint(picture(), "invisible4", SECRET, 1, "courier", 0)
        flat = study.flatten(frame)
        self.assertEqual(len(np.unique(flat[:200].reshape(-1, 3), axis=0)), 1)
        self.assertGreater(int(flat[305, 400, 0]), 200)


if __name__ == "__main__":
    sys.path.insert(0, ".")
    unittest.main()

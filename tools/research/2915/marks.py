"""Per-recipient marks and their detector, for ticket 2915.

A mark is painted over a finished frame, outside any game state. The detector is
informed: the operator holds the clean capture, because the engine's capture
repeats byte for byte.
"""

from __future__ import annotations

import hashlib

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

GRID_W, GRID_H = 40, 23
CELL = 32
INVISIBLE = {"invisible2": 2.0, "invisible4": 4.0, "invisible8": 8.0}
VISIBLE = ("visible_corner", "visible_tile", "visible_drift")
CANDIDATES = ("ordinary", *INVISIBLE, *VISIBLE)
ALPHABET = "ABCDEFGHJKMNPQRSTUVWXYZ23456789"
TILE_W, TILE_H = 320, 184
ALPHA = 0.20
THRESHOLD = 6.0
LUMA = np.array((0.299, 0.587, 0.114), dtype=np.float32)


def seeded(*parts: object) -> np.random.Generator:
    digest = hashlib.sha256("|".join(map(str, parts)).encode()).digest()
    return np.random.default_rng(int.from_bytes(digest[:8], "little"))


def chips(secret: str, recipient: int, state: int, game: str) -> np.ndarray:
    """One recipient's +1/-1 pattern for one state, on the coarse grid."""
    return seeded(secret, recipient, game, state).choice((-1.0, 1.0), (GRID_H, GRID_W)).astype(np.float32)


def code(secret: str, recipient: int) -> str:
    letters = seeded(secret, "code", recipient).integers(0, len(ALPHABET), 8)
    text = "".join(ALPHABET[index] for index in letters)
    return f"{text[:4]}-{text[4:]}"


def _smooth(pattern: np.ndarray, height: int, width: int) -> np.ndarray:
    picture = Image.fromarray(pattern, mode="F").resize((width, height), Image.BILINEAR)
    return np.asarray(picture, dtype=np.float32)


def paint_invisible(frame: np.ndarray, pattern: np.ndarray, strength: float) -> np.ndarray:
    height, width = frame.shape[:2]
    delta = _smooth(pattern, height, width) * strength
    return np.clip(frame.astype(np.float32) + delta[..., None], 0, 255).round().astype(np.uint8)


def _font(size: int) -> ImageFont.ImageFont:
    return ImageFont.load_default(size=size)


def text_layer(text: str, height: int, width: int, *, tile: bool, offset: tuple[int, int] = (0, 0)) -> np.ndarray:
    """A coverage layer from 0 to 1 holding a recipient's code."""
    layer = Image.new("L", (width, height), 0)
    draw = ImageDraw.Draw(layer)
    if tile:
        font = _font(22)
        for row, y in enumerate(range(-TILE_H, height + TILE_H, TILE_H)):
            for x in range(-TILE_W, width + TILE_W, TILE_W):
                draw.text((x + offset[0] + (row % 2) * TILE_W // 2, y + offset[1]), text, fill=255, font=font)
    else:
        draw.text((width - 170, height - 34), text, fill=255, font=_font(18))
    return np.asarray(layer, dtype=np.float32) / 255


def drift(secret: str, game: str, state: int) -> tuple[int, int]:
    rng = seeded(secret, "drift", game, state)
    return int(rng.integers(0, TILE_W)), int(rng.integers(0, TILE_H))


def recipient_layer(candidate: str, secret: str, recipient: int, game: str, state: int, height: int, width: int):
    text = code(secret, recipient)
    if candidate == "visible_corner":
        return text_layer(text, height, width, tile=False)
    if candidate == "visible_tile":
        return text_layer(text, height, width, tile=True)
    if candidate == "visible_drift":
        return text_layer(text, height, width, tile=True, offset=drift(secret, game, state))
    raise ValueError(candidate)


def paint(frame: np.ndarray, candidate: str, secret: str, recipient: int, game: str, state: int) -> np.ndarray:
    if candidate == "ordinary":
        return frame
    if candidate in INVISIBLE:
        return paint_invisible(frame, chips(secret, recipient, state, game), INVISIBLE[candidate])
    height, width = frame.shape[:2]
    layer = recipient_layer(candidate, secret, recipient, game, state, height, width)[..., None]
    mixed = frame.astype(np.float32) * (1 - ALPHA * layer) + 255 * ALPHA * layer
    return mixed.round().astype(np.uint8)


def luma(frame: np.ndarray) -> np.ndarray:
    return frame.astype(np.float32) @ LUMA


def block_means(values: np.ndarray, valid: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """Median of the valid pixels in each cell; a cell under half valid is left out."""
    means = np.zeros((GRID_H, GRID_W), dtype=np.float32)
    used = np.zeros((GRID_H, GRID_W), dtype=bool)
    height, width = values.shape
    for row in range(GRID_H):
        top, bottom = row * height // GRID_H, (row + 1) * height // GRID_H
        for col in range(GRID_W):
            left, right = col * width // GRID_W, (col + 1) * width // GRID_W
            cell = valid[top:bottom, left:right]
            if cell.mean() >= 0.5:
                means[row, col] = np.median(values[top:bottom, left:right][cell])
                used[row, col] = True
    return means, used


def high_pass(cells: np.ndarray, used: np.ndarray) -> np.ndarray:
    """Takes out what varies slowly across the grid: vignette, exposure, glare."""
    result = np.zeros_like(cells)
    for row in range(GRID_H):
        for col in range(GRID_W):
            if not used[row, col]:
                continue
            near = cells[max(0, row - 2):row + 3, max(0, col - 2):col + 3]
            mask = used[max(0, row - 2):row + 3, max(0, col - 2):col + 3]
            result[row, col] = cells[row, col] - np.median(near[mask])
    return result


def state_residual(received: np.ndarray, clean: np.ndarray, coverage: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """The received frame less the clean one, per grid cell, away from the game's text."""
    clean_luma, received_luma = luma(clean), luma(received)
    values, counts = np.unique(clean_luma.round(), return_counts=True)
    background = values[counts.argmax()]
    flat = np.abs(clean_luma - background) < 2
    shrunk = np.asarray(Image.fromarray(flat.astype(np.uint8) * 255).filter(ImageFilter.MinFilter(7)))
    valid = (shrunk > 0) & coverage
    cells, used = block_means(received_luma - clean_luma, valid)
    return high_pass(cells, used), used


def invisible_scores(residuals, secret: str, game: str, recipients: int) -> np.ndarray:
    """A score per recipient; with a random code it is a unit normal under the null."""
    observed = []
    for _, residual, used in residuals:
        spread = residual[used].std()
        observed.append(residual[used] / spread if spread > 0 else residual[used])
    joined = np.concatenate(observed)
    norm = np.linalg.norm(joined)
    scores = np.zeros(recipients)
    if norm == 0:
        return scores
    for recipient in range(recipients):
        bits = np.concatenate([chips(secret, recipient, state, game)[used] for state, _, used in residuals])
        scores[recipient] = float(joined @ bits) / norm
    return scores


def small_size(candidate: str) -> tuple[int, int]:
    # A drifting mark has a layer per state, so it is kept smaller; its states add up instead.
    return (320, 184) if candidate == "visible_drift" else (640, 368)


def small_layer(candidate, secret, recipient, game, state, height, width) -> np.ndarray:
    layer = recipient_layer(candidate, secret, recipient, game, state, height, width)
    size = small_size(candidate)
    return (np.asarray(Image.fromarray(layer, mode="F").resize(size, Image.BOX)) * 255).round().astype(np.uint8)


def visible_scores(received, clean_states, layers: np.ndarray) -> np.ndarray:
    """Correlates each recipient's drawn mark with what brightened, standardized over all recipients."""
    totals = np.zeros(layers.shape[0])
    drifting = layers.shape[1] > 1
    size = (layers.shape[3], layers.shape[2])
    for state, frame, coverage in received:
        difference = np.clip(luma(frame) - luma(clean_states[state]), 0, None)
        difference[~coverage] = 0
        small = np.asarray(Image.fromarray(difference, mode="F").resize(size, Image.BOX)).ravel()
        small = small - small.mean()
        stack = np.asarray(layers[:, state if drifting else 0]).reshape(layers.shape[0], -1).astype(np.float32)
        # The layout common to everyone is taken out, leaving what tells codes apart.
        stack -= stack.mean(axis=0)
        totals += (stack @ small) / (np.linalg.norm(stack, axis=1) * (np.linalg.norm(small) or 1.0) + 1e-9)
    center = float(np.median(totals))
    spread = float(1.4826 * np.median(np.abs(totals - center))) or 1.0
    return (totals - center) / spread


def verdict(scores: np.ndarray, truth: list[int]) -> dict:
    """The fixed test: a named recipient at 6 or more, and no innocent one at 6."""
    innocents = np.delete(scores, truth)
    best_true = float(scores[truth].max())
    best_innocent = float(innocents.max())
    return {
        "best_true": round(best_true, 2), "best_innocent": round(best_innocent, 2),
        "innocent_mean": round(float(innocents.mean()), 3), "innocent_std": round(float(innocents.std()), 3),
        "passes": bool(best_true >= THRESHOLD and best_true > best_innocent and best_innocent < THRESHOLD),
    }

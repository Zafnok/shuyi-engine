"""The source picture and sound of 2203's fixtures, made from nothing."""
import math
import struct
import sys
import wave

from PIL import Image

out = sys.argv[1]
W, H = 20, 12


def rgb(x, y):
    return (x * 255 // (W - 1), y * 255 // (H - 1), (x + y) * 255 // (W + H - 2))


opaque = Image.new("RGB", (W, H))
alpha = Image.new("RGBA", (W, H))
for y in range(H):
    for x in range(W):
        opaque.putpixel((x, y), rgb(x, y))
        alpha.putpixel((x, y), rgb(x, y) + (255 - (x + y) * 8,))
opaque.save(f"{out}/opaque.png")
alpha.save(f"{out}/picture.png")
opaque.save(f"{out}/picture.jpg", quality=90, subsampling=2)
alpha.save(f"{out}/picture.qoi")

# 50 ms of a 440 Hz and a 660 Hz sine, stereo, 16-bit, at 44.1 kHz.
rate, n = 44_100, 2_205
with wave.open(f"{out}/tone.wav", "wb") as w:
    w.setnchannels(2)
    w.setsampwidth(2)
    w.setframerate(rate)
    frames = b"".join(
        struct.pack(
            "<hh",
            int(8000 * math.sin(2 * math.pi * 440 * i / rate)),
            int(8000 * math.sin(2 * math.pi * 660 * i / rate)),
        )
        for i in range(n)
    )
    w.writeframes(frames)

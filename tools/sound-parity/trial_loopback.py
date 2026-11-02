"""Trial: the recorder against whatever the default endpoint is. With --tone it plays
a known WAV through WinMM and says what came back. Without it nothing is heard."""
import math
import struct
import sys
import tempfile
import time
import wave
from pathlib import Path

sys.path.insert(0, sys.argv[1])
import wasapi

tone = "--tone" in sys.argv
out = Path(tempfile.mkdtemp()) / "rec.pcm"
keeper = wasapi.Endpoint()
rec = wasapi.Loopback(out)
print("format", rec.form)
time.sleep(0.5)
if tone:
    import winsound
    wav = out.with_name("tone.wav")
    with wave.open(str(wav), "wb") as w:
        w.setnchannels(2)
        w.setsampwidth(2)
        w.setframerate(44100)
        frames = [round(16384 * math.sin(2 * math.pi * 440 * i / 44100)) for i in range(44100)]
        w.writeframes(b"".join(struct.pack("<hh", s, s) for s in frames))
    winsound.PlaySound(str(wav), winsound.SND_FILENAME)
time.sleep(0.5)
rec.stop()
keeper.close()
data = out.read_bytes()
samples = struct.unpack(f"<{len(data)//2}h", data)
peak = max((abs(s) for s in samples), default=0)
print("frames", rec.frames, "seconds", rec.frames / rec.form.rate, "peak", peak, "discontinuities", rec.discontinuities)
if tone:
    onset = next((i for i, s in enumerate(samples) if abs(s) > 64), None)
    print("onset", onset)
    if onset is not None:
        print("first samples", samples[onset:onset + 16])
        if rec.form.rate == 44100:
            left = samples[onset - onset % 2::2][:44000]
            want = [round(16384 * math.sin(2 * math.pi * 440 * i / 44100)) for i in range(len(left))]
            best = min(
                (max(abs(a - b) for a, b in zip(left[20:4000], want[20 + k:4000 + k])), k)
                for k in range(0, 60)
            )
            print("exact peak against the tone", best)

"""Drops a FLAC file's PADDING blocks (type 1), keeping the others."""
import sys
data = open(sys.argv[1], "rb").read()
assert data[:4] == b"fLaC"
at, blocks = 4, []
while True:
    head = data[at]
    last, kind = head & 0x80, head & 0x7F
    size = int.from_bytes(data[at + 1:at + 4], "big")
    blocks.append((kind, data[at + 4:at + 4 + size]))
    at += 4 + size
    if last:
        break
kept = [b for b in blocks if b[0] != 1]
out = bytearray(b"fLaC")
for i, (kind, body) in enumerate(kept):
    out.append(kind | (0x80 if i == len(kept) - 1 else 0))
    out += len(body).to_bytes(3, "big") + body
out += data[at:]
open(sys.argv[2], "wb").write(out)
print([k for k, _ in blocks], "->", [k for k, _ in kept], len(data), "->", len(out))

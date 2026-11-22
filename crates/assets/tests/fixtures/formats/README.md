# Fixtures of the picture and sound formats

The files that ticket 2203's formats are tested on
(`crates/assets/src/format/codec_tests.rs`), and whose decoded values
are pinned in `crates/assets/tests/golden/formats.ron`. They are our own,
made from nothing by the two scripts here, and are under CC0-1.0.

| File | What it is | Made with |
| ---- | ---------- | --------- |
| `picture.png` | 20 × 12 px, red rising to the right, green down, blue along the diagonal, alpha falling from 255 at the top left | `make_sources.py` (Pillow) |
| `opaque.png` | the same, without alpha | `make_sources.py` |
| `picture.jpg` | `opaque.png`, quality 90, colour at half size (4:2:0) | `make_sources.py` (Pillow) |
| `picture.qoi` | `picture.png` | `make_sources.py` (Pillow) |
| `lossless.webp` | `picture.png` | `cwebp -lossless -exact` (libwebp 1.6.0) |
| `picture.webp` | `picture.png`, lossy, its alpha lossless | `cwebp -q 80 -alpha_q 100` |
| `lossless.jxl` | `picture.png` | `cjxl -d 0` (libjxl 0.12.0) |
| `picture.jxl` | `opaque.png`, lossy | `cjxl -d 1.0` |
| `jpeg-by-libjpeg.png` | `picture.jpg` as libjpeg-turbo decodes it | `djpeg`, then Pillow to PNG |
| `webp-by-libwebp.png` | `picture.webp` as libwebp decodes it | `dwebp` |
| `jxl-by-libjxl.png` | `picture.jxl` as libjxl decodes it | `djxl` |
| `tone.flac` | 50 ms (2,205 samples) at 44.1 kHz, stereo, 16-bit: 440 Hz on the left, 660 Hz on the right | `make_sources.py` writes it as a WAV; `ffmpeg -c:a flac` (FFmpeg 9.0.2); `strip_flac.py` drops its 8 KiB of padding |
| `tone.mp3` | the same WAV, mono, 64 kbit/s, with a LAME tag | `lame -m m -b 64` (LAME 3.100) |

To make them again, run the commands of the last column in order
(MSYS2's `mingw-w64-x86_64` packages give the encoders). Another
encoder's version makes other bytes, and so other golden rows: bless
them (`cargo xtask golden --bless`) only when a fixture is remade on
purpose.

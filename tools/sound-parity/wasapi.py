"""WASAPI loopback, for `capture.py` on Windows (ticket 3031): what the default
render endpoint really sends, read through the system's documented COM interfaces
with nothing to install.

    IMMDeviceEnumerator, IMMDevice, IAudioClient, IAudioCaptureClient and
    IAudioRenderClient, as Microsoft Learn documents them.

A loopback client gets no packets while nothing renders, so `Endpoint` keeps a
silent render stream open beside it for the whole recording: time keeps moving, and
a sound arriving finds the endpoint already running. It decides nothing, as
`capture.py` doesn't.
"""

from __future__ import annotations

import ctypes
import struct
import threading
import time
import uuid
from ctypes import POINTER, byref, c_int32, c_uint32, c_uint64, cast
from dataclasses import dataclass
from pathlib import Path

CLSID_ENUMERATOR = "{BCDE0395-E52F-467C-8E3D-C4579291692E}"
IID_ENUMERATOR = "{A95664D2-9614-4F35-A746-DE8DB63617E6}"
IID_AUDIO_CLIENT = "{1CB9AD4C-DBFA-4C32-B178-C2F568A703B2}"
IID_CAPTURE = "{C8ADBD64-E71E-48A0-A4DE-185C395CD317}"
IID_RENDER = "{F294ACFC-3146-4483-A7BF-ADDCA7C260E2}"
CLSID_POLICY = "{870AF99C-171D-4F9E-AF0D-E63DF40C2BC9}"
IID_POLICY = "{F8679F50-850A-41CF-9C72-430F290290C8}"
# A WAVEFORMATEXTENSIBLE's subformats.
FLOAT_GUID = uuid.UUID("00000003-0000-0010-8000-00AA00389B71")
PCM_GUID = uuid.UUID("00000001-0000-0010-8000-00AA00389B71")
TAG_PCM = 1
TAG_FLOAT = 3
TAG_EXTENSIBLE = 0xFFFE
E_RENDER = 0
ROLE_CONSOLE = 0
CLSCTX_ALL = 23
COINIT_MULTITHREADED = 0
STREAMFLAGS_LOOPBACK = 0x00020000
BUFFER_SILENT = 2
BUFFER_DISCONTINUITY = 1
# Both streams' buffers, in 100 ns: half a second of room for a slow thread.
BUFFER_HNS = 5_000_000
ole32 = ctypes.OleDLL("ole32")


class Guid(ctypes.Structure):
    _fields_ = [("bytes", ctypes.c_ubyte * 16)]

    @classmethod
    def of(cls, text: str) -> Guid:
        return cls.from_buffer_copy(uuid.UUID(text).bytes_le)


def call(interface: ctypes.c_void_p, index: int, types: tuple[object, ...], *args: object) -> None:
    """Calls COM method `index` of `interface`: a failed answer raises."""
    table = cast(interface, POINTER(POINTER(ctypes.c_size_t))).contents
    function = ctypes.WINFUNCTYPE(ctypes.HRESULT, ctypes.c_void_p, *types)(table[index])
    function(interface, *args)


@dataclass
class Format:
    """An endpoint's mix format, the only one a shared loopback client gets."""

    rate: int
    channels: int
    bits: int
    floats: bool
    frame_bytes: int


def parse_format(raw: bytes) -> Format:
    tag, channels, rate, _, align, bits = struct.unpack_from("<HHIIHH", raw, 0)
    floats = tag == TAG_FLOAT
    if tag == TAG_EXTENSIBLE:
        floats = uuid.UUID(bytes_le=bytes(raw[24:40])) == FLOAT_GUID
    elif tag not in (TAG_PCM, TAG_FLOAT):
        raise SystemExit(f"an endpoint format of tag {tag}")
    return Format(rate, channels, bits, floats, align)


def coinit() -> None:
    # An error here says the thread is initialized already, which is enough.
    try:
        ole32.CoInitializeEx(None, COINIT_MULTITHREADED)
    except OSError:
        pass


def default_endpoint() -> ctypes.c_void_p:
    """The default render endpoint: its IMMDevice. No endpoint raises."""
    enumerator = ctypes.c_void_p()
    ole32.CoCreateInstance(
        byref(Guid.of(CLSID_ENUMERATOR)), None, CLSCTX_ALL, byref(Guid.of(IID_ENUMERATOR)), byref(enumerator)
    )
    device = ctypes.c_void_p()
    call(enumerator, 4, (c_int32, c_int32, POINTER(ctypes.c_void_p)), E_RENDER, ROLE_CONSOLE, byref(device))
    return device


def extensible(rate: int, bits: int, floats: bool) -> ctypes.Array:
    """A stereo WAVEFORMATEXTENSIBLE."""
    align = 2 * bits // 8
    subformat = FLOAT_GUID if floats else PCM_GUID
    raw = struct.pack("<HHIIHHHHI", TAG_EXTENSIBLE, 2, rate, rate * align, align, bits, 22, bits, 3) + subformat.bytes_le
    return (ctypes.c_ubyte * len(raw)).from_buffer_copy(raw)


def set_endpoint_rate(rate: int) -> None:
    """Sets the default endpoint's format, as its Advanced tab does: CI's virtual
    endpoint starts at 48 kHz, and a sample-exact comparison needs no resampling on
    the way. `IPolicyConfig` isn't documented; only this tool, on a runner, calls it."""
    device = default_endpoint()
    identifier = ctypes.c_wchar_p()
    call(device, 5, (POINTER(ctypes.c_wchar_p),), byref(identifier))
    policy = ctypes.c_void_p()
    ole32.CoCreateInstance(
        byref(Guid.of(CLSID_POLICY)), None, CLSCTX_ALL, byref(Guid.of(IID_POLICY)), byref(policy)
    )
    endpoint, mix = extensible(rate, 16, False), extensible(rate, 32, True)
    call(policy, 6, (ctypes.c_wchar_p, ctypes.c_void_p, ctypes.c_void_p), identifier.value, endpoint, mix)


class Client:
    """An IAudioClient on the default endpoint, in its mix format."""

    def __init__(self, flags: int) -> None:
        device = default_endpoint()
        self.client = ctypes.c_void_p()
        call(
            device,
            3,
            (POINTER(Guid), c_uint32, ctypes.c_void_p, POINTER(ctypes.c_void_p)),
            byref(Guid.of(IID_AUDIO_CLIENT)),
            CLSCTX_ALL,
            None,
            byref(self.client),
        )
        mix = ctypes.c_void_p()
        call(self.client, 8, (POINTER(ctypes.c_void_p),), byref(mix))
        tag_size = struct.unpack_from("<H", ctypes.string_at(mix, 18), 16)[0]
        self.format = parse_format(ctypes.string_at(mix, 18 + tag_size))
        call(
            self.client,
            3,
            (c_int32, c_uint32, ctypes.c_longlong, ctypes.c_longlong, ctypes.c_void_p, ctypes.c_void_p),
            0,
            flags,
            BUFFER_HNS,
            0,
            mix,
            None,
        )
        ole32.CoTaskMemFree(mix)

    def service(self, iid: str) -> ctypes.c_void_p:
        out = ctypes.c_void_p()
        call(self.client, 14, (POINTER(Guid), POINTER(ctypes.c_void_p)), byref(Guid.of(iid)), byref(out))
        return out

    def start(self) -> None:
        call(self.client, 10, ())

    def stop(self) -> None:
        call(self.client, 11, ())

    def size(self) -> int:
        frames = c_uint32()
        call(self.client, 4, (POINTER(c_uint32),), byref(frames))
        return frames.value

    def padding(self) -> int:
        frames = c_uint32()
        call(self.client, 6, (POINTER(c_uint32),), byref(frames))
        return frames.value


def to_s16(raw: bytes, form: Format) -> bytes:
    """`raw` as 16-bit stereo: a fraction of full scale is all that changes."""
    count = len(raw) // (form.bits // 8)
    if form.floats and form.bits == 32:
        samples = struct.unpack(f"<{count}f", raw[: count * 4])
        scaled = [max(-32768, min(32767, round(s * 32768.0))) for s in samples]
    elif form.bits == 16:
        scaled = list(struct.unpack(f"<{count}h", raw[: count * 2]))
    elif form.bits == 32:
        scaled = [s >> 16 for s in struct.unpack(f"<{count}i", raw[: count * 4])]
    else:
        raise SystemExit(f"an endpoint format of {form.bits} bits")
    if form.channels > 2:
        scaled = [s for frame in range(0, count, form.channels) for s in scaled[frame : frame + 2]]
    return struct.pack(f"<{len(scaled)}h", *scaled)


class Endpoint:
    """The silent stream that keeps the default endpoint running while it is open."""

    def __init__(self) -> None:
        self.running = True
        self.ready = threading.Event()
        self.failure: BaseException | None = None
        self.thread = threading.Thread(target=self.keep, daemon=True)
        self.thread.start()
        self.ready.wait(10)
        if self.failure is not None:
            raise self.failure

    def keep(self) -> None:
        coinit()
        try:
            client = Client(0)
            render = client.service(IID_RENDER)
            size = client.size()
            client.start()
            self.ready.set()
            while self.running:
                room = size - client.padding()
                if room > 0:
                    data = ctypes.c_void_p()
                    call(render, 3, (c_uint32, POINTER(ctypes.c_void_p)), room, byref(data))
                    # Written zeros, not the silent flag: an engine may drop a
                    # stream that says it is silent, and the loopback goes quiet.
                    ctypes.memset(data, 0, room * client.format.frame_bytes)
                    call(render, 4, (c_uint32, c_uint32), room, 0)
                time.sleep(0.02)
            client.stop()
        except OSError as error:
            self.failure = error
            self.ready.set()

    def close(self) -> None:
        self.running = False
        self.thread.join(5)


@dataclass
class Loss:
    """Where a recording lost frames (ticket 2047): after how many frames, how many
    the device's position skipped, how long the reader hadn't looked before the
    packet that says so, and the buffer's length. A wait as long as the buffer is the
    recorder's fault, as the buffer filled; a short one says the system dropped them.
    The packet's own timestamp says nothing here: it is the first one after the hole."""

    at: int
    lost: int
    waited_ms: float
    buffer_ms: float

    def __str__(self) -> str:
        return (
            f"{self.lost} frames after frame {self.at}, the reader having waited "
            f"{self.waited_ms:.0f} ms of the buffer's {self.buffer_ms:.0f}"
        )


class Loopback:
    """A recording of the default endpoint, into `path`, as 16-bit stereo at stop.

    `frames` counts what has come; `start` is the wall clock of frame 0, the earliest
    arrival less the frames before it, as `capture.py`'s recorder has it. `losses`
    holds each place the system says frames were dropped, and `longest_ms` the
    longest the reader went between two looks."""

    def __init__(self, path: Path) -> None:
        self.path = path
        self.frames = 0
        self.start = float("inf")
        self.losses: list[Loss] = []
        self.longest_ms = 0.0
        # How long the reader hadn't looked, at its latest look.
        self.waited_ms = 0.0
        # The device position the next packet should start at.
        self.position: int | None = None
        self.form: Format | None = None
        self.running = True
        self.failure: BaseException | None = None
        self.ready = threading.Event()
        self.thread = threading.Thread(target=self.pump, daemon=True)
        self.chunks: list[bytes] = []
        self.thread.start()
        self.ready.wait(10)
        if self.failure is not None:
            raise self.failure

    def pump(self) -> None:
        coinit()
        try:
            client = Client(STREAMFLAGS_LOOPBACK)
            self.form = client.format
            capture = client.service(IID_CAPTURE)
            buffer_ms = client.size() * 1000 / self.form.rate
            client.start()
            self.ready.set()
            looked = time.perf_counter()
            while self.running:
                time.sleep(0.005)
                now = time.perf_counter()
                self.waited_ms = (now - looked) * 1000
                self.longest_ms = max(self.longest_ms, self.waited_ms)
                looked = now
                while self.read_packet(capture, buffer_ms):
                    pass
            client.stop()
        except OSError as error:
            self.failure = error
            self.ready.set()

    def read_packet(self, capture: ctypes.c_void_p, buffer_ms: float) -> bool:
        waiting = c_uint32()
        call(capture, 5, (POINTER(c_uint32),), byref(waiting))
        if waiting.value == 0:
            return False
        data = ctypes.c_void_p()
        frames = c_uint32()
        flags = c_uint32()
        position = c_uint64()
        call(
            capture,
            3,
            (POINTER(ctypes.c_void_p), POINTER(c_uint32), POINTER(c_uint32), POINTER(c_uint64), POINTER(c_uint64)),
            byref(data),
            byref(frames),
            byref(flags),
            byref(position),
            None,
        )
        size = frames.value * self.form.frame_bytes
        if flags.value & BUFFER_SILENT:
            self.chunks.append(bytes(size))
        else:
            self.chunks.append(ctypes.string_at(data, size))
        # The first packet after a start says so, and means nothing was lost.
        if flags.value & BUFFER_DISCONTINUITY and self.frames > 0:
            lost = position.value - self.position if self.position is not None else 0
            self.losses.append(Loss(self.frames, lost, self.waited_ms, buffer_ms))
        self.position = position.value + frames.value
        self.frames += frames.value
        self.start = min(self.start, time.time() - self.frames / self.form.rate)
        call(capture, 4, (c_uint32,), frames.value)
        return True

    def stop(self) -> None:
        self.running = False
        self.thread.join(10)
        self.path.write_bytes(to_s16(b"".join(self.chunks), self.form))

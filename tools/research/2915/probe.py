"""Windows capture probe for ticket 2915: what each ordinary path sees of a window.

Opens a window of one known colour with a display-affinity flag, then asks each
software path for that rectangle from a separate process. A path counts as seeing
the window only if its picture holds the window's colour.

`play` shows a list of PNG frames fullscreen with the capture exclusion on, while
this process's own screen copy records what software saw. It is what a phone films.
"""

from __future__ import annotations

import ctypes
from ctypes import wintypes
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time

import numpy as np
from PIL import Image, ImageGrab

user32 = ctypes.WinDLL("user32", use_last_error=True)
gdi32 = ctypes.WinDLL("gdi32", use_last_error=True)
kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
opengl32 = ctypes.WinDLL("opengl32", use_last_error=True)

LRESULT = ctypes.c_ssize_t
WNDPROC = ctypes.WINFUNCTYPE(LRESULT, wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM)
user32.DefWindowProcW.restype = LRESULT
user32.DefWindowProcW.argtypes = (wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM)
user32.CreateWindowExW.restype = wintypes.HWND
user32.CreateWindowExW.argtypes = (wintypes.DWORD, wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.DWORD, ctypes.c_int,
                                   ctypes.c_int, ctypes.c_int, ctypes.c_int, wintypes.HWND, wintypes.HMENU,
                                   wintypes.HINSTANCE, wintypes.LPVOID)
user32.GetDC.restype = wintypes.HDC
user32.GetDC.argtypes = (wintypes.HWND,)
user32.ReleaseDC.argtypes = (wintypes.HWND, wintypes.HDC)
user32.SetWindowDisplayAffinity.argtypes = (wintypes.HWND, wintypes.DWORD)
user32.PrintWindow.argtypes = (wintypes.HWND, wintypes.HDC, wintypes.UINT)
user32.GetWindowRect.argtypes = (wintypes.HWND, ctypes.POINTER(wintypes.RECT))
user32.DestroyWindow.argtypes = (wintypes.HWND,)
user32.SetProcessDpiAwarenessContext.argtypes = (ctypes.c_void_p,)
kernel32.GetModuleHandleW.restype = wintypes.HMODULE
gdi32.StretchDIBits.argtypes = (wintypes.HDC, *([ctypes.c_int] * 8), ctypes.c_void_p, ctypes.c_void_p,
                                wintypes.UINT, wintypes.DWORD)
gdi32.SetStretchBltMode.argtypes = (wintypes.HDC, ctypes.c_int)
gdi32.CreateCompatibleDC.restype = wintypes.HDC
gdi32.CreateCompatibleDC.argtypes = (wintypes.HDC,)
gdi32.CreateCompatibleBitmap.restype = wintypes.HBITMAP
gdi32.CreateCompatibleBitmap.argtypes = (wintypes.HDC, ctypes.c_int, ctypes.c_int)
gdi32.SelectObject.argtypes = (wintypes.HDC, wintypes.HGDIOBJ)
gdi32.GetDIBits.argtypes = (wintypes.HDC, wintypes.HBITMAP, wintypes.UINT, wintypes.UINT, ctypes.c_void_p,
                            ctypes.c_void_p, wintypes.UINT)
gdi32.ChoosePixelFormat.argtypes = (wintypes.HDC, ctypes.c_void_p)
gdi32.SetPixelFormat.argtypes = (wintypes.HDC, ctypes.c_int, ctypes.c_void_p)
gdi32.SwapBuffers.argtypes = (wintypes.HDC,)
opengl32.wglCreateContext.restype = ctypes.c_void_p
opengl32.wglCreateContext.argtypes = (wintypes.HDC,)
opengl32.wglMakeCurrent.argtypes = (wintypes.HDC, ctypes.c_void_p)
opengl32.glClearColor.argtypes = [ctypes.c_float] * 4
opengl32.glClear.argtypes = (ctypes.c_uint,)

WDA = {"none": 0, "monitor": 1, "exclude": 0x11}
WS_POPUP, WS_VISIBLE, WS_CAPTION = 0x80000000, 0x10000000, 0x00C00000
WS_EX_TOPMOST, WS_EX_NOACTIVATE = 0x8, 0x08000000
COLOR = (214, 40, 160)
SIZE = 360
TITLE = "shuyi-capture-probe"


class PixelFormat(ctypes.Structure):
    _fields_ = [("nSize", wintypes.WORD), ("nVersion", wintypes.WORD), ("dwFlags", wintypes.DWORD),
                ("iPixelType", ctypes.c_byte), ("cColorBits", ctypes.c_byte), ("rest", ctypes.c_byte * 18),
                ("dwLayerMask", wintypes.DWORD), ("dwVisibleMask", wintypes.DWORD), ("dwDamageMask", wintypes.DWORD)]


class WindowClass(ctypes.Structure):
    _fields_ = [("style", wintypes.UINT), ("lpfnWndProc", WNDPROC), ("cbClsExtra", ctypes.c_int),
                ("cbWndExtra", ctypes.c_int), ("hInstance", wintypes.HINSTANCE), ("hIcon", wintypes.HICON),
                ("hCursor", wintypes.HANDLE), ("hbrBackground", wintypes.HBRUSH),
                ("lpszMenuName", wintypes.LPCWSTR), ("lpszClassName", wintypes.LPCWSTR)]


class BitmapInfoHeader(ctypes.Structure):
    _fields_ = [("biSize", wintypes.DWORD), ("biWidth", wintypes.LONG), ("biHeight", wintypes.LONG),
                ("biPlanes", wintypes.WORD), ("biBitCount", wintypes.WORD), ("biCompression", wintypes.DWORD),
                ("biSizeImage", wintypes.DWORD), ("biXPelsPerMeter", wintypes.LONG),
                ("biYPelsPerMeter", wintypes.LONG), ("biClrUsed", wintypes.DWORD), ("biClrImportant", wintypes.DWORD)]


def _default_proc(hwnd, message, wparam, lparam):
    return user32.DefWindowProcW(hwnd, message, wparam, lparam)


_PROC = WNDPROC(_default_proc)
_REGISTERED: set[str] = set()


def dpi_aware() -> None:
    user32.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))


def create_window(x: int, y: int, width: int, height: int) -> int:
    instance = kernel32.GetModuleHandleW(None)
    name = "ShuyiCaptureProbe"
    if name not in _REGISTERED:
        window_class = WindowClass(lpfnWndProc=_PROC, hInstance=instance, lpszClassName=name)
        if not user32.RegisterClassW(ctypes.byref(window_class)):
            raise ctypes.WinError(ctypes.get_last_error())
        _REGISTERED.add(name)
    hwnd = user32.CreateWindowExW(WS_EX_TOPMOST | WS_EX_NOACTIVATE, name, TITLE, WS_POPUP | WS_VISIBLE,
                                  x, y, width, height, None, None, instance, None)
    if not hwnd:
        raise ctypes.WinError(ctypes.get_last_error())
    return hwnd


def pump(seconds: float) -> None:
    end = time.time() + seconds
    message = wintypes.MSG()
    while time.time() < end:
        while user32.PeekMessageW(ctypes.byref(message), None, 0, 0, 1):
            user32.TranslateMessage(ctypes.byref(message))
            user32.DispatchMessageW(ctypes.byref(message))
        time.sleep(0.01)


def draw_rgb(hwnd: int, picture: np.ndarray, width: int, height: int) -> None:
    """Paints an RGB array over the whole window."""
    rows, cols = picture.shape[:2]
    bgra = np.dstack((picture[..., 2], picture[..., 1], picture[..., 0], np.zeros((rows, cols), np.uint8)))
    header = BitmapInfoHeader(biSize=ctypes.sizeof(BitmapInfoHeader), biWidth=cols, biHeight=-rows, biPlanes=1,
                              biBitCount=32, biCompression=0)
    dc = user32.GetDC(hwnd)
    gdi32.SetStretchBltMode(dc, 4)
    gdi32.StretchDIBits(dc, 0, 0, width, height, 0, 0, cols, rows, bgra.tobytes(), ctypes.byref(header), 0, 0x00CC0020)
    user32.ReleaseDC(hwnd, dc)


def paint_gl(hwnd: int) -> tuple[int, int]:
    dc = user32.GetDC(hwnd)
    # PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER, RGBA, 32 bits.
    pixel_format = PixelFormat(nSize=ctypes.sizeof(PixelFormat), nVersion=1, dwFlags=0x25, iPixelType=0, cColorBits=32)
    gdi32.SetPixelFormat(dc, gdi32.ChoosePixelFormat(dc, ctypes.byref(pixel_format)), ctypes.byref(pixel_format))
    context = opengl32.wglCreateContext(dc)
    opengl32.wglMakeCurrent(dc, context)
    opengl32.glClearColor(COLOR[0] / 255, COLOR[1] / 255, COLOR[2] / 255, 1)
    opengl32.glClear(0x4000)
    gdi32.SwapBuffers(dc)
    return dc, context


def holds_color(picture: np.ndarray) -> float:
    """The share of pixels within 12 levels of the window's colour."""
    return float((np.abs(picture[..., :3].astype(int) - np.array(COLOR)).max(axis=-1) <= 12).mean())


# The separate process: each path asks for the same rectangle.


def path_bitblt(box, _hwnd, _ffmpeg, _out) -> np.ndarray:
    return np.asarray(ImageGrab.grab(bbox=box, all_screens=True).convert("RGB"))


def path_print_window(box, hwnd, _ffmpeg, _out) -> np.ndarray:
    width, height = box[2] - box[0], box[3] - box[1]
    screen = user32.GetDC(None)
    memory = gdi32.CreateCompatibleDC(screen)
    bitmap = gdi32.CreateCompatibleBitmap(screen, width, height)
    gdi32.SelectObject(memory, bitmap)
    ok = user32.PrintWindow(hwnd, memory, 2)
    if not ok:
        raise RuntimeError("PrintWindow returned 0")
    header = BitmapInfoHeader(biSize=ctypes.sizeof(BitmapInfoHeader), biWidth=width, biHeight=-height, biPlanes=1,
                              biBitCount=32, biCompression=0)
    buffer = ctypes.create_string_buffer(width * height * 4)
    gdi32.GetDIBits(memory, bitmap, 0, height, buffer, ctypes.byref(header), 0)
    picture = np.frombuffer(buffer, np.uint8).reshape(height, width, 4)
    return picture[..., [2, 1, 0]]


def _grab_with_ffmpeg(args, ffmpeg, out) -> np.ndarray:
    path = Path(out) / "grab.png"
    process = subprocess.run([ffmpeg, "-y", "-loglevel", "error", *args, "-frames:v", "1", str(path)],
                             capture_output=True, text=True, timeout=30, check=False)
    if process.returncode != 0:
        raise RuntimeError(process.stderr.strip()[-200:])
    return np.asarray(Image.open(path).convert("RGB"))


def path_gdigrab(box, _hwnd, ffmpeg, out) -> np.ndarray:
    width, height = box[2] - box[0], box[3] - box[1]
    return _grab_with_ffmpeg(["-f", "gdigrab", "-offset_x", str(box[0]), "-offset_y", str(box[1]),
                              "-video_size", f"{width}x{height}", "-i", "desktop"], ffmpeg, out)


def path_desktop_duplication(box, _hwnd, ffmpeg, out) -> np.ndarray:
    width, height = box[2] - box[0], box[3] - box[1]
    graph = (f"ddagrab=offset_x={box[0]}:offset_y={box[1]}:video_size={width}x{height}:framerate=10,"
             "hwdownload,format=bgra")
    return _grab_with_ffmpeg(["-f", "lavfi", "-i", graph], ffmpeg, out)


def path_graphics_capture(box, _hwnd, _ffmpeg, _out) -> np.ndarray:
    from windows_capture import WindowsCapture, Frame, InternalCaptureControl

    done = threading.Event()
    result: dict[str, np.ndarray] = {}
    capture = WindowsCapture(cursor_capture=False, draw_border=False, window_name=TITLE)

    @capture.event
    def on_frame_arrived(frame: Frame, control: InternalCaptureControl) -> None:
        result["picture"] = np.array(frame.frame_buffer[..., [2, 1, 0]])
        done.set()
        control.stop()

    @capture.event
    def on_closed() -> None:
        done.set()

    capture.start_free_threaded()
    if not done.wait(8):
        raise RuntimeError("no frame in 8 seconds")
    if "picture" not in result:
        raise RuntimeError("closed with no frame")
    return result["picture"]


PATHS = {
    "GDI screen copy (BitBlt)": path_bitblt,
    "PrintWindow": path_print_window,
    "ffmpeg gdigrab": path_gdigrab,
    "Desktop Duplication (ffmpeg ddagrab)": path_desktop_duplication,
    "Windows.Graphics.Capture": path_graphics_capture,
}


def run_path(name: str, box_text: str, hwnd: str, ffmpeg: str) -> None:
    dpi_aware()
    box = tuple(map(int, box_text.split(",")))
    with tempfile.TemporaryDirectory() as out:
        try:
            picture = PATHS[name](box, int(hwnd), ffmpeg, out)
        except Exception as error:  # A path that refuses is a result, and is written down as one.
            print(json.dumps({"error": f"{type(error).__name__}: {error}"[:240]}))
            return
    print(json.dumps({"share_of_window_colour": round(holds_color(picture), 4), "shape": list(picture.shape),
                      "mean": [round(float(value), 1) for value in picture[..., :3].mean(axis=(0, 1))]}))


def automation_name(hwnd: int) -> dict:
    """What UI Automation reads of the window; accessibility tools use this tree, not pixels."""
    script = ("Add-Type -AssemblyName UIAutomationClient;"
              f"[Windows.Automation.AutomationElement]::FromHandle([IntPtr]{hwnd}).Current.Name")
    process = subprocess.Popen(["powershell.exe", "-NoProfile", "-Command", script],
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    deadline = time.time() + 30
    while process.poll() is None and time.time() < deadline:
        pump(0.05)
    if process.poll() is None:
        process.kill()
    stdout, stderr = process.communicate()
    return {"name": stdout.strip(), "error": stderr.strip()[-200:]}


def matrix(ffmpeg: str) -> dict:
    dpi_aware()
    results = {}
    for kind in ("gdi", "opengl"):
        for flag, value in WDA.items():
            hwnd = create_window(120, 120, SIZE, SIZE)
            if kind == "gdi":
                draw_rgb(hwnd, np.full((8, 8, 3), COLOR, np.uint8), SIZE, SIZE)
            else:
                paint_gl(hwnd)
            set_ok = bool(user32.SetWindowDisplayAffinity(hwnd, value))
            pump(0.6)
            box = (120, 120, 120 + SIZE, 120 + SIZE)
            for path in PATHS:
                # PrintWindow sends this window a message, so it must keep pumping while it waits.
                process = subprocess.Popen(
                    [sys.executable, __file__, "path", path, ",".join(map(str, box)), str(hwnd), ffmpeg],
                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                )
                deadline = time.time() + 30
                while process.poll() is None and time.time() < deadline:
                    pump(0.05)
                if process.poll() is None:
                    process.kill()
                stdout, stderr = process.communicate()
                try:
                    outcome = json.loads(stdout.strip().splitlines()[-1])
                except (IndexError, ValueError):
                    outcome = {"error": (stderr.strip() or "timed out")[-240:]}
                results[f"{kind}/{flag}/{path}"] = outcome
            results[f"{kind}/{flag}/set_ok"] = set_ok
            results[f"{kind}/{flag}/UI Automation"] = automation_name(hwnd)
            user32.DestroyWindow(hwnd)
            pump(0.1)
    return results


def other_process_may_set() -> dict:
    """Microsoft's rule, checked: a process other than the owner cannot set the flag."""
    dpi_aware()
    process = subprocess.Popen(
        [sys.executable, "-c", (
            "import ctypes,time,sys\n"
            "from ctypes import wintypes\n"
            "u=ctypes.WinDLL('user32',use_last_error=True)\n"
            "u.FindWindowW.restype=wintypes.HWND\n"
            f"h=u.FindWindowW(None,{TITLE!r})\n"
            "u.SetWindowDisplayAffinity.argtypes=(wintypes.HWND,wintypes.DWORD)\n"
            "print(bool(u.SetWindowDisplayAffinity(h,0)),ctypes.get_last_error())\n"
        )], stdout=subprocess.PIPE, text=True)
    hwnd = create_window(120, 120, SIZE, SIZE)
    user32.SetWindowDisplayAffinity(hwnd, WDA["exclude"])
    pump(1.5)
    out, _ = process.communicate(timeout=20)
    user32.DestroyWindow(hwnd)
    return {"another_process_cleared_the_flag": out.strip()}


def play(frames_list: str, record: str) -> None:
    """Plays frames fullscreen with capture exclusion on, recording what a screen copy saw."""
    dpi_aware()
    plan = json.loads(Path(frames_list).read_text(encoding="utf-8"))
    width, height = user32.GetSystemMetrics(0), user32.GetSystemMetrics(1)
    hwnd = create_window(0, 0, width, height)
    if plan["exclude"]:
        user32.SetWindowDisplayAffinity(hwnd, WDA["exclude"])
    seen = []
    scale = min(width / 1280, height / 736)
    picture_size = (int(1280 * scale), int(736 * scale))
    # Each step ends on a schedule from the start, so drawing's cost never adds up.
    deadline = time.time()
    for step in plan["steps"]:
        image = Image.open(step["path"]).convert("RGB")
        fitted = image if image.size == picture_size else image.resize(picture_size, Image.NEAREST)
        canvas = np.zeros((height, width, 3), np.uint8)
        top, left = (height - picture_size[1]) // 2, (width - picture_size[0]) // 2
        canvas[top:top + picture_size[1], left:left + picture_size[0]] = np.asarray(fitted)
        draw_rgb(hwnd, canvas, width, height)
        if step.get("check"):
            grabbed = np.asarray(ImageGrab.grab().convert("RGB"))
            seen.append({"state": step["check"], "screen_copy_mean": round(float(grabbed.mean()), 2),
                         "screen_copy_shares": round(float((grabbed == canvas).all(axis=-1).mean()), 4),
                         "shown_mean": round(float(canvas.mean()), 2)})
        deadline += step["seconds"]
        pump(max(0.0, deadline - time.time()))
    user32.DestroyWindow(hwnd)
    Path(record).write_text(json.dumps({"exclude": plan["exclude"], "seen": seen}, indent=1) + "\n",
                            encoding="utf-8", newline="\n")


def main() -> None:
    command = sys.argv[1]
    if command == "path":
        run_path(*sys.argv[2:6])
    elif command == "matrix":
        print(json.dumps({**matrix(sys.argv[2]), **other_process_may_set()}, indent=1, sort_keys=True))
    elif command == "play":
        play(sys.argv[2], sys.argv[3])
    else:
        raise SystemExit(f"unknown command {command}")


if __name__ == "__main__":
    main()

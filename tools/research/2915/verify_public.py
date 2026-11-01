"""Checks that public builds carry no capture restriction, for ticket 2915.

Three checks: no restricting call in the engine's source or its window backends; no
such import in a built game; and an ordinary game window and web page are captured
by the same software paths the probe uses. Run `scan`, `window` or `web`.
"""

from __future__ import annotations

import ctypes
from ctypes import wintypes
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import threading
import time

import numpy as np
from PIL import Image

import probe

ROOT = Path(__file__).resolve().parents[3]
RESTRICTING = re.compile(
    rb"SetWindowDisplayAffinity|WDA_EXCLUDE|WDA_MONITOR|FLAG_SECURE|setSecure|sharingType|NSWindowSharing"
    rb"|requestMediaKeySystemAccess|setMediaKeys|setContentProtection"
)
ENGINE_FOLDERS = ("crates", "templates", "examples", "skills")
SKIPPED_SUFFIXES = {".png", ".wasm", ".ttf", ".otf", ".ogg", ".wav", ".lock"}
CHROME = Path("C:/Program Files/Google/Chrome/Application/chrome.exe")


def scan_tree(folder: Path) -> list[str]:
    hits = []
    for path in folder.rglob("*"):
        if not path.is_file() or path.suffix in SKIPPED_SUFFIXES or "target" in path.parts:
            continue
        if RESTRICTING.search(path.read_bytes()):
            hits.append(str(path))
    return hits


def scan() -> dict:
    registry = next((Path.home() / ".cargo/registry/src").iterdir())
    backends = {name: scan_tree(registry / name) for name in ("macroquad-0.4.16", "miniquad-0.4.11")}
    return {"engine_hits": [hit for folder in ENGINE_FOLDERS for hit in scan_tree(ROOT / folder)],
            "backend_hits": backends}


def binary_hits(path: Path) -> list[str]:
    return sorted({match.decode() for match in RESTRICTING.findall(path.read_bytes())})


def windows_of(pid: int) -> list[int]:
    found = []
    callback_type = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)

    def callback(hwnd, _):
        owner = wintypes.DWORD()
        probe.user32.GetWindowThreadProcessId(hwnd, ctypes.byref(owner))
        if owner.value == pid and probe.user32.IsWindowVisible(hwnd):
            found.append(hwnd)
        return True

    probe.user32.EnumWindows.argtypes = (callback_type, wintypes.LPARAM)
    probe.user32.GetWindowThreadProcessId.argtypes = (wintypes.HWND, ctypes.POINTER(wintypes.DWORD))
    probe.user32.IsWindowVisible.argtypes = (wintypes.HWND,)
    probe.user32.EnumWindows(callback_type(callback), 0)
    return found


def window(executable: str, cwd: str, ffmpeg: str, shots: str | None = None) -> dict:
    """Runs a built game and asks each path for its window."""
    probe.dpi_aware()
    process = subprocess.Popen([executable], cwd=cwd)
    report = {"binary": Path(executable).name, "restricting_strings_in_binary": binary_hits(Path(executable))}
    try:
        hwnd = None
        deadline = time.time() + 40
        while time.time() < deadline and not hwnd:
            time.sleep(0.5)
            found = windows_of(process.pid)
            hwnd = found[0] if found else None
        if not hwnd:
            report["error"] = "no window"
            return report
        time.sleep(3)
        buffer = ctypes.create_unicode_buffer(256)
        probe.user32.GetWindowTextW.argtypes = (wintypes.HWND, wintypes.LPWSTR, ctypes.c_int)
        probe.user32.GetWindowTextW(hwnd, buffer, 256)
        probe.TITLE = buffer.value
        report["title"] = buffer.value
        # On top without taking the focus, so a screen copy sees this window and not another.
        probe.user32.SetWindowPos.argtypes = (wintypes.HWND, wintypes.HWND, *([ctypes.c_int] * 4), wintypes.UINT)
        probe.user32.SetWindowPos(hwnd, wintypes.HWND(-1), 0, 0, 0, 0, 0x0013)
        time.sleep(1)
        rect = wintypes.RECT()
        probe.user32.GetWindowRect(hwnd, ctypes.byref(rect))
        box = (rect.left + 12, rect.top + 40, rect.right - 12, rect.bottom - 12)
        with tempfile.TemporaryDirectory() as out:
            for name, path in probe.PATHS.items():
                try:
                    picture = path(box, hwnd, ffmpeg, out)
                    report[name] = {"shape": list(picture.shape), "distinct_levels": int(len(np.unique(picture))),
                                    "mean": round(float(picture.mean()), 1),
                                    "has_a_picture": bool(len(np.unique(picture)) > 8)}
                    if shots:
                        name_part = re.sub(r"\W+", "-", name)
                        Image.fromarray(picture).save(Path(shots) / f"{name_part}.png")
                except Exception as error:  # A path that refuses is a result.
                    report[name] = {"error": f"{type(error).__name__}: {error}"[:200]}
    finally:
        process.kill()
    return report


PAGE = """<!doctype html><title>shuyi web capture probe</title>
<canvas id=c width=320 height=200></canvas><button id=b>x</button><script>
const c=document.getElementById('c'),g=c.getContext('webgl',{preserveDrawingBuffer:false});
function paint(){g.clearColor(0.84,0.16,0.63,1);g.clear(g.COLOR_BUFFER_BIT);
 const px=new Uint8Array(4);g.readPixels(1,1,1,1,g.RGBA,g.UNSIGNED_BYTE,px);self.seen=Array.from(px);requestAnimationFrame(paint)}
paint();
async function run(){const out={canvas_readback:self.seen,capture_stream_ok:!!c.captureStream,eme_used:false};
 try{const s=await navigator.mediaDevices.getDisplayMedia({video:true,preferCurrentTab:true});
  const v=document.createElement('video');v.srcObject=s;v.muted=true;await v.play();
  await new Promise(r=>setTimeout(r,1500));
  const k=document.createElement('canvas');k.width=v.videoWidth;k.height=v.videoHeight;
  const x=k.getContext('2d');x.drawImage(v,0,0);const d=x.getImageData(0,0,k.width,k.height).data;
  let hit=0;for(let i=0;i<d.length;i+=4){if(Math.abs(d[i]-214)<14&&Math.abs(d[i+1]-41)<14&&Math.abs(d[i+2]-161)<14)hit++}
  out.display_media={size:[k.width,k.height],canvas_coloured_pixels:hit};s.getTracks().forEach(t=>t.stop())
 }catch(e){out.display_media={error:String(e)}}
 await fetch('/result',{method:'POST',body:JSON.stringify(out)})}
setTimeout(run,800);
</script>"""


def web() -> dict:
    """A WebGL page, the same kind as the engine's, captures itself in Chrome with a throwaway profile."""
    result: dict = {}
    done = threading.Event()

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            self.send_response(200)
            self.send_header("Content-Type", "text/html")
            self.end_headers()
            self.wfile.write(PAGE.encode())

        def do_POST(self):
            result.update(json.loads(self.rfile.read(int(self.headers["Content-Length"]))))
            self.send_response(204)
            self.end_headers()
            done.set()

        def log_message(self, *_):
            pass

    server = HTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as profile:
        process = subprocess.Popen([
            str(CHROME), f"--user-data-dir={profile}", "--no-first-run", "--no-default-browser-check",
            "--auto-select-tab-capture-source-by-title=shuyi web capture probe",
            "--window-size=500,400", f"http://127.0.0.1:{server.server_port}/"])
        try:
            result["finished"] = done.wait(40)
        finally:
            process.kill()
            subprocess.run(["taskkill", "/F", "/T", "/PID", str(process.pid)], capture_output=True, check=False)
            time.sleep(1)
    server.shutdown()
    shell = ROOT / "crates" / "cli" / "web"
    result["shell_hits"] = scan_tree(shell)
    return result


def main() -> None:
    command = sys.argv[1]
    if command == "scan":
        print(json.dumps(scan(), indent=1))
    elif command == "window":
        print(json.dumps(window(*sys.argv[2:6]), indent=1))
    elif command == "web":
        print(json.dumps(web(), indent=1))
    else:
        raise SystemExit(f"unknown command {command}")


if __name__ == "__main__":
    main()

"""Standard-library client for the live Slint viewer's development MCP server."""
from __future__ import annotations

import base64
import json
import os
from pathlib import Path
import socket
import struct
import subprocess
import sys
import tempfile
import time
import urllib.request


def free_port(requested: int = 0) -> int:
    if not 0 <= requested <= 65535:
        raise ValueError("SLINT_MCP_PORT must be a free port between 1 and 65535, or 0 for automatic selection")
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", requested))
        return listener.getsockname()[1]


class ViewerSession:
    def __init__(self, viewer: str, source: Path, evidence: Path, initial_properties=None):
        self.viewer, self.source, self.evidence = viewer, source, evidence
        self.initial_properties = initial_properties
        self.processes = []
        self.serial = 0
        self.http = urllib.request.build_opener(urllib.request.ProxyHandler({}))

    def __enter__(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="dandrum-slint-")
        self.directory = Path(self.temporary.name)
        self.env = os.environ.copy()
        try:
            # Verify even explicitly selected ports before starting anything;
            # an already running MCP server must never receive fixture actions.
            self.port = free_port(int(self.env.get("SLINT_MCP_PORT") or 0))
            self.env.update(SLINT_MCP_PORT=str(self.port), SLINT_EMIT_DEBUG_INFO="1")
            self.env.setdefault("SLINT_BACKEND", "winit-software")
            if sys.platform.startswith("linux") and not (self.env.get("DISPLAY") or self.env.get("WAYLAND_DISPLAY")):
                self.start_virtual_display()
            arguments = [self.viewer, str(self.source)]
            if self.initial_properties is not None:
                data = self.directory / "properties.json"
                data.write_text(json.dumps(self.initial_properties), encoding="utf-8")
                arguments.extend(["--load-data", str(data)])
            self.start("viewer", arguments)
            self.wait_until(lambda: self.connected(self.port))
            windows = self.content(self.call("list_windows"))["windowHandles"]
            if len(windows) != 1:
                raise AssertionError(f"Expected one fixture window, found {len(windows)}")
            self.window = windows[0]
            self.root = self.content(self.call("get_window_properties", windowHandle=self.window))["rootElementHandle"]
            return self
        except BaseException:
            self.__exit__(None, None, None)
            raise

    def start_virtual_display(self):
        executable = self.env.get("DANDRUM_XVFB")
        if not executable:
            raise RuntimeError("No display: run with xvfb-run or set DANDRUM_XVFB to Xvfb; use --check-only for compilation.")
        display = next(number for number in range(100, 200) if not Path(f"/tmp/.X{number}-lock").exists())
        auth = self.directory / "Xauthority"
        with auth.open("wb") as stream:
            stream.write(struct.pack(">H", 65535))
            for value in [b"", str(display).encode(), b"MIT-MAGIC-COOKIE-1", os.urandom(16)]:
                stream.write(struct.pack(">H", len(value)) + value)
        auth.chmod(0o600)
        tcp = self.env.get("DANDRUM_XVFB_TCP") == "1"
        transport = (["-nolisten", "unix", "-nolisten", "local", "-nolisten", "inet6", "-listen", "tcp"]
                     if tcp else ["-nolisten", "tcp"])
        self.env.update(DISPLAY=f"127.0.0.1:{display}" if tcp else f":{display}", XAUTHORITY=str(auth))
        self.start("xvfb", [executable, f":{display}", "-screen", "0", "1920x1200x24", *transport,
                            "-auth", str(auth), "-noreset"])
        self.wait_until(lambda: self.connected(6000 + display) if tcp else Path(f"/tmp/.X11-unix/X{display}").exists())

    @staticmethod
    def connected(port):
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=.1):
                return True
        except OSError:
            return False

    def start(self, name, arguments):
        path = self.evidence / f"{self.source.stem.lower()}-{name}.log"
        stream = path.open("w", encoding="utf-8")
        try:
            process = subprocess.Popen(arguments, env=self.env, stdout=stream, stderr=subprocess.STDOUT)
        except BaseException:
            stream.close()
            raise
        self.processes.append((process, stream, path))

    def wait_until(self, condition):
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            for process, stream, path in self.processes:
                if process.poll() is not None:
                    stream.flush()
                    raise RuntimeError(f"Process exited {process.returncode}: {path.read_text(encoding='utf-8')}")
            if condition():
                return
            time.sleep(.05)
        raise TimeoutError("The Slint viewer did not satisfy the expected condition within 15 seconds")

    def rpc(self, method, parameters):
        self.serial += 1
        body = {"jsonrpc": "2.0", "id": self.serial, "method": method, "params": parameters}
        request = urllib.request.Request(f"http://127.0.0.1:{self.port}/mcp", json.dumps(body).encode(),
                    headers={"Content-Type": "application/json", "Accept": "application/json, text/event-stream"})
        with self.http.open(request, timeout=15) as response:
            raw = response.read().decode()
        if raw.startswith(("event:", "data:")):
            raw = next(line[6:] for line in raw.splitlines() if line.startswith("data: "))
        result = json.loads(raw)
        if "error" in result:
            raise RuntimeError(result["error"])
        return result["result"]

    def call(self, name, **arguments):
        result = self.rpc("tools/call", {"name": name, "arguments": arguments})
        if result.get("isError"):
            raise RuntimeError(result)
        return result

    @staticmethod
    def content(result):
        text = "\n".join(item["text"] for item in result.get("content", []) if item.get("type") == "text")
        try:
            return json.loads(text)
        except json.JSONDecodeError:
            return text

    def find(self, qualified_id):
        handles = self.content(self.call("find_elements_by_id", windowHandle=self.window,
                                       elementsId=qualified_id))["elementHandles"]
        if len(handles) != 1:
            raise AssertionError(f"Expected one {qualified_id}, found {len(handles)}")
        return handles[0]

    def properties(self, handle):
        return self.content(self.call("get_element_properties", elementHandle=handle))

    def tree(self):
        return self.content(self.call("get_element_tree", elementHandle=self.root, maxElements=1000))

    def key(self, value, event="PressAndRelease"):
        self.call("dispatch_key_event", windowHandle=self.window, text=value, eventType=event)

    def screenshot(self, filename):
        result = self.call("take_screenshot", windowHandle=self.window)
        image = next(item for item in result["content"] if item["type"] == "image")
        (self.evidence / filename).write_bytes(base64.b64decode(image["data"]))

    def __exit__(self, exc_type, exc_value, traceback):
        if exc_type in (AssertionError, TimeoutError) and hasattr(self, "window"):
            try:
                self.screenshot(f"{self.source.stem.lower()}-failure.png")
            except (OSError, RuntimeError, KeyError, StopIteration):
                pass
        for process, stream, _ in reversed(self.processes):
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
            stream.close()
        self.temporary.cleanup()

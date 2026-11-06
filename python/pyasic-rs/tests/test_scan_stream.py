import asyncio
import json
import os
import subprocess
import sys
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from threading import Thread
from urllib.parse import urlsplit

from pyasic_rs import MinerFactory

FAST, SLOW = "127.0.0.2", "127.0.0.3"


def test_scan_stream_keeps_identifying_while_consumer_is_busy() -> None:
    # The SDK caches its HTTP client, including proxy configuration.
    subprocess.run([sys.executable, str(Path(__file__).resolve())], check=True, timeout=60)


def check_scan_stream_keeps_identifying_while_consumer_is_busy() -> None:
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self) -> None:
            if urlsplit(self.path).path != "/":
                self.send_error(404)
                return
            self.reply(b"<html><title>Braiins OS</title></html>", "text/html")

        def do_POST(self) -> None:
            target = urlsplit(self.path)
            if target.path != "/graphql":
                self.send_error(404)
                return
            query = json.loads(self.rfile.read(int(self.headers["Content-Length"])))["query"]
            if target.hostname == SLOW:
                time.sleep(1)
            if "modelName" in query:
                data = {"bosminer": {"info": {"modelName": "Antminer S19j XP"}}}
            else:
                data = {"bos": {"info": {"version": {"full": "2025-03-04-0-abc-25.03-plus"}}}}
            self.reply(json.dumps({"data": data}).encode(), "application/json")

        def reply(self, body: bytes, content_type: str) -> None:
            self.send_response(200)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, format: str, *args: object) -> None:
            pass

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    server.daemon_threads = True
    proxy = f"http://127.0.0.1:{server.server_port}"
    for name in ("HTTP_PROXY", "http_proxy"):
        os.environ[name] = proxy
    for name in ("NO_PROXY", "no_proxy"):
        os.environ[name] = ""
    thread = Thread(target=server.serve_forever, daemon=True)
    thread.start()

    async def check() -> None:
        factory = MinerFactory.from_range("127.0.0.2-3").with_port_check(False)
        found: list[str] = []
        async for ip, miner in factory.scan_stream_with_ip():
            if miner is None:
                continue
            if not found:
                # Block the event loop past the 5 s discovery HTTP deadline while
                # the slow miner is still being identified.
                time.sleep(6)
            found.append(str(ip))
        assert sorted(found) == [FAST, SLOW], found

    try:
        asyncio.run(check())
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2)


if __name__ == "__main__":
    check_scan_stream_keeps_identifying_while_consumer_is_busy()

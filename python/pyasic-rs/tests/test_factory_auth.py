import asyncio
import hashlib
import json
import os
import subprocess
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from threading import Thread
from urllib.parse import urlsplit
from urllib.request import parse_http_list, parse_keqv_list

import pytest

from pyasic_rs import MinerFactory
from pyasic_rs.data import DataField


def test_braiins_custom_discovery_auth_is_chainable() -> None:
    factory = MinerFactory()
    assert factory.with_firmware_discovery_auth("Braiins", "root", "custom-password") is factory


def test_invalid_discovery_auth_preserves_factory() -> None:
    factory = MinerFactory()
    with pytest.raises(ValueError, match="^Firmware is not registered for discovery authentication$"):
        factory.with_firmware_discovery_auth("Unknown Firmware", "root", "custom-password")
    assert factory.with_firmware_discovery_auth("AntMiner Stock", "root", "custom-password") is factory


def test_stock_custom_credentials_discover_and_read_protected_telemetry() -> None:
    # The SDK caches its HTTP client, including proxy configuration.
    subprocess.run([sys.executable, str(Path(__file__).resolve())], check=True, timeout=20)


def check_stock_custom_credentials() -> None:
    username, password = "operator", "custom&password=+"
    realm, nonce = "antMiner Configuration", "factory-auth-regression"
    authenticated_paths: list[str] = []
    payloads = {
        "/cgi-bin/miner_type.cgi": {"miner_type": "Antminer S21"},
        "/cgi-bin/summary.cgi": {"INFO": {"CompileTime": "Fri Jul 7 11:39:06 CST 2023"}},
        "/cgi-bin/get_system_info.cgi": {"macaddr": "00:11:22:33:44:55"},
    }

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self) -> None:
            target = urlsplit(self.path)
            if target.scheme != "http" or target.hostname != "127.0.0.2" or target.port not in (None, 80):
                self.send_error(400)
                return
            path = target.path or "/"
            authorization = self.headers.get("Authorization", "")
            fields = (
                parse_keqv_list(parse_http_list(authorization[7:]))
                if authorization.startswith("Digest ")
                else {}
            )
            ha1 = hashlib.md5(f"{username}:{realm}:{password}".encode()).hexdigest()
            ha2 = hashlib.md5(f"GET:{path}".encode()).hexdigest()
            expected = hashlib.md5(
                f"{ha1}:{nonce}:{fields.get('nc')}:{fields.get('cnonce')}:auth:{ha2}".encode()
            ).hexdigest()
            authorized = (
                fields.get("username") == username
                and fields.get("realm") == realm
                and fields.get("nonce") == nonce
                and fields.get("uri") == path
                and fields.get("qop") == "auth"
                and fields.get("response") == expected
            )
            if path == "/" or not authorized:
                self.send_response(401)
                self.send_header(
                    "WWW-Authenticate",
                    f'Digest realm="{realm}", nonce="{nonce}", algorithm=MD5, qop="auth"',
                )
                self.send_header("Content-Length", "0")
                self.end_headers()
                return

            authenticated_paths.append(path)
            body = json.dumps(payloads.get(path, {})).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
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
        default_factory = MinerFactory().with_identification_timeout_secs(3)
        assert await asyncio.wait_for(default_factory.get_miner("127.0.0.2"), 5) is None
        wrong_factory = MinerFactory().with_firmware_discovery_auth(
            "AntMiner Stock", username, "wrong-password"
        ).with_identification_timeout_secs(3)
        assert await asyncio.wait_for(wrong_factory.get_miner("127.0.0.2"), 5) is None
        assert not authenticated_paths

        factory = MinerFactory().with_identification_timeout_secs(3)
        assert factory.with_firmware_discovery_auth("AntMiner Stock", username, password) is factory
        miner = await asyncio.wait_for(factory.get_miner("127.0.0.2"), 5)
        assert miner is not None
        assert miner.model == "S21"
        exclude = [
            DataField.model_validate(field)
            for field in DataField.model_json_schema()["enum"]
            if field != "Mac"
        ]
        data = await asyncio.wait_for(miner.get_data(exclude), 5)
        assert str(data.mac) == "00:11:22:33:44:55"
        assert set(authenticated_paths) == set(payloads)

    try:
        asyncio.run(check())
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2)


if __name__ == "__main__":
    check_stock_custom_credentials()

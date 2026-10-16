"""load.py 自测：不依赖内核，用标准库 mock 服务验证 HTTP / WS 客户端与统计逻辑。

    python3 -m unittest discover -s scripts/perf -p 'test_*.py'
"""

from __future__ import annotations

import asyncio
import base64
import hashlib
import json
import os
import sys
import tempfile
import time
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import load  # noqa: E402

WS_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"


class MockKernel:
    """极简内核替身：setup / login / refresh / GET JSON（交替 chunked）/ /ws/metrics。"""

    def __init__(self, ttl: float = 1e9):
        self.ttl = ttl  # access token 有效期（秒），模拟内核 15 min TTL
        self.issued: dict[str, float] = {}
        self.gets = 0
        self.refreshes = 0
        self.ws_auth: list[dict] = []
        self.token = "tok-1"
        self.prev_token = ""
        self.stats_hits = 0

    async def handle(self, r: asyncio.StreamReader, w: asyncio.StreamWriter):
        try:
            while True:
                head = await r.readuntil(b"\r\n\r\n")
                lines = head.decode().split("\r\n")
                method, path, _ = lines[0].split(" ", 2)
                h = {k.lower(): v.strip() for k, _, v in (l.partition(":") for l in lines[1:] if l)}
                body = await r.readexactly(int(h.get("content-length", "0")))
                if path == "/ws/metrics":
                    await self.ws(r, w, h)
                    return
                await self.http(w, method, path, h, body)
        except (asyncio.IncompleteReadError, ConnectionError):
            pass
        finally:
            w.close()

    async def http(self, w, method, path, h, body):
        extra = ""
        if path == "/api/setup/initialize":
            step = json.loads(body)["step"]
            if step == "admin":
                self.issued[self.token] = time.monotonic()
            payload = {"ok": True} if step == "database" else {"token": self.token, "refresh_token": "r1"}
        elif path == "/api/auth/refresh":
            assert h.get("cookie", "").startswith("flame_refresh="), h
            self.refreshes += 1
            # 与内核一致：refresh 只轮换 refresh token，旧 access token 在过期前仍有效
            self.prev_token, self.token = self.token, f"tok-{self.refreshes + 1}"
            self.issued[self.token] = time.monotonic()
            payload = {"token": self.token, "refresh_token": "r2"}
            extra = "Set-Cookie: flame_refresh=r2; HttpOnly; Path=/api/auth\r\n"
        elif method == "GET":
            tok = h.get("authorization", "").removeprefix("Bearer ")
            if tok not in (self.token, self.prev_token) or time.monotonic() - self.issued.get(tok, 0) > self.ttl:
                return self.send(w, 401, b'{"error":"unauthorized"}')
            self.gets += 1
            if path.startswith("/api/users"):
                return self.send(w, 500, b'{"error":"boom"}')
            if path == "/api/kernel-logs":
                payload = {"data": {"enabled": True, "files": [
                    {"name": "flamepanel.2026-09-24.log", "current": False},
                    {"name": "flamepanel.2026-09-25.log", "current": True}]}}
                self.send(w, 200, json.dumps(payload).encode())
                return await w.drain()
            if path.startswith("/api/kernel-logs/stats"):
                # 与内核一致：file 为必填参数，缺失即 400
                if "file=flamepanel.2026-09-25.log" not in path:
                    return self.send(w, 400, b'{"error":"missing file"}')
                self.stats_hits += 1
            payload = {"data": {"path": path, "n": self.gets}}
            if self.gets % 2 == 0:  # 交替用 chunked 编码，覆盖两种响应体解析
                data = json.dumps(payload).encode()
                mid = len(data) // 2
                chunks = b"".join(b"%x\r\n%s\r\n" % (len(c), c) for c in (data[:mid], data[mid:]))
                w.write(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n"
                        b"Transfer-Encoding: chunked\r\n\r\n" + chunks + b"0\r\n\r\n")
                await w.drain()
                return
        else:
            return self.send(w, 404, b"{}")
        self.send(w, 200, json.dumps(payload).encode(), extra)
        await w.drain()

    @staticmethod
    def send(w, status, data: bytes, extra: str = ""):
        w.write(f"HTTP/1.1 {status} X\r\nContent-Type: application/json\r\n{extra}"
                f"Content-Length: {len(data)}\r\n\r\n".encode() + data)

    async def ws(self, r, w, h):
        accept = base64.b64encode(hashlib.sha1((h["sec-websocket-key"] + WS_GUID).encode()).digest()).decode()
        w.write(("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n"
                 f"Sec-WebSocket-Accept: {accept}\r\n\r\n").encode())
        op, data = await load.ws_read_frame(r)  # 客户端帧带掩码
        assert op == 0x1
        self.ws_auth.append(json.loads(data))
        w.write(bytes([0x89, 0]))  # Ping（服务端帧无掩码）
        for i in range(3):
            msg = json.dumps({"type": "tick", "i": i}).encode()
            w.write(bytes([0x81, len(msg)]) + msg)
        await w.drain()
        op, _ = await load.ws_read_frame(r)
        assert op == 0xA, "客户端必须回 Pong"
        w.write(bytes([0x88, 0]))  # Close → 客户端应重连
        await w.drain()


class PureFunctions(unittest.TestCase):
    def test_percentiles(self):
        data = list(range(1, 101))
        self.assertAlmostEqual(load.pct(data, 50), 50.5)
        self.assertAlmostEqual(load.pct(data, 99), 99.01)
        self.assertEqual(load.pct([], 95), 0.0)

    def test_slope(self):
        pts = [(t, 50 + t / 600.0 * 2) for t in range(0, 1800, 5)]  # 2 MB / 10min
        self.assertAlmostEqual(load.slope_per_10min(pts), 2.0, places=6)
        self.assertEqual(load.slope_per_10min([(0, 1)]), 0.0)

    def test_ws_frame_roundtrip(self):
        async def go():
            for n in (5, 300, 70000):
                payload = os.urandom(n)
                r = asyncio.StreamReader()
                r.feed_data(load.ws_frame(0x2, payload))
                r.feed_eof()
                op, data = await load.ws_read_frame(r)
                self.assertEqual((op, data), (0x2, payload))
        asyncio.run(go())

    def test_evaluate_thresholds(self):
        args = load.parse_args([])
        base = {"errors": 0, "requests": 10, "error_rate": 0.0, "duration_s": 1800,
                "process": {"died": False, "rss_mb_max": 40.0, "rss_slope_mb_per_10min": 0.2,
                            "fds_first": 30, "fds_last": 31}}
        ok = load.evaluate([{**base, "scenario": "idle"}, {**base, "scenario": "soak"}], args)
        self.assertTrue(all(c["pass"] for c in ok), ok)
        leak = {**base, "scenario": "soak", "process": {**base["process"], "rss_slope_mb_per_10min": 3.0}}
        self.assertFalse(all(c["pass"] for c in load.evaluate([leak], args)))
        short = {**leak, "duration_s": 120}  # --quick：斜率仅记录
        self.assertTrue(all(c["pass"] for c in load.evaluate([short], args)))


class AgainstMock(unittest.TestCase):
    def test_end_to_end(self):
        # 与生产同构（idle 300 s < 续期间隔 600 s < TTL 900 s）：idle 1.2 s < 续期 1.5 s < TTL 2.0 s。
        # 会话级计时：t=1.5 / 3.0 续期，token 最大年龄 1.5 s < TTL，全程无 401；
        # 若按场景计时（旧缺陷）：idle 内不续期，load 起点（t=1.2）重新计时到 t=2.7 才续期，
        # 而 token 在 t=2.0 已过期 → 必然出现 401（已用变异测试确认可捕获）
        mock = MockKernel(ttl=2.0)

        async def go():
            srv = await asyncio.start_server(mock.handle, "127.0.0.1", 0)
            port = srv.sockets[0].getsockname()[1]
            out = tempfile.mkdtemp()
            args = load.parse_args([
                "--base-url", f"http://127.0.0.1:{port}", "--setup", "--pid", str(os.getpid()),
                "--scenarios", "idle,load", "--idle-secs", "1.2", "--load-secs", "3.0",
                "--concurrency", "8", "--ws-clients", "2", "--think", "0.01",
                "--sample-interval", "0.5", "--refresh-every", "1.5", "--out", out,
            ])
            async with srv:
                rc = await load.main_async(args)
            return rc, out

        rc, out = asyncio.run(go())
        with open(os.path.join(out, "perf-report.json")) as f:
            rep = json.load(f)
        self.assertTrue(os.path.exists(os.path.join(out, "perf-report.md")))
        idle, ld = rep["scenarios"]
        # /api/users 被 mock 成 500 → 必须计为错误且整体判 FAIL
        self.assertEqual(rc, 1)
        self.assertGreater(ld["requests"], 50)
        self.assertTrue(any("HTTP 500 /api/users" in k for k in ld["error_kinds"]), ld["error_kinds"])
        self.assertEqual([k for k in ld["error_kinds"] if "500" not in k], [], "除注入的 500 外不应有错误")
        # 续期后新 token 必须被 worker 使用（否则会出现 401）
        self.assertGreaterEqual(mock.refreshes, 2)
        self.assertEqual(rep["auth"]["refresh_failures"], [])
        # 内核日志统计必须带上概览里的 current 文件名；报告附带五级计数
        self.assertGreater(mock.stats_hits, 0)
        self.assertNotIn("unavailable", rep.get("kernel_log_counts", {}))
        # WS：首帧鉴权 + Pong + Close 后重连计入 reconnects 而非错误
        self.assertTrue(all(a["type"] == "auth" and a["token"].startswith("tok-") for a in mock.ws_auth))
        self.assertGreater(ld["ws"]["messages"], 0)
        self.assertGreater(ld["ws"]["reconnects"], 0)
        self.assertEqual(ld["ws"]["errors"], {})
        # /proc 采样（采本测试进程自身）
        self.assertIsNotNone(idle["process"]["rss_mb_max"])
        self.assertIn("p95", ld["latency_ms"])


if __name__ == "__main__":
    unittest.main()

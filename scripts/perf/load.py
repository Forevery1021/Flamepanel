#!/usr/bin/env python3
"""Flamepanel 性能 / 稳定性压测驱动（Doc/32 §8 · W2-b）。

纯标准库（asyncio + 手写 HTTP/1.1 keep-alive 与最小 WebSocket 客户端），不引入任何依赖，
可在 CNB 云原生开发环境、CI 容器、用户服务器上直接运行。

场景（按顺序执行，可用 --scenarios 选择）：
  idle  —— 仅保持 1 个 /ws/metrics 连接，观测空载 RSS / CPU（基线）
  load  —— N 个并发 HTTP worker 轮询常用只读 API + M 个 WS 客户端（峰值并发）
  soak  —— 中等并发长时间运行，观测 RSS 斜率（内存泄漏）与 fd / 线程增长

采样：每 --sample-interval 秒读 /proc/<pid>/{status,stat,fd}（RSS / 线程 / CPU / fd）。
输出：<out>/perf-report.json + <out>/perf-report.md；阈值不达标时退出码 1。

鉴权：
  --setup      对全新实例走 /api/setup/initialize（database → admin）拿 token（2 次登录档配额）
  --username/--password  对已初始化实例走 /api/auth/login（1 次）
  Access Token 15 min 过期：后台每 --refresh-every 秒用 flame_refresh cookie 续期；
  WS 端每分钟复核 token，过期即断开——客户端按前端同样的方式用新 token 重连，
  这类断开计入 ws.reconnects 而**不**计入错误。

注意：登录档限流 5 次/分钟（按 IP，写死）；API 档默认 120 次/分钟——压测实例须设
OP_RATE_LIMIT_MAX 足够大（scripts/perf/run.sh 已处理），否则 429 会被计为错误。
"""

from __future__ import annotations

import argparse
import asyncio
import base64
import json
import os
import random
import ssl
import statistics
import sys
import time
from dataclasses import dataclass, field
from urllib.parse import quote, urlsplit

# ── 请求组合：前端各页面真实会轮询 / 打开即拉取的只读端点（权重≈访问频度） ──────────
ENDPOINT_MIX: list[tuple[str, int]] = [
    ("/api/overview/badges", 6),  # 顶栏徽标：每页常驻轮询
    ("/api/auth/me", 3),
    ("/api/metrics/history?range=1h", 3),  # 仪表盘曲线
    ("/api/kernel-logs", 1),  # 内核日志页概览（文件列表）
    ("/api/kernel-logs/stats?file={log_file}", 2),  # file 必填：与前端一致，取概览中 current 文件
    ("/api/nodes", 2),
    ("/api/memos", 2),
    ("/api/tasks", 2),
    ("/api/websites", 1),
    ("/api/settings", 1),
    ("/api/users", 1),
    ("/api/operation-logs", 1),
    ("/api/scheduled-tasks", 1),
    ("/api/kernel-logs/entries?limit=50", 1),
    ("/api/metrics/processes", 1),  # 最重：sysinfo 全进程扫描
]


# ── 统计 ────────────────────────────────────────────────────────────────────────
@dataclass
class Stats:
    latencies: list[float] = field(default_factory=list)  # 秒
    per_endpoint: dict[str, list[float]] = field(default_factory=dict)
    errors: dict[str, int] = field(default_factory=dict)
    requests: int = 0
    ws_messages: int = 0
    ws_reconnects: int = 0
    retried_after_refresh: int = 0
    ws_errors: dict[str, int] = field(default_factory=dict)

    def ok(self, path: str, dt: float) -> None:
        self.requests += 1
        self.latencies.append(dt)
        self.per_endpoint.setdefault(path.split("?")[0], []).append(dt)

    def err(self, kind: str) -> None:
        self.requests += 1
        self.errors[kind] = self.errors.get(kind, 0) + 1

    def ws_err(self, kind: str) -> None:
        self.ws_errors[kind] = self.ws_errors.get(kind, 0) + 1


def pct(data: list[float], p: float) -> float:
    if not data:
        return 0.0
    s = sorted(data)
    k = (len(s) - 1) * p / 100.0
    lo, hi = int(k), min(int(k) + 1, len(s) - 1)
    return s[lo] + (s[hi] - s[lo]) * (k - lo)


def slope_per_10min(points: list[tuple[float, float]]) -> float:
    """最小二乘斜率（y 单位 / 10 分钟）；点数 < 3 返回 0。"""
    if len(points) < 3:
        return 0.0
    xs = [p[0] for p in points]
    ys = [p[1] for p in points]
    mx, my = statistics.fmean(xs), statistics.fmean(ys)
    den = sum((x - mx) ** 2 for x in xs)
    if den == 0:
        return 0.0
    return sum((x - mx) * (y - my) for x, y in zip(xs, ys)) / den * 600.0


# ── 最小 HTTP/1.1 keep-alive 客户端 ───────────────────────────────────────────────
class HttpConn:
    def __init__(self, host: str, port: int, tls: bool):
        self.host, self.port, self.tls = host, port, tls
        self.r: asyncio.StreamReader | None = None
        self.w: asyncio.StreamWriter | None = None

    async def _open(self) -> None:
        ctx = ssl.create_default_context() if self.tls else None
        self.r, self.w = await asyncio.open_connection(self.host, self.port, ssl=ctx)

    async def close(self) -> None:
        if self.w:
            self.w.close()
            try:
                await self.w.wait_closed()
            except Exception:
                pass
        self.r = self.w = None

    async def request(
        self, method: str, path: str, headers: dict[str, str] | None = None, body: bytes = b""
    ) -> tuple[int, dict[str, str], bytes]:
        for attempt in (0, 1):  # keep-alive 连接被对端关闭时重开一次
            if self.w is None:
                await self._open()
            try:
                return await self._do(method, path, headers or {}, body)
            except (ConnectionError, asyncio.IncompleteReadError):
                await self.close()
                if attempt:
                    raise
        raise RuntimeError("unreachable")

    async def _do(self, method, path, headers, body):
        assert self.r and self.w
        lines = [f"{method} {path} HTTP/1.1", f"Host: {self.host}:{self.port}"]
        h = {"Connection": "keep-alive", "Accept": "application/json", **headers}
        if body:
            h["Content-Length"] = str(len(body))
        lines += [f"{k}: {v}" for k, v in h.items()]
        self.w.write(("\r\n".join(lines) + "\r\n\r\n").encode() + body)
        await self.w.drain()
        status_line = await self.r.readuntil(b"\r\n")
        if not status_line:
            raise ConnectionError("empty response")
        status = int(status_line.split()[1])
        resp_h: dict[str, str] = {}
        cookies: list[str] = []
        while True:
            line = await self.r.readuntil(b"\r\n")
            if line == b"\r\n":
                break
            k, _, v = line.decode("latin-1").partition(":")
            k, v = k.strip().lower(), v.strip()
            if k == "set-cookie":
                cookies.append(v)
            resp_h[k] = v
        if cookies:
            resp_h["set-cookie"] = "\n".join(cookies)
        if resp_h.get("transfer-encoding", "").lower() == "chunked":
            chunks = []
            while True:
                size = int((await self.r.readuntil(b"\r\n")).strip().split(b";")[0], 16)
                if size == 0:
                    await self.r.readuntil(b"\r\n")
                    break
                chunks.append(await self.r.readexactly(size))
                await self.r.readexactly(2)
            data = b"".join(chunks)
        else:
            data = await self.r.readexactly(int(resp_h.get("content-length", "0")))
        if resp_h.get("connection", "").lower() == "close":
            await self.close()
        return status, resp_h, data


# ── 会话（token + refresh cookie） ────────────────────────────────────────────────
class Session:
    def __init__(self, base: str):
        u = urlsplit(base)
        self.tls = u.scheme == "https"
        self.host = u.hostname or "127.0.0.1"
        self.port = u.port or (443 if self.tls else 80)
        self.token = ""
        self.refresh_cookie = ""
        self.log_file = ""  # 当前内核日志文件名（discover() 填充）
        self.token_at = time.monotonic()  # 当前 access token 获取时刻（会话级续期计时）
        self.refreshes = 0

    def conn(self) -> HttpConn:
        return HttpConn(self.host, self.port, self.tls)

    def _absorb(self, headers: dict[str, str], data: bytes) -> None:
        j = json.loads(data or b"{}")
        payload = j.get("data", j) if isinstance(j, dict) else {}
        if payload.get("token"):
            self.token = payload["token"]
            self.token_at = time.monotonic()
        for c in headers.get("set-cookie", "").split("\n"):
            if c.startswith("flame_refresh="):
                self.refresh_cookie = c.split(";")[0]
        if not self.refresh_cookie and payload.get("refresh_token"):
            self.refresh_cookie = f"flame_refresh={payload['refresh_token']}"

    async def post_json(self, path: str, obj: dict, extra: dict[str, str] | None = None):
        c = self.conn()
        try:
            h = {"Content-Type": "application/json", **(extra or {})}
            return await c.request("POST", path, h, json.dumps(obj).encode())
        finally:
            await c.close()

    async def setup(self, username: str, password: str) -> None:
        extra: dict[str, str] = {}
        s, _, d = await self.post_json(
            "/api/setup/initialize", {"step": "database", "database": {"db_type": "sqlite"}}, extra
        )
        if s >= 300:
            raise SystemExit(f"setup(database) 失败 HTTP {s}: {d[:300]!r}")
        s, h, d = await self.post_json(
            "/api/setup/initialize",
            {"step": "admin", "admin": {"username": username, "password": password}},
            extra,
        )
        if s >= 300:
            raise SystemExit(f"setup(admin) 失败 HTTP {s}: {d[:300]!r}")
        self._absorb(h, d)

    async def login(self, username: str, password: str) -> None:
        s, h, d = await self.post_json("/api/auth/login", {"username": username, "password": password})
        if s >= 300:
            raise SystemExit(f"login 失败 HTTP {s}: {d[:300]!r}")
        self._absorb(h, d)

    async def discover(self) -> None:
        """像前端内核日志页一样先取概览，拿到 current 文件名（stats 端点的必填参数）。"""
        c = self.conn()
        try:
            st, _, d = await c.request("GET", "/api/kernel-logs", {"Authorization": f"Bearer {self.token}"})
        finally:
            await c.close()
        if st < 300:
            j = json.loads(d)
            ov = j.get("data", j) if isinstance(j, dict) else {}
            cur = [f for f in ov.get("files", []) if f.get("current")] or ov.get("files", [])[:1]
            self.log_file = cur[0]["name"] if cur else ""

    async def refresh(self) -> bool:
        if not self.refresh_cookie:
            return False
        s, h, d = await self.post_json("/api/auth/refresh", {}, {"Cookie": self.refresh_cookie})
        if s < 300:
            self._absorb(h, d)
            self.refreshes += 1
            return True
        return False


# ── 最小 WebSocket 客户端（RFC 6455，客户端帧必须掩码） ─────────────────────────────
def ws_frame(opcode: int, payload: bytes) -> bytes:
    head = bytearray([0x80 | opcode])
    n = len(payload)
    if n < 126:
        head.append(0x80 | n)
    elif n < 65536:
        head += bytes([0x80 | 126]) + n.to_bytes(2, "big")
    else:
        head += bytes([0x80 | 127]) + n.to_bytes(8, "big")
    mask = os.urandom(4)
    return bytes(head) + mask + bytes(b ^ mask[i % 4] for i, b in enumerate(payload))


async def ws_read_frame(r: asyncio.StreamReader) -> tuple[int, bytes]:
    b1, b2 = await r.readexactly(2)
    n = b2 & 0x7F
    if n == 126:
        n = int.from_bytes(await r.readexactly(2), "big")
    elif n == 127:
        n = int.from_bytes(await r.readexactly(8), "big")
    mask = await r.readexactly(4) if b2 & 0x80 else b""
    data = await r.readexactly(n)
    if mask:
        data = bytes(b ^ mask[i % 4] for i, b in enumerate(data))
    return b1 & 0x0F, data


async def ws_client(sess: Session, path: str, stats: Stats, stop: asyncio.Event) -> None:
    """保持一条推送流；被服务端关闭（token 过期复核 / resync）后用最新 token 重连。"""
    first = True
    while not stop.is_set():
        w = None
        try:
            ctx = ssl.create_default_context() if sess.tls else None
            r, w = await asyncio.open_connection(sess.host, sess.port, ssl=ctx)
            key = base64.b64encode(os.urandom(16)).decode()
            w.write(
                (
                    f"GET {path} HTTP/1.1\r\nHost: {sess.host}:{sess.port}\r\n"
                    "Upgrade: websocket\r\nConnection: Upgrade\r\n"
                    f"Sec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n"
                ).encode()
            )
            await w.drain()
            head = await r.readuntil(b"\r\n\r\n")
            if b" 101 " not in head.split(b"\r\n")[0]:
                stats.ws_err("handshake:" + head.split(b"\r\n")[0].decode(errors="replace"))
                await asyncio.sleep(1)
                continue
            if not first:
                stats.ws_reconnects += 1
            first = False
            w.write(ws_frame(0x1, json.dumps({"type": "auth", "token": sess.token}).encode()))
            await w.drain()
            while not stop.is_set():
                try:
                    op, data = await asyncio.wait_for(ws_read_frame(r), timeout=1.0)
                except asyncio.TimeoutError:
                    continue
                if op == 0x9:  # Ping → Pong
                    w.write(ws_frame(0xA, data))
                    await w.drain()
                elif op == 0x8:  # Close：服务端主动（token 复核 / resync）→ 重连
                    break
                elif op in (0x1, 0x2):
                    stats.ws_messages += 1
        except (ConnectionError, asyncio.IncompleteReadError, OSError) as e:
            if not stop.is_set():
                stats.ws_err(type(e).__name__)
                await asyncio.sleep(0.5)
        finally:
            if w:
                w.close()


# ── HTTP worker ───────────────────────────────────────────────────────────────────
def build_mix(log_file: str = "") -> list[str]:
    """展开权重；需要当前日志文件名的端点在文件日志未启用时剔除（而不是制造 400）。"""
    out: list[str] = []
    for path, weight in ENDPOINT_MIX:
        if "{log_file}" in path:
            if not log_file:
                continue
            path = path.replace("{log_file}", quote(log_file))
        out += [path] * weight
    return out


async def http_worker(sess: Session, stats: Stats, stop: asyncio.Event, think: float) -> None:
    mix = build_mix(sess.log_file)
    conn = sess.conn()
    try:
        while not stop.is_set():
            path = random.choice(mix)
            t0 = time.perf_counter()
            try:
                tok = sess.token
                status, _, _ = await asyncio.wait_for(
                    conn.request("GET", path, {"Authorization": f"Bearer {tok}"}), 30
                )
                # 续期竞态：请求带着旧 token 发出、途中 token 已轮换 → 与前端一致，用新 token 重试一次。
                # 仅在 token 确实变化时重试，真正的 401 仍计为错误。
                if status == 401 and sess.token != tok:
                    stats.retried_after_refresh += 1
                    status, _, _ = await asyncio.wait_for(
                        conn.request("GET", path, {"Authorization": f"Bearer {sess.token}"}), 30
                    )
                dt = time.perf_counter() - t0
                if 200 <= status < 300:
                    stats.ok(path, dt)
                else:
                    stats.err(f"HTTP {status} {path.split('?')[0]}")
            except asyncio.TimeoutError:
                stats.err("timeout")
                await conn.close()
            except Exception as e:  # noqa: BLE001 — 任何异常都是一次失败请求
                stats.err(type(e).__name__)
                await conn.close()
            if think:
                await asyncio.sleep(random.uniform(0, 2 * think))
    finally:
        await conn.close()


# ── /proc 采样 ────────────────────────────────────────────────────────────────────
CLK_TCK = os.sysconf("SC_CLK_TCK") if hasattr(os, "sysconf") else 100


def read_proc(pid: int) -> dict[str, float] | None:
    try:
        with open(f"/proc/{pid}/status") as f:
            st = f.read()
        with open(f"/proc/{pid}/stat") as f:
            fields = f.read().rsplit(")", 1)[1].split()
        rss_kb = next(int(l.split()[1]) for l in st.splitlines() if l.startswith("VmRSS:"))
        threads = next(int(l.split()[1]) for l in st.splitlines() if l.startswith("Threads:"))
        cpu_ticks = int(fields[11]) + int(fields[12])  # utime + stime
        fds = len(os.listdir(f"/proc/{pid}/fd"))
        return {"rss_mb": rss_kb / 1024.0, "threads": threads, "cpu_ticks": cpu_ticks, "fds": fds}
    except (OSError, StopIteration, IndexError, ValueError):
        return None


async def sampler(pid: int, interval: float, out: list[dict], stop: asyncio.Event) -> None:
    t_start = time.monotonic()
    prev: tuple[float, int] | None = None
    while not stop.is_set():
        now = time.monotonic()
        s = read_proc(pid)
        if s is None:
            out.append({"t": now - t_start, "dead": True})
            return
        cpu = 0.0
        if prev:
            cpu = (s["cpu_ticks"] - prev[1]) / CLK_TCK / max(now - prev[0], 1e-6) * 100.0
        prev = (now, int(s["cpu_ticks"]))
        out.append({"t": round(now - t_start, 2), "rss_mb": round(s["rss_mb"], 2),
                    "cpu_pct": round(cpu, 1), "fds": s["fds"], "threads": s["threads"]})
        try:
            await asyncio.wait_for(stop.wait(), timeout=interval)
        except asyncio.TimeoutError:
            pass


# ── 场景执行 ──────────────────────────────────────────────────────────────────────
async def run_scenario(name: str, sess: Session, args, http_n: int, ws_n: int, duration: float,
                       think: float) -> dict:
    stats = Stats()
    samples: list[dict] = []
    stop = asyncio.Event()
    tasks = [asyncio.create_task(sampler(args.pid, args.sample_interval, samples, stop))] if args.pid else []
    tasks += [asyncio.create_task(ws_client(sess, "/ws/metrics", stats, stop)) for _ in range(ws_n)]
    tasks += [asyncio.create_task(http_worker(sess, stats, stop, think)) for _ in range(http_n)]

    print(f"[perf] ▶ {name}: http={http_n} ws={ws_n} duration={duration:.0f}s", flush=True)
    t0 = time.monotonic()
    try:
        while time.monotonic() - t0 < duration:
            await asyncio.sleep(min(10.0, duration - (time.monotonic() - t0)) or 0.01)
            el = time.monotonic() - t0
            rss = samples[-1].get("rss_mb", "?") if samples else "?"
            print(f"[perf]   {name} {el:5.0f}s req={stats.requests} err={sum(stats.errors.values())} "
                  f"ws_msg={stats.ws_messages} rss={rss}MB", flush=True)
            if samples and samples[-1].get("dead"):
                break
    finally:
        stop.set()
        await asyncio.gather(*tasks, return_exceptions=True)
    elapsed = time.monotonic() - t0
    return summarize(name, stats, samples, elapsed, http_n, ws_n)


def summarize(name, stats: Stats, samples, elapsed, http_n, ws_n) -> dict:
    alive = [s for s in samples if not s.get("dead")]
    rss = [s["rss_mb"] for s in alive]
    # 斜率：跳过前 20% 预热（连接池 / 缓存填充），只看稳态
    steady = alive[int(len(alive) * 0.2):]
    lat_ms = [x * 1000 for x in stats.latencies]
    n_err = sum(stats.errors.values())
    return {
        "scenario": name,
        "duration_s": round(elapsed, 1),
        "http_workers": http_n,
        "ws_clients": ws_n,
        "requests": stats.requests,
        "rps": round(stats.requests / elapsed, 1) if elapsed else 0,
        "errors": n_err,
        "error_rate": round(n_err / stats.requests, 6) if stats.requests else 0.0,
        "error_kinds": stats.errors,
        "retried_after_refresh": stats.retried_after_refresh,
        "latency_ms": {
            "p50": round(pct(lat_ms, 50), 2), "p95": round(pct(lat_ms, 95), 2),
            "p99": round(pct(lat_ms, 99), 2), "max": round(max(lat_ms), 2) if lat_ms else 0,
        },
        "endpoints_p95_ms": {k: round(pct([x * 1000 for x in v], 95), 2)
                             for k, v in sorted(stats.per_endpoint.items())},
        "ws": {"messages": stats.ws_messages, "reconnects": stats.ws_reconnects,
               "errors": stats.ws_errors},
        "process": {
            "died": any(s.get("dead") for s in samples),
            "rss_mb_min": round(min(rss), 1) if rss else None,
            "rss_mb_max": round(max(rss), 1) if rss else None,
            "rss_mb_last": round(rss[-1], 1) if rss else None,
            "rss_slope_mb_per_10min": round(slope_per_10min([(s["t"], s["rss_mb"]) for s in steady]), 3),
            "cpu_pct_avg": round(statistics.fmean([s["cpu_pct"] for s in alive[1:]]), 1) if len(alive) > 1 else None,
            "cpu_pct_max": max((s["cpu_pct"] for s in alive), default=None),
            "fds_max": max((s["fds"] for s in alive), default=None),
            # 稳态段首尾（去掉前 20% 连接建立期），否则 worker 建连本身会被误判为 fd 泄漏
            "fds_first": steady[0]["fds"] if steady else None,
            "fds_last": steady[-1]["fds"] if steady else None,
            "threads_max": max((s["threads"] for s in alive), default=None),
        },
        "samples": samples,
    }


# ── 阈值判定 ──────────────────────────────────────────────────────────────────────
def evaluate(results: list[dict], args, auth: dict | None = None) -> list[dict]:
    checks = []

    def add(name, ok, actual, limit, note=""):
        checks.append({"check": name, "pass": ok, "actual": actual, "limit": limit, "note": note})

    by = {r["scenario"]: r for r in results}
    if "idle" in by and by["idle"]["process"]["rss_mb_max"] is not None:
        v = by["idle"]["process"]["rss_mb_max"]
        add("idle RSS 峰值", v < args.max_idle_rss, f"{v} MB", f"< {args.max_idle_rss} MB")
    if "soak" in by:
        p = by["soak"]["process"]
        if by["soak"]["duration_s"] >= 600:
            add("soak RSS 斜率", p["rss_slope_mb_per_10min"] < args.max_slope,
                f"{p['rss_slope_mb_per_10min']} MB/10min", f"< {args.max_slope} MB/10min")
        else:
            add("soak RSS 斜率", True, f"{p['rss_slope_mb_per_10min']} MB/10min",
                f"< {args.max_slope} MB/10min", "时长 < 10 min，仅记录不判定（--quick）")
        if p["fds_first"] is not None:
            growth = p["fds_last"] - p["fds_first"]
            add("soak fd 增长", growth <= args.max_fd_growth, growth, f"<= {args.max_fd_growth}")
    if auth is not None:
        fails = auth.get("refresh_failures", [])
        add("token 续期", not fails, f"{auth.get('refreshes', 0)} 次成功 / {len(fails)} 次失败", "0 失败")
    for r in results:
        add(f"{r['scenario']} 错误率", r["error_rate"] <= args.max_error_rate,
            f"{r['errors']}/{r['requests']}", f"<= {args.max_error_rate}")
        add(f"{r['scenario']} 进程存活", not r["process"]["died"], "alive" if not r["process"]["died"] else "DIED", "alive")
    return checks


def render_md(report: dict) -> str:
    L = ["# Flamepanel 性能 / 稳定性报告", "",
         f"- 时间：{report['started_at']}  ·  目标：`{report['base_url']}`  ·  PID：{report['pid']}",
         f"- 主机：{report['host']['cpus']} CPU · {report['host']['mem_total_mb']} MB  ·  quick={report['quick']}",
         f"- 结论：**{'PASS' if report['pass'] else 'FAIL'}**", "",
         "## 场景汇总", "",
         "| 场景 | 时长 s | HTTP 并发 | WS | 请求 | RPS | 错误 | p50 ms | p95 ms | p99 ms | RSS 峰 MB | RSS 斜率 MB/10min | CPU 均/峰 % | fd 峰 | 线程峰 |",
         "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"]
    for r in report["scenarios"]:
        p, l = r["process"], r["latency_ms"]
        L.append(f"| {r['scenario']} | {r['duration_s']} | {r['http_workers']} | {r['ws_clients']} | "
                 f"{r['requests']} | {r['rps']} | {r['errors']} | {l['p50']} | {l['p95']} | {l['p99']} | "
                 f"{p['rss_mb_max']} | {p['rss_slope_mb_per_10min']} | {p['cpu_pct_avg']}/{p['cpu_pct_max']} | "
                 f"{p['fds_max']} | {p['threads_max']} |")
    L += ["", "## 阈值判定", "", "| 检查 | 结果 | 实测 | 阈值 | 备注 |", "|---|---|---|---|---|"]
    for c in report["checks"]:
        L.append(f"| {c['check']} | {'✅' if c['pass'] else '❌'} | {c['actual']} | {c['limit']} | {c['note']} |")
    for r in report["scenarios"]:
        if r["requests"] and r["endpoints_p95_ms"]:
            L += ["", f"## {r['scenario']} · 端点 p95（ms）", "", "| 端点 | p95 |", "|---|---|"]
            L += [f"| `{k}` | {v} |" for k, v in r["endpoints_p95_ms"].items()]
        if r["error_kinds"] or r["ws"]["errors"]:
            L += ["", f"**{r['scenario']} 错误明细**：`{json.dumps(r['error_kinds'], ensure_ascii=False)}` "
                      f"· WS：`{json.dumps(r['ws']['errors'], ensure_ascii=False)}`"]
    if report.get("kernel_log_counts"):
        L += ["", "## 内核日志（压测期间）", "", f"`{json.dumps(report['kernel_log_counts'], ensure_ascii=False)}`"]
    return "\n".join(L) + "\n"


def host_info() -> dict:
    mem = 0
    try:
        with open("/proc/meminfo") as f:
            mem = int(f.readline().split()[1]) // 1024
    except OSError:
        pass
    return {"cpus": os.cpu_count(), "mem_total_mb": mem}


async def main_async(args) -> int:
    sess = Session(args.base_url)
    if args.token:
        sess.token = args.token
    elif args.setup:
        await sess.setup(args.username, args.password)
    else:
        await sess.login(args.username, args.password)
    if not sess.token:
        raise SystemExit("未获得 access token")
    # 续期是**会话级**的：按 token 年龄计时、跨越所有场景。若每个场景各自计时，
    # idle 5 min + load 10 min 恰好耗尽 15 min 的 access TTL，soak 一开始就全部 401。
    refresh_failures: list[str] = []
    session_stop = asyncio.Event()

    async def refresher():
        while not session_stop.is_set():
            wait = max(0.05, args.refresh_every - (time.monotonic() - sess.token_at))
            try:
                await asyncio.wait_for(session_stop.wait(), timeout=wait)
            except asyncio.TimeoutError:
                if not await sess.refresh():
                    refresh_failures.append(time.strftime("%H:%M:%S"))
                    sess.token_at = time.monotonic()  # 失败也推迟下一次，避免打满登录档

    refresh_task = asyncio.create_task(refresher())
    await sess.discover()
    if not sess.log_file:
        print("[perf] 注意：内核文件日志未启用，内核日志统计端点不参与压测", flush=True)

    q = args.quick
    plan = {
        "idle": dict(http_n=0, ws_n=1, duration=args.idle_secs * q, think=0),
        "load": dict(http_n=args.concurrency, ws_n=args.ws_clients, duration=args.load_secs * q, think=args.think),
        "soak": dict(http_n=args.soak_concurrency, ws_n=max(1, args.ws_clients // 4),
                     duration=args.soak_secs * q, think=args.think),
    }
    results = []
    try:
        for name in args.scenarios.split(","):
            results.append(await run_scenario(name, sess, args, **plan[name.strip()]))
            if results[-1]["process"]["died"]:
                break
    finally:
        session_stop.set()
        await asyncio.gather(refresh_task, return_exceptions=True)

    report = {
        "started_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "base_url": args.base_url, "pid": args.pid, "quick": q, "host": host_info(),
        "thresholds": {"max_idle_rss_mb": args.max_idle_rss, "max_slope_mb_per_10min": args.max_slope,
                       "max_error_rate": args.max_error_rate, "max_fd_growth": args.max_fd_growth},
        "scenarios": results,
        "auth": {"refreshes": sess.refreshes, "refresh_failures": refresh_failures},
    }
    # 压测期间内核日志五级计数（W0 内核日志体系）：ERROR 激增往往先于错误率暴露问题
    try:
        c = sess.conn()
        if not sess.log_file:
            raise LookupError("file logging disabled")
        st, _, d = await c.request("GET", f"/api/kernel-logs/stats?file={quote(sess.log_file)}",
                                   {"Authorization": f"Bearer {sess.token}"})
        await c.close()
        if st < 300:
            j = json.loads(d)
            report["kernel_log_counts"] = j.get("data", j) if isinstance(j, dict) else j
    except Exception as e:  # noqa: BLE001 — 统计失败不影响结论
        report["kernel_log_counts"] = {"unavailable": type(e).__name__}
    report["checks"] = evaluate(results, args, report["auth"])
    report["pass"] = all(c["pass"] for c in report["checks"])
    os.makedirs(args.out, exist_ok=True)
    with open(os.path.join(args.out, "perf-report.json"), "w") as f:
        json.dump(report, f, ensure_ascii=False, indent=2)
    md = render_md(report)
    with open(os.path.join(args.out, "perf-report.md"), "w") as f:
        f.write(md)
    print(md)
    return 0 if report["pass"] else 1


def parse_args(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--base-url", default="http://127.0.0.1:8080")
    ap.add_argument("--pid", type=int, default=0, help="内核进程 PID（/proc 采样；0 = 不采样）")
    auth = ap.add_argument_group("鉴权")
    auth.add_argument("--setup", action="store_true", help="对全新实例执行 setup 向导 API")
    auth.add_argument("--username", default="perfadmin")
    auth.add_argument("--password", default="PerfPassw0rd!")
    auth.add_argument("--token", default="", help="直接使用现成 access token（不续期）")
    auth.add_argument("--refresh-every", type=float, default=600, help="续期间隔秒（Access TTL 15 min）")
    sc = ap.add_argument_group("场景")
    sc.add_argument("--scenarios", default="idle,load,soak")
    sc.add_argument("--quick", type=float, default=1.0, help="时长缩放系数（如 0.1 → 快速冒烟）")
    sc.add_argument("--idle-secs", type=float, default=300)
    sc.add_argument("--load-secs", type=float, default=600)
    sc.add_argument("--soak-secs", type=float, default=1800)
    sc.add_argument("--concurrency", type=int, default=200)
    sc.add_argument("--soak-concurrency", type=int, default=50)
    sc.add_argument("--ws-clients", type=int, default=20)
    sc.add_argument("--think", type=float, default=0.5, help="worker 平均思考时间秒（0 = 打满）")
    sc.add_argument("--sample-interval", type=float, default=5)
    th = ap.add_argument_group("阈值")
    th.add_argument("--max-idle-rss", type=float, default=80)
    th.add_argument("--max-slope", type=float, default=1.0)
    th.add_argument("--max-error-rate", type=float, default=0.0)
    th.add_argument("--max-fd-growth", type=int, default=20)
    ap.add_argument("--out", default="perf-out")
    return ap.parse_args(argv)


def main() -> int:
    args = parse_args()
    try:
        return asyncio.run(main_async(args))
    except KeyboardInterrupt:
        return 130


if __name__ == "__main__":
    sys.exit(main())

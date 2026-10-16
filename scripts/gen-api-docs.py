#!/usr/bin/env python3
"""OpenAPI → Doc/api/*.md 生成器（Doc/32 §6，纯标准库）。

单一事实源是 utoipa 注解导出的 `frontend/openapi.json`；本脚本额外静态扫描
`flame-kernel/src/api/**/*.rs` 里的 `.route("…", get(…).post(…))`，统计「代码路由 vs 已文档化」覆盖率，
未文档化的端点清单写入 `Doc/api/coverage.md`（取代 T-05 的手工清单）。

用法：
  python3 scripts/gen-api-docs.py            # 重新生成 Doc/api/
  python3 scripts/gen-api-docs.py --check    # CI：生成物与仓库不一致时退出 1
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = ROOT / "frontend" / "openapi.json"
SRC = ROOT / "flame-kernel" / "src" / "api"
OUT = ROOT / "Doc" / "api"
METHODS = ("get", "post", "put", "patch", "delete")
HEADER = "<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->\n"


# ─── 代码路由扫描 ────────────────────────────────────────────────────────────

def _balanced(s: str, start: int) -> int:
    """s[start] == '('，返回匹配的 ')' 下标（忽略字符串字面量里的括号）。"""
    depth, i, in_str = 0, start, False
    while i < len(s):
        c = s[i]
        if in_str:
            if c == "\\":
                i += 1
            elif c == '"':
                in_str = False
        elif c == '"':
            in_str = True
        elif c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return len(s) - 1


def scan_routes(src: Path = SRC) -> dict[tuple[str, str], str]:
    """{(METHOD, path): 相对文件路径}"""
    routes: dict[tuple[str, str], str] = {}
    for f in sorted(src.rglob("*.rs")):
        text = f.read_text(encoding="utf-8")
        # 跳过测试模块里的路由
        cut = text.find("#[cfg(test)]")
        if cut != -1:
            text = text[:cut]
        for m in re.finditer(r"\.route\(", text):
            end = _balanced(text, m.end() - 1)
            body = text[m.end():end]
            pm = re.match(r'\s*"([^"]+)"\s*,', body)
            if not pm:
                continue
            path = pm.group(1)
            rest = body[pm.end():]
            for meth in re.findall(r"\b(get|post|put|patch|delete)\s*\(", rest):
                routes.setdefault((meth.upper(), path), str(f.relative_to(ROOT)))
    return routes


# ─── Schema 工具 ────────────────────────────────────────────────────────────

class Spec:
    def __init__(self, doc: dict):
        self.doc = doc
        self.schemas = doc.get("components", {}).get("schemas", {})

    def resolve(self, s: dict | None) -> dict:
        seen = 0
        while isinstance(s, dict) and "$ref" in s and seen < 16:
            s = self.schemas.get(s["$ref"].rsplit("/", 1)[-1], {})
            seen += 1
        return s or {}

    def type_name(self, s: dict | None) -> str:
        if not isinstance(s, dict) or not s:
            return "any"
        if "$ref" in s:
            return s["$ref"].rsplit("/", 1)[-1]
        for key in ("oneOf", "anyOf"):
            if key in s:
                parts = [self.type_name(x) for x in s[key]]
                nullable = "null" in parts
                parts = [p for p in parts if p != "null"]
                return " \\| ".join(parts) + ("?" if nullable else "")
        if "allOf" in s:
            return " & ".join(self.type_name(x) for x in s["allOf"])
        t = s.get("type")
        nullable = False
        if isinstance(t, list):
            nullable = "null" in t
            t = next((x for x in t if x != "null"), "any")
        if t == "array":
            name = self.type_name(s.get("items")) + "[]"
        elif "enum" in s:
            name = " \\| ".join(f"`{json.dumps(v, ensure_ascii=False)}`" for v in s["enum"] if v is not None)
        elif t == "object" and "additionalProperties" in s and not s.get("properties"):
            ap = s["additionalProperties"]
            name = f"map<string, {self.type_name(ap) if isinstance(ap, dict) else 'any'}>"
        else:
            name = str(t or "any")
            if s.get("format"):
                name += f"({s['format']})"
        return name + ("?" if nullable else "")

    def example(self, s: dict | None, depth: int = 0, stack: tuple = ()):
        if not isinstance(s, dict) or depth > 6:
            return None
        if "example" in s:
            return s["example"]
        if "$ref" in s:
            name = s["$ref"].rsplit("/", 1)[-1]
            if name in stack:
                return {}
            return self.example(self.schemas.get(name, {}), depth + 1, stack + (name,))
        for key in ("oneOf", "anyOf"):
            if key in s:
                opts = [x for x in s[key] if x.get("type") != "null"]
                return self.example(opts[0], depth + 1, stack) if opts else None
        if "allOf" in s:
            out: dict = {}
            for part in s["allOf"]:
                v = self.example(part, depth + 1, stack)
                if isinstance(v, dict):
                    out.update(v)
            return out
        if "enum" in s:
            vals = [v for v in s["enum"] if v is not None]
            return vals[0] if vals else None
        if "default" in s:
            return s["default"]
        t = s.get("type")
        if isinstance(t, list):
            t = next((x for x in t if x != "null"), None)
        if t == "object" or "properties" in s:
            props = s.get("properties", {})
            return {k: self.example(v, depth + 1, stack) for k, v in props.items()}
        if t == "array":
            item = self.example(s.get("items"), depth + 1, stack)
            return [] if item is None else [item]
        fmt = s.get("format", "")
        if t == "string":
            return {
                "date-time": "2026-01-01T00:00:00Z",
                "date": "2026-01-01",
                "uuid": "00000000-0000-0000-0000-000000000000",
                "binary": "<binary>",
            }.get(fmt, "string")
        if t == "integer":
            return 0
        if t == "number":
            return 0.0
        if t == "boolean":
            return False
        return None

    def fields(self, s: dict | None) -> list[tuple[str, str, bool, str]]:
        """对象的一级字段：(名, 类型, 必填, 说明)。"""
        r = self.resolve(s)
        if "allOf" in r:
            out = []
            for part in r["allOf"]:
                out.extend(self.fields(part))
            return out
        required = set(r.get("required", []))
        rows = []
        for name, prop in r.get("properties", {}).items():
            desc = (prop.get("description") or self.resolve(prop).get("description") or "").strip()
            rows.append((name, self.type_name(prop), name in required, desc.replace("\n", " ").replace("|", "\\|")))
        return rows


# ─── 权限 ──────────────────────────────────────────────────────────────────

def permission_text(op: dict) -> str:
    perm = op.get("x-permission")
    if perm:
        return f"`{perm}`"
    if op.get("security"):
        return "登录即可"
    return "公开（无需登录）"


def auth_text(op: dict) -> str:
    return "Bearer JWT" if op.get("security") else "无"


# ─── 渲染 ──────────────────────────────────────────────────────────────────

def md_escape(s: str) -> str:
    return (s or "").replace("|", "\\|").replace("\n", " ").strip()


def sample_path(path: str) -> str:
    return re.sub(r"\{[^}]+\}", "1", path)


def render_op(spec: Spec, path: str, method: str, op: dict) -> str:
    out = [f"### {method.upper()} `{path}`", ""]
    summary = (op.get("summary") or "").strip()
    desc = (op.get("description") or "").strip()
    if summary:
        out += [summary, ""]
    if desc and desc != summary:
        out += [desc, ""]
    out += [
        f"- 认证：{auth_text(op)}",
        f"- 权限：{permission_text(op)}",
        f"- operationId：`{op.get('operationId', '')}`",
        "",
    ]
    params = op.get("parameters", [])
    if params:
        out += ["| 参数 | 位置 | 类型 | 必填 | 说明 |", "|---|---|---|---|---|"]
        for p in params:
            out.append(
                f"| `{p.get('name')}` | {p.get('in')} | {spec.type_name(p.get('schema'))} | "
                f"{'是' if p.get('required') else '否'} | {md_escape(p.get('description', ''))} |"
            )
        out.append("")
    body_example = None
    rb = op.get("requestBody")
    if rb:
        for ctype, media in rb.get("content", {}).items():
            schema = media.get("schema")
            out.append(f"**请求体**（`{ctype}`{'，必填' if rb.get('required') else ''}）：{spec.type_name(schema)}")
            out.append("")
            rows = spec.fields(schema)
            if rows:
                out += ["| 字段 | 类型 | 必填 | 说明 |", "|---|---|---|---|"]
                out += [f"| `{n}` | {t} | {'是' if r else '否'} | {d} |" for n, t, r, d in rows]
                out.append("")
            if ctype == "application/json":
                body_example = spec.example(schema)
                out += ["```json", json.dumps(body_example, ensure_ascii=False, indent=2), "```", ""]
            break
    responses = op.get("responses", {})
    if responses:
        out += ["| 状态 | 说明 | 响应体 |", "|---|---|---|"]
        for code in sorted(responses):
            r = responses[code]
            content = r.get("content", {})
            body = ""
            if content:
                ctype, media = next(iter(content.items()))
                body = spec.type_name(media.get("schema")) if media.get("schema") else ""
                if ctype != "application/json":
                    body = f"{body} (`{ctype}`)".strip()
            out.append(f"| {code} | {md_escape(r.get('description', ''))} | {body} |")
        out.append("")
    ok = next((responses[c] for c in sorted(responses) if c.startswith("2")), None)
    if ok:
        media = ok.get("content", {}).get("application/json")
        if media and media.get("schema"):
            ex = spec.example(media["schema"])
            if ex not in (None, {}, []):
                text = json.dumps(ex, ensure_ascii=False, indent=2)
                if text.count("\n") <= 40:
                    out += ["<details><summary>响应示例</summary>", "", "```json", text, "```", "", "</details>", ""]
    # fpctl 调用示例：本机免登录（fpctl 现签令牌），脚本 / AI 直接可用
    if path.startswith("/api/") and op.get("security"):
        cmd = f"fpctl api {method.upper()} '{sample_path(path)}'"
        if body_example is not None:
            cmd += f" -d '{json.dumps(body_example, ensure_ascii=False, separators=(',', ':'))}'"
        out += ["```bash", cmd + " --json", "```", ""]
    return "\n".join(out)


def ops_by_tag(doc: dict) -> dict[str, list[tuple[str, str, dict]]]:
    tags: dict[str, list] = defaultdict(list)
    for path, item in doc.get("paths", {}).items():
        for method in METHODS:
            if method in item:
                op = item[method]
                tag = (op.get("tags") or ["default"])[0]
                tags[tag].append((path, method, op))
    for v in tags.values():
        v.sort(key=lambda x: (x[0], METHODS.index(x[1])))
    return dict(sorted(tags.items()))


def domain_of(path: str) -> str:
    parts = [p for p in path.split("/") if p]
    if parts and parts[0] == "api" and len(parts) > 1:
        return parts[1]
    return parts[0] if parts else "/"


def render_all(doc: dict, routes: dict[tuple[str, str], str]) -> dict[str, str]:
    spec = Spec(doc)
    tags = ops_by_tag(doc)
    files: dict[str, str] = {}
    documented = {(m.upper(), p) for t in tags.values() for p, m, _ in t}

    for tag, ops in tags.items():
        lines = [HEADER, f"# API · {tag}", "", f"> {len(ops)} 个端点 · 通用约定见 [README](./README.md)", ""]
        lines += ["| 方法 | 路径 | 摘要 | 权限 |", "|---|---|---|---|"]
        for path, method, op in ops:
            lines.append(f"| {method.upper()} | `{path}` | {md_escape(op.get('summary', ''))} | {permission_text(op)} |")
        lines.append("")
        for path, method, op in ops:
            lines += [render_op(spec, path, method, op)]
        files[f"{tag}.md"] = "\n".join(lines).rstrip() + "\n"

    # 覆盖率：只统计 HTTP 接口（WebSocket 升级端点不适用 OpenAPI）
    code = {k: v for k, v in routes.items() if not k[1].startswith("/ws/")}
    missing = sorted(k for k in code if k not in documented)
    stale = sorted(k for k in documented if routes and k not in routes)
    total = len(code)
    covered = total - len(missing)
    pct = (covered * 100.0 / total) if total else 100.0

    by_domain: dict[str, list] = defaultdict(list)
    for m, p in missing:
        by_domain[domain_of(p)].append((m, p, code[(m, p)]))
    cov = [HEADER, "# API 文档覆盖率", "",
           f"> 代码路由 **{total}** 个（方法级，不含 `/ws/*`）· 已文档化 **{covered}** · 覆盖率 **{pct:.1f}%**", "",
           "补注解流程：handler 加 `#[utoipa::path(...)]` → 在 `flame-kernel/src/openapi.rs` 的 `paths(...)` 登记 → "
           "`just openapi && just api-docs` → 提交。清单由 CI 与生成物一致性检查保持最新。", ""]
    cov += ["| 域 | 未文档化 |", "|---|---|"]
    for d in sorted(by_domain, key=lambda d: (-len(by_domain[d]), d)):
        cov.append(f"| {d} | {len(by_domain[d])} |")
    cov.append("")
    for d in sorted(by_domain):
        cov += [f"## {d}（{len(by_domain[d])}）", ""]
        cov += [f"- `{m} {p}` — {f}" for m, p, f in by_domain[d]]
        cov.append("")
    if stale:
        cov += ["## 文档中存在但代码中未找到的路由", ""]
        cov += [f"- `{m} {p}`" for m, p in stale]
        cov.append("")
    files["coverage.md"] = "\n".join(cov).rstrip() + "\n"

    info = doc.get("info", {})
    readme = [HEADER, "# FlamePanel REST API 参考", "",
              f"> 版本 {info.get('version', '?')} · {sum(len(v) for v in tags.values())} 个已文档化端点 · "
              f"覆盖率 {pct:.1f}%（[未文档化清单](./coverage.md)）", "",
              "## 通用约定", "",
              "- **基础地址**：`http://<面板>/api/...`；机器可读文档 `GET /api/openapi.json`（无需登录），"
              "面板内浏览：侧边栏「API 文档」（`/api-docs`）。",
              "- **认证**：`POST /api/auth/login` 取 `access_token`，之后请求头 `Authorization: Bearer <token>`；"
              "部署主机上可直接用 `fpctl api <METHOD> <PATH>`（现签 60 s 管理员令牌，免登录，走完整鉴权与审计）。",
              "- **权限**：每个端点标注 `resource:action`（与 `flame-kernel/src/api/permissions.rs` 同源，"
              "角色授权见 [Doc/07](../07-权限体系设计.md)）；「登录即可」表示只校验身份。",
              "- **错误**：非 2xx 响应体为 `{\"code\": \"…\", \"error\": \"…\", \"message\": \"…\"}`，"
              "响应头 `x-request-id` 可在「系统日志 → 内核日志」或 `fpctl logs --grep <id>` 中检索。"
              "常见状态：400 参数错误 · 401 未登录 / 令牌失效 · 403 无权限或需先修改初始密码 · "
              "404 不存在 · 409 冲突 · 422 校验失败 · 429 频率限制。",
              "- **时间**：RFC 3339（UTC）。**分页**：`page` / `page_size`（各端点参数表为准）。",
              "",
              "## 按模块", "",
              "| 模块 | 端点数 | 文档 |", "|---|---|---|"]
    for tag, ops in tags.items():
        readme.append(f"| {tag} | {len(ops)} | [{tag}.md](./{tag}.md) |")
    readme += ["", "## 维护", "",
               "```bash",
               "just openapi     # 重新导出 frontend/openapi.json（后端注解变更后）",
               "just api-docs    # 重新生成 Doc/api/（本目录）",
               "```", "",
               "CI（CNB installer-regression / GitHub ci-cd）执行 `python3 scripts/gen-api-docs.py --check`，"
               "生成物与快照不一致即失败。", ""]
    files["README.md"] = "\n".join(readme).rstrip() + "\n"
    return files


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--check", action="store_true", help="只校验，不写文件")
    ap.add_argument("--spec", type=Path, default=SPEC)
    ap.add_argument("--out", type=Path, default=OUT)
    args = ap.parse_args(argv)

    doc = json.loads(args.spec.read_text(encoding="utf-8"))
    files = render_all(doc, scan_routes())
    existing = {p.name for p in args.out.glob("*.md")} if args.out.is_dir() else set()

    if args.check:
        bad = [n for n, c in files.items()
               if not (args.out / n).is_file() or (args.out / n).read_text(encoding="utf-8") != c]
        extra = sorted(existing - set(files))
        routes = scan_routes()
        documented = {(m.upper(), p) for p, item in doc.get("paths", {}).items() for m in METHODS if m in item}
        ghosts = sorted(k for k in documented if k not in routes)
        for m, p in ghosts:
            print(f"ghost: {m} {p} is documented but no such route exists (fix the #[utoipa::path] path)")
        # v1.1 起 HTTP 路由 100% 文档化：新增路由必须同时补 #[utoipa::path] 并登记 openapi.rs
        undocumented = sorted(k for k in routes if not k[1].startswith("/ws/") and k not in documented)
        for m, p in undocumented:
            print(f"undocumented: {m} {p} — {routes[(m, p)]}（补 #[utoipa::path] 并在 openapi.rs paths(...) 登记）")
        if bad or extra or ghosts or undocumented:
            for n in sorted(bad):
                print(f"stale: Doc/api/{n}")
            for n in extra:
                print(f"orphan: Doc/api/{n}")
            print("Doc/api 与 frontend/openapi.json 不一致：请运行 python3 scripts/gen-api-docs.py（或 just api-docs）并提交")
            return 1
        print(f"Doc/api up to date ({len(files)} files)")
        return 0

    args.out.mkdir(parents=True, exist_ok=True)
    for n in existing - set(files):
        (args.out / n).unlink()
    for n, c in files.items():
        (args.out / n).write_text(c, encoding="utf-8")
    print(f"wrote {len(files)} files to {args.out.relative_to(ROOT) if args.out.is_relative_to(ROOT) else args.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

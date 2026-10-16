#!/usr/bin/env python3
"""`-c` 调用点门禁（Doc/33 §5-3 · Doc/05 §9.4.1）。

扫描 flame-kernel/src 与 agent/src 的 Rust 源码，统计每个文件里字面量 "-c" 的出现次数，
与 scripts/shell-exec-allowlist.txt 比对：

- 文件不在清单里却出现 "-c"          → 失败（新增 `sh -c` 需先说明理由并登记）
- 出现次数超过清单登记值             → 失败
- 出现次数低于登记值                 → 失败，提示同步下调（棘轮：只减不增）

"-c" 并不都是 shell（`nginx -c <conf>`、`python3 -c <常量>`），清单里逐项写明用途；
门禁只保证「新增调用点必须经过人工登记」，不判断语义。无第三方依赖。
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
ALLOWLIST = ROOT / "scripts" / "shell-exec-allowlist.txt"
SCAN_DIRS = ["flame-kernel/src", "agent/src"]
PATTERN = re.compile(r'"-c"')


def load_allowlist():
    allowed = {}
    for n, raw in enumerate(ALLOWLIST.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        parts = line.split()
        if len(parts) != 2 or not parts[1].isdigit():
            sys.exit(f"{ALLOWLIST.name}:{n}: 格式应为 `<路径> <次数>  # 理由`")
        if "#" not in raw:
            sys.exit(f"{ALLOWLIST.name}:{n}: 缺少 `# 理由`")
        allowed[parts[0]] = int(parts[1])
    return allowed


def scan():
    found = {}
    for d in SCAN_DIRS:
        for path in sorted((ROOT / d).rglob("*.rs")):
            count = 0
            for line in path.read_text(encoding="utf-8").splitlines():
                if line.lstrip().startswith("//"):
                    continue
                count += len(PATTERN.findall(line))
            if count:
                found[path.relative_to(ROOT).as_posix()] = count
    return found


def main():
    allowed = load_allowlist()
    found = scan()
    errors = []
    for path, count in found.items():
        limit = allowed.get(path)
        if limit is None:
            errors.append(f'{path}: 新增 {count} 处 "-c"，未登记。优先改用 PrivilegedCommand 逐参数 argv'
                          f"（Doc/05 §9.4.1）；确需 shell 时用常量脚本 + 位置参数，并在 {ALLOWLIST.name} 登记理由")
        elif count > limit:
            errors.append(f'{path}: "-c" {count} 处，超过登记的 {limit} 处')
        elif count < limit:
            errors.append(f"{path}: 已减少到 {count} 处，请把 {ALLOWLIST.name} 的登记值下调为 {count}")
    for path in allowed:
        if path not in found:
            errors.append(f"{path}: 已无 \"-c\"（或文件已删除），请从 {ALLOWLIST.name} 移除")
    if errors:
        print("shell-exec gate FAILED:", *errors, sep="\n  ")
        return 1
    print(f'shell-exec gate OK: {sum(found.values())} 处 "-c"，{len(found)} 个文件，均已登记')
    return 0


if __name__ == "__main__":
    sys.exit(main())

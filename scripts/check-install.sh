#!/usr/bin/env bash
# CNB / GitHub / 本地共用：仅语法和隔离回归，绝不执行 install.sh。
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd -- "$SCRIPT_DIR/.."

bash -n install.sh
bash -n scripts/check-install.sh
bash -n uninstall.sh
python3 scripts/tests/test_install_download.py
python3 scripts/tests/test_install_ci_entry.py
python3 scripts/tests/test_install_platform.py

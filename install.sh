#!/bin/bash
set -euo pipefail

# ─── 版本 ──────────────────────────────────────────────────────────────────────
VERSION="1.2"

# ─── 颜色 ──────────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# ─── 默认值 ────────────────────────────────────────────────────────────────────
PANEL_USERNAME="admin"
PANEL_PASSWORD=""
PANEL_PORT="8080"
JWT_SECRET=""
NON_INTERACTIVE=false
INSTALL_DIR="/opt/flamepanel"
SERVICE_NAME="flamepanel"
BINARY_PATH="/usr/local/bin/flamepanel"
# 面板命令行（Doc/32 §3）：与面板同版本、同包分发
CTL_PATH="/usr/local/bin/fpctl"
RUN_USER="flamepanel"
RUN_GROUP="flamepanel"
# 是否启用 HTTPS（nginx 443 + 自签证书；80 重定向到 443）
ENABLE_TLS=false
# HTTPS 证书/密钥路径（默认生成自签证书）
TLS_CERT_PATH=""
TLS_KEY_PATH=""
# 自签证书信息
TLS_COUNTRY="CN"
TLS_STATE=""
TLS_CITY=""
TLS_ORG="Flamepanel"
TLS_DOMAIN=""
# 运行用户的环境文件（600 权限，含 JWT 密钥/数据库密码等敏感配置）
ENV_FILE="$INSTALL_DIR/flamepanel.env"

# ─── P0-5 / T0-5：内容安全策略基线 ────────────────────────────────────────────
# Doc/33 P-6：策略单一来源 frontend/scripts/csp.ts。前端构建产出 dist/csp.conf（含实算的首屏脚本
# 哈希），下文生成的站点配置 include 该片段——升级前端即自动更新，install.sh 不再手写哈希。
# 仅当前端包缺少 csp.conf（旧版本发布包）时写入下面的降级策略：指令相同、不放行内联脚本
# （首屏主题脚本被拦 → 可能闪烁一次，功能不受影响）。护栏见 frontend/src/config/__tests__/csp.spec.ts。
# add_header 有继承规则：下级 location 一旦自带 add_header，本层全部头不再继承，
# 故自带 add_header 的 location 内须重复 include。
CSP_FALLBACK_POLICY="default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self'; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'"

# ─── 帮助 ──────────────────────────────────────────────────────────────────────
usage() {
    cat << EOF
Flamepanel 安装脚本 v${VERSION}

用法: $0 [选项]

选项:
  -u, --username NAME    管理员用户名 (默认: admin)
  -p, --password PASS    管理员密码 (默认: 交互输入)
  -P, --port PORT        后端监听端口 (默认: 8080)
  -s, --secret SECRET    JWT 签名密钥 (默认: 自动生成)
  -t, --tls             启用 HTTPS（自签证书，443 端口；80 重定向到 443）
      --cert PATH       自定义 TLS 证书路径（配合 --tls，需同时提供 --key）
      --key PATH        自定义 TLS 私钥路径（配合 --tls）
  -n, --non-interactive  非交互模式，使用默认值 (密码将自动生成)
      --release VER     安装指定发布版本（默认解析 GitHub 最新版本；等价环境变量 FLAMEPANEL_VERSION）
      --insecure-skip-verify  跳过发布包 SHA-256 校验（不推荐；等价 FLAMEPANEL_SKIP_VERIFY=1）
  -h, --help             显示帮助信息

说明:
  - 部署二进制到 /usr/local/bin/flamepanel
  - 前端静态资源部署到 /opt/flamepanel/frontend
  - 自动配置 nginx 反向代理 (80 端口 -> 后端 API/WebSocket)
  - --tls 时额外配置 443 HTTPS（自签证书，浏览器需信任；生产建议改用证书颁发机构签发的证书）
  - 使用方式二/三时需先本地构建或在 GitHub Releases 提供产物

示例:
  $0                                          # 交互式安装
  $0 -u myadmin -p mypass -P 9090             # 自定义账号和端口
  $0 -n                                       # 静默安装，全部使用默认值
  $0 -t                                       # 启用 HTTPS（自签）
  $0 -t --cert /etc/ssl/flamepanel.pem --key /etc/ssl/flamepanel.key  # 使用自定义证书
  $0 -u ops -p 'Str0ng!P@ss' -P 9090 -s 'xxx' # 完整自定义

卸载:
  sudo ./uninstall.sh                          # 卸载（保留数据）
  sudo ./uninstall.sh -p                       # 卸载并删除数据
EOF
    exit 0
}

# ─── 参数解析 ──────────────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        -u|--username)
            PANEL_USERNAME="$2"
            shift 2
            ;;
        -p|--password)
            PANEL_PASSWORD="$2"
            shift 2
            ;;
        -P|--port)
            PANEL_PORT="$2"
            shift 2
            ;;
        -s|--secret)
            JWT_SECRET="$2"
            shift 2
            ;;
        -n|--non-interactive)
            NON_INTERACTIVE=true
            shift
            ;;
        -t|--tls)
            ENABLE_TLS=true
            shift
            ;;
        --cert)
            TLS_CERT_PATH="$2"
            shift 2
            ;;
        --key)
            TLS_KEY_PATH="$2"
            shift 2
            ;;
        --release)
            FLAMEPANEL_VERSION="$2"
            shift 2
            ;;
        --insecure-skip-verify)
            FLAMEPANEL_SKIP_VERIFY=1
            shift
            ;;
        -h|--help)
            usage
            ;;
        *)
            echo -e "${RED}未知参数: $1${NC}"
            usage
            ;;
    esac
done

# ─── TLS 参数校验 ────────────────────────────────────────────────────────────
if [[ "$ENABLE_TLS" == true ]]; then
    if [[ -n "$TLS_CERT_PATH" || -n "$TLS_KEY_PATH" ]]; then
        if [[ -z "$TLS_CERT_PATH" || -z "$TLS_KEY_PATH" ]]; then
            echo -e "${RED}错误: --cert 与 --key 必须同时提供${NC}"
            exit 1
        fi
        if [[ ! -f "$TLS_CERT_PATH" || ! -f "$TLS_KEY_PATH" ]]; then
            echo -e "${RED}错误: 证书或私钥文件不存在${NC}"
            exit 1
        fi
    fi
fi

# ─── 检查 root ─────────────────────────────────────────────────────────────────
if [[ $EUID -ne 0 ]]; then
    echo -e "${RED}请使用 root 权限运行此脚本 (sudo)${NC}"
    exit 1
fi

# ─── 平台适配函数（Doc/33 §4：发行版矩阵）──────────────────────────────────────
# 以下函数只读取可覆盖的路径变量（OS_RELEASE_FILE / NGINX_ROOT / SYSTEMD_RUN_DIR），
# 便于 scripts/tests/test_install_platform.py 在临时目录中离线回归，不触碰真实系统。

# 解析 os-release → OS_ID / OS_VERSION / OS_FAMILY（debian|rhel|suse|arch|alpine|unknown）
detect_platform() {
    local file="${OS_RELEASE_FILE:-/etc/os-release}" id="" like="" ver="" token
    if [[ -r "$file" ]]; then
        id=$(sed -n 's/^ID=//p' "$file" | head -n1 | tr -d '"'"'" | tr '[:upper:]' '[:lower:]')
        like=$(sed -n 's/^ID_LIKE=//p' "$file" | head -n1 | tr -d '"'"'" | tr '[:upper:]' '[:lower:]')
        ver=$(sed -n 's/^VERSION_ID=//p' "$file" | head -n1 | tr -d '"'"'")
    fi
    OS_ID="${id:-unknown}"
    OS_VERSION="${ver:-unknown}"
    OS_FAMILY="unknown"
    for token in $id $like; do
        case "$token" in
            debian|ubuntu|raspbian|linuxmint|kali|deepin|uos) OS_FAMILY="debian" ;;
            rhel|centos|rocky|almalinux|ol|fedora|amzn|anolis|opencloudos|openeuler|kylin) OS_FAMILY="rhel" ;;
            suse|opensuse*|sles|sled) OS_FAMILY="suse" ;;
            arch|archarm|manjaro|endeavouros) OS_FAMILY="arch" ;;
            alpine) OS_FAMILY="alpine" ;;
            *) continue ;;
        esac
        break
    done
}

# 按发行版族安装软件包（失败时输出日志尾部，不再静默吞错）
pkg_install() {
    local log
    log=$(mktemp "${TMPDIR:-/tmp}/flamepanel-pkg.XXXXXX")
    local rc=0
    case "$OS_FAMILY" in
        debian)
            { DEBIAN_FRONTEND=noninteractive apt-get update -qq \
                && DEBIAN_FRONTEND=noninteractive apt-get install -y -qq "$@"; } >"$log" 2>&1 || rc=$? ;;
        rhel)
            if command -v dnf &>/dev/null; then
                dnf install -y -q "$@" >"$log" 2>&1 || rc=$?
            else
                yum install -y -q "$@" >"$log" 2>&1 || rc=$?
            fi ;;
        suse)   zypper --non-interactive --quiet install "$@" >"$log" 2>&1 || rc=$? ;;
        arch)   pacman -Sy --needed --noconfirm "$@" >"$log" 2>&1 || rc=$? ;;
        alpine) apk add --no-cache "$@" >"$log" 2>&1 || rc=$? ;;
        *)      echo "unsupported OS_FAMILY=$OS_FAMILY" >"$log"; rc=1 ;;
    esac
    if [[ $rc -ne 0 ]]; then
        echo -e "${YELLOW}  -> 软件包安装失败（$*），日志尾部:${NC}" >&2
        tail -n 15 "$log" >&2
    fi
    rm -f "$log"
    return $rc
}

# 选择 nginx 站点配置目录；Arch 等主配置未 include 任何目录时自动补 include。
# 结果写入全局 NGINX_DIR（不用 $(...) 取值：子 shell 会丢失 NGINX_INCLUDE_ADDED）；
# 补 include 时置 NGINX_INCLUDE_ADDED=1（卸载时据此回滚）。
nginx_conf_dir() {
    local root="${NGINX_ROOT:-/etc/nginx}" main="${NGINX_ROOT:-/etc/nginx}/nginx.conf"
    NGINX_INCLUDE_ADDED=0
    # Alpine：http.d；Debian/RHEL/openSUSE：conf.d（绝对或相对路径 include 均识别）
    local d
    for d in http.d conf.d; do
        if [[ -f "$main" ]] && grep -Eq "^[[:space:]]*include[[:space:]]+(${root}/)?${d}/\*\.conf" "$main"; then
            NGINX_DIR="$root/$d"
            # 重装：先前由本脚本补的 include（带标记）仍记为已添加，卸载时才能回滚
            grep -Eq "include[[:space:]]+${root}/${d}/\*\.conf;[[:space:]]+# flamepanel" "$main" && NGINX_INCLUDE_ADDED=1
            return 0
        fi
    done
    # 主配置未包含任何站点目录（Arch 默认）：在 http { 后插入 conf.d include（先备份）
    mkdir -p "$root/conf.d"
    if [[ -f "$main" ]] && grep -Eq '^[[:space:]]*http[[:space:]]*\{' "$main"; then
        [[ -f "$main.flamepanel.bak" ]] || cp -p "$main" "$main.flamepanel.bak"
        sed -i -E "0,/^[[:space:]]*http[[:space:]]*\{/s||&\n    include ${root}/conf.d/*.conf;  # flamepanel|" "$main"
        NGINX_INCLUDE_ADDED=1
    fi
    NGINX_DIR="$root/conf.d"
}

# Debian/Ubuntu 默认站点 `listen 80 default_server` 会抢占 80 端口，按 IP 访问只能看到
# 「Welcome to nginx」。仅当它是发行版原装链接时禁用，并置 NGINX_DEFAULT_SITE_DISABLED=1。
disable_distro_default_site() {
    local root="${NGINX_ROOT:-/etc/nginx}"
    NGINX_DEFAULT_SITE_DISABLED=0
    local link="$root/sites-enabled/default"
    if [[ -L "$link" ]] && [[ "$(readlink "$link")" == *"sites-available/default" ]]; then
        rm -f "$link"
        NGINX_DEFAULT_SITE_DISABLED=1
    fi
}

# 探测 nginx worker 运行用户（Debian: www-data；RHEL/openSUSE/Arch/Alpine: nginx/http）
nginx_worker_user() {
    local main="${NGINX_ROOT:-/etc/nginx}/nginx.conf" u=""
    [[ -f "$main" ]] && u=$(sed -n -E 's/^[[:space:]]*user[[:space:]]+([^ ;]+).*/\1/p' "$main" | head -n1)
    if [[ -z "$u" ]]; then
        for u in www-data nginx http; do
            id "$u" &>/dev/null && break
        done
    fi
    echo "${u:-nobody}"
}

# SELinux Enforcing（RHEL/Rocky/Alma/Fedora 默认）：放行 nginx 反代到本机端口，并给前端/证书打标签。
# 否则 nginx → 127.0.0.1:端口 返回 502，/opt 下的静态文件因 default_t 标签返回 403。
configure_selinux() {
    command -v getenforce &>/dev/null || return 0
    [[ "$(getenforce 2>/dev/null)" == "Disabled" ]] && return 0
    echo -e "${CYAN}  检测到 SELinux ($(getenforce))，配置 nginx 访问策略...${NC}"
    setsebool -P httpd_can_network_connect 1 2>/dev/null \
        || echo -e "${YELLOW}  -> setsebool 失败，请手动执行: setsebool -P httpd_can_network_connect 1${NC}"
    if command -v semanage &>/dev/null; then
        semanage fcontext -a -t httpd_sys_content_t "$INSTALL_DIR(/releases/[^/]+)?/frontend(/.*)?" 2>/dev/null \
            || semanage fcontext -m -t httpd_sys_content_t "$INSTALL_DIR(/releases/[^/]+)?/frontend(/.*)?" 2>/dev/null || true
        semanage fcontext -a -t cert_t "$INSTALL_DIR/tls(/.*)?" 2>/dev/null || true
        restorecon -R "$INSTALL_DIR" 2>/dev/null || true
    else
        chcon -R -t httpd_sys_content_t "$INSTALL_DIR/frontend/" 2>/dev/null || true
        [[ -d "$INSTALL_DIR/tls" ]] && chcon -R -t cert_t "$INSTALL_DIR/tls" 2>/dev/null || true
    fi
}

# 主机防火墙放行 Web 端口（仅在 firewalld / ufw 处于活动状态时操作）
open_firewall_ports() {
    local ports=("$@") p
    if command -v firewall-cmd &>/dev/null && firewall-cmd --state &>/dev/null; then
        for p in "${ports[@]}"; do
            firewall-cmd --permanent --add-port="${p}/tcp" &>/dev/null || true
        done
        firewall-cmd --reload &>/dev/null || true
        echo -e "${GREEN}  -> firewalld 已放行: ${ports[*]}${NC}"
    elif command -v ufw &>/dev/null && ufw status 2>/dev/null | grep -q "Status: active"; then
        for p in "${ports[@]}"; do
            ufw allow "${p}/tcp" &>/dev/null || true
        done
        echo -e "${GREEN}  -> ufw 已放行: ${ports[*]}${NC}"
    fi
}

# ─── 系统检测 ──────────────────────────────────────────────────────────────────
detect_platform
OS="$OS_ID"
echo -e "${CYAN}检测到系统:${NC} $OS_ID $OS_VERSION（发行版族: $OS_FAMILY）"

# systemd 是面板与特权助手的服务托管前提（Alpine/OpenRC、无 init 容器不适用）
if [[ ! -d "${SYSTEMD_RUN_DIR:-/run/systemd/system}" ]]; then
    echo -e "${RED}错误: 未检测到 systemd（PID 1 非 systemd）。${NC}" >&2
    echo -e "  Alpine / OpenRC / 容器环境请改用 Docker 部署: docker compose up -d（见 Doc/06）" >&2
    exit 1
fi
if [[ "$OS_FAMILY" == "unknown" ]]; then
    echo -e "${YELLOW}  -> 未识别的发行版（$OS_ID），将尝试继续；依赖需预先安装: curl openssl nginx${NC}"
fi

# ─── 依赖检查 ────────────────────────────────────────────────────────────────
echo -e "${CYAN}检查系统依赖...${NC}"
missing_deps=()
for cmd in curl openssl; do
    if ! command -v "$cmd" &>/dev/null; then
        missing_deps+=("$cmd")
    fi
done
if [[ ${#missing_deps[@]} -gt 0 ]]; then
    echo -e "${YELLOW}  缺少依赖: ${missing_deps[*]}${NC}"
    echo -e "${CYAN}  将自动安装${NC}"
fi

# ─── 检查现有安装 ────────────────────────────────────────────────────────────
if [[ -f "$BINARY_PATH" ]] || [[ -f "/etc/systemd/system/${SERVICE_NAME}.service" ]]; then
    echo -e "${YELLOW}══════════════════════════════════════════${NC}"
    echo -e "${YELLOW}  检测到现有 Flamepanel 安装${NC}"
    echo -e "${YELLOW}══════════════════════════════════════════${NC}"
    echo ""
    if systemctl is-active --quiet "$SERVICE_NAME" 2>/dev/null; then
        echo -e "  ${GREEN}服务状态: 运行中${NC}"
    else
        echo -e "  ${YELLOW}服务状态: 未运行${NC}"
    fi
    echo ""
    echo -e "  ${YELLOW}建议先运行卸载脚本清理:${NC}"
    echo -e "    sudo bash uninstall.sh"
    echo ""
    if [[ "$NON_INTERACTIVE" == false ]]; then
        read -p "是否继续安装（将覆盖现有安装）? [y/N] " confirm
        if [[ "$confirm" != "y" ]] && [[ "$confirm" != "Y" ]]; then
            echo -e "${YELLOW}安装已取消${NC}"
            exit 0
        fi
    fi
fi

# ─── 交互式输入 ────────────────────────────────────────────────────────────────
if [[ "$NON_INTERACTIVE" == false ]]; then
    echo ""
    echo -e "${CYAN}══════════════════════════════════════════${NC}"
    echo -e "${CYAN}       Flamepanel 安装配置${NC}"
    echo -e "${CYAN}══════════════════════════════════════════${NC}"
    echo ""

    if [[ -z "$PANEL_USERNAME" ]] || [[ "$PANEL_USERNAME" == "admin" ]]; then
        read -p "管理员用户名 [admin]: " input_username
        PANEL_USERNAME="${input_username:-admin}"
    fi

    if [[ -z "$PANEL_PASSWORD" ]]; then
        while true; do
            read -s -p "管理员密码 (留空则自动生成): " input_password
            echo ""
            if [[ -z "$input_password" ]]; then
                PANEL_PASSWORD=$(openssl rand -base64 12 2>/dev/null || head -c 12 /dev/urandom | base64)
                echo -e "${YELLOW}已自动生成密码: ${GREEN}$PANEL_PASSWORD${NC}"
                break
            fi
            if [[ ${#input_password} -lt 8 ]]; then
                echo -e "${RED}密码长度不能少于 8 位${NC}"
                continue
            fi
            read -s -p "确认密码: " confirm_password
            echo ""
            if [[ "$input_password" != "$confirm_password" ]]; then
                echo -e "${RED}两次密码输入不一致，请重试${NC}"
                continue
            fi
            PANEL_PASSWORD="$input_password"
            break
        done
    fi

    if [[ -z "$PANEL_PORT" ]] || [[ "$PANEL_PORT" == "8080" ]]; then
        read -p "面板端口 [8080]: " input_port
        PANEL_PORT="${input_port:-8080}"
    fi

    echo ""
    echo -e "${CYAN}══════════════════════════════════════════${NC}"
    echo -e "  用户名: ${GREEN}$PANEL_USERNAME${NC}"
    echo -e "  密码:   ${GREEN}$(echo "$PANEL_PASSWORD" | sed 's/./*/g')${NC}"
    echo -e "  端口:   ${GREEN}$PANEL_PORT${NC}"
    echo -e "${CYAN}══════════════════════════════════════════${NC}"
    echo ""
    read -p "确认以上配置? [Y/n] " confirm
    if [[ "$confirm" == "n" ]] || [[ "$confirm" == "N" ]]; then
        echo "安装已取消"
        exit 0
    fi
fi

# 端口校验（1024–65535 之外的端口需 root 绑定或与 nginx 冲突）
if ! [[ "$PANEL_PORT" =~ ^[0-9]+$ ]] || (( PANEL_PORT < 1024 || PANEL_PORT > 65535 )); then
    echo -e "${RED}错误: 端口必须是 1024-65535 之间的整数（当前: $PANEL_PORT）${NC}" >&2
    exit 1
fi

# 非交互模式下自动生成密码
if [[ -z "$PANEL_PASSWORD" ]]; then
    PANEL_PASSWORD=$(openssl rand -base64 12 2>/dev/null || head -c 12 /dev/urandom | base64)
fi

# 自动生成 JWT 密钥
if [[ -z "$JWT_SECRET" ]]; then
    # 依赖安装前执行：最小化系统可能尚无 openssl / xxd，回退到 coreutils 的 od
    JWT_SECRET=$(openssl rand -hex 32 2>/dev/null || head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n')
fi

# ─── 安装依赖 ──────────────────────────────────────────────────────────────────
echo ""
echo -e "${CYAN}[1/5] 安装系统依赖...${NC}"

# Doc/33 §4：按发行版族安装（dnf 优先于 yum；新增 zypper / pacman；失败输出日志尾部）
DEPS=(curl openssl nginx)
# SELinux 工具（semanage）用于持久化文件标签；缺失时回退 chcon，不阻断安装
if [[ "$OS_FAMILY" == "rhel" ]] && ! command -v semanage &>/dev/null; then
    DEPS+=(policycoreutils-python-utils)
fi
if [[ "$OS_FAMILY" != "unknown" ]]; then
    if ! pkg_install "${DEPS[@]}"; then
        # RHEL 系部分精简镜像缺 policycoreutils-python-utils，去掉可选项重试
        pkg_install curl openssl nginx || true
    fi
fi

if ! command -v nginx &>/dev/null; then
    echo -e "${YELLOW}  -> 警告: nginx 安装失败，将跳过 Web 反向代理配置${NC}"
    echo -e "${YELLOW}     前端页面需自行配置 Web 服务器指向 $INSTALL_DIR/frontend/dist${NC}"
else
    echo -e "${GREEN}  -> 依赖安装完成${NC}"
fi

# ─── 创建目录结构 ──────────────────────────────────────────────────────────────
echo -e "${CYAN}[2/5] 创建目录结构与运行用户...${NC}"

# 创建专用系统用户/组（无登录 shell，最小权限运行后端）
if ! getent group "$RUN_GROUP" >/dev/null 2>&1; then
    groupadd --system "$RUN_GROUP"
    echo -e "${GREEN}  -> 已创建系统组 $RUN_GROUP${NC}"
fi
if ! id "$RUN_USER" >/dev/null 2>&1; then
    NOLOGIN_SHELL=$(command -v nologin || echo /usr/sbin/nologin)
    useradd --system --gid "$RUN_GROUP" --shell "$NOLOGIN_SHELL" --home-dir "$INSTALL_DIR" "$RUN_USER"
    echo -e "${GREEN}  -> 已创建系统用户 $RUN_USER${NC}"
fi

mkdir -p "$INSTALL_DIR/data"
mkdir -p "$INSTALL_DIR/logs"
mkdir -p "$INSTALL_DIR/frontend"

# 目录属主与权限（数据/日志 750，普通用户不可读；环境文件 600）
# Doc/33 §4：安装根目录用 711（仅可穿越、不可列目录）——旧的 750 使 nginx worker
# （www-data / nginx，不在 flamepanel 组）无法进入 frontend/dist，前端整站 403。
chown -R "$RUN_USER:$RUN_GROUP" "$INSTALL_DIR"
chmod 711 "$INSTALL_DIR"
chmod 750 "$INSTALL_DIR/data" "$INSTALL_DIR/logs"
chmod 755 "$INSTALL_DIR/frontend" 2>/dev/null || true

echo -e "${GREEN}  -> $INSTALL_DIR（属主 $RUN_USER:$RUN_GROUP，750）${NC}"

# ─── 部署二进制 ────────────────────────────────────────────────────────────────
echo -e "${CYAN}[3/5] 部署应用...${NC}"

# 查找本地编译好的二进制（支持 workspace 和子 crate 两种构建路径）
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
LOCAL_BINARY=""
for try_path in \
    "$SCRIPT_DIR/target/release/flame-kernel" \
    "$SCRIPT_DIR/target/release/flame-kernel.exe" \
    "$SCRIPT_DIR/flame-kernel/target/release/flame-kernel" \
    "$SCRIPT_DIR/flame-kernel/target/release/flame-kernel.exe"; do
    if [[ -f "$try_path" ]]; then
        LOCAL_BINARY="$try_path"
        break
    fi
done

# 版本化布局（Doc/32 §5：releases/<版本> + current 符号链接）重装时：
# 先摘掉指向 current 的稳定路径链接，下面按平铺方式写入真实文件，最后由 fpctl adopt 收编为新版本。
if [[ -L "$INSTALL_DIR/current" ]]; then
    for link in "$BINARY_PATH" "$CTL_PATH" "$INSTALL_DIR/frontend"; do
        [[ -L "$link" ]] && rm -f "$link"
    done
    echo -e "${CYAN}  -> 检测到版本化布局，部署后将自动收编为新版本${NC}"
fi

# GitHub Releases 下载（直连失败时自动尝试镜像站加速）
GH_BASE="https://github.com/Forevery1021/Flamepanel"
GH_MIRRORS=(
    "https://ghfast.top/https://github.com/Forevery1021/Flamepanel"
    "https://gh-proxy.com/https://github.com/Forevery1021/Flamepanel"
)
gh_download() {
    local url="$1" out="$2" mirror source temp_file
    local sources=("$url")
    # 每次调用使用独立临时文件；失败不覆盖既有产物，不留下半包。
    temp_file=$(mktemp "${out}.part.XXXXXX") || return 1
    for mirror in "${GH_MIRRORS[@]}"; do
        sources+=("${mirror}${url#${GH_BASE}}")
    done
    for source in "${sources[@]}"; do
        if [[ "$source" != "$url" ]]; then
            echo "  下载失败，尝试镜像: $source"
        fi
        if curl --fail --location --silent --show-error \
            --connect-timeout 10 --max-time 300 "$source" -o "$temp_file" \
            && [[ -s "$temp_file" ]]; then
            # 函数在 if 中调用，不能依赖 set -e；显式处理发布失败。
            if mv -f -- "$temp_file" "$out"; then
                GH_LAST_SOURCE="$source"
                return 0
            fi
            rm -f -- "$temp_file"
            return 1
        fi
        # 下一个来源不能复用本次响应或部分下载内容。
        if ! : > "$temp_file"; then
            rm -f -- "$temp_file"
            return 1
        fi
    done
    rm -f -- "$temp_file"
    return 1
}

# 私有暂存目录：下载/解包失败不直接覆盖正在使用的文件，退出时清理。
DOWNLOAD_DIR=$(mktemp -d "${TMPDIR:-/tmp}/flamepanel-install.XXXXXX")
trap 'rm -rf -- "$DOWNLOAD_DIR"' EXIT

# ─── 发布包完整性（Doc/33 排除项：Release SHA256 校验）────────────────────────
# 固定到具体版本（--release / FLAMEPANEL_VERSION，缺省解析 releases/latest 的重定向），
# 从该 tag 下载资源与 flamepanel-<版本>-checksums.txt，解包前逐个比对 SHA-256，不符即终止。
# 校验清单优先 GitHub 直连：资源走镜像、清单走直连时可发现镜像篡改；清单也只能从镜像取得时
# 仅能发现传输损坏 / 版本错配，会明确告警。未做发布签名（GPG / minisign），见 Doc/10 §2。
REL_VERSION=""
CHECKSUMS_FILE=""
resolve_release_version() {
    local src url tag
    if [[ -n "${FLAMEPANEL_VERSION:-}" ]]; then
        tag="v${FLAMEPANEL_VERSION#v}"
    else
        for src in "$GH_BASE" "${GH_MIRRORS[@]}"; do
            url=$(curl --fail --location --silent --show-error --connect-timeout 10 --max-time 30 \
                -o /dev/null -w '%{url_effective}' "$src/releases/latest" 2>/dev/null) || continue
            tag="${url%/}"
            tag="${tag##*/}"
            [[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+ ]] && break
            tag=""
        done
    fi
    if [[ ! "${tag:-}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([.+-][0-9A-Za-z.+-]+)?$ ]]; then
        return 1
    fi
    REL_VERSION="${tag#v}"
}
release_asset_url() {
    if [[ -n "$REL_VERSION" ]]; then
        printf '%s/releases/download/v%s/%s' "$GH_BASE" "$REL_VERSION" "$1"
    else
        printf '%s/releases/latest/download/%s' "$GH_BASE" "$1"
    fi
}
sha256_of() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum -- "$1" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 -- "$1" | awk '{print $1}'
    else
        openssl dgst -sha256 -r "$1" | awk '{print $1}'
    fi
}
# verify_asset <文件> <发布资源名>：失败返回非零并打印原因
verify_asset() {
    local file="$1" name="$2" want got
    if [[ "${FLAMEPANEL_SKIP_VERIFY:-0}" == "1" ]]; then
        echo -e "${YELLOW}  -> 警告: 已跳过 $name 的 SHA-256 校验（--insecure-skip-verify）${NC}"
        return 0
    fi
    if [[ -z "$CHECKSUMS_FILE" ]]; then
        CHECKSUMS_FILE="$DOWNLOAD_DIR/checksums.txt"
        if ! gh_download "$(release_asset_url "flamepanel-${REL_VERSION}-checksums.txt")" "$CHECKSUMS_FILE"; then
            CHECKSUMS_FILE=""
            echo -e "${RED}错误: 无法下载 v${REL_VERSION} 的校验清单 flamepanel-${REL_VERSION}-checksums.txt${NC}" >&2
            echo "    确认该版本存在；确需跳过校验可加 --insecure-skip-verify（不推荐）" >&2
            return 1
        fi
        if [[ "${GH_LAST_SOURCE:-}" != "$GH_BASE"* ]]; then
            echo -e "${YELLOW}  -> 警告: 校验清单来自镜像站，只能发现传输损坏，无法发现镜像篡改；建议另行核对 GitHub 上的清单${NC}"
        fi
    fi
    want=$(awk -v n="$name" '$2 == n || $2 == "*" n { print $1; exit }' "$CHECKSUMS_FILE")
    if [[ ! "$want" =~ ^[0-9a-fA-F]{64}$ ]]; then
        echo -e "${RED}错误: 校验清单中没有 $name，安装已终止${NC}" >&2
        return 1
    fi
    got=$(sha256_of "$file") || got=""
    if [[ "${got,,}" != "${want,,}" ]]; then
        echo -e "${RED}错误: $name SHA-256 不符（期望 $want，实际 ${got:-无法计算}），安装已终止${NC}" >&2
        return 1
    fi
    echo -e "${GREEN}  -> $name SHA-256 校验通过${NC}"
}
ensure_release_version() {
    [[ -n "$REL_VERSION" ]] && return 0
    if resolve_release_version; then
        echo "  发布版本: v$REL_VERSION"
        return 0
    fi
    if [[ "${FLAMEPANEL_SKIP_VERIFY:-0}" == "1" ]]; then
        echo -e "${YELLOW}  -> 无法解析发布版本，按 latest 下载（未校验）${NC}"
        return 0
    fi
    echo -e "${RED}错误: 无法确定要安装的发布版本（--release 格式应为 X.Y.Z，或网络无法访问 GitHub / 镜像）${NC}" >&2
    return 1
}

if [[ -n "$LOCAL_BINARY" ]]; then
    echo "  使用本地编译产物: $LOCAL_BINARY"
    if [[ ! -s "$LOCAL_BINARY" ]]; then
        echo -e "${RED}错误: 本地二进制为空，请重新构建${NC}" >&2
        exit 1
    fi
    cp "$LOCAL_BINARY" "$BINARY_PATH"
    CTL_SOURCE="$(dirname "$LOCAL_BINARY")/fpctl"
else
    echo "  从 GitHub Releases 下载..."
    ARCH=$(uname -m)
    case "$ARCH" in
        x86_64|amd64)  ARCH="amd64" ;;
        aarch64|arm64) ARCH="arm64" ;;
        *)
            echo -e "${RED}错误: 暂无 $ARCH 架构的预编译包（仅 amd64 / arm64）。${NC}" >&2
            echo "    请在本机构建后重试: cargo build --release --package flame-kernel" >&2
            exit 1 ;;
    esac
    ensure_release_version || exit 1
    DOWNLOAD_URL=$(release_asset_url "flamepanel-linux-${ARCH}.tar.gz")
    echo "  下载地址: $DOWNLOAD_URL"
    if ! gh_download "$DOWNLOAD_URL" "$DOWNLOAD_DIR/backend.tar.gz"; then
        echo -e "${RED}错误: 二进制下载失败，安装已终止。请先在仓库根目录构建:${NC}" >&2
        echo "    cargo build --release --package flame-kernel" >&2
        exit 1
    fi
    verify_asset "$DOWNLOAD_DIR/backend.tar.gz" "flamepanel-linux-${ARCH}.tar.gz" || exit 1
    mkdir -p "$DOWNLOAD_DIR/backend"
    if ! tar -xzf "$DOWNLOAD_DIR/backend.tar.gz" -C "$DOWNLOAD_DIR/backend"; then
        echo -e "${RED}错误: 二进制资源包无法解压，安装已终止${NC}" >&2
        exit 1
    fi
    if [[ ! -f "$DOWNLOAD_DIR/backend/flamepanel" || ! -s "$DOWNLOAD_DIR/backend/flamepanel" \
        || -L "$DOWNLOAD_DIR/backend/flamepanel" ]]; then
        echo -e "${RED}错误: 资源包缺少有效的 flamepanel 二进制，安装已终止${NC}" >&2
        exit 1
    fi
    cp "$DOWNLOAD_DIR/backend/flamepanel" "$BINARY_PATH"
    CTL_SOURCE="$DOWNLOAD_DIR/backend/fpctl"
fi

# 不再吞掉产物缺失或 chmod 失败，只有可执行文件就绪才报告成功。
chmod +x "$BINARY_PATH"
echo -e "${GREEN}  -> 二进制部署完成${NC}"

# fpctl 为可选产物：旧版资源包不含它时只告警，不阻断安装
if [[ -n "${CTL_SOURCE:-}" && -f "$CTL_SOURCE" && -s "$CTL_SOURCE" && ! -L "$CTL_SOURCE" ]]; then
    cp "$CTL_SOURCE" "$CTL_PATH"
    chmod 755 "$CTL_PATH"
    echo -e "${GREEN}  -> 命令行工具部署完成: $CTL_PATH${NC}"
else
    echo -e "${YELLOW}  -> 未找到 fpctl（旧版资源包或未构建），跳过命令行工具${NC}"
fi

# ─── 部署前端静态资源 ─────────────────────────────────────────────────────────
echo -e "${CYAN}[4/5] 部署前端静态资源...${NC}"

FRONTEND_DIR="$INSTALL_DIR/frontend"
mkdir -p "$FRONTEND_DIR"

# 前端静态资源需可被 nginx（www-data）读取
chown -R "$RUN_USER:$RUN_GROUP" "$FRONTEND_DIR" 2>/dev/null || true
chmod -R 755 "$FRONTEND_DIR" 2>/dev/null || true

LOCAL_FRONTEND=""
for try_path in \
    "$SCRIPT_DIR/frontend/dist" \
    "$SCRIPT_DIR/dist"; do
    if [[ -f "$try_path/index.html" ]]; then
        LOCAL_FRONTEND="$try_path"
        break
    fi
done

if [[ -n "$LOCAL_FRONTEND" ]]; then
    if [[ ! -s "$LOCAL_FRONTEND/index.html" ]]; then
        echo -e "${RED}错误: 本地前端 index.html 为空，请重新构建${NC}" >&2
        exit 1
    fi
    echo "  使用本地构建产物: $LOCAL_FRONTEND"
    rm -rf "$FRONTEND_DIR/dist"
    cp -r "$LOCAL_FRONTEND" "$FRONTEND_DIR/dist"
else
    echo "  从 GitHub Releases 下载前端资源..."
    ensure_release_version || exit 1
    FRONTEND_URL=$(release_asset_url "flamepanel-frontend.tar.gz")
    echo "  下载地址: $FRONTEND_URL"
    if ! gh_download "$FRONTEND_URL" "$DOWNLOAD_DIR/frontend.tar.gz"; then
        echo -e "${RED}错误: 前端资源下载失败，安装已终止。请先构建:${NC}" >&2
        echo "    cd frontend && npm ci && npm run build" >&2
        exit 1
    fi
    verify_asset "$DOWNLOAD_DIR/frontend.tar.gz" "flamepanel-frontend.tar.gz" || exit 1
    mkdir -p "$DOWNLOAD_DIR/frontend"
    if ! tar -xzf "$DOWNLOAD_DIR/frontend.tar.gz" -C "$DOWNLOAD_DIR/frontend"; then
        echo -e "${RED}错误: 前端资源包无法解压，安装已终止${NC}" >&2
        exit 1
    fi
    if [[ ! -f "$DOWNLOAD_DIR/frontend/index.html" || ! -s "$DOWNLOAD_DIR/frontend/index.html" \
        || -L "$DOWNLOAD_DIR/frontend/index.html" ]]; then
        echo -e "${RED}错误: 前端资源包缺少有效的 index.html，安装已终止${NC}" >&2
        exit 1
    fi
    # 完成解包和入口检查后才替换，失败包不会先删除现有前端。
    rm -rf "$FRONTEND_DIR/dist"
    cp -r "$DOWNLOAD_DIR/frontend" "$FRONTEND_DIR/dist"
fi

echo -e "${GREEN}  -> 前端资源部署完成${NC}"

# 部署完成后最终收紧权限：dist 需可被 nginx 读取，但目录不被运行用户之外改写
chown -R "$RUN_USER:$RUN_GROUP" "$FRONTEND_DIR/dist" 2>/dev/null || true
chmod -R 755 "$FRONTEND_DIR/dist" 2>/dev/null || true

# ─── 配置 nginx 反向代理 ───────────────────────────────────────────────────────
NGINX_CONFIGURED=false
NGINX_CONF_PATH=""
NGINX_INCLUDE_ADDED=0
NGINX_DEFAULT_SITE_DISABLED=0
if command -v nginx &>/dev/null; then
    # HTTPS：准备证书（自定义或自签）
    if [[ "$ENABLE_TLS" == true ]]; then
        if [[ -z "$TLS_CERT_PATH" || -z "$TLS_KEY_PATH" ]]; then
            # 生成自签证书（3650 天）
            TLS_DIR="$INSTALL_DIR/tls"
            mkdir -p "$TLS_DIR"
            TLS_CERT_PATH="$TLS_DIR/flamepanel.crt"
            TLS_KEY_PATH="$TLS_DIR/flamepanel.key"
            if [[ -z "$TLS_DOMAIN" ]]; then
                TLS_DOMAIN="flamepanel.local"
            fi
            echo -e "${CYAN}  生成自签 TLS 证书 (域名: $TLS_DOMAIN)...${NC}"
            openssl req -x509 -newkey rsa:2048 -nodes \
                -keyout "$TLS_KEY_PATH" \
                -out "$TLS_CERT_PATH" \
                -days 3650 \
                -subj "/C=${TLS_COUNTRY}/O=${TLS_ORG}/CN=${TLS_DOMAIN}" \
                -addext "subjectAltName=DNS:${TLS_DOMAIN}" \
                2>/dev/null
            chown root:root "$TLS_CERT_PATH" "$TLS_KEY_PATH"
            chmod 644 "$TLS_CERT_PATH"
            chmod 600 "$TLS_KEY_PATH"
            chmod 755 "$TLS_DIR"
        fi
        echo -e "${CYAN}  使用 TLS 证书: $TLS_CERT_PATH${NC}"
    fi

    nginx_conf_dir
    NGINX_CONF_PATH="$NGINX_DIR/flamepanel.conf"
    disable_distro_default_site
    [[ "$NGINX_INCLUDE_ADDED" == 1 ]] && echo -e "${CYAN}  -> 已在 nginx.conf 中补充 conf.d include（原文件备份为 nginx.conf.flamepanel.bak）${NC}"
    [[ "$NGINX_DEFAULT_SITE_DISABLED" == 1 ]] && echo -e "${CYAN}  -> 已禁用发行版默认站点 sites-enabled/default（卸载时恢复）${NC}"
    # Doc/33 P-6：CSP 片段随前端产物下发；旧版前端包缺失时写降级片段（nginx -t 需要文件存在）
    CSP_SNIPPET="$FRONTEND_DIR/dist/csp.conf"
    if [[ ! -s "$CSP_SNIPPET" ]]; then
        echo -e "${YELLOW}  -> 前端产物缺少 csp.conf（旧版本），写入降级 CSP（不放行首屏内联脚本）${NC}"
        mkdir -p "$FRONTEND_DIR/dist"
        printf 'add_header Content-Security-Policy "%s" always;\n' "$CSP_FALLBACK_POLICY" > "$CSP_SNIPPET"
    fi
    # 本机支持 IPv6 时同时监听 [::]
    LISTEN6_80=""; LISTEN6_443=""
    if [[ -f /proc/net/if_inet6 ]]; then
        LISTEN6_80="listen [::]:80;"
        LISTEN6_443="listen [::]:443 ssl;"
    fi

    # 站点主体（80 / 443 共用）。注意 add_header 继承规则：自带 add_header 的 location
    # 不继承 server 级头，故在其中重复声明（护栏：frontend/src/config/__tests__/csp.spec.ts）。
    SERVER_BODY=$(cat << BODYEOF
    root $FRONTEND_DIR/dist;
    index index.html;

    # 与 nginx.conf（Docker）同源：文件管理上传 / 长任务 / WebSocket 终端
    client_max_body_size 2g;
    proxy_read_timeout 600s;

    add_header X-Content-Type-Options "nosniff" always;
    add_header X-Frame-Options "DENY" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;
    include $CSP_SNIPPET;

    # 压缩（与 Docker nginx.conf 同口径）：前端 JS / CSS / SVG 与 API 列表 JSON 是主要收益；
    # 登录 / 令牌端点在 /api/auth/ 单独关闭（响应含令牌且回显用户输入，避免 BREACH 类侧信道）。
    # ETag 与 Last-Modified 为 nginx 静态文件默认行为，无需额外配置。
    gzip on;
    gzip_comp_level 5;
    gzip_min_length 1024;
    gzip_vary on;
    gzip_proxied any;
    gzip_types text/plain text/css text/javascript application/javascript application/json application/xml image/svg+xml;

    # 后端健康检查 / Prometheus 指标（/metrics 另需 metrics_token）
    location = /health {
        proxy_pass http://127.0.0.1:$PANEL_PORT;
    }
    location = /metrics {
        proxy_pass http://127.0.0.1:$PANEL_PORT;
    }

    # 入口页禁止缓存：升级后浏览器立即拿到新的资源清单
    location = /index.html {
        add_header Cache-Control "no-cache";
        add_header X-Content-Type-Options "nosniff" always;
        add_header X-Frame-Options "DENY" always;
        include $CSP_SNIPPET;
    }

    # 前端路由回退
    location / {
        try_files \$uri \$uri/ /index.html;
    }

    # 认证端点：不压缩（见上方 gzip 注释），其余与 /api/ 相同
    location /api/auth/ {
        gzip off;
        proxy_pass http://127.0.0.1:$PANEL_PORT;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }

    # API 代理到 Rust 后端（后端仅监听 127.0.0.1）
    location /api/ {
        proxy_pass http://127.0.0.1:$PANEL_PORT;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }

    # WebSocket（终端 / 日志 / 指标）：长连接放宽读超时
    location /ws/ {
        proxy_pass http://127.0.0.1:$PANEL_PORT;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "Upgrade";
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_read_timeout 3600s;
    }

    # 静态资源：Vite 产物文件名带内容哈希，内容变即换名 → 一年 + immutable（浏览器不再重验证）
    location /assets/ {
        add_header Cache-Control "public, max-age=31536000, immutable";
        add_header X-Content-Type-Options "nosniff" always;
        include $CSP_SNIPPET;
    }
BODYEOF
)

    if [[ "$ENABLE_TLS" == true ]]; then
        cat > "$NGINX_CONF_PATH" << NGINXEOF
# Flamepanel（install.sh 生成）：80 → 443 重定向 + HTTPS 站点
server {
    listen 80;
    $LISTEN6_80
    server_name _;
    include $CSP_SNIPPET;
    location = /health {
        proxy_pass http://127.0.0.1:$PANEL_PORT;
    }
    location / {
        return 301 https://\$host\$request_uri;
    }
}

server {
    listen 443 ssl;
    $LISTEN6_443
    server_name _;

    ssl_certificate     $TLS_CERT_PATH;
    ssl_certificate_key $TLS_KEY_PATH;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    ssl_session_cache shared:SSL:10m;
    ssl_session_timeout 10m;
$SERVER_BODY
}
NGINXEOF
    else
        cat > "$NGINX_CONF_PATH" << NGINXEOF
# Flamepanel（install.sh 生成）：前端静态资源以 755 部署，nginx worker 可读取
server {
    listen 80;
    $LISTEN6_80
    server_name _;
$SERVER_BODY
}
NGINXEOF
    fi

    configure_selinux
    if nginx -t 2>/dev/null; then
        systemctl enable nginx 2>/dev/null || true
        systemctl restart nginx 2>/dev/null || true
        NGINX_CONFIGURED=true
        if [[ "$ENABLE_TLS" == true ]]; then
            open_firewall_ports 80 443
            echo -e "${GREEN}  -> nginx 反向代理配置完成 (https://本机IP/，worker 用户: $(nginx_worker_user))${NC}"
        else
            open_firewall_ports 80
            echo -e "${GREEN}  -> nginx 反向代理配置完成 (http://本机IP/，worker 用户: $(nginx_worker_user))${NC}"
        fi
    else
        echo -e "${YELLOW}  -> 警告: nginx 配置校验失败，请检查 $NGINX_CONF_PATH${NC}"
        nginx -t 2>&1 | tail -n 5 >&2 || true
        mv "$NGINX_CONF_PATH" "$NGINX_CONF_PATH.bak" 2>/dev/null || true
    fi
fi

# 安装状态（供 uninstall.sh 精确回滚 nginx 变更）
cat > "$INSTALL_DIR/.install-state" << STATEEOF
NGINX_CONF_PATH=$NGINX_CONF_PATH
NGINX_INCLUDE_ADDED=$NGINX_INCLUDE_ADDED
NGINX_DEFAULT_SITE_DISABLED=$NGINX_DEFAULT_SITE_DISABLED
NGINX_ROOT=${NGINX_ROOT:-/etc/nginx}
STATEEOF
chmod 600 "$INSTALL_DIR/.install-state"

# 后端监听地址：nginx 反代就绪时仅监听本机（Doc/33 §4：避免绕过 nginx 的 TLS/安全头直连）；
# 无 nginx 时保持 0.0.0.0，由用户自备反代。
if [[ "$NGINX_CONFIGURED" == true ]]; then
    PANEL_HOST="127.0.0.1"
else
    PANEL_HOST="0.0.0.0"
fi

# ─── 配置 systemd ──────────────────────────────────────────────────────────────
echo -e "${CYAN}[5/5] 配置 systemd 服务...${NC}"

# 密钥写入 600 权限的环境文件（属主为运行用户），避免出现在可读的 unit 文件中
cat > "$ENV_FILE" << ENVEOF
OP_PORT=$PANEL_PORT
OP_HOST=$PANEL_HOST
OP_ADMIN_USERNAME=$PANEL_USERNAME
OP_ADMIN_PASSWORD=$PANEL_PASSWORD
OP_JWT_SECRET=$JWT_SECRET
OP_DATABASE_URL=sqlite:$INSTALL_DIR/data/flamepanel.db?mode=rwc
OP_FILE_ROOT=$INSTALL_DIR/workspace
OP_TERMINAL_CWD=$INSTALL_DIR/workspace
# OP_SMTP_HOST=smtp.example.com
# OP_SMTP_PORT=587
# OP_SMTP_USERNAME=
# OP_SMTP_PASSWORD=
# OP_SMTP_FROM=noreply@flamepanel.local
ENVEOF
chown "$RUN_USER:$RUN_GROUP" "$ENV_FILE"
chmod 600 "$ENV_FILE"

# 文件沙箱白名单根目录（终端/文件 API 仅可访问其内部）
mkdir -p "$INSTALL_DIR/workspace"
chown "$RUN_USER:$RUN_GROUP" "$INSTALL_DIR/workspace"
chmod 750 "$INSTALL_DIR/workspace"

# 默认非 root 运行（最小权限）+ systemd 加固：
# - NoNewPrivileges / ProtectSystem=strict / ProtectHome=true 防止提权与系统目录篡改
# - ReadWritePaths 仅放行数据、日志、工作区
# - NoNewPrivileges 下 sudo 不可用：Web 引擎站点配置读写与引擎校验/reload 由 root 特权助手
#   flamepanel-helper.service 代办（同一二进制 `flamepanel helper`，Unix socket + 路径/命令白名单，
#   Doc/32 W3-b）；防火墙规则同样经助手执行（往返校验白名单，Doc/30 助手扩展）；
#   其它需要 root 的操作（数据库 / 运行时 / 软件安装）请以 root / Docker 方式部署
# - 如需操作 docker.sock，可追加 Environment=OP_DOCKER_SOCKET=unix:///var/run/docker.sock
#   并在 systemd 中补充 SupplementaryGroups=docker（最小化，不授予 CAP_SYS_ADMIN）
# 本机特权助手（Doc/32 W3-b）：root 运行，只接受面板用户经 socket 发来的 Web 引擎配置读写
# 与白名单引擎命令（nginx -t / reload / systemctl start|stop <引擎> 等），以及防火墙命令
# （ufw / firewall-cmd / iptables：固定命令 + 规则命令往返校验）。
cat > /etc/systemd/system/flamepanel-helper.service << SYSTEMDEOF
[Unit]
Description=Flamepanel privileged helper (web engine config + reload, firewall)
Documentation=https://github.com/Forevery1021/Flamepanel
Before=flamepanel.service

[Service]
Type=simple
ExecStart=/usr/local/bin/flamepanel helper --allow-user $RUN_USER
RuntimeDirectory=flamepanel-helper
RuntimeDirectoryMode=0755
UMask=0022
Restart=always
RestartSec=3

# 加固：助手需写 /etc 下引擎配置并控制引擎服务，故不启用 ProtectSystem；其余面收紧
NoNewPrivileges=true
ProtectHome=true
PrivateTmp=true
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
# AF_NETLINK：iptables-nft / ufw 经 netlink 写规则
RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6 AF_NETLINK
LockPersonality=true

[Install]
WantedBy=multi-user.target
SYSTEMDEOF

cat > /etc/systemd/system/flamepanel.service << SYSTEMDEOF
[Unit]
Description=Flamepanel - Server Operations Management Panel
Documentation=https://github.com/Forevery1021/Flamepanel
After=network.target flamepanel-helper.service
Wants=flamepanel-helper.service

[Service]
Type=simple
User=$RUN_USER
Group=$RUN_GROUP
ExecStart=/usr/local/bin/flamepanel
WorkingDirectory=$INSTALL_DIR
EnvironmentFile=$ENV_FILE
Restart=always
RestartSec=5

# 安全加固：禁止提权、严格只读系统路径、隔离 HOME
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
# 仅允许写入数据/日志/工作区
ReadWritePaths=$INSTALL_DIR/data $INSTALL_DIR/logs $INSTALL_DIR/workspace

# 资源限制
LimitNOFILE=65535
LimitNPROC=32768

[Install]
WantedBy=multi-user.target
SYSTEMDEOF

# 收编为版本化布局（releases/<版本> + current），为 `fpctl upgrade` / `fpctl rollback` 做准备；失败不影响安装
if [[ -x "$CTL_PATH" ]]; then
    if FP_BIN_DIR="$(dirname "$BINARY_PATH")" "$CTL_PATH" --dir "$INSTALL_DIR" adopt >/dev/null 2>&1; then
        chown -h "$RUN_USER:$RUN_GROUP" "$INSTALL_DIR/current" "$INSTALL_DIR/frontend" 2>/dev/null || true
        echo -e "${GREEN}  -> 已启用版本化布局: $INSTALL_DIR/current -> $(readlink "$INSTALL_DIR/current" 2>/dev/null)${NC}"
    else
        echo -e "${YELLOW}  -> 版本化布局收编失败（不影响运行），可稍后执行: sudo fpctl adopt${NC}"
    fi
fi

systemctl daemon-reload
systemctl enable flamepanel-helper 2>/dev/null || true
# 助手先于面板就绪：面板启动时据 socket 是否存在决定是否委托（失败不阻断安装，面板退回只读部署）
if systemctl restart flamepanel-helper 2>/dev/null; then
    echo -e "${GREEN}  -> 特权助手已启动: flamepanel-helper（/run/flamepanel-helper/helper.sock）${NC}"
else
    echo -e "${YELLOW}  -> 特权助手启动失败（站点配置将无法写入 /etc），可查看: journalctl -u flamepanel-helper${NC}"
fi
systemctl enable flamepanel 2>/dev/null || true

# 尝试启动
if systemctl start flamepanel 2>/dev/null; then
    sleep 2
    if systemctl is-active --quiet flamepanel; then
        echo -e "${GREEN}  -> 服务启动成功${NC}"
    else
        echo -e "${YELLOW}  -> 服务已配置但启动失败，请检查日志: journalctl -u flamepanel -n 50${NC}"
    fi
else
    echo -e "${YELLOW}  -> 服务配置完成，请手动启动: systemctl start flamepanel${NC}"
fi

# ─── 获取外网 IP ───────────────────────────────────────────────────────────────
EXTERNAL_IP=$(curl -s --connect-timeout 3 ifconfig.me 2>/dev/null || curl -s --connect-timeout 3 ip.sb 2>/dev/null || echo "YOUR_SERVER_IP")
LOCAL_IP=$(hostname -I 2>/dev/null | awk '{print $1}' || echo "127.0.0.1")

# ─── 完成输出 ──────────────────────────────────────────────────────────────────
echo ""
echo -e "${GREEN}══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}          Flamepanel 安装完成！${NC}"
echo -e "${GREEN}══════════════════════════════════════════════════════${NC}"
echo ""
if [[ "$NGINX_CONFIGURED" == true ]]; then
    SCHEME="http"
    [[ "$ENABLE_TLS" == true ]] && SCHEME="https"
    echo -e "  ${CYAN}访问地址 (Web):${NC}"
    echo -e "    本地:   ${SCHEME}://${LOCAL_IP}/"
    if [[ "$EXTERNAL_IP" != "$LOCAL_IP" ]]; then
        echo -e "    外网:   ${SCHEME}://${EXTERNAL_IP}/"
    fi
    echo -e "  ${CYAN}后端 API:${NC} 仅本机监听 http://127.0.0.1:${PANEL_PORT}（经 nginx /api/ 对外）"
    if [[ "$ENABLE_TLS" == true ]]; then
        echo -e "  ${YELLOW}提示: 自签证书需在浏览器中手动信任；生产建议使用 CA 签发的证书。${NC}"
    fi
    echo -e "  ${YELLOW}云服务器请在安全组中放行 ${SCHEME^^} 端口（80$( [[ "$ENABLE_TLS" == true ]] && echo '/443')）。${NC}"
else
    echo -e "  ${YELLOW}未配置 nginx：后端仅提供 API，前端需自备 Web 服务器指向 $INSTALL_DIR/frontend/dist${NC}"
    echo -e "  ${CYAN}后端 API:${NC} http://${LOCAL_IP}:${PANEL_PORT}/api"
fi
echo ""
echo -e "  ${CYAN}登录信息:${NC}"
echo -e "    用户名: ${GREEN}${PANEL_USERNAME}${NC}"
echo -e "    密码:   ${GREEN}${PANEL_PASSWORD}${NC}"
echo ""
echo -e "  ${CYAN}服务管理:${NC}"
if [[ -x "$CTL_PATH" ]]; then
echo -e "    fpctl status                           # 状态 / 健康检查"
echo -e "    sudo fpctl restart                     # 重启并等待就绪"
echo -e "    fpctl logs -f                          # 实时内核日志"
echo -e "    fpctl doctor                           # 一键诊断"
echo -e "    sudo fpctl reset-password --yes        # 忘记密码时重置"
echo -e "    fpctl --help                           # 全部命令（--json 供脚本 / AI）"
else
echo -e "    systemctl status  flamepanel           # 查看状态"
echo -e "    systemctl restart flamepanel           # 重启服务"
echo -e "    systemctl stop    flamepanel           # 停止服务"
echo -e "    journalctl -u flamepanel -f            # 实时日志"
fi
echo ""
echo -e "  ${CYAN}卸载:${NC}"
echo -e "    sudo ./uninstall.sh                    # 卸载（保留数据）"
echo -e "    sudo ./uninstall.sh -p                 # 完全卸载（删除数据）"
echo ""
echo -e "  ${YELLOW}请妥善保管以上登录信息！${NC}"
if [[ "$NON_INTERACTIVE" == true ]] || [[ -z "$PANEL_PASSWORD" ]]; then
    echo -e "  ${YELLOW}⚠ 密码为自动生成/随机值，请立即登录并修改默认密码！${NC}"
    echo -e "  ${YELLOW}  首次登录面板将强制要求修改初始密码（v0.6.0+ 机制）。${NC}"
else
    echo -e "  ${YELLOW}首次登录需修改初始密码（面板强制改密机制，v0.6.0+）。${NC}"
fi
echo ""

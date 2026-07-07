#!/bin/bash
set -e

# ─── 默认值 ────────────────────────────────────────────────────────────────────
PANEL_USERNAME="admin"
PANEL_PASSWORD=""
PANEL_PORT="8080"
JWT_SECRET=""
NON_INTERACTIVE=false

# ─── 颜色 ──────────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# ─── 帮助 ──────────────────────────────────────────────────────────────────────
usage() {
    cat << EOF
Flamepanel 安装脚本

用法: $0 [选项]

选项:
  -u, --username NAME    管理员用户名 (默认: admin)
  -p, --password PASS    管理员密码 (默认: 交互输入)
  -P, --port PORT        面板监听端口 (默认: 8080)
  -s, --secret SECRET    JWT 签名密钥 (默认: 自动生成)
  -n, --non-interactive  非交互模式，使用默认值 (密码将自动生成)
  -h, --help             显示帮助信息

示例:
  $0                                          # 交互式安装
  $0 -u myadmin -p mypass -P 9090             # 自定义账号和端口
  $0 -n                                       # 静默安装，全部使用默认值
  $0 -u ops -p 'Str0ng!P@ss' -P 443 -s 'xxx' # 完整自定义
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
        -h|--help)
            usage
            ;;
        *)
            echo -e "${RED}未知参数: $1${NC}"
            usage
            ;;
    esac
done

# ─── 检查 root ─────────────────────────────────────────────────────────────────
if [[ $EUID -ne 0 ]]; then
    echo -e "${RED}请使用 root 权限运行此脚本${NC}"
    exit 1
fi

# ─── 系统检测 ──────────────────────────────────────────────────────────────────
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$ID
else
    OS="unknown"
fi
echo -e "${CYAN}检测到系统:${NC} $OS"

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
            if [[ ${#input_password} -lt 6 ]]; then
                echo -e "${RED}密码长度不能少于 6 位${NC}"
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

# 非交互模式下自动生成密码
if [[ -z "$PANEL_PASSWORD" ]]; then
    PANEL_PASSWORD=$(openssl rand -base64 12 2>/dev/null || head -c 12 /dev/urandom | base64)
fi

# 自动生成 JWT 密钥
if [[ -z "$JWT_SECRET" ]]; then
    JWT_SECRET=$(openssl rand -hex 32 2>/dev/null || head -c 32 /dev/urandom | xxd -p)
fi

# ─── 安装依赖 ──────────────────────────────────────────────────────────────────
echo ""
echo -e "${CYAN}[1/4] 安装系统依赖...${NC}"

if command -v apt-get &> /dev/null; then
    apt-get update -qq
    apt-get install -y -qq curl wget unzip openssl 2>/dev/null
elif command -v yum &> /dev/null; then
    yum install -y -q curl wget unzip openssl 2>/dev/null
elif command -v dnf &> /dev/null; then
    dnf install -y -q curl wget unzip openssl 2>/dev/null
elif command -v pacman &> /dev/null; then
    pacman -S --noconfirm curl wget unzip openssl 2>/dev/null
fi

echo -e "${GREEN}  -> 依赖安装完成${NC}"

# ─── 创建目录结构 ──────────────────────────────────────────────────────────────
echo -e "${CYAN}[2/4] 创建目录结构...${NC}"

INSTALL_DIR="/opt/flamepanel"
mkdir -p "$INSTALL_DIR/data"
mkdir -p "$INSTALL_DIR/logs"

echo -e "${GREEN}  -> $INSTALL_DIR${NC}"

# ─── 部署二进制 ────────────────────────────────────────────────────────────────
echo -e "${CYAN}[3/4] 部署应用...${NC}"

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

if [[ -n "$LOCAL_BINARY" ]]; then
    echo "  使用本地编译产物: $LOCAL_BINARY"
    cp "$LOCAL_BINARY" /usr/local/bin/flamepanel
    chmod +x /usr/local/bin/flamepanel
else
    echo "  从 GitHub Releases 下载..."
    ARCH=$(uname -m)
    case "$ARCH" in
        x86_64)  ARCH="amd64" ;;
        aarch64) ARCH="arm64" ;;
    esac
    DOWNLOAD_URL="https://github.com/Forevery1021/Flamepanel/releases/latest/download/flamepanel-linux-${ARCH}.tar.gz"
    echo "  下载地址: $DOWNLOAD_URL"
    if curl -L --connect-timeout 10 "$DOWNLOAD_URL" -o /tmp/flamepanel.tar.gz 2>/dev/null; then
        tar -xzf /tmp/flamepanel.tar.gz -C /usr/local/bin/ 2>/dev/null
        rm -f /tmp/flamepanel.tar.gz
        chmod +x /usr/local/bin/flamepanel
        echo -e "${GREEN}  -> 下载部署完成${NC}"
    else
        echo -e "${YELLOW}  警告: 未找到预编译二进制，请手动编译:${NC}"
        echo -e "    cd flame-kernel && cargo build --release"
        echo -e "    cp target/release/flame-kernel /usr/local/bin/flamepanel"
    fi
fi

chmod +x /usr/local/bin/flamepanel 2>/dev/null || true
echo -e "${GREEN}  -> 二进制部署完成${NC}"

# ─── 配置 systemd ──────────────────────────────────────────────────────────────
echo -e "${CYAN}[4/4] 配置 systemd 服务...${NC}"

cat > /etc/systemd/system/flamepanel.service << SYSTEMDEOF
[Unit]
Description=Flamepanel - Server Operations Management Panel
Documentation=https://github.com/Forevery1021/Flamepanel
After=network.target

[Service]
Type=simple
User=root
ExecStart=/usr/local/bin/flamepanel
WorkingDirectory=$INSTALL_DIR
Restart=always
RestartSec=5

# 资源限制
LimitNOFILE=65535
LimitNPROC=32768

# 环境变量
Environment=OP_PORT=$PANEL_PORT
Environment=OP_ADMIN_USERNAME=$PANEL_USERNAME
Environment=OP_ADMIN_PASSWORD=$PANEL_PASSWORD
Environment=OP_JWT_SECRET=$JWT_SECRET
Environment=OP_DATABASE_URL=sqlite:$INSTALL_DIR/data/flamepanel.db?mode=rwc

[Install]
WantedBy=multi-user.target
SYSTEMDEOF

systemctl daemon-reload
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
echo -e "  ${CYAN}访问地址:${NC}"
echo -e "    本地:   http://${LOCAL_IP}:${PANEL_PORT}"
if [[ "$EXTERNAL_IP" != "$LOCAL_IP" ]]; then
    echo -e "    外网:   http://${EXTERNAL_IP}:${PANEL_PORT}"
fi
echo ""
echo -e "  ${CYAN}登录信息:${NC}"
echo -e "    用户名: ${GREEN}${PANEL_USERNAME}${NC}"
echo -e "    密码:   ${GREEN}${PANEL_PASSWORD}${NC}"
echo ""
echo -e "  ${CYAN}服务管理:${NC}"
echo -e "    systemctl status  flamepanel   # 查看状态"
echo -e "    systemctl restart flamepanel   # 重启服务"
echo -e "    systemctl stop    flamepanel   # 停止服务"
echo -e "    journalctl -u flamepanel -f    # 实时日志"
echo ""
echo -e "  ${YELLOW}请妥善保管以上登录信息！${NC}"
echo -e "  ${YELLOW}建议首次登录后立即修改密码。${NC}"
echo ""

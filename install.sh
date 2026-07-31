#!/bin/bash
set -euo pipefail

# ─── 版本 ──────────────────────────────────────────────────────────────────────
VERSION="1.1"

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
  -n, --non-interactive  非交互模式，使用默认值 (密码将自动生成)
  -h, --help             显示帮助信息

说明:
  - 部署二进制到 /usr/local/bin/flamepanel
  - 前端静态资源部署到 /opt/flamepanel/frontend
  - 自动配置 nginx 反向代理 (80 端口 -> 后端 API/WebSocket)
  - 使用方式二/三时需先本地构建或在 GitHub Releases 提供产物

示例:
  $0                                          # 交互式安装
  $0 -u myadmin -p mypass -P 9090             # 自定义账号和端口
  $0 -n                                       # 静默安装，全部使用默认值
  $0 -u ops -p 'Str0ng!P@ss' -P 443 -s 'xxx' # 完整自定义

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
    echo -e "${RED}请使用 root 权限运行此脚本 (sudo)${NC}"
    exit 1
fi

# ─── 系统检测 ──────────────────────────────────────────────────────────────────
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$ID
    OS_VERSION=$VERSION_ID
else
    OS="unknown"
    OS_VERSION="unknown"
fi
echo -e "${CYAN}检测到系统:${NC} $OS $OS_VERSION"

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
echo -e "${CYAN}[1/5] 安装系统依赖...${NC}"

if command -v apt-get &> /dev/null; then
    apt-get update -qq
    apt-get install -y -qq curl wget unzip openssl nginx 2>/dev/null
elif command -v yum &> /dev/null; then
    yum install -y -q curl wget unzip openssl nginx 2>/dev/null
elif command -v dnf &> /dev/null; then
    dnf install -y -q curl wget unzip openssl nginx 2>/dev/null
elif command -v pacman &> /dev/null; then
    pacman -S --noconfirm curl wget unzip openssl nginx 2>/dev/null
fi

if ! command -v nginx &>/dev/null; then
    echo -e "${YELLOW}  -> 警告: nginx 安装失败，将跳过 Web 反向代理配置${NC}"
    echo -e "${YELLOW}     前端页面需自行配置 Web 服务器指向 $INSTALL_DIR/frontend/dist${NC}"
else
    echo -e "${GREEN}  -> 依赖安装完成${NC}"
fi

# ─── 创建目录结构 ──────────────────────────────────────────────────────────────
echo -e "${CYAN}[2/5] 创建目录结构...${NC}"

mkdir -p "$INSTALL_DIR/data"
mkdir -p "$INSTALL_DIR/logs"

echo -e "${GREEN}  -> $INSTALL_DIR${NC}"

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

# ─── 部署前端静态资源 ─────────────────────────────────────────────────────────
echo -e "${CYAN}[4/5] 部署前端静态资源...${NC}"

FRONTEND_DIR="$INSTALL_DIR/frontend"
mkdir -p "$FRONTEND_DIR"

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
    echo "  使用本地构建产物: $LOCAL_FRONTEND"
    rm -rf "$FRONTEND_DIR/dist"
    cp -r "$LOCAL_FRONTEND" "$FRONTEND_DIR/dist"
    echo -e "${GREEN}  -> 前端资源部署完成${NC}"
else
    echo "  从 GitHub Releases 下载前端资源..."
    FRONTEND_URL="https://github.com/Forevery1021/Flamepanel/releases/latest/download/flamepanel-frontend.tar.gz"
    echo "  下载地址: $FRONTEND_URL"
    if curl -L --connect-timeout 10 "$FRONTEND_URL" -o /tmp/flamepanel-frontend.tar.gz 2>/dev/null; then
        rm -rf "$FRONTEND_DIR/dist"
        mkdir -p "$FRONTEND_DIR/dist"
        tar -xzf /tmp/flamepanel-frontend.tar.gz -C "$FRONTEND_DIR/dist" 2>/dev/null
        rm -f /tmp/flamepanel-frontend.tar.gz
        if [[ -f "$FRONTEND_DIR/dist/index.html" ]]; then
            echo -e "${GREEN}  -> 前端资源下载部署完成${NC}"
        else
            echo -e "${YELLOW}  警告: 前端资源包格式异常，请检查 Releases 产物${NC}"
        fi
    else
        echo -e "${YELLOW}  警告: 未找到预编译前端资源，请手动构建:${NC}"
        echo -e "    cd frontend && npm run build"
        echo -e "    sudo mkdir -p $FRONTEND_DIR && sudo cp -r dist $FRONTEND_DIR/"
    fi
fi

# ─── 配置 nginx 反向代理 ───────────────────────────────────────────────────────
if command -v nginx &>/dev/null; then
    cat > /etc/nginx/conf.d/flamepanel.conf << NGINXEOF
server {
    listen 80;
    server_name _;

    root $FRONTEND_DIR/dist;
    index index.html;

    # 前端路由回退
    location / {
        try_files \$uri \$uri/ /index.html;
    }

    # API 代理到 Rust 后端
    location /api/ {
        proxy_pass http://127.0.0.1:$PANEL_PORT;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }

    # WebSocket 支持（终端）
    location /ws/ {
        proxy_pass http://127.0.0.1:$PANEL_PORT;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "Upgrade";
        proxy_set_header Host \$host;
    }

    # 静态资源缓存
    location /assets/ {
        expires 30d;
        add_header Cache-Control "public";
    }
}
NGINXEOF

    if nginx -t 2>/dev/null; then
        systemctl enable nginx 2>/dev/null || true
        systemctl restart nginx 2>/dev/null || true
        echo -e "${GREEN}  -> nginx 反向代理配置完成 (http://本机IP/)${NC}"
    else
        echo -e "${YELLOW}  -> 警告: nginx 配置校验失败，请检查 /etc/nginx/conf.d/flamepanel.conf${NC}"
        mv /etc/nginx/conf.d/flamepanel.conf /etc/nginx/conf.d/flamepanel.conf.bak 2>/dev/null || true
    fi
fi

# ─── 配置 systemd ──────────────────────────────────────────────────────────────
echo -e "${CYAN}[5/5] 配置 systemd 服务...${NC}"

# 密钥写入 600 权限的环境文件，避免出现在可读的 unit 文件中
cat > "$INSTALL_DIR/flamepanel.env" << ENVEOF
OP_PORT=$PANEL_PORT
OP_HOST=0.0.0.0
OP_ADMIN_USERNAME=$PANEL_USERNAME
OP_ADMIN_PASSWORD=$PANEL_PASSWORD
OP_JWT_SECRET=$JWT_SECRET
OP_DATABASE_URL=sqlite:$INSTALL_DIR/data/flamepanel.db?mode=rwc
# OP_SMTP_HOST=smtp.example.com
# OP_SMTP_PORT=587
# OP_SMTP_USERNAME=
# OP_SMTP_PASSWORD=
# OP_SMTP_FROM=noreply@flamepanel.local
ENVEOF
chmod 600 "$INSTALL_DIR/flamepanel.env"

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
EnvironmentFile=$INSTALL_DIR/flamepanel.env
Restart=always
RestartSec=5

# 资源限制
LimitNOFILE=65535
LimitNPROC=32768

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
if command -v nginx &>/dev/null; then
    echo -e "  ${CYAN}访问地址 (Web):${NC}"
    echo -e "    本地:   http://${LOCAL_IP}/"
    if [[ "$EXTERNAL_IP" != "$LOCAL_IP" ]]; then
        echo -e "    外网:   http://${EXTERNAL_IP}/"
    fi
    echo -e "  ${CYAN}后端 API:${NC} http://${LOCAL_IP}:${PANEL_PORT}/api"
else
    echo -e "  ${CYAN}访问地址:${NC}"
    echo -e "    本地:   http://${LOCAL_IP}:${PANEL_PORT}"
    if [[ "$EXTERNAL_IP" != "$LOCAL_IP" ]]; then
        echo -e "    外网:   http://${EXTERNAL_IP}:${PANEL_PORT}"
    fi
fi
echo ""
echo -e "  ${CYAN}登录信息:${NC}"
echo -e "    用户名: ${GREEN}${PANEL_USERNAME}${NC}"
echo -e "    密码:   ${GREEN}${PANEL_PASSWORD}${NC}"
echo ""
echo -e "  ${CYAN}服务管理:${NC}"
echo -e "    systemctl status  flamepanel           # 查看状态"
echo -e "    systemctl restart flamepanel           # 重启服务"
echo -e "    systemctl stop    flamepanel           # 停止服务"
echo -e "    journalctl -u flamepanel -f            # 实时日志"
echo ""
echo -e "  ${CYAN}卸载:${NC}"
echo -e "    sudo ./uninstall.sh                    # 卸载（保留数据）"
echo -e "    sudo ./uninstall.sh -p                 # 完全卸载（删除数据）"
echo ""
echo -e "  ${YELLOW}请妥善保管以上登录信息！${NC}"
echo -e "  ${YELLOW}建议首次登录后立即修改密码。${NC}"
echo ""

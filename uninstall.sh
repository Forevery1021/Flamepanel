#!/bin/bash
set -e

# ─── 颜色 ──────────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# ─── 配置 ──────────────────────────────────────────────────────────────────────
INSTALL_DIR="/opt/flamepanel"
SERVICE_NAME="flamepanel"
BINARY_PATH="/usr/local/bin/flamepanel"
CTL_PATH="/usr/local/bin/fpctl"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
NGINX_CONF="/etc/nginx/conf.d/flamepanel.conf"
RUN_USER="flamepanel"
RUN_GROUP="flamepanel"

usage() {
    cat << EOF
Flamepanel 卸载脚本

用法: $0 [选项]

选项:
  -f, --force      跳过确认提示
  -p, --purge      同时删除数据目录（数据库将丢失）
  -k, --keep-apps  卸载面板但保留全部应用数据（data/apps，零锁定纪律）
  -h, --help       显示帮助信息

示例:
  $0                # 交互式卸载（保留数据）
  $0 -f             # 静默卸载（保留数据）
  $0 -p             # 交互式卸载并删除数据
  $0 -f -p          # 完全卸载（删除所有数据）
  $0 -f --keep-apps # 卸载面板，应用数据（data/apps）保留可迁移
EOF
    exit 0
}

FORCE=false
PURGE=false
KEEP_APPS=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        -f|--force)
            FORCE=true
            shift
            ;;
        -p|--purge)
            PURGE=true
            shift
            ;;
        -k|--keep-apps)
            KEEP_APPS=true
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

# M-M1 零锁定纪律：--keep-apps 与 --purge 互斥（keep-apps 语义即保留应用数据）
if [[ "$KEEP_APPS" == true && "$PURGE" == true ]]; then
    echo -e "${RED}--keep-apps 与 --purge 不能同时使用${NC}"
    exit 1
fi

# ─── 检查 root ─────────────────────────────────────────────────────────────────
if [[ $EUID -ne 0 ]]; then
    echo -e "${RED}请使用 root 权限运行此脚本${NC}"
    exit 1
fi

# ─── 检查是否已安装 ─────────────────────────────────────────────────────────────
if [[ ! -f "$BINARY_PATH" ]] && [[ ! -f "$SERVICE_FILE" ]] && [[ ! -d "$INSTALL_DIR" ]]; then
    echo -e "${YELLOW}Flamepanel 未安装或已经卸载。${NC}"
    exit 0
fi

# ─── 确认 ──────────────────────────────────────────────────────────────────────
echo -e "${CYAN}══════════════════════════════════════════${NC}"
echo -e "${CYAN}       Flamepanel 卸载${NC}"
echo -e "${CYAN}══════════════════════════════════════════${NC}"
echo ""
echo -e "  将执行以下操作:"
echo -e "    ${YELLOW}•${NC} 停止并禁用 systemd 服务"
echo -e "    ${YELLOW}•${NC} 删除二进制文件: $BINARY_PATH、$CTL_PATH"
echo -e "    ${YELLOW}•${NC} 删除 nginx 反向代理配置"
echo -e "    ${YELLOW}•${NC} 删除 systemd 服务文件"
if [[ "$PURGE" == true ]]; then
    echo -e "    ${RED}•${NC} 删除数据目录: $INSTALL_DIR"
elif [[ "$KEEP_APPS" == true ]]; then
    echo -e "    ${GREEN}•${NC} 保留应用数据: $INSTALL_DIR/data/apps、data/compose（可迁移/重装恢复）"
    echo -e "    ${YELLOW}•${NC} 保留其余数据: $INSTALL_DIR"
else
    echo -e "    ${YELLOW}•${NC} 保留数据目录: $INSTALL_DIR"
fi
echo ""

if [[ "$FORCE" == false ]]; then
    read -p "确认卸载? [y/N] " confirm
    if [[ "$confirm" != "y" ]] && [[ "$confirm" != "Y" ]]; then
        echo "卸载已取消"
        exit 0
    fi
fi

echo ""

# ─── 停止服务 ──────────────────────────────────────────────────────────────────
echo -e "${CYAN}[1/4] 停止并禁用服务...${NC}"
if systemctl is-active --quiet "$SERVICE_NAME" 2>/dev/null; then
    systemctl stop "$SERVICE_NAME"
    echo -e "${GREEN}  -> 服务已停止${NC}"
else
    echo -e "${YELLOW}  -> 服务未运行，跳过${NC}"
fi

if systemctl is-enabled --quiet "$SERVICE_NAME" 2>/dev/null; then
    systemctl disable "$SERVICE_NAME" 2>/dev/null || true
    echo -e "${GREEN}  -> 服务已禁用${NC}"
fi
# 特权助手（Doc/32 W3-b）
if [[ -f /etc/systemd/system/flamepanel-helper.service ]]; then
    systemctl disable --now flamepanel-helper 2>/dev/null || true
    rm -f /etc/systemd/system/flamepanel-helper.service
    echo -e "${GREEN}  -> 特权助手已停止并移除${NC}"
fi

# ─── 删除二进制 ────────────────────────────────────────────────────────────────
echo -e "${CYAN}[2/5] 删除二进制文件...${NC}"
if [[ -f "$BINARY_PATH" || -L "$BINARY_PATH" ]]; then
    rm -f "$BINARY_PATH"
    echo -e "${GREEN}  -> 已删除: $BINARY_PATH${NC}"
else
    echo -e "${YELLOW}  -> 未找到二进制文件，跳过${NC}"
fi
if [[ -f "$CTL_PATH" || -L "$CTL_PATH" ]]; then
    rm -f "$CTL_PATH"
    echo -e "${GREEN}  -> 已删除: $CTL_PATH${NC}"
fi

# ─── 删除 nginx 反向代理配置 ──────────────────────────────────────────────────
echo -e "${CYAN}[3/5] 删除 nginx 反向代理配置...${NC}"
# 读取 install.sh 写入的安装状态（仅解析已知键，不 source 任意内容）
state_get() {
    local f="$INSTALL_DIR/.install-state"
    [[ -f "$f" ]] && sed -n "s/^$1=//p" "$f" | head -n1
}
STATE_CONF=$(state_get NGINX_CONF_PATH || true)
STATE_ROOT=$(state_get NGINX_ROOT || true)
STATE_ROOT="${STATE_ROOT:-/etc/nginx}"
NGINX_TOUCHED=false
for conf in "${STATE_CONF:-}" "$NGINX_CONF" "/etc/nginx/http.d/flamepanel.conf"; do
    [[ -z "$conf" ]] && continue
    if [[ -f "$conf" ]] || [[ -f "$conf.bak" ]]; then
        rm -f "$conf" "$conf.bak"
        echo -e "${GREEN}  -> 已删除: $conf${NC}"
        NGINX_TOUCHED=true
    fi
done
# 回滚：Debian/Ubuntu 默认站点
if [[ "$(state_get NGINX_DEFAULT_SITE_DISABLED || true)" == "1" ]] \
    && [[ -f "$STATE_ROOT/sites-available/default" ]] && [[ ! -e "$STATE_ROOT/sites-enabled/default" ]]; then
    ln -s "$STATE_ROOT/sites-available/default" "$STATE_ROOT/sites-enabled/default"
    echo -e "${GREEN}  -> 已恢复发行版默认站点 sites-enabled/default${NC}"
    NGINX_TOUCHED=true
fi
# 回滚：Arch 等补充的 conf.d include（仅删除带标记的那一行）
if [[ "$(state_get NGINX_INCLUDE_ADDED || true)" == "1" ]] && [[ -f "$STATE_ROOT/nginx.conf" ]]; then
    sed -i '/conf\.d\/\*\.conf;[[:space:]]*# flamepanel$/d' "$STATE_ROOT/nginx.conf"
    echo -e "${GREEN}  -> 已移除 nginx.conf 中由安装脚本补充的 include${NC}"
    NGINX_TOUCHED=true
fi
if [[ "$NGINX_TOUCHED" == true ]]; then
    if command -v nginx &>/dev/null; then
        nginx -t 2>/dev/null && systemctl restart nginx 2>/dev/null || true
    fi
else
    echo -e "${YELLOW}  -> 未找到 nginx 配置，跳过${NC}"
fi

# ─── 删除 systemd 服务文件 ─────────────────────────────────────────────────────
echo -e "${CYAN}[4/5] 删除 systemd 服务文件...${NC}"
if [[ -f "$SERVICE_FILE" ]]; then
    rm -f "$SERVICE_FILE"
    systemctl daemon-reload
    echo -e "${GREEN}  -> 已删除: $SERVICE_FILE${NC}"
else
    echo -e "${YELLOW}  -> 未找到服务文件，跳过${NC}"
fi

# ─── 删除数据目录 ──────────────────────────────────────────────────────────────
echo -e "${CYAN}[5/5] 处理数据目录...${NC}"
if [[ -d "$INSTALL_DIR" ]]; then
    if [[ "$PURGE" == true ]]; then
        rm -rf "$INSTALL_DIR"
        echo -e "${RED}  -> 已删除: $INSTALL_DIR${NC}"
        # 清理专用运行用户/组（仅完全卸载时）
        if id "$RUN_USER" >/dev/null 2>&1; then
            userdel "$RUN_USER" 2>/dev/null || true
            echo -e "${GREEN}  -> 已删除系统用户 $RUN_USER${NC}"
        fi
        if getent group "$RUN_GROUP" >/dev/null 2>&1; then
            groupdel "$RUN_GROUP" 2>/dev/null || true
            echo -e "${GREEN}  -> 已删除系统组 $RUN_GROUP${NC}"
        fi
    elif [[ "$KEEP_APPS" == true ]]; then
        # 零锁定纪律（Doc/20 §7.1）：面板可弃，应用数据保留
        echo -e "${GREEN}  -> 保留应用数据: $INSTALL_DIR/data/apps、$INSTALL_DIR/data/compose${NC}"
        echo -e "     （容器与 compose 已落盘，重装面板后可恢复管理）"
        echo -e "${YELLOW}  -> 其余面板数据已随目录保留: $INSTALL_DIR${NC}"
    else
        echo -e "${YELLOW}  -> 保留: $INSTALL_DIR${NC}"
        echo -e "${YELLOW}  -> 如需删除请手动运行: rm -rf $INSTALL_DIR${NC}"
    fi
else
    echo -e "${YELLOW}  -> 数据目录不存在，跳过${NC}"
fi

# ─── 完成 ──────────────────────────────────────────────────────────────────────
echo ""
echo -e "${GREEN}══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}          Flamepanel 卸载完成！${NC}"
echo -e "${GREEN}══════════════════════════════════════════════════════${NC}"
echo ""

if [[ "$PURGE" == false ]] && [[ -d "$INSTALL_DIR" ]]; then
    echo -e "  ${YELLOW}数据目录已保留:${NC}"
    echo -e "    $INSTALL_DIR"
    echo -e "  ${YELLOW}如需完全清理，请运行:${NC}"
    echo -e "    sudo rm -rf $INSTALL_DIR"
    echo ""
fi

echo -e "  ${CYAN}感谢使用 Flamepanel！${NC}"
echo ""

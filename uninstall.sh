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
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"

usage() {
    cat << EOF
Flamepanel 卸载脚本

用法: $0 [选项]

选项:
  -f, --force     跳过确认提示
  -p, --purge     同时删除数据目录（数据库将丢失）
  -h, --help      显示帮助信息

示例:
  $0               # 交互式卸载（保留数据）
  $0 -f            # 静默卸载（保留数据）
  $0 -p            # 交互式卸载并删除数据
  $0 -f -p         # 完全卸载（删除所有数据）
EOF
    exit 0
}

FORCE=false
PURGE=false

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
echo -e "    ${YELLOW}•${NC} 删除二进制文件: $BINARY_PATH"
echo -e "    ${YELLOW}•${NC} 删除 systemd 服务文件"
if [[ "$PURGE" == true ]]; then
    echo -e "    ${RED}•${NC} 删除数据目录: $INSTALL_DIR"
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

# ─── 删除二进制 ────────────────────────────────────────────────────────────────
echo -e "${CYAN}[2/4] 删除二进制文件...${NC}"
if [[ -f "$BINARY_PATH" ]]; then
    rm -f "$BINARY_PATH"
    echo -e "${GREEN}  -> 已删除: $BINARY_PATH${NC}"
else
    echo -e "${YELLOW}  -> 未找到二进制文件，跳过${NC}"
fi

# ─── 删除 systemd 服务文件 ─────────────────────────────────────────────────────
echo -e "${CYAN}[3/4] 删除 systemd 服务文件...${NC}"
if [[ -f "$SERVICE_FILE" ]]; then
    rm -f "$SERVICE_FILE"
    systemctl daemon-reload
    echo -e "${GREEN}  -> 已删除: $SERVICE_FILE${NC}"
else
    echo -e "${YELLOW}  -> 未找到服务文件，跳过${NC}"
fi

# ─── 删除数据目录 ──────────────────────────────────────────────────────────────
echo -e "${CYAN}[4/4] 处理数据目录...${NC}"
if [[ -d "$INSTALL_DIR" ]]; then
    if [[ "$PURGE" == true ]]; then
        rm -rf "$INSTALL_DIR"
        echo -e "${RED}  -> 已删除: $INSTALL_DIR${NC}"
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

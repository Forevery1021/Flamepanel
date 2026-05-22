#!/bin/bash
set -e

echo "🚀 Ops Panel 安装脚本 v0.1"

# 检测系统
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$ID
fi

echo "检测到系统: $OS"

# 安装依赖
if command -v apt-get &> /dev/null; then
    sudo apt-get update
    sudo apt-get install -y curl wget unzip nginx
elif command -v yum &> /dev/null; then
    sudo yum install -y curl wget unzip nginx
fi

# 创建目录
mkdir -p /opt/ops-panel/data
cd /opt/ops-panel

# 下载最新版本（后续改为 GitHub Releases）
echo "正在下载 Ops Panel..."
# curl -L -o ops-panel.tar.gz https://github.com/yourname/ops-panel/releases/latest/download/ops-panel-linux-x86_64.tar.gz
# tar -xzf ops-panel.tar.gz

# 设置权限
chmod +x /usr/local/bin/ops-panel

# 创建 systemd 服务
cat > /etc/systemd/system/ops-panel.service << EOF
[Unit]
Description=Ops Panel Rust Admin Panel
After=network.target

[Service]
Type=simple
User=root
ExecStart=/usr/local/bin/ops-panel
WorkingDirectory=/opt/ops-panel
Restart=always
Environment=OP_JWT_SECRET=$(openssl rand -hex 32)

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable --now ops-panel

echo "✅ Ops Panel 安装完成！"
echo "访问地址: http://$(curl -s ifconfig.me):8080"
echo "默认账号: admin / admin123"
export default {
  inspectionFinding: {
    unknown: {
      title: '{code}',
      advice: '该检查项暂无内置说明，请参考取证字段判断。',
    },
    kernel_eol: {
      title: '内核系列生命周期',
      advice:
        '当前内核主线系列已超过上游维护期或不在 LTS 表中。若为发行版厂商内核（RHEL / Ubuntu 会回移补丁），以厂商公告为准；否则请升级到发行版提供的受支持内核。',
    },
    os_eol: {
      title: '发行版生命周期',
      advice: '发行版版本已到达或临近 EOL，将不再收到安全更新。请规划升级到受支持的版本。',
    },
    reboot_required: {
      title: '待重启生效',
      advice: '已安装新内核或核心库但尚未重启，安全修复未真正生效。请在维护窗口执行重启。',
    },
    mac_disabled: {
      title: '强制访问控制（AppArmor / SELinux）',
      advice:
        '未检测到已启用的 MAC 模块。建议启用 AppArmor（Debian / Ubuntu）或 SELinux enforcing（RHEL 系）以限制被攻破服务的破坏范围。',
    },
    cpu_vulnerability: {
      title: 'CPU 侧信道漏洞缓解',
      advice:
        '该漏洞在当前内核 / 微码下未缓解。请更新内核与 CPU 微码（intel-microcode / amd64-microcode）并重启；云主机可联系服务商确认宿主机已修补。',
    },
    sysctl: {
      title: '内核参数加固',
      advice:
        '该 sysctl 值不在推荐范围。写入 /etc/sysctl.d/99-hardening.conf 并执行 sysctl --system 使其持久生效。',
    },
    sshd_config: {
      title: 'SSH 服务配置',
      advice: '未找到 sshd_config，无法评估 SSH 加固项（未安装 OpenSSH 服务端时可忽略）。',
    },
    sshd_root_login: {
      title: 'SSH 允许 root 直接登录',
      advice:
        '设置 PermitRootLogin no（或 prohibit-password），改用普通用户 + sudo 登录，然后重载 sshd。',
    },
    sshd_password_auth: {
      title: 'SSH 允许密码认证',
      advice: '配置公钥登录后设置 PasswordAuthentication no，可杜绝口令暴力破解。',
    },
    sshd_empty_passwords: {
      title: 'SSH 允许空口令登录',
      advice: '设置 PermitEmptyPasswords no 并重载 sshd。',
    },
    sshd_max_auth_tries: {
      title: 'SSH 认证尝试次数过多',
      advice: '将 MaxAuthTries 调整为 3–6，并配合 fail2ban 限制暴力破解。',
    },
    sshd_default_port: {
      title: 'SSH 使用默认端口 22',
      advice:
        '默认端口承受最多的自动化扫描。可改为非标准端口并在防火墙放行，或在面板「防火墙」中限制来源 IP。',
    },
    uid0_accounts: {
      title: '存在额外的 UID 0 账户',
      advice: '除 root 外不应有 UID 0 账户。核实来源，确认为后门时立即删除并排查入侵痕迹。',
    },
    empty_password_accounts: {
      title: '存在空口令账户',
      advice: '为该账户设置强口令（passwd）或直接锁定（passwd -l）。',
    },
    shadow_permissions: {
      title: '/etc/shadow 权限过宽',
      advice: '执行 chmod 0640 /etc/shadow（RHEL 系为 0000），确保属主为 root。',
    },
    passwd_permissions: {
      title: '/etc/passwd 权限过宽',
      advice: '执行 chmod 0644 /etc/passwd 并确保属主为 root:root。',
    },
    docker_socket_permissions: {
      title: 'Docker socket 对其他用户可写',
      advice:
        '可写 /var/run/docker.sock 等同于 root 权限。执行 chmod 0660 并只把可信用户加入 docker 组。',
    },
    firewall_inactive: {
      title: '防火墙未启用',
      advice:
        '启用 ufw / firewalld / nftables，仅放行面板、SSH 与业务端口。可在面板「防火墙」页开启。',
    },
    exposed_service: {
      title: '高危服务暴露在公网',
      advice:
        '数据库 / 缓存 / 远程管理服务绑定在 0.0.0.0。改为监听 127.0.0.1 或内网地址，或用防火墙限制来源。',
    },
    public_listeners: {
      title: '公网监听端口总览',
      advice: '信息项：请核对列出的端口是否都是预期对外提供的服务。',
    },
    pending_security_updates: {
      title: '存在待安装的安全更新',
      advice: '尽快执行 apt upgrade 或 dnf upgrade --security，涉及内核 / glibc 时需要重启。',
    },
    unattended_upgrades: {
      title: '未启用自动安全更新',
      advice:
        '安装并启用 unattended-upgrades（Debian / Ubuntu）或 dnf-automatic（RHEL 系），自动安装安全补丁。',
    },
    time_sync: {
      title: '系统时间未同步',
      advice:
        '启用 systemd-timesyncd 或 chrony（timedatectl set-ntp true）。时间漂移会导致证书校验与日志审计出错。',
    },
    root_disk_usage: {
      title: '根分区使用率',
      advice:
        '清理日志 / 缓存 / 旧镜像（docker system prune），或扩容磁盘。使用率 ≥ 95% 时数据库与日志写入可能失败。',
    },
    swap_missing: {
      title: '未配置 swap',
      advice:
        '小内存主机建议配置 1–2 GB swap（fallocate + mkswap + swapon），避免内存耗尽触发 OOM。',
    },
    panel_session_timeout: {
      title: '面板会话超时过长',
      advice:
        '在「面板设置 → 安全」中把会话超时调到 ≤ 12 小时（720 分钟），缩短令牌泄露后的风险窗口。',
    },
    panel_metrics_token: {
      title: '/metrics 未设置访问令牌',
      advice: '设置环境变量 OP_METRICS_TOKEN 后重启面板，避免 Prometheus 指标端点被匿名读取。',
    },
    panel_process_user: {
      title: '面板运行用户',
      advice: '信息项：面板需要 root 才能管理主机（防火墙 / 服务 / 文件）。',
    },
  },
}

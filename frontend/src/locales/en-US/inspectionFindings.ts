export default {
  inspectionFinding: {
    unknown: {
      title: '{code}',
      advice: 'No built-in guidance for this check yet; use the evidence field to judge.',
    },
    kernel_eol: {
      title: 'Kernel series lifecycle',
      advice:
        'The running kernel series is past upstream maintenance or not in the LTS table. For vendor kernels (RHEL / Ubuntu backport fixes) follow the vendor advisory; otherwise upgrade to a supported kernel from your distribution.',
    },
    os_eol: {
      title: 'Distribution lifecycle',
      advice:
        'This distribution release has reached or is close to end of life and will stop receiving security updates. Plan an upgrade to a supported release.',
    },
    reboot_required: {
      title: 'Reboot pending',
      advice:
        'A new kernel or core library was installed but the host has not rebooted, so the fixes are not active yet. Reboot during the next maintenance window.',
    },
    mac_disabled: {
      title: 'Mandatory access control (AppArmor / SELinux)',
      advice:
        'No active MAC module detected. Enable AppArmor (Debian / Ubuntu) or SELinux enforcing (RHEL family) to contain compromised services.',
    },
    cpu_vulnerability: {
      title: 'CPU side-channel mitigation',
      advice:
        'This vulnerability is not mitigated with the current kernel / microcode. Update the kernel and CPU microcode (intel-microcode / amd64-microcode) and reboot; on cloud hosts confirm with the provider that the hypervisor is patched.',
    },
    sysctl: {
      title: 'Kernel parameter hardening',
      advice:
        'This sysctl value is outside the recommended range. Put it in /etc/sysctl.d/99-hardening.conf and run sysctl --system to persist it.',
    },
    sshd_config: {
      title: 'SSH server configuration',
      advice:
        'sshd_config was not found, so SSH hardening could not be evaluated (safe to ignore if the OpenSSH server is not installed).',
    },
    sshd_root_login: {
      title: 'SSH allows direct root login',
      advice:
        'Set PermitRootLogin no (or prohibit-password), log in as a regular user with sudo, then reload sshd.',
    },
    sshd_password_auth: {
      title: 'SSH allows password authentication',
      advice:
        'After setting up key-based login, set PasswordAuthentication no to stop password brute force.',
    },
    sshd_empty_passwords: {
      title: 'SSH allows empty passwords',
      advice: 'Set PermitEmptyPasswords no and reload sshd.',
    },
    sshd_max_auth_tries: {
      title: 'SSH authentication attempts too high',
      advice: 'Set MaxAuthTries to 3–6 and pair it with fail2ban to throttle brute force.',
    },
    sshd_default_port: {
      title: 'SSH on default port 22',
      advice:
        'The default port receives the most automated scanning. Move to a non-standard port and allow it in the firewall, or restrict source IPs in the panel firewall.',
    },
    uid0_accounts: {
      title: 'Extra UID 0 accounts',
      advice:
        'Only root should have UID 0. Verify where this account came from; if it is a backdoor, remove it immediately and investigate the intrusion.',
    },
    empty_password_accounts: {
      title: 'Accounts with empty passwords',
      advice: 'Set a strong password (passwd) or lock the account (passwd -l).',
    },
    shadow_permissions: {
      title: '/etc/shadow is too permissive',
      advice:
        'Run chmod 0640 /etc/shadow (0000 on the RHEL family) and make sure it is owned by root.',
    },
    passwd_permissions: {
      title: '/etc/passwd is too permissive',
      advice: 'Run chmod 0644 /etc/passwd and make sure it is owned by root:root.',
    },
    docker_socket_permissions: {
      title: 'Docker socket writable by others',
      advice:
        'Write access to /var/run/docker.sock equals root. Run chmod 0660 and add only trusted users to the docker group.',
    },
    firewall_inactive: {
      title: 'Firewall disabled',
      advice:
        'Enable ufw / firewalld / nftables and allow only the panel, SSH and service ports. You can turn it on from the Firewall page.',
    },
    exposed_service: {
      title: 'High-risk service exposed to the internet',
      advice:
        'A database / cache / remote-admin service is bound to 0.0.0.0. Bind it to 127.0.0.1 or a private address, or restrict sources with the firewall.',
    },
    public_listeners: {
      title: 'Public listening ports',
      advice: 'Informational: confirm every listed port is a service you intend to expose.',
    },
    pending_security_updates: {
      title: 'Security updates pending',
      advice:
        'Run apt upgrade or dnf upgrade --security soon; kernel / glibc updates require a reboot.',
    },
    unattended_upgrades: {
      title: 'Automatic security updates disabled',
      advice:
        'Install and enable unattended-upgrades (Debian / Ubuntu) or dnf-automatic (RHEL family) to apply security patches automatically.',
    },
    time_sync: {
      title: 'System clock not synchronized',
      advice:
        'Enable systemd-timesyncd or chrony (timedatectl set-ntp true). Clock drift breaks certificate validation and log auditing.',
    },
    root_disk_usage: {
      title: 'Root filesystem usage',
      advice:
        'Clean logs / caches / old images (docker system prune) or grow the disk. At ≥ 95% databases and log writers may start failing.',
    },
    swap_missing: {
      title: 'No swap configured',
      advice:
        'Hosts with little RAM should have 1–2 GB of swap (fallocate + mkswap + swapon) to avoid the OOM killer under memory pressure.',
    },
    panel_session_timeout: {
      title: 'Panel session timeout too long',
      advice:
        'In Settings → Security set the session timeout to ≤ 12 hours (720 minutes) to shorten the window after a token leak.',
    },
    panel_metrics_token: {
      title: '/metrics has no access token',
      advice:
        'Set the OP_METRICS_TOKEN environment variable and restart the panel so the Prometheus endpoint cannot be read anonymously.',
    },
    panel_process_user: {
      title: 'Panel process user',
      advice:
        'Informational: the panel needs root to manage the host (firewall / services / files).',
    },
  },
}

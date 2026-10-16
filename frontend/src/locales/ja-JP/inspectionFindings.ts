export default {
  inspectionFinding: {
    unknown: {
      title: '{code}',
      advice: 'このチェック項目には内蔵の説明がまだありません。根拠欄を参考に判断してください。',
    },
    kernel_eol: {
      title: 'カーネル系列のライフサイクル',
      advice:
        '稼働中のカーネル系列は上流の保守期間を過ぎているか LTS 表にありません。ベンダーカーネル（RHEL / Ubuntu はパッチをバックポート）の場合はベンダーの告知に従い、それ以外はディストリビューションが提供するサポート対象カーネルへ更新してください。',
    },
    os_eol: {
      title: 'ディストリビューションのライフサイクル',
      advice:
        'このリリースは EOL に達したか間近で、セキュリティ更新が届かなくなります。サポート対象バージョンへの更新を計画してください。',
    },
    reboot_required: {
      title: '再起動待ち',
      advice:
        '新しいカーネルやコアライブラリがインストール済みですが再起動されておらず、修正がまだ有効になっていません。メンテナンス時間帯に再起動してください。',
    },
    mac_disabled: {
      title: '強制アクセス制御（AppArmor / SELinux）',
      advice:
        '有効な MAC モジュールが検出されませんでした。AppArmor（Debian / Ubuntu）または SELinux enforcing（RHEL 系）を有効にし、侵害されたサービスの影響範囲を限定してください。',
    },
    cpu_vulnerability: {
      title: 'CPU サイドチャネル脆弱性の緩和',
      advice:
        '現在のカーネル / マイクロコードではこの脆弱性が緩和されていません。カーネルと CPU マイクロコード（intel-microcode / amd64-microcode）を更新して再起動してください。クラウドの場合はホスト側の修正状況を事業者に確認してください。',
    },
    sysctl: {
      title: 'カーネルパラメータ強化',
      advice:
        'この sysctl 値は推奨範囲外です。/etc/sysctl.d/99-hardening.conf に記述し、sysctl --system で永続化してください。',
    },
    sshd_config: {
      title: 'SSH サーバー設定',
      advice:
        'sshd_config が見つからず SSH の強化項目を評価できませんでした（OpenSSH サーバー未導入なら無視して構いません）。',
    },
    sshd_root_login: {
      title: 'SSH で root の直接ログインを許可',
      advice:
        'PermitRootLogin no（または prohibit-password）を設定し、一般ユーザー + sudo でログインしてから sshd を再読み込みしてください。',
    },
    sshd_password_auth: {
      title: 'SSH でパスワード認証を許可',
      advice:
        '公開鍵ログインを設定した上で PasswordAuthentication no にすると、パスワード総当たりを防げます。',
    },
    sshd_empty_passwords: {
      title: 'SSH で空パスワードを許可',
      advice: 'PermitEmptyPasswords no を設定して sshd を再読み込みしてください。',
    },
    sshd_max_auth_tries: {
      title: 'SSH 認証試行回数が多すぎます',
      advice: 'MaxAuthTries を 3–6 に調整し、fail2ban と併用して総当たりを抑制してください。',
    },
    sshd_default_port: {
      title: 'SSH が既定ポート 22 を使用',
      advice:
        '既定ポートは自動スキャンを最も多く受けます。非標準ポートへ変更してファイアウォールで許可するか、パネルのファイアウォールで接続元 IP を制限してください。',
    },
    uid0_accounts: {
      title: 'root 以外の UID 0 アカウント',
      advice:
        'UID 0 は root のみであるべきです。由来を確認し、バックドアなら直ちに削除して侵入痕跡を調査してください。',
    },
    empty_password_accounts: {
      title: '空パスワードのアカウント',
      advice: '強いパスワードを設定する（passwd）か、アカウントをロック（passwd -l）してください。',
    },
    shadow_permissions: {
      title: '/etc/shadow のパーミッションが緩い',
      advice:
        'chmod 0640 /etc/shadow（RHEL 系は 0000）を実行し、所有者が root であることを確認してください。',
    },
    passwd_permissions: {
      title: '/etc/passwd のパーミッションが緩い',
      advice: 'chmod 0644 /etc/passwd を実行し、所有者が root:root であることを確認してください。',
    },
    docker_socket_permissions: {
      title: 'Docker ソケットが他ユーザーから書き込み可能',
      advice:
        '/var/run/docker.sock への書き込みは root 権限と同等です。chmod 0660 を実行し、信頼できるユーザーのみ docker グループに追加してください。',
    },
    firewall_inactive: {
      title: 'ファイアウォールが無効',
      advice:
        'ufw / firewalld / nftables を有効にし、パネル・SSH・業務ポートのみ許可してください。パネルの「ファイアウォール」ページから有効化できます。',
    },
    exposed_service: {
      title: '高リスクサービスがインターネットに露出',
      advice:
        'データベース / キャッシュ / リモート管理サービスが 0.0.0.0 にバインドされています。127.0.0.1 や内部アドレスで待ち受けるか、ファイアウォールで接続元を制限してください。',
    },
    public_listeners: {
      title: '公開待ち受けポート一覧',
      advice: '情報項目：列挙されたポートがすべて意図して公開しているサービスか確認してください。',
    },
    pending_security_updates: {
      title: '未適用のセキュリティ更新',
      advice:
        '早めに apt upgrade または dnf upgrade --security を実行してください。カーネル / glibc の更新には再起動が必要です。',
    },
    unattended_upgrades: {
      title: '自動セキュリティ更新が無効',
      advice:
        'unattended-upgrades（Debian / Ubuntu）または dnf-automatic（RHEL 系）を導入・有効化し、セキュリティパッチを自動適用してください。',
    },
    time_sync: {
      title: 'システム時刻が同期されていません',
      advice:
        'systemd-timesyncd または chrony を有効にしてください（timedatectl set-ntp true）。時刻のずれは証明書検証やログ監査を破綻させます。',
    },
    root_disk_usage: {
      title: 'ルートパーティション使用率',
      advice:
        'ログ / キャッシュ / 古いイメージを削除する（docker system prune）か、ディスクを拡張してください。使用率 95% 以上ではデータベースやログの書き込みが失敗することがあります。',
    },
    swap_missing: {
      title: 'swap が未設定',
      advice:
        'メモリの少ないホストでは 1–2 GB の swap（fallocate + mkswap + swapon）を設定し、メモリ枯渇時の OOM を避けてください。',
    },
    panel_session_timeout: {
      title: 'パネルのセッションタイムアウトが長すぎます',
      advice:
        '「パネル設定 → セキュリティ」でセッションタイムアウトを 12 時間（720 分）以下にし、トークン漏洩時のリスク期間を短くしてください。',
    },
    panel_metrics_token: {
      title: '/metrics にアクセストークンが未設定',
      advice:
        '環境変数 OP_METRICS_TOKEN を設定してパネルを再起動し、Prometheus エンドポイントの匿名読み取りを防いでください。',
    },
    panel_process_user: {
      title: 'パネルの実行ユーザー',
      advice:
        '情報項目：パネルはホスト管理（ファイアウォール / サービス / ファイル）のため root が必要です。',
    },
  },
}

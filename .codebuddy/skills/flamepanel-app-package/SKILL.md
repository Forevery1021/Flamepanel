---
name: flamepanel-app-package
description: 在 FlamePanel 开发、迁移或适配应用商店应用包时使用。当任务涉及 Flame/x-flame app.json 与 compose、1Panel data.yml 包迁移、宝塔包、适配器 infrastructure/app_store/adapter、SecurityScanner、变量映射、Compose 钩子时使用。给出三格式速查、x-flame 规范、安全红线与测试要求。
---

# FlamePanel · 应用商店包

> 代码：`infrastructure/app_store/`（`adapter/{flame,onepanel,baota}.rs`、`select_adapter`、`security_scanner`、`variable_mapper`）；文档：Doc/20、Doc/21（SDK）、Doc/22（1Panel 兼容）。

## 1. 三格式识别

| 格式 | 识别特征 | 结构 |
|---|---|---|
| Flame / x-flame | `app.json` + compose 内 `x-flame:` | `app.json`（元数据 + formFields）+ `<version>/docker-compose.yml` |
| 1Panel | 顶层 `data.yml` | `data.yml`（key/name/tags/type）+ `<version>/{data.yml, docker-compose.yml, scripts/}` |
| 宝塔 | `app.json` 含 `appname/apptitle/apptype` | `app.json`（field[]）+ `latest/` 或 `<version>/docker-compose.yml` |

`select_adapter` 自动路由；**做新包不需要改适配器**，除非引入新元数据字段（此时改 adapter + 补测试）。

## 2. x-flame 模板

```yaml
# apps/<key>/<version>/docker-compose.yml
x-flame:
  app:
    category: website
    arch: [amd64, arm64]
    hooks:                                  # Compose 生命周期钩子（可选）
      post_install: ["sh", "-c", "echo ok"]
  services:
    app:
      env:
        PANEL_APP_PORT_HTTP: { zh-CN: 网站端口, en-US: HTTP port, ja-JP: HTTPポート }
services:
  app:
    image: docker.io/library/wordpress:6.7.1     # 固定版本；禁止 :latest
    ports: ["${PANEL_APP_PORT_HTTP}:80"]
    volumes: ["${APP_PATH}/data:/var/www/html"]  # 禁止 /opt/... 绝对路径
    environment: { PUID: "${PUID}", PGID: "${PGID}", TZ: "${TZ}" }
    networks: [flamepanel-network]
networks:
  flamepanel-network: { external: true }
```

系统注入变量：`PUID / PGID / TZ / APP_ID / APP_PATH`；端口约定 `PANEL_APP_PORT_{HTTP,HTTPS,API,ADMIN,PROXY,DB}` 前端自动预填。

## 3. SecurityScanner 红线

| 情形 | 结果 |
|---|---|
| `privileged: true` | 阻断（UI 二次确认才可） |
| 挂载 `/`、`/etc`、`/var/run/docker.sock`、`/proc` | 阻断 |
| `:latest` 或无 tag | 阻断 |
| 非白名单镜像源（如 `docker.1panel.live`） | 警告；改 `docker.io` 或白名单国内源 |
| `network_mode: host`、`cap_add: SYS_ADMIN` | 警告 |

## 4. 1Panel 包迁移 checklist

- [ ] `/opt/1panel/apps/...` → `${APP_PATH}/...`
- [ ] 镜像源替换；tag 固定
- [ ] `1panel-network` 自动替换；其他自定义网络改 `flamepanel-network`
- [ ] `scripts/*.sh` → 迁到 `x-flame.app.hooks`（容器模式）或登记 native_scripts（原生模式）
- [ ] `data.yml` formFields 的 `envKey` 与 compose 变量一致

## 5. 测试（对齐 adapter 现有密度）

```bash
cd flame-kernel && cargo test -p flame-kernel app_store
```

必须覆盖：格式识别、字段映射含缺省、网络名替换、**恶意输入**（`../` 路径穿越、超长字段、非 UTF-8 包名、YAML 炸弹级嵌套）。测试 fixture 放 `flame-kernel/tests/fixtures/apps/<key>/`（小于 5 KB）。

本地导入验证：
```bash
curl -X POST http://localhost:8080/api/app-store/packages/import -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' -d '{"path":"/abs/path/apps/<key>"}'
```

## 6. 自查

- [ ] 无 `:latest` / 特权 / 敏感挂载；镜像源白名单
- [ ] `x-flame` 元数据（category / arch / env 三语言说明）齐
- [ ] 适配器改动有恶意输入用例
- [ ] Doc/21 若引入新字段同步；`Doc/20` 若改流程同步


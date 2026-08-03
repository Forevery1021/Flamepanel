# FlamePanel 应用商店 + Web 引擎 实现计划

> 依据 `next.md` 路线图，本次实现「全部实现」选项：App Store（1Panel/宝塔/内置格式 × 容器/原生/WASM 三模式）+ WebEngine 统一层（预设/资源感知/引擎切换）+ 前端完整 UI。

## 阶段 1：领域层与存储

1. `domain/entity.rs` 扩展：
   - `AppFormat { OnePanel, Baota, Flame }`、`InstallMode { Container, Native, Wasm }`
   - `AppMetadata`、`AppVersionInfo`、`FormField`（含 `FieldType` 枚举与校验）、`AppScripts`
   - `InstalledApp`（id, package_key, version, mode, name, status, access_url, install_path, params_json, created_at）
2. `domain/repository.rs` 新增：
   - `AppPackageRepository`：`list_packages/import_package/find_by_key/delete_package/list_versions`
   - `InstalledAppRepository`：`list/create/get/update/delete`
   - `PluginRepository`（WASM 持久化）：`save_plugin/list_plugins/get_plugin/delete_plugin`
3. `infrastructure/sqlite.rs` + `db.rs` 迁移：`app_packages`、`installed_apps`、`plugins` 三张表（SQLite 全量实现，InMemory 镜像）

## 阶段 2：适配器层（TDD）

4. `infrastructure/app_store/mod.rs`：`select_adapter()` 按目录结构自动检测
5. `flame.rs`（内置格式）：`app.json`（key/name/category/desc/version/mode/icon）+ `docker-compose.yml` 或 `install.sh`（原生）或 `plugin.wasm`（WASM）；复用 `builtin_apps()` 迁移为内置目录
6. `onepanel.rs`（1Panel）：根 `data.yml` 元数据 + 版本目录 `data.yml` formFields + `docker-compose.yml`，变量/网络替换
7. `baota.rs`（宝塔）：`app.json` + `latest/` 版本目录 + compose
8. `variable_mapper.rs`：`${VAR}`/`$VAR` 正则替换，未识别保留+警告
9. `security_scanner.rs`：privileged/敏感挂载/network_mode: host/镜像仓库白名单 → 阻断或警告

## 阶段 3：服务编排（三模式）

10. `application/app_store_service.rs`：
    - **容器**：select_adapter → 表单校验 → 变量替换 → 安全扫描 → ensure 网络 → 写 compose → DockerService::compose_deploy → 记录
    - **原生**：按 category 分派 `NativeInstaller`（MySQL/MariaDB/Redis → 复用 NativeDbManager；Nginx/Apache/Caddy/OLS/OpenResty → WebServerService；PHP-FPM/Node → PackageManager）
    - **WASM**：上传 wasm → PluginSandbox 注册 + PluginRepository 持久化 + 内置工具目录（内置 WASM 工具用可执行字节码种子）
    - 升级/卸载/日志 编排 + operation_log 审计

## 阶段 4：WASM 持久化

11. PluginSandbox/PluginRegistry 改造：加载时 `load_plugin`（DB）→ 沙箱；启动时从 DB 恢复
12. 内置 WASM 工具：生成 2-3 个最小 wasm（nop/echo/fib）作为商店种子

## 阶段 5：API + 权限

13. `api/handler/app_store/mod.rs`：
    - `GET /api/app-store/packages`（分页+分类过滤）
    - `POST /api/app-store/packages/import`（body: format+path）
    - `GET /api/app-store/packages/:key`、`/versions/:ver`
    - `POST /api/app-store/install`（mode 分支）
    - `GET /api/app-store/installed`、`POST /:id/upgrade|uninstall|restart`、`GET /:id/logs`
    - `GET /api/app-store/wasm/builtins`
14. `route_permission` 新增 `app_store:*` 映射；`default_permissions()` 种子

## 阶段 6：WebEngine 统一层

15. `webserver/engine_trait.rs`：`WebEngine` trait（kind/generate_site/generate_global/validate/write/remove/reload/apply_preset/capabilities），为 5 引擎实现（包装现有 generator + manager）
16. `webserver/preset.rs`：`PerformancePreset { Conservative, Balanced, Aggressive }`、`SystemResources::detect()`（sysinfo）
17. 端点：`POST /api/web-servers/:id/preset`（资源感知应用全局预设）、`POST /api/websites/:id/switch-engine`（备份→生成→校验→reload→回滚）
18. `site_engine_history` 表 + 回滚逻辑

## 阶段 7：前端

19. `api/appStore.ts` + `AppStoreView.vue`：三 Tab（商店/已安装/WASM 工具），商店卡片 + 分类筛选 + 安装向导（选版本→动态表单→安全扫描→确认），已安装列表（状态/日志/升级/卸载）
20. `WebsitesView`：性能预设下拉 + 切换引擎按钮；`WebServersView`：预设应用 + 配置编辑
21. Sidebar + router + 三语言 i18n

## 阶段 8：文档与验证

22. 全量 `cargo test` + `npm run lint` + `npm run build`；README/指南 changelog v0.2.0

## 完成状态（2026-08-01）

- [x] 阶段 1：领域层与存储（AppFormat/InstallMode/AppMetadata/AppVersionInfo/FormField/InstalledApp + AppPackageRepository/InstalledAppRepository/PluginRepository + `app_packages`/`installed_apps`/`plugins` 三表迁移，SQLite + InMemory 双实现）
- [x] 阶段 2：适配器层（select_adapter 自动检测、FlameAdapter/OnePanelAdapter/BaotaAdapter、VariableMapper、SecurityScanner，共 29 个单元测试）
- [x] 阶段 3：服务编排（AppStoreService 容器/原生/WASM 三模式 + 回滚 + upgrade/uninstall/logs；WASM 内置 wasm-hello 用真实字节码种子，run() 返回 42）
- [x] 阶段 4：WASM 持久化（Plugin.wasm_base64 + plugins 表 + restore_wasm_plugins 启动恢复）
- [x] 阶段 5：API + 权限（/api/app-store/* 12 个端点 + app_store:* 权限 + 6 个集成测试）
- [x] 阶段 6：WebEngine 统一（PerformancePreset 资源感知推荐 + switch-engine/preset 端点 + 8 个单元测试 + 3 个集成测试）
- [x] 阶段 7：前端（AppStoreView 三 Tab + 动态表单安装向导 + 安全风险确认 + 导入/日志；WebServersView 预设与引擎切换；WebsitesView 引擎切换；三语言 i18n）
- [x] 阶段 8：文档（开发指南 v0.2.0 更新日志）+ 全量验证（cargo test 139 通过、clippy 无新增警告、vue-tsc/eslint/vite build 通过）

# 阶段 9：Docker 增强 + Web 服务器原生控制（v0.5.0）

## 9.1 Docker 增强（参考 1Panel 容器部分）

1. `DockerRepository` trait 扩展 18 个方法：容器 inspect/rename/pause/unpause/kill/prune；网络 list/create/remove/connect/disconnect/prune；卷 list/create/remove/prune；镜像 pull/tag/prune；compose ls
2. `BollardDockerRepository` 全量实现（create_image 流式拉取、Ipam 子网、split_image_tag 解析）；InMemory 降级实现
3. `DockerService` 透传 + `api/handler/docker` 新增 16 个端点（容器 6、网络 6、卷 4、镜像 3、compose 1）+ `route_permission` 映射（docker:create/update 新增权限）

## 9.2 Web 服务器原生控制

4. `webserver/native.rs`：`NativeWebServerInfo`（installed/package/version/service/running/enabled/listening_ports）+ `WebServerNativeManager`（detect_all/install/uninstall/set_autostart/版本正则解析/ss 端口扫描）
5. `WebServerService`：native_detect/native_install（自动注册实例）/native_uninstall_by_engine/set_autostart_by_engine/native_status
6. 端点：GET /api/web-servers/native/detect、POST /native/install|uninstall|autostart、POST /:id/autostart、GET /:id/native-status

## 9.3 前端 + 验证

7. DockerView：容器搜索/详情/重命名/暂停/强杀/清理 + 网络/卷 Tab + 镜像拉取/清理 + Compose 项目列表；WebServersView 原生控制 Tab（检测/安装/卸载/自启开关）；三语言 i18n
8. 全量验证：cargo test 162 通过（98 集成 + 64 单元）、clippy 零警告、vue-tsc/eslint/vite build 通过

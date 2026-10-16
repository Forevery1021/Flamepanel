export interface paths {
    "/api/app-store/installed": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 已安装应用列表 */
        get: operations["GET /api/app-store/installed"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/installed/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 已安装应用详情 */
        get: operations["GET /api/app-store/installed/{id}"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/installed/{id}/launch": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 记录应用启动次数（常用应用排序）
         *     记录一次启动（常用应用排序）
         */
        post: operations["POST /api/app-store/installed/{id}/launch"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/installed/{id}/logs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 容器日志尾部（`{logs}`；原生模式为空） */
        get: operations["GET /api/app-store/installed/{id}/logs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/installed/{id}/native-restart": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 重启原生已安装应用服务
         *     重启已安装应用服务（`{success, id, status}`）
         */
        post: operations["POST /api/app-store/installed/{id}/native-restart"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/installed/{id}/native-start": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 启动原生已安装应用服务
         *     启动已安装应用服务（`{success, id, status}`）
         */
        post: operations["POST /api/app-store/installed/{id}/native-start"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/installed/{id}/native-status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * 原生已安装应用状态（running/stopped）
         *     运行状态（`{id, status}`，running / stopped）
         */
        get: operations["GET /api/app-store/installed/{id}/native-status"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/installed/{id}/native-stop": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 停止原生已安装应用服务
         *     停止已安装应用服务（`{success, id, status}`）
         */
        post: operations["POST /api/app-store/installed/{id}/native-stop"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/installed/{id}/uninstall": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 卸载应用（`{success: true}`） */
        post: operations["POST /api/app-store/installed/{id}/uninstall"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/installed/{id}/upgrade": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 升级应用（拒绝降级；同版本幂等）。容器模式对 compose 做三方合并保留用户修改，
         *     冲突时默认 409，可用 `conflict_strategy=local|upstream` 指定处理方式。
         */
        post: operations["POST /api/app-store/installed/{id}/upgrade"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/installed/{id}/upgrade-preview": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * 升级预览：渲染目标版本 compose 并与本地做三方合并，返回冲突 / 保留的本地修改 /
         *     生效的上游修改与合并结果；不改动任何文件。
         */
        get: operations["GET /api/app-store/installed/{id}/upgrade-preview"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/packages": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 应用商店包列表 */
        get: operations["GET /api/app-store/packages"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/packages/batch-import": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 批量导入本地目录应用包（Phase A2：`create_many` 事务语义，全成或全败） */
        post: operations["POST /api/app-store/packages/batch-import"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/packages/import": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 导入服务器本地目录为应用包（Flame / 1Panel / 宝塔格式自动识别；key 须为安全路径段，不跟随符号链接） */
        post: operations["POST /api/app-store/packages/import"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/packages/{key}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 应用包元数据 */
        get: operations["GET /api/app-store/packages/{key}"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/packages/{key}/install": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 安装应用（容器 / 原生 / WASM；高风险或含宿主机脚本需显式确认） */
        post: operations["POST /api/app-store/packages/{key}/install"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/packages/{key}/versions/{version}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 指定版本详情（compose 模板、表单字段、钩子、原生安装参数） */
        get: operations["GET /api/app-store/packages/{key}/versions/{version}"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/security": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 商店安全配置（验签策略 + 镜像仓库白名单） */
        get: operations["GET /api/app-store/security"];
        /** 更新商店安全配置（验签策略 / 镜像仓库白名单） */
        put: operations["PUT /api/app-store/security"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/sources": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 远程源列表 */
        get: operations["GET /api/app-store/sources"];
        put?: never;
        /** 添加远程源 */
        post: operations["POST /api/app-store/sources"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/sources/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** 更新远程源（name / url / enabled / public_key） */
        put: operations["PUT /api/app-store/sources/{id}"];
        post?: never;
        /** 删除远程源 */
        delete: operations["DELETE /api/app-store/sources/{id}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/sources/{id}/packages": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 按源过滤应用包（复用现有包列表，返回包元数据） */
        get: operations["GET /api/app-store/sources/{id}/packages"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/sources/{id}/sync": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 手动同步远程源索引（同步任务经统一任务中心跟踪） */
        post: operations["POST /api/app-store/sources/{id}/sync"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/app-store/wasm-builtins": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 内置 WASM 应用 */
        get: operations["GET /api/app-store/wasm-builtins"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/assistant/chat": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** `POST /api/assistant/chat` */
        post: operations["POST /api/assistant/chat"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/assistant/config": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/assistant/config` */
        get: operations["GET /api/assistant/config"];
        /** `PUT /api/assistant/config` */
        put: operations["PUT /api/assistant/config"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/assistant/test": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** `POST /api/assistant/test` */
        post: operations["POST /api/assistant/test"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/2fa": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 两步验证状态 */
        get: operations["GET /api/auth/2fa"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/2fa/disable": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 停用两步验证 */
        post: operations["POST /api/auth/2fa/disable"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/2fa/enable": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 确认启用：校验待绑定密钥的验证码，返回一次性恢复码 */
        post: operations["POST /api/auth/2fa/enable"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/2fa/recovery-codes": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 重新生成恢复码（旧码全部作废） */
        post: operations["POST /api/auth/2fa/recovery-codes"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/2fa/setup": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 开始绑定：生成待验证密钥与 otpauth URI（已启用时不影响旧密钥，确认后才替换） */
        post: operations["POST /api/auth/2fa/setup"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/change-password": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 修改当前用户密码 */
        post: operations["POST /api/auth/change-password"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/login": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 登录 */
        post: operations["POST /api/auth/login"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/login/2fa": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 登录第二步：提交挑战令牌 + TOTP 验证码（或一次性恢复码）换取会话令牌。
         * @description 失败计入与密码登录相同的「用户名 + IP」锁定计数；挑战令牌签发后若用户改过密码则失效。
         */
        post: operations["POST /api/auth/login/2fa"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/logout": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 退出登录：吊销当前 refresh token（加入黑名单），使其无法再换取新 access token。
         *     M4：logout 端点此前被中间件引用但路由缺失（404）；现补全并实现吊销。
         */
        post: operations["POST /api/auth/logout"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/me": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 当前登录用户信息（前端刷新页面恢复身份） */
        get: operations["GET /api/auth/me"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/refresh": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 刷新令牌：必须使用 Refresh Token（Cookie 或 Bearer），换取新的 Access Token + 轮换 Refresh Token
         *     剩余寿命不足一半时重新签发 Refresh Token（滑动过期）；否则复用原 Refresh Token
         *     刷新令牌（Refresh Token 换新 Access Token + 轮换 Refresh Token）
         */
        post: operations["POST /api/auth/refresh"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/rotate-secret": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 轮换 JWT 签名密钥（admin only）：更新后旧 access token 立即失效（短宽限期由客户端刷新换取新 token） */
        post: operations["POST /api/auth/rotate-secret"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/backup-accounts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 备份账号列表 */
        get: operations["GET /api/backup-accounts"];
        put?: never;
        /** 创建备份账号 */
        post: operations["POST /api/backup-accounts"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/backup-accounts/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** 更新备份账号 */
        put: operations["PUT /api/backup-accounts/{id}"];
        post?: never;
        /** 删除备份账号 */
        delete: operations["DELETE /api/backup-accounts/{id}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/backup-accounts/{id}/objects": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 列出云端账号的备份对象 */
        get: operations["GET /api/backup-accounts/{id}/objects"];
        put?: never;
        post?: never;
        /** 删除云端账号的备份对象（key 不存在时幂等成功） */
        delete: operations["DELETE /api/backup-accounts/{id}/objects"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/backup-accounts/{id}/objects/upload": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 上传本地备份文件到云端账号 */
        post: operations["POST /api/backup-accounts/{id}/objects/upload"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/backup-accounts/{id}/test": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 连通性测试（Stage 2：按账号类型构造真实后端探测，结果落库） */
        post: operations["POST /api/backup-accounts/{id}/test"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/backup-jobs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 备份任务列表 */
        get: operations["GET /api/backup-jobs"];
        put?: never;
        /** 创建备份任务 */
        post: operations["POST /api/backup-jobs"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/backup-jobs/restore-cloud": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 从云端账号直接拉取并恢复（按 key 前缀分流：面板库 .db / 应用数据 app/<key>/*.tar.gz） */
        post: operations["POST /api/backup-jobs/restore-cloud"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/backup-jobs/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** 更新备份任务 */
        put: operations["PUT /api/backup-jobs/{id}"];
        post?: never;
        /** 删除备份任务 */
        delete: operations["DELETE /api/backup-jobs/{id}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/backup-jobs/{id}/enabled": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** 启停备份任务 */
        put: operations["PUT /api/backup-jobs/{id}/enabled"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/backup-jobs/{id}/run": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 立即执行一次 */
        post: operations["POST /api/backup-jobs/{id}/run"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/backups": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 备份列表 */
        get: operations["GET /api/backups"];
        put?: never;
        /** 创建备份 */
        post: operations["POST /api/backups"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/backups/{filename}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 下载备份文件（流式） */
        get: operations["GET /api/backups/{filename}"];
        put?: never;
        post?: never;
        /** 删除备份 */
        delete: operations["DELETE /api/backups/{filename}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/backups/{filename}/restore": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 从备份恢复数据库：先把当前库另存为 `pre-restore-*.db`，再原子替换库文件。
         * @description 以请求体 `filename` 为准（路径段仅作路由）；运行中的连接仍指向旧文件，建议恢复后重启服务。
         */
        post: operations["POST /api/backups/{filename}/restore"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/certificates": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 证书列表 */
        get: operations["GET /api/certificates"];
        put?: never;
        /** 上传第三方证书 */
        post: operations["POST /api/certificates"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/certificates/issue/acme": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** ACME 签发（Let's Encrypt 等；HTTP-01 / DNS-01） */
        post: operations["POST /api/certificates/issue/acme"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/certificates/issue/self-signed": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 自签证书（内网/无公网场景） */
        post: operations["POST /api/certificates/issue/self-signed"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/certificates/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 证书详情 */
        get: operations["GET /api/certificates/{id}"];
        /** 更新证书（auto_renew / website_id） */
        put: operations["PUT /api/certificates/{id}"];
        post?: never;
        /** 删除证书记录（不删除落盘文件——运维可手动留存） */
        delete: operations["DELETE /api/certificates/{id}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/certificates/{id}/bind": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 证书绑定/解绑网站（website_id = None 解绑） */
        post: operations["POST /api/certificates/{id}/bind"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/certificates/{id}/renew": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 手动触发续签（ACME 来源；自签/上传返回 400） */
        post: operations["POST /api/certificates/{id}/renew"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/certificates/{id}/renewals": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 续签历史（S-10）：签发 / 自动续签成功与失败 / 到期预警时间线，数据来自事件落库 */
        get: operations["GET /api/certificates/{id}/renewals"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/databases": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 数据库实例列表（分页） */
        get: operations["GET /api/databases"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/databases/batch-status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        /**
         * Phase A2 扩展：数据库实例批量状态原子更新（批量写事务，PATCH /api/databases/batch-status）。
         *     批量更新实例状态（单事务）
         */
        patch: operations["PATCH /api/databases/batch-status"];
        trace?: never;
    };
    "/api/databases/mongodb/install": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 经 MongoDB 官方仓库（HTTPS + 签名校验）安装 mongod 并登记实例（端口缺省 27017；
         *     管理员密码经 mongosh stdin 传递，不进 argv）
         */
        post: operations["POST /api/databases/mongodb/install"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/databases/mysql/install": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 经系统包管理器安装 mysql 并登记实例（端口缺省 3306；口令经 stdin / 环境变量传递，不进 argv） */
        post: operations["POST /api/databases/mysql/install"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/databases/postgres/install": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 经系统包管理器安装 postgres 并登记实例（端口缺省 5432；口令经 stdin / 环境变量传递，不进 argv） */
        post: operations["POST /api/databases/postgres/install"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/databases/redis/install": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 经系统包管理器安装 redis 并登记实例（端口缺省 6379；口令经 stdin / 环境变量传递，不进 argv） */
        post: operations["POST /api/databases/redis/install"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/databases/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 单个数据库实例 */
        get: operations["GET /api/databases/{id}"];
        put?: never;
        post?: never;
        /** 删除实例登记（不卸载软件；卸载见 uninstall） */
        delete: operations["DELETE /api/databases/{id}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/databases/{id}/databases": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 列出用户库（系统库已排除） */
        get: operations["GET /api/databases/{id}/databases"];
        put?: never;
        /** 创建库（MySQL / MariaDB 可指定字符集，缺省 utf8mb4；PostgreSQL 忽略字符集） */
        post: operations["POST /api/databases/{id}/databases"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/databases/{id}/databases/{db_name}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** 删除库（系统库受保护） */
        delete: operations["DELETE /api/databases/{id}/databases/{db_name}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/databases/{id}/restart": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 重启数据库服务 */
        post: operations["POST /api/databases/{id}/restart"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/databases/{id}/start": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 启动数据库服务 */
        post: operations["POST /api/databases/{id}/start"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/databases/{id}/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 服务运行状态（`running` / `stopped`） */
        get: operations["GET /api/databases/{id}/status"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/databases/{id}/stop": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 停止数据库服务 */
        post: operations["POST /api/databases/{id}/stop"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/databases/{id}/uninstall": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 卸载数据库软件并删除实例登记 */
        post: operations["POST /api/databases/{id}/uninstall"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/databases/{id}/users": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 创建数据库用户（口令不进 argv） */
        post: operations["POST /api/databases/{id}/users"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/databases/{id}/users/{username}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** 删除数据库用户（host 固定 localhost；内置超级用户受保护） */
        delete: operations["DELETE /api/databases/{id}/users/{username}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/dns-providers": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** DNS 厂商账号列表 */
        get: operations["GET /api/dns-providers"];
        put?: never;
        /** 创建 DNS 厂商账号 */
        post: operations["POST /api/dns-providers"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/dns-providers/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** 更新 DNS 厂商账号 */
        put: operations["PUT /api/dns-providers/{id}"];
        post?: never;
        /** 删除 DNS 厂商账号 */
        delete: operations["DELETE /api/dns-providers/{id}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/dns-providers/{id}/test": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 连通性测试（Stage 4：凭据结构校验级探测；Stage 5 升级为真实厂商 API 调用） */
        post: operations["POST /api/dns-providers/{id}/test"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/compose": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Compose 项目列表（含已停止项目；`{name, status, config_files}`） */
        get: operations["GET /api/docker/compose"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/compose/convert-run": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** `docker run` 命令 → Compose YAML（纯文本转换，不执行命令、不接触 Docker） */
        post: operations["POST /api/docker/compose/convert-run"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/compose/deploy": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 部署 Compose 项目（等价于宿主机 root 能力，故归 `docker:create`） */
        post: operations["POST /api/docker/compose/deploy"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/compose/{project_name}/down": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 停止并移除 Compose 项目容器与网络（保留配置文件与数据） */
        post: operations["POST /api/docker/compose/{project_name}/down"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/compose/{project_name}/up": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 启动 Compose 项目（使用项目记录的配置文件） */
        post: operations["POST /api/docker/compose/{project_name}/up"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/containers": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 容器列表 */
        get: operations["GET /api/docker/containers"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/containers/prune": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 清理已停止容器 */
        post: operations["POST /api/docker/containers/prune"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/containers/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 单个容器（按 ID 或名称匹配） */
        get: operations["GET /api/docker/containers/{id}"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/containers/{id}/compose": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 既有容器 → Compose YAML（收编：导出后停止原容器，再以该 YAML 部署） */
        get: operations["GET /api/docker/containers/{id}/compose"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/containers/{id}/inspect": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 容器详情（Engine API inspect 原样） */
        get: operations["GET /api/docker/containers/{id}/inspect"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/containers/{id}/kill": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 强制终止容器 */
        post: operations["POST /api/docker/containers/{id}/kill"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/containers/{id}/logs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 容器日志（stdout + stderr，输出上限 1 MiB） */
        get: operations["GET /api/docker/containers/{id}/logs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/containers/{id}/pause": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 暂停容器 */
        post: operations["POST /api/docker/containers/{id}/pause"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/containers/{id}/remove": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 删除容器（非强制） */
        post: operations["POST /api/docker/containers/{id}/remove"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/containers/{id}/rename": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 重命名容器 */
        post: operations["POST /api/docker/containers/{id}/rename"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/containers/{id}/restart": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 重启容器（10 s 超时） */
        post: operations["POST /api/docker/containers/{id}/restart"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/containers/{id}/start": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 启动容器 */
        post: operations["POST /api/docker/containers/{id}/start"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/containers/{id}/stats": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 容器资源统计（Engine API 原样） */
        get: operations["GET /api/docker/containers/{id}/stats"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/containers/{id}/stop": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 停止容器（10 s 超时） */
        post: operations["POST /api/docker/containers/{id}/stop"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/containers/{id}/unpause": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 恢复容器 */
        post: operations["POST /api/docker/containers/{id}/unpause"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/images": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 镜像列表 */
        get: operations["GET /api/docker/images"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/images/prune": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 清理悬空镜像 */
        post: operations["POST /api/docker/images/prune"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/images/pull": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 拉取镜像（`{image, message}`） */
        post: operations["POST /api/docker/images/pull"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/images/{id}/remove": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 删除镜像 */
        post: operations["POST /api/docker/images/{id}/remove"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/images/{id}/tag": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 为镜像打标签 */
        post: operations["POST /api/docker/images/{id}/tag"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/networks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 网络列表 */
        get: operations["GET /api/docker/networks"];
        put?: never;
        /** 创建网络 */
        post: operations["POST /api/docker/networks"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/networks/prune": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 清理未使用网络 */
        post: operations["POST /api/docker/networks/prune"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/networks/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** 删除网络 */
        delete: operations["DELETE /api/docker/networks/{id}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/networks/{id}/connect": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 把容器接入网络 */
        post: operations["POST /api/docker/networks/{id}/connect"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/networks/{id}/disconnect": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 把容器移出网络（`force` 可强制） */
        post: operations["POST /api/docker/networks/{id}/disconnect"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/volumes": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 卷列表 */
        get: operations["GET /api/docker/volumes"];
        put?: never;
        /** 创建卷 */
        post: operations["POST /api/docker/volumes"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/volumes/prune": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 清理未使用卷 */
        post: operations["POST /api/docker/volumes/prune"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/docker/volumes/{name}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** 删除卷 */
        delete: operations["DELETE /api/docker/volumes/{name}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/files": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/files` — 列出目录 */
        get: operations["GET /api/files"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/files/chmod": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** `POST /api/files/chmod` — 修改权限 */
        post: operations["POST /api/files/chmod"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/files/compress": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** `POST /api/files/compress` — 压缩 */
        post: operations["POST /api/files/compress"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/files/copy": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** `POST /api/files/copy` — 批量复制到目录 */
        post: operations["POST /api/files/copy"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/files/create-dir": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** `POST /api/files/create-dir` — 新建目录 */
        post: operations["POST /api/files/create-dir"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/files/create-file": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** `POST /api/files/create-file` — 新建空文件 */
        post: operations["POST /api/files/create-file"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/files/delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** `DELETE /api/files/delete` — 删除（默认移入回收站） */
        delete: operations["DELETE /api/files/delete"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/files/download": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/files/download` — 下载文件（流式） */
        get: operations["GET /api/files/download"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/files/extract": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** `POST /api/files/extract` — 解压 */
        post: operations["POST /api/files/extract"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/files/move": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** `POST /api/files/move` — 批量移动到目录 */
        post: operations["POST /api/files/move"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/files/read": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/files/read` — 读取文本文件（≤ 10 MiB，UTF-8） */
        get: operations["GET /api/files/read"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/files/recycle": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/files/recycle` — 回收站列表 */
        get: operations["GET /api/files/recycle"];
        put?: never;
        post?: never;
        /** `DELETE /api/files/recycle` — 清空回收站 */
        delete: operations["DELETE /api/files/recycle"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/files/recycle/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** `DELETE /api/files/recycle/{id}` — 彻底删除单个条目 */
        delete: operations["DELETE /api/files/recycle/{id}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/files/recycle/{id}/restore": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** `POST /api/files/recycle/{id}/restore` — 还原到原位置 */
        post: operations["POST /api/files/recycle/{id}/restore"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/files/rename": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** `POST /api/files/rename` — 重命名 / 移动 */
        post: operations["POST /api/files/rename"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/files/upload": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** `POST /api/files/upload` — 上传文件（请求体为文件原始字节） */
        post: operations["POST /api/files/upload"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/files/write": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** `POST /api/files/write` — 保存文件（原子替换，保留权限与属主） */
        post: operations["POST /api/files/write"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/firewall/apply": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 把全部已启用规则下发到系统防火墙（ufw / firewalld / iptables；面板非 root 时经特权助手） */
        post: operations["POST /api/firewall/apply"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/firewall/disable": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 停用系统防火墙 */
        post: operations["POST /api/firewall/disable"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/firewall/enable": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 启用系统防火墙 */
        post: operations["POST /api/firewall/enable"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/firewall/reorder": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 按给定顺序重排规则优先级 */
        post: operations["POST /api/firewall/reorder"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/firewall/rules": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 防火墙规则列表（分页，按优先级） */
        get: operations["GET /api/firewall/rules"];
        put?: never;
        /** 新建防火墙规则（仅落库，需 apply 才生效） */
        post: operations["POST /api/firewall/rules"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/firewall/rules/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 单条防火墙规则 */
        get: operations["GET /api/firewall/rules/{id}"];
        /** 更新防火墙规则（字段缺省即保持不变） */
        put: operations["PUT /api/firewall/rules/{id}"];
        post?: never;
        /** 删除防火墙规则 */
        delete: operations["DELETE /api/firewall/rules/{id}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/firewall/rules/{id}/toggle": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 启用 / 停用单条规则 */
        post: operations["POST /api/firewall/rules/{id}/toggle"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/firewall/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 防火墙后端状态（检测到的后端、是否启用等键值） */
        get: operations["GET /api/firewall/status"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/health": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/health` — 详细健康检查（免认证） */
        get: operations["GET /api/health"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/iac/export": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["GET /api/iac/export"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/inspection": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * `GET /api/inspection` — 系统体检报告（内核信息 + 风险项）
         * @description 命中缓存时 `cache_age_secs > 0`；`findings[].code` 为稳定编码，前端据此展示三语言标题与建议，
         *     `evidence` 为语言无关的原始取证。`heuristic = true` 的结论依赖内置生命周期表（`data_version`）。
         */
        get: operations["GET /api/inspection"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/inspection/run": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * `POST /api/inspection/run` — 立即重新体检（忽略缓存）
         * @description 采集有 15 s 总预算，单条外部命令 3 s 超时；超时或不可用的项以 `unknown` 呈现而不是让请求失败。
         */
        post: operations["POST /api/inspection/run"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/kernel-logs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/kernel-logs` — 概览 */
        get: operations["GET /api/kernel-logs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/kernel-logs/entries": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/kernel-logs/entries` — 倒序分页读取 */
        get: operations["GET /api/kernel-logs/entries"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/kernel-logs/files/{name}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** `DELETE /api/kernel-logs/files/{name}` — 删除历史日志文件 */
        delete: operations["DELETE /api/kernel-logs/files/{name}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/kernel-logs/files/{name}/download": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/kernel-logs/files/{name}/download` — 下载原始日志文件 */
        get: operations["GET /api/kernel-logs/files/{name}/download"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/kernel-logs/level": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** `PUT /api/kernel-logs/level` — 运行时调整内核日志级别 */
        put: operations["PUT /api/kernel-logs/level"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/kernel-logs/stats": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/kernel-logs/stats` — 五级计数 */
        get: operations["GET /api/kernel-logs/stats"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/logs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 系统事件日志列表 */
        get: operations["GET /api/logs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/logs/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** 删除一条系统事件日志 */
        delete: operations["DELETE /api/logs/{id}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/mcp": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/mcp` — 不提供 SSE 推送流。 */
        get: operations["GET /api/mcp"];
        put?: never;
        /** `POST /api/mcp` — MCP JSON-RPC 消息（单条或批量） */
        post: operations["POST /api/mcp"];
        /** `DELETE /api/mcp` — 终止会话。 */
        delete: operations["DELETE /api/mcp"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/mcp/approvals": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/mcp/approvals` — 审批单分页 */
        get: operations["GET /api/mcp/approvals"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/mcp/approvals/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/mcp/approvals/{id}` — 审批单详情 */
        get: operations["GET /api/mcp/approvals/{id}"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/mcp/approvals/{id}/approve": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** `POST /api/mcp/approvals/{id}/approve` — 批准并执行 */
        post: operations["POST /api/mcp/approvals/{id}/approve"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/mcp/approvals/{id}/reject": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** `POST /api/mcp/approvals/{id}/reject` — 拒绝 */
        post: operations["POST /api/mcp/approvals/{id}/reject"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/mcp/calls": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/mcp/calls` — 工具调用审计 */
        get: operations["GET /api/mcp/calls"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/mcp/principals": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/mcp/principals` — 凭证列表 */
        get: operations["GET /api/mcp/principals"];
        put?: never;
        /** `POST /api/mcp/principals` — 为当前用户签发凭证 */
        post: operations["POST /api/mcp/principals"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/mcp/principals/{id}/revoke": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** `POST /api/mcp/principals/{id}/revoke` — 吊销凭证 */
        post: operations["POST /api/mcp/principals/{id}/revoke"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/mcp/prompts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/mcp/prompts` — 运维技能库 */
        get: operations["GET /api/mcp/prompts"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/mcp/prompts/{name}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/mcp/prompts/{name}` — 渲染技能（查询参数即技能参数） */
        get: operations["GET /api/mcp/prompts/{name}"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/mcp/sessions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/mcp/sessions` — 会话列表 */
        get: operations["GET /api/mcp/sessions"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/mcp/tools": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/mcp/tools` — 工具目录 */
        get: operations["GET /api/mcp/tools"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/memos": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 备忘录列表（仪表盘便签与待办） */
        get: operations["GET /api/memos"];
        put?: never;
        /** 新建备忘 / 待办 */
        post: operations["POST /api/memos"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/memos/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** 更新备忘内容或完成状态 */
        put: operations["PUT /api/memos/{id}"];
        post?: never;
        /** 删除备忘 */
        delete: operations["DELETE /api/memos/{id}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/metrics/history": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * `GET /api/metrics/history` — 监控历史降采样曲线（需 `node:read`）
         * @description 1 min 点由 3 s 采样在内存聚合后落库（保留 7 d），1 h 点由整点 rollup 得到（保留 90 d）；
         *     响应带 `step_secs`，前端按桶宽绘制。刚启动或历史不足时点数少于窗口理论值。
         */
        get: operations["GET /api/metrics/history"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/metrics/processes": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * `GET /api/metrics/processes` — 按 CPU 占用排序的进程 TOP 5（免认证）
         *     F-17: processes 接口复用共享的 sysinfo::System 实例，避免每请求全量重建。
         */
        get: operations["GET /api/metrics/processes"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/nodes": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 节点列表（分页） */
        get: operations["GET /api/nodes"];
        put?: never;
        /** 注册节点 */
        post: operations["POST /api/nodes"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/nodes/batch-execute": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 批量在多个节点并行执行命令
         * @description S4：同样走 Agent `/action`（`whitelisted_command`），agent 模式下仅执行白名单命令。
         */
        post: operations["POST /api/nodes/batch-execute"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/nodes/heartbeat/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Agent 心跳上报（免 JWT，Agent token 已在中间件集中校验——S15） */
        post: operations["POST /api/nodes/heartbeat/{id}"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/nodes/register": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Agent 注册（白名单免 JWT，供 Agent 启动时自动注册）
         *     请求体：`{name, host, agent_port, auth_token}`，返回 `{"id": <node_id>}`
         */
        post: operations["POST /api/nodes/register"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/nodes/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** 更新节点 */
        put: operations["PUT /api/nodes/{id}"];
        post?: never;
        /** 删除节点 */
        delete: operations["DELETE /api/nodes/{id}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/nodes/{id}/action": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 调用 Agent 动作枚举（Phase A1：安全动作白名单） */
        post: operations["POST /api/nodes/{id}/action"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/nodes/{id}/execute": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 在远程节点执行命令
         * @description S4：统一走 Agent `/action`（`whitelisted_command`）。在 `execution_mode=agent` 下，
         *     仅执行白名单命令，非白名单命令将返回 `ACTION_NOT_ALLOWED`（功能降级）。
         */
        post: operations["POST /api/nodes/{id}/execute"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/nodes/{id}/files": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 列出远程节点目录 */
        get: operations["GET /api/nodes/{id}/files"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/nodes/{id}/files/download": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 下载远程节点文件 */
        get: operations["GET /api/nodes/{id}/files/download"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/nodes/{id}/files/upload": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 上传文件到远程节点 */
        post: operations["POST /api/nodes/{id}/files/upload"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/nodes/{id}/metrics": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 节点最近指标快照 */
        get: operations["GET /api/nodes/{id}/metrics"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/nodes/{id}/metrics/history": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 节点指标历史（内存窗口：Agent 10 s 一点、最多 120 点 ≈ 20 分钟；面板重启后重新积累） */
        get: operations["GET /api/nodes/{id}/metrics/history"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/nodes/{id}/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 节点在线状态（惰性判定） */
        get: operations["GET /api/nodes/{id}/status"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/operation-logs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 操作审计日志列表 */
        get: operations["GET /api/operation-logs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/operation-logs/export": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * `GET /api/operation-logs/export?format=csv|json` — 导出全部审计日志
         * @description Stage4.4：支持 CSV / JSON 两种格式，便于归档与 SIEM 对接。
         *     CSV 带 BOM（UTF-8）以兼容 Excel；JSON 为 UTF-8 数组。
         */
        get: operations["GET /api/operation-logs/export"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/operation-logs/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** 删除一条审计日志 */
        delete: operations["DELETE /api/operation-logs/{id}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/outbox-events": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/outbox-events` — 分页查询事件落库（审计）。 */
        get: operations["GET /api/outbox-events"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/overview/badges": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * `GET /api/overview/badges` — 顶栏徽标聚合（节点 / 容器 / 任务 / 告警一次返回）
         * @description 服务端 10 s TTL 缓存 + 单飞：多页签并发轮询只触发一次真实聚合；`cache_age_secs > 0` 表示命中缓存。
         */
        get: operations["GET /api/overview/badges"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/plugins": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 已加载插件列表 */
        get: operations["GET /api/plugins"];
        put?: never;
        /** 加载 WASM 插件并持久化（同 ID 覆盖） */
        post: operations["POST /api/plugins"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/plugins/market": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 插件市场目录 */
        get: operations["GET /api/plugins/market"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/plugins/market/{id}/install": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 从市场安装插件 */
        post: operations["POST /api/plugins/market/{id}/install"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/plugins/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 单个插件 */
        get: operations["GET /api/plugins/{id}"];
        put?: never;
        /** 卸载插件（返回 id / status=unloaded / exec_count） */
        post: operations["POST /api/plugins/{id}"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/plugins/{id}/capabilities": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** 变更插件宿主能力授予（整体替换，立即生效并持久化） */
        put: operations["PUT /api/plugins/{id}/capabilities"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/plugins/{id}/disable": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 禁用插件 */
        post: operations["POST /api/plugins/{id}/disable"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/plugins/{id}/enable": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 启用插件 */
        post: operations["POST /api/plugins/{id}/enable"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/plugins/{id}/execute/{function}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 执行插件导出函数（fuel 限额 + 内存上限） */
        post: operations["POST /api/plugins/{id}/execute/{function}"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/plugins/{id}/metrics": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 插件执行指标 */
        get: operations["GET /api/plugins/{id}/metrics"];
        put?: never;
        post?: never;
        /** 重置插件执行指标 */
        delete: operations["DELETE /api/plugins/{id}/metrics"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/plugins/{id}/reload": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 热重载插件字节码 */
        post: operations["POST /api/plugins/{id}/reload"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/plugins/{id}/settings": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 插件设置（键值） */
        get: operations["GET /api/plugins/{id}/settings"];
        put?: never;
        /** 写入一项插件设置 */
        post: operations["POST /api/plugins/{id}/settings"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/plugins/{id}/settings/{key}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 读取一项插件设置（id / key / value） */
        get: operations["GET /api/plugins/{id}/settings/{key}"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/projects": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 项目列表 */
        get: operations["GET /api/projects"];
        put?: never;
        /** 创建项目 */
        post: operations["POST /api/projects"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/projects/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 项目详情 */
        get: operations["GET /api/projects/{id}"];
        /** 更新项目 */
        put: operations["PUT /api/projects/{id}"];
        post?: never;
        /** 删除项目（运行中自动停止；部署目录保留——零锁定纪律） */
        delete: operations["DELETE /api/projects/{id}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/projects/{id}/builder": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 查看项目构建器决策（部署时自动协商结果，Doc/20 §5.8 ②） */
        get: operations["GET /api/projects/{id}/builder"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/projects/{id}/deploy": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 部署项目（git clone / local_path 校验；部署 ≠ 启动） */
        post: operations["POST /api/projects/{id}/deploy"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/projects/{id}/health-check": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 手动健康探测（Stage A-9 · Doc/20 §5.8 ④） */
        post: operations["POST /api/projects/{id}/health-check"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/projects/{id}/links": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 项目的联链列表 */
        get: operations["GET /api/projects/{id}/links"];
        put?: never;
        /** 关联数据库（联链成功即在建库侧幂等创建业务库；连接串在启动时注入）。 */
        post: operations["POST /api/projects/{id}/links"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/projects/{id}/links/available": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 可关联的数据库实例清单（联链弹窗数据源；仅返回 running 实例）。 */
        get: operations["GET /api/projects/{id}/links/available"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/projects/{id}/links/{link_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** 解除联链 */
        delete: operations["DELETE /api/projects/{id}/links/{link_id}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/projects/{id}/logs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 最近日志（按执行面分流：容器模式 docker logs / 原生模式守护器日志） */
        get: operations["GET /api/projects/{id}/logs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/projects/{id}/procfile": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 查看项目 Procfile 进程声明 */
        get: operations["GET /api/projects/{id}/procfile"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/projects/{id}/restart": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 重启项目 */
        post: operations["POST /api/projects/{id}/restart"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/projects/{id}/start": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 启动项目 */
        post: operations["POST /api/projects/{id}/start"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/projects/{id}/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 实时进程状态（按执行面分流：容器模式 docker inspect / 原生模式守护器探测） */
        get: operations["GET /api/projects/{id}/status"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/projects/{id}/stop": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 停止项目 */
        post: operations["POST /api/projects/{id}/stop"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/projects/{id}/webhook": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Webhook push 触发部署（免 JWT，令牌经中间件校验；
         *     兼容 GitHub / GitLab / Gitee / Gitea / 通用 JSON——服务端不解析平台语义，
         *     任何 POST 命中即视为一次 push 部署请求）
         */
        post: operations["POST /api/projects/{id}/webhook"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/projects/{id}/webhook/disable": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 关闭项目 webhook 部署 */
        post: operations["POST /api/projects/{id}/webhook/disable"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/projects/{id}/webhook/enable": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 开启/重置项目 webhook 部署（返回一次性部署令牌） */
        post: operations["POST /api/projects/{id}/webhook/enable"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/runtimes": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 运行时列表 */
        get: operations["GET /api/runtimes"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/runtimes/available": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 可安装版本清单（声明版本 × 系统探测） */
        get: operations["GET /api/runtimes/available"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/runtimes/install": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 安装运行时 */
        post: operations["POST /api/runtimes/install"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/runtimes/kind/{kind}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 按语言过滤运行时 */
        get: operations["GET /api/runtimes/kind/{kind}"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/runtimes/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 运行时详情（GET /api/runtimes/{id}） */
        get: operations["GET /api/runtimes/{id}"];
        put?: never;
        post?: never;
        /** 卸载运行时 */
        delete: operations["DELETE /api/runtimes/{id}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/runtimes/{id}/default": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 设为同语言默认版本 */
        post: operations["POST /api/runtimes/{id}/default"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/runtimes/{id}/sites": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 按运行时列出绑定站点（M-B 主线二 Stage A-6 · Doc/20 §5.5） */
        get: operations["GET /api/runtimes/{id}/sites"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/scheduled-tasks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 定时任务列表（分页） */
        get: operations["GET /api/scheduled-tasks"];
        put?: never;
        /** 创建定时任务 */
        post: operations["POST /api/scheduled-tasks"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/scheduled-tasks/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 定时任务详情 */
        get: operations["GET /api/scheduled-tasks/{id}"];
        /** 更新定时任务 */
        put: operations["PUT /api/scheduled-tasks/{id}"];
        post?: never;
        /** 删除定时任务 */
        delete: operations["DELETE /api/scheduled-tasks/{id}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/scheduled-tasks/{id}/run": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 立即执行定时任务 */
        post: operations["POST /api/scheduled-tasks/{id}/run"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/scheduled-tasks/{id}/toggle": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 启用 / 停用定时任务 */
        post: operations["POST /api/scheduled-tasks/{id}/toggle"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/settings": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 设置列表（分页） */
        get: operations["GET /api/settings"];
        /** 更新设置 */
        put: operations["PUT /api/settings"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/settings/access": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 访问来源与访问控制策略 */
        get: operations["GET /api/settings/access"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/settings/batch": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        /** 批量更新设置（一次事务原子写入多键） */
        patch: operations["PATCH /api/settings/batch"];
        trace?: never;
    };
    "/api/settings/{key}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 读取单个设置项 */
        get: operations["GET /api/settings/{key}"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/setup/initialize": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** `POST /api/setup/initialize` */
        post: operations["POST /api/setup/initialize"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/setup/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/setup/status` */
        get: operations["GET /api/setup/status"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/tasks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 列出全部任务（可按状态过滤）。 */
        get: operations["GET /api/tasks"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/tasks/prune": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 清理全部终态任务。 */
        post: operations["POST /api/tasks/prune"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/tasks/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 查询单个任务。 */
        get: operations["GET /api/tasks/{id}"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/tasks/{id}/cancel": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 取消任务（`Pending → Cancelled` / `Running → Cancelled`）。 */
        post: operations["POST /api/tasks/{id}/cancel"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/toolbox/host": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/toolbox/host` — 主机详情 */
        get: operations["GET /api/toolbox/host"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/toolbox/processes": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** `GET /api/toolbox/processes` — 进程列表 */
        get: operations["GET /api/toolbox/processes"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/toolbox/processes/{pid}/kill": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** `POST /api/toolbox/processes/{pid}/kill` — 向进程发送信号 */
        post: operations["POST /api/toolbox/processes/{pid}/kill"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/users": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 用户列表（分页） */
        get: operations["GET /api/users"];
        put?: never;
        /** 创建用户 */
        post: operations["POST /api/users"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/users/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** 更新用户 */
        put: operations["PUT /api/users/{id}"];
        post?: never;
        /** 删除用户 */
        delete: operations["DELETE /api/users/{id}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/users/{id}/2fa/reset": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** 管理员重置指定用户的两步验证（用户丢失认证器且恢复码用尽时；需 `user:update` + admin 角色） */
        put: operations["PUT /api/users/{id}/2fa/reset"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/waf/apply": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 下发 WAF 规则到本机 Web 引擎（Nginx / OpenResty 原生配置；Doc/32 W3）
         * @description 规则变更时会自动尽力下发；本端点用于手动重试与查看逐引擎结果（校验失败时已回滚）。
         *     未配置下发能力返回 503。
         */
        post: operations["POST /api/waf/apply"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/waf/logs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** WAF 拦截日志（分页，按时间倒序） */
        get: operations["GET /api/waf/logs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/waf/logs/clear": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 清理 WAF 拦截日志（website_id = 0 清空全部） */
        post: operations["POST /api/waf/logs/clear"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/waf/rules": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** WAF 规则列表（website_id 查询参数过滤，0 = 全部） */
        get: operations["GET /api/waf/rules"];
        put?: never;
        /** 创建 WAF 规则 */
        post: operations["POST /api/waf/rules"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/waf/rules/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 获取 WAF 规则详情 */
        get: operations["GET /api/waf/rules/{id}"];
        /** 更新 WAF 规则 */
        put: operations["PUT /api/waf/rules/{id}"];
        post?: never;
        /** 删除 WAF 规则（拦截日志保留，规则名快照仍可读） */
        delete: operations["DELETE /api/waf/rules/{id}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/waf/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** WAF 引擎诊断（启用规则数 / 是否配置 Web 引擎下发 / 命中日志回灌状态） */
        get: operations["GET /api/waf/status"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/web-servers": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 面板登记的 Web 服务器实例（分页） */
        get: operations["GET /api/web-servers"];
        put?: never;
        /** 登记实例（不安装软件；安装见 native/install） */
        post: operations["POST /api/web-servers"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/web-servers/capabilities": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * 查询引擎支持的配套能力（PHP-FPM / Lua / Redis / 扩展模块 / 自动 HTTPS）。
         *     引擎可选配套能力（PHP-FPM / Lua / 缓存 / 扩展模块 / 自动 HTTPS）及推荐系统包
         */
        get: operations["GET /api/web-servers/capabilities"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/web-servers/capabilities/install": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 批量安装引擎的配套依赖。
         *     批量安装配套能力
         */
        post: operations["POST /api/web-servers/capabilities/install"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/web-servers/engines": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 支持的 Web 引擎及能力 */
        get: operations["GET /api/web-servers/engines"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/web-servers/native/autostart": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 按引擎设置 systemd 开机自启（未登记实例也可用） */
        post: operations["POST /api/web-servers/native/autostart"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/web-servers/native/detect": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 探测系统已安装的 Web 引擎（包管理器 / PATH / 候选路径 + systemd 状态 + 监听端口） */
        get: operations["GET /api/web-servers/native/detect"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/web-servers/native/install": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 经系统包管理器安装引擎并登记实例 */
        post: operations["POST /api/web-servers/native/install"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/web-servers/native/uninstall": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 卸载系统引擎并删除对应实例登记 */
        post: operations["POST /api/web-servers/native/uninstall"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/web-servers/presets": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 性能预设列表（按本机资源标注推荐项） */
        get: operations["GET /api/web-servers/presets"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/web-servers/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 单个实例 */
        get: operations["GET /api/web-servers/{id}"];
        /** 更新实例字段（`engine` / `version` / `config_path` / `binary_path` / `port`，可带 `resource_version` 做乐观并发） */
        put: operations["PUT /api/web-servers/{id}"];
        post?: never;
        /** 删除实例登记（不卸载软件） */
        delete: operations["DELETE /api/web-servers/{id}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/web-servers/{id}/autostart": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 设置实例对应 systemd 服务的开机自启 */
        post: operations["POST /api/web-servers/{id}/autostart"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/web-servers/{id}/config": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 读取实例主配置文件（仅限引擎配置根下；不存在时 content 为提示文本） */
        get: operations["GET /api/web-servers/{id}/config"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/web-servers/{id}/configtest": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 校验引擎配置（`nginx -t` 等） */
        post: operations["POST /api/web-servers/{id}/configtest"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/web-servers/{id}/native-status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 实例对应引擎的原生状态 */
        get: operations["GET /api/web-servers/{id}/native-status"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/web-servers/{id}/preset": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 应用性能预设：生成全局配置 → 原子写 → 校验 → reload，失败回滚 */
        post: operations["POST /api/web-servers/{id}/preset"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/web-servers/{id}/reload": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 平滑重载配置 */
        post: operations["POST /api/web-servers/{id}/reload"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/web-servers/{id}/restart": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 重启引擎 */
        post: operations["POST /api/web-servers/{id}/restart"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/web-servers/{id}/start": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 启动引擎 */
        post: operations["POST /api/web-servers/{id}/start"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/web-servers/{id}/stop": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 停止引擎 */
        post: operations["POST /api/web-servers/{id}/stop"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/web-servers/{id}/switch-engine": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 切换实例引擎（按新引擎默认路径重写登记） */
        post: operations["POST /api/web-servers/{id}/switch-engine"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/websites": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 网站列表（分页） */
        get: operations["GET /api/websites"];
        put?: never;
        /** 创建网站 */
        post: operations["POST /api/websites"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/websites/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 获取网站详情 */
        get: operations["GET /api/websites/{id}"];
        /** 更新网站 */
        put: operations["PUT /api/websites/{id}"];
        post?: never;
        /** 删除网站 */
        delete: operations["DELETE /api/websites/{id}"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/websites/{id}/deploy": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 手动部署站点配置（Doc/20 §5.5 站点配置流水线）：按当前库中状态渲染完整配置
         *     （含已绑定证书的 HTTPS），写入面板自有站点目录 → 配置校验 → reload，失败整体回滚。
         * @description 严格模式：引擎未安装 / 配置校验失败返回 400，同名非托管 OLS 虚拟主机返回 409。
         */
        post: operations["POST /api/websites/{id}/deploy"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/websites/{id}/rewrite": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 查询站点伪静态套用状态（当前模板 + 该引擎语法规则文本 + 可选模板清单） */
        get: operations["GET /api/websites/{id}/rewrite"];
        put?: never;
        /**
         * 一键套用伪静态规则模板——落库 + 写 vhost + 校验 + reload（「套用即生效」）。
         * @description 响应保持既有契约（生成的规则文本，按站点引擎语法）；写盘失败返回错误并回滚套用。
         */
        post: operations["POST /api/websites/{id}/rewrite"];
        /** 清除站点伪静态套用（回落引擎默认路由；同样落库 + 写 vhost）。 */
        delete: operations["DELETE /api/websites/{id}/rewrite"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/websites/{id}/runtime": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 绑定/解绑运行时（解绑传 site_type=static 且不传 id） */
        post: operations["POST /api/websites/{id}/runtime"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/websites/{id}/switch-engine": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 切换网站的 Web 服务器引擎（重写配置并重载） */
        post: operations["POST /api/websites/{id}/switch-engine"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/health": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 存活探针：进程能响应即返回纯文本 `OK`（负载均衡 / 容器健康检查用；详细检查见 `/api/health`） */
        get: operations["GET /health"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/metrics": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * `GET /metrics` — Prometheus 文本格式指标（面板自身系统指标）
         * @description Stage4.3：暴露 Prometheus 兼容的文本格式（`text/plain; version=0.0.4`），
         *     供 Prometheus/Grafana 直接抓取。指标来自最近一次 `MetricsSnapshot`
         *     （3s 周期采集）。公开可读（无敏感信息）。
         */
        get: operations["GET /metrics"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        /** @description 当前请求的访问来源与生效中的访问控制策略（设置页据此提示「你的 IP」，避免误锁） */
        AccessInfo: {
            allowed_ips: string[];
            bind_domains: string[];
            /** @description 面板看到的客户端 IP（经本机反代时取转发头中的真实客户端） */
            client_ip?: string | null;
            /** @description 访问所用域名 / 主机（含端口时已去除） */
            host?: string | null;
            /** @description 是否本机直连（回环且未经反代）；本机直连不受授权 IP / 绑定域名限制 */
            local: boolean;
            /** Format: int64 */
            login_lockout_minutes: number;
            /** Format: int32 */
            login_max_failures: number;
            /** Format: int64 */
            session_timeout_minutes: number;
        };
        /** @description 账号响应（config_json 不回显，防止敏感连接信息泄漏） */
        AccountDto: {
            account_type: string;
            /** Format: date-time */
            created_at: string;
            enabled: boolean;
            /** Format: int64 */
            id: number;
            /** Format: date-time */
            last_test_at?: string | null;
            last_test_status?: string | null;
            name: string;
            /** @description 最近一次探测失败原因（成功为 null；仅 test 端点返回） */
            test_message?: string | null;
            /** Format: date-time */
            updated_at: string;
        };
        /** @description AI 接入凭证。`token_hash` 永不序列化到 API 响应。 */
        AgentPrincipal: {
            /**
             * @description 是否允许发起写操作（M-AI-2）。缺省 false：凭证只能调用只读工具；
             *     为 true 时写工具仍需面板人工批准才会执行。
             */
            allow_write?: boolean;
            /** Format: date-time */
            created_at: string;
            /**
             * Format: date-time
             * @description 过期时间（None = 永不过期）
             */
            expires_at?: string | null;
            /** Format: int64 */
            id: number;
            /** Format: date-time */
            last_used_at?: string | null;
            /** @description 凭证名称（如 `claude-desktop@laptop`） */
            name: string;
            /**
             * Format: date-time
             * @description 吊销时间（None = 有效）
             */
            revoked_at?: string | null;
            /** @description 可展示前缀（`fpk_xxxxxxxx`），用于在列表中辨认凭证 */
            token_prefix: string;
            /**
             * Format: int64
             * @description 绑定用户（签发者）
             */
            user_id: number;
        };
        /** @description MCP 会话。 */
        AgentSession: {
            /** @description 客户端自报名称（`clientInfo.name`，如 `claude-ai` / `cursor`） */
            client_name: string;
            client_version: string;
            /** Format: date-time */
            closed_at?: string | null;
            /** Format: date-time */
            created_at: string;
            /** @description `Mcp-Session-Id`（uuid v4） */
            id: string;
            /** Format: date-time */
            last_seen_at: string;
            /** Format: int64 */
            principal_id: number;
            /** @description 协商后的协议版本 */
            protocol_version: string;
        };
        /** @description 告警徽标：需要人看一眼的事项计数；各分项为 `null` 表示该域不可见或不可用。 */
        AlertBadges: {
            /**
             * Format: int64
             * @description 14 天内到期（含已过期）的证书数
             */
            certificates_expiring?: number | null;
            /**
             * Format: int64
             * @description 离线节点数（total − online）
             */
            nodes_offline?: number | null;
            /**
             * Format: int64
             * @description 失败任务数（TaskTracker 中尚未清理的终态失败记录）
             */
            tasks_failed?: number | null;
            /**
             * Format: int64
             * @description 可见分项之和
             */
            total: number;
        };
        /** @enum {string} */
        AppFormat: "onepanel" | "baota" | "flame";
        /**
         * @description 应用包生命周期钩子（Doc/20 §4.3）
         *
         *     宿主机脚本路径（相对包根/版本目录）。容器模式安装/升级/卸载时由执行器
         *     经统一任务中心按序执行：超时 300s、输出落任务日志、失败即中止。
         *     脚本须通过 security_scanner 静态扫描（高危模式告警/阻断）。
         */
        AppHooks: {
            /** @description 容器安装（compose up）成功后执行 */
            post_install?: string | null;
            /** @description 升级成功后执行 */
            post_upgrade?: string | null;
            /** @description 容器安装前执行（相对包根，如 `scripts/pre_install.sh`） */
            pre_install?: string | null;
            /** @description 卸载（compose down）前执行 */
            pre_uninstall?: string | null;
            /** @description 升级（换镜像 compose up）前执行 */
            pre_upgrade?: string | null;
        };
        AppMetadata: {
            architectures: string[];
            category: string;
            default_version: string;
            format: components["schemas"]["AppFormat"];
            key: string;
            logo?: string | null;
            /** Format: int32 */
            min_memory_mb?: number | null;
            modes: components["schemas"]["InstallMode"][];
            name: string;
            readme?: string | null;
            /** @description 商店首页推荐位标记（v0.7.0） */
            recommended?: boolean;
            short_desc_en?: string | null;
            short_desc_zh: string;
            tags: string[];
            versions: string[];
        };
        /**
         * @description 远程应用源（M-A Issue 1.1，Doc/20 §4.1）
         *
         *     可配置多源：git（1Panel appstore 仓库镜像）| http（自建源 JSON manifest）。
         *     远程包同步走统一任务中心；安装时才按需拉取包体到本地缓存。
         */
        AppSource: {
            /** Format: date-time */
            created_at: string;
            enabled: boolean;
            /** Format: int64 */
            id: number;
            /**
             * Format: date-time
             * @description 最近一次索引同步时刻
             */
            last_synced_at?: string | null;
            name: string;
            /** @description Ed25519 公钥（应用包验签，Doc/20 §4.2；未配置则不做验签） */
            public_key?: string | null;
            /** @description 源类型：git | http */
            source_type: string;
            /** Format: date-time */
            updated_at: string;
            url: string;
        };
        AppStoreListResponse: {
            packages: components["schemas"]["AppMetadata"][];
            /** Format: int64 */
            total: number;
        };
        ApplyPresetRequest: {
            preset: string;
        };
        /** @description 伪静态模板套用请求 */
        ApplyRewriteRequest: {
            /** @description 模板名：wordpress | discuz | typecho | laravel | thinkphp | springboot */
            preset: string;
        };
        /**
         * @description 审批单状态。`pending` → `executing` → `succeeded` / `failed`；`pending` → `rejected` / `expired`。
         * @enum {string}
         */
        ApprovalStatus: "pending" | "executing" | "succeeded" | "failed" | "rejected" | "expired";
        /** @description 配置更新（字段均可选，缺省保持原值）。 */
        AssistantConfigPatch: {
            /** @description 缺省 = 保留原密钥；空字符串 = 清除；其他 = 替换 */
            api_key?: string | null;
            base_url?: string | null;
            enabled?: boolean | null;
            /** Format: int32 */
            max_steps?: number | null;
            model?: string | null;
            provider?: null | components["schemas"]["LlmProviderKind"];
            /** Format: float */
            temperature?: number | null;
            /** Format: int64 */
            timeout_secs?: number | null;
        };
        /** @description 配置对外视图：不含密钥，只告知是否已设置。 */
        AssistantConfigView: {
            /** @description 留空表示使用协议缺省地址 */
            base_url: string;
            /** @description 实际生效的地址 */
            effective_base_url: string;
            enabled: boolean;
            has_api_key: boolean;
            /** Format: int32 */
            max_steps: number;
            model: string;
            provider: components["schemas"]["LlmProviderKind"];
            /** Format: float */
            temperature: number;
            /** Format: int64 */
            timeout_secs: number;
        };
        AutostartRequest: {
            enabled: boolean;
        };
        /**
         * @description 备份账号（M-A 收尾 · Doc/20 §6.5）
         *
         *     云端备份目标账号：S3 兼容（AWS S3 / MinIO / OSS / COS / R2）| SFTP | 本地目录。
         *     `config_json` 为各后端各自的连接配置序列化；落库前经 `SecretCipher`（AES-256-GCM）
         *     加密为 `enc:v1:` 密文（Doc/20 §6.5 收尾），使用时透明解密，历史明文行静默兼容。
         */
        BackupAccount: {
            /** @description 账号类型：s3 | sftp | local */
            account_type: string;
            /** @description 后端连接配置（JSON，结构由各 BackupBackend 实现自解析） */
            config_json: string;
            /** Format: date-time */
            created_at: string;
            enabled: boolean;
            /** Format: int64 */
            id: number;
            /** Format: date-time */
            last_test_at?: string | null;
            /** @description 最近一次连通性测试：ok | failed（None 表示从未测试） */
            last_test_status?: string | null;
            name: string;
            /** Format: date-time */
            updated_at: string;
        };
        BackupEntryDto: {
            created_at: string;
            filename: string;
            /** Format: int64 */
            size: number;
        };
        /**
         * @description 备份任务（M-A 收尾 Stage 3 · Doc/20 §6.5）
         *
         *     定义「备份什么（scope）→ 何时备份（cron schedule）→ 备到哪（target_account_id）
         *     → 保留几份（retention）」的计划备份任务，由后台调度器周期 tick 驱动执行。
         */
        BackupJob: {
            /** Format: date-time */
            created_at: string;
            enabled: boolean;
            /** Format: int64 */
            id: number;
            last_message?: string | null;
            /** @description 最近一次成功执行的远端对象 key */
            last_remote_key?: string | null;
            /** Format: date-time */
            last_run_at?: string | null;
            /** @description 最近一次执行：success | failed（never 表示从未执行） */
            last_status: string;
            name: string;
            /** Format: date-time */
            next_run_at?: string | null;
            /**
             * Format: int64
             * @description 备份保留份数（超出后按最旧清理；0 = 不清理）
             */
            retention: number;
            /** @description 标准 5 字段 cron 表达式 */
            schedule: string;
            /** @description 备份范围：panel（面板库）| app（应用，scope_key 为应用 key） */
            scope: string;
            /** @description 范围内具体对象：panel 恒为空；app 为应用 key */
            scope_key?: string | null;
            /**
             * Format: int64
             * @description 目标备份账号 ID（指向 backup_accounts）
             */
            target_account_id: number;
            /** Format: date-time */
            updated_at: string;
        };
        BatchImportPackagesRequest: {
            paths: string[];
        };
        BatchImportPackagesResponse: {
            count: number;
            imported: components["schemas"]["AppMetadata"][];
        };
        /** @description Phase A2 扩展：数据库实例批量状态更新请求体（批量原子写事务）。 */
        BatchStatusRequest: {
            /** @description 每项为 `[instance_id, 状态]`，状态限 `running` / `stopped` / `error`，单次 ≤ 200 项 */
            updates: unknown[][];
        };
        BindCertificateRequest: {
            /** Format: int64 */
            website_id?: number | null;
        };
        /** @description 站点绑定请求（M-B 主线二 Stage A-6 · Doc/20 §5.5） */
        BindRuntimeRequest: {
            /**
             * Format: int64
             * @description 绑定的项目 ID（reverse_proxy + 项目场景）
             */
            project_id?: number | null;
            /**
             * Format: int64
             * @description 绑定的运行时 ID（runtime_php 场景）
             */
            runtime_id?: number | null;
            /** @description 站点类型：runtime_php | reverse_proxy | static | container_app */
            site_type: string;
        };
        /** @description 项目构建器决策（Stage A-8 · Doc/20 §5.8 ②）。 */
        BuilderDecisionDto: {
            /** @description 当前协商出的构建器：dockerfile | buildpacks | runtime */
            build_type: string;
            /** @description 容器模式本地镜像名（其余构建器为 None） */
            image?: string | null;
            /** Format: int64 */
            project_id: number;
        };
        CapabilityInfo: {
            hint: string;
            key: string;
            label_en: string;
            label_zh: string;
            /** @description 推荐系统包（按当前发行版计算；空表示引擎内置/无需安装） */
            packages: string[];
        };
        /**
         * @description 检查项分类（前端分组展示）。
         * @enum {string}
         */
        Category: "kernel" | "cpu" | "hardening" | "access" | "network" | "updates" | "storage" | "panel";
        /**
         * @description SSL 证书记录（M-B 主线三 §6.1 SSL 证书自动化 · Stage 1 数据层）。
         *
         *     一条记录 = 一次签发/上传的证书（SAN 列表维度唯一）。ACME 账户私钥等
         *     敏感凭据不落本表——账户密钥持久化在 `data/acme/account.key`（0600），
         *     DNS 厂商 API 凭据复用 `SecretCipher` 加密语义（落库仅密文）。
         */
        Certificate: {
            /** @description 是否自动续签 */
            auto_renew: boolean;
            /** @description 证书文件路径（PEM 全链） */
            cert_path?: string | null;
            /** Format: date-time */
            created_at: string;
            /** @description DNS-01 厂商：cloudflare | aliyun | tencent | huawei（HTTP-01/无 DNS 为 None） */
            dns_provider?: string | null;
            /** @description SAN 列表 JSON（主域名 + 备用域名/泛域名，如 `["example.com","*.example.com"]`） */
            domains: string;
            /**
             * Format: date-time
             * @description 到期时刻（PEM 解析得来，None = 未签发/待上传）
             */
            expires_at?: string | null;
            /** Format: int64 */
            id: number;
            /** @description 私钥文件路径（0600） */
            key_path?: string | null;
            /**
             * Format: date-time
             * @description 最近一次续签/签发时刻
             */
            last_renewed_at?: string | null;
            /** @description 证书来源：letsencrypt | zerossl | self_signed | uploaded */
            source: string;
            /** Format: date-time */
            updated_at: string;
            /**
             * Format: int64
             * @description 关联网站 ID（可选；解除网站绑定不删除证书记录）
             */
            website_id?: number | null;
        };
        CertificateListResponse: {
            certificates: components["schemas"]["Certificate"][];
            /** Format: int64 */
            total: number;
        };
        /** @description 证书续签概况 + 时间线（S-10） */
        CertificateRenewalHistory: {
            /** @description 当前自动续签开关 */
            auto_renew: boolean;
            /** Format: int64 */
            certificate_id: number;
            /** @description 时间线（最新在前） */
            entries: components["schemas"]["RenewalEntry"][];
            /** Format: date-time */
            expires_at?: string | null;
            /** Format: date-time */
            last_renewed_at?: string | null;
            /**
             * Format: date-time
             * @description 续签窗口起点（到期前 30 天）：此后每次后台扫描都会尝试续签
             */
            renewal_window_starts_at?: string | null;
            /**
             * Format: int64
             * @description 后台续签扫描周期（小时）
             */
            scan_interval_hours: number;
            /** @description 来源是否支持自动续签（仅 ACME；自签 / 上传为 false，开关应禁用） */
            supports_auto_renew: boolean;
            /** @description 是否还有更早的记录未返回 */
            truncated: boolean;
        };
        /** @description 修改密码请求（`POST /api/auth/change-password`） */
        ChangePasswordBody: {
            /** @description 需满足密码强度策略，且不得与用户名相同 */
            new_password: string;
            old_password: string;
        };
        ChatRequest: {
            /** @description 会话 ID（首轮留空，由服务端生成；后续轮次回传以便审计归组） */
            conversation_id?: string | null;
            /** @description 对话历史（≤ 40 条，末条为用户消息） */
            messages: components["schemas"]["ChatTurn"][];
        };
        ChatResponse: {
            /** @description 本轮生成的审批单 ID（便于前端直达审批页） */
            approval_ids: number[];
            conversation_id: string;
            /** @description 助手回复（Markdown 文本） */
            reply: string;
            /** @description 是否因达到步数上限而强制收尾 */
            step_limit_reached: boolean;
            /** @description 本轮工具调用轨迹 */
            steps: components["schemas"]["ToolStep"][];
        };
        /** @description 客户端提交的一条历史消息（只允许 user / assistant）。 */
        ChatTurn: {
            content: string;
            /** @description `user` 或 `assistant` */
            role: string;
        };
        /**
         * @description 检查项状态：`pass` 未发现风险 / `fail` 存在风险 / `unknown` 无法判定（数据缺失或不可读）。
         * @enum {string}
         */
        CheckStatus: "pass" | "fail" | "unknown";
        ChmodRequest: {
            /** @description 八进制权限，如 `644` / `0755` */
            mode: string;
            path: string;
        };
        ClearLogsRequest: {
            /**
             * Format: int64
             * @description 0 或缺省 = 清空全部
             */
            website_id?: number;
        };
        /** @description 转换结果：建议的项目名 / 服务名、YAML 与需人工确认的提示。 */
        ComposeConversion: {
            /** @description Compose YAML */
            compose_yaml: string;
            /** @description 建议的 Compose 项目名（满足 `validate_project_name`） */
            project_name: string;
            /** @description 生成的服务名 */
            service_name: string;
            /** @description 无法等价转换或需要人工确认的事项 */
            warnings: string[];
        };
        ComposeDeployRequest: {
            /** @description ≤ 256 KiB；文件持久化到 `data/compose/<project>/docker-compose.yml` */
            compose_yaml: string;
            /** @description `[A-Za-z0-9][A-Za-z0-9_.-]*`，≤ 63 */
            project_name: string;
        };
        ComposeDeployResponse: {
            message: string;
            project_name: string;
            status: string;
        };
        /** @description 合并结果。 */
        ComposeMerge: {
            conflicts: components["schemas"]["MergeConflict"][];
            /** @description 保留下来的本地修改路径。 */
            local_changes: string[];
            /** @description 合并后的 compose 文本（`Abort` 且有冲突时为保留本地值的草稿，不应落盘）。 */
            merged: string;
            /** @description 是否经结构化重新序列化（注释 / 格式不保留）。 */
            reformatted: boolean;
            /** @description 生效的上游修改路径。 */
            upstream_changes: string[];
        };
        CompressRequest: {
            /** @description 压缩包路径，扩展名决定格式：`.tar.gz` / `.tgz` / `.zip` */
            dest: string;
            overwrite?: boolean;
            /** @description 同一目录下的若干条目 */
            paths: string[];
        };
        CompressResult: {
            /** @description 写入的条目数（文件 + 目录 + 链接） */
            entries: number;
            /** @description 压缩包的根内相对路径 */
            path: string;
            /** Format: int64 */
            size: number;
            /** @description 跳过的条目（zip 中的符号链接、特殊文件） */
            skipped: number;
        };
        ConnectNetworkRequest: {
            container_id: string;
            force?: boolean;
        };
        /** @description 容器徽标：运行中 / 总数（本机 Docker，`status == "running"`）。 */
        ContainerBadges: {
            /** Format: int64 */
            running: number;
            /** Format: int64 */
            total: number;
        };
        /** @description `docker run` 转换请求 */
        ConvertRunRequest: {
            /** @description 完整的 `docker run …` 命令（可含 `\` 续行，≤ 16 KiB） */
            command: string;
        };
        CreateAccountRequest: {
            /** @description 账号类型：s3 | sftp | local */
            account_type: string;
            /** @description 后端连接配置（JSON 字符串，敏感字段按各后端约定脱敏存储） */
            config_json?: string | null;
            enabled?: boolean | null;
            name: string;
        };
        CreateDatabaseRequest: {
            charset?: string | null;
            name: string;
        };
        CreateDnsProviderRequest: {
            /** @description 厂商凭据（JSON 字符串，落库前统一加密） */
            credentials_json?: string | null;
            enabled?: boolean | null;
            name: string;
            /** @description 厂商标识：cloudflare | aliyun | tencent | huawei */
            provider_type: string;
        };
        CreateFirewallRuleRequest: {
            action?: string | null;
            /** @description 缺省 `tcp`；`tcp` / `udp` / `both` / `icmp` */
            description?: string | null;
            destination?: string | null;
            direction?: string | null;
            enabled?: boolean | null;
            name: string;
            port?: string | null;
            /** Format: int32 */
            priority?: number | null;
            protocol?: string | null;
            source?: string | null;
        };
        CreateJobRequest: {
            enabled?: boolean;
            name: string;
            /**
             * Format: int64
             * @description 备份保留份数（0 = 用默认 7）
             */
            retention?: number | null;
            /** @description 标准 5 字段 cron 表达式 */
            schedule: string;
            /** @description 备份范围：panel（面板库）| app（应用目录） */
            scope: string;
            /** @description 范围内具体对象：panel 省略；app 为应用 key */
            scope_key?: string | null;
            /**
             * Format: int64
             * @description 目标备份账号 ID
             */
            target_account_id: number;
        };
        CreateMemoRequest: {
            content: string;
            kind?: string;
        };
        CreateNetworkRequest: {
            driver?: string;
            name: string;
            subnet?: string | null;
        };
        /**
         * @description 节点注册请求：兼容两种格式
         *     - 面板/测试：`{"node": {name, hostname, ip_address, status, ...}}`
         *     - Agent 平铺：`{name, host, agent_port, auth_token}`
         */
        CreateNodeRequest: {
            /**
             * Format: int32
             * @description Agent HTTP 服务端口（默认 9527）
             */
            agent_port?: number;
            auth_token?: string | null;
            hostname?: string;
            ip_address?: string;
            name?: string;
            node?: null | components["schemas"]["ServerNode"];
            status?: string;
        };
        CreatePrincipalRequest: {
            /** @description 允许发起写操作（M-AI-2）；写操作仍须面板人工批准。缺省 false（只读） */
            allow_write?: boolean;
            /**
             * Format: int64
             * @description 有效天数（1–3650）；缺省 / null 表示永不过期
             */
            expires_in_days?: number | null;
            /** @description 凭证名称（1–64 字符），如 `claude-desktop@laptop` */
            name: string;
        };
        CreatePrincipalResponse: {
            /** @description MCP 端点路径（客户端配置 `url` 时拼上面板地址） */
            endpoint: string;
            principal: components["schemas"]["PrincipalView"];
            /** @description 明文令牌——**仅此一次返回**，请立即保存 */
            token: string;
        };
        /** @description 创建项目请求。 */
        CreateProjectRequest: {
            /** @description 留空自动取 /opt/flamepanel/projects/<name> */
            deploy_path?: string | null;
            /** @description 切换策略：restart | rolling（默认 restart） */
            deploy_strategy?: string | null;
            domain?: string | null;
            /** @description 健康检查路径（如 /health；空 = 未启用） */
            health_check_path?: string | null;
            /** @description php | jar | war | node | python | go | static */
            kind: string;
            name: string;
            /** Format: int64 */
            port?: number | null;
            /** @description 启动命令（模板变量 $RUNTIME / $DEPLOY 预留，当前原样使用） */
            run_command?: string | null;
            /** Format: int64 */
            runtime_id?: number | null;
            /** @description local_path | git */
            source: string;
            source_url?: string | null;
        };
        CreateRequest: {
            path: string;
        };
        CreateSourceRequest: {
            /** @description 未配置公钥的源须显式确认（Doc/20 §4.2：无签名第三方源二次确认） */
            confirm_unsigned?: boolean;
            name: string;
            /** @description Ed25519 公钥（可选，验签用） */
            public_key?: string | null;
            /** @description git | http */
            source_type: string;
            url: string;
        };
        CreateTaskRequest: {
            command: string;
            enabled?: boolean | null;
            name: string;
            schedule?: string | null;
        };
        CreateUserRequest: {
            password_hash: string;
            role: string;
            username: string;
        };
        CreateVolumeRequest: {
            driver?: string;
            name: string;
        };
        CreateWafRuleRequest: {
            /** @description 命中动作：log（默认，仅告警）| block */
            action?: string;
            /** @description 类型相关参数 JSON（cc：`{"threshold":120,"window_secs":60}`） */
            config_json?: string;
            enabled?: boolean;
            name: string;
            pattern?: string;
            /**
             * @description 规则类型：cc | sql_injection | xss | ip_blacklist | ip_whitelist
             *     | ua_blacklist | ua_whitelist | path_block
             */
            rule_type: string;
            /**
             * Format: int64
             * @description 作用网站 ID（0 = 全局规则）
             */
            website_id?: number;
        };
        CreateWebServerInstanceRequest: {
            /** @description 缺省按发行版取默认；只接受该引擎的已知二进制（裸程序名或系统 bin 目录下同名文件） */
            binary_path?: string | null;
            /** @description 缺省取引擎默认主配置；必须位于 Web 引擎配置根之下（`/etc/nginx`、`/etc/apache2`、`/etc/httpd`、`/etc/caddy`、`/usr/local/lsws/conf`、`/usr/local/openresty/nginx/conf`） */
            config_path?: string | null;
            /** @description `nginx` / `apache` / `openlitespeed` / `openresty` / `caddy` */
            engine: string;
            /** Format: int32 */
            port?: number | null;
            version?: string | null;
        };
        CreateWebsiteRequest: {
            website: components["schemas"]["Website"];
        };
        /** @description 创建数据库用户（schema 名加前缀，避免与面板用户 `CreateUserRequest` 冲突） */
        DatabaseCreateUserRequest: {
            host?: string | null;
            password: string;
            username: string;
        };
        /** @description 数据库实例（面板登记；root 口令不回显） */
        DatabaseInstanceResponse: {
            config_file: string;
            created_at: string;
            data_dir: string;
            db_type: string;
            /** Format: int64 */
            id: number;
            install_path: string;
            name: string;
            /** Format: int32 */
            port: number;
            /** Format: int64 */
            resource_version: number;
            root_user: string;
            status: string;
            updated_at: string;
            version: string;
        };
        DeleteObjectQuery: {
            key: string;
        };
        DnsConfig: {
            nameservers: string[];
            options: string[];
            search: string[];
        };
        /** @description 厂商账号响应（credentials_json 不回显，防止敏感凭据泄漏） */
        DnsProviderAccountDto: {
            /** Format: date-time */
            created_at: string;
            enabled: boolean;
            /** Format: int64 */
            id: number;
            /** Format: date-time */
            last_test_at?: string | null;
            last_test_status?: string | null;
            name: string;
            provider_type: string;
            /** @description 最近一次探测失败原因（成功为 null；仅 test 端点返回） */
            test_message?: string | null;
            /** Format: date-time */
            updated_at: string;
        };
        DockerContainer: {
            /** Format: date-time */
            created_at: string;
            id: string;
            image: string;
            name: string;
            /** Format: int64 */
            node_id: number;
            status: string;
        };
        /** @description 重命名容器（schema 加前缀，避免与文件管理 `RenameRequest` 同名覆盖） */
        DockerRenameRequest: {
            new_name: string;
        };
        EngineInfo: {
            /** Format: int32 */
            default_port: number;
            /** Format: int32 */
            default_ssl_port: number;
            description: string;
            name: string;
            supports_load_balancing: boolean;
            supports_reverse_proxy: boolean;
            supports_rewrite: boolean;
            supports_ssl: boolean;
        };
        ExecutePluginRequest: {
            /** @description 兼容旧客户端，已不使用 */
            args?: number[] | null;
            /** @description 调用输入（插件经 `flame.input_read` 读取），≤ 1 MiB */
            input?: string | null;
        };
        ExecutionResponse: {
            /** Format: int64 */
            execution_ms: number;
            /** @description 插件经 `flame.log` 输出的日志（≤ 100 条） */
            logs: string[];
            output: number[];
            output_hex: string;
            output_string: string;
        };
        ExtractRequest: {
            /** @description 解压到的目录（缺省为压缩包所在目录） */
            dest?: string | null;
            /** @description 同名顶层条目存在时替换（旧条目移入回收站）；缺省 false → 409 且不做改动 */
            overwrite?: boolean;
            /** @description 压缩包路径（zip / tar / tar.gz / tgz / tar.xz / txz / gz） */
            path: string;
        };
        ExtractResult: {
            /**
             * Format: int64
             * @description 解压出的字节数
             */
            bytes: number;
            /** @description 解压出的条目数 */
            entries: number;
            /** @description 因同名被替换（旧条目已移入回收站）的顶层条目名 */
            replaced: string[];
            /** @description 跳过的特殊文件数 */
            skipped: number;
            /** @description 移入目标目录的顶层条目名 */
            top_level: string[];
        };
        FileInfo: {
            is_dir: boolean;
            /** @description 条目本身是符号链接（`size` / `is_dir` 为链接目标的属性） */
            is_symlink?: boolean;
            mime_type?: string | null;
            modified_at: string;
            name: string;
            /** @description 根内相对路径（以 `/` 开头），可直接回传给其他文件接口 */
            path: string;
            /** @description 八进制权限（`644` / `755`，不含文件类型位） */
            permissions: string;
            /** Format: int64 */
            size: number;
        };
        /** @description 单条体检结论。 */
        Finding: {
            category: components["schemas"]["Category"];
            /** @description 稳定编码（如 `cpu_vulnerability` / `sshd_root_login`），前端据此映射标题与建议 */
            code: string;
            /** @description 语言无关的原始取证（配置行、sysctl 值、文件模式等） */
            evidence: string;
            /** @description 依赖内置生命周期表等启发式数据的结论 */
            heuristic: boolean;
            severity: components["schemas"]["Severity"];
            status: components["schemas"]["CheckStatus"];
            /** @description 细分主体（如漏洞名、端口、账号名），同一 `code` 可有多条 */
            subject?: string | null;
        };
        /** @description 防火墙规则（面板侧定义；`POST /api/firewall/apply` 才下发到系统防火墙） */
        FirewallRuleResponse: {
            action: string;
            created_at: string;
            description?: string | null;
            destination?: string | null;
            direction: string;
            enabled: boolean;
            /** Format: int64 */
            id: number;
            name: string;
            port?: string | null;
            /** Format: int32 */
            priority: number;
            protocol: string;
            source?: string | null;
            updated_at: string;
        };
        /** @description 健康探测结果 DTO。 */
        HealthCheckDto: {
            /** @description 探测窗口内是否健康 */
            healthy: boolean;
            /** Format: int64 */
            project_id: number;
        };
        /** @description 依赖检查结果 */
        HealthCheckItem: {
            detail?: string | null;
            status: string;
        };
        HealthChecks: {
            database: components["schemas"]["HealthCheckItem"];
            disk: components["schemas"]["HealthCheckItem"];
            docker: components["schemas"]["HealthCheckItem"];
        };
        /** @description 详细健康检查响应 */
        HealthDetail: {
            checks: components["schemas"]["HealthChecks"];
            status: string;
            /** Format: int64 */
            uptime_secs: number;
            version: string;
        };
        /**
         * @description Agent 心跳。前 4 项为旧 Agent 即有的字段；其余为 batch-12 起的完整快照字段
         *     （缺省 0，旧 Agent 照常可用），并入 `MetricsHub` 统一模型后供仪表盘按节点展示。
         */
        HeartbeatRequest: {
            /** Format: int64 */
            cpu_cores?: number;
            /** Format: float */
            cpu_usage: number;
            /** Format: double */
            disk_total_gb?: number;
            /** Format: float */
            disk_usage_percent: number;
            /** Format: double */
            disk_used_gb?: number;
            /** Format: float */
            load_fifteen?: number;
            /** Format: float */
            load_five?: number;
            /** Format: float */
            load_one: number;
            /** Format: int64 */
            memory_total_mb?: number;
            /** Format: float */
            memory_usage_percent: number;
            /** Format: int64 */
            memory_used_mb?: number;
            /**
             * Format: double
             * @description 网络接收速率（MB/s，Agent 按两次心跳间的字节差 / 实际间隔计算）
             */
            network_rx_mbps?: number;
            /**
             * Format: double
             * @description 网络发送速率（MB/s）
             */
            network_tx_mbps?: number;
        };
        HostCpu: {
            /** @description 1 / 5 / 15 分钟负载 */
            load_average: number[];
            logical_cores: number;
            model: string;
            physical_cores?: number | null;
        };
        /** @description `GET /api/toolbox/host` 响应。 */
        HostDetails: {
            cpu: components["schemas"]["HostCpu"];
            disks: components["schemas"]["HostDisk"][];
            dns: components["schemas"]["DnsConfig"];
            /** Format: date-time */
            generated_at: string;
            hosts: components["schemas"]["HostsEntry"][];
            /** @description hosts 条目超过上限被截断 */
            hosts_truncated: boolean;
            interfaces: components["schemas"]["HostInterface"][];
            memory: components["schemas"]["HostMemory"];
            panel: components["schemas"]["PanelRuntime"];
            system: components["schemas"]["HostSystem"];
            time: components["schemas"]["HostTime"];
        };
        HostDisk: {
            /** Format: int64 */
            available_bytes: number;
            device: string;
            fs_type: string;
            mount_point: string;
            /** Format: int64 */
            total_bytes: number;
        };
        HostInterface: {
            /** @description `addr/prefix`，IPv4 在前 */
            addresses: string[];
            /** @description 全零 MAC（如 `lo`）为 null */
            mac?: string | null;
            name: string;
            /** Format: int64 */
            rx_bytes: number;
            /** Format: int64 */
            tx_bytes: number;
            /** @description 回环 / 容器虚拟网卡（`lo` / `veth*` / `docker*` / `br-*` / `cni*` / `flannel*` …） */
            virtual_iface: boolean;
        };
        HostMemory: {
            /** Format: int64 */
            available_bytes: number;
            /** Format: int64 */
            swap_total_bytes: number;
            /** Format: int64 */
            swap_used_bytes: number;
            /** Format: int64 */
            total_bytes: number;
        };
        HostSystem: {
            arch: string;
            /**
             * Format: date-time
             * @description 开机时刻（不可探测为 null）
             */
            boot_time?: string | null;
            hostname: string;
            kernel_release: string;
            os_id: string;
            /** @description 发行版展示名（`PRETTY_NAME`） */
            os_name: string;
            os_version_id: string;
            /** Format: int64 */
            uptime_secs: number;
        };
        HostTime: {
            /**
             * Format: date-time
             * @description 服务器当前时间（用于和浏览器时间对比时钟偏差）
             */
            now: string;
            /** @description IANA 时区名（`Asia/Shanghai`），不可探测为 null */
            timezone?: string | null;
            /** @description 面板进程本地 UTC 偏移（`+08:00`） */
            utc_offset: string;
        };
        HostsEntry: {
            ip: string;
            names: string[];
        };
        IacApplication: {
            mode: string;
            name: string;
            package_key: string;
            /** Format: int32 */
            port?: number | null;
            version: string;
        };
        IacDatabase: {
            db_type: string;
            name: string;
            /** Format: int32 */
            port: number;
            version: string;
        };
        /** @description 配置清单而非完整备份 / 可执行部署声明；不包含密码、参数、原始配置或关联 ID。 */
        IacManifest: {
            api_version: string;
            applications?: components["schemas"]["IacApplication"][] | null;
            databases?: components["schemas"]["IacDatabase"][] | null;
            kind: string;
            websites?: components["schemas"]["IacWebsite"][] | null;
        };
        IacWebsite: {
            domain: string;
            engine: string;
            name: string;
            proxy_enabled: boolean;
            rewrite_preset?: string | null;
            root_path: string;
            site_type: string;
            ssl_enabled: boolean;
        };
        ImportPackageRequest: {
            path: string;
        };
        InitializeResponse: {
            message: string;
            refresh_token?: string | null;
            role?: string | null;
            status: string;
            token?: string | null;
            username?: string | null;
        };
        /** @description 一次体检报告。 */
        InspectionReport: {
            /**
             * Format: int64
             * @description 命中服务端缓存时为缓存年龄，刚生成为 0
             */
            cache_age_secs: number;
            /** @description 内置生命周期表版本 */
            data_version: string;
            /**
             * Format: int64
             * @description 采集 + 判定耗时
             */
            duration_ms: number;
            findings: components["schemas"]["Finding"][];
            /** Format: date-time */
            generated_at: string;
            kernel: components["schemas"]["KernelInfo"];
            summary: components["schemas"]["InspectionSummary"];
        };
        /** @description 汇总：评分 / 等级 / 各状态与严重度计数。 */
        InspectionSummary: {
            /**
             * Format: int32
             * @description 以下为 `fail` 项按严重度计数
             */
            critical: number;
            /** Format: int32 */
            failed: number;
            /** @description `A`（≥ 90）/ `B`（≥ 75）/ `C`（≥ 60）/ `D`（≥ 40）/ `F` */
            grade: string;
            /** Format: int32 */
            high: number;
            /** Format: int32 */
            low: number;
            /** Format: int32 */
            medium: number;
            /** Format: int32 */
            passed: number;
            /**
             * Format: int32
             * @description 0–100：100 减去所有 `fail` 项按严重度加权的扣分
             */
            score: number;
            /** Format: int32 */
            total: number;
            /** Format: int32 */
            unknown: number;
        };
        InstallAppRequest: {
            /** @description 安装包含原生脚本时需显式确认执行第三方脚本。 */
            acknowledge_scripts?: boolean;
            confirm_risky?: boolean;
            container_name?: string | null;
            mode?: string | null;
            name?: string | null;
            package_key: string;
            /** Format: int32 */
            port?: number | null;
            values?: {
                [key: string]: string;
            } | null;
            version?: string | null;
        };
        /** @description 批量安装配套请求：`{engine, capabilities: ["php", "cache", ...]}`。 */
        InstallCapabilitiesRequest: {
            capabilities: string[];
            engine: string;
        };
        /** @enum {string} */
        InstallMode: "container" | "native" | "wasm";
        /**
         * @description MongoDB 安装请求：`version` 取 `8.0`（缺省）/ `9.0`；填写 `password` 时创建 `admin`
         *     超级用户并开启鉴权（≥ 8 位），留空则仅本机（127.0.0.1）免鉴权访问。
         */
        InstallMongodbRequest: {
            name: string;
            password?: string | null;
            /** Format: int32 */
            port?: number | null;
            version?: string | null;
        };
        InstallMysqlRequest: {
            name: string;
            /** Format: int32 */
            port?: number | null;
            root_password?: string | null;
            version?: string | null;
        };
        InstallPostgresRequest: {
            name: string;
            password?: string | null;
            /** Format: int32 */
            port?: number | null;
            version?: string | null;
        };
        InstallRedisRequest: {
            name: string;
            password?: string | null;
            /** Format: int32 */
            port?: number | null;
            version?: string | null;
        };
        /** @description 创建运行时安装请求。 */
        InstallRuntimeRequest: {
            /** @description php | java | node | python | go */
            kind: string;
            /** @description 仅 Node：安装后同时执行 `npm install -g pm2`（其余 kind 传 true 返回 400） */
            pm2?: boolean | null;
            /**
             * @description 版本号（如 8.2 / 21 / 20）。Node 可写主版本（`22`）或次版本（`22.11`），
             *     安装时按镜像索引解析到最新补丁版（解析结果见 `metadata_json.version`）
             */
            version: string;
        };
        InstalledApp: {
            access_url?: string | null;
            container_name?: string | null;
            /** Format: date-time */
            created_at: string;
            /** Format: int64 */
            id: number;
            install_path: string;
            /**
             * Format: int64
             * @description 启动次数（常用应用排序，v0.7.0）
             */
            launch_count?: number;
            mode: string;
            name: string;
            package_key: string;
            params_json: string;
            /** Format: int32 */
            port?: number | null;
            status: string;
            /** Format: date-time */
            updated_at: string;
            version: string;
        };
        InstalledAppResponse: {
            app: components["schemas"]["InstalledApp"];
            logs?: string | null;
        };
        IssueAcmeRequest: {
            /** @description ACME 目录 URL（空 = Let's Encrypt 生产；staging 联调见常量） */
            directory_url?: string | null;
            /** @description 是否 DNS-01（泛域名必须） */
            dns01?: boolean;
            /**
             * Format: int64
             * @description DNS-01 厂商账号 ID（Stage 5：按账号解析真实厂商凭据；泛域名必填）
             */
            dns_provider_account_id?: number | null;
            /** @description SAN 域名列表（主域名 + 备用/泛域名） */
            domains: string[];
            /** @description HTTP-01 webroot（网站 root_path；DNS-01 时忽略） */
            webroot?: string | null;
            /**
             * Format: int64
             * @description 签发后绑定网站 ID（可选）
             */
            website_id?: number | null;
        };
        IssueSelfSignedRequest: {
            domains: string[];
            /** Format: int64 */
            website_id?: number | null;
        };
        /** @description 备份任务响应 */
        JobDto: {
            /** Format: date-time */
            created_at: string;
            enabled: boolean;
            /** Format: int64 */
            id: number;
            last_message?: string | null;
            last_remote_key?: string | null;
            /** Format: date-time */
            last_run_at?: string | null;
            /** @description never | success | failed */
            last_status: string;
            name: string;
            /** Format: date-time */
            next_run_at?: string | null;
            /** Format: int64 */
            retention: number;
            schedule: string;
            scope: string;
            scope_key?: string | null;
            /** Format: int64 */
            target_account_id: number;
            /** Format: date-time */
            updated_at: string;
        };
        /** @description 内核与主机基础信息。 */
        KernelInfo: {
            /** @description 面板进程架构（`x86_64` / `aarch64`） */
            arch: string;
            hostname: string;
            /** @description 是否为内置表中的 LTS 系列（不在表中为 null） */
            lts?: boolean | null;
            /** @description 已安装的最新内核版本（`/boot/vmlinuz-*`），与 `release` 不一致即待重启生效 */
            newest_installed_kernel?: string | null;
            /** @description 发行版 `ID`（`ubuntu` / `debian` / `rocky` …） */
            os_id: string;
            /** @description 发行版展示名（`PRETTY_NAME`） */
            os_name: string;
            /** @description 发行版 `VERSION_ID` */
            os_version_id: string;
            /** @description 存在 `/var/run/reboot-required` 或已安装更新内核但仍运行旧内核 */
            reboot_required: boolean;
            /** @description `uname -r`（`/proc/sys/kernel/osrelease`） */
            release: string;
            /** @description 内核主线系列（`6.1`） */
            series?: string | null;
            /** @description 系列上游 EOL 日期（`YYYY-MM-DD`，内置表） */
            series_eol?: string | null;
            /** Format: int64 */
            uptime_secs: number;
            /** @description `/proc/version` 全文（编译器 / 构建时间） */
            version: string;
            /** @description 虚拟化类型（`systemd-detect-virt`：`kvm` / `docker` / `none` …），不可探测为 null */
            virtualization?: string | null;
        };
        /** @description 解析后的一条内核日志。 */
        KernelLogEntry: {
            /** @description 其余结构化字段（事件字段 + span 链），前端行展开展示 */
            fields: unknown;
            level: components["schemas"]["LogLevel"];
            message: string;
            /** @description 所在请求的编号（取自 span 链上的 `request_id`，与响应头 `X-Request-Id` 一致） */
            request_id?: string | null;
            /** @description 事件来源模块（如 `flame_kernel::api::middleware`） */
            target: string;
            /** @description RFC 3339 时间戳（原样保留，非 JSON 行为空串） */
            timestamp: string;
        };
        KernelLogLevelResponse: {
            level: components["schemas"]["LogLevel"];
        };
        /** @description 概览：是否启用、目录、级别、保留期、文件列表、丢弃计数。 */
        KernelLogOverview: {
            dir?: string | null;
            /**
             * Format: int64
             * @description 因写入队列满而丢弃的条数（持续增长说明日志量过大，应调高级别）
             */
            dropped_lines: number;
            /** @description 文件日志是否启用（`OP_LOG_FILE=false` 或目录不可写时为 false） */
            enabled: boolean;
            files: components["schemas"]["LogFileInfo"][];
            /** @description 当前生效级别 */
            level: components["schemas"]["LogLevel"];
            /** Format: int32 */
            retention_days: number;
        };
        KernelLogStats: {
            counts: components["schemas"]["LevelCounts"];
            file: string;
            /** @description 文件超过统计扫描上限（256 MB）时为 true，计数只覆盖前 256 MB */
            truncated: boolean;
        };
        /** @description 结束进程请求体（字段均可省略，`{}` 即发送 TERM） */
        KillProcessRequest: {
            /** @description 信号：`TERM`（缺省）/ `KILL` / `HUP` / `INT` */
            signal?: components["schemas"]["ProcessSignal"];
            /**
             * Format: int64
             * @description 列表中看到的启动时间（Unix 秒）；与当前进程不一致时返回 409，防止误杀复用了 pid 的新进程
             */
            started_at?: number | null;
        };
        /** @description `POST /api/toolbox/processes/{pid}/kill` 响应。 */
        KillProcessResponse: {
            /** @description 是否经 `sudo -n` 提权发送 */
            elevated: boolean;
            name: string;
            /** Format: int32 */
            pid: number;
            signal: components["schemas"]["ProcessSignal"];
        };
        /** @description 按级别计数（五级 + 合计），供前端筛选按钮上的数字。 */
        LevelCounts: {
            /** Format: int64 */
            debug: number;
            /** Format: int64 */
            error: number;
            /** Format: int64 */
            info: number;
            /** Format: int64 */
            total: number;
            /** Format: int64 */
            trace: number;
            /** Format: int64 */
            warn: number;
        };
        /** @description 关联数据库请求。 */
        LinkDatabaseRequest: {
            /**
             * Format: int64
             * @description 数据库实例 ID（/api/databases 返回的实例）
             */
            db_instance_id: number;
            /** @description 业务库名（留空取项目名；redis 忽略） */
            db_name?: string | null;
            /** @description 注入的环境变量名（留空按 db_type 推导默认值） */
            env_name?: string | null;
        };
        /** @description 可关联的数据库实例（联链弹窗数据源）。 */
        LinkableDatabaseDto: {
            db_type: string;
            /** @description 推荐注入的环境变量名（DATABASE_URL / MYSQL_URL / REDIS_URL） */
            default_env_name: string;
            /** Format: int64 */
            id: number;
            name: string;
            /** Format: int32 */
            port: number;
            status: string;
        };
        /**
         * @description Provider 协议类型。
         * @enum {string}
         */
        LlmProviderKind: "openai" | "ollama";
        /** @description 加载插件（WASM 以 base64 提交；请求体受全局 2 MiB 上限约束） */
        LoadPluginRequest: {
            author?: string | null;
            /**
             * @description 授予的宿主能力（Doc/21 §5.7）：`fs:read:<dir>` / `fs:write:<dir>` / `metrics:read` /
             *     `settings:write`；缺省 = 无能力
             */
            capabilities?: string[] | null;
            description?: string | null;
            homepage?: string | null;
            /** @description 1–64 位 `[A-Za-z0-9._-]`，不以 `.` 开头 */
            id: string;
            license?: string | null;
            /** @description 1 MiB – 256 MiB，缺省 64 MiB */
            memory_limit_bytes?: number | null;
            name?: string | null;
            tags?: string[] | null;
            /**
             * Format: int64
             * @description 1 – 300000 ms，缺省 30000
             */
            timeout_ms?: number | null;
            version?: string | null;
            wasm_base64: string;
        };
        LogEntry: {
            /** Format: date-time */
            created_at: string;
            /** Format: int64 */
            id: number;
            level: string;
            message: string;
            metadata?: string | null;
            source: string;
        };
        LogFileInfo: {
            /** @description 是否为当前正在写入的文件 */
            current: boolean;
            /** @description 最后修改时间（RFC 3339） */
            modified: string;
            name: string;
            /** Format: int64 */
            size: number;
        };
        /**
         * @description 五级日志级别（与 `tracing::Level` 一一对应，按严重程度升序）。
         * @enum {string}
         */
        LogLevel: "trace" | "debug" | "info" | "warn" | "error";
        /**
         * @description 单条进程日志（原生模式：`journalctl -n` 或内建环形缓冲；
         *     容器模式：`docker logs --tail` 文本行，时间戳为 None）。
         */
        LogLine: {
            /** @description 日志内容（stdout/stderr 合并流） */
            message: string;
            /** @description 时间戳（RFC3339；内建守护无法取得时间时为空） */
            timestamp?: string | null;
        };
        LogPage: {
            /** @description 命中的日志（最新在前） */
            entries: components["schemas"]["KernelLogEntry"][];
            file: string;
            /**
             * Format: int64
             * @description 文件当前大小（前端追踪模式据此判断是否有新内容）
             */
            file_size: number;
            /**
             * Format: int64
             * @description 续读游标：传回 `before` 读取更早的日志；`None` 表示已到文件开头
             */
            next_before?: number | null;
            /**
             * Format: int64
             * @description 本次扫描的字节数
             */
            scanned_bytes: number;
        };
        LoginRequest: {
            password: string;
            username: string;
        };
        LoginResponse: {
            /** @description 两步验证挑战令牌（5 分钟有效；仅 `two_factor_required=true` 时存在） */
            challenge?: string | null;
            /** @description 是否需强制修改密码（新装面板首次登录） */
            must_change_password: boolean;
            /** @description 面板强制两步验证而该用户尚未绑定：登录后须先完成绑定 */
            must_setup_two_factor?: boolean;
            /** @description Refresh Token（长过期，默认 24 小时；用于换取新 Access Token） */
            refresh_token: string;
            role: string;
            /** @description Access Token（短过期，默认 15 分钟） */
            token: string;
            /** @description 密码已通过但需第二因子：此时 `token` 为空，凭 `challenge` 调用 `POST /api/auth/login/2fa` */
            two_factor_required?: boolean;
            username: string;
        };
        /** @description 登录第二步请求 */
        LoginTwoFactorRequest: {
            /** @description 第一步返回的挑战令牌 */
            challenge: string;
            /** @description 6 位 TOTP 验证码或一次性恢复码 */
            code: string;
        };
        MarketInstallRequest: {
            /** @description 插件申请了宿主能力时必须为 true（管理员已查看并同意能力清单） */
            accept_capabilities?: boolean;
        };
        MarketListResponse: {
            plugins: components["schemas"]["MarketPluginView"][];
            /** @description 远程索引拉取失败原因（内置目录仍可用） */
            remote_error?: string | null;
            /** @description 已配置的远程索引地址（设置项 `plugin_market_url`） */
            remote_url?: string | null;
        };
        /** @description 市场条目（列表展示 + 安装依据）。 */
        MarketPlugin: {
            author?: string;
            /** @description 插件申请的宿主能力（安装时须管理员同意） */
            capabilities?: string[];
            description?: string;
            /** @description 推荐调用的导出函数 */
            entry?: string;
            homepage?: string | null;
            id: string;
            license?: string | null;
            name: string;
            source?: components["schemas"]["MarketSource"];
            tags?: string[];
            version: string;
            /** @description 包体 SHA-256（远程条目必填；内置条目为编译后的哈希） */
            wasm_sha256?: string;
            /** @description 远程包体地址（内置条目为空） */
            wasm_url?: string | null;
        };
        MarketPluginView: components["schemas"]["MarketPlugin"] & {
            /** @description 已安装（同 ID 插件已注册） */
            installed: boolean;
            installed_version?: string | null;
        };
        /** @enum {string} */
        MarketSource: "builtin" | "remote";
        /** @description 运维技能参数。 */
        McpPromptArgument: {
            description: string;
            name: string;
            required: boolean;
        };
        /** @description 运维技能（Runbook 提示词）。 */
        McpPromptInfo: {
            arguments: components["schemas"]["McpPromptArgument"][];
            description: string;
            name: string;
            title: string;
        };
        /** @description 渲染后的技能提示词。 */
        McpPromptRendered: {
            name: string;
            text: string;
            title: string;
        };
        /** @description 当前登录用户 */
        MeResponse: {
            /** Format: int64 */
            id: number;
            /** @description 是否需强制修改密码 */
            must_change_password: boolean;
            role: string;
            username: string;
        };
        Memo: {
            content: string;
            /** Format: date-time */
            created_at: string;
            done: boolean;
            /** Format: int64 */
            id: number;
            /** @description memo | todo */
            kind: string;
            /** Format: date-time */
            updated_at: string;
        };
        /** @description 单处冲突：路径 + 三方取值（YAML 片段，过长截断；`None` = 该键不存在）。 */
        MergeConflict: {
            base?: string | null;
            local?: string | null;
            path: string;
            upstream?: string | null;
        };
        /** @description 监控历史降采样点（S-07）。一个桶内 `samples` 条 3 s 快照的均值 / 峰值。 */
        MetricsHistoryPoint: {
            /**
             * Format: int64
             * @description 桶起点（Unix 秒）
             */
            bucket_start: number;
            /** Format: float */
            cpu_avg: number;
            /** Format: float */
            cpu_max: number;
            /** Format: float */
            disk_avg: number;
            /** Format: double */
            load_one_avg: number;
            /** Format: float */
            memory_avg: number;
            /** Format: float */
            memory_max: number;
            /**
             * Format: double
             * @description 网络接收速率均值（MB/s）
             */
            net_rx_avg_mbps: number;
            /**
             * Format: double
             * @description 网络发送速率均值（MB/s）
             */
            net_tx_avg_mbps: number;
            /**
             * Format: int32
             * @description 桶内样本数
             */
            samples: number;
        };
        /** @description `GET /api/metrics/history` 响应。 */
        MetricsHistoryResponse: {
            /**
             * Format: int64
             * @description 窗口起点（Unix 秒，含）
             */
            from: number;
            /** @description 按时间升序的降采样点；刚启动或尚未写满时可能少于窗口理论点数 */
            points: components["schemas"]["MetricsHistoryPoint"][];
            /** @description 请求的范围（`1h` / `6h` / `24h` / `7d` / `30d`） */
            range: string;
            /**
             * Format: int64
             * @description 每个点的桶宽（秒）：60 或 3600
             */
            step_secs: number;
            /**
             * Format: int64
             * @description 窗口终点（Unix 秒，不含）
             */
            to: number;
        };
        MetricsSnapshot: {
            cpu_cores: number;
            /** Format: float */
            cpu_usage: number;
            /** Format: double */
            disk_total_gb: number;
            /** Format: float */
            disk_usage_percent: number;
            /** Format: double */
            disk_used_gb: number;
            /** Format: double */
            load_fifteen: number;
            /** Format: double */
            load_five: number;
            /** Format: double */
            load_one: number;
            /** Format: int64 */
            memory_total_mb: number;
            /** Format: float */
            memory_usage_percent: number;
            /** Format: int64 */
            memory_used_mb: number;
            /**
             * Format: double
             * @description 网络接收速率（MB/s）
             */
            network_rx_mbps: number;
            /**
             * Format: double
             * @description 网络发送速率（MB/s）
             */
            network_tx_mbps: number;
            /** Format: int64 */
            timestamp: number;
        };
        NativeAutostartRequest: {
            enabled: boolean;
            engine: string;
        };
        NativeInstallRequest: {
            engine: string;
            version?: string | null;
        };
        NativeUninstallRequest: {
            engine: string;
        };
        /** @description 原生 Web 服务器检测信息（1Panel 风格：安装状态 / 版本 / 服务 / 端口） */
        NativeWebServerInfo: {
            binary_path?: string | null;
            config_path: string;
            /** Format: int32 */
            default_port: number;
            description: string;
            /** @description systemd 是否开机自启 */
            enabled: boolean;
            engine: string;
            /** @description 二进制是否存在于 PATH（dpkg/rpm/apk 或 which） */
            installed: boolean;
            /** @description 当前监听中的端口（ss/netstat 检测） */
            listening_ports: number[];
            /** @description 发行版包是否已安装（nginx/httpd 等包名） */
            package_installed: boolean;
            /** @description systemd 服务是否运行中 */
            running: boolean;
            service_name?: string | null;
            version?: string | null;
        };
        /** @description 节点徽标：在线 / 总数（`status == "online"` 与节点列表页口径一致）。 */
        NodeBadges: {
            /** Format: int64 */
            online: number;
            /** Format: int64 */
            total: number;
        };
        OperationLog: {
            action: string;
            /** Format: date-time */
            created_at: string;
            /** Format: int64 */
            id: number;
            ip?: string | null;
            target?: string | null;
            username: string;
        };
        /** @description 事件落库（Outbox）记录：每条领域事件持久化存档，保证审计不丢（Stage6）。 */
        OutboxEvent: {
            /** Format: date-time */
            created_at: string;
            /** @description 事件类型名（如 `AppInstalled` / `UserLoggedIn`） */
            event_type: string;
            /** Format: int64 */
            id: number;
            /** @description 事件载荷（JSON 序列化的结构化字段） */
            payload: string;
            /** @description 是否已送达通知渠道（预留；当前仅持久化审计） */
            published: boolean;
        };
        /** @description 事件落库（Outbox）查询参数。 */
        OutboxListQuery: {
            /** Format: int64 */
            page?: number | null;
            /** Format: int64 */
            page_size?: number | null;
            /** @description 按事件类型过滤（如 `type=AppInstalled` 精确匹配变体名） */
            type?: string | null;
        };
        /** @description `GET /api/overview/badges` 响应：顶栏一次拉取的全部计数。 */
        OverviewBadges: {
            alerts?: null | components["schemas"]["AlertBadges"];
            /**
             * Format: int64
             * @description 快照年龄（秒）：> 0 表示命中服务端缓存
             */
            cache_age_secs: number;
            containers?: null | components["schemas"]["ContainerBadges"];
            /**
             * Format: date-time
             * @description 本快照的聚合时间
             */
            generated_at: string;
            nodes?: null | components["schemas"]["NodeBadges"];
            tasks?: null | components["schemas"]["TaskBadges"];
        };
        "PaginatedResponse<DatabaseInstanceResponse>": {
            data: {
                config_file: string;
                created_at: string;
                data_dir: string;
                db_type: string;
                /** Format: int64 */
                id: number;
                install_path: string;
                name: string;
                /** Format: int32 */
                port: number;
                /** Format: int64 */
                resource_version: number;
                root_user: string;
                status: string;
                updated_at: string;
                version: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<DatabaseInstanceResponse>_DatabaseInstanceResponse": {
            data: {
                config_file: string;
                created_at: string;
                data_dir: string;
                db_type: string;
                /** Format: int64 */
                id: number;
                install_path: string;
                name: string;
                /** Format: int32 */
                port: number;
                /** Format: int64 */
                resource_version: number;
                root_user: string;
                status: string;
                updated_at: string;
                version: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<FirewallRuleResponse>": {
            data: {
                action: string;
                created_at: string;
                description?: string | null;
                destination?: string | null;
                direction: string;
                enabled: boolean;
                /** Format: int64 */
                id: number;
                name: string;
                port?: string | null;
                /** Format: int32 */
                priority: number;
                protocol: string;
                source?: string | null;
                updated_at: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<FirewallRuleResponse>_FirewallRuleResponse": {
            data: {
                action: string;
                created_at: string;
                description?: string | null;
                destination?: string | null;
                direction: string;
                enabled: boolean;
                /** Format: int64 */
                id: number;
                name: string;
                port?: string | null;
                /** Format: int32 */
                priority: number;
                protocol: string;
                source?: string | null;
                updated_at: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<LogEntry>": {
            data: {
                /** Format: date-time */
                created_at: string;
                /** Format: int64 */
                id: number;
                level: string;
                message: string;
                metadata?: string | null;
                source: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<LogEntry>_LogEntry": {
            data: {
                /** Format: date-time */
                created_at: string;
                /** Format: int64 */
                id: number;
                level: string;
                message: string;
                metadata?: string | null;
                source: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<OperationLog>": {
            data: {
                action: string;
                /** Format: date-time */
                created_at: string;
                /** Format: int64 */
                id: number;
                ip?: string | null;
                target?: string | null;
                username: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<OperationLog>_OperationLog": {
            data: {
                action: string;
                /** Format: date-time */
                created_at: string;
                /** Format: int64 */
                id: number;
                ip?: string | null;
                target?: string | null;
                username: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<OutboxEvent>": {
            data: {
                /** Format: date-time */
                created_at: string;
                /** @description 事件类型名（如 `AppInstalled` / `UserLoggedIn`） */
                event_type: string;
                /** Format: int64 */
                id: number;
                /** @description 事件载荷（JSON 序列化的结构化字段） */
                payload: string;
                /** @description 是否已送达通知渠道（预留；当前仅持久化审计） */
                published: boolean;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<OutboxEvent>_OutboxEvent": {
            data: {
                /** Format: date-time */
                created_at: string;
                /** @description 事件类型名（如 `AppInstalled` / `UserLoggedIn`） */
                event_type: string;
                /** Format: int64 */
                id: number;
                /** @description 事件载荷（JSON 序列化的结构化字段） */
                payload: string;
                /** @description 是否已送达通知渠道（预留；当前仅持久化审计） */
                published: boolean;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<ScheduledTask>": {
            data: {
                command: string;
                /** Format: date-time */
                created_at: string;
                enabled: boolean;
                /** Format: int64 */
                id: number;
                last_output: string;
                /** Format: date-time */
                last_run_at?: string | null;
                /** @description never | success | failed */
                last_status: string;
                name: string;
                /** Format: date-time */
                next_run_at?: string | null;
                /** @description 标准 5 字段 cron 表达式 */
                schedule: string;
                /** Format: date-time */
                updated_at: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<ScheduledTask>_ScheduledTask": {
            data: {
                command: string;
                /** Format: date-time */
                created_at: string;
                enabled: boolean;
                /** Format: int64 */
                id: number;
                last_output: string;
                /** Format: date-time */
                last_run_at?: string | null;
                /** @description never | success | failed */
                last_status: string;
                name: string;
                /** Format: date-time */
                next_run_at?: string | null;
                /** @description 标准 5 字段 cron 表达式 */
                schedule: string;
                /** Format: date-time */
                updated_at: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<ServerNode>": {
            data: {
                /**
                 * Format: int32
                 * @description Agent HTTP 服务端口（面板据此构造 `http://<ip>:<agent_port>` 发起远程调用）
                 */
                agent_port?: number;
                /** @description Agent 注册时携带的认证令牌 */
                auth_token?: string | null;
                /** Format: date-time */
                created_at: string;
                hostname: string;
                /** Format: int64 */
                id: number;
                ip_address: string;
                /**
                 * Format: date-time
                 * @description 最近一次心跳时间（Agent 上报）
                 */
                last_heartbeat_at?: string | null;
                /** @description 最近指标快照 JSON：{cpu_usage, memory_usage_percent, disk_usage_percent, load_one} */
                metrics_json?: string | null;
                name: string;
                status: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<ServerNode>_ServerNode": {
            data: {
                /**
                 * Format: int32
                 * @description Agent HTTP 服务端口（面板据此构造 `http://<ip>:<agent_port>` 发起远程调用）
                 */
                agent_port?: number;
                /** @description Agent 注册时携带的认证令牌 */
                auth_token?: string | null;
                /** Format: date-time */
                created_at: string;
                hostname: string;
                /** Format: int64 */
                id: number;
                ip_address: string;
                /**
                 * Format: date-time
                 * @description 最近一次心跳时间（Agent 上报）
                 */
                last_heartbeat_at?: string | null;
                /** @description 最近指标快照 JSON：{cpu_usage, memory_usage_percent, disk_usage_percent, load_one} */
                metrics_json?: string | null;
                name: string;
                status: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<SettingEntry>": {
            data: {
                description: string;
                key: string;
                value: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<SettingEntry>_SettingEntry": {
            data: {
                description: string;
                key: string;
                value: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<ToolApproval>": {
            data: {
                /** @description 调用参数（JSON 文本，已脱敏） */
                arguments: string;
                /** Format: date-time */
                decided_at?: string | null;
                /**
                 * Format: int64
                 * @description 批准 / 拒绝者用户 ID
                 */
                decided_by?: number | null;
                /**
                 * Format: date-time
                 * @description 超过该时间仍未处理即过期
                 */
                expires_at: string;
                /** Format: date-time */
                finished_at?: string | null;
                /** Format: int64 */
                id: number;
                /** @description 执行前预览（dry-run）：将要发生的变更与当前状态 */
                preview: string;
                /** Format: int64 */
                principal_id: number;
                /** @description 拒绝理由（可空） */
                reason: string;
                /** Format: date-time */
                requested_at: string;
                /** @description 执行输出或失败原因（截断到 8 KiB） */
                result: string;
                risk: components["schemas"]["RiskLevel"];
                session_id?: string | null;
                status: components["schemas"]["ApprovalStatus"];
                tool: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<ToolApproval>_ToolApproval": {
            data: {
                /** @description 调用参数（JSON 文本，已脱敏） */
                arguments: string;
                /** Format: date-time */
                decided_at?: string | null;
                /**
                 * Format: int64
                 * @description 批准 / 拒绝者用户 ID
                 */
                decided_by?: number | null;
                /**
                 * Format: date-time
                 * @description 超过该时间仍未处理即过期
                 */
                expires_at: string;
                /** Format: date-time */
                finished_at?: string | null;
                /** Format: int64 */
                id: number;
                /** @description 执行前预览（dry-run）：将要发生的变更与当前状态 */
                preview: string;
                /** Format: int64 */
                principal_id: number;
                /** @description 拒绝理由（可空） */
                reason: string;
                /** Format: date-time */
                requested_at: string;
                /** @description 执行输出或失败原因（截断到 8 KiB） */
                result: string;
                risk: components["schemas"]["RiskLevel"];
                session_id?: string | null;
                status: components["schemas"]["ApprovalStatus"];
                tool: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<ToolCallRecord>": {
            data: {
                /** @description 调用参数（JSON 文本，已脱敏并截断到 2 KiB） */
                arguments: string;
                /** Format: date-time */
                created_at: string;
                /** Format: int64 */
                duration_ms: number;
                /** @description 失败原因（成功时为空） */
                error: string;
                /** Format: int64 */
                id: number;
                ok: boolean;
                /** Format: int64 */
                principal_id: number;
                session_id?: string | null;
                tool: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<ToolCallRecord>_ToolCallRecord": {
            data: {
                /** @description 调用参数（JSON 文本，已脱敏并截断到 2 KiB） */
                arguments: string;
                /** Format: date-time */
                created_at: string;
                /** Format: int64 */
                duration_ms: number;
                /** @description 失败原因（成功时为空） */
                error: string;
                /** Format: int64 */
                id: number;
                ok: boolean;
                /** Format: int64 */
                principal_id: number;
                session_id?: string | null;
                tool: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<User>": {
            data: {
                /** Format: date-time */
                created_at: string;
                /** Format: int64 */
                id: number;
                /** @description 是否强制修改密码（新装面板的种子 admin 为 true） */
                must_change_password: boolean;
                password_hash: string;
                role: string;
                username: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<User>_User": {
            data: {
                /** Format: date-time */
                created_at: string;
                /** Format: int64 */
                id: number;
                /** @description 是否强制修改密码（新装面板的种子 admin 为 true） */
                must_change_password: boolean;
                password_hash: string;
                role: string;
                username: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<WafLog>": {
            data: {
                /** @description 命中动作快照：log | block */
                action: string;
                /** @description 来源 IP */
                client_ip: string;
                /**
                 * Format: date-time
                 * @description 命中时间
                 */
                created_at: string;
                /** Format: int64 */
                id: number;
                /** @description 请求方法 + 路径摘要（如 `GET /login?id=1' OR`；超长截断） */
                request_summary: string;
                /**
                 * Format: int64
                 * @description 命中的规则 ID（规则删除后保留历史日志，页面显示规则名快照）
                 */
                rule_id: number;
                /** @description 规则名称快照（规则删除后仍可读） */
                rule_name: string;
                /** @description 规则类型快照 */
                rule_type: string;
                /** @description 命中的 UA（超长截断） */
                user_agent: string;
                /**
                 * Format: int64
                 * @description 命中的网站 ID（0 = 全局规则命中）
                 */
                website_id: number;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<WafLog>_WafLog": {
            data: {
                /** @description 命中动作快照：log | block */
                action: string;
                /** @description 来源 IP */
                client_ip: string;
                /**
                 * Format: date-time
                 * @description 命中时间
                 */
                created_at: string;
                /** Format: int64 */
                id: number;
                /** @description 请求方法 + 路径摘要（如 `GET /login?id=1' OR`；超长截断） */
                request_summary: string;
                /**
                 * Format: int64
                 * @description 命中的规则 ID（规则删除后保留历史日志，页面显示规则名快照）
                 */
                rule_id: number;
                /** @description 规则名称快照（规则删除后仍可读） */
                rule_name: string;
                /** @description 规则类型快照 */
                rule_type: string;
                /** @description 命中的 UA（超长截断） */
                user_agent: string;
                /**
                 * Format: int64
                 * @description 命中的网站 ID（0 = 全局规则命中）
                 */
                website_id: number;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<WebServerResponse>": {
            data: {
                binary_path?: string | null;
                config_path: string;
                created_at: string;
                engine: string;
                /** Format: int64 */
                id: number;
                /** Format: int32 */
                port: number;
                /** Format: int64 */
                resource_version: number;
                status: string;
                version?: string | null;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<WebServerResponse>_WebServerResponse": {
            data: {
                binary_path?: string | null;
                config_path: string;
                created_at: string;
                engine: string;
                /** Format: int64 */
                id: number;
                /** Format: int32 */
                port: number;
                /** Format: int64 */
                resource_version: number;
                status: string;
                version?: string | null;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<Website>": {
            data: {
                /** Format: date-time */
                created_at: string;
                domain: string;
                engine: string;
                /** Format: int64 */
                id: number;
                name: string;
                /** Format: int64 */
                node_id: number;
                /**
                 * Format: int64
                 * @description 绑定的项目 ID（site_type = reverse_proxy + 项目场景）
                 */
                project_id?: number | null;
                proxy_enabled: boolean;
                proxy_pass?: string | null;
                /**
                 * Format: int64
                 * @description 乐观并发控制版本号：写入时需匹配当前值，冲突返回稳定 `CONFLICT`。
                 */
                resource_version?: number;
                /**
                 * @description 已套用的伪静态模板名（Doc/20 §5.5 伪静态规则库；None = 未套用）。
                 *     生成站点配置时按该名解析模板并注入 vhost（套用即生效）。
                 */
                rewrite_preset?: string | null;
                root_path: string;
                /**
                 * Format: int64
                 * @description 绑定的运行时 ID（site_type = runtime_php 场景）
                 */
                runtime_id?: number | null;
                /** @description 站点类型：runtime_php | reverse_proxy | static | container_app（M-B 主线二 Stage A-6 · Doc/20 §5.5） */
                site_type?: string;
                ssl_enabled: boolean;
                status: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        "PaginatedResponse<Website>_Website": {
            data: {
                /** Format: date-time */
                created_at: string;
                domain: string;
                engine: string;
                /** Format: int64 */
                id: number;
                name: string;
                /** Format: int64 */
                node_id: number;
                /**
                 * Format: int64
                 * @description 绑定的项目 ID（site_type = reverse_proxy + 项目场景）
                 */
                project_id?: number | null;
                proxy_enabled: boolean;
                proxy_pass?: string | null;
                /**
                 * Format: int64
                 * @description 乐观并发控制版本号：写入时需匹配当前值，冲突返回稳定 `CONFLICT`。
                 */
                resource_version?: number;
                /**
                 * @description 已套用的伪静态模板名（Doc/20 §5.5 伪静态规则库；None = 未套用）。
                 *     生成站点配置时按该名解析模板并注入 vhost（套用即生效）。
                 */
                rewrite_preset?: string | null;
                root_path: string;
                /**
                 * Format: int64
                 * @description 绑定的运行时 ID（site_type = runtime_php 场景）
                 */
                runtime_id?: number | null;
                /** @description 站点类型：runtime_php | reverse_proxy | static | container_app（M-B 主线二 Stage A-6 · Doc/20 §5.5） */
                site_type?: string;
                ssl_enabled: boolean;
                status: string;
            }[];
            page: number;
            page_size: number;
            total: number;
            total_pages: number;
        };
        PaginationParams: {
            /**
             * Format: int64
             * @description 页码（从 1 开始）
             */
            page?: number | null;
            /**
             * Format: int64
             * @description 每页条数（1~200）
             */
            page_size?: number | null;
        };
        PanelRuntime: {
            /** Format: int32 */
            pid: number;
            running_as_root: boolean;
            /** Format: int32 */
            uid: number;
            version: string;
        };
        PluginMetricsResponse: {
            /** Format: double */
            avg_execution_ms: number;
            /** Format: int64 */
            failed_executions: number;
            /** Format: int64 */
            last_execution_ms: number;
            /** Format: int64 */
            max_execution_ms: number;
            /** Format: int64 */
            min_execution_ms: number;
            peak_memory_bytes: number;
            /** Format: int64 */
            successful_executions: number;
            /** Format: int64 */
            total_executions: number;
        };
        PluginReloadRequest: {
            /** @description 1 MiB – 256 MiB，缺省 64 MiB */
            memory_limit_bytes?: number | null;
            /**
             * Format: int64
             * @description 1 – 300000 ms，缺省 30000
             */
            timeout_ms?: number | null;
            wasm_base64: string;
        };
        PluginResponse: {
            author: string;
            /** @description 已授予的宿主能力 */
            capabilities: string[];
            dependencies: string[];
            description: string;
            enabled: boolean;
            /** Format: int64 */
            exec_count: number;
            homepage?: string | null;
            id: string;
            last_executed_at?: string | null;
            license?: string | null;
            loaded_at: string;
            name: string;
            status: string;
            tags: string[];
            version: string;
        };
        PluginSettingRequest: {
            /** @description 1–128 位 `[A-Za-z0-9._-]` */
            key: string;
            /** @description ≤ 4096 字节 */
            value: string;
        };
        /** @description 凭证视图（附签发者用户名与有效状态）。 */
        PrincipalView: components["schemas"]["AgentPrincipal"] & {
            /** @description 未吊销且未过期 */
            active: boolean;
            /** @description 签发者用户名（用户已删除时为空） */
            owner: string;
        };
        /** @description 单个进程类型条目。 */
        ProcEntry: {
            /** @description 启动命令 */
            command: string;
            /** @description 进程类型：web | worker | clock */
            proc_type: string;
        };
        /** @description 进程 TOP 条目（sysinfo 采集） */
        ProcessEntry: {
            /** Format: float */
            cpu: number;
            /** Format: int64 */
            memory_mb: number;
            name: string;
            /** Format: int32 */
            pid: number;
            status: string;
        };
        /** @description 一条进程记录。 */
        ProcessInfo: {
            /** @description 命令行（参数以空格拼接，最长 1024 字符；内核线程为 `[name]`） */
            command: string;
            /**
             * Format: float
             * @description CPU 占用（%，多核可超过 100）
             */
            cpu: number;
            /** @description 是否内核线程 */
            kernel_thread: boolean;
            /**
             * Format: int64
             * @description 常驻内存（字节）
             */
            memory_bytes: number;
            /** @description 进程名（`/proc/<pid>/comm`） */
            name: string;
            /** Format: int32 */
            pid: number;
            /**
             * Format: int32
             * @description 父进程 pid（内核根进程无）
             */
            ppid?: number | null;
            /**
             * Format: int64
             * @description 启动时间（Unix 秒）；结束进程时用于防 pid 复用
             */
            started_at: number;
            /** @description 进程状态（Run / Sleep / Zombie …） */
            status: string;
            /**
             * Format: int32
             * @description 属主 uid
             */
            uid?: number | null;
            /** @description 属主用户名（无法解析时为 None） */
            user?: string | null;
        };
        /** @description `GET /api/toolbox/processes` 响应。 */
        ProcessListResponse: {
            /** @description 截断后的进程行 */
            items: components["schemas"]["ProcessRow"][];
            /** @description 按条件命中数（截断前） */
            matched: number;
            /**
             * Format: int32
             * @description 面板进程 pid（前端高亮）
             */
            panel_pid: number;
            /**
             * Format: int64
             * @description 快照时间（Unix 毫秒）
             */
            sampled_at: number;
            /** @description 进程总数（不含用户态线程与内核线程） */
            total: number;
            /**
             * Format: int64
             * @description 所有进程常驻内存合计（字节，不含内核线程）
             */
            total_memory_bytes: number;
        };
        /** @description 列表中的一行：进程信息 + 保护原因（`null` = 允许结束）。 */
        ProcessRow: components["schemas"]["ProcessInfo"] & {
            protected?: null | components["schemas"]["ProtectedReason"];
        };
        /**
         * @description 允许发送的信号。
         * @enum {string}
         */
        ProcessSignal: "TERM" | "KILL" | "HUP" | "INT";
        /**
         * @description 进程运行状态快照（原生模式 = systemd/内建守护；容器模式 = docker inspect，
         *     两路同构：`state` 含 running | stopped | failed | unknown）。
         */
        ProcessStatus: {
            /**
             * Format: int32
             * @description 最近一次退出码（运行中为 None）
             */
            exit_code?: number | null;
            /**
             * Format: int64
             * @description 本次运行累计重启次数（systemd NRestarts / 内建守护计数 / 容器 RestartCount）
             */
            restarts: number;
            /** @description running | stopped | failed | unknown */
            state: string;
        };
        /** @description Procfile 进程声明（Stage A-8 · Doc/20 §5.8 ③，ps:scale 伸缩面板数据源）。 */
        ProcfileDto: {
            /** @description 进程类型 → 启动命令（web/worker/clock；无 Procfile 为空表） */
            processes: components["schemas"]["ProcEntry"][];
            /** Format: int64 */
            project_id: number;
        };
        /** @description 联链摘要（不含连接串）。 */
        ProjectDbLinkDto: {
            /** Format: int64 */
            db_instance_id: number;
            db_instance_name: string;
            db_type: string;
            env_name: string;
            /** Format: int64 */
            id: number;
            /** Format: int32 */
            port: number;
            /** Format: int64 */
            project_id: number;
        };
        /** @description 项目响应 DTO。 */
        ProjectDto: {
            /** @description webhook 自动部署开关 */
            auto_deploy: boolean;
            /** @description 构建器协商结果：dockerfile | buildpacks | runtime（Stage A-8 · Doc/20 §5.8 ②） */
            build_type: string;
            /** Format: date-time */
            created_at: string;
            deploy_path: string;
            /** @description 切换策略：restart（直接重启）| rolling（健康检查零停机） */
            deploy_strategy: string;
            /** @description 部署方式：manual | webhook */
            deploy_type: string;
            domain?: string | null;
            /** @description 是否声明了 Procfile 进程模型（Stage A-8 · Doc/20 §5.8 ③） */
            has_procfile: boolean;
            /** @description 是否已配置部署令牌（令牌明文不回显，enable 时一次性返回） */
            has_webhook_token: boolean;
            /** @description 健康检查路径（空串 = 未启用，Stage A-9 · Doc/20 §5.8 ④） */
            health_check_path: string;
            /** Format: int64 */
            id: number;
            /** @description php | jar | war | node | python | go | static */
            kind: string;
            name: string;
            /** Format: int64 */
            port?: number | null;
            process_unit?: string | null;
            run_command?: string | null;
            /** Format: int64 */
            runtime_id?: number | null;
            /** @description local_path | git */
            source: string;
            source_url?: string | null;
            /** @description building | running | stopped | error */
            status: string;
            /** Format: date-time */
            updated_at: string;
        };
        /**
         * @description 结束进程被拒绝的原因（前端据此禁用按钮并提示）。
         * @enum {string}
         */
        ProtectedReason: "init" | "kernel_thread" | "panel" | "panel_ancestor";
        PullImageRequest: {
            image: string;
        };
        PurgeResult: {
            purged: number;
        };
        RecoveryCodesResponse: {
            /** @description 一次性恢复码（明文仅此一次返回，请妥善保存） */
            recovery_codes: string[];
        };
        /** @description 回收站条目。 */
        RecycleItem: {
            /** Format: date-time */
            deleted_at: string;
            deleted_by: string;
            /** Format: date-time */
            expires_at: string;
            id: string;
            is_dir: boolean;
            name: string;
            /** @description 删除前的根内相对路径 */
            original_path: string;
            /**
             * Format: int64
             * @description 字节数（目录为递归合计）
             */
            size: number;
            /** @description 目录条目过多时体积只统计了前一部分 */
            size_truncated?: boolean;
        };
        RejectRequest: {
            /** @description 拒绝理由（≤ 500 字节，回传给 AI） */
            reason?: string | null;
        };
        /** @description Phase A1：Agent 动作枚举请求（动作 + 参数） */
        RemoteActionRequest: {
            action: string;
            params?: unknown;
        };
        RemoteBatchExecRequest: {
            command: string;
            node_ids: number[];
            /** Format: int64 */
            timeout_secs?: number | null;
        };
        RemoteExecRequest: {
            command: string;
            /** Format: int64 */
            timeout_secs?: number | null;
        };
        RemoteListQuery: {
            path?: string;
        };
        /** @description 远端备份对象（云端账号内） */
        RemoteObjectDto: {
            key: string;
            /** Format: int64 */
            size: number;
        };
        RemoteUploadRequest: {
            /** @description base64 编码的文件内容 */
            content_base64: string;
            path: string;
        };
        RenameRequest: {
            new_path: string;
            old_path: string;
        };
        /** @description 续签时间线的一条记录。 */
        RenewalEntry: {
            /** Format: date-time */
            at: string;
            /** @description 事件记录的主域名 */
            domain: string;
            /** @description 失败原因（仅 `auto_renew_failed`） */
            error?: string | null;
            /**
             * Format: int64
             * @description 对应的 Outbox 事件 ID（稳定排序键，前端列表 key）
             */
            event_id: number;
            /** @description 预警事件记录的到期时刻（RFC 3339，仅 `expiry_warning`） */
            expires_at?: string | null;
            kind: components["schemas"]["RenewalEventKind"];
            /** @description 签发来源（仅 `issued`：letsencrypt / zerossl / self_signed / uploaded） */
            source?: string | null;
        };
        /**
         * @description 时间线条目种类。
         * @enum {string}
         */
        RenewalEventKind: "issued" | "auto_renewed" | "auto_renew_failed" | "expiry_warning";
        ReorderRequest: {
            /** @description 规则 ID，按顺序赋优先级 10、20、30…（单事务）；未列出的规则保持原优先级，不存在的 ID 忽略 */
            ids: number[];
        };
        RestoreBackupRequest: {
            filename: string;
        };
        RestoreFromCloudRequest: {
            /**
             * Format: int64
             * @description 目标备份账号 ID
             */
            account_id: number;
            /** @description 远端对象 key（如 panel/flamepanel-20260907.db） */
            remote_key: string;
        };
        RestoreFromCloudResponse: {
            /** @description 还原对象名（面板库为 .db 文件名，应用为备份包文件名） */
            filename: string;
            /** @description 面向用户的下一步提示（是否需重启） */
            message: string;
            /** @description panel（面板库）| app（应用数据） */
            scope: string;
        };
        /**
         * @description 工具风险分级（Doc/30 §M-AI-2）。
         *
         *     - `read`：只读，调用即执行；
         *     - `write`：可逆变更（启停容器、重新下发站点配置），**必须**经面板人工批准后才执行；
         *     - `destructive`：不可逆变更（删除容器等），批准者还需 `mcp:delete`。
         * @enum {string}
         */
        RiskLevel: "read" | "write" | "destructive";
        /** @description 轮换 JWT 密钥请求（`POST /api/auth/rotate-secret`） */
        RotateSecretBody: {
            /** @description 新密钥（不少于 32 字节） */
            secret: string;
        };
        /** @description 运行时响应 DTO。 */
        RuntimeDto: {
            /** Format: date-time */
            created_at: string;
            /** Format: int64 */
            id: number;
            install_path: string;
            is_default: boolean;
            /** @description php | java | node | python | go */
            kind: string;
            metadata_json?: string | null;
            /** @description not_installed | installing | installed | running | error */
            status: string;
            /** Format: date-time */
            updated_at: string;
            version: string;
        };
        /** @description 可安装版本清单条目（`GET /api/runtimes/available?kind=php`）。 */
        RuntimeVersionInfo: {
            /** @description 系统包已安装 或 平台已登记安装 */
            installed: boolean;
            is_default: boolean;
            /**
             * Format: int64
             * @description 平台已登记时的 Runtime 记录 id
             */
            runtime_id?: number | null;
            version: string;
        };
        ScheduledTask: {
            command: string;
            /** Format: date-time */
            created_at: string;
            enabled: boolean;
            /** Format: int64 */
            id: number;
            last_output: string;
            /** Format: date-time */
            last_run_at?: string | null;
            /** @description never | success | failed */
            last_status: string;
            name: string;
            /** Format: date-time */
            next_run_at?: string | null;
            /** @description 标准 5 字段 cron 表达式 */
            schedule: string;
            /** Format: date-time */
            updated_at: string;
        };
        ServerNode: {
            /**
             * Format: int32
             * @description Agent HTTP 服务端口（面板据此构造 `http://<ip>:<agent_port>` 发起远程调用）
             */
            agent_port?: number;
            /** @description Agent 注册时携带的认证令牌 */
            auth_token?: string | null;
            /** Format: date-time */
            created_at: string;
            hostname: string;
            /** Format: int64 */
            id: number;
            ip_address: string;
            /**
             * Format: date-time
             * @description 最近一次心跳时间（Agent 上报）
             */
            last_heartbeat_at?: string | null;
            /** @description 最近指标快照 JSON：{cpu_usage, memory_usage_percent, disk_usage_percent, load_one} */
            metrics_json?: string | null;
            name: string;
            status: string;
        };
        /** @description 运行时调级请求 */
        SetKernelLogLevelRequest: {
            /** @description `trace` / `debug` / `info` / `warn` / `error` */
            level: string;
        };
        SettingEntry: {
            description: string;
            key: string;
            value: string;
        };
        SetupAdminInput: {
            password: string;
            username: string;
        };
        SetupDatabaseInput: {
            db_type: string;
            host?: string;
            mysql_root_password?: string;
            name?: string;
            password?: string;
            /** Format: int32 */
            port?: number;
            user?: string;
        };
        SetupRequest: {
            admin?: null | components["schemas"]["SetupAdminInput"];
            database?: null | components["schemas"]["SetupDatabaseInput"];
            language?: string | null;
            /** @description `database` | `admin` */
            step: string;
            theme?: string | null;
        };
        SetupStatusResponse: {
            /** @description Docker 可用性（尽力探测；未知为 null） */
            docker?: boolean | null;
            language: string;
            /** @description Nginx 可用性（尽力探测；未知为 null） */
            nginx?: boolean | null;
            status: string;
            theme: string;
        };
        /**
         * @description 严重度（升序排列便于比较：`Critical > High > …`）。
         * @enum {string}
         */
        Severity: "info" | "low" | "medium" | "high" | "critical";
        /** @description 站点部署结果（`POST /api/websites/{id}/deploy` 出参）。 */
        SiteDeployment: {
            enabled: boolean;
            engine: string;
            /** @description 站点配置文件（逻辑路径）。 */
            path: string;
            /** @description 是否已 reload（引擎未运行时为 false，下次启动生效）。 */
            reloaded: boolean;
        };
        /** @description 伪静态套用状态查询结果。 */
        SiteRewriteStatus: {
            /** @description 当前套用模板名（None = 未套用） */
            preset?: string | null;
            /** @description 按该站点引擎语法生成的规则文本（未套用为空串） */
            rules: string;
            /** @description 可选模板清单（模板库单一事实来源） */
            supported: string[];
        };
        SourcesResponse: {
            sources: components["schemas"]["AppSource"][];
            /** Format: int64 */
            total: number;
        };
        /**
         * @description 商店安全配置（Doc/20 §4.2 收口）：验签策略 + 镜像仓库白名单。
         *
         *     供「应用商店 · 安全设置」页读取：内置白名单只读展示、追加项可编辑、
         *     生效清单为两者合并去重结果（与安装期安全扫描实际使用的清单一致）。
         */
        StoreSecurityConfig: {
            /** @description 内置可信镜像仓库（只读，代码常量） */
            builtin_registries: string[];
            /** @description 生效白名单（内置 + 追加，去重保序）——与安全扫描实际使用清单一致 */
            effective_registries: string[];
            /** @description 用户追加的镜像仓库白名单（可编辑，settings 持久化） */
            registry_whitelist: string[];
            /** @description 验签策略：optional（源配公钥才验签）| required（远程包强制验签） */
            signature_policy: string;
            /** @description 未配置验签公钥的远程源数量（前端安全提示：这些源的应用包不做签名校验） */
            unsigned_source_count: number;
        };
        SwitchEngineRequest: {
            engine: string;
        };
        SwitchWebsiteEngineRequest: {
            /** @description 目标引擎：`nginx` | `openresty` | `caddy` | `apache` | `openlitespeed` */
            engine: string;
        };
        SyncSourceResponse: {
            added: number;
            packages: string[];
            skipped: number;
        };
        TagImageRequest: {
            /** @description 仓库名（不含 tag / digest），如 `registry.local:5000/team/app` */
            repo: string;
            /** @description `[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}` */
            tag: string;
        };
        /** @description 任务徽标：统一 TaskTracker 中各状态计数。 */
        TaskBadges: {
            /** Format: int64 */
            failed: number;
            /** Format: int64 */
            pending: number;
            /** Format: int64 */
            running: number;
        };
        /**
         * @description 任务类别：标识统一状态机服务的三类主要场景。
         * @enum {string}
         */
        TaskKind: "install" | "engine_switch" | "batch_node" | "generic";
        TaskListResponse: {
            tasks: components["schemas"]["TaskRecord"][];
            total: number;
        };
        /** @description 一条统一任务的完整记录。 */
        TaskRecord: {
            /** Format: date-time */
            created_at: string;
            /** Format: int64 */
            id: number;
            kind: components["schemas"]["TaskKind"];
            /** @description 结果信息（终态时描述成功/失败原因） */
            message: string;
            name: string;
            /**
             * Format: int32
             * @description 进度 0..=100
             */
            progress: number;
            state: components["schemas"]["TaskState"];
            /** Format: date-time */
            updated_at: string;
        };
        /**
         * @description 统一任务状态：长耗时操作的通用五态状态机。
         *
         *     迁移规则（`advance` 强校验）：
         *     - `Pending → Running`（任务开始执行）
         *     - `Running → Success | Failed | Cancelled`（任务结束）
         *     - `Pending → Cancelled`（尚未开始即被取消）
         * @enum {string}
         */
        TaskState: "pending" | "running" | "success" | "failed" | "cancelled";
        TestResult: {
            /** @description 已配置的模型是否在列表中（列表为空时为 null） */
            model_found?: boolean | null;
            /** @description 服务端返回的模型列表（≤ 200） */
            models: string[];
            ok: boolean;
        };
        ToggleJobRequest: {
            enabled: boolean;
        };
        ToggleRequest: {
            enabled: boolean;
        };
        ToggleTaskRequest: {
            enabled: boolean;
        };
        /**
         * @description 写操作工具的审批单。
         *
         *     `raw_arguments` 是执行时使用的原始参数，**永不序列化**到 API；对外只展示脱敏后的 `arguments`。
         */
        ToolApproval: {
            /** @description 调用参数（JSON 文本，已脱敏） */
            arguments: string;
            /** Format: date-time */
            decided_at?: string | null;
            /**
             * Format: int64
             * @description 批准 / 拒绝者用户 ID
             */
            decided_by?: number | null;
            /**
             * Format: date-time
             * @description 超过该时间仍未处理即过期
             */
            expires_at: string;
            /** Format: date-time */
            finished_at?: string | null;
            /** Format: int64 */
            id: number;
            /** @description 执行前预览（dry-run）：将要发生的变更与当前状态 */
            preview: string;
            /** Format: int64 */
            principal_id: number;
            /** @description 拒绝理由（可空） */
            reason: string;
            /** Format: date-time */
            requested_at: string;
            /** @description 执行输出或失败原因（截断到 8 KiB） */
            result: string;
            risk: components["schemas"]["RiskLevel"];
            session_id?: string | null;
            status: components["schemas"]["ApprovalStatus"];
            tool: string;
        };
        /** @description 工具调用审计记录。 */
        ToolCallRecord: {
            /** @description 调用参数（JSON 文本，已脱敏并截断到 2 KiB） */
            arguments: string;
            /** Format: date-time */
            created_at: string;
            /** Format: int64 */
            duration_ms: number;
            /** @description 失败原因（成功时为空） */
            error: string;
            /** Format: int64 */
            id: number;
            ok: boolean;
            /** Format: int64 */
            principal_id: number;
            session_id?: string | null;
            tool: string;
        };
        ToolInfo: {
            /** @description 当前用户是否可用 */
            allowed: boolean;
            description: string;
            input_schema: Record<string, never>;
            name: string;
            /** @description 所需权限 `resource:action`（由等价 REST 路由推导） */
            permission: string;
            /** @description 风险分级：read / write / destructive（非 read 需人工审批） */
            risk: string;
            title: string;
        };
        /** @description 一次工具调用的轨迹（前端展示用，参数与输出均已脱敏）。 */
        ToolStep: {
            /**
             * Format: int64
             * @description 写工具生成的审批单 ID
             */
            approval_id?: number | null;
            /** @description 参数（JSON 文本，已脱敏） */
            arguments: string;
            ok: boolean;
            /** @description 工具输出或错误（截断到 4 KiB） */
            output: string;
            tool: string;
        };
        /** @description 单项结果。 */
        TransferItem: {
            /** @description 失败原因 */
            error?: string | null;
            /** @description 失败类别：`conflict`（同名已存在）/ `not_found` / `forbidden` / `invalid` / `internal` */
            error_kind?: string | null;
            ok: boolean;
            /** @description 请求中的源路径（原样） */
            source: string;
            /** @description 成功时的目标路径（根内相对路径） */
            target?: string | null;
        };
        /** @enum {string} */
        TransferMode: "copy" | "move";
        TransferRequest: {
            /** @description 目标目录（必须已存在） */
            dest_dir: string;
            /** @description 同名条目存在时覆盖（旧条目移入回收站）；缺省 false → 该项失败（`error_kind = conflict`） */
            overwrite?: boolean;
            /** @description 源条目（根内路径，可来自不同目录；单次 ≤ 500 项） */
            sources: string[];
        };
        TransferResult: {
            failed: number;
            items: components["schemas"]["TransferItem"][];
            succeeded: number;
        };
        TwoFactorConfirmRequest: {
            /** @description 6 位验证码（停用时也可填恢复码） */
            code: string;
            /** @description 当前密码 */
            password: string;
        };
        TwoFactorEnableRequest: {
            /** @description 认证器 App 显示的 6 位验证码 */
            code: string;
        };
        /** @description 绑定信息（仅在 setup 时返回一次） */
        TwoFactorSetup: {
            /** @description `otpauth://` URI（生成二维码） */
            otpauth_uri: string;
            /** @description base32 密钥（无法扫码时手动输入） */
            secret: string;
        };
        TwoFactorSetupRequest: {
            /** @description 当前密码（二次确认身份） */
            password: string;
        };
        /** @description 当前用户的 2FA 状态 */
        TwoFactorStatus: {
            enabled: boolean;
            /** Format: date-time */
            enabled_at?: string | null;
            /** @description 存在尚未验证的待绑定密钥 */
            pending: boolean;
            /** @description 剩余可用恢复码数量 */
            recovery_codes_remaining: number;
        };
        TwoFactorStatusResponse: components["schemas"]["TwoFactorStatus"] & {
            /** @description 面板是否强制所有用户启用两步验证（设置 `two_factor_enabled`） */
            required: boolean;
        };
        UpdateAccountRequest: {
            account_type?: string | null;
            config_json?: string | null;
            enabled?: boolean | null;
            name?: string | null;
        };
        /** @description 变更插件宿主能力授予（整体替换） */
        UpdateCapabilitiesRequest: {
            capabilities: string[];
        };
        UpdateCertificateRequest: {
            auto_renew?: boolean | null;
            /** Format: int64 */
            website_id?: number | null;
        };
        UpdateDnsProviderRequest: {
            credentials_json?: string | null;
            enabled?: boolean | null;
            name?: string | null;
            provider_type?: string | null;
        };
        UpdateFirewallRuleRequest: {
            action?: string | null;
            description?: string | null;
            destination?: string | null;
            direction?: string | null;
            enabled?: boolean | null;
            name?: string | null;
            port?: string | null;
            /** Format: int32 */
            priority?: number | null;
            protocol?: string | null;
            source?: string | null;
        };
        UpdateJobRequest: {
            enabled?: boolean | null;
            name?: string | null;
            /** Format: int64 */
            retention?: number | null;
            schedule?: string | null;
            scope?: string | null;
            scope_key?: string | null;
            /** Format: int64 */
            target_account_id?: number | null;
        };
        UpdateMemoRequest: {
            content?: string | null;
            done?: boolean | null;
        };
        /** @description 更新项目请求（全量可编辑字段）。 */
        UpdateProjectRequest: {
            deploy_path?: string | null;
            deploy_strategy?: string | null;
            domain?: string | null;
            health_check_path?: string | null;
            kind: string;
            /** Format: int64 */
            port?: number | null;
            run_command?: string | null;
            /** Format: int64 */
            runtime_id?: number | null;
            source: string;
            source_url?: string | null;
        };
        UpdateSettingRequest: {
            key: string;
            value: string;
        };
        UpdateSettingsBatchRequest: {
            /** @description 批量设置项（key -> value），在一次事务内原子写入。 */
            settings: [
                string,
                string
            ][];
        };
        UpdateSourceRequest: {
            /** @description 清空公钥（签名源降级为未签名源）须显式确认 */
            confirm_unsigned?: boolean;
            enabled?: boolean | null;
            name?: string | null;
            public_key?: string | null;
            url?: string | null;
        };
        UpdateStoreSecurityRequest: {
            /** @description 镜像仓库白名单追加项（全量替换；与内置白名单合并去重） */
            registry_whitelist?: string[] | null;
            /** @description 验签策略：optional（源配公钥才验签）| required（远程包强制验签） */
            signature_policy?: string | null;
        };
        UpdateTaskRequest: {
            command?: string | null;
            enabled?: boolean | null;
            name?: string | null;
            schedule?: string | null;
        };
        UpdateUserRequest: {
            password_hash?: string | null;
            role: string;
            username: string;
        };
        UpdateWafRuleRequest: {
            action?: string | null;
            config_json?: string | null;
            enabled?: boolean | null;
            name?: string | null;
            pattern?: string | null;
            rule_type?: string | null;
            /** Format: int64 */
            website_id?: number | null;
        };
        /** @description 升级预览（`GET /api/app-store/installed/{id}/upgrade-preview`）。 */
        UpgradePreview: {
            /** @description 是否有安装时保存的 compose 基线；无基线时本地与新版本的每处差异都记为冲突 */
            base_available: boolean;
            from_version: string;
            merge?: null | components["schemas"]["ComposeMerge"];
            mode: string;
            to_version: string;
            /** @description 已是目标版本（无需升级） */
            up_to_date: boolean;
        };
        UploadCertificateRequest: {
            /** @description 证书 PEM（全链，含中间证书） */
            cert_pem: string;
            domains: string[];
            /** @description 私钥 PEM */
            key_pem: string;
            /** Format: int64 */
            website_id?: number | null;
        };
        /** @description 上传本地备份到云端账号请求 */
        UploadObjectRequest: {
            /** @description 本地备份文件名（位于备份目录内，如 `flamepanel-20260907.db`） */
            local_filename: string;
            /** @description 远端对象 key（缺省用文件名） */
            remote_key?: string | null;
        };
        User: {
            /** Format: date-time */
            created_at: string;
            /** Format: int64 */
            id: number;
            /** @description 是否强制修改密码（新装面板的种子 admin 为 true） */
            must_change_password: boolean;
            password_hash: string;
            role: string;
            username: string;
        };
        /** @description 下发报告（`POST /api/waf/apply` 出参）。 */
        WafApplyReport: {
            engines: components["schemas"]["WafEngineApply"][];
            /** @description 没有任何引擎下发失败（未安装的引擎不算失败） */
            ok: boolean;
        };
        /** @description 单个引擎的下发结果。 */
        WafEngineApply: {
            /** @description 是否已写入配置（未安装 / 无写权限时为 false，见 `reason`） */
            applied: boolean;
            /** @description 已下发的规则 ID */
            enforced: number[];
            /** @description 引擎名（nginx / openresty） */
            engine: string;
            /** @description 下发失败原因（配置校验失败等；已回滚） */
            error?: string | null;
            /** @description 未下发原因（未安装 / 无写权限） */
            reason?: string | null;
            /** @description 为接入 WAF 而重新部署的站点（旧版本渲染的站点缺少 WAF include） */
            redeployed_sites: string[];
            /** @description 是否已 reload（引擎未运行时为 false，下次启动生效） */
            reloaded: boolean;
            skipped: components["schemas"]["WafSkippedRule"][];
        };
        /** @description WAF 引擎诊断信息 */
        WafEngineStatus: {
            /** @description 是否已配置 Web 引擎下发（Nginx / OpenResty 原生配置） */
            enforcement: boolean;
            /** @description Web 引擎命中日志回灌状态（按引擎；未安装的引擎不出现） */
            ingest: components["schemas"]["WafIngestStatus"][];
            /** @description 引擎内启用规则数（规则变更即时重建生效） */
            loaded_rules: number;
        };
        /** @description 单个引擎的命中日志回灌状态（`GET /api/waf/status`）。 */
        WafIngestStatus: {
            /** @description 引擎名（nginx / openresty） */
            engine: string;
            /**
             * Format: int64
             * @description 本次进程启动以来导入的拦截日志条数
             */
            imported: number;
            /** @description 最近一次失败原因（成功后清空） */
            last_error?: string | null;
            /**
             * Format: date-time
             * @description 最近一次成功读取时间
             */
            last_run_at?: string | null;
            /**
             * Format: int64
             * @description 已读取到的日志偏移（字节）
             */
            offset: number;
        };
        /**
         * @description WAF 拦截日志（§6.2：网站维度记录，复用 operation_logs 查询/分页模式）。
         *
         *     敏感请求详情（headers/body）不落库——只记来源 IP、命中规则、请求行摘要，
         *     防止日志成为敏感信息二次泄漏点。
         */
        WafLog: {
            /** @description 命中动作快照：log | block */
            action: string;
            /** @description 来源 IP */
            client_ip: string;
            /**
             * Format: date-time
             * @description 命中时间
             */
            created_at: string;
            /** Format: int64 */
            id: number;
            /** @description 请求方法 + 路径摘要（如 `GET /login?id=1' OR`；超长截断） */
            request_summary: string;
            /**
             * Format: int64
             * @description 命中的规则 ID（规则删除后保留历史日志，页面显示规则名快照）
             */
            rule_id: number;
            /** @description 规则名称快照（规则删除后仍可读） */
            rule_name: string;
            /** @description 规则类型快照 */
            rule_type: string;
            /** @description 命中的 UA（超长截断） */
            user_agent: string;
            /**
             * Format: int64
             * @description 命中的网站 ID（0 = 全局规则命中）
             */
            website_id: number;
        };
        /**
         * @description WAF 规则（§6.2 · 首期：CC 频率限制 / SQL 注入 / XSS 特征 / IP·UA 黑白名单 / 路径封禁）。
         *
         *     `website_id = 0` 表示全局规则（作用于全部站点）；`config_json` 为各规则类型
         *     自解析的参数结构（如 cc 的 `{ "threshold": 120, "window_secs": 60 }`）。
         *     默认动作 `log`：仅告警不阻断（§6.2 风险对策 6——误拦截优先可观测）。
         */
        WafRule: {
            /** @description 命中动作：log | block */
            action: string;
            /** @description 类型相关参数（JSON；cc 规则放 threshold/window_secs，其他类型可为 `{}`） */
            config_json: string;
            /** Format: date-time */
            created_at: string;
            enabled: boolean;
            /** Format: int64 */
            id: number;
            /** @description 规则名称（展示用） */
            name: string;
            /** @description 匹配值：IP/CIDR、UA 子串、路径前缀或特征正则（按 rule_type 语义解析） */
            pattern: string;
            /** @description 规则类型（`WAF_RULE_TYPES` 白名单） */
            rule_type: string;
            /** Format: date-time */
            updated_at: string;
            /**
             * Format: int64
             * @description 作用网站 ID（0 = 全局规则）
             */
            website_id: number;
        };
        /** @description 未下发的规则。 */
        WafSkippedRule: {
            reason: string;
            /** Format: int64 */
            rule_id: number;
        };
        WebServerInstance: {
            binary_path?: string | null;
            config_path: string;
            /** Format: date-time */
            created_at: string;
            engine: string;
            /** Format: int64 */
            id: number;
            /** Format: int32 */
            port: number;
            /**
             * Format: int64
             * @description 乐观并发控制版本号：写入时需匹配当前值，冲突返回稳定 `CONFLICT`。
             */
            resource_version?: number;
            status: string;
            version?: string | null;
        };
        WebServerResponse: {
            binary_path?: string | null;
            config_path: string;
            created_at: string;
            engine: string;
            /** Format: int64 */
            id: number;
            /** Format: int32 */
            port: number;
            /** Format: int64 */
            resource_version: number;
            status: string;
            version?: string | null;
        };
        /** @description 开启 webhook 部署响应（令牌明文仅此一次返回）。 */
        WebhookDeployDto: {
            project: components["schemas"]["ProjectDto"];
            /** @description 部署令牌明文（仅本次响应返回，之后不可再获取） */
            token: string;
            /** @description 部署端点：/api/projects/:id/webhook?token=<token> */
            webhook_url: string;
        };
        Website: {
            /** Format: date-time */
            created_at: string;
            domain: string;
            engine: string;
            /** Format: int64 */
            id: number;
            name: string;
            /** Format: int64 */
            node_id: number;
            /**
             * Format: int64
             * @description 绑定的项目 ID（site_type = reverse_proxy + 项目场景）
             */
            project_id?: number | null;
            proxy_enabled: boolean;
            proxy_pass?: string | null;
            /**
             * Format: int64
             * @description 乐观并发控制版本号：写入时需匹配当前值，冲突返回稳定 `CONFLICT`。
             */
            resource_version?: number;
            /**
             * @description 已套用的伪静态模板名（Doc/20 §5.5 伪静态规则库；None = 未套用）。
             *     生成站点配置时按该名解析模板并注入 vhost（套用即生效）。
             */
            rewrite_preset?: string | null;
            root_path: string;
            /**
             * Format: int64
             * @description 绑定的运行时 ID（site_type = runtime_php 场景）
             */
            runtime_id?: number | null;
            /** @description 站点类型：runtime_php | reverse_proxy | static | container_app（M-B 主线二 Stage A-6 · Doc/20 §5.5） */
            site_type?: string;
            ssl_enabled: boolean;
            status: string;
        };
        WebsiteDto: {
            created_at: string;
            domain: string;
            engine: string;
            /** Format: int64 */
            id: number;
            name: string;
            /** Format: int64 */
            node_id: number;
            /** Format: int64 */
            project_id?: number | null;
            proxy_enabled: boolean;
            proxy_pass?: string | null;
            root_path: string;
            /** Format: int64 */
            runtime_id?: number | null;
            /** @description 站点类型：runtime_php | reverse_proxy | static | container_app */
            site_type: string;
            ssl_enabled: boolean;
            status: string;
        };
        WriteRequest: {
            content: string;
            /** @description 打开文件时看到的 `modified_at`；与当前不一致返回 409（缺省 = 强制覆盖） */
            expected_modified?: string | null;
            path: string;
        };
        /** @description 写入结果：新的修改时间（下一次保存用作 `expected_modified`）。 */
        WriteResult: {
            modified_at: string;
            /** Format: int64 */
            size: number;
        };
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    "GET /api/app-store/installed": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已安装应用列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InstalledApp"][];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/app-store/installed/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 已安装应用 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 详情 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InstalledAppResponse"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 app_store:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 已安装应用不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/app-store/installed/{id}/launch": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 已安装应用 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 更新后的应用 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InstalledApp"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 app_store:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 已安装应用不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/app-store/installed/{id}/logs": {
        parameters: {
            query?: {
                /** @description 尾行数，缺省 200，钳制到 1–5000 */
                tail?: number;
            };
            header?: never;
            path: {
                /** @description 已安装应用 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 日志 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 app_store:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 已安装应用不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/app-store/installed/{id}/native-restart": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 已安装应用 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已执行 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 app_store:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 已安装应用不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/app-store/installed/{id}/native-start": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 已安装应用 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已执行 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 app_store:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 已安装应用不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/app-store/installed/{id}/native-status": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 已安装应用 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 状态 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 app_store:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 已安装应用不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/app-store/installed/{id}/native-stop": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 已安装应用 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已执行 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 app_store:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 已安装应用不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/app-store/installed/{id}/uninstall": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 已安装应用 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已卸载 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 app_store:delete 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 已安装应用不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/app-store/installed/{id}/upgrade": {
        parameters: {
            query?: {
                /** @description 目标版本，缺省 latest */
                target_version?: string;
                /** @description compose 冲突处理：abort（默认，409）/ local（保留本地）/ upstream（采用新版本） */
                conflict_strategy?: string;
            };
            header?: never;
            path: {
                /** @description 已安装应用 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 升级后的应用 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InstalledApp"];
                };
            };
            /** @description 降级 / 新版本无 compose 模板 / 未知冲突策略 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 app_store:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 已安装应用不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 本地 compose 与新版本冲突（abort 策略） */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/app-store/installed/{id}/upgrade-preview": {
        parameters: {
            query?: {
                /** @description 目标版本，缺省 latest */
                target_version?: string;
            };
            header?: never;
            path: {
                /** @description 已安装应用 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 升级预览 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["UpgradePreview"];
                };
            };
            /** @description 降级 / compose 不是合法 YAML */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 app_store:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 已安装应用或版本不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/app-store/packages": {
        parameters: {
            query?: {
                /** @description 分类过滤 */
                category?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 包列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AppStoreListResponse"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/app-store/packages/batch-import": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["BatchImportPackagesRequest"];
            };
        };
        responses: {
            /** @description 批量导入结果 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["BatchImportPackagesResponse"];
                };
            };
            /** @description 非法路径 / 空列表 / 已存在 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/app-store/packages/import": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ImportPackageRequest"];
            };
        };
        responses: {
            /** @description 导入的元数据 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AppMetadata"];
                };
            };
            /** @description 不是目录 / 格式无法识别 / key 非法 / 已存在 / 验签失败 / 源与目标目录互相包含 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 app_store:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/app-store/packages/{key}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 应用包 key */
                key: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 元数据 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AppMetadata"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 app_store:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 应用包不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/app-store/packages/{key}/install": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 应用包 key */
                key: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["InstallAppRequest"];
            };
        };
        responses: {
            /** @description 已安装应用 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InstalledApp"];
                };
            };
            /** @description package_key 与路径不一致 / 端口或变量非法 / 未确认风险或脚本 / 验签失败 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 app_store:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 应用包不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/app-store/packages/{key}/versions/{version}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 应用包 key */
                key: string;
                /** @description 版本号 */
                version: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 版本详情（AppVersionInfo） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 app_store:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 应用包或版本不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/app-store/security": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 商店安全配置 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["StoreSecurityConfig"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PUT /api/app-store/security": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateStoreSecurityRequest"];
            };
        };
        responses: {
            /** @description 更新后的商店安全配置 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["StoreSecurityConfig"];
                };
            };
            /** @description 无更新项 / 参数非法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 设置服务未装配 */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/app-store/sources": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 远程源列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SourcesResponse"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/app-store/sources": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateSourceRequest"];
            };
        };
        responses: {
            /** @description 新建的远程源 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AppSource"];
                };
            };
            /** @description 参数非法 / url 重复 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PUT /api/app-store/sources/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 源 id */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateSourceRequest"];
            };
        };
        responses: {
            /** @description 更新后的远程源 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AppSource"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 源不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/app-store/sources/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 源 id */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 删除成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 源不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/app-store/sources/{id}/packages": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 源 id */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 该源下的应用包列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AppStoreListResponse"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 源不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/app-store/sources/{id}/sync": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 源 id */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 同步结果 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SyncSourceResponse"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 源不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 该源正在同步中 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/app-store/wasm-builtins": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 元数据列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AppMetadata"][];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 app_store:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/assistant/chat": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ChatRequest"];
            };
        };
        responses: {
            /** @description 助手回复与工具轨迹 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ChatResponse"];
                };
            };
            /** @description 消息不合法（角色、长度、条数） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 mcp:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 上一轮对话尚未结束 */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 助手未启用 / 模型服务不可用 */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/assistant/config": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 助手配置（不含密钥） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AssistantConfigView"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 mcp:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PUT /api/assistant/config": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AssistantConfigPatch"];
            };
        };
        responses: {
            /** @description 更新后的配置 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AssistantConfigView"];
                };
            };
            /** @description 字段不合法（如启用但未填模型、base_url 内嵌凭据） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 settings:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 主密钥不可用，拒绝明文保存 API Key */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/assistant/test": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 连通性测试结果 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TestResult"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 settings:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 无法连接模型服务 / 鉴权失败 */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/auth/2fa": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 当前用户的两步验证状态 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TwoFactorStatusResponse"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/auth/2fa/disable": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["TwoFactorConfirmRequest"];
            };
        };
        responses: {
            /** @description 已停用 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 密码或验证码错误 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 面板强制两步验证，不可停用 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 尝试过于频繁 */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/auth/2fa/enable": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["TwoFactorEnableRequest"];
            };
        };
        responses: {
            /** @description 已启用；恢复码仅此一次返回 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RecoveryCodesResponse"];
                };
            };
            /** @description 未开始绑定或验证码错误 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 尝试过于频繁 */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/auth/2fa/recovery-codes": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["TwoFactorConfirmRequest"];
            };
        };
        responses: {
            /** @description 新恢复码（仅此一次返回） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RecoveryCodesResponse"];
                };
            };
            /** @description 密码或验证码错误 / 未启用 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 尝试过于频繁 */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/auth/2fa/setup": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["TwoFactorSetupRequest"];
            };
        };
        responses: {
            /** @description 待绑定密钥 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TwoFactorSetup"];
                };
            };
            /** @description 密码错误 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 尝试过于频繁 */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/auth/change-password": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ChangePasswordBody"];
            };
        };
        responses: {
            /** @description 修改成功；该用户其他会话的 refresh token 失效 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 旧密码错误或新密码不满足强度策略 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/auth/login": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["LoginRequest"];
            };
        };
        responses: {
            /** @description 登录成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LoginResponse"];
                };
            };
            /** @description 凭据错误 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 尝试过于频繁 */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/auth/login/2fa": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["LoginTwoFactorRequest"];
            };
        };
        responses: {
            /** @description 登录成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LoginResponse"];
                };
            };
            /** @description 挑战令牌无效 / 过期，或验证码错误 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 尝试过于频繁 */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/auth/logout": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 退出成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 令牌无效/缺失 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/auth/me": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 当前用户 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MeResponse"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/auth/refresh": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 刷新成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LoginResponse"];
                };
            };
            /** @description 令牌无效/过期 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/auth/rotate-secret": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RotateSecretBody"];
            };
        };
        responses: {
            /** @description 轮换成功，旧 access token 立即失效 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 密钥过短 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 无权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/backup-accounts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 账号列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AccountDto"][];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/backup-accounts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateAccountRequest"];
            };
        };
        responses: {
            /** @description 创建成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AccountDto"];
                };
            };
            /** @description 参数错误 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PUT /api/backup-accounts/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 账号 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateAccountRequest"];
            };
        };
        responses: {
            /** @description 更新成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AccountDto"];
                };
            };
            /** @description 账号不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/backup-accounts/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 账号 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 删除成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 账号不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/backup-accounts/{id}/objects": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 账号 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 远端对象列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RemoteObjectDto"][];
                };
            };
            /** @description 账号不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/backup-accounts/{id}/objects": {
        parameters: {
            query: {
                /** @description 远端对象 key */
                key: string;
            };
            header?: never;
            path: {
                /** @description 账号 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 删除成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 账号不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/backup-accounts/{id}/objects/upload": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 账号 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UploadObjectRequest"];
            };
        };
        responses: {
            /** @description 上传成功，返回远端对象 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RemoteObjectDto"];
                };
            };
            /** @description 参数错误（文件名非法 / 账号停用） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 账号或本地文件不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/backup-accounts/{id}/test": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 账号 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 测试完成，返回账号（含 last_test_status） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AccountDto"];
                };
            };
            /** @description 账号不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/backup-jobs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 任务列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["JobDto"][];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/backup-jobs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateJobRequest"];
            };
        };
        responses: {
            /** @description 创建成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["JobDto"];
                };
            };
            /** @description 参数错误（cron 非法 / 账号不存在） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/backup-jobs/restore-cloud": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RestoreFromCloudRequest"];
            };
        };
        responses: {
            /** @description 恢复完成，返回范围/对象名与生效提示 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RestoreFromCloudResponse"];
                };
            };
            /** @description 参数错误（key 非法 / 包格式不支持 / 包内容不安全） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 账号、远端对象或目标应用不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PUT /api/backup-jobs/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 任务 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateJobRequest"];
            };
        };
        responses: {
            /** @description 更新成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["JobDto"];
                };
            };
            /** @description 任务不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/backup-jobs/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 任务 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 删除成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 任务不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PUT /api/backup-jobs/{id}/enabled": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 任务 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ToggleJobRequest"];
            };
        };
        responses: {
            /** @description 已更新 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["JobDto"];
                };
            };
            /** @description 任务不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/backup-jobs/{id}/run": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 任务 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 执行完成，返回任务（含执行结果） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["JobDto"];
                };
            };
            /** @description 任务不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/backups": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 备份列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["BackupEntryDto"][];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/backups": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 备份成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["BackupEntryDto"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/backups/{filename}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 备份文件名 */
                filename: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 备份文件 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/octet-stream": unknown;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 备份不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/backups/{filename}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 备份文件名 */
                filename: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 删除成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 备份不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/backups/{filename}/restore": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 备份文件名 */
                filename: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RestoreBackupRequest"];
            };
        };
        responses: {
            /** @description 恢复完成（建议随后重启服务） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 无权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 备份不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/certificates": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 证书列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CertificateListResponse"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/certificates": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UploadCertificateRequest"];
            };
        };
        responses: {
            /** @description 上传成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Certificate"];
                };
            };
            /** @description PEM 非法 / 域名列表非法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/certificates/issue/acme": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["IssueAcmeRequest"];
            };
        };
        responses: {
            /** @description 签发成功，返回证书记录 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Certificate"];
                };
            };
            /** @description 参数非法（泛域名未用 DNS-01 / 未配 DNS 厂商） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description ACME 服务器/验证失败 */
            502: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/certificates/issue/self-signed": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["IssueSelfSignedRequest"];
            };
        };
        responses: {
            /** @description 签发成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Certificate"];
                };
            };
            /** @description 域名列表非法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/certificates/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 证书 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 证书详情 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Certificate"];
                };
            };
            /** @description 证书记录不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PUT /api/certificates/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 证书 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateCertificateRequest"];
            };
        };
        responses: {
            /** @description 更新后的证书 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Certificate"];
                };
            };
            /** @description 自签 / 上传证书不支持开启自动续签 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 证书记录不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/certificates/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 证书 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 删除成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 证书记录不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/certificates/{id}/bind": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 证书 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["BindCertificateRequest"];
            };
        };
        responses: {
            /** @description 绑定后的证书 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Certificate"];
                };
            };
            /** @description 证书 SAN 不覆盖站点域名 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 证书/网站不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/certificates/{id}/renew": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 证书 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 续签成功，返回刷新后的证书记录 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Certificate"];
                };
            };
            /** @description 来源不支持自动续签（自签/上传）或验证配置缺失 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 证书记录不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description ACME 签发失败 */
            502: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/certificates/{id}/renewals": {
        parameters: {
            query?: {
                /** @description 返回条数：缺省 50，范围 1–200 */
                limit?: number | null;
            };
            header?: never;
            path: {
                /** @description 证书 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 续签概况与时间线（最新在前） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CertificateRenewalHistory"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 无 certificate:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 证书记录不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/databases": {
        parameters: {
            query?: {
                /** @description 页码（从 1 开始） */
                page?: number | null;
                /** @description 每页条数（1~200） */
                page_size?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 实例分页 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedResponse<DatabaseInstanceResponse>_DatabaseInstanceResponse"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 database:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PATCH /api/databases/batch-status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["BatchStatusRequest"];
            };
        };
        responses: {
            /** @description `"updated"` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 为空 / 超过 200 项 / 状态不在 running、stopped、error 之内 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 database:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/databases/mongodb/install": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["InstallMongodbRequest"];
            };
        };
        responses: {
            /** @description 已安装并登记 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DatabaseInstanceResponse"];
                };
            };
            /** @description 不支持的版本 / 发行版、密码过短或含控制字符、已安装 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 database:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 安装失败（非 root 且无 sudo 时不可用，见 Doc/10） */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/databases/mysql/install": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["InstallMysqlRequest"];
            };
        };
        responses: {
            /** @description 已安装并登记 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DatabaseInstanceResponse"];
                };
            };
            /** @description 口令含控制字符等校验失败 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 database:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 安装失败（非 root 且无 sudo 时不可用，见 Doc/10） */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/databases/postgres/install": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["InstallPostgresRequest"];
            };
        };
        responses: {
            /** @description 已安装并登记 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DatabaseInstanceResponse"];
                };
            };
            /** @description 口令含控制字符等校验失败 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 database:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 安装失败（非 root 且无 sudo 时不可用，见 Doc/10） */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/databases/redis/install": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["InstallRedisRequest"];
            };
        };
        responses: {
            /** @description 已安装并登记 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DatabaseInstanceResponse"];
                };
            };
            /** @description 口令含控制字符等校验失败 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 database:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 安装失败（非 root 且无 sudo 时不可用，见 Doc/10） */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/databases/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 实例 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DatabaseInstanceResponse"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 database:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/databases/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description `"deleted"` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 database:delete 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/databases/{id}/databases": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 库名列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": string[];
                };
            };
            /** @description Redis 不支持 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 database:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/databases/{id}/databases": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateDatabaseRequest"];
            };
        };
        responses: {
            /** @description `"created"` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 库名需为 1–64 位字母数字 / `_` / `$`；字符集不在白名单；Redis 不支持 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 database:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/databases/{id}/databases/{db_name}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
                /** @description 库名 */
                db_name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description `"dropped"` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 库名非法 / 系统库（mysql、information_schema、performance_schema、sys、postgres、template0/1）不可删除 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 database:delete 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/databases/{id}/restart": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description `"restarted"` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 database:start 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/databases/{id}/start": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description `"started"` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 database:start 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/databases/{id}/status": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 状态 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 database:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/databases/{id}/stop": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description `"stopped"` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 database:stop 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/databases/{id}/uninstall": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description `"uninstalled"` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 database:delete 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/databases/{id}/users": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DatabaseCreateUserRequest"];
            };
        };
        responses: {
            /** @description `"created"` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 用户名 / host / 口令校验失败 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 database:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/databases/{id}/users/{username}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
                /** @description 用户名 */
                username: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description `"dropped"` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 用户名非法 / root、postgres 等内置超级用户不可删除 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 database:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/dns-providers": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 账号列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DnsProviderAccountDto"][];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/dns-providers": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateDnsProviderRequest"];
            };
        };
        responses: {
            /** @description 创建成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DnsProviderAccountDto"];
                };
            };
            /** @description 参数错误 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PUT /api/dns-providers/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 账号 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateDnsProviderRequest"];
            };
        };
        responses: {
            /** @description 更新成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DnsProviderAccountDto"];
                };
            };
            /** @description 账号不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/dns-providers/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 账号 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 删除成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 账号不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/dns-providers/{id}/test": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 账号 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 测试完成，返回账号（含 last_test_status） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DnsProviderAccountDto"];
                };
            };
            /** @description 账号不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/docker/compose": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 项目 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown[];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/compose/convert-run": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ConvertRunRequest"];
            };
        };
        responses: {
            /** @description 转换结果；`warnings` 列出无法等价转换的参数 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ComposeConversion"];
                };
            };
            /** @description 不是 docker run 命令 / 含管道等 shell 结构 / 不支持的参数 / 超过 16 KiB */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/compose/deploy": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ComposeDeployRequest"];
            };
        };
        responses: {
            /** @description `{project_name, path, status}` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description 项目名非法 / YAML 为空或超过 256 KiB */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description docker compose up 失败 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/compose/{project_name}/down": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Compose 项目名 */
                project_name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 成功（`null`） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 项目名非法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:stop 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/compose/{project_name}/up": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Compose 项目名 */
                project_name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 成功（`null`） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 项目名非法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:start 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/docker/containers": {
        parameters: {
            query?: {
                /** @description 节点 ID，缺省 0 = 本机 */
                node_id?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 容器 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DockerContainer"][];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/containers/prune": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 清理结果 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:delete 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/docker/containers/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 容器 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DockerContainer"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 容器不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/docker/containers/{id}/compose": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 转换结果（YAML 仅供审阅，不会部署） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ComposeConversion"];
                };
            };
            /** @description 引用含非法字符 / 容器信息缺少镜像 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/docker/containers/{id}/inspect": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 详情 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/containers/{id}/kill": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 成功（`null`） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:start 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/docker/containers/{id}/logs": {
        parameters: {
            query?: {
                /** @description 尾行数，缺省 100，钳制到 1–10000 */
                tail?: number;
            };
            header?: never;
            path: {
                /** @description 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 日志文本 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/containers/{id}/pause": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 成功（`null`） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:start 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/containers/{id}/remove": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 成功（`null`） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:delete 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/containers/{id}/rename": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DockerRenameRequest"];
            };
        };
        responses: {
            /** @description 成功（`null`） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description ID 或新名称非法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/containers/{id}/restart": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 成功（`null`） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:start 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/containers/{id}/start": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 成功（`null`） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:start 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/docker/containers/{id}/stats": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 统计 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/containers/{id}/stop": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 成功（`null`） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:stop 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/containers/{id}/unpause": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 成功（`null`） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:start 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/docker/images": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 镜像 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown[];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/images/prune": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 清理结果 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:delete 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/images/pull": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PullImageRequest"];
            };
        };
        responses: {
            /** @description 拉取结果 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description 镜像引用非法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:start 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/images/{id}/remove": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 镜像 ID 或引用 */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 成功（`null`） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:delete 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/images/{id}/tag": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 镜像 ID 或引用 */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["TagImageRequest"];
            };
        };
        responses: {
            /** @description 成功（`null`） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 镜像 / 仓库 / tag 非法；repo 含 tag 或 digest */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/docker/networks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 网络 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown[];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/networks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateNetworkRequest"];
            };
        };
        responses: {
            /** @description 创建结果 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description 名称 / 驱动 / 子网（CIDR）非法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/networks/prune": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 清理结果 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:delete 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/docker/networks/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 网络 ID 或名称 */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 成功（`null`） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:delete 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/networks/{id}/connect": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 网络 ID 或名称 */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ConnectNetworkRequest"];
            };
        };
        responses: {
            /** @description 成功（`null`） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/networks/{id}/disconnect": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 网络 ID 或名称 */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ConnectNetworkRequest"];
            };
        };
        responses: {
            /** @description 成功（`null`） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/docker/volumes": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 卷 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown[];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/volumes": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateVolumeRequest"];
            };
        };
        responses: {
            /** @description 创建结果 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description 名称 / 驱动非法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/docker/volumes/prune": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 清理结果 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:delete 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/docker/volumes/{name}": {
        parameters: {
            query?: {
                /** @description 强制删除 */
                force?: boolean;
            };
            header?: never;
            path: {
                /** @description 卷名 */
                name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 成功（`null`） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 docker:delete 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/files": {
        parameters: {
            query?: {
                /** @description 目录（根内相对路径，缺省 `/`） */
                path?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 目录条目（目录在前，按名称排序；回收站目录不出现） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FileInfo"][];
                };
            };
            /** @description 不是目录 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 file:read 权限或路径越界 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 路径不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/files/chmod": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ChmodRequest"];
            };
        };
        responses: {
            /** @description 已修改 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 权限值非法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 file:write 权限或路径越界 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 路径不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/files/compress": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CompressRequest"];
            };
        };
        responses: {
            /** @description 已生成压缩包 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CompressResult"];
                };
            };
            /** @description 未选择条目 / 条目不在同一目录 / 扩展名不支持 / 超出 zip 限制 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 file:write 权限或路径越界 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 条目不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 压缩包已存在（未指定 overwrite） */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/files/copy": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["TransferRequest"];
            };
        };
        responses: {
            /** @description 逐项结果（部分失败仍为 200，见 items[].ok / error_kind） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TransferResult"];
                };
            };
            /** @description 未选择条目 / 超过 500 项 / 目标不是目录 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 file:write 权限或目标目录越界 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 目标目录不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/files/create-dir": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateRequest"];
            };
        };
        responses: {
            /** @description 已创建 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 路径非法或父目录不存在 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 file:write 权限或路径越界 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 同名条目已存在 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/files/create-file": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateRequest"];
            };
        };
        responses: {
            /** @description 已创建 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 路径非法或父目录不存在 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 file:write 权限或路径越界 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 同名条目已存在 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/files/delete": {
        parameters: {
            query: {
                path: string;
                /** @description 非空目录必须为 true */
                recursive?: boolean | null;
                /** @description true = 直接删除，不进回收站 */
                permanent?: boolean | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已删除（或已移入回收站） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 目录非空且未确认递归 / 位于其他文件系统无法移入回收站 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 file:write 权限、路径越界或试图删除根目录 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 路径不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/files/download": {
        parameters: {
            query: {
                /** @description 文件路径（根内相对路径） */
                path: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 文件内容 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/octet-stream": unknown;
                };
            };
            /** @description 不是文件 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 file:upload 权限或路径越界 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 路径不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/files/extract": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ExtractRequest"];
            };
        };
        responses: {
            /** @description 已解压 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ExtractResult"];
                };
            };
            /** @description 格式不支持 / 压缩包损坏 / 含路径穿越、包外符号链接、硬链接 / 超出体积与条目上限 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 file:write 权限或路径越界 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 压缩包不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 目标目录已有同名顶层条目（未指定 overwrite，未做任何改动） */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/files/move": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["TransferRequest"];
            };
        };
        responses: {
            /** @description 逐项结果（部分失败仍为 200，见 items[].ok / error_kind） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TransferResult"];
                };
            };
            /** @description 未选择条目 / 超过 500 项 / 目标不是目录 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 file:write 权限或目标目录越界 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 目标目录不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/files/read": {
        parameters: {
            query: {
                /** @description 文件路径（根内相对路径） */
                path: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 文件内容 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 不是文件 / 超过 10 MiB / 不是 UTF-8 文本 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 file:read 权限或路径越界 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 路径不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/files/recycle": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 回收站条目（新删除的在前；过期条目自动清理） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RecycleItem"][];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 file:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/files/recycle": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已清空 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PurgeResult"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 file:write 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/files/recycle/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 回收站条目 ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已彻底删除 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description ID 非法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 file:write 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 条目不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/files/recycle/{id}/restore": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 回收站条目 ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已还原 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RecycleItem"];
                };
            };
            /** @description ID 非法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 file:write 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 条目不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 原位置已存在同名条目 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/files/rename": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RenameRequest"];
            };
        };
        responses: {
            /** @description 已重命名 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 路径非法或把目录移入自身 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 file:write 权限或路径越界 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 源路径不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 目标已存在 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/files/upload": {
        parameters: {
            query: {
                /** @description 目标目录 */
                path: string;
                /** @description 文件名（不含路径） */
                name: string;
                /** @description 同名文件存在时是否覆盖（缺省 false → 409） */
                overwrite?: boolean | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description 文件原始字节（不是 multipart） */
        requestBody: {
            content: {
                "application/octet-stream": number[];
            };
        };
        responses: {
            /** @description 已上传 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 文件名非法 / 目标不是目录 / 超过 2 GiB */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 file:upload 权限或路径越界 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 同名文件已存在（未指定 overwrite） */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/files/write": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["WriteRequest"];
            };
        };
        responses: {
            /** @description 已保存，返回新的修改时间 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WriteResult"];
                };
            };
            /** @description 不是文件 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 file:write 权限或路径越界 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 路径不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 文件在打开后已被修改 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/firewall/apply": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description `"applied"` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 firewall:apply 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 防火墙后端执行失败 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/firewall/disable": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description `"disabled"` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 firewall:enable 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/firewall/enable": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description `"enabled"` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 firewall:enable 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/firewall/reorder": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ReorderRequest"];
            };
        };
        responses: {
            /** @description `"reordered"` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 firewall:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/firewall/rules": {
        parameters: {
            query?: {
                /** @description 页码（从 1 开始） */
                page?: number | null;
                /** @description 每页条数（1~200） */
                page_size?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 规则分页 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedResponse<FirewallRuleResponse>_FirewallRuleResponse"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 firewall:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/firewall/rules": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateFirewallRuleRequest"];
            };
        };
        responses: {
            /** @description 已创建 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FirewallRuleResponse"];
                };
            };
            /** @description 端口 / 地址 / 协议 / 方向校验失败 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 firewall:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/firewall/rules/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 规则 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 规则 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FirewallRuleResponse"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 firewall:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 规则不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PUT /api/firewall/rules/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 规则 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateFirewallRuleRequest"];
            };
        };
        responses: {
            /** @description 已更新 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FirewallRuleResponse"];
                };
            };
            /** @description 端口 / 地址 / 协议 / 方向校验失败 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 firewall:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 规则不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/firewall/rules/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 规则 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description `"deleted"` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 firewall:delete 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 规则不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/firewall/rules/{id}/toggle": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 规则 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ToggleRequest"];
            };
        };
        responses: {
            /** @description 切换后的规则 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FirewallRuleResponse"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 firewall:enable 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 规则不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/firewall/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 状态键值 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        [key: string]: string;
                    };
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 firewall:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/health": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 健康状态 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["HealthDetail"];
                };
            };
        };
    };
    "GET /api/iac/export": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 无凭据配置清单；无读权限的资源区块省略，不是可直接执行的部署声明 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/yaml": components["schemas"]["IacManifest"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 settings:read */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/inspection": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 系统体检报告 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InspectionReport"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 toolbox:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/inspection/run": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 新生成的体检报告 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InspectionReport"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 toolbox:execute 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/kernel-logs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 内核日志概览 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["KernelLogOverview"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 log:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/kernel-logs/entries": {
        parameters: {
            query?: {
                /** @description 文件名；缺省为当前正在写入的文件 */
                file?: string | null;
                /** @description 级别（逗号分隔，如 `warn,error`）；缺省不过滤 */
                levels?: string | null;
                /** @description 关键字（不区分大小写，匹配整行，含请求编号 / 用户名等 span 字段） */
                q?: string | null;
                /** @description target 前缀，如 `flame_kernel::api` */
                target?: string | null;
                /** @description 返回条数：缺省 200，范围 1–1000 */
                limit?: number | null;
                /** @description 续读游标（上一页的 `next_before`） */
                before?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 日志分页（最新在前） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LogPage"];
                };
            };
            /** @description 文件名或级别非法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 log:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 文件不存在或文件日志未启用 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/kernel-logs/files/{name}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 日志文件名 */
                name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已删除 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 文件名非法或为当前正在写入的文件 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 log:delete 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 文件不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/kernel-logs/files/{name}/download": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 日志文件名 */
                name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 原始 JSON Lines 文本 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": unknown;
                };
            };
            /** @description 文件名非法或文件过大 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 log:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 文件不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PUT /api/kernel-logs/level": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SetKernelLogLevelRequest"];
            };
        };
        responses: {
            /** @description 新级别已生效 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["KernelLogLevelResponse"];
                };
            };
            /** @description 级别非法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 settings:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 日志系统未初始化（嵌入式 / 测试进程） */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/kernel-logs/stats": {
        parameters: {
            query: {
                /** @description 文件名 */
                file: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 级别计数 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["KernelLogStats"];
                };
            };
            /** @description 文件名非法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 log:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 文件不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/logs": {
        parameters: {
            query?: {
                /** @description 页码（从 1 开始） */
                page?: number | null;
                /** @description 每页条数（1~200） */
                page_size?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 系统事件日志（分页，新在前） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedResponse<LogEntry>_LogEntry"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 无权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/logs/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 删除成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 无权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 日志不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/mcp": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 不提供服务端推送流 */
            405: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/mcp": {
        parameters: {
            query?: never;
            header?: {
                /** @description initialize 下发的会话 ID；缺省为无状态模式 */
                "Mcp-Session-Id"?: string | null;
            };
            path?: never;
            cookie?: never;
        };
        /** @description JSON-RPC 2.0 请求 / 通知（或其数组，≤ 32 条） */
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description JSON-RPC 响应（initialize 时响应头含 Mcp-Session-Id） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description 仅通知 / 响应，无响应体 */
            202: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description JSON 解析失败或非法批量 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description fpk_ 凭证缺失 / 无效 / 吊销 / 过期 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Origin 与 Host 不一致 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 会话不存在或已终止 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/mcp": {
        parameters: {
            query?: never;
            header: {
                /** @description 要终止的会话 ID */
                "Mcp-Session-Id": string;
            };
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 会话已终止 */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 Mcp-Session-Id */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description fpk_ 凭证无效 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 会话不存在或已终止 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/mcp/approvals": {
        parameters: {
            query?: {
                /** @description 按状态过滤：pending / executing / succeeded / failed / rejected / expired */
                status?: string | null;
                principal_id?: number | null;
                page?: number | null;
                page_size?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 审批单分页（最新在前） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedResponse<ToolApproval>_ToolApproval"];
                };
            };
            /** @description status 不合法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 mcp:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/mcp/approvals/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 审批单 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 审批单 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ToolApproval"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 mcp:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/mcp/approvals/{id}/approve": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 审批单 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已执行；status 为 succeeded 或 failed，result 为输出或错误 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ToolApproval"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 mcp:update，或缺少工具本身权限 / 高危工具的 mcp:delete */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 非待审批状态、已过期、凭证失效或签发者权限已变更 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/mcp/approvals/{id}/reject": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 审批单 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RejectRequest"];
            };
        };
        responses: {
            /** @description 已拒绝 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ToolApproval"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 mcp:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 非待审批状态 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/mcp/calls": {
        parameters: {
            query?: {
                principal_id?: number | null;
                session_id?: string | null;
                tool?: string | null;
                page?: number | null;
                page_size?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 调用记录分页（最新在前） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedResponse<ToolCallRecord>_ToolCallRecord"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 mcp:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/mcp/principals": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 凭证列表（不含令牌） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PrincipalView"][];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 mcp:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/mcp/principals": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreatePrincipalRequest"];
            };
        };
        responses: {
            /** @description 已签发；token 仅此一次返回 */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CreatePrincipalResponse"];
                };
            };
            /** @description 名称或有效期不合法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 mcp:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/mcp/principals/{id}/revoke": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 凭证 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已吊销（幂等） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PrincipalView"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 mcp:create 权限，或吊销他人凭证缺少 mcp:delete */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 凭证不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/mcp/prompts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 技能列表（与 MCP prompts/list 一致） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["McpPromptInfo"][];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 mcp:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/mcp/prompts/{name}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 技能名称 */
                name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 渲染后的提示词 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["McpPromptRendered"];
                };
            };
            /** @description 缺少必填参数 / 未声明的参数 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 mcp:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 技能不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/mcp/sessions": {
        parameters: {
            query?: {
                /** @description 按凭证过滤 */
                principal_id?: number | null;
                /** @description 返回条数（1–500，缺省 100） */
                limit?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 会话列表（最近活跃在前） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AgentSession"][];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 mcp:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/mcp/tools": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 工具目录 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ToolInfo"][];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 mcp:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/memos": {
        parameters: {
            query?: {
                /** @description `memo` | `todo` */
                kind?: string | null;
                done?: boolean | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 备忘 / 待办列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Memo"][];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/memos": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateMemoRequest"];
            };
        };
        responses: {
            /** @description 新建的条目 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Memo"];
                };
            };
            /** @description 内容为空 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PUT /api/memos/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateMemoRequest"];
            };
        };
        responses: {
            /** @description 更新后的条目 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Memo"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 条目不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/memos/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 删除成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 条目不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/metrics/history": {
        parameters: {
            query?: {
                /** @description 时间范围：`1h` / `6h` / `24h`（1 min 点）或 `7d` / `30d`（1 h 点）；缺省 `24h` */
                range?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 监控历史点列 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MetricsHistoryResponse"];
                };
            };
            /** @description range 非法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/metrics/processes": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 进程 TOP 列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProcessEntry"][];
                };
            };
        };
    };
    "GET /api/nodes": {
        parameters: {
            query?: {
                /** @description 页码（从 1 开始） */
                page?: number | null;
                /** @description 每页条数（1~200） */
                page_size?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 节点列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedResponse<ServerNode>_ServerNode"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/nodes": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateNodeRequest"];
            };
        };
        responses: {
            /** @description 注册成功，返回节点 ID */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": number;
                };
            };
            /** @description 参数错误 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/nodes/batch-execute": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RemoteBatchExecRequest"];
            };
        };
        responses: {
            /** @description 各节点执行结果聚合（agent 模式下受命令白名单约束） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/nodes/heartbeat/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 节点 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["HeartbeatRequest"];
            };
        };
        responses: {
            /** @description 心跳记录成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Agent token 无效 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/nodes/register": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateNodeRequest"];
            };
        };
        responses: {
            /** @description 注册成功，返回节点 ID */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description 参数错误 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PUT /api/nodes/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateNodeRequest"];
            };
        };
        responses: {
            /** @description 更新后的节点 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ServerNode"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 无权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 节点不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/nodes/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 删除成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 无权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 节点不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/nodes/{id}/action": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 节点 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RemoteActionRequest"];
            };
        };
        responses: {
            /** @description 动作结果 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 非法动作或参数 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/nodes/{id}/execute": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 节点 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RemoteExecRequest"];
            };
        };
        responses: {
            /** @description 执行结果（agent 模式下受命令白名单约束） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/nodes/{id}/files": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 节点 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 远程目录列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/nodes/{id}/files/download": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 节点 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 文件内容（base64） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/nodes/{id}/files/upload": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 节点 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RemoteUploadRequest"];
            };
        };
        responses: {
            /** @description 上传结果 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/nodes/{id}/metrics": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 最近一次心跳上报的指标（结构随 Agent 版本演进） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 无权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 节点不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/nodes/{id}/metrics/history": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 节点 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 按时间升序的快照（未收到过心跳为空数组） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MetricsSnapshot"][];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 node:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 节点不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/nodes/{id}/status": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 节点 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 在线状态 `{id, status, edge: EdgeInfo}` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 节点不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/operation-logs": {
        parameters: {
            query?: {
                /** @description 页码（从 1 开始） */
                page?: number | null;
                /** @description 每页条数（1~200） */
                page_size?: number | null;
                /** @description 按 action 前缀过滤（如 `action=LOGIN` 匹配 LOGIN_SUCCESS/LOGIN_FAILED） */
                action?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 操作审计日志（分页，新在前） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedResponse<OperationLog>_OperationLog"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 无权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/operation-logs/export": {
        parameters: {
            query?: {
                format?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 导出文件 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/operation-logs/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 删除成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 无权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 日志不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/outbox-events": {
        parameters: {
            query?: {
                page?: number | null;
                page_size?: number | null;
                /** @description 按事件类型过滤（如 `type=AppInstalled` 精确匹配变体名） */
                type?: string | null;
            };
            header?: never;
            path: {
                /** @description 页码（从 1 开始） */
                page: number | null;
                /** @description 每页条数（1~200） */
                page_size: number | null;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 事件落库分页列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedResponse<OutboxEvent>_OutboxEvent"];
                };
            };
        };
    };
    "GET /api/overview/badges": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 顶栏徽标聚合快照 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["OverviewBadges"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/plugins": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 插件列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PluginResponse"][];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 plugin:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/plugins": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["LoadPluginRequest"];
            };
        };
        responses: {
            /** @description 插件 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PluginResponse"];
                };
            };
            /** @description base64 / WASM 非法、为空；ID、内存或超时越界 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 plugin:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/plugins/market": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 市场目录（内置 + 远程） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MarketListResponse"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 plugin:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/plugins/market/{id}/install": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 市场插件 ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["MarketInstallRequest"];
            };
        };
        responses: {
            /** @description 已安装 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PluginResponse"];
                };
            };
            /** @description 未同意能力申请 / 包体哈希不符 / WASM 非法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 plugin:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 市场中无此插件 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 同 ID 插件已安装 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 远程市场不可达或 WASM 支持未编译 */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/plugins/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 插件 ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 插件 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PluginResponse"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 plugin:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 插件不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/plugins/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 插件 ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已卸载 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 plugin:delete 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 插件不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PUT /api/plugins/{id}/capabilities": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 插件 ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateCapabilitiesRequest"];
            };
        };
        responses: {
            /** @description 插件 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PluginResponse"];
                };
            };
            /** @description 能力非法（未知能力 / 敏感目录 / 超过 16 项） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 plugin:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 插件不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/plugins/{id}/disable": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 插件 ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 插件 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PluginResponse"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 plugin:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 插件不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/plugins/{id}/enable": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 插件 ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 插件 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PluginResponse"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 plugin:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 插件不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/plugins/{id}/execute/{function}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 插件 ID */
                id: string;
                /** @description 导出函数名 */
                function: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ExecutePluginRequest"];
            };
        };
        responses: {
            /** @description 执行结果 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ExecutionResponse"];
                };
            };
            /** @description 插件已禁用 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 plugin:execute 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 插件不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/plugins/{id}/metrics": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 插件 ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 指标 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PluginMetricsResponse"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 plugin:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 插件不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/plugins/{id}/metrics": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 插件 ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已重置 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 plugin:config 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 插件不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/plugins/{id}/reload": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 插件 ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PluginReloadRequest"];
            };
        };
        responses: {
            /** @description 插件 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PluginResponse"];
                };
            };
            /** @description base64 / WASM 非法；内存或超时越界 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 plugin:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 插件不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/plugins/{id}/settings": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 插件 ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 设置 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        [key: string]: string;
                    };
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 plugin:config 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 插件不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/plugins/{id}/settings": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 插件 ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PluginSettingRequest"];
            };
        };
        responses: {
            /** @description 已保存（message / id / key） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description key 须 1–128 位 `[A-Za-z0-9._-]`；value ≤ 4096 字节 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 plugin:config 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 插件不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/plugins/{id}/settings/{key}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 插件 ID */
                id: string;
                /** @description 设置键 */
                key: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 设置值 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 plugin:config 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 插件不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/projects": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 项目列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProjectDto"][];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/projects": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateProjectRequest"];
            };
        };
        responses: {
            /** @description 创建成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProjectDto"];
                };
            };
            /** @description 参数校验失败 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 项目重名 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/projects/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 项目 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 项目详情 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProjectDto"];
                };
            };
            /** @description 项目不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PUT /api/projects/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 项目 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateProjectRequest"];
            };
        };
        responses: {
            /** @description 更新成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProjectDto"];
                };
            };
            /** @description 项目不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 运行中禁止修改启动命令 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/projects/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 项目 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 删除成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 项目不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/projects/{id}/builder": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 项目 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 构建器决策 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["BuilderDecisionDto"];
                };
            };
            /** @description 项目不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/projects/{id}/deploy": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 项目 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 部署成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProjectDto"];
                };
            };
            /** @description 来源校验失败 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 项目操作互斥 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/projects/{id}/health-check": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 项目 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 探测结果 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["HealthCheckDto"];
                };
            };
            /** @description 未配置健康检查路径或端口 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 健康探测器不可用 */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/projects/{id}/links": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 项目 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 联链列表（不含连接串） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProjectDbLinkDto"][];
                };
            };
            /** @description 项目不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 联链组件未启用 */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/projects/{id}/links": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 项目 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["LinkDatabaseRequest"];
            };
        };
        responses: {
            /** @description 关联成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProjectDbLinkDto"];
                };
            };
            /** @description 参数校验失败 / 实例未运行 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 项目或数据库实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 环境变量名冲突或已关联该实例 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 联链组件未启用 */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/projects/{id}/links/available": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 项目 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 可关联实例清单 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LinkableDatabaseDto"][];
                };
            };
            /** @description 项目不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/projects/{id}/links/{link_id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 项目 ID */
                id: number;
                /** @description 联链 ID */
                link_id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 解除成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 项目或联链不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 联链组件未启用 */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/projects/{id}/logs": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 项目 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 日志列表（容器模式取 docker logs） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LogLine"][];
                };
            };
        };
    };
    "GET /api/projects/{id}/procfile": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 项目 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 进程声明（无 Procfile 为空表） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProcfileDto"];
                };
            };
            /** @description 项目不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/projects/{id}/restart": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 项目 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 重启成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProjectDto"];
                };
            };
        };
    };
    "POST /api/projects/{id}/start": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 项目 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 启动成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProjectDto"];
                };
            };
            /** @description 未配置启动命令 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/projects/{id}/status": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 项目 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 进程状态（容器模式自容器探测；两路同构） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProcessStatus"];
                };
            };
            /** @description 项目不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/projects/{id}/stop": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 项目 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 停止成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProjectDto"];
                };
            };
        };
    };
    "POST /api/projects/{id}/webhook": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 项目 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: {
            content: {
                "application/json": unknown;
            };
        };
        responses: {
            /** @description 部署已触发 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProjectDto"];
                };
            };
            /** @description 令牌缺失/无效 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 项目未启用 webhook */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 同项目操作互斥 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/projects/{id}/webhook/disable": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 项目 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已关闭 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProjectDto"];
                };
            };
            /** @description 项目不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/projects/{id}/webhook/enable": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 项目 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已开启（重复调用轮换令牌） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WebhookDeployDto"];
                };
            };
            /** @description 非 git 来源项目 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 项目不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/runtimes": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 运行时列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RuntimeDto"][];
                };
            };
        };
    };
    "GET /api/runtimes/available": {
        parameters: {
            query: {
                /** @description 语言种类：php | java | node | python | go */
                kind: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 版本清单 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RuntimeVersionInfo"][];
                };
            };
            /** @description 不支持的语言 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/runtimes/install": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["InstallRuntimeRequest"];
            };
        };
        responses: {
            /** @description 安装成功（或已登记并安装中） */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RuntimeDto"];
                };
            };
            /** @description 参数校验失败 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 已安装 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/runtimes/kind/{kind}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 语言种类 */
                kind: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 该语言运行时列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RuntimeDto"][];
                };
            };
        };
    };
    "GET /api/runtimes/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 运行时 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 运行时详情 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RuntimeDto"];
                };
            };
            /** @description 运行时不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/runtimes/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 运行时 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 卸载成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/runtimes/{id}/default": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 运行时 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 设置成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RuntimeDto"];
                };
            };
            /** @description 未安装 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/runtimes/{id}/sites": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 运行时 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 绑定该运行时的站点列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WebsiteDto"][];
                };
            };
            /** @description 运行时不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/scheduled-tasks": {
        parameters: {
            query?: {
                /** @description 页码（从 1 开始） */
                page?: number | null;
                /** @description 每页条数（1~200） */
                page_size?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 任务列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedResponse<ScheduledTask>_ScheduledTask"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/scheduled-tasks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateTaskRequest"];
            };
        };
        responses: {
            /** @description 创建成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ScheduledTask"];
                };
            };
            /** @description 参数错误 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/scheduled-tasks/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 任务 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 任务详情 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ScheduledTask"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 任务不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PUT /api/scheduled-tasks/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 任务 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateTaskRequest"];
            };
        };
        responses: {
            /** @description 更新成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ScheduledTask"];
                };
            };
            /** @description 任务不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/scheduled-tasks/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 任务 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 删除成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 任务不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/scheduled-tasks/{id}/run": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 任务 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 执行结果 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ScheduledTask"];
                };
            };
            /** @description 任务不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/scheduled-tasks/{id}/toggle": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 任务 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ToggleTaskRequest"];
            };
        };
        responses: {
            /** @description 更新后的任务 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ScheduledTask"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 任务不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/settings": {
        parameters: {
            query?: {
                /** @description 页码（从 1 开始） */
                page?: number | null;
                /** @description 每页条数（1~200） */
                page_size?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 设置列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedResponse<SettingEntry>_SettingEntry"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PUT /api/settings": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateSettingRequest"];
            };
        };
        responses: {
            /** @description 更新成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 值不合法 / 保留键 / 保存后会锁定当前访问来源 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/settings/access": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 当前访问来源与生效策略 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AccessInfo"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 无权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PATCH /api/settings/batch": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateSettingsBatchRequest"];
            };
        };
        responses: {
            /** @description 批量更新成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 请求体为空 / 值不合法（一次列出全部错误）/ 保存后会锁定当前访问来源 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/settings/{key}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 设置键 */
                key: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 设置项 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SettingEntry"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 设置项不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/setup/initialize": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SetupRequest"];
            };
        };
        responses: {
            /** @description 步骤完成；`admin` 步骤返回登录令牌 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InitializeResponse"];
                };
            };
            /** @description 参数错误 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 面板已初始化 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/setup/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 初始化状态与环境探测 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SetupStatusResponse"];
                };
            };
        };
    };
    "GET /api/tasks": {
        parameters: {
            query?: {
                /** @description 按状态过滤：pending/running/success/failed/cancelled */
                state?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 任务列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TaskListResponse"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/tasks/prune": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 清理数量 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/tasks/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 任务 id */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 任务详情 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TaskRecord"];
                };
            };
            /** @description 任务不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/tasks/{id}/cancel": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 任务 id */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已取消 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TaskRecord"];
                };
            };
            /** @description 任务不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 任务已处于终态，无法取消 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/toolbox/host": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 主机详情：系统 / CPU / 内存 / 时区 / DNS / hosts / 网卡 / 磁盘 / 面板自身 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["HostDetails"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 toolbox:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/toolbox/processes": {
        parameters: {
            query?: {
                /** @description 关键字：pid 精确匹配，或进程名 / 用户 / 命令行包含（不区分大小写） */
                q?: string | null;
                /** @description 仅显示该用户的进程 */
                user?: string | null;
                /** @description 排序字段：`cpu`（缺省）/ `memory` / `pid` / `name` / `started` */
                sort?: string | null;
                /** @description 排序方向：`desc`（缺省）/ `asc` */
                order?: string | null;
                /** @description 返回条数：缺省 100，范围 1–500 */
                limit?: number | null;
                /** @description 是否包含内核线程（缺省 false） */
                include_kernel?: boolean | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 进程列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProcessListResponse"];
                };
            };
            /** @description sort / order 非法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 toolbox:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/toolbox/processes/{pid}/kill": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 目标进程 pid */
                pid: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["KillProcessRequest"];
            };
        };
        responses: {
            /** @description 信号已送达 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["KillProcessResponse"];
                };
            };
            /** @description 受保护进程（init / 内核线程 / 面板自身及祖先）或请求体非法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 toolbox:execute 权限；或面板系统权限不足（message 含 insufficient privilege） */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 进程不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description started_at 不一致（pid 已被复用） */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/users": {
        parameters: {
            query?: {
                /** @description 页码（从 1 开始） */
                page?: number | null;
                /** @description 每页条数（1~200） */
                page_size?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 用户列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedResponse<User>_User"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/users": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateUserRequest"];
            };
        };
        responses: {
            /** @description 创建成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["User"];
                };
            };
            /** @description 参数错误 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PUT /api/users/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 用户 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateUserRequest"];
            };
        };
        responses: {
            /** @description 更新成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["User"];
                };
            };
            /** @description 用户不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/users/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 用户 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 删除成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 用户不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PUT /api/users/{id}/2fa/reset": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 用户 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已重置 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 user:update 权限或非 admin 角色 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 用户不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/waf/apply": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 逐引擎下发报告 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WafApplyReport"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 无权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未配置 Web 引擎下发 */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/waf/logs": {
        parameters: {
            query: {
                pagination: components["schemas"]["PaginationParams"];
                /** @description 过滤网站 ID（0 = 全部） */
                website_id?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 拦截日志分页 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedResponse<WafLog>_WafLog"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/waf/logs/clear": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ClearLogsRequest"];
            };
        };
        responses: {
            /** @description 清理条数 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": number;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/waf/rules": {
        parameters: {
            query?: {
                /** @description 网站 ID 过滤（0 = 全部） */
                website_id?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 规则列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WafRule"][];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/waf/rules": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateWafRuleRequest"];
            };
        };
        responses: {
            /** @description 创建成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WafRule"];
                };
            };
            /** @description 参数错误 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/waf/rules/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 规则 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 规则详情 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WafRule"];
                };
            };
            /** @description 规则不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PUT /api/waf/rules/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 规则 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateWafRuleRequest"];
            };
        };
        responses: {
            /** @description 更新成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WafRule"];
                };
            };
            /** @description 规则不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/waf/rules/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 规则 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 删除成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 规则不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/waf/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 引擎状态 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WafEngineStatus"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/web-servers": {
        parameters: {
            query?: {
                /** @description 页码（从 1 开始） */
                page?: number | null;
                /** @description 每页条数（1~200） */
                page_size?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 实例分页 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedResponse<WebServerResponse>_WebServerResponse"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/web-servers": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateWebServerInstanceRequest"];
            };
        };
        responses: {
            /** @description 已登记 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WebServerResponse"];
                };
            };
            /** @description 未知引擎 / config_path 不在引擎配置根下 / binary_path 不是该引擎的已知二进制 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/web-servers/capabilities": {
        parameters: {
            query: {
                engine: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description `{engine, capabilities: CapabilityInfo[]}` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description 未知引擎 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/web-servers/capabilities/install": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["InstallCapabilitiesRequest"];
            };
        };
        responses: {
            /** @description `{success, installed[], failed[]}` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description 未知引擎 / 未知能力（php / lua / cache / extra_modules / auto_https） */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/web-servers/engines": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 引擎列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["EngineInfo"][];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/web-servers/native/autostart": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["NativeAutostartRequest"];
            };
        };
        responses: {
            /** @description `{engine, autostart}` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description 未知引擎 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/web-servers/native/detect": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 各引擎检测结果 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["NativeWebServerInfo"][];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/web-servers/native/install": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["NativeInstallRequest"];
            };
        };
        responses: {
            /** @description 已安装并登记 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WebServerResponse"];
                };
            };
            /** @description 未知引擎 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:create 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 包管理器安装失败 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/web-servers/native/uninstall": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["NativeUninstallRequest"];
            };
        };
        responses: {
            /** @description `{deleted: true, engine}` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description 未知引擎 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:delete 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/web-servers/presets": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description `[{name, description, recommended, worker_processes}]` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>[];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/web-servers/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 实例 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WebServerResponse"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PUT /api/web-servers/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description 已更新 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WebServerResponse"];
                };
            };
            /** @description 未知引擎 / config_path 不在引擎配置根下 / binary_path 不是该引擎的已知二进制 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description resource_version 冲突 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/web-servers/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description `{deleted: true, id}` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:delete 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/web-servers/{id}/autostart": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AutostartRequest"];
            };
        };
        responses: {
            /** @description `{id, engine, autostart}` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/web-servers/{id}/config": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description `{id, config_path, content}` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/web-servers/{id}/configtest": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description `{output, id}` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:configtest 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/web-servers/{id}/native-status": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 检测结果 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["NativeWebServerInfo"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:read 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/web-servers/{id}/preset": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ApplyPresetRequest"];
            };
        };
        responses: {
            /** @description `{id, engine, preset, worker_processes, keepalive_timeout}` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description 未知预设 / 配置校验失败 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/web-servers/{id}/reload": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description `{message, id}` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:reload 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 引擎命令执行失败 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/web-servers/{id}/restart": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description `{message, id}` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:start 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 引擎命令执行失败 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/web-servers/{id}/start": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description `{message, id}` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:start 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 引擎命令执行失败 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/web-servers/{id}/stop": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description `{message, id}` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:stop 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 引擎命令执行失败 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/web-servers/{id}/switch-engine": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 实例 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SwitchEngineRequest"];
            };
        };
        responses: {
            /** @description 切换后的实例 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WebServerInstance"];
                };
            };
            /** @description 未知引擎 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 缺少 web_server:update 权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 实例不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/websites": {
        parameters: {
            query?: {
                /** @description 页码（从 1 开始） */
                page?: number | null;
                /** @description 每页条数（1~200） */
                page_size?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 网站列表 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedResponse<Website>_Website"];
                };
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/websites": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateWebsiteRequest"];
            };
        };
        responses: {
            /** @description 创建成功，返回网站 ID */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": number;
                };
            };
            /** @description 参数错误 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/websites/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 网站 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 网站详情 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Website"];
                };
            };
            /** @description 网站不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "PUT /api/websites/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 网站 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateWebsiteRequest"];
            };
        };
        responses: {
            /** @description 更新成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Website"];
                };
            };
            /** @description 网站不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/websites/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 网站 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 删除成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 网站不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/websites/{id}/deploy": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 网站 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 部署结果 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SiteDeployment"];
                };
            };
            /** @description 引擎未安装或配置校验失败 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 无权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 网站不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 存在同名非托管虚拟主机 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /api/websites/{id}/rewrite": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 网站 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 套用状态 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SiteRewriteStatus"];
                };
            };
            /** @description 网站不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/websites/{id}/rewrite": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 网站 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ApplyRewriteRequest"];
            };
        };
        responses: {
            /** @description 套用成功，返回规则文本 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 未知模板 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 网站不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description vhost 写盘/校验/reload 失败（套用已回滚） */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "DELETE /api/websites/{id}/rewrite": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 网站 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已清除 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SiteRewriteStatus"];
                };
            };
            /** @description 网站不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description vhost 写盘/校验/reload 失败（清除已回滚） */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/websites/{id}/runtime": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 网站 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["BindRuntimeRequest"];
            };
        };
        responses: {
            /** @description 绑定成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Website"];
                };
            };
            /** @description 参数错误/运行时或项目校验失败 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 网站不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "POST /api/websites/{id}/switch-engine": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 网站 ID */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SwitchWebsiteEngineRequest"];
            };
        };
        responses: {
            /** @description 切换后的网站 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Website"];
                };
            };
            /** @description 目标引擎不可用 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未认证 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 无权限 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 网站不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    "GET /health": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description OK */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
        };
    };
    "GET /metrics": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Prometheus 文本格式指标 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": unknown;
                };
            };
        };
    };
}

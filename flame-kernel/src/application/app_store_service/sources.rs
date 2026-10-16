//! 远程应用源（M-A Issue 1.2）：源 CRUD、索引同步、远程包拉取与安装时验签。

use super::*;
use crate::utils::sync::lock_or_recover;

impl AppStoreService {
    /// 远程源缓存根目录：`data/apps/remote`
    pub fn default_remote_cache_dir() -> PathBuf {
        PathBuf::from("data/apps/remote")
    }

    /// 添加远程源（url 去重由仓储 UNIQUE 约束保证）。
    ///
    /// `confirm_unsigned`：未配置公钥的源须显式确认为 true（Doc/20 §4.2
    /// 「无签名的第三方源默认弹风险确认」——前端二次确认 + 后端强制门禁）。
    pub async fn create_source(
        &self,
        name: &str,
        source_type: &str,
        url: &str,
        public_key: Option<String>,
        confirm_unsigned: bool,
    ) -> Result<AppSource, AppError> {
        if name.trim().is_empty() {
            return Err(AppError::ValidationError("源名称不能为空".into()));
        }
        if !AppSource::is_valid_source_type(source_type) {
            return Err(AppError::ValidationError(format!(
                "非法源类型: {}（仅支持 git | http）",
                source_type
            )));
        }
        if url.trim().is_empty() {
            return Err(AppError::ValidationError("源 url 不能为空".into()));
        }
        self.ensure_unsigned_source_allowed(public_key.as_deref(), confirm_unsigned)
            .await?;
        let source = AppSource {
            id: 0,
            name: name.trim().to_string(),
            source_type: source_type.to_string(),
            url: url.trim().to_string(),
            enabled: true,
            public_key,
            last_synced_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let id = self.source_repo.create(&source).await?;
        Ok(AppSource { id, ..source })
    }

    /// 远程源列表。
    pub async fn list_sources(&self) -> Result<Vec<AppSource>, AppError> {
        self.source_repo.list_all().await
    }

    /// 更新远程源（仅允许改 name / url / enabled / public_key）。
    ///
    /// `confirm_unsigned`：把源的公钥**清空**（签名源降级为未签名源）须显式确认为
    /// true——升级路径同样受门禁约束，避免绕过创建期校验卸载信任锚。
    pub async fn update_source(
        &self,
        id: i64,
        name: Option<String>,
        url: Option<String>,
        enabled: Option<bool>,
        public_key: Option<Option<String>>,
        confirm_unsigned: bool,
    ) -> Result<AppSource, AppError> {
        let mut source = self
            .source_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("远程源不存在: {}", id)))?;
        if let Some(n) = name {
            if n.trim().is_empty() {
                return Err(AppError::ValidationError("源名称不能为空".into()));
            }
            source.name = n.trim().to_string();
        }
        if let Some(u) = url {
            if u.trim().is_empty() {
                return Err(AppError::ValidationError("源 url 不能为空".into()));
            }
            // url 变更需查重（UNIQUE 约束语义前置校验，给出友好错误）
            if let Some(conflict) = self.source_repo.find_by_url(u.trim()).await? {
                if conflict.id != id {
                    return Err(AppError::Conflict(format!(
                        "该 url 已被源 {} 使用",
                        conflict.name
                    )));
                }
            }
            source.url = u.trim().to_string();
        }
        if let Some(e) = enabled {
            source.enabled = e;
        }
        if let Some(pk) = public_key {
            // 降级守卫：签名的源被清空公钥等同「卸载信任锚」，须显式确认
            let was_signed = !Self::is_unsigned_source(&source);
            let will_be_unsigned = pk.as_deref().unwrap_or("").trim().is_empty();
            if was_signed && will_be_unsigned {
                self.ensure_unsigned_source_allowed(None, confirm_unsigned)
                    .await?;
            }
            source.public_key = pk;
        }
        self.source_repo.update(&source).await?;
        Ok(source)
    }

    /// 删除远程源（不删除已同步的应用包索引，由用户显式管理）。
    pub async fn delete_source(&self, id: i64) -> Result<(), AppError> {
        self.source_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("远程源不存在: {}", id)))?;
        self.source_repo.delete(id).await
    }

    /// 同步远程源索引：经 `RemoteAppSource` 端口拉取索引 → `create_many` 幂等落库 →
    /// 触达 last_synced_at。同一源并发同步互斥。
    pub async fn sync_source(&self, id: i64) -> Result<SyncReport, AppError> {
        let source = self
            .source_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("远程源不存在: {}", id)))?;
        if !source.enabled {
            return Err(AppError::BadRequest(format!(
                "远程源已停用: {}",
                source.name
            )));
        }

        // 同源互斥
        {
            let mut locks = lock_or_recover(&self.sync_locks);
            if locks.contains_key(&id) {
                return Err(AppError::Conflict("该源正在同步中，请稍后再试".into()));
            }
            locks.insert(id, ());
        }
        // 确保退出时释放锁
        let _guard = SyncLockGuard { service: self, id };

        // 同步任务经统一任务中心跟踪（Running → Success/Failed）
        let task = self
            .task_tracker
            .create(
                crate::runtime::task_state::TaskKind::Generic,
                format!("同步远程应用源: {}", source.name),
            )
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        let _ = self
            .task_tracker
            .transition(task.id, crate::runtime::task_state::TaskState::Running)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;

        let result = self.do_sync_index(&source).await;

        match &result {
            Ok(report) => {
                let _ = self
                    .task_tracker
                    .transition(task.id, crate::runtime::task_state::TaskState::Success)
                    .await
                    .map_err(|e| AppError::internal(e.to_string()))?;
                let _ = self
                    .task_tracker
                    .update_progress(
                        task.id,
                        100,
                        &format!("新增 {}，跳过 {}", report.added, report.skipped),
                    )
                    .await
                    .map_err(|e| AppError::internal(e.to_string()))?;
            }
            Err(e) => {
                let _ = self
                    .task_tracker
                    .transition(task.id, crate::runtime::task_state::TaskState::Failed)
                    .await
                    .map_err(|e| AppError::internal(e.to_string()))?;
                let _ = self
                    .task_tracker
                    .update_progress(task.id, 0, &e.to_string())
                    .await
                    .map_err(|e| AppError::internal(e.to_string()))?;
            }
        }

        result
    }

    pub(super) async fn do_sync_index(&self, source: &AppSource) -> Result<SyncReport, AppError> {
        let remote = crate::infrastructure::app_store::remote_source::create_remote_source(
            source,
            self.runner.clone(),
            self.adapter_provider.clone(),
            &Self::default_remote_cache_dir(),
        )?;

        // 外部调用走 Circuit Breaker（避免远程源持续不可用时反复打满连接）
        let breaker = crate::resilience::CircuitBreaker::new(Default::default());
        let packages: Vec<AppPackage> = breaker
            .call(|| async { remote.sync_index(source).await })
            .await?;

        if packages.is_empty() {
            self.source_repo.touch_synced(source.id).await?;
            return Ok(SyncReport::default());
        }

        // M-A Issue 1.3（Doc/20 §4.2）：同步阶段验签。
        // - 源配置了 public_key → 强制验签（签名存放于包根 signature.sig 文件，base64）；
        //   git 源包体在 clone 缓存目录内就地验证；http 源索引同步阶段无包体，验签在
        //   安装时 fetch_package 后执行（见 verify_remote_package_at_install）。
        // - 源未配置 public_key：策略 optional 时放行；required 时拒绝整个源。
        self.verify_source_packages(source, &packages, &remote)
            .await?;

        let added = self.package_repo.create_many(&packages).await?;
        self.source_repo.touch_synced(source.id).await?;

        let keys: Vec<String> = packages.iter().map(|p| p.key.clone()).collect();
        Ok(SyncReport {
            added,
            skipped: packages.len() - added,
            packages: keys,
        })
    }

    /// 远程源同步阶段验签（M-A Issue 1.3）。
    pub(super) async fn verify_source_packages(
        &self,
        source: &AppSource,
        packages: &[AppPackage],
        remote: &crate::infrastructure::app_store::remote_source::BoxedRemoteSource,
    ) -> Result<(), AppError> {
        use crate::infrastructure::app_store::signature;
        let policy = self.signature_policy().await;
        match (&source.public_key, policy) {
            (None, signature::SignaturePolicy::Required) => {
                return Err(AppError::ValidationError(format!(
                    "验签策略为 required：远程源 {} 未配置公钥，拒绝同步",
                    source.name
                )));
            }
            (None, signature::SignaturePolicy::Optional) => return Ok(()),
            (Some(_), _) => {}
        }
        let pk = source.public_key.as_deref().unwrap_or_default();
        signature::validate_public_key(pk)?;
        for pkg in packages {
            // 仅验证有本地包体的源（git clone 缓存）；http 源 source_path=None，安装时验
            if let Some(path) = &pkg.source_path {
                let root = std::path::Path::new(path);
                let sig_file = root.join("signature.sig");
                if !sig_file.is_file() {
                    return Err(AppError::ValidationError(format!(
                        "远程包 {} 缺少 signature.sig 签名文件，验签失败",
                        pkg.key
                    )));
                }
                let sig_b64 = std::fs::read_to_string(&sig_file)
                    .map_err(|e| AppError::internal(format!("读取签名文件失败: {}", e)))?;
                signature::verify_package(root, pk, &sig_b64)?;
            }
        }
        // 触达一次 fetch_package 编译面（http 源安装时验签入口），无运行时副作用
        let _ = remote;
        tracing::info!(
            "远程源 {} 验签通过，共 {} 个包",
            source.name,
            packages.len()
        );
        Ok(())
    }

    /// 远程包懒加载拉取（M-A Issue 1.4，Doc/20 §4.1）：
    /// http 源包同步阶段只落索引（source_path = None），安装时在此按需下载包体
    /// 到本地缓存并回写 source_path；git 源与本地包已有包体，直接放行。
    pub(super) async fn ensure_remote_package_fetched(
        &self,
        key: &str,
    ) -> Result<AppPackage, AppError> {
        let pkg = self.get_package(key).await?;
        if pkg.source == "local" || pkg.source_path.is_some() {
            return Ok(pkg);
        }
        let source_url = pkg
            .source_url
            .clone()
            .ok_or_else(|| AppError::BadRequest(format!("远程包 {} 缺少来源 url", key)))?;
        let source = self
            .source_repo
            .find_by_url(&source_url)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("远程源不存在或已删除: {}", source_url)))?;
        if !source.enabled {
            return Err(AppError::BadRequest(format!(
                "远程源 {} 已停用，无法安装其应用包",
                source.name
            )));
        }
        let remote = crate::infrastructure::app_store::remote_source::create_remote_source(
            &source,
            self.runner.clone(),
            self.adapter_provider.clone(),
            &Self::default_remote_cache_dir(),
        )?;
        // 外部下载走 Circuit Breaker（与索引同步同一韧性策略）
        let breaker = crate::resilience::CircuitBreaker::new(Default::default());
        let root = breaker
            .call(|| async {
                remote
                    .fetch_package(&source, key, &Self::default_remote_cache_dir())
                    .await
            })
            .await?;
        // 回写 source_path，后续安装/验签/升级直接复用本地缓存
        self.package_repo
            .update_source_path(pkg.id, &root.to_string_lossy())
            .await?;
        tracing::info!("远程包 {} 包体已拉取至 {}", key, root.display());
        self.get_package(key).await
    }

    /// 远程包安装前验签（M-A Issue 1.3）：按包来源 url 查源并强制校验签名。
    ///
    /// 在 `get_version` 解析包体前调用：远程包（source != local）必须
    /// ① 源存在且配置公钥（required 策略下无公钥即拒绝）；② 包根带 signature.sig 且验签通过。
    pub async fn verify_remote_package_at_install(&self, pkg: &AppPackage) -> Result<(), AppError> {
        use crate::infrastructure::app_store::signature;
        if pkg.source == "local" || pkg.source_url.is_none() {
            return Ok(()); // 本地包不在远程验签路径
        }
        let source = self
            .source_repo
            .find_by_url(pkg.source_url.as_deref().unwrap_or_default())
            .await?
            .ok_or_else(|| {
                AppError::ValidationError(format!(
                    "应用包来源源不存在或已删除: {}",
                    pkg.source_url.as_deref().unwrap_or_default()
                ))
            })?;
        let policy = self.signature_policy().await;
        let Some(pk) = source.public_key.as_deref() else {
            if policy == signature::SignaturePolicy::Required {
                return Err(AppError::ValidationError(format!(
                    "验签策略为 required：源 {} 未配置公钥，拒绝安装其远程包",
                    source.name
                )));
            }
            return Ok(()); // optional 且未配公钥：不验签
        };
        signature::validate_public_key(pk)?;
        let root = std::path::Path::new(pkg.source_path.as_deref().unwrap_or_default());
        if !root.is_dir() {
            return Err(AppError::NotFound(format!(
                "远程包 {} 包体未拉取，无法验签",
                pkg.key
            )));
        }
        let sig_file = root.join("signature.sig");
        let sig_b64 = std::fs::read_to_string(&sig_file).map_err(|_| {
            AppError::ValidationError(format!(
                "远程包 {} 缺少 signature.sig 签名文件，验签失败",
                pkg.key
            ))
        })?;
        signature::verify_package(root, pk, &sig_b64)?;
        tracing::info!("远程包 {} 安装前验签通过", pkg.key);
        Ok(())
    }

    /// 按源 id 过滤应用包列表（复用现有包仓储）。
    pub async fn list_packages_by_source(
        &self,
        source_id: i64,
    ) -> Result<Vec<AppPackage>, AppError> {
        let source = self
            .source_repo
            .find_by_id(source_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("远程源不存在: {}", source_id)))?;
        let all = self.package_repo.list_all().await?;
        Ok(all
            .into_iter()
            .filter(|p| p.source_url.as_deref() == Some(source.url.as_str()))
            .collect())
    }

    pub async fn list_versions(&self, key: &str) -> Result<Vec<String>, AppError> {
        let meta = self.get_metadata(key).await?;
        Ok(meta.versions)
    }

    /// 解析版本信息（内置应用走内置模板，导入应用走适配器）
    pub async fn get_version(&self, key: &str, version: &str) -> Result<AppVersionInfo, AppError> {
        if let Some(v) = self.wasm_builtin_version(key, version) {
            return Ok(v);
        }
        let pkg = self.get_package(key).await?;

        // 内置应用
        if pkg.source_path.is_none() {
            if let Some(manifest) = crate::domain::entity::builtin_apps()
                .into_iter()
                .find(|m| m.key == pkg.key)
            {
                if version == manifest.version {
                    return Ok(manifest.to_version());
                }
            }
            return Err(AppError::NotFound(format!("版本不存在: {}", version)));
        }

        // 导入应用：优先适配器解析
        let root = Path::new(pkg.source_path.as_deref().unwrap_or_default());
        if root.is_dir() {
            let adapter = self.adapter_provider.select(root)?;
            return adapter.parse_version(root, version);
        }

        // 无目录（理论上不可达）：从 metadata 兜底
        Err(AppError::BadRequest(format!("应用包 {} 缺少源目录", key)))
    }
}

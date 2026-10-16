//! 应用包目录：内置种子、列表 / 详情、本地导入（含签名校验）。

use super::*;

impl AppStoreService {
    /// 幂等种子：将内置应用（当前 6 个）写入包仓库（Phase A2 扩展：统一接入 `set_many` 事务语义，批量原子写）
    pub async fn seed_builtin_apps(&self) -> Result<usize, AppError> {
        let mut to_write = Vec::new();
        for manifest in crate::domain::entity::builtin_apps() {
            if self
                .package_repo
                .find_by_key(&manifest.key)
                .await?
                .is_some()
            {
                continue;
            }
            let metadata = manifest.to_metadata();
            let pkg = AppPackage {
                id: 0,
                key: metadata.key.clone(),
                name: metadata.name.clone(),
                category: metadata.category.clone(),
                format: metadata.format.as_str().into(),
                description: metadata.short_desc_zh.clone(),
                logo: metadata.logo.clone(),
                metadata_json: serde_json::to_string(&metadata)
                    .map_err(|e| AppError::internal(format!("serialize metadata: {}", e)))?,
                source_path: None,
                source: "local".to_string(),
                source_url: None,
                synced_at: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            to_write.push(pkg);
        }
        self.package_repo.create_many(&to_write).await
    }

    pub async fn list_packages(
        &self,
        category: Option<&str>,
    ) -> Result<Vec<AppMetadata>, AppError> {
        let packages = self.package_repo.list_all().await?;
        let mut metas: Vec<AppMetadata> = packages
            .into_iter()
            .filter_map(|p| serde_json::from_str(&p.metadata_json).ok())
            .collect();
        if let Some(cat) = category {
            metas.retain(|m| m.category.eq_ignore_ascii_case(cat));
        }
        Ok(metas)
    }

    pub async fn get_package(&self, key: &str) -> Result<AppPackage, AppError> {
        self.package_repo
            .find_by_key(key)
            .await?
            .or_else(|| {
                // 内置 wasm-hello 仅注册于 wasm_builtins()，不在包仓储中——
                // 安装链路（install_inner 验签查询）需要它的实体形态。
                self.wasm_builtins()
                    .into_iter()
                    .find(|m| m.key == key)
                    .map(|m| AppPackage {
                        id: 0,
                        key: m.key.clone(),
                        name: m.name.clone(),
                        category: m.category.clone(),
                        format: m.format.as_str().into(),
                        description: m.short_desc_zh.clone(),
                        logo: m.logo.clone(),
                        metadata_json: serde_json::to_string(&m).unwrap_or_default(),
                        source_path: None,
                        source: "local".to_string(),
                        source_url: None,
                        synced_at: None,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    })
            })
            .ok_or_else(|| AppError::NotFound(format!("应用包不存在: {}", key)))
    }

    pub async fn get_metadata(&self, key: &str) -> Result<AppMetadata, AppError> {
        if let Some(m) = self.wasm_builtins().into_iter().find(|m| m.key == key) {
            return Ok(m);
        }
        let pkg = self.get_package(key).await?;
        serde_json::from_str(&pkg.metadata_json)
            .map_err(|e| AppError::internal(format!("parse metadata: {}", e)))
    }

    /// 导入本地目录应用包（自动检测格式）
    pub async fn import_package(&self, path: &str) -> Result<AppMetadata, AppError> {
        let pkg = self.prepare_import_package(path).await?;
        let key = pkg.key.clone();
        self.package_repo.create(&pkg).await?;
        self.get_metadata(&key).await
    }

    /// 批量导入本地目录应用包（Phase A2 扩展：对外批量写端点，`create_many` 事务语义）。
    ///
    /// 一次性解析全部目录，任一已存在或非法目录则整体失败（不落库，保持原子性）；
    /// 全部通过后用 `create_many` 原子写（SQLite 事务 / InMemory 单锁，要么全成功要么全回滚）。
    pub async fn batch_import_packages(
        &self,
        paths: &[String],
    ) -> Result<Vec<AppMetadata>, AppError> {
        if paths.is_empty() {
            return Err(AppError::BadRequest("导入路径列表不能为空".into()));
        }
        if paths.len() > 50 {
            return Err(AppError::BadRequest("单次最多导入 50 个目录".into()));
        }
        let mut prepared: Vec<AppPackage> = Vec::with_capacity(paths.len());
        // 失败时回滚已复制进商店目录的文件（仓储层 create_many 本身是原子的）
        let cleanup = |prepared: &[AppPackage]| {
            for p in prepared {
                if let Some(dir) = &p.source_path {
                    let _ = std::fs::remove_dir_all(dir);
                }
            }
        };
        for path in paths {
            match self.prepare_import_package(path).await {
                Ok(pkg) if prepared.iter().any(|p| p.key == pkg.key) => {
                    let key = pkg.key.clone();
                    prepared.push(pkg);
                    cleanup(&prepared);
                    return Err(AppError::BadRequest(format!(
                        "批次内应用包 key 重复: {key}"
                    )));
                }
                Ok(pkg) => prepared.push(pkg),
                Err(e) => {
                    cleanup(&prepared);
                    return Err(e);
                }
            }
        }
        // 全部解析成功后才原子落库
        if let Err(e) = self.package_repo.create_many(&prepared).await {
            cleanup(&prepared);
            return Err(e);
        }
        let mut metas = Vec::with_capacity(prepared.len());
        for pkg in prepared {
            if let Ok(m) = serde_json::from_str::<AppMetadata>(&pkg.metadata_json) {
                metas.push(m);
            }
        }
        Ok(metas)
    }

    /// 解析并复制单个本地目录为 `AppPackage`（不落库；已存在 / 非法目录返回错误）。
    pub(super) async fn prepare_import_package(&self, path: &str) -> Result<AppPackage, AppError> {
        let root = Path::new(path);
        if !root.is_dir() {
            return Err(AppError::BadRequest(format!("路径不是目录: {}", path)));
        }
        let adapter = self.adapter_provider.select(root)?;
        let metadata = adapter.parse_metadata(root)?;
        // Doc/33 P-9：key 来自包内 app.json / data.yml，未校验时 `"key": "../../root/.ssh"`
        // 会让下方复制以面板（root）身份写到商店目录之外。
        crate::infrastructure::app_store::remote_source::validate_key(&metadata.key)?;

        // 存在性检查必须先于复制：旧实现先覆盖已有包目录（含已验签包的脚本）再报「已存在」。
        if self
            .package_repo
            .find_by_key(&metadata.key)
            .await?
            .is_some()
        {
            return Err(AppError::BadRequest(format!(
                "应用包已存在: {}",
                metadata.key
            )));
        }

        // 复制到商店目录，保证独立性
        let dest = self.apps_dir.join(&metadata.key);
        // 源目录是目标的祖先（如导入 apps_dir 或其上级）时递归复制永不终止；
        // 源位于目标之内时，下方清理残留目录会先删掉源。两种都拒绝（apps_dir 内其他子目录可导入）。
        if let (Ok(src), Ok(apps)) = (root.canonicalize(), self.apps_dir.canonicalize()) {
            let dest_c = apps.join(&metadata.key);
            if dest_c.starts_with(&src) || src.starts_with(&dest_c) {
                return Err(AppError::BadRequest(format!(
                    "导入源与商店目标目录互相包含: {}",
                    path
                )));
            }
        }
        if dest.exists() {
            // 仓储无记录但目录残留（历史失败导入）：清掉再复制，避免新旧文件混杂
            let _ = std::fs::remove_dir_all(&dest);
        }
        std::fs::create_dir_all(&dest)
            .map_err(|e| AppError::internal(format!("创建商店目录失败: {}", e)))?;
        if let Err(e) = copy_recursive(root, &dest) {
            let _ = std::fs::remove_dir_all(&dest);
            return Err(e);
        }

        // M-A Issue 1.3（Doc/20 §4.2）：导入验签——若包根 app.json 携带 signature 字段，
        // 则必须能通过包内同字段签名验证（签名即针对除 app.json 外的包内容 + app.json
        // 摘要外露部分）。无 signature 字段时放行（本地导入不强制签名）。
        if let Err(e) = Self::verify_local_package_signature(&dest) {
            let _ = std::fs::remove_dir_all(&dest);
            return Err(e);
        }

        let pkg = AppPackage {
            id: 0,
            key: metadata.key.clone(),
            name: metadata.name.clone(),
            category: metadata.category.clone(),
            format: metadata.format.as_str().into(),
            description: metadata.short_desc_zh.clone(),
            logo: metadata.logo.clone(),
            metadata_json: serde_json::to_string(&metadata)
                .map_err(|e| AppError::internal(format!("serialize metadata: {}", e)))?,
            source_path: Some(dest.to_string_lossy().into_owned()),
            source: "local".to_string(),
            source_url: None,
            synced_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        Ok(pkg)
    }

    /// 本地导入包验签（M-A Issue 1.3）：app.json 带 `signature` 字段时强制校验。
    ///
    /// 签名语义：`signature` = 源私钥对 `signature_payload(app.json 剥离 signature 字段
    /// 后的规范化 JSON + 包内其余文件清单)` 的 Ed25519 签名（base64）。
    /// 公钥须随包提供（`public_key` 字段，base64）——本地导入场景下「包内自带公钥」
    /// 仅作完整性校验（防传输损坏/篡改后不带签名的静默导入）；信任锚（源 public_key）
    /// 在远程源验签路径上强制。
    pub(super) fn verify_local_package_signature(dest: &Path) -> Result<(), AppError> {
        use crate::infrastructure::app_store::signature;
        let app_json_path = dest.join("app.json");
        if !app_json_path.is_file() {
            return Ok(()); // 非 Flame 格式（1Panel/宝塔）无 app.json，不适用本验签
        }
        let raw = std::fs::read_to_string(&app_json_path)
            .map_err(|e| AppError::internal(format!("读取 app.json 失败: {}", e)))?;
        let value: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| AppError::internal(format!("解析 app.json 失败: {}", e)))?;
        let Some(sig_b64) = value
            .get("signature")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        else {
            return Ok(()); // 未签名：本地导入放行（远程源路径按策略强制）
        };
        let Some(pk_b64) = value
            .get("public_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        else {
            return Err(AppError::ValidationError(
                "应用包携带 signature 但缺少 public_key 字段，无法验签".into(),
            ));
        };
        signature::validate_public_key(&pk_b64)?;
        // 验签对象：剥离 signature 字段后的 app.json 规范化字节 + 其余文件清单
        let payload = signature::local_signature_payload(dest, &value, &sig_b64)?;
        signature::verify_payload(&payload, &pk_b64, &sig_b64)?;
        tracing::info!("本地导入包验签通过: {}", dest.display());
        Ok(())
    }

    // ─── 远程应用源（M-A Issue 1.2，Doc/20 §4.1）────────────────────────────
}

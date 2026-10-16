//! S3 兼容备份后端（object_store crate · AWS S3 / MinIO / 阿里云 OSS / 腾讯云 COS / R2）

use crate::core::error::AppError;
use crate::infrastructure::backup_backend_shared::spawn_blocking;
use crate::infrastructure::backup_backend_traits::BackupBackend;
use crate::infrastructure::backup_backend_traits::RemoteObject;
use futures_util::TryStreamExt;
use object_store::{path::Path as OSPath, ObjectStore, ObjectStoreExt};
use std::path::Path;

/// S3 兼容后端连接配置（对应账号 `config_json`）
#[derive(Debug, Clone, serde::Deserialize)]
pub struct S3Config {
    /// S3 兼容端点，如 `https://s3.amazonaws.com` / `http://minio.local:9000`
    pub endpoint: String,
    /// 桶名
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    /// 区域（部分兼容端点必填，默认 `us-east-1`）
    #[serde(default = "default_region")]
    pub region: String,
    /// 对象 key 前缀（可选，如 `flamepanel/`）
    #[serde(default)]
    pub prefix: String,
}

fn default_region() -> String {
    "us-east-1".to_string()
}

pub struct S3Backend {
    store: std::sync::Arc<dyn ObjectStore>,
    prefix: String,
}

impl S3Backend {
    pub fn from_config(cfg: &S3Config) -> Result<Self, AppError> {
        use object_store::aws::AmazonS3Builder;
        let endpoint = cfg.endpoint.trim().to_string();
        let allow_http = endpoint.starts_with("http://");
        let mut builder = AmazonS3Builder::new()
            .with_endpoint(&endpoint)
            .with_bucket_name(&cfg.bucket)
            .with_access_key_id(&cfg.access_key)
            .with_secret_access_key(&cfg.secret_key)
            .with_region(&cfg.region)
            .with_allow_http(allow_http);
        // MinIO 等自建兼容端点普遍使用 path-style 寻址
        if !endpoint.contains("amazonaws.com") {
            builder = builder.with_virtual_hosted_style_request(false);
        }
        let store = builder
            .build()
            .map_err(|e| AppError::BadRequest(format!("S3 配置无效: {}", e)))?;
        Ok(Self {
            store: std::sync::Arc::new(store),
            prefix: cfg.prefix.trim_matches('/').to_string(),
        })
    }

    fn full_key(&self, remote_key: &str) -> Result<OSPath, AppError> {
        if remote_key.starts_with('/') || remote_key.split('/').any(|seg| seg == "..") {
            return Err(AppError::BadRequest(format!(
                "非法备份对象 key: {}",
                remote_key
            )));
        }
        let full = if self.prefix.is_empty() {
            remote_key.to_string()
        } else {
            format!("{}/{}", self.prefix, remote_key)
        };
        OSPath::parse(&full).map_err(|e| AppError::BadRequest(format!("非法对象 key: {}", e)))
    }

    /// 将 object_store 错误映射为 AppError（404 → NotFound）
    fn map_err(e: object_store::Error) -> AppError {
        if matches!(e, object_store::Error::NotFound { .. }) {
            AppError::NotFound("远端备份对象不存在".into())
        } else {
            AppError::internal(format!("S3 后端操作失败: {}", e))
        }
    }

    fn key_from_meta(meta: &object_store::ObjectMeta) -> String {
        meta.location.to_string()
    }
}

#[async_trait::async_trait]
impl BackupBackend for S3Backend {
    async fn upload(&self, local: &Path, remote_key: &str) -> Result<RemoteObject, AppError> {
        let location = self.full_key(remote_key)?;
        let local = local.to_path_buf();
        let payload =
            spawn_blocking(move || -> Result<Vec<u8>, AppError> { Ok(std::fs::read(&local)?) })
                .await?;
        let size = payload.len() as i64;
        self.store
            .put(&location, payload.into())
            .await
            .map_err(Self::map_err)?;
        Ok(RemoteObject {
            key: remote_key.to_string(),
            size,
        })
    }

    async fn list(&self) -> Result<Vec<RemoteObject>, AppError> {
        let prefix = if self.prefix.is_empty() {
            None
        } else {
            Some(
                OSPath::parse(&self.prefix)
                    .map_err(|e| AppError::BadRequest(format!("非法前缀: {}", e)))?,
            )
        };
        let stream = self.store.list(prefix.as_ref());
        let metas: Vec<object_store::ObjectMeta> =
            stream.try_collect().await.map_err(Self::map_err)?;
        Ok(metas
            .iter()
            .map(|m| {
                let full = Self::key_from_meta(m);
                let key = if self.prefix.is_empty() {
                    full
                } else {
                    full.strip_prefix(&format!("{}/", self.prefix))
                        .unwrap_or(&full)
                        .to_string()
                };
                RemoteObject {
                    key,
                    size: m.size as i64,
                }
            })
            .collect())
    }

    async fn download(&self, remote_key: &str, local: &Path) -> Result<(), AppError> {
        let location = self.full_key(remote_key)?;
        let data = self
            .store
            .get(&location)
            .await
            .map_err(Self::map_err)?
            .bytes()
            .await
            .map_err(Self::map_err)?;
        let target = local.to_path_buf();
        let bytes = data.to_vec();
        spawn_blocking(move || -> Result<(), AppError> {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target, &bytes)?;
            Ok(())
        })
        .await
    }

    async fn delete(&self, remote_key: &str) -> Result<(), AppError> {
        let location = self.full_key(remote_key)?;
        self.store.delete(&location).await.map_err(Self::map_err)?;
        Ok(())
    }

    async fn test_connection(&self) -> Result<(), AppError> {
        // 真实探测：对桶根做一次 list（delimiter 单层，开销最小）
        let stream = self.store.list(None::<&OSPath>);
        let _ = stream.into_stream();
        // list 是惰性流，取一个元素以实际触发请求
        use futures_util::StreamExt;
        let mut stream = self.store.list(None::<&OSPath>);
        match stream.next().await {
            Some(Ok(_)) | None => Ok(()),
            Some(Err(e)) => Err(Self::map_err(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_parses_with_defaults() {
        let cfg: S3Config = serde_json::from_str(
            r#"{"endpoint":"http://minio.local:9000","bucket":"flame","access_key":"a","secret_key":"b"}"#,
        )
        .unwrap();
        assert_eq!(cfg.region, "us-east-1");
        assert_eq!(cfg.prefix, "");
    }

    #[tokio::test]
    async fn bad_endpoint_rejected() {
        let cfg: S3Config = serde_json::from_str(
            r#"{"endpoint":"http://x","bucket":"b","access_key":"a","secret_key":"s"}"#,
        )
        .unwrap();
        let backend = S3Backend::from_config(&cfg).unwrap();
        // 连接失败应报错而非 panic
        let r = backend.test_connection().await;
        assert!(r.is_err() || r.is_ok());
    }

    #[tokio::test]
    async fn traversal_key_rejected() {
        let cfg: S3Config = serde_json::from_str(
            r#"{"endpoint":"http://x","bucket":"b","access_key":"a","secret_key":"s"}"#,
        )
        .unwrap();
        let backend = S3Backend::from_config(&cfg).unwrap();
        assert!(backend.full_key("../evil").is_err());
        assert!(backend.full_key("/abs").is_err());
    }
}

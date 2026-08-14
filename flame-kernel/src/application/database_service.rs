//! 数据库领域服务（T8 拆分自原 `application/service.rs` 上帝文件）。
use crate::api::types::{PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::database::{mysql::MySqlManager, redis::RedisManager, NativeDbManager};
use crate::domain::entity::*;
use crate::domain::repository::*;
use std::sync::Arc;

pub struct DatabaseService {
    pub repo: Arc<dyn DatabaseRepository>,
    pub mysql_manager: MySqlManager,
    pub redis_manager: RedisManager,
}

impl DatabaseService {
    pub fn new(
        repo: Arc<dyn DatabaseRepository>,
        runner: crate::application::execution_mode::SharedCommandRunner,
    ) -> Self {
        Self {
            repo,
            mysql_manager: MySqlManager::new(runner.clone()),
            redis_manager: RedisManager::new(runner),
        }
    }

    /// T16：以可配置的 mysql/redis 配置文件路径构建（未提供时沿用默认路径）。
    pub fn new_with_config_paths(
        repo: Arc<dyn DatabaseRepository>,
        runner: crate::application::execution_mode::SharedCommandRunner,
        mysql_config: impl Into<String>,
        redis_config: impl Into<String>,
    ) -> Self {
        Self {
            repo,
            mysql_manager: MySqlManager::new(runner.clone()).with_config_file(mysql_config),
            redis_manager: RedisManager::new(runner).with_config_file(redis_config),
        }
    }

    pub async fn list_instances(&self) -> Result<Vec<DatabaseInstance>, AppError> {
        self.repo.list_all().await
    }

    pub async fn list_instances_paginated(
        &self,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<DatabaseInstance>, AppError> {
        // 分页下沉（Stage1）：直接 LIMIT/OFFSET，避免全表加载 + 内存切片
        let total = self.repo.count().await?;
        let data = self
            .repo
            .list_page(params.page_size(), params.offset())
            .await?;
        Ok(PaginatedResponse::new(data, total, params))
    }

    pub async fn get_instance(&self, id: i64) -> Result<DatabaseInstance, AppError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database instance {} not found", id)))
    }

    pub async fn delete_instance(&self, id: i64) -> Result<(), AppError> {
        self.repo.delete(id).await
    }

    pub async fn install_mysql(
        &self,
        version: Option<&str>,
        port: i32,
        password: &str,
        name: &str,
    ) -> Result<DatabaseInstance, AppError> {
        let db_type = DatabaseType::Mysql;
        self.mysql_manager.install(version, port, password).await?;
        let ver = self
            .mysql_manager
            .get_version()
            .await
            .unwrap_or_else(|_| "latest".into());
        let instance = DatabaseInstance {
            id: 0,
            db_type: db_type.as_str().into(),
            name: name.into(),
            version: ver,
            port,
            status: "running".into(),
            install_path: "/usr/bin/mysql".into(),
            data_dir: "/var/lib/mysql".into(),
            config_file: "/etc/mysql/mysql.conf.d/mysqld.cnf".into(),
            root_user: "root".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            resource_version: 0,
        };
        let id = self.repo.create(&instance).await?;
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::internal("Failed to create database instance"))
    }

    pub async fn install_redis(
        &self,
        version: Option<&str>,
        port: i32,
        password: Option<&str>,
        name: &str,
    ) -> Result<DatabaseInstance, AppError> {
        self.redis_manager
            .install(version, port, password.unwrap_or(""))
            .await?;
        let ver = self
            .redis_manager
            .get_version()
            .await
            .unwrap_or_else(|_| "latest".into());
        let instance = DatabaseInstance {
            id: 0,
            db_type: "redis".into(),
            name: name.into(),
            version: ver,
            port,
            status: "running".into(),
            install_path: "/usr/bin/redis-server".into(),
            data_dir: "/var/lib/redis".into(),
            config_file: "/etc/redis/redis.conf".into(),
            root_user: "".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            resource_version: 0,
        };
        let id = self.repo.create(&instance).await?;
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::internal("Failed to create database instance"))
    }

    pub async fn start(&self, id: i64) -> Result<(), AppError> {
        let inst = self.get_instance(id).await?;
        match inst.db_type.as_str() {
            "mysql" | "mariadb" => self.mysql_manager.start().await,
            "redis" => self.redis_manager.start().await,
            t => Err(AppError::BadRequest(format!(
                "Unknown database type: {}",
                t
            ))),
        }?;
        self.repo.update_status(id, "running").await
    }

    pub async fn stop(&self, id: i64) -> Result<(), AppError> {
        let inst = self.get_instance(id).await?;
        match inst.db_type.as_str() {
            "mysql" | "mariadb" => self.mysql_manager.stop().await,
            "redis" => self.redis_manager.stop().await,
            t => Err(AppError::BadRequest(format!(
                "Unknown database type: {}",
                t
            ))),
        }?;
        self.repo.update_status(id, "stopped").await
    }

    pub async fn restart(&self, id: i64) -> Result<(), AppError> {
        let inst = self.get_instance(id).await?;
        match inst.db_type.as_str() {
            "mysql" | "mariadb" => self.mysql_manager.restart().await,
            "redis" => self.redis_manager.restart().await,
            t => Err(AppError::BadRequest(format!(
                "Unknown database type: {}",
                t
            ))),
        }
    }

    pub async fn status(&self, id: i64) -> Result<String, AppError> {
        let inst = self.get_instance(id).await?;
        match inst.db_type.as_str() {
            "mysql" | "mariadb" => {
                if self.mysql_manager.is_running().await? {
                    Ok("running".into())
                } else {
                    Ok("stopped".into())
                }
            }
            "redis" => {
                if self.redis_manager.is_running().await? {
                    Ok("running".into())
                } else {
                    Ok("stopped".into())
                }
            }
            t => Err(AppError::BadRequest(format!(
                "Unknown database type: {}",
                t
            ))),
        }
    }

    /// 批量更新实例状态（Phase A2 扩展：统一接入 `set_many` 事务语义，原子写）。
    pub async fn update_instances_status_batch(
        &self,
        updates: &[(i64, String)],
    ) -> Result<(), AppError> {
        if updates.is_empty() {
            return Ok(());
        }
        self.repo.update_status_batch(updates).await
    }

    pub async fn create_database(
        &self,
        instance_id: i64,
        db_name: &str,
        charset: &str,
    ) -> Result<(), AppError> {
        let inst = self.get_instance(instance_id).await?;
        match inst.db_type.as_str() {
            "mysql" | "mariadb" => self.mysql_manager.create_database(db_name, charset).await,
            t => Err(AppError::BadRequest(format!(
                "Database creation not supported for: {}",
                t
            ))),
        }
    }

    pub async fn drop_database(&self, instance_id: i64, db_name: &str) -> Result<(), AppError> {
        let inst = self.get_instance(instance_id).await?;
        match inst.db_type.as_str() {
            "mysql" | "mariadb" => self.mysql_manager.drop_database(db_name).await,
            t => Err(AppError::BadRequest(format!(
                "Database drop not supported for: {}",
                t
            ))),
        }
    }

    pub async fn list_databases(&self, instance_id: i64) -> Result<Vec<String>, AppError> {
        let inst = self.get_instance(instance_id).await?;
        match inst.db_type.as_str() {
            "mysql" | "mariadb" => self.mysql_manager.list_databases().await,
            t => Err(AppError::BadRequest(format!(
                "Database listing not supported for: {}",
                t
            ))),
        }
    }

    pub async fn create_user(
        &self,
        instance_id: i64,
        username: &str,
        password: &str,
        host: &str,
    ) -> Result<(), AppError> {
        let _inst = self.get_instance(instance_id).await?;
        self.mysql_manager
            .create_user(username, password, host)
            .await
    }

    pub async fn drop_user(
        &self,
        instance_id: i64,
        username: &str,
        host: &str,
    ) -> Result<(), AppError> {
        let _inst = self.get_instance(instance_id).await?;
        self.mysql_manager.drop_user(username, host).await
    }

    pub async fn uninstall(&self, id: i64) -> Result<(), AppError> {
        let inst = self.get_instance(id).await?;
        match inst.db_type.as_str() {
            "mysql" | "mariadb" => self.mysql_manager.uninstall().await,
            "redis" => self.redis_manager.uninstall().await,
            t => Err(AppError::BadRequest(format!(
                "Unknown database type: {}",
                t
            ))),
        }?;
        self.repo.delete(id).await
    }
}

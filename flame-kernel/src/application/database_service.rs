//! 数据库领域服务（T8 拆分自原 `application/service.rs` 上帝文件）。
use crate::api::types::{PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::database::{
    mongodb::MongoManager, mysql::MySqlManager, postgres::PostgresManager, redis::RedisManager,
    NativeDbManager,
};
use crate::domain::entity::*;
use crate::domain::repository::*;
use std::sync::Arc;

pub struct DatabaseService {
    pub repo: Arc<dyn DatabaseRepository>,
    pub mysql_manager: MySqlManager,
    pub redis_manager: RedisManager,
    pub postgres_manager: PostgresManager,
    pub mongo_manager: MongoManager,
}

impl DatabaseService {
    pub fn new(
        repo: Arc<dyn DatabaseRepository>,
        runner: crate::application::execution_mode::SharedCommandRunner,
    ) -> Self {
        Self {
            repo,
            mysql_manager: MySqlManager::new(runner.clone()),
            redis_manager: RedisManager::new(runner.clone()),
            postgres_manager: PostgresManager::new(runner.clone()),
            mongo_manager: MongoManager::new(runner),
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
            redis_manager: RedisManager::new(runner.clone()).with_config_file(redis_config),
            postgres_manager: PostgresManager::new(runner.clone()),
            mongo_manager: MongoManager::new(runner),
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

    /// Doc/20 §6.4：PostgreSQL 原生安装（发行版包）。
    pub async fn install_postgres(
        &self,
        version: Option<&str>,
        port: i32,
        password: Option<&str>,
        name: &str,
    ) -> Result<DatabaseInstance, AppError> {
        self.postgres_manager
            .install(version, port, password.unwrap_or(""))
            .await?;
        let ver = self
            .postgres_manager
            .get_version()
            .await
            .unwrap_or_else(|_| "latest".into());
        let instance = DatabaseInstance {
            id: 0,
            db_type: "postgres".into(),
            name: name.into(),
            version: ver,
            port,
            status: "running".into(),
            install_path: "/var/lib/postgresql".into(),
            data_dir: "/var/lib/postgresql/data".into(),
            config_file: "/etc/postgresql".into(),
            root_user: "postgres".into(),
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

    /// Doc/30 M-C：MongoDB 原生安装（官方仓库 8.0 / 9.0）。填写密码时创建 `admin` 并开启鉴权。
    pub async fn install_mongodb(
        &self,
        version: Option<&str>,
        port: i32,
        password: Option<&str>,
        name: &str,
    ) -> Result<DatabaseInstance, AppError> {
        let password = password.unwrap_or("");
        self.mongo_manager.install(version, port, password).await?;
        let ver = self
            .mongo_manager
            .get_version()
            .await
            .unwrap_or_else(|_| "unknown".into());
        let instance = DatabaseInstance {
            id: 0,
            db_type: "mongodb".into(),
            name: name.into(),
            version: ver,
            port,
            status: "running".into(),
            install_path: "/usr/bin/mongod".into(),
            data_dir: "/var/lib/mongodb".into(),
            config_file: crate::database::mongodb::MONGOD_CONFIG.into(),
            root_user: if password.is_empty() { "" } else { "admin" }.into(),
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
            "postgres" => self.postgres_manager.start().await,
            "mongodb" => self.mongo_manager.start().await,
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
            "postgres" => self.postgres_manager.stop().await,
            "mongodb" => self.mongo_manager.stop().await,
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
            "postgres" => self.postgres_manager.restart().await,
            "mongodb" => self.mongo_manager.restart().await,
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
            "postgres" => {
                if self.postgres_manager.is_running().await? {
                    Ok("running".into())
                } else {
                    Ok("stopped".into())
                }
            }
            "mongodb" => {
                if self.mongo_manager.is_running().await? {
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
            // PostgreSQL 无 MySQL 式字符集建库参数（模板库决定），charset 忽略
            "postgres" => self.postgres_manager.create_database(db_name, None).await,
            t => Err(AppError::BadRequest(format!(
                "Database creation not supported for: {}",
                t
            ))),
        }
    }

    pub async fn drop_database(&self, instance_id: i64, db_name: &str) -> Result<(), AppError> {
        let inst = self.get_instance(instance_id).await?;
        ensure_droppable(&inst.db_type, DropTarget::Database, db_name)?;
        match inst.db_type.as_str() {
            "mysql" | "mariadb" => self.mysql_manager.drop_database(db_name).await,
            "postgres" => self.postgres_manager.drop_database(db_name).await,
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
            "postgres" => self.postgres_manager.list_databases().await,
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
        let inst = self.get_instance(instance_id).await?;
        match inst.db_type.as_str() {
            "postgres" => {
                self.postgres_manager
                    .create_user(username, password, host)
                    .await
            }
            "mysql" | "mariadb" => {
                self.mysql_manager
                    .create_user(username, password, host)
                    .await
            }
            // 旧实现兜底落到 MySQL：对 Redis / MongoDB 实例会去连本机 MySQL 建用户
            t => Err(AppError::BadRequest(format!(
                "User management not supported for: {t}"
            ))),
        }
    }

    pub async fn drop_user(
        &self,
        instance_id: i64,
        username: &str,
        host: &str,
    ) -> Result<(), AppError> {
        let inst = self.get_instance(instance_id).await?;
        ensure_droppable(&inst.db_type, DropTarget::User, username)?;
        match inst.db_type.as_str() {
            "postgres" => self.postgres_manager.drop_user(username, host).await,
            "mysql" | "mariadb" => self.mysql_manager.drop_user(username, host).await,
            t => Err(AppError::BadRequest(format!(
                "User management not supported for: {t}"
            ))),
        }
    }

    pub async fn uninstall(&self, id: i64) -> Result<(), AppError> {
        let inst = self.get_instance(id).await?;
        match inst.db_type.as_str() {
            "mysql" | "mariadb" => self.mysql_manager.uninstall().await,
            "redis" => self.redis_manager.uninstall().await,
            "postgres" => self.postgres_manager.uninstall().await,
            "mongodb" => self.mongo_manager.uninstall().await,
            t => Err(AppError::BadRequest(format!(
                "Unknown database type: {}",
                t
            ))),
        }?;
        self.repo.delete(id).await
    }
}

/// 删除目标种类（系统库 / 超级用户保护）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropTarget {
    Database,
    User,
}

/// 拒绝删除引擎自身依赖的系统库与超级用户：`DROP DATABASE mysql` 会让实例无法启动，
/// `DROP USER root` / `postgres` 则让面板永久失去管理通道。名称按大小写不敏感比较（保守）。
pub fn ensure_droppable(db_type: &str, target: DropTarget, name: &str) -> Result<(), AppError> {
    let protected: &[&str] = match (db_type, target) {
        ("mysql" | "mariadb", DropTarget::Database) => {
            &["mysql", "information_schema", "performance_schema", "sys"]
        }
        ("mysql" | "mariadb", DropTarget::User) => &[
            "root",
            "mysql.sys",
            "mysql.session",
            "mysql.infoschema",
            "mariadb.sys",
        ],
        ("postgres", DropTarget::Database) => &["postgres", "template0", "template1"],
        ("postgres", DropTarget::User) => &["postgres"],
        _ => &[],
    };
    if protected.iter().any(|p| p.eq_ignore_ascii_case(name)) {
        let what = match target {
            DropTarget::Database => "system database",
            DropTarget::User => "built-in superuser",
        };
        return Err(AppError::BadRequest(format!(
            "refusing to drop {what} {name:?}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod drop_guard_tests {
    use super::*;

    #[test]
    fn system_databases_and_superusers_are_protected() {
        for (t, k, n) in [
            ("mysql", DropTarget::Database, "mysql"),
            ("mysql", DropTarget::Database, "INFORMATION_SCHEMA"),
            ("mariadb", DropTarget::Database, "sys"),
            ("mysql", DropTarget::User, "root"),
            ("postgres", DropTarget::Database, "template1"),
            ("postgres", DropTarget::Database, "postgres"),
            ("postgres", DropTarget::User, "postgres"),
        ] {
            assert!(ensure_droppable(t, k, n).is_err(), "{t} {k:?} {n}");
        }
    }

    #[test]
    fn user_objects_are_droppable() {
        for (t, k, n) in [
            ("mysql", DropTarget::Database, "mysql_app"),
            ("mysql", DropTarget::Database, "analysis"),
            ("mysql", DropTarget::User, "rooter"),
            ("postgres", DropTarget::Database, "app"),
            ("postgres", DropTarget::User, "root"),
        ] {
            assert!(ensure_droppable(t, k, n).is_ok(), "{t} {k:?} {n}");
        }
    }
}

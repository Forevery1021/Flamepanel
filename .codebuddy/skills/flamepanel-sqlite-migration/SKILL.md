---
name: flamepanel-sqlite-migration
description: 在 FlamePanel 新增 SQLite 表、加列、加索引、写 SqliteXxxRepository 时使用。当任务涉及 CREATE TABLE、ALTER TABLE、run_migrations、幂等迁移、sqlx 查询时使用。给出与 sqlite.rs 现有写法完全一致的模板与老库验证命令。
---

# FlamePanel · SQLite 迁移与仓储

## 1. 位置

- 迁移：`flame-kernel/src/infrastructure/sqlite.rs::run_migrations(pool)`（`grep -n "pub async fn run_migrations"`），**追加到函数末尾** `Ok(())` 之前；
- 加列 helper：同文件 `add_column_if_missing(pool, table, column, definition)`（已存在，直接用）；
- 仓储：同文件追加 `pub struct SqliteFooRepository { pool: SqlitePool }` + `impl FooRepository for SqliteFooRepository`（若 sqlite.rs 已拆为目录模块，则放 `sqlite/foo.rs`）。

## 2. 幂等模板（照抄）

```rust
// 新表
sqlx::query(
    "CREATE TABLE IF NOT EXISTS foos (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        enabled INTEGER NOT NULL DEFAULT 1,
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    )",
)
.execute(pool)
.await
.map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

// 索引（高频 WHERE / ORDER BY 列：外键、状态、时间）
sqlx::query("CREATE INDEX IF NOT EXISTS idx_foos_enabled ON foos(enabled)")
    .execute(pool).await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

// 老表加列：必须 add_column_if_missing，新列必须带 DEFAULT 或允许 NULL
add_column_if_missing(pool, "foos", "note", "TEXT").await?;
add_column_if_missing(pool, "foos", "priority", "INTEGER NOT NULL DEFAULT 0").await?;
```

## 3. 规范

| 规则 | 原因 |
|---|---|
| 只追加，不修改既有 DDL 字符串 | 老库不会重跑 CREATE；改了对老用户无效且造成新旧库不一致 |
| 不 DROP 列 / 表 | SQLite 限制 + 数据安全；废弃列注释 `-- deprecated` |
| 时间用 `TEXT` ISO-8601（`datetime('now')`），布尔用 `INTEGER 0/1` | 与全库一致 |
| 密码 / token 列只存哈希（bcrypt / sha256）或 `SecretBox` 密文 | 红线 |
| 表名复数 snake_case；索引 `idx_<表>_<列>` | 一致性 |
| 循环写用事务：`let mut tx = pool.begin().await?; … tx.commit().await?;` | 性能 + 原子 |

## 4. 仓储模板

```rust
pub struct SqliteFooRepository { pool: SqlitePool }
impl SqliteFooRepository { pub fn new(pool: SqlitePool) -> Self { Self { pool } } }

#[async_trait]
impl FooRepository for SqliteFooRepository {
    async fn create(&self, foo: &Foo) -> Result<Foo, AppError> {
        let id = sqlx::query("INSERT INTO foos (name, enabled) VALUES (?, ?)")
            .bind(&foo.name).bind(foo.enabled as i64)
            .execute(&self.pool).await
            .map_err(|e| AppError::internal(format!("insert foo: {e}")))?
            .last_insert_rowid();
        self.find_by_id(id).await?.ok_or_else(|| AppError::internal("foo vanished after insert"))
    }
    async fn find_by_id(&self, id: i64) -> Result<Option<Foo>, AppError> {
        let row = sqlx::query("SELECT id, name, enabled, created_at FROM foos WHERE id = ?")
            .bind(id).fetch_optional(&self.pool).await
            .map_err(|e| AppError::internal(format!("query foo: {e}")))?;
        Ok(row.map(|r| Foo {
            id: r.get("id"), name: r.get("name"),
            enabled: r.get::<i64, _>("enabled") != 0, created_at: r.get("created_at"),
        }))
    }
    async fn list(&self, offset: i64, limit: i64) -> Result<(Vec<Foo>, i64), AppError> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM foos").fetch_one(&self.pool).await
            .map_err(|e| AppError::internal(format!("count foos: {e}")))?;
        let rows = sqlx::query("SELECT id, name, enabled, created_at FROM foos ORDER BY id DESC LIMIT ? OFFSET ?")
            .bind(limit).bind(offset).fetch_all(&self.pool).await
            .map_err(|e| AppError::internal(format!("list foos: {e}")))?;
        Ok((rows.into_iter().map(|r| /* 同上映射 */ todo!()).collect(), total))
    }
    // update / delete：affected_rows == 0 → AppError::NotFound
}
```

`UNIQUE` 冲突映射：`e.as_database_error().map(|d| d.is_unique_violation()).unwrap_or(false)` → `AppError::Conflict`。

## 5. 验证

```bash
cd flame-kernel && cargo test --all                       # InMemory + SQLite 集成路径
# 老库幂等：用现有库启动两次
cp ../data/app.db /tmp/old.db 2>/dev/null || true
OP_DATABASE_URL="sqlite:/tmp/old.db?mode=rwc" timeout 10 cargo run -q ; OP_DATABASE_URL="sqlite:/tmp/old.db?mode=rwc" timeout 10 cargo run -q
```

## 6. 自查

- [ ] DDL 与任务单逐字段一致（不一致在 PR 说明）
- [ ] 新列有 DEFAULT；索引已建；无 DROP
- [ ] `Doc/03-数据库设计.md` 加表 / 列；CHANGELOG 一段


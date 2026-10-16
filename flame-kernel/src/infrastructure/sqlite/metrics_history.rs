/// S-07: 监控历史降采样点的 SQLite 仓储。
///
/// 单表 `metrics_history` 以 `(resolution, bucket_start)` 为主键同时存放 1 min / 1 h 两级点，
/// 写入走 write 池（每分钟一行，压力可忽略），区间查询走 read 池并命中主键前缀索引。
use super::*;

pub struct SqliteMetricsHistoryRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteMetricsHistoryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self::new_rw(pool.clone(), pool)
    }

    /// F-14: 读写双池构造——write 池串行写（max=1），read 池并发读（WAL）。
    pub fn new_rw(write_pool: SqlitePool, read_pool: SqlitePool) -> Self {
        Self {
            pool: write_pool,
            read_pool,
        }
    }
}

fn row_to_point(row: &sqlx::sqlite::SqliteRow) -> MetricsHistoryPoint {
    MetricsHistoryPoint {
        bucket_start: row.get::<i64, _>("bucket_start"),
        samples: row.get::<i64, _>("samples").max(0) as u32,
        cpu_avg: row.get::<f64, _>("cpu_avg") as f32,
        cpu_max: row.get::<f64, _>("cpu_max") as f32,
        memory_avg: row.get::<f64, _>("memory_avg") as f32,
        memory_max: row.get::<f64, _>("memory_max") as f32,
        disk_avg: row.get::<f64, _>("disk_avg") as f32,
        load_one_avg: row.get::<f64, _>("load_one_avg"),
        net_rx_avg_mbps: row.get::<f64, _>("net_rx_avg"),
        net_tx_avg_mbps: row.get::<f64, _>("net_tx_avg"),
    }
}

#[async_trait]
impl MetricsHistoryRepository for SqliteMetricsHistoryRepository {
    async fn upsert(
        &self,
        resolution: MetricsResolution,
        point: &MetricsHistoryPoint,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO metrics_history (resolution, bucket_start, samples, cpu_avg, cpu_max, memory_avg, memory_max, disk_avg, load_one_avg, net_rx_avg, net_tx_avg)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(resolution, bucket_start) DO UPDATE SET
               samples = excluded.samples, cpu_avg = excluded.cpu_avg, cpu_max = excluded.cpu_max,
               memory_avg = excluded.memory_avg, memory_max = excluded.memory_max, disk_avg = excluded.disk_avg,
               load_one_avg = excluded.load_one_avg, net_rx_avg = excluded.net_rx_avg, net_tx_avg = excluded.net_tx_avg",
        )
        .bind(resolution.as_str())
        .bind(point.bucket_start)
        .bind(i64::from(point.samples))
        .bind(f64::from(point.cpu_avg))
        .bind(f64::from(point.cpu_max))
        .bind(f64::from(point.memory_avg))
        .bind(f64::from(point.memory_max))
        .bind(f64::from(point.disk_avg))
        .bind(point.load_one_avg)
        .bind(point.net_rx_avg_mbps)
        .bind(point.net_tx_avg_mbps)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn query(
        &self,
        resolution: MetricsResolution,
        from: i64,
        to: i64,
        limit: usize,
    ) -> Result<Vec<MetricsHistoryPoint>, AppError> {
        let rows = sqlx::query(
            "SELECT bucket_start, samples, cpu_avg, cpu_max, memory_avg, memory_max, disk_avg, load_one_avg, net_rx_avg, net_tx_avg
             FROM metrics_history
             WHERE resolution = ? AND bucket_start >= ? AND bucket_start < ?
             ORDER BY bucket_start ASC
             LIMIT ?",
        )
        .bind(resolution.as_str())
        .bind(from)
        .bind(to)
        .bind(limit.min(i64::MAX as usize) as i64)
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(rows.iter().map(row_to_point).collect())
    }

    async fn prune(&self, resolution: MetricsResolution, before: i64) -> Result<u64, AppError> {
        let res =
            sqlx::query("DELETE FROM metrics_history WHERE resolution = ? AND bucket_start < ?")
                .bind(resolution.as_str())
                .bind(before)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(res.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(bucket_start: i64, cpu: f32) -> MetricsHistoryPoint {
        MetricsHistoryPoint {
            bucket_start,
            samples: 20,
            cpu_avg: cpu,
            cpu_max: cpu + 5.0,
            memory_avg: 50.0,
            memory_max: 55.0,
            disk_avg: 40.0,
            load_one_avg: 1.25,
            net_rx_avg_mbps: 0.5,
            net_tx_avg_mbps: 0.25,
        }
    }

    async fn repo() -> SqliteMetricsHistoryRepository {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();
        SqliteMetricsHistoryRepository::new(pool)
    }

    #[tokio::test]
    async fn upsert_query_prune_roundtrip() {
        let r = repo().await;
        for i in 0..5 {
            r.upsert(MetricsResolution::OneMinute, &point(60 * i, i as f32))
                .await
                .unwrap();
        }
        // 1h 点与 1m 点同键不冲突
        r.upsert(MetricsResolution::OneHour, &point(0, 99.0))
            .await
            .unwrap();

        let got = r
            .query(MetricsResolution::OneMinute, 60, 240, 100)
            .await
            .unwrap();
        assert_eq!(
            got.iter().map(|p| p.bucket_start).collect::<Vec<_>>(),
            vec![60, 120, 180]
        );
        assert_eq!(got[0], point(60, 1.0));

        // limit 生效且保持升序
        let limited = r
            .query(MetricsResolution::OneMinute, 0, 1000, 2)
            .await
            .unwrap();
        assert_eq!(limited.len(), 2);
        assert_eq!(limited[1].bucket_start, 60);

        // 同桶覆盖
        r.upsert(MetricsResolution::OneMinute, &point(60, 42.0))
            .await
            .unwrap();
        let again = r
            .query(MetricsResolution::OneMinute, 60, 120, 10)
            .await
            .unwrap();
        assert_eq!(again.len(), 1);
        assert_eq!(again[0].cpu_avg, 42.0);

        // prune 只影响指定分辨率
        let removed = r.prune(MetricsResolution::OneMinute, 180).await.unwrap();
        assert_eq!(removed, 3);
        let hourly = r
            .query(MetricsResolution::OneHour, 0, 10, 10)
            .await
            .unwrap();
        assert_eq!(hourly.len(), 1);
        assert_eq!(hourly[0].cpu_avg, 99.0);
    }
}

//! R-01: real BackupService snapshots and pre-restore protection, in disposable directories.
#![cfg(feature = "sqlite")]

use flame_kernel::application::backup_service::BackupService;
use flame_kernel::infrastructure::sqlite::run_migrations;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::{Path, PathBuf};
use std::process::Command;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

async fn database(path: &Path) -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true),
        )
        .await
        .unwrap();
    run_migrations(&pool).await.unwrap();
    pool
}

#[test]
fn restore_drill_python_contracts() {
    let output = Command::new("python3")
        .args([
            "-m",
            "unittest",
            "discover",
            "-s",
            "scripts/restore-drill",
            "-v",
        ])
        .current_dir(repository())
        .output()
        .expect("R-01 regression suite requires Python 3");
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn restore_drill_real_snapshot_and_pre_restore_backup() {
    let tmp = tempfile::tempdir().unwrap();
    let live = tmp.path().join("live.db");
    let pool = database(&live).await;
    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO panel_settings(key, value) VALUES ('r01_probe', 'snapshot-value')")
        .execute(&pool)
        .await
        .unwrap();
    let service = BackupService::new(&live, tmp.path().join("backups"));
    // Exercise the production snapshot path while the source pool is open in WAL mode.
    let entry = service.create_backup().await.unwrap();
    let snapshot = service.get_backup_path(&entry.filename).await.unwrap();
    let original_snapshot = std::fs::read(&snapshot).unwrap();
    sqlx::query("UPDATE panel_settings SET value = 'after-snapshot' WHERE key = 'r01_probe'")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let original_live = std::fs::read(&live).unwrap();

    let report_dir = tmp.path().join("report");
    let output = Command::new("python3")
        .arg(repository().join("scripts/restore-drill/drill.py"))
        .arg("--backup")
        .arg(&snapshot)
        .arg("--out")
        .arg(&report_dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(report_dir.join("report.json")).unwrap()).unwrap();
    assert_eq!(report["status"], "PASS");
    assert_eq!(report["kernel_started"], false);
    assert_eq!(report["copies_removed"], true);

    // Separately test the real restore method; target is a stopped, disposable DB, not live.
    let target = tmp.path().join("restore-target.db");
    let target_pool = database(&target).await;
    sqlx::query("INSERT INTO panel_settings(key, value) VALUES ('r01_probe', 'before-restore')")
        .execute(&target_pool)
        .await
        .unwrap();
    target_pool.close().await;
    let safety_dir = tmp.path().join("safety-backups");
    std::fs::create_dir(&safety_dir).unwrap();
    let target_service = BackupService::new(&target, &safety_dir);
    target_service
        .restore_backup_from_path(&snapshot)
        .await
        .unwrap();
    let restored = SqlitePoolOptions::new()
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&target)
                .read_only(true),
        )
        .await
        .unwrap();
    let value: String =
        sqlx::query_scalar("SELECT value FROM panel_settings WHERE key = 'r01_probe'")
            .fetch_one(&restored)
            .await
            .unwrap();
    assert_eq!(value, "snapshot-value");
    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&restored)
        .await
        .unwrap();
    assert_eq!(integrity, "ok");
    restored.close().await;
    let safety = target_service.list_backups().await.unwrap();
    assert_eq!(safety.len(), 1);
    assert!(safety[0].filename.starts_with("pre-restore-"));
    let safety_pool = SqlitePoolOptions::new()
        .connect_with(
            SqliteConnectOptions::new()
                .filename(safety_dir.join(&safety[0].filename))
                .read_only(true),
        )
        .await
        .unwrap();
    let previous: String =
        sqlx::query_scalar("SELECT value FROM panel_settings WHERE key = 'r01_probe'")
            .fetch_one(&safety_pool)
            .await
            .unwrap();
    assert_eq!(previous, "before-restore");
    safety_pool.close().await;
    assert_eq!(std::fs::read(&live).unwrap(), original_live);
    assert_eq!(std::fs::read(&snapshot).unwrap(), original_snapshot);

    // Optional evidence export contains only this synthetic fixture, never real panel data.
    if let Some(destination) = std::env::var_os("FP_RESTORE_DRILL_ARTIFACTS") {
        let destination = PathBuf::from(destination);
        std::fs::create_dir(&destination).unwrap();
        std::fs::copy(&snapshot, destination.join("synthetic-snapshot.db")).unwrap();
        for name in ["report.json", "report.md"] {
            std::fs::copy(report_dir.join(name), destination.join(name)).unwrap();
        }
    }
}

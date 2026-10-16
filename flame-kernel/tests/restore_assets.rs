//! R-01C inventory against a real migrated database and real SecretCipher ciphertext.
#![cfg(all(feature = "sqlite", unix))]

use flame_kernel::infrastructure::sqlite::run_migrations;
use flame_kernel::utils::SecretCipher;
use serde_json::json;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

#[tokio::test]
async fn restore_assets_real_cipher_requires_key_but_presence_does_not_prove_match() {
    let tmp = tempfile::tempdir().unwrap();
    let database = tmp.path().join("snapshot.db");
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&database)
                .create_if_missing(true),
        )
        .await
        .unwrap();
    run_migrations(&pool).await.unwrap();
    let encrypted = SecretCipher::new(b"synthetic-right-master")
        .encrypt(r#"{"secret_key":"synthetic-backup-credential"}"#)
        .unwrap();
    sqlx::query(
        "INSERT INTO backup_accounts(name,account_type,config_json) VALUES ('fixture','s3',?)",
    )
    .bind(&encrypted)
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;
    let original = std::fs::read(&database).unwrap();
    let assets = tmp.path().join("assets");
    std::fs::create_dir(&assets).unwrap();
    std::fs::set_permissions(&assets, std::fs::Permissions::from_mode(0o700)).unwrap();
    for name in ["deployment", "jwt"] {
        let file = assets.join(name);
        std::fs::write(&file, b"synthetic-config-only").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let manifest = tmp.path().join("manifest.json");
    std::fs::write(
        &manifest,
        serde_json::to_vec(&json!({
            "schema": "flamepanel/recovery-assets/v1", "jwt_strategy": "preserve",
            "assets": [
                {"role": "deployment_config", "path": "deployment"},
                {"role": "jwt_signing", "path": "jwt"},
                {"role": "encryption_master", "path": "master"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    for (name, expected_exit, expected_status) in
        [("missing", 1, "BLOCKED"), ("present", 0, "REVIEW_REQUIRED")]
    {
        if name == "present" {
            let key = assets.join("master");
            // Deliberately wrong material: a metadata inventory must NOT claim a matching key.
            std::fs::write(&key, b"synthetic-wrong-master").unwrap();
            std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let out = tmp.path().join(name);
        let output = Command::new("python3")
            .arg(repository.join("scripts/restore-drill/inventory.py"))
            .arg("--backup")
            .arg(&database)
            .arg("--assets-root")
            .arg(&assets)
            .arg("--manifest")
            .arg(&manifest)
            .arg("--out")
            .arg(&out)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(expected_exit),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let bytes = std::fs::read(out.join("inventory.json")).unwrap();
        let report: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(report["status"], expected_status);
        assert_eq!(report["restore_ready"], false);
        assert_eq!(report["key_match_verified"], false);
        assert_eq!(report["secret_contents_read"], false);
        assert_eq!(
            report["observations"]["encrypted_fields"]["backup_accounts.config_json"],
            1
        );
        assert_eq!(report["copies_removed"], true);
        let serialized = String::from_utf8(bytes).unwrap();
        assert!(!serialized.contains("synthetic-wrong-master"));
        assert!(!serialized.contains(&encrypted));
        assert_eq!(std::fs::read(&database).unwrap(), original);
        if let Some(root) = std::env::var_os("FP_INVENTORY_EVIDENCE") {
            let dest = Path::new(&root).join(name);
            std::fs::create_dir_all(&dest).unwrap();
            for file in ["inventory.json", "inventory.md"] {
                let destination = dest.join(file);
                assert!(
                    !destination.exists(),
                    "evidence must not overwrite a previous run"
                );
                std::fs::copy(out.join(file), destination).unwrap();
            }
        }
    }
}

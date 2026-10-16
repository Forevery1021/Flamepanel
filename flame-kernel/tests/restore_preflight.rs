//! R-01E-1: bind reports from a real snapshot/probe/inventory/keycheck chain, never production startup.
#![cfg(all(feature = "sqlite", unix))]

use flame_kernel::application::backup_service::BackupService;
use flame_kernel::infrastructure::sqlite::run_migrations;
use flame_kernel::utils::SecretCipher;
use serde_json::{json, Value};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn private_file(path: &Path, bytes: &[u8]) {
    std::fs::write(path, bytes).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
}

fn script(name: &str) -> Command {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut command = Command::new("python3");
    command.arg(repository.join("scripts/restore-drill").join(name));
    command.current_dir(repository);
    command
}

fn succeeds(command: &mut Command) {
    let result = command.output().unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

fn plan(
    root: &Path,
    out: &Path,
    drill_report: &str,
    inventory_report: &str,
    keycheck_report: Option<&str>,
    strategy: &str,
) -> Value {
    let mut command = script("preflight.py");
    command
        .arg("--evidence-root")
        .arg(root)
        .arg("--drill-report")
        .arg(drill_report)
        .arg("--inventory-report")
        .arg(inventory_report)
        .arg("--jwt-strategy")
        .arg(strategy)
        .arg("--out")
        .arg(out);
    if let Some(path) = keycheck_report {
        command.arg("--keycheck-report").arg(path);
    }
    let output = command.output().unwrap();
    let report: Value =
        serde_json::from_slice(&std::fs::read(out.join("preflight.json")).unwrap()).unwrap();
    let expected = if report["status"] == "REVIEW_REQUIRED" {
        0
    } else {
        1
    };
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for field in [
        "restore_ready",
        "production_auth_verified",
        "key_match_verified",
        "kernel_started",
        "secret_contents_read",
        "asset_secret_files_opened",
        "database_opened",
        "sessions_modified",
        "keys_generated_or_rotated",
        "report_authenticity_verified",
        "freshness_verified",
    ] {
        assert_eq!(report[field], false, "{field}");
    }
    for file in ["preflight.json", "preflight.md"] {
        let text = std::fs::read_to_string(out.join(file)).unwrap();
        for private in [
            "synthetic-e1-master-canary",
            "synthetic-e1-credential-canary",
            "synthetic-jwt-placeholder",
            root.to_str().unwrap(),
        ] {
            assert!(!text.contains(private));
        }
        assert_eq!(
            std::fs::metadata(out.join(file))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    assert_eq!(
        std::fs::metadata(out).unwrap().permissions().mode() & 0o777,
        0o700
    );
    if let Some(destination) = std::env::var_os("FP_PREFLIGHT_EVIDENCE") {
        let destination = PathBuf::from(destination);
        std::fs::create_dir_all(&destination).unwrap();
        std::fs::set_permissions(&destination, std::fs::Permissions::from_mode(0o700)).unwrap();
        let target = destination.join(out.file_name().unwrap());
        std::fs::create_dir(&target).expect("evidence must not overwrite an earlier scenario");
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o700)).unwrap();
        for file in ["preflight.json", "preflight.md"] {
            std::fs::copy(out.join(file), target.join(file)).unwrap();
        }
    }
    report
}

#[tokio::test]
async fn restore_preflight_real_chain_binds_snapshot_but_cannot_grant_identity_or_startup() {
    let tmp = tempfile::tempdir().unwrap();
    let live = tmp.path().join("live.db");
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&live)
                .create_if_missing(true),
        )
        .await
        .unwrap();
    run_migrations(&pool).await.unwrap();
    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(&pool)
        .await
        .unwrap();
    let encrypted = SecretCipher::new(b"synthetic-e1-master-canary")
        .encrypt("synthetic-e1-credential-canary")
        .unwrap();
    sqlx::query(
        "INSERT INTO backup_accounts(name,account_type,config_json) VALUES('e1-fixture','s3',?)",
    )
    .bind(&encrypted)
    .execute(&pool)
    .await
    .unwrap();
    let service = BackupService::new(&live, tmp.path().join("backups"));
    let entry = service.create_backup().await.unwrap();
    let snapshot = service.get_backup_path(&entry.filename).await.unwrap();
    pool.close().await;
    let original = std::fs::read(&snapshot).unwrap();

    let assets = tmp.path().join("assets");
    std::fs::create_dir(&assets).unwrap();
    std::fs::set_permissions(&assets, std::fs::Permissions::from_mode(0o700)).unwrap();
    private_file(&assets.join("master"), b"synthetic-e1-master-canary");
    private_file(&assets.join("deployment"), b"synthetic-deployment-config");
    // Deliberately just metadata: presence of this placeholder MUST NOT grant production identity.
    private_file(&assets.join("jwt"), b"synthetic-jwt-placeholder");
    let manifest = tmp.path().join("assets.json");
    let mut manifest_data = json!({"schema":"flamepanel/recovery-assets/v1","jwt_strategy":"preserve","assets":[
        {"role":"deployment_config","path":"deployment"}, {"role":"jwt_signing","path":"jwt"},
        {"role":"encryption_master","path":"master"}]});
    private_file(&manifest, &serde_json::to_vec(&manifest_data).unwrap());
    let evidence = tmp.path().join("evidence");
    std::fs::create_dir(&evidence).unwrap();
    std::fs::set_permissions(&evidence, std::fs::Permissions::from_mode(0o700)).unwrap();
    succeeds(
        script("drill.py")
            .arg("--backup")
            .arg(&snapshot)
            .arg("--probe-bin")
            .arg(env!("CARGO_BIN_EXE_fp-restore-probe"))
            .arg("--out")
            .arg(evidence.join("drill"))
            .arg("--timeout")
            .arg("30"),
    );
    succeeds(
        script("inventory.py")
            .arg("--backup")
            .arg(&snapshot)
            .arg("--assets-root")
            .arg(&assets)
            .arg("--manifest")
            .arg(&manifest)
            .arg("--out")
            .arg(evidence.join("inventory")),
    );
    succeeds(
        script("keycheck.py")
            .arg("--backup")
            .arg(&snapshot)
            .arg("--assets-root")
            .arg(&assets)
            .arg("--key-path")
            .arg("master")
            .arg("--keycheck-bin")
            .arg(env!("CARGO_BIN_EXE_fp-restore-keycheck"))
            .arg("--allow-secret-read")
            .arg("--out")
            .arg(evidence.join("keycheck")),
    );
    let originals: Vec<_> = [
        "drill/report.json",
        "inventory/inventory.json",
        "keycheck/keycheck.json",
    ]
    .iter()
    .map(|path| {
        (
            (*path).to_owned(),
            std::fs::read(evidence.join(path)).unwrap(),
        )
    })
    .collect();

    let valid = plan(
        &evidence,
        &tmp.path().join("consistent"),
        "drill/report.json",
        "inventory/inventory.json",
        Some("keycheck/keycheck.json"),
        "preserve",
    );
    assert_eq!(valid["status"], "REVIEW_REQUIRED");
    assert_eq!(valid["evidence_consistent"], true);
    assert_eq!(valid["encrypted_references"], 1);
    let mut mixed: Value = serde_json::from_slice(&originals[2].1).unwrap();
    let mut different = mixed["snapshot_sha256"].as_str().unwrap().to_owned();
    different.replace_range(0..1, if different.starts_with('a') { "b" } else { "a" });
    mixed["snapshot_sha256"] = json!(different);
    private_file(
        &evidence.join("keycheck/mixed.json"),
        &serde_json::to_vec(&mixed).unwrap(),
    );
    let mismatch = plan(
        &evidence,
        &tmp.path().join("mixed-source"),
        "drill/report.json",
        "inventory/inventory.json",
        Some("keycheck/mixed.json"),
        "preserve",
    );
    assert_eq!(mismatch["status"], "BLOCKED");
    assert_eq!(mismatch["error"]["code"], "SNAPSHOT_BINDING_MISMATCH");

    let mut escalated: Value = serde_json::from_slice(&originals[0].1).unwrap();
    escalated["probe"]["production_auth_verified"] = json!(true);
    private_file(
        &evidence.join("drill/escalated.json"),
        &serde_json::to_vec(&escalated).unwrap(),
    );
    let claim = plan(
        &evidence,
        &tmp.path().join("forged-auth"),
        "drill/escalated.json",
        "inventory/inventory.json",
        Some("keycheck/keycheck.json"),
        "preserve",
    );
    assert_eq!(claim["status"], "BLOCKED");
    let conflict = plan(
        &evidence,
        &tmp.path().join("policy-conflict"),
        "drill/report.json",
        "inventory/inventory.json",
        Some("keycheck/keycheck.json"),
        "reissue",
    );
    assert_eq!(conflict["error"]["code"], "JWT_PLAN_MISMATCH");
    let missing = plan(
        &evidence,
        &tmp.path().join("missing-key"),
        "drill/report.json",
        "inventory/inventory.json",
        None,
        "preserve",
    );
    assert_eq!(missing["error"]["code"], "KEYCHECK_EVIDENCE_REQUIRED");

    manifest_data["jwt_strategy"] = json!("reissue");
    private_file(&manifest, &serde_json::to_vec(&manifest_data).unwrap());
    succeeds(
        script("inventory.py")
            .arg("--backup")
            .arg(&snapshot)
            .arg("--assets-root")
            .arg(&assets)
            .arg("--manifest")
            .arg(&manifest)
            .arg("--out")
            .arg(evidence.join("inventory-reissue")),
    );
    let reissue = plan(
        &evidence,
        &tmp.path().join("reissue-plan"),
        "drill/report.json",
        "inventory-reissue/inventory.json",
        Some("keycheck/keycheck.json"),
        "reissue",
    );
    assert_eq!(reissue["status"], "REVIEW_REQUIRED");
    assert!(reissue["next_actions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "approve_session_reissue_no_action_performed"));
    assert_eq!(std::fs::read(&snapshot).unwrap(), original);
    assert_eq!(
        std::fs::read(assets.join("master")).unwrap(),
        b"synthetic-e1-master-canary"
    );
    assert_eq!(
        std::fs::read(assets.join("jwt")).unwrap(),
        b"synthetic-jwt-placeholder"
    );
    for (path, bytes) in originals {
        assert_eq!(std::fs::read(evidence.join(path)).unwrap(), bytes);
    }
}

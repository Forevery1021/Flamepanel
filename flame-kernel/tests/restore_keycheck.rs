//! R-01D: real migrations + SecretCipher + compiled worker, with synthetic material only.
#![cfg(all(feature = "sqlite", unix))]

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use flame_kernel::infrastructure::sqlite::run_migrations;
use flame_kernel::utils::SecretCipher;
use serde_json::{json, Value};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};

const MASTER: &str = "r01d-synthetic-master-canary";
const PLAINTEXT: &str = "r01d-private-credential-canary";
const WORKER_SCHEMA: &str = "flamepanel/restore-keycheck-worker/v1";

async fn open(path: &Path) -> sqlx::SqlitePool {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true),
        )
        .await
        .unwrap()
}

fn check(database: &Path, assets: &Path, out: &Path, authorized: bool) -> Value {
    let before = std::fs::read(database).unwrap();
    let key_before = std::fs::read(assets.join("master")).unwrap();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut command = Command::new("python3");
    command
        .arg(repository.join("scripts/restore-drill/keycheck.py"))
        .arg("--backup")
        .arg(database)
        .arg("--assets-root")
        .arg(assets)
        .arg("--key-path")
        .arg("master")
        .arg("--keycheck-bin")
        .arg(env!("CARGO_BIN_EXE_fp-restore-keycheck"))
        .arg("--out")
        .arg(out);
    if authorized {
        command.arg("--allow-secret-read");
    }
    let output = command.output().unwrap();
    let report: Value =
        serde_json::from_slice(&std::fs::read(out.join("keycheck.json")).unwrap()).unwrap();
    let expected = if report["status"] == "MATCHED_WITHIN_SCOPE" {
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
    assert_eq!(report["restore_ready"], false);
    assert_eq!(report["production_auth_verified"], false);
    assert_eq!(report["kernel_started"], false);
    assert_eq!(report["copies_removed"], true);
    assert_eq!(
        std::fs::metadata(out).unwrap().permissions().mode() & 0o777,
        0o700
    );
    for name in ["keycheck.json", "keycheck.md"] {
        let bytes = std::fs::read(out.join(name)).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        for forbidden in [
            MASTER,
            PLAINTEXT,
            "synthetic-wrong-master",
            database.to_str().unwrap(),
            assets.to_str().unwrap(),
        ] {
            assert!(!text.contains(forbidden));
        }
        assert!(!text.contains("enc:v1:"));
        assert_eq!(
            std::fs::metadata(out.join(name))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    assert_eq!(std::fs::read(database).unwrap(), before);
    assert_eq!(std::fs::read(assets.join("master")).unwrap(), key_before);
    if let Some(root) = std::env::var_os("FP_KEYCHECK_EVIDENCE") {
        let dest = Path::new(&root).join(out.file_name().unwrap());
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        std::fs::create_dir(&dest).expect("evidence must not overwrite a previous run");
        std::fs::set_permissions(
            dest.parent().unwrap(),
            std::fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o700)).unwrap();
        for file in ["keycheck.json", "keycheck.md"] {
            std::fs::copy(out.join(file), dest.join(file)).unwrap();
        }
    }
    report
}

#[tokio::test]
async fn restore_keycheck_real_ciphertext_is_readonly_scoped_and_requires_consent() {
    let tmp = tempfile::tempdir().unwrap();
    let database = tmp.path().join("snapshot.db");
    let pool = open(&database).await;
    run_migrations(&pool).await.unwrap();
    let cipher = SecretCipher::new(MASTER.as_bytes());
    let encrypted = cipher.encrypt(PLAINTEXT).unwrap();
    sqlx::query(
        "INSERT INTO backup_accounts(name,account_type,config_json) VALUES('r01d-fixture','s3',?)",
    )
    .bind(&encrypted)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO dns_provider_accounts(name,provider_type,credentials_json) VALUES('r01d-dns','cloudflare',?)")
        .bind(&encrypted).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO user_two_factor(user_id,secret,pending_secret,updated_at) VALUES(123,?,?, 'synthetic-time')")
        .bind(&encrypted).bind(&encrypted).execute(&pool).await.unwrap();
    sqlx::query("INSERT OR REPLACE INTO panel_settings(key,value) VALUES('assistant.api_key',?)")
        .bind(&encrypted)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let assets = tmp.path().join("assets");
    std::fs::create_dir(&assets).unwrap();
    std::fs::set_permissions(&assets, std::fs::Permissions::from_mode(0o700)).unwrap();
    let key = assets.join("master");
    std::fs::write(&key, format!(" \n{MASTER}\r\n")).unwrap();
    std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();

    let unauthorized = check(&database, &assets, &tmp.path().join("unauthorized"), false);
    assert_eq!(unauthorized["status"], "BLOCKED");
    assert_eq!(unauthorized["secret_contents_read"], false);
    let matched = check(&database, &assets, &tmp.path().join("matched"), true);
    assert_eq!(matched["status"], "MATCHED_WITHIN_SCOPE");
    assert_eq!(matched["key_match_verified"], true);
    assert_eq!(matched["secret_contents_read"], true);
    assert_eq!(matched["authenticated_records"], 5);
    assert_eq!(matched["rejected_records"], 0);

    std::fs::write(&key, b"synthetic-wrong-master").unwrap();
    let wrong = check(&database, &assets, &tmp.path().join("wrong-key"), true);
    assert_eq!(wrong["status"], "NOT_VERIFIED");
    assert_eq!(wrong["key_match_verified"], false);
    assert_eq!(wrong["authenticated_records"], 0);
    assert_eq!(wrong["rejected_records"], 5);
    std::fs::write(&key, MASTER).unwrap();

    let pool = open(&database).await;
    sqlx::query("UPDATE backup_accounts SET config_json='enc:v1:broken'")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let corrupt = check(&database, &assets, &tmp.path().join("corrupt-record"), true);
    assert_eq!(corrupt["status"], "NOT_VERIFIED");
    assert_eq!(corrupt["authenticated_records"], 4);
    assert_eq!(corrupt["rejected_records"], 1);
    assert_eq!(corrupt["key_match_verified"], false);

    let pool = open(&database).await;
    sqlx::query("UPDATE backup_accounts SET config_json='enc:v9:unknown'")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let unknown = check(
        &database,
        &assets,
        &tmp.path().join("unknown-version"),
        true,
    );
    assert_eq!(unknown["status"], "INCONCLUSIVE");
    assert_eq!(unknown["secret_contents_read"], false);
    assert_eq!(unknown["key_match_verified"], false);

    let pool = open(&database).await;
    sqlx::query("UPDATE backup_accounts SET config_json=?")
        .bind(&encrypted)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DROP TABLE user_two_factor")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let gap = check(&database, &assets, &tmp.path().join("coverage-gap"), true);
    assert_eq!(gap["status"], "INCONCLUSIVE");
    assert_eq!(gap["secret_contents_read"], false);
    assert_eq!(gap["key_match_verified"], false);
}

fn worker(input: &[u8], authorized: bool) -> std::process::Output {
    let tmp = tempfile::tempdir().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_fp-restore-keycheck"));
    if authorized {
        command.arg("--stdin-authorized");
    }
    let mut child = command
        .current_dir(tmp.path())
        .env("OP_DATABASE_URL", "/nonexistent-production-database")
        .env("OP_FILE_ROOT", "/nonexistent-production-files")
        .env("OP_JWT_SECRET", "not-the-master-key")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if authorized {
        child.stdin.take().unwrap().write_all(input).unwrap();
    }
    let output = child.wait_with_output().unwrap();
    assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 0);
    output
}

#[test]
fn restore_keycheck_worker_protocol_has_no_legacy_passthrough_or_secret_output() {
    let encrypted = SecretCipher::new(MASTER.as_bytes())
        .encrypt(PLAINTEXT)
        .unwrap();
    let request = json!({"schema": WORKER_SCHEMA, "master_b64": BASE64.encode(MASTER), "ciphertexts": [encrypted.clone()]});
    let valid = worker(&serde_json::to_vec(&request).unwrap(), true);
    assert!(valid.status.success());
    let reply: Value = serde_json::from_slice(&valid.stdout).unwrap();
    assert_eq!(reply["authenticated"], 1);
    for forbidden in [MASTER, PLAINTEXT, encrypted.as_str()] {
        assert!(!String::from_utf8_lossy(&valid.stdout).contains(forbidden));
        assert!(!String::from_utf8_lossy(&valid.stderr).contains(forbidden));
    }
    let denied = worker(b"", false);
    assert!(!denied.status.success());
    for value in ["plaintext-canary", "enc:v2:unknown", ""] {
        let mut invalid = request.clone();
        invalid["ciphertexts"] = json!([value]);
        let output = worker(&serde_json::to_vec(&invalid).unwrap(), true);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains(value) || value.is_empty());
    }
    let mut extra = request;
    extra["plaintext"] = json!(PLAINTEXT);
    let output = worker(&serde_json::to_vec(&extra).unwrap(), true);
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stderr).contains(PLAINTEXT));
}

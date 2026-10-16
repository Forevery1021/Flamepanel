//! R-01B dedicated-process contracts, included by the existing cargo test gate.
#![cfg(all(feature = "sqlite", unix))]

use std::path::Path;
use std::process::Command;

#[test]
fn restore_probe_real_process_contracts() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let output = Command::new("python3")
        .arg(repository.join("scripts/restore-drill/probe_contracts.py"))
        .arg(env!("CARGO_BIN_EXE_fp-restore-probe"))
        .current_dir(repository)
        .output()
        .expect("R-01B probe contracts require Python 3");
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

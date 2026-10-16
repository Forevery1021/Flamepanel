//! R-01D internal crypto worker. No file paths, environment configuration, services or network.
//! Use the Python offline driver; this process accepts explicitly authorized material via stdin.
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use flame_kernel::utils::SecretCipher;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{Read, Write};

const SCHEMA: &str = "flamepanel/restore-keycheck-worker/v1";
const REQUEST_LIMIT: usize = 2 * 1024 * 1024;
const MASTER_LIMIT: usize = 4096;
const RECORD_LIMIT: usize = 1024;
const VALUE_LIMIT: usize = 64 * 1024;
const CIPHERTEXT_LIMIT: usize = 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema: String,
    master_b64: String,
    ciphertexts: Vec<String>,
}

fn verify(request: Request) -> Result<Value, &'static str> {
    if request.schema != SCHEMA
        || request.master_b64.len() > 4 * MASTER_LIMIT.div_ceil(3)
        || request.ciphertexts.is_empty()
        || request.ciphertexts.len() > RECORD_LIMIT
    {
        return Err("INVALID_REQUEST");
    }
    let mut total = 0usize;
    for stored in &request.ciphertexts {
        // SecretCipher intentionally passes historical plaintext through. That compatibility
        // behavior MUST NOT be used as evidence that an unknown ciphertext/key matched.
        if !stored.starts_with("enc:v1:") || stored.len() > VALUE_LIMIT {
            return Err("UNSUPPORTED_OR_OVERSIZED_CIPHERTEXT");
        }
        total = total.checked_add(stored.len()).ok_or("INVALID_REQUEST")?;
        if total > CIPHERTEXT_LIMIT {
            return Err("CIPHERTEXT_LIMIT");
        }
    }
    let bytes = BASE64
        .decode(&request.master_b64)
        .map_err(|_| "INVALID_MASTER_MATERIAL")?;
    if bytes.len() > MASTER_LIMIT {
        return Err("INVALID_MASTER_MATERIAL");
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| "INVALID_MASTER_MATERIAL")?;
    // Exact normalization used by load_or_create_master_secret; never call that writer here.
    let master = text.trim();
    if master.is_empty() {
        return Err("INVALID_MASTER_MATERIAL");
    }
    let cipher = SecretCipher::new(master.as_bytes());
    let mut authenticated = 0;
    for stored in &request.ciphertexts {
        // Decrypted values stay in this process and are immediately dropped, never serialized.
        if cipher.decrypt(stored).is_ok() {
            authenticated += 1;
        }
    }
    Ok(json!({
        "schema": SCHEMA,
        "checked": request.ciphertexts.len(),
        "authenticated": authenticated,
        "rejected": request.ciphertexts.len() - authenticated,
    }))
}

fn run() -> Result<(), &'static str> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 1 || args[0] != "--stdin-authorized" {
        return Err("EXPLICIT_STDIN_AUTHORIZATION_REQUIRED");
    }
    let mut input = Vec::new();
    std::io::stdin()
        .take((REQUEST_LIMIT + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| "INPUT_FAILED")?;
    if input.len() > REQUEST_LIMIT {
        return Err("REQUEST_LIMIT");
    }
    let request: Request = serde_json::from_slice(&input).map_err(|_| "INVALID_REQUEST")?;
    let result = verify(request)?;
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, &result).map_err(|_| "OUTPUT_FAILED")?;
    writeln!(stdout).map_err(|_| "OUTPUT_FAILED")?;
    Ok(())
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(code) => {
            // Never print raw serde/crypto errors or the request, even on malformed input.
            eprintln!("RESTORE_KEYCHECK_WORKER: {code}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(master: &[u8], ciphertexts: Vec<String>) -> Request {
        Request {
            schema: SCHEMA.to_owned(),
            master_b64: BASE64.encode(master),
            ciphertexts,
        }
    }

    #[test]
    fn matching_material_uses_persisted_file_trimming_without_emitting_values() {
        let ciphertext = SecretCipher::new(b"synthetic-master")
            .encrypt("private-fixture-value")
            .unwrap();
        let result = verify(request(
            b" \nsynthetic-master\r\n",
            vec![ciphertext.clone()],
        ))
        .unwrap();
        assert_eq!(result["authenticated"], 1);
        assert_eq!(result["rejected"], 0);
        let output = result.to_string();
        for value in ["synthetic-master", "private-fixture-value", &ciphertext] {
            assert!(!output.contains(value));
        }
    }

    #[test]
    fn wrong_material_and_corruption_are_not_misreported_as_a_match() {
        let stored = SecretCipher::new(b"fixture-a").encrypt("hidden").unwrap();
        let wrong = verify(request(b"fixture-b", vec![stored.clone()])).unwrap();
        assert_eq!(wrong["authenticated"], 0);
        assert_eq!(wrong["rejected"], 1);
        let mixed = verify(request(
            b"fixture-a",
            vec![stored, "enc:v1:broken".to_owned()],
        ))
        .unwrap();
        assert_eq!(mixed["checked"], 2);
        assert_eq!(mixed["authenticated"], 1);
        assert_eq!(mixed["rejected"], 1);
    }

    #[test]
    fn plaintext_and_unknown_versions_cannot_use_the_legacy_passthrough() {
        for value in ["legacy plaintext", "enc:v2:unknown", "enc:unknown", ""] {
            assert!(verify(request(b"k", vec![value.to_owned()])).is_err());
        }
    }

    #[test]
    fn missing_invalid_or_unbounded_inputs_are_rejected() {
        assert!(verify(request(b"k", Vec::new())).is_err());
        assert!(verify(request(b" \n", vec!["enc:v1:x".to_owned()])).is_err());
        assert!(verify(request(&[0xff], vec!["enc:v1:x".to_owned()])).is_err());
        assert!(verify(request(
            &[b'x'; MASTER_LIMIT + 1],
            vec!["enc:v1:x".to_owned()]
        ))
        .is_err());
        assert!(verify(request(b"k", vec!["enc:v1:x".to_owned(); RECORD_LIMIT + 1])).is_err());
        assert!(verify(request(
            b"k",
            vec![format!("enc:v1:{}", "x".repeat(VALUE_LIMIT))]
        ))
        .is_err());
    }

    #[test]
    fn invalid_transport_and_aggregate_ciphertext_budget_are_rejected() {
        let mut invalid = request(b"k", vec!["enc:v1:x".to_owned()]);
        invalid.master_b64 = "not-valid-base64!".to_owned();
        assert!(verify(invalid).is_err());
        let large = format!("enc:v1:{}", "x".repeat(VALUE_LIMIT - 7));
        assert!(verify(request(b"k", vec![large; 17])).is_err());
    }

    #[test]
    fn protocol_rejects_extra_and_duplicate_fields() {
        let extra =
            br#"{"schema":"x","master_b64":"aw==","ciphertexts":[],"secret":"not-allowed"}"#;
        assert!(serde_json::from_slice::<Request>(extra).is_err());
        let duplicate = br#"{"schema":"x","schema":"y","master_b64":"aw==","ciphertexts":[]}"#;
        assert!(serde_json::from_slice::<Request>(duplicate).is_err());
    }
}

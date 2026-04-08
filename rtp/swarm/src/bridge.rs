//! Bridge — Python↔Rust typed interface.
//!
//! The Trading Wing calls the Python fractal-swarm binary (night_shift.bin)
//! through this bridge and receives typed JSON proposals back.
//!
//! Week 3: stub binary path, tested with captured/fake JSON output.
//! Week 4: swap `NIGHT_SHIFT_BIN` to the real PyInstaller binary.

use serde::{Deserialize, Serialize};
use std::io::Write as IoWrite;
use thiserror::Error;

/// Path to the Python fractal-swarm binary.
/// Week 4: swap to the real PyInstaller output.
pub const NIGHT_SHIFT_BIN: &str = "night_shift.bin";

/// Request sent to the Python fractal-swarm binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeRequest {
    /// Trading symbol (e.g. "SOL/USDT").
    pub symbol: String,
    /// Strategy configuration as JSON.
    pub config: serde_json::Value,
}

impl BridgeRequest {
    pub fn new(symbol: &str, config: serde_json::Value) -> Self {
        Self {
            symbol: symbol.to_string(),
            config,
        }
    }
}

/// Response from the Python fractal-swarm binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeResponse {
    /// Proposed strategy identifier.
    pub strategy: String,
    /// Estimated annual yield (USDC).
    pub yield_estimate: f64,
    /// Confidence score (0.0–1.0).
    pub confidence: f64,
    /// Strategy parameters.
    pub params: serde_json::Value,
    /// Number of walk-forward folds validated.
    pub folds_validated: u32,
    /// Consistency score across folds (0.0–1.0).
    pub consistency: f64,
}

/// Errors from the bridge subprocess call.
#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("Binary not found: {0}")]
    BinaryNotFound(String),
    #[error("Process failed: {0}")]
    ProcessFailed(String),
    #[error("Output parse error: {0}")]
    ParseError(String),
}

/// Call the Python binary with a typed request and receive a typed response.
/// Uses `NIGHT_SHIFT_BIN` as the binary path.
pub fn call_bridge(request: &BridgeRequest) -> Result<BridgeResponse, BridgeError> {
    call_bridge_with_bin(NIGHT_SHIFT_BIN, request)
}

/// Call the bridge using a custom binary path (for testing).
pub fn call_bridge_with_bin(
    bin_path: &str,
    request: &BridgeRequest,
) -> Result<BridgeResponse, BridgeError> {
    let input = serde_json::to_string(request)
        .map_err(|e| BridgeError::ProcessFailed(format!("Serialize error: {}", e)))?;

    let mut child = std::process::Command::new(bin_path)
        .arg("--bridge-mode")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BridgeError::BinaryNotFound(bin_path.to_string())
            } else {
                BridgeError::ProcessFailed(format!("Spawn error: {}", e))
            }
        })?;

    // Write request JSON to stdin, then close it.
    if let Some(ref mut stdin) = child.stdin {
        stdin
            .write_all(input.as_bytes())
            .map_err(|e| BridgeError::ProcessFailed(format!("Stdin write: {}", e)))?;
    }
    drop(child.stdin.take());

    let output = child
        .wait_with_output()
        .map_err(|e| BridgeError::ProcessFailed(format!("Process error: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BridgeError::ProcessFailed(format!(
            "Binary exited with {}: {}",
            output.status, stderr
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str::<BridgeResponse>(&stdout).map_err(|e| {
        BridgeError::ParseError(format!(
            "Parse error: {} (output: {})",
            e,
            &stdout[..stdout.len().min(200)]
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_response() -> BridgeResponse {
        BridgeResponse {
            strategy: "mr_rsi_bb".to_string(),
            yield_estimate: 118.3,
            confidence: 0.92,
            params: serde_json::json!({
                "rsi_entry": 28,
                "rsi_exit": 72,
                "bb_period": 20,
                "stop_loss": 0.03,
                "hold_hours": 48,
            }),
            folds_validated: 9,
            consistency: 0.78,
        }
    }

    /// Write a mock shell script that echoes a given JSON string.
    fn write_mock_bin(path: &std::path::Path, output: &str) {
        // Clean up stale file from a previous run.
        let _ = std::fs::remove_file(path);
        std::fs::write(path, format!("#!/bin/bash\necho '{}'", output)).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        // Small pause to avoid ETXTBSY (text file busy) on Linux — the kernel
        // can return this error if execve() races with the final write+chmod.
        #[cfg(unix)]
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    // ── Serialization round-trips ─────────────────────────────────────

    #[test]
    fn request_roundtrip() {
        let req = BridgeRequest::new("SOL/USDT", serde_json::json!({"mode": "optimization"}));
        let json = serde_json::to_string(&req).unwrap();
        let parsed: BridgeRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.symbol, "SOL/USDT");
        assert_eq!(parsed.config["mode"], "optimization");
    }

    #[test]
    fn response_roundtrip() {
        let resp = sample_response();
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: BridgeResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.strategy, "mr_rsi_bb");
        assert!((parsed.yield_estimate - 118.3).abs() < 0.01);
        assert!((parsed.confidence - 0.92).abs() < 0.01);
        assert_eq!(parsed.folds_validated, 9);
        assert_eq!(parsed.consistency, 0.78);
    }

    #[test]
    fn response_parses_from_realistic_json() {
        let json = r#"{
            "strategy": "mr_rsi_bb",
            "yield_estimate": 118.3,
            "confidence": 0.92,
            "params": {"rsi_entry": 28, "rsi_exit": 72},
            "folds_validated": 9,
            "consistency": 0.78
        }"#;
        let resp: BridgeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.strategy, "mr_rsi_bb");
        assert_eq!(resp.params["rsi_entry"], 28);
    }

    #[test]
    fn response_rejects_incomplete_json() {
        let json = r#"{"strategy": "mr"}"#;
        assert!(serde_json::from_str::<BridgeResponse>(json).is_err());
    }

    #[test]
    fn response_rejects_empty_json() {
        assert!(serde_json::from_str::<BridgeResponse>("{}").is_err());
    }

    // ── Subprocess error handling ─────────────────────────────────────

    #[test]
    fn missing_binary_returns_not_found() {
        let req = BridgeRequest::new("SOL/USDT", serde_json::json!({}));
        let result = call_bridge_with_bin("nonexistent_rtp_test_bin", &req);
        assert!(matches!(result, Err(BridgeError::BinaryNotFound(_))));
    }

    #[test]
    fn missing_binary_err_contains_path() {
        let req = BridgeRequest::new("SOL/USDT", serde_json::json!({}));
        let err = call_bridge_with_bin("missing_test_bin", &req).unwrap_err();
        assert!(err.to_string().contains("missing_test_bin"));
    }

    // ── Mock binary tests ─────────────────────────────────────────────

    /// Generate a unique temp path to avoid parallel test collisions.
    fn unique_tmp(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("rtp_bridge_test_{}_{}", label, uuid::Uuid::new_v4()))
    }

    #[test]
    fn mock_binary_success() {
        let tmp = unique_tmp("ok");
        write_mock_bin(&tmp, &serde_json::to_string(&sample_response()).unwrap());

        let req = BridgeRequest::new("SOL/USDT", serde_json::json!({"mode": "test"}));
        let resp = call_bridge_with_bin(tmp.to_str().unwrap(), &req).unwrap();

        let _ = std::fs::remove_file(&tmp);
        assert_eq!(resp.strategy, "mr_rsi_bb");
        assert!((resp.confidence - 0.92).abs() < 0.01);
        assert_eq!(resp.folds_validated, 9);
    }

    #[test]
    fn mock_binary_malformed_output() {
        let tmp = unique_tmp("bad");
        write_mock_bin(&tmp, "not valid json");

        let req = BridgeRequest::new("SOL/USDT", serde_json::json!({}));
        let result = call_bridge_with_bin(tmp.to_str().unwrap(), &req);

        let _ = std::fs::remove_file(&tmp);
        match &result {
            Err(BridgeError::ParseError(_)) => {}
            other => panic!(
                "Expected ParseError, got: {:?} (tmp={})",
                other,
                tmp.display()
            ),
        }
    }

    #[test]
    fn mock_binary_nonzero_exit() {
        let tmp = unique_tmp("fail");
        // Script that exits non-zero (no echo, so no stdout to parse).
        std::fs::write(&tmp, "#!/bin/bash\nexit 1").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        let req = BridgeRequest::new("SOL/USDT", serde_json::json!({}));
        let result = call_bridge_with_bin(tmp.to_str().unwrap(), &req);

        let _ = std::fs::remove_file(&tmp);
        assert!(matches!(result, Err(BridgeError::ProcessFailed(_))));
    }

    // ── Constants ─────────────────────────────────────────────────────

    #[test]
    fn night_shift_bin_constant_is_swappable() {
        assert_eq!(NIGHT_SHIFT_BIN, "night_shift.bin");
    }
}

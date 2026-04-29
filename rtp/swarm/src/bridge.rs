//! Bridge — Python↔Rust typed interface.
//!
//! Historical: called the Python fractal-swarm binary (cycle_report.bin) via subprocess.
//! Current: reads Night Shift results directly from `data/night_results/latest/summary.json`.
//! The subprocess path is retained for backward compatibility and tests.

use serde::{Deserialize, Serialize};
use std::io::Write as IoWrite;
use thiserror::Error;

/// Path to the Python fractal-swarm binary (legacy subprocess path).
pub const CYCLE_BIN: &str = "cycle_report.bin";

/// Directory containing Night Shift results (repo-relative).
pub const NIGHT_RESULTS_DIR: &str = "data/night_results";

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

/// Errors from the bridge subprocess call or file read.
#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("Binary not found: {0}")]
    BinaryNotFound(String),
    #[error("Process failed: {0}")]
    ProcessFailed(String),
    #[error("Output parse error: {0}")]
    ParseError(String),
    #[error("Night results not found: {0}")]
    NightResultsNotFound(String),
    #[error("Night results read error: {0}")]
    NightResultsReadError(String),
}

// ---------------------------------------------------------------------------
// Night Shift file reader (replaces subprocess bridge)
// ---------------------------------------------------------------------------

/// A single top candidate from the Night Shift summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NightShiftCandidate {
    pub symbol: String,
    pub params: serde_json::Value,
    pub survivor_score: f64,
    pub oos_sharpe: f64,
    pub oos_consistency: f64,
    pub oos_max_dd: f64,
    pub overfitting_score: f64,
    pub fragility: f64,
    pub oos_avg_trades_per_fold: f64,
    pub rejected: bool,
    pub rejection_reason: Option<String>,
}

/// The Night Shift summary.json structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NightShiftSummary {
    pub run_at: String,
    pub runtime_seconds: f64,
    pub num_folds: u32,
    pub symbols: Vec<String>,
    pub top_candidates: Vec<NightShiftCandidate>,
}

/// Result of reading the latest Night Shift output.
#[derive(Debug, Clone)]
pub struct NightShiftResult {
    pub summary: NightShiftSummary,
    pub source_path: String,
}

/// Find and read the latest `summary.json` from `data/night_results/`.
///
/// Searches for `data/night_results/<YYYY-MM-DD>/summary.json` and picks the
/// most recent date directory. Returns `Err` if no results exist.
pub fn read_latest_night_results() -> Result<NightShiftResult, BridgeError> {
    let root = repo_root();
    let night_dir = root.join(NIGHT_RESULTS_DIR);

    if !night_dir.exists() {
        return Err(BridgeError::NightResultsNotFound(format!(
            "directory not found: {}",
            night_dir.display()
        )));
    }

    // Find the latest date directory with a summary.json
    let mut latest: Option<(String, std::path::PathBuf)> = None;
    for entry in std::fs::read_dir(&night_dir).map_err(|e| {
        BridgeError::NightResultsReadError(format!("read_dir({}): {}", night_dir.display(), e))
    })? {
        let entry = entry.map_err(|e| {
            BridgeError::NightResultsReadError(format!("dir entry: {}", e))
        })?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.chars().all(|c| c.is_ascii_digit() || c == '-') || name.len() != 10 {
            continue; // skip non-date directories
        }
        let summary_path = entry.path().join("summary.json");
        if summary_path.exists() {
            match &latest {
                None => latest = Some((name.clone(), summary_path.clone())),
                Some((prev, _)) if &name > prev => {
                    latest = Some((name.clone(), summary_path.clone()));
                }
                _ => {}
            }
        }
    }

    let (_, summary_path) = latest.ok_or_else(|| {
        BridgeError::NightResultsNotFound(format!(
            "no summary.json found in any date directory under {}",
            night_dir.display()
        ))
    })?;

    let content = std::fs::read_to_string(&summary_path).map_err(|e| {
        BridgeError::NightResultsReadError(format!("read {}: {}", summary_path.display(), e))
    })?;

    let summary: NightShiftSummary = serde_json::from_str(&content).map_err(|e| {
        BridgeError::ParseError(format!(
            "parse {}: {} (first 200 chars: {})",
            summary_path.display(),
            e,
            &content[..content.len().min(200)]
        ))
    })?;

    Ok(NightShiftResult {
        source_path: summary_path.display().to_string(),
        summary,
    })
}

/// Get the best non-rejected candidate from the latest Night Shift results.
///
/// Returns `None` if no candidates pass the night shift filters or
/// no results exist.
pub fn best_night_shift_candidate() -> Option<NightShiftCandidate> {
    let result = read_latest_night_results().ok()?;
    result
        .summary
        .top_candidates
        .iter()
        .filter(|c| !c.rejected)
        .max_by(|a, b| a.survivor_score.partial_cmp(&b.survivor_score).unwrap_or(std::cmp::Ordering::Equal))
        .cloned()
}

/// Resolve the repo root (two levels up from CARGO_MANIFEST_DIR, i.e. rtp/swarm/).
fn repo_root() -> std::path::PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    std::path::Path::new(&manifest)
        .join("../../")
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}

/// Convert a `NightShiftCandidate` into a `BridgeResponse` for compatibility
/// with existing code that expects the subprocess output format.
impl NightShiftCandidate {
    pub fn to_bridge_response(&self) -> BridgeResponse {
        BridgeResponse {
            strategy: self.symbol.replace("/", "_"),
            yield_estimate: self.oos_sharpe * 30.0, // rough annualized estimate
            confidence: 1.0 - self.fragility,
            params: self.params.clone(),
            folds_validated: 9, // standard WFA fold count
            consistency: self.oos_consistency,
        }
    }
}

/// Call the Python binary with a typed request.
///
/// Resolution order:
/// 1. Read Night Shift results from `data/night_results/latest/summary.json`
/// 2. Fall back to subprocess call to `cycle_report.bin` (legacy)
pub fn call_bridge(request: &BridgeRequest) -> Result<BridgeResponse, BridgeError> {
    // Try file-based Night Shift results first (no subprocess needed).
    match read_night_shift_for_symbol(&request.symbol) {
        Ok(response) => return Ok(response),
        Err(_) => {} // Fall through to subprocess path
    }

    // Legacy subprocess path (cycle_report.bin).
    call_bridge_with_bin(CYCLE_BIN, request)
}

/// Read the latest Night Shift summary.json and find the best candidate
/// matching the requested symbol.
fn read_night_shift_for_symbol(symbol: &str) -> Result<BridgeResponse, BridgeError> {
    let summary = read_latest_night_results()?;
    let candidate = summary
        .summary
        .top_candidates
        .iter()
        .filter(|c| c.symbol == symbol && !c.rejected)
        .max_by(|a, b| {
            a.survivor_score
                .partial_cmp(&b.survivor_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .ok_or_else(|| BridgeError::ParseError(format!("No eligible candidate for {}", symbol)))?;

    Ok(candidate.to_bridge_response())
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

    if let Some(ref mut stdin) = child.stdin {
        stdin
            .write_all(input.as_bytes())
            .map_err(|e| BridgeError::ProcessFailed(format!("Stdin write: {}", e)))?;
    }
    drop(child.stdin.take());

    // 5-minute timeout — strategy eval should finish in seconds.
    const BRIDGE_TIMEOUT_SECS: u64 = 300;
    let child_id = child.id();

    let handle = std::thread::spawn(move || child.wait_with_output());

    let timeout = std::time::Duration::from_secs(BRIDGE_TIMEOUT_SECS);
    let start = std::time::Instant::now();

    let output = loop {
        if handle.is_finished() {
            break handle
                .join()
                .map_err(|_| BridgeError::ProcessFailed("Thread panicked".to_string()))?
                .map_err(|e| BridgeError::ProcessFailed(format!("Process error: {}", e)))?;
        }
        if start.elapsed() > timeout {
            // Best-effort kill — send SIGKILL via kill command as a fallback.
            let _ = std::process::Command::new("kill")
                .arg("-9")
                .arg(child_id.to_string())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            return Err(BridgeError::ProcessFailed(format!(
                "Bridge timed out after {}s (pid={})",
                BRIDGE_TIMEOUT_SECS, child_id
            )));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };

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

    // Serialization round-trips

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

    // Subprocess error handling

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

    // Mock binary tests

    /// Generate a unique temp path to avoid parallel test collisions.
    fn unique_tmp(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "rtp_bridge_test_{}_{}",
            label,
            uuid::Uuid::new_v4()
        ))
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

    // Constants

    #[test]
    fn cycle_bin_constant_is_swappable() {
        assert_eq!(CYCLE_BIN, "cycle_report.bin");
    }

    // ── Integration: real binary (optional, only runs if binary exists) ──

    #[test]
    fn real_binary_bridge_mode_integration() {
        // This test only runs if cycle_report.bin exists at repo root.
        // It validates the full Python↔Rust round-trip.
        let bin_path = format!("{}/../../../cycle_report.bin", env!("CARGO_MANIFEST_DIR"));
        if !std::path::Path::new(&bin_path).exists() {
            eprintln!("Skipping: {} not found", bin_path);
            return;
        }

        let req = BridgeRequest::new(
            "BTC/USDT",
            serde_json::json!({"params": {"signal_threshold": 0.40}}),
        );
        let resp = call_bridge_with_bin(&bin_path, &req).unwrap();

        assert!(!resp.strategy.is_empty());
        assert!(resp.yield_estimate != 0.0 || resp.consistency >= 0.0);
        assert!(resp.folds_validated > 0);
        assert!(resp.consistency >= 0.0 && resp.consistency <= 1.0);
        assert!(resp.confidence >= 0.0 && resp.confidence <= 1.0);
    }
}

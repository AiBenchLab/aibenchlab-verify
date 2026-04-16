#![recursion_limit = "256"]

use clap::Parser;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process;

// ============================================================
// CLI
// ============================================================

/// Standalone integrity verifier for AiBenchLab MBX v2 files.
///
/// Verifies that an MBX file has not been modified since export by
/// recomputing the SHA-256 content hash and comparing it to the
/// stored value. Implements Section 6 of the MBX v2 Specification.
#[derive(Parser)]
#[command(name = "aibenchlab-verify", version, about)]
struct Cli {
    /// Path to an .mbx.json file, or a directory when used with --batch
    path: PathBuf,

    /// Verify all .mbx.json files in the given directory
    #[arg(long)]
    batch: bool,

    /// Show detailed field summary alongside the result
    #[arg(long)]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.batch {
        let code = run_batch(&cli.path, cli.verbose);
        process::exit(code);
    } else {
        let code = run_single(&cli.path, cli.verbose);
        process::exit(code);
    }
}

// ============================================================
// Single file verification
// ============================================================

fn run_single(path: &Path, verbose: bool) -> i32 {
    let filename = path.display();

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("\u{274c} ERROR: {filename}\n   Could not read file: {e}");
            return 1;
        }
    };

    match verify_mbx_content(&content) {
        Ok(result) => {
            if result.verified {
                print_pass(path, &result, verbose);
                0
            } else {
                print_fail(path, &result);
                1
            }
        }
        Err(e) => {
            eprintln!("\u{274c} ERROR: {filename}\n   {e}");
            1
        }
    }
}

// ============================================================
// Batch mode
// ============================================================

fn run_batch(dir: &Path, verbose: bool) -> i32 {
    if !dir.is_dir() {
        eprintln!("\u{274c} ERROR: {} is not a directory", dir.display());
        return 1;
    }

    let entries: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension().map_or(false, |ext| ext == "json")
                    && p.to_string_lossy().ends_with(".mbx.json")
            })
            .collect(),
        Err(e) => {
            eprintln!("\u{274c} ERROR: Could not read directory: {e}");
            return 1;
        }
    };

    if entries.is_empty() {
        println!("No .mbx.json files found in {}", dir.display());
        return 0;
    }

    let mut verified = 0u32;
    let mut failed = 0u32;
    let mut errors = 0u32;

    for path in &entries {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "\u{274c} ERROR: {}\n   Could not read file: {e}",
                    path.display()
                );
                errors += 1;
                continue;
            }
        };

        match verify_mbx_content(&content) {
            Ok(result) => {
                if result.verified {
                    print_pass(path, &result, verbose);
                    verified += 1;
                } else {
                    print_fail(path, &result);
                    failed += 1;
                }
            }
            Err(e) => {
                eprintln!("\u{274c} ERROR: {}\n   {e}", path.display());
                errors += 1;
            }
        }
    }

    println!();
    println!(
        "Results: {verified} verified, {failed} failed, {errors} errors"
    );

    if failed > 0 || errors > 0 {
        1
    } else {
        0
    }
}

// ============================================================
// Output formatting
// ============================================================

fn print_pass(path: &Path, result: &VerifyResult, verbose: bool) {
    println!("\u{2705} VERIFIED: {}", path.display());
    println!("   Hash: {}", result.computed_hash);
    if let Some(ref info) = result.info {
        println!("   Session: \"{}\"", info.session_name);
        println!("   Models: {}", info.model_ids);
        println!("   Tests: {}", info.test_count);
        println!("   Exported: {}", info.created_at);
        println!("   App Version: {}", info.app_version);

        if verbose {
            println!();
            if let Some(ref hw) = info.hardware_summary {
                println!("   --- Hardware ---");
                println!("   CPU: {}", hw.cpu);
                println!("   GPU: {}", hw.gpu);
                println!("   RAM: {} GB", hw.ram_gb);
                println!("   Fingerprint: {}", hw.fingerprint_hash);
            }
            if let Some(ref cfg) = info.config_summary {
                println!("   --- Config ---");
                println!("   Suite: {}", cfg.suite_id);
                println!("   Temperature: {}", cfg.temperature);
                println!("   Max Tokens: {}", cfg.max_tokens);
            }
            if let Some(ref stats) = info.score_stats {
                println!("   --- Scores ---");
                println!(
                    "   Min: {:.2}  Max: {:.2}  Mean: {:.2}  ({} scored)",
                    stats.min, stats.max, stats.mean, stats.count
                );
            }
            println!(
                "   Anomalies: {}",
                info.anomaly_count.unwrap_or(0)
            );
            if let Some(ref dur) = info.session_duration {
                println!("   Duration: {dur}");
            }
        }
    }
}

fn print_fail(path: &Path, result: &VerifyResult) {
    println!("\u{274c} FAILED: {}", path.display());
    println!("   Stored hash:   {}", result.stored_hash);
    println!("   Computed hash:  {}", result.computed_hash);
    println!("   Data has been modified since export.");
}

// ============================================================
// Verification engine
// ============================================================

#[derive(Debug)]
struct VerifyResult {
    verified: bool,
    stored_hash: String,
    computed_hash: String,
    info: Option<PackageInfo>,
}

#[derive(Debug)]
struct PackageInfo {
    session_name: String,
    model_ids: String,
    test_count: usize,
    created_at: String,
    app_version: String,
    hardware_summary: Option<HardwareSummary>,
    config_summary: Option<ConfigSummary>,
    score_stats: Option<ScoreStats>,
    anomaly_count: Option<usize>,
    session_duration: Option<String>,
}

#[derive(Debug)]
struct HardwareSummary {
    cpu: String,
    gpu: String,
    ram_gb: String,
    fingerprint_hash: String,
}

#[derive(Debug)]
struct ConfigSummary {
    suite_id: String,
    temperature: String,
    max_tokens: String,
}

#[derive(Debug)]
struct ScoreStats {
    min: f64,
    max: f64,
    mean: f64,
    count: usize,
}

/// Verify an MBX file's content hash. This is the core algorithm.
///
/// Implements Section 6 of the MBX v2 Specification:
/// 1. Parse as JSON
/// 2. Check format identifiers
/// 3. Save and clear content_hash
/// 4. Normalize all numbers to 6-decimal-place strings
/// 5. Serialize to compact canonical JSON (preserving key order)
/// 6. SHA-256 the UTF-8 bytes
/// 7. Compare to stored hash
fn verify_mbx_content(content: &str) -> Result<VerifyResult, String> {
    // Step 1: Parse
    let mut value: serde_json::Value =
        serde_json::from_str(content).map_err(|e| format!("Invalid JSON: {e}"))?;

    // Step 2: Check format identifiers (immutable access first)
    {
        let obj = value
            .as_object()
            .ok_or("MBX file must be a JSON object")?;

        let format = obj
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if format != "AiBenchLab-MBX" {
            return Err(format!(
                "Not an AiBenchLab MBX file (format: \"{}\")",
                format
            ));
        }

        let format_version = obj
            .get("format_version")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if format_version != "2.0" {
            return Err(format!(
                "Unsupported format version: \"{}\". This tool only supports MBX v2.0.",
                format_version
            ));
        }
    }

    // Extract info for display (immutable access)
    let info = extract_package_info(&value);

    // Step 3: Save content_hash, set to "" (mutable access)
    let obj = value
        .as_object_mut()
        .ok_or("MBX file must be a JSON object")?;

    let stored_hash = obj
        .get("content_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    obj.insert(
        "content_hash".to_string(),
        serde_json::Value::String(String::new()),
    );

    // Step 4: Normalize all numbers to 6-decimal-place strings
    normalize_floats(&mut value);

    // Step 5: Serialize to compact canonical JSON
    let canonical =
        serde_json::to_string(&value).map_err(|e| format!("Failed to serialize JSON: {e}"))?;

    // Step 6: SHA-256
    let computed_hash = format!("{:x}", Sha256::digest(canonical.as_bytes()));

    // Step 7: Compare
    Ok(VerifyResult {
        verified: computed_hash == stored_hash,
        stored_hash,
        computed_hash,
        info: Some(info),
    })
}

// ============================================================
// Deterministic float normalization
// ============================================================
// This MUST match mbx.rs normalize_floats() exactly.
// Every JSON Number is converted to a String with format!("{:.6}", f64_value).
// Strings, booleans, nulls are left unchanged.

fn normalize_floats(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                *value = serde_json::Value::String(format!("{:.6}", f));
            }
        }
        serde_json::Value::Array(arr) => arr.iter_mut().for_each(normalize_floats),
        serde_json::Value::Object(map) => map.values_mut().for_each(normalize_floats),
        _ => {}
    }
}

// ============================================================
// Info extraction (for display only — not part of verification)
// ============================================================

fn extract_package_info(value: &serde_json::Value) -> PackageInfo {
    let app_version = value
        .get("app_version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let created_at_ms = value.get("created_at").and_then(|v| v.as_u64());
    let created_at = match created_at_ms {
        Some(ms) => format_timestamp_ms(ms),
        None => "unknown".to_string(),
    };

    let session = value.get("session");

    let session_name = session
        .and_then(|s| s.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let model_ids = session
        .and_then(|s| s.get("model_ids"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "none".to_string());

    let test_results = session
        .and_then(|s| s.get("test_results"))
        .and_then(|v| v.as_array());

    let test_count = test_results.map_or(0, |arr| arr.len());

    // Hardware summary
    let hardware_summary = extract_hardware_summary(value);

    // Config summary
    let config_summary = extract_config_summary(value);

    // Score stats
    let score_stats = test_results.and_then(|r| extract_score_stats(r));

    // Anomaly count
    let anomaly_count = test_results.map(|results| {
        results
            .iter()
            .filter_map(|r| r.get("anomalies"))
            .filter_map(|a| a.as_array())
            .map(|a| a.len())
            .sum()
    });

    // Session duration
    let session_duration = extract_session_duration(session);

    PackageInfo {
        session_name,
        model_ids,
        test_count,
        created_at,
        app_version,
        hardware_summary,
        config_summary,
        score_stats,
        anomaly_count,
        session_duration,
    }
}

fn extract_hardware_summary(value: &serde_json::Value) -> Option<HardwareSummary> {
    let hw = value.get("hardware")?;
    let cpu = hw
        .get("cpu")
        .and_then(|c| c.get("brand"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let gpu = hw
        .get("gpus")
        .and_then(|g| g.as_array())
        .and_then(|arr| arr.first())
        .and_then(|g| g.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("None")
        .to_string();
    let ram_bytes = hw
        .get("memory")
        .and_then(|m| m.get("total_bytes"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let ram_gb = format!("{}", ram_bytes / (1024 * 1024 * 1024));
    let fingerprint_hash = value
        .get("hardware_fingerprint")
        .and_then(|f| f.get("combined_hash"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    Some(HardwareSummary {
        cpu,
        gpu,
        ram_gb,
        fingerprint_hash,
    })
}

fn extract_config_summary(value: &serde_json::Value) -> Option<ConfigSummary> {
    let cfg = value.get("benchmark_config")?;
    let suite_id = cfg
        .get("test_suite_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let temperature = cfg
        .get("temperature")
        .and_then(|v| v.as_f64())
        .map_or("unknown".to_string(), |t| format!("{t:.1}"));
    let max_tokens = cfg
        .get("max_tokens")
        .and_then(|v| v.as_u64())
        .map_or("unknown".to_string(), |t| t.to_string());

    Some(ConfigSummary {
        suite_id,
        temperature,
        max_tokens,
    })
}

fn extract_score_stats(results: &[serde_json::Value]) -> Option<ScoreStats> {
    let scores: Vec<f64> = results
        .iter()
        .filter_map(|r| r.get("score"))
        .filter_map(|v| v.as_f64())
        .collect();

    if scores.is_empty() {
        return None;
    }

    let min = scores.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mean = scores.iter().sum::<f64>() / scores.len() as f64;

    Some(ScoreStats {
        min,
        max,
        mean,
        count: scores.len(),
    })
}

fn extract_session_duration(session: Option<&serde_json::Value>) -> Option<String> {
    let session = session?;
    let start = session.get("start_time")?.as_str()?;
    let end = session.get("end_time")?.as_str()?;

    // Simple ISO 8601 duration calculation — parse as seconds since we
    // don't want to pull in chrono just for display
    let start_secs = parse_iso_timestamp_approx(start)?;
    let end_secs = parse_iso_timestamp_approx(end)?;
    let dur = end_secs.checked_sub(start_secs)?;

    let hours = dur / 3600;
    let minutes = (dur % 3600) / 60;
    let seconds = dur % 60;

    if hours > 0 {
        Some(format!("{hours}h {minutes}m {seconds}s"))
    } else if minutes > 0 {
        Some(format!("{minutes}m {seconds}s"))
    } else {
        Some(format!("{seconds}s"))
    }
}

/// Rough ISO 8601 timestamp → seconds since epoch.
/// Only needs to be accurate enough for duration display.
fn parse_iso_timestamp_approx(ts: &str) -> Option<u64> {
    // Expected: "2026-03-02T14:30:00Z" or "2026-03-02T14:30:00.123+00:00"
    let ts = ts.trim();
    if ts.len() < 19 {
        return None;
    }
    let year: u64 = ts[0..4].parse().ok()?;
    let month: u64 = ts[5..7].parse().ok()?;
    let day: u64 = ts[8..10].parse().ok()?;
    let hour: u64 = ts[11..13].parse().ok()?;
    let min: u64 = ts[14..16].parse().ok()?;
    let sec: u64 = ts[17..19].parse().ok()?;

    // Approximate days since epoch (good enough for duration calculation)
    let days = (year - 1970) * 365 + (year - 1969) / 4 + month_day_offset(month) + day - 1;
    Some(days * 86400 + hour * 3600 + min * 60 + sec)
}

fn month_day_offset(month: u64) -> u64 {
    match month {
        1 => 0,
        2 => 31,
        3 => 59,
        4 => 90,
        5 => 120,
        6 => 151,
        7 => 181,
        8 => 212,
        9 => 243,
        10 => 273,
        11 => 304,
        12 => 334,
        _ => 0,
    }
}

fn format_timestamp_ms(ms: u64) -> String {
    let secs = ms / 1000;
    let days_since_epoch = secs / 86400;

    // Simple date calculation
    let mut year = 1970u64;
    let mut remaining_days = days_since_epoch;

    loop {
        let days_in_year = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
            366
        } else {
            365
        };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let is_leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = [
        31,
        if is_leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];

    let mut month = 0usize;
    for (i, &dim) in days_in_month.iter().enumerate() {
        if remaining_days < dim {
            month = i;
            break;
        }
        remaining_days -= dim;
    }

    let day = remaining_days + 1;
    let time_of_day = secs % 86400;
    let hour = time_of_day / 3600;
    let minute = (time_of_day % 3600) / 60;
    let second = time_of_day % 60;

    format!(
        "{year}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        month + 1,
        day,
        hour,
        minute,
        second
    )
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Build a minimal valid MBX v2 JSON string with a correct content hash.
    /// This mirrors what mbx.rs would produce.
    fn build_test_mbx() -> String {
        let mut value = json!({
            "format": "AiBenchLab-MBX",
            "format_version": "2.0",
            "created_at": 1709395800000u64,
            "app_version": "0.7.0-beta",
            "content_hash": "",
            "session": {
                "id": "test-session",
                "name": "Test Session",
                "model_ids": ["gpt-4"],
                "model_descriptors": [],
                "test_suites": ["suite_1"],
                "start_time": "2026-01-01T00:00:00Z",
                "end_time": "2026-01-01T01:00:00Z",
                "status": "completed",
                "thermal_abort_triggered": false,
                "max_gpu_temperature": null,
                "peak_vram_used_gb": null,
                "vram_total_gb": null,
                "gpu_name": null,
                "gpu_vram_mb": null,
                "driver_version": null,
                "os_name": null,
                "backend_type": null,
                "backend_version": null,
                "model_hash": null,
                "quantization": null,
                "context_requested": null,
                "context_actual": null,
                "template_mode": null,
                "temperature_history": [],
                "test_results": [
                    {
                        "id": "result_1",
                        "provider_id": "openai",
                        "model_name": "gpt-4",
                        "test_id": "test_1",
                        "session_id": "test-session",
                        "output": "Hello world",
                        "execution_time_ms": 1500,
                        "tokens_generated": 100,
                        "score": 85.5,
                        "test_category": "reasoning",
                        "test_intent": null,
                        "test_prompt": null,
                        "required_capabilities": [],
                        "disallowed_capabilities": [],
                        "scoring_criteria": {},
                        "programming_language": null,
                        "ttft_ms": null,
                        "tpot_ms": null,
                        "e2e_latency_ms": null,
                        "tps": null,
                        "anomalies": [],
                        "plugin_id": null
                    }
                ],
                "context_summary": null,
                "total_tests": null,
                "completed_tests": null,
                "signature": null,
                "public_key": null,
                "hardware_fingerprint": null,
                "is_signed": false,
                "tamper_detected": false,
                "anomaly_summary": {
                    "total_anomaly_count": 0,
                    "tests_with_anomalies": 0
                },
                "plugin_ids": [],
                "selected_domains": [],
                "selected_languages": [],
                "plugin_versions": {},
                "scoring_engine_version": null,
                "suite_version": null,
                "baseline_session_id": null,
                "comparison_delta_score": null,
                "comparison_delta_latency": null,
                "model_run_statuses": [],
                "prompt_template_mode": null
            },
            "hardware": {
                "cpu": {
                    "brand": "Ryzen 9 7950X",
                    "physical_cores": 16,
                    "logical_cores": 32,
                    "frequency_mhz": 4500,
                    "architecture": "Zen 4"
                },
                "gpus": [{
                    "vendor": "NVIDIA",
                    "name": "RTX 4090",
                    "memory_bytes": 25769803776u64
                }],
                "memory": {
                    "total_bytes": 68719476736u64,
                    "available_bytes": 34359738368u64,
                    "used_bytes": 34359738368u64
                },
                "os": {
                    "name": "Windows",
                    "version": "11",
                    "arch": "x86_64"
                }
            },
            "hardware_fingerprint": {
                "cpu_brand_hash": "abc123",
                "cpu_physical_cores": 16,
                "cpu_logical_cores": 32,
                "gpu_name_hash": "def456",
                "gpu_memory_gb": 24,
                "ram_total_gb": 64,
                "os_hash": "ghi789",
                "combined_hash": "combined_abc123"
            },
            "benchmark_config": {
                "test_suite_id": "suite_1",
                "model_ids": ["gpt-4"],
                "temperature": 0.7,
                "max_tokens": 2048
            }
        });

        // Compute the content hash the same way mbx.rs does
        let hash = compute_hash_for_value(&mut value);
        value
            .as_object_mut()
            .unwrap()
            .insert("content_hash".to_string(), json!(hash));

        serde_json::to_string_pretty(&value).unwrap()
    }

    /// Compute the content hash for a serde_json::Value (same algorithm as mbx.rs).
    fn compute_hash_for_value(value: &mut serde_json::Value) -> String {
        value
            .as_object_mut()
            .unwrap()
            .insert("content_hash".to_string(), json!(""));

        let mut clone = value.clone();
        normalize_floats(&mut clone);
        let canonical = serde_json::to_string(&clone).unwrap();
        format!("{:x}", Sha256::digest(canonical.as_bytes()))
    }

    #[test]
    fn test_valid_mbx_passes_verification() {
        let content = build_test_mbx();
        let result = verify_mbx_content(&content).unwrap();
        assert!(
            result.verified,
            "Expected PASS but got FAIL.\n  Stored:   {}\n  Computed: {}",
            result.stored_hash, result.computed_hash
        );
    }

    #[test]
    fn test_tampered_score_fails_verification() {
        let content = build_test_mbx();
        // Tamper: change a score value
        let tampered = content.replace("85.5", "99.9");
        let result = verify_mbx_content(&tampered).unwrap();
        assert!(
            !result.verified,
            "Expected FAIL for tampered score but got PASS"
        );
    }

    #[test]
    fn test_tampered_model_name_fails_verification() {
        let content = build_test_mbx();
        let tampered = content.replace("\"gpt-4\"", "\"gpt-5\"");
        let result = verify_mbx_content(&tampered).unwrap();
        assert!(
            !result.verified,
            "Expected FAIL for tampered model name but got PASS"
        );
    }

    #[test]
    fn test_tampered_app_version_fails_verification() {
        let content = build_test_mbx();
        let tampered = content.replace("0.7.0-beta", "0.7.0-HACKED");
        let result = verify_mbx_content(&tampered).unwrap();
        assert!(
            !result.verified,
            "Expected FAIL for tampered app version but got PASS"
        );
    }

    #[test]
    fn test_malformed_json_returns_error() {
        let result = verify_mbx_content("this is not json {{{");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid JSON"));
    }

    #[test]
    fn test_non_object_json_returns_error() {
        let result = verify_mbx_content("[1, 2, 3]");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("JSON object"));
    }

    #[test]
    fn test_wrong_format_identifier_returns_error() {
        let content = build_test_mbx();
        let tampered = content.replace("AiBenchLab-MBX", "SomethingElse");
        let result = verify_mbx_content(&tampered);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Not an AiBenchLab MBX file"));
    }

    #[test]
    fn test_v1_format_version_returns_error() {
        let mut value: serde_json::Value = serde_json::from_str(&build_test_mbx()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("format_version".to_string(), json!("1.0"));
        let content = serde_json::to_string_pretty(&value).unwrap();

        let result = verify_mbx_content(&content);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("Unsupported format version") && err.contains("1.0"),
            "Expected clear v1 error, got: {err}"
        );
    }

    #[test]
    fn test_normalize_floats_matches_mbx_rs() {
        // This test verifies our normalize_floats produces identical output
        // to the mbx.rs implementation for the same input tree.
        let mut value = json!({
            "score": 0.123456789,
            "nested": {
                "arr": [1.1, 2.2, 3.3],
                "int": 42,
                "str": "hello"
            }
        });

        normalize_floats(&mut value);

        assert_eq!(value["score"], json!("0.123457"));
        assert_eq!(value["nested"]["arr"][0], json!("1.100000"));
        assert_eq!(value["nested"]["arr"][1], json!("2.200000"));
        assert_eq!(value["nested"]["arr"][2], json!("3.300000"));
        assert_eq!(value["nested"]["int"], json!("42.000000"));
        assert_eq!(value["nested"]["str"], json!("hello"));
    }

    #[test]
    fn test_hash_output_is_64_char_lowercase_hex() {
        let content = build_test_mbx();
        let result = verify_mbx_content(&content).unwrap();
        assert_eq!(result.computed_hash.len(), 64);
        assert!(
            result.computed_hash.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "Hash must be lowercase hex: {}",
            result.computed_hash
        );
    }

    #[test]
    fn test_empty_content_hash_fails_verification() {
        let mut value: serde_json::Value = serde_json::from_str(&build_test_mbx()).unwrap();
        // Set content_hash to empty
        value
            .as_object_mut()
            .unwrap()
            .insert("content_hash".to_string(), json!(""));
        let content = serde_json::to_string_pretty(&value).unwrap();

        let result = verify_mbx_content(&content).unwrap();
        assert!(
            !result.verified,
            "Empty content_hash should fail verification"
        );
    }

    #[test]
    fn test_batch_directory_counting() {
        // Create a temp directory with test files
        let dir = std::env::temp_dir().join("aibenchlab_verify_test_batch");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Write 2 valid files
        let valid = build_test_mbx();
        std::fs::write(dir.join("good1.mbx.json"), &valid).unwrap();
        std::fs::write(dir.join("good2.mbx.json"), &valid).unwrap();

        // Write 1 tampered file
        let tampered = valid.replace("85.5", "99.9");
        std::fs::write(dir.join("bad.mbx.json"), &tampered).unwrap();

        // Write a non-mbx json file (should be ignored)
        std::fs::write(dir.join("other.json"), "{}").unwrap();

        // Run batch
        let code = run_batch(&dir, false);
        assert_eq!(code, 1, "Batch should return 1 when any file fails");

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_format_timestamp_ms() {
        // 2026-01-01T00:00:00Z in milliseconds
        // This is approximate — just verify it produces reasonable output
        let ts = format_timestamp_ms(1767225600000);
        assert!(ts.starts_with("2026-"), "Expected 2026, got: {ts}");
        assert!(ts.ends_with('Z'));
    }

    #[test]
    fn test_info_extraction() {
        let content = build_test_mbx();
        let value: serde_json::Value = serde_json::from_str(&content).unwrap();
        let info = extract_package_info(&value);
        assert_eq!(info.session_name, "Test Session");
        assert_eq!(info.model_ids, "gpt-4");
        assert_eq!(info.test_count, 1);
        assert_eq!(info.app_version, "0.7.0-beta");
        assert!(info.hardware_summary.is_some());
        assert!(info.config_summary.is_some());
        assert!(info.score_stats.is_some());
        let stats = info.score_stats.unwrap();
        assert!((stats.min - 85.5).abs() < 0.01);
        assert!((stats.max - 85.5).abs() < 0.01);
        assert_eq!(stats.count, 1);
    }
}

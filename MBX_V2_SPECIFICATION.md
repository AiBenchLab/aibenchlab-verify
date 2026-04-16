# AiBenchLab MBX v2 Format Specification

**Format Identifier:** `AiBenchLab-MBX`
**Format Version:** `2.0`
**Status:** Authoritative
**Last Updated:** March 2026

---

## 1. Overview

MBX is AiBenchLab's portable benchmark export format. An MBX file contains a complete benchmark session — every test result, the hardware profile of the machine that ran it, a configuration snapshot, and an executive summary — bundled into a single JSON file with a SHA-256 content integrity hash.

The format exists to solve one problem: a consultant or procurement team runs an AI model benchmark on machine A and needs to deliver the results to a decision-maker on machine B, with reasonable assurance that the data was not altered in transit. MBX provides tamper-evidence through content hashing, not cryptographic proof of origin. See [Section 7: Scope & Limitations](#7-scope--limitations) for what this does and does not guarantee.

MBX files use the `.mbx.json` extension and are valid JSON. They can be opened in any text editor, parsed by any JSON library, and verified by any implementation of SHA-256.

---

## 2. Format Structure

An MBX v2 file is a single JSON object. Field order in serialization follows the struct definition below; the `format` and `format_version` fields appear first so the file is identifiable at a glance.

### 2.1 Top-Level Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `format` | string | Yes | Always `"AiBenchLab-MBX"`. Used to identify the file type without relying on the file extension. |
| `format_version` | string | Yes | `"2.0"` for this specification. |
| `created_at` | integer (u64) | Yes | UTC timestamp in milliseconds when the package was created. |
| `app_version` | string | Yes | Semantic version of the AiBenchLab application that produced this file (e.g., `"0.7.0-beta"`). |
| `content_hash` | string | Yes | Lowercase hexadecimal SHA-256 digest (64 characters). See [Section 4](#4-content-hash-methodology). |
| `session` | object | Yes | The complete benchmark session. See [Section 2.2](#22-session-object). |
| `hardware` | object | Yes | Hardware profile of the machine at export time. See [Section 2.3](#23-hardware-profile). |
| `hardware_fingerprint` | object | Yes | Privacy-safe hardware fingerprint. See [Section 5](#5-hardware-fingerprint). |
| `benchmark_config` | object | Yes | Configuration snapshot. See [Section 2.4](#24-benchmark-configuration). |
| `summary_metadata` | object or null | No | Executive summary metrics. Omitted from JSON when null. See [Section 2.5](#25-summary-metadata). |

### 2.2 Session Object

The `session` field contains the entire `BenchmarkSession` as recorded by AiBenchLab. This is the canonical data — everything else in the package is derived from or supplementary to it.

Key fields within the session:

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique session identifier (UUID). |
| `name` | string | Human-readable session name. |
| `model_ids` | string[] | Model identifiers benchmarked in this session. |
| `model_descriptors` | object[] | Detailed model metadata (architecture, parameter count, provider). |
| `test_suites` | string[] | Suite identifiers that were executed. |
| `start_time` | string | ISO 8601 timestamp when the benchmark began. |
| `end_time` | string or null | ISO 8601 timestamp when the benchmark finished. |
| `status` | string | Session status: `"running"`, `"completed"`, or `"aborted"`. |
| `test_results` | object[] | Array of individual test results. See [Section 2.2.1](#221-test-result-object). |
| `temperature_history` | object[] | GPU temperature readings taken during the benchmark run. |
| `thermal_abort_triggered` | boolean | Whether the session was stopped due to thermal limits. |
| `max_gpu_temperature` | number or null | Peak GPU temperature observed (Celsius). |
| `peak_vram_used_gb` | number or null | Peak VRAM usage observed (GB). |
| `vram_total_gb` | number or null | Total VRAM available (GB). |
| `anomaly_summary` | object | Aggregate anomaly statistics for the session. |
| `selected_domains` | string[] | Benchmark domains selected for this run (e.g., `"reasoning"`, `"coding"`). |
| `scoring_engine_version` | string or null | Version of the scoring engine used. |
| `suite_version` | string or null | Version of the test suite definition. |

Additional session fields capture execution environment (`gpu_name`, `driver_version`, `backend_type`, `backend_version`, `model_hash`, `quantization`, `context_requested`, `context_actual`, `template_mode`), plugin tracking (`plugin_ids`, `plugin_versions`), comparison data (`baseline_session_id`, `comparison_delta_score`, `comparison_delta_latency`), and per-model run status (`model_run_statuses`).

#### 2.2.1 Test Result Object

Each entry in `session.test_results` contains:

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique result identifier. |
| `provider_id` | string | Provider that served the model (e.g., `"ollama"`, `"openai"`, `"anthropic"`). |
| `model_name` | string | Model name as reported by the provider. |
| `test_id` | string | Identifier of the test case that was executed. |
| `session_id` | string | Back-reference to the parent session. |
| `output` | string | Raw model output text. |
| `execution_time_ms` | integer | Wall-clock execution time in milliseconds. |
| `tokens_generated` | integer | Number of output tokens produced. |
| `score` | number or null | Normalized score assigned by the scoring engine (0-100 scale). Null if scoring was not applicable. |
| `test_category` | string or null | Domain category (e.g., `"reasoning"`, `"coding"`, `"adversarial_safety"`). |
| `test_intent` | string or null | Description of what the test evaluates. |
| `test_prompt` | string or null | The prompt sent to the model. |
| `required_capabilities` | string[] | Capabilities the model must have to attempt this test. |
| `disallowed_capabilities` | string[] | Capabilities that should not be present. |
| `scoring_criteria` | object | Key-value pairs describing scoring dimensions. |
| `programming_language` | string or null | For code tests, the target language. |
| `ttft_ms` | integer or null | Time to First Token (milliseconds). Provider-dependent. |
| `tpot_ms` | number or null | Time Per Output Token (milliseconds). Provider-dependent. |
| `e2e_latency_ms` | integer or null | End-to-end latency (milliseconds). |
| `tps` | number or null | Tokens per second throughput. |
| `anomalies` | object[] | Anomalies detected for this result (empty response, latency spikes, etc.). |
| `plugin_id` | string or null | Plugin that provided this test, if any. |

### 2.3 Hardware Profile

The `hardware` field captures the raw hardware specifications of the machine at export time.

| Field | Type | Description |
|-------|------|-------------|
| `cpu.brand` | string | CPU model name (e.g., `"AMD Ryzen 9 7950X"`). |
| `cpu.physical_cores` | integer | Physical core count. |
| `cpu.logical_cores` | integer | Logical core count (includes hyperthreading). |
| `cpu.frequency_mhz` | integer | Base clock frequency in MHz. |
| `cpu.architecture` | string | CPU architecture (e.g., `"Zen 4"`, `"x86_64"`). |
| `gpus` | object[] | Array of GPU descriptors. |
| `gpus[].vendor` | string | GPU vendor (e.g., `"NVIDIA"`, `"AMD"`). |
| `gpus[].name` | string | GPU model name (e.g., `"RTX 4090"`). |
| `gpus[].memory_bytes` | integer | GPU memory in bytes. |
| `memory.total_bytes` | integer | Total system RAM in bytes. |
| `memory.available_bytes` | integer | Available system RAM in bytes at time of scan. |
| `memory.used_bytes` | integer | Used system RAM in bytes at time of scan. |
| `os.name` | string | Operating system name. |
| `os.version` | string | Operating system version. |
| `os.arch` | string | System architecture (e.g., `"x86_64"`, `"aarch64"`). |

### 2.4 Benchmark Configuration

The `benchmark_config` field is a quick-reference snapshot of the run parameters. This data also exists within the session object but is duplicated here for convenience — a validator or dashboard can read the configuration without parsing the full session.

| Field | Type | Description |
|-------|------|-------------|
| `test_suite_id` | string | Primary test suite that was executed. |
| `model_ids` | string[] | Models included in the benchmark. |
| `temperature` | number | Sampling temperature used (e.g., `0.7`). |
| `max_tokens` | integer | Maximum token limit for model responses. |

### 2.5 Summary Metadata

When present, `summary_metadata` contains high-level assessments generated by AiBenchLab's executive summary engine. All fields are optional and may be null.

| Field | Type | Description |
|-------|------|-------------|
| `agentic_readiness_score` | number or null | Composite score indicating model readiness for agentic workflows (0-100). |
| `agentic_readiness_label` | string or null | Human-readable label (e.g., `"Production Ready"`, `"Experimental"`). |
| `responsiveness_class` | string or null | Latency classification (e.g., `"Real-time"`, `"Interactive"`, `"Batch"`). |
| `consistency_rating` | string or null | Assessment of score consistency across test runs. |
| `hardware_efficiency_grade` | string or null | How efficiently the model uses the available hardware. |
| `the_one_thing` | string or null | Single most important takeaway from the benchmark. |
| `overall_assessment` | string or null | Narrative overall assessment. |

---

## 3. File Identification

An MBX v2 file can be identified without parsing the full document:

1. **File extension:** `.mbx.json`
2. **Magic fields:** The first two JSON keys are `"format"` and `"format_version"`. A quick check:
   - `format` equals `"AiBenchLab-MBX"`
   - `format_version` equals `"2.0"`
3. **Content type:** The file is valid UTF-8 JSON.

A minimal identification check in pseudocode:

```
data = parse_json(file_contents)
is_mbx_v2 = (data["format"] == "AiBenchLab-MBX") AND (data["format_version"] == "2.0")
```

---

## 4. Content Hash Methodology

The `content_hash` field contains a SHA-256 digest computed over a deterministic canonical representation of the package. The process is designed so that any implementation — in any language, on any platform — can independently reproduce the hash from the same data.

### 4.1 Algorithm

Given an MBX package as a JSON object:

**Step 1: Clear the hash field.**
Set `content_hash` to an empty string (`""`). This field is excluded from the hash computation by being present but empty.

**Step 2: Serialize to an intermediate JSON value tree.**
Parse the package into a mutable in-memory JSON value tree (object, array, number, string, boolean, null nodes). This is the tree that will be canonicalized.

**Step 3: Normalize all floating-point numbers.**
Walk every node in the JSON value tree recursively. For each numeric node:
- Interpret the number as a 64-bit IEEE 754 float (`f64`).
- Replace the numeric node with a **string** node containing the number formatted to exactly 6 decimal places using standard `printf`-style formatting: `sprintf("%.6f", value)`.
- Integer values that are stored as JSON numbers are also converted (e.g., `42` becomes `"42.000000"`).
- String, boolean, null, and already-string nodes are left unchanged.

This normalization eliminates cross-platform differences in float serialization (e.g., `0.1 + 0.2` producing `0.30000000000000004` on one platform vs. `0.3` on another).

**Step 4: Serialize to canonical JSON.**
Serialize the normalized tree to a compact (non-pretty-printed) JSON string with no extra whitespace. Key order within objects is determined by serde's struct serialization order, which is the field declaration order of the Rust structs. Since the same struct definitions produce the same key order regardless of platform, this serialization is deterministic.

**Step 5: Compute SHA-256.**
Hash the UTF-8 byte representation of the canonical JSON string using SHA-256. Encode the resulting 32-byte digest as a lowercase hexadecimal string (64 characters).

**Step 6: Store the hash.**
Set `content_hash` to the computed hex string.

### 4.2 Reference Implementation (Rust)

```rust
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

fn compute_content_hash(package: &MBXPackage) -> String {
    // Step 1: Clone and clear content_hash
    let mut clone = package.clone();
    clone.content_hash = String::new();

    // Step 2: Serialize to Value tree
    let mut value = serde_json::to_value(&clone).unwrap();

    // Step 3: Normalize floats
    normalize_floats(&mut value);

    // Step 4: Canonical JSON (compact, no whitespace)
    let canonical = serde_json::to_string(&value).unwrap();

    // Step 5: SHA-256
    format!("{:x}", sha2::Sha256::digest(canonical.as_bytes()))
}
```

### 4.3 Implementation in Other Languages

To reproduce the hash in Python, JavaScript, or any other language:

1. Parse the MBX JSON file.
2. Set `content_hash` to `""`.
3. Walk the entire JSON tree. Replace every number with a string: `f"{value:.6f}"` (Python) or `value.toFixed(6)` (JavaScript). This includes integers — `42` becomes `"42.000000"`.
4. Serialize to compact JSON. **Key order must match the original file's key order.** Use an ordered dictionary or a JSON library that preserves insertion order.
5. SHA-256 hash the resulting UTF-8 string.
6. Compare the lowercase hex digest to the stored `content_hash`.

If your JSON parser does not preserve key order, it cannot be used for MBX verification.

### 4.4 Key Ordering

The canonical JSON key order is the Rust struct field declaration order, which is the order shown in [Section 2](#2-format-structure) of this document. When re-serializing for hash verification, preserve the key order from the original file. If your JSON parser preserves insertion order (Python `json.loads` does by default since Python 3.7; JavaScript `JSON.parse` does in practice), no reordering is needed.

**Canonicalization Constraints:**

- Do not sort keys alphabetically. Preserve struct declaration order.
- Do not pretty-print. No newlines, no indentation, no extra whitespace.
- Do not re-encode Unicode escapes differently than the original serialization.
- Do not normalize line endings within string values.

Any reordering of Rust struct fields in future versions constitutes a breaking change to the hash computation and requires a `format_version` increment.

---

## 5. Hardware Fingerprint

The `hardware_fingerprint` object provides a privacy-safe, deterministic identifier for the machine that produced the benchmark. It allows two MBX files to be correlated as "from the same machine" without exposing serial numbers, MAC addresses, or other personally identifiable hardware identifiers.

### 5.1 Fingerprint Fields

| Field | Type | Description |
|-------|------|-------------|
| `cpu_brand_hash` | string | SHA-256 of the CPU brand string. |
| `cpu_physical_cores` | integer | Physical core count (not hashed — useful for hardware class comparison). |
| `cpu_logical_cores` | integer | Logical core count. |
| `gpu_name_hash` | string | SHA-256 of the primary GPU name. `SHA-256("no_gpu")` if no GPU is present. |
| `gpu_memory_gb` | integer | Primary GPU memory in whole gigabytes. |
| `ram_total_gb` | integer | Total system RAM in whole gigabytes. |
| `os_hash` | string | SHA-256 of `"{os_name} {os_version} {os_arch}"`. |
| `combined_hash` | string | SHA-256 of the concatenation of all above fields. Serves as the overall machine fingerprint. |

### 5.2 Privacy Guarantees

- CPU and GPU model names are stored only as SHA-256 hashes. The original strings cannot be recovered from the hash.
- No serial numbers, MAC addresses, hardware IDs, or user account names are included.
- Numeric fields (core counts, memory sizes) are rounded to whole units and are not uniquely identifying.
- The `combined_hash` is a one-way function of the individual fields. It identifies a hardware configuration class, not a specific physical machine.
- Before inclusion in the package, the fingerprint is checked against a blocklist of patterns that indicate PII leakage (serial number formats, MAC address patterns). The package will not be created if this check fails.

### 5.3 Determinism

The same hardware configuration produces the same fingerprint at time of export. OS updates, driver version changes, or memory upgrades will alter the fingerprint, as these represent a different configuration state. The fingerprint identifies a hardware configuration class at the time of export, not an immutable machine identity. Determinism is guaranteed by:
- Using SHA-256 (deterministic) on fixed input strings.
- Rounding memory values to whole gigabytes before hashing.
- Using the same OS version string format across platforms.

---

## 6. Verification Procedure

This procedure allows anyone to verify the integrity of an MBX file using only standard tools (a JSON parser and a SHA-256 implementation). No access to AiBenchLab is required.

### 6.1 Step-by-Step Verification

**Step 1: Parse the file.**
Read the `.mbx.json` file and parse it as JSON. If parsing fails, the file is corrupt or not a valid MBX file.

**Step 2: Check the format identifier.**
Verify that `format` equals `"AiBenchLab-MBX"` and `format_version` equals `"2.0"`. If either check fails, the file is not a valid MBX v2 package.

**Step 3: Check the application version.**
Verify that `app_version` is a recognized AiBenchLab version (currently: any version `0.1.0` or later, or `1.0.0` or later). An unrecognized version does not necessarily mean the data is invalid, but it may indicate the file was produced by a modified build.

**Step 4: Save and clear the content hash.**
Read the value of `content_hash` and save it. Set `content_hash` to `""` (empty string) in your in-memory representation.

**Step 5: Normalize floats.**
Walk the entire JSON tree. For every value that is a JSON number, replace it with a JSON string containing the number formatted to exactly 6 decimal places. See [Section 4.3](#43-implementation-in-other-languages) for language-specific guidance.

**Step 6: Serialize to canonical JSON.**
Serialize the modified tree to a compact JSON string (no pretty-printing, no extra whitespace). Preserve the original key order.

**Step 7: Compute SHA-256.**
Hash the UTF-8 bytes of the canonical JSON string using SHA-256. Encode as lowercase hexadecimal (64 characters).

**Step 8: Compare.**
If the computed hash equals the saved `content_hash`, the file has not been modified since the hash was computed. If they differ, the data has been altered.

### 6.2 Example Verification Script (Python)

```python
import json
import hashlib

def normalize_floats(obj):
    if isinstance(obj, dict):
        return {k: normalize_floats(v) for k, v in obj.items()}
    elif isinstance(obj, list):
        return [normalize_floats(v) for v in obj]
    elif isinstance(obj, (int, float)):
        return f"{float(obj):.6f}"
    else:
        return obj

def verify_mbx(file_path):
    with open(file_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    # Check format
    assert data["format"] == "AiBenchLab-MBX", "Not an MBX file"
    assert data["format_version"] == "2.0", "Not MBX v2"

    # Save and clear hash
    stored_hash = data["content_hash"]
    data["content_hash"] = ""

    # Normalize and hash
    normalized = normalize_floats(data)
    canonical = json.dumps(normalized, separators=(",", ":"), ensure_ascii=False)
    computed_hash = hashlib.sha256(canonical.encode("utf-8")).hexdigest()

    if computed_hash == stored_hash:
        print("PASS: Content hash verified.")
    else:
        print("FAIL: Content hash mismatch — data may have been tampered with.")
        print(f"  Stored:   {stored_hash}")
        print(f"  Computed: {computed_hash}")

verify_mbx("benchmark.mbx.json")
```

> **Note on `separators`:** The Python `json.dumps` default inserts spaces after `:` and `,`. Use `separators=(",", ":")` to produce compact JSON matching the canonical form.

### 6.3 AiBenchLab Built-In Validation

AiBenchLab's `validate_mbx_file` command performs the above verification automatically and additionally checks for:

- **Suspicious patterns:** All test results having identical execution times (statistically implausible), or all scores being within 0.01 of each other across multiple tests. These are flagged as warnings, not errors.
- **Version compatibility:** The `app_version` must be from a recognized release series.

---

## 7. Scope & Limitations

### 7.1 What MBX Content Hashing Provides

- **Tamper evidence.** If any field in the package — a score, a timestamp, a model name, a latency measurement — is modified after the hash is computed, verification will fail. This covers accidental corruption and deliberate editing.
- **Self-contained verification.** The hash can be verified using only the file itself and a SHA-256 implementation. No network access, no AiBenchLab installation, no external keys.
- **Cross-platform determinism.** The float normalization step ensures the same data produces the same hash on Windows, macOS, and Linux, in Rust, Python, JavaScript, or any other language.

### 7.2 What MBX Content Hashing Does NOT Provide

- **Proof of origin.** The hash does not prove the file was produced by AiBenchLab. Anyone with this specification can construct a valid MBX file with a correct hash. There is no cryptographic signature binding the file to a specific installation or license.
- **Proof of execution.** The hash does not prove the benchmark was actually run. It only proves the file has not been modified since the hash was computed.
- **Protection against a malicious exporter.** If the party producing the MBX file is adversarial, they can fabricate results and compute a valid hash over the fabricated data. The hash protects against modification *after* export, not fabrication *during* export.
- **Non-repudiation.** There is no private key involved. The exporter cannot be held accountable for the file's contents through cryptographic means.

### 7.3 Appropriate Use Cases

MBX content hashing is appropriate for:
- Delivering benchmark results to a client, where both parties trust the benchmarking process but want assurance the file was not corrupted or edited in transit.
- Archiving benchmark results with a tamper-evidence seal.
- Comparing two MBX files to confirm they contain identical data (same hash = same data).

MBX content hashing is **not** a substitute for:
- Cryptographic code signing or digital signatures.
- Chain-of-custody proof in adversarial settings.
- Third-party attestation of benchmark results.

---

## 8. Version History

| Version | Date | Notes |
|---------|------|-------|
| **2.0** | March 2026 | Current specification. Complete redesign. Replaces v1 with no backward compatibility. Key changes: full `BenchmarkSession` instead of split raw/normalized results; SHA-256 content hash replaces cryptographic signing; magic format identifiers added; `PackageManifest` and `BenchmarkSignature` removed; `summary_metadata` added. |
| **1.0** | 2025 | Deprecated. Used `PackageManifest`, `BenchmarkSignature` (key-pair signing), and split `raw_results`/`normalized_results` arrays. No longer produced or consumed by AiBenchLab. v1 files cannot be validated by current versions of the application. |

Future format changes must increment `format_version`. All MBX versions must remain self-describing through the `format` and `format_version` fields appearing as the first two keys in the JSON object.

---

*This document describes the MBX v2 format as implemented in AiBenchLab 0.7.0 and later.*

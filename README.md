# aibenchlab-verify

Standalone command-line tool for verifying the integrity of AiBenchLab MBX v2 files.

This tool recomputes the SHA-256 content hash of an MBX file and compares it to the stored value. If the hashes match, the file has not been modified since export. If they differ, the data has been altered.

**No AiBenchLab installation required.** This is an independent verifier that anyone can build and run.

## How it works

This tool implements Section 6 (Verification Procedure) of the [MBX v2 Specification](../docs_manual/MBX_V2_SPECIFICATION.md):

1. Parse the `.mbx.json` file as JSON
2. Verify the format identifier (`AiBenchLab-MBX`) and version (`2.0`)
3. Save the stored `content_hash`, then set it to `""`
4. Normalize all JSON numbers to 6-decimal-place strings (e.g., `42` becomes `"42.000000"`)
5. Serialize to compact canonical JSON (no pretty-printing, preserving key order)
6. SHA-256 hash the UTF-8 bytes
7. Compare the computed hash to the stored hash

The normalization and hashing logic is identical to the implementation in AiBenchLab's `mbx.rs`.

## Build

Requires Rust 1.70 or later.

```bash
cd aibenchlab-verify
cargo build --release
```

The binary is at `target/release/aibenchlab-verify` (or `aibenchlab-verify.exe` on Windows).

### Cross-compilation

```bash
# Linux (from any platform with the target installed)
cargo build --release --target x86_64-unknown-linux-gnu

# macOS
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin

# Windows
cargo build --release --target x86_64-pc-windows-msvc
```

## Usage

### Verify a single file

```bash
aibenchlab-verify benchmark.mbx.json
```

Output on success:

```
VERIFIED: benchmark.mbx.json
   Hash: a1b2c3d4e5f6...
   Session: "My Benchmark Run"
   Models: llama3.1:8b, mistral:7b
   Tests: 142
   Exported: 2026-03-02T14:30:00Z
   App Version: 0.7.0-beta
```

Output on failure:

```
FAILED: benchmark.mbx.json
   Stored hash:   a1b2c3d4e5f6...
   Computed hash:  x9y8z7w6v5u4...
   Data has been modified since export.
```

### Verbose mode

```bash
aibenchlab-verify --verbose benchmark.mbx.json
```

Shows everything from default mode plus hardware details, benchmark configuration, score distribution, anomaly count, and session duration.

### Batch verification

```bash
aibenchlab-verify --batch ./exports/
```

Verifies all `.mbx.json` files in the directory and prints a summary:

```
Results: 12 verified, 1 failed, 0 errors
```

### Help

```bash
aibenchlab-verify --help
aibenchlab-verify --version
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | All files verified successfully |
| 1 | One or more files failed verification, or an error occurred |

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | Command-line argument parsing |
| `serde_json` | JSON parsing and serialization |
| `sha2` | SHA-256 hashing |

No network access. No telemetry. No AiBenchLab installation check.

## Running tests

```bash
cargo test
```

Tests cover: valid file verification, tamper detection (score, model name, app version), malformed JSON handling, v1 format rejection, batch counting, float normalization correctness, and hash format validation.

## License

MIT

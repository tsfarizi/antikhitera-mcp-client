# 📦 Antikythera MCP - Build Guide

## 🎯 Build Modes

This project supports **2 build modes**:

### 1. CLI Mode (Native/Debug)
For debugging and native builds on your target platform.

```bash
# Run in development mode
cargo run

# Build for current platform (debug)
cargo build

# Build for current platform (release/production)
cargo build --release
```

**Output:** `target/debug/antikythera` or `target/release/antikythera`

### 2. WASM Mode (WebAssembly) - SDK Only
For web/browser deployments. **Only the SDK crate supports WASM.**

```bash
# Add WASM target (one-time)
rustup target add wasm32-unknown-unknown

# Build SDK for WASM (default: single-agent, minimal size)
cargo build -p antikythera-sdk --target wasm32-unknown-unknown --release

# Or use wasm-pack (recommended for web deployment)
wasm-pack build -p antikythera-sdk --release
```

**Output:** `target/wasm32-unknown-unknown/release/antikythera_sdk.wasm`

---

## 🔧 WASM Build Configuration

The SDK supports **configurable features** to minimize WASM binary size:

### Feature Flags

| Feature | Description | Binary Size Impact |
|:--------|:------------|:------------------|
| `wasm` (default) | WASM bindings | Base |
| `single-agent` (default) | Single agent support | +0 KB |
| `multi-agent` | Multi-agent orchestration | +200 KB |
| `cloud` | GCP integrations | +150 KB |
| `wasm-sandbox` | WASM tool sandboxing | +300 KB |
| `ffi` | C FFI bindings | +50 KB |
| `full` | All features | +700 KB |

### Build Examples

#### 1. Minimal WASM (Single Agent Only) - Recommended
```bash
# Default features = wasm + single-agent
cargo build -p antikythera-sdk --target wasm32-unknown-unknown --release
# Binary size: ~500 KB
```

#### 2. Multi-Agent WASM
```bash
cargo build -p antikythera-sdk --target wasm32-unknown-unknown --release \
  --no-default-features --features wasm,multi-agent
# Binary size: ~700 KB
```

#### 3. WASM with Cloud Support
```bash
cargo build -p antikythera-sdk --target wasm32-unknown-unknown --release \
  --no-default-features --features wasm,single-agent,cloud
# Binary size: ~650 KB
```

#### 4. Full-Featured WASM (Not Recommended)
```bash
cargo build -p antikythera-sdk --target wasm32-unknown-unknown --release \
  --features full
# Binary size: ~1.2 MB
```

#### 5. FFI Build (Native Library)
```bash
cargo build -p antikythera-sdk --release --features ffi
# Output: target/release/libantikythera_sdk.so/.dll/.dylib
```

---

## 📋 Available Binaries

Only **1 binary** is available:

| Binary | Command | Description |
|:-------|:--------|:------------|
| `antikythera` | `cargo run --bin antikythera` | CLI interface with TUI |

---

## 🔧 Feature Flags

Build with different feature sets:

```bash
# Default features (native-transport only)
cargo build

# Full-featured build (all capabilities)
cargo build --features full

# Minimal build (no cloud, no TUI)
cargo build -p antikythera-core --no-default-features

# WASM runtime support
cargo build --features wasm-runtime

# Multi-agent support
cargo build --features multi-agent
```

### Available Features

| Feature | Description | Dependencies |
|:--------|:------------|:-------------|
| `native-transport` (default) | Stdio/OS process management | `tokio/process`, `sysinfo` |
| `gcp` | Google Cloud integration | `reqwest`, `reqwest-eventsource` |
| `wasm-runtime` | Sandboxed WASM execution | `wasmtime`, `wasm-bindgen` |
| `wizard` | Interactive setup wizard | `crossterm`, `ratatui`, `clap` |
| `multi-agent` | Multi-agent orchestration | `redis`, `google-cloud-storage` |
| `full` | All features combined | All above |

---

## 🏗️ Build Artifacts

### Native Build
```
target/
├── debug/
│   └── antikythera(.exe)       # Debug binary
└── release/
    └── antikythera(.exe)       # Optimized binary
```

### WASM Build
```
target/wasm32-unknown-unknown/release/
└── antikythera_sdk.wasm         # WASM module
```

---

## 🚀 Usage Examples

### CLI Mode
```bash
# Run with default configuration
cargo run

# Run with custom config file
cargo run -- --config /path/to/client.toml

# Run with system prompt
cargo run -- --system "You are a helpful assistant"
```

### WASM Mode (JavaScript)
```javascript
import init, { Client } from './pkg/antikythera_sdk.js';

await init();
const client = await Client.new(config);
const response = await client.chat("Hello");
```

---

## ⚠️ Notes

1. **WASM Limitations**: The CLI binary cannot be built for WASM due to `tokio` dependencies. Only the SDK crate supports WASM.

2. **REST Server**: Removed in v0.8.0. The project now focuses on CLI and WASM modes only.

3. **Configuration**: Ensure `config/client.toml` exists before running, or the wizard will guide you through setup.

---

## 📊 Build Times (Approximate)

| Build Type | Time | Size |
|:-----------|:----:|:----:|
| Debug (native) | ~2 min | ~50 MB |
| Release (native) | ~5 min | ~15 MB |
| WASM (SDK only) | ~3 min | ~500 KB |

---

*Last Updated: 2026-04-01*
*Version: 0.8.0*

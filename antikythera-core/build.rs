use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let crate_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let src_dir = crate_dir.join("src");
    let mut violations = Vec::new();

    scan_dir(&src_dir, &mut violations);

    if !violations.is_empty() {
        eprintln!("logging policy violations detected in antikythera-core:");
        for violation in violations {
            eprintln!("  {violation}");
        }
        panic!(
            "build rejected: use antikythera-log / core logging wrappers instead of println!/eprintln!/dbg!/tracing"
        );
    }
}

fn scan_dir(dir: &Path, violations: &mut Vec<String>) {
    let entries = fs::read_dir(dir).expect("read dir");
    for entry in entries {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            scan_dir(&path, violations);
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            println!("cargo:rerun-if-changed={}", path.display());
            scan_file(&path, violations);
        }
    }
}

fn scan_file(path: &Path, violations: &mut Vec<String>) {
    let content = fs::read_to_string(path).expect("read file");
    let is_local_observability_reexport = path
        .to_string_lossy()
        .replace('\\', "/")
        .ends_with("src/application/observability/mod.rs");

    for (index, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with("//!") || trimmed.starts_with("///") {
            continue;
        }

        if is_local_observability_reexport && trimmed.starts_with("pub use tracing::{") {
            continue;
        }

        let banned = [
            "println!(",
            "eprintln!(",
            "dbg!(",
            "tracing::",
            "use tracing::",
        ];

        if banned.iter().any(|pattern| line.contains(pattern)) {
            violations.push(format!("{}:{}: {}", path.display(), index + 1, line.trim()));
        }
    }
}

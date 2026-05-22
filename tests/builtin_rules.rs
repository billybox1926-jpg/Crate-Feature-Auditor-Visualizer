use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{env, fs, path::PathBuf};

static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn builtin_rules_apply_outside_repo_checkout() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let manifest = std::fs::canonicalize("tests/fixtures/default-serde-active")
        .expect("canonicalize fixture manifest path");
    let cwd = empty_temp_dir();

    let output = Command::new(binary)
        .args([
            "--manifest-path",
            manifest.to_str().expect("fixture path should be utf-8"),
            "--format",
            "json",
            "--crate",
            "serde",
        ])
        .current_dir(&cwd)
        .output()
        .expect("run cargo-feature-lens outside repo root");

    let _ = fs::remove_dir_all(&cwd);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"kind\": \"DefaultFeature\""), "{stdout}");
    assert!(stdout.contains("Default features enable 'std'"), "{stdout}");
}

fn empty_temp_dir() -> PathBuf {
    let unique = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = env::temp_dir().join(format!(
        "cargo-feature-lens-outside-root-{}-{}",
        std::process::id(),
        unique
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

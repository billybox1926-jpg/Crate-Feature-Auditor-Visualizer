use std::process::Command;

#[test]
fn binary_can_render_markdown_for_fixture() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/basic",
            "--crate",
            "feature-lens-fixture",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Feature Footprint Report"));
    assert!(stdout.contains("feature-lens-fixture"));
}

#[test]
fn binary_can_render_json_for_fixture() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/basic",
            "--format",
            "json",
            "--crate",
            "feature-lens-fixture",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"crate_count\""));
    assert!(stdout.contains("\"name\": \"feature-lens-fixture\""));
    assert!(stdout.trim_start().starts_with('{'));
}

#[test]
fn check_mode_fails_at_configured_threshold() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/basic",
            "--crate",
            "feature-lens-fixture",
            "--check",
            "--fail-on",
            "warning",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Duplicate"));
}

#[test]
fn check_mode_passes_when_threshold_is_higher_than_findings() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/basic",
            "--crate",
            "feature-lens-fixture",
            "--check",
            "--fail-on",
            "error",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn terminal_report_includes_reqwest_conflict_finding() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/conflict-reqwest",
            "--crate",
            "reqwest",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("✖ Conflict"));
    assert!(stdout.contains("TLS backends are mutually exclusive"));
}

#[test]
fn markdown_report_includes_reqwest_conflict_finding() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/conflict-reqwest",
            "--format",
            "markdown",
            "--crate",
            "reqwest",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("| reqwest |"));
    assert!(stdout.contains("✖ Conflict: TLS backends are mutually exclusive"));
}

#[test]
fn json_report_includes_reqwest_conflict_finding() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/conflict-reqwest",
            "--format",
            "json",
            "--crate",
            "reqwest",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"kind\": \"Conflict\""));
    assert!(stdout.contains("\"severity\": \"Error\""));
    assert!(stdout.contains("\"feature\": \"native-tls, rustls-tls\""));
    assert!(stdout.contains("TLS backends are mutually exclusive"));
}

#[test]
fn reqwest_non_conflict_fixture_does_not_emit_conflict_finding() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/non-conflict-reqwest",
            "--crate",
            "reqwest",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("Conflict"));
    assert!(!stdout.contains("TLS backends are mutually exclusive"));
}

#[test]
fn check_mode_fails_on_conflict_threshold() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/conflict-reqwest",
            "--crate",
            "reqwest",
            "--check",
            "--fail-on",
            "error",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("✖ Conflict"));
}

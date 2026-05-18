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
            "tests/fixtures/duplicate-feature-multiparent",
            "--crate",
            "fixture-leaf",
            "--check",
            "--fail-on",
            "warning",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("⚠ Duplicate"));
    assert!(stdout.contains("feature `shared`"));
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
fn terminal_report_includes_duplicate_feature_lineage() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/duplicate-feature-multiparent",
            "--crate",
            "fixture-leaf",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("┌─ fixture-leaf (0.1.0)"));
    assert!(stdout.contains("⚠ Duplicate"));
    assert!(stdout.contains(
        "feature `shared` is requested through multiple lineages: mid-a -> fixture-leaf -> shared; mid-b -> fixture-leaf -> shared"
    ));
}

#[test]
fn markdown_report_includes_duplicate_feature_lineage() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/duplicate-feature-multiparent",
            "--format",
            "markdown",
            "--crate",
            "fixture-leaf",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("| fixture-leaf | 0.1.0 |"));
    assert!(stdout.contains("⚠ Duplicate: feature `shared` is requested through multiple lineages: mid-a -> fixture-leaf -> shared; mid-b -> fixture-leaf -> shared"));
}

#[test]
fn json_report_includes_duplicate_feature_lineage() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/duplicate-feature-multiparent",
            "--format",
            "json",
            "--crate",
            "fixture-leaf",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"name\": \"fixture-leaf\""));
    assert!(stdout.contains("\"kind\": \"Duplicate\""));
    assert!(stdout.contains("\"severity\": \"Warning\""));
    assert!(stdout.contains("\"feature\": \"shared\""));
    assert!(stdout.contains("feature `shared` is requested through multiple lineages: mid-a -> fixture-leaf -> shared; mid-b -> fixture-leaf -> shared"));
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

#[test]
fn terminal_report_includes_log_default_feature_suggestion() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/default-log-active",
            "--crate",
            "log",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ℹ DefaultFeature"));
    assert!(stdout.contains("Default features pull in no dependencies"));
}

#[test]
fn markdown_report_includes_log_default_feature_suggestion() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/default-log-active",
            "--format",
            "markdown",
            "--crate",
            "log",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("| log |"));
    assert!(stdout.contains("ℹ DefaultFeature: Default features pull in no dependencies"));
}

#[test]
fn json_report_includes_reqwest_default_feature_suggestion() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/default-reqwest-active",
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
    assert!(stdout.contains("\"kind\": \"DefaultFeature\""));
    assert!(stdout.contains("\"severity\": \"Info\""));
    assert!(stdout.contains("\"feature\": \"default\""));
    assert!(stdout.contains("Default features enable 'default-tls'"));
}

#[test]
fn disabled_default_features_do_not_emit_opt_out_suggestion() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/default-log-disabled",
            "--crate",
            "log",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("DefaultFeature"));
    assert!(!stdout.contains("Default features pull in no dependencies"));
}

#[test]
fn check_mode_fails_on_info_default_feature_threshold() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/default-log-active",
            "--crate",
            "log",
            "--check",
            "--fail-on",
            "info",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ℹ DefaultFeature"));
}

#[test]
fn check_mode_passes_when_default_feature_threshold_is_warning() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/default-log-active",
            "--crate",
            "log",
            "--check",
            "--fail-on",
            "warning",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

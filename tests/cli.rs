use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{env, fs, path::PathBuf};

static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

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
    assert!(stdout.contains("\"finding_summary\""));
    assert!(stdout.contains("\"by_severity\""));
    assert!(stdout.contains("\"by_kind\""));
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
    assert!(stdout.contains("Finding summary: 2 visible findings"));
    assert!(stdout.contains("severity: info 0, warning 2, error 0"));
    assert!(stdout.contains("kind: Unused 1, Duplicate 1"));
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
    let summary_position = stdout.find("## Finding Summary").unwrap();
    let table_position = stdout
        .find("| Crate | Version | Active Features | Findings |")
        .unwrap();
    assert!(summary_position < table_position);
    assert!(stdout.contains("- Total visible findings: **2**"));
    assert!(stdout.contains("- By severity: info 0, warning 2, error 0"));
    assert!(stdout.contains("- By kind: `Unused` 1, `Duplicate` 1"));
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
    assert!(stdout.contains("\"finding_summary\""));
    assert!(stdout.contains("\"total\": 2"));
    assert!(stdout.contains("\"warning\": 2"));
    assert!(stdout.contains("\"Unused\": 1"));
    assert!(stdout.contains("\"Duplicate\": 1"));
    assert!(stdout.contains("\"name\": \"fixture-leaf\""));
    assert!(stdout.contains("\"kind\": \"Duplicate\""));
    assert!(stdout.contains("\"severity\": \"Warning\""));
    assert!(stdout.contains("\"feature\": \"shared\""));
    assert!(stdout.contains("feature `shared` is requested through multiple lineages: mid-a -> fixture-leaf -> shared; mid-b -> fixture-leaf -> shared"));
}

#[test]
fn terminal_summary_respects_min_severity_filter() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/default-log-active",
            "--crate",
            "log",
            "--min-severity",
            "warning",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Finding summary: 0 visible findings"));
    assert!(stdout.contains("severity: info 0, warning 0, error 0"));
    assert!(!stdout.contains("kind: DefaultFeature"));
    assert!(!stdout.contains("ℹ DefaultFeature"));
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

fn write_temp_local_rules(contents: &str) -> PathBuf {
    let unique = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = env::temp_dir().join(format!(
        "cargo-feature-lens-test-{}-{}",
        std::process::id(),
        unique
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    fs::write(dir.join("feature-lens.toml"), contents).expect("write local rules");
    dir
}

fn fixture_manifest_path() -> String {
    std::fs::canonicalize("tests/fixtures/basic")
        .expect("canonicalize fixture")
        .display()
        .to_string()
}

#[test]
fn local_rules_emit_custom_bloat_in_terminal_markdown_and_json() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let cwd = write_temp_local_rules(
        r#"
[[bloat]]
crate = "feature-lens-fixture"
feature = "default"
severity = "warning"
message = "custom local bloat"
"#,
    );

    for format in [None, Some("markdown"), Some("json")] {
        let manifest = fixture_manifest_path();
        let mut command = Command::new(binary);
        command.args([
            "--manifest-path",
            &manifest,
            "--crate",
            "feature-lens-fixture",
        ]);
        if let Some(format) = format {
            command.args(["--format", format]);
        }
        let output = command.current_dir(&cwd).output().expect("run binary");
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("custom local bloat"), "{stdout}");
    }
}

#[test]
fn malformed_local_feature_lens_toml_exits_with_parse_error() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let cwd = write_temp_local_rules(
        r#"
[[bloat]]
crate = "feature-lens-fixture"
feature = "default"
severity = "definitely-not-valid"
message = "invalid severity"
"#,
    );

    let output = Command::new(binary)
        .args(["--manifest-path", &fixture_manifest_path()])
        .args(["--crate", "feature-lens-fixture"])
        .current_dir(&cwd)
        .output()
        .expect("run binary");

    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(combined.contains("failed to parse"), "{combined}");
    assert!(
        combined.contains("invalid `severity`: definitely-not-valid"),
        "{combined}"
    );
}

#[test]
fn check_mode_fails_on_local_rule_finding() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let cwd = write_temp_local_rules(
        r#"
[[bloat]]
crate = "feature-lens-fixture"
feature = "default"
severity = "warning"
message = "custom local bloat"
"#,
    );
    let output = Command::new(binary)
        .args(["--manifest-path", &fixture_manifest_path()])
        .args([
            "--crate",
            "feature-lens-fixture",
            "--check",
            "--fail-on",
            "warning",
        ])
        .current_dir(&cwd)
        .output()
        .expect("run binary");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("custom local bloat"));
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

#[test]
fn binary_can_render_dot_for_fixture_graph() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/duplicate-feature-multiparent",
            "--format",
            "dot",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("digraph feature_lens {"));
    assert!(stdout.contains("[label=\"duplicate-feature-root 0.1.0"));
    assert!(
        stdout.contains("duplicate_feature_root_0_1_0\" -> \"crate_path_")
            && stdout.contains("mid_a_0_1_0\";")
    );
}

#[test]
fn binary_can_render_mermaid_for_fixture_graph() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/duplicate-feature-multiparent",
            "--format",
            "mermaid",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("graph TD"));
    assert!(stdout.contains("duplicate_feature_root_0_1_0[\"duplicate-feature-root 0.1.0"));
    assert!(
        stdout.contains("duplicate_feature_root_0_1_0 --> crate_path_")
            && stdout.contains("mid_a_0_1_0")
    );
}

#[test]
fn binary_writes_dot_output_file() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let path = std::env::temp_dir().join(format!(
        "cargo-feature-lens-dot-{}-{}.dot",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let _ = std::fs::remove_file(&path);

    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/duplicate-feature-multiparent",
            "--format",
            "dot",
            "--output",
            path.to_str().expect("temp path should be valid utf-8"),
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    let rendered = std::fs::read_to_string(&path).expect("dot output file should exist");
    let _ = std::fs::remove_file(&path);
    assert!(rendered.starts_with("digraph feature_lens {"));
    assert!(
        rendered.contains("duplicate_feature_root_0_1_0\" -> \"crate_path_")
            && rendered.contains("mid_a_0_1_0\";")
    );
}

#[test]
fn source_cfg_usage_suppresses_unused_in_cli_report() {
    let binary = env!("CARGO_BIN_EXE_cargo-feature-lens");
    let output = Command::new(binary)
        .args([
            "--manifest-path",
            "tests/fixtures/unused-source-used",
            "--crate",
            "unused-source-used",
        ])
        .output()
        .expect("failed to run cargo-feature-lens");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("Unused"));
}

#[test]
fn unreferenced_active_feature_still_reports_unused_in_cli_and_check_mode() {
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
    assert!(stdout.contains("Unused"));
    assert!(stdout.contains("feature `shared`"));

    let check_output = Command::new(binary)
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
        .expect("failed to run cargo-feature-lens in check mode");

    assert!(!check_output.status.success());
    let check_stdout = String::from_utf8_lossy(&check_output.stdout);
    assert!(check_stdout.contains("Unused"));
}

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use cargo_feature_lens::analysis::{self, AnalysisContext, Finding, FindingKind, Severity};
use cargo_feature_lens::error::{Error, Result};
use cargo_feature_lens::manifest::ManifestCache;
use cargo_feature_lens::metadata;
use cargo_feature_lens::report::{self, OutputFormat, ReportOptions};
use cargo_feature_lens::resolver;

const BUILTIN_SUGGESTIONS_JSON: &str = include_str!("../docs/suggestions.json");
const MAX_REMOTE_CRATE_NAME_LEN: usize = 64;
const MAX_REMOTE_CRATE_VERSION_LEN: usize = 128;

#[derive(Debug)]
struct Cli {
    output: Option<PathBuf>,
    unused: bool,
    bloat: bool,
    crate_filter: Option<String>,
    manifest_path: PathBuf,
    format: Option<OutputFormat>,
    check: bool,
    fail_on: Severity,
    min_severity: Option<Severity>,
    remote: bool,
    crate_version: Option<String>,
}

fn main() -> Result<(), Error> {
    run()
}

fn run() -> Result<(), Error> {
    let cli = Cli::parse(cargo_aware_args())?;
    let remote_analysis = is_remote_analysis(&cli);

    let metadata = load_metadata_for_cli(&cli, remote_analysis)?;
    let mut manifests = ManifestCache::default();
    let graph = resolver::resolve(&metadata, &mut manifests)?;

    let current_dir = std::env::current_dir()?;
    let mut suggestions = load_builtin_suggestions()?;
    let local_suggestions_path = current_dir.join("feature-lens.toml");
    let local_suggestions = analysis::Suggestions::load_local_optional(&local_suggestions_path)?;
    suggestions.extend(local_suggestions);
    let context = AnalysisContext::new(&graph, &suggestions);
    let findings = analysis::run_all(&context);

    let format = cli.format.unwrap_or_else(|| {
        if cli.output.is_some() {
            OutputFormat::Markdown
        } else {
            OutputFormat::Terminal
        }
    });

    let report_crate_filter = if remote_analysis {
        None
    } else {
        cli.crate_filter.clone()
    };

    let options = ReportOptions {
        format,
        only_unused: cli.unused,
        only_bloat: cli.bloat,
        crate_filter: report_crate_filter,
        min_severity: cli.min_severity,
    };

    let rendered = report::render(&graph, &findings, &options)?;

    if let Some(path) = &cli.output {
        fs::write(path, rendered)?;
    } else {
        print!("{rendered}");
    }

    if cli.check && has_failing_findings(&findings, &cli, remote_analysis) {
        std::process::exit(1);
    }

    Ok(())
}

fn load_builtin_suggestions() -> Result<analysis::Suggestions> {
    Ok(analysis::parse_suggestions(BUILTIN_SUGGESTIONS_JSON))
}

fn load_metadata_for_cli(
    cli: &Cli,
    remote_analysis: bool,
) -> Result<metadata::Metadata> {
    if remote_analysis {
        return load_remote_crate_metadata(cli);
    }

    metadata::load_metadata(&cli.manifest_path)
}

fn is_remote_analysis(cli: &Cli) -> bool {
    cli.remote || should_analyze_remote(cli)
}

fn should_analyze_remote(cli: &Cli) -> bool {
    cli.crate_filter.is_some()
        && cli.manifest_path == Path::new(".")
        && !Path::new("Cargo.toml").exists()
}

fn load_remote_crate_metadata(cli: &Cli) -> Result<metadata::Metadata> {
    let crate_name = cli
        .crate_filter
        .as_deref()
        .ok_or("remote analysis requires --crate <name>")?;
    let crate_version = cli.crate_version.as_deref().unwrap_or("*");

    validate_remote_crate_name(crate_name)?;
    validate_remote_crate_version(crate_version)?;

    let dir = tempfile::tempdir().map_err(|e| format!("failed to create temp dir: {e}"))?;
    fs::create_dir_all(dir.path().join("src"))?;
    fs::write(dir.path().join("src").join("lib.rs"), "")?;
    let manifest = dir.path().join("Cargo.toml");
    fs::write(
        &manifest,
        format!(
            "[package]\nname = \"feature_lens_remote_probe\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n[dependencies]\n\"{crate_name}\" = \"{crate_version}\"\n"
        ),
    )?;

    let result = metadata::load_metadata_manifest(&manifest)
        .and_then(|metadata| re_root_remote_metadata(metadata, crate_name));
    dir.close()?;
    result
}

fn validate_remote_crate_name(crate_name: &str) -> Result<()> {
    if crate_name.is_empty() {
        return Err(Error::Cli("invalid --crate value: crate name cannot be empty".into()));
    }
    if crate_name.len() > MAX_REMOTE_CRATE_NAME_LEN {
        return Err(Error::Cli(format!(
            "invalid --crate value {crate_name:?}: crate name is longer than {MAX_REMOTE_CRATE_NAME_LEN} bytes"
        )));
    }
    if !crate_name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        return Err(Error::Cli(format!(
            "invalid --crate value {crate_name:?}: use only ASCII letters, numbers, hyphens, or underscores"
        )));
    }
    Ok(())
}

fn validate_remote_crate_version(crate_version: &str) -> Result<()> {
    if crate_version.is_empty() {
        return Err(Error::Cli("invalid --crate-version value: version requirement cannot be empty".into()));
    }
    if crate_version.len() > MAX_REMOTE_CRATE_VERSION_LEN {
        return Err(Error::Cli(format!(
            "invalid --crate-version value {crate_version:?}: version requirement is longer than {MAX_REMOTE_CRATE_VERSION_LEN} bytes"
        )));
    }
    if !crate_version
        .chars()
        .any(|ch| ch.is_ascii_digit() || ch == '*')
    {
        return Err(Error::Cli(format!(
            "invalid --crate-version value {crate_version:?}: version requirement must include a digit or `*`"
        )));
    }
    if !crate_version.chars().all(is_remote_version_char) {
        return Err(Error::Cli(format!(
            "invalid --crate-version value {crate_version:?}: use only Cargo version requirement characters"
        )));
    }
    Ok(())
}

fn is_remote_version_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
        || matches!(
            ch,
            '.' | '-' | '+' | '*' | '^' | '~' | '<' | '>' | '=' | ',' | ' '
        )
}

fn re_root_remote_metadata(
    mut metadata: metadata::Metadata,
    crate_name: &str,
) -> Result<metadata::Metadata> {
    let workspace_member = metadata
        .workspace_members
        .first()
        .cloned()
        .ok_or("remote metadata did not include the probe workspace member")?;
    let probe_node = metadata
        .resolve_nodes
        .iter()
        .find(|node| node.id == workspace_member)
        .ok_or("remote metadata did not include the probe resolve node")?;
    let target_id = probe_node
        .dependencies
        .iter()
        .find(|dependency_id| {
            metadata
                .packages
                .iter()
                .any(|package| package.id == **dependency_id && package.name == crate_name)
        })
        .cloned()
        .ok_or_else(|| format!("crate `{crate_name}` was not resolved from crates.io"))?;

    let mut reachable = std::collections::BTreeSet::new();
    let mut pending = vec![target_id.clone()];
    while let Some(package_id) = pending.pop() {
        if !reachable.insert(package_id.clone()) {
            continue;
        }
        if let Some(node) = metadata
            .resolve_nodes
            .iter()
            .find(|node| node.id == package_id)
        {
            pending.extend(node.dependencies.iter().cloned());
        }
    }

    metadata.workspace_members = vec![target_id];
    metadata
        .packages
        .retain(|package| reachable.contains(&package.id));
    metadata
        .resolve_nodes
        .retain(|node| reachable.contains(&node.id));

    Ok(metadata)
}

fn has_failing_findings(findings: &[Finding], cli: &Cli, remote_analysis: bool) -> bool {
    findings.iter().any(|finding| {
        finding.severity >= cli.fail_on
            && kind_selected(finding.kind, cli)
            && (remote_analysis
                || cli
                    .crate_filter
                    .as_deref()
                    .map(|filter| finding.crate_name.contains(filter))
                    .unwrap_or(true))
    })
}

fn kind_selected(kind: FindingKind, cli: &Cli) -> bool {
    (!cli.unused || kind == FindingKind::Unused) && (!cli.bloat || kind == FindingKind::Bloat)
}

impl Cli {
    fn parse(args: Vec<OsString>) -> Result<Self, Error> {
        let mut cli = Self {
            output: None,
            unused: false,
            bloat: false,
            crate_filter: None,
            manifest_path: PathBuf::from("."),
            format: None,
            check: false,
            fail_on: Severity::Warning,
            min_severity: None,
            remote: false,
            crate_version: None,
        };

        let mut iter = args.into_iter().skip(1);

        while let Some(arg) = iter.next() {
            let Some(arg) = arg.to_str() else { continue };

            match arg {
                "-o" | "--output" => {
                    let Some(value) = iter.next() else {
                        return Err("--output requires a file path".into());
                    };
                    cli.output = Some(PathBuf::from(value));
                }
                "--unused" => cli.unused = true,
                "--bloat" => cli.bloat = true,
                "--crate" => {
                    let Some(value) = iter.next().and_then(|value| value.into_string().ok()) else {
                        return Err("--crate requires a crate name".into());
                    };
                    cli.crate_filter = Some(value);
                }
                "--manifest-path" => {
                    if let Some(value) = iter.next() {
                        cli.manifest_path = PathBuf::from(value);
                    }
                }
                "--check" => cli.check = true,
                "--remote" => cli.remote = true,
                "--fail-on" => {
                    let Some(value) = iter.next().and_then(|value| value.into_string().ok()) else {
                        return Err("--fail-on requires one of: info, warning, error".into());
                    };
                    cli.fail_on = Severity::parse(&value)
                        .ok_or_else(|| format!("unsupported severity `{value}`"))?;
                }
                "--min-severity" => {
                    let Some(value) = iter.next().and_then(|value| value.into_string().ok()) else {
                        return Err("--min-severity requires one of: info, warning, error".into());
                    };
                    cli.min_severity = Some(
                        Severity::parse(&value)
                            .ok_or_else(|| format!("unsupported severity `{value}`"))?,
                    );
                }
                "--crate-version" => {
                    let Some(value) = iter.next().and_then(|value| value.into_string().ok()) else {
                        return Err("--crate-version requires a version requirement".into());
                    };
                    cli.crate_version = Some(value);
                }
                "--format" => {
                    let Some(value) = iter.next().and_then(|value| value.into_string().ok()) else {
                        return Err(
                            "--format requires one of: terminal, markdown, json, dot, mermaid"
                                .into(),
                        );
                    };

                    cli.format = Some(match value.as_str() {
                        "terminal" => OutputFormat::Terminal,
                        "markdown" => OutputFormat::Markdown,
                        "json" => OutputFormat::Json,
                        "dot" => OutputFormat::Dot,
                        "mermaid" => OutputFormat::Mermaid,
                        _ => return Err(format!("unsupported output format `{value}`").into()),
                    });
                }
                "-h" | "--help" => {
                    println!(
                        "Usage: cargo feature-lens [--output PATH] [--format terminal|markdown|json|dot|mermaid] [--check] [--fail-on info|warning|error] [--min-severity info|warning|error] [--unused] [--bloat] [--crate TEXT] [--remote] [--crate-version VERSION] [--manifest-path PATH]"
                    );
                    std::process::exit(0);
                }
                unknown => return Err(format!("unknown argument `{unknown}`").into()),
            }
        }

        Ok(cli)
    }
}

fn cargo_aware_args() -> Vec<OsString> {
    let mut args: Vec<OsString> = env::args_os().collect();

    if args.get(1).and_then(|arg| arg.to_str()) == Some("feature-lens") {
        args.remove(1);
    }

    args
}

#[cfg(test)]
mod tests {
    use super::{
        load_builtin_suggestions, re_root_remote_metadata, validate_remote_crate_name,
        validate_remote_crate_version, Cli,
    };
    use cargo_feature_lens::metadata::{Metadata, Package, ResolveNode};
    use cargo_feature_lens::Severity;
    use std::ffi::OsString;
    use std::path::PathBuf;

    #[test]
    fn parses_direct_binary_flags() {
        let cli = Cli::parse(vec![
            OsString::from("cargo-feature-lens"),
            OsString::from("--unused"),
            OsString::from("--crate"),
            OsString::from("serde"),
        ])
        .unwrap();

        assert!(cli.unused);
        assert_eq!(cli.crate_filter.as_deref(), Some("serde"));
        assert_eq!(cli.manifest_path, PathBuf::from("."));
        assert_eq!(cli.format, None);
        assert!(!cli.check);
        assert_eq!(cli.fail_on, Severity::Warning);
    }

    #[test]
    fn parses_cargo_subcommand_flags() {
        let cli = Cli::parse(vec![
            OsString::from("cargo-feature-lens"),
            OsString::from("--manifest-path"),
            OsString::from("Cargo.toml"),
            OsString::from("--crate"),
            OsString::from("cargo-feature-lens"),
        ])
        .unwrap();

        assert_eq!(cli.manifest_path, PathBuf::from("Cargo.toml"));
        assert_eq!(cli.crate_filter.as_deref(), Some("cargo-feature-lens"));
    }

    #[test]
    fn rejects_missing_value_for_output() {
        let error = Cli::parse(vec![
            OsString::from("cargo-feature-lens"),
            OsString::from("--output"),
        ])
        .expect_err("missing --output value should fail");

        assert_eq!(error.to_string(), "--output requires a file path");
    }

    #[test]
    fn rejects_missing_value_for_crate() {
        let error = Cli::parse(vec![
            OsString::from("cargo-feature-lens"),
            OsString::from("--crate"),
        ])
        .expect_err("missing --crate value should fail");

        assert_eq!(error.to_string(), "--crate requires a crate name");
    }

    #[test]
    fn rejects_missing_value_for_crate_version() {
        let error = Cli::parse(vec![
            OsString::from("cargo-feature-lens"),
            OsString::from("--crate-version"),
        ])
        .expect_err("missing --crate-version value should fail");

        assert_eq!(
            error.to_string(),
            "--crate-version requires a version requirement"
        );
    }

    #[test]
    fn strips_cargo_subcommand_name() {
        let mut args = vec![
            OsString::from("cargo-feature-lens"),
            OsString::from("feature-lens"),
            OsString::from("--manifest-path"),
            OsString::from("Cargo.toml"),
        ];

        if args.get(1).and_then(|arg| arg.to_str()) == Some("feature-lens") {
            args.remove(1);
        }

        let cli = Cli::parse(args).unwrap();
        assert_eq!(cli.manifest_path, PathBuf::from("Cargo.toml"));
    }

    #[test]
    fn embedded_builtin_suggestions_are_available() {
        let suggestions = load_builtin_suggestions().unwrap();

        assert!(!suggestions.conflicts.is_empty());
        assert!(!suggestions.bloat.is_empty());
        assert!(suggestions
            .default_features
            .iter()
            .any(|rule| rule.crate_name == "serde"));
    }

    #[test]
    fn validates_remote_crate_names() {
        for valid in ["serde", "serde_json", "tokio-util", "cargo-feature-lens"] {
            validate_remote_crate_name(valid).unwrap();
        }

        for invalid in [
            "",
            "serde json",
            "serde/path",
            "../serde",
            "serde\nother",
            "serde\" = \"*",
        ] {
            assert!(
                validate_remote_crate_name(invalid).is_err(),
                "{invalid:?} should be rejected"
            );
        }
    }

    #[test]
    fn validates_remote_crate_versions() {
        for valid in [
            "*",
            "1.0.0",
            "^1.0",
            "~1.2",
            ">= 1.0, < 2.0",
            "1.0.0-alpha.1+build.5",
        ] {
            validate_remote_crate_version(valid).unwrap();
        }

        for invalid in [
            "",
            "latest",
            "1.0/../../evil",
            "1.0\nserde = \"*",
            "1.0\"\nother = \"*",
            "{ version = \"1.0\" }",
        ] {
            assert!(
                validate_remote_crate_version(invalid).is_err(),
                "{invalid:?} should be rejected"
            );
        }
    }

    #[test]
    fn remote_metadata_is_re_rooted_at_requested_crate() {
        let metadata = Metadata {
            packages: vec![
                package("probe 0.0.0", "feature_lens_remote_probe"),
                package("demo 1.0.0", "demo"),
                package("helper 1.0.0", "helper"),
                package("unrelated 1.0.0", "unrelated"),
            ],
            workspace_members: vec!["probe 0.0.0".to_string()],
            resolve_nodes: vec![
                ResolveNode {
                    id: "probe 0.0.0".to_string(),
                    dependencies: vec!["demo 1.0.0".to_string()],
                    ..ResolveNode::default()
                },
                ResolveNode {
                    id: "demo 1.0.0".to_string(),
                    dependencies: vec!["helper 1.0.0".to_string()],
                    ..ResolveNode::default()
                },
                ResolveNode {
                    id: "helper 1.0.0".to_string(),
                    ..ResolveNode::default()
                },
                ResolveNode {
                    id: "unrelated 1.0.0".to_string(),
                    ..ResolveNode::default()
                },
            ],
        };

        let re_rooted = re_root_remote_metadata(metadata, "demo").unwrap();

        assert_eq!(re_rooted.workspace_members, vec!["demo 1.0.0"]);
        assert_eq!(
            re_rooted
                .packages
                .iter()
                .map(|package| package.name.as_str())
                .collect::<Vec<_>>(),
            vec!["demo", "helper"]
        );
        assert!(re_rooted
            .resolve_nodes
            .iter()
            .all(|node| node.id != "probe 0.0.0"));
    }

    fn package(id: &str, name: &str) -> Package {
        Package {
            id: id.to_string(),
            name: name.to_string(),
            version: "1.0.0".to_string(),
            ..Package::default()
        }
    }
}

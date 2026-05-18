use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use cargo_feature_lens::analysis::{self, AnalysisContext, Finding, FindingKind, Severity};
use cargo_feature_lens::manifest::ManifestCache;
use cargo_feature_lens::metadata;
use cargo_feature_lens::report::{self, OutputFormat, ReportOptions};
use cargo_feature_lens::resolver;

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

fn main() -> Result<(), Box<dyn Error>> {
    run()
}

fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse(cargo_aware_args())?;
    let remote_analysis = is_remote_analysis(&cli);

    let metadata = load_metadata_for_cli(&cli, remote_analysis)?;
    let mut manifests = ManifestCache::default();
    let graph = resolver::resolve(&metadata, &mut manifests)?;

    let current_dir = std::env::current_dir()?;
    let docs_suggestions_path = current_dir.join("docs").join("suggestions.json");
    let root_suggestions_path = current_dir.join("suggestions.json");
    let suggestions_path = if docs_suggestions_path.exists() {
        docs_suggestions_path
    } else {
        root_suggestions_path
    };
    let mut suggestions = analysis::Suggestions::load_optional(&suggestions_path)?;
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

fn load_metadata_for_cli(
    cli: &Cli,
    remote_analysis: bool,
) -> Result<metadata::Metadata, Box<dyn Error>> {
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

fn load_remote_crate_metadata(cli: &Cli) -> Result<metadata::Metadata, Box<dyn Error>> {
    let crate_name = cli
        .crate_filter
        .as_deref()
        .ok_or("remote analysis requires --crate <name>")?;
    let crate_version = cli.crate_version.as_deref().unwrap_or("*");
    let dir = std::env::temp_dir().join(format!(
        "cargo-feature-lens-remote-{}-{}",
        std::process::id(),
        crate_name
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
            .collect::<String>()
    ));
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(dir.join("src"))?;
    fs::write(dir.join("src").join("lib.rs"), "")?;
    let manifest = dir.join("Cargo.toml");
    fs::write(
        &manifest,
        format!(
            "[package]\nname = \"feature_lens_remote_probe\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n[dependencies]\n\"{crate_name}\" = \"{crate_version}\"\n"
        ),
    )?;

    let result = metadata::load_metadata_manifest(&manifest)
        .and_then(|metadata| re_root_remote_metadata(metadata, crate_name));
    let _ = fs::remove_dir_all(&dir);
    result
}

fn re_root_remote_metadata(
    mut metadata: metadata::Metadata,
    crate_name: &str,
) -> Result<metadata::Metadata, Box<dyn Error>> {
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
    fn parse(args: Vec<OsString>) -> Result<Self, Box<dyn std::error::Error>> {
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
                "-o" | "--output" => cli.output = iter.next().map(PathBuf::from),
                "--unused" => cli.unused = true,
                "--bloat" => cli.bloat = true,
                "--crate" => {
                    cli.crate_filter = iter.next().and_then(|value| value.into_string().ok())
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
                    cli.crate_version = iter.next().and_then(|value| value.into_string().ok())
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
    use super::{re_root_remote_metadata, Cli};
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

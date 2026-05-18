use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use cargo_feature_lens::analysis::{self, AnalysisContext};
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
}

fn main() -> Result<(), Box<dyn Error>> {
    run()
}

fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse(cargo_aware_args())?;

    if cli.crate_filter.is_some() && cli.manifest_path == PathBuf::from(".") {
        return Err(
            "`--crate` currently filters crates from local Cargo metadata; remote crates.io analysis is not implemented yet. Use `--manifest-path Cargo.toml --crate <name>` to filter a local project, or track issue #3 for remote crate analysis."
                .into(),
        );
    }

    let metadata = metadata::load_metadata(&cli.manifest_path)?;
    let mut manifests = ManifestCache::default();
    let graph = resolver::resolve(&metadata, &mut manifests)?;

    let suggestions_path = std::env::current_dir()?.join("suggestions.json");
    let suggestions = analysis::Suggestions::load_optional(&suggestions_path)?;
    let context = AnalysisContext::new(&graph, &suggestions);
    let findings = analysis::run_all(&context);

    let format = cli.format.unwrap_or_else(|| {
        if cli.output.is_some() {
            OutputFormat::Markdown
        } else {
            OutputFormat::Terminal
        }
    });

    let options = ReportOptions {
        format,
        only_unused: cli.unused,
        only_bloat: cli.bloat,
        crate_filter: cli.crate_filter,
    };

    let rendered = report::render(&graph, &findings, &options)?;

    if let Some(path) = cli.output {
        fs::write(&path, rendered)?;
    } else {
        print!("{rendered}");
    }

    Ok(())
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
                "--format" => {
                    let Some(value) = iter.next().and_then(|value| value.into_string().ok()) else {
                        return Err("--format requires one of: terminal, markdown, json".into());
                    };

                    cli.format = Some(match value.as_str() {
                        "terminal" => OutputFormat::Terminal,
                        "markdown" => OutputFormat::Markdown,
                        "json" => OutputFormat::Json,
                        _ => return Err(format!("unsupported output format `{value}`").into()),
                    });
                }
                "-h" | "--help" => {
                    println!(
                        "Usage: cargo feature-lens [--output PATH] [--format terminal|markdown|json] [--unused] [--bloat] [--crate TEXT] [--manifest-path PATH]"
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
    use super::Cli;
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
}

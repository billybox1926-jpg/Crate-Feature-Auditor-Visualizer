//! Core library for `cargo-feature-lens`.
//!
//! The binary is intentionally thin: it gathers CLI options, obtains Cargo
//! metadata, resolves feature information, runs independent analysis passes,
//! and delegates rendering to the report module.

pub mod analysis;
pub mod error;
pub mod graph_export;
pub mod manifest;
pub mod metadata;
pub mod report;
pub mod resolver;

pub use analysis::{AnalysisContext, Finding, FindingKind, Severity};
pub use error::{Error, Result};
pub use resolver::{FeatureGraph, FeatureNode, FeatureSource};

// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use clap::{Args, Parser, Subcommand};
use serde::de::DeserializeOwned;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod composer;
mod counts;
mod gate;
mod model;
mod mutations;
mod parser;

#[derive(Parser)]
#[command(
    name = "kani-qualify",
    about = "Verification evidence admission and qualification mutation checks"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Args)]
struct MutationArgs {
    /// Directory containing the declared mutation fixtures
    #[arg(long)]
    fixtures: PathBuf,
    /// Built kani-driver binary, or a sibling kani executable in the same bundle
    #[arg(long)]
    kani_bin: PathBuf,
    /// Publish an immutable JSON receipt
    #[arg(long)]
    receipt: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Compose complete, explicitly identified producer runs under one profile
    Compose {
        #[arg(long)]
        consumer: PathBuf,
        #[arg(long)]
        receipt: PathBuf,
        /// JSON array of strict InputRunEvidence records; raw logs are insufficient
        #[arg(long)]
        runs: PathBuf,
        #[arg(long)]
        toolchain: PathBuf,
    },
    /// Exercise infrastructure against the selected development build; does not qualify a release
    MutationSelfTest {
        #[command(flatten)]
        args: MutationArgs,
    },
    /// Run qualification mutations after enforcing the supplied runtime identities
    Mutations {
        #[command(flatten)]
        args: MutationArgs,
        #[arg(long)]
        toolchain: PathBuf,
    },
    /// Parse result syntax; successful parsing alone is not a qualification verdict
    Parse {
        #[arg(long)]
        log: PathBuf,
    },
}

fn write_receipt_no_clobber(path: &Path, content: &str) -> Result<(), String> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| format!("cannot stage immutable receipt: {e}"))?;
    temporary.write_all(content.as_bytes()).map_err(|e| format!("cannot write receipt: {e}"))?;
    temporary.as_file().sync_all().map_err(|e| format!("cannot sync receipt: {e}"))?;
    temporary.persist_noclobber(path).map_err(|e| {
        format!("cannot publish receipt at {} without overwriting: {}", path.display(), e.error)
    })?;
    Ok(())
}

fn manifest<T: DeserializeOwned>(
    path: &Path,
    validate: impl FnOnce(&T) -> Result<(), String>,
) -> Result<(T, String), String> {
    let content = fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let parsed = serde_json::from_slice(&content)
        .map_err(|e| format!("invalid manifest {}: {e}", path.display()))?;
    validate(&parsed)?;
    Ok((parsed, mutations::hash_bytes(&content)))
}

fn run_mutations(args: MutationArgs, mode: mutations::MutationMode) -> Result<ExitCode, String> {
    let context =
        mutations::MutationContext { kani_bin: args.kani_bin, fixtures_dir: &args.fixtures, mode };
    let receipt = mutations::run_all_mutations(&context)?;
    let json = serde_json::to_string_pretty(&receipt).map_err(|e| e.to_string())?;
    println!("{json}");
    if let Some(path) = args.receipt {
        write_receipt_no_clobber(&path, &json)?;
    }
    Ok(exit_code(receipt.status))
}

fn exit_code(status: model::GateStatus) -> ExitCode {
    match status {
        model::GateStatus::Pass => ExitCode::SUCCESS,
        model::GateStatus::Fail => ExitCode::FAILURE,
    }
}

fn run() -> Result<ExitCode, String> {
    match Cli::parse().command {
        Commands::Compose { consumer, receipt, runs, toolchain } => {
            let (toolchain, toolchain_sha256) =
                manifest(&toolchain, model::ToolchainManifest::validate)?;
            let (consumer, consumer_sha256) =
                manifest(&consumer, model::ConsumerManifest::validate)?;
            let input = fs::read(&runs).map_err(|e| format!("cannot read run evidence: {e}"))?;
            let input_runs: Vec<model::InputRunEvidence> = serde_json::from_slice(&input)
                .map_err(|e| format!("invalid run evidence JSON: {e}"))?;
            let composite = composer::compose_runs(
                &toolchain,
                &consumer,
                &toolchain_sha256,
                &consumer_sha256,
                &input_runs,
            )?;
            let json = serde_json::to_string_pretty(&composite).map_err(|e| e.to_string())?;
            write_receipt_no_clobber(&receipt, &json)?;
            println!("Composite receipt written to {}", receipt.display());
            Ok(exit_code(composite.status))
        }
        Commands::MutationSelfTest { args } => {
            run_mutations(args, mutations::MutationMode::InfrastructureSelfTest)
        }
        Commands::Mutations { args, toolchain } => {
            let (toolchain, sha256) = manifest(&toolchain, model::ToolchainManifest::validate)?;
            run_mutations(args, mutations::MutationMode::Qualification { toolchain, sha256 })
        }
        Commands::Parse { log } => {
            let content = fs::read_to_string(&log)
                .map_err(|e| format!("cannot read log {}: {e}", log.display()))?;
            let parsed = parser::parse_kani_output(&content)?;
            println!("Syntax parse successful: {} harnesses parsed", parsed.harnesses.len());
            for (name, harness) in &parsed.harnesses {
                println!(
                    " - {name}: {} ({} checks, {} unreachable)",
                    harness.verdict,
                    harness.total_checks,
                    harness.unreachable.unwrap_or(0)
                );
            }
            println!("Completion: {:?}", parsed.summary);
            println!("Warnings: {:?}", parsed.warnings);
            println!("Unsupported constructs: {:?}", parsed.unsupported_constructs);
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}

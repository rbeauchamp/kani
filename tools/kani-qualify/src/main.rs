// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod composer;
mod gate;
mod model;
mod mutations;
mod parser;

#[derive(Parser)]
#[command(name = "kani-qualify")]
#[command(about = "Verification qualification, profile gating, and red-mutation test runner")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute fail-closed red mutations to verify detection of unsound verification conditions
    Mutations {
        /// Path to toolchain manifest (e.g. qualification/manifests/core-v1/toolchain.json)
        #[arg(long)]
        toolchain: PathBuf,

        /// Path to mutation fixtures directory (e.g. qualification/fixtures/mutations)
        #[arg(long)]
        fixtures: PathBuf,

        /// Path to kani executable (defaults to "kani" on PATH)
        #[arg(long, default_value = "kani")]
        kani_bin: PathBuf,

        /// Optional path to write output JSON receipt
        #[arg(long)]
        receipt: Option<PathBuf>,
    },
    /// Parse and summarize Kani verification log
    Parse {
        /// Path to log file
        #[arg(long)]
        log: PathBuf,
    },
    /// Compose multiple partial or retry runs into a unified qualification receipt
    Compose {
        /// Path to toolchain manifest JSON
        #[arg(long)]
        toolchain: PathBuf,

        /// Path to consumer manifest JSON
        #[arg(long)]
        consumer: PathBuf,

        /// Path to input run evidence JSON file (array of InputRunEvidence)
        #[arg(long)]
        runs: Option<PathBuf>,

        /// Paths to raw log files (used if --runs is not provided, generating default InputRunEvidence records)
        #[arg(long, num_args = 1..)]
        logs: Option<Vec<PathBuf>>,

        /// Output path for receipt JSON
        #[arg(long)]
        receipt: PathBuf,
    },
}

fn write_receipt_no_clobber(path: &Path, content: &str) -> Result<(), String> {
    use std::io::Write;
    let mut file =
        std::fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(|e| {
            if path.exists() {
                format!("refusing to overwrite existing immutable receipt at: {}", path.display())
            } else {
                format!("failed to create receipt at {}: {e}", path.display())
            }
        })?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("failed to write receipt to {}: {e}", path.display()))?;
    file.flush().map_err(|e| format!("failed to flush receipt to {}: {e}", path.display()))?;
    Ok(())
}

fn run() -> Result<ExitCode, String> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Mutations { toolchain, fixtures, kani_bin, receipt } => {
            let toolchain_content = fs::read_to_string(&toolchain).map_err(|e| {
                format!("failed to read toolchain manifest {}: {e}", toolchain.display())
            })?;

            let toolchain_manifest: model::ToolchainManifest =
                serde_json::from_str(&toolchain_content)
                    .map_err(|e| format!("failed to parse toolchain manifest JSON: {e}"))?;
            toolchain_manifest
                .validate()
                .map_err(|e| format!("invalid toolchain manifest {}: {e}", toolchain.display()))?;

            let toolchain_sha256 = mutations::hash_bytes(toolchain_content.as_bytes());

            let ctx = mutations::MutationContext {
                kani_bin,
                fixtures_dir: &fixtures,
                toolchain_path: &toolchain,
                toolchain_sha256,
                toolchain: toolchain_manifest,
            };

            println!("Running fail-closed red-mutation probes...");
            let mutation_receipt = mutations::run_all_mutations(&ctx)?;
            let json_out = serde_json::to_string_pretty(&mutation_receipt)
                .map_err(|e| format!("failed to serialize mutation receipt: {e}"))?;
            println!("{json_out}");

            if let Some(receipt_path) = receipt {
                write_receipt_no_clobber(&receipt_path, &json_out)?;
                println!("Saved mutation receipt to {}", receipt_path.display());
            }

            match mutation_receipt.status {
                model::GateStatus::Pass => {
                    println!("All red mutations successfully detected (gate passed).");
                    Ok(ExitCode::SUCCESS)
                }
                model::GateStatus::Fail => {
                    eprintln!("One or more red mutations were NOT detected (gate failed).");
                    Ok(ExitCode::FAILURE)
                }
            }
        }
        Commands::Parse { log } => {
            let content = fs::read_to_string(&log)
                .map_err(|e| format!("failed to read log {}: {e}", log.display()))?;

            let parsed = parser::parse_kani_output(&content)?;
            println!("Syntax parse successful: {} harnesses parsed", parsed.harnesses.len());
            for (name, h) in &parsed.harnesses {
                let unreachable = h.unreachable.unwrap_or(0);
                let covers = match h.covers {
                    Some(c) => format!("covers: {}/{}", c.satisfied, c.total),
                    None => "no covers".to_string(),
                };
                println!(" - {name}: {} (unreachable: {unreachable}, {covers})", h.verdict);
            }
            if let Some(s) = parsed.summary {
                println!(
                    "Summary: {} successful, {} failed, {} total",
                    s.successful, s.failed, s.total
                );
            }
            if !parsed.warnings.is_empty() {
                println!("Warnings ({}):", parsed.warnings.len());
                for w in &parsed.warnings {
                    println!(" - {w}");
                }
            }
            if !parsed.unsupported_constructs.is_empty() {
                println!("Unsupported constructs ({}):", parsed.unsupported_constructs.len());
                for (c, count) in &parsed.unsupported_constructs {
                    println!(" - {c} ({count})");
                }
            }

            // Parse subcommand is syntax-only: exits 0 if the log parses cleanly
            Ok(ExitCode::SUCCESS)
        }
        Commands::Compose { toolchain, consumer, runs, logs, receipt } => {
            let toolchain_content = fs::read_to_string(&toolchain).map_err(|e| {
                format!("failed to read toolchain manifest {}: {e}", toolchain.display())
            })?;
            let toolchain_manifest: model::ToolchainManifest =
                serde_json::from_str(&toolchain_content)
                    .map_err(|e| format!("failed to parse toolchain manifest JSON: {e}"))?;
            toolchain_manifest
                .validate()
                .map_err(|e| format!("invalid toolchain manifest {}: {e}", toolchain.display()))?;
            let toolchain_sha256 = mutations::hash_bytes(toolchain_content.as_bytes());

            let consumer_content = fs::read_to_string(&consumer).map_err(|e| {
                format!("failed to read consumer manifest {}: {e}", consumer.display())
            })?;
            let consumer_manifest: model::ConsumerManifest =
                serde_json::from_str(&consumer_content)
                    .map_err(|e| format!("failed to parse consumer manifest JSON: {e}"))?;
            consumer_manifest
                .validate()
                .map_err(|e| format!("invalid consumer manifest {}: {e}", consumer.display()))?;
            let consumer_sha256 = mutations::hash_bytes(consumer_content.as_bytes());

            let input_runs = match (runs, logs) {
                (Some(runs_path), _) => {
                    let runs_content = fs::read_to_string(&runs_path).map_err(|e| {
                        format!("failed to read runs JSON {}: {e}", runs_path.display())
                    })?;
                    let loaded: Vec<model::InputRunEvidence> = serde_json::from_str(&runs_content)
                        .map_err(|e| format!("failed to parse input run evidence JSON: {e}"))?;
                    loaded
                }
                (None, Some(log_paths)) => {
                    let mut generated = Vec::new();
                    for (idx, p) in log_paths.iter().enumerate() {
                        let bytes = fs::read(p)
                            .map_err(|e| format!("failed to read log file {}: {e}", p.display()))?;
                        let sha256 = mutations::hash_bytes(&bytes);
                        let run_id = p.file_stem().map_or_else(
                            || format!("run-{}", idx + 1),
                            |s| s.to_string_lossy().to_string(),
                        );
                        generated.push(model::InputRunEvidence {
                            run_id,
                            log_path: p.to_string_lossy().to_string(),
                            sha256,
                            argv: vec![
                                "cargo".to_string(),
                                "kani".to_string(),
                                "--exact".to_string(),
                                "--output-format=terse".to_string(),
                            ],
                            exit_code: 0,
                            terminating_signal: None,
                        });
                    }
                    generated
                }
                (None, None) => {
                    return Err("either --runs or --logs must be provided to compose".to_string());
                }
            };

            let composite = composer::compose_runs(
                &toolchain_manifest,
                &consumer_manifest,
                &toolchain_sha256,
                &consumer_sha256,
                &input_runs,
            )?;
            let json_out = serde_json::to_string_pretty(&composite)
                .map_err(|e| format!("failed to serialize composite receipt: {e}"))?;
            write_receipt_no_clobber(&receipt, &json_out)?;
            println!("Composite receipt written to {}", receipt.display());

            match composite.status {
                model::GateStatus::Pass => Ok(ExitCode::SUCCESS),
                model::GateStatus::Fail => Ok(ExitCode::FAILURE),
            }
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}

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
    /// Compose multiple partial or retry logs into a unified receipt
    Compose {
        /// Paths to log files
        #[arg(long, num_args = 1..)]
        logs: Vec<PathBuf>,

        /// Optional path to toolchain manifest to bind and validate against
        #[arg(long)]
        toolchain: Option<PathBuf>,

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

            let ctx = mutations::MutationContext {
                kani_bin,
                fixtures_dir: &fixtures,
                toolchain_path: &toolchain,
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
            println!("Parsed {} harnesses", parsed.harnesses.len());
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

            if parsed.is_pass() { Ok(ExitCode::SUCCESS) } else { Ok(ExitCode::FAILURE) }
        }
        Commands::Compose { logs, toolchain, receipt } => {
            if let Some(toolchain_path) = &toolchain {
                let toolchain_content = fs::read_to_string(toolchain_path).map_err(|e| {
                    format!("failed to read toolchain manifest {}: {e}", toolchain_path.display())
                })?;
                let toolchain_manifest: model::ToolchainManifest =
                    serde_json::from_str(&toolchain_content)
                        .map_err(|e| format!("failed to parse toolchain manifest JSON: {e}"))?;
                toolchain_manifest.validate().map_err(|e| {
                    format!("invalid toolchain manifest {}: {e}", toolchain_path.display())
                })?;
            }

            let log_refs: Vec<&Path> = logs.iter().map(|p| p.as_path()).collect();
            let composite = composer::compose_logs(&log_refs)?;
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

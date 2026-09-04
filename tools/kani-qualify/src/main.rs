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

        /// Output path for receipt JSON
        #[arg(long)]
        receipt: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Mutations { toolchain, fixtures, kani_bin, receipt } => {
            if !toolchain.exists() {
                eprintln!("Error: toolchain manifest not found at {}", toolchain.display());
                return ExitCode::FAILURE;
            }
            if !fixtures.exists() {
                eprintln!("Error: fixtures directory not found at {}", fixtures.display());
                return ExitCode::FAILURE;
            }

            let toolchain_content = match fs::read_to_string(&toolchain) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("Error reading toolchain manifest: {e}");
                    return ExitCode::FAILURE;
                }
            };

            let toolchain_manifest: model::ToolchainManifest =
                match serde_json::from_str(&toolchain_content) {
                    Ok(manifest) => manifest,
                    Err(e) => {
                        eprintln!("Error parsing toolchain manifest JSON: {e}");
                        return ExitCode::FAILURE;
                    }
                };

            let ctx = mutations::MutationContext {
                kani_bin,
                fixtures_dir: &fixtures,
                toolchain_path: &toolchain,
                toolchain: toolchain_manifest,
            };

            println!("Running fail-closed red-mutation probes...");
            let mutation_receipt = match mutations::run_all_mutations(&ctx) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Error running mutation probes: {e}");
                    return ExitCode::FAILURE;
                }
            };

            let json_out = serde_json::to_string_pretty(&mutation_receipt).unwrap();
            println!("{json_out}");

            if let Some(receipt_path) = receipt {
                if let Err(e) = fs::write(&receipt_path, &json_out) {
                    eprintln!("Error writing receipt to {}: {e}", receipt_path.display());
                    return ExitCode::FAILURE;
                }
                println!("Saved mutation receipt to {}", receipt_path.display());
            }

            if mutation_receipt.status == model::GateStatus::Pass {
                println!("All red mutations successfully detected (gate passed).");
                ExitCode::SUCCESS
            } else {
                eprintln!("One or more red mutations were NOT detected (gate failed).");
                ExitCode::FAILURE
            }
        }
        Commands::Parse { log } => {
            let content = match fs::read_to_string(&log) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error reading log file: {e}");
                    return ExitCode::FAILURE;
                }
            };

            match parser::parse_kani_output(&content) {
                Ok(parsed) => {
                    println!("Parsed {} harnesses", parsed.harnesses.len());
                    for (name, h) in &parsed.harnesses {
                        let status = if h.successful { "PASS" } else { "FAIL" };
                        let unreachable = h.unreachable.unwrap_or(0);
                        let covers = match (h.covers_satisfied, h.covers_total) {
                            (Some(s), Some(t)) => format!("covers: {s}/{t}"),
                            _ => "no covers".to_string(),
                        };
                        println!(" - {name}: {status} (unreachable: {unreachable}, {covers})");
                    }
                    if let Some((s, f, t)) = parsed.summary {
                        println!("Summary: {s} successful, {f} failed, {t} total");
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("Error parsing log: {e}");
                    return ExitCode::FAILURE;
                }
            }
        }
        Commands::Compose { logs, receipt } => {
            let log_refs: Vec<&Path> = logs.iter().map(|p| p.as_path()).collect();
            match composer::compose_logs(&log_refs) {
                Ok(composite) => {
                    let json_out = serde_json::to_string_pretty(&composite).unwrap();
                    if let Err(e) = fs::write(&receipt, &json_out) {
                        eprintln!("Error writing composite receipt: {e}");
                        return ExitCode::FAILURE;
                    }
                    println!("Composite receipt written to {}", receipt.display());
                    if composite.status == model::GateStatus::Pass {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::FAILURE
                    }
                }
                Err(e) => {
                    eprintln!("Error composing logs: {e}");
                    return ExitCode::FAILURE;
                }
            }
        }
    }
}

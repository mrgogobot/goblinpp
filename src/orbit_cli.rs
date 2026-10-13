//! The same CLI contract is shared by goblin++ orbit and goblin-orbit.
use crate::error::Result;
use crate::orbit::{self, BaselineOptions, Classification, Limits};
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct OrbitArgs {
    #[command(subcommand)]
    pub command: OrbitCommand,
}

#[derive(Debug, Subcommand)]
pub enum OrbitCommand {
    /// Verify a captured baseline and its immediate parent copy against an external hash.
    VerifyBaseline {
        directory: PathBuf,
        #[arg(long)]
        expected_baseline_sha256: String,
        #[arg(long)]
        json: bool,
    },
    /// Capture a NEW baseline outside the scope; does not establish known-good state.
    Baseline {
        root: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        trust_declaration: String,
        #[arg(long)]
        authorization: String,
        #[arg(long, requires = "expected_parent_sha256")]
        parent_baseline: Option<PathBuf>,
        #[arg(long, requires = "parent_baseline")]
        expected_parent_sha256: Option<String>,
        #[command(flatten)]
        limits: Limits,
        #[arg(long)]
        json: bool,
    },
    /// Observe only the explicit root; preserve a report even for incomplete coverage.
    Check {
        root: PathBuf,
        #[arg(long)]
        baseline: PathBuf,
        #[arg(long)]
        expected_baseline_sha256: String,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        author: String,
        #[arg(long)]
        authorization: String,
        #[command(flatten)]
        limits: Limits,
        #[arg(long)]
        json: bool,
    },
    /// Offline copy/consistency verification, NOT a new check of original files.
    Verify {
        directory: PathBuf,
        #[arg(long)]
        expected_baseline_sha256: String,
        #[arg(long)]
        expected_receipt_sha256: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

pub fn dispatch(args: OrbitArgs) -> Result<i32> {
    match args.command {
        OrbitCommand::VerifyBaseline {
            directory,
            expected_baseline_sha256,
            json,
        } => {
            let baseline = orbit::verify_baseline(&directory, &expected_baseline_sha256)?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({"verification":"PINNED_BASELINE", "parent_baseline_sha256":baseline.parent_baseline_sha256,"known_good_certified":false})
                );
            } else {
                println!(
                    "GOBLIN ORBIT BASELINE VERIFICATION\n\nVERIFICATION=PINNED_BASELINE\nKNOWN_GOOD_CERTIFIED=NO\n{}",
                    orbit::NOTICE
                );
            }
            Ok(0)
        }
        OrbitCommand::Baseline {
            root,
            output,
            reason,
            trust_declaration,
            authorization,
            parent_baseline,
            expected_parent_sha256,
            limits,
            json,
        } => {
            let hash = orbit::capture(
                &root,
                &output,
                &limits,
                BaselineOptions {
                    reason: &reason,
                    trust_declaration: &trust_declaration,
                    authorization: &authorization,
                    parent: parent_baseline
                        .as_deref()
                        .zip(expected_parent_sha256.as_deref()),
                },
            )?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({"status":"CAPTURED_NOT_CERTIFIED", "baseline":output.join("baseline.json"), "baseline_sha256":hash, "notice":orbit::NOTICE})
                );
            } else {
                println!(
                    "GOBLIN ORBIT BASELINE\n\nBASELINE={}\nBASELINE_SHA256={}\nSTATUS=CAPTURED_NOT_CERTIFIED\n\nKeep this hash separately, not beside a baseline an attacker could replace.\n{}",
                    output.join("baseline.json").display(),
                    hash,
                    orbit::NOTICE
                );
            }
            Ok(0)
        }
        OrbitCommand::Check {
            root,
            baseline,
            expected_baseline_sha256,
            output,
            author,
            authorization,
            limits,
            json,
        } => {
            let result = orbit::check(
                &root,
                &baseline,
                &expected_baseline_sha256,
                &output,
                &limits,
                &author,
                &authorization,
            )?;
            if json {
                println!("{}", serde_json::to_string(&result)?);
            } else {
                println!(
                    "GOBLIN ORBIT CHECK\n\nORBIT_STATUS={}\nCOVERAGE={}\nREPORT={}/report/report.html\nRECEIPT_CORE_SHA256={}\nAUTHENTICATION=CHECKSUM_ONLY",
                    result.check.status,
                    if result.check.complete {
                        "COMPLETE"
                    } else {
                        "INCOMPLETE"
                    },
                    result.directory.display(),
                    result.receipt_core_sha256
                );
                for row in &result.check.results {
                    if row.classification != Classification::Match {
                        println!("{:?}: {}", row.classification, row.path);
                        if matches!(
                            row.classification,
                            Classification::ContentChanged
                                | Classification::Missing
                                | Classification::Added
                        ) {
                            println!(
                                "Baseline mismatch at {}; possible tampering or legitimate change—review required.",
                                row.path
                            );
                        }
                    }
                }
                println!("\n{}", orbit::NOTICE);
            }
            Ok(match result.check.status.as_str() {
                "MATCH" => 0,
                "REVIEW_REQUIRED" => 1,
                _ => 2,
            })
        }
        OrbitCommand::Verify {
            directory,
            expected_baseline_sha256,
            expected_receipt_sha256,
            json,
        } => {
            let check = orbit::verify(
                &directory,
                &expected_baseline_sha256,
                expected_receipt_sha256.as_deref(),
            )?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({"verification":"CONSISTENT_COPIES", "orbit_status":check.status, "trusted_baseline_matched":true, "trusted_receipt_matched":expected_receipt_sha256.is_some(), "notice":orbit::NOTICE})
                );
            } else {
                println!(
                    "GOBLIN ORBIT VERIFICATION\n\nVERIFICATION=CONSISTENT_COPIES\nRECORDED_ORBIT_STATUS={}\nTRUSTED_BASELINE_MATCH=PASS\nTRUSTED_RECEIPT_MATCH={}\n\nNo original files were rehashed; observations are not authenticated unless separately anchored.\n{}",
                    check.status,
                    if expected_receipt_sha256.is_some() {
                        "PASS"
                    } else {
                        "NOT_REQUESTED"
                    },
                    orbit::NOTICE
                );
            }
            Ok(0)
        }
    }
}

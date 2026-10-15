//! CLI-only acquisition. GBL execution/checking/compilation do not invoke this command.
use crate::acquisition::{self, AcquisitionStatus, FetchRequest};
use crate::error::Result;
use clap::Args;
use std::path::{Path, PathBuf};

// No Debug: raw URL arguments are deliberately excluded from diagnostic formatting.
#[derive(Args)]
pub struct FetchArgs {
    /// Public HTTPS URL. Do not put credentials or signed-URL secrets on a command line.
    pub url: String,
    /// Explicit lowercase DNS hostname; HTTPS port 443 only, no redirects.
    #[arg(long)]
    pub allow_host: String,
    /// New, private directory; existing paths are never overwritten.
    #[arg(long)]
    pub output: PathBuf,
    /// Local basename, independent of remote Content-Disposition.
    #[arg(long, default_value = "input.fits")]
    pub filename: String,
    /// Expected content SHA-256 obtained independently; not server-provided trust.
    #[arg(
        long,
        conflicts_with = "record_only",
        required_unless_present = "record_only"
    )]
    pub sha256: Option<String>,
    /// Explicitly record an observed hash only; no externally anchored content verification.
    #[arg(long, conflicts_with = "sha256")]
    pub record_only: bool,
    #[arg(long, default_value_t = acquisition::DEFAULT_MAX_BYTES)]
    pub max_bytes: u64,
    #[arg(long, default_value_t = acquisition::DEFAULT_TIMEOUT_SECONDS)]
    pub timeout_seconds: u64,
    #[arg(long)]
    pub json: bool,
}

impl std::fmt::Debug for FetchArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FetchArgs")
            .field("url", &"OMITTED")
            .finish_non_exhaustive()
    }
}

pub fn dispatch(args: FetchArgs) -> Result<i32> {
    let request = FetchRequest {
        url: args.url,
        allow_host: args.allow_host,
        expected_sha256: args.sha256,
        record_only: args.record_only,
        filename: args.filename,
        max_bytes: args.max_bytes,
        timeout_seconds: args.timeout_seconds,
    };
    let receipt = acquisition::fetch(&request, &args.output)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&receipt)?);
    } else {
        println!(
            "GOBLIN HTTPS ACQUISITION\n\nACQUISITION_STATUS={}\nPAYLOAD={}\nPAYLOAD_BYTES={}\nPAYLOAD_SHA256={}\nEXPECTED_CONTENT_HASH={}\nRECEIPT_CORE_SHA256={}\n\nHTTPS transport and preserved bytes are not scientific validation or authenticated authorship. Review plaintext content before sharing.",
            status_name(receipt.status),
            args.output
                .canonicalize()?
                .join(&receipt.payload_path)
                .display(),
            receipt.payload_bytes,
            receipt.payload_sha256,
            if receipt.expected_sha256.is_some() {
                "VERIFIED"
            } else {
                "NOT_REQUESTED_RECORD_ONLY"
            },
            receipt.receipt_core_sha256
        );
    }
    Ok(0)
}

pub fn verify(directory: &Path, expected_receipt: Option<&str>, json: bool) -> Result<i32> {
    let receipt = acquisition::verify(directory, expected_receipt)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"verification_status":"PASS", "acquisition_status":receipt.status,
            "expected_receipt_anchor_matched":expected_receipt.is_some(), "receipt_core_sha256":receipt.receipt_core_sha256,
            "expected_source_hash_declared":receipt.expected_sha256.is_some(),
            "source_content_hash_verified":receipt.status == AcquisitionStatus::CompleteHashVerified, "offline":true})
            )?
        );
    } else {
        println!(
            "GOBLIN OFFLINE ACQUISITION VERIFICATION\n\nVERIFICATION_STATUS=PASS\nACQUISITION_STATUS={}\nRECEIPT_CORE_SHA256={}\nTRUSTED_RECEIPT_MATCH={}\n\nReceipt and payload bytes match. A verified failed/observed-only receipt does not become a hash-verified acquisition.",
            status_name(receipt.status),
            receipt.receipt_core_sha256,
            if expected_receipt.is_some() {
                "PASS"
            } else {
                "NOT_REQUESTED"
            }
        );
    }
    Ok(0)
}

fn status_name(status: AcquisitionStatus) -> &'static str {
    match status {
        AcquisitionStatus::CompleteHashVerified => "COMPLETE_HASH_VERIFIED",
        AcquisitionStatus::CompleteObservedOnly => "COMPLETE_OBSERVED_ONLY",
        AcquisitionStatus::Failed => "FAILED",
    }
}

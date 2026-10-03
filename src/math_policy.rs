use crate::error::Result;
use crate::hashing::sha256_file;
use serde_json::{Value, json};

pub fn policy() -> Value {
    json!({
        "schema": "goblin.math-policy.v1",
        "id": "goblin.rust-platform-finite-f64.v1",
        "backend": "rust-std-platform",
        "backend_version_source": "math_environment.launcher_build.rustc and execution.compiler.rustc",
        "cross_platform_bitwise_guarantee": false,
        "comparison_policy": crate::numeric_comparison::POLICY,
        "default_tolerance": null,
        "evidence_verification": "EXACT_SHA256"
    })
}

pub fn environment() -> Result<Value> {
    Ok(json!({
        "launcher_build": {
            "rustc": env!("GOBLIN_BUILD_RUSTC"),
            "target": env!("GOBLIN_BUILD_TARGET"),
            "profile": env!("GOBLIN_BUILD_PROFILE"),
            "target_features": env!("GOBLIN_BUILD_TARGET_FEATURES"),
        },
        "launcher_binary_sha256": sha256_file(std::env::current_exe()?)?,
        "native_compiler": null,
        "native_binary_sha256": null,
        "platform_math_library": "UNPINNED_PLATFORM_IMPLEMENTATION"
    }))
}

/// Legacy freezes have no math metadata. Do not retrospectively invent a pin.
pub fn frozen_policy_matches(receipt: &Value) -> bool {
    receipt
        .get("math_policy")
        .is_none_or(|frozen| *frozen == policy())
}

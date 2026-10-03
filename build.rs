use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    for key in ["RUSTC", "TARGET", "PROFILE", "CARGO_CFG_TARGET_FEATURE"] {
        println!("cargo:rerun-if-env-changed={key}");
    }
    let compiler = std::env::var_os("RUSTC").expect("Cargo must provide RUSTC");
    let result = Command::new(compiler)
        .args(["--version", "--verbose"])
        .output()
        .expect("Unable to record the Rust build identity");
    assert!(
        result.status.success(),
        "Unable to identify the Rust compiler"
    );
    let identity = String::from_utf8(result.stdout).expect("Rust identity must be UTF-8");
    println!(
        "cargo:rustc-env=GOBLIN_BUILD_RUSTC={}",
        identity.lines().next().unwrap()
    );
    for (key, output) in [
        ("TARGET", "GOBLIN_BUILD_TARGET"),
        ("PROFILE", "GOBLIN_BUILD_PROFILE"),
        ("CARGO_CFG_TARGET_FEATURE", "GOBLIN_BUILD_TARGET_FEATURES"),
    ] {
        println!(
            "cargo:rustc-env={output}={}",
            std::env::var(key).expect("Cargo build identity missing")
        );
    }
}

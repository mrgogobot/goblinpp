use goblinpp::{
    ai::{self, Method, Request, ResourceGuard},
    ai_cli,
    ai_hardware::{self, Cpu, Device, Hardware, MemoryLimit},
};
use std::{fs, process::Command};
use tempfile::tempdir;
const GIB: u64 = 1 << 30;
fn hardware() -> Hardware {
    Hardware {
        schema: ai_hardware::SCHEMA.into(),
        observed_utc: "2026-11-05T12:00:00Z".into(),
        os: "linux".into(),
        architecture: "x86_64".into(),
        cpu: Cpu {
            usable_logical_threads: 8,
            instruction_features: vec![],
        },
        total_ram_bytes: Some(8 * GIB),
        available_ram_bytes: Some(6 * GIB),
        memory_limits: vec![],
        available_disk_bytes: Some(16 * GIB),
        devices: vec![],
        observations: vec![],
    }
}
fn request() -> Request {
    serde_json::from_value(serde_json::json!({"schema":ai::REQUEST_SCHEMA,"model":"tiny fixture","device":"cpu","method":"lora","model_parameters":100,"trainable_parameters":10,"weight_bits":32,"micro_batch":2,"sequence_length":4,"gradient_accumulation":3,"activation_bytes_per_token":8,"attention_workspace_bytes_per_sequence":16,"runtime_workspace_bytes":64,"host_buffer_bytes":32,"additional_disk_bytes":100,"max_steps":3,"max_seconds":30,"max_threads":2})).unwrap()
}
fn gpu(unified: bool) -> Device {
    Device {
        id: "gpu0".into(),
        name: "fixture".into(),
        api: "metal".into(),
        unified_memory: unified,
        total_bytes: Some(8 * GIB),
        available_bytes: Some(6 * GIB),
        recommended_working_set_bytes: Some(5 * GIB),
        compute_capability: None,
        backend_execution_tested: false,
    }
}
#[test]
fn lora_hand_calculated_costs() {
    let p = ai::plan(&hardware(), &request()).unwrap();
    let e = p.core.estimate;
    assert_eq!(
        (
            e.base_weights_bytes,
            e.trainable_weights_and_master_bytes,
            e.gradients_bytes,
            e.adam_states_bytes
        ),
        (400, 40, 40, 80)
    );
    assert_eq!((e.activations_bytes, e.attention_workspace_bytes), (64, 32));
    assert_eq!(
        (e.training_bytes_with_headroom, e.host_bytes_with_headroom),
        (900, 940)
    );
    assert_eq!(
        (e.checkpoint_two_slots_bytes, e.disk_required_bytes),
        (8456, 8556)
    );
    assert_eq!(e.effective_batch_size, 6);
    assert_eq!(p.core.limits.threads, 2);
    assert_eq!(p.core.decision, "WITHIN_ESTIMATED_BUDGET");
    assert_eq!(p.core.training_execution, "NOT_IMPLEMENTED_NOT_STARTED");
}
#[test]
fn full_training_includes_master_weights_and_optimizer_checkpoints() {
    let mut r = request();
    r.method = Method::Full;
    r.trainable_parameters = 100;
    r.weight_bits = 16;
    let e = ai::plan(&hardware(), &r).unwrap().core.estimate;
    assert_eq!(e.training_bytes_with_headroom, 2450);
    assert_eq!(e.host_bytes_with_headroom, 2490);
    assert_eq!(e.disk_required_bytes, 11372);
}
#[test]
fn qlora_includes_explicit_quantization_overhead() {
    let mut r = request();
    r.method = Method::Qlora;
    r.weight_bits = 4;
    r.quantization_overhead_bytes = 10;
    let e = ai::plan(&hardware(), &r).unwrap().core.estimate;
    assert_eq!(e.training_bytes_with_headroom, 475);
    assert_eq!(e.host_bytes_with_headroom, 515);
    r.quantization_overhead_bytes = 0;
    assert!(ai::plan(&hardware(), &r).is_err());
}
#[test]
fn accumulation_and_checkpointing_do_not_invent_memory_savings() {
    let a = ai::plan(&hardware(), &request()).unwrap();
    let mut r = request();
    r.gradient_accumulation = 30;
    r.gradient_checkpointing = true;
    let b = ai::plan(&hardware(), &r).unwrap();
    assert_eq!(
        a.core.estimate.activations_bytes,
        b.core.estimate.activations_bytes
    );
    assert_eq!(
        a.core.estimate.host_bytes_with_headroom,
        b.core.estimate.host_bytes_with_headroom
    );
    assert_eq!(b.core.estimate.effective_batch_size, 60);
    assert_eq!(b.core.request, r);
}
#[test]
fn unified_memory_is_counted_once_and_both_caps_apply() {
    let mut h = hardware();
    h.os = "macos".into();
    h.architecture = "aarch64".into();
    h.devices.push(gpu(true));
    let mut r = request();
    r.device = "gpu0".into();
    r.max_memory_bytes = Some(940);
    r.max_device_bytes = Some(940);
    let p = ai::plan(&h, &r).unwrap();
    assert_eq!(p.core.estimate.shared_pool_peak_bytes, Some(940));
    assert_eq!(p.core.decision, "WITHIN_ESTIMATED_BUDGET");
    assert_eq!(p.core.backend_hint, "mlx-candidate");
    r.max_device_bytes = Some(939);
    assert_eq!(ai::plan(&h, &r).unwrap().core.decision, "REFUSED");
    r.max_device_bytes = None;
    r.reserve_device_bytes = 4 * GIB;
    assert_eq!(ai::plan(&h, &r).unwrap().core.decision, "REFUSED");
}
#[test]
fn unknown_vram_is_not_a_fit_and_no_device_fallback() {
    let mut h = hardware();
    let mut d = gpu(false);
    d.available_bytes = None;
    h.devices.push(d);
    let mut r = request();
    r.device = "gpu0".into();
    assert_eq!(
        ai::plan(&h, &r).unwrap().core.decision,
        "INSUFFICIENT_INFORMATION"
    );
    r.device = "missing".into();
    let p = ai::plan(&h, &r).unwrap();
    assert_eq!(p.core.decision, "REFUSED");
    assert_eq!(p.core.request.device, "missing");
}
#[test]
fn dedicated_gpu_and_host_budgets_are_independent() {
    let mut h = hardware();
    h.devices.push(gpu(false));
    let mut r = request();
    r.device = "gpu0".into();
    r.max_memory_bytes = Some(40);
    r.max_device_bytes = Some(900);
    assert_eq!(
        ai::plan(&h, &r).unwrap().core.decision,
        "WITHIN_ESTIMATED_BUDGET"
    );
    r.max_memory_bytes = Some(39);
    assert_eq!(ai::plan(&h, &r).unwrap().core.decision, "REFUSED");
}
#[test]
fn cgroup_total_and_remaining_capacity_constrain_ram() {
    let mut h = hardware();
    h.memory_limits.push(MemoryLimit {
        source: "parent".into(),
        limit_bytes: GIB,
        current_bytes: Some(GIB / 4),
    });
    let p = ai::plan(&h, &request()).unwrap();
    assert_eq!(p.core.limits.ram_bytes, Some(GIB / 2));
    h.memory_limits[0].current_bytes = None;
    assert_eq!(
        ai::plan(&h, &request()).unwrap().core.decision,
        "INSUFFICIENT_INFORMATION"
    );
}
#[test]
fn known_refusal_dominates_unknown_capacity() {
    let mut h = hardware();
    h.available_disk_bytes = None;
    let mut r = request();
    r.max_memory_bytes = Some(0);
    let p = ai::plan(&h, &r).unwrap();
    assert_eq!(p.core.decision, "REFUSED");
    assert!(
        p.core
            .reasons
            .contains(&"WORKSPACE_DISK_CAPACITY_UNKNOWN".into())
    );
    h.available_ram_bytes = None;
    assert_eq!(ai::plan(&h, &r).unwrap().core.decision, "REFUSED");
}
#[test]
fn cpu_feature_absence_is_not_a_backend_success_claim() {
    let mut h = hardware();
    h.cpu.usable_logical_threads = 1;
    let p = ai::plan(&h, &request()).unwrap();
    assert_eq!(p.core.limits.threads, 1);
    assert!(p.core.enforcement.contains("PREFLIGHT_ESTIMATE_ONLY"));
    h.cpu.usable_logical_threads = 0;
    assert_eq!(
        ai::plan(&h, &request()).unwrap().core.decision,
        "INSUFFICIENT_INFORMATION"
    );
}
#[test]
fn malformed_requests_and_overflow_refuse() {
    let h = hardware();
    for key in [
        "model_parameters",
        "trainable_parameters",
        "micro_batch",
        "sequence_length",
        "gradient_accumulation",
        "activation_bytes_per_token",
        "runtime_workspace_bytes",
        "max_steps",
        "max_seconds",
    ] {
        let mut v = serde_json::to_value(request()).unwrap();
        v[key] = 0.into();
        let r: Request = serde_json::from_value(v).unwrap();
        assert!(ai::plan(&h, &r).is_err(), "{key}");
    }
    let mut r = request();
    r.activation_bytes_per_token = ai_hardware::MAX_BYTES;
    assert!(ai::plan(&h, &r).is_err());
    r = request();
    r.max_device_bytes = Some(1);
    assert!(ai::plan(&h, &r).is_err());
    r = request();
    r.model = "bad\nmodel".into();
    assert!(ai::plan(&h, &r).is_err());
}
#[test]
fn strict_request_json_and_final_symlinks() {
    let d = tempdir().unwrap();
    let p = d.path().join("r.json");
    let raw = serde_json::to_string(&request()).unwrap();
    for bad in [
        raw.replace("{", "{\"schema\":\"duplicate\","),
        raw.replace("{", "{\"surprise\":1,"),
        raw.replace("\"micro_batch\":2", "\"micro_batch\":2.5"),
        " ".repeat(65_537),
    ] {
        fs::write(&p, bad).unwrap();
        assert!(ai::read_request(&p).is_err());
    }
    fs::write(&p, raw).unwrap();
    assert!(ai::read_request(&p).is_ok());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&p, d.path().join("link")).unwrap();
        assert!(ai::read_request(&d.path().join("link")).is_err());
    }
}
#[test]
fn policy_reconstruction_catches_resealed_arithmetic_and_decisions() {
    let original = ai::plan(&hardware(), &request()).unwrap();
    for change in 0..3 {
        let mut p = original.clone();
        match change {
            0 => p.core.estimate.disk_required_bytes += 1,
            1 => p.core.decision = "REFUSED".into(),
            _ => p.core.policy = "invented".into(),
        };
        p.core_sha256 = goblinpp::hashing::sha256_bytes(&serde_json::to_vec(&p.core).unwrap());
        assert!(ai::checked_report(&serde_json::to_string(&p).unwrap()).is_err());
    }
    assert!(ai::checked_report(&serde_json::to_string(&original).unwrap()).is_ok());
}
#[test]
fn bundles_verify_exact_inventory_anchor_and_refusal_without_overwriting() {
    let d = tempdir().unwrap();
    let p = d.path().join("plan");
    let mut r = request();
    r.max_memory_bytes = Some(0);
    let report = ai::plan(&hardware(), &r).unwrap();
    let raw = serde_json::to_vec(&r).unwrap();
    let hash = ai_cli::save_bundle(&p, &raw, &report).unwrap();
    assert_eq!(
        ai_cli::verify_bundle(&p, Some(&hash))
            .unwrap()
            .core
            .decision,
        "REFUSED"
    );
    assert!(ai_cli::verify_bundle(&p, Some(&"0".repeat(64))).is_err());
    assert!(ai_cli::save_bundle(&p, &raw, &report).is_err());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&p).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(p.join("request.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    fs::write(p.join("extra"), b"x").unwrap();
    assert!(ai_cli::verify_bundle(&p, None).is_err());
    fs::remove_file(p.join("extra")).unwrap();
    fs::write(p.join("plan.json"), b"{}").unwrap();
    assert!(ai_cli::verify_bundle(&p, None).is_err());
}
#[test]
fn cooperative_guard_accounts_before_work_and_preserves_failed_state() {
    let mut g = ResourceGuard::new(100, 2, 30).unwrap();
    g.reserve(90).unwrap();
    assert!(g.reserve(11).is_err());
    assert_eq!(g.live_bytes(), 90);
    assert!(g.release(91).is_err());
    assert_eq!(g.live_bytes(), 90);
    g.release(90).unwrap();
    g.step().unwrap();
    g.step().unwrap();
    assert!(g.step().is_err());
    assert_eq!(g.completed_steps(), 2);
    g.cancel();
    assert!(g.reserve(1).is_err());
    assert!(ResourceGuard::new(1, 0, 1).is_err());
}
#[test]
fn meminfo_and_visible_cgroup_ancestors_fail_closed() {
    assert_eq!(
        ai_hardware::meminfo("MemTotal: 100 kB\nMemAvailable: 40 kB"),
        Some((102400, 40960))
    );
    assert!(ai_hardware::meminfo("MemTotal: 100 kB\nMemAvailable: 101 kB").is_none());
    let d = tempdir().unwrap();
    fs::create_dir(d.path().join("child")).unwrap();
    for (folder, max, current) in [
        (d.path(), "1024", "512"),
        (d.path().join("child").as_path(), "max", "1"),
    ] {
        fs::write(folder.join("memory.max"), max).unwrap();
        fs::write(folder.join("memory.high"), "max").unwrap();
        fs::write(folder.join("memory.current"), current).unwrap();
    }
    let limits = ai_hardware::cgroup_limits(d.path(), std::path::Path::new("child")).unwrap();
    assert_eq!(limits.len(), 1);
    assert_eq!(limits[0].current_bytes, Some(512));
    assert!(ai_hardware::cgroup_limits(d.path(), std::path::Path::new("missing")).is_none());
    assert!(ai_hardware::cgroup_limits(d.path(), std::path::Path::new("../bad")).is_none());
    fs::remove_file(d.path().join("memory.current")).unwrap();
    assert_eq!(
        ai_hardware::cgroup_limits(d.path(), std::path::Path::new("child")).unwrap()[0]
            .current_bytes,
        None
    );
}
#[test]
fn observed_hardware_is_bounded_and_never_claims_backend_execution() {
    let d = tempdir().unwrap();
    let h = ai_hardware::probe(d.path()).unwrap();
    assert!(!h.devices.iter().any(|d| d.backend_execution_tested));
    assert!(h.cpu.usable_logical_threads > 0);
    ai_hardware::validate(&h).unwrap();
}
#[test]
fn new_names_preserve_alpha45_verification_grammar() {
    use goblinpp::parser::{self, SyntaxMode};
    let old=parser::evidence_mode(&serde_json::json!({"goblin_version":"0.1.0-alpha.45","parser_policy":parser::PARSER_POLICY})).unwrap();
    assert_eq!(old, SyntaxMode::PreAi);
    let s = "g_func ai_hardware(x) { return x + 1 }\na=ai_hardware(41)\nseal a";
    assert!(parser::parse_source_with_mode(s, old).is_ok());
    assert!(parser::parse_source(s).is_err());
}
#[test]
fn functions_require_private_mode_and_used_results() {
    assert!(ai::arity("ai_plan", 0).is_err());
    use goblinpp::parser::{parse_source, validate_execution};
    for s in ["x=ai_hardware()", "GO_PRIVATE\nai_hardware()"] {
        assert!(
            parse_source(s)
                .and_then(|p| validate_execution(&p.program))
                .is_err(),
            "{s}"
        );
    }
}
#[test]
fn real_cli_and_language_plans_are_offline_and_create_only_explicit_bundle() {
    let d = tempdir().unwrap();
    fs::write(
        d.path().join("request.json"),
        serde_json::to_vec(&request()).unwrap(),
    )
    .unwrap();
    fs::write(d.path().join("test.gbl"),"GO_PRIVATE\nhardware=ai_hardware()\nplan=ai_plan(\"request.json\")\nstatus=ai_plan_status(plan)\nprint(status)\n").unwrap();
    let bin = env!("CARGO_BIN_EXE_goblinpp");
    let o = Command::new(bin)
        .args(["run", "test.gbl"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(
        [
            "WITHIN_ESTIMATED_BUDGET",
            "REFUSED",
            "INSUFFICIENT_INFORMATION"
        ]
        .iter()
        .any(|decision| String::from_utf8_lossy(&o.stdout).contains(decision))
    );
    assert!(!d.path().join("runs").exists());
    assert!(!d.path().join(".goblin").exists());
    let o = Command::new(bin)
        .args(["ai", "plan", "request.json", "--output", "bundle"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(
        matches!(o.status.code(), Some(0..=2)),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(ai_cli::verify_bundle(&d.path().join("bundle"), None).is_ok());
}

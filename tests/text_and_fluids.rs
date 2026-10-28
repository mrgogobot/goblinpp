use goblinpp::{
    audit::verify_run,
    compiler::compile,
    evaluator::{Evaluation, Value},
    parser::parse_source,
    runtime::{RunOptions, read_receipt, run_file},
    text_runtime as text,
};
use std::{fs, process::Command};
use tempfile::tempdir;

fn eval(source: &str) -> Evaluation {
    Evaluation::new(".")
        .eval_program(&parse_source(source).unwrap().program)
        .unwrap()
}
fn q(e: &Evaluation, name: &str) -> (f64, [i32; 6]) {
    match e.env[name] {
        Value::Quantity(v) => (v.value_si, v.dimension),
        _ => panic!("quantity required"),
    }
}

#[test]
fn unicode_bytes_case_and_order_are_explicit() {
    let e = eval(
        "upper = str_upper(\"Straße π\")\nlower = str_lower(\"ADA Π\")\ncharacters = len(\"π\")\nbytes = str_bytes_len(\"π\")\nsize = sizeof(\"π\")\na = str_compare(\"Ada\",\"ada\")\nb = str_compare(\"π\",\"π\")\ncounter = str_compare(\"z\",\"a\")\n",
    );
    assert_eq!(e.env["upper"].render(), "STRASSE Π");
    assert_eq!(e.env["lower"].render(), "ada π");
    assert_eq!(q(&e, "characters").0, 1.0);
    assert_eq!(q(&e, "bytes").0, 2.0);
    assert_eq!(q(&e, "size").0, 2.0);
    assert_eq!(q(&e, "a").0, -1.0);
    assert_eq!(q(&e, "b").0, 0.0);
    assert_eq!(q(&e, "counter").0, 1.0);
    // Existing operators keep their historical meaning; ordering is explicit.
    let parsed = parse_source("bad = \"a\" < \"b\"\n").unwrap();
    assert!(Evaluation::new(".").eval_program(&parsed.program).is_err());
}

#[test]
fn text_encodings_roundtrip_utf8_and_refuse_invalid_bytes() {
    assert_eq!(text::byte_encoding("str_to_hex", "Aπ").unwrap(), "41cf80");
    assert_eq!(text::byte_encoding("str_to_octal", "A").unwrap(), "101");
    assert_eq!(
        text::byte_encoding("str_to_binary", "A").unwrap(),
        "01000001"
    );
    for (encoder, decoder) in [
        ("str_to_hex", "str_from_hex"),
        ("str_to_octal", "str_from_octal"),
        ("str_to_binary", "str_from_binary"),
    ] {
        for original in ["", "hello", "\0π你好🧌", "{literal}"] {
            let encoded = text::byte_encoding(encoder, original).unwrap();
            assert_eq!(text::byte_encoding(decoder, &encoded).unwrap(), original);
        }
    }
    for (name, bad) in [
        ("str_from_hex", "ff"),
        ("str_from_hex", "0"),
        ("str_from_hex", "+1"),
        ("str_from_hex", "0x41"),
        ("str_from_hex", "41 42"),
        ("str_from_hex", "π"),
        ("str_from_octal", "400"),
        ("str_from_binary", "00000002"),
    ] {
        assert!(text::byte_encoding(name, bad).is_err(), "{name} {bad}");
    }
}

#[test]
fn integer_radix_conversions_have_exact_range_and_no_implicit_coercion() {
    for base in [2.0, 8.0, 10.0, 16.0] {
        for n in [
            -9_007_199_254_740_991.0,
            -65.0,
            0.0,
            65.0,
            9_007_199_254_740_991.0,
        ] {
            assert_eq!(
                text::from_base(&text::to_base(n, base).unwrap(), base).unwrap(),
                n
            );
        }
    }
    assert_eq!(text::to_base(65.0, 16.0).unwrap(), "41");
    for n in [0.5, 9_007_199_254_740_992.0, f64::INFINITY, f64::NAN] {
        assert!(text::to_base(n, 16.0).is_err());
    }
    for (s, b) in [
        ("20000000000000", 16.0),
        ("0x41", 16.0),
        ("", 2.0),
        ("1.0", 10.0),
        ("2", 2.0),
        ("1_0", 10.0),
        ("1", 3.0),
    ] {
        assert!(text::from_base(s, b).is_err());
    }
}

#[test]
fn text_expansion_is_bounded_before_allocation() {
    assert!(text::change_case(&"ß".repeat(text::MAX_TEXT_BYTES / 2 + 1), true).is_err());
    assert!(text::change_case(&"İ".repeat(text::MAX_TEXT_BYTES / 3 + 1), false).is_err());
    for (name, width) in [("str_to_hex", 2), ("str_to_octal", 3), ("str_to_binary", 8)] {
        assert!(text::byte_encoding(name, &"a".repeat(text::MAX_TEXT_BYTES / width + 1)).is_err());
    }
}

#[test]
fn fluid_results_match_hand_calculated_expectations_and_dimensions() {
    let e = eval(
        r#"
rho = fluid_density(1 kg, 0.001 m^3)
mu = fluid_dynamic_viscosity(1e-6 m^2/s, rho)
nu = fluid_kinematic_viscosity(mu, rho)
flow = fluid_volume_flow(0.01 m^2, 2 m/s)
speed = fluid_flow_speed(flow, 0.01 m^2)
reynolds = fluid_reynolds(rho, speed, 0.01 m, mu)
pressure = fluid_dynamic_pressure(rho, speed)
total_pressure = fluid_bernoulli_total_pressure(100000 Pa, rho, speed)
pipe_drop = fluid_pipe_pressure_drop(rho, mu, 1 m, 0.01 m, 1e-6 m^3/s)
reverse = fluid_pipe_pressure_drop(rho, mu, 1 m, 0.01 m, -1e-6 m^3/s)
zero_drop = fluid_pipe_pressure_drop(rho, mu, 1 m, 0.01 m, 0 m^3/s)
"#,
    );
    for (name, expected, dim) in [
        ("rho", 1000.0, [1, -3, 0, 0, 0, 0]),
        ("mu", 0.001, [1, -1, -1, 0, 0, 0]),
        ("nu", 1e-6, [0, 2, -1, 0, 0, 0]),
        ("flow", 0.02, [0, 3, -1, 0, 0, 0]),
        ("speed", 2.0, [0, 1, -1, 0, 0, 0]),
        ("reynolds", 20000.0, [0; 6]),
        ("pressure", 2000.0, [1, -1, -2, 0, 0, 0]),
        ("total_pressure", 102000.0, [1, -1, -2, 0, 0, 0]),
        ("pipe_drop", 0.25464790894703254, [1, -1, -2, 0, 0, 0]),
        ("reverse", -0.25464790894703254, [1, -1, -2, 0, 0, 0]),
        ("zero_drop", 0.0, [1, -1, -2, 0, 0, 0]),
    ] {
        let (actual, d) = q(&e, name);
        assert!(
            (actual - expected).abs() <= 1e-12 * expected.abs().max(1.0),
            "{name} {actual}"
        );
        assert_eq!(d, dim);
    }
}

#[test]
fn incorrect_types_units_domains_and_discarded_results_are_refused() {
    for source in [
        "x=sizeof(1 kg)",
        "x=str_upper(1)",
        "x=str_compare(\"a\",1)",
        "x=to_base(1 kg,16)",
        "x=from_base(\"41\",16 m)",
        "x=str_upper()",
        "x=fluid_density(1 kg)",
        "str_lower(\"A\")",
        "fluid_density(1 kg,1 m^3)",
        "x=fluid_density(1 s,1 m^3)",
        "x=fluid_density(1 kg,0 m^3)",
        "x=fluid_density(-1 kg,1 m^3)",
        "x=fluid_reynolds(1000 kg/m^3,1 m/s,1 m,0 Pa*s)",
        "x=fluid_dynamic_pressure([1],2 m/s)",
        "x=fluid_pipe_pressure_drop(1000 kg/m^3,0.001 Pa*s,1 m,0.01 m,1 m^3/s)",
        "x=fluid_pipe_pressure_drop(1000 kg/m^3,0.001 Pa*s,1 m,0 m,1e-6 m^3/s)",
        "x=fluid_dynamic_pressure(1e300 kg/m^3,1e300 m/s)",
        "x=fluid_density(1e-300 kg,1e300 m^3)",
    ] {
        let p = parse_source(&format!("{source}\n")).unwrap();
        assert!(
            Evaluation::new(".").eval_program(&p.program).is_err(),
            "accepted {source}"
        );
    }
}

#[test]
fn both_examples_run_compile_and_verify_with_identical_seals_and_plaintext_output() {
    for example in ["text_toolkit.gbl", "fluid_mechanics.gbl"] {
        let temp = tempdir().unwrap();
        let path = temp.path().join(example);
        fs::copy(format!("examples/{example}"), &path).unwrap();
        let mut results = Vec::new();
        for compile in [false, true] {
            let run = run_file(
                &path,
                &RunOptions {
                    compile,
                    ..Default::default()
                },
            )
            .unwrap();
            let receipt = read_receipt(&run).unwrap();
            assert_eq!(
                receipt["status"], "PASS",
                "{example} compile={compile}: {:?}",
                receipt["failure"]
            );
            assert!(verify_run(&run).unwrap().verified);
            results.push((
                receipt["sealed_artifacts"].clone(),
                fs::read(run.join("stdout.log")).unwrap(),
            ));
        }
        assert_eq!(results[0], results[1]);
        // Exercise the standalone generated runtime, without the audited launcher.
        let parsed = parse_source(&fs::read_to_string(&path).unwrap()).unwrap();
        let compiled = compile(&parsed, temp.path().join("standalone"), &[]).unwrap();
        let output = Command::new(compiled.binary).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, results[0].1);
    }
}

#[test]
fn failures_are_preserved_and_verifiable_in_both_engines() {
    for body in [
        "x = str_from_hex(\"ff\")",
        "x = fluid_density(1 kg, 0 m^3)",
        "x = to_base(0.5,16)",
    ] {
        let t = tempdir().unwrap();
        let path = t.path().join("bad.gbl");
        fs::write(&path, format!("GO_PARANOID\n{body}\n")).unwrap();
        for compile in [false, true] {
            let run = run_file(
                &path,
                &RunOptions {
                    compile,
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(read_receipt(&run).unwrap()["status"], "MACHINERY_FAIL");
            assert!(verify_run(&run).unwrap().verified);
        }
    }
}

#[test]
fn standalone_native_refuses_malformed_text_and_nonlaminar_flow() {
    let temp = tempdir().unwrap();
    let parsed = parse_source(
        r#"
if argv(1) == "hex" {
    value = str_from_hex(argv(2))
    print(value)
} else {
    flow = parse_number(argv(2)) * (1 m^3/s)
    drop = fluid_pipe_pressure_drop(1000 kg/m^3, 0.001 Pa*s, 1 m, 0.01 m, flow)
    print(drop)
}
"#,
    )
    .unwrap();
    let binary = compile(&parsed, temp.path().join("standalone"), &[])
        .unwrap()
        .binary;
    for (mode, arg, succeeds) in [
        ("hex", "41cf80", true),
        ("hex", "ff", false),
        ("hex", "+1", false),
        ("pipe", "0.000001", true),
        ("pipe", "1", false),
    ] {
        let output = Command::new(&binary).args([mode, arg]).output().unwrap();
        assert_eq!(
            output.status.success(),
            succeeds,
            "{mode} {arg}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn pipe_reynolds_gate_and_dimension_diagnostics_are_explicit() {
    use goblinpp::fluid_runtime::{self as f, DENSITY, DYNAMIC_VISCOSITY, FLOW};
    let radius = 0.01;
    let threshold_flow = 2000.0 * 0.001 * std::f64::consts::PI * radius / (2.0 * 1000.0);
    for factor in [0.99, 1.01] {
        let result = f::call(
            "fluid_pipe_pressure_drop",
            &[
                (1000.0, DENSITY),
                (0.001, DYNAMIC_VISCOSITY),
                (1.0, [0, 1, 0, 0, 0, 0]),
                (radius, [0, 1, 0, 0, 0, 0]),
                (threshold_flow * factor, FLOW),
            ],
        );
        assert_eq!(result.is_ok(), factor < 1.0);
        if let Err(e) = result {
            assert!(e.message.contains("Reynolds number < 2000"));
        }
    }
    let bad = parse_source("bad = fluid_density(1 s, 1 m^3)\n").unwrap();
    let e = Evaluation::new(".").eval_program(&bad.program).unwrap_err();
    assert_eq!(e.code, "G201");
    assert!(e.message.contains("argument 1 (mass)"));
}

#[test]
fn historic_user_function_names_still_parse_but_new_execution_refuses_collisions() {
    for name in ["str_upper", "sizeof", "to_base", "fluid_density"] {
        let source = format!("g_func {name}(x) {{ return x }}\nresult = {name}(1)\n");
        let parsed = parse_source(&source).expect("historical source must remain canonicalizable");
        assert!(parsed.canonical_sha256().is_ok());
        let error = Evaluation::new(".")
            .eval_program(&parsed.program)
            .unwrap_err();
        assert!(error.message.contains("registered builtin function"));
        let parsed = parse_source(&format!("g_func f({name}) {{ return {name} }}\n")).unwrap();
        assert!(Evaluation::new(".").eval_program(&parsed.program).is_err());
    }
}

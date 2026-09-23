use goblinpp::audit::verify_run;
use goblinpp::evaluator::Evaluation;
use goblinpp::parser::parse_source;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use goblinpp::text_runtime;
use std::fs;
use tempfile::tempdir;

const EVERYDAY: &str = r#"GO_PARANOID
values = [1, 2, 3, 4, 5, 6]
sum = 0
visited = 0
for value in values {
    if value % 2 != 0 {
        continue
    }
    if value > 4 {
        break
    }
    sum = sum + value
    visited = visited + 1
}
short_and = false and missing_symbol == 1
short_or = true or missing_symbol == 1
inverted = not false
parsed = parse_integer(" -42 ")
remainder = -7 % 3
print("sum={sum}; visited={visited}; and={short_and}; or={short_or}; not={inverted}; parsed={parsed}; remainder={remainder}")
seal sum
seal visited
seal short_and
seal short_or
seal inverted
seal parsed
seal remainder
"#;

#[test]
fn alpha12_everyday_features_agree_across_engines_and_verify() {
    let root = tempdir().unwrap();
    let source = root.path().join("everyday.gbl");
    fs::write(&source, EVERYDAY).unwrap();
    for compile in [false, true] {
        let run = run_file(
            &source,
            &RunOptions {
                compile,
                ..RunOptions::default()
            },
        )
        .unwrap();
        let receipt = read_receipt(&run).unwrap();
        assert_eq!(
            receipt["status"], "PASS",
            "failure={:?}",
            receipt["failure"]
        );
        assert_eq!(
            fs::read_to_string(run.join("stdout.log")).unwrap(),
            "sum=6; visited=2; and=false; or=true; not=true; parsed=-42; remainder=-1\n"
        );
        assert!(verify_run(run).unwrap().verified);
    }
}

#[test]
fn direct_iteration_uses_an_independent_array_value() {
    let parsed = parse_source(
        "values = [1, 2, 3]\nsum = 0\nfor value in values {\n values[1] = 99\n sum = sum + value\n}\nseal sum\n",
    )
    .unwrap();
    let result = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    assert_eq!(result.sealed["sum"].render(), "6");
    assert_eq!(result.env["values"].render(), "[1, 99, 3]");
}

#[test]
fn nested_loop_control_applies_to_the_nearest_loop() {
    let parsed = parse_source(
        "count = 0\nfor outer in [1, 2] {\n for inner in [1, 2, 3] {\n  if inner == 2 { break }\n  count = count + 1\n }\n}\nseal count\n",
    )
    .unwrap();
    let result = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    assert_eq!(result.sealed["count"].render(), "2");
}

#[test]
fn logical_operators_require_booleans_and_foreach_requires_an_array() {
    for source in ["x = 1 and true\n", "x = not 1\n", "for x in 3 { y = x }\n"] {
        let parsed = parse_source(source).unwrap();
        let error = Evaluation::new(".")
            .eval_program(&parsed.program)
            .unwrap_err();
        assert_eq!(error.code, "G203", "source={source:?}; error={error:?}");
    }
}

#[test]
fn loop_control_outside_a_loop_is_rejected_during_parsing() {
    for source in ["break\n", "continue\n", "if true { break }\n"] {
        assert!(parse_source(source).is_err(), "accepted {source:?}");
    }
}

#[test]
fn checked_integer_and_remainder_inputs_are_refused() {
    for source in [
        "x = 2.5 % 2\n",
        "x = 2 kg % 2\n",
        "x = 2 % 0\n",
        "x = parse_integer(\"2.5\")\n",
        "x = parse_integer(\"9007199254740992\")\n",
    ] {
        let parsed = parse_source(source).unwrap();
        let error = Evaluation::new(".")
            .eval_program(&parsed.program)
            .unwrap_err();
        assert_eq!(error.code, "G202", "source={source:?}; error={error:?}");
    }
    assert_eq!(text_runtime::parse_integer("+0").unwrap(), 0.0);
    assert_eq!(
        text_runtime::parse_integer("9007199254740991").unwrap(),
        9_007_199_254_740_991.0
    );
}

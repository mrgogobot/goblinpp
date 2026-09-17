use goblinpp::audit::verify_run;
use goblinpp::custody::create_freeze;
use goblinpp::evaluator::Evaluation;
use goblinpp::interaction::InputPolicy;
use goblinpp::parser::parse_source;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use goblinpp::text_runtime::{self, MAX_TEXT_BYTES};
use std::fs;
use tempfile::tempdir;

const STRINGS: &str = r#"GO_PARANOID
g_func label(name, amount) {
    return str_trim(name) + ": " + to_text(amount)
}
name = "  Ada π  "
clean = str_trim(name)
count = len(clean)
found = str_contains(clean, "π")
changed = str_replace(clean, "π", "Lovelace")
parts = str_split("red,,blue", ",")
joined = str_join("/", parts)
number = parse_number("  +1.25e2  ")
answer = label("  count  ", number)
print("{clean}; len={count}; found={found}; changed={changed}; joined={joined}; answer={answer}")
seal clean
seal count
seal found
seal joined
seal number
seal answer
"#;

#[test]
fn g_strings_interpreted_and_compiled_runs_agree_and_verify() {
    let root = tempdir().unwrap();
    let source = root.path().join("strings.gbl");
    fs::write(&source, STRINGS).unwrap();
    create_freeze(&source).unwrap();
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
            "compile={compile}; failure={:?}",
            receipt["failure"]
        );
        assert_eq!(
            fs::read_to_string(run.join("stdout.log")).unwrap(),
            "Ada π; len=5; found=true; changed=Ada Lovelace; joined=red//blue; answer=count: 125\n"
        );
        assert!(verify_run(&run).unwrap().verified);
    }
}

#[test]
fn input_text_can_be_converted_to_number_in_both_modes() {
    let root = tempdir().unwrap();
    let source = root.path().join("number.gbl");
    fs::write(
        &source,
        r#"raw = input("Number: ")
value = parse_number(raw)
answer = value * 2
print("answer = {answer}")
seal answer
"#,
    )
    .unwrap();
    for compile in [false, true] {
        let run = run_file(
            &source,
            &RunOptions {
                compile,
                input_policy: InputPolicy::Provided(vec![" 2.5 ".into()]),
                ..RunOptions::default()
            },
        )
        .unwrap();
        assert_eq!(read_receipt(&run).unwrap()["status"], "PASS");
        assert_eq!(
            fs::read_to_string(run.join("stdout.log")).unwrap(),
            "answer = 5\n"
        );
        assert!(verify_run(&run).unwrap().verified);
    }
}

#[test]
fn unicode_length_counts_scalars_not_utf8_bytes_or_graphemes() {
    let parsed = parse_source("a = len(\"π\")\nb = len(\"é\")\nseal a\nseal b\n").unwrap();
    let result = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    assert_eq!(result.sealed["a"].render(), "1");
    assert_eq!(result.sealed["b"].render(), "2");
}

#[test]
fn invalid_numeric_text_is_refused_without_inventing_a_value() {
    for value in [
        "",
        "12 kg",
        "NaN",
        "inf",
        "1e9999",
        "1e-9999",
        "1_000",
        "1 2",
        "--1",
        "0x10",
        "1e",
        "9007199254740993",
    ] {
        let source = format!("x = parse_number({value:?})\n");
        let parsed = parse_source(&source).unwrap();
        let error = Evaluation::new(".")
            .eval_program(&parsed.program)
            .unwrap_err();
        assert_eq!(error.code, "G202", "value={value:?}; {error:?}");
    }
    let root = tempdir().unwrap();
    let source = root.path().join("bad.gbl");
    fs::write(&source, "x = parse_number(\"12 kg\")\n").unwrap();
    for compile in [false, true] {
        let run = run_file(
            &source,
            &RunOptions {
                compile,
                ..RunOptions::default()
            },
        )
        .unwrap();
        assert_eq!(read_receipt(&run).unwrap()["status"], "MACHINERY_FAIL");
        assert!(verify_run(&run).unwrap().verified);
    }
}

#[test]
fn wrong_text_types_and_empty_separators_fail() {
    for source in [
        "x = \"one\" + 2\n",
        "x = len(2)\n",
        "x = str_split(\"a,b\", \"\")\n",
        "x = str_replace(\"abc\", \"\", \"x\")\n",
        "x = str_join(\",\", [1, 2])\n",
        "x = to_text([1, 2])\n",
    ] {
        let parsed = parse_source(source).unwrap();
        assert!(
            Evaluation::new(".").eval_program(&parsed.program).is_err(),
            "accepted {source}"
        );
    }
}

#[test]
fn generated_text_and_split_parts_have_explicit_limits() {
    let max = "a".repeat(MAX_TEXT_BYTES);
    assert_eq!(
        text_runtime::concat(&max, "").unwrap().len(),
        MAX_TEXT_BYTES
    );
    assert!(text_runtime::concat(&max, "b").is_err());
    assert!(text_runtime::replace("a", "a", &max.repeat(2)).is_err());
    assert!(text_runtime::join("", &[max, "b".into()]).is_err());
    assert!(text_runtime::split(&"a,".repeat(100_000), ",").is_err());
}

use goblinpp::evaluator::Evaluation;
use goblinpp::parser::parse_source;

#[test]
fn accepts_the_python_007_normative_grammar_corpus() {
    let cases = [
        "",
        "\n\n",
        "# nothing to execute\n",
        "x = 1",
        "GO_PARANOID\n",
        "GO_PARANOID()\n",
        "x = 1\n",
        "x = 1\nseal x\n",
        "1 + 2\n",
        "x = .5\n",
        "x = 1.\n",
        "x = 6.022e23\n",
        "print(\"line\\nnext\")\n",
        "ω = 2\nΔ = ω + 1\n",
        "mass = 1 kg\n",
        "print()\n",
        "print(1, 2, 3)\n",
        "print(print(1))\n",
        "x = (1 + 2) * 3\n",
        "x = -+-2\n",
        "x = 2^3^2\n",
        "x = 2 pi\n",
        "x = 3 (2 + 1)\n",
        "x = c²\n",
        " x\t= 2 # value\nseal x # artifact\n",
        "x = 1\ny = x + 2\nprint(x, y)\nseal y\n",
    ];
    for source in cases {
        parse_source(source)
            .unwrap_or_else(|error| panic!("reference grammar case failed: {source:?}: {error}"));
    }
}

#[test]
fn rejects_the_python_007_normative_negative_grammar_corpus() {
    let cases = [
        "x = 1\r\n",
        "x = 1; y = 2\n",
        "print('x')\n",
        "é = 1\n",
        "x = c⁴\n",
        "x = 1 +\n",
        "x = (1 + 2\n",
        "seal\n",
        "GO_PARANOID(1)\n",
        "x =\n",
        "print(1,)\n",
        "x = 1 y = 2\n",
        "x = 1, 2\n",
        "x = 1)\n",
    ];
    for source in cases {
        assert!(
            parse_source(source).is_err(),
            "reference rejection was accepted: {source:?}"
        );
    }
}

#[test]
fn alpha4_comparisons_are_intentional_extensions_to_the_reference_grammar() {
    for source in [
        "x = 1 == 1\n",
        "x = 1 != 2\n",
        "x = 1 <= 2\n",
        "x = 1 >= 2\n",
    ] {
        assert!(
            parse_source(source).is_ok(),
            "new comparison rejected: {source:?}"
        );
    }
}

#[test]
fn canonical_equivalence_matches_the_python_007_corpus() {
    let equivalent = [
        ("x = c^2\n", "x = c²\n"),
        ("x = c^3\n", "x = c³\n"),
        ("x = pi\n", "x = π\n"),
        ("x = hbar\n", "x = ħ\n"),
        ("x = 2 * pi\n", "x = 2 pi\n"),
        ("x = 2\n", " # note\n x\t= 2 # same\n"),
        ("GO_PARANOID\n", "GO_PARANOID()\n"),
        ("x = 1\n", "x = 1.0\n"),
    ];
    for (left, right) in equivalent {
        assert_eq!(
            parse_source(left).unwrap().canonical_sha256().unwrap(),
            parse_source(right).unwrap().canonical_sha256().unwrap()
        );
    }
    let distinct = [
        ("x = 2^3^2\n", "x = (2^3)^2\n"),
        ("x = -2^2\n", "x = -(2^2)\n"),
        ("x = 1 + 2 + 3\n", "x = 1 + (2 + 3)\n"),
    ];
    for (left, right) in distinct {
        assert_ne!(
            parse_source(left).unwrap().canonical_sha256().unwrap(),
            parse_source(right).unwrap().canonical_sha256().unwrap()
        );
    }
}

#[test]
fn unicode_canonical_hashes_match_python_007_exactly() {
    let cases = [
        (
            "print(\"π\")\n",
            "e1a49d7c15e2a71f9100438cabc8bf34f514ccc1281a5570103b0814f8646920",
        ),
        (
            "ω = 2\nΔ = ω + 1\n",
            "bc9c9db1942c8517a21da3fd03f5a05839fe549c07df11916324db691fbdaefc",
        ),
        (
            "print(\"goblin 👺\")\n",
            "e1f4bc9482b71be428d8df450a3a71c3a07255a5b26374633eb530c4076135b9",
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(
            parse_source(source).unwrap().canonical_sha256().unwrap(),
            expected
        );
    }
}

#[test]
fn evaluates_the_python_007_normative_success_corpus() {
    let cases = [
        "",
        "x = 2.5\n",
        "x = \"goblin\"\n",
        "x = 500 g\n",
        "x = 1 kg + 500 g\n",
        "x = 2 km - 500 m\n",
        "mass = 1 kg\nenergy = mass * c^2\n",
        "x = 10 m / 2 s\n",
        "x = (2 m)^3\n",
        "x = 2^-2\n",
        "x = -+-2\n",
        "x = 1\nx = 2\n",
        "x = 1\nseal x\nx = 2\n",
        "GO_PARANOID\n",
        "print()\n",
        "print(2, 3 kg, \"ok\")\n",
        "x = 1500 m\nprint(\"x={x:.2e}\")\n",
        "print(\"area m² and π\")\n",
        "x = 2 kg\nprintf(\"%.1f kg\", x)\n",
        "x = print(\"hello\")\n",
        "label = \"trial\"\nseal label\n",
        "x = pi\ny = π\n",
        "c = 1\nx = c\n",
        "1 + 2\n",
    ];
    for source in cases {
        let parsed = parse_source(source).unwrap();
        Evaluation::new(".")
            .eval_program(&parsed.program)
            .unwrap_or_else(|error| panic!("reference semantic case failed: {source:?}: {error}"));
    }
    let formatted = Evaluation::new(".")
        .eval_program(
            &parse_source("x = 1500 m\nprint(\"x={x:.2e}\")\n")
                .unwrap()
                .program,
        )
        .unwrap();
    assert_eq!(formatted.stdout, ["x=1.50e+03 m"]);
}

#[test]
fn errors_match_the_python_007_normative_codes() {
    let cases = [
        ("x = missing\n", "G101"),
        ("mystery()\n", "G101"),
        ("seal missing\n", "G101"),
        ("x = 1 kg + 1 s\n", "G201"),
        ("x = 1 m - 1 K\n", "G201"),
        ("x = 2^0.5\n", "G201"),
        ("x = 2^(1 s)\n", "G201"),
        ("x = \"a\" + \"b\"\n", "G000"),
        ("x = -\"a\"\n", "G000"),
        ("printf()\n", "G000"),
        ("printf(1)\n", "G000"),
        ("printf(\"%d\")\n", "G000"),
        ("print(\"{missing}\")\n", "G101"),
        ("x = 1\nprint(\"{x:not-a-format}\")\n", "G000"),
        ("x = 1 / 0\n", "G202"),
        ("x = 0^-1\n", "G202"),
        ("x = 1e309\n", "G202"),
        ("x = 1e308 * 1e308\n", "G202"),
    ];
    for (source, code) in cases {
        let parsed = parse_source(source).unwrap();
        let error = Evaluation::new(".")
            .eval_program(&parsed.program)
            .unwrap_err();
        assert_eq!(error.code, code, "wrong error code for {source:?}: {error}");
    }
}

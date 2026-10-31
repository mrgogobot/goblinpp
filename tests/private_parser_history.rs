use goblinpp::parser::{self, SyntaxMode};
use serde_json::json;
#[test]
fn recorded_runtime_pins_keyword_additions_without_reinterpreting_old_sources() {
    let historical = parser::evidence_mode(
        &json!({"goblin_version":"0.1.0-alpha.42","parser_policy":parser::PARSER_POLICY}),
    )
    .unwrap();
    assert_eq!(historical, SyntaxMode::PrePrivate);
    for source in [
        "GO_PRIVATE = 2\nx = GO_PRIVATE\nseal x",
        "g_func encrypt_text(x) { return x }\nx=encrypt_text(2)\nseal x",
        "g_func f(input_secret) { return input_secret }\nx=f(2)\nseal x",
    ] {
        assert!(
            parser::parse_source_with_mode(source, historical).is_ok(),
            "{source}"
        );
        assert!(
            parser::parse_source(source)
                .and_then(|p| parser::validate_execution(&p.program))
                .is_err(),
            "{source}"
        );
    }
    assert_eq!(
        parser::evidence_mode(
            &json!({"goblin_version":"0.1.0-alpha.43","parser_policy":parser::PARSER_POLICY})
        )
        .unwrap(),
        SyntaxMode::Current
    );
}
#[test]
fn historical_final_state_and_loop_budget_keep_their_canonical_meaning() {
    let source = "COMPLETELY_MAD\nGO_LOOP_BUDGET 500\nx=2\nseal x";
    assert_eq!(
        parser::parse_source(source)
            .unwrap()
            .canonical_bytes()
            .unwrap(),
        parser::parse_source_with_mode(source, SyntaxMode::PrePrivate)
            .unwrap()
            .canonical_bytes()
            .unwrap()
    );
}
#[test]
fn historical_mode_is_verification_only_and_cannot_compile_private_features() {
    let parsed = parser::parse_source_with_mode("x=2\nseal x", SyntaxMode::PrePrivate).unwrap();
    let d = tempfile::tempdir().unwrap();
    assert!(goblinpp::compiler::compile(&parsed, d.path().join("no-binary"), &[]).is_err());
    let historical =
        parser::parse_source_with_mode("GO_PRIVATE\nprint(1)", SyntaxMode::PrePrivate).unwrap();
    assert!(!goblinpp::confidential::private(&historical.program));
}

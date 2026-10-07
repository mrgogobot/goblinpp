use goblinpp::evaluator::Value;
use goblinpp::matrix;
use goblinpp::quantity::{DIMENSIONLESS, Dimension, LENGTH, Quantity, TIME};
use goblinpp::random::{Randomness, verify_evidence};

fn number(value: f64) -> Value {
    Value::Quantity(Quantity::scalar(value).unwrap())
}

fn quantities(values: &[f64], dimension: Dimension) -> Value {
    Value::Array(
        values
            .iter()
            .map(|value| Value::Quantity(Quantity::new(*value, dimension).unwrap()))
            .collect(),
    )
}

fn numbers(values: &[f64]) -> Value {
    quantities(values, DIMENSIONLESS)
}

fn values(value: &Value) -> Vec<Quantity> {
    let Value::Array(items) = value else {
        panic!("expected array")
    };
    items
        .iter()
        .map(|item| item.quantity("test").unwrap())
        .collect()
}

fn numeric(value: &Value) -> Vec<f64> {
    values(value)
        .iter()
        .map(|quantity| quantity.value_si)
        .collect()
}

fn operation(name: &str, args: &[Value]) -> Value {
    matrix::call(name, args, &mut Randomness::default()).unwrap()
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:?}; expected={expected:?}; tolerance={tolerance}"
    );
}

fn seeded() -> Randomness {
    let mut rng = Randomness::default();
    rng.call("rng_seed", &[number(42.0), number(54.0)]).unwrap();
    rng
}

#[test]
fn flat_transpose_and_matrix_product_match_hand_calculation() {
    let left = quantities(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], LENGTH);
    let original = left.clone();
    let transposed = operation(
        "matrix_transpose",
        &[left.clone(), number(2.0), number(3.0)],
    );
    assert_eq!(numeric(&transposed), [1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
    assert_eq!(values(&transposed)[0].dimension, LENGTH);
    let right = quantities(&[7.0, 8.0, 9.0, 10.0, 11.0, 12.0], TIME);
    let product = operation(
        "matrix_multiply",
        &[
            left.clone(),
            number(2.0),
            number(3.0),
            right,
            number(3.0),
            number(2.0),
        ],
    );
    assert_eq!(numeric(&product), [58.0, 64.0, 139.0, 154.0]);
    assert!(
        values(&product)
            .iter()
            .all(|q| q.dimension == [0, 1, 1, 0, 0, 0])
    );
    assert_eq!(
        left, original,
        "matrix operations must not alias or mutate inputs"
    );
}

#[test]
fn matrix_product_preserves_cancellation_and_refuses_overflow() {
    let output = operation(
        "matrix_multiply",
        &[
            numbers(&[1e16, 1.0, -1e16]),
            number(1.0),
            number(3.0),
            numbers(&[1.0, 1.0, 1.0]),
            number(3.0),
            number(1.0),
        ],
    );
    assert_eq!(numeric(&output), [1.0]);
    assert!(
        matrix::call(
            "matrix_multiply",
            &[
                numbers(&[f64::MAX]),
                number(1.0),
                number(1.0),
                numbers(&[2.0]),
                number(1.0),
                number(1.0)
            ],
            &mut Randomness::default()
        )
        .is_err()
    );
    let exponent_max = quantities(&[1.0], [i32::MAX, 0, 0, 0, 0, 0]);
    let error = matrix::call(
        "matrix_multiply",
        &[
            exponent_max,
            number(1.0),
            number(1.0),
            quantities(&[1.0], [1, 0, 0, 0, 0, 0]),
            number(1.0),
            number(1.0),
        ],
        &mut Randomness::default(),
    )
    .unwrap_err();
    assert_eq!(error.code, "G201");
}

#[test]
fn shape_arity_type_and_resource_limits_are_explicit() {
    for (name, count) in [
        ("matrix_transpose", 3),
        ("matrix_multiply", 6),
        ("vector_norm", 1),
        ("vector_unit", 1),
        ("covariance", 4),
        ("cholesky", 3),
        ("mvnormal", 5),
    ] {
        assert!(matrix::is_function(name));
        assert!(matrix::require_arity(name, count).is_ok());
        assert!(matrix::require_arity(name, count + 1).is_err());
        assert!(matrix::call(name, &[], &mut Randomness::default()).is_err());
    }
    assert!(!matrix::is_function("matrix_magic"));
    let mut rng = Randomness::default();
    for args in [
        vec![numbers(&[1.0]), number(0.0), number(1.0)],
        vec![numbers(&[1.0]), number(1.5), number(1.0)],
        vec![numbers(&[1.0]), number(100_001.0), number(1.0)],
        vec![numbers(&[1.0]), number(100_000.0), number(2.0)],
        vec![numbers(&[1.0, 2.0]), number(1.0), number(1.0)],
        vec![Value::Text("not an array".into()), number(1.0), number(1.0)],
        vec![
            Value::Array(vec![Value::Bool(true)]),
            number(1.0),
            number(1.0),
        ],
        vec![
            Value::Array(vec![numbers(&[1.0])]),
            number(1.0),
            number(1.0),
        ],
        vec![numbers(&[1.0]), quantities(&[1.0], LENGTH), number(1.0)],
    ] {
        assert!(matrix::call("matrix_transpose", &args, &mut rng).is_err());
    }
    assert!(
        matrix::call(
            "matrix_multiply",
            &[
                numbers(&[1.0]),
                number(1.0),
                number(1.0),
                numbers(&[1.0, 2.0]),
                number(2.0),
                number(1.0)
            ],
            &mut rng
        )
        .is_err()
    );
    let mixed = Value::Array(vec![
        Value::Quantity(Quantity::new(1.0, LENGTH).unwrap()),
        Value::Quantity(Quantity::new(1.0, TIME).unwrap()),
    ]);
    assert_eq!(
        matrix::call("vector_unit", &[mixed], &mut rng)
            .unwrap_err()
            .code,
        "G201"
    );
    let nonfinite = Value::Array(vec![Value::Quantity(Quantity {
        value_si: f64::NAN,
        dimension: DIMENSIONLESS,
    })]);
    assert!(matrix::call("vector_unit", &[nonfinite], &mut rng).is_err());
}

#[test]
fn vector_magnitude_and_unit_are_scaled_dimension_checked_and_zero_safe() {
    let vector = quantities(&[3.0, 4.0], LENGTH);
    let norm = operation("vector_norm", std::slice::from_ref(&vector))
        .quantity("test")
        .unwrap();
    assert_eq!(norm, Quantity::new(5.0, LENGTH).unwrap());
    let unit = operation("vector_unit", &[vector]);
    let unit = values(&unit);
    assert!(unit.iter().all(|q| q.dimension == DIMENSIONLESS));
    near(unit[0].value_si, 0.6, 1e-15);
    near(unit[1].value_si, 0.8, 1e-15);
    let huge = numbers(&[f64::MAX, f64::MAX]);
    let unit = operation("vector_unit", std::slice::from_ref(&huge));
    near(numeric(&unit)[0], 1.0 / 2.0_f64.sqrt(), 1e-15);
    assert!(matrix::call("vector_norm", &[huge], &mut Randomness::default()).is_err());
    assert_eq!(
        operation("vector_norm", &[numbers(&[0.0, 0.0])])
            .quantity("test")
            .unwrap()
            .value_si,
        0.0
    );
    assert!(
        matrix::call(
            "vector_unit",
            &[numbers(&[0.0, 0.0])],
            &mut Randomness::default()
        )
        .is_err()
    );
    let smallest = f64::from_bits(1);
    assert_eq!(
        numeric(&operation("vector_unit", &[numbers(&[smallest, 0.0])])),
        [1.0, 0.0]
    );
    assert_eq!(
        operation("vector_norm", &[numbers(&[smallest, 0.0])])
            .quantity("test")
            .unwrap()
            .value_si,
        smallest
    );
}

#[test]
fn covariance_matches_known_sample_population_and_offset_fixtures() {
    // Rows (1,2), (2,4), (3,6): centered products sum to [[2,4],[4,8]].
    let samples = quantities(&[1.0, 2.0, 2.0, 4.0, 3.0, 6.0], LENGTH);
    let sample = operation(
        "covariance",
        &[samples.clone(), number(3.0), number(2.0), number(1.0)],
    );
    for (actual, expected) in numeric(&sample).iter().zip([1.0, 2.0, 2.0, 4.0]) {
        near(*actual, expected, 1e-14);
    }
    assert!(
        values(&sample)
            .iter()
            .all(|q| q.dimension == [0, 2, 0, 0, 0, 0])
    );
    let population = operation(
        "covariance",
        &[samples, number(3.0), number(2.0), number(0.0)],
    );
    for (actual, expected) in
        numeric(&population)
            .iter()
            .zip([2.0 / 3.0, 4.0 / 3.0, 4.0 / 3.0, 8.0 / 3.0])
    {
        near(*actual, expected, 1e-14);
    }
    let offset = operation(
        "covariance",
        &[
            numbers(&[1e15 + 1.0, 1e15 + 2.0, 1e15 + 3.0]),
            number(3.0),
            number(1.0),
            number(1.0),
        ],
    );
    assert_eq!(numeric(&offset), [1.0]);
    let opposite = operation(
        "covariance",
        &[
            numbers(&[1e150, -1e150]),
            number(2.0),
            number(1.0),
            number(0.0),
        ],
    );
    near(numeric(&opposite)[0] / 1e300, 1.0, 1e-15);
    let constant = operation(
        "covariance",
        &[
            numbers(&[7.0, 7.0, 7.0]),
            number(3.0),
            number(1.0),
            number(1.0),
        ],
    );
    assert_eq!(numeric(&constant), [0.0]);
}

#[test]
fn covariance_refuses_unsupported_ddof_shapes_and_overflow() {
    let mut rng = Randomness::default();
    for args in [
        vec![numbers(&[1.0]), number(1.0), number(1.0), number(1.0)],
        vec![numbers(&[1.0, 2.0]), number(2.0), number(1.0), number(2.0)],
        vec![numbers(&[1.0, 2.0]), number(2.0), number(1.0), number(0.5)],
        vec![numbers(&[1.0]), number(1.0), number(129.0), number(0.0)],
        vec![
            numbers(&[f64::MAX, -f64::MAX]),
            number(2.0),
            number(1.0),
            number(0.0),
        ],
    ] {
        assert!(matrix::call("covariance", &args, &mut rng).is_err());
    }
    assert_eq!(
        numeric(&operation(
            "covariance",
            &[numbers(&[8.0]), number(1.0), number(1.0), number(0.0)]
        )),
        [0.0]
    );
}

#[test]
fn cholesky_reconstructs_symmetric_positive_definite_matrix_and_dimensions() {
    let dimension = [0, 2, 0, 0, 0, 0];
    let input = quantities(&[4.0, 2.0, 2.0, 3.0], dimension);
    let lower = operation("cholesky", &[input.clone(), number(2.0), number(0.0)]);
    let values = values(&lower);
    near(values[0].value_si, 2.0, 1e-15);
    assert_eq!(values[1].value_si, 0.0);
    near(values[2].value_si, 1.0, 1e-15);
    near(values[3].value_si, 2.0_f64.sqrt(), 1e-15);
    assert!(values.iter().all(|q| q.dimension == LENGTH));
    let transposed = operation(
        "matrix_transpose",
        &[lower.clone(), number(2.0), number(2.0)],
    );
    let reconstruction = operation(
        "matrix_multiply",
        &[
            lower,
            number(2.0),
            number(2.0),
            transposed,
            number(2.0),
            number(2.0),
        ],
    );
    for (actual, expected) in numeric(&reconstruction).iter().zip(numeric(&input)) {
        near(*actual, expected, 2e-15);
    }
    let huge = operation("cholesky", &[numbers(&[1e300]), number(1.0), number(0.0)]);
    assert_eq!(numeric(&huge), [1e150]);
    let tiny = operation("cholesky", &[numbers(&[1e-300]), number(1.0), number(0.0)]);
    assert_eq!(numeric(&tiny), [1e-150]);
}

#[test]
fn three_dimensional_cholesky_has_known_factor_and_reconstruction() {
    // Hand construction from L = [[5,0,0],[3,3,0],[-1,1,3]].
    let covariance = numbers(&[25.0, 15.0, -5.0, 15.0, 18.0, 0.0, -5.0, 0.0, 11.0]);
    let lower = operation("cholesky", &[covariance.clone(), number(3.0), number(0.0)]);
    for (actual, expected) in numeric(&lower)
        .iter()
        .zip([5.0, 0.0, 0.0, 3.0, 3.0, 0.0, -1.0, 1.0, 3.0])
    {
        near(*actual, expected, 1e-14);
    }
    let transposed = operation(
        "matrix_transpose",
        &[lower.clone(), number(3.0), number(3.0)],
    );
    let reconstruction = operation(
        "matrix_multiply",
        &[
            lower,
            number(3.0),
            number(3.0),
            transposed,
            number(3.0),
            number(3.0),
        ],
    );
    for (actual, expected) in numeric(&reconstruction).iter().zip(numeric(&covariance)) {
        near(*actual, expected, 1e-13);
    }
}

#[test]
fn cholesky_refuses_asymmetry_indefinite_singular_and_odd_dimensions() {
    let mut rng = Randomness::default();
    for matrix_values in [
        [1.0, 0.25, 0.5, 1.0],
        [1.0, 2.0, 2.0, 1.0],
        [1.0, 1.0, 1.0, 1.0],
        [49.0, 21.0, 21.0, 9.0],
        [0.0, 0.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0, 1.0],
    ] {
        assert!(
            matrix::call(
                "cholesky",
                &[numbers(&matrix_values), number(2.0), number(0.0)],
                &mut rng
            )
            .is_err()
        );
    }
    assert!(
        matrix::call(
            "cholesky",
            &[quantities(&[1.0], LENGTH), number(1.0), number(0.0)],
            &mut rng
        )
        .is_err()
    );
    assert!(
        matrix::call(
            "cholesky",
            &[numbers(&[1.0]), number(1.0), number(-1.0)],
            &mut rng
        )
        .is_err()
    );
    assert!(
        matrix::call(
            "cholesky",
            &[numbers(&[1.0]), number(129.0), number(0.0)],
            &mut rng
        )
        .is_err()
    );
}

#[test]
fn declared_symmetry_tolerance_uses_lower_triangle_without_averaging() {
    let source = [4.0, 2.00000001, 2.0, 3.0];
    assert!(
        matrix::call(
            "cholesky",
            &[numbers(&source), number(2.0), number(0.0)],
            &mut Randomness::default()
        )
        .is_err()
    );
    let lower = operation("cholesky", &[numbers(&source), number(2.0), number(1e-8)]);
    near(numeric(&lower)[2], 1.0, 1e-15);
    let tiny = [1e-200, 1e-201, 2e-201, 1e-200];
    assert!(
        matrix::call(
            "cholesky",
            &[numbers(&tiny), number(2.0), number(1e-8)],
            &mut Randomness::default()
        )
        .is_err(),
        "relative tolerance must not accept tiny asymmetry via an absolute 1.0 floor"
    );
}

#[test]
fn mvnormal_consumes_declared_stream_replays_and_preserves_units() {
    let args = [
        quantities(&[10.0, 20.0], LENGTH),
        quantities(&[4.0, 2.0, 2.0, 3.0], [0, 2, 0, 0, 0, 0]),
        number(2.0),
        number(0.0),
        number(54.0),
    ];
    let mut first = seeded();
    let mut second = seeded();
    let a = matrix::call("mvnormal", &args, &mut first).unwrap();
    let b = matrix::call("mvnormal", &args, &mut second).unwrap();
    assert_eq!(a, b);
    assert_eq!(first.evidence(), second.evidence());
    assert!(
        values(&a)
            .iter()
            .all(|q| q.dimension == LENGTH && q.value_si.is_finite())
    );
    // Independently derive the first normal from PCG's published reference
    // words, the specified alpha.26 uniform mapping, and Box-Muller v1.
    let uniform =
        |high: u64, low: u64| (((high >> 5) << 26) | (low >> 6)) as f64 / 9_007_199_254_740_992.0;
    let first_uniform = uniform(0xa15c02b7, 0x7b47f409);
    let second_uniform = uniform(0xba1d3330, 0x83d2f293);
    let first_normal = (-2.0 * (1.0 - first_uniform).ln()).sqrt()
        * (2.0 * std::f64::consts::PI * second_uniform).cos();
    near(numeric(&a)[0], 10.0 + 2.0 * first_normal, 1e-14);
    let evidence = first.evidence();
    assert_eq!(evidence.streams[0].operations, 4);
    assert_eq!(evidence.streams[0].raw_words, 8);
    verify_evidence(&serde_json::to_value(evidence).unwrap()).unwrap();
}

#[test]
fn mvnormal_invalid_inputs_never_advance_seeded_rng() {
    let mut rng = seeded();
    let initial = rng.evidence();
    for args in [
        vec![
            numbers(&[1.0, 2.0]),
            numbers(&[1.0, 1.0, 1.0, 1.0]),
            number(2.0),
            number(0.0),
            number(54.0),
        ],
        vec![
            numbers(&[1.0, 2.0]),
            numbers(&[1.0, 0.0, 0.0, 1.0]),
            number(2.0),
            number(0.0),
            number(99.0),
        ],
        vec![
            quantities(&[1.0, 2.0], LENGTH),
            numbers(&[1.0, 0.0, 0.0, 1.0]),
            number(2.0),
            number(0.0),
            number(54.0),
        ],
        vec![
            numbers(&[1.0]),
            numbers(&[1.0, 0.0, 0.0, 1.0]),
            number(2.0),
            number(0.0),
            number(54.0),
        ],
        vec![
            numbers(&[1.0]),
            numbers(&[1.0]),
            number(1.0),
            number(-1.0),
            number(54.0),
        ],
        vec![
            numbers(&[1.0]),
            numbers(&[1.0]),
            number(1.0),
            number(0.0),
            number(54.5),
        ],
    ] {
        assert!(matrix::call("mvnormal", &args, &mut rng).is_err());
        assert_eq!(rng.evidence(), initial);
    }
}

#[test]
fn published_matrix_guide_code_blocks_execute_as_written() {
    let guide = include_str!("../docs/MATRICES_COVARIANCE.md");
    let blocks: Vec<_> = guide
        .split("```goblin\n")
        .skip(1)
        .map(|block| block.split("\n```").next().unwrap())
        .collect();
    assert_eq!(blocks.len(), 5);
    for (index, source) in blocks.iter().enumerate() {
        let program = goblinpp::parser::parse_source(source)
            .unwrap_or_else(|error| panic!("guide block {index} parse failure: {error}"));
        goblinpp::evaluator::Evaluation::new(".")
            .eval_program(&program.program)
            .unwrap_or_else(|error| panic!("guide block {index} execution failure: {error}"));
    }
}

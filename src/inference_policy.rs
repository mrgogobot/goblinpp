//! Scientific helper contract and bounded execution summaries, not a statistical verdict.
use crate::evaluator::Value;
use crate::hashing::sha256_bytes;
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub fn policy() -> Json {
    json!({"id":"goblin.inference.v1",
        "sampling":"replacement; explicit pre-seeded stream; empirical sample size unchanged for bootstrap",
        "normal_transform":"Box-Muller cosine; two uniforms per scalar; no cached spare",
        "normal_portability":"platform transcendental last bits may differ; exact seals preserved",
        "confidence_interval":"none implicit; choose quantile probabilities explicitly",
        "limits":{"maximum_array_items":crate::resampling::MAX_ITEMS,"maximum_bootstrap_selections":crate::resampling::MAX_BOOTSTRAP_SELECTIONS,"maximum_matrix_entries":crate::matrix::MAX_ENTRIES,"maximum_covariance_dimension":crate::matrix::MAX_COVARIANCE_DIMENSION,"maximum_matrix_scalar_products":crate::matrix::MAX_MULTIPLY_WORK},
        "covariance":"explicit ddof 0 or 1; homogeneous units; flat row-major",
        "cholesky":"explicit relative symmetry tolerance; declared lower triangle; no jitter or repair",
        "cholesky_pivot_guard":"scaled pivot > 16*f64::EPSILON*n*max(abs(scaled diagonal),abs(product sum)); near-singular refusal, never repair",
        "trace":"length-framed serde JSON typed argument/result SHA256 streams by function; final argument summaries; not mathematical replay"})
}

pub fn requires_policy(receipt: &Json) -> bool {
    receipt["goblin_version"].as_str().is_some_and(|v| {
        !v.starts_with("0.0.")
            && v.strip_prefix("0.1.0-alpha.")
                .is_none_or(|n| n.parse::<u64>().is_ok_and(|n| n >= 27))
    })
}

pub fn frozen_policy_matches(receipt: &Json) -> bool {
    receipt
        .get("inference_policy")
        .map_or(!requires_policy(receipt), |p| *p == policy())
}

#[derive(Debug, Default)]
struct Entry {
    calls: u64,
    inputs: Sha256,
    outputs: Sha256,
    last_arguments: Vec<Json>,
}

#[derive(Debug, Default)]
pub struct Trace(BTreeMap<String, Entry>);

fn encoded(value: &Value) -> Vec<u8> {
    // Value serialization cannot contain nonfinite numbers: scientific calls validate these.
    serde_json::to_vec(&value.to_json()).expect("typed value JSON serializes")
}

fn update(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
}

impl Trace {
    pub fn record(&mut self, name: &str, args: &[Value], result: &Value) {
        let entry = self.0.entry(name.into()).or_default();
        entry.calls += 1;
        let input = serde_json::to_vec(&args.iter().map(Value::to_json).collect::<Vec<_>>())
            .expect("typed arguments serialize");
        update(&mut entry.inputs, &input);
        update(&mut entry.outputs, &encoded(result));
        entry.last_arguments = args.iter().map(|v| match v {
            Value::Array(values) => json!({"kind":"array", "items":values.len(), "sha256":sha256_bytes(&encoded(v))}),
            _ => v.to_json(),
        }).collect();
    }

    pub fn evidence(&self) -> Json {
        json!({"schema":"goblin.inference-trace.v1", "functions":self.0.iter().map(|(name,e)|
            json!({"function":name,"calls":e.calls,"arguments_sha256":format!("{:x}",e.inputs.clone().finalize()),
                "results_sha256":format!("{:x}",e.outputs.clone().finalize()),"last_arguments":e.last_arguments})).collect::<Vec<_>>()})
    }
}

pub fn validate_evidence(value: &Json) -> bool {
    if value["schema"] != "goblin.inference-trace.v1" {
        return false;
    }
    let Some(entries) = value["functions"].as_array() else {
        return false;
    };
    let mut names = std::collections::BTreeSet::new();
    entries.len() <= 13
        && entries.iter().all(|entry| {
            let Some(name) = entry["function"].as_str() else {
                return false;
            };
            (crate::resampling::is_function(name) || crate::matrix::is_function(name))
                && names.insert(name)
                && entry["calls"].as_u64().is_some_and(|n| n > 0)
                && entry["last_arguments"].is_array()
                && ["arguments_sha256", "results_sha256"].iter().all(|key| {
                    entry[*key].as_str().is_some_and(|s| {
                        s.len() == 64
                            && s.bytes()
                                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    })
                })
        })
}

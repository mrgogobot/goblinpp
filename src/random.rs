//! Versioned scientific randomness, NOT cryptographic randomness.
//! PCG32 XSH-RR setseq follows M. E. O'Neill's PCG reference (Apache-2.0).
//! See docs/RANDOMNESS.md and third-party/licenses/pcg/NOTICE.md.
use crate::error::{GoblinError, Result};
use crate::evaluator::Value;
use crate::quantity::{DIMENSIONLESS, Quantity};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const POLICY: &str = "goblin.pcg32-xsh-rr-setseq.v1";
pub const MAX_STREAMS: usize = 128;
pub const MAX_OPERATIONS: u64 = 1_000_000;
pub const MAX_WORDS: u64 = 8_000_000;
pub const MAX_SEGMENTS: usize = 4096;
const MAX_SAFE: f64 = 9_007_199_254_740_991.0;
const MULTIPLIER: u64 = 6_364_136_223_846_793_005;

pub fn is_function(name: &str) -> bool {
    matches!(
        name,
        "rng_seed" | "rng_word" | "rng_uniform" | "rng_integer"
    )
}

pub fn require_arity(name: &str, count: usize) -> Result<()> {
    let valid = match name {
        "rng_seed" => (1..=2).contains(&count),
        "rng_word" | "rng_uniform" => count <= 1,
        "rng_integer" => (2..=3).contains(&count),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(error(format!(
            "Invalid argument count for {name}(). See RANDOMNESS.md."
        )))
    }
}

pub fn policy() -> serde_json::Value {
    json!({"id": POLICY, "algorithm": "PCG32-XSH-RR-64-32-setseq",
        "implementation": "goblin-pcg32-v1", "seeding": "pcg-setseq-init-v1",
        "seed_encoding": "canonical unsigned decimal u64; stream u63; default stream 0",
        "word_encoding": "u32 little-endian bytes for SHA-256",
        "uniform_mapping": "((word1 >> 5) * 2^26 + (word2 >> 6)) / 2^53; [0,1)",
        "integer_mapping": "u64 high-word-first rejection modulo width; [low,high)",
        "result_encoding": "word:u32LE; uniform:f64-bitsLE; integer:i64LE",
        "explicit_seed_required": true, "reseed_allowed": false, "cryptographic": false,
        "max_streams": MAX_STREAMS, "max_operations": MAX_OPERATIONS,
        "max_raw_words": MAX_WORDS, "max_operation_segments": MAX_SEGMENTS})
}

pub fn requires_policy(receipt: &serde_json::Value) -> bool {
    receipt["goblin_version"].as_str().is_some_and(|v| {
        if let Some(n) = v.strip_prefix("0.1.0-alpha.") {
            n.parse::<u64>().is_ok_and(|n| n >= 26)
        } else {
            true
        }
    })
}

pub fn frozen_policy_matches(receipt: &serde_json::Value) -> bool {
    receipt
        .get("rng_policy")
        .map_or(!requires_policy(receipt), |p| *p == policy())
}

fn error(message: impl Into<String>) -> GoblinError {
    GoblinError::new("G204", message)
}

fn unsigned(value: &Value, maximum: u64, context: &str) -> Result<u64> {
    let n = match value {
        Value::Text(s)
            if !s.is_empty()
                && s.bytes().all(|b| b.is_ascii_digit())
                && (s == "0" || !s.starts_with('0')) =>
        {
            s.parse::<u64>().ok()
        }
        Value::Quantity(q)
            if q.dimension == DIMENSIONLESS
                && q.value_si.is_finite()
                && q.value_si >= 0.0
                && q.value_si <= MAX_SAFE
                && q.value_si.fract() == 0.0 =>
        {
            Some(q.value_si as u64)
        }
        _ => None,
    };
    n.filter(|n| *n <= maximum).ok_or_else(|| error(format!("{context} requires an unsigned exact integer: numeric values up to 2^53-1, or canonical decimal text up to {maximum}.")))
}

fn bound(value: &Value) -> Result<i64> {
    match value {
        Value::Quantity(q)
            if q.dimension == DIMENSIONLESS
                && q.value_si.is_finite()
                && q.value_si.abs() <= MAX_SAFE
                && q.value_si.fract() == 0.0 =>
        {
            Ok(q.value_si as i64)
        }
        _ => Err(error(
            "rng_integer bounds must be dimensionless numeric integers within +/- (2^53-1).",
        )),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Segment {
    pub operation: String,
    pub low: Option<i64>,
    pub high: Option<i64>,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StreamEvidence {
    pub seed: String,
    pub stream: String,
    pub operations: u64,
    pub raw_words: u64,
    pub final_state_hex: String,
    pub raw_words_sha256: String,
    pub results_sha256: String,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub schema: String,
    pub policy: String,
    pub usage: String,
    pub streams: Vec<StreamEvidence>,
}

#[derive(Debug, Clone)]
struct Stream {
    seed: u64,
    id: u64,
    state: u64,
    operations: u64,
    words: u64,
    raw_hash: Sha256,
    result_hash: Sha256,
    segments: Vec<Segment>,
}

impl Stream {
    fn new(seed: u64, id: u64) -> Self {
        let mut s = Self {
            seed,
            id,
            state: 0,
            operations: 0,
            words: 0,
            raw_hash: Sha256::new(),
            result_hash: Sha256::new(),
            segments: Vec::new(),
        };
        s.step();
        s.state = s.state.wrapping_add(seed);
        s.step();
        s
    }

    fn step(&mut self) -> u32 {
        let previous = self.state;
        self.state = previous
            .wrapping_mul(MULTIPLIER)
            .wrapping_add((self.id << 1) | 1);
        let shifted = (((previous >> 18) ^ previous) >> 27) as u32;
        shifted.rotate_right((previous >> 59) as u32)
    }

    fn word(&mut self) -> u32 {
        let word = self.step();
        self.words += 1;
        self.raw_hash.update(word.to_le_bytes());
        word
    }

    fn snapshot(&self) -> StreamEvidence {
        StreamEvidence {
            seed: self.seed.to_string(),
            stream: self.id.to_string(),
            operations: self.operations,
            raw_words: self.words,
            final_state_hex: format!("{:016x}", self.state),
            raw_words_sha256: format!("{:x}", self.raw_hash.clone().finalize()),
            results_sha256: format!("{:x}", self.result_hash.clone().finalize()),
            segments: self.segments.clone(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Randomness {
    streams: BTreeMap<u64, Stream>,
    operations: u64,
    words: u64,
    segments: usize,
}

impl Randomness {
    /// Validate a required, explicitly seeded stream without consuming it.
    pub fn validate_stream(&self, id: u64) -> Result<()> {
        if self.streams.contains_key(&id) {
            Ok(())
        } else {
            Err(error(format!(
                "RNG stream {id} is unseeded. Call rng_seed(seed, stream) before drawing."
            )))
        }
    }

    /// Typed wrapper retaining the alpha.26 uniform mapping and evidence.
    pub fn draw_uniform(&mut self, id: u64) -> Result<f64> {
        self.draw(id, "rng_uniform", None, None)
    }

    /// Unbiased zero-based sample index, retaining alpha.26 integer evidence.
    pub fn draw_index(&mut self, upper: usize, id: u64) -> Result<usize> {
        if upper == 0 || upper as u128 > MAX_SAFE as u128 {
            return Err(error(
                "Sample index upper bound must be positive and within the exact integer range.",
            ));
        }
        Ok(self.draw(id, "rng_integer", Some(0), Some(upper as i64))? as usize)
    }

    pub fn evidence(&self) -> Evidence {
        Evidence {
            schema: "goblin.rng-evidence.v1".into(),
            policy: POLICY.into(),
            usage: if self.streams.is_empty() {
                "NOT_USED"
            } else if self.operations == 0 {
                "INITIALIZED_NO_DRAWS"
            } else {
                "USED"
            }
            .into(),
            streams: self.streams.values().map(Stream::snapshot).collect(),
        }
    }

    fn seed(&mut self, seed: u64, id: u64) -> Result<()> {
        if self.streams.contains_key(&id) {
            return Err(error(format!(
                "RNG stream {id} is already seeded. Reseeding is refused; use a new stream ID or a new run."
            )));
        }
        if self.streams.len() >= MAX_STREAMS {
            return Err(error("RNG stream limit exceeded."));
        }
        self.streams.insert(id, Stream::new(seed, id));
        Ok(())
    }

    fn draw(
        &mut self,
        id: u64,
        operation: &str,
        low: Option<i64>,
        high: Option<i64>,
    ) -> Result<f64> {
        if self.operations >= MAX_OPERATIONS {
            return Err(error("RNG operation budget exceeded."));
        }
        let s = self.streams.get_mut(&id).ok_or_else(|| {
            error(format!(
                "RNG stream {id} is unseeded. Call rng_seed(seed, stream) before drawing."
            ))
        })?;
        let extends = s.segments.last().is_some_and(|last| {
            last.operation == operation && last.low == low && last.high == high
        });
        if !extends && self.segments >= MAX_SEGMENTS {
            return Err(error("RNG evidence segment budget exceeded."));
        }
        // Reserve a bounded rejection allowance before mutating the state.
        if self.words > MAX_WORDS - 128 {
            return Err(error("RNG raw-word budget exceeded."));
        }
        let before = s.words;
        let state_before = s.state;
        let hash_before = s.raw_hash.clone();
        let result = match operation {
            "rng_word" if low.is_none() && high.is_none() => {
                let word = s.word();
                s.result_hash.update(word.to_le_bytes());
                word as f64
            }
            "rng_uniform" if low.is_none() && high.is_none() => {
                let numerator = ((s.word() as u64 >> 5) << 26) | (s.word() as u64 >> 6);
                let value = numerator as f64 / 9_007_199_254_740_992.0;
                s.result_hash.update(value.to_bits().to_le_bytes());
                value
            }
            "rng_integer" if low.is_some() && high.is_some() => {
                let (a, b) = (low.unwrap(), high.unwrap());
                if a >= b
                    || a.unsigned_abs() > MAX_SAFE as u64
                    || b.unsigned_abs() > MAX_SAFE as u64
                {
                    return Err(error(
                        "rng_integer requires low < high within the safe integer range.",
                    ));
                }
                let width = (b - a) as u64;
                let threshold = width.wrapping_neg() % width;
                let mut accepted = None;
                for _ in 0..64 {
                    let word = ((s.word() as u64) << 32) | s.word() as u64;
                    if word >= threshold {
                        accepted = Some(a + (word % width) as i64);
                        break;
                    }
                }
                let Some(value) = accepted else {
                    // Failed requests are transactional: no draw is consumed.
                    s.state = state_before;
                    s.words = before;
                    s.raw_hash = hash_before;
                    return Err(error("RNG bounded rejection limit exceeded."));
                };
                s.result_hash.update(value.to_le_bytes());
                value as f64
            }
            _ => return Err(error("Unsupported RNG operation or bounds.")),
        };
        if extends {
            s.segments.last_mut().unwrap().count += 1;
        } else {
            s.segments.push(Segment {
                operation: operation.into(),
                low,
                high,
                count: 1,
            });
            self.segments += 1;
        }
        s.operations += 1;
        self.operations += 1;
        self.words += s.words - before;
        Ok(result)
    }

    pub fn call(&mut self, name: &str, args: &[Value]) -> Result<Value> {
        require_arity(name, args.len())?;
        let index = match name {
            "rng_seed" => 1,
            "rng_integer" => 2,
            _ => 0,
        };
        let id = args
            .get(index)
            .map(|v| unsigned(v, u64::MAX >> 1, "RNG stream"))
            .transpose()?
            .unwrap_or(0);
        if name == "rng_seed" {
            self.seed(unsigned(&args[0], u64::MAX, "RNG seed")?, id)?;
            return Ok(Value::Bool(true));
        }
        let (low, high) = if name == "rng_integer" {
            (Some(bound(&args[0])?), Some(bound(&args[1])?))
        } else {
            (None, None)
        };
        Quantity::scalar(self.draw(id, name, low, high)?).map(Value::Quantity)
    }
}

/// Decode a required stream using the unchanged alpha.26 stream encoding.
pub fn stream_id(value: &Value) -> Result<u64> {
    unsigned(value, u64::MAX >> 1, "RNG stream")
}

/// Bounded replay of mappings and raw-word/result digests, without executing user code.
pub fn verify_evidence(value: &serde_json::Value) -> Result<()> {
    let evidence: Evidence =
        serde_json::from_value(value.clone()).map_err(|_| error("Malformed RNG evidence."))?;
    if evidence.schema != "goblin.rng-evidence.v1" || evidence.policy != POLICY {
        return Err(error("Unknown RNG evidence schema/policy."));
    }
    let mut replay = Randomness::default();
    for stream in &evidence.streams {
        let seed = unsigned(
            &Value::Text(stream.seed.clone()),
            u64::MAX,
            "RNG evidence seed",
        )?;
        let id = unsigned(
            &Value::Text(stream.stream.clone()),
            u64::MAX >> 1,
            "RNG evidence stream",
        )?;
        replay.seed(seed, id)?;
        if stream.segments.len() > MAX_SEGMENTS {
            return Err(error("RNG evidence too large."));
        }
        for segment in &stream.segments {
            if segment.count == 0 || segment.count > MAX_OPERATIONS {
                return Err(error("Invalid RNG operation count."));
            }
            for _ in 0..segment.count {
                replay.draw(id, &segment.operation, segment.low, segment.high)?;
            }
        }
    }
    if replay.evidence() != evidence {
        return Err(error(
            "RNG replay does not match counts, state, mappings or digests.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budgets_refuse_without_consuming_state() {
        let mut rng = Randomness::default();
        rng.seed(42, 0).unwrap();
        for budget in ["operations", "words", "segments"] {
            rng.operations = if budget == "operations" {
                MAX_OPERATIONS
            } else {
                0
            };
            rng.words = if budget == "words" { MAX_WORDS } else { 0 };
            rng.segments = if budget == "segments" {
                MAX_SEGMENTS
            } else {
                0
            };
            let before = rng.evidence();
            assert!(rng.draw(0, "rng_uniform", None, None).is_err());
            assert_eq!(rng.evidence(), before);
        }
    }

    #[test]
    fn adjacent_operations_compress_but_changed_bounds_do_not() {
        let mut rng = Randomness::default();
        rng.seed(42, 0).unwrap();
        for _ in 0..100 {
            rng.draw(0, "rng_integer", Some(0), Some(3)).unwrap();
        }
        rng.draw(0, "rng_integer", Some(0), Some(4)).unwrap();
        let evidence = rng.evidence();
        assert_eq!(evidence.streams[0].segments.len(), 2);
        assert_eq!(evidence.streams[0].segments[0].count, 100);
        verify_evidence(&serde_json::to_value(evidence).unwrap()).unwrap();
    }
}

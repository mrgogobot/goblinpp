//! Captured wall-clock inputs, not certified time or a monotonic timer.
use crate::error::{GoblinError, Result};
use chrono::{DateTime, Datelike, FixedOffset, Local, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const MAX_READINGS: usize = 16_384;
pub const MAX_REPLAY_BYTES: u64 = 8 * 1024 * 1024;
pub const FORMAT: &str = "%d-%m-%Y:%H:%M:%S";

pub fn is_function(name: &str) -> bool {
    name == "time_date"
}
pub fn policy() -> Value {
    json!({"id":"goblin.captured-wall-clock.v1", "format":"DD-MM-YYYY:HH:MM:SS",
        "default":"local", "modes":["local","utc"], "result":"captured-text",
        "source":"system-wall-clock", "authority":"not-certified-time",
        "local_zone":"system-local-with-captured-fixed-offset",
        "maximum_readings":MAX_READINGS, "native_audited":"reference-readings-replayed-in-call-order"})
}
pub fn requires_policy(receipt: &Value) -> bool {
    let v = receipt["goblin_version"].as_str().unwrap_or("");
    !(v.starts_with("0.0.")
        || v.strip_prefix("0.1.0-alpha.")
            .and_then(|n| n.parse::<u32>().ok())
            .is_some_and(|n| n < 29))
}
pub fn frozen_policy_matches(receipt: &Value) -> bool {
    receipt
        .get("clock_policy")
        .map_or(!requires_policy(receipt), |p| *p == policy())
}
pub fn program_uses_clock(program: &crate::ast::Program) -> bool {
    fn visit(v: &Value) -> bool {
        v.as_array().is_some_and(|a| {
            (a.first().is_some_and(|x| x == "call")
                && a.get(1).and_then(Value::as_str).is_some_and(is_function))
                || a.iter().any(visit)
        })
    }
    visit(&program.canonical())
}
fn error(message: impl Into<String>) -> GoblinError {
    GoblinError::new("G205", message)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Reading {
    pub index: usize,
    pub mode: String,
    pub utc: String,
    pub timezone: String,
    pub offset_seconds: i32,
    pub value: String,
}
impl Reading {
    pub fn at(index: usize, mode: &str, utc: DateTime<Utc>, offset_seconds: i32) -> Result<Self> {
        let offset = FixedOffset::east_opt(offset_seconds)
            .ok_or_else(|| error("Invalid clock UTC offset."))?;
        let date = utc.with_timezone(&offset);
        if !(0..=9999).contains(&date.year()) {
            return Err(error("time_date() requires a four-digit year."));
        }
        if !matches!(mode, "local" | "utc") || (mode == "utc" && offset_seconds != 0) {
            return Err(error(
                "time_date() mode must be exactly \"local\" or \"utc\".",
            ));
        }
        Ok(Self {
            index,
            mode: mode.into(),
            utc: utc.to_rfc3339_opts(SecondsFormat::Nanos, true),
            timezone: if mode == "utc" { "UTC" } else { "system-local" }.into(),
            offset_seconds,
            value: date.format(FORMAT).to_string(),
        })
    }
    fn validate(&self, index: usize) -> Result<()> {
        let utc = DateTime::parse_from_rfc3339(&self.utc)
            .map_err(|_| error("Invalid captured UTC clock reading."))?
            .with_timezone(&Utc);
        if *self != Self::at(index, &self.mode, utc, self.offset_seconds)? {
            return Err(error(
                "Captured clock text, index, timezone or offset is inconsistent.",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub readings: Vec<Reading>,
}
pub fn verify_evidence(value: &Value) -> Result<Evidence> {
    let evidence: Evidence = serde_json::from_value(value.clone())
        .map_err(|_| error("Missing or malformed clock evidence."))?;
    if evidence.readings.len() > MAX_READINGS {
        return Err(error("Clock reading limit exceeded."));
    }
    for (index, reading) in evidence.readings.iter().enumerate() {
        reading.validate(index)?;
    }
    Ok(evidence)
}

pub fn read_replay(path: &std::path::Path) -> Result<Evidence> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > MAX_REPLAY_BYTES
    {
        return Err(error("Invalid or oversized native clock replay file."));
    }
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(MAX_REPLAY_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_REPLAY_BYTES {
        return Err(error("Oversized native clock replay file."));
    }
    verify_evidence(&serde_json::from_slice(&bytes)?)
}

#[derive(Debug, Default)]
pub struct Clock {
    evidence: Evidence,
    replay: Option<Evidence>,
    native_configured: bool,
}
impl Clock {
    pub fn evidence(&self) -> &Evidence {
        &self.evidence
    }
    pub fn replay(evidence: Evidence) -> Result<Self> {
        verify_evidence(&serde_json::to_value(&evidence)?)?;
        Ok(Self {
            replay: Some(evidence),
            native_configured: true,
            ..Self::default()
        })
    }
    pub fn configure_native_replay(&mut self) -> Result<()> {
        if self.native_configured {
            return Ok(());
        }
        if let Some(path) = std::env::var_os("GOBLIN_CLOCK_REPLAY_PATH") {
            self.replay = Some(read_replay(std::path::Path::new(&path))?);
        }
        self.native_configured = true;
        Ok(())
    }
    pub fn capture(&mut self, mode: &str) -> Result<String> {
        if !matches!(mode, "local" | "utc") {
            return Err(error(
                "time_date() mode must be exactly \"local\" or \"utc\".",
            ));
        }
        let index = self.evidence.readings.len();
        if index >= MAX_READINGS {
            return Err(error(
                "time_date() clock reading limit exceeded (16,384 per execution).",
            ));
        }
        let reading = if let Some(replay) = &self.replay {
            let reading = replay
                .readings
                .get(index)
                .ok_or_else(|| error("Native clock replay exhausted."))?;
            if reading.mode != mode {
                return Err(error(
                    "Native clock replay mode/order differs from the interpreter reference.",
                ));
            }
            reading.clone()
        } else if mode == "utc" {
            Reading::at(index, mode, Utc::now(), 0)?
        } else {
            let local = Local::now();
            Reading::at(
                index,
                mode,
                local.with_timezone(&Utc),
                local.offset().local_minus_utc(),
            )?
        };
        let value = reading.value.clone();
        self.evidence.readings.push(reading);
        Ok(value)
    }
    pub fn finish(&self) -> Result<()> {
        if self
            .replay
            .as_ref()
            .is_some_and(|r| r.readings.len() != self.evidence.readings.len())
        {
            return Err(error("Native clock replay was not fully consumed."));
        }
        Ok(())
    }
}

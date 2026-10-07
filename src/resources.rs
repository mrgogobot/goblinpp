//! Versioned, bounded execution and streaming policy; never a sandbox claim.
use crate::ast::{Program, Stmt};
use crate::error::{GoblinError, Result};
use serde_json::{Value, json};

pub const DEFAULT_LOOP_BUDGET: u64 = 1_000_000;
pub const MAX_LOOP_BUDGET: u64 = 50_000_000;
pub fn policy() -> Value {
    json!({"id":"goblin.resources.v1", "loop_counter":"shared-loop-body-entry-v1",
        "default_loop_budget":DEFAULT_LOOP_BUDGET,"maximum_loop_budget":MAX_LOOP_BUDGET,
        "streaming":{"id":"goblin.delimited-scan.v1","maximum_file_bytes":crate::streaming::MAX_FILE_BYTES,
            "maximum_record_bytes":crate::streaming::MAX_RECORD_BYTES,"maximum_columns":crate::streaming::MAX_COLUMNS,
            "aggregation":"ordered-neumaier-sum-v1","missing_values":"refuse","empty_data":"refuse"}})
}
pub fn requires_policy(receipt: &Value) -> bool {
    let version = receipt["goblin_version"].as_str().unwrap_or("");
    !(version.starts_with("0.0.")
        || version
            .strip_prefix("0.1.0-alpha.")
            .and_then(|v| v.parse::<u32>().ok())
            .is_some_and(|v| v <= 26))
}
pub fn frozen_policy_matches(receipt: &Value) -> bool {
    receipt
        .get("resource_policy")
        .map_or(!requires_policy(receipt), |value| value == &policy())
}
pub fn requested(program: &Program) -> Option<u64> {
    program.statements.iter().find_map(|s| {
        if let Stmt::LoopBudget(v) = s {
            Some(*v)
        } else {
            None
        }
    })
}
pub fn validate_program(program: &Program) -> Result<()> {
    fn nested(statement: &Stmt) -> bool {
        let body_has = |body: &[Stmt]| {
            body.iter()
                .any(|s| matches!(s, Stmt::LoopBudget(_)) || nested(s))
        };
        match statement {
            Stmt::Function { body, .. }
            | Stmt::For { body, .. }
            | Stmt::ForEach { body, .. }
            | Stmt::While { body, .. } => body_has(body),
            Stmt::If {
                branches,
                else_body,
            } => {
                branches.iter().any(|(_, b)| body_has(b))
                    || else_body.as_ref().is_some_and(|b| body_has(b))
            }
            Stmt::Switch { cases, default, .. } => {
                cases.iter().any(|(_, b)| body_has(b))
                    || default.as_ref().is_some_and(|b| body_has(b))
            }
            _ => false,
        }
    }
    let budgets: Vec<_> = program
        .statements
        .iter()
        .filter_map(|s| {
            if let Stmt::LoopBudget(v) = s {
                Some(*v)
            } else {
                None
            }
        })
        .collect();
    if budgets.len() > 1
        || budgets.iter().any(|v| !(1..=MAX_LOOP_BUDGET).contains(v))
        || program.statements.iter().any(nested)
    {
        return Err(GoblinError::parse(
            "GO_LOOP_BUDGET requires one top-level main-source declaration between 1 and 50000000.",
        ));
    }
    Ok(())
}
pub fn evidence(requested: Option<u64>, used: u64) -> Value {
    json!({"requested_loop_budget":requested,"effective_loop_budget":requested.unwrap_or(DEFAULT_LOOP_BUDGET),"used_loop_iterations":used})
}
pub fn validate_evidence(value: &Value, requested: Option<Option<u64>>) -> Result<()> {
    let fields = value
        .as_object()
        .ok_or_else(|| GoblinError::data("Missing resource evidence object."))?;
    if fields.len() != 3
        || ![
            "requested_loop_budget",
            "effective_loop_budget",
            "used_loop_iterations",
        ]
        .iter()
        .all(|field| fields.contains_key(*field))
    {
        return Err(GoblinError::data(
            "Resource evidence must have exactly three fields.",
        ));
    }
    let request = if value["requested_loop_budget"].is_null() {
        None
    } else {
        Some(
            value["requested_loop_budget"]
                .as_u64()
                .ok_or_else(|| GoblinError::data("Invalid requested loop budget."))?,
        )
    };
    if request.is_some_and(|v| !(1..=MAX_LOOP_BUDGET).contains(&v))
        || value["effective_loop_budget"].as_u64() != Some(request.unwrap_or(DEFAULT_LOOP_BUDGET))
        || value["used_loop_iterations"]
            .as_u64()
            .is_none_or(|v| v > request.unwrap_or(DEFAULT_LOOP_BUDGET))
        || requested.is_some_and(|v| request != v)
    {
        return Err(GoblinError::data(
            "Resource evidence does not match the declared source budget or hard limits.",
        ));
    }
    Ok(())
}

pub fn native_evidence(text: &str) -> Result<Value> {
    let mut entries = text.lines().filter(|line| line.starts_with("RESOURCE\t"));
    let line = entries
        .next()
        .ok_or_else(|| GoblinError::compile("Native resource evidence is missing."))?;
    if entries.next().is_some() {
        return Err(GoblinError::compile("Duplicate native resource evidence."));
    }
    let fields = line.split('\t').collect::<Vec<_>>();
    let ["RESOURCE", request, effective, used] = fields.as_slice() else {
        return Err(GoblinError::compile("Malformed native resource evidence."));
    };
    let parse = |s: &str| {
        s.parse::<u64>()
            .map_err(|_| GoblinError::compile("Invalid native resource counter."))
    };
    let request = if *request == "null" {
        None
    } else {
        Some(parse(request)?)
    };
    let result = evidence(request, parse(used)?);
    if parse(effective)? != result["effective_loop_budget"].as_u64().unwrap() {
        return Err(GoblinError::compile(
            "Native effective loop budget mismatch.",
        ));
    }
    validate_evidence(&result, None)?;
    Ok(result)
}

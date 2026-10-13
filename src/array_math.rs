//! Versioned element-wise semantics. Historical receipts remain verifiable;
//! a historical freeze is not implicit permission to adopt new array semantics.
use crate::elementwise_runtime::{self, Operand};
use crate::error::{GoblinError, Result};
use crate::evaluator::Value;
use serde_json::{Value as Json, json};

pub fn policy() -> Json {
    json!({"id":"goblin.elementwise-numeric-arrays.v1",
        "operators":["unary +","unary -","+","-","*","/","%","^"],
        "functions":"abs,sqrt,min,max,floor,ceil,round,exp,ln,log10,sin,cos,tan,asin,acos,atan,atan2,sind,cosd,tand,sinr,cosr,tanr,asind,acosd,atand,asinr,acosr,atanr,atan2d,atan2r,deg2rad,rad2deg,hypot",
        "shape":"one-dimensional; equal array lengths; explicit scalar repetition; no length-one array broadcasting",
        "values":"finite homogeneous numeric quantities; independent result copy",
        "empty_arrays":"empty result; validate shape and numeric scalar types; no scalar domain evaluation without lanes",
        "ordering":"left-to-right arguments; ascending zero-based lanes; shape/type preflight before scalar computation",
        "failure":"first failing lane; no partial assignment; prior explicit artifacts remain preserved",
        "dimension":"scalar checked rules per lane; homogeneous output dimensions required",
        "maximum_array_items":elementwise_runtime::MAX_ITEMS,
        "loop_budget":"lane mapping does not enter user loop bodies; bounded per operation, not a process RAM sandbox",
        "comparisons":"unchanged; is_close and same_bits remain scalar; no masks or reductions",
        "math_backend":"unchanged rust-std-platform; no cross-platform bitwise guarantee"})
}

pub fn requires_policy(receipt: &Json) -> bool {
    receipt["goblin_version"].as_str().is_some_and(|version| {
        !version.starts_with("0.0.")
            && version
                .strip_prefix("0.1.0-alpha.")
                .is_none_or(|n| n.parse::<u64>().is_ok_and(|n| n >= 33))
    })
}

pub fn supports_receipt(receipt: &Json) -> bool {
    receipt
        .get("array_math_policy")
        .map_or(!requires_policy(receipt), |p| *p == policy())
}

pub fn apply(
    name: &str,
    args: &[Value],
    scalar: impl FnMut(Vec<Value>) -> Result<Value>,
) -> Result<Value> {
    let inputs: Vec<_> = args
        .iter()
        .map(|value| match value {
            Value::Array(items) => Operand::Array(items),
            value => Operand::Scalar(value),
        })
        .collect();
    elementwise_runtime::map(name, &inputs,
        |value| matches!(value, Value::Quantity(q) if q.value_si.is_finite()),
        |a, b| matches!((a,b), (Value::Quantity(a), Value::Quantity(b)) if a.dimension == b.dimension),
        scalar,
        |message| GoblinError::new("G203", message),
        |mut error, index| { error.message = format!("{name}: element index {index} (zero-based)\n\n{}", error.message); error })
        .map(Value::Array)
}

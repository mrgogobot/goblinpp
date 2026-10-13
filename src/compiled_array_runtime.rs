// Included verbatim after generated Value and shared lane mapping definitions.
fn goblin_array_apply(name: &str, args: &[Value], scalar: impl FnMut(Vec<Value>) -> Result<Value, String>) -> Result<Value, String> {
    let inputs: Vec<_> = args.iter().map(|value| match value {
        Value::Array(items) => goblin_elementwise::Operand::Array(items),
        value => goblin_elementwise::Operand::Scalar(value),
    }).collect();
    goblin_elementwise::map(name, &inputs,
        |value| matches!(value, Value::Q(v, _) if v.is_finite()),
        |a,b| matches!((a,b), (Value::Q(_,ad), Value::Q(_,bd)) if ad == bd),
        scalar, |message| message,
        |error,index| format!("{name}: element index {index} (zero-based)\n\n{error}"))
        .map(Value::Array)
}

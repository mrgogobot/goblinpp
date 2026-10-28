// Included verbatim in generated Rust programs. `Value` is the generated value type.
fn goblin_string_call(name: &str, values: Vec<Value>) -> Result<Value, String> {
    let expected = match name {
        "to_text" | "parse_number" | "parse_integer" | "str_trim" => 1,
        "str_contains" | "str_split" | "str_join" => 2,
        "str_replace" => 3,
        _ => return Err(format!("UNKNOWN SYMBOL: {name}")),
    };
    if values.len() != expected {
        return Err(format!(
            "{name}() expects {expected} argument(s), got {}.",
            values.len()
        ));
    }
    let mut values = values.into_iter();
    let result = match name {
        "to_text" => {
            let value = values.next().unwrap();
            if matches!(value, Value::Array(_)) {
                return Err(
                    "to_text() does not render arrays; use str_join() for text arrays.".into(),
                );
            }
            Value::Text(goblin_text::bounded(value.render())?)
        }
        "parse_number" => Value::scalar(goblin_text::parse_number(&expect_text(
            values.next().unwrap(),
            name,
        )?)?)?,
        "parse_integer" => Value::scalar(goblin_text::parse_integer(&expect_text(
            values.next().unwrap(),
            name,
        )?)?)?,
        "str_trim" => {
            let text = expect_text(values.next().unwrap(), name)?;
            Value::Text(goblin_text::bounded(text.trim().to_string())?)
        }
        "str_contains" => {
            let text = expect_text(values.next().unwrap(), name)?;
            let needle = expect_text(values.next().unwrap(), name)?;
            Value::Bool(text.contains(&needle))
        }
        "str_replace" => {
            let text = expect_text(values.next().unwrap(), name)?;
            let old = expect_text(values.next().unwrap(), name)?;
            let new = expect_text(values.next().unwrap(), name)?;
            Value::Text(goblin_text::replace(&text, &old, &new)?)
        }
        "str_split" => {
            let text = expect_text(values.next().unwrap(), name)?;
            let separator = expect_text(values.next().unwrap(), name)?;
            Value::Array(
                goblin_text::split(&text, &separator)?
                    .into_iter()
                    .map(Value::Text)
                    .collect(),
            )
        }
        "str_join" => {
            let separator = expect_text(values.next().unwrap(), name)?;
            let Value::Array(items) = values.next().unwrap() else {
                return Err("str_join() requires an array of text.".into());
            };
            let parts = items
                .into_iter()
                .map(|item| expect_text(item, name))
                .collect::<Result<Vec<_>, _>>()?;
            Value::Text(goblin_text::join(&separator, &parts)?)
        }
        _ => unreachable!(),
    };
    Ok(result)
}

fn expect_text(value: Value, context: &str) -> Result<String, String> {
    match value {
        Value::Text(text) => Ok(text),
        _ => Err(format!("{context}() requires text.")),
    }
}

fn goblin_text_toolkit(name: &str, values: Vec<Value>) -> Result<Value, String> {
    let expected = goblin_text::toolkit_arity(name).ok_or("Unknown text toolkit function.")?;
    if values.len() != expected {
        return Err(format!("{name}() expects {expected} arguments, got {}.", values.len()));
    }
    let scalar = |v: &Value| -> Result<f64, String> {
        match v {
            Value::Q(n, dim) if *dim == ZERO => Ok(*n),
            _ => Err(format!("{name}() requires unitless scalar numbers.")),
        }
    };
    let text = |i: usize| -> Result<&str, String> {
        match &values[i] {
            Value::Text(s) => Ok(s),
            _ => Err(format!("{name}() requires text.")),
        }
    };
    match name {
        "str_upper" | "str_lower" => Ok(Value::Text(goblin_text::change_case(text(0)?, name == "str_upper")?)),
        "str_bytes_len" | "sizeof" => Value::scalar(text(0)?.len() as f64),
        "str_compare" => Value::scalar(goblin_text::compare(text(0)?, text(1)?)),
        "to_base" => Ok(Value::Text(goblin_text::to_base(scalar(&values[0])?, scalar(&values[1])?)?)),
        "from_base" => Value::scalar(goblin_text::from_base(text(0)?, scalar(&values[1])?)?),
        _ => Ok(Value::Text(goblin_text::byte_encoding(name, text(0)?)?)),
    }
}

use crate::ast::{Expr, Program, Stmt};
use crate::constants::by_id;
use crate::error::{GoblinError, Result};
use crate::hashing::{sha256_bytes, sha256_file};
use crate::inline_rust::{InlineRustBlock, approved};
use crate::parser::ParsedSource;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Compilation {
    pub binary: PathBuf,
    pub generated_source: PathBuf,
    pub binary_sha256: String,
    pub generated_source_sha256: String,
    pub rustc_version: String,
}

pub fn compile(
    parsed: &ParsedSource,
    output: impl AsRef<Path>,
    allowed_inline: &[String],
) -> Result<Compilation> {
    parsed.require_executable_program()?;
    approved(&parsed.inline_rust, allowed_inline)?;
    reject_uncompilable_data_calls(&parsed.program)?;
    let output = output.as_ref();
    if output.exists() {
        return Err(GoblinError::compile(format!(
            "Refusing to overwrite compiled artifact: {}",
            output.display()
        )));
    }
    let generated_source = output.with_extension("goblin.rs");
    if generated_source.exists() {
        return Err(GoblinError::compile(format!(
            "Refusing to overwrite generated source: {}",
            generated_source.display()
        )));
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let source = generate(&parsed.program, &parsed.inline_rust)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&generated_source)
        .map_err(|error| {
            GoblinError::compile(format!(
                "Unable to create {}: {error}",
                generated_source.display()
            ))
        })?;
    file.write_all(source.as_bytes())?;
    file.sync_all()?;
    let result = Command::new("rustc")
        .arg("--crate-name")
        .arg("goblinpp_program")
        .arg("--edition=2024")
        .arg("-C")
        .arg("opt-level=2")
        .arg("-C")
        .arg("overflow-checks=yes")
        .arg("-o")
        .arg(output)
        .arg(&generated_source)
        .output()
        .map_err(|error| GoblinError::compile(format!("Unable to start rustc: {error}")))?;
    if !result.status.success() {
        let _ = fs::remove_file(output);
        return Err(GoblinError::compile(format!(
            "Native compilation failed. Generated source is preserved at {}.\n\n{}",
            generated_source.display(),
            String::from_utf8_lossy(&result.stderr)
        )));
    }
    let rustc_version = Command::new("rustc")
        .arg("--version")
        .output()
        .map(|value| String::from_utf8_lossy(&value.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".into());
    Ok(Compilation {
        binary: output.to_path_buf(),
        generated_source: generated_source.clone(),
        binary_sha256: sha256_file(output)?,
        generated_source_sha256: sha256_bytes(source.as_bytes()),
        rustc_version,
    })
}

fn reject_uncompilable_data_calls(program: &Program) -> Result<()> {
    fn visit(expression: &Expr) -> bool {
        match expression {
            Expr::Call { name, args } => {
                name.starts_with("fits_")
                    || name.starts_with("write_")
                    || name.starts_with("plot_")
                    || args.iter().any(visit)
            }
            Expr::Unary { expr, .. } => visit(expr),
            Expr::Array(items) => items.iter().any(visit),
            Expr::Index { array, index } => visit(array) || visit(index),
            Expr::Slice { array, start, stop } => {
                visit(array)
                    || start.as_ref().is_some_and(|v| visit(v))
                    || stop.as_ref().is_some_and(|v| visit(v))
            }
            Expr::Binary { left, right, .. } => visit(left) || visit(right),
            Expr::Compare { left, right, .. } => visit(left) || visit(right),
            _ => false,
        }
    }
    fn visit_stmt(statement: &Stmt) -> bool {
        match statement {
            Stmt::Assign { expr, .. } | Stmt::Expression(expr) => visit(expr),
            Stmt::IndexAssign { index, expr, .. } => visit(index) || visit(expr),
            Stmt::For {
                start,
                stop,
                step,
                body,
                ..
            } => visit(start) || visit(stop) || visit(step) || body.iter().any(visit_stmt),
            Stmt::While { condition, body } => visit(condition) || body.iter().any(visit_stmt),
            Stmt::If {
                branches,
                else_body,
            } => {
                branches
                    .iter()
                    .any(|(condition, body)| visit(condition) || body.iter().any(visit_stmt))
                    || else_body
                        .as_ref()
                        .is_some_and(|body| body.iter().any(visit_stmt))
            }
            Stmt::Switch {
                selector,
                cases,
                default,
            } => {
                visit(selector)
                    || cases
                        .iter()
                        .any(|(label, body)| visit(label) || body.iter().any(visit_stmt))
                    || default
                        .as_ref()
                        .is_some_and(|body| body.iter().any(visit_stmt))
            }
            _ => false,
        }
    }
    if program.statements.iter().any(visit_stmt) {
        return Err(GoblinError::compile(
            "Native code generation for FITS and generated-output calls is not yet implemented. Interpret this program without --compile; both facilities still run as native Rust inside the interpreter.",
        ));
    }
    Ok(())
}

fn generate(program: &Program, blocks: &[InlineRustBlock]) -> Result<String> {
    let body = generate_statements(&program.statements, blocks)?;

    Ok(format!(
        r#"// Generated by Goblin++ {version}. Reviewable build evidence; do not edit in place.
use std::collections::{{BTreeMap, HashMap}};
use std::fs::OpenOptions;
use std::io::{{BufRead, Read, Write}};

type Dim = [i32; 5];
const ZERO: Dim = [0, 0, 0, 0, 0];
const MAX_LOOP_ITERATIONS: u64 = 1_000_000;

#[derive(Clone, Debug)]
enum Value {{ Q(f64, Dim), Text(String), Bool(bool), Array(Vec<Value>) }}

impl Value {{
    fn q(value: f64, dim: Dim) -> Result<Self, String> {{
        if !value.is_finite() {{ Err("NON-FINITE NUMERIC VALUE".into()) }} else {{ Ok(Self::Q(value, dim)) }}
    }}
    fn scalar(value: f64) -> Result<Self, String> {{ Self::q(value, ZERO) }}
    fn render(&self) -> String {{
        match self {{
            Self::Text(value) => value.clone(),
            Self::Bool(value) => value.to_string(),
            Self::Array(items) => format!("[{{}}]", items.iter().map(Value::render).collect::<Vec<_>>().join(", ")),
            Self::Q(value, dim) => {{
                let number = format_number(*value);
                let unit = format_dim(*dim);
                if unit == "1" {{ number }} else {{ format!("{{number}} {{unit}}") }}
            }}
        }}
    }}
}}

fn as_q(value: Value) -> Result<(f64, Dim), String> {{ match value {{ Value::Q(v, d) => Ok((v, d)), _ => Err("Arithmetic requires numeric quantities.".into()) }} }}
fn binary(op: char, left: Value, right: Value) -> Result<Value, String> {{
    let (a, ad) = as_q(left)?; let (b, bd) = as_q(right)?;
    match op {{
        '+' | '-' if ad != bd => Err(format!("INCOMPATIBLE DIMENSIONS: {{}} and {{}}", format_dim(ad), format_dim(bd))),
        '+' => Value::q(a + b, ad), '-' => Value::q(a - b, ad),
        '*' => Value::q(a * b, std::array::from_fn(|i| ad[i] + bd[i])),
        '/' if b == 0.0 => Err("DIVISION BY ZERO".into()),
        '/' => Value::q(a / b, std::array::from_fn(|i| ad[i] - bd[i])),
        '^' if bd != ZERO || b.fract() != 0.0 || b < i32::MIN as f64 || b > i32::MAX as f64 => Err("Exponent must be an integer scalar.".into()),
        '^' => Value::q(a.powi(b as i32), ad.map(|item| item * b as i32)),
        _ => Err(format!("Unknown operator {{op}}")),
    }}
}}
fn compare(op: &str, left: Value, right: Value) -> Result<Value, String> {{
    let result = match (left, right) {{
        (Value::Q(a, ad), Value::Q(b, bd)) => {{
            if ad != bd {{ return Err("Comparison requires matching dimensions.".into()); }}
            match op {{ "==" => a == b, "!=" => a != b, "<" => a < b, "<=" => a <= b, ">" => a > b, ">=" => a >= b, _ => return Err("Unknown comparison operator.".into()) }}
        }}
        (Value::Text(a), Value::Text(b)) => match op {{ "==" => a == b, "!=" => a != b, _ => return Err("Text supports == and !=, not ordering.".into()) }},
        (Value::Bool(a), Value::Bool(b)) => match op {{ "==" => a == b, "!=" => a != b, _ => return Err("Booleans support == and !=, not ordering.".into()) }},
        (Value::Array(a), Value::Array(b)) if op == "==" || op == "!=" => {{
            let equal = a.len() == b.len() && a.iter().zip(&b).all(|(x, y)| match (x, y) {{
                (Value::Q(x, xd), Value::Q(y, yd)) => x == y && xd == yd,
                (Value::Text(x), Value::Text(y)) => x == y,
                (Value::Bool(x), Value::Bool(y)) => x == y,
                _ => false,
            }});
            equal == (op == "==")
        }},
        _ => return Err("Comparison requires two values of the same kind.".into()),
    }};
    Ok(Value::Bool(result))
}}
fn as_bool(value: Value, context: &str) -> Result<bool, String> {{ match value {{ Value::Bool(v) => Ok(v), _ => Err(format!("{{context}} condition must be true or false.")) }} }}
fn switch_equal(left: &Value, right: &Value) -> Result<bool, String> {{
    match compare("==", left.clone(), right.clone())? {{ Value::Bool(value) => Ok(value), _ => Err("switch comparison did not return a Boolean.".into()) }}
}}
fn range_integer(value: Value, context: &str) -> Result<i64, String> {{
    const MAX_SAFE: f64 = 9_007_199_254_740_991.0;
    let (number, dim) = as_q(value)?;
    if dim != ZERO || number.fract() != 0.0 || number.abs() > MAX_SAFE {{ return Err(format!("{{context}} must be a dimensionless, exactly representable integer.")); }}
    Ok(number as i64)
}}
fn loop_tick(steps: &mut u64) -> Result<(), String> {{
    if *steps >= MAX_LOOP_ITERATIONS {{ return Err(format!("LOOP LIMIT EXCEEDED: at most {{MAX_LOOP_ITERATIONS}} loop-body iterations per run.")); }}
    *steps += 1;
    Ok(())
}}
fn unary(op: char, value: Value) -> Result<Value, String> {{ let (v, d) = as_q(value)?; Value::q(if op == '-' {{ -v }} else {{ v }}, d) }}
fn get(env: &HashMap<String, Value>, name: &str) -> Result<Value, String> {{ env.get(name).cloned().ok_or_else(|| unknown(name)) }}
fn unknown(name: &str) -> String {{ format!("UNKNOWN SYMBOL: {{name}}") }}
const MAX_ARRAY_ITEMS: usize = 100_000;
fn same_array_type(a: &Value, b: &Value) -> bool {{ match (a, b) {{
    (Value::Q(_, ad), Value::Q(_, bd)) => ad == bd,
    (Value::Text(_), Value::Text(_)) | (Value::Bool(_), Value::Bool(_)) => true,
    _ => false,
}} }}
fn array_literal(items: Vec<Result<Value, String>>) -> Result<Value, String> {{
    let items = items.into_iter().collect::<Result<Vec<_>, _>>()?;
    if items.len() > MAX_ARRAY_ITEMS {{ return Err("Array item limit exceeded.".into()); }}
    if let Some(first) = items.first() {{
        if matches!(first, Value::Array(_)) || items[1..].iter().any(|v| !same_array_type(first, v)) {{
            return Err("Array elements must share a type and, for quantities, a dimension; nested arrays are unsupported.".into());
        }}
    }}
    Ok(Value::Array(items))
}}
fn array_index(value: Value) -> Result<usize, String> {{
    let (number, dim) = as_q(value)?;
    if dim != ZERO || number.fract() != 0.0 || number < 0.0 || number > MAX_ARRAY_ITEMS as f64 {{ return Err("Array index requires a non-negative integer within the array limit.".into()); }}
    Ok(number as usize)
}}
fn array_get(array: Value, index: Value) -> Result<Value, String> {{
    let index = array_index(index)?;
    let Value::Array(items) = array else {{ return Err("Indexing requires an array.".into()); }};
    items.get(index).cloned().ok_or_else(|| format!("Array index {{index}} is out of bounds for length {{}}.", items.len()))
}}
fn array_slice(array: Value, start: Option<Value>, stop: Option<Value>) -> Result<Value, String> {{
    let Value::Array(items) = array else {{ return Err("Slicing requires an array.".into()); }};
    let start = start.map(array_index).transpose()?.unwrap_or(0);
    let stop = stop.map(array_index).transpose()?.unwrap_or(items.len());
    if start > stop || stop > items.len() {{ return Err(format!("Invalid slice [{{start}}:{{stop}}] for length {{}}.", items.len())); }}
    Ok(Value::Array(items[start..stop].to_vec()))
}}
fn array_set(env: &mut HashMap<String, Value>, name: &str, index: Value, value: Value) -> Result<(), String> {{
    let index = array_index(index)?;
    let current = env.get_mut(name).ok_or_else(|| unknown(name))?;
    let Value::Array(items) = current else {{ return Err("Indexed assignment requires an array.".into()); }};
    if index >= items.len() {{ return Err(format!("Array index {{index}} is out of bounds for length {{}}.", items.len())); }}
    if !same_array_type(&items[0], &value) {{ return Err("Array elements must share a type and dimension.".into()); }}
    items[index] = value;
    Ok(())
}}
fn array_append(array: Value, value: Value) -> Result<Value, String> {{
    let Value::Array(mut items) = array else {{ return Err("append() requires an array.".into()); }};
    if items.len() >= MAX_ARRAY_ITEMS {{ return Err("Array item limit exceeded.".into()); }}
    if matches!(value, Value::Array(_)) || items.first().is_some_and(|first| !same_array_type(first, &value)) {{ return Err("Array elements must share a type and dimension; nested arrays are unsupported.".into()); }}
    items.push(value); Ok(Value::Array(items))
}}
fn array_len(array: Value) -> Result<Value, String> {{
    let Value::Array(items) = array else {{ return Err("len() requires an array.".into()); }};
    Value::scalar(items.len() as f64)
}}
fn goblin_argv(index: Value, argv: &[String]) -> Result<Value, String> {{
    let (index, dim) = as_q(index)?;
    if dim != ZERO || index.fract() != 0.0 || index < 0.0 || index >= argv.len() as f64 {{
        return Err(format!("argv index out of range; argc is {{}}.", argv.len()));
    }}
    Ok(Value::Text(argv[index as usize].clone()))
}}
fn goblin_input(prompt: Value, env: &HashMap<String, Value>, argc: usize, steps: &mut usize) -> Result<Value, String> {{
    if *steps >= 1_024 {{ return Err("input() call limit exceeded.".into()); }}
    *steps += 1;
    let prompt = match prompt {{ Value::Text(text) => interpolate(&text, env, argc)?, _ => return Err("input() prompt must be text.".into()) }};
    if prompt.len() > 65_536 {{ return Err("input() prompt is too long.".into()); }}
    if std::env::var_os("GOBLIN_STDIN_REPLAY").is_none() {{
        if std::env::var("GOBLIN_INPUT_PROMPT_PROTOCOL").as_deref() == Ok("hex-v1") {{
            let encoded = prompt.as_bytes().iter().map(|byte| format!("{{byte:02x}}")).collect::<String>();
            eprintln!("GOBLIN_INPUT_PROMPT_V1\t{{encoded}}");
        }} else {{ eprint!("{{prompt}}"); }}
        std::io::stderr().flush().map_err(|error| format!("Cannot display input prompt: {{error}}"))?;
    }}
    let mut line = String::new();
    let mut limited = std::io::stdin().lock().take(65_538);
    let read = limited.read_line(&mut line).map_err(|error| format!("Cannot read standard input: {{error}}"))?;
    if read == 0 {{ return Err("input() reached end of standard input.".into()); }}
    if line.ends_with('\n') {{ line.pop(); if line.ends_with('\r') {{ line.pop(); }} }}
    if line.len() > 65_536 {{ return Err("input() response is too long.".into()); }}
    Ok(Value::Text(line))
}}

fn call_print(values: Vec<Result<Value, String>>, env: &HashMap<String, Value>, argc: usize) -> Result<(), String> {{
    let values = values.into_iter().collect::<Result<Vec<_>, _>>()?;
    let text = if values.len() == 1 {{
        match &values[0] {{ Value::Text(template) => interpolate(template, env, argc)?, value => value.render() }}
    }} else {{ values.iter().map(Value::render).collect::<Vec<_>>().join(" ") }};
    println!("{{text}}"); Ok(())
}}
fn call_printf(values: Vec<Result<Value, String>>) -> Result<(), String> {{
    let values = values.into_iter().collect::<Result<Vec<_>, _>>()?;
    let template = match values.first() {{ Some(Value::Text(value)) => value, _ => return Err("printf needs a format string".into()) }};
    let mut output = String::new(); let mut cursor = 0; let mut argument = 1;
    while let Some(relative) = template[cursor..].find('%') {{
        let start = cursor + relative; output.push_str(&template[cursor..start]);
        if template[start..].starts_with("%%") {{ output.push('%'); cursor = start + 2; continue; }}
        let tail = &template[start + 1..]; let end = tail.find(['f', 's']).ok_or("unsupported printf format")?; let spec = &tail[..=end];
        let value = values.get(argument).ok_or("printf has too few values")?;
        if spec.ends_with('s') {{ output.push_str(&value.render()); }} else {{
            let precision = spec.strip_prefix('.').and_then(|v| v.strip_suffix('f')).and_then(|v| v.parse::<usize>().ok()).unwrap_or(6);
            let numeric = match value {{ Value::Q(value, _) => *value, _ => return Err("printf numeric format needs a quantity".into()) }};
            output.push_str(&format!("{{:.*}}", precision, numeric));
        }}
        argument += 1; cursor = start + 1 + spec.len();
    }}
    output.push_str(&template[cursor..]); if argument != values.len() {{ return Err("printf has too many values".into()); }} println!("{{output}}"); Ok(())
}}
fn interpolate(template: &str, env: &HashMap<String, Value>, argc: usize) -> Result<String, String> {{
    let mut output = String::new(); let mut cursor = 0;
    while let Some(relative) = template[cursor..].find('{{') {{
        let start = cursor + relative; output.push_str(&template[cursor..start]);
        let end = template[start + 1..].find('}}').map(|v| start + 1 + v).ok_or("unclosed interpolation")?;
        let field = &template[start + 1..end]; let (name, spec) = field.split_once(':').map_or((field, None), |(n, s)| (n, Some(s)));
        let argc_value = Value::Q(argc as f64, ZERO);
        let value = if name == "argc" {{ &argc_value }} else {{ env.get(name).ok_or_else(|| unknown(name))? }};
        match (value, spec) {{
            (Value::Q(number, dim), Some(spec)) => {{
                output.push_str(&format_numeric(*number, spec).ok_or("only .Nf and .Ne formatting are supported")?); let unit = format_dim(*dim); if unit != "1" {{ output.push(' '); output.push_str(&unit); }}
            }}
            (value, None) => output.push_str(&value.render()), _ => return Err("format requires quantity".into()),
        }}
        cursor = end + 1;
    }}
    output.push_str(&template[cursor..]); Ok(output)
}}
fn format_number(value: f64) -> String {{
    if value == 0.0 {{ return "0".into(); }}
    if value.abs() >= 1e15 || value.abs() < 1e-4 {{
        let raw = format!("{{value:.14e}}"); let (mantissa, exponent) = raw.split_once('e').unwrap();
        format!("{{}}e{{:+}}", mantissa.trim_end_matches('0').trim_end_matches('.'), exponent.parse::<i32>().unwrap())
    }} else {{ format!("{{value:.15}}").trim_end_matches('0').trim_end_matches('.').to_string() }}
}}
fn format_numeric(value: f64, spec: &str) -> Option<String> {{
    let digits = spec.strip_prefix('.')?;
    if let Some(precision) = digits.strip_suffix('f').and_then(|value| value.parse::<usize>().ok()) {{ return Some(format!("{{:.*}}", precision, value)); }}
    if let Some(precision) = digits.strip_suffix('e').and_then(|value| value.parse::<usize>().ok()) {{
        let raw = format!("{{:.*e}}", precision, value); let (mantissa, exponent) = raw.split_once('e')?; let exponent = exponent.parse::<i32>().ok()?; let sign = if exponent < 0 {{ '-' }} else {{ '+' }};
        return Some(format!("{{mantissa}}e{{sign}}{{:02}}", exponent.unsigned_abs()));
    }}
    None
}}
fn format_dim(dim: Dim) -> String {{
    if dim == ZERO {{ return "1".into(); }} if dim == [1,2,-2,0,0] {{ return "J".into(); }}
    let names = ["kg", "m", "s", "K", "mol"]; let mut top = vec![]; let mut bottom = vec![];
    for (name, power) in names.iter().zip(dim) {{ if power == 0 {{ continue; }} let item = if power.abs() == 1 {{ name.to_string() }} else {{ format!("{{name}}^{{}}", power.abs()) }}; if power > 0 {{ top.push(item) }} else {{ bottom.push(item) }} }}
    let numerator = if top.is_empty() {{ "1".into() }} else {{ top.join("*") }}; if bottom.is_empty() {{ numerator }} else {{ format!("{{numerator}}/{{}}", bottom.join("*")) }}
}}

fn hex(bytes: &[u8]) -> String {{
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {{
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 15) as usize] as char);
    }}
    output
}}
fn write_native_results(sealed: &BTreeMap<String, Value>) -> Result<(), String> {{
    let Ok(path) = std::env::var("GOBLIN_NATIVE_RESULT_PATH") else {{ return Ok(()); }};
    let mut file = OpenOptions::new().write(true).create_new(true).open(&path).map_err(|error| format!("cannot create native result manifest {{path}}: {{error}}"))?;
    for (name, value) in sealed {{
        let line = match value {{
            Value::Q(number, dim) => format!("Q\t{{}}\t{{:016x}}\t{{}},{{}},{{}},{{}},{{}}\n", hex(name.as_bytes()), number.to_bits(), dim[0], dim[1], dim[2], dim[3], dim[4]),
            Value::Text(text) => format!("T\t{{}}\t{{}}\n", hex(name.as_bytes()), hex(text.as_bytes())),
            Value::Bool(value) => format!("B\t{{}}\t{{}}\n", hex(name.as_bytes()), value),
            Value::Array(items) => {{
                let payload = items.iter().map(|item| match item {{
                    Value::Q(number, dim) => Ok(format!("Q\t{{:016x}}\t{{}},{{}},{{}},{{}},{{}}", number.to_bits(), dim[0], dim[1], dim[2], dim[3], dim[4])),
                    Value::Text(text) => Ok(format!("T\t{{}}", hex(text.as_bytes()))),
                    Value::Bool(value) => Ok(format!("B\t{{value}}")),
                    Value::Array(_) => Err("Nested arrays cannot be sealed.".to_string()),
                }}).collect::<Result<Vec<_>, String>>()?.join("\n");
                format!("A\t{{}}\t{{}}\n", hex(name.as_bytes()), hex(payload.as_bytes()))
            }},
        }};
        file.write_all(line.as_bytes()).map_err(|error| format!("cannot write native result manifest: {{error}}"))?;
    }}
    file.sync_all().map_err(|error| format!("cannot sync native result manifest: {{error}}"))?;
    Ok(())
}}

fn goblin_main() -> Result<(), String> {{
    let mut goblin_env: HashMap<String, Value> = HashMap::new();
    let mut goblin_args = vec![std::env::var("GOBLIN_SOURCE_ARGV0").unwrap_or_else(|_| std::env::args().next().unwrap_or_default())];
    goblin_args.extend(std::env::args().skip(1));
    let mut goblin_input_steps: usize = 0;
    let mut goblin_loop_steps: u64 = 0;
    let mut goblin_sealed_values: BTreeMap<String, Value> = BTreeMap::new();
{body}    write_native_results(&goblin_sealed_values)?;
    Ok(())
}}

fn main() {{ if let Err(error) = goblin_main() {{ eprintln!("GOBLIN NATIVE ERROR\n\n{{error}}"); std::process::exit(1); }} }}
"#,
        version = crate::VERSION,
        body = body,
    ))
}

fn generate_statements(statements: &[Stmt], blocks: &[InlineRustBlock]) -> Result<String> {
    let mut body = String::new();
    for statement in statements {
        match statement {
            Stmt::Directive(_) => {
                body.push_str("    // GO_PARANOID: enforced by the launcher and receipt policy.\n")
            }
            Stmt::Assign { name, expr } => {
                body.push_str(&format!("    let value = ({})?;\n", generate_expr(expr)?));
                body.push_str(&format!(
                    "    goblin_env.insert({:?}.into(), value);\n",
                    name
                ));
            }
            Stmt::IndexAssign { name, index, expr } => {
                body.push_str(&format!(
                    "    array_set(&mut goblin_env, {name:?}, ({})?, ({})?)?;\n",
                    generate_expr(index)?,
                    generate_expr(expr)?
                ));
            }
            Stmt::Expression(Expr::Call { name, args }) if name == "print" => {
                body.push_str(&format!(
                    "    call_print(vec![{}], &goblin_env, goblin_args.len())?;\n",
                    args.iter()
                        .map(generate_expr)
                        .collect::<Result<Vec<_>>>()?
                        .join(", ")
                ));
            }
            Stmt::Expression(Expr::Call { name, args }) if name == "printf" => {
                body.push_str(&format!(
                    "    call_printf(vec![{}])?;\n",
                    args.iter()
                        .map(generate_expr)
                        .collect::<Result<Vec<_>>>()?
                        .join(", ")
                ));
            }
            Stmt::Expression(expr) => {
                body.push_str(&format!("    let _ = ({})?;\n", generate_expr(expr)?));
            }
            Stmt::Seal(name) => {
                body.push_str(&format!(
                    "    goblin_sealed_values.insert({name:?}.into(), get(&goblin_env, {name:?})?);\n"
                ));
            }
            Stmt::For {
                variable,
                start,
                stop,
                step,
                body: loop_body,
            } => {
                body.push_str("    {\n");
                body.push_str(&format!(
                    "    let start = range_integer(({})?, \"range start\")? as i128;\n",
                    generate_expr(start)?
                ));
                body.push_str(&format!(
                    "    let stop = range_integer(({})?, \"range stop\")? as i128;\n",
                    generate_expr(stop)?
                ));
                body.push_str(&format!(
                    "    let step = range_integer(({})?, \"range step\")? as i128;\n",
                    generate_expr(step)?
                ));
                body.push_str(
                    "    if step == 0 { return Err(\"range step cannot be zero.\".into()); }\n",
                );
                body.push_str("    let mut index = start;\n    while if step > 0 { index < stop } else { index > stop } {\n");
                body.push_str("    loop_tick(&mut goblin_loop_steps)?;\n");
                body.push_str(&format!(
                    "    goblin_env.insert({variable:?}.into(), Value::scalar(index as f64)?);\n"
                ));
                body.push_str(&generate_statements(loop_body, blocks)?);
                body.push_str("    index += step;\n    }\n    }\n");
            }
            Stmt::While {
                condition,
                body: loop_body,
            } => {
                body.push_str("    loop {\n");
                body.push_str(&format!(
                    "    if !as_bool(({})?, \"while\")? {{ break; }}\n",
                    generate_expr(condition)?
                ));
                body.push_str("    loop_tick(&mut goblin_loop_steps)?;\n");
                body.push_str(&generate_statements(loop_body, blocks)?);
                body.push_str("    }\n");
            }
            Stmt::If {
                branches,
                else_body,
            } => {
                for (index, (condition, branch_body)) in branches.iter().enumerate() {
                    if index == 0 {
                        body.push_str("    if ");
                    } else {
                        body.push_str("    else if ");
                    }
                    body.push_str(&format!(
                        "as_bool(({})?, \"if\")? {{\n",
                        generate_expr(condition)?
                    ));
                    body.push_str(&generate_statements(branch_body, blocks)?);
                    body.push_str("    }");
                }
                if let Some(branch_body) = else_body {
                    body.push_str(" else {\n");
                    body.push_str(&generate_statements(branch_body, blocks)?);
                    body.push_str("    }");
                }
                body.push('\n');
            }
            Stmt::Switch {
                selector,
                cases,
                default,
            } => {
                body.push_str("    {\n");
                body.push_str(&format!(
                    "    let switch_value = ({})?;\n",
                    generate_expr(selector)?
                ));
                body.push_str("    let mut switch_matched = false;\n");
                for (label, case_body) in cases {
                    body.push_str("    if !switch_matched {\n");
                    body.push_str(&format!(
                        "    let case_value = ({})?;\n",
                        generate_expr(label)?
                    ));
                    body.push_str("    if switch_equal(&switch_value, &case_value)? {\n    switch_matched = true;\n");
                    body.push_str(&generate_statements(case_body, blocks)?);
                    body.push_str("    }\n    }\n");
                }
                if let Some(case_body) = default {
                    body.push_str("    if !switch_matched {\n");
                    body.push_str(&generate_statements(case_body, blocks)?);
                    body.push_str("    }\n");
                }
                body.push_str("    }\n");
            }
            Stmt::InlineRust { index, .. } => {
                let block = blocks
                    .get(*index)
                    .ok_or_else(|| GoblinError::compile("Missing inline Rust block."))?;
                body.push_str(&format!(
                    "    // BEGIN REVIEWED INLINE RUST SHA256={}\n",
                    block.sha256
                ));
                body.push_str("    {\n        let goblin_env = &goblin_env;\n");
                for line in block.code.lines() {
                    body.push_str("        ");
                    body.push_str(line);
                    body.push('\n');
                }
                body.push_str("    }\n    // END REVIEWED INLINE RUST\n");
            }
        }
    }
    Ok(body)
}

fn generate_expr(expression: &Expr) -> Result<String> {
    Ok(match expression {
        Expr::Number(value) => format!("Value::scalar({value:?})"),
        Expr::Bool(value) => format!("Ok::<Value, String>(Value::Bool({value}))"),
        Expr::Text(value) => format!("Ok::<Value, String>(Value::Text({value:?}.into()))"),
        Expr::QuantityLiteral { value, unit } => {
            let unit = crate::quantity::unit(unit)
                .ok_or_else(|| GoblinError::compile(format!("Unknown unit {unit}.")))?;
            format!("Value::q({:?}, {:?})", value * unit.factor, unit.dimension)
        }
        Expr::Array(items) => format!(
            "array_literal(vec![{}])",
            items
                .iter()
                .map(generate_expr)
                .collect::<Result<Vec<_>>>()?
                .join(", ")
        ),
        Expr::Index { array, index } => format!(
            "array_get(({})?, ({})?)",
            generate_expr(array)?,
            generate_expr(index)?
        ),
        Expr::Slice { array, start, stop } => {
            let start = start
                .as_ref()
                .map(|v| generate_expr(v).map(|e| format!("Some(({e})?)")))
                .transpose()?
                .unwrap_or_else(|| "None".into());
            let stop = stop
                .as_ref()
                .map(|v| generate_expr(v).map(|e| format!("Some(({e})?)")))
                .transpose()?
                .unwrap_or_else(|| "None".into());
            format!("array_slice(({})?, {start}, {stop})", generate_expr(array)?)
        }
        Expr::Name {
            source,
            canonical_id,
        } => match canonical_id {
            Some(id) => {
                let constant = by_id(id)
                    .ok_or_else(|| GoblinError::compile(format!("Missing constant {id}.")))?;
                format!(
                    "Value::q({:?}, {:?})",
                    constant.value_si, constant.dimension
                )
            }
            None if source == "argc" => "Value::scalar(goblin_args.len() as f64)".into(),
            None => format!("get(&goblin_env, {source:?})"),
        },
        Expr::Unary { op, expr } => format!("unary({op:?}, ({})?)", generate_expr(expr)?),
        Expr::Binary { op, left, right } => format!(
            "binary({op:?}, ({})?, ({})?)",
            generate_expr(left)?,
            generate_expr(right)?
        ),
        Expr::Compare { op, left, right } => format!(
            "compare({op:?}, ({})?, ({})?)",
            generate_expr(left)?,
            generate_expr(right)?
        ),
        Expr::Call { name, args } if name == "argv" => {
            if args.len() != 1 {
                return Err(GoblinError::compile("argv() requires one index."));
            }
            format!("goblin_argv(({})?, &goblin_args)", generate_expr(&args[0])?)
        }
        Expr::Call { name, args } if name == "len" => {
            if args.len() != 1 {
                return Err(GoblinError::compile("len() requires one array."));
            }
            format!("array_len(({})?)", generate_expr(&args[0])?)
        }
        Expr::Call { name, args } if name == "append" => {
            if args.len() != 2 {
                return Err(GoblinError::compile(
                    "append() requires an array and a value.",
                ));
            }
            format!(
                "array_append(({})?, ({})?)",
                generate_expr(&args[0])?,
                generate_expr(&args[1])?
            )
        }
        Expr::Call { name, args } if name == "input" => {
            if args.len() > 1 {
                return Err(GoblinError::compile("input() accepts zero or one prompt."));
            }
            let prompt = if let Some(expr) = args.first() {
                generate_expr(expr)?
            } else {
                "Ok::<Value, String>(Value::Text(String::new()))".into()
            };
            format!(
                "goblin_input(({prompt})?, &goblin_env, goblin_args.len(), &mut goblin_input_steps)"
            )
        }
        Expr::Call { name, .. } => {
            return Err(GoblinError::compile(format!(
                "Function {name} cannot be nested inside a compiled expression."
            )));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_source;
    #[test]
    fn generated_source_contains_direct_operations() {
        let parsed = parse_source("x = 2 + 3\nprint(\"x = {x}\")\n").unwrap();
        let source = generate(&parsed.program, &[]).unwrap();
        assert!(source.contains("binary('+', (Value::scalar(2.0))?, (Value::scalar(3.0))?)"));
        assert!(!source.contains("parse_source"));
    }
}

use crate::ast::{Expr, Program, Stmt};
use crate::constants::by_id;
use crate::error::{GoblinError, Result};
use crate::fits::{ColumnValue, FitsFile, HeaderValue};
use crate::interaction::{InputPolicy, Interaction};
use crate::output::{
    self, GeneratedOutput, MAX_PLOT_POINTS, Sampling, ScatterRequest, delimited_bytes,
};
use crate::quantity::{DIMENSIONLESS, Quantity, format_dimension};
use crate::text_runtime;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Quantity(Quantity),
    Text(String),
    Bool(bool),
    Array(Vec<Value>),
}

impl Value {
    pub fn render(&self) -> String {
        match self {
            Self::Quantity(value) => value.render(),
            Self::Text(value) => value.clone(),
            Self::Bool(value) => value.to_string(),
            Self::Array(items) => format!(
                "[{}]",
                items
                    .iter()
                    .map(Value::render)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    pub fn quantity(&self, context: &str) -> Result<Quantity> {
        match self {
            Self::Quantity(value) => Ok(*value),
            _ => Err(GoblinError::new(
                "G000",
                format!("{context} requires numeric quantities."),
            )),
        }
    }

    pub fn text(&self, context: &str) -> Result<&str> {
        match self {
            Self::Text(value) => Ok(value),
            _ => Err(GoblinError::data(format!("{context} requires text."))),
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Self::Quantity(value) => serde_json::json!({
                "value_si": value.value_si,
                "dimension": value.dimension,
                "unit_si": format_dimension(value.dimension),
            }),
            Self::Text(value) => serde_json::Value::String(value.clone()),
            Self::Bool(value) => serde_json::Value::Bool(*value),
            Self::Array(items) => {
                serde_json::json!({"kind": "array", "items": items.iter().map(Value::to_json).collect::<Vec<_>>() })
            }
        }
    }
}

pub const MAX_LOOP_ITERATIONS: u64 = 1_000_000;
pub const MAX_ARRAY_ITEMS: usize = 100_000;
pub const MAX_FUNCTION_DEPTH: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConstantUse {
    pub id: String,
    pub source_alias: String,
    pub value_si: String,
    pub dimension: [i32; 5],
    pub status: String,
    pub registry: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataImport {
    pub path: String,
    pub sha256: String,
    pub byte_count: u64,
    pub format: String,
    pub access: Vec<String>,
    pub evidence_path: Option<String>,
    pub evidence_sha256: Option<String>,
}

#[derive(Debug)]
struct LoadedData {
    fits: FitsFile,
    access: Vec<String>,
}

#[derive(Debug)]
pub struct Evaluation {
    pub env: HashMap<String, Value>,
    pub stdout: Vec<String>,
    pub sealed: BTreeMap<String, Value>,
    pub generated: BTreeMap<String, GeneratedOutput>,
    pub constants_used: BTreeMap<String, ConstantUse>,
    pub paranoid: bool,
    pub interaction: Interaction,
    loop_iterations: u64,
    base_dir: PathBuf,
    data: BTreeMap<PathBuf, LoadedData>,
    functions: HashMap<String, (Vec<String>, Vec<Stmt>)>,
    function_depth: usize,
    return_value: Option<Value>,
}

impl Evaluation {
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        Self {
            env: HashMap::new(),
            stdout: Vec::new(),
            sealed: BTreeMap::new(),
            generated: BTreeMap::new(),
            constants_used: BTreeMap::new(),
            paranoid: false,
            interaction: Interaction::default(),
            loop_iterations: 0,
            base_dir: base_dir.as_ref().to_path_buf(),
            data: BTreeMap::new(),
            functions: HashMap::new(),
            function_depth: 0,
            return_value: None,
        }
    }

    pub fn configure_interaction(&mut self, argv: Vec<String>, policy: InputPolicy) -> Result<()> {
        self.interaction.configure(argv, policy)?;
        Ok(())
    }

    pub fn eval_program(mut self, program: &Program) -> Result<Self> {
        self.register_functions(program)?;
        for statement in &program.statements {
            self.eval_stmt(statement)?;
        }
        Ok(self)
    }

    pub fn register_functions(&mut self, program: &Program) -> Result<()> {
        for statement in &program.statements {
            if let Stmt::Function { name, params, body } = statement
                && self
                    .functions
                    .insert(name.clone(), (params.clone(), body.clone()))
                    .is_some()
            {
                return Err(GoblinError::parse(format!("Duplicate g_func {name:?}.")));
            }
        }
        Ok(())
    }

    pub fn eval_stmt(&mut self, statement: &Stmt) -> Result<()> {
        match statement {
            Stmt::Function { .. } => Ok(()),
            Stmt::Return(expr) => {
                self.return_value = Some(self.eval_expr(expr)?);
                Ok(())
            }
            Stmt::Directive(name) if name == "GO_PARANOID" => {
                self.paranoid = true;
                Ok(())
            }
            Stmt::Directive(name) => Err(GoblinError::parse(format!("Unknown directive: {name}"))),
            Stmt::Assign { name, expr } => {
                let value = self.eval_expr(expr)?;
                self.env.insert(name.clone(), value);
                Ok(())
            }
            Stmt::IndexAssign { name, index, expr } => {
                let index = array_index(&self.eval_expr(index)?, "array assignment")?;
                let value = self.eval_expr(expr)?;
                let current = self.env.get_mut(name).ok_or_else(|| unknown(name))?;
                let Value::Array(items) = current else {
                    return Err(GoblinError::new(
                        "G203",
                        "Indexed assignment requires an array.",
                    ));
                };
                if index >= items.len() {
                    return Err(GoblinError::new(
                        "G203",
                        format!(
                            "Array index {index} is out of bounds for length {}.",
                            items.len()
                        ),
                    ));
                }
                require_same_array_type(&items[0], &value)?;
                items[index] = value;
                Ok(())
            }
            Stmt::Expression(expr) => {
                self.eval_expr(expr)?;
                Ok(())
            }
            Stmt::Seal(name) => {
                let value = self.env.get(name).cloned().ok_or_else(|| unknown(name))?;
                self.sealed.insert(name.clone(), value);
                Ok(())
            }
            Stmt::For {
                variable,
                start,
                stop,
                step,
                body,
            } => {
                let start = loop_integer(&self.eval_expr(start)?, "range start")? as i128;
                let stop = loop_integer(&self.eval_expr(stop)?, "range stop")? as i128;
                let step = loop_integer(&self.eval_expr(step)?, "range step")? as i128;
                if step == 0 {
                    return Err(GoblinError::numeric("range step cannot be zero."));
                }
                let mut index = start;
                while if step > 0 { index < stop } else { index > stop } {
                    self.loop_tick()?;
                    self.env.insert(
                        variable.clone(),
                        Value::Quantity(Quantity::scalar(index as f64)?),
                    );
                    for statement in body {
                        self.eval_stmt(statement)?;
                        if self.return_value.is_some() {
                            return Ok(());
                        }
                    }
                    index += step;
                }
                Ok(())
            }
            Stmt::While { condition, body } => loop {
                let condition = self.eval_expr(condition)?;
                let Value::Bool(continue_loop) = condition else {
                    return Err(GoblinError::new(
                        "G203",
                        "while condition must be true or false.",
                    ));
                };
                if !continue_loop {
                    return Ok(());
                }
                self.loop_tick()?;
                for statement in body {
                    self.eval_stmt(statement)?;
                    if self.return_value.is_some() {
                        return Ok(());
                    }
                }
            },
            Stmt::If {
                branches,
                else_body,
            } => {
                for (condition, body) in branches {
                    let value = self.eval_expr(condition)?;
                    let Value::Bool(yes) = value else {
                        return Err(GoblinError::new(
                            "G203",
                            "if condition must be true or false.",
                        ));
                    };
                    if yes {
                        for statement in body {
                            self.eval_stmt(statement)?;
                            if self.return_value.is_some() {
                                return Ok(());
                            }
                        }
                        return Ok(());
                    }
                }
                if let Some(body) = else_body {
                    for statement in body {
                        self.eval_stmt(statement)?;
                        if self.return_value.is_some() {
                            return Ok(());
                        }
                    }
                }
                Ok(())
            }
            Stmt::Switch {
                selector,
                cases,
                default,
            } => {
                let selected = self.eval_expr(selector)?;
                for (label, body) in cases {
                    let candidate = self.eval_expr(label)?;
                    if compare_values("==", &selected, &candidate)? {
                        for statement in body {
                            self.eval_stmt(statement)?;
                            if self.return_value.is_some() {
                                return Ok(());
                            }
                        }
                        return Ok(());
                    }
                }
                if let Some(body) = default {
                    for statement in body {
                        self.eval_stmt(statement)?;
                        if self.return_value.is_some() {
                            return Ok(());
                        }
                    }
                }
                Ok(())
            }
            Stmt::InlineRust { sha256, .. } => Err(GoblinError::protocol(
                "INLINE_RUST_REQUIRES_NATIVE_COMPILATION",
                format!(
                    "Inline Rust cannot execute in interpreter mode.\n\nBLOCK_SHA256={sha256}\n\nUse --compile and authorize the reviewed block's exact hash."
                ),
            )),
        }
    }

    fn loop_tick(&mut self) -> Result<()> {
        if self.loop_iterations >= MAX_LOOP_ITERATIONS {
            return Err(GoblinError::new(
                "G203",
                format!(
                    "LOOP LIMIT EXCEEDED\n\nA run may execute at most {MAX_LOOP_ITERATIONS} loop-body iterations in total. This failure is preserved in the run receipt."
                ),
            ));
        }
        self.loop_iterations += 1;
        Ok(())
    }

    pub fn eval_expr(&mut self, expression: &Expr) -> Result<Value> {
        match expression {
            Expr::Number(value) => Ok(Value::Quantity(Quantity::scalar(*value)?)),
            Expr::Bool(value) => Ok(Value::Bool(*value)),
            Expr::Text(value) => Ok(Value::Text(value.clone())),
            Expr::QuantityLiteral { value, unit } => {
                Ok(Value::Quantity(Quantity::from_unit(*value, unit)?))
            }
            Expr::Array(items) => {
                if items.len() > MAX_ARRAY_ITEMS {
                    return Err(GoblinError::new("G203", "Array item limit exceeded."));
                }
                let values = items
                    .iter()
                    .map(|expr| self.eval_expr(expr))
                    .collect::<Result<Vec<_>>>()?;
                validate_array(&values)?;
                Ok(Value::Array(values))
            }
            Expr::Index { array, index } => {
                let array = self.eval_expr(array)?;
                let index = array_index(&self.eval_expr(index)?, "array index")?;
                let Value::Array(items) = array else {
                    return Err(GoblinError::new("G203", "Indexing requires an array."));
                };
                items.get(index).cloned().ok_or_else(|| {
                    GoblinError::new(
                        "G203",
                        format!(
                            "Array index {index} is out of bounds for length {}.",
                            items.len()
                        ),
                    )
                })
            }
            Expr::Slice { array, start, stop } => {
                let array = self.eval_expr(array)?;
                let Value::Array(items) = array else {
                    return Err(GoblinError::new("G203", "Slicing requires an array."));
                };
                let start = if let Some(start) = start {
                    array_index(&self.eval_expr(start)?, "slice start")?
                } else {
                    0
                };
                let stop = if let Some(stop) = stop {
                    array_index(&self.eval_expr(stop)?, "slice stop")?
                } else {
                    items.len()
                };
                if start > stop || stop > items.len() {
                    return Err(GoblinError::new(
                        "G203",
                        format!("Invalid slice [{start}:{stop}] for length {}.", items.len()),
                    ));
                }
                Ok(Value::Array(items[start..stop].to_vec()))
            }
            Expr::Name {
                source,
                canonical_id,
            } => {
                if let Some(id) = canonical_id {
                    let constant = by_id(id).ok_or_else(|| {
                        GoblinError::unknown(format!("Constant registry lost {id}."))
                    })?;
                    self.constants_used
                        .entry(id.clone())
                        .or_insert_with(|| ConstantUse {
                            id: id.clone(),
                            source_alias: source.clone(),
                            value_si: constant.value_si.to_string(),
                            dimension: constant.dimension,
                            status: constant.status.into(),
                            registry: constant.registry.into(),
                        });
                    Ok(Value::Quantity(constant.quantity()))
                } else {
                    if source == "argc" {
                        self.interaction.arguments_accessed = true;
                        return Quantity::scalar(self.interaction.argv.len() as f64)
                            .map(Value::Quantity);
                    }
                    self.env.get(source).cloned().ok_or_else(|| unknown(source))
                }
            }
            Expr::Unary { op, expr } => {
                let value = self.eval_expr(expr)?.quantity("Unary arithmetic")?;
                match op {
                    '+' => Ok(Value::Quantity(value)),
                    '-' => Ok(Value::Quantity(Quantity::new(
                        -value.value_si,
                        value.dimension,
                    )?)),
                    _ => Err(GoblinError::parse(format!("Unknown unary operator {op}."))),
                }
            }
            Expr::Binary { op, left, right } => {
                let left = self.eval_expr(left)?;
                let right = self.eval_expr(right)?;
                if let (Value::Text(left), Value::Text(right)) = (&left, &right)
                    && *op == '+'
                {
                    return text_runtime::concat(left, right)
                        .map(Value::Text)
                        .map_err(|message| GoblinError::new("G203", message));
                }
                let left = left.quantity("Arithmetic")?;
                let right = right.quantity("Arithmetic")?;
                let value = match op {
                    '+' => left.checked_add(right)?,
                    '-' => left.checked_sub(right)?,
                    '*' => left.checked_mul(right)?,
                    '/' => left.checked_div(right)?,
                    '^' => {
                        if right.dimension != DIMENSIONLESS
                            || right.value_si.fract() != 0.0
                            || right.value_si < i32::MIN as f64
                            || right.value_si > i32::MAX as f64
                        {
                            return Err(GoblinError::dimension(
                                "Goblin++ requires an integer dimensionless exponent.",
                            ));
                        }
                        left.powi(right.value_si as i32)?
                    }
                    _ => return Err(GoblinError::parse(format!("Unknown operator {op}."))),
                };
                Ok(Value::Quantity(value))
            }
            Expr::Compare { op, left, right } => {
                let left = self.eval_expr(left)?;
                let right = self.eval_expr(right)?;
                Ok(Value::Bool(compare_values(op, &left, &right)?))
            }
            Expr::Call { name, args } => self.eval_call(name, args),
        }
    }

    fn eval_call(&mut self, name: &str, args: &[Expr]) -> Result<Value> {
        match name {
            "len" => {
                require_args(name, args, 1)?;
                let value = self.eval_expr(&args[0])?;
                let length = match value {
                    Value::Array(items) => items.len(),
                    Value::Text(text) => text_runtime::length(&text),
                    _ => return Err(GoblinError::new("G203", "len() requires an array or text.")),
                };
                Quantity::scalar(length as f64).map(Value::Quantity)
            }
            "to_text" => {
                require_args(name, args, 1)?;
                let value = self.eval_expr(&args[0])?;
                if matches!(value, Value::Array(_)) {
                    return Err(GoblinError::new(
                        "G203",
                        "to_text() does not render arrays; use str_join() for text arrays.",
                    ));
                }
                text_runtime::bounded(value.render())
                    .map(Value::Text)
                    .map_err(|message| GoblinError::new("G203", message))
            }
            "parse_number" => {
                require_args(name, args, 1)?;
                let value = self.eval_expr(&args[0])?;
                let value = value.text(name)?;
                let number = text_runtime::parse_number(value).map_err(GoblinError::numeric)?;
                Quantity::scalar(number).map(Value::Quantity)
            }
            "str_trim" | "str_contains" | "str_replace" | "str_split" | "str_join" => {
                self.eval_string_call(name, args)
            }
            "append" => {
                require_args(name, args, 2)?;
                let array = self.eval_expr(&args[0])?;
                let value = self.eval_expr(&args[1])?;
                let Value::Array(mut items) = array else {
                    return Err(GoblinError::new("G203", "append() requires an array."));
                };
                if items.len() >= MAX_ARRAY_ITEMS {
                    return Err(GoblinError::new("G203", "Array item limit exceeded."));
                }
                if let Some(first) = items.first() {
                    require_same_array_type(first, &value)?;
                } else {
                    validate_array(std::slice::from_ref(&value))?;
                }
                items.push(value);
                Ok(Value::Array(items))
            }
            "argv" => {
                require_args(name, args, 1)?;
                let index = integer_scalar(&self.eval_expr(&args[0])?, name)? as usize;
                Ok(Value::Text(self.interaction.argv(index)?.to_string()))
            }
            "input" => {
                require_args_one_of(name, args, &[0, 1])?;
                let prompt = if let Some(expr) = args.first() {
                    let value = self.eval_expr(expr)?;
                    self.interpolate(value.text(name)?)?
                } else {
                    String::new()
                };
                Ok(Value::Text(self.interaction.input(prompt)?))
            }
            "print" => {
                let values = args
                    .iter()
                    .map(|arg| self.eval_expr(arg))
                    .collect::<Result<Vec<_>>>()?;
                let text = if values.len() == 1 {
                    match &values[0] {
                        Value::Text(template) => self.interpolate(template)?,
                        value => value.render(),
                    }
                } else {
                    values
                        .iter()
                        .map(Value::render)
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                self.stdout.push(text);
                Quantity::scalar(0.0).map(Value::Quantity)
            }
            "printf" => {
                if args.is_empty() {
                    return Err(GoblinError::new(
                        "G000",
                        "printf() requires a format string.",
                    ));
                }
                let values = args
                    .iter()
                    .map(|arg| self.eval_expr(arg))
                    .collect::<Result<Vec<_>>>()?;
                let format = values[0]
                    .text("printf()")
                    .map_err(|_| GoblinError::new("G000", "printf() requires a format string."))?;
                let text = printf(format, &values[1..]).map_err(|error| {
                    GoblinError::new("G000", format!("printf formatting failed: {error}"))
                })?;
                self.stdout.push(text);
                Quantity::scalar(0.0).map(Value::Quantity)
            }
            "fits_header" => {
                require_args_one_of(name, args, &[2, 3])?;
                let values = self.eval_args(args)?;
                let path = values[0].text(name)?.to_string();
                let (hdu, key) = if values.len() == 2 {
                    (0, values[1].text(name)?.to_string())
                } else {
                    (
                        integer_scalar(&values[1], name)? as usize,
                        values[2].text(name)?.to_string(),
                    )
                };
                let fits = self.load_fits(&path, format!("header:hdu={hdu}:key={key}"))?;
                header_value(fits.header_value(hdu, &key)?)
            }
            "fits_axis" => {
                require_args_one_of(name, args, &[2, 3])?;
                let values = self.eval_args(args)?;
                let path = values[0].text(name)?.to_string();
                let (hdu, axis) = if values.len() == 2 {
                    (0, integer_scalar(&values[1], name)? as usize)
                } else {
                    (
                        integer_scalar(&values[1], name)? as usize,
                        integer_scalar(&values[2], name)? as usize,
                    )
                };
                let fits = self.load_fits(&path, format!("axis:hdu={hdu}:axis={axis}"))?;
                Quantity::scalar(fits.axis(hdu, axis)? as f64).map(Value::Quantity)
            }
            "fits_count" => {
                require_args_one_of(name, args, &[1, 2])?;
                let values = self.eval_args(args)?;
                let path = values[0].text(name)?.to_string();
                let hdu = if values.len() == 1 {
                    0
                } else {
                    integer_scalar(&values[1], name)? as usize
                };
                let fits = self.load_fits(&path, format!("count:hdu={hdu}"))?;
                Quantity::scalar(fits.pixel_count(hdu)? as f64).map(Value::Quantity)
            }
            "fits_pixel" => {
                require_args_one_of(name, args, &[2, 3])?;
                let values = self.eval_args(args)?;
                let path = values[0].text(name)?.to_string();
                let (hdu, index) = if values.len() == 2 {
                    (0, integer_scalar(&values[1], name)? as usize)
                } else {
                    (
                        integer_scalar(&values[1], name)? as usize,
                        integer_scalar(&values[2], name)? as usize,
                    )
                };
                let fits = self.load_fits(&path, format!("pixel:hdu={hdu}:index={index}"))?;
                let value = fits.pixel(hdu, index)?;
                if value.is_nan() {
                    return Err(GoblinError::data(format!(
                        "FITS HDU {hdu} pixel {index} is BLANK/NaN."
                    )));
                }
                Quantity::scalar(value).map(Value::Quantity)
            }
            "fits_mean" => {
                require_args_one_of(name, args, &[1, 2])?;
                let values = self.eval_args(args)?;
                let path = values[0].text(name)?.to_string();
                let hdu = if values.len() == 1 {
                    0
                } else {
                    integer_scalar(&values[1], name)? as usize
                };
                let fits = self.load_fits(&path, format!("mean:hdu={hdu}"))?;
                Quantity::scalar(fits.mean(hdu)?).map(Value::Quantity)
            }
            "fits_hdu_count" => {
                require_args(name, args, 1)?;
                let path = self.eval_expr(&args[0])?;
                let path = path.text(name)?.to_string();
                let fits = self.load_fits(&path, "hdu-count".into())?;
                Quantity::scalar(fits.hdu_count() as f64).map(Value::Quantity)
            }
            "fits_rows" => {
                require_args(name, args, 2)?;
                let values = self.eval_args(args)?;
                let path = values[0].text(name)?.to_string();
                let hdu = integer_scalar(&values[1], name)? as usize;
                let fits = self.load_fits(&path, format!("rows:hdu={hdu}"))?;
                Quantity::scalar(fits.row_count(hdu)? as f64).map(Value::Quantity)
            }
            "fits_columns" => {
                require_args(name, args, 2)?;
                let values = self.eval_args(args)?;
                let path = values[0].text(name)?.to_string();
                let hdu = integer_scalar(&values[1], name)? as usize;
                let fits = self.load_fits(&path, format!("columns:hdu={hdu}"))?;
                Quantity::scalar(fits.column_count(hdu)? as f64).map(Value::Quantity)
            }
            "fits_column" => {
                require_args_one_of(name, args, &[4, 5])?;
                let values = self.eval_args(args)?;
                let path = values[0].text(name)?.to_string();
                let hdu = integer_scalar(&values[1], name)? as usize;
                let column = values[2].text(name)?.to_string();
                let row = integer_scalar(&values[3], name)? as usize;
                let element = if values.len() == 5 {
                    Some(integer_scalar(&values[4], name)? as usize)
                } else {
                    None
                };
                let fits = self.load_fits(
                    &path,
                    format!("column:hdu={hdu}:name={column}:row={row}:element={element:?}"),
                )?;
                column_value(
                    fits.column_value(hdu, &column, row, element)?,
                    hdu,
                    &column,
                    row,
                )
            }
            "fits_column_valid_count"
            | "fits_column_mean"
            | "fits_column_min"
            | "fits_column_max" => {
                require_args(name, args, 3)?;
                let values = self.eval_args(args)?;
                let path = values[0].text(name)?.to_string();
                let hdu = integer_scalar(&values[1], name)? as usize;
                let column = values[2].text(name)?.to_string();
                let fits = self.load_fits(&path, format!("{name}:hdu={hdu}:name={column}"))?;
                let stats = fits.column_stats(hdu, &column)?;
                let value = match name {
                    "fits_column_valid_count" => stats.valid_count as f64,
                    "fits_column_mean" => stats.mean,
                    "fits_column_min" => stats.minimum,
                    "fits_column_max" => stats.maximum,
                    _ => unreachable!(),
                };
                Quantity::scalar(value).map(Value::Quantity)
            }
            "fits_select_stats" => {
                require_args_one_of(name, args, &[6, 7])?;
                let values = self.eval_args(args)?;
                let path = values[0].text(name)?.to_string();
                let hdu = integer_scalar(&values[1], name)? as usize;
                let selection = values[2].text(name)?.to_string();
                let lower = values[3].quantity(name)?;
                let upper = values[4].quantity(name)?;
                if lower.dimension != DIMENSIONLESS || upper.dimension != DIMENSIONLESS {
                    return Err(GoblinError::data(
                        "fits_select_stats() bounds must be dimensionless.",
                    ));
                }
                let value_column = values[5].text(name)?.to_string();
                let weight_column = values
                    .get(6)
                    .map(|value| value.text(name).map(str::to_string))
                    .transpose()?;
                let access = serde_json::json!({
                    "operation": name,
                    "hdu": hdu,
                    "selection_column": selection,
                    "lower_inclusive": lower.value_si,
                    "upper_exclusive": upper.value_si,
                    "value_column": value_column,
                    "weight_column": weight_column,
                })
                .to_string();
                let fits = self.load_fits(&path, access)?;
                let stats = fits.selected_column_stats(
                    hdu,
                    &selection,
                    lower.value_si,
                    upper.value_si,
                    &value_column,
                    weight_column.as_deref(),
                )?;
                [
                    stats.selected_rows as f64,
                    stats.used_rows as f64,
                    stats.weight_sum,
                    stats.mean,
                ]
                .into_iter()
                .map(|value| Quantity::scalar(value).map(Value::Quantity))
                .collect::<Result<Vec<_>>>()
                .map(Value::Array)
            }
            "write_text" => {
                if args.len() < 2 {
                    return Err(GoblinError::parse(format!(
                        "write_text() requires a filename and at least one value, got {} argument(s).",
                        args.len()
                    )));
                }
                let values = self.eval_args(args)?;
                let name = values[0].text(name)?.to_string();
                let extension = output::extension(&name)?;
                let media_type = match extension.as_str() {
                    "txt" => "text/plain; charset=utf-8",
                    "md" => "text/markdown; charset=utf-8",
                    _ => {
                        return Err(GoblinError::artifact(
                            "write_text() output must use .txt or .md.",
                        ));
                    }
                };
                let mut text = values[1..]
                    .iter()
                    .map(|value| self.export_cell(value))
                    .collect::<Result<Vec<_>>>()?
                    .join("\n");
                text.push('\n');
                let artifact = GeneratedOutput::new(
                    &name,
                    media_type,
                    "write_text",
                    text.into_bytes(),
                    serde_json::json!({"kind": "text", "lines": values.len() - 1}),
                )?;
                self.add_output(artifact)?;
                Ok(Value::Text(name))
            }
            "write_csv" | "write_tsv" => {
                if args.len() < 4 {
                    return Err(GoblinError::parse(format!(
                        "{name}() requires a filename, column count, and at least one complete row."
                    )));
                }
                let values = self.eval_args(args)?;
                let file_name = values[0].text(name)?.to_string();
                let columns = usize::try_from(integer_scalar(&values[1], name)?)
                    .map_err(|_| GoblinError::artifact("Column count is too large."))?;
                if columns == 0 || columns > 1_024 {
                    return Err(GoblinError::artifact(
                        "Delimited output column count must be between 1 and 1024.",
                    ));
                }
                if !(values.len() - 2).is_multiple_of(columns) {
                    return Err(GoblinError::artifact(format!(
                        "{name}() received {} cells, which is not divisible by {columns} columns.",
                        values.len() - 2
                    )));
                }
                let (expected_extension, delimiter, media_type) = if name == "write_csv" {
                    ("csv", b',', "text/csv; charset=utf-8")
                } else {
                    ("tsv", b'\t', "text/tab-separated-values; charset=utf-8")
                };
                if output::extension(&file_name)? != expected_extension {
                    return Err(GoblinError::artifact(format!(
                        "{name}() output must use .{expected_extension}."
                    )));
                }
                let cells = values[2..]
                    .iter()
                    .map(|value| self.export_cell(value))
                    .collect::<Result<Vec<_>>>()?;
                let rows = cells
                    .chunks(columns)
                    .map(<[String]>::to_vec)
                    .collect::<Vec<_>>();
                let artifact = GeneratedOutput::new(
                    &file_name,
                    media_type,
                    name,
                    delimited_bytes(&rows, delimiter),
                    serde_json::json!({
                        "kind": expected_extension,
                        "columns": columns,
                        "rows": rows.len(),
                        "first_row_is_header": true,
                    }),
                )?;
                self.add_output(artifact)?;
                Ok(Value::Text(file_name))
            }
            "write_json" => {
                if args.len() < 3 || args.len().is_multiple_of(2) {
                    return Err(GoblinError::parse(format!(
                        "write_json() requires a filename followed by one or more key/value pairs, got {} argument(s).",
                        args.len()
                    )));
                }
                let values = self.eval_args(args)?;
                let file_name = values[0].text(name)?.to_string();
                if output::extension(&file_name)? != "json" {
                    return Err(GoblinError::artifact("write_json() output must use .json."));
                }
                let mut object = serde_json::Map::new();
                for pair in values[1..].as_chunks::<2>().0 {
                    let key = pair[0].text(name)?.to_string();
                    if object.contains_key(&key) {
                        return Err(GoblinError::artifact(format!(
                            "write_json() repeats key {key:?}."
                        )));
                    }
                    object.insert(key, export_json_value(&pair[1]));
                }
                let mut bytes = serde_json::to_vec_pretty(&serde_json::Value::Object(object))?;
                bytes.push(b'\n');
                let artifact = GeneratedOutput::new(
                    &file_name,
                    "application/json",
                    name,
                    bytes,
                    serde_json::json!({"kind": "json", "keys": (values.len() - 1) / 2}),
                )?;
                self.add_output(artifact)?;
                Ok(Value::Text(file_name))
            }
            "plot_fits_histogram" => {
                require_args(name, args, 7)?;
                let values = self.eval_args(args)?;
                let output_name = values[0].text(name)?.to_string();
                let path = values[1].text(name)?.to_string();
                let hdu = integer_scalar(&values[2], name)? as usize;
                let column = values[3].text(name)?.to_string();
                let bins = integer_scalar(&values[4], name)? as usize;
                let requested = plot_points(&values[5], name)?;
                let title = values[6].text(name)?.to_string();
                let (sample, input_sha256) = {
                    let fits = self.load_fits(
                        &path,
                        format!("plot-histogram:hdu={hdu}:column={column}:sample={requested}"),
                    )?;
                    (
                        fits.numeric_column_sample(hdu, &column, requested)?,
                        fits.sha256.clone(),
                    )
                };
                let sampling = Sampling {
                    population_rows: sample.population_rows,
                    requested_points: requested,
                    examined_rows: sample.examined_rows,
                    valid_points: sample.values.len(),
                };
                let artifact = output::histogram(
                    &output_name,
                    &title,
                    &column,
                    &sample.values,
                    bins,
                    sampling,
                    &input_sha256,
                )?;
                self.add_output(artifact)?;
                Ok(Value::Text(output_name))
            }
            "plot_fits_scatter" => {
                require_args(name, args, 7)?;
                let values = self.eval_args(args)?;
                let output_name = values[0].text(name)?.to_string();
                let path = values[1].text(name)?.to_string();
                let hdu = integer_scalar(&values[2], name)? as usize;
                let x_column = values[3].text(name)?.to_string();
                let y_column = values[4].text(name)?.to_string();
                let requested = plot_points(&values[5], name)?;
                let title = values[6].text(name)?.to_string();
                let (sample, input_sha256) = {
                    let fits = self.load_fits(
                        &path,
                        format!(
                            "plot-scatter:hdu={hdu}:x={x_column}:y={y_column}:sample={requested}"
                        ),
                    )?;
                    (
                        fits.paired_numeric_column_sample(hdu, &x_column, &y_column, requested)?,
                        fits.sha256.clone(),
                    )
                };
                let sampling = Sampling {
                    population_rows: sample.population_rows,
                    requested_points: requested,
                    examined_rows: sample.examined_rows,
                    valid_points: sample.points.len(),
                };
                let artifact = output::scatter(ScatterRequest {
                    name: &output_name,
                    title: &title,
                    x_label: &x_column,
                    y_label: &y_column,
                    points: &sample.points,
                    sampling,
                    input_sha256: &input_sha256,
                })?;
                self.add_output(artifact)?;
                Ok(Value::Text(output_name))
            }
            _ => self.eval_user_function(name, args),
        }
    }

    fn eval_string_call(&mut self, name: &str, args: &[Expr]) -> Result<Value> {
        let expected = match name {
            "str_trim" => 1,
            "str_contains" | "str_split" | "str_join" => 2,
            "str_replace" => 3,
            _ => return Err(unknown(name)),
        };
        require_args(name, args, expected)?;
        let values = self.eval_args(args)?;
        let result = match name {
            "str_trim" => Value::Text(
                text_runtime::bounded(values[0].text(name)?.trim().to_string())
                    .map_err(|message| GoblinError::new("G203", message))?,
            ),
            "str_contains" => Value::Bool(values[0].text(name)?.contains(values[1].text(name)?)),
            "str_replace" => Value::Text(
                text_runtime::replace(
                    values[0].text(name)?,
                    values[1].text(name)?,
                    values[2].text(name)?,
                )
                .map_err(|message| GoblinError::new("G203", message))?,
            ),
            "str_split" => Value::Array(
                text_runtime::split(values[0].text(name)?, values[1].text(name)?)
                    .map_err(|message| GoblinError::new("G203", message))?
                    .into_iter()
                    .map(Value::Text)
                    .collect(),
            ),
            "str_join" => {
                let separator = values[0].text(name)?;
                let Value::Array(items) = &values[1] else {
                    return Err(GoblinError::new(
                        "G203",
                        "str_join() requires an array of text.",
                    ));
                };
                let parts = items
                    .iter()
                    .map(|item| item.text(name).map(str::to_string))
                    .collect::<Result<Vec<_>>>()?;
                Value::Text(
                    text_runtime::join(separator, &parts)
                        .map_err(|message| GoblinError::new("G203", message))?,
                )
            }
            _ => unreachable!(),
        };
        Ok(result)
    }

    fn eval_user_function(&mut self, name: &str, args: &[Expr]) -> Result<Value> {
        let (params, body) = self
            .functions
            .get(name)
            .cloned()
            .ok_or_else(|| unknown(name))?;
        if args.len() != params.len() {
            return Err(GoblinError::new(
                "G203",
                format!(
                    "g_func {name} expects {} argument(s), got {}.",
                    params.len(),
                    args.len()
                ),
            ));
        }
        if self.function_depth >= MAX_FUNCTION_DEPTH {
            return Err(GoblinError::new(
                "G203",
                format!("g_func call depth exceeded {MAX_FUNCTION_DEPTH}."),
            ));
        }
        let values = args
            .iter()
            .map(|arg| self.eval_expr(arg))
            .collect::<Result<Vec<_>>>()?;
        let local = params.into_iter().zip(values).collect();
        let previous_env = std::mem::replace(&mut self.env, local);
        let previous_return = self.return_value.take();
        self.function_depth += 1;
        let result = (|| {
            for statement in &body {
                self.eval_stmt(statement)?;
                if let Some(value) = self.return_value.take() {
                    return Ok(value);
                }
            }
            Err(GoblinError::new(
                "G203",
                format!("g_func {name} reached the end without return."),
            ))
        })();
        self.function_depth -= 1;
        self.env = previous_env;
        self.return_value = previous_return;
        result
    }

    fn export_cell(&self, value: &Value) -> Result<String> {
        match value {
            Value::Text(template) => self.interpolate(template),
            value => Ok(value.render()),
        }
    }

    fn add_output(&mut self, artifact: GeneratedOutput) -> Result<()> {
        if self.generated.contains_key(&artifact.name) {
            return Err(GoblinError::artifact(format!(
                "Generated output {:?} is declared more than once. Goblin++ refuses ambiguous overwrites.",
                artifact.name
            )));
        }
        self.generated.insert(artifact.name.clone(), artifact);
        Ok(())
    }

    fn load_fits(&mut self, requested: &str, access: String) -> Result<&FitsFile> {
        let input = Path::new(requested);
        let path = if input.is_absolute() {
            input.to_path_buf()
        } else {
            self.base_dir.join(input)
        };
        let metadata = std::fs::symlink_metadata(&path).map_err(|error| {
            GoblinError::data(format!(
                "Unable to inspect FITS input {}: {error}",
                path.display()
            ))
        })?;
        if metadata.file_type().is_symlink() {
            return Err(GoblinError::data(format!(
                "Refusing symbolic-link FITS input: {}",
                path.display()
            )));
        }
        let key = path.canonicalize().map_err(|error| {
            GoblinError::data(format!(
                "Unable to resolve FITS input {}: {error}",
                path.display()
            ))
        })?;
        if !self.data.contains_key(&key) {
            self.data.insert(
                key.clone(),
                LoadedData {
                    fits: FitsFile::open(&key)?,
                    access: Vec::new(),
                },
            );
        }
        let loaded = self.data.get_mut(&key).unwrap();
        if !loaded.access.contains(&access) {
            loaded.access.push(access);
            loaded.access.sort();
        }
        Ok(&loaded.fits)
    }

    fn eval_args(&mut self, args: &[Expr]) -> Result<Vec<Value>> {
        args.iter()
            .map(|argument| self.eval_expr(argument))
            .collect()
    }

    fn interpolate(&self, template: &str) -> Result<String> {
        let mut output = String::new();
        let mut cursor = 0;
        while let Some(relative) = template[cursor..].find('{') {
            let start = cursor + relative;
            output.push_str(&template[cursor..start]);
            let end = template[start + 1..]
                .find('}')
                .map(|value| start + 1 + value)
                .ok_or_else(|| GoblinError::parse("Unclosed print interpolation."))?;
            let field = &template[start + 1..end];
            let (name, spec) = field
                .split_once(':')
                .map_or((field, None), |(name, spec)| (name, Some(spec)));
            if name.is_empty()
                || !name.chars().enumerate().all(|(index, ch)| {
                    if index == 0 {
                        ch.is_ascii_alphabetic() || ch == '_'
                    } else {
                        ch.is_ascii_alphanumeric() || ch == '_'
                    }
                })
            {
                return Err(GoblinError::parse(format!(
                    "Invalid print interpolation {{{field}}}."
                )));
            }
            let argc_value = Value::Quantity(Quantity::scalar(self.interaction.argv.len() as f64)?);
            let value = if name == "argc" {
                &argc_value
            } else {
                self.env.get(name).ok_or_else(|| unknown(name))?
            };
            output.push_str(&format_value(value, spec)?);
            cursor = end + 1;
        }
        output.push_str(&template[cursor..]);
        Ok(output)
    }

    pub fn data_imports(&self) -> Vec<DataImport> {
        self.data
            .values()
            .map(|loaded| DataImport {
                path: loaded.fits.path.display().to_string(),
                sha256: loaded.fits.sha256.clone(),
                byte_count: loaded.fits.byte_count,
                format: "FITS-multi-HDU".into(),
                access: loaded.access.clone(),
                evidence_path: None,
                evidence_sha256: None,
            })
            .collect()
    }

    pub fn copy_data_evidence(&self, sha256: &str, destination: &Path) -> Result<String> {
        let loaded = self
            .data
            .values()
            .find(|loaded| loaded.fits.sha256 == sha256)
            .ok_or_else(|| {
                GoblinError::data("Loaded FITS source disappeared before evidence preservation.")
            })?;
        loaded.fits.copy_evidence(destination)
    }
}

fn require_args(name: &str, args: &[Expr], expected: usize) -> Result<()> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(GoblinError::parse(format!(
            "{name}() requires {expected} argument(s), got {}.",
            args.len()
        )))
    }
}

fn require_args_one_of(name: &str, args: &[Expr], expected: &[usize]) -> Result<()> {
    if expected.contains(&args.len()) {
        Ok(())
    } else {
        let choices = expected
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(" or ");
        Err(GoblinError::parse(format!(
            "{name}() requires {choices} argument(s), got {}.",
            args.len()
        )))
    }
}

fn header_value(value: HeaderValue) -> Result<Value> {
    match value {
        HeaderValue::Integer(value) => Quantity::scalar(value as f64).map(Value::Quantity),
        HeaderValue::Float(value) => Quantity::scalar(value).map(Value::Quantity),
        HeaderValue::Boolean(value) => {
            Quantity::scalar(if value { 1.0 } else { 0.0 }).map(Value::Quantity)
        }
        HeaderValue::Text(value) => Ok(Value::Text(value)),
    }
}

fn column_value(value: ColumnValue, hdu: usize, column: &str, row: usize) -> Result<Value> {
    match value {
        ColumnValue::Number(value) => Quantity::scalar(value).map(Value::Quantity),
        ColumnValue::Text(value) => Ok(Value::Text(value)),
        ColumnValue::Boolean(value) => {
            Quantity::scalar(if value { 1.0 } else { 0.0 }).map(Value::Quantity)
        }
        ColumnValue::Null => Err(GoblinError::data(format!(
            "FITS HDU {hdu} column {column} row {row} is null/NaN."
        ))),
    }
}

fn integer_scalar(value: &Value, context: &str) -> Result<i64> {
    let quantity = value.quantity(context)?;
    if quantity.dimension != DIMENSIONLESS
        || quantity.value_si.fract() != 0.0
        || quantity.value_si < 0.0
    {
        return Err(GoblinError::data(format!(
            "{context} requires a non-negative integer scalar."
        )));
    }
    Ok(quantity.value_si as i64)
}

fn loop_integer(value: &Value, context: &str) -> Result<i64> {
    const MAX_SAFE: f64 = 9_007_199_254_740_991.0;
    let quantity = value.quantity(context)?;
    if quantity.dimension != DIMENSIONLESS
        || quantity.value_si.fract() != 0.0
        || quantity.value_si.abs() > MAX_SAFE
    {
        return Err(GoblinError::new(
            "G203",
            format!("{context} must be a dimensionless, exactly representable integer."),
        ));
    }
    Ok(quantity.value_si as i64)
}

fn compare_values(op: &str, left: &Value, right: &Value) -> Result<bool> {
    match (left, right) {
        (Value::Quantity(left), Value::Quantity(right)) => {
            if left.dimension != right.dimension {
                return Err(GoblinError::dimension(
                    "Comparison requires matching dimensions.",
                ));
            }
            Ok(match op {
                "==" => left.value_si == right.value_si,
                "!=" => left.value_si != right.value_si,
                "<" => left.value_si < right.value_si,
                "<=" => left.value_si <= right.value_si,
                ">" => left.value_si > right.value_si,
                ">=" => left.value_si >= right.value_si,
                _ => return Err(GoblinError::parse("Unknown comparison operator.")),
            })
        }
        (Value::Text(left), Value::Text(right)) => match op {
            "==" => Ok(left == right),
            "!=" => Ok(left != right),
            _ => Err(GoblinError::new(
                "G203",
                "Text supports == and !=, not ordering.",
            )),
        },
        (Value::Bool(left), Value::Bool(right)) => match op {
            "==" => Ok(left == right),
            "!=" => Ok(left != right),
            _ => Err(GoblinError::new(
                "G203",
                "Booleans support == and !=, not ordering.",
            )),
        },
        (Value::Array(left), Value::Array(right)) if op == "==" || op == "!=" => {
            Ok((left == right) == (op == "=="))
        }
        _ => Err(GoblinError::new(
            "G203",
            "Comparison requires two values of the same kind.",
        )),
    }
}

fn plot_points(value: &Value, context: &str) -> Result<usize> {
    let points = integer_scalar(value, context)? as usize;
    if points == 0 || points > MAX_PLOT_POINTS {
        return Err(GoblinError::artifact(format!(
            "{context} sample size must be between 1 and {MAX_PLOT_POINTS}."
        )));
    }
    Ok(points)
}

fn export_json_value(value: &Value) -> serde_json::Value {
    match value {
        Value::Text(value) => serde_json::Value::String(value.clone()),
        Value::Bool(value) => serde_json::Value::Bool(*value),
        Value::Quantity(quantity) if quantity.dimension == DIMENSIONLESS => {
            serde_json::json!(quantity.value_si)
        }
        Value::Quantity(quantity) => serde_json::json!({
            "value_si": quantity.value_si,
            "dimension": quantity.dimension,
            "unit_si": format_dimension(quantity.dimension),
        }),
        Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(export_json_value).collect())
        }
    }
}

fn array_index(value: &Value, context: &str) -> Result<usize> {
    let quantity = value.quantity(context)?;
    if quantity.dimension != DIMENSIONLESS
        || quantity.value_si.fract() != 0.0
        || quantity.value_si < 0.0
        || quantity.value_si > MAX_ARRAY_ITEMS as f64
    {
        return Err(GoblinError::new(
            "G203",
            format!("{context} requires a non-negative integer index within the array limit."),
        ));
    }
    Ok(quantity.value_si as usize)
}

fn require_same_array_type(first: &Value, value: &Value) -> Result<()> {
    let matching = match (first, value) {
        (Value::Quantity(a), Value::Quantity(b)) => a.dimension == b.dimension,
        (Value::Text(_), Value::Text(_)) | (Value::Bool(_), Value::Bool(_)) => true,
        _ => false,
    };
    if matching {
        Ok(())
    } else {
        Err(GoblinError::new(
            "G203",
            "Array elements must share a type and, for quantities, a dimension.",
        ))
    }
}

fn validate_array(items: &[Value]) -> Result<()> {
    if let Some(first) = items.first() {
        if matches!(first, Value::Array(_)) {
            return Err(GoblinError::new(
                "G203",
                "Nested arrays are not supported in this stage.",
            ));
        }
        for value in &items[1..] {
            require_same_array_type(first, value)?;
        }
    }
    Ok(())
}

fn format_value(value: &Value, spec: Option<&str>) -> Result<String> {
    match (value, spec) {
        (Value::Quantity(quantity), Some(spec)) => {
            let rendered = format_numeric(quantity.value_si, spec).ok_or_else(|| {
                GoblinError::new(
                    "G000",
                    format!("Invalid print format {spec:?}. Supported forms are .Nf and .Ne."),
                )
            })?;
            let unit = format_dimension(quantity.dimension);
            Ok(if unit == "1" {
                rendered
            } else {
                format!("{rendered} {unit}")
            })
        }
        (_, Some(spec)) => Err(GoblinError::new(
            "G000",
            format!("Format {spec:?} requires a numeric quantity."),
        )),
        (value, None) => Ok(value.render()),
    }
}

fn format_numeric(value: f64, spec: &str) -> Option<String> {
    let digits = spec.strip_prefix('.')?;
    if let Some(precision) = digits
        .strip_suffix('f')
        .and_then(|value| value.parse::<usize>().ok())
    {
        return Some(format!("{:.*}", precision, value));
    }
    if let Some(precision) = digits
        .strip_suffix('e')
        .and_then(|value| value.parse::<usize>().ok())
    {
        let raw = format!("{:.*e}", precision, value);
        let (mantissa, exponent) = raw.split_once('e')?;
        let exponent = exponent.parse::<i32>().ok()?;
        let sign = if exponent < 0 { '-' } else { '+' };
        return Some(format!("{mantissa}e{sign}{:02}", exponent.unsigned_abs()));
    }
    None
}

fn printf(format: &str, values: &[Value]) -> Result<String> {
    let mut output = String::new();
    let mut cursor = 0;
    let mut argument = 0;
    while let Some(relative) = format[cursor..].find('%') {
        let start = cursor + relative;
        output.push_str(&format[cursor..start]);
        if format[start..].starts_with("%%") {
            output.push('%');
            cursor = start + 2;
            continue;
        }
        let tail = &format[start + 1..];
        let ending = tail
            .find(['f', 's'])
            .ok_or_else(|| GoblinError::parse("Unsupported printf format."))?;
        let spec = &tail[..=ending];
        let value = values
            .get(argument)
            .ok_or_else(|| GoblinError::parse("printf has too few values."))?;
        let rendered = if spec.ends_with('s') {
            value.render()
        } else {
            let precision = spec
                .strip_prefix('.')
                .and_then(|value| value.strip_suffix('f'))
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(6);
            format!("{:.*}", precision, value.quantity("printf")?.value_si)
        };
        output.push_str(&rendered);
        argument += 1;
        cursor = start + 1 + spec.len();
    }
    output.push_str(&format[cursor..]);
    if argument != values.len() {
        return Err(GoblinError::parse("printf has too many values."));
    }
    Ok(output)
}

fn unknown(name: &str) -> GoblinError {
    GoblinError::unknown(format!(
        "UNKNOWN SYMBOL\n\n    {name}\n\n{name} is not a declared variable, registered constant, known function, or unit.\n\nGoblin refuses to invent meaning."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_source;

    #[test]
    fn evaluates_energy_and_output() {
        let parsed = parse_source("GO_PARANOID\nmass = 1 kg\nenergy = mass * c^2\nprint(\"Energy = {energy}\")\nseal energy\n").unwrap();
        let evaluation = Evaluation::new(".").eval_program(&parsed.program).unwrap();
        assert!(evaluation.paranoid);
        assert_eq!(
            evaluation.sealed["energy"]
                .quantity("test")
                .unwrap()
                .dimension,
            [1, 2, -2, 0, 0]
        );
        assert!(evaluation.stdout[0].starts_with("Energy = 8.987551787"));
    }

    #[test]
    fn refuses_dimension_mismatch() {
        let parsed = parse_source("a = 1 kg\nb = 1 s\nx = a + b\n").unwrap();
        assert_eq!(
            Evaluation::new(".")
                .eval_program(&parsed.program)
                .unwrap_err()
                .code,
            "G201"
        );
    }
}

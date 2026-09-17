use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Bool(bool),
    Text(String),
    Name {
        source: String,
        canonical_id: Option<String>,
    },
    QuantityLiteral {
        value: f64,
        unit: String,
    },
    Array(Vec<Expr>),
    Index {
        array: Box<Expr>,
        index: Box<Expr>,
    },
    Slice {
        array: Box<Expr>,
        start: Option<Box<Expr>>,
        stop: Option<Box<Expr>>,
    },
    Unary {
        op: char,
        expr: Box<Expr>,
    },
    Binary {
        op: char,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Compare {
        op: String,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Directive(String),
    Assign {
        name: String,
        expr: Expr,
    },
    IndexAssign {
        name: String,
        index: Expr,
        expr: Expr,
    },
    Expression(Expr),
    Seal(String),
    For {
        variable: String,
        start: Expr,
        stop: Expr,
        step: Expr,
        body: Vec<Stmt>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    If {
        branches: Vec<(Expr, Vec<Stmt>)>,
        else_body: Option<Vec<Stmt>>,
    },
    Switch {
        selector: Expr,
        cases: Vec<(Expr, Vec<Stmt>)>,
        default: Option<Vec<Stmt>>,
    },
    InlineRust {
        index: usize,
        sha256: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

impl Expr {
    pub fn canonical(&self) -> Value {
        match self {
            Expr::Number(value) => json!(["number", value]),
            Expr::Bool(value) => json!(["bool", value]),
            Expr::Text(value) => json!(["string", value]),
            Expr::Name {
                source,
                canonical_id,
            } => match canonical_id {
                Some(id) => json!(["constant", id]),
                None => json!(["name", source]),
            },
            Expr::QuantityLiteral { value, unit } => json!(["quantity", value, unit]),
            Expr::Array(items) => json!([
                "array",
                items.iter().map(Expr::canonical).collect::<Vec<_>>()
            ]),
            Expr::Index { array, index } => json!(["index", array.canonical(), index.canonical()]),
            Expr::Slice { array, start, stop } => json!([
                "slice",
                array.canonical(),
                start.as_ref().map(|v| v.canonical()),
                stop.as_ref().map(|v| v.canonical())
            ]),
            Expr::Unary { op, expr } => json!(["unary", op.to_string(), expr.canonical()]),
            Expr::Binary { op, left, right } => {
                json!([
                    "binary",
                    op.to_string(),
                    left.canonical(),
                    right.canonical()
                ])
            }
            Expr::Compare { op, left, right } => {
                json!(["compare", op, left.canonical(), right.canonical()])
            }
            Expr::Call { name, args } => {
                let canonical_name = if name == "printf" { "print" } else { name };
                json!([
                    "call",
                    canonical_name,
                    args.iter().map(Expr::canonical).collect::<Vec<_>>()
                ])
            }
        }
    }
}

impl Stmt {
    pub fn canonical(&self) -> Value {
        match self {
            Stmt::Directive(name) => json!(["directive", name]),
            Stmt::Assign { name, expr } => json!(["assign", name, expr.canonical()]),
            Stmt::IndexAssign { name, index, expr } => {
                json!(["index_assign", name, index.canonical(), expr.canonical()])
            }
            Stmt::Expression(expr) => json!(["expr", expr.canonical()]),
            Stmt::Seal(name) => json!(["seal", name]),
            Stmt::For {
                variable,
                start,
                stop,
                step,
                body,
            } => json!([
                "for",
                variable,
                start.canonical(),
                stop.canonical(),
                step.canonical(),
                body.iter().map(Stmt::canonical).collect::<Vec<_>>()
            ]),
            Stmt::While { condition, body } => json!([
                "while",
                condition.canonical(),
                body.iter().map(Stmt::canonical).collect::<Vec<_>>()
            ]),
            Stmt::If {
                branches,
                else_body,
            } => json!([
                "if",
                branches
                    .iter()
                    .map(|(condition, body)| json!([
                        condition.canonical(),
                        body.iter().map(Stmt::canonical).collect::<Vec<_>>()
                    ]))
                    .collect::<Vec<_>>(),
                else_body
                    .as_ref()
                    .map(|body| body.iter().map(Stmt::canonical).collect::<Vec<_>>())
            ]),
            Stmt::Switch {
                selector,
                cases,
                default,
            } => json!([
                "switch",
                selector.canonical(),
                cases
                    .iter()
                    .map(|(label, body)| json!([
                        label.canonical(),
                        body.iter().map(Stmt::canonical).collect::<Vec<_>>()
                    ]))
                    .collect::<Vec<_>>(),
                default
                    .as_ref()
                    .map(|body| body.iter().map(Stmt::canonical).collect::<Vec<_>>())
            ]),
            Stmt::InlineRust { sha256, .. } => json!(["inline_rust", sha256]),
        }
    }
}

impl Program {
    pub fn canonical(&self) -> Value {
        json!([
            "program",
            self.statements
                .iter()
                .map(Stmt::canonical)
                .collect::<Vec<_>>()
        ])
    }
}

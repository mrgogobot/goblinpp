use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoblinError {
    pub code: &'static str,
    pub category: &'static str,
    pub message: String,
    pub classification: Option<String>,
}

impl GoblinError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            category: "MACHINERY_FAIL",
            message: message.into(),
            classification: None,
        }
    }

    pub fn protocol(classification: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: "G401",
            category: "PROTOCOL_VIOLATION",
            message: message.into(),
            classification: Some(classification.into()),
        }
    }

    pub fn lex(message: impl Into<String>) -> Self {
        Self::new("G001", message)
    }
    pub fn parse(message: impl Into<String>) -> Self {
        Self::new("G002", message)
    }
    pub fn unknown(message: impl Into<String>) -> Self {
        Self::new("G101", message)
    }
    pub fn dimension(message: impl Into<String>) -> Self {
        Self::new("G201", message)
    }
    pub fn numeric(message: impl Into<String>) -> Self {
        Self::new("G202", message)
    }
    pub fn artifact(message: impl Into<String>) -> Self {
        Self::new("G301", message)
    }
    pub fn freeze(message: impl Into<String>) -> Self {
        Self::new("G402", message)
    }
    pub fn revision(message: impl Into<String>) -> Self {
        Self::new("G403", message)
    }
    pub fn ledger(message: impl Into<String>) -> Self {
        Self::new("G404", message)
    }
    pub fn compile(message: impl Into<String>) -> Self {
        Self::new("G501", message)
    }
    pub fn data(message: impl Into<String>) -> Self {
        Self::new("G601", message)
    }

    pub fn pretty(&self) -> String {
        if self.category == "PROTOCOL_VIOLATION" {
            format!("GOBLIN PROTOCOL VIOLATION\n\n{}", self.message)
        } else {
            format!("GOBLIN ERROR {}\n\n{}", self.code, self.message)
        }
    }
}

impl Display for GoblinError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for GoblinError {}

impl From<std::io::Error> for GoblinError {
    fn from(value: std::io::Error) -> Self {
        Self::new("G999", format!("I/O failure: {value}"))
    }
}

impl From<serde_json::Error> for GoblinError {
    fn from(value: serde_json::Error) -> Self {
        Self::new("G999", format!("JSON failure: {value}"))
    }
}

pub type Result<T> = std::result::Result<T, GoblinError>;

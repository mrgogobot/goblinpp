use crate::error::{GoblinError, Result};
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Read, Write};

pub const MAX_ARGUMENTS: usize = 256;
pub const MAX_TEXT_BYTES: usize = 65_536;
pub const MAX_INPUT_EVENTS: usize = 1_024;

#[derive(Debug, Clone, Default)]
pub enum InputPolicy {
    #[default]
    Disabled,
    Interactive,
    Provided(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InputEvent {
    pub prompt: String,
    pub response: Option<String>,
}

#[derive(Debug)]
pub struct Interaction {
    pub argv: Vec<String>,
    pub input_events: Vec<InputEvent>,
    pub arguments_accessed: bool,
    policy: InputPolicy,
    next_provided: usize,
}

impl Default for Interaction {
    fn default() -> Self {
        Self {
            argv: Vec::new(),
            input_events: Vec::new(),
            arguments_accessed: false,
            policy: InputPolicy::Disabled,
            next_provided: 0,
        }
    }
}

impl Interaction {
    pub fn configure(&mut self, argv: Vec<String>, policy: InputPolicy) -> Result<()> {
        if argv.len() > MAX_ARGUMENTS || argv.iter().any(|value| value.len() > MAX_TEXT_BYTES) {
            return Err(GoblinError::new(
                "G203",
                "Too many or oversized program arguments.",
            ));
        }
        self.argv = argv;
        self.policy = policy;
        Ok(())
    }

    pub fn argc(&mut self) -> usize {
        self.arguments_accessed = true;
        self.argv.len()
    }

    pub fn argv(&mut self, index: usize) -> Result<&str> {
        self.arguments_accessed = true;
        self.argv.get(index).map(String::as_str).ok_or_else(|| {
            GoblinError::new(
                "G203",
                format!(
                    "argv({index}) is out of range; argc is {}.",
                    self.argv.len()
                ),
            )
        })
    }

    pub fn input(&mut self, prompt: String) -> Result<String> {
        if prompt.len() > MAX_TEXT_BYTES {
            return Err(GoblinError::new("G203", "input() prompt is too long."));
        }
        if self.input_events.len() >= MAX_INPUT_EVENTS {
            return Err(GoblinError::new("G203", "input() call limit exceeded."));
        }
        let response = match &self.policy {
            InputPolicy::Disabled => {
                return Err(GoblinError::new(
                    "G203",
                    "input() needs an interactive run or explicitly supplied input; check never reads stdin.",
                ));
            }
            InputPolicy::Provided(lines) => {
                let value = lines.get(self.next_provided).cloned();
                self.next_provided += 1;
                value
            }
            InputPolicy::Interactive => {
                if std::env::var("GOBLIN_INPUT_PROMPT_PROTOCOL").as_deref() == Ok("hex-v1") {
                    let encoded = prompt
                        .as_bytes()
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<String>();
                    eprintln!("GOBLIN_INPUT_PROMPT_V1\t{encoded}");
                } else {
                    eprint!("{prompt}");
                }
                io::stderr().flush().map_err(|error| {
                    GoblinError::new("G203", format!("Cannot display input prompt: {error}"))
                })?;
                let mut line = String::new();
                let mut limited = io::stdin().lock().take((MAX_TEXT_BYTES + 2) as u64);
                let read = limited.read_line(&mut line).map_err(|error| {
                    GoblinError::new("G203", format!("Cannot read standard input: {error}"))
                })?;
                if read == 0 {
                    None
                } else {
                    if line.ends_with('\n') {
                        line.pop();
                        if line.ends_with('\r') {
                            line.pop();
                        }
                    }
                    Some(line)
                }
            }
        };
        if let Some(value) = &response
            && (value.len() > MAX_TEXT_BYTES || value.contains(['\n', '\r']))
        {
            return Err(GoblinError::new(
                "G203",
                "input() response is too long or contains a line break.",
            ));
        }
        self.input_events.push(InputEvent {
            prompt,
            response: response.clone(),
        });
        response.ok_or_else(|| GoblinError::new("G203", "input() reached end of standard input."))
    }

    pub fn evidence_needed(&self) -> bool {
        self.arguments_accessed || self.argv.len() > 1 || !self.input_events.is_empty()
    }
}

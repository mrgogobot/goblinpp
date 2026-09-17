use crate::error::{GoblinError, Result};
use crate::hashing::sha256_bytes;

pub const BEGIN: &str = "RUST_INLINE_BEGIN";
pub const END: &str = "RUST_INLINE_END";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineRustBlock {
    pub index: usize,
    pub code: String,
    pub sha256: String,
    pub start_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedSource {
    pub language_source: String,
    pub blocks: Vec<InlineRustBlock>,
}

pub fn extract(source: &str) -> Result<ExtractedSource> {
    let mut output = String::new();
    let mut blocks = Vec::new();
    let mut active: Option<(usize, String)> = None;

    for (line_index, line_with_end) in source.split_inclusive('\n').enumerate() {
        let line = line_with_end.strip_suffix('\n').unwrap_or(line_with_end);
        let marker = line.trim();
        if let Some((start_line, code)) = active.as_mut() {
            if marker == END {
                let index = blocks.len();
                let code = std::mem::take(code);
                let sha256 = sha256_bytes(code.as_bytes());
                blocks.push(InlineRustBlock {
                    index,
                    code,
                    sha256: sha256.clone(),
                    start_line: *start_line,
                });
                output.push_str(&format!("__goblin_inline_rust_{index}\n"));
                active = None;
            } else {
                code.push_str(line_with_end);
                output.push('\n');
            }
            continue;
        }
        if marker == BEGIN {
            active = Some((line_index + 1, String::new()));
            output.push('\n');
        } else if marker == END {
            return Err(GoblinError::parse(format!(
                "{END} at line {} has no matching {BEGIN}.",
                line_index + 1
            )));
        } else {
            output.push_str(line_with_end);
        }
    }
    if let Some((line, _)) = active {
        return Err(GoblinError::parse(format!(
            "{BEGIN} at line {line} has no matching {END}."
        )));
    }
    Ok(ExtractedSource {
        language_source: output,
        blocks,
    })
}

pub fn approved(blocks: &[InlineRustBlock], allowed: &[String]) -> Result<()> {
    for block in blocks {
        if !allowed.iter().any(|item| item == &block.sha256) {
            return Err(GoblinError::protocol(
                "INLINE_RUST_NOT_AUTHORIZED",
                format!(
                    "Inline Rust block {} is arbitrary native code.\n\nBLOCK_SHA256={}\n\nExecution refused. Re-run only after reviewing the block and authorizing this exact hash with --allow-inline-rust {}.",
                    block.index + 1,
                    block.sha256,
                    block.sha256
                ),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_and_hashes_exact_block_bytes() {
        let source = "x = 1\nRUST_INLINE_BEGIN\nprintln!(\"hi\");\nRUST_INLINE_END\nseal x\n";
        let result = extract(source).unwrap();
        assert_eq!(result.blocks[0].code, "println!(\"hi\");\n");
        assert!(result.language_source.contains("__goblin_inline_rust_0"));
    }
}

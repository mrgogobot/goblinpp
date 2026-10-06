//! Static local function libraries. Loading never evaluates library statements.
use crate::ast::Stmt;
use crate::error::{GoblinError, Result};
use crate::hashing::sha256_bytes;
use crate::parser::{ParsedSource, SyntaxMode, parse_source_with_mode};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path};

pub const MAX_MODULES: usize = 32;
pub const MAX_MODULE_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleImport {
    pub path: String,
    pub sha256: String,
    pub byte_count: usize,
    pub evidence_path: String,
}

#[derive(Debug)]
pub struct ResolvedSource {
    pub parsed: ParsedSource,
    pub imports: Vec<ModuleImport>,
    bytes: BTreeMap<String, Vec<u8>>,
}

fn local_path(parent: &Path, requested: &str) -> Result<String> {
    let path = Path::new(requested);
    if requested.contains('\\')
        || requested.chars().any(char::is_control)
        || requested
            .split('/')
            .any(|part| matches!(part, "" | "." | ".."))
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        || path.extension().and_then(|s| s.to_str()) != Some("gbl")
    {
        return Err(GoblinError::parse(
            "Imports require a relative .gbl path without traversal, backslashes or control characters.",
        ));
    }
    Ok(parent.join(path).to_string_lossy().into_owned())
}

fn resolve_with(
    text: &str,
    mode: SyntaxMode,
    mut read: impl FnMut(&str) -> Result<Vec<u8>>,
) -> Result<ResolvedSource> {
    fn visit(
        parsed: &mut ParsedSource,
        parent: &Path,
        active: &mut HashSet<String>,
        visited: &mut HashSet<String>,
        imports: &mut Vec<ModuleImport>,
        bytes: &mut BTreeMap<String, Vec<u8>>,
        read: &mut impl FnMut(&str) -> Result<Vec<u8>>,
    ) -> Result<()> {
        let mode = parsed.syntax_mode;
        let mut expanded = Vec::new();
        for stmt in std::mem::take(&mut parsed.program.statements) {
            if let Stmt::Import(requested) = stmt {
                let path = local_path(parent, &requested)?;
                if active.contains(&path) {
                    return Err(GoblinError::parse(format!(
                        "Cyclic import involving {path:?}."
                    )));
                }
                if visited.contains(&path) {
                    continue;
                }
                if visited.len() >= MAX_MODULES {
                    return Err(GoblinError::parse(
                        "At most 32 local modules may be imported.",
                    ));
                }
                visited.insert(path.clone());
                active.insert(path.clone());
                let raw = read(&path)?;
                if raw.len() > MAX_MODULE_BYTES {
                    return Err(GoblinError::parse(
                        "A module may contain at most 1 MiB of source.",
                    ));
                }
                let text = std::str::from_utf8(&raw)
                    .map_err(|_| GoblinError::parse("Module source must be UTF-8."))?;
                let mut module = parse_source_with_mode(text, mode)?;
                if !module.inline_rust.is_empty()
                    || module
                        .program
                        .statements
                        .iter()
                        .any(|s| !matches!(s, Stmt::Import(_) | Stmt::Function { .. }))
                {
                    return Err(GoblinError::parse(format!(
                        "Library {path:?} may contain only imports and g_func definitions; top-level execution, directives and inline Rust are forbidden."
                    )));
                }
                let sha256 = sha256_bytes(&raw);
                imports.push(ModuleImport {
                    path: path.clone(),
                    evidence_path: format!("modules/{sha256}.gbl"),
                    sha256,
                    byte_count: raw.len(),
                });
                bytes.insert(path.clone(), raw);
                visit(
                    &mut module,
                    Path::new(&path).parent().unwrap_or(Path::new("")),
                    active,
                    visited,
                    imports,
                    bytes,
                    read,
                )?;
                active.remove(&path);
                expanded.extend(module.program.statements);
            } else {
                expanded.push(stmt);
            }
        }
        parsed.program.statements = expanded;
        Ok(())
    }
    let mut parsed = parse_source_with_mode(text, mode)?;
    let mut imports = Vec::new();
    let mut bytes = BTreeMap::new();
    visit(
        &mut parsed,
        Path::new(""),
        &mut HashSet::new(),
        &mut HashSet::new(),
        &mut imports,
        &mut bytes,
        &mut read,
    )?;
    let mut names = HashSet::new();
    for stmt in &parsed.program.statements {
        if let Stmt::Function { name, .. } = stmt
            && !names.insert(name)
        {
            return Err(GoblinError::parse(format!(
                "Duplicate imported or local g_func {name:?}; names are never silently shadowed."
            )));
        }
    }
    imports.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(ResolvedSource {
        parsed,
        imports,
        bytes,
    })
}

pub fn resolve(source: &Path, text: &str) -> Result<ResolvedSource> {
    resolve_with_mode(source, text, SyntaxMode::Current)
}

pub fn resolve_with_mode(source: &Path, text: &str, mode: SyntaxMode) -> Result<ResolvedSource> {
    let base = source
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .canonicalize()?;
    resolve_with(text, mode, |relative| {
        let mut path = base.clone();
        for part in Path::new(relative).components() {
            path.push(part.as_os_str());
            if fs::symlink_metadata(&path)?.file_type().is_symlink() {
                return Err(GoblinError::parse(
                    "Symbolic links are not accepted in module paths.",
                ));
            }
        }
        let metadata = fs::metadata(&path)?;
        if !metadata.is_file() || metadata.len() > MAX_MODULE_BYTES as u64 {
            return Err(GoblinError::parse(
                "Module must be an ordinary file no larger than 1 MiB.",
            ));
        }
        use std::io::Read;
        let mut bytes = Vec::new();
        fs::File::open(path)?
            .take(MAX_MODULE_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        Ok(bytes)
    })
}

impl ResolvedSource {
    pub fn snapshot(&self) -> serde_json::Value {
        snapshot(&self.imports)
    }
    pub fn preserve(&self, run: &Path) -> Result<()> {
        if self.imports.is_empty() {
            return Ok(());
        }
        fs::create_dir(run.join("modules"))?;
        let mut written = HashSet::new();
        for entry in &self.imports {
            if written.insert(&entry.evidence_path) {
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(run.join(&entry.evidence_path))?;
                file.write_all(&self.bytes[&entry.path])?;
                file.sync_all()?;
            }
        }
        Ok(())
    }
    pub fn unchanged(&self, source: &Path) -> bool {
        let base = source
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        self.imports.iter().all(|entry| {
            crate::hashing::sha256_file(base.join(&entry.path))
                .ok()
                .as_deref()
                == Some(&entry.sha256)
        })
    }
}

pub fn snapshot(imports: &[ModuleImport]) -> serde_json::Value {
    serde_json::json!(
        imports
            .iter()
            .map(|m| serde_json::json!({"path":m.path,"sha256":m.sha256,"byte_count":m.byte_count}))
            .collect::<Vec<_>>()
    )
}

pub fn verify_preserved(
    run: &Path,
    text: &str,
    entries: &serde_json::Value,
) -> Result<ParsedSource> {
    verify_preserved_with_mode(run, text, entries, SyntaxMode::Current)
}

pub fn verify_preserved_with_mode(
    run: &Path,
    text: &str,
    entries: &serde_json::Value,
    mode: SyntaxMode,
) -> Result<ParsedSource> {
    let entries: Vec<ModuleImport> = serde_json::from_value(entries.clone())?;
    if entries.len() > MAX_MODULES {
        return Err(GoblinError::parse("Too many module evidence entries."));
    }
    let mut available = BTreeMap::new();
    for entry in &entries {
        local_path(Path::new(""), &entry.path)?;
        if entry.sha256.len() != 64
            || !entry.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            || entry.evidence_path != format!("modules/{}.gbl", entry.sha256)
            || entry.byte_count > MAX_MODULE_BYTES
            || available.contains_key(&entry.path)
        {
            return Err(GoblinError::parse("Invalid module evidence descriptor."));
        }
        let path = run.join(&entry.evidence_path);
        if fs::symlink_metadata(&path)?.file_type().is_symlink()
            || fs::symlink_metadata(run.join("modules"))?
                .file_type()
                .is_symlink()
        {
            return Err(GoblinError::parse(
                "Module evidence cannot be a symbolic link.",
            ));
        }
        let raw = fs::read(path)?;
        if raw.len() != entry.byte_count || sha256_bytes(&raw) != entry.sha256 {
            return Err(GoblinError::parse(
                "Module evidence hash or byte count mismatch.",
            ));
        }
        available.insert(entry.path.clone(), raw);
    }
    let resolved = resolve_with(text, mode, |path| {
        available
            .get(path)
            .cloned()
            .ok_or_else(|| GoblinError::parse(format!("Missing module evidence for {path:?}.")))
    })?;
    if resolved.imports != entries {
        return Err(GoblinError::parse(
            "Module graph does not match preserved evidence.",
        ));
    }
    Ok(resolved.parsed)
}

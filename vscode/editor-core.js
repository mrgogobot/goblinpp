"use strict";

const path = require("path");

const IDENTIFIER_PATTERN = /^[A-Za-z_πħωΩΔΣλμσθ∇∂][A-Za-z0-9_πħωΩΔΣλμσθ∇∂]*$/u;
const IDENTIFIER_START = /[A-Za-z_πħωΩΔΣλμσθ∇∂]/u;
const IDENTIFIER_CONTINUE = /[A-Za-z0-9_πħωΩΔΣλμσθ∇∂]/u;

const CONSTANT_ICONS = Object.freeze({
  "math.pi": "◯",
  "physical.speed_of_light": "↯",
  "physical.planck_constant": "ℎ",
  "physical.reduced_planck_constant": "ℏ",
  "physical.gravitational_constant": "↓",
  "physical.boltzmann_constant": "♨",
  "physical.avogadro_constant": "⚛",
});

const INSERTABLE_IDENTIFIER_GLYPHS = Object.freeze([
  ["ω", "omega", "Identifier glyph; no built-in scientific meaning."],
  ["Ω", "capital omega", "Identifier glyph; no built-in scientific meaning."],
  ["Δ", "capital delta", "Identifier glyph; no built-in scientific meaning."],
  ["Σ", "capital sigma", "Identifier glyph; no built-in scientific meaning."],
  ["λ", "lambda", "Identifier glyph; no built-in scientific meaning."],
  ["μ", "mu", "Identifier glyph; no built-in scientific meaning."],
  ["σ", "sigma", "Identifier glyph; no built-in scientific meaning."],
  ["θ", "theta", "Identifier glyph; no built-in scientific meaning."],
  ["∇", "nabla", "Identifier glyph; no built-in scientific meaning."],
  ["∂", "partial", "Identifier glyph; no built-in scientific meaning."],
]);

const KNOWN_COMMANDS = new Set([
  "audit-ledger",
  "check",
  "doctor",
  "freeze",
  "lineage",
  "revise",
  "run",
  "status",
  "verify",
]);

const INPUT_PROMPT_PREFIX = "GOBLIN_INPUT_PROMPT_V1\t";

function decodeInputPromptLine(line) {
  if (typeof line !== "string" || !line.startsWith(INPUT_PROMPT_PREFIX)) {
    return null;
  }
  const hex = line.slice(INPUT_PROMPT_PREFIX.length);
  if (hex.length > 131072 || hex.length % 2 !== 0 || !/^[0-9a-f]*$/.test(hex)) {
    return null;
  }
  const bytes = Buffer.from(hex, "hex");
  const prompt = bytes.toString("utf8");
  return Buffer.from(prompt, "utf8").equals(bytes) ? prompt : null;
}

function parseProgramArguments(value) {
  const parsed = JSON.parse(value);
  if (!Array.isArray(parsed) || parsed.length > 255 || !parsed.every((item) =>
    typeof item === "string" && Buffer.byteLength(item, "utf8") <= 65536 && !item.includes("\0"))) {
    throw new Error("Enter a JSON array of at most 255 text arguments.");
  }
  return parsed;
}

function goblinArgs(command, target, extra = []) {
  if (!KNOWN_COMMANDS.has(command)) {
    throw new Error(`Unsupported Goblin++ editor command: ${command}`);
  }
  if (typeof target !== "string" || target.length === 0) {
    throw new Error("Goblin++ editor commands require an explicit target.");
  }
  if (!Array.isArray(extra) || !extra.every((value) => typeof value === "string")) {
    throw new Error("Goblin++ editor command arguments must be strings.");
  }
  return [command, target, ...extra];
}

function selectExecutable(configured, workspaceRoot, exists, homeDirectory, platform = process.platform, arch = process.arch) {
  if (typeof configured === "string" && configured.trim()) {
    return configured.trim();
  }
  const candidates = [];
  if (workspaceRoot && platform === "darwin" && arch === "arm64") {
    candidates.push(path.join(workspaceRoot, "dist", "macos-arm64", "goblin++"));
  }
  if (homeDirectory && platform !== "win32") {
    candidates.push(path.join(homeDirectory, ".local", "bin", "goblin++"));
  }
  return candidates.find((candidate) => exists(candidate)) || "goblin++";
}

function resultKind(stdout, stderr, exitCode) {
  const text = `${stdout || ""}\n${stderr || ""}`;
  if (
    /PROTOCOL_VIOLATION|LEDGER_STATUS=FAIL|FREEZE_RECEIPT=FAIL|STATE=(?:FROZEN_INVALID|FROZEN_UNREGISTERED|.*(?:FAILURE|TAMPERED|CHANGE_AFTER_FREEZE|MISMATCH|MISSING))/.test(
      text,
    )
  ) {
    return "protocol-violation";
  }
  if (/VERIFICATION_STATUS=FAIL/.test(text)) {
    return "verification-failure";
  }
  if (/MACHINERY_FAIL|GOBLIN ERROR G\d+/.test(text)) {
    return "machinery-failure";
  }
  if (
    exitCode === 0 &&
    /(?:RUN|CHECK|VERIFICATION|LEDGER|DOCTOR|TUTORIAL|LEXICON|GRAMMAR|SEMANTICS)_STATUS=PASS|PROGRAM=FROZEN|GOBLIN LINEAGE/.test(
      text,
    )
  ) {
    return "pass";
  }
  return exitCode === 0 ? "pass" : "failure";
}

function parseCheckReport(stdout) {
  let report;
  try {
    report = JSON.parse(stdout);
  } catch (_error) {
    throw new Error("Goblin++ check did not return valid JSON.");
  }
  if (!report || report.schema !== "goblin.check.v1") {
    throw new Error("Goblin++ check returned an unsupported schema.");
  }
  if (!["PASS", "FAIL"].includes(report.status)) {
    throw new Error("Goblin++ check returned an invalid status.");
  }
  if (
    report.authority !== "PREVIEW_ONLY_NOT_EVIDENCE" ||
    report.evidence_created !== false ||
    report.custody_checked !== false
  ) {
    throw new Error("Goblin++ check crossed its preview-only trust boundary.");
  }
  const diagnostic = report.diagnostic;
  if (report.status === "PASS" && diagnostic !== null) {
    throw new Error("A passing Goblin++ check cannot contain a diagnostic.");
  }
  if (report.status === "FAIL" && (
    !diagnostic || typeof diagnostic.code !== "string" ||
    typeof diagnostic.message !== "string" ||
    typeof diagnostic.category !== "string"
  )) {
    throw new Error("A failing Goblin++ check must contain a valid diagnostic.");
  }
  return { ...report, diagnostics: diagnostic ? [diagnostic] : [] };
}

function parseCheckResponse(stdout, stderr, exitCode) {
  try {
    const report = parseCheckReport(stdout);
    if (exitCode !== (report.status === "PASS" ? 0 : 1)) {
      throw new Error("Goblin++ check status and exit code disagree.");
    }
    return report;
  } catch (error) {
    const match = exitCode === 2
      ? /^GOBLIN ERROR (G00[12])\s+([^\n]+)/m.exec(stderr.trim())
      : null;
    if (!match) {
      throw error;
    }
    return {
      schema: "goblin.check.v1",
      status: "FAIL",
      authority: "PREVIEW_ONLY_NOT_EVIDENCE",
      evidence_created: false,
      custody_checked: false,
      diagnostic: { code: match[1], category: "MACHINERY_FAIL", message: match[2] },
      diagnostics: [{ code: match[1], category: "MACHINERY_FAIL", message: match[2] }],
      derivedFromCliError: true,
    };
  }
}

function dimensionLabel(dimension) {
  const names = ["mass", "length", "time", "temperature", "amount"];
  const parts = [];
  for (let index = 0; index < names.length; index += 1) {
    if (dimension[index]) {
      parts.push(`${names[index]}^${dimension[index]}`);
    }
  }
  return parts.length ? parts.join(" · ") : "dimensionless";
}

function vocabularyEntries(lexicon, runtimeVocabulary = {}) {
  const vocabulary = lexicon.vocabulary || {};
  const entries = [];
  for (const constant of vocabulary.constants || []) {
    for (const alias of constant.aliases) {
      entries.push({
        spelling: alias,
        kind: "constant",
        detail: constant.canonical_id,
      });
    }
  }
  for (const unit of vocabulary.units || []) {
    entries.push({
      spelling: unit.spelling,
      kind: "unit",
      detail: `${dimensionLabel(unit.dimension)}; SI factor ${unit.si_factor}`,
    });
  }
  for (const item of [...(runtimeVocabulary.statements || []), ...(runtimeVocabulary.functions || [])]) {
    if (entries.some((entry) => entry.spelling === item.spelling)) {
      throw new Error(`Duplicate editor vocabulary: ${item.spelling}`);
    }
    entries.push({ ...item });
  }
  return entries;
}

function completionSnippet(entry) {
  if (entry.snippet) {
    return entry.snippet;
  }
  if (entry.spelling === "print") {
    return 'print("${1:message}")';
  }
  if (entry.spelling === "printf") {
    return 'printf("${1:format}", ${2:value})';
  }
  return entry.kind === "function" ? `${entry.spelling}(\${1:arguments})` : undefined;
}

function isGoblinIdentifier(value) {
  return typeof value === "string" && IDENTIFIER_PATTERN.test(value);
}

function symbolPaletteItems(lexicon) {
  const items = [];
  const aliasedGlyphs = new Set();
  for (const constant of (lexicon.vocabulary || {}).constants || []) {
    const icon = CONSTANT_ICONS[constant.canonical_id] || "•";
    for (const alias of constant.aliases || []) {
      aliasedGlyphs.add(alias);
      items.push({
        label: `${icon}  ${alias}`,
        description: "registered constant",
        detail: `${constant.canonical_id}; inserts exact source spelling ${JSON.stringify(alias)}`,
        insertText: alias,
        kind: "constant",
        canonicalId: constant.canonical_id,
      });
    }
  }
  for (const [glyph, name, detail] of INSERTABLE_IDENTIFIER_GLYPHS) {
    if (!aliasedGlyphs.has(glyph)) {
      items.push({
        label: `${glyph}  ${name}`,
        description: "user-defined identifier glyph",
        detail,
        insertText: glyph,
        kind: "identifier",
      });
    }
  }
  for (const [glyph, name] of [
    ["²", "superscript two"],
    ["³", "superscript three"],
  ]) {
    items.push({
      label: `${glyph}  ${name}`,
      description: "canonical notation alias",
      detail: `Inserts ${JSON.stringify(glyph)}; Goblin++ canonicalizes it to ^${glyph === "²" ? "2" : "3"}.`,
      insertText: glyph,
      kind: "notation",
    });
  }
  return items;
}

function iconMappingError(value, allowEmpty = true) {
  if (typeof value !== "string") {
    return "The icon must be text.";
  }
  const icon = value.trim();
  if (!icon) {
    return allowEmpty ? undefined : "An icon is required.";
  }
  if (/\r|\n/u.test(icon)) {
    return "An icon must fit on one line.";
  }
  if (Array.from(icon).length > 8) {
    return "Use at most eight Unicode characters for an icon.";
  }
  return undefined;
}

function normalizeUserIconMappings(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return {};
  }
  const normalized = {};
  for (const [identifier, rawIcon] of Object.entries(value)) {
    if (isGoblinIdentifier(identifier) && !iconMappingError(rawIcon, false)) {
      normalized[identifier] = rawIcon.trim();
    }
  }
  return normalized;
}

function iconMappings(lexicon, userMappings = {}) {
  const mappings = {};
  for (const constant of (lexicon.vocabulary || {}).constants || []) {
    const icon = CONSTANT_ICONS[constant.canonical_id];
    if (!icon) {
      continue;
    }
    for (const alias of constant.aliases || []) {
      mappings[alias] = {
        icon,
        kind: "registered constant",
        detail: constant.canonical_id,
      };
    }
  }
  for (const [identifier, icon] of Object.entries(normalizeUserIconMappings(userMappings))) {
    mappings[identifier] = {
      icon,
      kind: "explicit user mapping",
      detail: "visual only; no scientific meaning is inferred",
    };
  }
  return mappings;
}

function identifierOccurrences(text) {
  const occurrences = [];
  let index = 0;
  let insideRust = false;
  while (index < text.length) {
    if (index === 0 || text[index - 1] === "\n") {
      const nextLine = text.indexOf("\n", index);
      const end = nextLine < 0 ? text.length : nextLine;
      const line = text.slice(index, end).trim();
      if (line === "RUST_INLINE_BEGIN") {
        insideRust = true;
      }
      if (insideRust || line === "RUST_INLINE_END") {
        if (line === "RUST_INLINE_END") {
          insideRust = false;
        }
        index = end;
        if (index < text.length) {
          index += 1;
        }
        continue;
      }
    }
    const character = text[index];
    if (character === "#") {
      while (index < text.length && text[index] !== "\n") {
        index += 1;
      }
      continue;
    }
    if (character === '"') {
      index += 1;
      while (index < text.length) {
        if (text[index] === "\\") {
          index += 2;
        } else if (text[index] === '"') {
          index += 1;
          break;
        } else {
          index += 1;
        }
      }
      continue;
    }
    if (IDENTIFIER_START.test(character)) {
      const start = index;
      index += 1;
      while (index < text.length && IDENTIFIER_CONTINUE.test(text[index])) {
        index += 1;
      }
      occurrences.push({ name: text.slice(start, index), start, end: index });
      continue;
    }
    index += 1;
  }
  return occurrences;
}

function iconDecorations(text, mappings) {
  const original = String(text);
  return identifierOccurrences(original)
    .filter((occurrence) => Object.hasOwn(mappings, occurrence.name))
    .map((occurrence) => ({
      ...occurrence,
      ...mappings[occurrence.name],
    }));
}

module.exports = {
  CONSTANT_ICONS,
  INPUT_PROMPT_PREFIX,
  KNOWN_COMMANDS,
  decodeInputPromptLine,
  dimensionLabel,
  goblinArgs,
  iconDecorations,
  iconMappingError,
  iconMappings,
  identifierOccurrences,
  isGoblinIdentifier,
  normalizeUserIconMappings,
  completionSnippet,
  parseCheckReport,
  parseCheckResponse,
  parseProgramArguments,
  resultKind,
  selectExecutable,
  symbolPaletteItems,
  vocabularyEntries,
};

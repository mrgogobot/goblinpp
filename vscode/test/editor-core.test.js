"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const core = require("../editor-core");

const lexicon = JSON.parse(
  fs.readFileSync(path.join(__dirname, "..", "spec", "lexicon.v0.json"), "utf8"),
);
const runtimeVocabulary = JSON.parse(
  fs.readFileSync(path.join(__dirname, "..", "spec", "rust-alpha10-editor.json"), "utf8"),
);
const grammar = JSON.parse(
  fs.readFileSync(
    path.join(__dirname, "..", "syntaxes", "goblinpp.tmLanguage.json"),
    "utf8",
  ),
);
const manifest = JSON.parse(
  fs.readFileSync(path.join(__dirname, "..", "package.json"), "utf8"),
);

test("commands are explicit argument arrays without a shell string", () => {
  assert.deepEqual(core.goblinArgs("run", "/tmp/a file.gbl"), [
    "run",
    "/tmp/a file.gbl",
  ]);
  assert.deepEqual(core.goblinArgs("check", "/tmp/a file.gbl", ["--json"]), [
    "check",
    "/tmp/a file.gbl",
    "--json",
  ]);
  assert.deepEqual(
    core.goblinArgs("revise", "parent.gbl", ["child.gbl", "--reason", "clear reason"]),
    ["revise", "parent.gbl", "child.gbl", "--reason", "clear reason"],
  );
  assert.deepEqual(core.goblinArgs("run", "experiment.gbl", ["--compile"]), [
    "run", "experiment.gbl", "--compile",
  ]);
  assert.throws(() => core.goblinArgs("invent", "a.gbl"), /Unsupported/);
  assert.throws(() => core.goblinArgs("run", ""), /explicit target/);
});

test("status classification keeps scientific and custody failures distinct", () => {
  assert.equal(core.resultKind("RUN_STATUS=PASS", "", 0), "pass");
  assert.equal(core.resultKind("CHECK_STATUS=PASS", "", 0), "pass");
  assert.equal(core.resultKind("VERIFICATION_STATUS=FAIL", "", 1), "verification-failure");
  assert.equal(
    core.resultKind("RUN_STATUS=MACHINERY_FAIL", "GOBLIN ERROR G201", 1),
    "machinery-failure",
  );
  assert.equal(
    core.resultKind("RUN_STATUS=PROTOCOL_VIOLATION", "", 1),
    "protocol-violation",
  );
  assert.equal(
    core.resultKind("STATE=FROZEN_INVALID", "", 0),
    "protocol-violation",
  );
  assert.equal(core.resultKind("", "not found", 1), "failure");
});

test("Rust v1 check reports and parser errors preserve the preview-only boundary", () => {
  const passing = {
    schema: "goblin.check.v1",
    status: "PASS",
    authority: "PREVIEW_ONLY_NOT_EVIDENCE",
    evidence_created: false,
    custody_checked: false,
    diagnostic: null,
  };
  assert.deepEqual(core.parseCheckReport(JSON.stringify(passing)).diagnostics, []);
  assert.equal(core.parseCheckResponse(JSON.stringify(passing), "", 0).status, "PASS");
  const failing = {
    ...passing,
    status: "FAIL",
    diagnostic: {
      code: "G201",
      category: "MACHINERY_FAIL",
      message: "INCOMPATIBLE DIMENSIONS",
    },
  };
  assert.deepEqual(core.parseCheckResponse(JSON.stringify(failing), "", 1).diagnostics, [failing.diagnostic]);
  assert.throws(() => core.parseCheckReport("not json"), /valid JSON/);
  assert.throws(
    () => core.parseCheckReport(JSON.stringify({ ...passing, evidence_created: true })),
    /trust boundary/,
  );
  assert.throws(
    () => core.parseCheckReport(JSON.stringify({ ...passing, status: "FAIL" })),
    /must contain a valid diagnostic/,
  );
  assert.throws(
    () => core.parseCheckReport(JSON.stringify({
      ...passing,
      status: "FAIL",
      diagnostic: { code: "G201", message: "bad" },
    })),
    /valid diagnostic/,
  );
  assert.throws(() => core.parseCheckResponse(JSON.stringify(passing), "", 1), /disagree/);
  const syntax = core.parseCheckResponse("", "GOBLIN ERROR G002\n\nUnclosed statement block.\n", 2);
  assert.equal(syntax.diagnostics[0].code, "G002");
  assert.equal(syntax.derivedFromCliError, true);
  assert.throws(() => core.parseCheckResponse("", "GOBLIN ERROR G999\n\nI/O failure", 2), /valid JSON/);
});

test("native executable discovery prefers explicit path, bundled build, then user install", () => {
  const root = path.join(path.sep, "work", "project");
  const bundled = path.join(root, "dist", "macos-arm64", "goblin++");
  const local = path.join(path.sep, "home", "snow", ".local", "bin", "goblin++");
  assert.equal(core.selectExecutable("", root, (candidate) => candidate === bundled || candidate === local, "/home/snow", "darwin", "arm64"), bundled);
  assert.equal(core.selectExecutable("", root, (candidate) => candidate === local, "/home/snow", "darwin", "arm64"), local);
  assert.equal(core.selectExecutable("/opt/goblin++", root, () => false, "/home/snow", "darwin", "arm64"), "/opt/goblin++");
  assert.equal(core.selectExecutable("", root, () => false, "/home/snow", "darwin", "arm64"), "goblin++");
});

test("completion combines legacy constant spellings with alpha.10 runtime features", () => {
  const entries = core.vocabularyEntries(lexicon, runtimeVocabulary);
  assert.equal(entries.length, 72);
  assert.equal(new Set(entries.map((entry) => entry.spelling)).size, 72);
  assert.deepEqual(
    entries.find((entry) => entry.spelling === "π"),
    { spelling: "π", kind: "constant", detail: "math.pi" },
  );
  assert.match(entries.find((entry) => entry.spelling === "km").detail, /SI factor 1000/);
  assert(entries.some((entry) => entry.spelling === "fits_column_mean"));
  assert(entries.some((entry) => entry.spelling === "plot_fits_scatter"));
  assert(entries.some((entry) => entry.spelling === "while"));
  for (const keyword of ["if", "else", "switch", "case", "default"]) {
    assert(entries.some((entry) => entry.spelling === keyword));
  }
  assert.match(core.completionSnippet(entries.find((entry) => entry.spelling === "switch")), /default/);
  assert.match(core.completionSnippet(entries.find((entry) => entry.spelling === "g_func")), /return/);
  assert.match(core.completionSnippet(entries.find((entry) => entry.spelling === "parse_number")), /text/);
  for (const keyword of ["argc", "argv", "input"]) {
    assert(entries.some((entry) => entry.spelling === keyword));
  }
  assert.match(core.completionSnippet(entries.find((entry) => entry.spelling === "for")), /range/);
  assert.equal(core.completionSnippet(entries.find((entry) => entry.spelling === "fits_mean")), "fits_mean(${1:arguments})");
});

test("input prompt protocol and program argument editor parsing are bounded", () => {
  assert.equal(core.decodeInputPromptLine("GOBLIN_INPUT_PROMPT_V1\t4e616d653f20"), "Name? ");
  assert.equal(core.decodeInputPromptLine("GOBLIN_INPUT_PROMPT_V1\txyz"), null);
  assert.equal(core.decodeInputPromptLine("ordinary error"), null);
  assert.deepEqual(core.parseProgramArguments('["--name","Ada Lovelace"]'), ["--name", "Ada Lovelace"]);
  assert.throws(() => core.parseProgramArguments('"--name"'));
  assert.throws(() => core.parseProgramArguments('["bad\\u0000"]'));
});

test("symbol palette inserts exact supported spellings without inventing meaning", () => {
  const items = core.symbolPaletteItems(lexicon);
  assert.equal(items.length, 26);
  assert(items.some((item) => item.insertText === "hbar" && item.canonicalId === "physical.reduced_planck_constant"));
  assert(items.some((item) => item.insertText === "ħ" && item.canonicalId === "physical.reduced_planck_constant"));
  const omega = items.find((item) => item.insertText === "ω");
  assert.equal(omega.kind, "identifier");
  assert.match(omega.detail, /no built-in scientific meaning/);
  assert(items.some((item) => item.insertText === "²" && item.kind === "notation"));
  assert(!items.some((item) => ["≤", "≥", "≠"].includes(item.insertText)));
});

test("icon mappings distinguish registered constants from explicit user choices", () => {
  const mappings = core.iconMappings(lexicon, {
    energy: "⚡",
    omega: "ω",
    "not valid": "x",
    empty: "",
  });
  assert.deepEqual(mappings.hbar, {
    icon: "ℏ",
    kind: "registered constant",
    detail: "physical.reduced_planck_constant",
  });
  assert.deepEqual(mappings.energy, {
    icon: "⚡",
    kind: "explicit user mapping",
    detail: "visual only; no scientific meaning is inferred",
  });
  assert.equal(mappings.omega.icon, "ω");
  assert.equal(mappings["not valid"], undefined);
  assert.equal(mappings.empty, undefined);
});

test("icon scanning excludes strings and comments and never changes source", () => {
  const source = [
    "energy = hbar * omega",
    'print("hbar {energy}") # energy hbar omega',
    "",
  ].join("\n");
  const before = Buffer.from(source);
  const occurrences = core.identifierOccurrences(source);
  assert.deepEqual(occurrences.map((item) => item.name), [
    "energy",
    "hbar",
    "omega",
    "print",
  ]);
  const decorations = core.iconDecorations(
    source,
    core.iconMappings(lexicon, { energy: "⚡" }),
  );
  assert.deepEqual(decorations.map((item) => [item.name, item.icon]), [
    ["energy", "⚡"],
    ["hbar", "ℏ"],
  ]);
  assert.equal(Buffer.compare(before, Buffer.from(source)), 0);
});

test("icon scanning also ignores arbitrary inline Rust blocks", () => {
  const source = "energy = hbar\nRUST_INLINE_BEGIN\nlet hbar = 3;\nRUST_INLINE_END\nprint(\"{energy}\")\n";
  assert.deepEqual(core.identifierOccurrences(source).map((entry) => entry.name), [
    "energy", "hbar", "print",
  ]);
});

test("user icon validation is bounded and identifier-only", () => {
  assert.equal(core.isGoblinIdentifier("energy"), true);
  assert.equal(core.isGoblinIdentifier("ω"), true);
  assert.equal(core.isGoblinIdentifier("energy value"), false);
  assert.equal(core.iconMappingError("⚡", false), undefined);
  assert.match(core.iconMappingError("", false), /required/);
  assert.match(core.iconMappingError("line\nbreak", true), /one line/);
  assert.match(core.iconMappingError("123456789", true), /eight/);
});

test("dimension labels expose the complete five-component basis", () => {
  assert.equal(core.dimensionLabel([0, 0, 0, 0, 0]), "dimensionless");
  assert.equal(core.dimensionLabel([1, 2, -2, 0, 0]), "mass^1 · length^2 · time^-2");
});

test("all TextMate regular expressions compile", () => {
  function visit(value) {
    if (Array.isArray(value)) {
      value.forEach(visit);
    } else if (value && typeof value === "object") {
      for (const [key, child] of Object.entries(value)) {
        if (["match", "begin", "end"].includes(key)) {
          assert.doesNotThrow(() => new RegExp(child), `${key}: ${child}`);
        } else {
          visit(child);
        }
      }
    }
  }
  visit(grammar);
});

test("alpha.10 language coloring and extension identity are internally consistent", () => {
  assert.equal(manifest.name, "goblinpp");
  assert.equal(manifest.publisher, "goblinpp-project");
  assert.equal(manifest.version, "0.1.5");
  assert(manifest.contributes.commands.some((item) => item.command === "goblinpp.runCompiledFile"));
  assert.equal(grammar.repository.unsupported, undefined);
  const comparisons = new RegExp(grammar.repository.operators.patterns[0].match);
  for (const spelling of ["<", "<=", ">", ">=", "==", "!=", "≤", "≥", "≠"]) {
    assert(comparisons.test(spelling));
  }
  const builtins = new RegExp(grammar.repository.functions.patterns[0].match);
  for (const spelling of ["input", "argv", "len", "append", "parse_number", "str_trim", "str_join", "fits_column_mean", "write_json", "plot_fits_scatter"]) {
    assert(builtins.test(`${spelling}(`));
  }
  const branches = new RegExp(grammar.repository.statements.patterns[1].match);
  for (const spelling of ["if", "else", "switch", "case", "default"]) {
    assert(branches.test(spelling));
  }
  const functionKeywords = new RegExp(grammar.repository.statements.patterns[2].match);
  assert(functionKeywords.test("g_func"));
  assert(functionKeywords.test("return"));
  assert(grammar.repository.inlineRust.patterns[0].contentName === "source.rust");
});

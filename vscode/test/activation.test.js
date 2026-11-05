"use strict";

const assert = require("node:assert/strict");
const childProcess = require("node:child_process");
const { EventEmitter } = require("node:events");
const Module = require("node:module");
const path = require("node:path");
const test = require("node:test");

test("extension activates and registers commands and language providers", async () => {
  const commands = new Map();
  const completions = [];
  const hovers = [];
  const symbols = [];
  const settings = new Map();
  let inserted;
  const confidentialLog = [];
  let lastDecorations = [];
  const disposable = () => ({ dispose() {} });
  const source = "energy = hbar * omega # hbar";
  const selection = { isEmpty: false, active: 0 };
  const document = {
    languageId: "goblinpp",
    uri: { scheme: "file", fsPath: "/tmp/energy.gbl" },
    fileName: "/tmp/energy.gbl",
    isDirty: false,
    getText(range) { return range ? "energy" : source; },
    positionAt(offset) { return offset; },
  };
  const activeEditor = {
    document,
    selection,
    selections: [selection],
    async edit(callback) {
      callback({ replace(_selection, value) { inserted = value; } });
      return true;
    },
    setDecorations(_type, decorations) { lastDecorations = decorations; },
  };

  class CompletionItem {
    constructor(label, kind) {
      this.label = label;
      this.kind = kind;
    }
  }
  class SnippetString {
    constructor(value) {
      this.value = value;
    }
  }

  const fakeVscode = {
    CompletionItem,
    CompletionItemKind: { Keyword: 1, Function: 2, Constant: 3, Unit: 4 },
    ConfigurationTarget: { Global: 1, Workspace: 2 },
    Diagnostic: class Diagnostic {},
    DiagnosticSeverity: { Error: 0, Warning: 1 },
    DocumentSymbol: class DocumentSymbol {},
    Hover: class Hover {},
    MarkdownString: class MarkdownString {
      appendMarkdown() {}
      appendText() {}
    },
    SnippetString,
    StatusBarAlignment: { Left: 1 },
    SymbolKind: { Variable: 1, File: 2 },
    ThemeColor: class ThemeColor {},
    Range: class Range {
      constructor(start, end) {
        this.start = start;
        this.end = end;
      }
    },
    Uri: { file: (fsPath) => ({ fsPath, scheme: "file" }) },
    commands: {
      registerCommand(name, callback) {
        commands.set(name, callback);
        return disposable();
      },
    },
    languages: {
      createDiagnosticCollection() {
        return { clear() {}, delete() {}, set() {}, dispose() {} };
      },
      registerCompletionItemProvider(selector, provider) {
        completions.push({ selector, provider });
        return disposable();
      },
      registerHoverProvider(selector, provider) {
        hovers.push({ selector, provider });
        return disposable();
      },
      registerDocumentSymbolProvider(selector, provider) {
        symbols.push({ selector, provider });
        return disposable();
      },
    },
    window: {
      activeTextEditor: activeEditor,
      visibleTextEditors: [activeEditor],
      createOutputChannel: () => ({ append(value) { confidentialLog.push(value); }, appendLine(value) { confidentialLog.push(value); }, show() {}, dispose() {} }),
      createStatusBarItem: () => ({ show() {}, dispose() {} }),
      createTextEditorDecorationType: () => disposable(),
      onDidChangeVisibleTextEditors: () => disposable(),
      showQuickPick: async (items) => items.find((item) => item.insertText === "hbar"),
      showInputBox: async (options) => options?.title === "Goblin++ secret input" ? "public masked code passphrase" : options?.title === "Goblin++ program input" ? "Ada" : options?.title === "Goblin++ program arguments" ? '["--name", "Ada"]' : "⚡",
      showInformationMessage() {},
      showWarningMessage() {},
      showErrorMessage() {},
    },
    workspace: {
      isTrusted: true,
      workspaceFolders: [],
      getConfiguration: () => ({
        get: (name, fallback) => settings.has(name) ? settings.get(name) : fallback,
        async update(name, value) { settings.set(name, value); },
      }),
      getWorkspaceFolder: () => undefined,
      onDidChangeConfiguration: () => disposable(),
      onDidChangeTextDocument: () => disposable(),
      onDidSaveTextDocument: () => disposable(),
      onDidCloseTextDocument: () => disposable(),
    },
  };

  const originalLoad = Module._load;
  Module._load = function load(request, parent, isMain) {
    if (request === "vscode") {
      return fakeVscode;
    }
    return originalLoad.call(this, request, parent, isMain);
  };
  try {
    const extensionPath = path.join(__dirname, "..");
    const extension = require("../extension");
    const context = { extensionPath, subscriptions: [] };
    extension.activate(context);

    assert.equal(commands.size, 21);
    assert.equal(completions.length, 1);
    assert.equal(hovers.length, 1);
    assert.equal(symbols.length, 1);
    assert.equal(context.subscriptions.length, 33);

    const items = completions[0].provider.provideCompletionItems();
    assert.equal(items.length, 356);
    for (const name of ["csv_join", "tsv_join"]) assert(items.some((item) => item.label === name && item.kind === 2));
    assert(items.some((item) => item.label === "time_date" && item.kind === 2));
    assert(items.some((item) => item.label === "csv_numbers" && item.kind === 2));
    assert(items.some((item) => item.label === "tsv_column" && item.kind === 2));
    assert(items.some((item) => item.label === "import"));
    assert(items.some((item) => item.label === "sum" && item.kind === 2));
    assert(items.some((item) => item.label === "mean" && item.kind === 2));
    for (const name of ["sort", "median", "quantile", "std_population", "std_sample", "ecdf"]) {
      assert(items.some((item) => item.label === name && item.kind === 2));
    }
    assert(items.some((item) => item.label === "π" && item.kind === 3));
    assert(items.some((item) => item.label === "km" && item.kind === 4));
    assert(items.some((item) => item.label === "fits_mean" && item.kind === 2));
    assert(items.some((item) => item.label === "fits_select_stats" && item.kind === 2));
    for (const name of ["fits_where", "fits_all", "fits_any", "fits_column_text", "fits_export_csv", "fits_export_tsv"]) {
      assert(items.some((item) => item.label === name && item.kind === 2));
    }
    assert(items.some((item) => item.label === "sind" && item.kind === 2));
    assert(items.some((item) => item.label === "sinr" && item.kind === 2));
    assert.match(items.find((item) => item.label === "for").insertText.value, /range/);
    assert.match(items.find((item) => item.label === "switch").insertText.value, /default/);
    assert.match(items.find((item) => item.label === "g_func").insertText.value, /return/);
    assert.match(items.find((item) => item.label === "parse_number").insertText.value, /text/);
    assert.match(items.find((item) => item.label === "parse_integer").insertText.value, /text/);
    assert.match(items.find((item) => item.label === "sqrt").insertText.value, /value/);
    assert.match(items.find((item) => item.label === "hypot").insertText.value, /x/);
    assert.match(items.find((item) => item.label === "is_close").insertText.value, /relative_tolerance/);
    assert.match(items.find((item) => item.label === "same_bits").insertText.value, /expected/);
    assert.match(items.find((item) => item.label === "spherical2cartesiand").insertText.value, /inclination/);
    assert.match(items.find((item) => item.label === "velocity_add_relativistic_collinear").insertText.value, /first/);
    assert.match(items.find((item) => item.label === "chem_molar_mass").insertText.value, /H2O/);
    assert(items.some((item) => item.label === "µL" && item.kind === 4));
    assert.match(items.find((item) => item.label === "input").insertText.value, /Prompt/);
    assert(commands.has("goblinpp.runCompiledFile"));
    for (const name of ["encryptFile", "encryptText", "decryptFile", "readTextFile"]) assert(commands.has("goblinpp." + name));
    assert(items.some(item => item.label === "str_b64"));

    await commands.get("goblinpp.insertSymbol")();
    assert.equal(inserted, "hbar");
    await commands.get("goblinpp.toggleIconView")();
    assert.equal(settings.get("iconView.enabled"), true);
    assert.equal(lastDecorations.length, 1);
    assert.equal(lastDecorations[0].renderOptions.after.contentText, " ℏ");
    assert.equal(source, "energy = hbar * omega # hbar");

    await commands.get("goblinpp.setIdentifierIcon")();
    assert.deepEqual(settings.get("iconMappings"), { energy: "⚡" });
    assert.deepEqual(
      lastDecorations.map((item) => item.renderOptions.after.contentText),
      [" ⚡", " ℏ"],
    );
    assert.equal(source, "energy = hbar * omega # hbar");

    const launched = [];
    const answers = [];
    const originalSpawn = childProcess.spawn;
    settings.set("executablePath", "/mock/goblin++");
    childProcess.spawn = (executable, args, options) => {
      launched.push({ executable, args, options });
      const process = new EventEmitter();
      process.stdout = new EventEmitter();
      process.stderr = new EventEmitter();
      process.stdin = { write(text) { answers.push(text); }, end() {} };
      queueMicrotask(() => {
        if (launched.length === 1) {
          process.stderr.emit("data", "GOBLIN_INPUT_PROMPT_V1\t4e616d653f20\n");
        }
        if (launched.length === 2) process.stderr.emit("data", "GOBLIN_SECRET_PROMPT_V1\t506173737068726173653a20\n");
        process.stdout.emit("data", "RUN_STATUS=PASS\n");
        setImmediate(() => process.emit("close", 0));
      });
      return process;
    };
    try {
      await commands.get("goblinpp.runFile")();
      await commands.get("goblinpp.runCompiledFile")();
      await commands.get("goblinpp.runFileWithArguments")();
    } finally {
      childProcess.spawn = originalSpawn;
    }
    assert.deepEqual(launched.map((item) => item.args), [
      ["run", "/tmp/energy.gbl"],
      ["run", "/tmp/energy.gbl", "--compile"],
      ["run", "/tmp/energy.gbl", "--", "--name", "Ada"],
    ]);
    assert.deepEqual(answers.map(a => a.toString()), ["Ada\n", "public masked code passphrase\n"]);
    assert(!confidentialLog.join("\n").includes("public masked code passphrase"));

    // File protection prompts hide passwords and send them only through child stdin.
    const secret = "public mock passphrase";
    const privateText = "private mock text";
    const secureLaunches = [];
    const prompts = [];
    fakeVscode.window.showOpenDialog = async () => [{scheme: "file", fsPath: "/tmp/input.age"}];
    fakeVscode.window.showSaveDialog = async () => ({scheme: "file", fsPath: "/tmp/output.file"});
    fakeVscode.window.showWarningMessage = async () => "Display text";
    fakeVscode.window.showInputBox = async opts => { prompts.push(opts); return opts.prompt.startsWith("Text to encrypt") ? privateText : secret; };
    childProcess.spawn = (executable, args, options) => {
      const process = new EventEmitter();
      process.stdout = new EventEmitter(); process.stderr = new EventEmitter();
      const call = {args, options}; secureLaunches.push(call);
      process.stdin = new EventEmitter();
      process.stdin.end = (bytes, callback) => { call.payload = Buffer.from(bytes); if (callback) callback(); call.cleared = bytes.every(n => n === 0); };
      queueMicrotask(() => { process.stdout.emit("data", Buffer.from("STATUS=PASS\n")); process.emit("close", 0); });
      return process;
    };
    try {
      for (const mode of ["encryptFile", "encryptText", "decryptFile", "readTextFile"]) await commands.get("goblinpp." + mode)();
      assert.equal(secureLaunches.length, 4);
      assert(secureLaunches.every(call => call.options.shell === false));
      assert(secureLaunches.every(call => !call.args.join(" ").includes(secret) && !call.args.join(" ").includes(privateText)));
      assert.equal(secureLaunches[1].payload.toString(), secret + "\n" + privateText);
      assert(secureLaunches.every(call => call.cleared));
      assert(prompts.every(opts => opts.password === true));
      assert(!confidentialLog.join("\n").includes(secret)); assert(!confidentialLog.join("\n").includes(privateText));
      fakeVscode.window.showInputBox = async () => undefined;
      await commands.get("goblinpp.encryptFile")();
      assert.equal(secureLaunches.length,4,"cancelled password prompt must not spawn");
      fakeVscode.workspace.isTrusted = false;
      await commands.get("goblinpp.encryptFile")();
      assert.equal(secureLaunches.length,4,"an untrusted workspace must not start file protection");
      fakeVscode.workspace.isTrusted = true;
    } finally { childProcess.spawn = originalSpawn; }

    assert(launched.every((item) => item.executable === "/mock/goblin++" && item.options.shell === false));
  } finally {
    Module._load = originalLoad;
  }
});

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
    DiagnosticSeverity: { Error: 0 },
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
      createOutputChannel: () => ({ append() {}, appendLine() {}, show() {}, dispose() {} }),
      createStatusBarItem: () => ({ show() {}, dispose() {} }),
      createTextEditorDecorationType: () => disposable(),
      onDidChangeVisibleTextEditors: () => disposable(),
      showQuickPick: async (items) => items.find((item) => item.insertText === "hbar"),
      showInputBox: async (options) => options?.title === "Goblin++ program input" ? "Ada" : options?.title === "Goblin++ program arguments" ? '["--name", "Ada"]' : "⚡",
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

    assert.equal(commands.size, 17);
    assert.equal(completions.length, 1);
    assert.equal(hovers.length, 1);
    assert.equal(symbols.length, 1);
    assert.equal(context.subscriptions.length, 29);

    const items = completions[0].provider.provideCompletionItems();
    assert.equal(items.length, 73);
    assert(items.some((item) => item.label === "π" && item.kind === 3));
    assert(items.some((item) => item.label === "km" && item.kind === 4));
    assert(items.some((item) => item.label === "fits_mean" && item.kind === 2));
    assert(items.some((item) => item.label === "fits_select_stats" && item.kind === 2));
    assert.match(items.find((item) => item.label === "for").insertText.value, /range/);
    assert.match(items.find((item) => item.label === "switch").insertText.value, /default/);
    assert.match(items.find((item) => item.label === "g_func").insertText.value, /return/);
    assert.match(items.find((item) => item.label === "parse_number").insertText.value, /text/);
    assert.match(items.find((item) => item.label === "input").insertText.value, /Prompt/);
    assert(commands.has("goblinpp.runCompiledFile"));

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
    assert.deepEqual(answers, ["Ada\n"]);
    assert(launched.every((item) => item.executable === "/mock/goblin++" && item.options.shell === false));
  } finally {
    Module._load = originalLoad;
  }
});

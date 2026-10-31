"use strict";

const childProcess = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");
const vscode = require("vscode");
const core = require("./editor-core");

let output;
let status;
let extensionLexicon;
let iconDecorationType;
let diagnosticCollection;
const checkGenerations = new Map();

const identifierPattern = /[A-Za-z_πħωΩΔΣλμσθ∇∂][A-Za-z0-9_πħωΩΔΣλμσθ∇∂]*/u;

function workspaceRootFor(uri) {
  const folder = uri ? vscode.workspace.getWorkspaceFolder(uri) : undefined;
  if (folder) {
    return folder.uri.fsPath;
  }
  const first = vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders[0];
  return first ? first.uri.fsPath : undefined;
}

function ensureTrusted() {
  if (vscode.workspace.isTrusted) {
    return true;
  }
  vscode.window.showWarningMessage(
    "Goblin++ commands are disabled because this VS Code workspace is not trusted.",
  );
  return false;
}

async function currentSource(saveRequired = false) {
  if (!ensureTrusted()) {
    return undefined;
  }
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.uri.scheme !== "file") {
    vscode.window.showWarningMessage("Open a local Goblin++ .gbl file first.");
    return undefined;
  }
  if (editor.document.languageId !== "goblinpp" && !editor.document.fileName.endsWith(".gbl")) {
    vscode.window.showWarningMessage("The active file is not a Goblin++ .gbl program.");
    return undefined;
  }
  if (editor.document.isDirty) {
    if (!saveRequired) {
      vscode.window.showWarningMessage(
        "Save the current file before inspecting its on-disk Goblin++ custody state.",
      );
      return undefined;
    }
    if (!(await editor.document.save())) {
      vscode.window.showErrorMessage("Goblin++ stopped because the current file could not be saved.");
      return undefined;
    }
  }
  return editor.document.uri;
}

function executableFor(workspaceRoot) {
  const configured = vscode.workspace
    .getConfiguration("goblinpp")
    .get("executablePath", "");
  return core.selectExecutable(configured, workspaceRoot, fs.existsSync, os.homedir());
}

function setStatus(kind) {
  if (kind === "check-pass") {
    status.text = "$(beaker) Goblin++ CHECK PASS";
    status.tooltip = "Saved source passed a read-only machinery preview. This is not evidence.";
    status.backgroundColor = undefined;
  } else if (kind === "check-fail") {
    status.text = "$(warning) Goblin++ CHECK FAIL";
    status.tooltip = "Saved source failed a read-only machinery preview.";
    status.backgroundColor = new vscode.ThemeColor("statusBarItem.warningBackground");
  } else if (kind === "pass") {
    status.text = "$(pass-filled) Goblin++ PASS";
    status.tooltip = "The most recent Goblin++ command passed.";
    status.backgroundColor = undefined;
  } else if (kind === "protocol-violation") {
    status.text = "$(shield) Goblin++ PROTOCOL VIOLATION";
    status.tooltip = "Goblin++ refused a custody or frozen-source violation.";
    status.backgroundColor = new vscode.ThemeColor("statusBarItem.warningBackground");
  } else if (kind === "verification-failure") {
    status.text = "$(shield) Goblin++ VERIFY FAIL";
    status.tooltip = "Preserved run evidence did not verify.";
    status.backgroundColor = new vscode.ThemeColor("statusBarItem.errorBackground");
  } else if (kind === "machinery-failure") {
    status.text = "$(error) Goblin++ MACHINERY FAIL";
    status.tooltip = "The program failed lexically, syntactically, or semantically.";
    status.backgroundColor = new vscode.ThemeColor("statusBarItem.errorBackground");
  } else {
    status.text = "$(error) Goblin++ COMMAND FAILED";
    status.tooltip = "The Goblin++ command did not complete successfully.";
    status.backgroundColor = new vscode.ThemeColor("statusBarItem.errorBackground");
  }
  status.show();
}

function runCli(command, target, extra, cwd, label) {
  return new Promise((resolve) => {
    const executable = executableFor(cwd);
    const args = core.goblinArgs(command, target, extra);
    output.appendLine(`\n=== ${label} ===`);
    output.appendLine(`Executable: ${executable}`);
    output.appendLine(`Arguments: ${JSON.stringify(args)}`);
    output.appendLine(`Workspace: ${cwd}`);
    if (vscode.workspace.getConfiguration("goblinpp").get("revealOutput", true)) {
      output.show(true);
    }

    const child = childProcess.spawn(executable, args, {
      cwd,
      shell: false,
      windowsHide: true,
      env: command === "run"
        ? { ...process.env, GOBLIN_INPUT_PROMPT_PROTOCOL: "hex-v1" }
        : process.env,
    });
    let stdout = "";
    let stderr = "";
    let stderrPending = "";
    let startFailed = false;
    child.stdout.on("data", (chunk) => {
      const text = chunk.toString();
      stdout += text;
      output.append(text);
    });
    child.stderr.on("data", (chunk) => {
      const text = chunk.toString();
      stderr += text;
      if (command !== "run") {
        output.append(text);
        return;
      }
      stderrPending += text;
      let newline;
      while ((newline = stderrPending.indexOf("\n")) !== -1) {
        const line = stderrPending.slice(0, newline).replace(/\r$/, "");
        stderrPending = stderrPending.slice(newline + 1);
        const secretPrompt = core.decodeSecretPromptLine(line);
        if (secretPrompt !== null) {
          output.appendLine("Secret input requested (response is not logged).");
          vscode.window.showInputBox({title: "Goblin++ secret input", prompt: secretPrompt, password: true, ignoreFocusOut: true,
            validateInput: value => !value || /[\r\n\0]/u.test(value) || Buffer.byteLength(value, "utf8") > 4096 ? "Use 1-4096 UTF-8 bytes, without newline or NUL." : undefined}).then(answer => {
              if (answer === undefined || !answer || /[\r\n\0]/u.test(answer) || Buffer.byteLength(answer,"utf8")>4096) child.stdin?.end();
              else { const bytes=Buffer.from(answer+"\n","utf8");child.stdin?.write(bytes,()=>bytes.fill(0)); }
            },()=>child.stdin?.end());
          continue;
        }
        if (line.startsWith(core.SECRET_PROMPT_PREFIX)) { child.stdin?.end(); output.appendLine("Invalid secret prompt; input closed."); continue; }
        const prompt = core.decodeInputPromptLine(line);
        if (prompt !== null) {
          output.appendLine("Input requested: " + prompt);
          vscode.window.showInputBox({
            title: "Goblin++ program input",
            prompt,
            ignoreFocusOut: true,
          }).then((answer) => {
            if (answer === undefined || answer.includes("\n") || answer.includes("\r")) {
              child.stdin?.end();
            } else {
              child.stdin?.write(answer + "\n");
            }
          }, () => child.stdin?.end());
        } else {
          output.appendLine(line);
          if (line.startsWith(core.INPUT_PROMPT_PREFIX)) {
            child.stdin?.end();
          }
        }
      }
    });
    child.on("error", (error) => {
      startFailed = true;
      output.appendLine(`Unable to start Goblin++: ${error.message}`);
      setStatus("failure");
      vscode.window.showErrorMessage(
        `Goblin++ could not start ${executable}. Configure goblinpp.executablePath if needed.`,
      );
      resolve({ exitCode: null, kind: "failure", stdout, stderr });
    });
    child.on("close", (exitCode) => {
      if (startFailed) {
        return;
      }
      if (stderrPending) {
        output.append(stderrPending);
      }
      const kind = core.resultKind(stdout, stderr, exitCode);
      setStatus(kind);
      if (kind === "pass") {
        vscode.window.showInformationMessage(`Goblin++ ${label}: PASS`);
      } else if (kind === "protocol-violation") {
        vscode.window.showWarningMessage(`Goblin++ ${label}: protocol violation refused.`);
      } else if (kind === "verification-failure") {
        vscode.window.showErrorMessage(`Goblin++ ${label}: evidence verification failed.`);
      } else if (kind === "machinery-failure") {
        vscode.window.showErrorMessage(`Goblin++ ${label}: program machinery failed.`);
      } else {
        vscode.window.showErrorMessage(`Goblin++ ${label}: command failed.`);
      }
      resolve({ exitCode, kind, stdout, stderr });
    });
  });
}

async function runSourceCommand(command, label, extra = [], saveRequired = false) {
  const uri = await currentSource(saveRequired);
  if (!uri) {
    return;
  }
  const root = workspaceRootFor(uri) || path.dirname(uri.fsPath);
  await runCli(command, uri.fsPath, extra, root, label);
}

function registerCommand(context, name, callback) {
  context.subscriptions.push(vscode.commands.registerCommand(name, callback));
}

function editableGoblinEditor() {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== "goblinpp") {
    vscode.window.showWarningMessage("Open a Goblin++ .gbl editor first.");
    return undefined;
  }
  return editor;
}

function configurationTarget() {
  return vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders.length
    ? vscode.ConfigurationTarget.Workspace
    : vscode.ConfigurationTarget.Global;
}

function iconViewConfiguration() {
  return vscode.workspace.getConfiguration("goblinpp");
}

function applyIconView(editor) {
  if (!iconDecorationType || !editor || typeof editor.setDecorations !== "function") {
    return;
  }
  const configuration = iconViewConfiguration();
  const enabled = configuration.get("iconView.enabled", false);
  if (!enabled || editor.document.languageId !== "goblinpp") {
    editor.setDecorations(iconDecorationType, []);
    return;
  }
  const mappings = core.iconMappings(
    extensionLexicon,
    configuration.get("iconMappings", {}),
  );
  const decorations = core
    .iconDecorations(editor.document.getText(), mappings)
    .map((item) => ({
      range: new vscode.Range(
        editor.document.positionAt(item.start),
        editor.document.positionAt(item.end),
      ),
      hoverMessage:
        `Icon View: ${item.name} displays ${item.icon}. ` +
        `${item.kind}; ${item.detail}. Source text is unchanged.`,
      renderOptions: {
        after: { contentText: ` ${item.icon}` },
      },
    }));
  editor.setDecorations(iconDecorationType, decorations);
}

function refreshIconViews() {
  for (const editor of vscode.window.visibleTextEditors || []) {
    applyIconView(editor);
  }
}

function publishCheckReport(uri, report, announce) {
  const warningDiagnostics = report.warnings.map((warning) => {
    const diagnostic = new vscode.Diagnostic(
      new vscode.Range(0, 0, 0, 0),
      warning,
      vscode.DiagnosticSeverity.Warning,
    );
    diagnostic.code = "G302";
    diagnostic.source = "Goblin++ check (preview only)";
    return diagnostic;
  });
  if (report.status === "PASS") {
    if (warningDiagnostics.length) {
      diagnosticCollection.set(uri, warningDiagnostics);
    } else {
      diagnosticCollection.delete(uri);
    }
    setStatus("check-pass");
    if (announce) {
      const message = warningDiagnostics.length
        ? `Goblin++ check: PASS with ${warningDiagnostics.length} migration warning(s) (preview only).`
        : "Goblin++ check: PASS (preview only; no evidence or custody check).";
      vscode.window.showInformationMessage(message);
    }
    return;
  }
  const item = report.diagnostics[0];
  const range = item.range
    ? new vscode.Range(item.range.start.line, item.range.start.character,
      item.range.end.line, item.range.end.character)
    : new vscode.Range(0, 0, 0, 0);
  const diagnostic = new vscode.Diagnostic(
    range,
    `${item.code}: ${item.message}`,
    vscode.DiagnosticSeverity.Error,
  );
  diagnostic.code = item.code;
  diagnostic.source = "Goblin++ check (preview only)";
  diagnosticCollection.set(uri, [...warningDiagnostics, diagnostic]);
  setStatus("check-fail");
  if (announce) {
    vscode.window.showWarningMessage(
      `Goblin++ check: ${item.code} ${item.message.split("\n", 1)[0]} (preview only).`,
    );
  }
}

function runCheck(uri, announce = false) {
  const root = workspaceRootFor(uri) || path.dirname(uri.fsPath);
  const executable = executableFor(root);
  const args = core.goblinArgs("check", uri.fsPath, ["--json"]);
  const generation = (checkGenerations.get(uri.fsPath) || 0) + 1;
  checkGenerations.set(uri.fsPath, generation);
  if (announce) {
    output.appendLine("\n=== read-only check ===");
    output.appendLine(`Executable: ${executable}`);
    output.appendLine(`Arguments: ${JSON.stringify(args)}`);
    output.appendLine(`Workspace: ${root}`);
    if (vscode.workspace.getConfiguration("goblinpp").get("revealOutput", true)) {
      output.show(true);
    }
  }
  return new Promise((resolve) => {
    const child = childProcess.spawn(executable, args, {
      cwd: root,
      shell: false,
      windowsHide: true,
    });
    let stdout = "";
    let stderr = "";
    let startFailed = false;
    child.stdout.on("data", (chunk) => { stdout += chunk.toString(); });
    child.stderr.on("data", (chunk) => { stderr += chunk.toString(); });
    child.on("error", (error) => {
      startFailed = true;
      if (announce) {
        output.appendLine(`Unable to start Goblin++ check: ${error.message}`);
        vscode.window.showErrorMessage(
          `Goblin++ could not start ${executable}. Configure goblinpp.executablePath if needed.`,
        );
      }
      setStatus("failure");
      resolve(undefined);
    });
    child.on("close", (exitCode) => {
      if (startFailed) {
        return;
      }
      if (checkGenerations.get(uri.fsPath) !== generation) {
        resolve(undefined);
        return;
      }
      if (announce) {
        output.append(stdout);
        output.append(stderr);
      }
      let report;
      try {
        report = core.parseCheckResponse(stdout, stderr, exitCode);
      } catch (error) {
        diagnosticCollection.delete(uri);
        setStatus("failure");
        if (announce) {
          vscode.window.showErrorMessage(error.message);
        }
        resolve(undefined);
        return;
      }
      publishCheckReport(uri, report, announce);
      resolve(report);
    });
  });
}

function completionProvider(entries) {
  return {
    provideCompletionItems() {
      return entries.map((entry) => {
        const kinds = {
          statement: vscode.CompletionItemKind.Keyword,
          function: vscode.CompletionItemKind.Function,
          constant: vscode.CompletionItemKind.Constant,
          unit: vscode.CompletionItemKind.Unit,
        };
        const item = new vscode.CompletionItem(entry.spelling, kinds[entry.kind]);
        item.detail = `Goblin++ ${entry.kind}: ${entry.detail}`;
        const snippet = core.completionSnippet(entry);
        if (snippet) {
          item.insertText = new vscode.SnippetString(snippet);
        }
        return item;
      });
    },
  };
}

function hoverProvider(entries) {
  const bySpelling = new Map(entries.map((entry) => [entry.spelling, entry]));
  return {
    provideHover(document, position) {
      const range = document.getWordRangeAtPosition(
        position,
        /[A-Za-z_πħωΩΔΣλμσθ∇∂][A-Za-z0-9_πħωΩΔΣλμσθ∇∂]*/,
      );
      if (!range) {
        return undefined;
      }
      const spelling = document.getText(range);
      const entry = bySpelling.get(spelling);
      if (!entry) {
        return undefined;
      }
      const markdown = new vscode.MarkdownString();
      markdown.appendMarkdown(`**${entry.spelling}** — Goblin++ ${entry.kind}\n\n`);
      markdown.appendText(entry.detail);
      return new vscode.Hover(markdown, range);
    },
  };
}

function symbolProvider() {
  const identifier = "[A-Za-z_πħωΩΔΣλμσθ∇∂][A-Za-z0-9_πħωΩΔΣλμσθ∇∂]*";
  const assignment = new RegExp(`^\\s*(${identifier})\\s*=(?!=)`);
  const sealed = new RegExp(`^\\s*seal\\s+(${identifier})\\s*(?:#.*)?$`);
  return {
    provideDocumentSymbols(document) {
      const symbols = [];
      for (let lineNumber = 0; lineNumber < document.lineCount; lineNumber += 1) {
        const line = document.lineAt(lineNumber);
        const assignmentMatch = assignment.exec(line.text);
        const sealMatch = sealed.exec(line.text);
        if (assignmentMatch) {
          symbols.push(
            new vscode.DocumentSymbol(
              assignmentMatch[1],
              "assignment",
              vscode.SymbolKind.Variable,
              line.range,
              line.range,
            ),
          );
        } else if (sealMatch) {
          symbols.push(
            new vscode.DocumentSymbol(
              sealMatch[1],
              "sealed artifact",
              vscode.SymbolKind.File,
              line.range,
              line.range,
            ),
          );
        }
      }
      return symbols;
    },
  };
}


// Confidential commands never use runCli's scientific input protocol or argument logger.
// Passwords/text travel over a private pipe, not process arguments or an IDE terminal.
async function confidentialCommand(mode) {
  if (!ensureTrusted()) return;
  let input;
  let text;
  if (mode === "encryptText") {
    text = await vscode.window.showInputBox({prompt: "Text to encrypt (hidden; not saved in a .gbl file)", password: true, ignoreFocusOut: true, validateInput: value => Buffer.byteLength(value, "utf8") > 1048576 ? "New text entry is limited to 1 MiB; encrypt a file instead." : undefined});
    if (text === undefined) return;
  } else {
    const chosen = await vscode.window.showOpenDialog({canSelectFiles: true, canSelectFolders: false, canSelectMany: false, title: mode === "decryptFile" ? "Select an age-encrypted file" : "Select a local file"});
    if (!chosen || !chosen[0]) return;
    input = chosen[0];
    if (input.scheme !== "file") { vscode.window.showErrorMessage("Select a local file."); return; }
  }
  const reading = mode === "readTextFile";
  let destination;
  let password;
  const encrypt = mode.startsWith("encrypt");
  if (!reading) {
    destination = await vscode.window.showSaveDialog({title: "Choose a NEW output file (existing files are refused)", saveLabel: encrypt ? "Encrypt to new file" : "Decrypt to new file"});
    if (!destination || destination.scheme !== "file") return;
    password = await vscode.window.showInputBox({prompt: encrypt ? "Long, unique encryption passphrase (minimum 12 UTF-8 bytes)" : "Decryption passphrase", password: true, ignoreFocusOut: true,
      validateInput: value => /[\r\n\0]/u.test(value) || Buffer.byteLength(value, "utf8") > 4096 || !value || (encrypt && Buffer.byteLength(value, "utf8") < 12) ? "Use a valid passphrase; encryption requires at least 12 UTF-8 bytes." : undefined});
    if (password === undefined) return;
    if (encrypt) {
      const confirmation = await vscode.window.showInputBox({prompt: "Confirm encryption passphrase", password: true, ignoreFocusOut: true});
      if (confirmation === undefined) return;
      if (confirmation !== password) { vscode.window.showErrorMessage("Passphrases did not match."); return; }
    }
  } else {
    const answer = await vscode.window.showWarningMessage("Display plaintext in the Goblin++ Output panel? It may remain in IDE logs and screen history.", {modal: true}, "Display text");
    if (answer !== "Display text") return;
  }
  const cwd = workspaceRootFor(input) || (input ? path.dirname(input.fsPath) : process.cwd());
  const args = reading ? ["read-file", input.fsPath] : [encrypt ? "encrypt" : "decrypt", "--key-stdin", "--output", destination.fsPath,
    ...(input ? ["--input", input.fsPath] : ["--text-stdin"])];
  await new Promise(resolve => {
    const child = childProcess.spawn(executableFor(cwd), args, {cwd, shell: false, windowsHide: true});
    let seen = 0;
    let failed = false;
    output.appendLine(`\n=== Goblin++ ${reading ? "explicit text preview" : "confidential file operation"} ===`);
    output.show(true);
    const receive = chunk => {
      seen += chunk.length;
      if (seen > 8 * 1024 * 1024) { failed = true; child.kill(); return; }
      output.append(chunk.toString());
    };
    child.stdout.on("data", receive);
    child.stderr.on("data", receive);
    child.on("error", () => { failed = true; vscode.window.showErrorMessage("Goblin++ could not start. Check the configured executable."); resolve(); });
    child.stdin.on("error", () => {}); // An authentication/validation failure can close stdin early.
    const payload = reading ? Buffer.alloc(0) : Buffer.from(password + "\n" + (text === undefined ? "" : text), "utf8");
    password = undefined; text = undefined;
    child.stdin.end(payload, () => payload.fill(0));
    child.on("close", code => {
      if (!failed && code === 0) vscode.window.showInformationMessage(reading ? "Goblin++ text preview completed." : "Goblin++ confidential file operation completed. Original files remain unchanged.");
      else vscode.window.showErrorMessage("Goblin++ file operation did not complete. See its redacted diagnostic.");
      resolve();
    });
  });
}

function activate(context) {
  for (const mode of ["encryptFile", "encryptText", "decryptFile", "readTextFile"]) {
    registerCommand(context, "goblinpp." + mode, () => confidentialCommand(mode));
  }
  output = vscode.window.createOutputChannel("Goblin++");
  status = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
  status.command = "goblinpp.statusFile";
  status.text = "$(shield) Goblin++ ready";
  status.tooltip = "Select to inspect the current Goblin++ source.";
  context.subscriptions.push(output, status);
  diagnosticCollection = vscode.languages.createDiagnosticCollection("goblinpp-check");
  context.subscriptions.push(diagnosticCollection);

  const lexiconPath = path.join(context.extensionPath, "spec", "lexicon.v0.json");
  const lexicon = JSON.parse(fs.readFileSync(lexiconPath, "utf8"));
  const runtimeVocabularyPath = path.join(context.extensionPath, "spec", "rust-alpha33-editor.json");
  const runtimeVocabulary = JSON.parse(fs.readFileSync(runtimeVocabularyPath, "utf8"));
  extensionLexicon = lexicon;
  const entries = core.vocabularyEntries(lexicon, runtimeVocabulary);
  const selector = { language: "goblinpp", scheme: "file" };
  context.subscriptions.push(
    vscode.languages.registerCompletionItemProvider(selector, completionProvider(entries)),
    vscode.languages.registerHoverProvider(selector, hoverProvider(entries)),
    vscode.languages.registerDocumentSymbolProvider(selector, symbolProvider()),
  );

  iconDecorationType = vscode.window.createTextEditorDecorationType({
    after: {
      color: new vscode.ThemeColor("descriptionForeground"),
      fontStyle: "normal",
      margin: "0 0 0 0.3em",
    },
  });
  context.subscriptions.push(iconDecorationType);

  registerCommand(context, "goblinpp.insertSymbol", async () => {
    const editor = editableGoblinEditor();
    if (!editor) {
      return;
    }
    const selected = await vscode.window.showQuickPick(core.symbolPaletteItems(lexicon), {
      matchOnDescription: true,
      matchOnDetail: true,
      placeHolder: "Choose an exact Goblin++ source spelling",
      title: "Insert a scientific symbol or notation alias",
    });
    if (!selected) {
      return;
    }
    const changed = await editor.edit((builder) => {
      for (const selection of editor.selections) {
        builder.replace(selection, selected.insertText);
      }
    });
    if (!changed) {
      vscode.window.showErrorMessage("Goblin++ could not insert the selected symbol.");
    }
  });
  registerCommand(context, "goblinpp.toggleIconView", async () => {
    const configuration = iconViewConfiguration();
    const enabled = !configuration.get("iconView.enabled", false);
    await configuration.update("iconView.enabled", enabled, configurationTarget());
    refreshIconViews();
    vscode.window.showInformationMessage(
      `Goblin++ Icon View is ${enabled ? "on" : "off"}. Source text is unchanged.`,
    );
  });
  registerCommand(context, "goblinpp.setIdentifierIcon", async () => {
    const editor = editableGoblinEditor();
    if (!editor) {
      return;
    }
    const selection = editor.selection;
    const range = selection && !selection.isEmpty
      ? selection
      : editor.document.getWordRangeAtPosition(selection.active, identifierPattern);
    const identifier = range ? editor.document.getText(range) : "";
    if (!core.isGoblinIdentifier(identifier)) {
      vscode.window.showWarningMessage(
        "Select one complete Goblin++ identifier or place the cursor inside it first.",
      );
      return;
    }
    const configuration = iconViewConfiguration();
    const currentMappings = core.normalizeUserIconMappings(
      configuration.get("iconMappings", {}),
    );
    const value = await vscode.window.showInputBox({
      title: `Visual icon for ${identifier}`,
      prompt:
        "Enter a mnemonic icon. Leave blank to remove the mapping. This never changes Goblin++ meaning.",
      value: currentMappings[identifier] || "",
      ignoreFocusOut: true,
      validateInput: (candidate) => core.iconMappingError(candidate, true),
    });
    if (value === undefined) {
      return;
    }
    const nextMappings = { ...currentMappings };
    if (value.trim()) {
      nextMappings[identifier] = value.trim();
    } else {
      delete nextMappings[identifier];
    }
    await configuration.update("iconMappings", nextMappings, configurationTarget());
    refreshIconViews();
    vscode.window.showInformationMessage(
      value.trim()
        ? `Goblin++ will display ${value.trim()} beside ${identifier} in Icon View.`
        : `Goblin++ removed the visual icon for ${identifier}.`,
    );
  });
  registerCommand(context, "goblinpp.checkFile", async () => {
    const uri = await currentSource(false);
    if (uri) {
      await runCheck(uri, true);
    }
  });
  registerCommand(context, "goblinpp.toggleCheckOnSave", async () => {
    const configuration = vscode.workspace.getConfiguration("goblinpp");
    const enabled = !configuration.get("diagnostics.onSave", false);
    await configuration.update("diagnostics.onSave", enabled, configurationTarget());
    if (!enabled) {
      diagnosticCollection.clear();
    }
    vscode.window.showInformationMessage(
      `Goblin++ check on save is ${enabled ? "on" : "off"}. Checks are previews, not evidence.`,
    );
  });

  registerCommand(context, "goblinpp.runFile", () =>
    runSourceCommand("run", "run", [], true),
  );
  registerCommand(context, "goblinpp.runFileWithArguments", async () => {
    const raw = await vscode.window.showInputBox({
      title: "Goblin++ program arguments",
      prompt: 'Enter a JSON array of text arguments, for example ["--name", "Ada Lovelace"].',
      value: "[]",
      ignoreFocusOut: true,
    });
    if (raw === undefined) {
      return;
    }
    let programArgs;
    try {
      programArgs = core.parseProgramArguments(raw);
    } catch (error) {
      vscode.window.showErrorMessage("Invalid Goblin++ program arguments: " + error.message);
      return;
    }
    await runSourceCommand("run", "run with arguments", ["--", ...programArgs], true);
  });
  registerCommand(context, "goblinpp.runCompiledFile", () =>
    runSourceCommand("run", "compiled run", ["--compile"], true),
  );
  registerCommand(context, "goblinpp.statusFile", () =>
    runSourceCommand("status", "source status"),
  );
  registerCommand(context, "goblinpp.lineageFile", () =>
    runSourceCommand("lineage", "lineage"),
  );
  registerCommand(context, "goblinpp.freezeFile", async () => {
    const uri = await currentSource(true);
    if (!uri) {
      return;
    }
    const answer = await vscode.window.showWarningMessage(
      `Freeze exact source bytes?\n${uri.fsPath}\n\nFuture changes require a child revision.`,
      { modal: true },
      "Freeze exact source",
    );
    if (answer !== "Freeze exact source") {
      return;
    }
    const root = workspaceRootFor(uri) || path.dirname(uri.fsPath);
    await runCli("freeze", uri.fsPath, [], root, "freeze");
  });
  registerCommand(context, "goblinpp.reviseFile", async () => {
    const parent = await currentSource(true);
    if (!parent) {
      return;
    }
    const child = await vscode.window.showSaveDialog({
      defaultUri: vscode.Uri.file(
        parent.fsPath.replace(/\.gbl$/i, "_R1.gbl"),
      ),
      filters: { "Goblin++ programs": ["gbl"] },
      saveLabel: "Choose child revision",
      title: "Create a new Goblin++ child revision",
    });
    if (!child) {
      return;
    }
    if (fs.existsSync(child.fsPath)) {
      vscode.window.showErrorMessage("Goblin++ will not overwrite an existing child file.");
      return;
    }
    const reason = await vscode.window.showInputBox({
      title: "Reason for the Goblin++ revision",
      prompt: "State why this child revision is necessary.",
      ignoreFocusOut: true,
      validateInput: (value) => (value.trim() ? undefined : "A revision reason is required."),
    });
    if (!reason || !reason.trim()) {
      return;
    }
    const answer = await vscode.window.showWarningMessage(
      `Create child revision?\nParent: ${parent.fsPath}\nChild: ${child.fsPath}\nReason: ${reason.trim()}`,
      { modal: true },
      "Create revision",
    );
    if (answer !== "Create revision") {
      return;
    }
    const root = workspaceRootFor(parent) || path.dirname(parent.fsPath);
    const result = await runCli(
      "revise",
      parent.fsPath,
      [child.fsPath, "--reason", reason.trim()],
      root,
      "revision",
    );
    if (result.kind === "pass") {
      const document = await vscode.workspace.openTextDocument(child);
      await vscode.window.showTextDocument(document);
    }
  });
  registerCommand(context, "goblinpp.verifyRun", async () => {
    if (!ensureTrusted()) {
      return;
    }
    const selected = await vscode.window.showOpenDialog({
      canSelectFiles: false,
      canSelectFolders: true,
      canSelectMany: false,
      openLabel: "Verify this run",
      title: "Select a Goblin++ run directory",
    });
    if (!selected || !selected[0]) {
      return;
    }
    const root = workspaceRootFor(selected[0]) || path.dirname(selected[0].fsPath);
    await runCli("verify", selected[0].fsPath, [], root, "verification");
  });
  registerCommand(context, "goblinpp.auditLedger", async () => {
    if (!ensureTrusted()) {
      return;
    }
    const root = workspaceRootFor() || process.cwd();
    await runCli("audit-ledger", root, [], root, "ledger audit");
  });
  registerCommand(context, "goblinpp.doctor", async () => {
    if (!ensureTrusted()) {
      return;
    }
    const root = workspaceRootFor() || process.cwd();
    await runCli("doctor", root, [], root, "workspace check");
  });
  registerCommand(context, "goblinpp.openEditorGuide", async () => {
    const guide = vscode.Uri.file(path.join(context.extensionPath, "docs", "EDITOR_GUIDE.md"));
    const document = await vscode.workspace.openTextDocument(guide);
    await vscode.window.showTextDocument(document, { preview: true });
  });
  registerCommand(context, "goblinpp.openAuthoringTutorial", async () => {
    const tutorial = vscode.Uri.file(
      path.join(context.extensionPath, "docs", "AUTHORING_TUTORIAL.md"),
    );
    const document = await vscode.workspace.openTextDocument(tutorial);
    await vscode.window.showTextDocument(document, { preview: true });
  });

  context.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument((event) => {
      for (const editor of vscode.window.visibleTextEditors || []) {
        if (editor.document === event.document) {
          applyIconView(editor);
        }
      }
    }),
    vscode.window.onDidChangeVisibleTextEditors(() => refreshIconViews()),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (
        event.affectsConfiguration("goblinpp.iconView.enabled") ||
        event.affectsConfiguration("goblinpp.iconMappings")
      ) {
        refreshIconViews();
      }
    }),
    vscode.workspace.onDidSaveTextDocument((document) => {
      if (
        vscode.workspace.isTrusted &&
        document.uri.scheme === "file" &&
        document.languageId === "goblinpp" &&
        vscode.workspace.getConfiguration("goblinpp").get("diagnostics.onSave", false)
      ) {
        runCheck(document.uri, false);
      }
    }),
    vscode.workspace.onDidCloseTextDocument((document) => {
      diagnosticCollection.delete(document.uri);
      checkGenerations.delete(document.uri.fsPath);
    }),
  );
  refreshIconViews();
}

function deactivate() {}

module.exports = { activate, deactivate };

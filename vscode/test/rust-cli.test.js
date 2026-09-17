"use strict";

const assert = require("node:assert/strict");
const childProcess = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const test = require("node:test");
const core = require("../editor-core");

test("actual alpha.10 CLI check, run, and verify contracts", {
  skip: !process.env.GOBLINPP_BIN,
}, (context) => {
  const binary = process.env.GOBLINPP_BIN;
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "goblinpp-vscode-test-"));
  context.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const invoke = (args) => childProcess.spawnSync(binary, args, {
    cwd: root,
    encoding: "utf8",
    shell: false,
  });

  const version = invoke(["--version"]);
  assert.equal(version.status, 0);
  assert.match(version.stdout, /0\.1\.0-alpha\.10/);

  const source = path.join(root, "everyday.gbl");
  fs.writeFileSync(source, 'x = 2\nprint("x = {x}")\nwrite_text("answer.txt", "x = {x}")\n');
  const checked = invoke(core.goblinArgs("check", source, ["--json"]));
  assert.equal(checked.status, 0);
  const report = core.parseCheckResponse(checked.stdout, checked.stderr, checked.status);
  assert.equal(report.schema, "goblin.check.v1");
  assert.equal(report.status, "PASS");
  assert.equal(report.paranoid_mode, false);
  assert.equal(report.generated_output_preview.length, 1);

  const run = invoke(core.goblinArgs("run", source));
  assert.equal(run.status, 0);
  assert.equal(core.resultKind(run.stdout, run.stderr, run.status), "pass");
  const runDir = /^RUN_DIR=(.+)$/m.exec(run.stdout)?.[1];
  assert(runDir);
  assert.equal(fs.readFileSync(path.join(runDir, "outputs", "answer.txt"), "utf8"), "x = 2\n");
  const verified = invoke(core.goblinArgs("verify", runDir));
  assert.equal(verified.status, 0);
  assert.match(verified.stdout, /VERIFICATION_STATUS=PASS/);

  const branched = path.join(root, "branching.gbl");
  fs.writeFileSync(branched, 'x = 2\nif x > 1 { label = "high" } else { label = "low" }\nswitch label { case "high" { code = 1 } default { code = 0 } }\nprint("code = {code}")\n');
  const branchCheck = invoke(core.goblinArgs("check", branched, ["--json"]));
  assert.equal(branchCheck.status, 0);
  const branchRun = invoke(core.goblinArgs("run", branched));
  assert.equal(branchRun.status, 0);
  assert.match(branchRun.stdout, /code = 1/);

  const strings = path.join(root, "strings.gbl");
  fs.writeFileSync(strings, 'name = str_trim("  Ada  ")\nvalue = parse_number("2.5") * 2\nprint("Hello, " + name + "; value = " + to_text(value))\n');
  const stringCheck = invoke(core.goblinArgs("check", strings, ["--json"]));
  assert.equal(stringCheck.status, 0);
  const stringRun = invoke(core.goblinArgs("run", strings, ["--compile"]));
  assert.equal(stringRun.status, 0, stringRun.stderr);
  assert.match(stringRun.stdout, /Hello, Ada; value = 5/);

  const bad = path.join(root, "bad.gbl");
  fs.writeFileSync(bad, "for i in range(3) {\n x = i\n");
  const rejected = invoke(core.goblinArgs("check", bad, ["--json"]));
  assert.equal(rejected.status, 2);
  const diagnostic = core.parseCheckResponse(rejected.stdout, rejected.stderr, rejected.status);
  assert.equal(diagnostic.status, "FAIL");
  assert.equal(diagnostic.diagnostics[0].code, "G002");
});

"use strict";

const assert = require("node:assert/strict");
const childProcess = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const test = require("node:test");
const core = require("../editor-core");

test("actual alpha.28 CLI check, run, and verify contracts", {
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
  assert.match(version.stdout, /0\.1\.0-alpha\.28/);

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

  const everyday12 = path.join(root, "everyday-alpha12.gbl");
  fs.writeFileSync(everyday12, 'values = [1, 2, 3, 4]\nsum = 0\nfor value in values {\n if value % 2 != 0 { continue }\n sum = sum + value\n}\nok = sum == parse_integer("6") and not false\nprint("sum = {sum}; ok = {ok}")\n');
  const everyday12Run = invoke(core.goblinArgs("run", everyday12, ["--compile"]));
  assert.equal(everyday12Run.status, 0, everyday12Run.stderr);
  assert.match(everyday12Run.stdout, /sum = 6; ok = true/);

  const chemistry = path.join(root, "chemistry.gbl");
  fs.writeFileSync(chemistry, 'mm = chem_molar_mass("H2O")\namount = chem_moles(36.03 g, mm)\nvolume = 25 µL\nprint("amount = {amount}; volume = {volume}")\nseal amount\n');
  const chemistryRun = invoke(core.goblinArgs("run", chemistry, ["--compile"]));
  assert.equal(chemistryRun.status, 0, chemistryRun.stderr);
  assert.match(chemistryRun.stdout, /amount = 2 mol/);

  const electrical = path.join(root, "electrical.gbl");
  fs.writeFileSync(electrical, 'current = ee_current(12 V, 4.7 kΩ)\nmilliamps = ee_in_unit(current, "mA")\nprint("current = {milliamps:.3f} mA")\nseal current\n');
  const electricalRun = invoke(core.goblinArgs("run", electrical, ["--compile"]));
  assert.equal(electricalRun.status, 0, electricalRun.stderr);
  assert.match(electricalRun.stdout, /current = 2\.553 mA/);

  const statistics = path.join(root, "statistics.gbl");
  fs.writeFileSync(statistics, 'GO_PARANOID\nvalues = [10 m, 11 m, 12 m]\ntotal = sum(values)\naverage = mean(values)\nprint("total = {total}; mean = {average}")\nseal total\nseal average\n');
  for (const extra of [[], ["--compile"]]) {
    const statisticsRun = invoke(core.goblinArgs("run", statistics, extra));
    assert.equal(statisticsRun.status, 0, statisticsRun.stderr);
    assert.match(statisticsRun.stdout, /total = 33 m; mean = 11 m/);
    const statisticsDir = /^RUN_DIR=(.+)$/m.exec(statisticsRun.stdout)?.[1];
    assert(statisticsDir);
    assert.equal(invoke(core.goblinArgs("verify", statisticsDir)).status, 0);
  }
  const units = path.join(root, "units.gbl");
  fs.copyFileSync(path.join(__dirname, "..", "..", "examples", "compound_units.gbl"), units);
  for (const extra of [[], ["--compile"]]) {
    const executed = invoke(core.goblinArgs("run", units, extra));
    assert.equal(executed.status, 0, executed.stderr);
    assert.match(executed.stdout, /area = 3 m\^2; whole square = 9 m\^2/);
    const directory = /^RUN_DIR=(.+)$/m.exec(executed.stdout)?.[1];
    assert(directory);
    assert.equal(invoke(core.goblinArgs("verify", directory)).status, 0);
  }
  const distributions = path.join(root, "distributions.gbl");
  const random = path.join(root, "seeded_random.gbl");
  fs.copyFileSync(path.join(__dirname, "..", "..", "examples", "seeded_random.gbl"), random);
  const preview = invoke(core.goblinArgs("check", random, ["--json"]));
  assert.equal(preview.status, 0, preview.stderr);
  assert.equal(JSON.parse(preview.stdout).rng_preview.usage, "USED");
  let evidence;
  for (const extra of [[], ["--compile"]]) {
    const executed = invoke(core.goblinArgs("run", random, extra));
    assert.equal(executed.status, 0, executed.stderr);
    const directory = /^RUN_DIR=(.+)$/m.exec(executed.stdout)?.[1];
    assert(directory);
    const receipt = JSON.parse(fs.readFileSync(path.join(directory, "receipt.json"), "utf8"));
    assert.equal(receipt.rng_policy.id, "goblin.pcg32-xsh-rr-setseq.v1");
    if (evidence) assert.deepEqual(receipt.rng, evidence);
    evidence = receipt.rng;
    assert.equal(invoke(core.goblinArgs("verify", directory)).status, 0);
  }
  fs.copyFileSync(path.join(__dirname, "..", "..", "examples", "statistics_distribution.gbl"), distributions);
  assert.equal(invoke(core.goblinArgs("check", distributions, ["--json"])).status, 0);
  for (const extra of [[], ["--compile"]]) {
    const executed = invoke(core.goblinArgs("run", distributions, extra));
    assert.equal(executed.status, 0, executed.stderr);
    assert.match(executed.stdout, /median = 4\.5 m; Q1 = 4 m; Q3 = 5\.5 m/);
    assert.match(executed.stdout, /fraction at or below 5 m = 0\.750/);
    const directory = /^RUN_DIR=(.+)$/m.exec(executed.stdout)?.[1];
    assert(directory);
    assert.equal(invoke(core.goblinArgs("verify", directory)).status, 0);
  }

  fs.copyFileSync(path.join(__dirname, "..", "..", "examples", "sample.fits"), path.join(root, "sample.fits"));
  const selection = path.join(root, "selection.gbl");
  fs.writeFileSync(selection, 'stats = fits_select_stats("sample.fits", 1, "Z", 0, 3, "Z", "QUALITY")\nmean = stats[3]\nprint("weighted mean = {mean}")\nseal stats\n');
  const selectionCheck = invoke(core.goblinArgs("check", selection, ["--json"]));
  assert.equal(selectionCheck.status, 0, selectionCheck.stderr);
  const selectionRun = invoke(core.goblinArgs("run", selection));
  assert.equal(selectionRun.status, 0, selectionRun.stderr);
  assert.match(selectionRun.stdout, /weighted mean = 1\.6125/);
  const selectionRunDir = /^RUN_DIR=(.+)$/m.exec(selectionRun.stdout)?.[1];
  assert(selectionRunDir);
  const selectionVerify = invoke(core.goblinArgs("verify", selectionRunDir));
  assert.equal(selectionVerify.status, 0);
  assert.match(selectionVerify.stdout, /VERIFICATION_STATUS=PASS/);

  const subset = path.join(root, "subset.gbl");
  fs.writeFileSync(subset, 'GO_PARANOID\nlow = fits_where("Z", ">=", 0)\nhigh = fits_where("Z", "<", 3)\ncut = fits_all([low, high])\ncount = fits_export_csv("subset.csv", "sample.fits", 1, ["OBJECT", "Z"], cut)\nprint("selected = {count}")\nseal count\n');
  assert.equal(invoke(core.goblinArgs("check", subset, ["--json"])).status, 0);
  for (const extra of [[], ["--compile"]]) {
    const executed = invoke(core.goblinArgs("run", subset, extra));
    assert.equal(executed.status, 0, executed.stderr);
    assert.match(executed.stdout, /selected = 2/);
    const dir = /^RUN_DIR=(.+)$/m.exec(executed.stdout)?.[1];
    assert(dir);
    assert.equal(fs.readFileSync(path.join(dir, "outputs/subset.csv"), "utf8"), "OBJECT,Z\nGALAXY,0.125\nQSO,2.25\n");
    assert.equal(invoke(core.goblinArgs("verify", dir)).status, 0);
  }

  const library = path.join(root, "library.gbl");
  fs.writeFileSync(library, 'g_func measured_mean(numbers) { return mean(numbers) * 1 m }\n');
  fs.writeFileSync(path.join(root, "data.csv"), 'sample,length\nA,10\nB,11\nC,12\n');
  const tables = path.join(root, "tables.gbl");
  fs.writeFileSync(tables, 'GO_PARANOID\nimport "library.gbl"\nvalues = csv_numbers("data.csv", "length")\naverage = measured_mean(values)\nwrite_text("table.txt", "average = {average}")\nprint("average = {average}")\nseal average\n');
  assert.equal(invoke(core.goblinArgs("check", tables, ["--json"])).status, 0);
  for (const extra of [[], ["--compile"]]) {
    const executed = invoke(core.goblinArgs("run", tables, extra));
    assert.equal(executed.status, 0, executed.stderr);
    assert.match(executed.stdout, /average = 11 m/);
    const directory = /^RUN_DIR=(.+)$/m.exec(executed.stdout)?.[1];
    assert.equal(invoke(core.goblinArgs("verify", directory)).status, 0);
  }
  const compiledSelection = invoke(core.goblinArgs("run", selection, ["--compile"]));
  assert.equal(compiledSelection.status, 0, compiledSelection.stderr);
  assert.match(compiledSelection.stdout, /weighted mean = 1\.6125/);
  const compiledSelectionDir = /^RUN_DIR=(.+)$/m.exec(compiledSelection.stdout)?.[1];
  assert.equal(invoke(core.goblinArgs("verify", compiledSelectionDir)).status, 0);

  const bad = path.join(root, "bad.gbl");
  const inference = path.join(root, "inference.gbl");
  fs.writeFileSync(inference, fs.readFileSync(path.join(__dirname, "..", "..", "examples", "inference.gbl")));
  const inferencePreview = invoke(core.goblinArgs("check", inference, ["--json"]));
  assert.equal(inferencePreview.status, 0, inferencePreview.stderr);
  assert.equal(JSON.parse(inferencePreview.stdout).resources_preview.effective_loop_budget, 50000000);
  for (const extra of [[], ["--compile"]]) {
    const result = invoke(core.goblinArgs("run", inference, extra));
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /row-major product = \[58, 64, 139, 154\]/);
    const dir = /^RUN_DIR=(.+)$/m.exec(result.stdout)?.[1];
    assert.equal(invoke(core.goblinArgs("verify", dir)).status, 0);
  }
  const scan = path.join(root, "scan.gbl");
  fs.writeFileSync(scan, 'summary = csv_scan_stats("data.csv", "length")\nprint(summary)\nseal summary\n');
  for (const extra of [[], ["--compile"]]) {
    const result = invoke(core.goblinArgs("run", scan, extra));
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /\[3, 33, 11, 10, 12\]/);
    const dir = /^RUN_DIR=(.+)$/m.exec(result.stdout)?.[1];
    assert.equal(invoke(core.goblinArgs("verify", dir)).status, 0);
  }
  fs.writeFileSync(bad, "for i in range(3) {\n x = i\n");
  const batchSource = path.join(root, "batch_catalogue.gbl");
  fs.writeFileSync(batchSource, fs.readFileSync(path.join(__dirname, "..", "..", "examples", "batch_catalogue.gbl")));
  fs.writeFileSync(path.join(root, "batch_catalogue.csv"), fs.readFileSync(path.join(__dirname, "..", "..", "examples", "batch_catalogue.csv")));
  const batchPreview = invoke(core.goblinArgs("check", batchSource, ["--json"]));
  assert.equal(batchPreview.status, 0, batchPreview.stderr);
  assert.equal(JSON.parse(batchPreview.stdout).generated_output_preview[0].metadata.complete, true);
  for (const extra of [[], ["--compile"]]) {
    const result = invoke(core.goblinArgs("run", batchSource, extra));
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /rows = 4; total measurement = 11/);
    const dir = /^RUN_DIR=(.+)$/m.exec(result.stdout)?.[1];
    assert.equal(invoke(core.goblinArgs("verify", dir)).status, 0);
  }
  const rejected = invoke(core.goblinArgs("check", bad, ["--json"]));
  assert.equal(rejected.status, 2);
  const diagnostic = core.parseCheckResponse(rejected.stdout, rejected.stderr, rejected.status);
  assert.equal(diagnostic.status, "FAIL");
  assert.equal(diagnostic.diagnostics[0].code, "G002");
});

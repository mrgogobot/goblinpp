# Write your first Goblin++ programs

This short tutorial uses the Rust engine 0.1.0-alpha.12. Create a new `.gbl` file in a trusted project folder. The VS Code extension helps you type it; Goblin++ decides what it means.

## 1. Begin with an everyday calculation

Write `acceptance_ratio.gbl`:

```goblin
samples = 12
accepted = 9
fraction = accepted / samples

print("accepted fraction = {fraction:.3f}")
write_text("answer.txt", "fraction = {fraction:.3f}")
```

Choose **Goblin++: Run Current File**. You should see `accepted fraction = 0.750` and a `RUN_STATUS=PASS`. Goblin++ puts `answer.txt` under the reported `RUN_DIR/outputs`, not beside your source. Neither `GO_PARANOID` nor `seal` is required. The run still has a receipt and hashes.

What question does this answer? “What fraction of the 12 samples was accepted?” The result is dimensionless. A run can be technically valid while answering the wrong scientific question, so write down the question and assumptions before a serious analysis.

## 2. Use a loop

```goblin
total = 0
for n in range(1, 6) {
    total = total + n
}
print("total = {total}")
```

`range(1, 6)` visits 1 through 5: its stop is excluded. `while condition { ... }` also works when the condition is Boolean. The interpreter and compiler share a one-million-loop-body-iteration limit; it catches a runaway loop but is not a security sandbox.

You can also name a reusable calculation. Type `g_func` in the editor for a starter snippet:

```goblin
g_func fraction_of(accepted, samples) {
    return accepted / samples
}

fraction = fraction_of(9, 12)
print("fraction = {fraction:.3f}")
```

Arguments are local copies. A function must explicitly `return` on the path taken; place `seal fraction` in the caller if you want a separate typed artifact. See the engine's `docs/FUNCTIONS.md` for limits and compiled-mode boundaries.

Text from `input()` or `argv()` is not automatically numeric. Use `parse_number(raw)` for a finite unitless decimal or `parse_integer(raw)` for checked decimal integer syntax; invalid text produces an auditable failure. Text `+`, `len(text)`, and the `str_...` built-ins cover everyday string work. See the engine's `docs/STRINGS.md` before using text conversions in scientific calculations.

## 3. Make a decision

```goblin
fraction = 9 / 12
if fraction >= 0.75 {
    verdict = "meets threshold"
} else {
    verdict = "below threshold"
}
switch verdict {
    case "meets threshold" { code = 1 }
    default { code = 0 }
}
print("verdict = {verdict}; code = {code}")
```

`if` needs a Boolean condition. `switch` runs the first exact matching case, without fallthrough. For approximate floating-point decisions, use a tolerance comparison in `if`, rather than `switch` equality. Your threshold is a scientific choice: document why it is appropriate.

## 4. Read a scientific file

Place the bundled `sample.fits` beside your `.gbl` file. Its second HDU is a small binary table:

```goblin
file = "sample.fits"
hdus = fits_hdu_count(file)
rows = fits_rows(file, 1)
mean_z = fits_column_mean(file, 1, "Z")

print("HDUs = {hdus}")
print("rows = {rows}")
print("mean Z = {mean_z}")
write_json("summary.json", "rows", rows, "mean_z", mean_z)
```

HDU indexes start at zero. Goblin++ hashes the input in a run; the file is read-only. `Z` is a column name, not a guaranteed cosmological interpretation. For an unfamiliar file, inspect its HDUs and columns first with `goblin++ fits-info FILE --quick`; quick inspection is explicitly **unhashed**, not run evidence.

For a full-table, explicitly bounded and optionally weighted summary, see
`examples/fits_selection.gbl` and the engine's `docs/FITS.md`. The bundled
three-row fixture teaches syntax; it is not a DESI science result. The editor's
`fitsselect` snippet is only a starting point: you must choose justified
columns, interval, and weights for your own catalogue.

## 5. Decide what to preserve

Add `seal mean_z` only if you want `mean_z` preserved as its own typed scientific artifact. Printing and writing files do not require it. Add `GO_PARANOID` at the top if you want the extra postflight source observation and self-verification gate. Neither choice makes a scientific assumption true or executes untrusted native code safely.

Use **Goblin++: Verify Run Directory** on the `RUN_DIR` reported by a run. Freeze exact source bytes only after reviewing your program and assumptions. To change frozen source later, create a child revision with an explicit reason.

## Current boundaries

FITS and generated-output calls work in the interpreter, not yet in compiled runs (including inside `g_func`). Inline Rust requires a separately reviewed exact hash and has your account's full OS authority. One-dimensional arrays and `g_func` functions are available; nested collections remain future work. No editor color or completion is a substitute for checking the data and testing the claim. Data first; goblins last.

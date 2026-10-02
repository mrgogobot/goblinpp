# Protected values and immediate text capture (alpha.20)

This correctness update addresses the first three WB-1 backlog items, in both
execution engines. It does not claim the remaining scientific additions are done.

## Constant names are read-only

`h` always means Planck's constant; `c` always means the speed of light.
All registered aliases are protected. `h = 2` fails with **h is a registered
constant**, before evaluating the right-hand side. Choose descriptive variables
such as `height` or `concentration`. Nested assignments, function names,
parameters and loop variables are covered. `print(h)`, `print("{h}")` and
`seal h` use the same registry value and record its use in run evidence.

## Consume value-only builtin results

Arrays and slices still have independent-copy semantics:

```goblin
samples = []
samples = append(samples, 1)
print(samples)
```

`append(samples, 1)` alone is refused, not silently ignored. Direct statement
calls to other registered value-only builtins, such as `sqrt(9)` or
`str_trim(" text ")`, are also refused. Assign, return or pass their result
to a consumer. `print`, `printf`, writers, plots, `input` and user-defined
functions may still be statements because they can have intentional effects.
This is not a general unused-value checker for arbitrary expressions or user
functions. Execution validation checks nested bodies before side effects.

## Source templates capture immediately, once

```goblin
x = 1
saved = "{x}"
x = 2
print(saved)                    # 1
write_text("saved.txt", saved) # 1, not 2
```

The rule applies when *any source string expression* is evaluated, not only
assignment: array items, function arguments and returned strings behave the
same way. In a loop, a newly constructed template captures that iteration's
value. Function templates see local parameters/variables. `{argc}` and constant
aliases (including `π` and `ħ`) also work. Numeric formats remain `.Nf` and
`.Ne`; dimensions are preserved.

An unknown `{missing}` fails immediately. Malformed identifier placeholders
and unclosed placeholders fail rather than becoming deferred templates.
Use `"{{x}}"` to obtain literal `{x}`; doubled opening/closing braces paired
this way escape a placeholder. Non-placeholder JSON/code braces remain literal,
including nested JSON closing braces. A bare opening or closing brace is text.
Text expansion is limited to 1,048,576 UTF-8 bytes.

Stored text is never scanned again by `print`, prompts or file exports. Text
returned by `input`, `argv`, CSV/TSV/FITS readers or string operations remains
ordinary text, even when it contains `{x}`. Inserted placeholder text is not
recursively expanded. Escape a source-literal search pattern such as
`str_replace(text, "{{x}}", "replacement")` if you mean literal braces.

## Existing frozen projects

The execution rules changed even though source syntax/canonical hashes did not.
New run and freeze receipts hash the policy
`goblin.eager-text-and-protected-values.v1` in `language_semantics` for alpha.20.
Alpha.21 supersedes this with `goblin.eager-text-and-roundtrip-numbers.v2`
to record lossless default numeric formatting; see [numeric text](NUMERIC_TEXT.md).
Alpha.19 and earlier run evidence remains independently verifiable; verifying
it checks historical integrity, not whether it would produce the same result now.

An existing freeze without this policy (or with another policy) is refused as
`LANGUAGE_SEMANTICS_CHANGED_AFTER_FREEZE`. Its integrity still verifies, and it
can parent an explicit revision. Do not delete or rewrite that freeze:

```sh
goblin++ revise experiment.gbl experiment_R1.gbl --reason "adopt alpha.20 immediate text capture"
# Review/edit the child; rename variables that collide with constants.
goblin++ freeze experiment_R1.gbl
goblin++ experiment_R1.gbl
```

Use the old engine if you need old execution semantics. Comparisons across
policy epochs classify `LANGUAGE_SEMANTICS_CHANGE`, not identical or notation-only
results. Nothing here authenticates authorship or supplies a sandbox; custody
and input-privacy limitations are unchanged.

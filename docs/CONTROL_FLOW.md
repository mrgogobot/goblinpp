# Control flow in Goblin++ 0.1.0-alpha.6

Goblin++ supports `for`, `while`, `if`/`else if`/`else`, and `switch`/`case`/`default` in interpreter and compiled mode. The examples here are executable `.gbl`, not Python or inline Rust.

```goblin
GO_PARANOID

sum = 0
for i in range(1, 6) {
    sum = sum + i
}
print("sum = {sum}")
seal sum
```

Braces delimit the body; each statement still ends at a newline. Nested loops are allowed. The loop variable is assigned at the start of each iteration and remains visible afterward if the loop executed. A zero-iteration loop does not create the variable. Changing the loop variable inside the body does not change the next `range` value.

`range(stop)` starts at zero. `range(start, stop)` uses a step of one. `range(start, stop, step)` allows a nonzero positive or negative step. The stop is exclusive. Arguments are evaluated once when the loop starts and must be dimensionless, exactly representable integers in the range ±(2^53−1). A zero step and unit-bearing or fractional bounds fail explicitly.

```goblin
remaining = 3
while remaining > 0 {
    print("remaining = {remaining}")
    remaining = remaining - 1
}
finished = remaining == 0
seal finished
```

Comparisons `==`, `!=`, `<`, `<=`, `>`, and `>=` produce `true` or `false`. Numeric comparisons require matching dimensions. Text and Boolean values support equality and inequality, not ordering. A `while` condition must be Boolean; numeric truthiness is deliberately not inferred.

```goblin
score = 9
if score < 5 {
    label = "low"
} else if score >= 9 {
    label = "high"
} else {
    label = "middle"
}

switch label {
    case "low" { code = 1 }
    case "high" { code = 2 }
    default { code = 3 }
}
print("label = {label}; code = {code}")
```

`if` and every reached `else if` require a Boolean condition; only the selected body executes. `switch` evaluates its selector once, then case labels in source order until the first exact `==` match. There is no fallthrough. `default` is optional, unique, and last; at least one `case` is required. A case label may be an expression. Matching quantities must have the same dimensions; incomparable kinds or dimensions fail explicitly, rather than silently skipping a case. Floating-point equality is exact, so use a tolerance comparison in `if` for approximate scientific results. Branch assignments remain visible afterward, just like loop assignments; an unexecuted branch creates no variable.

All loop-body executions, including nested loops, share a limit of 1,000,000 per run. The next iteration is refused with `G203 LOOP LIMIT EXCEEDED`; the failed run remains auditable. The limit is a safeguard against accidental infinite loops, not a full resource sandbox. Expensive work inside one iteration, and authorized inline Rust outside loops, still require judgment.

`GO_PARANOID` and inline Rust blocks remain top-level constructs. Source freezing hashes the exact source bytes and canonical AST. Sealed values, including Booleans, are checked for interpreter/native parity in compiled runs. Existing pre-branch source hashes remain unchanged. Interpreted branches may call FITS and output functions; native compilation still refuses those calls anywhere in the program, including unreachable branches.

Current boundaries: there is no `break`, `continue`, list or table iteration, or `and`/`or`/`not` yet. User-defined `g_func` functions are documented in [FUNCTIONS.md](FUNCTIONS.md). `for` currently iterates integer ranges, not FITS rows directly. Large scientific catalogue analyses should continue to use the built-in streaming FITS aggregates until explicit row filtering and weighted-statistics semantics are added; a loop alone does not create a defensible survey-selection correction.

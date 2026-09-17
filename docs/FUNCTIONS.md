# User-defined functions (`g_func`)

`g_func` defines a reusable calculation in ordinary Goblin++ code. The declaration is top-level, but calls may appear in expressions and in other functions. Declarations may follow their first use.

```goblin
g_func mean_of_two(a, b) {
    total = a + b
    return total / 2
}

value = mean_of_two(2 kg, 4 kg)
print("mean = {value}")
seal value
```

`return expression` ends the function immediately, including from a nested `if`, `switch`, `for`, or `while`. There is no implicit return: reaching the end produces a preserved `G203` failure. Calls require exactly the declared number of arguments, and duplicate or reserved function/parameter names are rejected. At most 32 parameters and 16 active calls are allowed; the latter bounds accidental recursion. The existing shared loop limit still applies inside functions.

Arguments are evaluated in the caller and copied into a fresh local variable environment. Local assignments do not change the caller's variables, and arrays do not share backing storage. A function cannot implicitly read caller variables; pass them as arguments instead. Registered constants, `argc`/`argv`, and built-ins remain available. `print` and `printf` inside a function append to the run's normal hashed output.

`GO_PARANOID`, `seal`, nested `g_func` declarations, and inline Rust blocks are not allowed inside functions. A caller can seal a returned value. The interpreter permits FITS and generated-output built-ins in functions, with the usual import/output evidence. The native compiler refuses those built-ins anywhere in a program, even in an uncalled function, until their compiled semantics are implemented. Source, canonical program, outputs and sealed values retain the existing freeze/run/verify rules.

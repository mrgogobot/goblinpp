# Input, arguments, arrays, and slices

`input("prompt")` reads one UTF-8 text line and returns text. `argc` is the read-only count of program arguments, including `argv(0)`; `argv(i)` returns a zero-based text argument. For Goblin++ runs, `argv(0)` is the `.gbl` path. Put program arguments after `--`:

```console
goblin++ examples/greeting.gbl
goblin++ examples/program_args.gbl -- Ada
goblin++ run examples/program_args.gbl -- Ada
```

`input` and `argv` return text, not numbers. Use `parse_number(text)` for an explicit, checked conversion to a finite dimensionless quantity; see [STRINGS.md](STRINGS.md). Each prompt and response is capped at 65,536 bytes; a run allows at most 1,024 prompts and 256 arguments (including `argv(0)`). `check` never reads standard input; if it reaches `input`, it reports that input is required. The VS Code Run command shows an input box. Run with Arguments asks for a JSON array, for example `["--name", "Ada"]`.

**Privacy:** prompt text, responses, and arguments are preserved unredacted in `interaction.json` inside the run directory and hash-linked to the receipt. Do not use `input` or command-line arguments for passwords, tokens, or other secrets. `verify` checks the preserved evidence; `diff` reports whether interaction evidence matches. This is auditability, not confidentiality.

Arrays are one-dimensional and homogeneous: elements must all be text, all Boolean, or all quantities of the same dimension. Empty `[]` adopts a type on its first `append`. Nested arrays are not supported. The maximum is 100,000 items.

```goblin
values = [1 kg, 2 kg, 3 kg, 4 kg]
print(values[0])       # 1 kg
part = values[1:3]     # [2 kg, 3 kg]; stop is exclusive
part[0] = 20 kg        # values[1] stays 2 kg
extended = append(values, 5 kg)  # returns a new array
count = len(values)    # 4
for i in range(len(values)) {
    print(values[i])
}
seal values
```

Indexes are zero-based, exact, non-negative dimensionless integers. Slices allow omitted bounds (`values[:]`, `values[:2]`, `values[2:]`), and `start:stop` is half-open. Out-of-range and reversed bounds fail explicitly. Assignment, slicing, and `append` use **independent copies**: there is no Go-style shared backing array. This choice prevents a derived scientific subset from silently changing its source. `seal` snapshots an array as a typed, hashed artifact. Interpreter and compiled execution are tested for the same array results.

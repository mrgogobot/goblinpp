# Your first Goblin++ AI resource check

**Alpha.46 • about five minutes • no Python required**

This first stage answers: **what can this computer report, and does an explicit
training-memory estimate fit its budget?** It does not download or train a model.
You can use the functions offline from the command line or your usual IDE.

## 1. Install the new runtime

Open a terminal in the extracted **macOS arm64 runtime bundle**. Run each command
in order:

```sh
./install.sh --prefix "$HOME/.local"
rehash
goblin++ --version
```

The last line should say `goblin++ 0.1.0-alpha.46`. If `rehash` is unavailable in
your shell, open a new terminal. An older version means your terminal or IDE is
still selecting an older executable.

## 2. Look at your computer

```sh
goblin++ ai hardware
```

The report shows CPU threads, total and currently available RAM, free space on
the current filesystem, and accessible GPU observations. Memory figures are
bytes: 1 GiB is 1,073,741,824 bytes. This command does not allocate a model.

`UNKNOWN` means Goblin++ could not establish that value. A zero device count can
mean no GPU was accessible to this process; it does not prove the computer has
no GPU. Detection alone never proves that a training backend works.

## 3. Run the short example

Stay in the runtime bundle's main folder:

```sh
goblin++ run examples/ai_quickstart.gbl
```

The source is deliberately small:

```goblin
GO_PRIVATE

plan = ai_plan("ai_resources.json")
decision = ai_plan_status(plan)
print("Resource plan = {decision}")
print("No training was started.")
```

What happens:

1. `GO_PRIVATE` keeps this local inspection outside automatic scientific receipts.
2. `ai_plan` reads the supplied request **beside the source file** and observes
   current resources. Keep `ai_resources.json` in the same folder as this example.
3. `ai_plan_status` checks the plan's arithmetic and returns its resource decision.
4. `print` shows the result. No model is loaded, downloaded or trained.

The result is one of:

| Decision | What to do |
|---|---|
| `WITHIN_ESTIMATED_BUDGET` | The declared estimate fits the observed budget. This is not permission or a guarantee to train. |
| `REFUSED` | Read the detailed plan's reasons; reduce the declared workload or investigate the limiting resource. |
| `INSUFFICIENT_INFORMATION` | A required capacity is unknown. Investigate it instead of treating it as spare memory. |

`RUN_STATUS=PASS` only means this inspection script completed. Read the separate
resource decision; a successfully reported refusal is still a refusal.

For the detailed JSON plan, run:

```sh
goblin++ run examples/ai_resources.gbl
```

## 4. Try the same source in an IDE

Install the Goblin++ extension **0.1.37 for VS Code** or plugin **0.3.27 for
JetBrains** for current help/completion, or use
Goblin++ IDE 0.2.4 with its bundled engine. Open the supplied `Examples`/`examples`
folder as a project, open `ai_quickstart.gbl`, and press the usual Run/Play button.
Both `.gbl` and `.json` files must be present together in a writable project.

If you copied only the source into a new project, also copy `ai_resources.json`.
Do not paste JSON into the `.gbl` file. A plugin update alone does not install
the runtime; check the executable selected in the IDE's settings.

For compiled execution, after the matching Rust 1.92.0 toolchain and locked
dependencies have been prepared:

```sh
goblin++ run examples/ai_quickstart.gbl --compile
```

The interpreter needs neither Python nor a Rust compiler in the bundled Mac
installation. Native compilation uses Cargo offline; it does not fetch missing
dependencies during execution. Prefer the normal Run action for this first try.

## 5. Save and verify a plan

To preserve the original request, hardware snapshot, plan and checksum receipt:

```sh
goblin++ ai plan examples/ai_resources.json --output my-first-ai-plan
goblin++ ai verify-plan my-first-ai-plan
```

Choose a **new directory name** each time; existing files are never overwritten.
Verification checks the saved bytes and resource arithmetic. It does not reserve
memory, test a model or certify current hardware availability.

For stronger custody, keep the printed `PLANNING_RECEIPT_CORE_SHA256` elsewhere
and use `--expected-receipt-sha256` when verifying. A checksum alone is not a
publisher signature or proof that the original estimates were correct.

## A small, safe edit

Open `examples/ai_resources.json`. These fields are easy to recognize:

```json
"micro_batch": 1,
"sequence_length": 256,
"gradient_accumulation": 8,
"max_threads": 2
```

They describe one sequence per micro-batch, 256 tokens per sequence, accumulation
over eight micro-batches and a requested two-thread ceiling. Effective batch is
`1 * 8 = 8`. These are **planning inputs**; no training loop runs in alpha.46.

Try changing `sequence_length` to `512`, save, and run the detailed example again.
The activation estimate increases; Goblin++ does not silently lower your context
or change the model to make it fit. Attention workspace is a separate declared
estimate: for a real model it may also grow with context and must be calibrated.

The supplied 100-million-parameter configuration is illustrative, not a measured
model profile. Never treat its activation/workspace numbers as universal defaults.
Restore the file or keep your experiment in a separate copy when finished.

## Where we go next

Next comes a small, measured **native CPU training backend**, followed by a
separately validated optional Mac MLX adapter. We will measure memory, time and
training behavior before making speed or compatibility claims. Python is not a
dependency of the current hardware/planning functions, and no hidden Python
fallback is used.

For formulas, limits and every request field, see [the full AI resource reference](AI_RESOURCES.md).

Optional: [Goblin++ merchandise](https://h4k3rl1f3.myspreadshop.co.uk).

# Train your first Goblin++ model offline

Use a Goblin++ build that lists `ai train` in `goblin++ ai --help`. From this
source tree you can build it with `cargo build --locked --offline` once the
locked Rust dependencies are cached. In the commands below, use
`./target/debug/goblinpp` in place of `goblin++` if you built from source.
Training itself needs only the Goblin++ executable and a local CSV file.

## 1. Look at the example

Open [examples/ai_linear.csv](../examples/ai_linear.csv). Each row pairs one
input `x` with its target `y`. All five rows follow `y = 2*x + 3`, so you can
check the learned answer yourself.

## 2. Train the model

From the repository or bundle root, run this in your terminal or IDE terminal.

```sh
goblin++ ai train examples/ai_linear.csv \
  --output my-linear-model.json \
  --steps 300 --max-steps 300 --learning-rate 0.1 \
  --max-memory-bytes 1024 --max-seconds 10
```

The model starts with both coefficients at zero. Each full-batch step uses
all five rows in file order to update the slope and intercept. Expect
`TRAINING_STATUS=PASS`, five rows, 300 steps, and an initial mean squared error
of 11. The final mean squared error should be below `1e-12`.

## 3. Inspect and check the result

```sh
cat my-linear-model.json
shasum -a 256 my-linear-model.json
```

Look under `model` for a slope close to 2 and an intercept close to 3. For a
new input of `x = 2`, the formula `slope * 2 + intercept` gives about 7.
The reported error is measured on the five training rows. It tells you how
closely the model fits those rows.

The JSON also records the input SHA-256, algorithm, target OS and architecture,
configuration, completed steps and peak data-buffer bytes. The checksum from
`shasum` should match the printed `MODEL_SHA256`. Keep that checksum separately
if you want to check the file again later. The algorithm uses binary64, fixed
row order, zero initialization and zero randomness. Compare coefficients with
a tolerance when checking results across builds or platforms.

## 4. Try a refusal

```sh
goblin++ ai train examples/ai_linear.csv \
  --output over-budget.json --steps 300 --max-steps 10
```

Expect an error about the step budget and no new `over-budget.json`.
Use a fresh output filename for each successful run. An existing model is
preserved.

## 5. Try your own data

Use the exact header `x,y`, followed by at least two rows of finite numbers.
This reader supports plain numeric UTF-8 CSV and accepts both LF and CRLF.
Start with feature values on a small scale, such as −1 to 1. If fitting becomes
unstable, rescale your data or lower the learning rate. Inspect the initial and
final error as well as the coefficients.

| Limit | What Goblin++ enforces |
|---|---|
| CPU | One training thread |
| Data memory | The combined CSV byte buffer and parsed rows must fit `--max-memory-bytes`, up to 8 MiB. Each parsed row uses 16 bytes. |
| Input | 2 to 100,000 rows, with up to 256 bytes of text per data row |
| Updates | `--steps` must fit `--max-steps`, with a ceiling of 100,000 updates and 10 million total row evaluations |
| Time | The trainer checks `--max-seconds` during input, fitting and evaluation, and before saving. The accepted range is 1 to 300 seconds. |

The memory limit measures the trainer's two data buffers. Process overhead and
allocator overhead are outside that budget. The deadline is checked at regular
points in the trainer and does not interrupt a blocked filesystem operation.
The hardware and plan commands remain available for inspecting other workloads.
This tutorial fits a two-parameter CPU model and runs entirely offline in Rust.

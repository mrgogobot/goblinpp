# AI resources — alpha.46

New here? Start with [the five-minute tutorial](AI_QUICKSTART.md).
For actual fitting, see the separate [native CPU trainer](AI_CPU_TRAINER.md).

This is the first AI-library stage: **local hardware observations and conservative
resource planning**. It does not load, train, tune, distil or execute a model.
It uses no Python process, network request, downloaded model, new ML dependency
or GPU stress test. Rust remains 1.92.0.

## Try it

From the extracted runtime bundle, install the new version first:

```sh
./install.sh --prefix "$HOME/.local"
rehash
goblin++ --version
goblin++ ai hardware
goblin++ run examples/ai_resources.gbl
```

Use alpha.46 or later. On shells without `rehash`, open a new
terminal. The example uses a hypothetical configuration, not a supplied model;
its memory coefficients are illustrative, not measured training requirements.

For a saved, independently verifiable planning bundle:

```sh
goblin++ ai plan examples/ai_resources.json --output my-ai-plan
goblin++ ai verify-plan my-ai-plan
```

The output directory must be new. Its parent must exist. Four files are written:
`request.json` (original bytes), `hardware.json`, `plan.json`, and `receipt.json`.
On Unix the new directory is owner-only (0700), and files are owner-only (0600).
Existing files are never replaced. Interrupted writes leave an incomplete bundle;
verification refuses it. Use trusted parent directories: these operations are not
a defence against a hostile process modifying ancestor paths concurrently.

Keep the printed `PLANNING_RECEIPT_CORE_SHA256` independently if you want an
external checksum anchor:

```sh
goblin++ ai verify-plan my-ai-plan --expected-receipt-sha256 YOUR_SAVED_HASH
```

Verification checks the exact four-file inventory, hashes, byte counts, request,
hardware snapshot, versioned policy and reconstructed resource arithmetic.
It does not re-probe the machine. A valid preserved refusal verifies successfully
and remains a refusal. Checksums are not signatures: rewriting the entire snapshot
and its checksums cannot establish independent authenticity without an external
trusted anchor.

`ai hardware`, `ai plan` and `ai verify-plan` support `--json`. Hardware/planning
support `--workspace PATH` to choose the existing filesystem whose free space is
observed. Planning defaults to the request's parent directory; the output's
filesystem may differ, so select the intended training/checkpoint workspace.

## Functions in ordinary Goblin++ code and IDEs

```goblin
GO_PRIVATE
hardware = ai_hardware()
plan = ai_plan("ai_resources.json")
decision = ai_plan_status(plan)
print("resource decision = {decision}")
print(plan)
```

All three return strings. `ai_hardware()` returns a JSON snapshot.
`ai_plan(path)` reads a JSON request relative to the entry source directory
(absolute paths are also accepted) and returns a JSON plan.
`ai_plan_status(plan)` reconstructs that plan and returns its decision.
Keep returned values: dropping these calls' results is refused.

The functions require `GO_PRIVATE` because they read live machine state and local
configuration. They work in the interpreter and native compiler. They create no
scientific run receipts, source snapshots, ledger entries or automatic final-state
captures. `GO_PARANOID` and `COMPLETELY_MAD` cannot be mixed with this private
script. The explicitly requested CLI planning bundle above is a separate artifact.
Printing or saving private information explicitly can disclose it; GO_PRIVATE is
not an OS sandbox or secure erasure.

Open `examples/ai_resources.gbl` in VS Code or a JetBrains IDE and use the normal
Run action. Install the new runtime as well as the plugin: upgrading a plugin
alone does not upgrade `~/.local/bin/goblin++`. Completion/help/highlighting are
included in VS Code 0.1.37 and JetBrains 0.3.27.

## What the decisions mean

| Decision | Meaning | `ai plan` exit code |
|---|---|---:|
| `WITHIN_ESTIMATED_BUDGET` | Every required known capacity covers this declared estimate | 0 |
| `REFUSED` | A known budget is exceeded, or the explicitly requested device was not observed | 1 |
| `INSUFFICIENT_INFORMATION` | Required capacity cannot be established | 2 |

A known refusal takes precedence over unknown information. Invalid requests fail
with G910. Private-script runtime errors use the existing deliberately generic
G902 message; use the explicit CLI planner to diagnose a non-secret request.
An estimated fit does **not** certify backend compatibility, allocation success,
speed, model quality, thermal safety or absence of OOM. Other workloads can change
available memory immediately after observation. No settings are silently changed.

## Hardware observations and limits

- CPU: OS-visible usable logical parallelism and runtime instruction features.
  Missing AVX/NEON does not by itself forbid a CPU estimate or promise a fallback
  kernel. Default planned threads are `min(8, max(1, usable_threads - 1))`, further
  capped by explicit `max_threads`. CPU features are not benchmark results.
- macOS: installed RAM, conservative free-plus-inactive memory, workspace free
  disk and the default Metal device. The device's unified-memory flag and
  recommended working-set size are queried when supported. Compressed/purgeable
  memory is not assumed freely available. No MLX library is loaded or tested.
- Linux: `/proc/meminfo`, visible cgroup-v2 ancestor `memory.max` and `memory.high`
  limits/current use, process address-space limits, workspace free disk, and
  bounded DRM sysfs device/VRAM observations. Legacy, unreadable or nonstandard
  cgroup mounts leave RAM availability unknown. Hidden host constraints are not
  certified. NVIDIA-specific NVML/CUDA discovery and GPU kernel tests remain
  future work; missing VRAM information stays unknown.
- Other systems: CPU/disk where supported; unsupported memory/GPU probes stay
  unknown. No Windows training-support claim is made.

Hardware observations omit hostname, username, hardware serial numbers and
filesystem paths. Planning bundles preserve your original request, including
whatever labels or personal information you put in it. They also contain
device/resource information: share only deliberately.

RAM admission uses the smaller installed/visible finite memory-limit total and
the smallest known remaining capacity. A finite limit with unknown usage leaves
capacity unknown. Default RAM reserve is a quarter of that total, bounded to
256 MiB–2 GiB; `reserve_ram_bytes` may increase it. Budget is the minimum of
remaining-minus-reserve, total-minus-reserve, `memory_percent` of total and an
optional `max_memory_bytes`. The default percentage is 50; maximum is 80.

Dedicated GPU memory requires both total and available VRAM. Its reserve is at
least 10% of total or 128 MiB, whichever is larger, and may be increased explicitly.
For unified memory, host and device needs use **one shared pool**, constrained by
RAM and the Metal recommended working set. RAM and VRAM are never added together.
`reserve_device_bytes` further reduces that device budget; `max_device_bytes`
caps it. Device-only controls on a CPU request are refused rather than ignored.

Disk reserve is a quarter of available space, bounded to 64 MiB–1 GiB, or a larger
explicit reserve. `max_disk_bytes` may further reduce usable space. Filesystem
observations do not reserve disk or certify quotas, sparse-file behavior or space
required by future backend-specific formats.

## Request fields and estimator v1

Use `examples/ai_resources.json` as the starting point. JSON must be a regular
UTF-8 file, at most 64 KiB; final symlinks, unknown/duplicate fields, fractional
integer fields and invalid ranges are refused. All memory counts are **bytes**.
No floating-point arithmetic is used to compute these estimates; products/sums
are checked and fractional bytes round upward. Supported byte counts are at most
2^52, with additional configuration bounds enforced by the request validator.

| Fields | Required meaning |
|---|---|
| `schema`, `model`, `device` | Schema `goblin.ai-plan-request.v1`; explicit model label; `cpu` or exact device ID from hardware JSON |
| `method` | `full`, `lora` or `qlora`; a memory-accounting configuration, not an available trainer |
| `model_parameters`, `trainable_parameters` | Positive exact counts; trainable cannot exceed total |
| `weight_bits` | Resident base-weight representation: 4, 8, 16 or 32 |
| `micro_batch`, `sequence_length`, `gradient_accumulation` | Explicit workload; effective batch is micro-batch times accumulation |
| `activation_bytes_per_token` | Positive model/configuration-specific live activation estimate, multiplied by context and micro-batch |
| `attention_workspace_bytes_per_sequence` | Additional per-sequence workspace, multiplied by micro-batch; include context-dependent attention cost here |
| `runtime_workspace_bytes`, `host_buffer_bytes` | Positive backend workspace estimate and host buffers; include loaders/caches/staging explicitly |
| `max_steps`, `max_seconds` | Required positive future execution budgets; at most 1 billion steps and 7 days |
| `max_threads`, `max_memory_bytes`, `max_device_bytes`, `max_disk_bytes` | Optional stricter caps; never increase observed capacity |
| `reserve_ram_bytes`, `reserve_device_bytes`, `reserve_disk_bytes` | Optional larger reserves, default 0 |
| `memory_percent`, `headroom_percent` | Defaults 50 and 25; supported ranges 1–80 and 20–100 |
| `quantization_overhead_bytes` | Required positive estimate for QLoRA, otherwise zero; includes scales/metadata/dequantization buffers |
| `additional_disk_bytes` | Dataset, model staging, logs and other disk requirements beyond two checkpoints; default 0 |
| `gradient_checkpointing` | Declared flag, default false; does not magically reduce the supplied activation estimate |

Full training requires trainable count equal to total and 16/32-bit base weights.
LoRA requires 16/32-bit frozen base weights and separate FP32 adapter weights.
QLoRA requires 4/8-bit frozen base weights plus explicit quantization overhead.
These choices do not establish that a particular quantized format/kernel exists.

For P model and T trainable parameters, v1 accounts for:

1. Base weights: `ceil(P * weight_bits / 8)`.
2. Full FP16 training: FP32 master weights `4*T`; full FP32 adds no duplicate
   master. Adapter methods instead include `4*T` FP32 adapter weights.
3. FP32 gradients `4*T` and two Adam moment buffers `8*T`.
4. Explicit activations, attention, runtime and quantization workspaces.
5. Declared percentage headroom, rounded up. CPU includes host buffers before
   applying headroom. Dedicated GPU training/host estimates are separate;
   unified-memory admission uses their sum once.
6. Two complete checkpoint generations: saved model/adapter weights, applicable
   master weights and Adam state; each has 10% plus 4096 bytes serialization
   allowance. `additional_disk_bytes` is then added.

The coefficients are an explicit **Adam/FP32-state estimator**, not universal
memory profiling. Lazy optimizer allocation, attention scaling, allocator
fragmentation, kernel scratch space, checkpoint serialization, CPU offloading
and data loaders must be calibrated for a real backend before training admission.

The Rust `ResourceGuard` provides checked cooperative byte reservations, releases,
step counts, deadline and cancellation checks. The small CPU trainer uses it
for its own explicit data and work budgets. The planner CLI records the requested
budgets for the planned workload without applying them to other Goblin++ code or
processes.

## Next: measured small-device training

Beyond the CPU trainer, larger-model work should use a local reference model and
fixed held-out data, then measure time, peak memory and loss for a portable CPU path. Optimizations
can include small batches, accumulation, bounded streaming, checkpoint/resume and
supported quantized kernels. No automatic model substitution or change of precision,
context, dataset or RNG is permitted without an explicit recorded configuration.

Apple Silicon is a promising optional MLX path, particularly LoRA/QLoRA. It must
be validated separately from the CPU backend, with pinned versions, local-only
artifact handling and backend memory controls. The current `mlx-candidate` hint
is not an installed MLX backend, benchmark or compatibility guarantee.

Primary references informing this design:

- [MLX unified memory](https://ml-explore.github.io/mlx/build/html/usage/unified_memory.html)
- [MLX-LM LoRA and QLoRA](https://github.com/ml-explore/mlx-lm/blob/main/mlx_lm/LORA.md)
- [MLX memory-limit API](https://ml-explore.github.io/mlx/build/html/python/_autosummary/mlx.core.set_memory_limit.html)
- [Native Rust Candle backend candidate](https://github.com/huggingface/candle)
- [Linux cgroup v2 memory controller](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html)
- [Metal recommended working set](https://developer.apple.com/documentation/metal/mtldevice/recommendedmaxworkingsetsize)

Optional: [Goblin++ merchandise](https://h4k3rl1f3.myspreadshop.co.uk).

# Benchmarking Rust ACT-R against Lisp ACT-R

This directory compares three ways to run the same ACT-R models:

| label | what runs | transport |
|---|---|---|
| `lisp` | the upstream ACT-R 7 kernel under SBCL | RPC dispatcher (`framework/dispatcher.lisp`) |
| `rust` | the Rust port (`rust_actr7/src/bin/actr.rs server`) | the same RPC dispatcher protocol |
| `python` | the Rust engine in-process, through the `rust_actr` PyO3 binding | none |

The Lisp and Rust runs use the **same stock client**, `actr7.x/tutorial/python/actr.py`.
Only the server changes, so the comparison isolates the engine.  The Python
run replays the same models and event counts through the native session that
the Rust RPC server itself uses (`rust_actr._native.Session`); the public
`actr.Model` builder API cannot load an arbitrary `.lisp` model file, so the
benchmark bypasses the builder and drives the same command layer directly.

## Prerequisites

```sh
# Rust release server (not the debug build)
cd rust_actr7 && cargo build --release

# In-process Python binding
cd rust_actr7/python
maturin build --release
python -m pip install --force-reinstall target/wheels/rust_actr-*.whl

# Lisp run
sbcl --version
```

The first Lisp run may compile ACT-R's `.fasl` files and takes longer; later
server startups are about a second.

### Windows

The native Windows build uses the same commands:

```powershell
cargo build --release --manifest-path rust_actr7\Cargo.toml
python benchmarking\run.py --engine all
```

- `cargo` writes `rust_actr7\target\release\actr.exe`; `harness.rust_binary()`
  resolves the `.exe` automatically.
- `sbcl` must be on `PATH` (for example
  `C:\ProgramData\chocolatey\bin\sbcl.exe`).
- `--pin-cpu` is Linux-only and is ignored on other platforms.
- `parity.py` escapes native paths before writing them into the temporary Lisp
  script, so SBCL handles drive-letter paths.
- Stop any ACT-R server already listening on port 2650 before starting a run;
  the harness reports an occupied port as an error.

## Quick start

```sh
# All three backends, all workloads, results under benchmarking/results/
python benchmarking/run.py --engine all

# Just swap the server: same client, same workloads
python benchmarking/run.py --engine lisp
python benchmarking/run.py --engine rust

# In-process Python only
python benchmarking/run.py --engine python

# One real-model scenario, or just the synthetic suite
python benchmarking/run.py --engine all --workload real-fan-no-pm
python benchmarking/run.py --engine all --workload production-loop --workload dm-scan-2000

# Attach to a server you started yourself (e.g. a Lisp image in another shell)
python benchmarking/run.py --engine lisp --connect --workload dm-scan-2000

# Compare saved runs
python benchmarking/compare.py benchmarking/results/<timestamp>
```

Useful flags:

| flag | effect |
|---|---|
| `--workload NAME` | run one workload (repeatable; default: all) |
| `--repeats N` / `--warmup N` | override the per-workload defaults |
| `--scale X` | multiply synthetic event counts / scenario repeat counts |
| `--reuse-server` | one server for all workloads instead of a fresh one per workload |
| `--connect` | do not start a server; use the one already on port 2650 |
| `--pin-cpu N` | pin both the server and client to one CPU with `taskset` (Linux) |
| `--json-dir DIR` / `--tag TEXT` | result location / label |
| `--list` | print the workload list |

Results are saved as `results/<UTC timestamp>[-tag]/summary.json`, with one
`<engine>.json` per backend.  `run.py` prints the comparison table at the end.

## Workloads

There are two families.

### Synthetic feature workloads

These models live in `models/`; the large declarative memories are generated
by `gen_models.py` into `models/generated/` (deterministic, not checked in).
They use `run-n-events`, so every backend executes the same number of
scheduler events.

| workload | model | measured work |
|---|---|---|
| `production-loop` | `production-loop.lisp` | 200k production firings, no DM or device |
| `retrieval-loop` | `retrieval-loop.lisp` | 100k events, one successful retrieval per cycle |
| `dm-scan-200` | generated | retrieval over 200 chunks, fixed 50 ms latency |
| `dm-scan-2000` | generated | retrieval over 2,000 chunks |
| `partial-matching-2000` | generated | partial match against 2,000 chunks |

### Real-model scenarios

These are existing models from `actr7.x`, driven through the same RPC command
sequence their tutorial/test driver uses.  The sequence is defined once in
`scenarios.py` and executes unchanged against Lisp, Rust, and the in-process
binding.  They are device-free: no windows, vision, or motor modules.

| workload | model | scenario |
|---|---|---|
| `real-fan-no-pm` | `tutorial/unit5/fan-no-pm-model.lisp` | 36 fan retrieval trials (person and location terms) |
| `real-semantic` | `tutorial/unit1/semantic.lisp` | canary category queries (bird/animal/fish) |
| `real-semantic-dynamic` | `tutorial/unit8/semantic-dynamic-model.lisp` | g1→g2→g3→g2 dynamic-goal/compilation sequence |
| `real-dynamic-compilation` | `test-models/models/dynamic-production-compilation-tests.lisp` | 15 production-compilation configurations |

The scenario record distinguishes:

* **run phase** — wall time of the `run(...)` calls only; this is the engine
  measurement and the headline table.
* **full scenario** — wall time including resets and setup commands
  (`pdisable`, `mod-focus`, `goal-focus`, `penable`, …).  This is realistic
  end-to-end time, but it includes RPC setup traffic.
* **model time and outcome** — recorded per trial so Lisp and Rust can be
  checked for doing the same work.

Trace output is disabled with `(sgp :v nil)` in every workload.

### Correctness checks

Synthetic models are checked line by line:

```sh
python benchmarking/parity.py           # all synthetic models, 1 model second each
python benchmarking/parity.py --verbose --workload partial-matching-2000
```

`parity.py` runs each model through the direct Rust runner and a fresh SBCL
image, enables the trace, and compares every trace line.

Real-model scenarios are checked for work parity:

```sh
python benchmarking/verify_scenarios.py
python benchmarking/verify_scenarios.py --workload real-fan-no-pm
```

`verify_scenarios.py` runs each scenario in Lisp and Rust and compares trial
count, per-trial model time, and outcome.  All four pass.

## Methodology and caveats

* **Same client, same calls.** Lisp and Rust are both driven by
  `actr.py`; the only difference is the server process.  The Python backend
  uses the same `CoreSession` command layer in-process.  Real-model scenarios
  execute the identical Python command sequence on all three.
* **Same work.** Synthetic workloads use event counts and record the model
  time after each run.  Real scenarios record per-trial model time and
  response; `verify_scenarios.py` confirms they match across engines.
* **Two views of real models.** The run-phase number isolates engine work;
  the full-scenario number includes reset/setup.  The full-scenario number is
  dominated by RPC setup for tiny scenarios — for `real-fan-no-pm` there are
  six setup calls per trial before the timed run.
* **Trace off.** The Lisp dispatcher still checks trace filters per event even
  with `:v nil`; `:high-performance` has no Rust equivalent.  The Rust engine
  has no per-event trace-filter cost when trace is off.  This is a real
  difference in the systems, but it means a small part of the gap is trace
  plumbing rather than kernel work.
* **RPC overhead is measured.** Each client reports the median round-trip time
  of a no-op command (about 25–30 µs on the development machine).  It is
  negligible for the long synthetic runs but visible for tiny real scenarios:
  the Python run-phase numbers are roughly the Rust RPC numbers minus that
  per-run round trip.
* **Server isolation.** By default each workload gets a fresh server.  ACT-R
  auto-loads `actr7.x/user-loads/*.lisp`, and the Lisp dispatcher has no remote
  `clear-all` command, so the client first loads a generated
  `(delete-model ...)` sidecar.  `--reuse-server` reuses one process and relies
  on that sidecar; `--connect` assumes the server is yours.
* **Seeds.** The scenarios replicate the test driver's seed handling,
  including `:starting-parameters (:seed ...)` so every reset restores the
  RNG.  `verify_scenarios.py` compares model time and outcomes with a fixed
  seed.
* **Not measured:** device/vision/motor models, GC pauses, memory use,
  startup time, or multi-model experiments.
* Results are machine- and build-specific.  Record the commit, `rustc`,
  `sbcl`, and CPU with the results (`summary.json` does this automatically).

## Baseline results

Captured on an AMD Ryzen AI 7 350, Fedora, SBCL 2.6.9, rustc 1.99.1, the
release build, commit `6a66750`, with each workload's default repeat count.

### Synthetic feature workloads

```
workload               events  lisp ms  rust ms  python ms  rust/lisp  python/lisp
---------------------  ------  -------  -------  ---------  ---------  -----------
production-loop        200000  183.37   64.34    60.96      0.35x      0.33x
retrieval-loop         100000  129.89   86.24    83.66      0.66x      0.64x
dm-scan-200            10000   217.19   100.43   86.36      0.46x      0.40x
dm-scan-2000           4000    1503.88  371.45   355.26     0.25x      0.24x
partial-matching-2000  1000    631.60   109.31   107.73     0.17x      0.17x
```

### Real-model run phase

Sum of the `run(...)` calls per scenario block; this is the engine-only view.

```
scenario                  trials  lisp run ms  rust run ms  python run ms  rust/lisp  python/lisp
------------------------  ------  -----------  -----------  -------------  ---------  -----------
real-fan-no-pm            36      7.12         4.32         3.16           0.61x      0.44x
real-semantic             3       0.42         0.29         0.20           0.68x      0.48x
real-semantic-dynamic     4       0.72         0.77         0.58           1.07x      0.81x
real-dynamic-compilation  15      1.72         0.78         0.34           0.45x      0.20x
```

### Real-model full scenario

Includes resets and setup commands; realistic end-to-end time, but setup RPC
traffic dominates the tiny scenarios.

```
scenario                  trials  lisp ms  rust ms  python ms  rust/lisp  python/lisp
------------------------  ------  -------  -------  ---------  ---------  -----------
real-fan-no-pm            36      25.27    11.87    4.73       0.47x      0.19x
real-semantic             3       1.43     0.70     0.31       0.49x      0.22x
real-semantic-dynamic     4       1.35     1.18     0.64       0.87x      0.48x
real-dynamic-compilation  15      6.18     2.08     0.55       0.34x      0.09x
```

Takeaways from this run:

* Synthetic workloads still show the widest range: Rust is **1.5×–5.9×
  faster** than SBCL, with partial matching and large declarative memories
  dominating.
* On the real models, Rust is **1.6× faster** on fan retrieval and **2.2×
  faster** on the production-compilation scenario in the run phase.
  `semantic-dynamic` is essentially at parity (Rust about 7% slower in this
  run), so dynamic-goal/production-compilation performance is not uniformly
  better.
* The Python in-process run-phase numbers are close to `rust rpc minus the
  per-run round trip`: for fan, 36 trials × ~30 µs ≈ 1.1 ms, which is almost
  exactly the 4.32 → 3.16 ms difference.  The PyO3 binding itself is not a
  meaningful cost.
* Full-scenario Python times are much lower than the RPC backends because
  resets and setup commands are direct calls rather than socket round trips.
  Use the run-phase table for engine comparisons and the full-scenario table
  for end-to-end expectations.
* Model loading and reset are also generally faster in Rust; those are
  outside the timed run phase and are reported in `summary.json`.

## Files

```
benchmarking/
  run.py             # server setup, workload loop, JSON output, table
  client.py          # one benchmark client; BENCH_JSON lines on stdout
  workloads.py       # workload registry (synthetic + real scenarios)
  scenarios.py       # real-model RPC command sequences
  harness.py         # Lisp/Rust server lifecycle + subprocess plumbing
  report.py          # table formatting
  gen_models.py      # generated large-DM models + clear-user-models sidecar
  parity.py          # Lisp/Rust trace comparison for synthetic models
  verify_scenarios.py # Lisp/Rust work comparison for real models
  compare.py         # compare saved result directories
  models/            # hand-written models
  models/generated/  # generated at run time (ignored by git)
  results/           # result JSON (ignored by git)
```

To add a synthetic workload, add a `Workload` entry to `workloads.py` and a
model file under `models/`; run `parity.py --workload <name>` before
benchmarking it.  To add a real-model scenario, add a `ScenarioWorkload` entry
and a `Scenario` in `scenarios.py`, then run `verify_scenarios.py --workload
<name>`.

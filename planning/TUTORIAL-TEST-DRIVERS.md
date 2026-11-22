# Tutorial test drivers in the engine

Status: **deferred — documented for a later cleanup**

This cleanup was intentionally not performed during the Unit 6 browser work.
The Unit 6 BST task is implemented in JS and does not depend on the native
driver. Keep the native drivers for the regression tests until the feature-gate
or test-support extraction is scheduled.

The production engine still contains experiment-specific task drivers that are
used only by the tutorial regression corpus. They are not general ACT-R engine
capabilities and should not be part of the normal engine/RPC/wasm surface.

This note records the current state so the cleanup is not lost.

## Evidence

The Building Sticks driver is the clearest case:

- `Engine::bst_driver` — `rust_actr7/src/engine/driver.rs`
- `BstMode` — `rust_actr7/src/lib.rs`
- `Step::Bst`, `Step::BstTrial`, `Step::BstCollect`, `Step::BstSummary`
- Call sites:
  - `rust_actr7/tests/test_models.rs`
  - `rust_actr7/tests/cases/bst_learn.rs`
  - `rust_actr7/tests/cases/bst_nolearn.rs`
  - `rust_actr7/tests/cases/tutorial_unit6_bst_experiment_data.rs`
  - `rust_actr7/tests/cases/tutorial_unit6_bst_utility_trace.rs`
  - `rust_actr7/tests/cases/tutorial_unit8_bst_experiment_data.rs`

There is no call site in:

- `src/bin`
- the RPC server/session
- `WebSession` or wasm
- the Python port
- the native Environment

The Python BST port (`rust_actr7/python/tutorial/unit6/bst.py`) builds the model
and drives the session/task callbacks itself; it does not call the native
`bst_driver`.

Lisp ACT-R also has no `run-bst` RPC command. It has tutorial functions such as
`bst-experiment`, `bst-test`, and `bst-set` in
`actr7.x/tutorial/lisp/bst.lisp`. We should not invent a `run-bst` RPC command
merely to expose the Rust test driver.

## Broader inventory

The same pattern exists beyond BST. Tutorial/regression-specific drivers and
state currently in `src/engine` include:

- BST:
  - `bst_driver`, `bst_single_trial`
  - `bst_collect_start`, `bst_begin_trial`, `bst_finish_trial`,
    `bst_process_actions`, `bst_summary`
  - `bst_*` engine fields and run/reset/event hooks
- One-hit blackjack:
  - `OnehitMode`, `onehit_driver`
  - `onehit_hands`, `onehit_show_cards`, `onehit_show_results`
- Zbrodoff:
  - `ZbrodoffMode`, `zbrodoff_driver`, `zbrodoff_problem`
- Siegler:
  - `siegler_driver`, `siegler_experiment`
- Pcomp:
  - `PcompGameConfig`, `pcomp_game`, `pcomp_utility_names`
- Data collectors:
  - `FanCollect` / `FanSummary` / `FanExperimentSummary`
  - `PairedCollect` / `PairedSummary`
  - `SperlingCollect` / `SperlingSummary`
  - `BstCollect` / `BstSummary`
- Other tutorial/test entry points in `driver.rs`:
  - `zz_driver`, `al_driver`
  - `PastTenseWarning`, `TestingChildSlot`
  - `AgiSetup` / `AgiModify`
  - `CreatingImageSetup`
  - `AdaptiveNoiseTrace`, `BaseLevelInhibition`
  - `ThoroughP`
  - tracker setup

Some of these are lower-level test fixtures rather than full experiments, but
they share the same layering problem: they live in the production library and
are public through `Step`/`lib.rs`.

## Why this is a problem

- The engine crate's public API includes test-only experiment drivers.
- The wasm/browser build includes code that cannot be reached from the browser.
- The native release binary carries test-only task logic.
- Task/experiment logic is coupled to engine internals and fields, making the
  engine harder to understand and maintain.
- It is easy to mistake the drivers for supported product features, especially
  when adding a new tutorial unit.

The drivers are useful as regression oracles. The goal is not to delete the
tests, but to move or isolate the test logic.

## Proposed cleanup

Preferred short-term approach: feature-gate the tutorial drivers.

1. Add a `tutorial-drivers` Cargo feature. It must be **off by default**.
2. Gate tutorial-specific public enums/configs:
   - `BstMode`, `OnehitMode`, `ZbrodoffMode`, `PcompGameConfig`, and related
     driver structs.
3. Gate tutorial-specific `Step` variants and their `apply_engine_steps` arms.
4. Move the experiment-driver methods into a dedicated module such as
   `src/engine/tutorial_drivers.rs` and gate that module.
5. Gate engine state and hooks used only by those drivers:
   - BST collection fields and run/reset/event hooks
   - Fan/paired/Sperling collectors
   - Sperling event capture
   - Any other task-only state discovered by building with
     `--no-default-features`
6. Add:
   ```toml
   [[test]]
   name = "test_models"
   path = "tests/test_models.rs"
   required-features = ["tutorial-drivers"]
   ```
7. Build and test:
   - Normal engine: `cargo test`
   - Regression corpus: `cargo test --features tutorial-drivers`
   - Browser wasm: `cargo build --release --target wasm32-unknown-unknown --no-default-features`
8. Update:
   - `.github/workflows/release.yml`
   - `.github/workflows/macos-float-check.yml`
   - `README.md` test/build commands
   - Any planning docs that say `cargo test` runs the full regression corpus

Longer-term alternative: move the drivers into a separate test-support crate
or `tests/` support module. This is cleaner but requires exposing more engine
APIs or a narrow test-support interface. Do that only after the feature gate
proves the boundary.

## Constraints

- Keep the AST/trace regression cases working.
- Keep the default test suite self-contained.
- Do not add an `run-bst` RPC command.
- Do not expose `bst_driver` through `WebSession` or wasm.
- Unit 6 web work should implement BST in JS, not depend on the native driver.
- Do not remove a driver before moving/gating its tests and CI invocation.

## Related note

`planning/PARITY-AUDIT.md` already records that task logic outside the engine
is a known blind spot. This note is the concrete cleanup backlog for the
experiment drivers themselves.

# RPC Custom Modules Work

Last updated: 2026-11-27
Branch: `main`
Starting commit: `d00fb51`
Latest relevant code commit: `b94bdd2` (Milestone 7 in working tree)
Status: Milestones 1-7 complete; Milestone 8 next

This document tracks support for RPC-defined custom modules in the Rust
`actr-engine`. It records the design, the ordered milestones, the current
progress, and enough context for another agent to pick up the work.

The target is full parity with the vendored Python examples:

- `actr7.x/examples/creating-modules/external/goal.py`
- `actr7.x/examples/creating-modules/external/simple_declarative.py`

`actr7.x` is the oracle. Do not edit it to make the Rust port pass.

## 1. Goal

Allow an external RPC client (the tutorial `actr.py` client) to:

- call `define-module` and `undefine-module`
- register remote module callbacks with `add-command`
- expose module buffers, parameters, and query names to models
- replace the built-in goal and declarative modules
- issue nested ACT-R commands from module callbacks
- run the vendored `goal.py` and `simple_declarative.py` examples against
  `actr server`

The correct design is a dispatch-aware model loader, not a post-load replay
shim. Model-body `add-dm` and `goal-focus` forms must reach an overriding
command/module in the same load phase they would in Lisp.

## 2. Acceptance

The work is complete when:

- `goal.py` runs unit 1 tutorial models against `actr server` with the custom
  goal module.
- `simple_declarative.py` runs unit 1 count, addition, and semantic models with
  the custom declarative module.
- Existing Rust model tests still pass.
- The override path dispatches model-body forms through the command/module
  layer; no replay-after-load compatibility shim remains.
- The synchronous-callback reentrancy limitation is documented as a remaining
  full-parity gap.
- This document, `PYTHON-INTERFACE-WORK.md`,
  `PARITY-AUDIT.md`, and `PORTING-NOTES.md` reflect the final state.

Exact trace equality with Lisp is not an acceptance criterion. Behavioral
results and model outcomes are.

## 3. Non-goals

- Exact Lisp trace ordering for all replacement-module runs.
- Arbitrary Lisp execution in model files.
- Replacing built-in modules other than the goal/declarative cases exercised by
  the two examples.
- Rewriting every built-in module as a `Module` implementation. The immediate
  work uses an override-triggered dispatch path.

## 4. Current baseline

What exists:

- Rust custom modules can be registered through `ModuleFactory`; the Python
  binding exposes an in-process `actr.Module` API.
- `Session::add_module_factory` registers Rust-side factories and enforces the
  no-model-loaded restriction.
- The engine already routes requests and queries through
  `module_index_for_buffer` before hardcoded buffer behavior
  (`src/engine/events.rs`).
- The RPC server supports owner-aware client callbacks and a reentrant callback
  path for a few sites (`src/rpc/server.rs`).
- Allowlisted module commands (`add-dm`, `dm`, `sdp`, `goal-focus`,
  `mod-focus`) can be removed and replaced while no model is loaded, with
  built-in restoration on override removal/disconnect.
- RPC `define-module`/`undefine-module` register per-connection remote module
  factories with parsed buffers, parameters, metadata, and callback names.
- Remote module `create`, `query`, `params`, `search`, and `offset` callbacks
  run synchronously through `RpcModuleHost`; nested ACT-R commands from them
  are rejected with a clear error.
- Deferred `reset`, `notify-on-clear`, `update`, `run-start`, `run-end`, and
  `delete` callbacks are queued by the engine and drained with the session
  borrow released. `run`/`run-full-time`/`run-until-time` yield between events
  so a callback can issue nested commands before the next event.
- Session-scoped chunk-spec IDs and `define-chunk-spec`,
  `chunk-spec-to-chunk-def`, `release-chunk-spec-id`.
- Remote `request`/`buffer-mod` callbacks receive spec IDs and run through the
  deferred bridge.
- `schedule-set-buffer-chunk`, spec-aware `schedule-mod-buffer-chunk`,
  `set-buffer-failure`, `schedule-event-after-module`, `delete-event`, and
  external event IDs.
- Scheduled client commands are drained to their owning connection.
- RPC model loads defer remote load actions to the server so goal-focus and
  other nested callbacks run with the session borrow released.
- The vendored Python `goal.py` replacement runs count, addition, and semantic
  with matching model outcomes.

The vendored `simple_declarative.py` replacement now runs count, addition, and
semantic with matching model outcomes. Its model-body `add-dm` is dispatched to
the client, `define-chunks` returns chunk names, and the DM utility commands
(`find-matching-chunks`, `merge-chunks`, `chunk-not-storable`,
`make-chunk-immutable`, `extend-chunks`, `model-warning`) are available.

Deliberate limits:

- `extend-chunks` supports default values and `merge-value-function`; the full
  Lisp copy/merge option matrix is not implemented.
- Declarative metadata is session-side for external chunks. The built-in DM
  viewer and stepper do not understand a replacement module's private store.
- `find-matching-chunks` supports the built-in slot tests; caller-supplied
  custom test functions are not dispatched.

## 5. Architecture decisions

### 5.1 Loader refactor before RPC modules

The model-loader refactor lands first. It gives the RPC proxy a correct load
contract and avoids writing and then deleting a replay shim.

### 5.2 Dispatch-aware loader with an override-triggered fast path

Normal models keep the current static parse path. The dispatch-aware path runs
when a replacement is active:

- a command such as `add-dm`, `dm`, `sdp`, `goal-focus`, or `mod-focus` has
  been replaced by a remote/client command, or
- a custom module owns `GOAL` or `RETRIEVAL`.

The dispatch path must match the Lisp `define-model-fct` phases:

1. create model/module instances
2. primary module reset
3. module parameter defaults
4. secondary module reset
5. execute model-body load actions in order
6. tertiary module reset

Reference: `actr7.x/framework/model.lisp:384-577`.

For RPC loads, phase 5 is paused after the engine is built: the session queues
`PendingLoadDispatch`, the server invokes the actions with the session borrow
released, then the session runs phase 6. In-process/test remotes that do not need
nested callbacks keep the synchronous path.

### 5.3 No replay shim

Do not synthetically replay parsed chunks/goals after the engine is built.
Record ordered load actions and execute them through the command/module layer
during the load.

### 5.4 Callback split

Deferred side-effect callbacks, run with the session borrow released:

- `request`
- `buffer-mod`
- `notify-on-clear`
- `warning`
- `reset`
- `delete`
- `run-start`
- `run-end`

Synchronous data callbacks, run while the engine is borrowed:

- `create`
- `query`
- `params` get
- `search`
- `offset`

Synchronous callbacks must not re-enter ACT-R. Detect nested attempts and
return a clear error instead of deadlocking. The vendored examples satisfy this
constraint because their synchronous callbacks only read remote state.

Milestone 3 implements the synchronous half via `RpcModuleHost`; Milestone 4
adds the deferred side-effect queue and runs those callbacks through
`call_remote` with the session borrow released.

A general reentrant synchronous-callback design remains a full-parity
follow-up.

### 5.5 Command override scope

Allow remote replacement/removal of module-related built-in commands only:

- `add-dm`
- `dm`
- `sdp`
- `goal-focus`
- `mod-focus`

Do this only when no model is loaded. Do not open all built-ins to replacement
without a separate decision.

Removed built-ins are stashed. Removing the remote replacement, or disconnecting
its owner, restores the original handler. A bare removal with no replacement
stays removed until the command is re-added.

## 6. Milestones

Status markers: `[ ]` not started, `[~]` in progress, `[x]` complete.

### Milestone 1 — Dispatch-aware model loader

- [x] Add `ModelLoadAction` data and `Model.load_actions` in
  `rust_actr7/src/model.rs`.
- [x] Keep the existing static `Model` fields for validation, compilation, and
  the fast path.
- [x] Add session load-action dispatch and wire Lisp-like load phases in
  `rust_actr7/src/rpc/session.rs`.
- [x] Trigger dispatch when a relevant command override exists. The
  buffer-ownership trigger is deferred to Milestone 3/6 because there is no
  remote callback target yet.
- [x] Prevent double application of `add-dm`/`goal-focus` when the dispatch path
  is active.
- [x] Preserve reset, reload, and multi-model behavior.
- [x] Test with `Session::add_remote`, a fake `Remote`, and an in-process module
  owning `RETRIEVAL`.
- [x] Verify the existing full model corpus is unchanged.

Acceptance: a fake remote `add-dm`/`goal-focus` receives load-time forms in
phase order without any post-load replay.

Implementation notes:

- Dispatched actions are `ADD-DM`, `DEFINE-CHUNKS`, `MERGE-CHUNKS`, and
  `GOAL-FOCUS`.
- The static fields for an overridden action are cleared before engine
  construction, so the built-in path cannot apply it a second time.
- Reset re-dispatches the same actions. The engine defers the tertiary module
  reset until after the actions run.
- Runtime engine rebuilds do not replay load actions.
- `Session::load_source` keeps the static fast path (`allow_load_dispatch =
  false`); the dispatch path is used by `load-act-r-model`/`load-act-r-code`
  with an RPC client.

### Milestone 2 — Built-in command override foundation

- [x] Allow removal/replacement of the allowlisted module commands when no
  model is loaded.
- [x] Preserve normal built-in behavior when no override exists.
- [x] Add tests for `check`, `remove`, `add`, and restore behavior.

Acceptance: a remote client can remove `add-dm`/`goal-focus`, register its own
version, and have dispatch use it.

Implementation notes:

- `OVERRIDABLE_BUILTINS` is the §5.5 allowlist; other built-ins remain
  protected.
- `Session::remove_checked` and `Session::add_remote_checked` enforce the
  no-model-loaded rule. A built-in must be removed before a replacement can be
  added.
- Removed built-ins live in `Session::removed_builtins`. Removing the remote
  replacement or disconnecting its owner restores the original handler.
- RPC `add`/`remove` return `[false, message]` when the dispatcher rejects the
  request.

### Milestone 3 — `define-module` registration and remote proxy

- [x] Add a scoped synchronous data-callback bridge for module `create`,
  `query`, `params` get, `search`, and `offset`. The bridge returns a value
  while the session is borrowed and must not allow nested ACT-R commands.
- [x] Wire the bridge into `RemoteModule` so model construction can call
  `create` and params-get and production matching can call `query`.
- [x] Detect and reject nested commands from synchronous callbacks with a clear
  error.
- [x] Add tests for returning `create`/`query`/`params` callbacks and for
  nested-call rejection.
- [x] Add `define-module` and `undefine-module` to the RPC command registry in
  `rust_actr7/src/rpc/session.rs`.
- [x] Parse buffer definitions, remote parameter specs, callback names, and
  metadata from the Python client shape.
- [x] Track the owning connection for later callbacks.
- [x] Add `rust_actr7/src/rpc/remote_module.rs` with a `RemoteModuleFactory` and
  `RemoteModule`.
- [x] Implement descriptor generation without remote side effects.
- [x] Remove remote factories owned by a disconnected client when allowed.
- [x] Add tests for define, undefine, metadata, ownership, and duplicate names.

Acceptance: `define-module` registers a module whose buffers, queries, and
parameters are visible to a loaded model.

Implementation notes:

- `RemoteModuleFactory` stores the parsed descriptor, callback names, owner
  connection, and an `Rc<dyn ModuleHost>`.
- `RpcModuleHost` shares the actor's event receiver and deferred queue so it can
  wait for a callback reply while `&mut Session` is borrowed. It times out after
  30 seconds and answers nested requests with a clear error.
- M3 wires only the synchronous callbacks. Milestone 4 adds the deferred
  `reset`, `notify-on-clear`, `update`, `run-start`, `run-end`, and `delete`
  paths; `request`/`buffer-mod` wait for Milestone 5.
- Built-in `undefine-module('goal'/'declarative')` still returns `false`; full
  built-in module suppression and focus-command replacement remain Milestone 6.

### Milestone 4 — Deferred callback bridge

- [x] Add an engine-level pending remote-callback queue and a
  `ModuleCtx` method to enqueue side-effect callbacks.
- [x] Make the run loop yield before the next event when callbacks are pending.
- [x] Drain callbacks in `rust_actr7/src/rpc/server.rs` with the session borrow
  released.
- [x] Route deferred callbacks through the owner-aware reentrant callback path.
- [x] Test a deferred reset callback that issues a nested ACT-R command.

Acceptance: a remote module callback can issue ACT-R commands that take effect
before the next event.

Implementation notes:

- `ModuleCtx::enqueue_callback` records the owner connection, callback command,
  model, and args. `Engine::run_to_limit` yields before the next event while
  the queue is non-empty.
- `RemoteModule` now queues `reset`, `notify-on-clear`, `update`, `run-start`,
  `run-end`, and `delete`. `delete` has no engine call site yet.
- The server drains with `call_remote`, so nested commands are serviced with the
  session borrow released; a depth guard prevents recursive draining.
- `request`/`buffer-mod` remain Milestone 5 because their callback args are
  chunk-spec IDs.
- `run-n-events` still drains callbacks only after the command; the
  run/run-full-time/run-until-time paths yield between events.
- Multi-model run-start/run-end callbacks are not invoked by the multi runner
  yet.

### Milestone 5 — Chunk specs and scheduling

- [x] Add a session-scoped chunk-spec ID registry.
- [x] Implement `define-chunk-spec`, `chunk-spec-to-chunk-def`, and a real
  `release-chunk-spec-id`.
- [x] Assign spec IDs to module `request` and `buffer-mod` callback arguments.
  `warning` still has no engine call site.
- [x] Wire `RemoteModule::request` and `buffer_mod` to enqueue the deferred
  callbacks once spec IDs exist.
- [x] Implement `schedule-set-buffer-chunk`.
- [x] Implement `schedule-event-after-module`.
- [x] Implement `set-buffer-failure` and `delete-event`.
- [x] Extend `schedule-mod-buffer-chunk` to accept spec IDs.
- [x] Clear or invalidate spec IDs at the correct model lifecycle boundaries.

Acceptance: the goal-style remote module can receive a request spec and schedule
the resulting buffer change through the normal event queue.

Implementation notes:

- `ChunkSpecTable` is session-scoped and shared with every engine. Request and
  buffer-mod specs get IDs only for external modules.
- Scheduled events carry external `rpc_id`s; `delete-event` removes queued and
  waiting events.
- `schedule-event-after-module` releases its event when the matching module’s
  next event executes. `dynamic`/`precondition` are accepted but ignored and
  `include-maintenance` is best-effort.
- Client commands scheduled through the engine are registered as task commands
  and drained to their owning connection.
- `chunk-spec-to-chunk-def` rejects negated/relative spec slots for now; full
  matching semantics remain Milestone 7.

### Milestone 6 — Goal replacement parity

- [x] Enable replacement of `goal-focus`/`mod-focus` and the `GOAL` buffer via
  the loader dispatch path.
- [x] Ensure model-body goal focus uses the custom module when `GOAL` is owned
  by a remote module.
- [x] Run `goal.py` against unit 1 tutorial models.
- [x] Compare model outcomes and fix lifecycle/timing mismatches.
- [x] Add a regression test or scripted smoke test.

Acceptance: `goal.py` replaces the goal module and the unit 1 tutorial models
still run with expected results.

Implementation notes:

- RPC model loads now build the engine, defer model-body load actions to the
  server, dispatch remote actions with the session borrow released, then run the
  tertiary reset. This fixes the deadlock where goal-focus issued nested
  commands during load.
- Client-invoked remote commands now route through `call_remote`, so runtime
  `goal-focus`/`mod-focus` callbacks can also issue nested commands.
- The Python replacement `goal-focus` only accepts chunk names; spec-list args
  are converted to a generated chunk name before the remote callback.
- Scheduled `release-chunk-spec-id` now executes in the engine against the
  shared spec table.
- `undefine-module('goal')` records the built-in suppression and returns true.
  The hardcoded GOAL buffer still exists, but module-buffer ownership routes
  requests/queries to the replacement.
- `fixtures/rpc-goal-smoke.py` runs the real vendored `goal.py` against count,
  addition, and semantic.

Trace differences from the built-in goal module remain (for example the
`clear-delayed-goal` NONE line and `SET-BUFFER-CHUNK … NIL`), but the model
outcomes and outputs match.

### Milestone 7 — Declarative replacement parity

- [x] Enable replacement of `add-dm`, `dm`, and `sdp` through the dispatch path.
- [x] Add the missing DM utility commands listed in section 4.
- [x] Reuse existing chunk matching/merge machinery where possible.
- [x] Ensure model-body `add-dm` reaches the custom module during load.
- [x] Run `simple_declarative.py` against unit 1 count, addition, and semantic.
- [x] Add a regression test or scripted smoke test.

Acceptance: `simple_declarative.py` replaces declarative memory and the unit 1
models still run with expected results. The gated
`rpc_declarative_conformance` driver runs the real vendored module against all
three models; count prints `FOUR`, addition prints `SEVEN`, and semantic returns
yes/yes/no for the tested goals. `cargo test` covers the utility commands,
scheduled destination routing, and built-in DM merge suppression.

### Milestone 8 — Docs and polish

- [ ] Update this document's status, progress log, and commit list.
- [ ] Update `planning/PYTHON-INTERFACE-WORK.md` §4.2 and §8.
- [ ] Update `planning/PARITY-AUDIT.md` known follow-ups.
- [ ] Add or update the `planning/PORTING-NOTES.md` architectural note for the
  callback split and synchronous reentrancy limitation.
- [ ] Add a one-line pointer from `planning/PORTING-PLAN.md` if useful.
- [ ] Document any remaining gaps in user-facing Python/RPC docs.

## 7. Known limitations and risks

- **Synchronous callback reentrancy.** `query`, `params` get, `create`,
  `search`, and `offset` need immediate return values. The M3 bridge returns
  them while the engine is borrowed and rejects nested ACT-R commands. Full
  reentrant synchronous callbacks remain a follow-up. The vendored examples do
  not require nesting from these callbacks.
- **Synchronous callback cost.** Every remote `query`/`params` call is one RPC
  round-trip. `query` can be hot during production matching, though only for
  models whose buffer is owned by a remote module. Consider mirroring or
  caching state if a real workload makes it visible.
- **Blocking module callbacks.** A synchronous callback blocks the shared actor
  until the client replies or the 30 s timeout fires. A hung client therefore
  stalls all models.
- **Loader regression surface.** The loader refactor touches model parsing,
  engine construction, reset ordering, and trace output. Keep the static fast
  path for non-override models and run the full corpus after each change.
- **Built-in replacement scope.** `undefine-module('goal'/'declarative')` records
  a suppression and returns true, but the hardcoded GOAL buffer/initial-goal
  paths remain. Module-buffer ownership routes requests/queries correctly, and
  a custom `RETRIEVAL` owner suppresses the built-in DM clear hook. Both vendored
  Python replacements are covered.
- **Chunk parameter scope.** `extend-chunks` supports the default value and
  merge-value callback used by `simple_declarative.py`. Copy functions, custom
  merge functions, and full per-chunk parameter integration with the built-in
  DM viewer are not implemented.
- **Scheduled destinations.** A remote scheduled action with a destination gets
  the model name prepended, matching the vendored external modules whose
  creation callback returns the model name. Arbitrary external creation return
  values are not stored or replayed.
- **Top-level remote command monitors.** Client commands invoked directly via
  `evaluate` now route through `call_remote` so they can nest; their dispatcher
  before/after/simple monitors are not run on that path.
- **Goal replacement traces.** The Python goal module adds
  `clear-delayed-goal`/`… NIL` trace differences even though model outcomes
  match.
- **Deferred callback coverage.** `request`/`buffer-mod` are wired. `warning`
  and `delete` have no engine call site yet.
- **`run-n-events` ordering.** It drains module callbacks after the command
  completes rather than yielding between events. Use `run`/`run-full-time`/
  `run-until-time` when callback ordering matters.
- **After-module scheduling.** `dynamic` and `precondition` are accepted but
  ignored; `include-maintenance` is best-effort. The goal-style after-module
  path is covered.
- **Multi-model lifecycle.** Remote module instances are per model; callback
  ownership, factory cleanup, and reset ordering need more coverage. The multi
  runner does not yet invoke module `run-start`/`run-end` callbacks, and fired
  task commands use the current model.
- **Disconnect cleanup.** Owned remote factories are removed immediately when no
  model is loaded, or pruned after the last model is cleared when a model was
  loaded at disconnect time.
- **Chunk-spec lifetime.** The table clears on reset, model load, and clear-all.
  `chunk-spec-to-chunk-def` rejects negated/relative slots;
  `find-matching-chunks` supports direct, negated, and relative built-in tests
  but not caller-supplied custom test functions or variable characters.
- **Trace parity.** Rust trace ordering may differ from Lisp even when model
  results match.

## 8. File map

Primary implementation:

- `rust_actr7/src/model.rs` — static parse, `ModelLoadPlan`/actions
- `rust_actr7/src/rpc/session.rs` — command registry, load phases,
  define/undefine, chunk specs
- `rust_actr7/src/rpc/server.rs` — callback dispatch and reentrant drain
- `rust_actr7/src/rpc/builtin_docs.rs` — new command documentation
- `rust_actr7/src/rpc/remote_module.rs` — new remote module proxy
- `rust_actr7/src/engine.rs` — run-loop callback yield
- `rust_actr7/src/engine/module.rs` — `ModuleCtx` callback queue
- `rust_actr7/src/engine/events.rs` — request/query routing precedence
- `rust_actr7/src/engine/commands.rs` — model-command dispatch details
- `rust_actr7/src/engine/dm.rs` — chunk matching/merging helpers

Tests:

- `rust_actr7/tests/rpc_modules.rs` — new integration coverage
- `rust_actr7/tests/environment_rpc.rs` — TCP client/test pattern
- `rust_actr7/tests/stepper_rpc.rs` — callback test pattern
- `rust_actr7/tests/rpc_conformance.rs` — gated Python/ACT-R conformance
- `rust_actr7/fixtures/models/rpc-modules/probe.lisp` — custom-buffer query
  fixture for the synchronous bridge test
- `rust_actr7/fixtures/models/rpc-modules/request-probe.lisp` — request/spec
  buffer scheduling fixture
- `rust_actr7/fixtures/rpc-goal-smoke.py` — real `goal.py` smoke driver for
  count, addition, and semantic
- `rust_actr7/fixtures/rpc-declarative-smoke.py` — real
  `simple_declarative.py` smoke driver, including DM utility checks

Examples and oracle:

- `actr7.x/examples/creating-modules/external/readme.txt`
- `actr7.x/examples/creating-modules/external/goal.py`
- `actr7.x/examples/creating-modules/external/goal_complete.py`
- `actr7.x/examples/creating-modules/external/simple_declarative.py`
- `actr7.x/tutorial/python/actr.py` — client behavior and callback shape
- `actr7.x/framework/model.lisp` — Lisp load phase ordering

Docs:

- `planning/RPC-MODULES-WORK.md` — this file
- `planning/PYTHON-INTERFACE-WORK.md`
- `planning/PARITY-AUDIT.md`
- `planning/PORTING-NOTES.md`
- `planning/PORTING-PLAN.md`

## 9. Verification

From `rust_actr7`:

```powershell
cargo fmt --check
cargo test
```

New focused tests should also be runnable as:

```powershell
cargo test --test rpc_modules
```

If the Python example smoke test is added, it needs Python and a built
`actr` binary. Keep any such test environment-gated like
`tests/rpc_conformance.rs`.

Do not modify `actr7.x` to make tests pass. Fix the Rust side or add a
project-authored surrogate/fixture.

## 10. Progress log

Add one entry per milestone or meaningful checkpoint, with the commit range.

- 2026-11-27: Created this planning document. No implementation started.
  Starting commit `d00fb51`.
- 2026-11-27: Milestone 1 complete. Added `ModelLoadAction`/`load_actions`
  (`70de38c`) and the session dispatch path with reset-phase handling and tests
  (`b5a8aab`). The full 560-case model corpus and 136 library tests passed during
  the code work. After the final `load_source` gating change, `cargo test --lib`
  and `cargo test --test rpc_modules` (9 tests) were rerun and passed;
  `cargo fmt --check` clean.
- 2026-11-27: Milestone 2 complete; the scoped synchronous data bridge moved to
  the start of Milestone 3 (`d73f2d0`). Added the overridable-builtin allowlist,
  checked add/remove with model-state guards, built-in stash/restore, disconnect
  restoration, and RPC error replies. `cargo test` passed the 560-case corpus,
  145 lib tests, 10 `rpc_modules` tests, and 28 `environment_rpc` tests;
  `cargo fmt --check` clean.
- 2026-11-27: Milestone 3 complete. Added `ModuleCtx::model_name` (`792bbac`),
  remote module parsing/proxy (`65288a3`), `define-module`/`undefine-module`
  registration and ownership cleanup (`a959b03`), and the scoped synchronous
  bridge with nested-command rejection (`f3e0a8e`). `cargo test` passed the
  560-case corpus, 154 lib tests, 31 `environment_rpc` tests, 10 `rpc_modules`
  tests, and 2 `stepper_rpc` tests; `cargo fmt --check` clean.
- 2026-11-27: Milestone 4 complete. Added the engine pending-callback queue and
  run-loop yield (`0352417`), wired deferred `reset`/notify/update/run-start/
  run-end callbacks (`83fa3f7`), and drained them through the owner-aware
  reentrant path (`af4bb7c`). `cargo test` passed the 560-case corpus, 156 lib
  tests, 32 `environment_rpc` tests, 10 `rpc_modules` tests, and 2 `stepper_rpc`
  tests; `cargo fmt --check` clean.
- 2026-11-27: Milestone 5 complete. Added the shared chunk-spec table and spec
  IDs (`aa839fb`), external event IDs, waiting after-module events, buffer
  scheduling/failure, spec-aware mod requests, and server command draining
  (`dbc0427`). `cargo test` passed the 560-case corpus, 160 lib tests, 34
  `environment_rpc` tests, 10 `rpc_modules` tests, and 2 `stepper_rpc` tests;
  `cargo fmt --check` clean.
- 2026-11-27: Milestone 6 complete. Added deferred RPC load dispatch and
  reentrant top-level remote commands (`23ebfa0`), scheduled chunk-spec release
  (`4ad8515`), and goal replacement tests plus the gated Python goal smoke
  driver (`b94bdd2`). `cargo test` passed the 560-case corpus, 162 lib tests, 36
  `environment_rpc` tests, and 10 `rpc_modules` tests. The ignored
  `rpc_goal_conformance` test ran the real `goal.py` against count, addition,
  and semantic successfully.
- 2026-11-27: Milestone 7 complete in the working tree. `define-chunks`/`add-dm`
  now return chunk names, `extend-chunks` adds external chunk parameters, and
  `find-matching-chunks`/`merge-chunks` reuse the engine chunk machinery.
  Scheduled remote actions with a destination prepend the model name, and a
  custom `RETRIEVAL` owner suppresses the built-in DM clear merge. Added
  `rpc-declarative-smoke.py` and `rpc_declarative_conformance`; the real
  `simple_declarative.py` runs count, addition, and semantic successfully.
  `cargo test` passed 164 lib tests, 36 `environment_rpc`, 11 `rpc_modules`, and
  the 560-case model corpus. `cargo fmt --check` is clean.

## 11. Handoff notes

Next action: start Milestone 8, docs and polish. Update the Python interface and
parity notes, then review the remaining declarative limitations above.

Before editing:

- Read `planning/RPC-MODULES-WORK.md` section 4 command list and Milestone 8.
- Review `planning/PYTHON-INTERFACE-WORK.md` §4.2 and §8.
- Review `planning/PARITY-AUDIT.md` known follow-ups.
- Review `planning/PORTING-NOTES.md` for the callback split and synchronous
  reentrancy architectural note.

When finishing a milestone:

- update the checkbox and add an acceptance note
- append the commit hash(es) to the progress log
- update the related planning docs listed in Milestone 8
- leave the next action explicit

Working tree note: `actr7.x/environment/gui/init/00-net-config.tcl` may be
modified by the environment. It is unrelated to this work; do not commit it.

## 12. Related notes

- `planning/PORTING-NOTES.md` #219 and #261 cover existing callback reentrancy
  fixes and the remaining latent deadlock shape.
- `planning/PARITY-AUDIT.md` records post-corpus follow-ups.
- `planning/PYTHON-INTERFACE-WORK.md` §4.2 documents current callback timing and
  §8 lists known limitations.

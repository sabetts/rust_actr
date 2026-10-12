# TODO — ACT-R 7 test-model coverage

Status legend: `- [ ]` todo · `- [x]` done · `(IN PROGRESS)` picked up.

## Ground rules

- **Oracle**: `actr7.x/test-models/*.ref`. Reproduce it **byte-exact**, even where
  it contradicts current ACT-R source.
- **Keep `cargo test` green.** `rust_actr7/golden/test-models/enabled.txt` is the
  green allowlist; only add a spec there once it matches.
- **Commit as work proceeds**, identity:
  `git -c user.name=sabetts -c user.email=sabetts@localhost commit`.
- **GOTCHAS.md**: add an entry for non-obvious behavior (tied to a source line),
  not a changelog.
- **No command layer.** COMMAND-scope specs and unit-test refs are out of scope
  unless re-triaged in Phase B.
- Device sidecars (`*.device.sexp`) are hand-authored and must not be clobbered;
  spec regeneration is additive (`SPEC_LIST`).

## Recon (2026-10-08)

- 571 `.ref` files (563 top-level); **219 specs**; **58 enabled/green**;
  **2 failing**; **159 ignored**.
- Of the ignored: **156** are `:COMMAND` (classified from stub source tokens in
  `golden/extract-specs.lisp`, which is over-broad). Cross-checking the refs:
  **23 are pure model traces**, **42 near-pure** (≤3 non-trace lines), **91 have
  real command output**.
- The remaining **344 non-timestamped refs** are unit-test/command dumps
  (chunk-type/sgp/production-text) — out of scope.
- Engine is split into `rust_actr7/src/engine/*.rs`; crate is `cargo fmt`-clean.

## How to run

```sh
cd rust_actr7
# probe every runnable spec (pass/fail + first diff)
cargo test --test test_models probe_runnable -- --ignored --nocapture
# ... with the ignored reasons appended
PROBE_IGNORED=1 cargo test --test test_models probe_runnable -- --ignored --nocapture
# dump one spec's actual trace, line-numbered
DUMP_SPEC=<name> cargo test --test test_models dump_spec -- --ignored --nocapture
```

Regenerate specs (additive; never clobbers hand-authored device sidecars):

```sh
# all timestamped refs:
rust_actr7/golden/regen-specs.sh
# explicit subset (one test name per line, no .ref):
SPEC_LIST=/tmp/opencode/spec-list rust_actr7/golden/regen-specs.sh
```

Enable a passing spec by appending its name to
`rust_actr7/golden/test-models/enabled.txt`.

## Definition of done (per task)

- [ ] spec(s) added to `enabled.txt`
- [ ] `cargo test` green
- [ ] committed
- [ ] `GOTCHAS.md` entry if the fix is non-obvious

---

## Phase A — close the frontier (2 known failures)

- [x] **A1. Sperling vision finst model** → enable
      `tutorial-unit3-sperling-trial-trace`.
  - [x] Add a vision finst list + `NEW` state on visicon feature creation.
  - [x] `:attended` semantics: `:attended nil` matches `feat-attended ∈ {NEW, nil}`;
        `:attended new` matches `NEW`; `t`/`requested` excluded
        (`vision.lisp:2269`, `test-attended` `vision.lisp:2006`).
  - [x] `add-finst` on `encoding_complete`; cap at `:visual-num-finsts`
        (evict oldest) + expire by `:visual-finst-span`.
  - [x] Report phase: drop the stale window-clear re-encode when a move-attention
        starts (proc-display lock deferral). Result: 5th find `VISUAL-LOCATION1`,
        report at `1.110`. GOTCHA #191.
  - [x] Enable the spec (59 specs); commit.
  - Refs: `engine/perception.rs` (`find_location`, `encoding_complete`,
    `add_visual_finst`, `check_visual_finsts`, `loc_matches`), `engine.rs`
    (`Feature`, `Config`), `engine/events.rs` (`MoveAttention`, `UnstuffVl`),
    `actr7.x/core-modules/vision.lisp:2006,2269,2282,2247,2929,1542`.

- [x] **A2. paired-learning utility / selection** → enable `paired-learning`.
  - [x] Root cause was spec generation, not the engine: the extractor never
        applied `ssp :starting-parameters (:seed …)` / `:alpha`, and the device
        sidecar was authored with the wrong seed. Fixed in
        `golden/extract-specs.lisp` + `Step::RunFullTime` per-run seeding;
        regenerated the spec/device. GOTCHA #192.
  - [x] Composition fixes closed the remaining hunks (`rename-variables`
        (buffer, slot), `MAP-IMAGINAL-BUFFER` RHS-`*`, `COMPOSE-GOAL-BUFFER`
        `(4 12 13 44)` a1=, `buffer=-union` a1*/a2=). GOTCHA #193.
  - [x] Enable the spec (60 specs); commit.

## Phase B — unlock hidden real-model traces (coverage ROI)

- [x] **B1. Content-based scope triage.** `*command-tokens*` no longer treats
      `(sgp :v …)`/`*out-file*`/`schedule-mod-buffer-chunk` as command output,
      and a stub with no command token whose ref is a filtered fragment (no
      run end) stays `:command` (`ref-has-run-end-p`). `fresh-state` per test
      and `*out-file*` bound. Trace gating: `w-run` records the model's `:v`
      again, `w-reset` applies the testing function (default
      `set-testing-traces`), engine handles `:V`. `run-until-time`/
      `run-n-events` steps added (extractor + engine). GOTCHA #194.
- [x] **B2. Regenerate the 23 pure-trace candidates** (additive `SPEC_LIST`),
      recording real steps/seeds. 7 now pass and are enabled
      (`tutorial-unit1-semantic-g2/-g3-trace`,
      `tutorial-unit5modeling-v7-p4-run`/`-v8-p6-run-high-detail`/
      `-v9-p6-retrieval`/`-v9-p8-actions`,
      `tutorial-unit7modeling-ul-issues-v1-second-run`); 14 were reclassified
      `:command` (filtered `model-output` fragments); 2 fail (Phase C).
      Candidates:
      `hear-self`, `imaginal-action`, `tutorial-unit1-semantic-g2-trace`,
      `tutorial-unit1-semantic-g3-trace`,
      `tutorial-unit2-demo2-find-unattended-letter-actions`,
      `tutorial-unit2-demo2-visual-reencode-actions`,
      `tutorial-unit2code-demo2-manual-actions-trace`,
      `tutorial-unit3-sperling-first-production`,
      `tutorial-unit3-sperling-stuff-visual-location`,
      `tutorial-unit3code-sperling-clear-exp-window-trace`,
      `tutorial-unit3modeling-v3-clear-imaginal`,
      `tutorial-unit3modeling-v4-next-segment-trace`,
      `tutorial-unit3modeling-v4-previous-segment-trace`,
      `tutorial-unit5-count-mod-goal-event`,
      `tutorial-unit5-count-mod-request-goal-event`,
      `tutorial-unit5-imaginal-mod-event`,
      `tutorial-unit5-imaginal-mod-request-event`,
      `tutorial-unit5modeling-v7-p4-run`,
      `tutorial-unit5modeling-v8-p6-run-high-detail`,
      `tutorial-unit5modeling-v9-p6-retrieval`,
      `tutorial-unit5modeling-v9-p8-actions`,
      `tutorial-unit7modeling-ul-issues-v1-second-run`,
      `tutorial-unit8-simple-dynamic-extend-buffer-chunk`.
- [x] **B3. Probe + enable passers**; file every mismatch as a Phase C task.
      Frontier 60 → 67 specs. Remaining B2 failures:
  - `hear-self` — the speech module is missing the `SPEAK <text>` playback
    line and the subvocalize sound (`speech.lisp`), and the run stops early.
  - `imaginal-action` — the model uses the `imaginal-action` buffer
    (`+imaginal-action> … action "cmd" slots (count)`, a command-layer
    feature); the parser rejects `slots (count)`.
- [x] **B4. Near-pure subset.** Literal `(model-output "? …")` echoes are
      replayed (`:MODEL-OUTPUT` step); 22 of the 63 `? …` refs now pass and are
      enabled. Frontier 67 → 89 specs.
  - [ ] Remaining `? …` refs need the command output itself: `(dm)`, `(sdm …)`,
        `(buffer-chunk …)`, `(whynot …)`, `(sgp …)`, `mp-show-queue`
        (`Events in the queue:`), and the `pcomp-issues` break/queue ones.
        Samples: `tutorial-unit1code-semantic-{buffers,buffer-specific,sdm,sdm2,
        whynot,goal-focus}`, `tutorial-unit1modeling-broken-addition-v*-goal-buffer`,
        `tutorial-unit5modeling-v6-p4-run-queue`,
        `tutorial-unit7modeling-pcomp-issues-v9-non-win-composition`.

## Phase C — engine features surfaced by B

- [ ] **C1.** Cluster the new mismatches by subsystem (vision / audio / DM /
      production-compilation / imaginal / temporal / …) and fix one cluster at a
      time; each cluster gets its own spec(s) in `enabled.txt`. Current
      runnable mismatches:
  - `hear-self` — speech module: missing the `SPEAK <text>` playback line and
    the subvocalize sound (`speech.lisp`), run stops early.
  - `imaginal-action` — the `imaginal-action` buffer (command layer):
    `+imaginal-action> … action "cmd" slots (count)`; parser rejects `slots (…)`.
  - `tutorial-unit5-fan-lawyer-store-trace` — `?visual-location> buffer
    unrequested` query not implemented.
  - `tutorial-unit3-sperling-experiment`, `tutorial-unit3modeling-v6-next-run`,
    `tutorial-unit4-paired-experiment` — task/experiment mismatches.

## Phase D — in-scope leftovers

- [ ] **D1.** `unit-1-together-1-mp` — multi-model trace (model column). Needs
      the engine to run several models and emit the model-column trace
      (`golden/test-models/specs/unit-1-together-1-mp.spec`).
- [ ] **D2.** `tutorial_unit2code_initial` — extraction error: the stub loads a
      missing `.lisp`. Fix the extractor or skip it.
- [ ] **D3.** `spacing-effect` — `:OTHER` (no run recorded); determine whether it
      is a real run worth speccing.

## Phase E — hygiene (ongoing)

- [ ] **E1.** Probe coverage dashboard: pass / fail / ignored-by-reason plus the
      re-triage candidate list, so coverage is one command away.
- [ ] **E2.** Keep `GOTCHAS.md` current as fixes land.
- [x] Engine split into `rust_actr7/src/engine/*.rs` (commit `3b5c476`).
- [x] `cargo fmt` the crate (commit `8494640`).

## Phase R — RPC engine (drop-in for the Python/other-language clients)

Goal: the vendored Python client (`actr7.x/tutorial/python/actr.py`) and the
other-language interfaces connect to the Rust engine over the ACT-R RPC
protocol and drive models the engine already loads — **no embedded Lisp**
(the bounded Rust parser only, extended as far as the tutorial needs).
Scope excludes client-defined modules (`define-module`).

Protocol facts: TCP `127.0.0.1:2650` (or `~/act-r-port-num.txt` /
`~/act-r-address.txt`); JSON messages terminated by `chr(4)`; each request is
`{"method","id","params"}` and each reply `{"id","result","error"}`; the work
is `evaluate <command-name> <args>` (a name→handler lookup). Client-registered
commands fire via a server→client `{"method":"evaluate","params":[name,model,
…]}` callback (`actr.py:297`, `dispatcher.lisp:1663`). Methods: `add`,
`remove`, `evaluate`, `monitor`, `remove-monitor`, `list-commands`,
`list-connections`, `check`, `set-name`, `close`.

- [x] **R1. Transport + protocol.** `src/json.rs` (dependency-free JSON),
      `src/rpc.rs` (framing, methods, envelopes, callback shape),
      `src/rpc/server.rs` (TCP listener + port/host files + the server→client
      `evaluate` callback), `src/bin/actr-server.rs`. Verified end-to-end
      against the vendored `actr.py`: it connects, `load_act_r_model`,
      `run`, `reset`, `goal-focus` all round-trip. (TCP only; the
      `p_interface` pipe variant is skipped when `start-pipe-connection` is
      not implemented.)
- [x] **R2. Command registry + core commands.** `name → handler`
      (built-in / task-defun / remote) routed through `evaluate`:
      `run`/`run-full-time`/`run-until-time`/`run-n-events`, `reset`/`reload`,
      `load-act-r-model`/`load-act-r-code`, `goal-focus`/`mod-focus`,
      `set-parameter-value`/`get-parameter-value`, `set-system-parameter-value`/
      `get-system-parameter-value`, `new-symbol`, `current-model`/
      `set-current-model`, `buffer-read`/`buffer-status`/`clear-buffer`/
      `set-buffer-chunk`, `chunk-p`/`chunk-copied-from`/`chunk-slot-value`,
      `act-r-random`/`permute-list`, `schedule-event[-relative|-now]`,
      `add-dm(-fct)`/`define-chunks(-fct)`, `record-history`,
      `predict-bold-response`, `mp-time`/`mp-time-ms`/`get-time`,
      `model-output`/`command-output`/`print-warning`/`act-r-output`,
      `release-chunk-spec-id`, and the protocol-level `add`/`remove`/`monitor`/
      `remove-monitor`/`check`/`set-name`. Verified through `actr.py`. Minor
      remainder (unused by the tutorial tests): `set-chunk-slot-value`/
      `copy-chunk`/`delete-chunk`, `schedule-set-buffer-chunk`/
      `schedule-mod-buffer-chunk`/`schedule-event-after-module`.
- [x] **R3. Output commands** (shared with Phase C1): `pprint-chunks`/
      `pprint-chunks-fct`, `dm` (newest-first), `sdm` (with a spec matcher),
      `buffer-chunk`, `buffer-status`, `sgp`/`spp`/`sdp` getters+setters.
      `pprint-a-chunk` now sorts slots by chunk-type order, skips NIL/ISA, and
      names a spec-built goal chunk `GOAL-CHUNK{n}`. Verified through
      `actr.py`: `tutorial_unit2code_{dm,sdm,buffer_status}` and
      `tutor-model-solution` match. Remaining:
  - `whynot` (production match details) — `tutorial_unit2code_whynot`.
  - `mp-show-queue` (`Events in the queue:`), `pp`, `sji`.
  - `tutorial_unit2code_buffer_chunk` still shows a stray `SEVEN ` line (a
    `!output!` leaking before the run) — investigate the `model-output` path.
- [ ] **R4. Device/vision/motor** as the tutorial needs: `open-exp-window`,
      `add-text`/`add-line`/`add-button-to-exp-window`, `modify-text-for-exp-
      window`, `remove-items-from-exp-window`, `clear-exp-window`,
      `install-device`, `process-events`, `add`/`modify`/`delete-visicon-
      features`, `print-visicon`, `new-tone-sound`/`new-word-sound`/
      `new-digit-sound`.
- [ ] **R5. Bounded Lisp subset → whole tutorial.** Extend `model.rs`/`eval.rs`
      to load every tutorial model **and task file** the Python tests exercise,
      driven by failures (macros, `let`, loops, `format`, string ops,
      `multiple-value-bind`, …). Bounded to the tutorial; `model-tester.cl`
      only if we want `tester.py` itself.
- [ ] **R6. Conformance harness.** End-to-end: start the Rust server, attach
      the vendored `actr.py`, run `test-models/*.py`, diff against the same
      `.ref`s. A manual harness (`/tmp/opencode/rpc_conf.py`) already drives
      the vendored client and compares; 8/22 Python test-models match
      (`addition`, `count`, `semantic`, `semantic-dynamic`,
      `tutorial_unit2code_{dm,sdm,buffer_status}`, `tutor-model-solution`).
      Seven more (`bst-*`, `fan-no-pm`, `paired-learning-dynamic`,
      `tutorial_unit5_onehit_*`) need the `tester.py` harness
      (`sys.stdout = tester.out_file`), and `simple-dynamic-model`/
      `simple-ppm-test` need R4 device commands. Wire into `cargo test` as an
      ignored integration test once the server starts/stops from the harness.

Ordering: R1 → R2 → R3 first (fastest demonstrable loop against `count.py`/
`semantic.py`), then R4/R5 per tutorial unit.

---

## Explicitly out of scope (revisit only on request)

- The 91 command-output specs and the remaining command-only specs (need the
  command layer).
- The 344 non-timestamped unit-test refs.

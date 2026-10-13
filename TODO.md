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
- **Command layer in scope (2026-10-12).** The goal is a drop-in RPC
  replacement: a Python ACT-R client connects and behaves identically to Lisp
  ACT-R, the only divergence being that arbitrary Lisp cannot be executed.
  The client-facing command surface *and* whatever the refs exercise are now
  in scope. The command inventory (`golden/command-inventory.txt`, regenerate
  with `golden/command-inventory.py`) is the work list.
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
  - [x] `whynot` (production match details) — `tutorial_unit2code_whynot`
        matches. Engine tracks the last conflict set and reports the first
        failing condition.
  - [ ] `mp-show-queue` (`Events in the queue:`), `pp`, `sji`.
  - [ ] `tutorial_unit2code_buffer_chunk` still shows a stray `SEVEN ` line (a
        `!output!` leaking before the run) — investigate the `model-output` path.
  - [ ] `simple-ppm-test`: the `:cst` instantiation block's `:UTILITY` value is
        `-0.174` vs the ref `-0.713` (a base-utility/`:egs`-noise nuance; the
        ppm penalty and `:U` now match, 82/82 lines).
  - [ ] `simple-dynamic-model`: 96/98. Fixed the render-side and
        application-side dynamic-slot handling (`=context =x`). Remaining: the
        `EXTEND-BUFFER-CHUNK IMAGINAL` trace line (`#\=` RHS mod that adds a
        slot) and the request-time reusable-buffer name (`IMAGINAL-CHUNK0`) so
        `buffer-chunk` names the chunk instead of `NIL`. Naming is entangled
        with the DM copy path (`engine.rs:2788`), so it needs care.
- [x] **R4. Device/vision/motor** (command surface wired). Added RPC handlers
      for `add`/`modify`/`delete`(all)-`visicon-features`, `print-visicon`,
      `new-tone-sound`, `new-word`/`new-digit-sound`, `pbreak`/`punbreak`,
      `correlation`/`mean-deviation`, and the window scaffolding
      (`open-exp-window`/`clear-exp-window`/`add-text-to-exp-window`/
      `install-device`/`process-events`). `correlation`/`mean-deviation` are
      exact (Pearson r / RMSD, printing `LABEL: x.xxx`) and unit-tested;
      `print-visicon` is an approximate placeholder (no oracle yet);
      `new-word`/`new-digit-sound` are no-ops. See `GOTCHAS.md` #196. The
      Python tests that need it all route through `tester.py`, so they still
      need R6b.
- [ ] **R5. Bounded Lisp subset → whole tutorial.** Extend `model.rs`/`eval.rs`
      to load every tutorial model **and task file** the Python tests exercise,
      driven by failures (macros, `let`, loops, `format`, string ops,
      `multiple-value-bind`, …). Bounded to the tutorial; `model-tester.cl`
      only if we want `tester.py` itself.
- [x] **R6. Conformance harness** (started, growing). `golden/rpc-conformance.py`
      drives the vendored `actr.py`, `golden/rpc-conformance.sh` starts/stops
      the server, `golden/rpc-enabled.txt` is the must-pass allowlist, and
      `tests/rpc_conformance.rs` runs it (`cargo test --test rpc_conformance --
      --ignored`). Currently asserts **9** Python test-models
      (`addition`, `count`, `semantic`, `semantic-dynamic`,
      `tutorial_unit2code_{dm,sdm,buffer_status,whynot}`, `tutor-model-solution`).
- [ ] **R6b. `tester.py` support** (plumbing done). `import tester` now works
      (`model-tester.cl` loads), the harness captures `tester.out_file`
      interleaved with the trace, and the engine gained the two runtime pieces
      the tester needs: `(sgp :seed …)` actually applies now, and
      `:testing-function` is dispatched as the `testing-module` reset hook
      (`set-testing-traces` / registered client commands). See `GOTCHAS.md`
      #198. Still failing: `bst-*`, `fan-no-pm`, `paired-learning-dynamic`,
      `tutorial_unit5_onehit_*`. Blocker = a **live** device/window (the R4
      `open-exp-window`/`add-text-to-exp-window`/… are stubs) plus exact
      per-run RNG draw alignment through model runs.
Ordering: R1 → R2 → R3 → R4 (done), then R6b for the tester-driven Python
test-models.

### R7. Full drop-in command surface (in scope 2026-10-12)

Goal: a Python ACT-R client can call **every** command it can reach and get
Lisp-identical output. Target surface = client-facing (`actr.py`, 123
commands) ∪ ref-exercised (86), minus implemented → **72 commands**
(`golden/command-inventory.txt`). Families: chunk-dm 7, production 9,
window 8, events 15, audio 6, modules 7, history 2, misc 32 (incl. 3 pipe).

- [ ] **M0. Inventory + harness.** Inventory done (`golden/command-inventory.py`
      + `.txt`). Harness done (`a7f9e50`): the extractor records driver
      commands generically (`(:command NAME ARG…)`) and the spec harness
      replays them through `Session::evaluate` (`run_spec_session`), so a spec
      exercises the same dispatch a client hits. The 122 command-scope specs
      were regenerated (only those — the enabled corpus is hand-tuned); 43 now
      execute, `:COMMAND` drops to 79 (source scrapes + Lisp-hook refs = the
      accepted divergence), and enabled grew 89 → **92**. The 43 new failures
      are the M1/M2 work list. Still: the coverage dashboard (E1).
- [ ] **M1. Implement the remaining commands, by family** (chunk-dm →
      production → window → audio → modules → history → events → misc), each
      enabled against its ref(s). Corrected gap: **57** (not 72) after
      excluding the Lisp-hook-only commands (`evt-*`/`format-event`/
      `add/delete-event-hook`), which no client reaches and no Lisp-free
      engine can replay.
      Done: `chunk-type` (`9f773c0`), `?buffer> buffer requested/unrequested`
      (`9f773c0`), visual-location `lowest`/`highest` request constraints
      (`4ce0ccf`), `?visual-location> state error` (`e6b6542`), the
      extractor now records `load-act-r-code` task files (`a6dffeb` — specs
      were silently missing the task's `defun`s), `whynot` now
      recomputes the conflict set (`Engine::pmatches`) and reports the
      failure reason in ACT-R's compiled condition order
      (`9384f4c`, `02e64cb`, GOTCHAS #203), the model-body forms
      `start-hand-at-mouse`/`-keypad`/`set-audloc-default`/
      `set-parameter-value` are accepted (removes 4 hard load errors), a
      scheduled command's trace module is its schedule-time `:module`
      (`1ecc980`, GOTCHAS #204), and reusable-buffer naming is correct:
      a production naming a buffer's variable makes it non-reusable, so
      `simple-dynamic-model` is `IMAGINAL-CHUNK0` while `act-touch`/
      `merging-test` are `CHUNK{n}` (`d0ee65b`, GOTCHAS #205), and a merge
      renames references to the merged buffer chunk (`9a413cd`, GOTCHAS #206),
      chunk slots print in model-wide slot order (`8436c08`, GOTCHAS #207),
      a non-numeric `:reward` clears the utility-learning history
      (`aab5f28`, GOTCHAS #208), `:er` permutes the whole conflict set with
      `permute-list` (one draw per candidate, even a lone one) and
      `find-best-feature` draws unless the location chunk's visicon entry is
      still live (`73b045f`, GOTCHAS #209/#210). Flips `simple-dynamic-model`,
      `tutorial-unit5modeling-v9-p7-8-run`, `tutorial-unit5-fan-lawyer-store-trace`,
      `tutorial-unit5modeling-v8-p6-run-retrieved-chunk`,
      `tutorial-unit7modeling-ul-issues-v2-first-run`,
      `tutorial-unit3modeling-v6-next-run` and `hear-self` (speech
      `:hear-self` gating, style-change prep, subvocalize, the device-notify
      line; `0e556df`, GOTCHAS #211/#212). Then `paired-learning-dynamic`:
      the composed-production `mappings` substitution (`7dfe8f8`, GOTCHAS #213)
      and the stub's starting-parameters seed (`1a3a4b7`, GOTCHAS #214) —
      **100 pass / 35 fail**.
      Current blockers from the probe: `print-conflict-tree` (2 specs),
      `print-chunk-activation-trace`/`print-activation-trace` (1),
      `print-visicon`'s table format (several), `!bind!` (`which-one`),
      motor/device work (`keypad-test`, `agi-modify-test`), and the audio
      sound-object gap.
      **The audio specs need sound objects, not just notifications**: a task
      that calls `new-tone-sound` outside a run (`unstuff-aural`, `siegler`,
      `audio-updates`) has its `new-sound` line recorded as a sidecar
      command, but not the sound itself, so the engine never emits the
      `set-buffer-chunk aural-location …` that follows at
      `onset + tone-detect-delay`. Recording `new-tone-sound` as a replayed
      step (`*generic-commands*` + regen) is the fix.
      **`test-ppm` cannot go green**: its first 122/163 lines now match
      byte-exact, but the third section sets `:PPM-HOOK SHOW-PPM-VALUES`, a
      stub-defined Lisp callback `actr.py` cannot supply (the documented
      "no arbitrary Lisp" divergence). Same for `testing-child-slot`, whose
      ref records a Lisp error. Neither is counted as an engine gap.
      **`adaptive-noise` is also out of scope** (task driver): the model has no
      productions, and the whole ref trace comes from `run-tests`'s periodic
      `module-request`/`merge-dm`/`erase-buffer` driver, which the extractor
      cannot record (in-run commands are dropped; symbol actions fail its
      `(stringp action)` guard) and the engine cannot replay. Only
      `adaptive-noise` + `base-level-inhibition` use `schedule-periodic-event`.
      **`print-visicon`'s table format is a poor spec target**: of the vision
      cluster, `removed-and-restored-visual-item`, `sperling`,
      `tutorial-unit3-sperling-experiment` and `creating-an-image` all
      `add-act-r-command`/`monitor-act-r-command` (client callbacks the golden
      harness cannot supply), so only `proc-display-with-pending` is
      callback-free — and it also needs the pending
      add/delete/modify sections plus `add-visicon-features` recording and
      regen. The table columns are the feature chunk's insertion order
      (`chunk-filled-slots-list-fct`), reversed, which is *not* the engine's
      `Feature::loc_slots()` order and differs per feature construction path.
- [ ] **M2. Trace fidelity** for the command-scope timestamped specs: `:cmdt`
      lines (`PRODUCTION-SELECTED`, `BUFFER-READ-ACTION`, `QUERY-BUFFER-ACTION`,
      `MODULE-REQUEST`, `MODULE-BUFFER`), `:v high` (`proc-display`,
      `visicon-update`, `Find-location`, `Move-attention`, `Encoding-complete`),
      module verbose output, utility-learning lines.
      Done: `EXTEND-BUFFER-CHUNK` (`ba4f292`, GOTCHAS #202);
      **`!output!` gating** — `RhsAction::Output` now goes through `out()`, so
      it is suppressed by `(:v nil)` like ACT-R's `model-output`
      (`framework/misc-utils.lisp:801`); flips the RPC model
      `tutorial_unit2code_buffer_chunk` (GOTCHAS #215).
      **Compiled-production substitution (`:pct`) — DONE for the `:ppm`-off
      path** (`7dfe8f8`, GOTCHAS #213). The composed production's printed form
      and the installed production (which the engine parses back from the
      rendered text) both
      come from substituting ACT-R's `mappings` into `p1-s`/`p2-s` **one
      mapping at a time** (`compose-productions`,
      `modules/production-compilation.lisp:1556-1619`), with a **car-rewrite**
      that cascades later entries (this is how `=OTHER` resolves to `NUMBER`).
      A variable that is bound but never mapped stays `=VAR` (so `POST =STATE`
      stays symbolic). `mappings` = the union of each used buffer's `MAP-*`
      function (`tools/goal-compilation.lisp:216` (9)/(8 9 12 13) branch for a
      GOAL RHS `=` firing against a GOAL LHS `=`; `tools/retrieval-compilation
      .lisp:71` for a RETRIEVAL `+` firing against a `=`), sorted by the
      `constant-value-p` keys `0`/`1`/`2` — **a chunk name is not a constant**.
      Ground truth for `paired-learning-dynamic`'s `PRODUCTION15`, in pop order:
      `=DEST→=OTHER`, `=RETRIEVAL→RETRIEVAL-CHUNK0`,
      `RETRIEVING-RESULT→RETRIEVING-RESULT`, `=VAL→"8"`, `=DEST→NUMBER`,
      `=STIMULUS→"xray"`, `=TARGET→WORD`. Engine gaps (all in `compose_pair`,
      `engine/productions.rs:1339`): the `map` port omits p1's request
      *slot-name* vars, p2's LHS *dynamic slot names* and the value lookup at
      the *instantiated* name, and the whole `MAP-GOAL-BUFFER` (9) branch;
      `render_compiled` (`:1296`) substitutes with **p2's bindings only**
      (so p1's vars stay symbolic, while a naive p1∪p2 merge wrongly resolves
      `=STATE`); `subst` never rewrites slot *names*; no car-rewrite.
      — all four now fixed by `Engine::compose_mappings` + the sequential
      apply with the car-rewrite; the `:ppm`-on path (where the map functions
      build their slot sets from partially-matched specs with extra
      `buffer-condition-union` instantiating) still uses the raw firings, so
      `ppm-compilation` is unaffected.
      Probe: patch `compose-productions` to print `mappings` and drive
      `(load-act-r-code "ACT-R:tutorial;lisp;paired.lisp")` +
      `(load-act-r-model "ACT-R:tutorial;unit8;paired-dynamic-model.lisp")` +
      `(paired-task 2 2)` (a plain `load` of `paired.lisp` leaves
      `do-experiment` undefined). Independent remaining causes in
      `paired-learning-dynamic`: the compiled-production `:utility` equation
      (15 lines) and retrieval-latency drift (`15.571`→vs→`15.570`, then
      `21.133` vs `20.748`). The drift is **not** the `exp` precision:
      matching `exp-coerced` (double `exp`, narrow to single before `round`)
      changed nothing, so it is a genuine activation difference that grows.
      **Localized to the RNG state, not draw count/order.** Instrumenting
      `Mt::noise` (engine) vs a wrapped `genrand_real2`/`act-r-noise` (ACT-R)
      shows both sides make the **same number** of noise draws (95) but the
      values differ from the **first** `act-r-noise` call — so the draw
      *events* align and the generator *state* does not. Two candidate causes
      were ruled out: the retrieval candidate order (switching to the
      `dm-chunks` signature-alist order, `dm_alist_order()`, changed nothing)
      and the latency `exp` precision (above). The cause was the **seed**: a
      stub's `(ssp :starting-parameters (:seed (1 0)))` is evaluated at load
      and re-applied by the central-parameters reset hook, so it is the seed
      the run used — but `w-ssp` only records `:drive`-mode calls and the stub
      is loaded in `:load` mode, so it was dropped from the spec; `:RUNS` kept
      the `(100 0)` placeholder and the engine seeded with it
      (`init_genrand(100)`). **Fixed** (`1a3a4b7`, GOTCHAS #214): `make-spec`
      prepends an `(:SGP :STARTING-PARAMETERS (:SEED (S C)))` step, and the
      harness no longer applies a per-run `(100 0)`. `paired-learning-dynamic`
      is byte-exact — the `:utility` lines were a symptom, not a separate
      compiled-production utility-equation bug.
- [ ] **M3. RPC 22/22** (the tester cluster: vision timing + RNG draw
      alignment).
      **Reusable-buffer naming — DONE** (`d0ee65b`, GOTCHAS #205). The
      earlier attempt regressed `merging-test`/`act-touch` because it named
      at set time without the real *non-reuse* rule: a production that
      names a buffer's variable (`=imaginal`) makes that buffer
      non-reusable (`buffer-requires-copies`,
      `support/production-parsing.lisp:482`). With that set computed at load
      (`Model::no_reuse_buffers`) and a modify storing a chunk-valued
      `=buffer` ref by name rather than copying it, `simple-dynamic-model`
      is green with no regressions.
       **Re-baselined 2026-10-13: 11/22** — `rpc-enabled.txt` was stale
       (9/22) and `simple-dynamic-model` / `simple-ppm-test` already conform.
       Swept all 22 with one process per model (the harness has no per-test
       timeout), against a live `actr-server`:
       - OK (11): addition, count, semantic, semantic-dynamic,
         simple-dynamic-model, simple-ppm-test, tutorial_unit2code_dm,
         tutorial_unit2code_sdm, tutorial_unit2code_buffer_status,
         tutorial_unit2code_whynot, tutor-model-solution.
       - FAIL (7): `bst-nolearn` (line 2/380/2484),
         `paired-learning-dynamic` (207/803), `tutorial_unit2code_buffer_chunk`
         (1/9), `tutorial_unit2code_initial` (2/65/64),
         `tutorial_unit5_onehit_blocks` (2/21/2),
         `tutorial_unit5_onehit_hands` (2/13/6),
         `tutorial_unit5_onehit_learning` (2/9952/2).
       - Blocked (4): `paired-learning` (hangs), and `bst-learn`,
         `bst-learn-ppm`, `fan-no-pm`, where the driver process exits mid-run
         (`bst-learn`'s `bst.experiment` calls `exit`) before any verdict.
         `rpc-conformance.sh --expect` now requires the `RPC-RESULT` line so
         that early exit cannot read as a false pass.
       - `paired-learning`/`paired-learning-dynamic`/`simple-dynamic-model`/
         `simple-ppm-test` are golden-PASS, so their gap is the *client
         command surface*, not engine semantics; `bst-*`/`fan-no-pm` are
         golden-FAIL too, so work there counts on both tracks.
       **Vision timing — half of M3 is done** (`0c9d5df`, GOTCHAS #216). The
       scripted timeline re-encodes the attended location on a clear via
       `pending_clears`; the client `clear-exp-window` did not, leaving
       `?visual> state free` true 85 ms (`:visual-attention-latency`) too
       early. `paired-learning-dynamic` now matches through line 222; its
       remaining 90 lines are **all** noise-driven `:utility` values plus 1 ms
       retrieval latencies — i.e. the other half: **RNG draw alignment**.
       Both paths seed `(1 0)` (so GOTCHAS #214's fix is not the issue here);
       the RPC driver's extra client calls must consume a different number of
       draws before the first utility computation. Next probe: instrument
       `Mt::noise` / `genrand_real2` on both paths and find the first
       index where the streams differ.
       **`tutorial_unit2code_initial` needs a second concurrent connection.**
       Its model deliberately opens a fresh connection with stdout redirected
       to capture "ACT-R connection has been started." (then stops it and
       restores the backup). The Rust server serves one connection at a time,
       so the second attempt times out and the model outputs
       "failed to connect to act-r with exception (timeout)". This is a server
       concurrency gap, not a model/output gap.
       **RNG draw alignment — one draw short (`paired-learning-dynamic`).**
       `3e23b26`/GOTCHAS #217 put the RPC run on ACT-R's stream (`:seed`
       `(1 0)` instead of unit4's `(200 4)`), and the diagnosis is now exact.
       Instrumenting `Mt::noise` and `Mt::random_int` (env `DBG_RNG`) and
       comparing the golden spec against the RPC model: the RPC run makes
       **the same 95 `noise` draws with value-for-value identical numbers**,
       merely shifted by one index. Before the run's first noise the golden
       path consumes 3 draws — `(:CONSUME-RNG 2)` (2 raw `next_u32`) plus one
       `random_int(1)` — while the RPC path consumes 2 (its `permute-list`,
       `random_int(2)` + `random_int(1)`). Both paths have the same
       `random_int(1)` per run (8 per execution), so it is the *ordering*
       relative to the first noise that differs, not the presence: the golden
       path takes that draw before `NOISE 1`, the RPC path after. Since the
       ref was generated by the same Python driver, the golden path's extra
       pre-noise draw is the one to explain — next probe: tag each
       `random_int` call site (the CR tie-break `productions.rs:2824`, the
       imaginal/action randomizes, `randomize_ms` in `eval.rs:296`) and log
       which one runs first in each path.
       Note the RPC device path applies its window ops *immediately* while the
       scripted timeline schedules them as `pending_*` waves (GOTCHAS #216),
       which is the likely source of the reordering.
- Known divergence: the pipe transport and `define-module`/`undefine-module`
  (client-defined modules) are covered by "no arbitrary Lisp" unless needed.

---

## Explicitly out of scope (revisit only on request)

- The 344 non-timestamped unit-test refs **that only scrape the tutorial
  source files** (194 of the 332; the stub `with-open-file`/`read-line`s a
  model file and `model-output`s a fragment). They validate the tutorial
  text, not the engine — no client command reaches them. The remaining ~138
  are real command-output tests and are in scope (Phase R7).

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
       **Declarative activation-trace narration — DONE** (GOTCHAS #232/#233).
       `partial-matching` narrates nil-valued `-` slots and always prints
       `effective similarity value is`; the engine skipped nil slots and dropped
       that line in the `sim ≠ ms` branch. Both `tutorial-unit5modeling-v6-p5-run-with-mp-2`
       and `-v8-p6-run` were strict subsequences of their refs (pure
       insertions), so the fix could not move a number. `:nsji` is now
       three-valued (`t`/`warn`/`nil`) and the negative-Sji `model-warning` is
       gated on `:model-warnings`, which the golden environment leaves nil.
       **Frontier 101 → 103**; both specs enabled.
       **Extractor audit (GOTCHAS #236).** The recurring "the task's setup
       vanished" failures were an incomplete wrapper list, not nesting. Audit
       recipe + the 97-symbol result are in GOTCHAS #236; the actionable part is
       a short list of task-level state changes. Recording one is a
       `("name" . function)` pair in `*generic-commands*` when the engine already
       has the command — `add-dm`, `set-buffer-chunk`, `clear-buffer`,
       `schedule-event-now`, `schedule-{mod,set}-buffer-chunk`,
       `add/delete/modify-visicon-features`, `permute-list`, `correlation`,
       `mean-deviation` are now recorded (and `base-level-safety-check`,
       `proc-display-with-pending`, `removed-and-restored-visual-item`
       regenerated), which is the precondition for the specs below but flips
       none of them yet.
       Still needs an engine command first: `module-request`, `merge-dm`,
       `delete-production`, `buffer-slot-value`, `new-ongoing-sound`,
       `end-ongoing-sound`, `new-digit-sound`, `pbreak`.
       **`proc-display-with-pending` is DONE and enabled** (golden 104/135):
       `printed-visicon` (the table, its column order, and the pending
       additions/deletions/modifications) plus the vision proc-display lock
       that makes the pending sections meaningful (GOTCHAS #238-#241).
       `removed-and-restored-visual-item` was also carrying the leak-polluted
       `ADD-VISICON-FEATURES (ISA (VISUAL-LOCATION TEXT) …)` steps from the
       earlier extractor commit and has been regenerated: 830 → 114 lines and
       its tables now match. What is left there is behavioural, not a renderer
       gap — the ref's restore produces a *new* `VISUAL-LOCATION1` where ours
       reuses `VISUAL-LOCATION0`, and the ref has events ours stops before
       (`encoding-complete`, `no visual-object found`, a second `visicon-update`
       at 0.845). The lock is modelled only for `warn-vision`'s pair; other
       ACT-R lock/unlock pairs (`find-location`, `stuff-visloc-buffer`,
       `update-tracking-mth`) are not, so a model that locks without a later
       `+visual>` request stays blocked — worth checking if a vision spec runs
       to the event limit.
       **`removed-and-restored-visual-item` is DONE and enabled** (golden
       106/135). Its one real divergence was line 70: the object's `screen-pos`
       printed `visual-location0-1` where the ref has `-0`. The auto-stuffed
       `-0` and the `find-location` `-1` are identical chunks, so on the
       intervening `clear-buffer` the second merges into the first in DM and
       every reference is remapped; `set-chunk-slot-value-fct` stores the
       *true* name, so the object prints `-0` while the encoding trace still
       names the marker `-1`. `encoding_complete` now resolves the alias
       (GOTCHAS #257). Copies with differing slots (tracking) never merge, so
       `tracking-test` is untouched.
       **The `fan` family was blocked on an extractor gap, not the engine**
       (GOTCHAS #234/#235). Tasks drive a trial with
       `(mod-focus-fct (arg1 x arg2 y))` (`tutorial/lisp/fan-no-pm.lisp:57`) and
       the extractor never recorded it, so the goal's probe slots stayed unset,
       no production matched, and every run ended at t=0. Recording it (plus the
       RPC `mod-focus` handler, which was listed but had no arm — the
       `tutorial_unit5_onehit_*` models errored on it) took `fan-no-pm` from 191
       to **1830 of 1844** lines, first divergence line 2 → 1830. Two more traps
       found on the way: `:STARTING-PARAMETERS` carries the reset hook
       (`trace-fan-run` sets `:act t`) and the harness was applying it straight
       to the engine, and `run_spec` only uses the session driver for specs with
       a `:COMMAND` step — both now fixed.
       **`fan-no-pm`'s remaining 14 lines are the task's `fan-experiment`
       summary table** (`correlation:`/`mean deviation:` plus the targets/foils
       grid), which is `format` output the spec cannot replay — so it is the
       same accepted divergence as the other task-hook refs, not engine work.
       `fan`/`fan-w-tree` additionally need the `break` mechanism
       (`break-fan-run` → `pbreak encode-person`), which prints the production
       text and pauses the run.
       Next in this family: `base-level-safety-check` (also a strict
       subsequence — needs the retrieval-failure render path, the leading
       `--------------------------` header, and `No chunk above the retrieval
       threshold`), and `fan-saved-act` (needs the
       `print-chunk-activation-trace` command).
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
      **Audio cluster — narrower than it looked (2026-10-13).** The three audio
      specs do *not* share one mechanism (GOTCHAS #227/#228):
      - `unstuff-aural` — **DONE, enabled** (golden 101). The replay + units
        fixes took it 5 → 19 → 248 → 448 of 448 byte-exact. The last piece was
        the unstuff half of `stuff-sound-buffer` (`audio.lisp:1056`): armed at
        each stuff (`offset + :decay-time` for `t`, a delay for a number), its
        precondition requires an unmodified copy like the stuff test, and
        `:decay-time` is 3000 ms. Two ACT-R probes of the block settled both —
        guessing `decay-time` from the ref's first erase (5000) put every
        later erase in the wrong block, and reading the precondition as
        `id`-only did the same in the other direction. GOTCHAS #227/#228.
      - `audio-updates` — also creates `new-ongoing-sound`/`end-ongoing-sound`,
        which the engine does not implement; its tones alone cannot finish it.
      - `siegler` — **does not call `new-tone-sound` at all**. Its sounds come
        from another path (its stub installs a speech monitor), and the ref's
        `AUDIO new-sound` plus a 300 ms aural-location detection looks like the
        speech/`new-other-sound` route (`hear-self`'s shape). Before counting it
        as engine work, confirm which call creates its sounds: if it is a Lisp
        task hook the engine cannot replay, it belongs in the
        documented-divergence list, not the work list.
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
       **Re-baselined 2026-10-13: 9/22 → 14/22** — `rpc-enabled.txt` was stale
       at the start (11/22 already conformed), then
       `tutorial_unit2code_buffer_chunk` (`!output!` gating, #215),
       `paired-learning-dynamic` (#216/#217/#218) and `paired-learning`
       (#219/#220). **Both paired models pass, so the paired cluster is
       closed.**
       **`bst-learn`/`bst-learn-ppm` — the driver runs, the model stops at
       t=0.** GOTCHAS #221 fixed the `spp`/`no-output` crash that made them
       vanish from the sweep; GOTCHAS #222 (centring the button label) then
       fixed the placed location, so all three `bst-*` specs now diverge at
       line 4 rather than line 2 — the first two trace lines match.
       Remaining: the engine emits one `visicon-update` per queued
       `ProcDisplay` (9 at t=0 for the 8-item display + the clear) where the
       ref has one. Coalescing same-time displays was tried and **reverted**:
       routing it through `debug.rs` wholesale dropped the golden suite to
       98/100 (the same `Event::ProcDisplay` line is used by the visicon API
       `add-visicon`/`clear-visicon`, which the device-timeline specs rely on).
       A narrower version — coalesce only the window helpers, and only while no
       display has executed at that time — is the next thing to try.
       **Device buttons/lines — DONE; `bst-*` now blocks on the motor module**
       (`f2a472f` + `6cf13f9`, GOTCHAS #223/#224). The extractor records
       `add-button`/`add-line`, the sidecar carries them, the harness parses
       them (16 buttons / 22 lines for `bst-nolearn`), and the engine applies
       due ops through the same helpers the RPC path uses, merged by `at_ms`.
       The wave did not run because `chunk-type` rebuilds the engine for its
       in-place definition change and a fresh engine arms device **run 0** — so
       the `(RESET) (CHUNK-TYPE …) (RUN 60 …)` replay wiped the display the
       reset had just armed, and the run ended after the initial goal + one
       conflict-resolution. `Session::rebuild_engine_in_place` now carries the
       run across the rebuild.
       All four `bst-*` specs build the display and run into the model's first
       manual action. The next blocker was the **motor request slot**
       `(:loc <visual-location>)` (`not implemented: manual request slot
       ("LOC", …)`), which `keypad-test` (`("HAND", RIGHT)`) shares — fixed by
       ungating the built-in manual commands (GOTCHAS #225/#226), after which
       `bst-nolearn` matches the ref for 81 lines.
       **The blocker is now the button feature** (`827480d` + this session,
       GOTCHAS #229). The productions ask for the buttons as
       `kind oval, value "b"` / `value "reset"`, but the engine builds a
       `kind BUTTON` box whose value is the window-item key, plus a separate
       `kind TEXT` label — so no request can match, the model takes
       `find-loc-failure`, and the run ends. The `!eval!`/`=over`/`=under`
       theory was **wrong**: instrumenting the LHS shows the evaluation and the
       selection are correct (`(< 97 (- 62 25))` → NIL, `(< 62 (- 97 25))` → T,
       `DECIDE-UNDER` wins the equal-utility tie and is fired). Next: probe
       ACT-R for the button feature's real `kind`/`value`/case (the GOTCHAS
       #227/#228 technique), build it that way, re-diff.
       **Trace bookkeeping — SETTLED** (GOTCHAS #231). The trace's
       `decide-over`/`DECIDE-UNDER` puzzle was a rebuild bug, not model
       semantics: `chunk-type` rebuilds the engine in place and a fresh engine
       clears `out_lines`, so every trial's trace was wiped before its run and
       only the last survived. `rebuild_engine_in_place` now carries the trace
       across; it accumulates 150 → 297 → 447 → 594 lines and the first
       divergence moves 82 → 97.
       **Next for `bst-*`: the mouse movement's duration.** At 1.375 both fire
       `move-cursor`; ours frees the manual module at 1.625 (250 ms) and the ref
       at 1.926 (551 ms), so `click-mouse` fires 300 ms early and everything
       after drifts. `zz_mouse_request` uses the extended-motor-actions
       feature-prep model (4 features × 50 ms); ACT-R's `move-cursor` goes
       through the mouse device and its Fitts/minjerk time. Ours also emits
       `MOUSE CLICK-MOUSE BST (0 0 1)` / `release-mouse` device lines the ref
       does not have.

       **Resolved the recipe (verified by hand, not yet landed).** The ref's
       phases are prep(200) → init(50) → exec(201) → finish(50), free at
       finish. `move-cursor`'s exec is `fitts(coeff, r, w)` with
       `coeff = cursor-fitts-coeff` (0.1), `r` and `w` both in *degrees of
       visual angle*: `r = pm-pixels-to-angle(pixel-distance)`
       (`device-interface.lisp:743`, default dist `ppi*viewing-distance` =
       72×15 = 1080), and `w = approach-width(loc)` (`vision.lisp:3717`) —
       the target's width/height seen from the approach angle
       `atan2(dy,dx)`, then converted the same way. **With the cursor's start
       at the window origin `(300,300)`** (not `(0,0)`), button C at
       `(325,385)` gives `r/w = 3.539`, `fit = 201 ms`, and the trace lands
       byte-exactly: notify 1.826, finish 1.876, `click-mouse` 1.926.
       A prototype (cursor start = `window_origin`, this geometry, and the
       `MOUSE` line suppressed) matched bst through line 105 (was 96) and
       moved the first divergence to the visicon feature *numbering*
       (`set-buffer-chunk visual-location visual-location12` vs
       `…-location2` at 2.136) — a separate vision issue.

       **DONE: `bst-nolearn` passes byte-exactly (2484/2484) and is enabled
       (suite green 105/105).** The first pass (below) fixed the mouse/vision
       semantics and the two regressions it introduced; a second pass added
       the task's dynamic window (`MODIFY-LINE`/`REMOVE-LINE` sidecar ops),
       case-insensitive string slot matching, and the reward/goal-set ordering
       (GOTCHAS #247–#255).

       Changes made (all verified against the refs / an SBCL probe):

       1. **Mouse Fitts geometry** (`motor.rs`, `events.rs`): `move-cursor`'s
          exec is `fitts(cursor-fitts-coeff, r, w)` with both `r` and `w` in
          degrees of visual angle — `r = pm-pixels-to-angle(pixel-dist)`
          (default dist `ppi*viewing-distance` = 1080) and
          `w = pm-pixels-to-angle(approach-width(w,h,atan2(dy,dx)))`
          (`vision.lisp:3717`), falling back to the vision module's
          `default-width` (`:default-target-width`, 1.0) when the target has
          no width/height. Computed at request time (`zz_mouse_request`) and
          carried on `Event::ZzMousePrep.fit_ms`. SBCL probe
          (`/tmp/opencode/bst_probe.lisp`) confirms `d=4.6978 w=1.3271 →
          0.20143` for bst's first move.
       2. **Cursor starts at the window origin** (`engine.rs`): SBCL probe
          (`/tmp/opencode/bst_probe2.lisp`) shows the first `move-cursor`
          reads `start=(300 300 1080) target=(325 385 1080)`. `zz_mouse_pos`
          is initialised from `window_origin`.
       3. **Window origin from the model body** (`model.rs`, `engine.rs`):
          `(install-device (open-exp-window … :x X :y Y))` is parsed into
          `Model::window_origin` (scan for the keywords, since the window
          name is the first argument). Default stays `(300,300)`. This makes
          `zz-extended-motor-actions`'s window `(0,0)` (so its `LOC1` move is
          ~100 ms) while bst's stays `(300,300)`.
       4. **`MOUSE` trace lines gated on `show`** (`events.rs`): the refs
          trace `MOUSE` only at `:trace-detail high` (only
          `zz-extended-motor-actions` has them).
       5. **Manual buffer held through the movement** (`events.rs`):
          `ZzMousePrep` sets `man_exec_busy` (GOTCHAS #246).
       6. **Time-based `new`** (`engine.rs`, `perception.rs`):
          `checknew` (`vision.lisp:2227`) clears `new` once
          `tstamp-elapsed > :visual-onset-span` (default **500 ms**,
          confirmed by probe). Added `Config::onset_span_ms` and
          `Engine::feature_is_new`; used by `newest_feature` and the
          `:attended new` query.
       7. **Encoded object type = the feature's kind** (`perception.rs`,
          `productions.rs`): the object's `isa` is `(visual-object <kind>)`,
          so `convert_loc_to_object`/the encode path store `LINE`/`OVAL`/
          `TEXT`, and `isa_matches` treats `visual-object` as a supertype.
          Needed because `bst`'s `choose-c` tests `=visual> isa line`.
       8. **Relational test with a variable RHS** (`productions.rs`):
          `<= width =difference` binds the RHS from the bindings
          (`rel_test_v`) instead of always failing. Needed by `choose-c`.
       9. **No-op `move-cursor` aborts** (`motor.rs`): when the cursor is
          already at the target, emit the `MOVE-CURSOR LOC …` line but run
          no ply (`motor.lisp:1316`), so the manual module stays free.

       ### bst-learn (in progress, prefix 392/8872) / bst-learn-ppm (69)

       `bst-learn`'s sidecar now has the full per-trial `update-current-line`
       redraws (derived from the ref by a throwaway script: per trial, pair
       the ref's click-triggered `production-fired` choices with its
       `visicon-update` times and replay the task rule — a click adds or
       subtracts its `:action` length, and `update-current-line` adds /
       modifies / removes the line at (75,135)). Its common prefix is now 392
       (was 157). The run-boundary fix (GOTCHAS #256) and a `(CLEAR ms)` at
       each trial start (a new window, which re-encodes the gone attended
       object) landed with it.

       **Next blocker (bst-learn, line 393):** the trial-start auto-stuff
       picks a feature number one lower than the ref (`visual-location18` vs
       `19`). Only the *trial-start* auto-stuffs differ; the within-trial
       `find-location` picks and everything else match. So the ref's new
       window has one more leftmost-eligible feature (or a different
       `random-item` draw) than ours. Investigate the visicon feature count
       and the `random-item` RNG draw at a `build-display`.

       **bst-learn-ppm / -w-tree** diverge much earlier (line 69): after
       `encode-line-goal` the ref fires `decide-under` at 1.055 where ours
       waits for the next window wave. That is a model/strategy difference,
       not the device timeline — a separate investigation.
       - GOTCHAS #247–#256 cover the landed bst/motor/vision/utility
         semantics.
       Diagnostic note that cost a session: **check which run step the spec
       uses before blaming the scheduler.** `(:RUN 60 …)` → `run_secs`, whose
       queue-draining terminator ("no events left to process") is *correct*;
       only `:RUN-FULL-TIME` pushes a hidden `Dummy` at the limit ("time limit
       reached"). The earlier "the Dummy should make this impossible" reasoning
       was based on the wrong step.
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
       - Blocked (now 3): `bst-learn`,
         `bst-learn-ppm`, `fan-no-pm`, where the driver process exits mid-run
         (`bst-learn`'s `bst.experiment` calls `exit`) before any verdict.
         `rpc-conformance.sh --expect` now requires the `RPC-RESULT` line so
         that early exit cannot read as a false pass.
       - `paired-learning`/`paired-learning-dynamic`/`simple-dynamic-model`/
         `simple-ppm-test` are golden-PASS, so their gap is the *client
         command surface*, not engine semantics; `bst-*`/`fan-no-pm` are
         golden-FAIL too, so work there counts on both tracks.
       **Vision timing — DONE** (`0c9d5df`, GOTCHAS #216). The
       scripted timeline re-encodes the attended location on a clear via
       `pending_clears`; the client `clear-exp-window` did not, leaving
       `?visual> state free` true 85 ms (`:visual-attention-latency`) too
       early. That moved `paired-learning-dynamic`'s first divergence from
       line 207 to 223, whose remaining lines were all noise-driven (`:utility`
       values plus 1 ms retrieval latencies) — the RNG half, now also fixed
       (below).
       **`tutorial_unit2code_initial` needs a second concurrent connection.**
       Its model deliberately opens a fresh connection with stdout redirected
       to capture "ACT-R connection has been started." (then stops it and
       restores the backup). The Rust server serves one connection at a time,
       so the second attempt times out and the model outputs
       "failed to connect to act-r with exception (timeout)". This is a server
       concurrency gap, not a model/output gap.
       **RNG draw alignment — DONE** (`paired-learning-dynamic` passes;
       `3e23b26`/GOTCHAS #217, GOTCHAS #218). Three layers, each of which
       masked the next:
       - the run used `init_genrand(200)` (unit4's `(sgp :seed (200 4))`)
         instead of the client's `:seed (1 0)`, because `set-system-parameter-value`
         settings were dropped on a model reload (#217);
       - the client's `:starting-parameters` seed then put the run on ACT-R's
         stream, but the whole 95-draw `noise` sequence matched the ref only
         *shifted by one index*;
       - the missing draw was `permute-list`: ACT-R draws once per pass
         *including* the final single-element pass (n draws), the RPC handler
         used Fisher-Yates (n-1) (#218).
       Method that settled it: tag every `random_int` call site under
       `DBG_RNG` (`PERMUTE`, `RNG_DRAW`, `TIE`, `RANDOMIZE_MS`, `AR_RANDOM_RPC`)
       and diff the tag sequences of the golden spec against the RPC model —
       the tag names the algorithm, so an n vs n-1 difference is visible at a
       glance. Instrumenting `Mt::noise` (not raw `next_u32`, whose log is
       dominated by `set_seed` preludes) is the reliable stream.
       Remaining vision-adjacent note: the RPC device path still applies window
       ops *immediately* while the scripted timeline schedules them as
       `pending_*` waves (GOTCHAS #216). That did not matter here, but it is
       the thing to suspect if a future vision model is off by one event.
- Known divergence: the pipe transport and `define-module`/`undefine-module`
  (client-defined modules) are covered by "no arbitrary Lisp" unless needed.

---

## Explicitly out of scope (revisit only on request)

- The 344 non-timestamped unit-test refs **that only scrape the tutorial
  source files** (194 of the 332; the stub `with-open-file`/`read-line`s a
  model file and `model-output`s a fragment). They validate the tutorial
  text, not the engine — no client command reaches them. The remaining ~138
  are real command-output tests and are in scope (Phase R7).

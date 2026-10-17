# Changelog

Development history of the Rust port of the ACT-R 7 single-threaded kernel.

## Status

- **Golden harness** (`tests/extra_traces.rs`) — green.
- **S-expression reader + `define-model` loader** ([`sexpr`], [`model`]).
- **Discrete-event engine** ([`engine`]) — scheduler, buffers,
  procedural match/select/fire, exact-match declarative retrieval.
  First 3 goldens reproduce byte-exact.
- **Randomness & subsymbolic noise** ([`rand`]) — Lisp-identical MT
  stream, activation noise (`:ans`), utility noise (`:egs`), `:er` ties.
  Two synthetic noisy goldens reproduce byte-exact, including failures
  and non-50 ms latencies. Still out: `:bll`/`:mas`/`:mp` (activation
  math), `:ul` (utility learning), imaginal/temporal/devices.
- **Base-level learning** — reference histories via merge-on-clear,
  decay equation wired into retrieval, `bll-count` golden byte-exact.
  Remaining subsymbolics: `:mas` spreading, `:mp` partial matching, `:ul`.
- **Device buffers** — scripted window device, visicon + onset stuffing,
  visual-location find, move-attention + encoding, manual press-key
  pipeline (prep/init/output/finish), implicit `?buffer> state free`
  queries. `attend` golden byte-exact.
- **Imaginal buffer** — `+visual> clear`, digit keys + prep differences,
  VL SET-copy naming. `paired-static` golden (failed recall → study →
  learned recall → keypress → idle stop) byte-exact.
- **Multi-trial paired** — `paired-2x2` (2 pairs × 2 blocks, fixed 60 s
  schedule): harvest-named pair chunks, hand-change prep differences,
  long-run RNG/learning stability. Byte-exact.
- **Full paired task** — `paired-full` (2 pairs, 5 s prompt + study
  segments with window clears, single 20 s run): CLEAR/END sidecar
  actions, maintenance event classes, time-limit tail. Byte-exact.
- **Partial matching** — signature candidacy, similarity scoring with
  var-skipping, `mp-test` golden (similarity decides between two
  non-matching candidates). Byte-exact.
- **Utility learning** — selection history, reward propagation with
  recency discount, `ul-test` penalty-flip golden. Byte-exact (`:iu`
  honored; `:ult` trace lines not implemented).
- **ACT-R regression corpus** (`actr7.x/test-models/`) — a second oracle.
  `fixtures/regen-specs.sh` extracts runner specs; `tests/test_models.rs`
  diffs engine output against `.ref` with ACT-R's comparator. Work landed
  against it:
  - Per-spec trace module width (`max-module-name-length` overrides),
    integer-vs-float slot printing, and a scripted driver
    (`run_model_steps`: repeated `run`, `goal-focus`,
    `:v`/`:trace-detail`/`:seed`, `(reset)`), which made the multi-run
    `semantic` trace green.
  - `define-chunks`, model-body
    `declare-buffer-usage`/`spp`/`set-base-levels`/`set-similarities`,
    `?buffer> buffer empty|full`, `+goal>` requests, `-` request slots,
    spreading (`:mas`, with DM fan tables), and signature-based
    partial-matching candidacy — the unit5 modeling cluster now runs;
    `:act` activation traces plus request-spec `=buffer` chunk copies
    made `sji-merge` green; `:crt` conflict-resolution text and unnamed
    `add-dm` auto-naming made `buffer-queries` green; and LHS variable
    slot names, `slot =var` requiring filled, named-merge renaming, and
    type-free `isa` matching made `semantic-dynamic` green.
  - Chunk-valued references + merge aliasing and `!stop!` made
    `merging-test` green; the reference-count base-level fix, retrieval
    finsts (`:recently-retrieved`), and symbol similarities made the
    unit5 `v4-p3-run`/`v6-run-p3` traces green.
  - `require-extra` recognized/recorded and chunk-type default slots
    applied; the `blending` extra ported: first-class inequality request
    constraints (`> < >= <=`), the `BLENDING` buffer/module with
    `state free/busy` and `error nil` queries, and `start-blending`
    (signature/inequality candidacy in DM-alist order, softmax
    probabilities at temperature `:tmp`/`√2·:ans`, numeric weighted
    averages, similarity-based chunk values, log-sum-exp blended
    activation, and the `:blt` trace) — making `blending` green.
  - The `:cst` conflict-set trace (instantiated `print-production-text`
    plus production parameters) made
    `tutorial-unit8-simple-ppm-model-initial-run` green. Production
    compilation (`:ppm` procedural partial matching, the
    `goal-focus-fct`/`(car (define-chunks …))` model form, and the
    `:epl`/`:pct` composition/trace for the goal test+modify case) made
    `ppm-compilation` green. Runtime `:sgp` driver steps plus
    partial-match annotations/utility in the `:cst` trace made the
    `simple-ppm` family green (`simple-ppm-test`, `…-ppm-run`,
    `…-medium-chosen-run`).
  - `*buffer>` modification requests and indirect-action validity
    (`:cia`) cover run 1 of `indirect-action-compilation`; indirect-action
    composition + installing compiled productions into the runtime made
    `indirect-action-compilation` green. A bounded `!eval!`/`!bind!`
    evaluator (pure operators + `safe-` variants; unknown heads error)
    made `dynamic-slot-compilation` green. A bounded task-file `defun`
    interpreter plus the sole-`isa` matching rule made `bind-test` and
    `remote-bind-test` green. A `:ACT` `:sgp` arm enabled the unit5-v3
    activation trace (`tutorial-unit5modeling-v3-first-run`).
  - A scripted device window for the test-model corpus (an `ADD-TEXT`
    sidecar, `:attended`/`nil` request-parameter matching, the QWERTY key
    table, and a `clear_on_keypress` behavior that re-encodes the vanished
    attended object) made `demo2` green. Model-body `schedule-event` plus
    a bounded task-command dispatch (`add-act-r-command` aliases,
    `goal-focus`/`set-buffer-chunk`/`schedule-event-relative`/`act-r-random`),
    `declare-buffer-usage`, `(spp :reward R)` propagation and its `:ult`
    update trace made the `ul-issues` cluster green (fixing the extractor's
    stray runtime `:goal-focus` steps and a `run_secs` event-drop bug).
  - The `TEMPORAL` module (tick/noise dynamics, `isa time`/`isa clear`
    requests, `:record-ticks`, `delete-event` via heap retain) and
    `(spp :at S)` action-time overrides made `test-temporal` green.
    `:randomize-time` (`:vpft`/`:vidt`) jitter for production, vision,
    imaginal and motor latencies made `demo2-randomized` green.
  - The full `:cia`/`:rir` production-compilation engine (composed
    productions instantiate indirect-action variables, expand indirect
    requests to chunk specs, retain indirect retrievals under `:rir`, and
    handle `!bind!` composition conditions) made
    `retain-indirect-retrievals` green across all three runs. The
    partial-matching `:full` activation trace, finst removal trace,
    retrieval-threshold messages, `buffer empty` vs failure, and
    `!eval!` task-command dispatch made `grouped` green.
  - Remaining: task-supplied commands (`grouped`), production validation
    (`test-bad-productions`, blocked by cross-test warning state),
    `!eval!` timing (`dynamic-real-time`), other `require-extra`
    behaviors (associative-learning/act-touch/z-threads/extended-motor),
    `schedule-event` scaffolding, and multi-model traces.
- **Next**: audio/speech devices, production compilation.

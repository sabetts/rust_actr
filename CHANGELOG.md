# Changelog

Milestone history of the Rust port of the ACT-R 7 single-threaded kernel.

## Status

- M1: golden harness (`tests/golden.rs`) — green.
- M2a (done): S-expression reader ([`sexpr`]) + `define-model` loader
  ([`model`]).
- M2b (done): discrete-event [`engine`] — scheduler, buffers,
  procedural match/select/fire, exact-match declarative retrieval.
  First 3 M0 goldens reproduce byte-exact.
- M2c (done): Lisp-identical MT stream ([`rand`]), activation noise
  (`:ans`), utility noise (`:egs`), `:er` ties. Two synthetic noisy
  goldens reproduce byte-exact, including failures and non-50 ms
  latencies. Still out: `:bll`/`:mas`/`:mp` (activation math),
  `:ul` (utility learning), imaginal/temporal/devices.
- M4 (done): base-level learning — reference histories via
  merge-on-clear, decay equation wired into retrieval, `bll-count`
  golden byte-exact. Remaining subsymbolics: `:mas` spreading,
  `:mp` partial matching, `:ul`.
- M5a (done): device buffers — scripted window device, visicon +
  onset stuffing, visual-location find, move-attention + encoding,
  manual press-key pipeline (prep/init/output/finish), implicit
  `?buffer> state free` queries. `attend` golden byte-exact.
- M5b (done): imaginal buffer, `+visual> clear`, digit keys +
  prep differences, VL SET-copy naming. `paired-static` golden
  (failed recall → study → learned recall → keypress → idle stop)
  byte-exact.
- M6 (done): multi-trial paired (`paired-2x2`, 2 pairs × 2 blocks,
  fixed 60 s schedule): harvest-named pair chunks, hand-change prep
  differences, long-run RNG/learning stability. Byte-exact.
- M7 (done): full paired task (`paired-full`, 2 pairs, 5 s prompt +
  study segments with window clears, single 20 s run): CLEAR/END
  sidecar actions, maintenance event classes, time-limit tail.
  Byte-exact.
- M8 (done): partial matching — signature candidacy, similarity
  scoring with var-skipping, `mp-test` golden (similarity decides
  between two non-matching candidates). Byte-exact.
- M9 (done): utility learning — selection history, reward
  propagation with recency discount, `ul-test` penalty-flip golden.
  Byte-exact (`:iu` honored; `:ult` trace lines not implemented).
- M10 (in progress): ACT-R's own regression corpus
  (`actr7.x/test-models/`) is integrated as a second oracle:
  `golden/regen-specs.sh` extracts runner specs, `tests/test_models.rs`
  diffs engine output against `.ref` with ACT-R's comparator, and
  `golden/test-models/enabled.txt` is the green allowlist (38 specs so
  far). This milestone added per-spec trace module width
  (`max-module-name-length` overrides), integer-vs-float slot printing,
  and a scripted driver (`run_model_steps`: repeated `run`, `goal-focus`,
  `:v`/`:trace-detail`/`:seed`, `(reset)`), which made the multi-run
  `semantic` trace green. Added `define-chunks`, model-body
  `declare-buffer-usage`/`spp`/`set-base-levels`/`set-similarities`,
  `?buffer> buffer empty|full`, `+goal>` requests, `-` request slots, and
  spreading (`:mas`, with DM fan tables) and signature-based
  partial-matching candidacy — the unit5 modeling cluster now runs
  (5 specs green), and `:act` activation traces plus request-spec
  `=buffer` chunk copies made `sji-merge` green; `:crt`
  conflict-resolution text and unnamed `add-dm` auto-naming made
  `buffer-queries` green; and LHS variable slot names, `slot =var`
  requiring filled, named-merge renaming, and type-free `isa` matching
  made `semantic-dynamic` green. Remaining: `!bind!`/`!eval!`,
  `require-extra`, `schedule-event` scaffolding, and multi-model traces.
  Chunk-valued references + merge aliasing and `!stop!` made
  `merging-test` green; the reference-count base-level fix, retrieval
  finsts (`:recently-retrieved`), and symbol similarities made the
  unit5 `v4-p3-run`/`v6-run-p3` traces green. `require-extra` is now
  recognized/recorded and chunk-type default slots are applied, and the
  `blending` extra is ported: first-class inequality request constraints
  (`> < >= <=`), the `BLENDING` buffer/module with its `state free/busy`
  and `error nil` queries, and `start-blending` (signature/inequality
  candidacy in DM-alist order, softmax probabilities at temperature
  `:tmp`/`√2·:ans`, numeric weighted averages, similarity-based chunk
  values, log-sum-exp blended activation, and the `:blt` trace) — making
  `blending` green. The `:cst` conflict-set trace (instantiated
  `print-production-text` plus production parameters) made
  `tutorial-unit8-simple-ppm-model-initial-run` green. Production
  compilation (`:ppm` procedural partial matching, the
  `goal-focus-fct`/`(car (define-chunks …))` model form, and the
  `:epl`/`:pct` composition/trace for the goal test+modify case) made
  `ppm-compilation` green. Runtime `:sgp` driver steps plus partial-match
  annotations/utility in the `:cst` trace made the `simple-ppm` family
  green (`simple-ppm-test`, `…-ppm-run`, `…-medium-chosen-run`).
  `*buffer>` modification requests and indirect-action validity
  (`:cia`) cover run 1 of `indirect-action-compilation`; indirect-action
  composition + installing compiled productions into the runtime made
  `indirect-action-compilation` green. A bounded `!eval!`/`!bind!`
  evaluator (pure operators + `safe-` variants; unknown heads error)
  made `dynamic-slot-compilation` green. A bounded task-file `defun`
  interpreter plus the sole-`isa` matching rule made `bind-test` and
  `remote-bind-test` green. A `:ACT` `:sgp` arm enabled the unit5-v3
  activation trace (`tutorial-unit5modeling-v3-first-run`). A scripted
  device window for the test-model corpus (an `ADD-TEXT` sidecar,
  `:attended`/`nil` request-parameter matching, the QWERTY key table,
  and a `clear_on_keypress` behavior that re-encodes the vanished
  attended object) made `demo2` green. Model-body `schedule-event`
  plus a bounded task-command dispatch (`add-act-r-command` aliases,
  `goal-focus`/`set-buffer-chunk`/`schedule-event-relative`/`act-r-random`),
  `declare-buffer-usage`, `(spp :reward R)` propagation and its `:ult`
  update trace made the `ul-issues` cluster green (fixing the extractor's
  stray runtime `:goal-focus` steps and a `run_secs` event-drop bug).
  The `TEMPORAL` module (tick/noise dynamics, `isa time`/`isa clear`
  requests, `:record-ticks`, `delete-event` via heap retain) and
  `(spp :at S)` action-time overrides made `test-temporal` green.
  `:randomize-time` (`:vpft`/`:vidt`) jitter for production, vision,
  imaginal and motor latencies made `demo2-randomized` green.
  The full `:cia`/`:rir` production-compilation engine (composed
  productions instantiate indirect-action variables, expand indirect
  requests to chunk specs, retain indirect retrievals under `:rir`, and
  handle `!bind!` composition conditions) made
  `retain-indirect-retrievals` green across all three runs. The
  partial-matching `:full` activation trace, finst removal trace,
  retrieval-threshold messages, `buffer empty` vs failure, and
  `!eval!` task-command dispatch made `grouped` green.
  Remaining: task-supplied commands
  (`grouped`), production validation (`test-bad-productions`, blocked
  by cross-test warning state), `!eval!` timing (`dynamic-real-time`),
  other `require-extra` behaviors (associative-learning/act-touch/
  z-threads/extended-motor), `schedule-event` scaffolding, and
  multi-model traces.
- Next: audio/speech devices, production compilation.

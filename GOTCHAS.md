# ACT-R → Rust Gotchas

Quirks of the Lisp source that have bitten us or will bite the port.
Each entry is tagged **verified** (hit it during M0 execution) or
**code-read** (seen in source, not yet exercised end-to-end).

## 1. `sgp` quotes its arguments — use `sgp-fct` programmatically (verified)

`(defmacro sgp (&rest parameters) \`(sgp-fct ',parameters))`
(`actr7.x/framework/parameters.lisp:202`).

`(sgp :seed seed)` passes the *symbol* `SEED`, warns
(`Parameter :SEED cannot take value SEED...`), and silently keeps the
old value. Same for `(sgp :v trace-path)`. In M0 this produced
plausible-looking output (trace to stdout, time-seeded RNG) with no
error — the failure mode is silent wrong-behavior, not a crash.
Programmatic callers must use `(sgp-fct (list :seed seed))`.
Expect this macro-quoting pattern everywhere in the command layer.

## 2. Rebinding `*standard-output*` does NOT capture the trace (verified)

ACT-R's printing module holds its own stream objects per model
(`printing-module-v` / `-c` streams, `framework/printing.lisp:240+`),
so `let`-binding `*standard-output*` around `(run ...)` captures
nothing. The sanctioned sink is `(sgp :v <pathname-or-stream>)`
(+ `:cmdt` for command output), which writes byte-identical content
to the stdout trace (diff-verified in M0).

## 3. `:v` file output appends — delete stale traces first (code-read)

`safe-open-printing-stream` opens with `:if-exists :append`
(`framework/printing.lisp:270`). Regenerating goldens without deleting
first silently concatenates runs. `run-golden.lisp` deletes before each run.

## 4. `(reset)` closes `:v`/`:cmdt` file streams (code-read)

`reset-printing-module` closes file streams and resets the stream to `t`
(`framework/printing.lisp:365`). Install the `:v` redirect *after*
`(reset)`, and re-close with `(sgp-fct (list :v t))` after `(run ...)`
(the set-back-to-`t` path closes the file).

## 5. RNG is time-seeded unless `:seed` is set (verified)

The random module seeds from `get-internal-real-time` at creation
(`framework/random.lisp:394`). Without an explicit
`(sgp :seed (S C))` — a 2-int list of seed + advance-count — two
identical runs diverge whenever subsymbolic noise (`:esc t`) draws
randomness. M0 sets the seed per model *before* `(reset)` and the
setting survives reset (cross-process diff-verified).

## 6. Model files nuke state on load (verified)

Every tutorial model file starts with `(clear-all)`, so loading a model
wipes previously defined models. Upside: many models can run
sequentially in one SBCL process (M0 does 3). Downside: load order is
load-bearing, and anything stashed between loads must be re-established
(seed, `:v` redirect — see 4, 5).

## 7. The whole model-definition surface is macros (verified by grep)

`define-model` (`framework/model.lisp:381`), `p`
(`commands/procedural-cmds.lisp:767`), `chunk-type`
(`framework/chunk-types.lisp:336`), `add-dm`
(`core-modules/declarative-memory.lisp:1992`), `goal-focus`
(`core-modules/goal.lisp:270`) are all `defmacro`. There is no
function-call-level model API to FFI into — the Rust side needs its own
S-expression reader plus an evaluator for at least this surface, with
macroexpansion semantics decided per form.

## 8. `(run N)` is an upper bound, not an exact duration (verified)

A run stops early with `------ Stopped because no events left to
process` when the event queue drains (all 3 M0 models halt at t<1.0
under `(run 10)`). Golden run-times must exceed the model's natural halt;
a model that *doesn't* halt is a hang, not a diff — harness timeouts
belong in `regen.sh`/CI, not in the trace comparison.

## 9. Load-time warnings bypass the trace (verified)

E.g. `Creating chunk CATEGORY with no slots` prints during
`load-act-r-model`, i.e. outside the `:v` capture region. The oracle
contract is "bytes from `(reset)` through end of `(run)` only" — banner,
version strings, and load warnings are explicitly excluded (they go to
the driver's stdout).

## 10. First load compiles ~231 `.fasl` files into the source tree (verified)

`compile-and-load` (`actr7.x/load-act-r.lisp:561`) writes binaries next
to sources, and switching between full / single-threaded mode forces a
recompile (`load-mode.lisp` / `require-mode.lisp` tracking). Don't
commit `*.fasl`; expect a slow first `regen.sh` on a fresh checkout and
fast warm runs. We standardize on single-threaded mode
(`load-single-threaded-act-r.lisp`); the full build needs Quicklisp +
threads/sockets and is out of scope.

## 11. There is no test suite; the tutorial is the oracle (verified)

No `test/` dir, no fiveam/lisp-unit/prove usage (grep-clean). The
de-facto suite is `tutorial/unit*/` models + `tutorial/lisp/` task
drivers + `tutorial/python/` remote-interface scripts, with expected
behavior buried in PDF prose. Every golden added to `manifest.lisp` is
a judgment call about "this output is correct" — anchored to tutorial
text, not to assertions.

## 12. `sgp` getters return lists (observed)

`(sgp :v)` returns `(T)`, not `T` (`sgp-fct` → parameter-list protocol,
`framework/parameters.lisp:205`). Any Rust code echoing parameter state
must unwrap single-element lists.

## 13. Float formatting is the biggest exact-match risk (anticipated)

Trace lines print times like `0.050` and parameters like `:lf .05`.
Lisp float printing vs Rust `{}`/`{:?}` formatting is the most likely
source of byte-diffs once the engine runs (`misc-utils.lisp` already
carries per-Lisp double-float conditionals). If exact-match proves
brittle here, fall back to timestamp-normalized diffing before
weakening anything else.

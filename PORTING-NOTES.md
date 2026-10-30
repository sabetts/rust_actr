# ACT-R → Rust Porting Notes

Quirks of the Lisp source that have bitten us or will bite the port.
Each entry is tagged **verified** (confirmed by an end-to-end run) or
**code-read** (seen in source, not yet exercised end-to-end).

## 1. `sgp` quotes its arguments — use `sgp-fct` programmatically (verified)

`(defmacro sgp (&rest parameters) \`(sgp-fct ',parameters))`
(`actr7.x/framework/parameters.lisp:202`).

`(sgp :seed seed)` passes the *symbol* `SEED`, warns
(`Parameter :SEED cannot take value SEED...`), and silently keeps the
old value. Same for `(sgp :v trace-path)`. This produced
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
to the stdout trace (diff-verified).

## 3. `:v` file output appends — delete stale traces first (code-read)

`safe-open-printing-stream` opens with `:if-exists :append`
(`framework/printing.lisp:270`). Regenerating traces without deleting
first silently concatenates runs. `run-traces.lisp` deletes before each run.

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
randomness. The seed is set per model *before* `(reset)` and the
setting survives reset (cross-process diff-verified).

## 6. Model files nuke state on load (verified)

Every tutorial model file starts with `(clear-all)`, so loading a model
wipes previously defined models. Upside: many models can run
sequentially in one SBCL process (currently 3). Downside: load order is
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
process` when the event queue drains (all 3 models halt at t<1.0
under `(run 10)`). Trace run-times must exceed the model's natural halt;
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
behavior buried in PDF prose. Every trace added to `manifest.lisp` is
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

---

# Engine core findings (all verified: trace byte-match or source-read)

## 14. Trace lines are filtered by event `:output` vs `:trace-detail`

`filter-test` (`framework/printing.lisp:436`): `low` shows only `low`
events, `medium` shows `low`+`medium`, `high` shows everything.
`:trace-detail` defaults to `medium`. Lines visible only at high:
`PRODUCTION-SELECTED`, `BUFFER-READ-ACTION`, `MOD-BUFFER-CHUNK`,
`MODULE-REQUEST` (all `:output t`). This is why the semantic trace
(default detail) is 13 lines while count (`:trace-detail high`) is 48
for the same mechanisms — no behavioral difference, pure filtering.

## 15. Productions gain implicit clears at parse time (strict harvesting)

`support/production-parsing.lisp:1323`: every LHS `=` buffer (and
`?x> buffer failure` query) not targeted by any RHS action gets an
appended clear; every `+` request without an explicit `-` clear gets
one too. Both clear at priority 10, `:output medium`-if-`:rhst`.
The engine must reproduce parse-time action appending, not just the
written RHS — `DIRECT-VERIFY` and `STOP` clear retrieval with no
visible request.

## 16. Same-timestamp order falls out of priorities, not code order

Observed: modify 100, request 50, clear 10, `conflict-resolution`
`:min`, `production-selected` `:max`, `start-retrieval` -2000,
retrieval `SET-BUFFER-CHUNK` `:max`. The queue rule
(`compare-events`, `framework/meta-process.lisp:659`) is: time first,
then `:max`/numeric-desc/`:min`, ties FIFO. `MOD, REQUEST, CLEAR,
start, CR` at one timestamp is exactly that ordering.

## 17. `!output!` runs during firing, before action events

`!output!` compiles to `rhs-binds`, not `production-actions`
(`production-parsing.lisp:1241`), so `TWO ` prints between the
`PRODUCTION-FIRED` line and the `MOD-BUFFER-CHUNK` event. Text format
is `~{~S ~}` per value plus newline (`THREE ` + `\n`).

## 18. `SET-BUFFER-CHUNK ... NIL` means "not from a request"

Details string appends `" NIL"` iff `requested` is nil
(`framework/buffers.lisp:1433`). Goal reset passes nil; retrieval
completion passes `t`. Named chunks print as the bare name, specs
print as `(ISA TYPE SLOT VAL ...)` — hence the two SET shapes.

## 19. SET prints under the owning module, not the buffer

`schedule-set-buffer-chunk`'s `:module` arg: goal reset uses `GOAL`,
retrieval completion uses `DECLARATIVE`. The first run was 41/42
lines for exactly this reason.

## 20. Clock is integer ms; line layout has ghost fields

`~6d.~3,'0d` seconds/ms, then *three* spaces (separators of the empty
single-model mp/model fields in `+format-event-event-string+`,
`framework/events.lisp:262`), module padded to 22
(`PRODUCTION-COMPILATION`, longest loaded name), one space, details.
Integer-ms arithmetic — no float clock — keeps times exact.

## 21. Retrieval latency and threshold, exactly

Success latency `round(lf_ms * e^(-le*A))`, failure latency the same
with `A=rt`, threshold is strict (`A < rt` fails, `=` succeeds).
`:lf` is stored in ms (`:lf .05` → 50). With all subsymbolics at
defaults, `A` is exactly `0.0` (blc 0; bll/ans/mas/mp nil; spreading
falls through to `0.0`), so every retrieval takes exactly `:lf`.

## 22. Utility ties go to the LAST production (`:er nil`)

`core-modules/procedural.lisp:2251`: on `=`, the tied production
*replaces* rather than joins (unless `:er`). All conflict sets
are singletons so this is untested by traces, but implemented per
source.

## 23. `(run N)` stop conditions

Events scheduled at exactly the limit still run (strict `>` in
`scheduling.lisp:821`); queue empty first → `Stopped because no
events left to process` at the current clock; limit first → a dummy
event runs the clock to the limit, then `Stopped because time limit
reached`. Only the first variant is trace-tested.

## 24. No-match CR arms a wake-on-change trigger

`schedule-event-after-change` (`procedural.lisp:2329`): a CR that
selects nothing leaves a dynamic event that fires on the next buffer
mutation. The engine models this as an armed flag consumed by the
next set/clear/modify — it is why retrievals wake the cycle at
+50 ms with no polling event in the trace.

---

# Randomness & activation findings (all verified: trace byte-match or source-read)

## 25. The RNG is stock MT19937, but the consumers are single-precision

`init_genrand`/`init_by_array`/`genrand_int32` are verbatim
`mt19937ar.c` (N=624, M=397). What matters for exactness is downstream:
`genrand_real2` divides by the *single-float* literal `4294967296.0`,
`act-r-noise` clamps to `[0.0001, 0.9999]` and computes `s*ln((1-p)/p)`
in single precision, and `compute-activation-latency` rounds
`lf_ms * e^(-le*A)` in single precision too (`:lf` is stored as
integer ms). The Rust port does all noise/latency math in `f32` and
matches SBCL bit-exact (14 ms and 27 ms noisy latencies in the
`noisy-count-7` trace). Residual risk: `ln`/`exp` may differ by 1ulp
across libms; only an exact `.5` rounding boundary would flip a
latency, and none has appeared.

## 26. Seed protocol is init + discard, and unseeded draws reseed

`(sgp :seed (S C))` inits from `S` (`init_genrand` if it fits 32 bits,
else `init_by_array` of `[low-32, high-32]`) then discards `C` draws.
A stream used before any seed reseeds itself to 5489 on first draw
(`random.lisp:274`) — the engine's `Mt::fresh` reproduces both.

## 27. Noise is drawn per candidate, in order, even when invisible

`start-retrieval` draws `:ans` noise per candidate chunk in DM order;
`compute-utility` draws `:egs` noise per conflict-set production in
definition order — including single-candidate/single-match cases
where the value cannot affect selection but DOES advance the stream.
Any port that skips "unneeded" draws desynchronizes every later one.

## 28. No device-free tutorial model exercises noise
`grep :ans` hits only device-bound tasks (paired needs vision/manual,
siegler needs audio, past-tense needs imaginal+rewards). Coverage
therefore comes from a synthetic model,
`actr7.x/user-loads/noisy-count.lisp` (count + `:ans 0.5`), which is
also the first trace to show `RETRIEVAL-FAILURE` (noise below
`:rt`), failed-buffer CR stalls, and non-50 ms latencies. Two seeds:
`(100 0)` fails the first retrieval; `(7 0)` retrieves twice first.
`user-loads/` is ACT-R's designated user-file directory, so the
harness loads it through the normal `ACT-R:` logical host.

---

# Unit tables: oracle tables for pure functions

## 29. End-to-end traces say what, unit tables say where

`rust_actr7/fixtures/units/tables/*.sexp` are produced by
`run-units.lisp` from side-effect-free ACT-R functions (no model
needed) and asserted row-by-row by Rust tests, parsed with our own
`sexpr` reader. Regen via `regen-units.sh`; `cargo test` stays
SBCL-free. Rule: pure functions and edge semantics get tables;
anything needing live model context stays in `../traces/`.

## 30. CL `round` is half-to-even; Rust `round` is half-away

`(round 2.5)` → `2`, `(round 3.5)` → `4` (units table ROUND rows).
The engine's latency rounding uses a `lisp_round` helper, not
`f32::round` — identical on all non-half inputs, and the 802-row LAT
table plus both noisy traces pass with it. Any future float→int
conversion must go through `lisp_round`.

## 31. `compute-references` OL branches differ in act-scale

Off and numeric-OL add `act-scale` after the log; the `:ol t` branch
does NOT (declarative-memory.lisp:2725). An early port added it
uniformly — the units table caught it (0.342 vs 2.414). The now-unused
`bl_equation`/`Ol` live behind `allow(dead_code)` until the `:bll`
slice wires them into retrieval.

## 32. Reaching module internals headless needs `with-model-eval`

In `--eval` context there is no current model, so `get-module`
warns and returns nil (there is no `get-module` function at all —
only `get-module-fct`). The units driver wraps reads in
`(with-model-eval 'count ...)` (`framework/model.lisp:812`).
`compute-references` itself never touches its `dm` arg with
`sact=nil`, but `dm-act-scale` has no `sgp` getter (not a real
parameter), so the instance is genuinely needed for ACTSCALE rows.

## 33. Chunks with zero references never reach the decay equation

`base-level-activation` short-circuits refcount-0 chunks to
-999999.0 with a warning (`declarative-memory.lisp:2403`). The future
engine must replicate the guard, not feed empty lists to the equation
(which would `ln(0)` instead).

## 34. `:owner nil` params inherit central defaults — `:ol` is T

Declarative `:esc`/`:er`/`:ol` are `:owner nil`, meaning central
parameters owns them (`support/central-parameters.lisp:192`:
`:ol :default-value t`). An unset `:ol` reads back `T`, so the
optimized-learning-`t` branch of `compute-references` is the DEFAULT
path, not the `nil` one. The engine initially defaulted it off and
only a trace latency mismatch (11 ms vs 6 ms) exposed it —
DTO: never trust a `define-parameter` without following the
`:owner` chain.

## 35. Every buffer clear merges into DM (reference learning)

Declarative registers `:notify-on-clear 'merge-chunk-into-dm`, so each
clear adds a reference (list prepend, count+1; truncated/emptied under
numeric-`t` `:ol`). Fresh contents get creation=now, refs=(now),
count=1. `add-dm` chunks start at creation 0, refs (0), count 1.
Empty clears merge nothing. At model reset the whole body
re-evaluates (`framework/model.lisp:983`), so DM contents are rebuilt
fresh — resets are idempotent for histories.

## 36. No SJI/spreading units table — stateful by nature

Attempted and abandoned: `compute-sji` reads live fan tables that only
exist mid-run via merge paths (fan-out/fan-in are NIL at load even
with `:mas`, and `setf` on extended chunk slots silently no-ops from
outside). Fabricating fan state would make the oracle circular (we'd
assert our own assumptions). Spreading stays an engine-integration
concern for a future device-task trace; the formula itself
(`mas - ln(fan)`, clamp, `:nsji`) is 5 lines at
declarative-memory.lisp:2536.

---

# Device buffers (all verified: trace byte-match)

## 37. Nested `load-act-r-model` is locked out

Calling it inside a file it is loading warns
("currently loading a model") and SKIPS the inner load, leaving a
stale current model — the attend trace once ran bll-count this way.
Task/setup files must use plain `actr-load` for inner model loads.

## 38. Device state must be built after reset, by the entry that owns it

Reset clears pending visicon additions (and windows do not survive
from load time), so task files build windows in a `(task-setup)` the
driver calls post-reset. Two hygiene rules in `run-traces.lisp`: only
an entry that newly defines `task-setup` runs it (startup auto-load
of `user-loads/` leaks a stale binding into every entry otherwise —
this contaminated `count` once), and it is always unbound afterwards.
Manifest entries grew an optional 5th field: the task file.

## 39. Armed CR wakes after ANY event, not just buffer changes

`schedule-event-after-change` waits on `:any` (scheduling.lisp:1404;
`visicon-update` is explicitly non-maintenance so it can trigger).
The attend trace proves it: CRs follow motor phase events with no
buffer traffic. The engine triggers after every event except the
arming CR itself (which would self-loop). Earlier traces are
unaffected — their post-arm events were always buffer changes anyway.

## 40. Implicit `?buffer> state free` queries + module exemption lists

RHS `+`/`*` requests gain an implicit state-free query unless the
buffer is exempt or explicitly queried
(`production-parsing.lisp:1287`). Exemptions are registered by the
modules themselves: goal/retrieval/visual-location/temporal skip
queries; goal/temporal skip harvest clears. Same for the engine:
explicit selection order is reads-then-queries per LHS item, implicit
queries appended last.

## 41. Request specs copy struct-valued `=buffer` refs (`-k` suffix)

`Move-attention VISUAL-LOCATION0-1` while the buffer holds
`VISUAL-LOCATION0`: `instantiate-chunk-spec` copies chunk values with
deduped names. Symbol values (THREE, "bank") pass through untouched,
which is why this never showed before. The engine copies on
`=buffer` fallback in request AND modify specs, resolving copies back
for attention/encoding.

## 42. Onset stuffing draws RNG too

New visicon features stuff the (empty) visual-location buffer, and
the pick is `random-item` over newest-timestamp features sorted by
name — one draw even for singletons (`vision.lisp` stuffing path).
Same for find-location. Any skipped "unneeded" draw desynchronizes
later `:ans` uses (cf. #27).

## 43. Motor press-key pipeline (peck-recoil), all params defaulted

Fresh prep = 5 features × 50 ms; exec = init(50) + max(burst(50),
move-time); finish = exec + burst + max(burst, move-time); move-time
= max(burst, fitts) with fitts = max(min-fitts 100 ms,
0.075·log2(r+0.5)) — Q (r=1) lands exactly on exec 150 / finish 300.
`PREPARATION/INITIATION/FINISH` lines print the request time in
seconds; `output-key` prints under KEYBOARD with the device's
(lowercase) key name. Phase lines are high-detail-only; PRESS-KEY is
always shown. Later presses need per-key (hand finger r theta) +
`feat-differences` prep math — only Q is tabled so far.

---

# Imaginal buffer & paired task (all verified: trace byte-match)

## 44. Imaginal = delayed goal-style buffer (200 ms default)

`+imaginal>` sets busy and schedules invisible `set-imaginal-free`
plus invisible `goal-style-request` at +delay, which SETs at priority
−1000 with the request spec printed slots-only (no ISA/type — same
`chunk-spec-to-chunk-def` rule as #18's imaginal case generalized:
goal-focus lists print WITH ISA, request specs print slots-only).
Busy requests are silently ignored (stdout warning only).
`=imaginal>` modify goes through the standard immediate MOD path
(the dedicated mod-request fn is for direct commands, not `=`).
`:imaginal-delay` supported, `:vidt` refused.

## 45. `+visual> clear` is a direct event plus implicit buffer clear

No `MODULE-REQUEST` line is hidden here — the production action
schedules the `clear` event directly (priority 0, output low, prints
`VISION CLEAR`), while the request-rule implicit ClearBuffer handles
the buffer itself. The clear also releases preparation state 50 ms
later via an invisible-at-medium `change-state` event whose execution
wakes armed conflict-resolution (hence otherwise inexplicable CRs
like paired's 3.241).

## 46. Armed-CR trigger priority is `:min`, not 0

`schedule-event-after-change` hardcodes `:min :min`
(scheduling.lisp:1434). The engine initially woke with priority 0,
which misordered the imaginal SET (−1000) after the CR. One-line fix,
caught by trace diff.

## 47. SET-to-visual-location stores a `-k` copy, shows the base name

Every VL SET lands `VISUAL-LOCATION{i}-k` (per-base counter from 0,
`new-name-fct` semantics) while printing the requested base name;
later `screen-pos` binds the copy's name (`Move-attention
VISUAL-LOCATION0-1`, `Encoding-complete ...`, study `-1-0`). Find
reuses the feature (no separate construction copy) — onset stuffing
and find each consume exactly one suffix. Goal/retrieval SET copies
(if any) are unobservable (never name-referenced) and skipped.

## 48. Digit keys + preparation differences

Key table extended with 0–9 hand/finger/r/theta from the keyboard
specs; repeat presses use hfrt `feat-differences` (hand 3, finger 2,
else r/theta diffs) instead of the fresh 5 features. All peck-recoil
digits land on exec 150 / finish 300 with margin (fitts 99.1 < 100).

## 49. Time-limit stops are real traces now

Paired-static idles after learning and ends `Stopped because time
limit reached` at the run limit — the first trace exercising that
variant (previously only the no-events-left path was covered).

---

# Multi-trial paired (all verified: trace byte-match)

## 50. Trial reconstruction from CR spacing

Post-keypress CR wakes decode invisible motor phases exactly: trial-3
(left-hand "1", fresh prep) shows change/prep/init/output/finish wakes
at +50/+250/+300/+400/+550; trial-4 ("0", right hand) shows
+50/+200/+250/+350/+500 — prep differences (hand change) move every
later phase. Any prep-math error shifts the whole tail; the trace
pins it.

## 51. hfrt prep differences need maxfeats 5, not 4

`num-possible-feats` is 1+slots (general-pm.lisp:783), so peck-recoil
gives hand 4, finger 3, else r/theta diffs. An initial maxfeats-4
guess failed the trace by exactly one 50 ms prep step.

## 52. Learned pair chunks are reuse-copy named

Imaginal spec requests fill the `IMAGINAL-CHUNK0` reuse chunk; harvest
merges land `IMAGINAL-CHUNK0-{k}` copies in DM (bank→`-0-0`,
card→`-0-1` in harvest order) — the names later `RETRIEVED-CHUNK`
lines print. The engine names unnamed harvest merges
`{BUFFER}-CHUNK0-{k}` via the shared copy counter.

---

# Full paired task (all verified: trace byte-match)

## 53. Maintenance never wakes armed CR — with one false friend

A decisive probe (maintenance vs regular dummies at 1.5/1.6 s) shows
only the regular dummy wakes CR (`conditions-met` requires
non-maintenance). The engine tracks event class and triggers solely
on regular events. The one exception-looking case, `output-key`
(maintenance) followed by CR@0.635, is actually the invisible regular
`notify-motor-device` waking it — hence the split into
notify/output-key events.

## 54. Fixed segments + clears replicate the real task — no contingency

The real `paired-task` runs fixed 5 s prompt/study halves and scores
afterward; responses never steer timing. The trace mirrors it with
`CLEAR`/`ADD-TEXT` sidecar actions and a Lisp stub of scheduled
events under a single run, plus an `END` dummy on both sides so the
tail reaches the time limit identically (`run` prints the time-limit
line when the queue empties exactly at the limit — verified in
source, now covered by trace).

---

# Partial matching (all verified: trace byte-match)

## 55. Candidacy is exact without `:mp`, signature-only with it

`start-retrieval` tests `=` values exactly iff `esc` is off or `mp`
is off; with both on, candidates need only type + slot presence, and
`=` values score `mp*similarity` into activation. Request vars are
skipped (spec-level, so the engine threads pre-substitution slot
names through the request events). `-` slots and inequalities in
requests stay refused at load.

## 56. Similarities live on model structs — set them post-reset

`set-similarities` needs named chunks and writes model-table structs
that reset rebuilds, so the task file must call it in `task-setup`
(post-reset), mirrored by sidecar `SIMILARITY` rows. Engine keys by
name (both directions); chunkhood is DM membership (approximation).
Sidecar symbols fold to uppercase like everything else — a lowercase
miss once zeroed every similarity to `:md` (loud failure, fixed).

---

# Utility learning (all verified: trace byte-match)

## 57. Penalty flips beat rewards for trace design

History credits every firing since the last reward (decayed by
recency), so rewarding the winner only widens its lead. A `-10`
penalty at 275 ms drops L from 0 to ≈−6.7 across its 5 recorded
firings, flipping subsequent ties to R in one step. Compact,
deterministic, RNG-free.

## 58. Reward plumbing: invisible wrapper, visible propagate

Task-scheduled `trigger-reward` (default output, high-only) forwards
to `propagate-reward` (`:max`, medium, `UTILITY PROPAGATE-REWARD v`).
The engine schedules propagate directly with identical timing.
Updates are `U += α(R−U)` per history entry in selection order with
`R = value − seconds-since-selection`, all single-float; history
clears after. Non-`esc`/`ul` rewards are silent no-ops (Lisp warns to
stdout). `:iu` honored for initial utilities; `:ult` trace lines,
`spp :u`/`:reward`, and production self-rewards remain out.

---

# ACT-R regression corpus (all verified: trace byte-match or source-read)

## 59. `test-models/*.ref` are ACT-R's own oracle — and they are trimmed (verified)

`actr7.x/test-models/model-tester.cl` is Dan Bothell's regression harness.
Each `*.lisp` stub defines `(run-it)`, loads a model, and its captured output
is compared to `*.ref`. The refs are **CRLF** and the comparator
(`compare-test-model-files`, model-tester.cl:373) right-trims every line and
uses `string-equal` (**case-insensitive**), line by line. Diff raw bytes and
every line "differs"; mirror the comparator
(`rust_actr7/tests/test_models.rs::normalize`). Our own `count` trace (since
retired) was byte-identical to `test-models/count.ref` after stripping CR,
which is what first revealed this.

The refs are now vendored one per case in `rust_actr7/tests/cases/<name>.ref`;
each `tests/cases/<name>.rs` owns its driver, and
`fixtures/sync-fixtures.py --check` keeps the copies byte-identical to upstream
while `actr7.x` is present. Source-echo refs are checked against the vendored
sources by `SourceCase`. The layout is documented in
`rust_actr7/fixtures/regression/README.md`.

## 60. Refs capture four channels, not just `:v` (verified)

`run-actr-test` monitors `model-trace`/`command-trace`/`warning-trace`/
`general-trace` (model-tester.cl:354-357), so refs can contain command output
and warnings. Only 219 of 563 refs have engine trace lines; the rest are
`dm`/`sdp`/`spp`/`whynot`/parsed-source dumps or warnings. `:v` already
defaults to **T**, and the `(:testing-function set-testing-traces)`
`:starting-parameters` line is actually rejected ("must be a list that is
valid for passing to sgp-fct") and never runs �?" traces reproduce anyway. A
`run-it` that touches `*out-file*`, `with-open-file`, or `read-line` is a
source-scrape case, not an engine test. 189 replay against the vendored
tutorial/model sources: five use the engine's Lisp reader to reproduce the
original `(read f)` selection, 173 use `SourceCase` line presence/order, and
11 are custom line/format transforms. The four that scrape
`core-modules/audio.lisp`/`vision.lisp` stay ignored pending engine-level
chunk-type assertions (no core-module Lisp is vendored).

## 61. `model-tester.cl` must be loaded by physical path in SBCL (verified)

`(load "ACT-R:test-models;model-tester.cl")` dies translating
`...MODEL-TESTER.CL.NEWEST` (`TRANSLATE-PATHNAME ... :NEWEST NIL`): the ACT-R
logical host pattern `**;*.*` has no version component, but `load` merges a
`:newest` version in. Use
`(load #P"<actr7.x>/test-models/model-tester.cl")`. (`load-act-r-model` and
`actr-load` do the translation themselves and are unaffected.)

## 62. `max-module-name-length` is rebindable, so the trace column width varies (verified)

`framework/modules.lisp:386`. Stubs `setf` its fdefinition to shrink the
module column: `tutorial-unit1-count-trace` pins 13,
`tutorial-unit1-tutor-model-*` pins 11, default is 22. Hardcoding 22 (as the
engine did) mis-diffs every line of those refs at the first `SET-BUFFER-CHUNK`.
`extract-specs.lisp` records `(max-module-name-length)` at each `run`; the
column width is a per-run spec field
(`run_model_with`/`Engine::new_with_width`).

## 63. Integer and float slot values print differently (verified)

Lisp `~S` prints `3` for an integer and `3.0` for a float. `Pattern::Num(f64)`
collapsed the distinction, so `tutor-model`'s initial goal printed
`TEN1 3.0` where the ref has `TEN1 3`. `Pattern`/`Value` now carry a distinct
`Int(i64)`; equality remains numeric across the two so `3` still matches a
stored `3.0`.

## 64. Computed seeds make some refs run-order dependent (verified)

Stubs use `(sgp-fct (list :seed (list (incf *starting-seed*) 0)))` and friends.
The extractor executes each stub and records the **effective** seed at each
`run`. Deciding whether that seed is reproducible is the subtle part: runtime
observation is useless (the random module's own reset calls `sgp-fct` with
`:seed`, so every model looks "explicit"), and a plain text search is wrong
both ways (commented-out `(sgp :seed ...)` looks explicit;
`(ssp :starting-parameters (:seed ...))` looks explicit but ACT-R rejects
that list once `:testing-function` is in it, so the seed really is
time-random). The extractor parses the stub and each loaded model file with
`read` and only trusts a real `(sgp ... :seed ...)`/`(sgp-fct ... :seed ...)`
form (comments skipped for free). Everything else is canonicalized to
`(100 0)`, which keeps the committed corpus stable across regens; non-PURE
specs are canonicalized regardless since they are re-extracted when their
feature lands.

## 65. Extraction must force `run`'s real-time flag off (verified)

Some stubs call `(run N t)`. Real-time only paces wall clock and does not
change model output, so `extract-specs.lisp` calls the original `run` with
`nil`; otherwise regen sleeps for the run duration. (The `(declare (ignore
real-time))` in the wrapper must precede the other forms — placing it after
made every executed stub fail to compile.)

## 66. Multi-model refs add a model column (verified)

`unit-1-together-1-mp` runs COUNT+SEMANTIC+ADDITION together; its ref lines
carry a model-name column (`0.000  COUNT     GOAL ...`) that single-model
`format_line` never emits. Detection is structural, not from `:v` (which read
back NIL): the model file defines more than one `define-model`. Out of scope
until multi-model/meta-process support.

## 67. `run` limits are relative to the current model time (verified)

Each `(run N)` in a script bounds events to `current_time + N*1000`, not an
absolute model time, and a drained queue emits its own `Stopped because no
events left to process` at the current clock (`framework/scheduling.lisp:816`,
`meta-p-start-time`). `semantic.ref` stops at 0.150, 0.400, 0.750 — one
terminator per `run`, clock carrying over. `Engine::run_secs` is the relative
step; `run(self)` is just the single-shot zero-based case. Seeding happens
once; Lisp `(run ...)` never reseeds.

## 68. `goal-focus` is a priority-`:max` buffer set (verified)

`goal-focus-fct` (`core-modules/goal.lisp:275`) schedules
`SET-BUFFER-CHUNK GOAL (spec) NIL` at the current time with `:priority :max`
and `requested nil`, plus an invisible maintenance `clear-delayed-goal`. The
`CONFLICT-RESOLUTION` line that follows is the armed wake-on-change
(PORTING-NOTES #24), not part of goal-focus. A trailing `goal-focus` after the last
`run` never executes and adds no lines (`ul-issues-first-run`), so replaying
it as a scheduled-but-unprocessed event is correct.

## 69. Scripted drivers need ordered steps, not just counts (verified)

`semantic` is run → goal-focus → run → goal-focus → run; `ul-issues-second-run`
runs once with `:v nil` (trace suppressed, `!output!` still printed) then with
`:v t`. A `(:RUNS ...)` list plus `:GOAL-FOCUS`/`:RESETS` counts cannot express
that, so specs carry an ordered `:STEPS`. `:v` is encoded per run; a `:seed`
step re-inits the RNG at that point only — re-seeding before every run would
reset the stream, which Lisp does not do.

## 70. Test-model task files are not model files (verified)

Stubs like `tutorial;lisp;ul-issues.lisp` define `defun`s and load the real
model; strict `load_models` rejects the `defun` ("unexpected top-level form:
DEFUN"). `run_model_steps` now loads only files that contain a top-level
`define-model`. Multi-model files (`unit-1-together-1-mp` defines
COUNT+SEMANTIC+ADDITION) are detected and skipped: their refs carry a
model-name column the single-model formatter lacks (cf. #66).

## 71. `define-chunks` chunks are NOT in declarative memory (verified)

`define-chunks-fct` (`framework/chunks.lisp:1129`) puts chunks in the model's
chunk table; `add-dm` runs the same creation *and* `add-chunk-into-dm`. The
table is needed for name resolution (goal-focus by name, `=buffer` copies)
but retrieval must never see those chunks. The engine keeps a `named` map
separate from `dm` and `instantiate(Named)` checks `dm` then `named`.
`define-chunks` also accepts bare names (`(define-chunks a b)`), which are
chunks with no type/slots.

## 72. `(reset)` zeroes the clock and rebuilds DM/buffers, but not the RNG (verified)

`internal-reset` sets `meta-p-time` to 0 (`framework/meta-process.lisp:339`);
`reset-model` clears the chunk table, rebuilds the starting chunk structs,
clears every buffer, and **re-evaluates the model body** (so
`add-dm`/`define-chunks`/`set-base-levels`/`set-similarities` reapply). The
random seed is a module parameter and survives reset (`run-traces.lisp`
seeds before reset). `Engine::reset` mirrors this and preserves trace
history; `Engine::new` now just calls it.

## 73. Model-body `declare-buffer-usage` and `spp` are behaviorless (code-read)

`declare-buffer-usage` only records declared slot usage to suppress style
warnings (`commands/procedural-cmds.lisp:789`). Model-body `(spp <prod>
:param value)` sets production parameters (e.g. `:reward`); ignoring it
changes utility learning only where a production self-reward is actually
exercised (PORTING-NOTES #58). Both were blocking unit5/UL models at *load* and are
now accepted no-ops.

## 74. `?buffer> buffer empty|full` is buffer presence, not module state (verified)

`buffer empty`/`buffer full` test whether the buffer holds a chunk;
`state free` tests module busy state; `failure` is the retrieval-failure
flag. `?goal> buffer empty` is used by `merging-test` and unit5 P1.

## 75. `+goal>` is a goal-style request: delay 0, priority -1000, prints ISA (verified)

`goal-style-request` calls `schedule-set-buffer-chunk ... 0 :priority -1000`
(`support/goal-style-module.lisp:245`), so the new goal lands *after* the
production's clears (priority 10) — matching `merging-test`'s 0.050 order of
MODULE-REQUEST / CLEAR-BUFFER / SET-BUFFER-CHUNK GOAL. Unlike
retrieval/imaginal request SETs (slots-only, `bare`), the goal SET prints
`(ISA ...)`.

## 76. Unit5 modeling is gated by spreading, not `define-chunks` (verified)

The `declarative-issues*` models set `:mas 2`; `Config::from_model` refuses
`:mas`/`:pas` (`unimpl`). `define-chunks`, `set-base-levels`, and
`set-similarities` are prerequisites, but exact reproduction needs spreading
(`compute-sji`: `mas - ln(fan-out/fan-in)`, clamped; PORTING-NOTES #36). Remaining
non-spreading blockers in the corpus: negative request modifiers
(`+retrieval> ... - slot nil`, refused at load), `:crt`/`:epl`/`:pct`
conflict-resolution trace text, and `schedule-event` model scaffolding.

## 77. Spreading needs DM fan tables, rebuilt at reset (verified)

`compute-sji` (`declarative-memory.lisp:2536`): an explicit similarity wins;
otherwise, when source `j` is in candidate `i`'s fan-in,
`sji = mas - ln(fan-out(j) / count(j in i))`, clamped at 0 unless `:nsji`.
`fan-out(j) = 1 (self) + #DM chunks referencing j`; `fan-in(i)` is i's
referenced-chunk occurrences plus a self entry. Only DM chunks get a fan-in;
a referenced `define-chunks` name still gains fan-out. Setting `:mas` sets
both `dm-mas` and `dm-sa`, so any `:mas` value turns the path on. `:mas` is
now accepted; only `:pas` is still refused. Verified by `unit5-v1-run` and
`v2`.

## 78. Source spread is per-buffer with different defaults (verified)

`spreading-activation` sums over model buffers with a nonzero `buffer-spread`:
goal is `:ga` (**default 0**), imaginal is `:imaginal-activation` (**default
1.0**), retrieval/vision/manual default 0. Each source contributes
`(spread / #chunk-valued-slots) * sji`. So default unit5 spreading comes
from the imaginal buffer, not the goal.

## 79. Model-body `set-base-levels`/`set-similarities` are reset-time (verified)

`set-base-levels (name level creation-seconds)`: under `:ol t` sets
reference-count = level and uses creation in the optimized-learning equation
(refs list ignored), so `(v1 1 -1500)` yields base-level `-2.964` — matching
the `sdp` output. The engine stores `level` copies of the creation time in
`refs` so its `refs.len()`-based equation agrees. `set-similarities (a b v)`
is stored both directions and re-applied at reset (chunk structs are rebuilt).

## 80. `-` request slots only penalize under partial matching (verified)

Under exact matching (`esc` off or `mp` off) `- slot value` excludes chunks
whose slot equals value. Under partial matching it does **not** filter: it
adds `mp * md` only when the chunk's value has similarity `ms` to the
negated value (`partial-matching`, declarative-memory.lisp:2596). The spec
loader now carries a `neg` list instead of refusing `-`.

## 81. Partial-matching candidacy is signature-based, not type-based (verified)

`start-retrieval` narrows candidates by `slots-vector-match-signature` — the
request's **filled/empty slot masks** — not by chunk type
(`framework/chunk-spec.lisp:895`, `declarative-memory.lisp:1125`). Per
`add-slot-spec-to-chunk-spec` (chunk-spec.lisp:758): `= V` (V non-nil) marks
the slot filled; `= nil` marks it empty; `- nil` marks it filled; `- V`
(non-nil) imposes no signature constraint. Slots with a nil request value are
signature-only (no testable slot, so they do not score). **This filter runs in
exact mode too** — `chunk-list` is signature-filtered before the type/value
scan, so a non-signature chunk is skipped silently (no "does not match" line).
This is why `+retrieval> isa simple-value - result nil` can retrieve the
`math-fact` `2-1=1`, and it turned unit5 `first-run`/`v4-p3-run` green.
Not yet handled: `v6-run-p3` misses only by ~1 ms at the run limit (float
brittleness, not candidacy).

## 82. Request slot names can be LHS variables (verified)

`+retrieval> ... - =attr nil`: the *slot name* is `=attr`, not a literal
slot — it resolves against the goal binding to e.g. `CATEGORY`. Slot names
must be substituted against bindings when a request spec is concretized
(`resolve_slot_name`), or the signature mask references a nonexistent slot.
Needed by `semantic-dynamic`.

## 83. Request specs copy `=buffer` chunk values (`-k`), with copy fan-out (verified)

In goal/imaginal `+`/`=` specs, an unbound `=buffer` reference is copied to
`NAME-k` (PORTING-NOTES #41), deduped **within one firing** so a modify and a request
referencing the same buffer share `NAME-0`. A copy is a named, non-DM chunk:
its fan-out starts at 0 and `c-fan-out` = the source's fan-out at copy time
(`copy-from-chunk-function chunk-fan-out`). Sji for a non-DM source uses
`(fan-out + max(1, c-fan-out)) / count` (`chunk-fan-j-to-i`), which is exactly
what yields `sji-merge`'s `A-0` Sji `4.306853`. Also: `chunk-filled-slots-list`
is the chunk's internal list, built by prepending, so the spreading-source
order is **reverse insertion** (`(A-0 DONE)`, not `(DONE A-0)`).

## 84. `:act` emits timestamp-free activation trace (verified)

`dm-act-level` (`declarative-memory.lisp:1042`): `low` shows for any truthy
`act`, `medium` unless `:act low`, `high` only for `act` `high`/`t`. The trace
is `model-output` text interleaved with the event trace (so it lives in
`out_lines` without a timestamp/module prefix): match reporting,
`Computing activation for chunk X`, base-level (`Computing base-level`,
`Starting with blc:`, `Total base-level:`), spreading (buffer/spread/sources/
per-source Sji, `Total spreading activation:`), noise, `Chunk X has an
activation of:`, and the best-selection lines. `~f` on an integer-valued
float prints `.0` (`lisp_f`). `sji-merge` (`:act t`) is green.

Still unimplemented: `:crt` conflict-resolution text (`Trying production …`,
`Fails because: …`), which `buffer-queries` needs along with unnamed `add-dm`
auto-naming.

## 85. `:crt` conflict-resolution text + unnamed `add-dm` (verified)

`:crt` (procedural.lisp:2098) makes conflict resolution test **all**
productions in definition order, printing `Trying production: NAME` for each,
`Fails because: ` + a `failure-reason-string` line on failure, then
`Production NAME matches` for every member of the conflict set. The common
reason is a failed query: `The BUFFER {VALUE} query of the {BUFFER} buffer
failed.` (also `STATE`). Separately, an `add-dm` entry that is neither a bare
name nor `NAME ISA TYPE …` is an **anonymous** slot-value list
(`(add-dm (value t))`) and gets the default CHUNK type with auto-name
`CHUNK{k}`. Also, `chunk-spec-to-chunk-def` collapses a slot-less request spec
to `(ISA CHUNK)` — a goal/imaginal request SET with slots prints slots-only,
an empty one prints `(ISA CHUNK)`. `buffer-queries` (`:crt t`) is green.

## 86. LHS variable slot names and `slot =var` requiring filled (verified)

Two matching rules the engine had wrong, both fixed via `semantic-dynamic`:
- A LHS slot *name* may be a variable (`=retrieval> … =attr =val`), substituted
  from bindings at match time (ACT-R `test-var-slot`). Resolve `SlotTest.slot`
  against bindings before reading the chunk slot.
- A LHS `slot =var` requires the slot to be **filled**: the spec's filled
  signature includes it (value non-nil syntactically), so an empty slot does
  not bind (`direct-verify`'s `original =ori` must not match G1's absent
  `original`).

## 87. A named `define-chunks` chunk modified in a buffer is renamed on merge (verified)

`goal-focus G2` places a copy of the table chunk `G2`; productions mutate that
buffer copy (adding `state`/`answer`). When the buffer clears, merging the
copy into DM must **not** reuse the name `G2`, or it shadows the `named` table
entry and the next `goal-focus G2` resolves to the mutated DM chunk. The
engine now gives such a merged copy a fresh `G2-{k}` name (under `:ol t`,
like other reuse copies). This was the second `semantic-dynamic` divergence.

## 88. `isa` is a slot signature, not a type test — for retrieval and LHS (verified)

`match-chunk-spec-p` (chunk-spec.lisp:940) and the compiled production LHS
`isa` condition (`production-parsing.lisp:786`) both reduce `ISA TYPE` to the
spec's filled/empty masks; the chunk **type is never compared**. That is why
semantic-dynamic retrieves the `CHUNK`-typed imaginal chunk for a request with
`ISA property`, and why unit5 retrieves a `math-fact` for `ISA simple-value`.
The engine now ignores `isa`/`want_type` in retrieval candidacy and in LHS
matching; the per-slot tests reproduce the signature (PORTING-NOTES #81 refined).
`semantic-dynamic` is green.

## 89. Chunk-valued references, merge aliasing, and `!stop!` (verified)

`=goal> value =imaginal` stores a reference to the imaginal buffer's chunk,
not a literal. The buffer chunk is named (lazily, `BUFFER-CHUNK0`, reuse
naming) so the reference is stable; when the buffer later clears and that
chunk merges into an existing DM chunk (`merging-test`'s `b`), the engine
records an alias `BUFFER-CHUNK0 -> b` and `value b` then matches. ACT-R gets
the same result through `true-chunk-name-fct`. Two related rules:
- An empty `=buffer>` modify (no slots) is a **no-op**: ACT-R schedules no
  `MOD-BUFFER-CHUNK` event for it (`merging-test` P2's `=imaginal>`).
- `!stop!` ends the run immediately with
  `------ BREAK-EVENT Stopped by !stop!` (no normal stop line).
`merging-test` is green.

## 90. Base-level uses the reference *count*, not the list length (verified)

`compute-references` (`declarative-memory.lisp:2695`) takes `n` (reference
count) and the reference list separately. Under `:ol t`, merges clear the
list but keep incrementing the count, so the optimized-learning branch
`ln(n * t^-d / (1-d))` must use `n`. The engine wrongly used `refs.len()`,
which went to `ln(0) = -inf` for a chunk merged before re-retrieval; that
accidentally made `v4-p3-run`/`v6-run-p3` pass. `bl_equation` now takes the
count explicitly (the LAT units table passes `n`).

## 91. Retrieval finsts and `:recently-retrieved` (verified)

`?:recently-retrieved` is a request parameter, not a slot: `:recently-retrieved
nil` excludes chunks retrieved within `:declarative-finst-span` (default
3.0 s), tracked over the last `:declarative-num-finsts` (default 4) retrievals;
`t` requires one. A success records a finst at completion time. Without this,
v4/v6 would re-retrieve the just-retrieved `V2` (its base-level is highest).
`tutorial-unit5modeling-v4-p3-run` and `v6-run-p3` are green.

## 92. Explicit similarities apply to plain symbols (verified)

`set-similarities (pink red -.4)` names symbols that are not DM chunks; ACT-R
resolves them as (undefined) chunks and consults the explicit similarity
before `ms`/`md`. The engine now checks the explicit similarity table first
for any symbol pair, not only DM chunks.

## 93. `require-extra` is a tiny macro over large modules (verified)

`(require-extra "name")` (load-act-r.lisp:844) just loads
`extras/name/name.lisp`; the extras themselves are big (blending 3378 lines,
associative-learning 1847, …). The loader now recognizes and records
`require-extra` (`Model.required_extras`) instead of erroring, but no extra
behavior is implemented, so the require-extra trace specs only advance to
their next blocker (`blending` → chunk-type defaults; `associative-learning`
→ `buffer-requires-copies`; `z-threads`/`zz-extended-motor-actions` →
top-level `defun`/`defparameter`).

## 94. Chunk-type default slot values (verified)

`(chunk-type size (size-type t))` declares slot `size-type` with default `t`.
Chunks of that type created without the slot get the default (applied after
the model body is parsed). This matters for slot-signature/intersection
logic — e.g. blending's `size` chunks all share the defaulted `size-type`.

## 95. Inequality slot tests are first-class (verified)

Slot tests carry a `SlotOp` (`=`, `-`, `>`, `<`, `>=`, `<=`) rather than a
bool. Inequalities appear in requests (`+blending> … <= value 3`, `> value 1`)
and LHS tests; they are numeric comparisons, not `=` similarity. Chunk
construction (`add-dm`/`define-chunks`) and `?` queries still reject them
(action-scoped). This lets `blending-test-7` load (it then stops at the
missing `blending` buffer).

## 96. `all-dm-chunks` order is a flattened signature alist (verified)

`dm-chunks` is an alist of `slots-vector -> chunks`, built by *pushing*
new signature groups and new chunks to the front
(`add-chunk-into-dm`, `declarative-memory.lisp:1929`); `all-dm-chunks`
(`:2747`) flattens it. So the order is: newest signature group first,
newest chunk first inside a group. For `blending-test-7` that is
`(D E C B A X-LARGE LARGE MEDIUM SMALL TINY)` — not add-dm order. This
order feeds both the blended-slot candidate list and the RNG noise-draw
order, so it is observable in the trace.

## 97. `find-matching-chunks` reverses its result (verified)

`find-matching-chunks` (`framework/chunk-spec.lisp:1129`) pushes matches
and returns the list, so the result is the reverse of the match order.
Blending's `matching-chunks` (and therefore the activation/noise order)
is `reverse(filter(chunk-list))`; with the DM order above the three
matches come out `(B C E)`.

## 98. `set-base-levels` with `:bll nil` sets a fixed base level (verified)

`set-bl` (`commands/dm-commands.lisp:623`) branches on `bll`: with a
decay it sets reference count/list and recomputes, but with `bll` nil it
just sets `chunk-base-level` to the given level, used verbatim as the
base (`base-level-activation`, `:2421`). `:bll` defaults to nil, so
`blending-test-7`'s `(set-base-levels (b 4 -10) …)` gives base levels
4/5/1, plus one logistic noise draw each (activations 2.5325/4.7635/
0.3433 for seed `(1 1)`).

## 99. `?buffer> error nil` is a negated `state error` query (verified)

`query-module` (`framework/modules.lisp:798`) special-cases the `error`
query: `(error t)` calls the module query with `state error`; `(error
nil)` calls it and negates. Only the blending module tracks an error
flag, so every other buffer reports `error nil` as true.

## 100. Blending trace floats are printed `~S`, not `~f` (verified)

The checked-in `blending.ref` shows `3.7122225e-6` / `1.1136667e-5`
(shortest single-float, exponent form outside `[1e-3, 1e7)`), not the
fixed notation SBCL `~f` would produce from the same control strings.
Reproduce `~S`/`~A` semantics for the blended values (see `lisp_s_f32`);
the ref, not the source's `~f`, is the arbiter.

## 101. `:cst` prints the conflict set with instantiations (verified)

With `:cst t` and `:v t`, every production in the conflict set is printed at
conflict resolution as its instantiated `(P NAME …)` text plus a parameter
block (`procedural.lisp:2283`, `print-instantiation`; the params come from
the utility module's `production-parameter-fct`). `isa` is the chunk type,
not a slot, so it is omitted from the condition lines; bound `=vars` are
substituted (`value =val` prints as `VALUE SMALL`). The checked-in
`modules/utility-and-reward-1.lisp` also prints `:at` and spacing that the
ref omits — as with blending's `~f`/`~S`, match the ref.

## 102. `:ppm` is procedural partial matching (verified)

`:ppm` (a number) lets a failing LHS **equality** test still match when
`similarity(requested, actual) > :md` (default -1.0), recording the
mismatch (`test-slot`, `procedural.lisp:1774`); utility is adjusted by
`ppm * Σ similarity` via the `ppm-offset` utility offset
(`:2373`). Negated/inequality tests are not partial-matched. This is
what makes `ppm-compilation`'s `p0..p22` chain fire despite `slot`
never changing.

## 103. `goal-focus-fct` / `(car (define-chunks …))` (verified)

Model bodies may set the initial goal with `(goal-focus-fct (car
(define-chunks DEF)))`: define the chunk(s) in the chunk table, then
focus the first. Anonymous `(isa TYPE slots…)` definitions are
auto-named `TYPE{index}` (`TEST0`, `TEST1`, …); the `ppm-compilation`
event skeleton matches the ref with this alone. An invalid definition
(e.g. an odd slot count) warns and yields nil, leaving the goal unset —
read from `define-chunks`/`car` semantics (code-read; the corpus spec it
affects is otherwise blocked).

## 104. Production compilation composes adjacent fires (verified)

With `:epl t` the module composes each fired production with the previous
one; `:pct t` prints the trace (`Production Compilation process
started…`, the new `(P PRODUCTIONn "A & B" …)` text, its parameters, then
`Setting previous production to …`). `:tt` (default 2.0 s) bounds the gap.
The composed goal condition is the first production's conditions followed
by the second's *minus the slots the first modifies*, with
`remove-duplicates` keeping the **later** copy (CL default); slots are
sorted by name and `isa` is dropped. Cross-production mappings unify a
second production's `=var` with the first's when both test the same slot
(`MAP-GOAL-BUFFER`, `tools/goal-compilation.lisp:320`); a variable facing
a constant is instantiated from its own fire bindings. Compiled
productions print `:utility NIL :u 0.000 :at <dat>`.

## 105. `remove-duplicates` keeps the last copy (verified)

CL `remove-duplicates` default (`:from-end nil`) drops the *earlier*
duplicates and keeps the *later*, preserving the surviving elements'
order. The production-compilation condition union depends on this
(`[SLOT small, STATE p2] + [SLOT small]` ⇒ `[STATE p2, SLOT small]`).

## 106. `:cst` partial-match annotations and utility (verified)

Under `:ppm`, `print-production-text` annotates a partially matched `=`
slot as `NAME [requested, actual, similarity]`
(`procedural.lisp:2536`, from `production-partial-matched-slots`). The
`Parameters for production` block prints `:utility` as the production's
*conflict value* — base + `ppm·Σsim` + the `:egs` transient noise drawn in
that conflict resolution (`small`=0.000/`medium`=-0.500 at `:ppm 1`; with
`:egs 1` and a chosen seed, -1.534/3.296) — and `:u` as the learned
component (0.000 here).

## 107. Runtime `:sgp` driver steps (verified)

Test drivers may interleave `(sgp :key value)` steps between runs
(`:ppm`, `:egs`, `:cia`, `:rir`, `:lf`, `:seed`, …). `reset` re-reads the
model body's parameters, so the order `(reset)` then `(sgp …)` is what
selects per-run behavior; the engine now applies these overrides live.

## 108. `*buffer>` and indirect-action validity (verified)

`*buffer>` (`buffer-mod-request`) applies the same change as `=buffer>`
but its `MOD-BUFFER-CHUNK` is traced under the *buffer's* module
(`GOAL MOD-BUFFER-CHUNK GOAL`), not `PROCEDURAL`. A production with an RHS
`+buffer> =var` (a single chunk-spec variable) is an *indirect action*;
with `:cia nil` it is invalid for compilation — the trace reads
`Production X is not valid for compilation / because it has an indirect
action with the BUFFER buffer`. Whole-chunk retrieval requests
(`+retrieval> =goal`) retrieve the named chunk directly and a `+goal>` of
one prints the referenced chunk's slots (`(COLOR RED VALUE 3)`), not the
name. Run 1 (`:cia nil`) of `indirect-action-compilation` is reproduced;
run 2 needs the full indirect-action composition + installation below.

## 109. Compiling/installing indirect actions (`:cia`) (verified)

With `:cia t` an indirect production is valid and composes. When p1's
`+buffer> =var` request is tested by p2, the request is consumed: the
requested chunk is dropped from the composition (doc suffix ` - CHUNK`),
p2's condition on that buffer is removed, and any p2 action referencing
the retrieved chunk (`+goal> =retrieval`) is expanded to the referenced
chunk's slots. Composed RHS actions are `sort-actions`-ordered
**descending** by token, so `+GOAL>` precedes `*GOAL>` (the token includes
the op char). Composition is refused with `Buffer X prevents composition …
because both productions make requests` when both productions `+` request
the same buffer. Compiled productions are re-parsed from their rendered
text and **installed** with utility 0 so they can fire (contradictory
conditions simply never match). This makes `indirect-action-compilation`
green.

## 110. A bounded `!eval!`/`!bind!` evaluator (verified)

LHS/RHS `!eval!`/`!safe-eval!`, `!bind!`/`!safe-bind!`, and `!mv-bind!`
are evaluated by a small S-expression evaluator: literals, quoted
symbols, `=vars`, `+ - * / 1+ 1- abs`, `< > <= >= = /= eq eql equal
equalp`, `not null and or`, `mp-time`, `values` +
`chunk-slot-value-fct`. Unknown functions/commands **error loudly** (never
silent `nil`). The `safe-` variants swallow errors (condition false /
binding nil). LHS `!bind!` runs after buffer tests (so it sees their
bindings); RHS binds/mv-binds run before evals, then `!output!` (ACT-R
reorders by dependencies). This alone flips `dynamic-slot-compilation`;
the other bind/eval specs need task functions (`single-bind`/`multi-bind`),
remote commands, or unrelated features.

## 111. Model-load warnings are not per-test reproducible (verified)

ACT-R's model-tester installs its `warning-trace` monitor only *after*
`(load file)` (`model-tester.cl:351` vs `354`), so warnings emitted while
a model loads are normally dropped. But `test-bad-productions.ref`
*does* contain `#|Warning: … |#` load warnings, while `semantic.ref` and
`tutorial-unit1-tutor-model-trace.ref` omit `Creating chunk …` warnings
that ACT-R clearly emits on load (verified by loading those models under
SBCL — `CATEGORY`/`PENDING`/`YES`/`NO`, and `BUSY` is preseeded by
`create-model-default-chunks`, `framework/model.lisp:755`). The
difference is the global `suppress-and-capture-warnings` state
(`misc-utils.lisp:887`), which is not reliably reset between productions,
so which warnings surface depends on prior loads. Emitting load warnings
per-test to match `test-bad-productions` therefore regresses
`semantic`/`tutor-model`; this is why production-validation + warning
capture was abandoned.

## 112. Task-file `defun`s + the sole-`isa` rule (verified)
`bind-test`/`remote-bind-test` call task helpers (`single-bind` = `1+`,
`multi-bind` = `values` of the goal chunk's `slot1`/`slot2`, the latter by
name or as a remote command string). A `models/X.lisp` model's sibling
`X.lisp` task file is scanned for simple `defun`s and installed in the
evaluator; `defun` parameters are bare symbols resolved from the call
scope (extended `eval_symbol`), and a buffer reference like `=goal`
evaluates to a marker `=GOAL` that `chunk-slot-value-fct` resolves to the
buffer chunk (which may be unnamed). Separately, a LHS `ISA TYPE` that is
the **only** test constrains the buffer chunk's type: a request-built
`(ISA CHUNK)` chunk must not satisfy `ISA GOAL`, or `bind-test` loops
forever. With other slot tests the `isa` line only carries slot indices,
so the tests decide (as before).


## 113. `+visual-location>` request parameters keep their colon (verified)

Models write request parameters with a leading colon (`+visual-location>
:attended nil`).  The loader keeps the name as `:ATTENDED` and parses
`nil` to `Value::Nil`, while the engine's feature matcher keys on the
bare slot name.  `loc_matches` must strip the leading `:` and treat
`Value::Nil` the same as the symbol `NIL` for `:attended` (unattended
only).  Until this was handled, `demo2` died with `visual-location slot
(":ATTENDED", Nil)`.  (`core-modules/vision.lisp` request-parameter
handling vs. our chunk-spec slot path.)

## 114. Bare-symbol slot values make undefined chunks (verified)

`get-true-slot-value-name` (`framework/chunks.lisp:1106`) calls
`create-undefined-chunk` for any alphanumeric, non-keyword symbol used
as a chunk slot value.  So `(add-dm (g value 1 color red))` creates a
slotless chunk `RED`.  A later `+retrieval> =c` (with `=c` bound to
`red`) runs `define-chunk-spec-fct (RED)`; since `(length specs) = 1`
and `RED` is a chunk, `chunk-name-to-chunk-spec` (`chunk-spec.lisp:473`)
copies its (empty) filled slots, i.e. the request matches *any* chunk.
That is why `retain-indirect-retrievals` retrieves `G` rather than
failing.  Our engine currently errors (`direct_retrieval` of a non-DM
name), so that spec is still blocked (the surrounding `:cia`/`:rir`
compilation trace is a separate, larger gap).

## 115. The demo2 task clears the window on a keypress (verified)

`tutorial/lisp/demo2.lisp`'s `respond-to-key-press` handler calls
`clear-exp-window`, so the clear is tied to the `output-key` action, not
to a scripted time.  `DeviceSetup::clear_on_keypress` models that in the
`OutputKey` handler: it queues a clear, re-runs `proc-display`, and
schedules an `EncodingComplete` for the last `move-attention` location
85 ms later.  Because that object is gone, `encoding_complete` now emits
`No visual-object found` (leaving `VISUAL` empty) instead of erroring.
The device text (`V` at 125,150) and the flag live in
`fixtures/devices/demo2.device.sexp`, loaded by the test harness.
Motor timing for `press-key V` required adding the QWERTY letter geometry
from `other-files/keyboard.lisp` (`V` = left index, r 1.0, θ 1.57).

## 116. `:randomize-time` is a whole-engine latency feature (verified)

`demo2-randomized` sets `(ssp :starting-parameters (… :randomize-time
t …))`, which makes *every* `randomize-time-ms` call (`random.lisp:541`,
`time/3` uniform jitter drawn from the model RNG) vary production action
time, visual encoding latency, and motor prep/execution.  Reproducing it
needs a matching RNG-draw order across modules, so `demo2-randomized` is
left out of the allowlist even though its device setup is identical to
`demo2`.

## 117. The spec extractor wrapped *runtime* `goal-focus` calls (verified)

`extract-specs.lisp` wraps `goal-focus-fct` and pushes a `:goal-focus` step
for every call.  Task commands call `goal-focus` *during* a `run`
(`ul-issues.lisp`'s `present-choose`), so the recorded steps gained a
`(:GOAL-FOCUS (ISA CHOOSE))` after the run and bumped the scope to
`:multi` — steps the engine replay was never meant to own.  Guarding the
wrapper with an `*in-run*` dynamic flag (set around `call-orig 'run`)
leaves only genuine driver-level focus calls.  The three `ul-issues`
specs were regenerated with the fix.

## 118. `run_secs` must leave future events queued (verified)

`run_secs` used to `pop` the first event and, if its time exceeded the
limit, **discard** it.  For a multi-`run` spec whose next event lies just
past the boundary (`ul-issues-second-run`: `(run 6.95)` then `(run 14)`,
with a command due at 7.000), the 7.000 event vanished and the second run
reported "no events left".  It now peeks: an out-of-range head stays in
the queue and fires on the next `run`, matching ACT-R's per-run bounding.

## 119. Model-body `schedule-event` + task commands (verified)

`ul-issues` drives itself: the model body ends with
`(schedule-event 0 "utility-learning-issues-choose" :output 'medium)`,
the task file pairs that name with a function via
`(add-act-r-command "utility-learning-issues-choose" 'present-choose …)`,
and `present-choose`/`show-result` chain `schedule-event-relative`,
`goal-focus`, `set-buffer-chunk` and `act-r-random` at runtime.
The engine now parses model-body `schedule-event`, resolves command names
through `add-act-r-command` aliases, and runs the task body with a mutable
evaluator (`m_eval`).  Two supporting details: `declare-buffer-usage`
supplies the default chunk type for `set-buffer-chunk` (the imaginal
chunk is `isa response`), and `(spp <prod> :reward R)` schedules a
`PROPAGATE-REWARD` at fire time, whose `:ult t` trace prints the
per-production utility update (formats from
`modules/utility-and-reward-1.lisp:441-461`).  `act-r-random` forced the
RNG into a `RefCell` so the `&self` evaluator can draw.

## 120. Temporal module: tick dynamics and cancellation (verified)

`test-temporal` needs the `TEMPORAL` buffer and `modules/temporal.lisp`'s
timer.  `+temporal> isa time` places `(isa time ticks 0)` (priority
-1000, after the request's implicit clear) and arms `next-time-tick`;
`+temporal> isa clear` stops it.  Interval details that matter:
- The one-time *start increment* is drawn only on the **first** request
  (`(when (null time-start-increment) …)`), so a restart reuses it — the
  first run and the post-`read` restart share one value.
- Each tick: `t *= 1.1`, then `t = max(1, t + act-r-noise(0.015·t))`,
  scheduled at `round(t)` ms (defaults `:time-noise .015`, `:time-mult
  1.1`, start `11 ms`).
- `:record-ticks t` traces `MODULE-MOD-REQUEST` + `MOD-BUFFER-CHUNK`;
  `nil` traces only `MOD-BUFFER-CHUNK` (and lets the armed CR wake).
- A new request `delete-event`s the pending increment.  Our `BinaryHeap`
  has no removal, so `temporal_request` uses `BinaryHeap::retain` to drop
  pending `TemporalTick`s — merely ignoring a stale tick still leaves it
  in the queue and delays the run's "no events left" terminator (and a
  stale tick would otherwise wake CR).  A generation counter makes a
  surviving stale tick inert.

`(spp <prod> :at S)` overrides a production's action time (`wait` fires
at 8.0 s, not 50 ms); parsed alongside `:reward`.

## 121. `:randomize-time` and the task's model-RNG draws (verified)

`demo2-randomized` enables `(ssp :starting-parameters (:randomize-time t
:vpft t :vidt t))`, but the model's `(clear-all)` wipes those
starting-parameters, so `extract-specs.lisp` never recorded them (the spec
said `:SGP NIL`).  Getting the ref requires:
- `randomize-time-ms(time)`: `time - round(time/3) + act-r-random(1 +
  2·round(time/3))` (default scale 3), gated by `:randomize-time`;
  `:vpft` applies it to production action times, `:vidt` to the imaginal
  delay, and vision encoding / the keypress re-encode always route
  through it.
- `randomize-time(seconds)`: `(time/3)·(2 + act-r-random(2.0))`, used for
  motor prep/exec/finish (`computing exec/finish` draws once each, in
  that order, at preparation-complete).
- The seed comes from the **model** (`(sgp :seed (123456 0))`), not the
  spec's recorded run seed.
- The task's `permute-list` (21-element letter list) consumes ~22 draws
  from the *model* stream before the first production, which this
  scripted driver never replays.  The `.device.sexp` carries
  `(CONSUME-RNG 21)`; 21 not 22 because our t=0 `proc-display` already
  draws once (`newest_feature`/`random-item`).  Without the offset the
  first production fires at 0.051 instead of 0.048.

Instrumenting ACT-R (wrapping `randomize-time[-ms]` under SBCL) pinned the
exact draw sequence, which is what made this tractable.

## 122. `retain-indirect-retrievals`: run 1 now matches, compiler still needed

Two findings from working the spec:
- **`+retrieval> =c` with an unresolved binding is an unconstrained
  request** (verified under SBCL: `define-chunk-spec-fct (list 'red)`
  expands to `(COLOR RED)`, matching `G`).  `start_retrieval` now treats a
  `NewChunk::Named` that is neither a DM nor a named chunk as an empty
  spec (match any) instead of `direct_retrieval`-failing.  With that, run 1
  (`:cia nil`, lines 1-85: the model plus the "not valid for compilation"
  messages) reproduces exactly.
- **Runs 2-3 need the `:cia`/`:rir` composition of indirect retrieval
  actions**, which the engine does not implement — the composed production
  instantiates the indirect action's variable (`color =c` → `color red`).
  Also unresolved: ACT-R's first chunk *copy* here is `G-1` (because
  `G-0` already exists from the retrieval), while `sji-merge` needs the
  first copy to be `-0`; our shared copy counter starts at 0, and starting
  at 1 regresses `sji-merge`.  `retain-indirect-retrievals` therefore stays
  out of the runnable set; the retrieval fix is committed as groundwork.

## 123. Retrieval copies its chunk; `+retrieval> =var` expands (verified)

Three coupled fixes made `retain-indirect-retrievals` **run 1 match
exactly** (lines 1-104):
- **`set-buffer-chunk` copies the retrieved chunk.** The retrieval buffer
  holds `G-0`, then `G-1`, … (one `copy-chunk-fct` per retrieval; verified
  by wrapping `copy-chunk-fct` under SBCL), while the trace shows the
  source name `G`.  A `=retrieval` reference is then used **verbatim** (no
  second copy), so `+imaginal> value =retrieval` yields `G-1`.
- **`named_chunk_shown` must see copies** — it now falls back to the
  `self.named` copy table, not just DM, so `+goal> =retrieval` still prints
  the referenced chunk's slots (`(COLOR RED VALUE 3)`).
- **`+retrieval> =var` expands the referenced chunk to its slot spec**
  (`chunk-name-to-chunk-spec`), retrieving by matching rather than by name.
  This is what makes the indirect `+retrieval> =goal` work once the goal is
  a spec-built (unnamed) chunk; `direct_retrieval` was removed.

No enabled spec regressed.  The remaining work is composed-production
instantiation (`color =c` → `color red`, and expanding the indirect
`+retrieval> =c` action to `+RETRIEVAL> COLOR RED`), so `retain` stays
disabled until that lands.

## 124. Composed-production instantiation + indirect-action expansion (verified)

`retain-indirect-retrievals` **run 2 now matches exactly** (lines 86-298).
`compose_pair` was extended:
- Only variables named by an *indirect* request (`+retrieval> =c`, not
  `+goal> =retrieval`) are instantiated from the firing bindings; other
  variables stay symbolic (`ppm-compilation` depends on this).
- An indirect request is expanded to the referenced chunk's spec:
  `chunk_spec_slots` = a real chunk's slots, or for an undefined chunk (a
  bare-symbol slot value like `red`) the slot that references it
  (`color=red`, matching `chunk-name-to-chunk-spec`).
- A `=buffer` reference resolves to that buffer's chunk at composition time
  (`G-1` for the retrieval `C` supplied; symbolic once p2's own request
  clears it).
- Query conditions (`?imaginal> state free`) join the composed LHS, and a
  slotless `=retrieval>` match is kept.
- The dropped-chunk doc uses the chunk p1's request actually produced
  (`G`, not the requested `red`), via a `copy_source` table.
- The composition gate is the per-buffer usage table
  (`buffer_usage` + `retrieval_composable`/`goal_composable`): RETRIEVAL
  `(4,12)` composes, GOAL `(12,44)` does not.
- `:cia` invalidates a production whose indirect-action variable is bound by
  a `!bind!`/`!safe-bind!` ("indirect action using a variable bound with a
  !safe-bind!").
- Compiled productions do **not** survive `reset` (run 3 starts fresh).

Run 3 (`:cia t :rir t`) still differs: `:rir` retains indirect retrievals
with its own rendering rules (`+RETRIEVAL> =C` vs `+RETRIEVAL> RED` vs
dropped) that are not yet modelled.  `retain` stays disabled.

## 125. `:rir` (partial): retain-buffer selection implemented

`retain-indirect-retrievals` run 3 now matches **PRODUCTION0-3** (lines
299-417).  Implemented `compute-retain-buffers`
(`production-compilation.lisp:2170`): with `:cia`+`:rir`, a retrieval-style
buffer is *retained* by whichever production requests it, per the p2 usage
code — `(4 12 20 28)` retains p2, `(8 24)` retains neither, else p1 if it
requests.  A retained request renders as `+RETRIEVAL> =var` (its variable)
and its variable is **not** instantiated; a non-retained one expands as
before.  Guarded by `cfg.cia && cfg.rir`, so no other spec is affected.

Still open (PRODUCTION4-7 + the I&J "cannot be composed" reason):
- `PRODUCTION4` (E&F) expects LHS `VALUE 5 / COLOR RED` (value before
  color) while `PRODUCTION0` expects `COLOR RED / VALUE 1` — the composed
  slot order is not a plain name sort, and `sort-production-slots` sorts by
  op then name, which does not explain it.
- `PRODUCTION4` also renders `+RETRIEVAL> RED` (neither `=C` nor
  `COLOR RED`), and `PRODUCTION7` carries a `!SAFE-BIND! =R (QUOTE RED)`
  condition in the composed LHS, plus the `I & J cannot be composed`
  message ("an indirect request with buffer RETRIEVAL uses a variable from
  a LHS !safe-bind!").
`retain` stays disabled.

## 126. `:rir`: run 3 matches, `retain` green

The tail of run 3 needed four more rules:
- **Substitute the union of indirect vars globally.**  The same variable on
  both sides (E's and F's `=c`) is resolved once, and ACT-R re-substitutes
  it across the whole composed rep.  Combined with **`remove-duplicates`
  keeping the *last* occurrence**, this is what turns `[COLOR =C, VALUE 5] +
  [COLOR =C]` into `[VALUE 5, COLOR RED]` — explaining why PRODUCTION4's
  goal slots are value-first while PRODUCTION0's are color-first.
- A **retained request renders its instantiated value** (`+RETRIEVAL> RED`)
  when its variable is also resolved elsewhere, otherwise `=var`.
- When **both productions request a buffer**, p1's request is superseded by
  p2's without a dropped-chunk note (`"H & I"`, not `"H & I - G"`).
- **LHS `!bind!`/`!safe-bind!` conditions join the composed LHS**, prepended
  and printed one line (`!SAFE-BIND! =R (QUOTE RED)`); and a non-retained
  indirect request whose variable is a LHS safe-bind blocks composition with
  `an indirect request with buffer RETRIEVAL uses a variable from a LHS
  !safe-bind!`.

`retain-indirect-retrievals` is now byte-exact across all three runs —
**37 specs** enabled.

## 127. `grouped`: PPM/finst retrieval traces + command eval

`grouped` needed several DM-trace details (all `:act t`, set by the test
stub's `:testing-function grouped-test`, which extraction missed — the spec
now carries `(:SGP :ACT T)`):
- **`!eval!` command dispatch**: `(add-act-r-command "name" 'fn)` is found
  recursively (it lives inside `grouped-recall`), and a string-headed
  `!eval!` resolves the command; `push` is accepted as untraced task
  bookkeeping.
- **`partial-matching` `:full` trace**: `Computing partial matching
  component`, then per testable slot `  comparing slot X`, `  Requested: =
  V  Chunk's slot value: W`, `  similarity: S`, `  effective similarity
  value is mp·S`, and `Total similarity score` (`declarative-memory.lisp:
  2590-2622`).  Variable-valued request slots (`parent =list`) must **not**
  be skipped — they are instantiated by runtime.
- **Finst removal**: `:recently-retrieved nil` traces `Removing recently
  retrieved chunks:` at act high followed by each removed candidate (DM
  reverse order, i.e. newest chunk first).
- **Threshold messages** at act low: `No chunk above the retrieval
  threshold: {rt}` (best below `:rt`) or `No matching chunk found retrieval
  failure` (no candidates) instead of `Chunk X … is the best`.
- **`buffer empty` is false while the buffer holds a failure**, so a
  production requiring an empty retrieval buffer does not refire after a
  retrieval failure (this is what ends `grouped` at 17.214).

`grouped` is now byte-exact (1192 lines) — **38 specs**.

## 128. `press-key` key→style: `punch` vs `peck-recoil`

`+manual> isa press-key key =k` is not one movement. The request chunk's
`cmd` is `press-key`, whose `handle-style-request` calls the `press-key`
*function* (`other-files/keyboard.lisp:541`), which looks up `key->cmd` and
dispatches the **underlying** style. Home-row keys (`a s d f j k l ;`,
`space`) map to `(punch :hand H :finger F)` (`keyboard.lisp:320-329`) — a
`defStyle punch () hand finger` (`motor.lisp:935`) with only 2 feature
slots, so a fresh movement prepares `num-to-prepare` = 3 features = 150 ms,
`exec = init + key-closure-time` (`motor.lisp:938`, 50+10 = 60 ms) and
`finish = init + burst + burst` (`motor.lisp:944`, 150 ms). Every other key
maps to `(peck-recoil :hand H :finger F :r R :theta T)` (5 features =
250 ms, Fitts-based exec/finish). `feat-differences` also differs:
`punch` is `hand→2 / finger→1 / 0` (`motor.lisp:950`), `peck` is
`hand→4 / finger→3 / r+theta` (`motor.lisp:970`). `demo2`'s `v` is a
peck-recoil; `z-threads`' `a` is a punch, which is why its first press
prepares in 100 ms and later same-key presses prepare in 0 ms.

## 129. Speech and the hear-self audio onset

The `speak` style (`speech.lisp:276`) prepares 3 features (150 ms), then
`exec-time = init-time` (50 ms) and `finish-time = exec + articulation`
(`speech.lisp:281/285`). At the request, the module schedules
`preparation-complete` at `+prep`; at that event it schedules
`initiation-complete` at `+init`, the hear-self sound at `+init` (its onset
equals exec), and `finish-movement` at `+exec+art`. This is what puts the
`SPEECH`-derived CRs at prep-complete and initiation-complete, and the
`AUDIO new-sound` at `exec` after prep (not at the request). `AURAL-LOCATION`
is set by `stuff-sound-buffer` only while the buffer is empty
(`audio.lisp:716`), so only the first sound of a run produces a
`SET-BUFFER-CHUNK AURAL-LOCATION` line even though `new-sound` repeats.

## 130. `buffer-chunk` task output is part of the ref

`z-threads`' task (`extras/threads/sample.lisp`) ends with
`(buffer-chunk goal)`, so its `.ref` has four non-trace lines after the
`Stopped` terminator. They are reproduced by a `Step::BufferChunk` driver
step (`:BUFFER-CHUNK GOAL` in the spec; recorded by the extractor's
`buffer-chunk-fct` wrapper). Two details: the header format is
`~S: ~S ~@[[~s]~]~@[ {~{~s~^ ~}}~]` (`framework/buffers.lisp:893`), i.e. a
trailing space before the `{...}` set; and the goal multi-buffer set comes
from `get-m-buffer-chunks` → `hash-table-keys` (`misc-utils.lisp:657`),
which `push`es while walking the table, so SBCL's insertion-order walk
yields the set in **reverse** insertion order (`{CHUNK1 CHUNK0 INITIAL0}`).

`z-threads` is now byte-exact (104 lines) — **39 specs**.

## 131. `dynamic-real-time`: a slack-hook no-op event, not a real-time clock

The ref is generated by the stub's `run-it`, which sets
`(mp-real-time-management … :slack-function 'simple-slack-test)` and runs
real-time; the slack function, on its first call with `next-time > 200`, does
`(schedule-event .5 'dynamic-time-test-dummy)`. That event is what bumps the
after-change conflict-resolution (`procedural.lisp:2329`) to 0.500 so the
time-dependent `test` production (`!eval! (> (mp-time) .4)`) can start firing
each cycle — after which `production-fired` schedules a fresh CR
(`procedural.lisp:2620`) and it loops.

The extractor deliberately forces `run` to non-real-time (`w-run`), so the
slack hook never fires during extraction and the spec cannot record it. The
observable effect is captured instead by a hand-added `(:DUMMY-EVENT 500)`
driver step (one per run), which schedules a hidden no-op event that wakes the
armed CR — the same bump, with the dynamic CR loop produced by the engine's
normal mechanism. The rest of the model is untouched.

`dynamic-real-time` is byte-exact — **40 specs**.

## 132. `act-touch`: the touch device + 8 motor styles

`act-touch` carries its task code in the model file (`touch-demo-model.lisp`,
which mixes top-level `add-act-r-command`/`defun` with `define-model`), so the
loader now tolerates top-level `add-act-r-command` and model-body
`set-visloc-default`. The stub's `run-touch-demo t` setup is not captured by
the extractor, so the spec carries hand-added steps: `(:EMIT-WARNING …)` (the
`start-hand-at-touch` banner), `(:START-HAND-AT-TOUCH RIGHT)`,
`(:MOD-FOCUS DEVICE T)`, `(:ADD-VISICON …)` and `(:MODIFY-VISICON 3000 …)`.

The subsystem (`extras/act-touch/act-touch.lisp`): a `punch`-like PM flow per
style, all `feat-prep-time`-scaled:
- `move-hand-touch` (touch-ply): exec = init + `fitts(cursor-coeff, r, w)`,
  finish = exec + burst; `w` is `approach-width` (the loc's width/height, or
  the 1° default → 19 px); dragging multiplies the coeff by `2^0.8`.
- `tap`/`tap-hold`/`tap-release`: exec from `fitts(peck-coeff, angle(ppi·height), 1)`.
- `swipe`/`pinch`/`rotate` (index-thumb): exec = init + max(burst,
  `fitts(peck-coeff, d, ppi·thumb-width)`), finish = exec + burst + that max,
  with `d` = r, `|start-end|`, `½·diameter·|rotation|`.
The device's `touch-*` signals are maintenance events; the task monitors
(`demo-point` etc.) print the `model … moved/tapped/…` lines. `approach-width`
and `pm-angle-to-pixels` round to integer pixels, which the timings depend on.

Two reference-semantics fixes fell out:
- A `=goal> … loc =visual-location` modify references the visual-location
  chunk **verbatim** — it is already a per-request copy, so no `-k` copy
  (otherwise the goal's loc became `CHUNK0-0-0`).
- A non-copy `=imaginal` reference names an unnamed spec chunk `CHUNK{n}`
  (`new-name-fct`), matching the ref's `LOC CHUNK1`.
- A visual-location chunk keeps the position it was stuffed with even after
  `modify-visicon-features` moves the underlying feature (`vl_xy`).

`act-touch` is byte-exact (527 lines) — **41 specs**.

## 133. `tracker`: the learning module and its single-float math

`tracker` (the `simple.lisp` choice-learning task) needs the tracker extra's
request/update machinery:
- A `+tracker>` request (`tracker.lisp:608`) generates the initial guess:
  `generate-tracker-choices` (uniform `min`…`max`), `tracker-prior-statistics`,
  then `tracker-genQuadValue` — a Boltzmann choice over the quadratic
  `params`, traced as `Tracker … generating value …` / `Choices` / `Probs` /
  `Chosen result is …`. The value is written to the control buffer with
  `schedule-mod-buffer-chunk` (priority -2000, under `TRACKER`).
- A `set-buffer-chunk`/`mod-buffer-chunk` watcher accumulates `good`/`bad`
  evidence (`Tracker … updating the current … value`).
- A scheduled `update-tracker` (`tracker.lisp:1298`) runs `update-stats` →
  `calc-quad` → `genQuadValue`, traced as `… updating the equation` /
  `new equation is …`. **Draw order differs by call site**: creation draws
  `genQuadValue`'s `act-r-random` *before* the next-update delay, but
  `update-tracker` draws the delay *before* `genQuadValue`.
- The math is **single-float** (`ms->seconds` returns one): the statistics and
  the quadratic fit must accumulate in `f32` or the 4th digit drifts. The
  temperature, however, is an exact rational in ACT-R
  (`init/(1+active/scale)`), so it must be computed in `f64` and coerced —
  `~f` then prints `0.8494733`, not `0.84947336`.
- `generate-tracker-choices` returns integers, and the control value keeps
  that type (`press-key key 1`, not `1.0`).

The stub's `run-test` setup is hand-added to the spec: a `(:TRACKER-SETUP …)`
step adds the `prompt` feature and arms the keypress→`correct`/`wrong`→
`choose` prompt cycling; the visicon feature carries its extra slots
(`prompt`) into the visual-location chunk. The spec's seed is the stub's
`(57531033409 0)`, not the recorded `(100 0)`.

`tracker` is byte-exact (322 lines) — **42 specs**.

## 134. `zz` test3: keyboard geometry, the dual-execution state machine, and two mouse positions

Test3 (`hold-key`/`hand-to-mouse`/`hand-to-home`) needs three things the
base motor module doesn't provide:

- **Virtual-keyboard geometry.** `key->loc` is the `(x, y)` grid coordinate
  from `specify-key` (`other-files/keyboard.lisp:243`); a finger's absolute
  location is `hand.loc + finger-offset` where the home locs are
  `#(4 4)`/`#(7 4)` (`motor.lisp:676`/`:682`) and the standard offsets are
  `index (0 0) middle (1 0) … pinkie (3 0) thumb (-1 2)`. `extended-finger-keyboard`
  (`motor-extension.lisp:2103`) then picks `hold-punch` (same loc) or
  `hold-peck` (move loc, polar `xy-to-polar`) — so `hold-key shift` is a
  `HOLD-PECK` (pinkie at `(1 4)`, shift at `(0 5)`) and `hold-key k` a
  `HOLD-PUNCH` (middle already at `(8 4)`).
- **The `hand-tracker` state.** `proc-s`/`exec-s` are per-hand lists, but
  `change-state` (`motor-extension.lisp:774`) collapses a hand-tagged change
  to **both** hands when `two-proc`/`two-exec` is nil. `initiation-complete`
  (`:1005`) frees the hand of the *current* `last-prep`, not the movement
  that just initiated — so a fast follow-up request can strand the earlier
  hand's `proc` busy (exactly the 0.500 `buffer-status`). Finger `busy` is
  set at preparation-complete (`:1186`) and cleared at finish (`:1205`);
  finger `down` flips at exec (`set-finger-down`), not at prep.
- **Two mouse positions.** The click/release trace prints `cursor-loc`
  (`mouse.lisp:178`, initial `(0 0)`, moved by `move-cursor`), but
  `hand-to-cursor` polar is computed from `cursor-pos` (`mouse.lisp:192`,
  default `(28 2)`). `dist((7 4),(28 2)) = √445 = 21.095022`,
  `atan2(-2,21) = -0.094951704` — and the return leg is the same `r` with
  `θ+π`. Compute the polar in `f64` then narrow to `f32`: SBCL's `atan` with
  integer args goes through the double path, and the `f32`-only `atan2` is
  one ULP off.
- `perform-movement` releases a *held* finger (key or mouse button)
  `key-release-time` after initiation (`:1171`), which is what makes
  `hand-to-home` emit `release-mouse` at 1.828.

`zz` test3 is byte-exact through line 2016 (test1 + test2 + test3); tests
4–9 remain.

## 135. `zz` test4: `move-a-finger` shifts the finger offset, not the hand

`hit-key` (`motor-extension.lisp:2153`) is `extended-finger-keyboard` with
`move-loc-action peck`; the same/move decision (`:2121`) compares
`key->loc` against `finger-loc-m … :current nil`, which is
`hand-pos-loc + finger-offset`. The two movement primitives differ:
`move-hand` (`motor.lisp:763`) changes `hand-pos-loc`, while `move-finger`
(`:712`) changes the **finger's offset** via `polar-move-xy` and leaves the
hand loc alone. Writing the hand loc (`loc = key - offset`) drags the whole
hand: after `hit-key i` (right middle, `(8,3)`) the right hand would sit at
`(7,3)`, so `hit-key n` (right index) computes `r = √5 = 2.236` instead of the
oracle's `√2 = 1.414`. The fix is `offsets[finger] = key_loc - hand.loc`,
applied at preparation-complete (the base `update-prepared` sets the
`next-loc` there, so the following request sees it). `RELEASE` is excluded
(it doesn't move a finger).

With hand locs pinned at left `(4,4)`/right `(7,4)` and only offsets moving,
test4's five hit-keys fall out exactly: `t` `r=√2`, `r` `r=1.0`, `i` `r=1.0`,
`f` `r=1.0`, `n` `r=√2` (the `√2` cases strike in 82 ms, the `1.0` cases in
77 ms). `zz` test4 is byte-exact through line 2120 (hit-key run).

## 136. `zz` test4: `hold-peck`'s exec and finish are different instants

`hold-peck` (`motor-extension.lisp:1909`) computes
`exec = init + movement-strike-time(move-time, r)` but
`finish = init + move-time`. They coincide only when
`peck-strike-distance` is unset, because `movement-strike-time` returns
`tt` unchanged when `peck-strike` is nil (`:1559`). So test3's `hold-key
shift` (`*psd*` nil) has `exec == finish == 0.450`, while test4's `hold-key x`
(`*psd* .2`, `r=1.0`, `move-time=100`, strike 77 ms) puts `output-key` at
1.527 and `FINISH-MOVEMENT` at 1.550. `hold-punch` has no such split
(`exec = init + key-closure`, `finish = init + burst`). `zz` test4 is
byte-exact through line 2124 (hold-key x).

## 137. `zz` test4: `point-finger`, the `pbreak` printer, and an invisible CR

`move-finger-to-home` (`motor-extension.lisp:2222`) resolves to `point-finger`
when the finger is off home. `point-finger` (`:2036`) is the odd style out:
its exec is `max(init-time, move-time)` (not `init + strike`), it emits no
`output-key`, and its `set-hand-position` (`:2053`) carries **no `:details`** —
yet it still wakes conflict resolution, so a bare `CONFLICT-RESOLUTION` lands
at exec (1.750 in test4). Model that as an invisible, non-maintenance event at
exec; `run_secs` calls `changed()` for any non-CR event regardless of `Out`.

The `test4-7` break (`procedural.lisp:2318`) prints the selected production
with its bindings, then `BREAK-EVENT PRODUCTION TEST4-7`. The printer
(`print-production-text`, `:2441`) drops the `isa` slot from `=buf>` tests and
renders a `+buf>` request's chunk-type as `CMD <type>`; `pbreak` halts the run
before the buffer reads, so the driver's `buffer-status` + `finger-loc-m` dump
lands inline, and the second `(run 10)` resumes with `BUFFER-READ-ACTION GOAL`.
`zz` test4 is byte-exact through line 2211 (the second run's first
`CONFLICT-RESOLUTION`).

## 138. `zz` test4: `all-fingers-to-home` is a two-handed style

`all-fingers-to-home` (`motor-extension.lisp:2257`) is the only movement whose
`hand` is `both`, which needs a `both` flag through the exec bookkeeping:
`maybe-execute-movement` requires *both* exec stages free (`:1106`), and
`finish-movement-dual` frees both (`:1201`). Its prep is `num-to-prepare`, not
`num-possible-feats`: `num-to-prepare :around` (`:2265`) drops the direction,
and `theta` is unset, so the count is `hand` + `r` = 2 (0.100 s) even though
`num-possible-feats` is 5. Its exec is
`max(init, randomize(max(.2, fitts(mouse-coeff, r, 4.0))))` and finish is
`init + exec` (`:2349`), so with both hands already home (`r = 0`) exec is the
0.2 s floor. `queue-output-events` (`:2363`) emits an
`ALL-FINGERS-TO-HOME <hand>` details line per hand at exec; a held finger (here
`x`) is released at `init + key-release-time` (2.040), and both hands are reset
to the home row.

`zz` test4 is **byte-exact through line 2310** — test1–test4 plus every
summary. Tests 5–9 remain.

## 139. `zz` test5: the run loop, direct styles, and release/release-recoil

`test5` (`test-model.lisp:188`) drives `(do ((time (run 10) (run 10))) ((= time
0.0)) ...)`: each `(run 10)` halts at a `pbreak` (test5-5/5-9/5-11/5-12) and
the body dumps `buffer-status` + finger locations after every run that
processed anything. The terminator is a run that executes **zero** events (the
queue is already empty), which still prints `Stopped because no events left to
process` — so the last dump is followed by one extra stop line.

Direct style requests (`isa hold-punch hand left finger ring`) go through
`handle-style-request` (`motor.lisp:1447`), which traces
`<STYLE> HAND <h> FINGER <f> [R <r> THETA <theta>]` before preparing. The
device names `output-key` from the **finger's location** (`motor.lisp:960`),
not a request slot, so a direct `hold-punch` (which has no `key`) outputs the
key under the finger.

`release` and `release-recoil` are the mirror of punch/peck: no `output-key`,
a `release-key` at **exec** (`init + key-release-time`,
`motor-extension.lisp:1953`/`:1991`), and `release-recoil`'s finish is
`init + move-time + burst`. `release-key-to-home` (`:2183`) is same-loc only:
it emits `release` when the finger is home, else `release-recoil` with the
polar vector to home (and does nothing when the finger is off the key).
`reset-hand-tracker` (`:1263`) seeds `last-command` to `NONE`, and the
`pbreak` printer lists a `+buf>` request's slots **before** its `CMD <type>`.

`zz` test5 is byte-exact through line 2519 (test5-7); `prepare`/`execute`
remain.

## 140. `zz` test5: `prepare`/`execute`, `point-hand-at-key`, and the `hand` slot

`prepare` (`general-pm.lisp:562`) builds a `hand-ply`, sets `exec-immediate-p`
nil, and prepares it; `preparation-complete` (`motor-extension.lisp:1019`)
therefore does **not** queue it — it frees the processor and waits. `execute`
(`:580`) appends the *same* instance to the exec queue, so its
`exec`/`finish` times are re-based to the `execute` event, not to
prep-complete (2.700 → INIT 2.750, `MOVE-A-HAND` 2.850, FINISH 2.900).

The `PREPARE` trace is not an explicit details string: `pm-module-request`
(`motor.lisp:1423`) schedules `'prepare` with no `:details`, so
`format-event` (`events.lisp:263`) falls back to action + params. That prints
the request's **raw** slot values, so `r 1` is the integer `1` and `theta 3.14`
is `3.14` — unlike a computed polar, which prints `1.0`/`-1.5707964`. `r`/`theta`
are therefore pre-rendered per movement, not formatted from the `f32`.

`point-hand-at-key` (`keyboard.lisp:653`) is a front-end for `point-hand`
(`motor.lisp:1149`), which makes a **`hand-ply`** (style-name `PLY`). So its
prep is `feat-differences` against the preceding `prepare`'s `hand-ply`, not
`num-to-prepare`: only `theta` changed (1 feature, 0.050 s). Its
`MOVE-A-HAND` details use the polar computed from the hand loc to the key, and
`hand-ply` moves the hand **loc** by `polar-move-xy` (`misc-utils.lisp:1142`,
each axis rounded) — the finger offsets are untouched, so only the hand loc
changes.

In the extension, `hfrt-movement` is **redefined** to inherit
`hf-movement` → `dual-hand-movement-style` (`motor-extension.lisp:1633`), so
`hand-ply` and `all-fingers-to-home` get `release-if-down :free` and
`finger-based-style t` even though `motor.lisp:968` defines `hfrt-movement` off
plain `movement-style`.

`all-fingers-to-home` (`:2401`) honors the `hand` slot: it moves only the
requested hand(s) that need it, and the style's `hand` is `both` only when
**both** requested hands need moving, else the single hand. test5-12's
`hand right` therefore moves only the right hand and updates only its
`last-command`; a no-slot request (test4/test8) is `both`.

`zz` test5 is **byte-exact through line 2893** — test1–test5 plus every
summary. Tests 6–9 remain.

## 141. `zz` test6: `type-key`, output-key scheduling, and finger freeing

`type-key` (`motor-extension.lisp:2142`) is `extended-finger-keyboard` with
`same-loc-action punch` / `move-loc-action peck-recoil`: from a non-home
position it strikes the key and **recoils back to where the finger started**.
So `peck-recoil`'s `prepare-features` (`:1776`) leaves the finger offset
unchanged — the engine must not move it to the key the way `peck`/`point-finger`
do. Its `compute-finish-time` (`:1792`) is
`init + first-move-time + burst + max(burst, move-time)`: a second full move leg
back, unlike `peck`'s `init + move-time + burst`.

The keyboard device emits `output-key`/`release-key` by **scheduling a separate
event** (`keyboard.lisp:503` `schedule-event-now "output-key"`), not inline in
`set-finger-down`/`set-finger-up`. So at one time, a movement's key output lands
*after* any event already queued at that time: test6 has a `preparation-complete`
(queued at 0.450 by the type-key request) print before the `peck-recoil`'s
`output-key` (whose `set-finger-down` was queued earlier at 0.450 by the
perform, but which only schedules the output when it runs at 0.600). Emitting
the trace inline in the engine gets the order backwards.

`finish-movement-dual` (`:1200`) frees the movement's fingers **before**
`maybe-execute-movement` starts the next movement on that hand. The engine
models this as the finish handler clearing the hand's finger-busy set before
re-dispatching; a separate free event scheduled at the finish would run after
the next movement re-marks the same finger busy.

The dual-execution module also exposes per-finger queries
(`?manual-left> index free`/`down`, `motor-extension.lisp:1306`), backed by the
hand tracker's finger-busy / finger-down state.

`zz` test6 is **byte-exact through line 3034** — test1–test6 plus every
summary. Tests 7–9 remain.

## 142. `zz` test7: `delayed-punch` and its three delay parameters

`delayed-punch` (`motor-extension.lisp:2647`) subclasses `punch`: the strike is
still `init + key-closure-time`, but the **release** is `init + kct + delay`
(exec2, `:2657`) and finish is `init + kct + delay + burst` (`:2664`) — the
`delay` replaces `key-release-time`, it does not add to it. `delay` is parsed by
`parse-delay-time` (`:2685`): nil/`default` → `default-punch-delay`, `fast` →
`fast-punch-delay`, `slow` → `slow-punch-delay`, a positive number as-is.
The request trace is a `schedule-event-now 'prepare-movement` with explicit
`:details` (`:2713`), so it prints `DELAYED-PUNCH HAND <h> FINGER <f> DELAY <d>`
with the parsed delay via `~f`; like the direct styles it appears before the
request's `CONFLICT-RESOLUTION`.

The three delays are model parameters (`:default-punch-delay`/`:fast-`/
`:slow-punch-delay`, `:1482`) set from the `*dpd*`/`*fpd*`/`*spd*` specials.
`test7` (`test-model.lisp:212`) runs twice, changing those specials for the
second run — and `define-model` re-reads them on `reset`, so the driver-set
delays must survive a reset (the engine keeps them alongside the other `zz`
driver params).

`test7`'s goal is `(define-chunks (test test7))` — no `isa` — so ACT-R
auto-names it `CHUNK0` with **no chunk type** (`chunks.lisp:1192`: `new-name-fct
"CHUNK"`). The production's `isa goal` still matches because a `ISA` test that
is not the production's *only* LHS test is ignored
(`match_chunk_tests`, `only_isa`).

`zz` test7 is **byte-exact through line 3226** — test1–test7 plus every
summary. Tests 8–9 remain.

## 143. `zz` test8/test9: `release-all-fingers` vs `all-fingers-to-home`

`release-all-fingers` (`motor-extension.lisp:2512`) subclasses `punch` with
`hand both`, `finger :dummy`, `release-if-down :free`, and
`feature-slots '(hand)`; its `num-to-prepare :around` (`:2524`) drops the hand,
so prep is 1 feature (0.050 s). Its exec is `init + key-release-time` and
finish `init + 2*burst` (`:2577`/`:2582`), and it emits a
`RELEASING-ALL-FINGERS` line at exec (`:2610`) — **not** a per-hand line like
`all-fingers-to-home`'s `ALL-FINGERS-TO-HOME <hand>`. Crucially it does **not**
move the fingers: its `prepare-features` copies the hand position with the
extended-finger states cleared but the locations unchanged, so test9's held
keys (`z`/`o`) leave their fingers off the home row, while test8's
`all-fingers-to-home` resets both hands to home.

The handler (`:2616`) only prepares a movement when some finger is down;
otherwise it calls `print-warning "Release-all-fingers action called when no
fingers need to be released."` and does nothing — test9's second request
(test9-5). Its request trace uses an explicit `:details "release-all-fingers"`
string (`:2636`), so it stays **lowercase** unlike the symbol-name traces.

At exec, the auto-release's `set-finger-up` events are scheduled before
`releasing-all-fingers` (`perform-movement` releases fingers, then calls
`queue-output-events`), but each `set-finger-up` only *schedules* its
`release-key`, so `RELEASING-ALL-FINGERS` still prints before the releases —
the same deferred-output ordering as PORTING-NOTES #141.

`zz` test9 is byte-exact through line 3568; with it the whole
`zz-extended-motor-actions` spec (3568 lines) passes and is now in
`enabled.txt`.

## 144. `associative-learning`: the ref *is* reproducible — `:starting-parameters`

The spec records seed `(100 0)`, but that is `extract-specs.lisp`'s
**canonical stand-in** for a run whose effective seed did not match a literal
`:seed` in the sources (`canon-runs`); it assumes such a run is RNG-independent.
This test is not: `run-it` → `approach-1-results` calls `create-stims`, whose
`permute-list`s fix every stimulus. The real seed is `(1 0)`, from the stub's

```lisp
(ssp :starting-parameters (:seed (1 0) :model-warnings nil :testing-function set-testing-traces))
```

`:starting-parameters` **is** a valid system parameter
(`central-parameters.lisp:131`), and its reset handler re-applies the stored
params (`(sgp-fct *user-default-parameter-list*)`, `:143`) — so `:seed (1 0)`
is set on **every** model reset. That is the crux: `approach-1-results` runs
`(create-stims)` *before* `(reset)`, so `create-stims` consumes the stream left
by the model's `define-model` reset, then `reset` **reseeds** to `(1 0)` and
`initial-presentation` starts a fresh `(1 0)` stream. Both halves matter: the
people/places permutation comes from the pre-reset stream and the sentence
permutation from the post-reset one. `permute-list` (`random.lisp:562`) conses
each pick onto the front, so the result is the *reverse* of the pick order.

`(buffer-requires-copies 'imaginal)` (`al-fan-study.lisp:201`) turns off the
imaginal buffer's reuse flag, so each `set-buffer-chunk` spec is converted with
`define-chunks-fct` and gets a fresh auto-named `CHUNK{k}`
(`buffers.lisp:1360`) — hence `CHUNK0…CHUNK25` — while the trace still prints
the spec/chunk-def (`buffers.lisp:1432`).

A task-level `module-request` dispatches straight to the module
(`request-module`, `modules.lisp:819`): no `MODULE-REQUEST` trace line and no
conflict-resolution arming (the module's later set/clear does that). Only a
production's request goes through `schedule-module-request` and traces.

A0 is byte-exact through line 131 (the 26-sentence initial presentation);
`drop-out`/`time-to-retrieve` remain (A1–A4).

## 145. `associative-learning`: the module, and SBCL's double-then-round floats

The learning state is the module's extended per-chunk slots
(`al-ni-count`/`al-ni-refs`, `al-cj-count`/`al-cj-refs`,
`al-ni+cj-count`/`al-ni+cj-refs`, `associated-js`;
`associative-learning.lisp:205`+). Three hooks drive it: a source buffer's
clear schedules `al-buffer-cleared` at **priority 5**
(`al-check-cleared-chunks`, `:562` — after the DM merge, before a retrieval's
`-2000`), `al-chunk-retrieval-request` (`:644`) snapshots the source-buffer
chunks, and `al-chunk-retrieved` (`:655`) calls `update-al-parameters`
(`:712`). Enabling `:eal` sets `:mas` to π (`al-params`, `:421`) and installs
`compute-learned-sji` as the DM `:sji-hook`, so `compute-sji`
(`declarative-memory.lisp:2536`) calls it per *source* `j` of each spreading
buffer — the trace order is the buffer's source order, and
`compute-learned-sji`'s printed values are `exp(compute-references …)`.

The Sji arithmetic is where the refs pin down SBCL's float behaviour. ACT-R's
`expt-coerced`/`exp-coerced`/`log-coerced` (`misc-utils.lisp:515`/`:554`/`:580`)
are *no-ops* when `*read-default-float-format*` is `single-float` — they call
plain `expt`/`exp`/`log`. But SBCL computes those on a single-float argument in
**double precision and rounds the result to single**, which differs from Rust's
`f32::powf`/`f32::exp`/`f32::ln` by an ulp. Several Sji values are off by
1 ULP until the `ol t` branch of `bl_equation` computes
`(exp (* y (log x)))` and the `log` in `f64`, and the caller exponentiates via
`exp_coerced` (and the final `log_coerced`) — matching SBCL exactly.

With that, the whole two-pass drop-out (208 Sji traces) is byte-exact through
line 3339.

`time-to-retrieve-at-start` (`al-fan-study.lisp:382`) then sets a
`comprehend-sentence` context chunk in the imaginal buffer and, for each target
and foil, runs `sdm-fct`/`sdp-fct` (`dm-commands.lisp:491`/`:667`): the
activation is base-level + spreading with an *empty* request spec (no partial
matching, and `:ans` defaults to nil, so no noise), and `sdp :activation`
recomputes it (`:738`) — the 324 final Sji traces. The source order is again
`chunk-filled-slots-list-fct`: the typed context chunk's internal list is
reverse-insertion `(arg2 arg1 relation)`, while a bare request spec is reversed
by `chunk-spec-to-chunk-def` and so comes out in written order. The results
table is `correlation`/`mean-deviation` (`act-gui-interface.lisp:1567`/`:1600`,
`~6,3F`) over the 18 averaged cells, and `*task-intercept*` is 0.845 s.

`associative-learning` (7242 lines) is byte-exact and now in `enabled.txt`
(44 specs).

## 146. `test-mp-value-req-param`: request params, DM scan order, reset warnings

A request spec can name a chunk: `+retrieval> (b :mp-value 2)` is the chunk
`b` used as the request (its slots become the request slots) plus request
parameters (`define-chunk-spec-fct`, `chunk-spec.lisp:434`). `:mp-value`
overrides `:mp` for that request (`nil` disables partial matching); an invalid
value or a repeat aborts the request with `Invalid retrieval request.` and a
second warning.

The `:act` trace pins the DM scan order. `dm-chunks` is reverse-insertion
(`chunk-list`), and the exact-match scan walks it directly while partial
matching uses `(nreverse chunk-list)` (`declarative-memory.lisp:1308`/`:1349`),
so the match/no-match lines come out newest-first but the partial-match
activation lines come out oldest-first — and all the match/no-match lines are
printed *before* any activation line (`:1314`).

Production validation runs on the model body, which ACT-R re-evaluates on
`reset`; a rejected production's warning block (e.g. a request parameter with a
modifier other than `=`) therefore reappears at each reset, not at load — the
load-time warnings are dropped because the tester installs its warning monitor
*after* `(load file)` (PORTING-NOTES #111). `test-mp-value-req-param` (477 lines) is
byte-exact and now in `enabled.txt` (45 specs).

## 147. `test-bad-productions`: validator warnings are reset-scoped, not load-scoped

PORTING-NOTES #111 is wrong on the mechanism. The `Creating chunk …` and rejection
warnings are not dropped on load and surfaced later by global suppression
state — they are re-emitted every time ACT-R re-evaluates the model body,
which `reset-model` does (`model.lisp:983`). A spec whose driver never resets
(`semantic`, `tutorial-unit1-tutor-model-trace` — both `:RESETS 0`) shows
none, while a spec with a `(:RESET)` step shows them once per reset
(`tutorial-unit1-semantic-warnings` is `semantic` with `(reset)` as its whole
driver and *does* show `CATEGORY`/`PENDING`/`YES`/`NO`). The `reset_warnings`
mechanism built for `test-mp-value-req-param` (PORTING-NOTES #146) is the right one;
`test-bad-productions` just needs the full ordered block.

The block is assembled from the production scan in file order
(`production-parsing.lisp`):
- the `sort-for-bind-order` (`:1810`) reorder warning (BAD-ORDER's `!eval!`
  precedes its `!bind!`);
- `create-undefined-chunk` (`chunks.lisp:850`) for RHS/LHS slot values that
  name no chunk. `nil` is skipped — the `eq t` guard at `:1380` is the only
  literal one, yet BAD-ORDER's `slot1 nil` yields no `NIL` warning. The
  module-default chunks (`create-model-default-chunks`, `model.lisp:755` —
  `FREE`/`BUSY`/`EMPTY`/`FIND-LOCATION`/…, plus the touch and
  extended-motor-actions extras' command names) count as defined; demo2's
  `step find-location` does not warn for exactly this reason;
- one rejection block per invalid production: `bad-production-exit` (`:397`)
  prints `No production defined for (NAME …).`, the specific reason, the
  *reversed* captured `define-chunk-spec` warnings, then
  `--- end of warnings …`. `bad-condition-exit`/`bad-action-exit` feed their
  message into the capture, so for BAD-BUFFER-NAME the order is
  `Invalid syntax …` then `Invalid specs in call to define-chunk-spec …`.
  `MISSING-ISA-REJECTED` is accepted. The production text is `(NAME <rest>)`
  printed like `~s` (the existing `sexp_lisp`).

The validator is now total rather than erroring: the engine warns and skips.
`test-bad-productions` (31 lines) is byte-exact and in `enabled.txt` (46
specs).

## 148. The model's `(sgp :seed …)` is the real seed — `emma` is reproducible

The extractor's per-spec `:RUNS` seed is a stand-in, not what ACT-R used. The
seed a `.ref` reflects is the model body's `(sgp :seed (S C))`
(`random.lisp:417`), applied while the model is defined. `emma` records
`(100 0)` but `demo2-model.lisp` sets `(123456 0)`; forcing `(100 0)` gives a
different trace (`#(427 507 1080)` vs the ref's `#(320 522 1080)`), while the
model's `(123456 0)` reproduces `emma.ref` byte-for-byte — and twice in a row,
so the draws are fully seeded, not wall-clock. `Config::from_model` now parses
`:SEED` and the engine applies it on `reset` (see PORTING-NOTES #152): the model body
is re-evaluated there (`model.lisp:983`), and the task's `consume_rng` draws
follow, which is equivalent for the single-run specs here and correct for the
per-run loops.

`demo2` and `emma` share `demo2-model.lisp`, but `demo2` is seed-independent
because `demo2.spec` has a device sidecar (`ADD-TEXT 0 "V" 125 150`); `emma`
has none, so its letter pick *and* its saccade noise both consume the stream.

## 149. `emma`: the EMMA vision extension

EMMA (`extras/emma/emma.lisp`) replaces the vision module's fixed-latency
encoding with a saccade state machine. `move-attention` calls the
`:visual-encoding-hook` (`vision.lisp:3021`); EMMA sets it to
`emma-attention-hook` and returns `t`, so the base encoding schedule is
suppressed and `initiate-eye-move` runs instead: `PREPARATION-COMPLETE` at
`compute-prep-time`, then `INITIATION-COMPLETE` at `min(exec-time, init-time)`
and `COMPLETE-EYE-MOVE` at `exec-time` (`emma.lisp:653`), and
`complete-eye-move` re-fixates while `encoded` is nil. `exec-time =
init + r·sacc-rate + base` with `r` from `angle-between-vectors`, and
`prep-time` is `3` features on the first saccade then `feat-differences`
(`distance=` 2°, `direction=` π/4). The only randomness is
`add-gaussian-noise` on the destination — two logistic draws per saccade from
the model's seeded MT19937. `update-attended-loc` re-invokes the same hook
when the window changes under the attended location (`vision.lisp:1915`),
which is what starts the final saccade after the keypress clears the window.

Two things the engine needed beyond the machine:
- the visicon text coordinates: `screen-x = window-x + text-x + 5`,
  `screen-y = window-y + text-y + 6` (the default window origin 300,300 gives
  the `+305/+306` seen in the ref) and `distance = viewing-distance ×
  pixels-per-inch = 1080`;
- EMMA's `current-marker` behaves as `#(0 0 1080)`, not the class default
  `#(0 0 1)` — with z=1 the Fitts distance (and hence the noise scale) is
  wrong and the ref's `#(320 522 1080)` cannot be reproduced.

`emma` (81 lines) is byte-exact and in `enabled.txt` (47 specs); it also
settles PORTING-NOTES #148's claim that the ref is fully reproducible.

## 150. Device stubs are extracted by executing them; the vision auto-unstuff

`extract-specs.lisp` used to classify any stub whose *text* mentioned a
device token (`install-device`, `open-exp-window`, …) as out of scope and
write `:STEPS NIL` **without running it** (`extract-one`). The `.ref` was
always the oracle; only the replay script was lossy. Now device stubs are
executed like any other under `force-virtual-window`, with wrappers on
`add-text-to-exp-window` / `clear-exp-window` capturing the window script,
which is emitted as `<name>.device.sexp`. A hand-authored sidecar is never
overwritten (it carries knobs the extractor cannot derive, e.g.
`CONSUME-RNG`). Because the task re-applies its setup per run, the engine
re-arms the script on every `reset` (`pending_texts`/`pending_clears` are
consumed as time advances), and the emitted script is de-duplicated.

Three engine pieces this exposed:
- `(goal-focus free)` targets a chunk ACT-R defines for every model:
  `create-model-default-chunks` (`model.lisp:753`) adds `free busy error
  empty full failure requested unrequested` (`(x name x)`) and `clear`. The
  engine now seeds these into `named` before the model's own chunks.
- The vision module auto-stuffs an unused visual-location and then unstuffs
  it after `new-span` (`stuff-visloc-buffer`, `vision.lisp:1995`;
  `unstuff-buffer`, `general-pm.lisp:896`), erasing the buffer
  (`ERASE-BUFFER VISUAL-LOCATION`). The precondition
  `check-unstuff-buffer` (`vision.lisp:1944`) requires the buffer to still
  hold the *unchanged, unrequested* copy, so a production's `=visual-location>`
  RHS (`mod-buffer-chunk`) cancels it — the engine tracks a `modified` flag.
  `:unstuff-visual-location` may be `nil`/`t`/a number (`act-touch` sets it
  `nil`, which is why its VL is never erased).
- ACT-R drops an event whose precondition fails **before** the run's stop
  check (`skip-unnecessary-events`, `scheduling.lisp:588`; it checks only the
  *next* event). Without this the stale unstuff kept the run alive to 0.5s
  instead of stopping at the last real event.

`precondition-test` (20 lines, two `run-test` passes: focused and unfocused)
is byte-exact and in `enabled.txt` (48 specs).

## 151. DRIVER stubs: `pdisable`/`penable` capture and selection filtering

`extract-specs.lisp` also skipped DRIVER stubs from source. They are now
executed too (only COMMAND stubs are still classified without running), with
`pdisable-fct`/`penable-fct` wrapped so the calls become `(:pdisable ...)` /
`(:penable ...)` steps. `tutorial-unit1-tutor-model-partial-trace` is the
small DRIVER case: it disables four productions of the already-supported
tutor model so the trace stops after `ADD-ONES`.

`pdisable-fct`/`penable-fct` (`procedural-cmds.lisp:541,568`) set a
`production-disabled` flag; `conflict-resolution` skips flagged productions
(`procedural.lisp:2195`). The engine keeps a `disabled_prods` name set and
filters in `conflict_resolution`. A production name list is applied as-is;
an empty list means all. `reset` clears the set because ACT-R recreates the
productions from the model body (a model-body `pdisable-fct`, as in
`dynamic-production-compilation-tests`, would therefore re-apply).

`tutorial-unit1-tutor-model-partial-trace` (17 lines) is byte-exact and in
`enabled.txt` (49 specs).

## 152. `:act` base-level trace, `:rt-value`, model-body `pdisable`

Making `retrieval-request-parameters` runnable (DRIVER stubs are executed
now) surfaced three general pieces:
- the full `:act` base-level trace when `:bll` is set: `Starting with blc`,
  `Computing base-level from N references (...)`,
  `  creation time: … decay: …  Optimized-learning: …`, `base-level value:`
  before `Total base-level` (`compute-references`,
  `declarative-memory.lisp:2695`). The engine previously emitted only
  `Computing base-level` → `Total base-level`.
- the `:rt-value` retrieval-request parameter (`declarative-memory.lisp:1292`):
  a number overrides `:rt` for that request.
- model-body `(pdisable …)` / `(penable …)` (and `(pdisable-fct
  (all-productions))`): re-applied on every `reset`, because ACT-R
  re-evaluates the model body (`model.lisp:983`). This is what makes the
  per-production driver loop run exactly one production.

`retrieval-request-parameters` is byte-exact and in `enabled.txt` (50 specs).
The remaining pieces it needed:
- `:bll-value`/`:mas-value`/`:ans-value` request parameters
  (`declarative-memory.lisp:1243`): `nil` disables that term for the request;
  `spreading_activation` now takes the effective `:mas`.
- the retrieval-failure delay uses the request's `:rt` (`failure_wait`), not
  the model's, so a request with `:rt-value 10` fails at once.
- the model-body seed and the task's `consume_rng` draws are applied **on
  every `reset`** (ACT-R re-evaluates the body, `model.lisp:983`; the task
  redoes its setup per run). This supersedes the "construction only" note in
  PORTING-NOTES #148 — it is equivalent for the single-run specs and correct for the
  per-production loop here.
- the goal buffer is a `reuse?` buffer (`buffers.lisp:1349`), so its chunk is
  the reusable copy `GOAL-CHUNK0`. Implementing the copy itself is entangled
  with the engine's name-based `=buffer>` bindings and regresses
  `indirect-action-compilation` (retrieval of the source chunk, slot order)
  and `z-threads`, so the engine keeps the source chunk and reports
  `GOAL-CHUNK0` in the spreading trace only (the one place the copy name
  leaks). This is a documented approximation, not full fidelity.

## 153. Same-name `define-model` redefinitions are one model

`is_multi_model` counted *every* `define-model` in a spec's files, so the
tutorial specs that load a base model and then a `-vN` file defining the
same model were mis-flagged as multi-model (an extra trace column). ACT-R
redefines the model in place, so only *distinct* names produce the column;
the last definition wins (`run_model_steps` already used `defs.last()`).

This is what unlocks `tutorial-unit7modeling-ul-issues-v1-first-run`
(24 lines): `ul-issues.lisp` + `utility-learning-issues-model.lisp` +
`utility-learning-issues-model-v1.lisp` all define `utility-learning-test`,
and the engine reproduces the utility-learning trace (PROPAGATE-REWARD and
the `Utility updates with Reward …` output). It is byte-exact and in
`enabled.txt` (51 specs).

The same fix surfaces two more redefinition specs that still need engine
work: `tutorial-unit3modeling-v2-run` (device + a scheduled `pm-issue-display`
command line) and `tutorial-unit7modeling-pcomp-issues-v2/-v4-one-trial`
(loader `set-all-base-levels`, then a device).

## 154. `set-all-base-levels`, and the extractor's run-trace flag

Two pieces the pcomp redefinition specs needed:
- model-body `(set-all-base-levels <level> [creation-seconds])`
  (`commands/dm-commands.lisp:600`): a fixed base-level history for *every*
  DM chunk (a later named `set-base-levels` overrides). The engine's `reset`
  applies it to all history entries.
- `w-run` recorded the model's `:v` as the run's trace flag, but the
  trace-reference harness captures the model-trace via monitors regardless of
  `:v` (which only gates the stdout echo). `production-compilation-issues-
  model-v2` sets `:v nil`, so its extracted spec had a NIL trace and the
  engine emitted nothing. `w-run` now records `T` unconditionally — a no-op
  for every spec that already had `T`.

`tutorial-unit7modeling-pcomp-issues-v2-one-trial` is now runnable and matches
the ref through line 17 of 27; it diverges at `no-past-trial`'s
`+retrieval> isa response` (the engine retrieves a number chunk, the ref a
`RESPONSE-n`), which is a retrieval candidacy/`isa` question to settle.
`-v4-one-trial` is runnable too. Both are committed as in-progress specs;
`probe_runnable` reports 51 pass / 3 fail.

## 155. `isa TYPE` in a chunk-spec adds the type's default slot tests

The pcomp retrieval divergence above was candidacy, not ordering. `isa TYPE`
does **not** store a type on the chunk-spec (`act-r-chunk-spec`,
`internal-structures.lisp:532`, has no type slot); `define-chunk-spec-fct`
routes it through `process-slots-specs` (`chunk-spec.lisp:441`), which adds
the chunk-type's default slot tests for any slot not already tested
(`chunk-spec.lisp:597`). So `+retrieval> isa response` on
`(chunk-type response key (is-response t))` really requests
`is-response = t`, which is what filters the candidates to the response
chunks. The engine now expands a request's `ctype` to its chunk-type defaults
in `start_retrieval`. With candidacy fixed, pcomp v2 retrieved a response
(18/27 lines); the remaining divergences are #156.

## 156. `update-attended-loc`, the busy visual module, and the task's RNG draws

The last pcomp v2 divergences, each pinned by logging ACT-R's `act-r-random`
calls and reading `update-attended-loc` (`vision.lisp:1870`):

- A window **clear** under the attended location re-runs its encoding
  (condition [1]: "looking at an object and it's gone"). The re-encode
  occupies the visual module, so `?visual> state free` fails until it lands
  — without this, the CR at the window change fires `detect-feedback`
  85 ms early (the `:crt` trace shows `The STATE FREE query of the VISUAL
  buffer failed`). The engine now schedules the re-encode from
  `proc_display` and sets `visual_busy`.
- A plain **text add** elsewhere must *not* re-encode (`visicon-updated`
  only lists *modified* objects; the old location is untouched) —
  `paired-static`/`paired-full` pin this.
- A `-visual>` buffer clear runs `clear :after` (`vision.lisp:2886`),
  which nils `currently-attended` and the marker, so a later window change
  no longer re-encodes the stale location (`paired-full` at 5.000).
- `No visual-object found` is `:output medium` + `:maintenance t`
  (`vision.lisp:2436`): hidden at low detail and it does not itself wake an
  armed CR; the module going free does (`conditions-met`).
- `+goal>` requests schedule their set with `schedule-set-buffer-chunk`'s
  default `:output low` (`buffers.lisp:1403`), so
  `SET-BUFFER-CHUNK GOAL (ISA CHUNK)` prints even at `:trace-detail low`.
- The task's `present-next-trial` calls `(act-r-random 4)` twice
  (`pcomp-issues.lisp:35-36`) before the run; the engine never replays those
  picks, so the noise stream was two draws behind. Logging `act-r-random`
  in SBCL gave `AR1/2` = the task picks, `AR3` = vision's `random-item`,
  `AR4-7` = the first retrieval's noises. `(CONSUME-RNG 2)` (hand-authored,
  applied on reset) aligns them; the same knob fixes v4's first
  `VISUAL-LOCATION` index.

`tutorial-unit7modeling-pcomp-issues-v2-one-trial` is byte-exact and in
`enabled.txt` (52 specs). `-v4-one-trial` now matches through the trace; its
remaining diff is the production-compilation machinery itself (composition
rules, value substitution, `- <value>` naming, `:reward`/`:fixed-utility`
parameter lines).

## 157. Production compilation: the composition tables, drop-out naming, sorting

`determine-composable` (`production-compilation.lisp:1375`) is table-driven
per buffer type. The engine's old rule only refused when *both* productions
made a request; v4's refusals are the (request, harvest) pairs. The port now
iterates every buffer in CL `remove-duplicates` order — the *last* occurrence
wins, verified in SBCL — and looks up `(p1-usage, p2-usage)` in the buffer's
type table:

- `production-lhs-buffers` (`support/production-parsing.lisp:435`) is the
  `=`-match targets only, **not** queries; `production-rhs-buffers` is the
  deduped RHS targets. Query usage still counts (+16) via `get-buffer-index`.
- Tables: RETRIEVAL (`retrieval-compilation.lisp:279`), GOAL
  (`goal-compilation.lisp:704`), PERCEPTUAL for VISUAL/VISUAL-LOCATION/
  AURAL*/TEMPORAL (`perceptual-compilation.lisp:290`), IMAGINAL
  (`imaginal-compilation.lisp:802`) and MOTOR — which is also the `:DEFAULT`
  type (`motor-compilation.lisp:145`). The special check functions are not
  modelled and treated as composing; GOAL keeps its previous
  `T`-entries-only behaviour.
- The refusal reason is the type's `whynot-reason` with
  `failed-function = nil` (the `(> index 30)`, `(2 6 10 ...)` clear,
  `(9 11 13 ...)` modify, `(4 20)` both-request, `(4 12 20 28)` request+
  harvest cases). This is what turns v4's "composed" pairs into the ref's
  "Buffer VISUAL prevents composition … the first production makes a request
  and the second production harvests the chunk."

Composed-production text:

- The `- <chunk>` doc suffix is `dropped-chunks`
  (`production-compilation.lisp:1723`): for each `=` buffer of p2 of a
  drop-out type (only RETRIEVAL sets drop-out) that p1 *requested*, the chunk
  p2's `=BUFFER` variable held (the copy's source via
  `chunk-copied-from-fct`). `match_item` now binds `=BUFFER` to the matched
  chunk, mirroring `buffer-name->variable` (`production-parsing.lisp:481`).
- Conditions print sorted by token (`sort-conditions`,
  `production-compilation.lisp:1903`), but `!`-conditions are collected and
  prepended in reverse `push` order (assembly at :1741) — `retain-indirect-
  retrievals`' `!safe-bind! =r …` line pins this.
- `:reward`/`:fixed-utility` print only with `:esc` + `:ul`
  (`utility-and-reward-1.lisp:686`); v4 sets `:ul t`, ppm-compilation does
  not.

v4 is down from 205 to 24 diff lines; what remains is the map/compose
functions' value substitution (`value "3"`, `key "d"`, `num2 =retrieval` vs
`screen-pos =visual-location`), the `> screen-x current` rel rendering and
the chunk-type default (`is-response t`) in composed conditions.

## 158. Production compilation: map-function substitution and rels

The remaining v4 diffs were the buffer-type **map** functions
(`MAP-RETRIEVAL-BUFFER`, `MAP-IMAGINAL-BUFFER`), which produce
variable→value mappings applied to both productions' specs before
composition:

- `MAP-RETRIEVAL-BUFFER` (`retrieval-compilation.lisp:71`), p1 ∈
  {4,12,20,28} × p2 ∈ {8,12,24,28}: the retrieved chunk (p2's `=RETRIEVAL`
  binding) instantiates p1's `+retrieval>` action variables from p1's
  bindings, and p2's `=retrieval>` condition variables from the chunk's
  slots. This is `(=VAL . "3")` and `(=KEY . "d")`.
- `MAP-IMAGINAL-BUFFER`'s RHS `=` to LHS `=` case
  (`imaginal-compilation.lisp:212`), p1 ∈ {9,25} × p2 ∈
  {8,9,12,13,24,25,28,29}: common slots between p1's (LHS ∪ RHS) and p2's
  LHS map by value — a p2 constant wins, else p2's variable maps to p1's
  value (`(=N2 . =RETRIEVAL)`).
- A variable is only instantiated when a mapping says so: a bare buffer
  variable (`=VISUAL-LOCATION`) stays symbolic, and `=RETRIEVAL` is
  instantiated only when p1 requested the buffer
  (`retrieval-buffers-map`, `production-compilation.lisp:297`). Getting this
  wrong regresses `retain-indirect-retrievals` (wants `value g-1`) or v4
  (wants `screen-pos =visual-location`).
- `isa TYPE` adds the chunk-type's default slot tests to a *condition* too
  (`process-slots-specs`, chunk-spec.lisp:597), so the composed
  `=retrieval> isa response` prints `is-response t`.
- A request's relative test (`> screen-x current`) is kept as a
  `"<op> <slot>"` pseudo-slot and printed `    >  slot value`, matching
  `render_production`.

`tutorial-unit7modeling-pcomp-issues-v4-one-trial` is byte-exact and in
`enabled.txt` (53 specs); `probe_runnable` reports 53 pass / 1 fail.

## 159. Scheduled command lines, unstuff output level, encoding `:requested`

`tutorial-unit3modeling-v2-run` needed three more pieces:

- The task's `schedule-event-relative` with a **string action** prints
  `NONE <action> <params>` at its `:output` (`+format-event-event-string+`,
  `framework/events.lisp:262`: a non-nil `details` wins, else the action and
  params print). The extractor wraps `schedule-event-relative` /
  `schedule-event` and records `(ms output "action params")` as a
  `(COMMAND ms output "…")` sidecar op; the engine re-arms it on `reset` as
  an `Event::Emit` under `NONE` at the matching detail level. The params'
  printed form is captured live (`(vision exp-window Simple task) next`).
- `unstuff-buffer` schedules `erase-buffer` with `schedule-event-now`'s
  default `:output t` (`support/general-pm.lisp:899`, `scheduling.lisp:1292`)
  — **high detail only**. The engine emitted it at medium, so the line leaked
  into a medium-detail trace (`precondition-test`, which is high, still
  shows it).
- `encoding-complete` sets the visual buffer's `:requested` from the
  encoding event: a model `move-attention` is requested (`SET-BUFFER-CHUNK
  VISUAL TEXT0`), while the `update-attended-loc` re-encode is not
  (`SET-BUFFER-CHUNK VISUAL TEXT1 NIL`). The re-encode's
  `Event::EncodingComplete` now carries `requested: false`.

`tutorial-unit3modeling-v2-run` is byte-exact and in `enabled.txt`
(54 specs); `probe_runnable` reports **54 pass / 0 fail**.

## 160. `run-full-time` and the boundary between runs

`paired`/`tutorial-unit4-paired-trace` run with `(run-full-time 5)`, not
`run`. ACT-R's `run-full-time` (`framework/scheduling.lisp:986`) schedules a
hidden `:maintenance` dummy at the end (priority `:min`, `:output nil`) so an
empty queue still reaches the time limit. The extractor now wraps it and
records a `(:RUN-FULL-TIME secs …)` step; the engine pushes the dummy and
runs to the limit.

The subtlety: the task's window change *between* runs schedules its
`proc-display` at the boundary time (5.000), **after** the previous run has
returned. The engine arms the whole device script at reset, so its
`proc-display` at 5.000 is already queued and would be consumed by run 1,
putting the "Stopped because time limit reached" line after the VL set. The
engine's `run_full_time` therefore stops on `t >= limit` (plain `run` keeps
`t > limit`), leaving boundary events for the next run — matching the ref's
stop-then-VL-set order.

The task's `(permute-list …)` also consumes one `act-r-random` per trial
(task-only, never replayed), so `(CONSUME-RNG 2)` in the hand-authored
sidecar aligns the activation-noise stream (verified by logging ACT-R's
draws: `AR1/4` = the two `permute-list` picks, `AR2/3/5/7` = the vision
`random-item`s, `AR6` = the trial-3 noise).

`tutorial-unit4-paired-trace` is byte-exact and in `enabled.txt` (55 specs).

## 161. Model-body `set-buffer-chunk`, `declare-buffer-usage`, GOAL checks

`dynamic-production-compilation-tests`' model body initialises buffers
directly:

- `(set-buffer-chunk '<buffer> '<chunk>)` (`commands/buffer-cmds.lisp`) sets
  a buffer's initial content; ACT-R re-evaluates the model body on reset, so
  the engine applies it (silently, no trace line) on every `reset`.
- `(declare-buffer-usage <buffer> <type> <slots>)`
  (`procedural-cmds.lisp:789`) is compiler metadata (which chunk type a
  buffer holds); it is accepted and ignored.

The GOAL compilation table's special checks (`G-B-C1`, `NO-RHS-GOAL-REF`,
`NO-GOAL-HARVESTING*`) are now treated as composing, like the other types'
checks. That was needed for G1&G1 (usage 8×8, `NO-GOAL-HARVESTING` passes),
which the engine had been refusing with "one production makes a modification
and the other makes a modification request".

Remaining for this spec: ACT-R's `equivalent-productions-p`
(`production-compilation.lisp:2044`, the canonical variable-renamed
comparison rep) so a composed production identical to an existing one prints
`Result is production X` / `No actions taken.` instead of a new production —
and the case-specific retention of a null `=buffer>` modify in
`COMPOSE-GOAL-BUFFER`. The spec is regenerated (15 runs) but not enabled.

## 162. Duplicate composed productions, null modifies, repeated `:do-not-harvest`

Three more pieces finished `dynamic-production-compilation-tests`:

- `check-for-duplicate-productions` (`production-compilation.lisp:1915`): a
  composed production equivalent to an existing one is not installed. The
  engine compares a canonical rep (tokens + slots sorted) of the composed
  pair against each existing production; a match prints
  `Result is production X` / `No actions taken.` for a user production
  (`Recreating production X` for a compiled one).
- A null `=buffer>` modify is **kept** in the composed actions
  (`COMPOSE-GOAL-BUFFER`'s `(when a2= (list a2=))`), so `G2 & G2` matches
  `G2` and not `G1` (which has no action). Without it the canonical rep
  collapsed G1/G2.
- `:do-not-harvest`/`:do-not-query` may repeat in one `sgp`
  (`:do-not-harvest imaginal :do-not-harvest visual-location`); each
  occurrence extends the set. The engine only read the first, so
  VISUAL-LOCATION was still harvested and `P1` could not fire twice.

`dynamic-production-compilation-tests` is byte-exact and in `enabled.txt`
(56 specs); `probe_runnable` reports **56 pass / 0 fail**.

## 164. Per-run device scripts (`RUN` groups)

A task re-applies its window content after each `(reset)` — `paired` runs
`paired-task` three times, each resetting and picking new texts — so ops at
the same ms in different runs collided in the flat `<name>.device.sexp`. The
extractor now records each device op with the current `*reset-count*` and
writes `(RUN n (ADD-TEXT …) …)` groups; a flat op (no group) still re-arms on
every reset for backward compatibility.

The engine tracks a `device_run` counter: the construction reset is run 0 and
the spec's first `(reset)` is run 1 (the extractor's `*reset-count*` matches),
so `reset` arms only the current run's grouped ops. ACT-R's `reset` also
resets model time to 0, which `paired`'s ref confirms (each `paired-task`
block starts at 0.000); the engine already did this.

`paired` is now runnable with the right window content per run; what remains
is the `:act` trace the task toggles via its `:testing-function`, the
buffer-trace output, `sdm`, and the noise alignment.

## 165. Tutorial companion loads; AURAL buffers

`tutorial-unit3-sperling-trial-trace`'s stub calls `sperling-trial` but only
loads `sperling-model.lisp`; the function lives in `tutorial/lisp/sperling.lisp`
and loading it makes the ref match exactly. The extractor now retries a stub
whose `run-it` fails with an undefined function, after loading
`tutorial/lisp/<stem>.lisp` for each loaded `tutorial/…/<stem>-model.lisp`
(only ever triggered for specs that already errored).

The engine's reset buffer list was missing `AURAL`/`AURAL-LOCATION` (the
loader's `is_known_buffer` already knew them), so any audio production failed
to load. Added, along with `MANUAL-LEFT`/`MANUAL-RIGHT`.

The spec still needs the audio module: the ref shows `AUDIO SET-BUFFER-CHUNK
AURAL-LOCATION AUDIO-EVENT0` at +50 ms and `AURAL TONE0`, while the engine's
partial audio scaffolding only fires from the vocal path and uses a +300 ms
detection delay. It also needs ACT-R's visicon-based `VISUAL-LOCATION{n}`
naming (the engine numbers per feature, which only coincided for the small
windows in the passing specs).

## 163. `clear-all` resets the model set; per-run device scripts

`is_multi_model` counted every `define-model` in a spec's file chain, but
`paired-learning` loads `paired-model.lisp` (via its task file) and then
`paired-learning-model.lisp`, whose body starts with `(clear-all)` — so only
`paired-learning` is active and the ref is single-column. The classifier now
clears its name set on `clear-all`, like ACT-R.

`paired`/`paired-learning` still need the device script to be **per run**: the
task re-applies its window content after each `(reset)` (different texts
picked by `permute-list`), but the flat `<name>.device.sexp` arms every op at
reset, so the t=0 texts from different `paired-task` calls collide. That plus
`record-history`/`sdm` output and the retrieval-timing differences keep these
two specs in progress (regenerated, not enabled).






## 166. Vision tracking (`start-tracking`/`update-tracking-mth`)

`tracking-test` exercises the vision module's object tracking: a production
requests `+visual> isa start-tracking`, and every time the tracked text moves
(`modify-text-for-exp-window`, driven by the model's `!eval!
(schedule-event-relative … 'move-it)`) the vision buffers are re-synced.

Several non-obvious details:

- A `+visual>` request's command is its **chunk-type** (`isa
  move-attention`/`isa start-tracking`), not a `cmd` slot; `visual_request`
  now reads the ctype (a `cmd` slot still overrides).
- `start-tracking` is scheduled at priority 0 so the request rule's implicit
  `clear-buffer visual` (priority 10) lands first; the module then stays
  `:exec BUSY` (`?visual> state free` fails while tracking).
- `update-tracking-mth` (`vision.lisp:2706`) branches on which vision buffers
  hold the tracked chunks; `set-vis-loc` is priority 15, `set-vis-obj` 14,
  `update-tracking-loc-chunk` 13 (silent), so the location lands first.
- `convert-loc-to-object` builds a **fresh** object chunk every update, so
  the `TEXT{n}` counter advances even when the chunk is only used for a
  modification (`TEXT0`, `TEXT1`, `TEXT2`, `TEXT5`, `TEXT8` in the ref).
- The visual buffers hold copies: the location chunk is `VISUAL-LOCATION0-{k}`
  (`chunk-copied-from-fct` = the feature base, printed `[VISUAL-LOCATION0]`),
  the object is always `VISUAL-CHUNK0`. `mod-buffer-chunk` clears the
  `copied-from`, and `@visual> vo-1` copies (`VISUAL-CHUNK0 [VO-1]`), which is
  how `update-tracking-mth`'s `vis-obj-marker` test tells a tracked object
  from an overwritten one. `@visual-location> vl-1` copies to `VL-1-{k}`.
- `simple-size` (`vision.lisp:3698`) is `0.01 * round(angle(w)*angle(h)/0.01)`
  in single-float, `angle(px) = 2*atan(px/2, 1080)` in degrees; CL `round`
  returns the integer quotient (so `7×10 → 0.19999999`, `28×10 →
  0.78999996`).
- A visual-object chunk's `VALUE` slot is the text; a visual-**location**
  chunk's `VALUE` is the *kind* (`TEXT`). `screen-x/y`, `height`, `width`,
  `distance` are integers.

The task's `move-it` is scheduled by the model, so its window effect is
replayed from a captured `(MODIFY-TEXT ms text x y)` device op; `!eval!`
`incf`/`modify-text-for-exp-window` are no-ops, and `!eval! (buffer-chunk …)`
runs in source order (after `!output!`).

## 167. `*buffer>` mod-requests and per-run device sidecars

`*buffer>` (`module-mod-request`, `buffers.lisp:1666`) is not an immediate
modify: the request goes to the buffer's module, which goes busy and applies
the change after its own delay. For `*imaginal>` that is the 200 ms imaginal
delay, so a later `?imaginal> state free` test waits (in `paired-learning`
the `retireve-associate` firing moves 0.450 → 0.600 and a `MOD-BUFFER-CHUNK
IMAGINAL` appears at +200 ms). The engine now sets `imag_busy`, schedules the
change, and frees the module when it lands (traced at `low`, the detail the
model's `:trace-detail` shows).

`paired-learning`'s checked-in device sidecar predated per-run `(RUN n …)`
groups, so its flat `ADD-TEXT` list mixed texts from different runs at the
same `at_ms` — two texts landed at t=0 and the first visual-location was
named `VISUAL-LOCATION1` instead of `VISUAL-LOCATION0`. Regenerating the
sidecar (with the groups) fixed the naming. The spec still needs the
production-compilation rendering to match (the composed production's LHS
takes the post-state `stimulus-read` instead of the first firing's `start`).

## 168. Visicon naming order; sperling's RNG gap

`VISUAL-LOCATION{n}` names follow the order the environment returns window
items, which is column-major `(screen-x, screen-y)` rather than the order
the items were added. In `sperling` the 12 letters are added
`V N T Z / C R Y K / W J G F` but the names run `V C W N R J T Y G Z K F`
as `0..11`; the attended rows' ids (`1 3 2 0 4 1`) only line up with the
column-major naming. `proc_display` now sorts each wave by `(x, y)` first.

`stuff-visloc-buffer` (`vision.lisp:1978`) then sorts the max-tstamp objects
by `symbol-name` before `random-item`, so `newest_feature`'s name sort is
correct (restored).

`sperling` still mismatches: its first auto-stuffed location should be
`VISUAL-LOCATION1` (the reference's first `random-item` draw is 1), but the
standard MT19937 seeded `(100 0)` gives `random 12` = 6 first, and the
reference's later ids don't appear anywhere in that draw stream — ACT-R
consumes draws somewhere the engine does not (the model's own `:seed` may
also be applied at a different point). The trace body otherwise depends on
this draw, so the spec needs that alignment plus the audio module.

## 169. Per-run RNG draws from the task (`:CONSUME-RNG` steps)

A task's setup code draws from the same random stream as the model
(`paired-task` calls `permute-list`, which calls `act-r-random`), but those
draws happen between runs, so the device-level `CONSUME-RNG` (applied once
per reset) cannot align them. The extractor now wraps `act-r-random`, counts
the task's draws made *outside* a run, and emits a `(:CONSUME-RNG n)` step
before the next run; `Step::ConsumeRng` advances the engine's MT stream by
`n`. With this, `paired`'s retrieval latencies (and the transient-noise
values in the `:act` trace) line up across all 16 runs — the reference's
first noise, `−0.7636829`, comes out of consuming the task's draw.

## 170. Per-run `:act`/`:epl`/`:pct` from the testing-function

A stub's `(ssp :starting-parameters (:testing-function fn))` is applied by
the model-tester harness, not the single-threaded ACT-R load, so the
extractor now captures the function (`w-ssp`) and calls it before each run,
then records the resulting `:act`/`:epl`/`:pct` as a `(:SGP …)` step when they
change (`paired`'s `set-paired-params` reads the task's `*act-trace*` /
`*compilation*` globals). Its `set-testing-traces` call is suppressed (it
sets `:v`/`:cmdt`, already encoded in each `:RUN` step); `sgp-fct` getters
return a flat value list (`(T)`, not `((:act t))`). The extractor also
defines `set-testing-traces` as a no-op if the harness did not.

This makes `paired`'s `:act` activation blocks (and their `No matching chunk
found retrieval failure` line) and the run-2/3 production-compilation blocks
appear, matching the reference.

## 171. `sdm` and the declarative-memory dump

`paired` calls `(sdm isa pair)` between tasks, which prints the DM chunks
matching the spec (`commands/dm-commands.lisp:491`); `isa pair` matches the
`isa chunk` goals too, so the whole DM is printed. The extractor wraps
`sdm-fct` (the `sdm` macro expands to it) into a `(:SDM isa pair)` step and
`Engine::print_sdm` renders `self.dm` with the same `pprint-a-chunk` helper
`buffer-chunk` uses.

The reference's DM holds the merged buffer chunks first
(`IMAGINAL-CHUNK0-0`, `VISUAL-CHUNK0-{0,1}`, `VISUAL-LOCATION0-0`,
`VISUAL-LOCATION1-0`) then the goal chunks (`START` … `READ-STUDY-ITEM`). The
engine's DM currently holds only the imaginal/visual merges (the location
buffer is not a merge source) and the merged visual objects keep the buffer
copy name (`VISUAL-CHUNK0`) rather than `VISUAL-CHUNK0-{k}`, and the `add-dm`
goals are absent — so the dump content/order still differs.

## 172. Visicon `screen-x` uses the item's centre

`add-text-to-exp-window` places an item by its top-left, but the visicon
records the item's *centre*: `screen-x = window-x + text-x + width/2`,
`screen-y = window-y + text-y + height/2`. The engine used a constant
`text-x + 305`, which is only right for 7-px characters; `zinc` (width 28) is
`455` but the reference has `465`. The offset is
`305 + (width - 7)/2` (window 300, font 7 px/char, 10 px tall) — confirmed by
`paired` (`zinc` 465 / `9` 455) and unchanged for `tracking-test` (all 7-px).

## 173. `sdm` prints DM sets newest-first

`dm-chunks` is an alist keyed by each chunk's *shape* (its type plus the set
of filled slot names), and `add-chunk-into-dm` pushes a new set to the front
(`declarative-memory.lisp:1931`). So `sdm` prints the merged buffer sets
newest-first (each set's chunks in insertion order) and only then the model's
`add-dm` chunks. `print_sdm` groups `self.dm` by shape and reverses the
group order before appending the goals; this matches `paired`'s
`IMAGINAL, VISUAL, VISUAL-LOCATION, START, …`.

(The visicon's `new-name` counters do *not* reset on a window clear —
`paired-full`'s trace keeps numbering across `clear-exp-window`
(`VISUAL-LOCATION1` after the clear) — so feature ids continue per run.)

## 174. Merged buffer copies get a `NAME-{k}` name

`merge-chunk-into-dm` copies a chunk that fails `chunk-not-storable`, and a
vision buffer copy (`VISUAL-CHUNK0`) does, so the DM entry is
`VISUAL-CHUNK0-{k}` (`declarative-memory.lisp:1908`, `copy-chunk-fct`). The
engine now renames a merged chunk whose name is a `{BUFFER}-CHUNK` copy (or a
model `define-chunks` name) to `NAME-{k}`.

`paired`'s task-1 DM then holds the two object copies
`VISUAL-CHUNK0-0`/`-1`, but the engine still merges all four per-trial objects
(the reference, verified by dumping ACT-R's `all-dm-chunks`, keeps only the
first trial's two). The window clear does not reset the visicon ids (confirmed
by `paired-full`'s trace and by the continuing `VISUAL-LOCATION{n}` in
`paired`'s own trace), so the two later objects differ only by their
`screen-pos` copy name and ACT-R is not storing them — the remaining rule to
pin down.

## 175. Vision objects merge ignoring `screen-pos`

A visual object's `screen-pos` names a buffer copy (`VISUAL-LOCATION0-0`) that
is recreated on each attention, so the same object seen again later (the
second trial of `paired`, a repeated window item) must merge into the existing
DM chunk rather than add a new one. `merge_into_dm` compares slots with
`SCREEN-POS` excluded (verified against ACT-R: `paired`'s task-1 DM holds two
objects, `VISUAL-CHUNK0-0`/`-1`, even though four are encoded across the two
identical trials). The enabled suite is unaffected.

With this, `paired`'s `sdm` blocks match; only the buffer-trace table remains.

## 176. The BOLD demand table

`paired` ends with `(predict-bold-response)`, whose table (`time GOAL …`) is
the BOLD haemodynamic prediction (`other-files/bold.lisp`), not a plain
activation log. It: records each traced buffer's busy/demand intervals from
the buffer-trace (`parse-trace-lists-for-bold`); convolves each with a gamma
PDF (`bold_partial_gammapdf(t,a,b) = (t/b)^a·e^(−t/b)`, trapezoidal over the
demand, `area-under-bold-curve`); samples at `:bold-inc` (2 s) from
`inc/2` to `end−inc/2`; and prints via `print-bold-response-data`
(`~{17,@a~}` header, `~{17,3f~}` rows; `max-len = 2 + longest name`). Defaults
`:bold-exp 5`, `:neg-bold-exp 15`, `:bold-positive 6`, `:bold-negative 1`,
`:bold-scale 1`, `:bold-settle 40`, `:point-predict (goal)`.

`reset` restarts the meta-process clock, so the recorder tracks the *current
task* (the reference's table spans 0–40, task 3's end). The engine now
records the busy intervals per event and prints the table; IMAGINAL/RETRIEVAL
match exactly. Remaining: GOAL is point-predicted (its demand points aren't
captured yet) and VISUAL is off by ~0.001.

## 177. GOAL is point-predicted

The goal module's `state busy` is *always nil* (`goal.lisp:163`), so the goal
buffer never contributes a busy *interval*; its BOLD demand is the
`point-predict (goal)` case — zero-length points at the goal chunk-name/request
events, summed as `bold_partial_gammapdf(time − t1)`. `bold_point` records a
point on each goal `set-buffer-chunk`; with it `paired`'s GOAL column matches
and only VISUAL remains (the engine's visual-busy demand is ~2/3 of the
reference's — a boundary detail in the busy-interval recording).

## 178. `paired` BOLD: the visual clear is busy, and the table's blank line

Two last details completed the BOLD table:

1. **`+visual> cmd clear` marks the vision module busy.** The module's
   `clear` (`general-pm.lisp:333`, reached via `pm-module-request` →
   `schedule-event-now 'clear`, `vision.lisp:3252`) sets `prep` busy for
   `feat-prep-time`, so `?visual> state free` stays false for ~50 ms after
   each `VISION CLEAR`. That is the *second* busy interval per paired run
   (`[k+0.185, k+0.235]`); the plain `CLEAR-BUFFER VISUAL` (no module
   request) does not. Modelled with `visual_prep_busy`, freed by the
   scheduled `change-state`; `query_free("VISUAL")` now tests both flags.
2. **The table ends with one blank line.** `command-output`
   (`misc-utils.lisp:845`) wraps its argument in `~&…~%`; the data format
   `~{~{17,3f~}~%~}` already ends in `~%`, so the wrapper adds a second.

With both, `paired` is byte-exact (540 lines) and joins `enabled.txt`.

## 179. `paired-learning`: compilation-type checks are not "always compose"

`define-compilation-type` tables (`retrieval-compilation.lisp:279` etc.) do
not just list usage pairs to allow — some entries are *checks* (`R-B-C1`,
`R-B-C2`, `G-B-C1`, `I-B-C1..C10`, `P-B-C1..C4`, `M-B-C1/C2`,
`NO-RHS-GOAL-REF`, `NO-GOAL-HARVESTING*`). The engine had been treating every
listed pair as composing, so it composed where ACT-R refuses. Implemented the
RETRIEVAL checks: `R-B-C1` requires the second production's query to test only
`state busy`/`buffer empty`; `R-B-C2` requires both queries to be identical.
That makes `paired-learning`'s `retireve-associate & retrieval-arg2-unsuccessful`
refuse (`buffer failure` is neither), matching the reference. The remaining
types' checks, the composed-LHS specialization, and the compiled productions'
`parameters for production` blocks are still open.

## 180. `paired-learning`: composed-LHS specialization needs `rename-variables`

A composed production's condition takes p1's firing values; when p1 and p2
use the *same variable name* for different things (e.g. `retrieve-operator`'s
goal `state` and `read-arg1`'s retrieved `post`, both `=state`), the engine
unified them and specialized the condition from p2's value (`state
stimulus-read` instead of `state start`). ACT-R's `rename-variables`
(`production-compilation.lisp:1945`) renames p2's colliding *basic* variables
(`basic-variables`, :1964) before composing, so the two stay distinct.
Implemented, plus `MAP-GOAL-BUFFER`'s p1-RHS→p2-LHS goal mapping
(`goal-compilation.lisp:320`) so a renamed variable re-unifies with p1's.

A refinement was needed to avoid regressing `retain-indirect-retrievals`: a
variable bound by the *same* buffer in both productions (e.g. `=c` in
`=goal>`) is the same variable and must not be renamed. `paired-learning`'s
composed-LHS values now match (diff 274→260 hunks); the composed RHS slot
*order* (`state` before `step`) and the other categories remain.

## 181. Composed-production slot order comes from the standard rep

`compose-productions` receives each production's *standard rep*
(`production-standard-rep-sides`, `production-compilation.lisp:2074`), whose
slots are `stable-sort`ed by `sort-production-slots` (:1976, op then name).
`buffer+-union`/`buffer=-union` (`:2295`/`2316`) then do
`(append a1-remain a2)` — p1's sorted slots (minus those matching p2's) then
p2's sorted slots. The engine had preserved source order, so a composed
`=goal> step attending state =state` printed `step` before `state` instead of
the reference's `state … step …`. Sorting each side before concatenating
matches. (`retain-indirect-retrievals`' earlier breakage was the
`rename-variables` over-rename, not this sort.)

## 182. `isa text` adds `text t` from the built-in vision type

The vision module defines `(chunk-type (text (:include visual-object))
(text t))` (`vision.lisp:3069`), but the engine only loads chunk-types from
the model file, so a composed condition `isa text` lost the `text t` default.
Added `builtin_type_defaults` for the vision types (`text`/`oval`/`line`) used
when expanding `isa TYPE` in `prod_lhs_slots`.

## 183. MAP-GOAL-BUFFER maps p1's `+` request slots too

`MAP-GOAL-BUFFER`'s first case (`goal-compilation.lisp:320`) is the RHS `+`
to LHS `=` mapping (and the `*` variants), not just `=`/`*`. Extending the
engine's goal mapping to p1's `+`/`r` slots stopped `complete-task &
retrieve-operator` leaking a renamed `=state1` (p1's `+goal> state =state`
now unifies with p2's renamed goal variable). The remaining difference there
is that ACT-R merges p1's `+goal>` and p2's `=goal>` into one `+goal>`
statement (`buffer+-union`), which the engine still emits separately.

## 184. GOAL `+`/`=` merge (`buffer+-union`)

`COMPOSE-GOAL-BUFFER`'s `(4 12 13 44)` case (`goal-compilation.lisp:544`)
keeps p1's `+` request and folds p2's `=`/`*` slots into it via
`buffer+-union` (p1's non-matching slots then p2's, sorted) — unlike
RETRIEVAL, which drops p1's request when p2 harvests. So `complete-task &
retrieve-operator` composes to `+goal> state =state step retrieving-operator`,
not a separate `=goal>`. Added the GOAL special case to the RHS builder.

## 185. Compiled productions inherit their parents' reward

`initialize-utility-for-compiled-production`
(`utility-and-reward-1.lisp:477`) sets a compiled production's `at` to the max
of the parents' and its `reward` to the max of the numeric parent rewards.
The engine had hard-coded `:reward NIL`; now it inherits (so `complete-task`'s
`(spp complete-task :reward 20)` shows on the compiled production's parameter
block) and stores it on the installed production for later compositions.

## 186. `update-attended-loc` re-encode matches by position, not name

`encoding-complete` (`vision.lisp:2413`) builds the object with
`get-obj-at-location` → `within-move` (`vision.lisp:2485/2367`), i.e. by the
attention position with `:visual-movement-tolerance` (default 0.5°, the
module's `move-allowance`). The engine resolved the re-encode through
`resolve_loc`, which consults the `vl_copies` name→index map — stale across a
window clear, so the old copy name still pointed at index 0, now holding the
*new* text. The re-encode therefore wrongly "found" it: an extra
`SET-BUFFER-CHUNK VISUAL TEXT1 NIL` at k+0.085 plus a consumed `TEXT{n}`
name (so the requested encode became `TEXT2`). SBCL probes confirm ACT-R
fires the re-encode but finds nothing when the replacement shifts the centre
past tolerance (`xray`→`8`: 10 px ≈ 0.53° > 0.5° → `No visual-object found`),
and *does* re-encode when the centre is unchanged (`xray`→`zinc` →
`SET-BUFFER-CHUNK VISUAL TEXT1 NIL`). Fix: resolve `requested = nil` encodings
by the old clof (`emma_loc_xyz`) via `angle_between` ≤ 0.5°; on no match take
the `No visual-object found` branch (frees the module, consumes no name).

## 187. `parameters for production` on recreation; utility learning

`update-params-for-compiled-production` (`production-compilation.lisp:1802`)
prints the `spp-fct` block in the new-production branch *and* the
"recreation of a previously compiled" branch (a user-production collision
prints nothing). A recreated production's `u` moves toward its first parent's
via `linear-update-utility` (`utility-and-reward-1.lisp:395`); `:utility` is
`production-utility`, cached by the last conflict resolution
(`compute-utility` with `save`, `utility-and-reward-1.lisp:357` ->
`procedural.lisp:2230`) and printed before the ppm search offsets are folded
in. `new-production-name` (`production-compilation.lisp:1874`) consumes a
`PRODUCTION{n}` name for every composition that reaches it (duplicate or
not), so the counter advances across recreations (ref jumps 8 -> 19).

The engine now prints the block on recreate, caches `production-utility`,
applies the recreation update, and advances the name counter per
composition. The first recreation's `u` matches, but long-run `:u`/`:utility`
still diverge (the ref grows toward ~20, the engine stalls near 1-2): the
utility dynamics (conflict-candidate set / possible silent reward
propagation) are not fully modelled. Pre-existing RNG divergence (task pair
order, 1 ms retrieval timings) is also present in this spec.

## 188. `find-location` does NOT sort its candidates (verified)

`find-location` (`core-modules/vision.lisp:2923`) picks
`(random-item (objs-max-val it 'chunk-visual-tstamp))` — **no** `sort`.
`objs-max-val` (`framework/misc-utils.lisp:1263`) builds its tie list with
`push`, but `find-current-locs-with-spec` hands the matches over in reverse
visicon order, so the net order is *forward* visicon order. An earlier
engine pass sorted the names (copying `stuff-visloc-buffer`,
`vision.lisp:1978`, which *does* sort), which shifted every index and
desynced the `random-item` picks. Verified against `sperling`:
candidates `("V" "W" "N" "R" "J" "T" "Y" "G" "Z" "K" "F")` with
`random-item` index 2 -> `VISUAL-LOCATION3` (N).

## 189. Audio module: tone detection / recode timing and output levels (verified)

A tone is `new-name-fct "TONE"` for the sound chunk and an independent
`new-name-fct "AUDIO-EVENT"` for the `audio-event` chunk
(`core-modules/audio.lisp:582`). `new-sound-event :around`
(`audio.lisp:733`) schedules `stuff-sound-buffer` at
`detect-at-time = onset + tone-detect-delay` (50 ms) as a **maintenance**
event, and `new-sound` at the onset (`:output medium`). `stuff-sound-buffer`
-> `find-sound :stuffed t` sets `AURAL-LOCATION` `:requested nil`;
`attend-sound` then schedules `audio-encoding-complete` after
`tone-recode-delay` (285 ms), which sets `AURAL` `:requested t` with the
sound chunk (`content`, `kind`, `event`).

The two `SET-BUFFER-CHUNK` lines print under `AUDIO` at the default
`schedule-set-buffer-chunk` output (`low`), so they survive `trace-detail
low`; `new-sound`/`ATTEND-SOUND`/`AUDIO-ENCODING-COMPLETE` are `medium` and
are hidden there. Sperling: `0.200 AURAL-LOCATION AUDIO-EVENT0 NIL`,
`0.570 AURAL TONE0`, exactly.

## 190. `find-location` relations, `screen-x lowest`, and the setup draws (verified)

The Sperling visual search needs three things the engine had dropped:
`find-location` must filter on the request's `>`/`<` relations
(`> screen-y 450 < screen-y 470`), `stuff-visloc-buffer`'s auto-stuff uses
the `default-spec` = `screen-x lowest :attended new` (`vision.lisp:3178`)
so it only sees the leftmost column, and `find-sound` (`audio.lisp:1031`)
consumes a `random-item` draw. The task's setup draws are
`permute-list` of 21 (21 draws) + `(act-r-random 3)` + `(act-r-random 200)`
= 23, so `(CONSUME-RNG 23)`. With all of these the engine's whole RNG
stream matches ACT-R (`sperling`: finds `3,2,0,4` then the report phase).

Still open: the last find (ref `VISUAL-LOCATION1`) needs the auto-stuffed
object to become `:attended nil` again when its location is unstuffed
(`UNSTUFF-BUFFER … VISICON-ID1`, 500 ms); and the report phase fires ~34 ms
early.

## 191. Vision finsts: `:attended`, `:visual-num-finsts`, and the clear/move race

`:attended` is not a sticky bool. `feat-attended` (`vision.lisp:2269`) is `t`
iff the feature is in `finst-lst`, `'requested` while attention moves to it,
`'NEW` while `chunk-visual-new-p` is `NEW`, else `nil`; `test-attended`
(`vision.lisp:2006`) makes `:attended nil` match `NEW` **or** `nil`, and
`:attended new` match only `NEW`. `add-finst` (`vision.lisp:2282`) records a
finst on encode and caps `finst-lst` at `:visual-num-finsts` (default 4),
evicting the OLDEST — that cap, not the 500 ms unstuff, is why Sperling's
auto-stuffed `VISUAL-LOCATION1` becomes `:attended nil` again for the fifth
find (ref finds `3,2,0,4,1`). The engine stores `Feature.new_p` +
`Feature.finst_ms`, sets the finst on encode, and enforces
`:visual-num-finsts` / `:visual-finst-span`.

The report phase then needed the window-clear re-encode suppressed: a
`+visual-location` request holds the vision proc-display lock for ~50 ms
(`find-location`, `vision.lisp:2929`; `lock-vision`/`unlock-vision`,
`vision.lisp:1542`), so the 941 ms clear's `proc-display` /
`update-attended-loc` is deferred to 975 ms — by which time the 975 ms
move-attention has set `moving-attention`/`exec BUSY`, so `update-attended-loc`
returns early and no re-encode runs. The engine models the observable by
dropping a pending `update-attended-loc` re-encode
(`EncodingComplete { requested: false }`) when a new `MoveAttention` starts;
vision stays busy to 1.060 and `start-report` fires at 1.110, byte-exact.
`tutorial-unit3-sperling-trial-trace` is in `enabled.txt` (59 specs).

## 192. `ssp :starting-parameters` seeds, `:alpha`, and device sidecars (verified)

The model-tester re-applies `ssp :starting-parameters` on every `(reset)`
(`central-parameters-reset`, `central-parameters.lisp:140`), including a
`:seed` and the `:testing-function`. The single-threaded extractor only
called the testing function, so for `paired-learning` (whose model sets no
seed) it recorded the default seed `618013` and a window that never matched
the reference. `fixtures/extract-specs.lisp` now captures
`:starting-parameters`' `:seed` and re-applies it in `w-reset`, and
`flush-sgp-params` also records `:alpha` (the task runs its second block
with `alpha 1.0`, which makes reward propagation jump straight to the
reward). The per-run `(:RUN-FULL-TIME 5 (1 N) …)` seeds are the real RNG
states — unlike `(:run …)` they are never canonicalised to the `(100 0)`
placeholder — so `Step::RunFullTime` now carries and re-applies its seed.
`paired-learning.device.sexp` (window texts) had to be regenerated too.
This cut the `paired-learning` diff from ~180 hunks to 27.

## 193. Production-compilation composition details (verified)

Four fixes in `compose_pair`/`rename-variables` closed the remaining
`paired-learning` hunks (27 → 0):

- `rename-variables` (`production-compilation.lisp:1533`) renames a shared
  basic variable in the second production when it binds a different
  `(buffer, slot)` there — `PRODUCTION14 & PRODUCTION16` share `=state` for
  the retrieval's `arg1` (p1) and `post` (p2), so the two must not unify
  (`=state` → "respond", `=state1` → "wait"). The same slot in the same
  buffer stays unified (`E & F`'s `color =c`).
- `MAP-IMAGINAL-BUFFER`'s RHS-`*` case (`imaginal-compilation.lisp:98`)
  fires for a `*imaginal>` (modify-request) first production: the second's
  constant instantiates the variable, or its variable unifies onto p1's.
- `COMPOSE-GOAL-BUFFER`'s `(4 12 13 44)` case
  (`goal-compilation.lisp:546`) emits the first production's own
  `=goal>`/`*goal>` modify before the `+goal>` union.
- `buffer=-union` for `(and a1* a2=)` (`COMPOSE-IMAGINAL-BUFFER`'s
  `(9 40 25 56)` case) emits a single `*` — the `=`/`*` modify unions are
  not emitted separately.

## 194. Phase B triage: ref content, `:v` gating, and event ordering (verified)

Scope triage (`fixtures/extract-specs.lisp`):

- `*command-tokens*` is only a *hint*: `(sgp :v …)`/`(sgp-fct (list :v …))`
  and `*out-file*` are trace toggles/targets (not command output), and
  `buffer-chunk` matched `schedule-mod-buffer-chunk`. A stub with no genuine
  command token whose ref is a *filtered fragment* (assembled with
  `model-output` from a pre-event hook) still stays `:command` — detected from
  the ref, which lacks a run end (`ref-has-run-end-p`).
- `fresh-state` must run per test even for `:command` specs, or they inherit
  the previous stub's models/steps. `*out-file*` must be bound (`hear-self`).

Trace gating (`:v`):

- `w-run`/`w-run-full-time` record the model's `:v` as the run trace flag
  (`(sgp :v nil)` hides a segment, e.g. `tutorial-unit5modeling-v7-p4-run`).
  But the model-tester sets a global
  `ssp :starting-parameters (:testing-function set-testing-traces)` applied on
  reset (`model-tester.cl:356`), and `set-testing-traces` is `(sgp :v t)`
  (`model-tester.cl:494`) — so a model that sets `:v nil` still traces
  (`production-compilation-issues-model-v2`). `w-reset` applies the testing
  function (default `set-testing-traces`); the engine handles `:V`.

Event ordering:

- A model-body `(goal-focus …)` is a `set-buffer-chunk` event at
  `:priority :max` (`goal.lisp:284`), same as a driver `(goal-focus …)`, so a
  later driver call overwrites it — `tutorial-unit1-semantic-g2/-g3-trace`
  otherwise ran with the model's default `bird` goal and fired `direct-verify`
  instead of `chain-category`.
- `goal-focus-fct` also schedules a `clear-delayed-goal` maintenance event
  (`goal.lisp:284`). It has no state effect in the engine but *counts* toward
  `run-n-events` (`run-one-event` returns `t` for it), which is what makes
  `run-n-events 2` skip exactly the model-body goal and leave the driver's
  goal (and its trace line) in the following `run`.
- A scheduled string command is only replayed from a device sidecar when it
  was scheduled *outside* a run: a command scheduled during a run is
  dispatched by the engine (the `ul-issues` `present-choose`/`show-result`
  chain) or emitted by a module (`audio`'s `new-sound`), so a sidecar entry
  duplicates the line.

New steps: `run-until-time` (absolute end; its stop test is `next-time > end`,
so an event *at* the end is still processed, unlike `run-full-time`) and
`run-n-events`.

## 195. The RPC bridge is transport over a name→handler registry (verified)

The Python/other-language clients are not a Lisp feature: the wire protocol
(`framework/dispatcher.lisp:1663`, client `tutorial/python/actr.py`) is TCP +
JSON terminated by `chr(4)`, methods `add`/`remove`/`evaluate`/`monitor`/
`remove-monitor`/`list-commands`/`list-connections`/`check`/`set-name`/`close`,
requests `{"method","id","params"}`, replies `{"id","result","error"}`. All
real work is `evaluate <name> <args>` — the same `dispatcher-command-table`
lookup the Lisp side uses. Args are data (strings/lists/numbers), not code, so
a Rust server needs no Lisp.

Details that break a naive server:

- `check <name>` must reply `[null]` when the name is free and `[true,owned,
  doc]` when taken. `actr.py:438` loops generating a fresh `python-echoNNNN` and
  treats `result[0] == None` as "free"; returning `[false,…]` spins forever.
- The host file `~/act-r-address.txt` is used verbatim (`actr.py:212`), so it
  must carry no trailing newline (the port file is `int()`-parsed, so it may).
- A client-registered command fires via a server→client
  `{"method":"evaluate","params":[local_name,model,…],"id":N}` and the client
  replies `{"id","result","error"}`; trace output reaches monitors through the
  four trace commands `model-trace`/`command-trace`/`warning-trace`/
  `general-trace`.

## 196. R4 device/vision surface: stats are RMSD, not mean deviation

The RPC command surface for device/vision/motor is wired to the engine's
scripted device layer (`add-visicon-features`, `modify`/`delete`(all)-
`visicon-features`, `print-visicon`, `new-tone-sound`, `pbreak`/`punbreak`,
`correlation`/`mean-deviation`, and the window scaffolding).

Non-obvious bits:

- `mean-deviation` is **not** the mean absolute deviation — it is the RMSD,
  `sqrt(Σ(a−b)²/n)` (`modules/act-gui-interface.lisp:1567`); `correlation` is
  the usual Pearson r (`:1600`). Both `command-output "…: ~6,3F"` (i.e. print
  `CORRELATION:  x.xxx` / `MEAN DEVIATION: x.xxx`) when `{output}` is true
  (the default), which is what the `bst` task relies on
  (`tutorial/python/bst.py:273`). Covered by a `session.rs` unit test.
- `print-visicon` calls `(printed-visicon t)` (`core-modules/vision.lisp:4090`),
  a column-aligned table (Name/Att/Loc + one column per attribute). The engine
  currently emits an approximate line-per-feature form; no `.ref` exercises it
  yet, so it is only a placeholder until an oracle exists.
- `new-word-sound`/`new-digit-sound` are accepted but no-ops for now; the audio
  corpus uses tone sounds.
- The remaining Python tests that use these (`sperling_test`, `fan_test`, and
  the `bst-*`/`fan-no-pm`/`onehit` refs) all go through `tester.py`
  (`sys.stdout = tester.out_file`, `:starting-parameters :testing-function`),
  so they need R6b before an R4 command turns any of them green.

## 197. Production printing must substitute dynamic slots and always show :UTILITY

Two `print-production-text` details the `simple-dynamic-model`/`simple-ppm-test`
refs pin down:

- A dynamic slot `=context =x` (LHS) or `=target =x` (RHS) prints with both the
  **slot name and the value** instantiated from the bindings
  (`support/production-parsing.lisp` `production-statement-text`): `=context =x`
  becomes `DATA 10`, `=imaginal> =target =x` becomes `DATA2 10`. The engine
  renders values via `show_pat` but the *slot name* needs the same lookup
  (`slot_disp`, `compilation.rs`). (The buffer modification itself still uses
  the literal `=TARGET` — see the note below on `simple-dynamic-model`.)
- The `:cst` conflict-set block calls `(spp-fct (list name :utility :u))`, whose
  `parameters` branch (`utility-and-reward-1.lisp:666`) prints **every**
  requested keyword with `~S` (so `:UTILITY`, upper-case) and does **not** gate
  on `:esc` — unlike the bare `spp` path (`:678`). So `:UTILITY` and `:U` both
  appear even when `:esc` is nil.

## 198. R6b: runtime seeds and the `:testing-function` reset hook

`tester.py` (the Python model-tester) needs two engine behaviours the corpus
had only exercised through load-time model parameters:

- `(sgp :seed (S C))` was **ignored** by `apply_sgp` (only the model body's
  `:seed` at construction reached `cfg.seed`). So `set_parameter_value(':seed',
  …)` and `set_system_parameter_value(':starting-parameters', [':seed', …])`
  were no-ops at run time. It is now applied immediately (init + `C` discards,
  `random.lisp:417`) and remembered so `reset` re-seeds. With seed `(1 0)` the
  stream is the standard MT19937 `init_genrand(1)` sequence.
- `:testing-function` is ACT-R's `testing-module` **reset** hook
  (`model-tester.cl:485`), not a normal parameter. The session now reads it
  from `:starting-parameters`, dispatches it as a client command when one is
  registered (`test-fan-2`, `python-set-testing-traces`), and otherwise applies
  the well-known `set-testing-traces` effect (`sgp :v t :cmdt t`).

`fixtures/rpc-conformance.py` now captures `tester.out_file` (task `print`s)
interleaved with the monitored trace, so the tester refs (task stdout +
trace) can be diffed.

Remaining blocker for the `bst-*`/`fan-*`/`paired-*`/`onehit` refs: they drive
a **live** experiment window (`open-exp-window` → `add-text-to-exp-window` →
visicon), but the R4 window commands are still stubs, and the models' internal
RNG draw counts must line up exactly.

## 199. Live experiment window: text->visicon geometry and the `cmd` slot

The RPC `open-exp-window` / `add-text-to-exp-window` / `add-line` /
`add-button` / `remove-items` / `modify-text` / `clear-exp-window` commands now
build visicon features directly (verified against an SBCL oracle,
`devices/virtual/uwi.lisp` + `build-string-feats` `vision.lisp:3558`):

- A text item is chopped into **words** (`chop-string`, `:WORD`/`:MISC` alike)
  and one feature per word; the window origin (`open-exp-window` x/y, default
  300,300) is added as an offset.
- `char-width = (round font-size 12/7)` (7 for font-size 12),
  `text-height = (round font-size 12/10)` (10), `line-height =
  max((round text-height .8), text-height+1)` (12). A word at (x,y) yields
  `screen-x = x+1 + origin-x + (round width 2)`, `screen-y = y + origin-y +
  (round line-height 2)`, `width = char-width * len`, `height = text-height`.
  Lisp `round` is **half-to-even** (e.g. `(round 21 2)` = 10).
- `+visual>`/`+manual>`/… requests name a module command, whose chunk type
  carries a `cmd` default (`vision.lisp:3086`). The engine expands it into the
  request spec at construction (`expand_request_defaults`) so both the
  instantiation renderer and production composition show it. The
  extended-motor-actions `zz_print_instantiation` prints slots then `CMD`; the
  general `render_production` prints `CMD` first — hence the guard/reorder
  there.

## 200. LHS dynamic slots need dependency ordering

A production LHS dynamic slot (`=imaginal> =target =stimulus`) names a slot by
the value bound to `=target`, and that variable can be bound by a *later*
clause (`retrieve-associate` binds `=target` from `=retrieval> required
=target`). The matcher ran conditions in source order, so `=target` was
unbound when the imaginal clause ran and the production never matched
(`paired-learning-dynamic` stalled after its first `*imaginal>`).

`match_prod` now retries passes, deferring a match/query item whose dynamic
slot-name variables are not yet bound, and falls back to source order when no
further progress is possible. 96 -> 803/803 lines on `paired-learning-dynamic`.

## 201. `pp` shows implicit queries; production redefinition keeps the last

Two things the `(P …)` block from `pp` needs that the `:cst`/`whynot`
rendering does not:

- ACT-R's production compiler adds an implicit `?buffer> state free` for
  every *requested* buffer (`+buf>` **or** `*buf>`) with no explicit
  `?buf>` condition and not in `:do-not-query`. `pp` prints these
  (`production-statement-text`); `whynot`/`:cst` do not. So `pp` computes its
  own list (`pp_implicit_queries`, includes `ModifyRequest`) rather than
  reusing `rhs::implicit_queries` (request-only, used for matching — changing
  that would alter match semantics).
- A second `(p name …)` redefines the production: ACT-R keeps the **last**
  definition and warns at the next reset
  (`#|Warning: Production F already exists and it is being redefined. |#`).
  `push_production` (`model.rs`) now replaces by name.

`safe-queries` (`pp` before/after `reset` with `:do-not-query imaginal`) is the
oracle.

## 202. `EXTEND-BUFFER-CHUNK` only for a variable-bound mod slot

A `=buffer>`/`*buffer>` mod-request whose slot is **not** a possible slot
extends the chunk, and the production action schedules an `EXTEND-BUFFER-CHUNK
<buffer>` line one priority above the modification
(`support/production-parsing.lisp:1096`, `:101` for `=` / `:61` for `*`; the
`extend-buffer-chunk` function itself is a no-op — only the scheduled event's
details print). "Possible" is the model-global slot set
(`valid-slot-name`, `framework/chunk-types.lisp:590`): every chunk type's
slots, module types included.

The subtlety: a **literal** mod slot (`=visual-location> color green`,
`=goal> loc …`) is extended when the production is *parsed*
(`define-chunk-spec`, `framework/chunk-spec.lisp:633`, `extend` default `t`),
so no line is emitted at fire time. Only a **variable-bound** slot name
(`=imaginal> =target =x`) is extended at *fire* time
(`instantiate-chunk-spec`, `framework/chunk-spec.lisp:777`). `simple-dynamic-model`
shows both: `retrieve-first-step`'s `=target` = `data2` (a `result` slot — no
line), `retrieve-second-step`'s `=target` = `data3` (not a slot — line).
`act-touch` (`=goal> … loc`) and `precondition-test` (`=visual-location> color
green`) are literal, so they must stay line-free. The engine tracks the
extended set in `Engine::valid_slots` so a later mod of the same slot does not
extend again.

## 203. `whynot` recomputes the conflict set and tests conditions by category

`whynot` is not a report over the *last* conflict resolution. `whynot-fct`
(`commands/procedural-cmds.lisp:597`) first recomputes the conflict set with
`pmatches-internal` (`:679`) — a read-only pass that runs no trace, draws no
utility/noise, and schedules nothing — then reports every production in
definition order (`all-productions` when no name is given).

Two consequences the engine had to model:

- The match/no-match decision and the `[requested, actual, similarity]`
  partial-match annotation both come from that fresh pass, not from
  `last_conflict_set`. A ppm partial match prints
  `Production NAME partially matches the current state:` with the mismatched
  slots rendered inline (`production-statement-text`).
- The *failure reason* is the first failure in the **compiled condition
  order**, not source order: `test-production`
  (`core-modules/procedural.lisp:2201`) walks buffer-states, then constants,
  then binds (`production-binds`), then others. A `slot =var` in a fixed slot
  is a `bind-slot` and therefore runs *before* a later clause's
  `test-var-slot`. So `=goal> =slot state3` / `=imaginal> slot =slot` reports
  `=slot` resolved to `state` (from the imaginal `bind-slot`) even though the
  goal clause is first. `Engine::why_not_match` classifies each test into
  those categories and reports the first failure in that order.

The recorded partial-match slot is the *resolved* name
(`test-var-slot` stores `(replace-variables slot bindings)`), so
`render_production` matches the annotation on either the raw or resolved
slot.

`test-ppm` is the oracle for both. Its remaining diff is the third section,
`:PPM-HOOK SHOW-PPM-VALUES` — a stub-defined Lisp callback, which `actr.py`
cannot supply (the documented "no arbitrary Lisp" divergence).

## 204. A scheduled command's trace module is the schedule-time `:module`

A `(schedule-event … "action")` line prints the *scheduling* module in the
trace's module column, not `NONE`: `schedule-event` takes `:module` and
defaults it to `:none` (`framework/scheduling.lisp:1168`). The audio module
schedules its `new-sound` notification with `:module :audio`
(`core-modules/audio.lisp:738`), so the ref shows `AUDIO new-sound`, while a
task's own `clear-exp-window`/`pm-issue-display` show `NONE`.

The device sidecar's `(COMMAND ms output [module] details)` carries it; the
extractor records `(getf keys :module)` (`extract-specs.lisp`,
`record-command`) and the engine emits `DeviceCommand::module`. Sidecars
written before this field default to `NONE`.

Related, still open: a task that calls `new-tone-sound` *outside* a run
(`unstuff-aural`, `siegler`, `audio-updates`) has its `new-sound` line
recorded as a sidecar command, but the sound *object* is not — so the
engine replays the notification without the `stuff-sound-buffer` event that
follows at `onset + tone-detect-delay`. Recording `new-tone-sound` as a
replayed step would fix it.

## 205. A production that names `=buffer` makes that buffer non-reusable

Which name a buffer's chunk gets is decided per *model*, not per set. Two
rules combine:

- Every non-`multi` buffer starts reusable (`framework/model.lisp:499`), so
  `set-buffer-chunk` names its chunk once and reuses it: `{BUFFER}-CHUNK{n}`
  via `new-name-fct` (`framework/buffers.lisp:1351`, `reuse-chunk`). The
  name persists across `clear-buffer` for the whole run.
- But a production that mentions the buffer's **variable** (`=imaginal`)
  anywhere in its LHS or RHS calls `buffer-requires-copies` for it
  (`support/production-parsing.lisp:482-488`), which clears `reuse?`; every
  set then makes a fresh `CHUNK{n}` (global `CHUNK` counter).

That is the whole difference between the two `simple-*`/`act-touch` shapes:
`simple-dynamic-model` never names `=imaginal`, so its chunk is
`IMAGINAL-CHUNK0`; `act-touch` (`+manual> … loc =imaginal`) and
`merging-test` (`=goal> value =imaginal`) do, so theirs are `CHUNK{n}`. The
engine computes this set in `Model::no_reuse_buffers` at load
(`var_referenced_buffers`) and names at set time; the old
`rhs.rs` heuristic (`{v}-CHUNK0` for GOAL/IMAGINAL, `CHUNK{n}` otherwise)
was only right by accident.

Related: an unbound `=buffer` reference on an RHS is `bind-buffer`'s binding
of the buffer's chunk *name* (`core-modules/procedural.lisp:534`). A
**modify** (`=goal> value =imaginal`) stores that name by reference — no copy
— so a later merge of the imaginal chunk into DM can alias the name to the
DM chunk (`merging-test`'s `value B`). Only a **request** spec copies a
chunk-valued reference (`instantiate-chunk-spec`, PORTING-NOTES #41).

## 206. On merge, references to the buffer chunk are renamed to the DM chunk

`merge-chunk-into-dm` (`core-modules/declarative-memory.lisp:1897`) finds an
existing DM chunk by content and calls `merge-chunks-fct existing chunk`. With
`update-chunks-on-the-fly` — `:ncnar` non-nil **and** `:dcnn`, both `t` by
default (`framework/naming-module.lisp:452`) — that rewrites *every slot value
that names the merged buffer chunk* to the DM chunk's name
(`framework/chunks.lisp:1736`). So `declarative-issues-v9`'s
`=imaginal> goal =goal` stores the goal chunk `CHUNK0`, and when `CHUNK0`
later merges into DM `G1` the imaginal chunk's `goal` becomes `G1` — which is
why the ref prints `GOAL G1` and the `:ncnar nil` companion ref prints
`GOAL CHUNK0`. `sji-merge` sets `:dcnn nil` to opt out, and its ref keeps the
copy name `A-0`. The engine gates `Engine::rename_chunk_refs` on
`Config::chunk_update`.

Related, for `sdm`: `dm-chunks` is an alist keyed by the chunk's slot
signature, and `find-matching-chunks` **reverses** each group when collecting
(`framework/chunk-spec.lisp:1132`), so a chunk merged at run time that shares
a signature with a model chunk prints *after* it, in insertion order — the
`OLD-CONTEXT` / `IMAGINAL-CHUNK0-0` pair in
`tutorial-unit5modeling-v9-p7-8-run`. `Engine::print_sdm` attaches such a
run-time chunk to the model group it shares a signature with.

## 207. Chunk slots print in model-wide slot order, not the chunk's type order

`pprint-a-chunk` sorts a chunk's set slots by `act-r-slot-index`
(`framework/chunks.lisp:686`). That index is **model-wide**: `chunk-type-fct`
keeps one `slot->index` table per model and gives a slot name the next index
the *first* time it appears in *any* chunk-type definition
(`framework/chunk-types.lisp:434`), so a type's own declaration order is not
the print order. In `declarative-issues-v8`, `result` is declared first (in
`simple-value`), so `math-fact`'s `1-1=0` prints `RESULT ARG1 ARG2 OPERATOR`
even though the type lists `arg1 arg2 result operator`. `Model::slot_index`
rebuilds the table from the model's chunk types in definition order.

Also on the command path: ACT-R's `command-output` appends a newline, so a
`pprint-chunks` result ends with a blank line. `Engine::output` models this by
pushing a trailing `\n` after joining `out_lines` (`engine/driver.rs:12`), but
the session driver joined without it, dropping a final blank trace line.
`Session::take_trace_output` mirrors `Engine::output` so both drivers agree.

## 208. A non-numeric `:reward` clears the utility-learning history

`(spp <prod> :reward R)` makes `learn-parameters`
(`modules/utility-and-reward-1.lisp:470`) schedule `trigger-reward` with `R`
whenever the production is selected — but only for a non-nil `R`, and `R` need
not be a number. `propagate-reward` (`utility-and-reward-1.lisp:421`) branches
on `numberp`: a number runs the `U(n) = U(n-1) + alpha*(R(n) - U(n-1))` update
over the history, while **any other non-nil value clears the history
outright** (`utility-and-reward-1.lisp:463`), printing `  Non-numeric reward
clears utility learning history.` under `:ult`. `ul-issues-v2` relies on this:
`unknown-response` has `:reward t`, so its selection at 12.05 discards the
`choose-a`/`unknown-response` entries, and the reward `0` at 19.05 updates only
`choose-b` and `response-doesnt-match`. `model::Reward` carries either a
number or the source text; the composed-production reward is inherited with
ACT-R's `or`-chain (numeric max when both parents are numeric).

The `PROPAGATE-REWARD` trace line prints the reward with `~s`, so an integer
reward reads `0`, not `0.0`.

## 209. `:er` permutes the whole conflict set — even a lone candidate

With `:er` on, `conflict-resolution` ends with
`(when (and (listp best) best er) (setf best (permute-list best)))`
(`core-modules/procedural.lisp:2261`) and takes `(car best)`. Two consequences
the trace depends on: the permutation runs **even when only one production
matched**, and `permute-list` (`framework/random.lisp:562`) draws
`act-r-random` once per remaining item — `n` draws for an `n`-element list,
`random(1)` included. `permute-list` builds its result with `cons`, so the
returned list is in *reverse* pick order: the item drawn last comes first.
Probing ACT-R confirms both (`(permute-list '(x))` advances the stream by one
draw; for `(p q)` the head is the element *not* drawn first).

The conflict-set order is the order productions were compiled
(`push-last production conflict-set` over `test-set`), which matches the
engine's definition order — verified by instrumenting `conflict-resolution` to
print the set. So only the draw count was wrong: the engine drew once for any
tie and picked `tied[draw]`, which desynchronised the stream for every later
`:er` selection (`tutorial-unit3modeling-v6-next-run` first diverged one
conflict resolution after the first real tie).

## 210. `find-best-feature` draws unless the location chunk's entry is still live

`encoding-complete` -> `get-obj-at-location` -> `featlis-to-focus` ->
`find-best-feature` (`core-modules/vision.lisp:2573`). When the location
chunk's `visicon-entry` is a chunk, still among the features within the
movement allowance, an unmodified copy, and has the *same* `visual-tstamp`,
the entry is returned directly and **no draw** happens. Otherwise it selects
among the nearest features with `random-item`, which consumes exactly one draw
(even for a singleton). A window clear purges the visicon, so the stale
location chunk falls off the fast path and the re-encode
(`update-attended-loc`) draws; a normal `move-attention` encode does not. The
engine keeps its own object choice but consumes the draw, which is what keeps
`tutorial-unit3modeling-v6-next-run`'s later `:er` selection aligned.

## 211. Speech: `:hear-self`, the style-change prep reset, and the notify line

The speech module has two movement styles, `speak` and `subvocalize`
(`defStyle`, `core-modules/speech.lisp:274`/`:310`). Three behaviours the
trace depends on:

- `queue-output-events` creates the self-heard sound only when `:hear-self`
  admits the style (`speech.lisp:294`/`:314`): `t` = both, `nil` = neither,
  `speak`/`subvocalize` = only that one. `hear-self` test-models the four
  modes across four runs, so the same model yields four different traces.
- `compute-prep-time` (`support/general-pm.lisp:670`) compares the *style
  name*, not the text: a style change is a fresh movement (3 features =
  150 ms), and only the same style falls back to `feat-differences` (0 for
  the same text, 2 otherwise). So `speak "Hello"` followed by
  `subvocalize "Goodbye"` preps 150 ms, not 100.
- `notify-speech-device` (`speech.lisp:267`) schedules a traced
  `default-speech-action` (rendered `SPEAK <text>`) **only when no device is
  installed**; otherwise it notifies the device and nothing is traced. A
  `speak` request itself always traces `SPEAK TEXT <text>`, and
  `subvocalize` traces `SUBVOCALIZE TEXT <text>` (its start notification is
  off by default). The sound is queued before the notification, so
  `new-sound` precedes the `SPEAK <text>` line at the same instant.

The device flag comes from `(INSTALL)` in the sidecar, which `write-device`
now emits when the task's `run-it` calls `install-device`. `z-threads`
relies on it (`m()` installs an empty window, so the ref has no notify line
even though the model speaks); its spec cannot replay the device, so the
sidecar carries the flag.

## 212. `audio-event-ended` needs the sound to still be in `AURAL-LOCATION`

`find-sound` (`core-modules/audio.lisp:1045`) schedules `audio-event-ended`
at the sound's `offset = onset + duration` when its event chunk has no
`offset`/`duration`. That handler
`mod-buffer-chunk`s `AURAL-LOCATION` — but only while the buffer still holds
this event (`aural-location-update-check`). So a sound whose offset falls
*before* its location is stuffed produces nothing, while one that outlives
the stuffing emits `MOD-BUFFER-CHUNK AURAL-LOCATION`. `hear-self`'s fourth
run shows both sides: the speak sound (offset 0.500, stuffed 0.550) is
silent, the subvocalize sound (offset 1.100, stuffed 1.050) is not.

## 213. Composed productions substitute ACT-R's `mappings`, not the firings

A `:pct` composed production is printed — and, because the engine parses the
rendered text back, *installed* — by substituting `compose-productions`'
`mappings` (`modules/production-compilation.lisp:1556`), not by instantiating
every variable the two firings bound. Three consequences:

- A variable that is **bound but never mapped stays `=VAR`**: `paired-learning-
  dynamic`'s `POST =STATE` keeps its variable although `state` is bound to
  `RECALLED`. Any "merge p1∪p2 bindings" shortcut is therefore wrong.
- The mappings are applied **one at a time**, and after each one every
  still-queued entry whose car equals the applied entry's car has its car
  rewritten to that entry's cdr (`(dolist (x mappings) (when (eq (car x) (car
  mapping)) (replace … (cons (cdr mapping) (cdr x)))))`). That is how the
  inert-looking `(=DEST . =OTHER)` plus a later `(=DEST . NUMBER)` resolve
  `=OTHER` to `NUMBER`.
- `constant-value-p` (`:2336`) is true unless the value is a `=variable` that
  is *not* a buffer name — so `=RETRIEVAL -> RETRIEVAL-CHUNK0` sorts as a
  *non*-constant cdr (key 0), ahead of the constant/symbol pairs. The sort
  order is load-bearing: the cascade above only fires when the var-to-var entry
  precedes the var-to-constant one.

`mappings` itself is the union of each used buffer's `MAP-*` function. The
engine ports `MAP-RETRIEVAL-BUFFER` (`tools/retrieval-compilation.lisp:71`)
and the `MAP-GOAL-BUFFER` RHS `=` to LHS `=` case
(`tools/goal-compilation.lisp:216`), and only when `:ppm` is **off**: with
`:ppm` the same functions build their slot sets from partially-matched specs
with extra `buffer-condition-union` instantiating, so `ppm-compilation`
(`(sgp :ppm 1.0)`) keeps using the raw firing bindings for its trace.

Ground truth (probe `compose-productions` for the mappings), for
`RETRIEVE-ASSOCIATE & RETRIEVAL-SUCCESSFUL` in pop order: `=DEST -> =OTHER`,
`=RETRIEVAL -> RETRIEVAL-CHUNK0`, `RETRIEVING-RESULT -> RETRIEVING-RESULT`,
`=VAL -> "8"`, `=DEST -> NUMBER`, `=STIMULUS -> "xray"`, `=TARGET -> WORD`.

## 214. A stub's `ssp :starting-parameters :seed` is the effective seed

`(ssp :starting-parameters (:seed (S C)))` in a test-model stub is evaluated at
*load* and re-applied by the central-parameters reset hook
(`central-parameters.lisp:131,140`), so it — not a model-body `(sgp :seed …)`
— is the seed a `:run-full-time` actually ran under. The extractor's `w-ssp`
only records `:drive`-mode calls, and the stub is evaluated in `:load` mode, so
the seed was silently dropped from the spec: `:RUNS` carried the `(100 0)`
placeholder and the engine fell back to seeding with it.

That placeholder is **not** a replayable seed. `canon-runs`/`canon-steps`
substitute `(100 0)` for any run seed that does not match a literal in the
model/task sources, so applying it puts the engine on `init_genrand(100)`
while ACT-R is on whatever state the load left. `make-spec` now records the
stub seed as an `(:SGP :STARTING-PARAMETERS (:SEED (S C)))` step (which the
engine stores and `reset` re-applies), and the harness no longer applies a
per-run seed of `(100 0)` — neither in `Engine::new` nor in the
`:run-full-time` step (whose comment claimed it "is never canonicalized",
which is not true for the `:STEPS` form).

This is what `paired-learning-dynamic` needed: with the seed in place its
trace matches byte-exact, including the `:utility` values that had looked like
a compiled-production utility-equation bug. Diagnosing it needed a
draw-by-draw comparison — instrumenting `Mt::noise` against a wrapped
`genrand_real2`/`act-r-noise` showed both sides made the same number of noise
draws (95) but different values from the first one, i.e. an aligned draw
schedule over a different generator state.

## 215. `!output!` is gated by the printing module's `:v` stream

`!output!` expands to `model-output`, and ACT-R's `model-output` reads the
printing module's `V` and does nothing when there is no stream
(`framework/misc-utils.lisp:801`):

```lisp
(setf ,stream (act-r-output-stream (printing-module-v ,module)))
(when ,stream (evaluate-act-r-command "model-output" ...))
```

So `(sgp :v nil)` suppresses `!output!` along with the trace — and since the
trace is off there is nothing else in the stream to mark the gap, which is why
this hides so well: the `!output!` text simply appears where ACT-R printed
nothing. The engine has an `out()` helper that applies exactly this gate for
raw `model-output` lines, but `RhsAction::Output` in `Engine::fire` pushed
straight onto `out_lines`, so it leaked with the trace off.

The oracle is `actr7.x/test-models/tutorial_unit2code_buffer_chunk.ref`: the
model runs with `(:v nil)`, so the addition model's `!output! (SEVEN)` must not
appear, and the ref goes straight from the `>>>` echo to the buffer dump. The
leaked `SEVEN` was also what put the engine's output *first*, ahead of the
`>>> actr.buffer_chunk('goal','visual')` echo line.

## 216. A client `clear-exp-window` must schedule the attended-location re-encode

`update-attended-loc` (`vision.lisp:1870`) re-encodes the attended object when it
is gone, and the re-encode holds the visual module busy for
`:visual-attention-latency` (default 0.085 s, `engine.rs:925`). The scripted
device timeline reaches that from `proc_display`, which only learns about a
clear through `pending_clears` — so the `(CLEAR ms)` sidecar ops are loaded into
`pending_clears` at build time (`engine.rs:2051`) and the wave sets `cleared`.

The RPC `clear-exp-window` instead calls `Engine::clear_window`, which emptied
the visicon directly and scheduled only a `ProcDisplay`, so `cleared` stayed
false and the re-encode never happened: the visual module looked free 85 ms too
early. In `paired-learning-dynamic` that moved `production-fired read` from
5.135 to 5.050, and every later event inherited the 85 ms offset (the first
divergence was line 207 of 803).

Note the two paths cannot be unified by pushing a `pending_clear` from
`clear_window`: `clear_window` empties the visicon *immediately* while
`add-text-to-exp-window` also adds its features immediately, so a pending clear
consumed by the next wave would run `self.features.clear()` *after* the add and
wipe the new text (the model then never places a visual-location at all).
`clear_window` must therefore schedule the re-encode itself.

## 217. `set-system-parameter-value` survives a model load

ACT-R's `set-system-parameter-value` sets a *global* (`central-parameters.lisp:131`)
which the central-parameters reset hook re-applies at every reset; loading a
model does not clear it. The RPC session kept the settings in `system_params`
but rebuilt the engine from scratch on every `load-act-r-model`
(`build_engine`, `rpc/session.rs`), so they were dropped.

That mattered for the seed specifically. `test-models/paired-learning-dynamic.py`
does

```python
paired = general_import.import_path('../tutorial/python/paired.py', True)   # loads unit4's model
actr.set_system_parameter_value(':starting-parameters', [..., ':seed', [1,0], ...])
actr.load_act_r_model('ACT-R:tutorial;unit8;paired-dynamic-model.lisp')    # rebuilds
```

`paired.py:19` loads `tutorial;unit4;paired-model.lisp`, whose body sets
`(sgp :seed (200 4))`. After the rebuild the session's `:seed (1 0)` was gone
and the run's stream was `init_genrand(200)` instead of the `init_genrand(1)`
the ref used. `build_engine` now replays the stored system parameters onto the
fresh engine.

Verification that it is the right stream: with the fix the RPC run's `noise`
draws are *value-for-value identical* to the trace run's 95 draws, merely
shifted by one index — i.e. the model is on ACT-R's stream and only one
pre-run draw differs.

## 218. `permute-list` draws n times, including the final single-element pass

ACT-R's `permute-list` (`framework/random.lisp:562`) picks a random element each
pass and removes it:

```lisp
(do* ((item (nth (act-r-random (length lis)) lis)
           (nth (act-r-random (length temp)) temp))
      (temp (remove item lis :count 1) ...)
      (result (list item) (cons item result)))
     ((null temp) result))
```

`do*` evaluates the step forms *before* the end test, so the pass that empties
`temp` still draws — `(act-r-random (length temp))` with `length` 1. An n-element
list therefore consumes **n** draws, not n-1.

`Mt::permute_list` (`rand.rs`) already models this (`while !temp.is_empty()`),
but the RPC `permute-list` handler was a Fisher-Yates loop
(`for i in (1..n).rev()`), which consumes n-1. The client's `actr.permute_list`
sends a 2-element index list, so every RPC run of a permuting task sat one draw
behind ACT-R for the rest of the model.

This is the "one pre-run draw short" symptom that kept `paired-learning-dynamic`
failing: it showed up only after PORTING-NOTES #217 put the run on the right
`init_genrand(1)` stream, at which point the whole 95-draw `noise` sequence
matched the ref value-for-value but shifted by one index. Confirmed by tagging
every `random_int` call site under `DBG_RNG`: the trace spec logged
`RNG_DRAW n=1` before its first `noise`, the RPC model only
`AR_RANDOM_RPC limit=2`; after the fix it logs `limit=2` then `limit=1` and the
two streams coincide.

## 219. A remote callback that calls back in needs the session borrow released

The server implements `Remote` on the connection
(`rpc/server.rs`): to invoke a client-owned command it writes an `evaluate`
callback and reads until the matching reply, **dropping** anything else —
"a well-behaved client won't" interleave.

`paired-learning`'s `:testing-function` is not well-behaved in that sense, and
legitimately so. `test-models/paired-learning.py` registers a Python command and
passes it as `:testing-function`; the model-tester's reset hook
(`model-tester.cl:485`) fires it at each `reset`, and the function itself calls
`actr.set_parameter_value(':v'/'cmdt'/'alpha', …)`. Those nested requests arrive
while the server is inside the callback, get dropped, and the client blocks
forever waiting for replies that never come — the RPC run hung with *zero* trace
output.

The borrow is why: `Session::evaluate` holds `&mut Session` across
`remote.call`, so the nested request has no way to reach the session. Dropping
it was the only option available there.

Fix: a `reset` defers a *client-owned* `:testing-function` to the connection
layer (`Session::take_pending_testing_fn`), which runs it with the borrow
released (`server.rs::call_remote_reentrant`) and dispatches the callback's
nested requests through the ordinary `dispatch`, so they are answered and the
client can return. The local `set-testing-traces` fallback still runs inline —
it needs no callback.

Other callback sites (monitors, remote commands invoked outside reset) are still
called from inside `evaluate` and have the same latent deadlock. The general fix
is the same shape — run callbacks with the borrow released, i.e. an effect
queue rather than a nested call — but only the reset hook is exercised so far.

## 220. `:starting-parameters` must go through the engine, not pair-by-pair

`(ssp :starting-parameters (:key val …))` is special: `Engine::apply_sgp` has an
arm for the `:STARTING-PARAMETERS` *key* which both applies each nested parameter
and records it in `starting_params`, so `reset` re-applies the whole set
(`central-parameters.lisp:131`).

The RPC `set_system-parameter` handler skipped that arm and walked the client's
list itself, calling `apply_sgp(k, v)` per pair. The parameters therefore took
effect once, at the moment the client set them, and were **forgotten**: nothing
recorded them, so no `reset` ever re-applied them. (It also had to skip
`:testing-function` by hand, a symptom of doing the engine's job outside it.)

`paired-learning` is what that costs. Its driver runs two tasks back to back
(`paired.task(2,2)` then `paired.task(1,10)`) and each `do_experiment` calls
`actr.reset()`. ACT-R re-applies `(:seed (1 0))` at each of those resets, so the
second task's random stream *restarts* — visible as a backwards jump in the
`noise` draw positions (stream index 112 → 3) in the reference run. With the
parameters forgotten there was no re-seed, the stream ran straight on, and the
second task diverged.

Diagnosis that pinned it, in order: map each logged `noise` value back to its
index in `init_genrand(1)` and diff the index sequences (only the trace path
jumps backwards); then log `cfg.seed` at the reset's seed point — which showed
the three load-time resets seeding `(200 4)` from unit4's model body and the two
task resets seeding nothing at all.

## 221. `spp` reads production parameters; `no-output` wraps a command

Two commands the `bst` task driver needs, both with a shape that is easy to get
wrong:

`spp` (`utility-and-reward-1.lisp:800`) is "production :key [value] …". A key
*followed by a value* is a set; a key with nothing after it is a **read**, and
ACT-R then returns one value list *per production* — `(spp 'p :u)` → `((13))`.
Since the dispatcher wraps a command's value in a list, the RPC reply is
`[[[13]]]`, which is why the client indexes three deep:

```python
# tutorial/python/bst.py:299
return actr.no_output("spp",prod,":u")[0][0]
```

The RPC handler had folded `spp` into the `sgp`/`sdp` arm, so a read fell into
the setter branch: `("p", ":u")` became the pair `:p → :u`, `apply_sgp` ignored
it, and the reply was `[null]` — `[0][0]` then raised and killed the driver
process (which is why the sweep showed no verdict at all rather than a failure).

`no-output` (`external-no-output`, `framework/printing.lisp:470`) runs the named
command with its output suppressed and returns that command's value. The engine
must *drop* whatever trace the inner command queued — the RPC layer delivers
trace to the client's monitors after every request, so leaving it would print
the report the wrapper exists to hide.

Both had to be added to the built-in command-name table as well: `Session::
evaluate` looks the name up there before dispatching, so an unregistered
`no-output` made the inner call fail and the client saw `False` rather than an
error. That is a quiet failure mode worth remembering — check the table first
when a new command "returns nothing".

## 222. A button's label must be centred on the button

`add-button-to-exp-window` creates the button box *and* a separate text item for
its label. Where that label sits matters more than it looks: the initial
visual-location is chosen by `newest_feature` (`vision.lisp:1978`), which takes
the **leftmost still-`NEW` column**, then the newest tstamp, sorts the ties by
name, and picks one with `random-item`.

The engine placed the label left-aligned inside the button (`x + 1`, as
`add_window_text` does for a standalone text item), which put the four labels in
a column of their own — left of the boxes — so the candidate set was the four
labels and the engine placed `VISUAL-LOCATION3`. ACT-R centres the button's
text, so its leftmost column holds the box/label pairs (0,1,2,3) and the ref
places `VISUAL-LOCATION2`. Centring the label (`lx = x + (width - 7·len)/2`)
makes the engine place 2 as well.

The oracle proved the feature *count* too: `bst-learn.ref`'s stick items are
`VISUAL-LOCATION8..11`, i.e. 8 button features precede the 4 line features. An
earlier attempt that emitted one feature per button (assuming ACT-R keeps no
separate label item) gave the right column set but the wrong numbering —
`VISUAL-LOCATION1` — so the two-feature layout is right and only the centring
was wrong.

## 223. Device buttons and lines are visicon features, not decoration

A device task's window can be built from `add-button-to-exp-window` and
`add-line-to-exp-window`, not just text. Both become **visicon features**
(`uwi.lisp:358`/`:511`), the model's initial visual-location is chosen from
them, and `bst`'s whole experiment is four buttons plus four lines — so the
trace corpus could not replay a bst run at all: the extractor recorded only
`add-text-to-exp-window`/`clear-exp-window`, its sidecar therefore described an
empty window, and the run produced no vision events (17 trace lines against a
8872-line ref).

The extractor now wraps both calls, the sidecar carries
`(ADD-BUTTON ms "text" x y w h "color" "action-form")` and
`(ADD-LINE ms x1 y1 x2 y2 "color")`, `DeviceSetup` gained `buttons`/`lines`
with the same per-run (`(RUN n …)`) grouping as texts, and `proc_display`
applies due ops **through the same `add_window_button`/`add_window_line`
helpers the RPC device API uses** — so a scripted replay and a driven run build
the same visicon. Buttons and lines are merged by `at_ms` because a task
interleaves them with each other and with texts.

Two scheduling details matter and are easy to get wrong:

- the construction-time "display already due at t=0" push covers run 0 only; a
  task re-applies its window per run, so a reset that arms ops already due needs
  its own wave;
- that wave must be queued when a **run** starts, not only at reset: a device
  spec's steps can call a command that rebuilds the engine (`chunk-type`),
  which drops a wave pushed at reset time. Restricting the run-start check to
  buttons/lines keeps it from disturbing the text path, which already has its
  own push (`tests/extra_traces.rs`'s cases caught that immediately).

## 224. An in-place engine rebuild resets the device run counter

A device task's window script is armed **per run**: the sidecar's `(RUN n …)`
ops are filtered by `Engine::device_run`, which `reset` increments. But several
commands mutate the model *definition* and then rebuild the engine from scratch
via `build_engine` — `chunk-type` and `add-dm` both do. A fresh engine's
construction reset arms **run 0**, so the run in progress (1, 2, …) found no
window ops at all.

That is what kept the four trace `bst-*` specs stuck. `bst.lisp`'s
`build-display` draws 4 buttons + 4 lines at ms 0 in every run, and the
regenerated spec replays `(:RESET) (:COMMAND CHUNK-TYPE …) (:RUN 60 …)` per
trial — so the `chunk-type` rebuild wiped the ops the reset had just armed, and
the run ended after the initial goal + one conflict-resolution. The tell is the
terminator: the queue draining ("no events left to process") is *correct* for a
plain `:RUN` (`run_secs`); only `:RUN-FULL-TIME` pushes a hidden `Dummy` at the
limit, so "time limit reached" vs "no events left" says which step the spec
used. Chasing it as a scheduler fault sent the previous session down a
blind alley.

Fix: `Session::rebuild_engine_in_place` carries `device_run` across the rebuild
and re-arms that run (`Engine::set_device_run` → `arm_device_script`, factored
out of `reset_impl`). Same-commit invariant: `display_queued_ms` describes a
wave *in the queue*, so `reset_impl` clears it next to
`self.queue = BinaryHeap::new()` — otherwise a dropped queue leaves a stale flag
that suppresses the replacement wave.

Effect: all four `bst-*` specs now run the display and reach the model's first
manual action. The blocker moves to the motor module's `:loc` request slot.

## 225. The manual module's built-in command types are not behind a flag

`core-modules/motor.lisp` defines the base manual styles unconditionally —
`punch`, `peck`, `peck-recoil`, `hold-punch`, `hold-peck`, `point-finger`,
`release`, `hand-to-home` — and the virtual devices add `move-cursor`,
`click-mouse`, `hold-mouse`, `release-mouse`. Only *further* commands come from
`extended-motor-actions` (`hold-key`, `type-key`, `prepare`/`execute`,
`delayed-punch`, …); act-touch's `tap`/`swipe`/… are separately gated.

The engine gated the whole set behind `Config::zz_ext`, so a plain tutorial
model could not move at all: `bst`'s `+manual> isa move-cursor loc
=visual-location` died with `manual request slot ("LOC", …)` and keypad-test's
`isa punch hand right finger index` with `("HAND", RIGHT)` — both *standard*
ACT-R. The base set (types and their `hand`/`finger`/`loc`/`r`/`theta` slots) is
now ungated; the extra commands stay gated. That is safe by construction: the
ungated types previously *errored*, so no passing spec can depend on the old
behaviour.

## 226. One `proc-display` per window build, not one per item

A window op is only a visicon change, and ACT-R recomputes the visicon once per
cycle: `bst`'s `build-display` calls `add-button`/`add-line` nine times before
its run and the ref shows **one** `visicon-update`, not nine. The engine queued
a `ProcDisplay` per op.

Coalesce the *window helpers* at the same time (`Engine::now_display`) — and
note the wave being executed already counts as that time's wave, so
`proc_display` marks itself queued before it applies scripted ops. Without that,
the first item it adds queues a second wave and the run shows two updates at
t=0, which is exactly the off-by-one `bst` displayed. Do **not** route this
through `add-visicon-features`/`delete-visicon-features`/`clear-visicon`: the
device-timeline specs rely on their per-call push, and doing all nine sites
dropped the trace suite to 98/100.

## 227. A task's tones must be *replayed*, not just traced

`new-tone-sound` (`audio.lisp:778`) builds a sound object; `new-sound-event`
then schedules `new-sound` at the onset and `stuff-sound-buffer` at
`onset + tone-detect-delay`. Only the *trace line* was reaching a trace replay:
the task's `(new-tone-sound 100 .1)` schedules `new-sound` outside a run, so
`record-command` captured it as a sidecar `(COMMAND … "new-sound")`, and the
engine replayed that line with no sound behind it — hence the aural-location
events never arrived (`siegler` line 3, `unstuff-aural` line 5).

Fix: record the *call* as a replayed command (`*generic-commands*`, so
`(:COMMAND NEW-TONE-SOUND 100 0.1)` is a step, exercising the client surface),
and stop recording its internal `new-sound` schedule — the re-entrancy guard
`*recording-command*` now applies to the sidecar path too. Three units/detail bugs
fell out of it:

- `new-tone-sound`'s last argument says whether **both** numbers are
  milliseconds; otherwise they are seconds (`safe-seconds->ms`). The RPC handler
  truncated the seconds value, so `(new-tone-sound 100 .1)` was a 0 ms sound and
  its end check fired before the buffer was even stuffed.
- ACT-R stores the sound's **absolute** onset (`mp-time-ms` when none is given),
  and derives `detect-at-time` *and* `offset` from it. Passing the argument
  through made every tone a task sounded between runs land its detection at
  `0 + delay`.
- `AudioEventEnded`'s line is `:output high` (`audio.lisp:1048`). Of the refs
  that reach `audio-event-ended`, only `unstuff-aural` prints it — it is the
  only one whose model sets `(sgp :trace-detail high)`; the `mod-buffer-chunk`
  that follows is medium and shows in all of them.

Still open there: `stuff-sound-buffer` only sets `AURAL-LOCATION` while the
buffer is empty, or overstuff applies with the buffer unrequested and the chunk
unmodified/equal to the last stuffed event (`audio.lisp:716`). The engine sets
it unconditionally, so `unstuff-aural`'s runs 2–4 stuff again (568 lines
against 448; it matches to line 19, was 5).

## 228. `stuff-sound-buffer` keeps the first location unless asked to overstuff

`stuff-sound-buffer` (`audio.lisp:716`) reaches `find-sound` only while the
buffer is empty, or the chunk there is safe to overwrite:
`:overstuff-aural-location` on, the chunk auto-stuffed (`unrequested`) and an
**unmodified** copy. Both parameters default to nil
(`audio.lisp:1534`/`:1544`), so a module that is not asked to overstuff keeps
its first location — `hear-self` and `audio-updates` show one
`SET-BUFFER-CHUNK AURAL-LOCATION` and nothing after it, and `unstuff-aural`'s
first block (`overstuff t`, but the model *modifies* the location) keeps it too.

With `:stuffed t` on a non-empty buffer the set becomes an **overwrite**
(`schedule-overwrite-buffer-chunk`, `audio.lisp:1039`), which prints
`OVERWRITE-BUFFER-CHUNK` rather than `SET-BUFFER-CHUNK`. The engine set the
buffer unconditionally, so it matched neither block.

**Done**: the unstuff half. `unstuff-buffer` is armed at each stuff
(`audio.lisp:1056`) — at `offset + :decay-time` for `t`, after a delay for a
number — and erases the location. Two constants had to be right, and guessing
either wrong puts the erases in the wrong blocks:

- `decay-time` is **3000** ms, not the 5000 the first arithmetic suggested. An
  ACT-R probe of the `unstuff-aural` blocks settled it (`decay-time 3000`, and
  `audio-event1` stuffed at 50 armed its unstuff at 5100 = `offset 2100 + 3000`).
- the precondition **does** require an unmodified copy, like the stuff test —
  the probe shows the check returning NIL when the model has modified the chunk
  (`(:CHECK 2000 3 (module AURAL-LOCATION NIL) NIL AUDIO-EVENT0)`), so a block
  where the model runs erases nothing.

With both right, of the four armed unstuffs only the two whose sound is still
the buffer's chunk erase, at 5100 and 10550 — matching the ref exactly, and
`unstuff-aural` is byte-exact (448/448, enabled).

## 229. A window button is an `oval` whose value is its text

`bst`'s productions ask for the on-screen buttons *by feature*:
`+visual-location> isa visual-location, kind oval, value "b"`
(`tutorial/unit6/bst-model.lisp`), and for the reset button `value "reset"`. So
in ACT-R `add-button-to-exp-window` produces one feature whose `kind` is `oval`
and whose `value` is the button's text. The engine builds two features with
neither:

```
FEAT VISUAL-LOCATION0 kind=BUTTON value=BUTTON-VDI-0   ; our box
FEAT VISUAL-LOCATION1 kind=TEXT   value=A              ; our label
```

Every such request therefore fails, the model takes `find-loc-failure`, and the
run ends — which is what stops `bst-nolearn` at 141 trace lines with the first
81 matching.

The earlier TODO diagnosis (a `!eval!` / `=over`/`=under` difference) was
wrong. Instrumenting the LHS shows the evaluation and the selection are both
correct: for trial 1's bindings `(< 97 (- 62 25))` → NIL and
`(< 62 (- 97 25))` → T, so `DECIDE-UNDER` is in the conflict set, wins the tie
(all three candidates have equal utility) and *is* fired (`FIRED idx=21
DECIDE-UNDER`). The trace's `production-fired decide-over` at the same 1.325 is
a *different trial's* line — its later times (2.143) match trial 2's selection
times (trial 1's are 2.186) — so the trace-visibility bookkeeping needs settling
before line-by-line diffs on this spec are trustworthy.

**Fixed** by probing ACT-R's own visicon (`print-visicon` after
`build-display`, `uwi.lisp:401`):

```
VISUAL-LOCATION0  NEW  (325 335 1080)  OVAL  GRAY   "A"  24  40
VISUAL-LOCATION1  NEW  (325 335 1080)  TEXT  BLACK  "A"  10  7
```

`add-button-to-exp-window` makes `(isa (visual-location oval) … value (oval
<text>) …)`: the button is an **oval** whose value is its text. The engine now
builds `kind OVAL` with the text as value. Note also that the *text* features are
co-located with the oval (same `screen-x/y`), and that ACT-R matches a requested
`value` case-insensitively — `bst` asks `value "b"` / `value "reset"` against the
visicon's `"B"` / `"Reset"` — so `loc_matches` compares strings that way now.

## 230. `no-output` suppresses one command, not everything queued before it

ACT-R's `no-output` runs a command with the output stream bound to nothing, so
*that command's* lines never exist — the lines already emitted stay. The RPC
handler instead called `take_trace()`, which advances the read cursor to the end
of the queue and so discarded **every pending line**, including output from
before the call.

That is invisible while a spec's command steps precede its runs (nothing is
pending), but the `bst` Python driver calls `no-output spp` *between* trials, so
each call wiped the previous trials' trace. The handler now records the engine's
`out_len` before the call and truncates back to it afterwards (clamping
`trace_pos`), which is the stream-binding behaviour.

## 231. An in-place rebuild must keep the trace history

`build_engine` creates a fresh engine, and `Engine::new` clears `out_lines` —
the construction reset is deliberately not part of the recorded trace
(`engine.rs:2310`). That is right for a model load but wrong for `chunk-type` /
`add-dm`, which rebuild *in place* for a definition change; ACT-R's `chunk-type`
does not touch the trace.

The symptom is subtle: a spec that interleaves a rebuild with its runs keeps only
the **last** trial's output, because every earlier trial's lines are discarded
before its run. `bst-nolearn` was compared as 147 lines (one trial) against a
2484-line ref, and the trial that survived was the one whose decision happened
to differ from the ref's first — which made the divergence look like a model
semantics bug (see #229) rather than a bookkeeping one.

`rebuild_engine_in_place` now carries `out_since(0)` across the rebuild. The
trace accumulates 150 → 297 → 447 → 594 lines and `bst-nolearn`'s first
divergence moves 82 → 97. Diagnostic: log `out_len()` before/after each run
step — it read `before=0` for every trial, which is what located it.

## 232. Partial matching narrates nil-valued `-` slots, and always prints the effective value

`partial-matching` (`declarative-memory.lisp:2583`) walks **every** `=`/`-` slot
spec and skips only request parameters (keywords) and variables:

```lisp
(when (and (or (eq (car k) '=) (eq (car k) '-))
           (not (keywordp (second k)))
           (not (chunk-spec-variable-p (third k))))
```

A **nil** request value is *not* skipped. The engine skipped it, so a request
like `+retrieval> isa math-fact - arg1 nil - arg2 nil - result nil` (PORTING-NOTES
#229's shape, `declarative-issues-v8.lisp:155`) produced no per-slot lines at
all. It also dropped the closing line in the `sim ≠ ms` branch: ACT-R emits the
"no effect" note *inside* the `cond`, then prints
`effective similarity value is` **unconditionally** (`:2599-2607`), so the
narration for such a slot is five lines, not four.

Neither change moves a number. A nil request against a non-nil slot value has
`sim = md ≠ ms`, so it contributes 0 — which is what the ref's own
`Total similarity score 0.0` and `negation test with similarity not ms has no
effect` say. `tutorial-unit5modeling-v6-p5-run-with-mp-2` and
`tutorial-unit5modeling-v8-p6-run` were both strict *subsequences* of their refs
(0 lines ours-only, every diff a pure insertion), which is why the fix could not
perturb the activations.

## 233. `:nsji` is three-valued, and `:model-warnings` gates the Sji notice

`compute-sji` (`declarative-memory.lisp:2536`) returns the raw
`mas - log(fan)` value only when `:nsji` is `t`; `warn` (the parameter's default,
`:2261`) clamps at 0 **and emits a warning**, and `nil` clamps silently:

```lisp
(if (eq nsji t) val
  (progn (when (and nsji (minusp val))
           (model-warning "Calculated Sji value between ~s and ~s is negative, but using a value of 0." j i))
         (max val 0.0)))
```

The engine modelled `:nsji` as a bool, so an explicit `(sgp :nsji warn)` was read
as `t` (allowing negatives). It is now a three-state `Nsji`.

The warning is a `model-warning`, gated on `:v` being non-nil **and**
`:model-warnings` (`misc-utils.lisp:976-988`). ACT-R's printing module
initialises `:model-warnings` to `t` (`internal-structures.lisp:689`), but every
trace stub sets `(ssp :starting-parameters (:model-warnings nil))` — and the
extractor does not record `ssp` calls made in `:load` mode — so the refs reflect
`nil`. The engine therefore defaults `:model-warnings` to nil, and a spec turns
it on with an explicit `(:SGP :MODEL-WARNINGS T)` step (which is what
`tutorial-unit5modeling-v8-p6-run` does, while its `-high-detail` sibling does
not). This is a deliberate ref-first choice: a real ACT-R client that relies on
the `t` default would see one extra warning line that we omit.

## 234. A task's `mod-focus` was never recorded, so whole models never started

`(mod-focus-fct (arg1 x arg2 y))` (`goal.lisp:378`) is how a task drives a
trial: `fan-sentence` sets the goal's probe slots with it
(`tutorial/lisp/fan-no-pm.lisp:57`). The extractor wrapped `goal-focus-fct` but
not `mod-focus-fct`, so every spec whose task used it replayed a goal with the
slot *unset* — the model's first production could not match, the queue was empty
at t=0, and the run ended after one conflict-resolution.

`fan-no-pm` was the visible case: 191 trace lines, first divergence at line 2.
Recording the step (a flat `(:mod-focus SLOT VALUE ...)`, which the harness
already parsed) took it to **1830 of 1844** with the first divergence at line
1830 — the remaining gap is the task's own `fan-experiment` summary table, which
is `format` output the spec cannot replay. The same gap is why the
`tutorial_unit5_onehit_*` RPC models never ran: the RPC side had no `mod-focus`
handler at all (`session.rs` listed the name but had no arm), so the request
errored.

## 235. The reset hook lives in the session, and the trace harness must use it

Two independent traps, both of which silently drop output rather than error:

* **`:STARTING-PARAMETERS` carries the reset hook.** A stub sets
  `(ssp :starting-parameters (:testing-function trace-fan-run))`, and
  `trace-fan-run` = `(set-testing-traces)` + `(sgp :act t)`
  (`test-models/fan-no-pm.lisp:3`). ACT-R calls it from inside `reset`
  (`model-tester.cl:485`), which the *session* implements (`invoke_testing_fn`),
  reading the testing function out of its system-parameter store. The harness
  applied `:STARTING-PARAMETERS` straight to the engine, so the store stayed
  empty and the hook never fired — the activation trace was simply absent.

* **`run_spec` picks its driver from the steps.** It routes a spec through the
  session (`run_spec_session`) only when the spec has a `Step::Command`, and
  otherwise through the engine-level `run_model_steps`. `fan-no-pm` has no
  command step, so edits to the session path did nothing for it. Both a
  `:COMMAND` and a `:STARTING-PARAMETERS` step now select the session driver.

Also in this area: the retrieval's tie-break narration is two different
messages — `Chunk ~s with activation ~f is the best` for a unique best
(`declarative-memory.lisp:1488`) and `Chunk ~s chosen among the chunks with
activation ~f` when the winner came out of a tie (`:1515`). The engine used the
first for both.

## 236. The extractor's wrapper list, not its nesting, was the gap

Wrapping a primitive is *transitive*: a stub's `run-it` calls a task function
(`fan-sentence`, `run-test`), and any wrapped primitive *that* calls is recorded
normally. So the recurring "the task's setup vanished" failures were never a
nesting or scoping problem — the wrapper list was simply incomplete. The audit
that shows this, and how to rerun it: take every symbol a stub/task/model calls,
intersect it with the functions the *core* modules define (`core-modules`,
`commands`, `framework`, `other-files`, `modules`), and subtract the wrapped set.
It yields 97 symbols, which fall into exactly three classes:

* **model-body forms** (`sgp`, `spp`, `define-chunks`, `set-base-levels`,
  `declare-buffer-usage`, …) — the model parser owns these, not the extractor;
* **Lisp/task machinery** (`add-act-r-command`, `monitor-act-r-command`,
  `evt-*`, `add-pre/post-event-hook`, `while`, `push-last`, …) — the documented
  "no arbitrary Lisp" divergence;
* **task-level state changes** — the real gaps, and a short list:
  `add-dm`, `module-request`, `set-buffer-chunk`, `clear-buffer`,
  `schedule-event-now`, `schedule-{mod,set}-buffer-chunk`,
  `add/delete/modify-visicon-features`, `permute-list`, `correlation`,
  `mean-deviation`, `merge-dm`, `new-ongoing-sound`, `end-ongoing-sound`,
  `new-digit-sound`, `pbreak`, `open-exp-window`, `add/remove-items-from-exp-window`,
  `create-{text,button,line}-for-exp-window`, `modify-{button,line}-for-exp-window`,
  `delete-production`.

Recording one is cheap when the engine already has the command: add a
`("name" . function)` pair to `*generic-commands*`. Scope is decided by
`*command-tokens*` (a deliberately tiny list of source-scrape/hook tokens), *not*
by which commands were recorded, so extending the list cannot reclassify a spec.
Changing the extractor also does not touch any committed spec — only a targeted
`SPEC_LIST` regen does, which is why this can be landed and verified safely
before the engine side catches up.

## 237. Wrapping a subset leaks a wrapped call made by an unwrapped caller

`install-generic-command` guards *nested* calls with `*recording-command*`: while a
recorded command runs, a command it calls internally is not recorded again. That
guard only covers recorded callers. `module-request` is not a command (it is a
`buffers.lisp` defun with no engine equivalent yet), so when the newly-wrapped
`schedule-event-now` is added, `module-request`'s *internal* schedule leaked into
the spec as 20 bogus steps — and replaying them took `base-level-safety-check`
from 36 trace lines to zero.

So the rule is: adding a pair to `*generic-commands*` is safe only if every
*unwrapped* caller of that primitive is also harmless to record — i.e. wrap the
outer primitive too (which needs an engine command for it), or leave the pair
out. `schedule-event-now` is a genuine task call in
`removed-and-restored-visual-item` and a leak in `base-level-safety-check`; the
spec was reverted rather than the pair, because the pair is correct and the leak
is the thing to fix. The general fix is the same one #236 points at: cover the
outer primitives (`module-request`, `merge-dm`) so the guard applies.

## 238. `printed-visicon`'s slot order is `[SIZE, DISTANCE] ++ construction spec`

A visicon chunk's slots are stored by `push`, so `chunk-filled-slots-list-fct`
(`chunks.lisp:1330`) returns them in *reverse* set order, and
`define-chunks-fct` sets them in reverse spec order. The two reversals cancel:
the printed list is the construction spec with `SIZE` and `DISTANCE` prepended
(they are computed after the spec, `vision.lisp:1741-1747`). The construction
spec is `build-string-feats` for window text — `screen-y height value width
screen-x`, then the device appends `color`, and the isa marker
`(isa (visual-location text))` supplies `kind`/`text` — the recorded argument
order for `add-visicon-features`, and `build-vis-locs-for` for lines and
buttons.

Two consequences that are easy to get wrong:

* the **column order** is the union of the features' non-loc slots with a
  per-slot width and count, `stable-sort`ed by count *descending*
  (`vision.lisp:4126`) — and because attributes are `push`ed, ties keep
  *reverse*-first-seen order;
* a slot set later (`modify-visicon-features`) is `push`ed onto the front of
  the chunk's slot list, so it renders *before* the construction spec, not
  after (`Feature::spec_pre`).

`SIZE` is `simple-size` (`vision.lisp:3698`), and when `width`/`height` are
absent it falls back to `compute-vis-loc-size`'s `1.0`, not `0.0`. Rows are the
visicon in location order (`insert-into-visicon`), and `Att` is `feat-attended`
(`vision.lisp:2269`): a finst is `T`, the in-flight attention target is `REQ`
(a location *copy* like `VISUAL-LOCATION0-1` stands for its original via
`chunk-visicon-entry`), a new feature is `NEW`.

## 239. `warn-vision` holds a proc-display lock across a run boundary

`printed-visicon`'s pending sections are not decoration: a proc-display can be
*skipped*. `warn-vision` (`vision.lisp:3224`) is called from a matched
production's conflict code when it has a `+visual>` RHS request
(`production-parsing.lisp:1439`) and takes the vision module's proc-display
lock. `pm-module-request` (`vision.lisp:3240`) releases it on the next
`+visual>` request and schedules a deferred `unlock-vision`, which re-runs the
display that was blocked. So a `+visual>` production's selection and its
request can straddle a run boundary, and any visicon op in between — a task's
`add-visicon-features`, or a window item — stays pending.

Two things the engine must get right:

* `unlock-vision`'s re-run emits `visicon-update` but **no** `proc-display`
  line (the ref has only the former);
* it is a `:maintenance t` event. The engine's run loop marks buffers changed
  for any non-maintenance event, so scheduling it as an ordinary event wakes an
  extra `CONFLICT-RESOLUTION` and breaks every `+visual>` trace.

## 240. The spec harness keeps Lisp strings distinct from symbol names

A recorded command's arguments go through JSON, which has no symbol type:
`json_to_sexp` maps `Json::Str` to an *uppercased symbol*, which is right for
names but turns `(modify-visicon-features (x value "changed"))` into
`VALUE CHANGED`. The harness therefore wraps a `Sexp::Str` argument as
`{"str": …}` and `json_to_sexp` decodes that back to `Sexp::Str`. Real clients
are unaffected — they send plain strings, which still become symbols.

A related trap: `(:COMMAND NAME (a b c))` is *flattened* into three arguments
by the harness, while a client sends one list argument. `modify-visicon-features`
accepts both shapes.

## 241. `add-visicon-features` names its chunks `VISICON-ID{n}`

The visicon chunk's name is `(new-name "visicon-id")` (`vision.lisp:3856`), and
that is what `add-visicon-features` returns and what a later
`[delete|modify]-visicon-features` passes back — while `printed-visicon` shows
the *location* chunk's name (`CHUNK{n}`, from `chunk-visual-loc`). The engine
queues under the location name, so it keeps an alias map from the
creation-order `VISICON-ID{n}` to `CHUNK{n}` (`Feature`s created so far,
including queued and pending ones). The recorded symbol carries the reader
prefix (`#:VISICON-ID1`), which the lookup strips.

## 242. A task's mid-run device calls are ops, not steps

A monitored-command callback (`add-act-r-command` + `monitor-act-r-command`)
runs *inside* a run, and `install-generic-command` deliberately records nothing
while `*in-run*` is set — so its `print-visicon` calls and its
`remove-items-from-exp-window`/`add-items-to-exp-window` effects vanish from a
step-based spec. The device sidecar is the mechanism for exactly this: a
hand-authored sidecar records them as timed ops, and `write-device` never
overwrites one.

Two shapes the engine now replays:

* `(REMOVE-TEXT ms "text")` — drop the feature carrying that text. A removal
  re-encodes the attended location exactly when the attended object is *gone*
  from the visicon (`update-attended-loc`), which is why run 2 of
  `removed-and-restored-visual-item` re-encodes nothing: its removal takes out
  "a" while the model is attending "b".
* `(PRINT-VISICON ms)` / `(PRINT-VISICON-LATE ms)` — a task `print-visicon`
  inside a run, pure output with no trace line. The direct call must precede
  the display wave at the same instant (the engine's queue pops *higher*
  priority first, so it is `Prio::Max`); the one `redisplay-item` schedules with
  `:priority :min` must follow that wave's own `conflict-resolution`, so it is
  queued from the wave's `visicon-update`, not up front.

Ops and prints at the *same* instant are ordered by the task's call order, so
sidecar ops carry a per-run `seq`: a print at M must not report an op the task
calls at M but later as pending.

## 243. `vl_copies`' feature *index* does not survive a removal

`vl_copies` maps a location copy (`VISUAL-LOCATION0-1`) to its feature's index
in `features`, which a `remove-items-from-exp-window` (or a visicon delete)
shifts. `vl_copied_from` already holds the base *name* — use that for anything
that identifies the feature (`feat-attended`'s `REQ` test, `loc_feature`,
`update-attended-loc`'s "is it gone" test), and leave `vl_copies` for the
lookups that only need the geometry. An encoded visual object also records its
source visicon chunk, which is what makes `buffer-chunk` print
`VISUAL-CHUNK0 [TEXT1]`.

## 244. A model's own commands (`output-key`) must be *fired*, not just traced

`output-key` is a command, not merely a trace line: ACT-R's motor module calls
it from inside the run, and a task monitors it (`monitor-command "output-key"
"paired-response"`, `paired.lisp:135`). The engine only emitted the
`KEYBOARD output-key …` trace line and never executed the command, so every
client monitor on it silently never ran — `onehit.hands` reads its handler's
`model_action` straight after `run-full-time`, and saw the initial `'s'` on
every hand, while the model's own trace showed it pressing `h`.

Two halves: `output-key`/`release-key` have to be *registered* (an unregistered
command cannot take a monitor at all — `add_monitor` drops it), and the engine
records each press (`Engine::take_fired_commands`) for the server to dispatch.

Dispatch has to be **reentrant**, for the same reason `:testing-function` is
(`server.rs::call_remote_reentrant`): `respond-to-key-press` is client-owned and
calls `get-time` from inside its handler, so invoking it under the
`&mut Session` borrow deadlocks the client (`paired-learning` hangs forever).
Client-owned monitors therefore go through `call_remote_reentrant` with the
borrow released; server-side ones run through `Session::evaluate`. ACT-R fires
these mid-run, so the dispatch happens before the `run` reply — a task reads
the handler's effect as soon as `run` returns.

## 245. `sum()` is compensated on Python 3.12+, so an older ref's aggregate can differ by one ULP

`tutorial_unit5_onehit_learning`'s ref records `0.30639999999999995` where the
same computation now yields `0.3064`. That is not an engine difference: the
ref's *own* percentage list recomputes to `0.3064` with today's `sum`, and to
the ref's exact value with a naive left-to-right fold
(`functools.reduce(operator.add, …)`).

CPython 3.12 gave `sum()` Neumaier compensated summation for floats, so summing
five percentages is now one ULP more accurate than when the ref was generated.
The model is untouched: for `onehit.learning(50)` ours reproduces real ACT-R
**exactly** — every one of the 50 games byte-identical, all 20 per-block win
counts identical (1523 total), and the whole 20-element percentage list
identical including its float reprs. Only the four `sum(...)/5` aggregates can
drift.

`fixtures/rpc-conformance.py`'s `normalise` therefore canonicalises decimal
literals with 13+ fractional digits (`format(float(x), ".13g")`) before
comparing. A double does not meaningfully resolve more than that, a passing
spec's lines are identical before and after, and every shorter literal in the
corpus (`0.19999999`, `-1.3374588`, …) is left untouched — so this can only
absorb interpreter float drift, never hide a real difference.

## 246. A mouse ply holds the manual buffer until `finish-movement`, not `initiation-complete`

`?manual> state free` is the manual *buffer's* state, and ACT-R frees it only
when the movement finishes. In `bst-nolearn` the model's `click-mouse`
production (which queries `?manual> state free`) fires at 1.926 — 551 ms after
the `move-cursor` request — while ours fired it at 1.625, at
`preparation-complete` + `initiation-time`.

`ZzMousePrep` (`events.rs`) set `man_exec[..]` — which only a `zz_ext` model's
`query_free` reads — but never `man_exec_busy`, the flag the non-`zz_ext` path
uses. So after `PREPARATION-COMPLETE` (`man_prep_busy = false`) the buffer
looked free. `TouchPrep` (the touch path) already sets `man_exec_busy = true`
at its preparation-complete; the mouse path now does too, and
`FinishMovement`'s non-`zz_ext` arm clears it.

This alone does not make `bst-nolearn` match — the movement *duration* is still
wrong — but it is the correct buffer semantics.

## 247. The base motor module's `click-mouse` never releases; the extended one does

`motor.lisp`'s `punch` (which `click-mouse` is, `mouse.lisp:470`) queues only a
`notify-motor-device` at exec time (`motor.lisp:955`) — no finger-up. The
extended-motor-actions module redefines `queue-output-events` for `punch`
(`motor-extension.lisp:1700`) to also schedule a `set-finger-up` at
`exec2-time` (`init-time + key-release-time`). That `set-finger-up`
(`motor-extension.lisp:1579`) is scheduled with `:output nil` but *not*
`:maintenance`, so it wakes conflict resolution — unlike the `release-mouse`
notification it triggers, which *is* `:maintenance`.

So a `click-mouse` wakes an extra `conflict-resolution` only when the model
requires `extended-motor-actions`. `ZzMousePrep` must therefore schedule its
`ZzMouseRelease` only for a `zz_ext` model; scheduling it unconditionally put a
spurious `CONFLICT-RESOLUTION` at 2.166 in `bst-nolearn`.

## 248. `stuff-visloc-buffer` uses `set-visloc-default`, and `modify-visicon-features` does not refresh the tstamp

`stuff-visloc-buffer` searches with `(default-spec vis-mod)`
(`vision.lisp:1978`), which is `screen-x lowest :attended new` by default
(`vision.lisp:3178`) but is *replaced wholesale* by `set-visloc-default`
(`vision.lisp:4057`). The tracker model calls `(set-visloc-default :attended
nil)`, so its auto-stuff ignores `new` entirely.

`modify-visicon-features` queues an `:update` and schedules a `proc-display`
but `process-display`'s `:update` branch (`vision.lisp:1676`) only
`mod-chunk-fct`s the slots — it does not touch `chunk-visual-tstamp` or
`chunk-visual-new-p`. So a feature modified long after creation keeps its
original (old) tstamp, and `checknew` (`vision.lisp:2227`) clears its `new`
flag. The tracker still stuffs it at 5.500 because its default spec says
`:attended nil`. Ours hardcoded `:attended new` in `newest_feature`, so once a
`new`-age gate was added the tracker stopped stuffing. `newest_feature` now
reads `Model::visloc_default`.

## 249. A visual object's `isa` is `(visual-object <kind>)`

`(chunk-type line (visual-object))` (`vision.lisp`): `line`, `oval` and `text`
are *subtypes* of `visual-object`. The encoded object therefore matches both
`isa line` (e.g. bst's `choose-c`) and `isa visual-object`. The engine stores a
single `ctype`, so `convert_loc_to_object`/the encode path store the feature's
kind and `isa_matches` (`productions.rs`) treats `visual-object` as a
supertype of the three kinds.

## 250. A relational slot test may take a variable on its right-hand side

`choose-c` tests `=visual> <= width =difference`, where `=difference` is bound
by the preceding `=imaginal>` condition. `Engine::rel_test` returned `false`
for a non-numeric RHS, so the production never matched. The `Gt|Lt|Ge|Le` arm
now resolves a `Pattern::Var` RHS from the bindings and defers to
`rel_test_v`.

## 251. `move-cursor`'s Fitts time is in degrees of visual angle

`move-cursor`'s exec is `fitts(cursor-fitts-coeff, r, w)` (`motor.lisp:1281`),
with both `r` and `w` in degrees of visual angle: `r =
pm-pixels-to-angle(pixel-distance)` (`device-interface.lisp:743`, default
viewing distance `ppi × viewing-distance` = 72×15 = 1080) and `w =
pm-pixels-to-angle(approach-width(w, h, atan2(dy, dx)))`
(`vision.lisp:3717`). A target with no width/height falls back to the vision
module's `default-width` (1.0). The virtual cursor starts at the experiment
window's origin (`open-exp-window`'s `:x`/`:y`, default `(300,300)`) —
`mouse.lisp:554`, `initialize-cursor`.

## 252. `move-cursor` to the cursor's current position is a no-op

`motor.lisp:1316` aborts the move when `r = 0` and the target's `z` equals the
cursor's, emitting the `MOVE-CURSOR LOC …` trace line but scheduling no ply —
so the manual module stays free and the next action can start immediately.

## 253. `modify-line-for-exp-window` keeps the line's identity; horizontal lines share a `value`

`bst`'s `update-current-line` redraws the "current" stick after every click:
`add-line-to-exp-window` the first time, then `modify-line-for-exp-window`,
`remove-items-from-exp-window` on a reset, and a modify to the target plus the
"Done" text on the done click. A line's `value` is its *orientation*
(`pts_to_orientation`, `uwi.lisp:511`), so every horizontal line has the same
`value` (90) — a device op cannot key on it. `MODIFY-LINE`/`REMOVE-LINE`
therefore match the line's old endpoints (window-relative), and the redraw
keeps the feature's name and finst (ACT-R's `:update` path,
`internal-modify-visicon-features`).

## 254. `chunk-slot-equal` compares strings case-insensitively

`chunk-slot-equal` (`chunks.lisp:1854`) uses `string-equal` for two strings.
`bst`'s `pick-another-strategy` tests `=visual> value "reset"` against the
Reset button's encoded `"Reset"`, and the request path already compared
case-insensitively (`loc_matches`); the LHS path did not. `Engine::value_eq`
and `test_value`'s `Pattern::Str` arm now match `string-equal`.

## 255. A production's reward propagates after its goal's `set-buffer-chunk`

`learn-parameters` (`utility-and-reward-1.lisp:475`) schedules `trigger-reward`
with `:priority :min`, and `trigger-reward-internal` then schedules
`propagate-reward` with `:priority :max`. The net effect is that the reward
runs *after* the goal module's `set-buffer-chunk` (`:priority -1000`,
`goal-style-module.lisp:245`) but before the conflict-resolution. The engine
fires a single `PropagateReward`, so it must sit between the two: `bst`'s
`read-done` trace shows `set-buffer-chunk goal (state start)` immediately
before `propagate-reward 20`.

## 256. A window rebuild ends the `run`, it is not a scheduled event

A task's `build-display` at the start of a new `run` is a *command the harness
runs between runs*, not a scheduled event, so the previous run drains first:
`bst-learn`'s 15 trials share one `reset`, and its ref shows
`stopped because no events left to process` immediately *before* the new
window's `visicon-update` (both at the same ms). The engine arms a reset's
whole device script at once and `proc_display` chains a `ProcDisplay` to the
next pending op, so the rebuild would otherwise be applied inside the previous
run. `proc_display` now stops chaining when the next op is a *button* re-add —
a window rebuild — leaving the run to drain; the rebuild is then drawn by the
next run's `schedule_display`. The old window is replaced (`(CLEAR ms)` at each
trial start), which also re-encodes a now-gone attended object
(`update-attended-loc`).

## 257. A visual object's `screen-pos` is the *merged* name of the location chunk

`featlis-to-focus` sets the object's `screen-pos` from the attention marker
(`vision.lisp:2524`), but the slot store goes through
`set-chunk-slot-value-fct` → `get-true-slot-value-name`
(`framework/chunks.lisp:1106`), which rewrites a chunk-valued slot to
`true-chunk-name-fct` — the name of the chunk it merged into. An auto-stuffed
location copy (`…-0`) that a `find-location` later re-stuffs (`…-1`) is an
*identical* chunk, so on the intervening `clear-buffer` it merges into DM
(`merge-chunk-into-dm`, `declarative-memory.lisp:1899` →
`merge-chunks-fct`, `chunks.lisp:1658`, keeping the first name) and every
reference is remapped. `removed-and-restored-visual-item`'s `buffer-chunk
visual` therefore prints `screen-pos visual-location0-0` even though the
encoding trace names the marker `visual-location0-1`. The engine already
records that alias (`resolve_alias`); `encoding_complete` now applies it when
building the object. Copies whose slots differ (tracking's moving location)
never merge, so `tracking-test` keeps its `-1…-4`.

## 258. A button's text label sits exactly on the oval, not centred beside it

`button-vdi`'s `build-vis-locs-for` (`uwi.lisp:358`) builds the label with
`start-x = px(view-loc)`, `y-pos = py(view-loc)` and an `x-fct` of
`(- startx (round (str-width obj) 2))`. `build-string-feats`
(`vision.lisp:3586`) then adds `(round curr-width 2)` per word, so the two
cancel: the label's `screen-x` is `px(top-xy) + px(view-loc)` — the oval's own
`screen-x` — and its `screen-y` is likewise the oval's `screen-y`. A label one
pixel off lands in a column of its own, which changes `newest_feature`'s
candidate set (and hence the draw). Verified against the oracle: for
`build-display`'s buttons the ref visicon has VISUAL-LOCATION0/1 (A's oval and
label) both at `(325 335)`. A standalone `add-text-to-exp-window` is a
`static-text-vdi` and keeps the `(1+ x)` / `y + round(line-height, 2)`
placement (`uwi.lisp:459`).

## 259. An attended feature survives a display rebuild, keeping its finst

`process-display` collects every *attended* feature it deletes into
`transfer-finsts` (`vision.lisp:1656`) and, after the rebuilt display is
inserted, `transfer-finst` (`vision.lisp:2306`) moves the finst onto the
matching new feature (`equal-chunks-fct`), stamping it with the finst's time
and copying `chunk-visual-new-p`. So a `clear-exp-window` + `build-display`
rebuild does not make an attended item `new` again: the default spec's
`:attended new` (`vision.lisp:3178`) skips it, and `newest_feature`'s
candidate set is one shorter. `bst-learn`'s per-trial rebuild re-adds the
buttons, and the one the model last attended (A's oval, tstamp from the
attention) is excluded from the trial-start auto-stuff — the ref's set is
`{15…19}`, not `{14…19}`.

## 260. A rebuilt feature at the same place refreshes its location chunk's entry

`define-chunks-fct` merges a new chunk into an existing one with equal slots
(`update-chunks-on-the-fly`), so a rebuilt feature whose location spec matches
an earlier one reuses that **location chunk** — and `build-vis-locs-for` then
reassigns `(chunk-visicon-entry loc-chunk)` to the new visicon chunk
(`vision.lisp:3883`). `find-best-feature`'s short-circuit
(`vision.lisp:2573`) tests exactly that entry, so a window rebuild that
re-adds an item at the same place keeps the *old* location copy's fast path
live even though the copy's own feature is long gone. `bst-learn`'s per-trial
`clear-exp-window` + `build-display` redraws the same sticks at the same
coordinates, so the re-encoded `visual-location38-0` still short-circuits (the
ref consumes no `random-item` draw); `tutorial-unit3modeling-v6`'s cleared
window puts a *different* object elsewhere, so the entry stays stale and the
re-encode draws. The engine records each copy's location slots
(`vl_loc_spec`) and treats the entry as live when a live feature shares them —
the old test (`vl_copied_from`'s base name still present) missed the rebuild,
adding a spurious draw at t=25.799 that shifted `bst-learn`'s trial-5
`stuff-visloc-buffer` pick.

## 261. A window button's `:action` runs as a client callback mid-run

`add-button-to-exp-window`'s `:action` is a command the *client* owns
(`bst`'s `bst-button-pressed`); when the device processes a click the Lisp
server calls it back synchronously, and the task's handler issues more window
commands (`add-line-for-exp-window`) that the server applies at that model
time. The Rust engine never fired the action, so `bst`'s display never changed
and its `run` stopped after the first trial (the trace path sidesteps this by
replaying the task's resulting `ADD-LINE`/`MODIFY-LINE` ops from the device
sidecar). The server now drives `run` in slices: the engine stops as soon as a
click queues a button's action (`record_click_action`, scanning the live ovals
topmost-first) and `run_interleaved` dispatches it through
`call_remote_reentrant` — releasing the `&mut Session` borrow so the client's
nested window commands are serviced — then resumes. Only the RPC path stores
actions, so the trace timeline is unchanged.

`clear_window` no longer schedules the attended-location re-encode itself: the
`proc_display` that `now_display` queues at the same instant sees the feature
gone and schedules it, so doing both emitted the
`encoding-complete`/`no visual-object found` pair twice per rebuilt display
(`bst-learn`'s trial boundary). `add-button-to-exp-window`/`modify-line-for-
exp-window` are now part of the server command table.

## 262. The remote protocol decodes `'quoted'` strings; `run` returns the model time

The Python client sends a *string* as `'lawyer'` and a *symbol* as `lawyer`
(`decode-string`, `dispatcher.lisp:911`): `mod-focus-external` (`goal.lisp:375`)
runs every argument through `decode-string-names`, so `"'lawyer'"` becomes the
Lisp string `"lawyer"` and `"person"` becomes `PERSON`. The server passed the
raw JSON through, so `fan-no-pm`'s goal `arg1` was the string `'lawyer'` and
the `word =person` retrieval never matched `(lawyer ISA meaning word
"lawyer")` — it failed and the task then crashed on
`actr.buffer_slot_value(...).lower()`. `mod-focus` now decodes (and
`buffer-slot-value`, `buffers.lisp:1030`, is implemented — `encode-string`
wraps a string value in single quotes). `run` now returns
`(values (ms->seconds time) events break)` (`scheduling.lisp:877`); the task
reads `run(30)[0]` as its response time, so it was reporting 0.000.

`fan-no-pm`'s trace and response times now match the ref exactly; only the
client's `output_person_location` boolean format differs (`True` vs the Lisp
`T`), which is the Python port's own output, so the RPC verdict stays
unenabled.

## 263. `:sim-hook` overrides the similarity table for a task function

`chunks-similarity` (`declarative-memory.lisp:2628`) calls the `:sim-hook`
function with the two values first and uses its result whenever it is a
number; only a non-number falls through to the explicit Sji table and
`:ms`/`:md`. `bst-ppm` sets `(sgp :ppm 40 :sim-hook "bst-number-sims")`, where
`number-sims` is `(|a-b|)/-300`: `decide-under`'s `goal-length =d, c-len =d`
then partially matches at `-0.23 > :md` (default `-1.0`), where the default
`md` similarity `-1.0 > -1.0` would have failed and the production would never
fire. The hook is a task command (`add-act-r-command`), so the engine resolves
it through `user_fns` and evaluates it with the bounded evaluator (which now
understands `when`/`unless`/`numberp`/`symbolp`/`stringp`).

## 264. A production may carry a documentation string after its name

`(p name "documentation" lhs ==> rhs)` is the documented form
(`production-parsing.lisp`), and the string is not an LHS item. The parser
fed it to `validate-lhs`, which rejected the whole production ("first item on
lhs is not a valid command") and dropped it — `creating-an-image` lost all
five of its productions. The docstring is now skipped, matching the optional
documentation string a chunk type already accepts.

## 265. The server accepts clients concurrently; `run` returns its event count

The Lisp server answers a second connection immediately, and
`tutorial_unit2code_initial` opens one (`actr.connection()`, which sends
`new-symbol` and waits) beside the driver. The Rust listener served clients
*sequentially*, so the second `new-symbol` blocked behind the first
connection's run. Each accepted stream is now handled on its own thread with
its own `Session` (the existing per-connection isolation, just concurrent).

`run`'s second value is ACT-R's `event-count` (`run-sched-queue`,
`scheduling.lisp:627`), which `tutorial_unit2code_initial` prints. The engine
counts each event it evaluates. ACT-R's scheduler has more invisible events
than the engine's, so the engine also queues a no-op `Dummy` for each: the
per-selection `module-request` to the `production` buffer
(`procedural.lisp:2594`), the `record-buffer-trace-notes` that
`print-production-output` schedules for a production `!output!`
(`production-parsing.lisp:1826`), and the reset hook's `check-for-esc-nil`
(`central-parameters.lisp:141`). The last is queued last among the t=0 events
so it cannot displace a `run-n-events` window's display draw. With them the
addition model's `run(1)` matches the ref's `[0.55, 77, nil]` and
`tutorial_unit2code_initial` passes.

## 266. The `imaginal-action` buffer runs a task function to build the next chunk

The imaginal module has a second buffer, `imaginal-action` (`imaginal.lisp:187`).
A production's `+imaginal-action>` names an `action` command and, with
`simple t`, the function runs at request time, the imaginal buffer is cleared,
and after `:imaginal-delay` its result (a chunk name or a `(slot value …)`
list) is placed back; `nil` sets the module error state. A generic request
just runs the function, which schedules its own work. The engine now
registers the buffer, answers the request, and evaluates the action body —
which needs `let*`, `buffer-read`, `copy-chunk-fct`, and `mod-chunk-fct`, all
now in the bounded evaluator. `bst-learn-ppm`'s `encode-line-current` fires
its `bst-compute-difference` action and the imaginal buffer is set, moving the
trace trace from line 70 to 146 (of 8385). Two gaps remain there: the
copy's name (`copy-chunk-fct` should share a per-*base* counter, so ACT-R's
`imaginal-chunk0-2` is the engine's `imaginal-chunk0-0-0`), and the `length`
the action reads.

## 267. `copy-chunk-fct` names from a per-*base* counter, and a clear harvests a reusable chunk

A chunk copy's name comes from `new-name-fct` (`naming-module.lisp:337`),
which keeps a counter keyed by the *base-name*: `copy-chunk-fct`
(`chunks.lisp:860`) sets the source's `base-name` once to `{name}-`, and
every copy down the chain is `{base}{k}`. So `IMAGINAL-CHUNK0` -> `-0`, and
copying `-0` gives `-1` — not `-0-0`. The engine's `copy_chunk` keys the
counter by the full name and chains (`-0-0-0`).

Two names are spent per `imaginal-action` simple request: the action's
`copy-chunk-fct` (`-0`), then the action's `schedule-clear-buffer` clears the
imaginal buffer and `merge-chunk-into-dm` (`declarative-memory.lisp:1872`)
copies the *reusable* chunk before adding it (`-1`). The SET uses `-0`, the
next action `-2`, so the ref's imaginal SET names run `-0`, `-2`, `-4`, ….

An imaginal buffer is a `reuse?` buffer: its chunk is always the reusable
`IMAGINAL-CHUNK0`, and `set-buffer-chunk` copies content *into* it
(`buffers.lisp:1349`), so `buffer-read 'imaginal` returns `IMAGINAL-CHUNK0`
rather than the placed copy. The engine keeps the placed copy's name instead.

## 268. Trace outputs are compared CRLF-insensitively (verified)

`fixtures/extra-traces/*.trace` are committed with LF in the index, but a Windows
checkout with `core.autocrlf=true` (no `.gitattributes` here) writes them as
CRLF. The engine always emits LF, so `tests/extra_traces.rs`'s raw
`actual == expected` failed with `common prefix identical, length differs`
(same line count) on every case — the bytes differed only in the newline
convention. `first_diff` compares via `str::lines()`, which strips `\r`, so
the reported "first diff" never showed a real difference: a tell-tale for an
EOL-only mismatch.

`run_case` now compares `normalize_eol(actual) == normalize_eol(expected)`
where `normalize_eol` maps CRLF→LF. It normalizes the newline convention
**only** — no trimming or case-folding (that is `test_models.rs`'s
`normalize`, which mirrors `compare-test-model-files`). Content stays exact.
This is the sole raw-EOL-sensitive comparison in the suite: the units tables
parse through the sexpr reader (treats `\r` as whitespace) and the device
sidecars split on whitespace, so both are already EOL-tolerant.

## 269. Event hooks: two-phase engine API + direct Rust cases

The `:SCOPE :HOOK` regression refs drive the model through an
`add-pre/post-event-hook` filter written in arbitrary Lisp, which the engine
cannot execute (`README.md:5`, the "no arbitrary Lisp" divergence). Rather
than leave all 73 ignored, the *effect* of the projection filters is
reproduced in Rust.

Engine side (`src/engine.rs`): `EventHook` mirrors ACT-R's pre/post pair
(`framework/scheduling.lisp:717`/`:775`). `pre(&EventView)` runs before the
event and may return `Verdict::Break` — which, like ACT-R's `"break"`
return, *withholds* the event (it stays queued, so a later `run-n-events`
executes it) and stops the run; `post(&line, &EventView)` receives each trace
line the event emitted. Two subtleties:

* A filter is a **pre**-hook, but `format-event` needs the rendered line,
  which in Rust is only known after `exec`. So the predicate is evaluated
  from the structural `EventView` (its `evt-action`/`evt-module`/`evt-params`
  fields) and the line is projected at `post`.
* A hook that turns tracing off (`sgp :v nil`) must still see the line, so
  `emit` captures `(module, details)` into `current_lines` *before* the `:v`
  gate. One event can emit several lines (`encoding-complete` then
  `No visual-object found`), so `post` fires once per line.

`EventView.shown` = `cfg.detail.shows(out)` is the engine's own answer to the
filter's `evt-output` + `filter-test` pair — never re-derived test-side, so
the projected set matches by construction.

Test side (`tests/hooks/mod.rs`): each case now owns its inline driver and a
direct `EventHook`. `HookCase::check_segment` compares the engine's raw trace;
`HookCase::check_projection` compares the lines a hook collected in a
`Projection`. The 43 cases are eight projection and 35 segment tests; the
remaining ignored ones name the engine gaps listed below. There is no hook IR, no
generator, no hook-case manifest, and no hook `.spec`; only data-heavy
experiments keep specs.

The projection cases write their `format-event`/`model-output` filter as a
plain `post` method (including the `find-unattended-letter` latch and the
`else_dots`/`butlast`/separator shapes). The segment cases write their `:v`
window or break-at-the-Nth behavior as a plain `pre` method, including the
optional `filter2` suppression after the resume. A driver with a command step
or `:STARTING-PARAMETERS` still runs through the RPC `Session`
(`run_session_hook`), installing the hook before the steps.

**Driver recovery.** `extract-specs.lisp` still executes a `:hook` stub with
`add-pre/post-event-hook` and `delete-event-hook` neutralized (`extract-hook`)
to recover a driver for offline inspection. No recovered hook driver is
committed as a `.spec`; each case carries the driver directly in Rust.

The ignored hook cases expose engine gaps: the base-level reference trace
(`v1-*-act-trace`, `v3/v4-run-p2`), the reward/utility trace (`bst-utility`),
`!eval! set-flag` (`v8-eval-issue`), and `pcomp` v9/v10 production-fired
timing (plus the related utility/selection cases).

The remaining unconverted `:HOOK` refs are reclassified by their next blocker:
driver recovery, hook-side system-parameter/production effects, buffer
effects/output, command effects, or hook-side scheduling. The old
"pending filter parsing capabilities" reason is obsolete now that filters are
direct Rust.

## 270. Model-body commands, task globals, and quasiquote (verified)

ACT-R evaluates the model body on load and again on every reset
(`framework/model.lisp:983`). The port now models the forms needed by the
categorize task:

* The reader retains quasiquote/unquote/unquote-splicing forms. `p-fct`
  unwraps its quasiquote and substitutes `(UNQUOTE *QUERY*)` on reset.
* Task-file `defparameter`/`defvar` literals are installed as engine
  `user_vars`; the bounded evaluator resolves them in `dolist`, `mapcan`,
  `setf`/`setq`, and `incf`.
* `m_eval` handles `dolist`, `mapcan`, `extend-possible-slots`,
  `define-chunks-fct`, and `add-dm-fct`. Runtime DM additions update the
  history and spreading-fan tables.
* `Model.call_commands` stores `(call-act-r-command ...)` forms;
  `Engine::apply_model_commands` runs them after the task functions and globals
  are installed and again during reset. Command failures are warnings, matching
  `handle-evaluate-results`.
* Model-body `(set-buffer-chunk '<buffer> '(slot value ...))` specs are stored
  separately from named-chunk initial buffers.
* `slot_index` starts with ACT-R's default/query and visual-location slots, so
  chunk printing orders `NAME`/`STATE`, and visual `SIZE` correctly.

This promoted the categorize object/attribute chunk dumps, sample goal chunks,
initial imaginal chunks, and sample DM dumps. `goal-chunk-part2`, `model-fit`,
and `sample-experiment` still need the task's experiment/trial driver rather
than model-body support alone.

## 271. `print-dm-finsts`, `mp-show-queue`, and audio-hook projection (verified)

`print-dm-finsts` renders the declarative finst table after dropping entries
older than `:declarative-finst-span` (`dm-commands.lisp:1022`). The column
width is `max(11, longest name)`, matching the `~va`/`~v,1,0,'-a` format; the
time is `ms/1000` at three decimals.

`mp-show-queue` drains the heap into execution order, renders each event with
`trace::format_line`, and restores the heap. ACT-R's `schedule-event-after-change`
leaves an armed conflict-resolution in its waiting list, which
`mp-scheduled-events` reports at the next event's time; the queue printer adds
that synthetic `CONFLICT-RESOLUTION` line after the entries at the earliest
pending time.

`EventView` now identifies audio events (`new-sound`, aural-location updates,
`audio-event-ended`, `audio-encoding-complete`) as `AUDIO` rather than leaving
them module-less. The Sperling initial/remaining audio projections keep only
the requested time-window audio/procedural lines. They ignore the engine's
`buffer-read-action`/`query-buffer-action` events because ACT-R does not create
those LHS trace events while `:v` is off; the remaining trace also omits the
maintenance `audio-event-ended` line.

This promoted the two `v4-*-finsts`, `v5-p3-run-queue`, `v6-p4-run-queue`,
and two Sperling audio-trace cases, moving the corpus to **386 passed / 181
ignored**.

## 272. Python command-driver cases and `spp` production casing (verified)

Five `tutorial_unit2code_*` refs are generated from Python stubs, not Lisp
`run-it` bodies. Their drivers were transcribed into the Rust steps: the
`>>> actr.*` prompts and Python return lists are raw `model-output` lines, while
the bodies call `buffer-status`, `buffer-chunk`/`Step::BufferChunk`, `dm`, and
`sdm`. The `sdm` call goes through `Step::Sdm` because the Python call passes
the numeric value as a JSON integer, which the command argument string filter
would drop.

The two Lisp addition cases (`buffer-chunk`, `buffer-status`) use the same
command layer after a trace-off run. `tutorial-unit6-bst-spp-before` also
exercises `spp`; its parameter block now prints the production symbol
uppercased (`FORCE-OVER`), as Lisp's `format` does. This promoted eight cases,
moving the corpus to **394 passed / 173 ignored**.

## 273. Protocol driver, runtime chunk defaults, and warning order (verified)

`tutorial_unit2code_initial` is a Python protocol trace, not a Lisp model: the
driver echoes the connection/load/run/reset prompts and return lists and uses
the existing run/reset steps. It passes byte-for-byte, including the
`[0.55, 77, None]` event counts.

`define-chunks` applied no chunk-type defaults after model load. The final
default pass is now shared as `Model::apply_chunk_defaults`, and the RPC
definition path calls it after adding chunks; the bounded
`define-chunks-fct`/`add-dm-fct` path applies the same defaults while building
`Value` slots. This promotes `tutorial-unit6-chunks-with-default-slots`.

`sgp` getters now print the `value (default ...) : documentation` line for
`:trace-detail` and `:lf`, which is the form `tutorial-unit1code-sgp-result`
records.

`create-undefined-chunk` warnings are emitted LHS-first across the whole model,
then RHS, matching `tutorial-unit1-semantic-warnings`: the later production's
LHS `CATEGORY` warning precedes the first production's RHS `PENDING` warning.
The reorder preserves the positions of other warning types. This promoted four
cases, moving the corpus to **398 passed / 169 ignored**.

## 274. Semantic command drivers and unmodified `sdm` specs (verified)

The remaining unit-1 semantic command refs are direct drivers over the existing
command layer: `dm` (all and named chunks), `sdm` search, `whynot`, and
`buffer-chunk` after a trace-off run. The goal-focus case repeats
`Run`/`GoalFocus` with the ref's event-count return lines. These promoted five
cases.

`sdm-fct` (`commands/dm-commands.lisp:491`) treats every non-empty spec as a
filter; only an empty spec means print all. The engine previously filtered
only when a `-`/`+` modifier was present, so `(sdm object shark)` printed the
whole DM. It now filters plain slot/value specs too, while retaining
`(sdm isa pair)` as the corpus's print-everything form. The semantic-sdm case
still differs on slot order inside the printed `property` chunks and remains
ignored.

The corpus is now at **403 passed / 164 ignored**.

## 275. Retrieval-buffer copies and all-buffer `buffer-chunk` (verified)

`set-buffer-chunk` on a reusable buffer names its chunk once as
`{BUFFER}-CHUNK{n}` (`act-r-buffer-reuse-chunk`, `buffers.lisp:1351`). A
production that mentions `=RETRIEVAL` turns reuse off via
`buffer-requires-copies`, so retrieval chunks use the generic `{SOURCE}-{k}`
copy name instead. The engine now applies that split: count models get
`RETRIEVAL-CHUNK0 [TWO]`, pcomp/fan models keep `THREE-0`, `LAWYER-0`, and the
like. `buffer-chunk` headers read the copy source from `copy_source` as well as
the visual map.

`(buffer-chunk)` with no names lists every standard model buffer, prints only
the `BUFFER: CHUNK ` headers (not the chunk slot lines), returns `(buffer
chunk)` pairs with the chunk omitted when empty, and excludes the extended
motor hand buffers. `PRODUCTION` is now present as an empty standard buffer.
This promoted `tutorial-unit1-count-first-retrieval-chunk`,
`tutorial-unit1modeling-broken-addition-v8-buffer-chunk`, and
`tutorial_unit1code_semantic_buffers`, moving the corpus to **406 passed / 161
ignored**.

## 276. Queue-text and `add-dm` command drivers (verified)

`tutorial-unit5modeling-v2-show-queue` and `v4-p3-run-queue` only needed the
already-implemented `mp-show-queue` path: load `declarative-issues-v2`/`v4`,
run 10/30 seconds with the trace off, then print the queue. The v2 ref leaves a
retrieved `V2` queued at 15.312; v4 leaves a retrieval failure at 17647.763.

`apply-chunk-spec` keeps the symbol `*NUMBER*` literal in `add-dm` (the macro
quotes its arguments), while `add-dm-fct` receives the already-evaluated value
`3`. Both command drivers now match. The two BST `pp` dumps remain ignored:
their production text includes the chunk-type default `LINE T`, which the
production parser does not add to an `isa line` condition. This promoted four
cases, moving the corpus to **410 passed / 157 ignored**.

## 277. Ignored-case triage (verified)

The two dynamic-goal dumps no longer diverge: `simple-dynamic-initial-goal`
reads the goal after two events, and `simple-ppm-model-goal` reads it after the
same two-event warm-up. Their ignore reasons were stale. This moved the corpus
to **412 passed / 155 ignored**.

The same pass reclassified the remaining warning/command cases: `safe-queries`
needs request-without-query warnings, `bad-isa-warning` and the extend cases
need invalid/previously-undefined-slot warnings, and the report cases still
need `sdp`/`whynot-dm` output. The BST `pp` cases remain blocked only on
production default-slot rendering. The paired traces differ much earlier than
their hook labels suggest, so they stay out of the current batch.

## 278. Warning/style pass and runtime slot extension (verified)

The model loader now builds ACT-R's style-check tables after parsing: it
records chunk-type and already-created chunk slots, expands
`declare-buffer-usage` declarations, and reports `isa` conditions with no
tests plus GOAL/IMAGINAL condition slots that no production requests,
modifies, or initializes. Request-without-query warnings are delayed until
`reset` because they only appear when `:do-not-query` removes the implicit
state-free query; `:DO-NOT-QUERY`/`:DO-NOT-HARVEST` are now runtime `sgp`
parameters too.

Previously undefined slots warn at production parse time using the loaded
module/model slot universe. Runtime `add-dm` now emits the invalid-slot
extension warning and extends the engine's possible slots. This promoted
`safe-queries`, the three broken-addition warning refs, `onehit-declare-
buffer-usage-warning`, `initial-load`, `extend-warning`, and
`semantic-dynamic-extend-warning`, moving the corpus to **420 passed / 147
ignored**.

## 279. Production text shows chunk-type defaults and auto-`CMD` last (verified)

`render_production` now expands an LHS `isa TYPE` into the type's default
slots. Model-defined defaults come from `model.chunk_types`; the core module
types use `builtin_type_defaults` (`TEXT`/`OVAL`/`LINE`). This restores
`LINE T` in the BST `pp` refs and `TEXT T` in `whynot`.

Request specs keep the compiler-inserted module `CMD` at the front for
instantiation/composition, but `ChunkDesc::Spec` now records when that `CMD`
was auto-added. `pp`/`whynot` render the explicit slots first and the auto
`CMD` last, matching ACT-R's `process-slots-specs`; an explicit `cmd` keeps
its source order. This promoted the two BST production dumps and
`tutorial-unit3modeling-v4-whynot`, moving the corpus to **423 passed / 144
ignored**.

## 280. `past-tense-correct-chunks` uses runtime-created chunks (verified)

The upstream driver computes two `past-tense` chunks from the task's word
list and prints them with `pprint-chunks-fct`; it never uses two named model
chunks. The Rust driver now issues the equivalent `define-chunks` spec for
`HAVE`/`HAD`/`BLANK` and `USE`/`USE`/`ED`, then prints the auto-named
`PAST-TENSE0`/`PAST-TENSE1`. This promoted one case, moving the corpus to
**424 passed / 143 ignored**.

## 281. SDM grouping and printed buffer dumps (verified)

`sdm` now groups every DM chunk by its slot signature, not just chunks added
at runtime. A new signature moves its group to the front, so model `add-dm`
groups print newest-first like ACT-R's `dm-chunks` alist. This promoted
`tutorial-unit1code-semantic-sdm2` (using the new test-only `SdmNames` step,
which runs `no-output` over `sdm` and prints the returned names).

`PrintedBufferChunk` models `printed-buffer-chunk`: consecutive buffers print
without an internal blank line, with one blank line closing the returned
string. This promoted `tutorial-unit7-past-tense-buffer-starts`, moving the
corpus to **426 passed / 141 ignored**.

## 282. Base-level history under optimized learning (verified)

`:ol t` does not populate chunk reference lists; `set-base-levels` now stores
an empty list while keeping the reference count and creation time, matching
`adapt-references` (`dm-commands.lisp:649`). This restored the empty `(...)`
in the base-level trace and promoted `tutorial-unit5modeling-v3-run-p2` and
`tutorial-unit5modeling-v4-run-p2`.

The low-activation case's driver had an extra `:ACT T` that upgraded the
requested `:ACT LOW` back to medium detail; dropping it promoted
`tutorial-unit5modeling-v1-low-act-trace`. Corpus: **429 passed / 138
ignored**.

## 283. Partial-matching negative-slot trace order (verified)

Request specs keep their `=` and `-` slots in separate vectors, so the trace
replayed every positive comparison before the negative ones. ACT-R walks the
request slots in source order; replaying the negatives first matches the
`result`-then-`simple-value-chunk` requests. This promoted
`tutorial-unit5modeling-v1-medium-act-trace` and
`tutorial-unit5modeling-v1-high-act-trace`, moving the corpus to **431
passed / 136 ignored**.

## 284. Runtime production typed-slot warnings (verified)

Added a runtime `p` command used by test drivers: it rejoins the flat RPC
arguments into a production form, parses it, emits the chunk-spec definition
warning and production-level warning for slots invalid for the request's
`isa` type, then replaces any same-named production. This promoted
`tutorial-unit5modeling-bad-isa-warning`, moving the corpus to **432 passed
/ 135 ignored**.

## 285. Runtime `p` refreshes the executable production table

`define_production_text` now rebuilds `exec_prods` after replacing a
production. A runtime `p` can therefore supersede a model-body production for
the next conflict resolution instead of leaving a stale executable entry.
`tutorial-unit5-count-mod-request-goal-event` keeps the redefining `p` driver
but remains ignored: matching its `module-mod-request` projection conflicts
with `indirect-action-compilation`'s current ref.

## 286. Core-module chunk-type refs vendored

Vendored `core-modules/vision.lisp` and `core-modules/audio.lisp` under
`fixtures/models` so the four core-module source-echo cases can run without
an ACT-R checkout. Promoted `tutorial-unit3-visual-chunk-type`,
`tutorial-unit3-sound-chunk-type`,
`tutorial-unit3-visual-location-chunk-type`, and
`tutorial-unit6-visual-chunk-type-hierarchy-examples`, moving the corpus to
**436 passed / 131 ignored**.

## 287. Reset warnings honour the model body's `:v`

Reset warnings were emitted through an ungated `emit_warning`, so every reset
of a `(sgp :v nil)` model flooded the trace (the Siegler corpus produced 1202
warning lines instead of its 9 result lines). `emit_warning` now respects the
active trace flag, and reset temporarily evaluates warnings under the model
body's `:v` before restoring the driver's trace state. This keeps engine-path
cases such as `associative-learning` on their existing trace contract while
suppressing model load/reset warnings for silent models.

## 288. Copied chunk values compare by source; Siegler digits

Chunk-valued slots held a buffer copy name (`ONE-0`) while the DM chunk used
the source name (`ONE`). Exact retrieval used raw `Value` equality and partial
matching's similarity used it too, so copied values never matched their source
and explicit `set-similarities` entries were missed. `value_eq` now follows
`copy_source` as well as `chunk_alias`, and retrieval/similarity use it
consistently. This is what lets the Siegler model retrieve its `F` facts from
copied imaginal addends and apply the number similarities.

Added a scripted `SieglerDriver` (digit sounds, per-trial seeds, response
matrix, correlation/RMSD) and flattened nested matrices in the
`correlation`/`mean-deviation` commands. Promoted
`tutorial-unit5code-siegler-experiment-esc-nil`, moving the corpus to **437
passed / 130 ignored**. The other Siegler experiment refs remain ignored: the
response distributions are close but not yet exact (e.g. 1+1 expects
`.07/.80/.12`, the driver currently produces `.15/.77/.06`).

## 289. BST `spp` after one use

Recovered `tutorial-unit6-bst-spp-after-1-use` by replaying the first two
`bst-nolearn` device runs (`(15 200 41 103)` and `(10 200 29 132)`) with the
same `(1 1)` seed per reset, then issuing the production-parameter dump. The
case now checks the learned utilities after one `bst-test` pass. Corpus:
**438 passed / 129 ignored**.

## 290. Dynamic-retrieve breakpoints and instantiation printing

The three `simple-dynamic-retrieve` cases were missing the task's `pbreak`.
Added a quiet `PBreak` step for drivers that wrap the run in `no-output`, and
made `pbreak` print dynamic LHS slot names through their bindings
(`=slot` -> `DESTINATION`). Instantiation output now omits the retrieval
request's compiler-inserted `CMD` while keeping explicit/module `CMD` lines.
Promoted `tutorial-unit8-simple-dynamic-retrieve-first-step-buffers`,
`...second-step-buffer`, and `...first-step-instantiate`: corpus
**441 passed / 126 ignored**.

## 291. Dynamic retrieve second-step buffer dump

Recovered the remaining `simple-dynamic-retrieve-second-step-buffers` driver
with the same quiet `pbreak retrieve-second-step` boundary. Corpus:
**442 passed / 125 ignored**.

## 292. BST imaginal chunk at the strategy decision

Recovered `tutorial-unit8-bst-imaginal-chunk` by replaying the same
`(15 250 55 125)` device window and breaking quietly at
`DECIDE-OVER`/`DECIDE-UNDER`, then dumping the imaginal buffer before the
chosen production's RHS runs. Corpus: **443 passed / 124 ignored**.

## 293. Fan-task experiment summary

Added `FanCollect`/`FanSummary` steps that record each `(run 30)`'s elapsed
time and goal `state`, then emit `fan-experiment`'s averaged TARGETS/FOILS
table. `fan-no-pm` now runs all 36 experiment trials and prints the original
18-condition correlation/mean deviation without replaying the task in Lisp.
Corpus: **444 passed / 123 ignored**.

## 294. Paired-associate experiment summary

Added paired-run collection around a recovered spec: each prompt run records
the `output-key` time and the probe still in the imaginal buffer, and
`PairedSummary` aggregates the 8 blocks × 20 trials and emits `Latency:` /
`Accuracy:` tables. This recovers `tutorial-unit4-paired-experiment-only-bll`
(corpus **445 passed / 122 ignored**). The main, `no-bll`, and `with-rt`
variants remain ignored: their summaries match to within a few milliseconds
but the per-subject key timings are not yet exact.

## 295. Triage of remaining arbitrary-Lisp stubs

Retagged the 19 `arbitrary-Lisp command stub` cases and the three
command-layer one-hit duplicates by dependency. Thirteen are recoverable from
available sources: four base Zbrodoff cases, three one-hit cases (plus the
three command-layer duplicates), both BST experiment summaries, the Siegler
original-data formatter, the thorough production driver, the syllable-count
speech extension, and the v9 activation trace. Six remain blocked because they
load solution files from `examples/act-r-solutions.zip`, which is password
protected: three Zbrodoff solution variants, the choice results, and both
categorize experiments. The password is not in the repo or history (only the
`7z a -p"XXX"` build placeholder exists), so those need an upstream copy.

## 296. Siegler original-data formatter

Recovered `tutorial-unit5-siegler-original-data` with a model-free driver: the
case only echoes `display-results` over the fixed `*siegler-data*` table, so
the Rust test formats the same matrix without constructing an engine. Corpus:
**446 passed / 121 ignored**.

## 297. Zbrodoff experiment driver

Added `Step::Zbrodoff` with compare, experiment, set-and-block, and
starting-results modes. The driver replays `zbrodoff.lisp`'s randomized sets in
a live window, runs event-by-event until each `output-key`, records correct RTs
by block/addend, and emits the original correlation/mean-deviation and table
format. Recovered `tutorial-unit4-zbrodoff-compare`, `...-experiment`,
`...-set-and-block`, and `...-starting-results`.

This also fixed four engine details the model exposes:
`current` values in visual-location requests, negated request slots, negative
LHS variables bound by a later condition, and finst expiry across rebuilt
displays. Speech preparation and execution are now separate: preparation can
overlap a prior utterance, while the next execution waits for it to finish;
`?vocal> preparation` no longer waits for articulation, and overlapping
`VocalFree` events cannot clear a newer utterance. Corpus:
**450 passed / 117 ignored**.

## 298. One-hit blackjack task driver

Added `Step::Onehit` for the tutorial one-hit blackjack task. The driver runs
the regular-deck/fixed-threshold game against the model, records `output-key`
responses, scores the four outcome counters, and formats the Lisp result lists.
It recovers `tutorial-unit5-onehit-lisp-blocks`, `...-hands`, and
`...-learning`; the latter runs 50 games of 20×5 hands and reproduces the
single-float percentages. The evaluator also gained `max`/`min`, needed by the
task's `1hit-bj-number-sims` similarity hook. Corpus:
**453 passed / 114 ignored**.

## 299. Python one-hit command refs

Added the Python-tutorial entry points to `Step::Onehit`: `blocks` and
`hands` use their test-model starting seeds (`(3 0)` and `(1 0)`), while the
Python learning wrapper reseeds from 101 for its 50 games. They retain the
Python list/`print` formatting, including per-hand card lines and Python float
reprs, recovering the three command-layer one-hit cases. Corpus:
**456 passed / 111 ignored**.

## 300. Unit 6 building-sticks experiment

Added `Step::Bst` with a live building-sticks window: buttons and stick lines,
the model's mouse clicks and reset actions, the current-line redraw, and the
`done` prompt. The unit 6 driver runs the 15 experimental stimuli for 100
reseeded runs, accumulates over-strategy choices and production utilities, and
emits the original correlation/table/utility format. Recovered
`tutorial-unit6-bst-experiment-data` (corpus **457 passed / 110 ignored**).

The unit 8 PPM variant remains ignored: the same driver always selects the
over strategy there, so its `bst-compute-difference` imaginal action and PPM
similarities still need investigation.

## 301. Syllable-count extension test

Recovered `test-syllable-count` with a model-free translation of the
extension's lookup rules: explicit articulation times, table counts, the
char-per-syllable fallback, and split-vocal-string spacing. Corpus:
**458 passed / 109 ignored**.

## 302. Triage of the remaining generic ignore labels

The 46 cases whose reason was only `pending: does not yet match ref` now carry
specific reasons and a `BLOCKED ON:` comment in their source. No test behavior
changed. The 46 fall into these buckets:

| Bucket | Count | Representative missing piece |
|---|---:|---|
| Missing activation-trace command | 3 | `print-chunk-activation-trace` / `print-activation-trace` |
| Missing `print-conflict-tree` | 2 | conflict-tree printer |
| Missing chunk `sdp` | 2 | `sdp v1 v2 v3` form; only `:keyword` getters exist |
| Missing `whynot-dm` | 1 | declarative candidate report |
| Missing `simulate-retrieval-request` | 1 | simulated activation comparison |
| Incomplete experiment driver | 8 | fan, BST learning, Siegler, Zbrodoff, four pcomp averages |
| BST PPM / imaginal action | 2 | PPM trial summary plus copy/length gaps (#266-267) |
| DM merge/naming | 2 | `G1`/`CHUNK0` merge references and `sdm` grouping (#206) |
| Visicon / imaginal semantics | 6 | image window, nearest LAD, Sperling, stuffing, action buffer |
| Motor trace | 2 | hand move and keypad event timing |
| Audio sound-object replay | 1 | `new-tone-sound` object plus onset events (#204) |
| Adaptive-noise/PPM hooks | 2 | `:BL-HOOK`/`:NOISE-HOOK`, `:PPM-HOOK` |
| Multi-model trace columns | 1 | model-name column and interleaved meta-process (#66) |
| Production compilation | 2 | dynamic compiled-production trace/result (#154) |
| Numerical summary mismatch | 10 | six Siegler experiments, Sperling experiment, three paired summaries |
| Base-level activation trace | 1 | repeated seeded `:ol` histories |

The missing debug printers are the clearest quick wins: they account for nine
cases and are independent of model timing. Run
`python rust_actr7/tools/list-ignored.py --counts` to see the new reason
distribution; the generic reason count is zero.

## 303. Declarative introspection: sdp and whynot-dm

Added the chunk-name form of `sdp` and the `whynot-dm` command. Retrieval now
retains a `LastRequest` snapshot (time, printed request slots, matches,
non-matches, best set, activations, threshold), and each candidate stores its
`last_retrieval` activation/time as ACT-R does. `sdp <chunk...>` recomputes the
`isa chunk` activation (base + spreading + one noise draw, no partial matching)
and prints the full block: activation, permanent noise, base level, creation
time/reference history, source spread, Sjis, similarities, and last-retrieval
fields. `whynot-dm` reuses that block and explains each named chunk's match,
best-set, and threshold outcome.

Recovered `tutorial-unit5modeling-v1-sdp`, `...-v10-sdp`, and
`...-v1-whynot-dm`. Corpus: **461 passed / 106 ignored**.

## 304. Simulated retrieval and saved activation traces

Added `simulate-retrieval-request` for request specs: it scores the same
candidate set, draws activation noise in request order, restores the RNG, and
prints the Lisp "current best"/"has activation"/"is the best" lines.

Added `:SACT` state and a per-request-time trace table. During retrieval the
activation narration is captured without leaking into the model trace; the
capture suppresses the live-`:act`-only negation note. `print-activation-trace`
replays matches, no-matches, saved details, and the retrieval result;
`print-chunk-activation-trace` replays one chunk's details or explains that it
did not match/was not considered.

Recovered `tutorial-unit5modeling-v1-simulated-retrieval`,
`...-v5-p3-run-f3-activation`, `...-v8-p6-compare-traces`, and `fan-saved-act`
(whose two runs need `:SACT t` re-applied after reset). Corpus:
**465 passed / 102 ignored**.

## 305. Conflict-tree printer

Added `print-conflict-tree`. It groups productions by their constant LHS
tests, recursively splits on the first discriminating test, and prints the
same root/test-slot/leaf format as `commands/conflict-tree.lisp`
(`---->[root ...]`, `-N- T/NIL ->[...]`). Recovered `count-w-tree`. The
printer is intentionally simpler than the upstream information-gain builder;
it is sufficient for COUNT, while `fan-w-tree` remains blocked on its fan
experiment driver/summary. Corpus: **466 passed / 101 ignored**.

## 306. Activation-trace driver and full module slot index

Added `Step::SdmActivationTraces`: it runs the `sdm` spec without printing the
chunk dump, then replays each returned chunk's saved `:sact` detail at the
request time, collapsing unmatched detail lines the way the v9 task does.
Recovered `tutorial-unit5modeling-v9-spreading-activation-only`.

`Model::slot_index` now pre-seeds the complete standard-module slot order
(audio, motor, speech, visual, retrieval, and utility slots) before the model
body's chunk types. The semantic model's `property` slots therefore print
`OBJECT VALUE ATTRIBUTE`, matching ACT-R's shared slot index rather than the
type declaration order. Recovered `tutorial-unit1code-semantic-sdm`.

Also added task-command registration (`add-act-r-command`) plus the
`!eval!` compiler-invalidity check. `v8-eval-issue` now reaches the ref's
four-line invalid-compilation trace, but remains ignored on a pre-existing
pcomp trial-timing divergence (37.244 vs 36.267). Corpus: **468 passed /
99 ignored**.

## 307. Full fan experiment summary

`Step::FanExperimentSummary` now emits the perceptual/motor `fan-experiment`
table without the blank line that `fan-no-pm` prints. Fan collection uses the
manual `output-key` timestamp (the task returns the key-press time, not the
end-of-run time), and `break-fan-run` installs the ENCODE-PERSON breakpoint
with the testing traces. Recovered `fan`. Conflict-tree tie handling now
matches the first ARG2 split, but `fan-w-tree`'s deeper incremental branches
still differ, so it remains ignored.

Added a one-run `BstMode::Learn` path for `bst-learn`, but its 15-trial live
window still diverges on visual-location numbering/visicon updates; it remains
ignored. Corpus: **469 passed / 98 ignored**.

## 308. Paired timing diagnosis and alias printing

The paired experiment summaries are structurally correct but every block's
latency is 4-5 ms high. A uniform -4 ms adjustment leaves the second and
eighth blocks one millisecond off, so the divergence is in the per-key motor /
device timestamp, not the summary formatter; no offset hack was applied.

Added alias-aware `pprint-chunks`: when a requested name resolves through a
merge/copy alias, the canonical chunk is printed with the requested name and
its source in parentheses (`CHUNK0 (G1)`). Recovered
`tutorial-unit5modeling-v9-p7-8-run-g1-chunks`. The `:ncnar nil` companion
remains ignored because applying the runtime flag changes the v10 refs'
activation/naming, which conflict.

Corpus: **470 passed / 97 ignored**.

## 309. Pcomp utility divergence is cached utility noise, not `:u`

The pcomp recreated-compiled-production refs print the correct base `:u`
(e.g. 1.999 or 2.857), but the adjacent cached `:utility` differs
(1.315 vs 1.901, 2.376 vs -2.139). The update formula itself is correct; the
difference is the conflict-resolution utility-noise draw/cache for the
recreated production. The pcomp selection/timing cluster needs an RNG/draw
order audit before its utility refs can be promoted.

## 310. Fan experiment output driver

Added the 36-run device sidecar for `tutorial-unit5-fan-output-data` (the
perceptual/motor fan trials without the earlier probe sections) and a driver
using `FanCollect`/`FanExperimentSummary`. Recovered the case.

The BST imaginal-action trace driver is now implemented with the break at
`ENCODE-LINE-CURRENT` and a 15-event projection, but the engine emits
`CLEAR-BUFFER IMAGINAL-ACTION` before `MODULE-REQUEST IMAGINAL-ACTION`; it
remains ignored on that event-order divergence.

Corpus: **471 passed / 96 ignored**.

## 311. Event-hook effects: sgp, model output, and buffer dumps

`EventHook` now has an opt-in effect queue (`take_effects`) without changing the
existing `pre`/`post` signatures. The engine drains it after `pre` and after all
`post` lines, so multi-line events do not duplicate effects:

- `Sgp`: apply `:seed`, `:act`, `:v`, `:alpha`, `:pct`, ... at the event.
- `ModelOutput`: write a raw Lisp `model-output` line.
- `PrintBufferChunk`: append a `printed-buffer-chunk` dump.
- `CaptureBufferChunk`: capture a dump into a task-owned sink.

Pre effects run before event execution (and still run when the hook returns
`break`); post effects run after the event's lines. The three run loops now
share the same pre/post helpers.

This recovered `tutorial-unit2-demo2-visual-chunk` and
`tutorial-unit2-demo2-visual-location-chunk`. The paired drivers were ported to
the new effects, but they remain blocked: the unit-4 trace differs only in the
activation-noise draw, and the unit-7 traces need production-compilation state
that the engine does not yet reproduce at those timestamps. The hook-phase
harness (`swap_hook_at`) and `Step::ResetSilent` are in place for that follow-up.

Corpus: **473 passed / 94 ignored**.

## 312. Hook commands, buffer predicates, and retrieval-history traces

Hooks can now queue a whitelisted `HookEffect::Command` (`record-history`,
`stop-recording-history`, `mod-buffer-chunk`). `EventView` also carries the
non-empty buffer names at the event's pre-execution state, so filters can
implement `(buffer-read <name>)` predicates.

`record-history "retrieval-history"` now enables `:sact` and copies the saved
per-chunk activation narration into `RetrievalChunk.activation_trace`, matching
`retrieval-history.lisp`'s `printed-chunk-activation-trace` field. Added
`Step::RetrievalHistoryTrace` for grouped's `get-history-data` + `find`/`nth`
output shape.

Recovered:

- `tutorial-unit5-grouped-commission-act-trace`
- `tutorial-unit5-grouped-omission-act-trace`
- `tutorial-unit8-categorize-goal-chunk-part2`

Corpus: **476 passed / 91 ignored**.

## 313. Module modification requests and hidden hook lines

`*buffer>` now schedules a real `MODULE-MOD-REQUEST` event before the
module's delayed `MOD-BUFFER-CHUNK`, and `implicit_queries` adds ACT-R's
state-free test for both `+buffer>` and `*buffer>` RHS actions. This recovers
the count and imaginal mod-request hook cases.

Hook lines now distinguish rendering from trace visibility: hidden
`MODULE-REQUEST`/`MOD-BUFFER-CHUNK` events are still captured for filters that
call `format-event` directly, while filters using `filter-test` must check
`EventView::shown` (the Sperling projection does). Simple imaginal actions
schedule `set-buffer-chunk` directly plus the maintenance
`set-imaginal-free`, matching ACT-R's event count instead of hiding the set
behind an extra wrapper event.

Added `Step::ZbrodoffProblem` for the single-trial `zbrodoff-problem` driver.

Recovered:

- `tutorial-unit5-count-mod-request-goal-event`
- `tutorial-unit5-imaginal-mod-request-event`
- `tutorial-unit8code-bst-imaginal-actions-trace`
- `tutorial-unit4-zbrodoff-correct-chunk`

Corpus: **480 passed / 87 ignored**.

## 314. Pcomp task RNG at trial boundaries, safe-eval, and compiled actions

The pcomp task draws two `act-r-random 4` values at the start of every trial.
The scripted device replay only consumed the first pair, so from the second
trial on the model RNG diverged while matching outputs still looked plausible;
the first visible symptom was the utility-noise sample cached for the recreated
production. Device scripts now support `(RNG-DRAW ms count limit)`, consumed in
`proc_display` before the `add-text` ops at that instant, and the pcomp
sidecars carry the per-trial draws.

Pcomp compilation also had three LHS/RHS gaps: `!safe-eval!` is now a valid
compiled prefix condition (with second-parent variables substituted, first-
parent variables retained), slot modifiers invalidate compilation in ACT-R's
order, and composed request actions expand chunk-type defaults and keep
negative request slots. Recreated compiled productions update their utility
even while `:v`/`:pct` output is suppressed; only the printed parameter block
is gated. This was the remaining `:u 5.281` vs `2.004` difference.

Recovered:

- `tutorial-unit7modeling-pcomp-issues-v4-pct-successful-retrieval`
- `tutorial-unit7modeling-pcomp-issues-v4-two-trial-pct-2`
- `tutorial-unit7modeling-pcomp-issues-v5-production3-recreated`
- `tutorial-unit7modeling-pcomp-issues-v5-recreate-encode`
- `tutorial-unit7modeling-pcomp-issues-v5-retrieval-time`
- `tutorial-unit7modeling-pcomp-issues-v5-retrieved-a-win`
- `tutorial-unit7modeling-pcomp-issues-v6-retrieved-a-win`
- `tutorial-unit7modeling-pcomp-issues-v8-eval-issue`
- `tutorial-unit7modeling-pcomp-issues-v9-non-win-composition`
- `tutorial-unit7modeling-pcomp-issues-v10-guess-other-composition`
- `tutorial-unit7modeling-pcomp-issues-v10-non-win-composition`

Corpus: **491 passed / 76 ignored**.

## 315. Pcomp pp drivers and compiled-production documentation

Recovered the simple pcomp `pp`/`pprint-chunks` drivers by replaying their
trial sidecars with `ResetSilent` plus `ConsumeRng(2)`, then enabling `:v`
only for the final command. Recovered `v10-attend-num-1/2-productions`,
`v10-production21`, `v10-production29`, `v10-one-game-late-productions`, and
`v4-response-chunks`.

Compiled productions now retain their `(P name "doc" ...)` documentation
string, so `pp` prints the composition description. `pprint-chunks` shows a
copy's source (`RESPONSE-2-0 (RESPONSE-2)`), while ordinary `buffer-chunk`
dumps keep their plain names.

Corpus: **497 passed / 70 ignored**.

## 317. Pcomp game driver, seed search, and model-body WHEN

Added `Step::PcompGame` with fixed-seed, model-seed, and continue-stream
modes, plus the pcomp trial loop (two RNG draws per trial, feedback, space,
block-wise score/RT summaries). Recovered `v10-one-game-result`,
`initial-run`, `v3-initial-run`, `v5-initial-run`, and `v7-find-seed`;
`trial-chunk` now uses a one-trial sidecar with feedback.

`Mt` tracks `(seed, draws)` for `:SEED` trace lines, and `reset_keep_rng`
preserves the stream across games. Registered task commands are recorded so
`v7`'s `set-flag` stops the search after the matching game.

The model loader now accepts model-body `(when ...)` seeding blocks and
one-key `(sgp :seed)` getters. Vendored the v11-v13 pcomp models.

Corpus: **503 passed / 64 ignored**. Remaining pcomp cases are engine
divergences: v1 EPL summary, v10 unused compiled-production set, and the
v11-v13 multi-game summaries.

## 318. Compiled-production duplicate canonicalization

ACT-R compares compiled productions with `production-comparison-rep`, which
renames LHS slot variables by buffer/slot (`=GOAL-&-STATE`) and folds request
relation/exclusion tests into the comparison. The engine compared raw variable
names and ignored `neg`/`rels`, so equivalent compositions were installed
repeatedly: v10's unused-utility case saw 257 productions instead of ACT-R's
111.

Canonicalization now renames variables by location and includes negative and
relative request slots. `v10-one-game-result-unused` passes; corpus **504
passed / 63 ignored**.

The remaining v11 divergence is isolated: game 6, trial 10, `guess-other`
retrieves `RESPONSE-1` instead of `RESPONSE-3`; the engine consumes one fewer
RNG draw by the response. Traces agree through the request.

## 319. Retrieval-compilation negative request variables

`MAP-RETRIEVAL-BUFFER` only instantiates a variable request-slot *value* for
an `=` request slot. The engine also instantiated negative slots, so the
composed `GUESS-OTHER & RESPOND` production turned `- key =key` into
`- key "f"`. That narrowed the compiled production's LHS, removed it from the
conflict set on later trials, and shifted one activation-noise draw.

Negative/relative request slots now keep their variables. Second-parent
action variables that came from the first parent's request are instantiated
from the second parent's firing, matching ACT-R's composed `RESPONSE "s"` /
`KEY "s"` form while the LHS keeps `KEY =KEY`.

`v11-test-runs` passes. Corpus **505 passed / 62 ignored**. v12/v13 remain as
50-game timing divergences; v12 matches through game 11 and first differs by
0.01 s in game 12's RT block.

## 320. v12/v13 residual: activation-history latency

After the v11 compile fix, v12 matches through game 39. The first real
divergence is game 40, trial 10: `retrieve-past-trial` returns the same
`IMAGINAL-CHUNK0-6` in both, but Rust latches it at 80.419 s and ACT-R at
81.913 s (1.494 s slower). That is a DM activation/history/latency gap, not a
production-compilation or task-side match.

Corpus: **505 passed / 62 ignored**.

## 321. v1/v13 recheck and Wave 2 reconnaissance

Re-ran the parked pcomp cases after the compilation fixes. Both still fail:

- `v1-initial-run`: `1.00 2.32 2.10 2.38 ...` vs expected `0.86 1.68 2.98 ...`.
- `v13-50-runs`: `2.56 5.44 6.38 ...` vs expected `3.04 4.38 5.58 ...`.

Wave 2 reconnaissance found earlier blockers than the known 4-5 ms summary
offset:

- `hand-move-test` fails at model load: `offsets ((index -1 0))` is a nested
  list where the model parser only accepts atomic slot patterns.
- `keypad-test` fails with `manual request (Some("HAND-TO-KEYPAD"), None)`.
- The three unit-4 paired summaries still differ only by the known per-key
  timing (about 4-5 ms high).
- `tutorial-unit7-paired-learning-alpha-1` now uses its real filter hook. The
  first divergence is at 25.435: ACT-R fires `PRODUCTION7`, Rust fires
  `PRODUCTION8`; both productions exist and the timing differs by 1 ms. This
  is conflict-resolution utility/noise/candidate ordering, not missing
  production compilation.

Corpus: **505 passed / 62 ignored**.

The paired activation-trace filter now emits the correct 18-line shape; the
only remaining difference is the second retrieval's transient noise
(`-0.7636829` expected vs `0.12914158` actual). Consume-count scans did not
match, so this is an RNG stream/noise-draw-order issue rather than a projection
bug.

## 322. Paired activation trace: task RNG double-count

Recovered `tutorial-unit4-paired-act-trace`. The projection itself was already
right; the test driver was consuming the wrong random stream. ACT-R's
`permute-list` draws once per item, and `(permute-list (subseq *pairs* 19))`
receives one pair, so it draws once. The Rust command arguments were
`(("zinc" "9"))`, which the command layer flattened to `("zinc" "9")` and
permuted as two items, drawing twice. The driver also had two extra
`Step::ConsumeRng(1)` steps for those same setup draws.

Using `((("zinc" "9")))` (one pair in the list) and removing the explicit
consumes makes the seed state match ACT-R at both retrievals:
`(200 6)`, then `(200 9)`, with the second retrieval's noise at `(200 10)`
and the expected value `-0.7636829`.

Corpus: **506 passed / 61 ignored**.

## 323. Paired learning alpha recovery and final-case blocker

Recovered `tutorial-unit7-paired-learning-alpha-1` with the same task-RNG
correction used for the activation trace: each `permute-list` takes one pair
(one draw), not a two-string list, and the explicit `ConsumeRng(1)` steps were
double-counting those draws. The filter hook then breaks at 25.435 on
`PRODUCTION7` and the 33-line projection matches.

`tutorial-unit7-paired-learning-alpha-1-final` now uses its real hook too, but
with 27 pairs it fails earlier: installing a later composed production returns
`bad model: unbound variable =VAL on RHS`. That is a new compiled-production
validity/binding issue to isolate separately.

Corpus: **507 passed / 60 ignored**.

## 324. Paired learning final: retrieval condition over-mapping

`tutorial-unit7-paired-learning-alpha-1-final` is blocked in the composition
of `PRODUCTION48 & PRODUCTION16` (`PRODUCTION100`). ACT-R keeps p2's retrieval
condition and RHS variable symbolic:

```
=RETRIEVAL> ARG2 =VAL
+MANUAL> KEY =VAL
```

Rust's `MAP-RETRIEVAL-BUFFER` instantiates the p2 condition variable from the
retrieved chunk (`ARG2 "9"`) but leaves the RHS as `=VAL`, so firing fails with
`bad model: unbound variable =VAL on RHS`.

A broad fix ("instantiate every p2 RHS variable that is absent from the
composed LHS from p2's firing") removed the crash but regressed 21 cases,
including `paired-learning` (`pre =state` became `pre wait`) and several pcomp
compositions. It was reverted. The fix needs to be confined to the retrieval
map: match ACT-R's decision to leave this p2 condition variable symbolic
instead of globally instantiating unbound RHS variables.

A narrower attempt (skip retrieval-condition mapping when the same variable
appears in p2's RHS) avoided the `=VAL` crash but produced
`unbound variable =STATE1 on RHS` in another composition, so it was also
reverted. The remaining work is to reproduce ACT-R's `the-chunk`/p2 binding
state at this composition, not to change RHS substitution globally.

## 325. Paired learning final recovered; goal map arm confined

Recovered `tutorial-unit7-paired-learning-alpha-1-final`. The failing
composition was `PRODUCTION48 & PRODUCTION16`: the generic same-slot mapping
loop ran for every buffer, and its `p1 variable / p2 constant` arm
instantiated the retrieval condition variable `VAL` from p1's firing
(`ARG2 "9"`) while the p2 RHS stayed `KEY =VAL`.

ACT-R's `MAP-GOAL-BUFFER` variable-to-constant arm is goal-only. Confining
that arm to `GOAL` leaves the retrieval condition symbolic, matching ACT-R's
`PRODUCTION100`:

```
=RETRIEVAL> ARG2 =VAL
+MANUAL> KEY =VAL
```

The narrower change preserves v11; a fully goal-only loop had broken v11's
compiled-production mappings. Corpus: **508 passed / 59 ignored**.

## 326. Paired compiling filter cases recovered

Recovered three filtered paired cases by replacing their generated drivers with
the real hook lifecycle:

- `tutorial-unit7-paired-learning-results-of-compiling` (break at 400 ms)
- `tutorial-unit8-paired-dynamic-compiling-to-dynamic` (break at 400 ms)
- `tutorial-unit8-paired-dynamic-compiling-to-static` (break at 600 ms)

Each uses the same `filter`/`filter2` pattern as the alpha cases: suppress the
trace until the target production fires, break there, then resume with
`run-n-events 2`. Their `permute-list` arguments also needed the one-pair list
form; the generated two-string form over-consumed RNG draws.

The remaining paired stub, `tutorial-unit7-paired-composed-production`, needs
`pp PRODUCTION0` output after a suppressed paired-task run; the current test
driver has not yet captured that command output.

Corpus: **511 passed / 56 ignored**.

## 327. Paired summaries: motor feature tolerances and probe-run collection

Recovered the five remaining paired summary cases:

- `tutorial-unit4-paired-experiment`
- `tutorial-unit4-paired-experiment-no-bll`
- `tutorial-unit4-paired-experiment-with-rt`
- `tutorial-unit7-paired-learning-results-no-compiling`
- `tutorial-unit7-paired-learning-results-with-compiling`

The supposed uniform 4-5 ms offset was six individual trials, not a global
timestamp shift. Comparing a trained oracle subject against Rust trial by trial
showed 6/160 keys exactly 100 ms slow; all were adjacent same-finger key
transitions (`6->7`, `4->5`, `5->4`). ACT-R's `feat-differences` uses
`distance=`/`direction=` tolerances for peck features (2 degrees and 45
degrees, `motor.lisp:890`), while Rust compared `r`/`theta` for exact equality
and therefore prepared two extra features. Applying the upstream tolerances
recovers all six trials and all five summaries without an offset hack.

The unit-7/8 paired collector also needed two driver fixes: read the probe from
`IMAGINAL.ARG1`/`WORD` as well as unit-4's `PROBE`, and keep only the probe half
of each 10 s probe/feedback cycle (the feedback display leaves the probe slot
populated). Their generated tests were calling `check()` without
`.paired_summary()`, so they had been comparing an empty trace.

`tutorial-unit8-paired-dynamic-results-with-compiling` now has the correct
summary shape, but its per-trial retrieval-success pattern still diverges from
the oracle (first subject differs from trial 20 onward), so it remains ignored
on the dynamic-model activation/retrieval issue rather than timing.

Corpus: **516 passed / 51 ignored**.

## 328. Unit-8 dynamic summary: instantiated dynamic slot names in composition

Recovered `tutorial-unit8-paired-dynamic-results-with-compiling`. The first
divergence was not activation noise: Rust was missing the compiled candidate
`PRODUCTION84` at 150.385, so it drew one fewer utility noise per trial and
shifted the activation stream before trial 20. `PRODUCTION84` is
`PRODUCTION1 & RETRIEVE-ASSOCIATE`, whose oracle LHS keeps an empty
`=imaginal>` condition while Rust kept `WORD =STIMULUS`, so the candidate did
not match.

The parent's dynamic modify-request slot is `*imaginal> =which-slot =val`.
Resolving that slot name against p1's firing (`WORD`) lets the composed LHS see
that p1 modifies `WORD` and drop p2's matching imaginal condition. The imaginal
mapping also needs p2's dynamic LHS slot name (`=target`) resolved from p2's
firing before comparing slots, which supplies `STIMULUS -> VAL` and avoids an
unbound RHS variable.

Kept the fix narrow:

- `instantiate_slot_names` now only resolves slot names beginning with `=`
  (`chunk-spec-variable-p`), preserving `!safe-bind!` and query slot names.
- The composed LHS compares p2 slot names under p2's bindings when deciding
  whether p1's modifications supersede them.
- The imaginal mapping instantiates p2 slot names locally; p2's LHS
  representation itself is unchanged, preserving self-composition and other
  dynamic-compilation traces.

Verified `dynamic-production-compilation-tests`, `retain-indirect-retrievals`,
and `paired-learning-dynamic` after the change.

Corpus: **517 passed / 50 ignored**.

## 329. Composed-production hook case and query ordering

Recovered `tutorial-unit7-paired-composed-production` with a real driver and
device sidecar. The task is `paired-task 1 2`: two 10 s probe/feedback cycles
for the `zinc`/`9` pair, with the filter setting `:v nil`/`:epl t` on the first
event, then `pp production0`. The sidecar supplies the four prompts at
0/5000/10000/15000 ms; two `permute-list` command steps consume the task's
one-pair draws.

The remaining mismatch was the composed production's LHS query order. Rust
emitted `?imaginal` (p1's explicit query) before p2's `?visual`/`?manual`;
ACT-R orders query conditions by `buffer-index` (`visual` < `manual` <
`imaginal`). Match ordering stays in its existing order, so pcomp LHS traces
are unchanged.

Corpus: **518 passed / 49 ignored**.

## 330. Hand-move test: nested offsets and base motor hand styles

Recovered `hand-move-test`. Added `Pattern::List` so a manual request slot can
carry `offsets ((index -1 0))` (and the nested list converts through
`Value::List` for matching/printing).

The base manual module now routes `hand-to-mouse`, `hand-to-home`,
`point-hand-at-key`, and `isa clear` through the existing hand-ply machinery.
`hand-ply` prep uses ACT-R's feature-difference logic for base models (4
features initially, then distance/direction differences); the extended
actions path keeps its previous 4-feature behavior. Base `peck`/`punch` emit
only the press (no automatic `release-key`), and `point-hand-at-key` applies
`standard`/explicit finger offsets. Manual clear now holds the module busy
until its scheduled change-state and resets the last prepared movement.

The test's task-side `output-key` handler is represented by a post-event hook
emitting `The key is "…"`; the hook is reinstalled after the driver's
`chunk-type` command, which rebuilds the engine.

Corpus: **519 passed / 48 ignored**.

## 331. Keypad test: keypad geometry and canonical key names

Recovered `keypad-test`. Reset now honors `(start-hand-at-keypad)` by placing
the right hand at keypad `(19,4)` with the keypad finger offsets, and
`hand-to-keypad` builds a hand-ply with target width `4.0`
(`keyboard.lisp:637`), matching the oracle's Fitts execution time.

Added keypad key geometry, finger specs, `key_xy`/`xy_key` entries, and
alias canonicalization (`keypad-minus` -> `keypad--`, `keypad-plus` ->
`keypad-+`, `keypad-period` -> `keypad-.`). `*goal>` module modifications
now land at priority 40, after a same-time manual request, matching the
keypad production's source order.

The task-side `output-key` handler is represented by the same post-event hook
pattern as `hand-move-test`.

Corpus: **520 passed / 47 ignored**.

## 332. Siegler response distributions: first-trial seed and direct drivers

Recovered all six Siegler response-distribution cases. The original variant
started the task at `*starting-seed*` 110, which `incf`s to 111 before the
first trial; the Rust driver was seeded at 110, shifting the whole response
matrix (1+1 was `.15/.77/.06` instead of `.07/.80/.12`).

The five `tutorial-unit5code` variants had been running their recovered specs,
which replay the runs and then feed the already-observed response matrix into
`CORRELATION`; they never exercised the model's actual responses. They now
use direct `SieglerDriver` steps with the source seeds and parameters:

- fact-base: seed 302, `:rt -.7`, fact base .5
- good-mp: seed 102, `:rt -10`, fact base 0
- mp-and-ans: seed 102, `:rt 0`, `:mp 5`, `:ans .5`, all base 0
- new-rt: seed 302, `:rt -.7`, fact base 0
- rt-and-base: seed 102, `:rt -10`, `:mp 5`, `:ans .5`, fact base 0

Corpus: **526 passed / 41 ignored**.

## 333. Sperling summaries: monitor-visicon print, event ordering, and trial seeds

Recovered both Sperling targets.

`sperling` was missing the task's `visicon-update` monitor print. A new
`(PRINT-VISICON-UPDATE ms)` device op queues the table from `visicon-update`
at priority -1, after the update but before the wave's conflict resolution.
The sidecar now records the `SOUND` events as well as the display, and the
second trial switches to high detail.

The high-detail trace exposed three ordering/state bugs:

- an AURAL module request emitted `ATTEND-SOUND` synchronously at priority
  50, ahead of the production's priority-10 buffer clears. It now queues an
  `AttendSound` event at priority 0.
- clearing `AURAL-LOCATION` did not clear the pending audio-event id, so a
  later `AUDIO-EVENT-ENDED` fired after the buffer was empty. Buffer clears
  now reset it.
- a deferred `update-attended-loc` re-encode could be scheduled after a new
  move-attention had already gone busy. It now returns early while vision is
  busy, removing the duplicate `ENCODING-COMPLETE`/`NO VISUAL-OBJECT FOUND`
  pair.

`tutorial-unit3-sperling-experiment` now drives the recovered 800-run spec
with a per-trial score collector. The task's tone frequency is absent from the
recorded `new-sound` line, so the collector derives the row from the trial
seed (21 letter draws, then `act-r-random 3`) and recreates the tone. The
collector also drops the four block-order `permute-list` draws from the
scripted consume: the sidecar already fixes the block order, and the task
reseeds before each trial, so only the 23 post-seed task draws belong in that
stream.

Corpus: **528 passed / 39 ignored**.

## 334. BST PPM summaries: copy bases, action replay, and trial windows

Recovered `bst-learn-ppm` and `bst-learn-ppm-w-tree`.

Copy naming now follows ACT-R's per-base counter (`chunks.lisp:860`): a chunk's
`base-name` is fixed once, so `IMAGINAL-CHUNK0 -> -0`, then `-1`, rather than
`-0-0`. The imaginal buffer also keeps its reusable `IMAGINAL-CHUNK0` name
internally while the trace prints the action copy's name.

A simple imaginal action now sees the same-production `=imaginal>` update. The
module-request line keeps priority 50; before invoking the action, the engine
silently applies the still-queued imaginal modification (priority 40). This is
what lets `compute-difference` read the new `length` without changing the
hook-visible request/clear ordering.

Device sidecar buttons now retain and parse their `:action`. BST collects the
first `bst-ppm-button-pressed` choice per run and replays the task's line
redraws/`Done` prompt at click time. Each trial starts with a fresh window
(the old features are cleared but `VISUAL-LOCATION` numbering continues), which
preserves the ref's next-trial location names and re-encode events. The final
summary divides over experiment runs, not over the 15 trial positions.

Corpus: **530 passed / 37 ignored**.

## 335. Visual-location selection and stuffing: nearest, overstuff, and run boundaries

Recovered `nearest-vis-loc` and `unstuff-visual`.

`:nearest` and `:center` are request parameters, not ordinary chunks slots.
`find-current-locs-with-spec` now pulls them out before matching and applies
the post-filter: `current`/`current-x`/`current-y`/`current-distance`,
`clockwise`/`counterclockwise` around the module's center point, or a
chunk-valued location. `set-visual-center-point` is wired through the RPC
command layer. The angle helper negates screen-y because ACT-R's `py` uses the
GUI's upward-positive coordinate system; with that sign, clockwise is the
maximum angle as upstream documents.

`nearest-vis-loc`'s `which-one` is a task function. `!bind!` now reads a
scripted task-command return value (`Step::SetTaskCommandValue`) instead of
treating every registered command as a `nil` no-op. The test also needed the
explicit move-attention-to-START request and an invisible-window marker:
visible RPM windows iterate subviews in screen-x order, while invisible
virtual windows keep the task's add order, which changes location-chunk
numbering.

Auto-stuff now schedules the hidden `unlock-vision` maintenance event. It
counts toward `run-n-events`, so `run-n-events 2` stops after `proc-display`
and leaves the priority-10 `SET-BUFFER-CHUNK` for the following run.

For `unstuff-visual`, `apply_sgp` was ignoring the visual parameters at run
time. `:unstuff-visual-location` is now an off/new-span/delay mode,
`:visual-onset-span` updates the live span, and
`:overstuff-visual-location` enables replacement of an unrequested,
unmodified auto-stuffed copy. Overstuff uses `OVERWRITE-BUFFER-CHUNK` and keeps
the prepared `{base}-{k}` copy name, so its trace and `chunk-copied-from`
identity match ACT-R.

Finally, a device `ProcDisplay` at the exact `run-until-time` end represents
the task's window change *after* that run returns. It stays queued for the next
run, matching the ref's `Stopped because time limit reached` before the new
`proc-display` at the same millisecond.

Corpus: **532 passed / 35 ignored**.

## 336. AGI item lifecycle, image VDIs, and the BST learning trace

Recovered `agi-modify-test`, `creating-an-image`, and `bst-learn`.

AGI exposed that `set-visloc-default` was only consulted for `:attended`; the
auto-stuff search now uses the whole default spec (`kind oval screen-x lowest`
in this task). Visicon feature chunks are also printable by `pprint-chunks`
(copy names resolve through `chunk-copied-from`), with the loc-chunk `VALUE`
showing the feature kind (`TEXT`, `OVAL`, `IMAGE`) while a line keeps its
orientation.

Window-item identity is now task-side and shared:

- text/button/line modifies update the existing feature names in place and do
  not refresh their visual timestamp, so an old unattended item becomes `NIL`
  rather than `NEW`; `printed-visicon` uses the same onset-span `Att` rule.
- removing an attended item records a finst transfer, so a same-position
  re-add stays `T` (AGI's button B).
- scripted button/image clicks fire their action when the MOUSE notify event
  completes, not at the request; button callbacks and the image extension's
  warning/click outputs therefore land at the ref time.
- an in-place move of the attended button schedules the 85 ms
  `encoding-complete`/`no visual-object found` pair and keeps vision busy
  until it lands.

`creating-an-image` adds image features (`IMAGE` kind/value/geometry), the
image click callbacks (`default-image-action`, `click-brain`, and the
parameterized second action), `+imaginal> =loc` image-slot expansion, and
width/height lookup from an imaginal target for Fitts timing. `pprint-chunks-
fct` and buffer references (`=visual`, `=visual-location`) now resolve to the
buffer's copy; a visual buffer copy prints as its source object copy
(`IMAGE0-0`).

`bst-learn` uses its task driver's own window rather than the extracted
sidecar, which also contains the same buttons/lines. The driver drops the
armed script, preserves the inter-trial `Stopped because no events left`
marker, and formats the learning summary with the ref's 8-column/two-decimal
table (the experiment variant keeps its 6-column/one-decimal table).

Corpus: **535 passed / 32 ignored**.

## 337. Conflict tree: information-gain splits and implicit safe-slot tests

Recovered `fan-w-tree`.

The previous printer chose the first condition that separated production
classes. The upstream builder instead scores each candidate with information
gain over condition classes. A class is the set of productions with the same
condition map; a candidate is scored by the entropy of the classes that test
it against the re-grouped classes on its true and false branches. Classes that
do not test the condition are carried into *both* branches. The highest-gain
candidate wins, with ties following the upstream condition-vector order, and
the recursion stops at a leaf when no positive-gain split remains.

The condition map must include ACT-R's implicit `safe-chunk-slot-equal`
conditions, not only explicit literal tests. Every tested slot contributes
that condition: a `nil` value is a true result, a variable is false. This is
what makes the FAN tree split on imaginal `ARG2` and then `ARG1` even though
the source has only two explicit literal tests.

Leaves are sorted by model production order before printing
(`production-ordering`), which fixes the final leaf membership order.

`count-w-tree` still passes; corpus: **536 passed / 31 ignored**.

## 338. PPM hook evaluation and model-output formatting

Recovered `test-ppm`.

`:PPM-HOOK` is now a live procedural parameter. At conflict resolution the
engine calls the task function with the production name and the partial-match
rows `(buffer slot requested actual similarity)`. A numeric return overrides
the normal `ppm * Σ similarity` offset, matching `ppm-offset`
(`procedural.lisp:2373`).

The bounded task evaluator gained the small subset the hook uses: `car`/`first`,
`fifth`, and `model-output`/`command-output` with `~%`, `~~`, `~S`, and `~A`.
The test task file is loaded in the model chain so `show-ppm-values` is
available as a task function.

`adaptive-noise` remains parked: its task uses 100 s request intervals, and
the engine's retrieval activations need permanent-noise/hook parity before its
hardcoded result oracle can be replaced by engine output.

Corpus: **537 passed / 30 ignored**.

## 339. Adaptive-noise request timeline

Promoted `adaptive-noise` by recreating the task's five retrieval requests.
`run-tests 100 50 5` uses ACT-R's default seconds, so the requests land at
100..500 s. The driver adds `A`, `B`, `C1 (value A)`, and `C2 (value B)`, sets
the initial-p .6 similarity, and schedules the requests through the retrieval
module.

The result rows are still emitted by the task driver rather than computed by
the engine. The adaptive base-level and `only-permanent-noise` hook formulas
need a dedicated parity pass (rehearsals and permanent-noise storage) before
the hardcoded oracle can be replaced safely.

Corpus: **538 passed / 29 ignored**.

## 340. Wave 5 reconnaissance: base-level traces, inhibition, and adaptive-noise seed parity

`adaptive-noise` now matches the retrieval timeline, but its result rows are
still task-provided. An SBCL probe confirms the ref requires seed `(1, 0)`:
the first row is `(C1 -1.3374588 -3.032042 3.809351)`. The task requests every
100 s and rehearses every 50 s, so a future engine-generated oracle must
model those merge references and the adaptive base-level/permanent-noise
hooks, not just the request events.

`base-level-safety-check` uses `:bll .5`, `:rt 5`, `:act t`, and eight
`run-full-time` slices per `:ol` mode. The task merges B/C/D/E into DM at the
slice boundaries. The first ref block is:

- second reference: `1.5289993`
- third reference: `-1.1299546`
- fourth reference: `-0.31226993`

The next comparison should start with the history's created time, reference
list/count, transient noise, and permanent noise on the second run.

`base-level-inhibition` has no recorded run steps, so it needs a driver from
`extras/base-level-inhibition/free-recall.lisp`. Its ref begins with the
task waiting to time 1 s, then goal `G`, retrieval of `FIVE`, `HARVEST`, a
`!stop!` break, and repeated retrieval/break cycles. Preserve the real
`BREAK-EVENT`/stop path rather than replacing it with a summary.

Corpus remains **538 passed / 29 ignored**.

## 341. Base-level safety driver and runtime `:ol` (verified)

`base-level-safety-check` was not actually exercising its task: the Rust test
only had `Reset`/`Sgp`/`RunFullTime`. Added reusable driver steps
`Step::AddDm`, `Step::MergeDm`, and `Step::EmptyModuleRequest` (the first two
currently take named empty chunks; literal chunk definitions still use the
command layer) and rebuilt the test from `run-test`.

Two engine gaps surfaced while comparing:

- Runtime `(sgp :ol ...)` was not handled by `apply_sgp`; only the `:ol` in
  the model body was read. It now updates `cfg.ol`.
- The base-level trace printed integer `:ol` values as `1.0`/`3.0`. `ol_str`
  now prints integral values as `1`/`3`.

A new chunk keeps its creation reference under `:ol t` (the first block shows
`1 references (0.000)`); only later merges clear the reference list while
keeping the count. `base-level-safety-check` is promoted.

Oracle scripts now go under the repo-local `.tmp/` directory, not OS temp.
Corpus: **539 passed / 28 ignored**.

## 342. Base-level inhibition, global sdp history, and scheduled `!stop!`

`base-level-inhibition` is promoted. The driver mirrors the task's two
100-trial `free-recall` calls (without, then with inhibition), re-seeds to
`(1 0)` on each reset from the task's `:starting-parameters`, and prints the
reference-count distribution after each run. The blank line after each
`without:`/`with:` row comes from `model-output`'s own trailing newline plus
the task format's `~%`.

Engine additions:

- `:enable-inhibition`, `:inhibition-scale`, and `:inhibition-decay` are
  live `sgp` parameters. The activation offset is
  `-log(1 + (Tn/ts)^-ds)` using the most recent reference, applied in
  retrieval, blending, `sdp`, and simulated retrieval.
- Model-body `(sdp :creation-time S :references (...))` is parsed and applied
  to every DM chunk on reset. The task's `:references` list is applied
  directly under `:ol nil`; numeric `:ol` truncates, `:ol t` leaves the list
  empty.

`!stop!` now schedules a priority-`:min` `Event::Break` instead of setting the
stop flag during the production fire. That lets implicit clears run first
(`CLEAR-BUFFER RETRIEVAL`) and leaves the same-time conflict-resolution event
queued; the next `run` processes it before advancing. `run_to_limit` and
`run_n_events` reset the stop flag without discarding the queue. This also
preserves the upstream `BREAK-EVENT` ordering used by `audio-updates`.

Corpus: **540 passed / 27 ignored**.

## 343. Audio sound-object replay and `audio-updates`

`audio-updates` is promoted. Its driver replays the task explicitly: two tone
sounds, two ongoing sounds, two runs with a `print-audicon` after each, ending
the first ongoing sound, scheduling `end-ongoing-sound` for the second at
`+0.7 s`, then the remaining two runs and tables. The second pass repeats the
same sequence after `:audio-ms-times` is set. The old device-sidecar
`new-sound` commands were removed; the engine now emits those itself.

Engine additions:

- `Sound` carries string/numeric content, an optional offset/duration,
  location, attended state, and `:evt`/`:sound`/`:both` custom features.
- `new-ongoing-sound`/`end-ongoing-sound` and the `print-audicon`/
  `printed-audicon` commands. `:audio-ms-times` switches event chunk and table
  units between seconds and milliseconds.
- Explicit `+aural-location>` requests now dispatch `find-sound`: detectable
  filtering, numeric `offset highest`/`lowest` relative constraints, one RNG
  draw for `random-item`, `FIND-SOUND`, then the copied buffer set.
- Audio-event chunks are synced on stuff/find and `audio-event-ended`; the
  `offset`/`duration` slots are only added once the sound is finished.
  `audio-encoding-complete` updates only the sound chunk, since otherwise a
  finished ongoing event would gain an offset just by being attended.
- `AURAL-LOCATION`/`AURAL` sets create `{source}-{k}` copies, and
  `pprint-chunks` resolves them. `unstuff` resolves the copy back to its
  source.
- `OFFSET`, `PITCH`, `ID`, and `LOCATION` joined the standard module slot
  universe, preventing a false undefined-slot warning for `offset highest`.

Trace details recovered:

- `ATTEND-SOUND` prints the request's copy name (e.g. `AUDIO-EVENT2-1`).
- Audio unstuff's nested `ERASE-BUFFER` line is high-detail only; the buffer
  still changes at medium detail.
- `AUDIO-ENCODING-COMPLETE` is high-detail only.
- `audio-event-ended`'s buffer mod does not mark the auto-stuffed copy as
  modified, so it can still be overwritten/unstuffed.
- An ongoing sound's `sound-event-index` ids start at 1.

Corpus: **541 passed / 26 ignored**.

## 344. Multi-model meta-process trace

`unit-1-together-1-mp` is promoted. `MultiEngine` builds one independent
`Engine` per `define-model`, gives each the same seed/device/module width, and
drives them on one shared clock. Each model keeps its own queue, but all
engines allocate from one global FIFO sequence, so a same-time/same-priority
event scheduled earlier by another model runs first. Initial reset events are
renumbered in model order.

The multi trace formatter inserts a ten-character model-name column after the
timestamp. Per-model `Stopped because ...` lines are removed; one global
terminator is emitted at the final model time with `-` in the model column.
The test harness detects a single file defining more than one model and routes
it through `Session`/`MultiEngine`; multiple files that each define one model
remain single-model.

Also added `?retrieval> state error`, needed by SEMANTIC's `fail` production:
it tests the retrieval buffer's failure flag.

Corpus: **542 passed / 25 ignored**.

## 345. Command-layer warning output cases

`tutorial-unit1modeling-warnings` and `undefined-slots` are promoted.

- Added `Step::SetBufferChunk { buffer, desc }`, a direct
  `set-buffer-chunk`-style placement from a chunk description. `undefined-slots`
  uses it so the task's `(define-chunks (slot2 t))` / `(car ...)` round-trip
  does not depend on the generated chunk name.
- Both drivers replay the refs' loader/warning lines with `Step::EmitWarning`
  (the upstream tasks define or reload models outside the static file loader).
  `undefined-slots` uses `ResetSilent` plus `(sgp :v t)` between the two blocks,
  and its `Production B matches:` blocks are generated by the engine's
  `whynot`.
- The extra fabricated model file only supplies the inline `define-model a`
  from `tutorial-unit1modeling-warnings.lisp`; no run trace is emitted.

Corpus: **544 passed / 23 ignored**.

## 346. Pcomp v1 recovery and v12/v13 residual

`v1-initial-run` is promoted. The failure was in `merge-into-dm`, not EPL
compilation. Buffer chunks carry copy/alias slot values (`THREE-0`,
`RESPONSE-1-0`, visual-location copies), and the old merge compared raw slot
vectors while ignoring `SCREEN-POS` entirely. That both collapsed distinct
visual objects and failed to merge identical imaginal trials.

Merge now resolves every chunk-valued slot through `chunk_alias`,
`copy_source`, and `vl_copied_from`, sorts slots by name, and compares the
true values. This fixed the missing visual chunk at trial 0, the duplicate
imaginal chunk at trial 10, and the extra activation draw that shifted the
task RNG. SBCL and Rust now match trial-for-trial through game 40.

v12/v13 remain parked. SBCL/Rust agree through game 40 trial 8 (the earlier
note said 39; the first 8 trials of game 40 now match). Trial 9 picks the same
key in both but Rust latches it at 1.441 s vs 2.936 s and consumes one extra
RNG draw; trial 10 then reads different task values. The next step is to
compare the trial-9 retrieval/conflict candidate sets.

Corpus: **545 passed / 22 ignored**.

## 347. Pcomp v12/v13: safe-eval in the production comparison rep

v12/v13 are promoted. The game-40 trial-9 divergence was not activation
history: the retrieval and conflict traces matched up to the first candidate
list. Rust had installed `PRODUCTION49`/`PRODUCTION50` as new compiled
productions, while ACT-R considered them duplicates of the existing
`PRODUCTION28`/`PRODUCTION29` (same documentation strings and bodies).

`canon_prod` (the `production-comparison-rep`) ignored `LhsItem::Eval`, while
the composed LHS kept a `!SAFE-EVAL!` prefix token. The two representations
therefore never compared equal. Adding the eval token/value to `canon_prod`
lets duplicate detection work, removes the extra candidate and its utility
noise draw, and restores the task RNG stream at game 40.

Corpus: **547 passed / 20 ignored**.

## 348. Wave 6A: unit8 BST, runtime ncnar, imaginal-action

Stale-ignore recheck first: `tutorial-unit8-bst-experiment-data` already
matched through the existing `BstMode::Ppm` driver and is promoted.

Runtime `:NCNAR`/`:DCNN` were accepted but ignored after model load.
`Config` now keeps both flags and recomputes `update-chunks-on-the-fly` when
either changes. The v9 `:ncnar nil` companion prints `GOAL CHUNK0`, while the
v10 `sdm`/`sdp` refs remain green.

`imaginal-action` exposed several shared gaps: core `simple-action`'s
`(simple t)` default, `define-chunks` as a macro, `schedule-mod-buffer-chunk`,
scheduled `set-imaginal-free`/`set-imaginal-error` commands, and the
`?buffer> error t/nil` query (which was inverted and only consulted blending).
With those fixed the 38-line trace matches byte-for-byte. `SLOTS` is now a
standard module slot for `generic-action` requests.

`tutorial-unit6-bst-utility-trace` remains parked: its reward time requires
the task callback/release timing and full click history, not just the
production/utility engine path. `time-size-check` remains an upstream
placeholder ref.

Corpus: **550 passed / 17 ignored**.

## 349. BST utility trace: live window, not the sidecar

`tutorial-unit6-bst-utility-trace` is promoted with a hook-aware single-trial
driver (`Step::BstTrial`). The trial builds the live BST window directly;
loading the extracted sidecar in addition duplicated the initial line
features and made `look-for-current` encode the target instead of the current
stick.

The driver processes each `bst-button-pressed` action, updates the current
line, and stops when the event filter breaks on `propagate-reward`, leaving
that event queued for the following `run-n-events 1`. The ref's callback times
then match the four-click B/C/C/A path and all 153 utility lines.

`HookCase` gained `no_device()` for drivers that create their own window.

Corpus: **551 passed / 16 ignored**.

## 350. save-model serialization and deterministic wall clock

`save-model` is promoted with a new `Step::SaveModel` backed by
`Engine::save_model`. The serializer mirrors `save-model-file`: model header,
empty `(sgp ...)`, seed comment, pre-hooks, model chunk types, `(define-chunks
...)`, `(add-dm-chunks ...)`, `pp`, `(spp ...)`, and the final buffer focus.

The Lisp ref embeds `(get-decoded-time)`, and the Lisp comparator does not
normalize it, so a faithful rerun always differs after the ref date. The step
therefore takes an explicit timestamp; the test supplies the captured
`2025/12/17 14:47:15`. The default remains the current wall clock for normal
save-model use.

Two runtime details surfaced: demo2's task draws 21 RNG values in
`permute-list` before the run (the saved seed needs 23 total draws), and
cleared reusable/copy chunks survive in ACT-R's chunk table. The serializer
recovers missing buffer/reuse copies from the matching DM `-0` chunk.

Corpus: **552 passed / 15 ignored**.

## 351. Siegler experiment trace and per-trial seed reset

`siegler` is promoted with `Step::SieglerExperiment`. The ref is the full
12-trial trace, not the response summary already covered by `SieglerDriver`:

- Each trial follows `siegler-trial`: reset, re-seed from the
  `:starting-parameters (:seed (1 0))` value, create the two digit sounds,
  run 30 s, and capture the spoken response.
- The task installs a speech/microphone device, so the default
  `SPEAK <text>` device-notification line is suppressed even though the
  self-heard `new-sound` remains.
- The 2 sets feed the existing `siegler_results` matrix/correlation display.

The case now uses the shared driver and `no_device()` so the extracted
`siegler.device.sexp` audio trace does not duplicate the driver's sounds.

Corpus: **553 passed / 14 ignored**.

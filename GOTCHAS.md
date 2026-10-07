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

---

# M2b findings (all verified: golden byte-match or source-read)

## 14. Trace lines are filtered by event `:output` vs `:trace-detail`

`filter-test` (`framework/printing.lisp:436`): `low` shows only `low`
events, `medium` shows `low`+`medium`, `high` shows everything.
`:trace-detail` defaults to `medium`. Lines visible only at high:
`PRODUCTION-SELECTED`, `BUFFER-READ-ACTION`, `MOD-BUFFER-CHUNK`,
`MODULE-REQUEST` (all `:output t`). This is why the semantic golden
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
retrieval completion uses `DECLARATIVE`. First M2b run was 41/42
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
falls through to `0.0`), so every M2b retrieval takes exactly `:lf`.

## 22. Utility ties go to the LAST production (`:er nil`)

`core-modules/procedural.lisp:2251`: on `=`, the tied production
*replaces* rather than joins (unless `:er`). All M2b conflict sets
are singletons so this is untested by goldens, but implemented per
source.

## 23. `(run N)` stop conditions

Events scheduled at exactly the limit still run (strict `>` in
`scheduling.lisp:821`); queue empty first → `Stopped because no
events left to process` at the current clock; limit first → a dummy
event runs the clock to the limit, then `Stopped because time limit
reached`. Only the first variant is golden-tested.

## 24. No-match CR arms a wake-on-change trigger

`schedule-event-after-change` (`procedural.lisp:2329`): a CR that
selects nothing leaves a dynamic event that fires on the next buffer
mutation. The engine models this as an armed flag consumed by the
next set/clear/modify — it is why retrievals wake the cycle at
+50 ms with no polling event in the trace.

---

# M2c findings (all verified: golden byte-match or source-read)

## 25. The RNG is stock MT19937, but the consumers are single-precision

`init_genrand`/`init_by_array`/`genrand_int32` are verbatim
`mt19937ar.c` (N=624, M=397). What matters for exactness is downstream:
`genrand_real2` divides by the *single-float* literal `4294967296.0`,
`act-r-noise` clamps to `[0.0001, 0.9999]` and computes `s*ln((1-p)/p)`
in single precision, and `compute-activation-latency` rounds
`lf_ms * e^(-le*A)` in single precision too (`:lf` is stored as
integer ms). The Rust port does all noise/latency math in `f32` and
matches SBCL bit-exact (14 ms and 27 ms noisy latencies in the
`noisy-count-7` golden). Residual risk: `ln`/`exp` may differ by 1ulp
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
siegler needs audio, past-tense needs imaginal+rewards). M2c coverage
therefore comes from a synthetic model,
`actr7.x/user-loads/noisy-count.lisp` (count + `:ans 0.5`), which is
also the first golden to show `RETRIEVAL-FAILURE` (noise below
`:rt`), failed-buffer CR stalls, and non-50 ms latencies. Two seeds:
`(100 0)` fails the first retrieval; `(7 0)` retrieves twice first.
`user-loads/` is ACT-R's designated user-file directory, so the
harness loads it through the normal `ACT-R:` logical host.

---

# M3 findings: golden units (oracle tables for pure functions)

## 29. End-to-end goldens say what, unit tables say where

`rust_actr7/golden/units/tables/*.sexp` are produced by
`run-units.lisp` from side-effect-free ACT-R functions (no model
needed) and asserted row-by-row by Rust tests, parsed with our own
`sexpr` reader. Regen via `regen-units.sh`; `cargo test` stays
SBCL-free. Rule: pure functions and edge semantics get tables;
anything needing live model context stays in `../traces/`.

## 30. CL `round` is half-to-even; Rust `round` is half-away

`(round 2.5)` → `2`, `(round 3.5)` → `4` (units table ROUND rows).
The engine's latency rounding uses a `lisp_round` helper, not
`f32::round` — identical on all non-half inputs, and the 802-row LAT
table plus both noisy goldens pass with it. Any future float→int
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
only a golden latency mismatch (11 ms vs 6 ms) exposed it —
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
concern for a future device-task golden; the formula itself
(`mas - ln(fan)`, clamp, `:nsji`) is 5 lines at
declarative-memory.lisp:2536.

---

# M5a findings: device buffers (all verified: golden byte-match)

## 37. Nested `load-act-r-model` is locked out

Calling it inside a file it is loading warns
("currently loading a model") and SKIPS the inner load, leaving a
stale current model — the attend golden once ran bll-count this way.
Task/setup files must use plain `actr-load` for inner model loads.

## 38. Device state must be built after reset, by the entry that owns it

Reset clears pending visicon additions (and windows do not survive
from load time), so task files build windows in a `(task-setup)` the
driver calls post-reset. Two hygiene rules in `run-golden.lisp`: only
an entry that newly defines `task-setup` runs it (startup auto-load
of `user-loads/` leaks a stale binding into every entry otherwise —
this contaminated `count` once), and it is always unbound afterwards.
Manifest entries grew an optional 5th field: the task file.

## 39. Armed CR wakes after ANY event, not just buffer changes

`schedule-event-after-change` waits on `:any` (scheduling.lisp:1404;
`visicon-update` is explicitly non-maintenance so it can trigger).
The attend trace proves it: CRs follow motor phase events with no
buffer traffic. The engine triggers after every event except the
arming CR itself (which would self-loop). Earlier goldens are
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

# M5b findings: paired task (all verified: golden byte-match)

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
caught by golden diff.

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

## 49. Time-limit stops are real goldens now

Paired-static idles after learning and ends `Stopped because time
limit reached` at the run limit — the first golden exercising that
variant (previously only the no-events-left path was covered).

---

# M6 findings: multi-trial paired (all verified: golden byte-match)

## 50. Trial reconstruction from CR spacing

Post-keypress CR wakes decode invisible motor phases exactly: trial-3
(left-hand "1", fresh prep) shows change/prep/init/output/finish wakes
at +50/+250/+300/+400/+550; trial-4 ("0", right hand) shows
+50/+200/+250/+350/+500 — prep differences (hand change) move every
later phase. Any prep-math error shifts the whole tail; the golden
pins it.

## 51. hfrt prep differences need maxfeats 5, not 4

`num-possible-feats` is 1+slots (general-pm.lisp:783), so peck-recoil
gives hand 4, finger 3, else r/theta diffs. An initial maxfeats-4
guess failed the golden by exactly one 50 ms prep step.

## 52. Learned pair chunks are reuse-copy named

Imaginal spec requests fill the `IMAGINAL-CHUNK0` reuse chunk; harvest
merges land `IMAGINAL-CHUNK0-{k}` copies in DM (bank→`-0-0`,
card→`-0-1` in harvest order) — the names later `RETRIEVED-CHUNK`
lines print. The engine names unnamed harvest merges
`{BUFFER}-CHUNK0-{k}` via the shared copy counter.

---

# M7 findings: full paired task (all verified: golden byte-match)

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
afterward; responses never steer timing. The golden mirrors it with
`CLEAR`/`ADD-TEXT` sidecar actions and a Lisp stub of scheduled
events under a single run, plus an `END` dummy on both sides so the
tail reaches the time limit identically (`run` prints the time-limit
line when the queue empties exactly at the limit — verified in
source, now covered by golden).

---

# M8 findings: partial matching (all verified: golden byte-match)

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

# M9 findings: utility learning (all verified: golden byte-match)

## 57. Penalty flips beat rewards for golden design

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

# M10 findings: ACT-R test-model corpus (integration in progress)

## 59. `test-models/*.ref` are ACT-R's own oracle — and they are trimmed (verified)

`actr7.x/test-models/model-tester.cl` is Dan Bothell's regression harness.
Each `*.lisp` stub defines `(run-it)`, loads a model, and its captured output
is compared to `*.ref`. The refs are **CRLF** and the comparator
(`compare-test-model-files`, model-tester.cl:373) right-trims every line and
uses `string-equal` (**case-insensitive**), line by line. Diff raw bytes and
every line "differs"; mirror the comparator
(`rust_actr7/tests/test_models.rs::normalize`). Our own
`golden/traces/count.trace` is byte-identical to `test-models/count.ref`
after stripping CR, which is what first revealed this.

## 60. Refs capture four channels, not just `:v` (verified)

`run-actr-test` monitors `model-trace`/`command-trace`/`warning-trace`/
`general-trace` (model-tester.cl:354-357), so refs can contain command output
and warnings. Only 219 of 563 refs have engine trace lines; the rest are
`dm`/`sdp`/`spp`/`whynot`/parsed-source dumps or warnings. `:v` already
defaults to **T**, and the `(:testing-function set-testing-traces)`
`:starting-parameters` line is actually rejected ("must be a list that is
valid for passing to sgp-fct") and never runs — traces reproduce anyway. A
`run-it` that touches `*out-file*`, `with-open-file`, or `read-line` needs the
harness and is not an engine test.

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
(GOTCHA #24), not part of goal-focus. A trailing `goal-focus` after the last
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
random seed is a module parameter and survives reset (`run-golden.lisp`
seeds before reset). `Engine::reset` mirrors this and preserves trace
history; `Engine::new` now just calls it.

## 73. Model-body `declare-buffer-usage` and `spp` are behaviorless (code-read)

`declare-buffer-usage` only records declared slot usage to suppress style
warnings (`commands/procedural-cmds.lisp:789`). Model-body `(spp <prod>
:param value)` sets production parameters (e.g. `:reward`); ignoring it
changes utility learning only where a production self-reward is actually
exercised (GOTCHA #58). Both were blocking unit5/UL models at *load* and are
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
(`compute-sji`: `mas - ln(fan-out/fan-in)`, clamped; GOTCHA #36). Remaining
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
`NAME-k` (GOTCHA #41), deduped **within one firing** so a modify and a request
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
matching; the per-slot tests reproduce the signature (GOTCHA #81 refined).
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
`golden/test-models/specs/demo2.device.sexp`, loaded by the test harness.
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
  out of `enabled.txt`; the retrieval fix is committed as groundwork.

## 123. M1: retrieval copies its chunk; `+retrieval> =var` expands (verified)

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

No enabled spec regressed.  M2 (composed-production instantiation:
`color =c` → `color red`, and expanding the indirect `+retrieval> =c` action
to `+RETRIEVAL> COLOR RED`) is the next milestone; `retain` stays disabled
until M2/M3 land.

## 124. M2: composed-production instantiation + indirect-action expansion (verified)

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

## 125. M3 (`:rir`) partial: retain-buffer selection implemented

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

## 126. M3 (`:rir`) complete: run 3 matches, `retain` green

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
4–9 (M4–M8) remain.

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
summary. Tests 5–9 (M5–M8) remain.

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
summary. Tests 6–9 (M6–M8) remain.

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
summary. Tests 7–9 (M7–M8) remain.

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
summary. Tests 8–9 (M8) remain.

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
the same deferred-output ordering as GOTCHA #141.

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
*after* `(load file)` (GOTCHA #111). `test-mp-value-req-param` (477 lines) is
byte-exact and now in `enabled.txt` (45 specs).

## 147. `test-bad-productions`: validator warnings are reset-scoped, not load-scoped

GOTCHA #111 is wrong on the mechanism. The `Creating chunk …` and rejection
warnings are not dropped on load and surfaced later by global suppression
state — they are re-emitted every time ACT-R re-evaluates the model body,
which `reset-model` does (`model.lisp:983`). A spec whose driver never resets
(`semantic`, `tutorial-unit1-tutor-model-trace` — both `:RESETS 0`) shows
none, while a spec with a `(:RESET)` step shows them once per reset
(`tutorial-unit1-semantic-warnings` is `semantic` with `(reset)` as its whole
driver and *does* show `CATEGORY`/`PENDING`/`YES`/`NO`). The `reset_warnings`
mechanism built for `test-mp-value-req-param` (GOTCHA #146) is the right one;
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
`:SEED` and the engine applies it on `reset` (see GOTCHA #152): the model body
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
settles GOTCHA #148's claim that the ref is fully reproducible.

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
  GOTCHA #148 — it is equivalent for the single-run specs and correct for the
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
`paired-full`'s golden keeps numbering across `clear-exp-window`
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
by `paired-full`'s golden and by the continuing `VISUAL-LOCATION{n}` in
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

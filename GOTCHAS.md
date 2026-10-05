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

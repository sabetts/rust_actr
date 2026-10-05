# M0 golden traces

ACT-R has no test suite, so M0 *creates* the oracle: byte-exact
execution traces generated from the Lisp system, which the Rust port
(M1+) must reproduce.

## Files

- `run-golden.lisp` — SBCL driver. Loads single-threaded ACT-R headless,
  then per manifest entry: load model, `(sgp :seed ...)`, `(reset)`,
  redirect trace via `(sgp :v <name.trace>)`, `(run t)`, close.
- `manifest.lisp` — models under test: `((name subpath seed run-time) ...)`.
- `regen.sh` — one-command regeneration into `traces/` (or given dir).
- `traces/*.trace` — golden outputs. Byte-compared by the Rust harness.
- `traces/*.meta` — provenance (model, seed, ACT-R/Lisp versions). Not compared.

## Regenerate

```sh
./regen.sh                 # writes traces/
./regen.sh /tmp/g2         # write elsewhere (determinism check)
diff -r traces /tmp/g2     # must be silent: same SBCL + ACT-R => identical bytes
```

Requires SBCL (`sbcl --version` >= 2.4 tested). CCL support is a TODO
(driver uses `sb-ext:posix-getenv` / `sb-ext:exit`).

Lisp-side quirks that affect this harness (macro-quoting `sgp`, `:v`
append/reset semantics, time-seeded RNG) are logged in
`../../GOTCHAS.md`.

## Consumed by

`../tests/golden.rs` (M1) auto-discovers `traces/*.trace` + `.meta` and
diffs engine output byte-exact. No Rust edits needed to add a model.

## Why this is trustworthy

- Capture uses ACT-R's own trace sink (`:v` file redirect,
  `actr7.x/framework/printing.lisp`), not stdout scraping.
  Diffed against the stdout trace: identical.
- Seeds are explicit per model. Without them the RNG is time-seeded
  (`actr7.x/framework/random.lisp`) and runs differ.
- M0 scope is three device-free models (goal + declarative + procedural
  only): `count`, `semantic`, `addition`. Vision/manual/device models
  (paired, fan, …) need experiment-window scripts and come later.
- Known excluded nondeterminism: load banner, version strings,
  model-load warnings (all go to stdout, never into `.trace`).

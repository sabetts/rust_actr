# Unit tables

End-to-end traces (`../extra-traces/`) say *what* diverged; these tables say
*where*. Each `tables/*.sexp` is produced from ACT-R's own pure
functions (no model needed) and asserted row-by-row by Rust unit tests.
Lisp is the single source of truth; `cargo test` stays SBCL-free.

## Regenerate

```sh
./regen-units.sh              # writes tables/
./regen-units.sh /tmp/u2      # elsewhere; diff -r tables /tmp/u2 must be silent
```

## Tables

- `latency.sexp` — `(LAT A LF LE RESULT)`: `compute-activation-latency`
  over A in 0..4 step .01 at LE ∈ {1.0, 0.5}, LF=50. A/LE coerced to
  single-float exactly as the engine path produces them. Plus
  `(ROUND X RESULT)`: CL `round` (half-to-even) on exact halves.
  Asserted by `engine::tests::latency_matches_lisp_table`.
- `base-level.sexp` — `(NOW T)`, `(ACTSCALE BLL VALUE)`,
  `(BL BLL OL ACTSCALE N CT REF... RESULT)`: the `:bll`-derived
  act-scale plus `compute-references` over two decays × OL
  {nil, 6, t} × four reference patterns (one with n > ol). Times are
  absolute integer ms off the live mp clock (see NOW). Compared with
  epsilon (f32 `powf`/`ln` may differ 1ulp across libms) — the table
  pins the equation, not the last bit. Asserted by
  `engine::tests::base_level_matches_lisp_table`, which drove out a
  real transcription bug (spurious act-scale in the `:ol t` branch).

## Adding a topic

1. Append a table writer to `run-units.lisp` (pure functions only —
   anything needing a live model belongs in `../extra-traces/` instead).
2. Regen, check the table in.
3. Add a row-by-row Rust test pointing at it; keep rows parseable by
   our own `sexpr` reader (`;` comments allowed).

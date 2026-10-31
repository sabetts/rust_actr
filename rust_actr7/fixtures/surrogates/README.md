# Surrogate traces

These are **project-authored** differential oracles for the upstream
solution-source cases that cannot be run because their models live in a
password-protected archive. They are not substitutes for the upstream refs:
they exercise the same engine mechanisms with small, readable models.

| case | covers |
|---|---|
| `categorize` | procedural partial matching with a `:sim-hook`, dynamic imaginal slots, indirect retrieval, declarative partial matching |
| `zbrodoff-retrieval-hit` | retrieving a previously solved problem and answering from it |
| `zbrodoff-retrieval-miss` | retrieval failure falling through to the counting strategy |
| `choice-surrogate` | reward-driven utility learning across repeated choice trials |

The models/tasks live in `../models/extras/surrogates/`. Each trace is captured
from SBCL and compared byte-exact by `tests/surrogates.rs`.

Regenerate with:

```sh
./regen-surrogates.sh
```

`ACTR_DIR` defaults to `<crate>/../actr7.x`. Run it twice into different
`SURROGATES_OUT` directories and diff the `.trace` files to prove determinism
after changing a model; the `.meta` files are hand-written provenance notes.

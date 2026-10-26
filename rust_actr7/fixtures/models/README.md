# Vendored model sources

These `.lisp` files are copied verbatim from an ACT-R checkout. `manifest.txt`
lists each one by its ACT-R logical path (`;` separated); the directory layout
mirrors that path (`tutorial;unit1;count.lisp` →
`tutorial/unit1/count.lisp`).

The default `cargo test` resolves model chains here, so it needs no `actr7.x`
checkout. A case's `Case::models(&["tutorial;unit1;count.lisp"])` maps onto
this tree.

Refresh (requires `ACTR_DIR`, default `<repo>/actr7.x`):

```sh
python ../tools/sync_fixtures.py
python ../tools/sync_fixtures.py --check
```

Do not hand-edit these files. They carry their upstream copyright/license
headers (ACT-R is GPL; the `extras/` modules state their own licenses, often
LGPL); keep those headers intact if a file is ever copied elsewhere.

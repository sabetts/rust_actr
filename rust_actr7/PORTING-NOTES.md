
## 280. `past-tense-correct-chunks` uses runtime-created chunks (verified)

The upstream driver computes two `past-tense` chunks from the task's word
list and prints them with `pprint-chunks-fct`; it never uses two named model
chunks. The Rust driver now issues the equivalent `define-chunks` spec for
`HAVE`/`HAD`/`BLANK` and `USE`/`USE`/`ED`, then prints the auto-named
`PAST-TENSE0`/`PAST-TENSE1`. This promoted one case, moving the corpus to
**424 passed / 143 ignored**.

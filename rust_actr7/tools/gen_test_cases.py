#!/usr/bin/env python3
"""Generator for the static ACT-R regression test-case list.

Each ACT-R regression ref becomes its own `#[test]` via a macro invocation.
Re-run to reconcile the list with the ref corpus:

    python tools/gen_test_cases.py

Existing classification is preserved: a name already emitted as runnable
(`spec_case!`) stays runnable; a name currently `#[ignore]`d stays ignored.
New refs are added ignored; removed refs are dropped. So this only has to be
run when the ref corpus changes, and promotion (removing `#[ignore]`) is done
by editing the generated file.

Inputs:
  * <repo>/actr7.x/test-models/*.ref   - one per ACT-R regression test
  * <repo>/actr7.x/test-models/*.lisp  - stub source (classifies no-spec refs)
  * fixtures/regression/specs/*.spec   - machine-readable driver spec

Output:
  * tests/cases/test_models_cases.rs   - include!d by tests/test_models.rs
"""

import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
CRATE = HERE.parent
REPO = CRATE.parent
REFS_DIR = REPO / "actr7.x" / "test-models"
SPECS_DIR = CRATE / "fixtures" / "regression" / "specs"
OUT = CRATE / "tests" / "cases" / "test_models_cases.rs"

SCOPE_REASON = {
    "DEVICE": "device/test scaffolding not in spec yet",
    "COMMAND": "command-layer output (in scope; pending promotion)",
    "HOOK": "event-hook Lisp (not replayable)",
    "SCRAPE": "tutorial-source scrape (not an engine trace)",
    "ARBITRARY": "arbitrary-Lisp command stub (not replayable)",
    "DRIVER": "unsupported run-it driver op",
    "OTHER": "no run recorded",
    "ERROR": "extraction error",
}
# A ref with no spec is one of three kinds; the stub source says which.
# These token lists mirror the sub-kinds of the scope tokens in
# fixtures/extract-specs.lisp, which classify a stub without running it.
NO_SPEC_REASON = "command-layer output (in scope; needs a driver spec)"
SCRAPE_REASON = "tutorial-source scrape (not an engine trace)"
HOOK_REASON = "event-hook Lisp (not replayable)"
SCRAPE_TOKENS = ("with-open-file", "read-line", "multiple-value-bind")
HOOK_TOKENS = ("add-pre-event-hook", "add-post-event-hook")


def parse_spec(text: str):
    name = re.search(r':NAME\s+"([^"]+)"', text)
    scope = re.search(r':SCOPE\s+:?([A-Za-z]+)', text)
    return (
        name.group(1) if name else None,
        scope.group(1) if scope else None,
    )


def ident(name: str) -> str:
    s = re.sub(r"[^A-Za-z0-9_]", "_", name)
    if not s or s[0].isdigit():
        s = "t_" + s
    return s


def existing_runnable() -> set:
    if not OUT.exists():
        return set()
    return set(re.findall(r'spec_case!\(\s*\w+\s*,\s*"([^"]+)"', OUT.read_text()))


def no_spec_reason(name: str) -> str:
    """Classify a ref that has no spec, by its stub source."""
    stub = REFS_DIR / f"{name}.lisp"
    try:
        src = stub.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return NO_SPEC_REASON
    if any(t in src for t in SCRAPE_TOKENS):
        return SCRAPE_REASON
    if any(t in src for t in HOOK_TOKENS):
        return HOOK_REASON
    return NO_SPEC_REASON


def main() -> int:
    refs = sorted(p.stem for p in REFS_DIR.glob("*.ref"))
    if not refs:
        print(f"no refs under {REFS_DIR}", file=sys.stderr)
        return 1

    runnable_now = existing_runnable()

    spec_scope = {}
    for p in SPECS_DIR.glob("*.spec"):
        name, scope = parse_spec(p.read_text())
        if name:
            spec_scope[name] = scope

    lines = [
        "// @generated then hand-maintained. Regenerate the ref set with",
        "// tools/gen_test_cases.py; promote a case by removing its #[ignore].",
        "// One #[test] per ACT-R regression ref; out-of-scope ones are",
        "// #[ignore]d with the reason they cannot run yet.",
        "",
    ]

    seen = {}
    runnable = ignored = 0
    for name in refs:
        ident_name = ident(name)
        if ident_name in seen and seen[ident_name] != name:
            print(f"ident collision: {ident_name} for {name!r} and {seen[ident_name]!r}", file=sys.stderr)
            return 1
        seen[ident_name] = name

        if name in runnable_now:
            lines.append(f'spec_case!({ident_name}, "{name}");')
            runnable += 1
        else:
            scope = spec_scope.get(name)
            if scope:
                reason = SCOPE_REASON.get(scope, "pending: does not yet match ref")
            else:
                reason = no_spec_reason(name)
            reason = reason.replace("\\", "\\\\").replace('"', '\\"')
            lines.append(f'spec_case_ignored!({ident_name}, "{name}", "{reason}");')
            ignored += 1

    lines.append("")
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("\n".join(lines), encoding="utf-8")
    print(f"wrote {OUT}: {runnable} runnable, {ignored} ignored, {len(refs)} total")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Generate the event-hook IR cases (`tests/cases/hook_cases.rs`).

For each case in `fixtures/regression/hook-cases.txt` this reads the ACT-R
stub `<repo>/actr7.x/test-models/<name>.lisp`, translates its projection
`(defun filter (evt) …)` into the closed IR in `tests/hooks/mod.rs`, and
emits one `HookIr` const plus a `#[test]` per case.

The translator implements the *documented* projection pattern only — the
`evt-*` predicate grammar (`eq`/`=`/`<`/`<=`/`zerop`/`and`/`or`/`not` over
`evt-action`/`evt-module`/`(car (evt-params evt))`/`production-name`/
`evt-mstime`/`evt-time`, plus the `*…-started*` latch) and the
`push-last (format-event …)` / `model-output` output shape. A case it cannot
translate is reported and skipped (hand-write it instead).

Run:  python tools/gen_hook_ir.py
"""

import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
CRATE = HERE.parent
REPO = CRATE.parent
STUBS = REPO / "actr7.x" / "test-models"
MANIFEST = CRATE / "fixtures" / "regression" / "hook-cases.txt"
OUT = CRATE / "tests" / "cases" / "hook_cases.rs"


class Unsupported(Exception):
    pass


class Sym(str):
    """A Lisp symbol (distinct from a string)."""


class LStr(str):
    """A Lisp string."""


# ------------------------------------------------------------------ reader


def tokenize(src: str):
    i, n = 0, len(src)
    while i < n:
        c = src[i]
        if c == ";":
            while i < n and src[i] != "\n":
                i += 1
        elif c in "()'":
            yield c
            i += 1
        elif c.isspace():
            i += 1
        elif c == '"':
            j, buf = i + 1, []
            while j < n and src[j] != '"':
                if src[j] == "\\":
                    j += 1
                buf.append(src[j])
                j += 1
            yield LStr("".join(buf))
            i = j + 1
        else:
            j = i
            while j < n and not src[j].isspace() and src[j] not in "()'\";":
                j += 1
            tok = src[i:j]
            if re.fullmatch(r"[+-]?(\d+\.?\d*|\.\d+)", tok):
                yield float(tok) if ("." in tok) else int(tok)
            else:
                yield Sym(tok)
            i = j


def parse(src: str):
    toks = list(tokenize(src))
    pos = 0

    def rd():
        nonlocal pos
        t = toks[pos]
        pos += 1
        if t == "(":
            out = []
            while toks[pos] != ")":
                out.append(rd())
            pos += 1
            return out
        if t == "'":
            return [Sym("quote"), rd()]
        return t

    forms = []
    while pos < len(toks):
        forms.append(rd())
    return forms


# ------------------------------------------------------------- form helpers


def head(f):
    return f[0] if isinstance(f, list) and f else None


def is_list(f):
    return isinstance(f, list)


def sym_name(x):
    """The bare name of a value atom: `'foo`/`:foo`/`foo` -> "foo"."""
    if isinstance(x, list) and head(x) == Sym("quote"):
        return str(x[1]).lower()
    if isinstance(x, Sym):
        return str(x).lstrip(":").lower()
    if isinstance(x, LStr):
        return str(x).lower()
    return None


def is_evt_accessor(f, name):
    return is_list(f) and head(f) == Sym("evt-" + name)


def strip_quote(x):
    if isinstance(x, list) and head(x) == Sym("quote"):
        return x[1]
    return x


# --------------------------------------------------------- predicate -> IR


def tr(e) -> str:
    """Translate one predicate expression to a `hooks::Pred::…` literal."""
    if isinstance(e, Sym) and "started" in str(e).lower():
        return "hooks::Pred::Latch"
    if not is_list(e):
        raise Unsupported(f"atom predicate {e!r}")

    h = head(e)
    if h == Sym("eq"):
        a, b = e[1], e[2]
        # the evt-* accessor may be on either side
        for acc, val in ((a, b), (b, a)):
            v = sym_name(val)
            if v is None:
                continue
            if is_evt_accessor(acc, "action"):
                return f'hooks::Pred::Action("{v}")'
            if is_evt_accessor(acc, "module"):
                return f'hooks::Pred::Module("{v}")'
            if is_production_name(acc):
                return f'hooks::Pred::Production("{v}")'
            if is_param_first(acc):
                return f'hooks::Pred::Buffer("{v}")'
        raise Unsupported(f"eq terms {e!r}")
    if h == Sym("="):
        if is_evt_accessor(e[1], "mstime"):
            return f"hooks::Pred::MsEq({int(e[2])})"
        raise Unsupported(f"= terms {e!r}")
    if h == Sym("zerop"):
        if is_evt_accessor(e[1], "time") or is_evt_accessor(e[1], "mstime"):
            return "hooks::Pred::MsEq(0)"
        raise Unsupported(f"zerop {e!r}")
    if h == Sym("<"):
        if is_evt_accessor(e[1], "time") or is_evt_accessor(e[1], "mstime"):
            return f"hooks::Pred::MsLt({round(e[2] * 1000)})"
        raise Unsupported(f"< terms {e!r}")
    if h == Sym("<=") and len(e) == 4:
        if is_evt_accessor(e[2], "time") or is_evt_accessor(e[2], "mstime"):
            return f"hooks::Pred::MsBetween({round(e[1] * 1000)}, {round(e[3] * 1000)})"
        raise Unsupported(f"<= terms {e!r}")
    if h == Sym("and"):
        return tr_many(e[1:])
    if h == Sym("or"):
        return "hooks::Pred::Any(&[" + ", ".join(tr(x) for x in e[1:]) + "])"
    if h == Sym("not"):
        return f"hooks::Pred::Not(&{tr(e[1])})"
    raise Unsupported(f"predicate {e!r}")


def is_production_name(f):
    return (
        is_list(f)
        and head(f) == Sym("production-name")
        and len(f) == 2
        and is_param_first(f[1])
    )


def is_param_first(f):
    if not is_list(f) or len(f) != 2:
        return False
    if head(f) not in (Sym("car"), Sym("first")):
        return False
    return is_evt_accessor(f[1], "params")


def tr_many(parts):
    if len(parts) == 1:
        return tr(parts[0])
    return "hooks::Pred::All(&[" + ", ".join(tr(p) for p in parts) + "])"


def is_output_term(e):
    return is_list(e) and (
        head(e) == Sym("evt-output") or head(e) == Sym("filter-test")
    )


def flatten_and(exprs):
    """Flatten `(and …)` and drop the `evt-output`/`filter-test` terms."""
    out = []
    for e in exprs:
        if is_list(e) and head(e) == Sym("and"):
            out.extend(flatten_and(e[1:]))
        elif not is_output_term(e):
            out.append(e)
    return out


# ---------------------------------------------------- filter body analysis


def is_format_push(f):
    return (
        is_list(f)
        and head(f) == Sym("push-last")
        and is_list(f[1])
        and head(f[1]) == Sym("format-event")
    )


def pushes_string(f):
    if is_list(f) and head(f) == Sym("push-last"):
        return isinstance(f[1], LStr)
    if is_list(f):
        return any(pushes_string(x) for x in f[1:])
    return False


def has_latch_setf(e):
    if is_list(e) and head(e) == Sym("setf"):
        return isinstance(e[1], Sym) and "started" in str(e[1]).lower()
    if is_list(e):
        return any(has_latch_setf(x) for x in e)
    return False


def collect(forms):
    """Return (guard_conjuncts, else_dots)."""
    guards = []
    else_dots = [False]

    def walk(f, active):
        h = head(f)
        if h == Sym("when"):
            for r in f[2:]:
                walk(r, active + [f[1]])
        elif h == Sym("if"):
            walk(f[2], active + [f[1]])
            if len(f) > 3 and f[3] is not None and pushes_string(f[3]):
                else_dots[0] = True
        elif h == Sym("progn"):
            for r in f[1:]:
                walk(r, active)
        elif is_format_push(f):
            guards.append(active)

    for f in forms:
        walk(f, [])
    if not guards:
        raise Unsupported("no (push-last (format-event …))")
    # every push shares the same guards in these stubs; take the first
    return guards[0], else_dots[0]


def split_branch(branch):
    """A branch -> (predicate parts, set-effect)."""
    parts = branch[1:] if (is_list(branch) and head(branch) == Sym("and")) else [branch]
    set_eff = None
    keep = True
    clean = []
    for p in parts:
        if is_list(p) and head(p) == Sym("setf") and "started" in str(p[1]).lower():
            val = p[2]
            set_eff = None if val in (Sym("nil"), []) else True
            if val == Sym("nil") or val == []:
                set_eff = False
            keep = set_eff is not False
        else:
            clean.append(p)
    return clean, set_eff, keep


def rule(when, *, keep=True, set_eff=None, trace=None, stop=None):
    return {
        "when": when,
        "keep": keep,
        "set": set_eff,
        "trace": trace,
        "stop": stop,
    }


def build_rules(leaf_exprs):
    """Translate the flattened guard leaves into ordered Project rules."""
    or_leaf = next(
        (e for e in leaf_exprs
         if is_list(e) and head(e) == Sym("or") and has_latch_setf(e)),
        None,
    )
    if or_leaf is None:
        return [rule(tr_many(leaf_exprs))]
    common = [e for e in leaf_exprs if e is not or_leaf]
    rules = []
    for br in or_leaf[1:]:
        clean, set_eff, keep = split_branch(br)
        rules.append(rule(tr_many(common + clean), keep=keep, set_eff=set_eff))
    return rules


# ------------------------------------------------------------------- driver


def find_defun(forms, name):
    for f in forms:
        if head(f) == Sym("defun") and str(f[1]).lower() == name:
            return f
    return None


def join_and_butlast(run_it):
    """Find the `(model-output FMT ARG)` and read join/butlast."""
    found = []

    def walk(f):
        if is_list(f):
            if head(f) == Sym("model-output") and len(f) >= 2:
                found.append(f)
            for x in f[1:]:
                walk(x)

    walk(run_it)
    if not found:
        raise Unsupported("no model-output")
    f = found[-1]
    fmt = str(f[1])
    butlast = False
    if len(f) > 2:
        arg = f[2]
        butlast = is_list(arg) and head(arg) == Sym("butlast")
    join = "NewlineDots" if "..." in fmt else "Newline"
    return join, butlast


# --------------------------------------------------------------------- emit


def ident(name):
    s = re.sub(r"[^A-Za-z0-9]", "_", name)
    return s


def opt_bool(v):
    return "None" if v is None else ("Some(true)" if v else "Some(false)")


def opt_int(v):
    return "None" if v is None else f"Some({v})"


def rust_rule(r, indent):
    return (
        f"{indent}hooks::Rule {{\n"
        f"{indent}    when: {r['when']},\n"
        f"{indent}    keep: {'true' if r['keep'] else 'false'},\n"
        f"{indent}    set: {opt_bool(r['set'])},\n"
        f"{indent}    trace: {opt_bool(r['trace'])},\n"
        f"{indent}    stop_after: {opt_int(r['stop'])},\n"
        f"{indent}}},"
    )


def rust_phase(ph, indent):
    rules = "\n".join(rust_rule(r, indent + "        ") for r in ph["rules"])
    return (
        f"{indent}hooks::Phase {{\n"
        f"{indent}    on_enter_trace: {opt_bool(ph['on_enter_trace'])},\n"
        f"{indent}    rules: &[\n{rules}\n{indent}    ],\n"
        f"{indent}}},"
    )


def emit(name, driver, ir, ignore=None):
    ident_ = ident(name)
    const = "HOOK_" + ident_.upper()
    phases = "\n".join(rust_phase(p, "        ") for p in ir["phases"])
    attr = f'#[ignore = "{ignore}"]\n' if ignore else ""
    return f"""const {const}: hooks::HookIr = hooks::HookIr {{
    driver: "{driver}",
    phases: &[
{phases}
    ],
    output: hooks::OutputMode::{ir['output']},
    else_dots: {'true' if ir['else_dots'] else 'false'},
    butlast: {'true' if ir['butlast'] else 'false'},
    join: hooks::Join::{ir['join']},
}};

#[test]
{attr}fn hook_{ident_}() {{
    if let Err(e) = hooks::run_hook_case("{name}", &{const}) {{
        panic!("{{e}}");
    }}
}}
"""


def find_break_count(filt):
    """`(= (incf *count*) N)` in the filter."""
    found = []

    def walk(f):
        if is_list(f):
            if (
                head(f) == Sym("=")
                and len(f) == 3
                and is_list(f[1])
                and head(f[1]) == Sym("incf")
                and isinstance(f[2], int)
            ):
                found.append(f[2])
            for x in f[1:]:
                walk(x)

    walk(filt)
    return found[0] if found else None


def find_run_n_events(run_it):
    found = []

    def walk(f):
        if is_list(f):
            if head(f) == Sym("run-n-events") and len(f) == 2 and isinstance(f[1], int):
                found.append(f[1])
            for x in f[1:]:
                walk(x)

    walk(run_it)
    return found[0] if found else None


def convert_ctrl(forms, run_it):
    """The `pcomp-issues` control shape: `:v nil` + break-at-count on
    production-fired, then a second hook showing only production-fired for the
    `run-n-events` resume. Segment output."""
    filt = find_defun(forms, "filter")
    filt2 = find_defun(forms, "filter2")
    if not filt or not filt2:
        raise Unsupported("no filter/filter2 pair")
    n = find_break_count(filt)
    if n is None:
        raise Unsupported("no (= (incf …) N) break")
    k = find_run_n_events(run_it)
    if k is None:
        raise Unsupported("no run-n-events")
    return {
        "phases": [
            {
                "on_enter_trace": None,
                "rules": [
                    rule("hooks::Pred::MsEq(0)", trace=False),
                    rule('hooks::Pred::Action("production-fired")', stop=n),
                ],
            },
            {
                "on_enter_trace": True,
                "rules": [
                    rule(
                        'hooks::Pred::Not(&hooks::Pred::Action("production-fired"))',
                        trace=False,
                    )
                ],
            },
        ],
        "output": "Segment",
        "else_dots": False,
        "butlast": False,
        "join": "Newline",
    }


def convert(name, driver):
    src = (STUBS / f"{name}.lisp").read_text(encoding="utf-8", errors="replace")
    forms = parse(src)
    run_it = find_defun(forms, "run-it")
    if run_it is None:
        raise Unsupported("no (defun run-it …)")
    try:
        return convert_ctrl(forms, run_it)
    except Unsupported:
        pass
    filt = find_defun(forms, "filter")
    if filt is None:
        raise Unsupported("no (defun filter …)")
    guard_exprs, else_dots = collect(filt[3:])
    leaves = flatten_and(guard_exprs)
    rules = build_rules(leaves)
    join, butlast = join_and_butlast(run_it)
    return {
        "phases": [{"on_enter_trace": None, "rules": rules}],
        "output": "Project",
        "else_dots": else_dots,
        "butlast": butlast,
        "join": join,
    }


def rustfmt(path: Path):
    """Best-effort `rustfmt` so the generated file is readable; the file is
    `include!`d, so `cargo fmt` does not reach it."""
    try:
        subprocess.run(
            ["rustfmt", "--edition", "2021", str(path)],
            check=True,
            capture_output=True,
        )
    except (OSError, subprocess.CalledProcessError):
        print(f"note: rustfmt unavailable; {path.name} left unformatted", file=sys.stderr)


def main() -> int:
    check = "--check" in sys.argv[1:]
    manifest = []
    for line in MANIFEST.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split(None, 2)
        name, driver = parts[0], parts[1]
        ignore = parts[2] if len(parts) > 2 else None
        manifest.append((name, driver, ignore))

    header = (
        "// @generated by tools/gen_hook_ir.py — do not edit.\n"
        "// Event-hook cases (`.lisp` add-pre/post-event-hook) run through the IR\n"
        "// in tests/hooks/mod.rs; owner list fixtures/regression/hook-cases.txt.\n"
        "\n"
    )
    chunks = [header]
    converted = skipped = 0
    for name, driver, ignore in manifest:
        try:
            ir = convert(name, driver)
        except Unsupported as e:
            print(f"SKIP {name}: {e}", file=sys.stderr)
            skipped += 1
            continue
        chunks.append(emit(name, driver, ir, ignore))
        converted += 1

    text = "\n".join(chunks)
    if check:
        import tempfile

        with tempfile.TemporaryDirectory() as d:
            tmp = Path(d) / "hook_cases.rs"
            tmp.write_text(text, encoding="utf-8")
            rustfmt(tmp)
            produced = tmp.read_text(encoding="utf-8")
        current = OUT.read_text(encoding="utf-8") if OUT.exists() else ""
        if current != produced:
            print(f"DRIFT: {OUT} is out of date; run tools/gen_hook_ir.py", file=sys.stderr)
            return 1
        print(f"{OUT} up to date: {converted} converted, {skipped} skipped")
        return 0

    OUT.write_text(text, encoding="utf-8")
    rustfmt(OUT)
    print(f"wrote {OUT}: {converted} converted, {skipped} skipped")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Port the ACT-R tutorial models and task scripts to self-contained modules.

Each generated module contains a ``build()`` function plus, when the model has
a vendored task script, that task's code.  The modules are checked in and can
be edited directly; re-run this script after changing a vendored source.

Usage:
    python tools/port_tutorial.py [--actr DIR]
"""

from __future__ import annotations

import argparse
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_ACTR = ROOT.parents[1] / "actr7.x"
OUT = ROOT / "tutorial"


# ---------------------------------------------------------------- reader
class Sym(str):
    pass


class Str(str):
    pass


def parse(text: str):
    tokens = _tokenize(text)
    pos = 0

    def read():
        nonlocal pos
        token = tokens[pos]
        pos += 1
        if token == "(":
            out = []
            while tokens[pos] != ")":
                out.append(read())
            pos += 1
            return out
        if token == ")":
            raise SyntaxError("unexpected )")
        if token in ("QUOTE", "BACKQUOTE"):
            return [Sym(token), read()]
        if token.startswith('"'):
            return Str(_unescape(token[1:-1]))
        if re.fullmatch(r"[+-]?\d+", token):
            return int(token)
        if re.fullmatch(r"[+-]?(?:\d+\.\d*|\.\d+)(?:[eE][+-]?\d+)?", token):
            return float(token)
        return Sym(token.upper())

    return read()


def parse_all(text: str):
    tokens = _tokenize(text)
    forms = []
    pos = 0

    def read():
        nonlocal pos
        token = tokens[pos]
        pos += 1
        if token == "(":
            out = []
            while tokens[pos] != ")":
                out.append(read())
            pos += 1
            return out
        if token == ")":
            raise SyntaxError("unexpected )")
        if token in ("QUOTE", "BACKQUOTE"):
            return [Sym(token), read()]
        if token.startswith('"'):
            return Str(_unescape(token[1:-1]))
        if re.fullmatch(r"[+-]?\d+", token):
            return int(token)
        if re.fullmatch(r"[+-]?(?:\d+\.\d*|\.\d+)(?:[eE][+-]?\d+)?", token):
            return float(token)
        return Sym(token.upper())

    while pos < len(tokens):
        forms.append(read())
    return forms


def _tokenize(text: str):
    out = []
    i = 0
    while i < len(text):
        c = text[i]
        if c in " \t\r\n":
            i += 1
        elif c == ";":
            while i < len(text) and text[i] != "\n":
                i += 1
        elif c in "()":
            out.append(c)
            i += 1
        elif c == '"':
            j = i + 1
            while j < len(text):
                if text[j] == "\\":
                    j += 2
                elif text[j] == '"':
                    break
                else:
                    j += 1
            out.append(text[i : j + 1])
            i = j + 1
        elif c == "'":
            out.append("QUOTE")
            i += 1
        elif c == "`":
            out.append("BACKQUOTE")
            i += 1
        else:
            j = i
            while j < len(text) and text[j] not in " \t\r\n();":
                j += 1
            out.append(text[i:j])
            i = j
    return out


def _unescape(text: str) -> str:
    return (
        text.replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace('\\"', '"')
        .replace("\\\\", "\\")
    )


# ---------------------------------------------------------------- writer
def lisp(value) -> str:
    if isinstance(value, Str):
        escaped = (
            value.replace("\\", "\\\\")
            .replace('"', '\\"')
            .replace("\n", "\\n")
            .replace("\t", "\\t")
        )
        return f'"{escaped}"'
    if isinstance(value, Sym):
        return str(value)
    if isinstance(value, bool):
        return "t" if value else "nil"
    if isinstance(value, list):
        return "(" + " ".join(lisp(x) for x in value) + ")"
    return str(value)


def py(value):
    if isinstance(value, Str):
        return f'actr.String({str(value)!r})'
    if isinstance(value, Sym):
        if value == "NIL":
            return "None"
        if value == "T":
            return "True"
        return repr(str(value))
    if isinstance(value, list):
        return "[" + ", ".join(py(x) for x in value) + "]"
    return repr(value)


def kw(name: str) -> str:
    return name.replace("-", "_")


def emit_form(form, lines: list[str]) -> None:
    if not isinstance(form, list) or not form:
        return
    head = str(form[0]).upper() if isinstance(form[0], Sym) else ""
    if head == "SGP":
        pairs = form[1:]
        for i in range(0, len(pairs) - 1, 2):
            lines.append(f"    m.sgp({str(pairs[i])!r}, {py(pairs[i + 1])})")
    elif head == "CHUNK-TYPE":
        _emit_chunk_type(form, lines)
    elif head == "ADD-DM":
        for entry in form[1:]:
            _emit_dm(entry, lines, define=False)
    elif head in ("DEFINE-CHUNKS", "DEFINE-CHUNKS-FCT"):
        for entry in form[1:]:
            _emit_dm(entry, lines, define=True)
    elif head in ("GOAL-FOCUS", "GOAL-FOCUS-FCT"):
        _emit_goal_focus(form[1], lines)
    elif head in ("P", "P-FCT"):
        source = lisp(form)
        lines.append("    m.add_production(")
        lines.append("        '''" + source.replace("'''", "\\'\\'\\'") + "'''")
        lines.append("    )")
    elif head == "SPP":
        name = str(form[1])
        pairs = []
        i = 2
        while i + 1 < len(form):
            key = kw(str(form[i]).lstrip(":"))
            pairs.append(f"{key!r}: {py(form[i + 1])}")
            i += 2
        lines.append(f"    m.spp({name!r}, **{{{', '.join(pairs)}}})")
    elif head == "SET-SIMILARITIES":
        entries = [f"({py(e[0])}, {py(e[1])}, {py(e[2])})" for e in form[1:]]
        lines.append(f"    m.set_similarities([{', '.join(entries)}])")
    elif head == "SET-BASE-LEVELS":
        entries = ["(" + ", ".join(py(x) for x in e) + ")" for e in form[1:]]
        lines.append(f"    m.set_base_levels([{', '.join(entries)}])")
    elif head == "SET-ALL-BASE-LEVELS":
        args = ", ".join(py(x) for x in form[1:])
        lines.append(f"    m.set_all_base_levels({args})")
    elif head == "DECLARE-BUFFER-USAGE":
        args = ", ".join(py(x) for x in form[1:])
        lines.append(f"    m.declare_buffer_usage({args})")
    elif head == "SET-VISLOC-DEFAULT":
        slots = []
        for i in range(1, len(form) - 1, 2):
            slots.append(f"{kw(str(form[i]).lstrip(':'))}={py(form[i + 1])}")
        lines.append(f"    m.set_visloc_default({', '.join(slots)})")
    elif head == "CALL-ACT-R-COMMAND":
        name = str(form[1]) if len(form) > 1 else ""
        args = ", ".join(py(x) for x in form[2:])
        suffix = f", {args}" if args else ""
        lines.append(f"    m.call_act_r_command({name!r}{suffix})")
    elif head == "SET-BUFFER-CHUNK":
        _emit_initial_buffer(form, lines)
    elif head == "INSTALL-DEVICE":
        _emit_install_device(form, lines)
    else:
        source = lisp(form)
        lines.append(f"    m.form({source!r})")


def _emit_chunk_type(form, lines: list[str]) -> None:
    if len(form) < 2:
        return
    name = form[1]
    if isinstance(name, list):
        # (chunk-type (name (:include parent)) slot ...)
        name = name[0]
    slots = []
    defaults = {}
    includes = []
    for slot in form[2:]:
        if isinstance(slot, list):
            if slot and str(slot[0]).upper() == ":INCLUDE":
                includes.append(py(slot[1]))
            elif len(slot) >= 2:
                slots.append(str(slot[0]))
                defaults[kw(str(slot[0]))] = slot[1]
        else:
            slots.append(str(slot))
    parts = [repr(str(name))] + [repr(str(s)) for s in slots]
    if defaults:
        pairs = ", ".join(f"{repr(k)}: {py(v)}" for k, v in defaults.items())
        parts.append(f"defaults={{{pairs}}}")
    if includes:
        parts.append(f"include={includes[0] if len(includes) == 1 else '[' + ', '.join(includes) + ']'}")
    lines.append(f"    m.chunk_type({', '.join(parts)})")


def _emit_dm(entry, lines: list[str], define: bool) -> None:
    method = "define_chunks" if define else "add_dm"
    if isinstance(entry, Sym):
        lines.append(f"    m.{method}({str(entry)!r})")
        return
    if not isinstance(entry, list):
        lines.append(f"    m.form({lisp(entry)!r})")
        return
    name = entry[0]
    if len(entry) == 1:
        lines.append(f"    m.{method}({str(name)!r})")
        return
    isa = None
    rest = entry[1:]
    if len(rest) >= 2 and isinstance(rest[0], Sym) and str(rest[0]).upper() == "ISA":
        isa = str(rest[1])
        rest = rest[2:]
    if not isinstance(name, Sym) or len(rest) % 2:
        lines.append(f"    m.form({lisp(entry)!r})")
        return
    kwargs = []
    for i in range(0, len(rest), 2):
        kwargs.append(f"{repr(str(rest[i]))}: {py(rest[i + 1])}")
    if define:
        ctor = f"actr.Chunk({str(name)!r}, {isa!r}, {{{', '.join(kwargs)}}})"
        lines.append(f"    m.define_chunks({ctor})")
    elif isa:
        lines.append(f"    m.add_chunk({str(name)!r}, {isa!r}, **{{{', '.join(kwargs)}}})")
    else:
        lines.append(f"    m.add_chunk({str(name)!r}, **{{{', '.join(kwargs)}}})")


def _emit_goal_focus(arg, lines: list[str]) -> None:
    if isinstance(arg, Sym):
        lines.append(f"    m.goal_focus({str(arg)!r})")
    elif isinstance(arg, list) and arg and isinstance(arg[0], Sym) and str(arg[0]).upper() == "ISA":
        typ = str(arg[1])
        slots = []
        for i in range(2, len(arg) - 1, 2):
            slots.append(f"{repr(str(arg[i]))}: {py(arg[i + 1])}")
        lines.append(f"    m.goal_focus(isa={typ!r}, **{{{', '.join(slots)}}})")
    else:
        lines.append(f"    m.form({lisp(['GOAL-FOCUS', arg])!r})")


def _emit_initial_buffer(form, lines: list[str]) -> None:
    buffer = form[1]
    chunk = form[2]
    if isinstance(chunk, Sym):
        lines.append(f"    m.set_initial_buffer({str(buffer)!r}, {str(chunk)!r})")
    elif isinstance(chunk, list) and chunk and isinstance(chunk[0], Sym) and str(chunk[0]).upper() == "ISA":
        typ = str(chunk[1])
        slots = []
        for i in range(2, len(chunk) - 1, 2):
            slots.append(f"{repr(str(chunk[i]))}: {py(chunk[i + 1])}")
        lines.append(f"    m.set_initial_buffer({str(buffer)!r}, isa={typ!r}, **{{{', '.join(slots)}}})")
    else:
        lines.append(f"    m.form({lisp(form)!r})")


def _emit_install_device(form, lines: list[str]) -> None:
    arg = form[1] if len(form) > 1 else None
    if isinstance(arg, list) and arg and isinstance(arg[0], Sym) and str(arg[0]).upper() == "OPEN-EXP-WINDOW":
        title = arg[1] if len(arg) > 1 else Sym("WINDOW")
        opts = []
        i = 2
        while i + 1 < len(arg):
            key = str(arg[i]).lstrip(":").replace("-", "_")
            opts.append(f"{key}={py(arg[i + 1])}")
            i += 2
        lines.append(f"    m.install_device({str(title)!r}{', ' + ', '.join(opts) if opts else ''})")
    else:
        lines.append(f"    m.form({lisp(form)!r})")


TASK_DEFAULTS: dict[str, tuple[str | None, list]] = {
    "demo2": ("experiment", []),
    "unit2": ("experiment", []),
    "fan": ("experiment", []),
    "fan_no_pm": ("experiment", []),
    "subitize": ("experiment", []),
    "zbrodoff": ("experiment", []),
    "pm_issues": ("task", []),
    "sperling": ("experiment", [1]),
    "paired": ("experiment", [1]),
    "siegler": ("experiment", [1]),
    "bst": ("experiment", [1]),
    "bst_ppm": ("experiment", [1]),
    "categorize": ("experiment", [1]),
    "onehit": ("hands", [2]),
    "grouped": ("recall", []),
    "past_tense": ("trials", [1]),
    "pcomp_issues": ("game", [1]),
    "choice": (None, []),
    "ul_issues": (None, []),
}

LOAD_MODEL_RE = re.compile(
    r"""actr\.load_act_r_model\s*\(\s*["']([^"']+)["']\s*\)"""
)
IMPORT_ACTR_RE = re.compile(r"(?m)^import actr[ \t]*$\n?")
LOAD_LINE_RE = re.compile(
    r"(?m)^([ \t]*)actr\.load_act_r_model\s*\([^)]*\)[ \t]*$"
)


def model_stem_name(logical: str) -> str:
    parts = logical.replace("ACT-R:", "").replace(";", "/").replace(".lisp", "")
    return parts.rsplit("/", 1)[-1].replace("-", "_")


def read_define_model(path: pathlib.Path):
    text = path.read_text(encoding="utf-8", errors="replace")
    forms = parse_all(text)
    define = None
    for form in forms:
        if (
            isinstance(form, list)
            and form
            and isinstance(form[0], Sym)
            and str(form[0]).upper() in ("DEFINE-MODEL", "DEFINE-MODEL-FCT")
        ):
            define = form
    if define is None:
        return None
    return str(define[1]), define[2:]


def task_model_logical(path: pathlib.Path) -> str | None:
    text = path.read_text(encoding="utf-8", errors="replace")
    match = LOAD_MODEL_RE.search(text)
    return match.group(1) if match else None


def build_lines(model_name: str, body) -> list[str]:
    lines = [
        "def build(seed=(100, 0)) -> actr.Model:",
        f"    m = actr.Model({model_name!r}, seed=seed)",
    ]
    for form in body:
        emit_form(form, lines)
    lines.append("    return m")
    return lines


def model_only_module(logical: str, model_name: str, body) -> str:
    lines = [
        f'"""Python port of {logical}."""',
        "",
        "from __future__ import annotations",
        "",
        "import rust_actr.actr as actr",
        "",
        f"LOGICAL_PATH = {logical!r}",
        f"MODEL_NAME = {model_name!r}",
        "DEFAULT_RUN_TIME = 10.0",
        "",
        "",
    ]
    lines.extend(build_lines(model_name, body))
    lines.extend(
        [
            "",
            "",
            "def main(argv=None) -> None:",
            "    import sys",
            "",
            "    argv = list(sys.argv[1:] if argv is None else argv)",
            "    seconds = float(argv[0]) if argv else DEFAULT_RUN_TIME",
            '    print(build().run(seconds), end="")',
            "",
            "",
            'if __name__ == "__main__":',
            "    main()",
            "",
        ]
    )
    return "\n".join(lines)


def task_module(
    logical: str,
    model_name: str,
    body,
    task_path: pathlib.Path,
    function: str | None,
    arguments: list,
) -> str:
    source = task_path.read_text(encoding="utf-8", errors="replace")
    source = IMPORT_ACTR_RE.sub("", source, count=1)
    source, replaced = LOAD_LINE_RE.subn(
        r"\1MODEL = actr.use(build())", source
    )
    if replaced != 1:
        raise RuntimeError(
            f"{task_path}: expected one load_act_r_model line, found {replaced}"
        )

    lines = [
        f'"""Python port of {logical} plus the {task_path.stem} task."""',
        "",
        "from __future__ import annotations",
        "",
        "import json",
        "import sys",
        "",
        "import rust_actr.actr as actr",
        "",
        f"LOGICAL_PATH = {logical!r}",
        f"MODEL_NAME = {model_name!r}",
        f"DEFAULT_FUNCTION = {function!r}",
        f"DEFAULT_ARGS = {arguments!r}",
        "",
        "",
    ]
    lines.extend(build_lines(model_name, body))
    lines.extend(
        [
            "",
            "",
            "actr.clear_state()",
            "",
            source.rstrip(),
            "",
            "",
            "def _value(text):",
            "    try:",
            "        return json.loads(text)",
            "    except json.JSONDecodeError:",
            "        return text",
            "",
            "",
            "def main(argv=None):",
            "    argv = list(sys.argv[1:] if argv is None else argv)",
            "    if argv:",
            "        name = argv[0]",
            "        if name not in globals():",
            "            public = sorted(",
            "                key",
            "                for key, value in globals().items()",
            "                if callable(value) and not key.startswith('_')",
            "            )",
            "            raise SystemExit(",
            "                f'unknown function {name!r}; available: {public}'",
            "            )",
            "        function = globals()[name]",
            "        arguments = [_value(arg) for arg in argv[1:]]",
            "    elif DEFAULT_FUNCTION is not None:",
            "        function = globals()[DEFAULT_FUNCTION]",
            "        arguments = list(DEFAULT_ARGS)",
            "    else:",
            "        print(MODEL.run(10.0))",
            "        return None",
            "    result = function(*arguments)",
            "    if result is not None:",
            "        print(result)",
            "    return result",
            "",
            "",
            'if __name__ == "__main__":',
            "    main()",
            "",
        ]
    )
    return "\n".join(lines)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--actr", default=str(DEFAULT_ACTR))
    args = ap.parse_args()
    root = pathlib.Path(args.actr)

    models: dict[str, tuple[pathlib.Path, str, list]] = {}
    for path in sorted(root.glob("tutorial/unit*/*.lisp")):
        if path.name == "broken-addition.lisp":
            continue
        rel = path.relative_to(root).as_posix()
        logical = "ACT-R:" + rel.replace("/", ";")
        parsed = read_define_model(path)
        if parsed is None:
            print(f"skip {logical}: no define-model")
            continue
        name, body = parsed
        models[logical.upper()] = (path, name, body)

    tasks: dict[str, pathlib.Path] = {}
    for path in sorted((root / "tutorial" / "python").glob("*.py")):
        if path.name in ("actr.py", "jenvstarter.py"):
            continue
        logical = task_model_logical(path)
        if logical is None:
            print(f"skip task {path.name}: no load_act_r_model")
            continue
        key = logical.upper()
        if key not in models:
            raise SystemExit(f"{path.name}: model {logical} is not under tutorial/unit*")
        if key in tasks:
            raise SystemExit(
                f"{path.name}: {logical} already has task {tasks[key].name}"
            )
        tasks[key] = path

    written: list[pathlib.Path] = []
    for key, (model_path, name, body) in models.items():
        rel = model_path.relative_to(root).as_posix()
        unit = rel.split("/")[1]
        logical = "ACT-R:" + rel.replace("/", ";")
        task = tasks.get(key)
        if task is not None:
            function, arguments = TASK_DEFAULTS.get(task.stem, (None, []))
            target = OUT / unit / f"{task.stem}.py"
            source = task_module(logical, name, body, task, function, arguments)
        else:
            target = OUT / unit / f"{model_stem_name(logical)}.py"
            source = model_only_module(logical, name, body)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(source, encoding="utf-8")
        written.append(target)

    print(f"wrote {len(written)} modules to {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

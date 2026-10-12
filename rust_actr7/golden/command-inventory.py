#!/usr/bin/env python3
"""Build the drop-in command inventory.

The target surface is the union of:

  * the commands the Python client wrapper (`actr7.x/tutorial/python/actr.py`)
    can call, and
  * the commands the `actr7.x/test-models/*.lisp` stubs exercise,

minus the commands the Rust RPC registry already implements.

Writes `golden/command-inventory.txt` (name<TAB>family<TAB>needed-by).
Re-run after adding commands; the file is the work list for Phase R7.
"""
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
RUST = os.path.dirname(HERE)
ACTR = os.environ.get("ACTR_DIR", os.path.normpath(os.path.join(RUST, "..", "actr7.x")))
OUT = os.path.join(HERE, "command-inventory.txt")


def client_commands():
    src = open(os.path.join(ACTR, "tutorial", "python", "actr.py")).read()
    return set(re.findall(r'evaluate(?:_single)?\("([^"]+)"', src))


def ref_commands():
    """Commands named as a call `(cmd ...)` in any test-model stub."""
    tmdir = os.path.join(ACTR, "test-models")
    blobs = []
    for f in os.listdir(tmdir):
        if f.endswith(".lisp"):
            blobs.append(open(os.path.join(tmdir, f), errors="replace").read())
    blob = "\n".join(blobs)
    # the core command table (framework/core-modules/modules/devices)
    core = []
    for sub in ("framework", "core-modules", "modules", "devices"):
        d = os.path.join(ACTR, sub)
        for root, _, files in os.walk(d):
            for f in files:
                if f.endswith(".lisp"):
                    core.append(open(os.path.join(root, f), errors="replace").read())
    core.append(open(os.path.join(ACTR, "load-act-r.lisp"), errors="replace").read())
    names = set(re.findall(r'\((?:add|define)-act-r-command +"([^"]+)"', "\n".join(core)))
    used = set()
    for n in names:
        if re.search(r"\(" + re.escape(n) + r"(?:[ )]|$)", blob):
            used.add(n)
    return used


def implemented():
    """Every quoted lowercase token appearing in the rpc/engine sources."""
    toks = set()
    for d in ("rpc", "engine"):
        base = os.path.join(RUST, "src", d)
        for root, _, files in os.walk(base):
            for f in files:
                if f.endswith(".rs"):
                    src = open(os.path.join(root, f)).read()
                    toks |= set(re.findall(r'"([a-z][a-z0-9!?<>=*/+-]*)"', src))
    for f in ("lib.rs", "model.rs"):
        src = open(os.path.join(RUST, "src", f)).read()
        toks |= set(re.findall(r'"([a-z][a-z0-9!?<>=*/+-]*)"', src))
    return toks


FAMILIES = [
    ("pipe", ["pipe"]),
    ("modules", ["define-module", "undefine-module", "get-module", "modules-",
                 "sorted-module-names", "printed-parameter-details"]),
    ("history", ["history", "activation-trace", "trigger-reward"]),
    ("audio", ["sound", "audicon", "articulation", "speech"]),
    ("window", ["exp-window", "visicon", "finsts", "attend-visual",
                "visual-coordinates", "visual-center"]),
    ("events", ["evt-", "event-hook", "schedule-", "format-event",
                "module-request", "mod-buffer-chunk", "overwrite-buffer-chunk",
                "delete-event"]),
    ("production", ["pdisable", "penable", "pbreak", "punbreak", "pp",
                    "spp", "sdp", "production-text", "all-productions",
                    "used-production-buffers", "mp-queue", "mp-models",
                    "mp-show", "simulate-retrieval", "run-until-condition"]),
    ("chunk-dm", ["chunk", "dm", "buffers", "buffer-slot-value", "mod-chunk",
                  "merge-dm", "extend-possible-slots", "purge-chunk",
                  "make-chunk", "sji", "pprint"]),
]


def family(name):
    for fam, keys in FAMILIES:
        if any(name == k or name.startswith(k) for k in keys):
            return fam
    return "misc"


def main():
    cli = client_commands()
    ref = ref_commands()
    impl = implemented()
    target = cli | ref
    gap = sorted(target - impl)
    with open(OUT, "w") as f:
        f.write("# Drop-in command inventory: commands a Python ACT-R client or a\n")
        f.write("# test-model stub can reach, minus those the Rust RPC registry\n")
        f.write("# already implements.  Regenerate with golden/command-inventory.py.\n")
        f.write("# format: name<TAB>family<TAB>needed-by\n")
        for c in gap:
            why = []
            if c in cli:
                why.append("actr.py")
            if c in ref:
                why.append("refs")
            f.write(f"{c}\t{family(c)}\t{','.join(why)}\n")
    by_fam = {}
    for c in gap:
        by_fam.setdefault(family(c), []).append(c)
    print(f"target={len(target)} implemented={len(target & impl)} gap={len(gap)}")
    for fam in sorted(by_fam):
        print(f"  {fam:12s} {len(by_fam[fam]):3d}")


if __name__ == "__main__":
    main()

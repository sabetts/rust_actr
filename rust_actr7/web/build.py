#!/usr/bin/env python3
import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

WEB_DIR = Path(__file__).resolve().parent
CRATE_DIR = WEB_DIR.parent
REPO_ROOT = CRATE_DIR.parent
WASM_TARGET = "wasm32-unknown-unknown"
WASM_BUILD = CRATE_DIR / "target" / WASM_TARGET / "release" / "actr_engine.wasm"
WASM_STAGED = WEB_DIR / "actr_engine.wasm"
MODELS_DIR = WEB_DIR / "models"

MODEL_GROUPS = (
    {
        "id": "unit1",
        "label": "Unit 1",
        "source": REPO_ROOT / "actr7.x" / "tutorial" / "unit1",
        "models": (
            ("count", "Count", "count.lisp", None),
            ("addition", "Addition", "addition.lisp", None),
            ("semantic", "Semantic", "semantic.lisp", None),
            ("tutor-model", "Tutor Model (starter)", "tutor-model.lisp", None),
            ("tutor-model-solution", "Tutor Model (solution)", "tutor-model-solution.lisp", None),
            ("broken-addition", "Broken Addition (intentionally broken)", "broken-addition.lisp", None),
        ),
    },
    {
        "id": "unit2",
        "label": "Unit 2",
        "source": REPO_ROOT / "actr7.x" / "tutorial" / "unit2",
        "models": (
            ("demo2", "Demo2", "demo2-model.lisp", "unit2-demo2"),
            (
                "unit2-assignment",
                "Unit 2 Assignment (starter)",
                "unit2-assignment-model.lisp",
                "unit2-assignment",
            ),
        ),
    },
)


def stage_models() -> None:
    for group in MODEL_GROUPS:
        for _, _, filename, _ in group["models"]:
            source = group["source"] / filename
            if not source.is_file():
                raise SystemExit(f"missing model source: {source}")

    if MODELS_DIR.exists():
        shutil.rmtree(MODELS_DIR)
    MODELS_DIR.mkdir(parents=True)

    groups = []
    for group in MODEL_GROUPS:
        group_dir = MODELS_DIR / group["id"]
        group_dir.mkdir()
        models = []
        for model_id, label, filename, task in group["models"]:
            shutil.copyfile(group["source"] / filename, group_dir / filename)
            entry = {
                "id": f'{group["id"]}-{model_id}',
                "label": label,
                "path": f'{group["id"]}/{filename}',
            }
            if task is not None:
                entry["task"] = task
            models.append(entry)
        groups.append({"id": group["id"], "label": group["label"], "models": models})

    manifest = {"default": groups[0]["models"][0]["id"], "groups": groups}
    manifest_path = MODELS_DIR / "manifest.json"
    manifest_path.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )

    count = sum(len(group["models"]) for group in MODEL_GROUPS)
    print(f"staged {count} models in {MODELS_DIR}")


def build_wasm() -> int:
    command = [
        "cargo",
        "build",
        "--manifest-path",
        str(CRATE_DIR / "Cargo.toml"),
        "--release",
        "--target",
        WASM_TARGET,
    ]
    try:
        subprocess.run(command, cwd=REPO_ROOT, check=True)
    except FileNotFoundError:
        print("cargo not found; install Rust or add it to PATH", file=sys.stderr)
        return 1
    except subprocess.CalledProcessError as error:
        print(f"cargo build failed (exit {error.returncode})", file=sys.stderr)
        print(f"install the wasm target with: rustup target add {WASM_TARGET}", file=sys.stderr)
        return error.returncode or 1

    if not WASM_BUILD.is_file():
        print(f"missing built wasm: {WASM_BUILD}", file=sys.stderr)
        return 1

    shutil.copyfile(WASM_BUILD, WASM_STAGED)
    print(f"staged {WASM_STAGED}")
    return 0


def print_serve() -> None:
    print("serve the page over HTTP:")
    print(f'  cd "{WEB_DIR}"')
    print("  python -m http.server 8000")
    print("  open http://localhost:8000/")


def main() -> int:
    parser = argparse.ArgumentParser(description="Build and stage the browser ACT-R page.")
    parser.add_argument(
        "--models-only",
        action="store_true",
        help="stage the model files and manifest without building wasm",
    )
    args = parser.parse_args()

    stage_models()
    if not args.models_only:
        status = build_wasm()
        if status != 0:
            return status
    elif not WASM_STAGED.is_file():
        print(
            f"note: {WASM_STAGED.name} is not staged; run without --models-only to build it",
            file=sys.stderr,
        )

    print_serve()
    return 0


if __name__ == "__main__":
    sys.exit(main())

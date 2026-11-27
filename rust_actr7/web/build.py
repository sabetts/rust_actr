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
ASSETS_DIR = WEB_DIR / "assets"
HEAD_ICON = CRATE_DIR / "src" / "data" / "actr-head.png"
HEAD_ICON_STAGED = ASSETS_DIR / "actr-head.png"
AGI_IMAGES_SRC = REPO_ROOT / "actr7.x" / "environment" / "gui" / "AGI-images"
AGI_IMAGES_DIR = WEB_DIR / "AGI-images"

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
    {
        "id": "unit3",
        "label": "Unit 3",
        "source": REPO_ROOT / "actr7.x" / "tutorial" / "unit3",
        "models": (
            ("sperling", "Sperling", "sperling-model.lisp", "sperling"),
            (
                "perceptual-motor-issues",
                "Perceptual-Motor Issues",
                "perceptual-motor-issues-model.lisp",
                "pm-issues",
            ),
            ("subitize", "Subitize (starter)", "subitize-model.lisp", "subitize"),
        ),
    },
    {
        "id": "unit4",
        "label": "Unit 4",
        "source": REPO_ROOT / "actr7.x" / "tutorial" / "unit4",
        "models": (
            ("paired", "Paired Associate", "paired-model.lisp", "paired"),
            (
                "zbrodoff",
                "Alpha-Arithmetic (starter)",
                "zbrodoff-model.lisp",
                "zbrodoff",
            ),
        ),
    },
    {
        "id": "unit5",
        "label": "Unit 5",
        "source": REPO_ROOT / "actr7.x" / "tutorial" / "unit5",
        "models": (
            ("fan", "Fan Effect", "fan-model.lisp", "fan"),
            ("fan-no-pm", "Fan Effect (no PM)", "fan-no-pm-model.lisp", "fan-no-pm"),
            ("grouped", "Grouped Recall", "grouped-model.lisp", "grouped"),
            ("siegler", "Siegler Addition", "siegler-model.lisp", "siegler"),
            (
                "onehit",
                "1-Hit Blackjack (starter)",
                "1hit-blackjack-model.lisp",
                "onehit",
            ),
            ("declarative-issues", "Declarative Issues", "declarative-issues.lisp", None),
        ),
    },
    {
        "id": "unit6",
        "label": "Unit 6",
        "source": REPO_ROOT / "actr7.x" / "tutorial" / "unit6",
        "models": (
            ("bst", "Building Sticks", "bst-model.lisp", "bst"),
            ("choice", "Choice (starter)", "choice-model.lisp", "choice"),
        ),
    },
    {
        "id": "unit7",
        "label": "Unit 7",
        "source": REPO_ROOT / "actr7.x" / "tutorial" / "unit7",
        "models": (
            (
                "paired-learning",
                "Paired Associate Learning",
                "paired-learning-model.lisp",
                "paired",
            ),
            ("past-tense", "Past Tense (starter)", "past-tense-model.lisp", "past-tense"),
            (
                "utility-learning-issues",
                "Utility Learning Issues",
                "utility-learning-issues-model.lisp",
                "ul-issues",
            ),
            (
                "production-compilation-issues",
                "Production Compilation Issues (starter)",
                "production-compilation-issues-model.lisp",
                "pcomp-issues",
            ),
        ),
    },
    {
        "id": "unit8",
        "label": "Unit 8",
        "source": REPO_ROOT / "actr7.x" / "tutorial" / "unit8",
        "models": (
            ("simple-ppm", "Simple PPM", "simple-ppm-model.lisp", None),
            (
                "bst-ppm",
                "Building Sticks (PPM)",
                "bst-ppm-model.lisp",
                "bst-ppm",
            ),
            ("simple-dynamic", "Simple Dynamic", "simple-dynamic-model.lisp", None),
            (
                "semantic-dynamic",
                "Semantic Network (Dynamic)",
                "semantic-dynamic-model.lisp",
                "semantic-dynamic",
            ),
            (
                "paired-dynamic",
                "Paired Associate (Dynamic)",
                "paired-dynamic-model.lisp",
                "paired",
            ),
            ("categorize", "Categorize (starter)", "categorize-model.lisp", "categorize"),
        ),
    },
)


def stage_assets() -> None:
    if not HEAD_ICON.is_file():
        raise SystemExit(f"missing icon source: {HEAD_ICON}")
    ASSETS_DIR.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(HEAD_ICON, HEAD_ICON_STAGED)
    print(f"staged {HEAD_ICON_STAGED}")

    if not AGI_IMAGES_SRC.is_dir():
        raise SystemExit(f"missing AGI image source: {AGI_IMAGES_SRC}")
    if AGI_IMAGES_DIR.exists():
        shutil.rmtree(AGI_IMAGES_DIR)
    shutil.copytree(AGI_IMAGES_SRC, AGI_IMAGES_DIR)
    print(f"staged {AGI_IMAGES_DIR}")


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

    manifest = {"groups": groups}
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

    stage_assets()
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

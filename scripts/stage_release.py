#!/usr/bin/env python3
"""Stage one self-contained ACT-R release bundle.

This script is shared by the GitHub Actions release workflow and local builds.
It copies the platform binary, wheel, tutorial scripts, examples, clients, and
licenses into a single directory and optionally zips it.

Examples:
    python scripts/stage_release.py --platform windows-x86_64 \
        --binary rust_actr7/target/release/actr.exe \
        --wheel dist/wheels/rust_actr-0.1.0-cp39-abi3-win_amd64.whl \
        --zip

    python scripts/stage_release.py --platform linux-x86_64 \
        --binary rust_actr7/target/release/actr \
        --wheel dist/wheels/rust_actr-0.1.0-cp39-abi3-manylinux_2_17_x86_64.whl \
        --dist dist --zip
"""

from __future__ import annotations

import argparse
import re
import shutil
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION_RE = re.compile(r'^version\s*=\s*"([^"]+)"', re.MULTILINE)
PLATFORM_RE = re.compile(r"^[A-Za-z0-9._-]+$")


def fail(message: str) -> None:
    raise SystemExit(f"stage_release: {message}")


def read_version(path: Path) -> str:
    text = path.read_text(encoding="utf-8")
    match = VERSION_RE.search(text)
    if not match:
        fail(f"could not find a version in {path}")
    return match.group(1)


def project_version() -> str:
    manifests = {
        "crate": ROOT / "rust_actr7" / "Cargo.toml",
        "python crate": ROOT / "rust_actr7" / "python" / "Cargo.toml",
        "wheel": ROOT / "rust_actr7" / "python" / "pyproject.toml",
    }
    versions = {name: read_version(path) for name, path in manifests.items()}
    unique = set(versions.values())
    if len(unique) != 1:
        details = ", ".join(f"{name}={version}" for name, version in versions.items())
        fail(f"version mismatch: {details}")
    return unique.pop()


def copy_file(src: Path, dest: Path, executable: bool = False) -> None:
    if not src.is_file():
        fail(f"missing file: {src}")
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dest)
    if executable:
        dest.chmod(dest.stat().st_mode | 0o111)


def copy_tutorial(dest: Path) -> int:
    src = ROOT / "rust_actr7" / "python" / "tutorial"
    if not src.is_dir():
        fail(f"missing tutorial directory: {src}")

    count = 0
    for path in sorted(src.rglob("*.py")):
        if "__pycache__" in path.parts:
            continue
        copy_file(path, dest / path.relative_to(src))
        count += 1
    if count == 0:
        fail(f"no tutorial scripts found under {src}")
    return count


def copy_licenses(dest: Path) -> None:
    src = ROOT / "rust_actr7" / "python" / "LICENSES"
    if not src.is_dir():
        fail(f"missing licenses directory: {src}")

    copied = 0
    for path in sorted(src.iterdir()):
        if path.is_file():
            copy_file(path, dest / path.name)
            copied += 1
    if copied == 0:
        fail(f"no license files found under {src}")


def copy_optional(src: Path, dest: Path) -> bool:
    if not src.is_file():
        return False
    copy_file(src, dest)
    return True


def make_zip(bundle: Path) -> Path:
    zip_path = bundle.parent / f"{bundle.name}.zip"
    if zip_path.exists():
        zip_path.unlink()
    with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(bundle.rglob("*")):
            if not path.is_file():
                continue
            arcname = Path(bundle.name) / path.relative_to(bundle)
            archive.write(path, arcname.as_posix())
    return zip_path


def stage(args: argparse.Namespace) -> None:
    if not PLATFORM_RE.match(args.platform):
        fail(f"invalid platform name: {args.platform!r}")

    version = args.version or project_version()
    binary = args.binary.resolve()
    wheel = args.wheel.resolve()
    dist = args.dist.resolve()

    if not binary.is_file():
        fail(f"missing binary: {binary}")
    if not wheel.is_file():
        fail(f"missing wheel: {wheel}")

    bundle = dist / f"actr-{version}-{args.platform}"
    if bundle.exists():
        shutil.rmtree(bundle)
    bundle.mkdir(parents=True)

    windows = args.platform.startswith("windows")
    copy_file(binary, bundle / ("actr.exe" if windows else "actr"), executable=not windows)
    copy_file(wheel, bundle / "wheels" / wheel.name)

    tutorial_count = copy_tutorial(bundle / "tutorial")
    copy_licenses(bundle / "LICENSES")

    copy_file(ROOT / "scripts" / "release" / "README.md", bundle / "README.md")
    copy_file(ROOT / "scripts" / "release" / "QUICKSTART.txt", bundle / "QUICKSTART.txt")

    copy_optional(
        ROOT / "rust_actr7" / "python" / "examples" / "lunar_lander.py",
        bundle / "examples" / "lunar_lander.py",
    )
    copy_optional(
        ROOT / "actr7.x" / "tutorial" / "python" / "actr.py",
        bundle / "clients" / "actr.py",
    )

    (bundle / "VERSION").write_text(f"{version}\n", encoding="utf-8")

    print(f"staged {bundle} ({tutorial_count} tutorial scripts)")
    if args.zip:
        zip_path = make_zip(bundle)
        print(f"wrote {zip_path}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platform", required=True, help="e.g. windows-x86_64")
    parser.add_argument("--binary", required=True, type=Path, help="path to actr/actr.exe")
    parser.add_argument("--wheel", required=True, type=Path, help="path to the matching wheel")
    parser.add_argument("--dist", type=Path, default=Path("dist"), help="output directory")
    parser.add_argument("--version", help="override the version read from the manifests")
    parser.add_argument("--zip", action="store_true", help="also write a .zip next to the bundle")
    stage(parser.parse_args())
    return 0


if __name__ == "__main__":
    sys.exit(main())

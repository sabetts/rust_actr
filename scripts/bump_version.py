#!/usr/bin/env python3
"""Bump or check the ACT-R Rust release version.

The version is duplicated in three manifests and two Cargo.lock files. This
script keeps them in sync so a release tag cannot be built from stale metadata.

Bump:
    python scripts/bump_version.py 0.1.1
    python scripts/bump_version.py v0.1.1 --dry-run

Check (used by the release workflow):
    python scripts/bump_version.py --check
    python scripts/bump_version.py --check --tag v0.1.1

Full release steps:
    1. python scripts/bump_version.py 0.1.1
    2. git add -A
       git commit -m "release: v0.1.1"
       git push origin main
    3. git tag v0.1.1
       git push origin v0.1.1

Follow the release run:
    gh run list --workflow release.yml
    gh run watch <run-id> --exit-status
    gh run view <run-id> --log-failed
    gh release view v0.1.1
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION_RE = re.compile(r"^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$")
MANIFEST_VERSION_RE = re.compile(r'^(version\s*=\s*)"[^"]*"', re.MULTILINE)


def fail(message: str) -> None:
    raise SystemExit(f"bump_version: {message}")


def read_text(path: Path) -> str:
    with path.open("r", encoding="utf-8", newline="") as handle:
        return handle.read()


def write_text(path: Path, text: str) -> None:
    with path.open("w", encoding="utf-8", newline="") as handle:
        handle.write(text)


def manifest_version(path: Path) -> str:
    match = MANIFEST_VERSION_RE.search(read_text(path))
    if not match:
        fail(f"could not find a version in {path}")
    return match.group(0).split('"', 2)[1]


def replace_manifest_version(path: Path, version: str) -> str | None:
    text = read_text(path)
    match = MANIFEST_VERSION_RE.search(text)
    if not match:
        fail(f"could not find a version in {path}")
    old = match.group(0).split('"', 2)[1]
    if old == version:
        return None
    write_text(path, text[: match.start()] + f'{match.group(1)}"{version}"' + text[match.end() :])
    return old


def lock_version(text: str, package: str, path: Path) -> str:
    pattern = re.compile(
        rf'name = "{re.escape(package)}"\r?\nversion = "([^"]+)"',
        re.MULTILINE,
    )
    match = pattern.search(text)
    if not match:
        fail(f"could not find {package!r} in {path}")
    return match.group(1)


def replace_lock_version(text: str, package: str, version: str, path: Path) -> tuple[str, str | None]:
    pattern = re.compile(
        rf'(name = "{re.escape(package)}"\r?\nversion = ")[^"]*(")',
        re.MULTILINE,
    )
    match = pattern.search(text)
    if not match:
        fail(f"could not find {package!r} in {path}")
    old = match.group(0).split('"')[3]
    if old == version:
        return text, None
    updated = text[: match.start()] + match.group(1) + version + match.group(2) + text[match.end() :]
    return updated, old


def paths() -> dict[str, Path]:
    return {
        "crate": ROOT / "rust_actr7" / "Cargo.toml",
        "python crate": ROOT / "rust_actr7" / "python" / "Cargo.toml",
        "wheel": ROOT / "rust_actr7" / "python" / "pyproject.toml",
        "crate lock": ROOT / "rust_actr7" / "Cargo.lock",
        "python lock": ROOT / "rust_actr7" / "python" / "Cargo.lock",
    }


def check(tag: str | None) -> str:
    manifest_paths = paths()
    versions = {
        name: manifest_version(manifest_paths[name])
        for name in ("crate", "python crate", "wheel")
    }
    if len(set(versions.values())) != 1:
        details = ", ".join(f"{name}={version}" for name, version in versions.items())
        fail(f"manifest versions disagree: {details}")
    version = versions["crate"]

    lock_checks = (
        ("crate lock", "actr-engine"),
        ("python lock", "actr-engine"),
        ("python lock", "rust-actr-python"),
    )
    for lock_name, package in lock_checks:
        lock_path = manifest_paths[lock_name]
        actual = lock_version(read_text(lock_path), package, lock_path)
        if actual != version:
            fail(f"{lock_path} has {package}={actual}, expected {version}")

    if tag is not None:
        expected = f"v{version}"
        if tag != expected:
            fail(f"tag {tag} does not match manifest version {expected}")

    print(f"version {version}: manifests and lockfiles agree")
    if tag is not None:
        print(f"tag {tag} matches")
    return version


def bump(version: str, dry_run: bool) -> None:
    if version.startswith("v"):
        version = version[1:]
    if not VERSION_RE.match(version):
        fail(f"not a semantic version: {version!r}")

    manifest_paths = paths()
    changes: list[str] = []

    for name in ("crate", "python crate", "wheel"):
        path = manifest_paths[name]
        old = manifest_version(path)
        if old == version:
            continue
        changes.append(f"{path}: {old} -> {version}")
        if not dry_run:
            replace_manifest_version(path, version)

    lock_packages = (
        ("crate lock", "actr-engine"),
        ("python lock", "actr-engine"),
        ("python lock", "rust-actr-python"),
    )
    for lock_name, package in lock_packages:
        path = manifest_paths[lock_name]
        text = read_text(path)
        updated, old = replace_lock_version(text, package, version, path)
        if old is None:
            continue
        changes.append(f"{path}: {package} {old} -> {version}")
        if not dry_run:
            write_text(path, updated)

    if not changes:
        print(f"already at {version}")
        return

    for change in changes:
        print(change)
    if dry_run:
        print("dry run: no files written")
        return

    check(None)
    # Release steps: commit the manifests and lockfiles, push main, then tag
    # and push the tag. The release workflow verifies that v<version> matches
    # the manifests before building anything.
    print("next:")
    print("  git add -A")
    print(f'  git commit -m "release: v{version}"')
    print("  git push origin main")
    print(f"  git tag v{version}")
    print(f"  git push origin v{version}")
    print("  gh run list --workflow release.yml")
    print("  gh run watch <run-id> --exit-status")
    print(f"  gh release view v{version}")


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("version", nargs="?", help="new version, e.g. 0.1.1")
    parser.add_argument("--check", action="store_true", help="verify manifests and lockfiles")
    parser.add_argument("--tag", help="tag to compare with --check, e.g. v0.1.1")
    parser.add_argument("--dry-run", action="store_true", help="show changes without writing")
    args = parser.parse_args()

    if args.check:
        if args.version:
            parser.error("--check does not take a version")
        check(args.tag)
        return 0

    if args.tag:
        parser.error("--tag is only valid with --check")
    if not args.version:
        parser.error("provide a version, or use --check")
    bump(args.version, args.dry_run)
    return 0


if __name__ == "__main__":
    sys.exit(main())

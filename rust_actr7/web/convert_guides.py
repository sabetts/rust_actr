#!/usr/bin/env python3
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

WEB_DIR = Path(__file__).resolve().parent
GUIDES_DIR = WEB_DIR / "guides"
TEMPLATE = WEB_DIR / "guide.template.html"
MANIFEST = GUIDES_DIR / "manifest.json"

sys.path.insert(0, str(WEB_DIR))
from build import MODEL_GROUPS

DOC_SPECS = (
    ("{unit}.odt", "Unit guide", "guide", "{unit}"),
    ("{unit}_code.odt", "Code walkthrough", "code", "{unit}-code"),
    ("{unit}_modeling.odt", "Modeling notes", "modeling", "{unit}-modeling"),
    ("productions-summary.odt", "Productions summary", "productions", "productions-summary"),
)


def require_tool(name: str) -> str:
    path = shutil.which(name)
    if not path:
        raise SystemExit(f"required tool not found: {name}")
    return path


def run(command: list[str], cwd: Path) -> None:
    result = subprocess.run(command, cwd=cwd, capture_output=True, text=True)
    if result.returncode == 0:
        return
    details = "\n".join(part for part in (result.stdout, result.stderr) if part).strip()
    raise SystemExit(f"command failed ({result.returncode}): {' '.join(command)}\n{details}")


def normalize_headings(html: str) -> str:
    seen_title = False

    def replace(match: re.Match[str]) -> str:
        nonlocal seen_title
        current = int(match.group(1))
        attrs = match.group(2) or ""
        body = match.group(3)
        text = re.sub(r"<[^>]+>", "", body).strip()
        if not text:
            return ""
        if current == 1 and not seen_title:
            seen_title = True
            return f"<h1{attrs}>{body}</h1>"
        number = re.match(r"(\d+(?:\.\d+)*)", text)
        if number:
            level = max(2, min(6, 1 + number.group(1).count(".")))
        elif current == 1:
            level = 2
        else:
            level = current
        return f"<h{level}{attrs}>{body}</h{level}>"

    return re.sub(r"<h([1-6])(\s[^>]*)?>(.*?)</h\1>", replace, html, flags=re.S)


def convert_doc(
    pandoc: str,
    magick: str,
    source: Path,
    unit_dir: Path,
    stem: str,
    page_title: str,
) -> Path:
    unit_dir.mkdir(parents=True, exist_ok=True)
    output = unit_dir / f"{stem}.html"
    media = Path("media") / stem
    run(
        [
            pandoc,
            str(source),
            "--from=odt",
            "--to=html5",
            "--standalone",
            f"--template={TEMPLATE}",
            "--metadata",
            f"pagetitle={page_title}",
            "--metadata",
            f"unit={page_title}",
            f"--extract-media={media.as_posix()}",
            "-o",
            output.name,
        ],
        cwd=unit_dir,
    )

    html = normalize_headings(output.read_text(encoding="utf-8"))
    media_dir = unit_dir / media
    if media_dir.is_dir():
        for wmf in sorted(media_dir.rglob("*.wmf")):
            png = wmf.with_suffix(".png")
            run([magick, str(wmf), str(png)], cwd=unit_dir)
            rel_wmf = (media / wmf.relative_to(media_dir)).as_posix()
            rel_png = (media / png.relative_to(media_dir)).as_posix()
            html = html.replace(rel_wmf, rel_png)
            wmf.unlink()
    output.write_text(html, encoding="utf-8")
    return output


def main() -> int:
    pandoc = require_tool("pandoc")
    magick = require_tool("magick")
    if not TEMPLATE.is_file():
        raise SystemExit(f"missing guide template: {TEMPLATE}")

    manifest: dict[str, list[dict[str, str]]] = {}
    for group in MODEL_GROUPS:
        unit_id = group["id"]
        source_dir = group["source"]
        unit_dir = GUIDES_DIR / unit_id
        if unit_dir.exists():
            shutil.rmtree(unit_dir)
        entries: list[dict[str, str]] = []
        for pattern, label, kind, stem_pattern in DOC_SPECS:
            source = source_dir / pattern.format(unit=unit_id)
            if not source.is_file():
                continue
            stem = stem_pattern.format(unit=unit_id)
            title = f"{group['label']} · {label}"
            convert_doc(pandoc, magick, source, unit_dir, stem, title)
            entries.append(
                {
                    "id": f"{unit_id}-{kind}",
                    "label": label,
                    "kind": kind,
                    "path": f"guides/{unit_id}/{stem}.html",
                }
            )
        manifest[unit_id] = entries
        print(f"converted {len(entries)} guides for {group['label']}")

    MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    total = sum(len(entries) for entries in manifest.values())
    print(f"wrote {total} guides to {GUIDES_DIR}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

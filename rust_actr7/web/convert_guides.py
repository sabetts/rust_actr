#!/usr/bin/env python3
import html as html_lib
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

WEB_DIR = Path(__file__).resolve().parent
GUIDES_DIR = WEB_DIR / "guides"
TEMPLATE = WEB_DIR / "guide.template.html"
LISP_FILTER = WEB_DIR / "lisp-code.lua"
MANIFEST = GUIDES_DIR / "manifest.json"

sys.path.insert(0, str(WEB_DIR))
from build import MODEL_GROUPS

DOC_SPECS = (
    ("{unit}.odt", "Unit guide", "guide", "{unit}"),
    ("{unit}_code.odt", "Code walkthrough", "code", "{unit}-code"),
    ("{unit}_modeling.odt", "Modeling notes", "modeling", "{unit}-modeling"),
    ("productions-summary.odt", "Productions summary", "productions", "productions-summary"),
)

KEYWORD_FORMS = {
    "clear-all",
    "define-model",
    "chunk-type",
    "add-dm",
    "goal-focus",
    "p",
    "sgp",
    "set-buffer-chunk",
    "clear-buffer",
    "define-chunks",
    "pdisable",
    "penable",
    "mod-focus",
    "buffer-read",
    "buffer-slot-value",
    "copy-chunk",
    "chunk-slot-value",
    "mod-chunk",
    "!output!",
}

LISP_PRE_RE = re.compile(
    r'(<pre[^>]*\bclass="[^"]*\blisp\b[^"]*"[^>]*>\s*<code[^>]*>)(.*?)(</code>\s*</pre>)',
    re.S,
)


def escape(value: str) -> str:
    return html_lib.escape(value, quote=False)


def classify_word(word: str) -> str:
    if re.fullmatch(r"[=+\-*][A-Za-z0-9_-]*>", word):
        return "tok-buffer"
    if word == "==>":
        return "tok-arrow"
    if word.lower() == "isa":
        return "tok-isa"
    if re.fullmatch(r"-?\d+(?:\.\d+)?", word):
        return "tok-number"
    if word.lower() in KEYWORD_FORMS:
        return "tok-keyword"
    return ""


def tokenize_lisp(text: str) -> str:
    out: list[str] = []
    index = 0
    length = len(text)
    while index < length:
        char = text[index]
        if char == ";":
            end = text.find("\n", index)
            stop = length if end == -1 else end
            out.append(f'<span class="tok-comment">{escape(text[index:stop])}</span>')
            index = stop
            continue
        if char == '"':
            end = index + 1
            while end < length:
                if text[end] == "\\":
                    end += 2
                elif text[end] == '"':
                    end += 1
                    break
                else:
                    end += 1
            out.append(f'<span class="tok-string">{escape(text[index:end])}</span>')
            index = end
            continue
        if char == "#" and index + 1 < length and text[index + 1] == "|":
            end = index + 2
            depth = 1
            while end < length and depth > 0:
                if text.startswith("#|", end):
                    depth += 1
                    end += 2
                elif text.startswith("|#", end):
                    depth -= 1
                    end += 2
                else:
                    end += 1
            out.append(f'<span class="tok-comment">{escape(text[index:end])}</span>')
            index = end
            continue
        if char == "|":
            end = index + 1
            while end < length and text[end] != "|":
                end += 1
            if end < length:
                end += 1
            out.append(f'<span class="tok-string">{escape(text[index:end])}</span>')
            index = end
            continue
        if char in "()":
            out.append(f'<span class="tok-paren">{char}</span>')
            index += 1
            continue
        if char.isspace():
            out.append(escape(char))
            index += 1
            continue
        end = index
        while end < length and not (text[end].isspace() or text[end] in '();"'):
            end += 1
        word = text[index:end]
        kind = classify_word(word)
        out.append(f'<span class="{kind}">{escape(word)}</span>' if kind else escape(word))
        index = end
    return "".join(out)


def highlight_lisp_blocks(text: str) -> str:
    def replace(match: re.Match[str]) -> str:
        code = html_lib.unescape(match.group(2))
        return match.group(1) + tokenize_lisp(code) + match.group(3)

    return LISP_PRE_RE.sub(replace, text)


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
            f"--lua-filter={LISP_FILTER}",
            "--no-highlight",
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

    html = highlight_lisp_blocks(output.read_text(encoding="utf-8"))
    html = normalize_headings(html)
    media_dir = unit_dir / media
    if media_dir.is_dir():
        for wmf in sorted(media_dir.rglob("*.wmf")):
            png = wmf.with_suffix(".png")
            run([magick, str(wmf), "-strip", str(png)], cwd=unit_dir)
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
    if not LISP_FILTER.is_file():
        raise SystemExit(f"missing Lisp filter: {LISP_FILTER}")

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

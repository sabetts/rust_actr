function skipLineComment(text, index) {
  while (index < text.length && text[index] !== "\n") index += 1;
  return index;
}

function skipString(text, index, state) {
  index += 1;
  while (index < text.length) {
    const char = text[index];
    if (char === "\\") {
      if (text[index + 1] === "\n") state.line += 1;
      index += 2;
      continue;
    }
    if (char === '"') return index + 1;
    if (char === "\n") state.line += 1;
    index += 1;
  }
  return index;
}

function skipBlockComment(text, index, state) {
  let depth = 1;
  index += 2;
  while (index < text.length && depth > 0) {
    if (text[index] === "#" && text[index + 1] === "|") {
      depth += 1;
      index += 2;
    } else if (text[index] === "|" && text[index + 1] === "#") {
      depth -= 1;
      index += 2;
    } else {
      if (text[index] === "\n") state.line += 1;
      index += 1;
    }
  }
  return index;
}

function skipQuotedSymbol(text, index, state) {
  index += 1;
  while (index < text.length) {
    if (text[index] === "|") return index + 1;
    if (text[index] === "\n") state.line += 1;
    index += 1;
  }
  return index;
}

export function scanParens(text) {
  const pairs = new Map();
  const problems = [];
  const stack = [];
  const state = { line: 1 };
  let index = 0;

  while (index < text.length) {
    const char = text[index];
    if (char === "\n") {
      state.line += 1;
      index += 1;
      continue;
    }
    if (char === ";") {
      index = skipLineComment(text, index);
      continue;
    }
    if (char === '"') {
      index = skipString(text, index, state);
      continue;
    }
    if (char === "#" && text[index + 1] === "|") {
      index = skipBlockComment(text, index, state);
      continue;
    }
    if (char === "|") {
      index = skipQuotedSymbol(text, index, state);
      continue;
    }
    if (char === "(") {
      stack.push({ index, line: state.line });
      index += 1;
      continue;
    }
    if (char === ")") {
      const open = stack.pop();
      if (open) {
        pairs.set(open.index, index);
        pairs.set(index, open.index);
      } else {
        problems.push({ index, char: ")", line: state.line });
      }
      index += 1;
      continue;
    }
    index += 1;
  }

  for (const open of stack) {
    problems.push({ index: open.index, char: "(", line: open.line });
  }
  problems.sort((a, b) => a.index - b.index);

  return { pairs, problems };
}

function lineAt(text, index) {
  let line = 1;
  for (let i = 0; i < index && i < text.length; i += 1) {
    if (text[i] === "\n") line += 1;
  }
  return line;
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function candidates(message) {
  const found = [];
  const add = (value) => {
    const text = String(value || "").trim();
    if (text.length < 3) return;
    if (/^(nil|isa|t|true|false)$/i.test(text)) return;
    if (!found.includes(text)) found.push(text);
  };

  for (const match of message.matchAll(/`([^`]+)`/g)) add(match[1]);
  for (const match of message.matchAll(/\b(?:production|chunk[- ]type|slot)\s+([A-Za-z0-9_*+<>=!?./-]+)/gi)) {
    add(match[1]);
  }
  for (const match of message.matchAll(/\bgot\s+"([^"]+)"/gi)) add(match[1]);
  for (const match of message.matchAll(/\bgot\s+([A-Za-z0-9_*+<>=!?./-]+)/gi)) add(match[1]);
  return found;
}

export function locateErrorAnchor(message, source) {
  if (!message || !source) return null;

  for (const name of candidates(message)) {
    const escaped = escapeRegExp(name);
    const anchors = [
      new RegExp(`\\(\\s*p\\s+${escaped}\\b`, "i"),
      new RegExp(`\\(\\s*chunk-type\\s+${escaped}\\b`, "i"),
      new RegExp(`\\(\\s*define-model\\s+${escaped}\\b`, "i"),
      new RegExp(`(^|[\\s()])${escaped}(?=[\\s()]|$)`, "i"),
    ];
    for (const pattern of anchors) {
      const match = pattern.exec(source);
      if (match) {
        const offset = match.index + (match[1] ? match[1].length : 0);
        return { line: lineAt(source, offset), anchor: name };
      }
    }
  }
  return null;
}

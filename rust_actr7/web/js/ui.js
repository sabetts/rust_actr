import {
  GLOSSARY,
  HOW_TO_STEPS,
  modelInfo,
  unitInfo,
} from "./catalog.js";

const $ = (id) => document.getElementById(id);

const THEME_KEY = "actr-web-theme";
const WELCOME_KEY = "actr-web-welcome-dismissed";
const MAX_HIGHLIGHT_LINES = 12000;
const TRACE_TIMESTAMP_RE =
  /^(\s*)(\d+(?:\.\d+)?)(\s{2,})(\S+)(\s{2,})(\S+)(.*)$/;

const KEYWORD_FORMS = new Set([
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
]);

const BADGE_LABELS = {
  starter: "Starter",
  solution: "Solution",
  broken: "Broken",
  demo: "Demo",
};

const OUTLINE_LABELS = {
  "define-model": "model",
  "chunk-type": "type",
  "add-dm": "memory",
  "goal-focus": "goal",
  p: "prod",
  sgp: "params",
};

function readStorage(key) {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

function writeStorage(key, value) {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    return;
  }
}

function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text != null) node.textContent = text;
  return node;
}

export function setupTheme() {
  const root = document.documentElement;
  const button = $("theme-toggle");
  const meta = document.querySelector('meta[name="theme-color"]');
  const media = window.matchMedia?.("(prefers-color-scheme: dark)");

  const apply = (theme) => {
    root.dataset.theme = theme;
    if (button) {
      button.setAttribute("aria-pressed", theme === "dark" ? "true" : "false");
      button.title = theme === "dark" ? "Switch to light mode" : "Switch to dark mode";
    }
    if (meta) meta.content = theme === "dark" ? "#0a0f16" : "#f4f6fa";
  };

  apply(root.dataset.theme === "dark" ? "dark" : "light");

  button?.addEventListener("click", () => {
    const next = root.dataset.theme === "dark" ? "light" : "dark";
    apply(next);
    writeStorage(THEME_KEY, next);
  });

  media?.addEventListener?.("change", (event) => {
    if (readStorage(THEME_KEY)) return;
    apply(event.matches ? "dark" : "light");
  });
}

export function setupDialogs() {
  for (const trigger of document.querySelectorAll("[data-dialog-open]")) {
    trigger.addEventListener("click", () => {
      const dialog = $(trigger.dataset.dialogOpen);
      if (!dialog) return;
      if (typeof dialog.showModal === "function") {
        dialog.showModal();
      } else {
        dialog.setAttribute("open", "");
      }
    });
  }

  for (const dialog of document.querySelectorAll("dialog")) {
    dialog.querySelectorAll("[data-dialog-close]").forEach((button) => {
      button.addEventListener("click", () => dialog.close());
    });
    dialog.addEventListener("click", (event) => {
      if (event.target === dialog) dialog.close();
    });
    dialog.addEventListener("keydown", (event) => {
      if (event.key === "Escape") {
        event.preventDefault();
        dialog.close();
      }
    });
  }
}

export function renderAboutContent() {
  const steps = $("how-to-steps");
  if (steps) {
    steps.replaceChildren();
    for (const text of HOW_TO_STEPS) steps.append(element("li", null, text));
  }

  const glossary = $("glossary-grid");
  if (glossary) {
    glossary.replaceChildren();
    for (const item of GLOSSARY) {
      const wrapper = element("div", "glossary-item");
      const dl = element("dl");
      dl.append(element("dt", null, item.term), element("dd", null, item.definition));
      wrapper.append(dl);
      glossary.append(wrapper);
    }
  }
}

export function setupWelcome({ onStart, onExplore } = {}) {
  const card = $("welcome-card");
  if (!card) return { show() {}, hide() {} };

  const hide = (persist = true) => {
    card.hidden = true;
    if (persist) writeStorage(WELCOME_KEY, "1");
  };

  if (readStorage(WELCOME_KEY) === "1") hide(false);

  $("welcome-start")?.addEventListener("click", () => {
    hide();
    onStart?.();
  });
  $("welcome-explore")?.addEventListener("click", () => {
    onExplore?.();
  });
  $("welcome-dismiss")?.addEventListener("click", () => hide());

  return { show: () => { card.hidden = false; }, hide: () => hide(false) };
}

export function setupTabs() {
  const tabs = [...document.querySelectorAll("[role=tab]")];
  const visibleTabs = () => tabs.filter((tab) => !tab.hidden);

  const activate = (id, { focus = false } = {}) => {
    let found = false;
    for (const tab of tabs) {
      const selected = tab.dataset.tab === id && !tab.hidden;
      if (selected) found = true;
      tab.setAttribute("aria-selected", selected ? "true" : "false");
      tab.tabIndex = selected ? 0 : -1;
      const panel = $(tab.getAttribute("aria-controls"));
      if (panel) panel.hidden = !selected;
      if (selected && focus) tab.focus();
    }
    if (!found && id !== "task") {
      const first = visibleTabs()[0];
      if (first && first.dataset.tab !== id) activate(first.dataset.tab, { focus });
    }
  };

  const current = () =>
    tabs.find((tab) => tab.getAttribute("aria-selected") === "true")?.dataset.tab ||
    null;

  for (const tab of tabs) {
    tab.addEventListener("click", () => activate(tab.dataset.tab));
    tab.addEventListener("keydown", (event) => {
      if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
      event.preventDefault();
      const list = visibleTabs();
      if (!list.length) return;
      let index = list.indexOf(tab);
      if (event.key === "ArrowRight") index = (index + 1) % list.length;
      else if (event.key === "ArrowLeft") index = (index - 1 + list.length) % list.length;
      else if (event.key === "Home") index = 0;
      else index = list.length - 1;
      activate(list[index].dataset.tab, { focus: true });
    });
  }

  const setTaskEnabled = (enabled) => {
    const taskTab = $("tab-task");
    if (!taskTab) return;
    taskTab.hidden = !enabled;
    if (!enabled) {
      const panel = $("panel-task");
      if (panel) panel.hidden = true;
      if (current() === "task") activate("source");
    }
  };

  const initial = tabs.find((tab) => tab.getAttribute("aria-selected") === "true");
  activate(initial?.dataset.tab || "source");
  setTaskEnabled(!$("tab-task")?.hidden);

  return { activate, current, setTaskEnabled };
}

export function createModelBrowser({ groups }) {
  const list = $("model-list");
  const search = $("model-search");
  const select = $("model-preset");
  if (!list || !select) return { markSelected() {}, focusSearch() {} };

  const openState = new Map();
  let query = "";

  const findEntry = (id) => {
    for (const group of groups) {
      const model = group.models.find((item) => item.id === id);
      if (model) return { group, model };
    }
    return null;
  };

  const choose = (id) => {
    if (![...select.options].some((option) => option.value === id)) return;
    select.value = id;
    select.dispatchEvent(new Event("change", { bubbles: true }));
  };

  const badge = (kind) => {
    if (!kind) return null;
    const node = element("span", `badge badge-${kind}`, BADGE_LABELS[kind] || kind);
    return node;
  };

  const renderRows = (group, models, activeId) => {
    const body = element("div", "unit-body");
    body.id = `unit-body-${group.id}`;
    for (const model of models) {
      const meta = modelInfo(model, group.id);
      const row = element("button", "model-row");
      row.type = "button";
      row.dataset.modelId = model.id;
      row.setAttribute("aria-current", model.id === activeId ? "true" : "false");
      row.title = meta.description;

      const text = element("span", "model-row-text");
      text.append(element("span", "model-row-title", model.label || model.id));
      text.append(element("span", "model-row-sub", meta.description));
      if (meta.isRecommended) text.append(element("span", "start-here", "Start here"));

      row.append(text);
      const tag = badge(meta.badge);
      if (tag) row.append(tag);
      row.addEventListener("click", () => choose(model.id));
      body.append(row);
    }
    return body;
  };

  const render = () => {
    list.replaceChildren();
    const activeId = select.value;
    const needle = query.trim().toLowerCase();
    let visibleCount = 0;

    for (const group of groups) {
      const info = unitInfo(group.id, group.label);
      const matches = group.models.filter((model) => {
        if (!needle) return true;
        const meta = modelInfo(model, group.id);
        return `${model.label} ${meta.description} ${info.title} ${info.blurb}`
          .toLowerCase()
          .includes(needle);
      });
      if (!matches.length) continue;
      visibleCount += matches.length;

      const unit = element("section", "unit");
      unit.dataset.unit = group.id;
      unit.dataset.open = needle || openState.get(group.id) ? "true" : "false";

      const head = element("button", "unit-head");
      head.type = "button";
      head.setAttribute("aria-expanded", unit.dataset.open === "true" ? "true" : "false");
      head.setAttribute("aria-controls", `unit-body-${group.id}`);

      head.append(
        element(
          "span",
          "unit-number",
          String(group.label || group.id || "").replace(/[^0-9]/g, "") || "?"
        )
      );
      const headText = element("span", "unit-head-text");
      headText.append(element("span", "unit-title", info.title));
      headText.append(element("span", "unit-blurb", info.blurb));
      head.append(headText);
      head.append(element("span", "unit-count", String(matches.length)));
      head.append(element("span", "unit-caret", "▾"));

      head.addEventListener("click", () => {
        const open = unit.dataset.open === "true";
        unit.dataset.open = open ? "false" : "true";
        head.setAttribute("aria-expanded", open ? "false" : "true");
        openState.set(group.id, !open);
      });

      unit.append(head, renderRows(group, matches, activeId));
      list.append(unit);
    }

    if (!visibleCount) {
      list.append(element("div", "model-empty", "No models match that search."));
    }
  };

  const markSelected = (id) => {
    for (const row of list.querySelectorAll(".model-row")) {
      row.setAttribute("aria-current", row.dataset.modelId === id ? "true" : "false");
    }
    const entry = findEntry(id);
    if (entry) {
      openState.set(entry.group.id, true);
      const unit = list.querySelector(`.unit[data-unit="${entry.group.id}"]`);
      if (unit && unit.dataset.open !== "true") {
        unit.dataset.open = "true";
        unit.querySelector(".unit-head")?.setAttribute("aria-expanded", "true");
      }
    }
  };

  search?.addEventListener("input", () => {
    query = search.value;
    render();
  });

  for (const group of groups) {
    if (group.id === "unit1") openState.set(group.id, true);
  }
  render();

  return {
    choose,
    markSelected,
    render,
    focusSearch() {
      search?.focus();
      search?.select();
    },
  };
}

export function renderModelInfo({ model, unitId, descriptor }) {
  const host = $("model-info");
  if (!host) return;
  host.replaceChildren();

  if (!model) {
    host.append(element("div", "model-info-kicker", "No model selected"));
    host.append(element("h2", null, "Choose a model"));
    host.append(element("p", null, "Pick a tutorial model from the list to get started."));
    return;
  }

  const unit = unitInfo(unitId, unitId);
  const meta = modelInfo(model, unitId);

  const kicker = element(
    "div",
    "model-info-kicker",
    `${unitId?.replace("unit", "Unit ") || "Model"} · ${unit.title}`
  );
  const title = element("h2", null, model.label || model.id);
  const description = element("p", null, meta.description);
  const tags = element("div", "model-tags");

  if (meta.badge) tags.append(element("span", `badge badge-${meta.badge}`, BADGE_LABELS[meta.badge]));
  if (descriptor) tags.append(element("span", "tag tag-accent", "Interactive task"));
  else tags.append(element("span", "tag", "Trace only"));

  host.append(kicker, title, description, tags);

  if (meta.highlights.length) {
    const list = element("ul", "model-highlights");
    for (const text of meta.highlights) list.append(element("li", null, text));
    host.append(list);
  }
}

export function setStatus(text, forcedState) {
  const node = $("status");
  if (!node) return;
  node.textContent = text;
  node.dataset.state = forcedState || statusState(text);
}

export function statusState(text) {
  const value = String(text || "").trim().toLowerCase();
  if (!value) return "idle";
  if (value.startsWith("ok")) return "ok";
  if (value.startsWith("error") || value.includes("failed")) return "error";
  if (value.startsWith("cancelled")) return "cancelled";
  if (value.includes("running") || value.includes("loading") || value.includes("cancelling")) {
    return "running";
  }
  return "idle";
}

export function setRunState(running) {
  const run = $("run");
  if (!run) return;
  run.classList.toggle("btn-loading", Boolean(running));
  run.setAttribute("aria-busy", running ? "true" : "false");
}

export function setTaskHeader({ model, descriptor }) {
  const title = $("task-title");
  const subtitle = $("task-subtitle");
  if (!title || !subtitle) return;
  if (!model) {
    title.textContent = "Task";
    subtitle.textContent = "Run the model to see the experiment.";
    return;
  }
  title.textContent = model.label || model.id;
  const meta = modelInfo(model, model.unitId);
  subtitle.textContent = descriptor
    ? meta.description
    : "This model has no interactive task. Run it to read the trace.";
}

export function setupTraceView({ onCountChange } = {}) {
  const view = $("trace-view");
  const output = $("output");
  const search = $("trace-search");
  const wrapButton = $("trace-wrap");
  const followButton = $("trace-follow");
  const meta = $("trace-meta");
  const count = $("trace-count");

  if (!view || !output) {
    return { set() {}, clear() {}, getText: () => "", focusSearch() {} };
  }

  let raw = "";
  let filter = "";
  let wrap = true;
  let follow = true;
  let followPaused = false;

  const matches = (line) => !filter || line.toLowerCase().includes(filter);

  const lineClass = (line) => {
    const lower = line.toLowerCase();
    if (/(^|\s)warn(ing)?(\s|:|$)/.test(lower)) return "trace-warn";
    if (/(^|\s)(error|failed|cannot|can't|invalid)(\s|:|$)/.test(lower)) {
      return "trace-error";
    }
    if (/^stopped because/.test(lower) || /^summary/.test(lower) || /^==/.test(lower)) {
      return "trace-heading";
    }
    return "";
  };

  const appendTimestampLine = (text, fragment) => {
    const match = text.match(TRACE_TIMESTAMP_RE);
    if (!match) return false;
    const [, lead, time, gap1, module, gap2, event, rest] = match;
    fragment.append(lead);
    fragment.append(element("span", "trace-time", time));
    fragment.append(gap1);
    fragment.append(element("span", "trace-module", module));
    fragment.append(gap2);
    fragment.append(element("span", "trace-event", event));
    fragment.append(rest);
    return true;
  };

  const render = () => {
    const lines = raw ? raw.split("\n") : [];
    const visible = lines.filter((line) => matches(line));
    const highlight = lines.length <= MAX_HIGHLIGHT_LINES;
    view.replaceChildren();

    if (!lines.length) {
      view.append(renderTraceEmpty());
    } else if (!visible.length) {
      const empty = element("div", "trace-empty");
      empty.append(element("strong", null, "No matching lines"));
      empty.append(element("span", null, "Try a different filter or clear the search."));
      view.append(empty);
    } else {
      const frag = document.createDocumentFragment();
      let lineNumber = 0;
      for (const line of lines) {
        lineNumber += 1;
        if (!matches(line)) continue;
        const row = element("div", "trace-line");
        const tone = lineClass(line);
        if (tone) row.classList.add(tone);
        row.append(element("span", "trace-num", String(lineNumber)));
        const text = element("span", "trace-text");
        if (!highlight || !appendTimestampLine(line, text)) {
          text.textContent = line;
          if (!line.trim()) row.classList.add("trace-blank");
          else if (
            !TRACE_TIMESTAMP_RE.test(line) &&
            /^[A-Z0-9 _().<>-]+$/.test(line.trim())
          ) {
            row.classList.add("trace-output");
          }
        }
        row.append(text);
        frag.append(row);
      }
      view.append(frag);
    }

    view.classList.toggle("nowrap", !wrap);
    if (meta) {
      meta.textContent = !lines.length
        ? "No run yet"
        : filter
          ? `${visible.length} of ${lines.length} lines`
          : `${lines.length} lines`;
    }
    if (count) {
      count.textContent = String(lines.length);
      count.hidden = !lines.length;
    }
    onCountChange?.(lines.length);

    if (follow && !followPaused) {
      view.scrollTop = view.scrollHeight;
    }
  };

  const renderTraceEmpty = () => {
    const empty = element("div", "trace-empty");
    empty.id = "trace-empty";
    const icon = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    icon.setAttribute("viewBox", "0 0 24 24");
    icon.setAttribute("fill", "none");
    icon.setAttribute("stroke", "currentColor");
    icon.setAttribute("stroke-width", "1.6");
    icon.innerHTML =
      '<path d="M4 5h16M4 9h10M4 13h13M4 17h7" stroke-linecap="round"/>';
    empty.append(icon);
    empty.append(element("strong", null, "Your trace will appear here"));
    empty.append(
      element(
        "span",
        null,
        "Press Run to see buffers, retrievals, and production firings from the engine."
      )
    );
    return empty;
  };

  const set = (text, { error = false } = {}) => {
    raw = text || "";
    output.value = raw;
    output.classList.toggle("err", Boolean(error));
    followPaused = false;
    render();
  };

  const clear = () => {
    raw = "";
    output.value = "";
    output.classList.remove("err");
    render();
  };

  const getText = () => raw;

  search?.addEventListener("input", () => {
    filter = search.value.trim().toLowerCase();
    render();
  });

  wrapButton?.addEventListener("click", () => {
    wrap = !wrap;
    wrapButton.setAttribute("aria-pressed", wrap ? "true" : "false");
    render();
  });

  followButton?.addEventListener("click", () => {
    follow = !follow;
    followPaused = false;
    followButton.setAttribute("aria-pressed", follow ? "true" : "false");
    if (follow) view.scrollTop = view.scrollHeight;
  });

  view.addEventListener("scroll", () => {
    const atBottom = view.scrollHeight - view.scrollTop - view.clientHeight < 28;
    if (!atBottom && follow) {
      follow = false;
      followPaused = true;
      followButton?.setAttribute("aria-pressed", "false");
    }
  });

  render();

  return { set, clear, getText, focusSearch: () => search?.focus() };
}

export function createEditor({ onChange } = {}) {
  const textarea = $("model");
  const gutter = $("editor-gutter-lines");
  const highlight = $("editor-highlight");
  const highlightCode = highlight?.querySelector("code") || highlight;
  const outline = $("model-outline");
  const meta = $("source-meta");
  const toggle = $("toggle-outline");
  if (!textarea || !highlight) {
    return { setSource() {}, getSource: () => "", focus() {} };
  }

  let source = "";
  let pending = 0;
  let outlineOpen = true;

  const escape = (value) =>
    value
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");

  const classifyWord = (word) => {
    if (/^[=+\-*][A-Za-z0-9_-]*>$/.test(word)) return "tok-buffer";
    if (word === "==>") return "tok-arrow";
    if (word.toLowerCase() === "isa") return "tok-isa";
    if (/^-?\d+(?:\.\d+)?$/.test(word)) return "tok-number";
    if (KEYWORD_FORMS.has(word.toLowerCase())) return "tok-keyword";
    return "";
  };

  const tokenize = (text) => {
    let out = "";
    let index = 0;
    const length = text.length;
    while (index < length) {
      const char = text[index];
      if (char === ";") {
        const end = text.indexOf("\n", index);
        const stop = end === -1 ? length : end;
        out += `<span class="tok-comment">${escape(text.slice(index, stop))}</span>`;
        index = stop;
        continue;
      }
      if (char === '"') {
        let end = index + 1;
        while (end < length) {
          if (text[end] === "\\") end += 2;
          else if (text[end] === '"') {
            end += 1;
            break;
          } else end += 1;
        }
        out += `<span class="tok-string">${escape(text.slice(index, end))}</span>`;
        index = end;
        continue;
      }
      if (char === "(" || char === ")") {
        out += `<span class="tok-paren">${char}</span>`;
        index += 1;
        continue;
      }
      if (/\s/.test(char)) {
        out += char;
        index += 1;
        continue;
      }
      let end = index;
      while (end < length && !/[\s();"]/.test(text[end])) end += 1;
      const word = text.slice(index, end);
      const kind = classifyWord(word);
      out += kind ? `<span class="${kind}">${escape(word)}</span>` : escape(word);
      index = end;
    }
    return out;
  };

  const lineOffsets = (lines) => {
    const offsets = [];
    let offset = 0;
    for (const line of lines) {
      offsets.push(offset);
      offset += line.length + 1;
    }
    return offsets;
  };

  const renderOutline = (lines, offsets) => {
    if (!outline) return;
    outline.replaceChildren();

    const groups = new Map();
    lines.forEach((line, index) => {
      const match = line.match(
        /^\s*\(\s*(define-model|chunk-type|add-dm|goal-focus|p|sgp)\b(.*)$/i
      );
      if (!match) return;
      const kind = match[1].toLowerCase();
      const rest = match[2].trim();
      let name = rest;
      if (kind === "define-model" || kind === "chunk-type" || kind === "p") {
        name = rest.split(/\s+/)[0] || kind;
      } else if (kind === "add-dm") name = "declarative chunks";
      else if (kind === "goal-focus") name = "initial goal";
      else if (kind === "sgp") name = "parameters";
      const label =
        kind === "define-model"
          ? "Model"
          : kind === "chunk-type"
            ? "Chunk types"
            : kind === "add-dm"
              ? "Memory"
              : kind === "goal-focus"
                ? "Goal"
                : kind === "p"
                  ? "Productions"
                  : "Parameters";
      if (!groups.has(label)) groups.set(label, []);
      groups.get(label).push({ name, line: index + 1, offset: offsets[index] ?? 0, kind });
    });

    if (!groups.size) {
      outline.hidden = true;
      return;
    }

    for (const [label, items] of groups) {
      const group = element("div", "outline-group");
      group.append(element("div", "outline-label", label));
      for (const item of items) {
        const button = element("button", "outline-item");
        button.type = "button";
        button.append(element("span", "outline-kind", OUTLINE_LABELS[item.kind] || item.kind));
        button.append(element("span", "outline-item-name", item.name));
        button.append(element("span", "outline-line", String(item.line)));
        button.addEventListener("click", () => {
          textarea.focus();
          textarea.setSelectionRange(item.offset, item.offset);
          const lineHeight = parseFloat(
            getComputedStyle(textarea).getPropertyValue("line-height")
          ) || 20;
          textarea.scrollTop = Math.max(0, lineHeight * (item.line - 1) - 40);
        });
        group.append(button);
      }
      outline.append(group);
    }
    outline.hidden = !outlineOpen;
  };

  const refresh = () => {
    pending = 0;
    const text = textarea.value;
    const lines = text.split("\n");
    if (gutter) {
      const fragment = document.createDocumentFragment();
      for (let i = 1; i <= lines.length; i += 1) {
        fragment.append(element("div", "editor-gutter-line", String(i)));
      }
      gutter.replaceChildren(fragment);
    }
    highlight.innerHTML = text.length > 0 ? tokenize(text) : "";
    if (meta) {
      meta.textContent = `${lines.length} lines · ${text.length.toLocaleString()} chars`;
    }
    renderOutline(lines, lineOffsets(lines));
  };

  const scheduleRefresh = () => {
    if (pending) return;
    pending = window.requestAnimationFrame(refresh);
  };

  const syncScroll = () => {
    if (gutter) {
      gutter.style.transform = `translateY(${-textarea.scrollTop}px)`;
    }
    highlightCode.style.transform = `translate(${-textarea.scrollLeft}px, ${-textarea.scrollTop}px)`;
  };

  textarea.addEventListener("input", () => {
    scheduleRefresh();
    onChange?.(textarea.value);
  });
  textarea.addEventListener("scroll", syncScroll, { passive: true });

  if (toggle) {
    toggle.setAttribute("aria-pressed", "true");
    toggle.addEventListener("click", () => {
      outlineOpen = !outlineOpen;
      toggle.setAttribute("aria-pressed", outlineOpen ? "true" : "false");
      if (outline) outline.hidden = !outlineOpen;
    });
  }

  const setSource = (text) => {
    source = text || "";
    textarea.value = source;
    textarea.scrollTop = 0;
    textarea.scrollLeft = 0;
    refresh();
    syncScroll();
  };

  const getSource = () => textarea.value;

  window.addEventListener("resize", scheduleRefresh, { passive: true });

  return {
    setSource,
    getSource,
    getOriginal: () => source,
    refresh,
    focus: () => textarea.focus(),
  };
}

export function copyText(text, label = "Copied to clipboard") {
  const value = String(text ?? "");
  if (!value) return Promise.resolve(false);
  const done = () => showToast(label);

  if (navigator.clipboard?.writeText) {
    return navigator.clipboard.writeText(value).then(
      () => {
        done();
        return true;
      },
      () => fallbackCopy(value, done)
    );
  }
  return Promise.resolve(fallbackCopy(value, done));
}

function fallbackCopy(value, done) {
  const helper = document.createElement("textarea");
  helper.value = value;
  helper.setAttribute("readonly", "");
  helper.style.position = "fixed";
  helper.style.opacity = "0";
  document.body.append(helper);
  helper.select();
  let ok = false;
  try {
    ok = document.execCommand("copy");
  } catch {
    ok = false;
  }
  helper.remove();
  if (ok) done();
  return ok;
}

export function downloadText(text, filename, type = "text/plain") {
  const blob = new Blob([String(text ?? "")], { type });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  document.body.append(link);
  link.click();
  link.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export function showToast(message) {
  const region = $("toast-region");
  if (!region) return;
  const toast = element("div", "toast");
  const icon = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  icon.setAttribute("viewBox", "0 0 24 24");
  icon.setAttribute("fill", "none");
  icon.setAttribute("stroke", "currentColor");
  icon.setAttribute("stroke-width", "2");
  icon.innerHTML = '<path d="M5 13l4 4L19 7" stroke-linecap="round" stroke-linejoin="round"/>';
  toast.append(icon, element("span", null, message));
  region.append(toast);
  window.setTimeout(() => {
    toast.style.opacity = "0";
    window.setTimeout(() => toast.remove(), 200);
  }, 1800);
}

export function setupToolbar({ editor, trace, getModelId }) {
  $("copy-source")?.addEventListener("click", () => {
    copyText(editor.getSource(), "Model source copied");
  });

  $("copy-trace")?.addEventListener("click", () => {
    const text = trace.getText();
    if (!text) {
      showToast("Nothing to copy yet");
      return;
    }
    copyText(text, "Trace copied");
  });

  $("download-trace")?.addEventListener("click", () => {
    const text = trace.getText();
    if (!text) {
      showToast("Nothing to save yet");
      return;
    }
    const id = getModelId?.() || "trace";
    downloadText(text, `${id}-trace.txt`);
  });
}

export function setupGlobalShortcuts({ run, trace }) {
  document.addEventListener("keydown", (event) => {
    if (document.querySelector("dialog[open]")) return;
    if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
      event.preventDefault();
      run?.();
    }
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
      event.preventDefault();
      document.getElementById("model-search")?.focus();
    }
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "f") {
      if (document.activeElement?.id === "model") return;
      const panel = document.getElementById("panel-trace");
      if (panel && !panel.hidden) {
        event.preventDefault();
        trace?.focusSearch();
      }
    }
  });
}

export function updateTraceCount(count) {
  const node = $("trace-count");
  if (!node) return;
  node.textContent = String(count);
  node.hidden = !count;
}

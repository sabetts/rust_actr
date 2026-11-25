import {
  GLOSSARY,
  HOW_TO_STEPS,
  modelInfo,
  unitInfo,
} from "./catalog.js";
import { scanParens } from "./lisp-scan.js";

const $ = (id) => document.getElementById(id);

const THEME_KEY = "actr-web-theme";
const WELCOME_KEY = "actr-web-welcome-dismissed";
const GUIDE_VERSION = "20261123";
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

const GUIDE_ICONS = {
  guide:
    '<path d="M4 5.5A2.5 2.5 0 0 1 6.5 3H20v18H6.5A2.5 2.5 0 0 0 4 23z"/><path d="M4 5.5A2.5 2.5 0 0 0 6.5 8H20"/>',
  code: '<path d="m8 8-4 4 4 4M16 8l4 4-4 4M13.5 5l-3 14"/>',
  modeling: '<path d="m4 20 4.5-1 10-10a2.1 2.1 0 0 0-3-3l-10 10L4 20z"/>',
  productions:
    '<path d="M8 6h12M8 12h12M8 18h12M4 6h.01M4 12h.01M4 18h.01"/>',
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

export function setupWelcome({ onStart } = {}) {
  const card = $("welcome-card");
  if (!card) return { show() {}, hide() {} };

  const hide = (persist = true) => {
    card.hidden = true;
    if (persist) writeStorage(WELCOME_KEY, "1");
  };
  const isDismissed = () => readStorage(WELCOME_KEY) === "1";

  if (isDismissed()) hide(false);

  $("welcome-start")?.addEventListener("click", () => {
    hide();
    onStart?.();
  });
  $("welcome-dismiss")?.addEventListener("click", () => hide());

  return {
    show: () => {
      if (!isDismissed()) card.hidden = false;
    },
    hide: () => hide(false),
  };
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

  const setGuideEnabled = (enabled) => {
    const guideTab = $("tab-guide");
    if (!guideTab) return;
    guideTab.hidden = !enabled;
    if (!enabled) {
      const panel = $("panel-guide");
      if (panel) panel.hidden = true;
      if (current() === "guide") activate("source");
    }
  };

  const initial = tabs.find((tab) => tab.getAttribute("aria-selected") === "true");
  activate(initial?.dataset.tab || "source");
  setTaskEnabled(!$("tab-task")?.hidden);
  setGuideEnabled(!$("tab-guide")?.hidden);

  return { activate, current, setTaskEnabled, setGuideEnabled };
}

export function createModelBrowser({ groups, guides = {}, onOpenGuide }) {
  const list = $("model-list");
  const select = $("model-preset");
  if (!list || !select) return { markSelected() {} };

  const openState = new Map();

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

  const guideIcon = (kind) => {
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("viewBox", "0 0 24 24");
    svg.setAttribute("fill", "none");
    svg.setAttribute("stroke", "currentColor");
    svg.setAttribute("stroke-width", "1.8");
    svg.setAttribute("stroke-linecap", "round");
    svg.setAttribute("stroke-linejoin", "round");
    svg.setAttribute("aria-hidden", "true");
    svg.innerHTML = GUIDE_ICONS[kind] || GUIDE_ICONS.guide;
    return svg;
  };

  const renderGuides = (group, info) => {
    const unitGuides = Array.isArray(guides[group.id]) ? guides[group.id] : [];
    if (!unitGuides.length) return null;
    const block = element("div", "unit-guides");
    block.append(element("div", "unit-guides-label", "Guides"));
    for (const guide of unitGuides) {
      if (!guide || !guide.path || !guide.label) continue;
      const row = element("div", "unit-guide-row");
      const link = document.createElement("a");
      link.className = "unit-guide";
      link.href = `./${guide.path}?v=${GUIDE_VERSION}`;
      link.title = `${info.title} · ${guide.label}`;
      link.dataset.guideId = guide.id || "";
      link.addEventListener("click", (event) => {
        if (
          event.button !== 0 ||
          event.metaKey ||
          event.ctrlKey ||
          event.shiftKey ||
          event.altKey ||
          !onOpenGuide
        ) {
          return;
        }
        event.preventDefault();
        onOpenGuide({ guide, group, info });
      });
      link.append(guideIcon(guide.kind));
      link.append(element("span", "unit-guide-label", guide.label));

      const open = document.createElement("a");
      open.className = "unit-guide-open";
      open.href = `./${guide.path}?v=${GUIDE_VERSION}`;
      open.target = "_blank";
      open.rel = "noopener";
      open.title = `Open ${guide.label} for ${info.title} in a new tab`;
      open.setAttribute("aria-label", `Open ${guide.label} for ${info.title} in a new tab`);
      open.textContent = "↗";

      row.append(link, open);
      block.append(row);
    }
    return block.children.length > 1 ? block : null;
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
    const guideBlock = renderGuides(group, unitInfo(group.id, group.label));
    if (guideBlock) body.append(guideBlock);
    return body;
  };

  const render = () => {
    list.replaceChildren();
    const activeId = select.value;

    for (const group of groups) {
      const info = unitInfo(group.id, group.label);
      const models = group.models;
      if (!models.length) continue;

      const unit = element("section", "unit");
      unit.dataset.unit = group.id;
      unit.dataset.open = openState.get(group.id) ? "true" : "false";

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
      head.append(element("span", "unit-count", String(models.length)));
      head.append(element("span", "unit-caret", "▾"));

      head.addEventListener("click", () => {
        const open = unit.dataset.open === "true";
        unit.dataset.open = open ? "false" : "true";
        head.setAttribute("aria-expanded", open ? "false" : "true");
        openState.set(group.id, !open);
      });

      unit.append(head, renderRows(group, models, activeId));
      list.append(unit);
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

  render();

  return {
    choose,
    markSelected,
    render,
  };
}

export function createGuideViewer({ tabs }) {
  const frame = $("guide-frame");
  const title = $("guide-title");
  const meta = $("guide-meta");
  const tabLabel = $("tab-guide-label");
  const closeButton = $("guide-close");
  if (!frame || !tabs) return { open() {}, close() {} };

  let previousTab = "source";
  let frameObserver = null;

  const fitFrame = () => {
    let doc = null;
    try {
      doc = frame.contentDocument;
    } catch {
      return;
    }
    const body = doc?.querySelector(".guide-body") || doc?.body;
    if (!body) return;
    const height = Math.ceil(body.getBoundingClientRect().height);
    if (height > 0) {
      const cap = Math.round(window.innerHeight * 0.85);
      frame.style.height = `${Math.min(height, cap)}px`;
      frame.style.maxHeight = "none";
    }
  };

  const syncFrame = () => {
    let doc = null;
    try {
      doc = frame.contentDocument;
    } catch {
      return;
    }
    if (!doc?.documentElement) return;
    doc.documentElement.classList.add("embedded");
    doc.documentElement.dataset.theme =
      document.documentElement.dataset.theme || "light";
    for (const link of doc.querySelectorAll('a[href^="http://"], a[href^="https://"]')) {
      link.setAttribute("target", "_blank");
      link.setAttribute("rel", "noopener noreferrer");
    }
    fitFrame();
    if (typeof ResizeObserver === "function") {
      frameObserver?.disconnect();
      frameObserver = null;
      const body = doc.querySelector(".guide-body") || doc.body;
      if (body) {
        frameObserver = new ResizeObserver(() => fitFrame());
        frameObserver.observe(body);
      }
    }
  };

  frame.addEventListener("load", syncFrame);
  window.addEventListener("resize", () => fitFrame());

  const themeObserver = new MutationObserver(() => {
    let doc = null;
    try {
      doc = frame.contentDocument;
    } catch {
      return;
    }
    if (doc?.documentElement) {
      doc.documentElement.dataset.theme =
        document.documentElement.dataset.theme || "light";
    }
  });
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-theme"],
  });

  const open = ({ guide, group, info }) => {
    if (!guide?.path) return;
    const current = tabs.current();
    if (current && current !== "guide") previousTab = current;
    if (title) title.textContent = guide.label;
    if (meta) {
      meta.textContent = `${group?.label || group?.id || "Unit"} · ${info?.title || ""}`.replace(
        / · $/,
        ""
      );
    }
    if (tabLabel) tabLabel.textContent = guide.label;
    frame.style.height = "";
    frame.style.maxHeight = "";
    frame.title = `${info?.title || group?.label || "Unit"} · ${guide.label}`;
    frame.src = `./${guide.path}?v=${GUIDE_VERSION}`;
    tabs.setGuideEnabled(true);
    tabs.activate("guide", { focus: true });
  };

  const close = () => {
    tabs.setGuideEnabled(false);
    const fallback = document.getElementById(`tab-${previousTab}`);
    tabs.activate(fallback && !fallback.hidden ? previousTab : "source", {
      focus: true,
    });
    frameObserver?.disconnect();
    frameObserver = null;
    frame.style.height = "";
    frame.style.maxHeight = "";
    frame.src = "about:blank";
    frame.title = "Unit guide";
    if (title) title.textContent = "Guide";
    if (meta) meta.textContent = "";
    if (tabLabel) tabLabel.textContent = "Guide";
  };

  closeButton?.addEventListener("click", close);

  return { open, close };
}

export function renderModelInfo({ model, unitId }) {
  const host = $("model-info");
  if (!host) return;
  host.replaceChildren();

  if (!model) {
    host.hidden = true;
    return;
  }

  host.hidden = false;

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

  host.append(kicker, title, description);
  if (tags.children.length) host.append(tags);

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
  let hasError = false;

  const matches = (line) => !filter || line.toLowerCase().includes(filter);

  const applyState = () => {
    for (const node of [count, meta]) {
      if (!node) continue;
      if (hasError) {
        node.dataset.state = "error";
        node.title = "Trace contains an error";
      } else {
        delete node.dataset.state;
        node.title = "";
      }
    }
  };

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
    applyState();
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
    hasError = Boolean(error);
    output.value = raw;
    output.classList.toggle("err", hasError);
    followPaused = false;
    render();
  };

  const clear = () => {
    raw = "";
    hasError = false;
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
  const marker = $("editor-line-marker");
  const parenStatus = $("paren-status");
  const errorStatus = $("error-status");
  if (!textarea || !highlight) {
    return {
      setSource() {},
      getSource: () => "",
      getOriginal: () => "",
      refresh() {},
      focus() {},
      setErrorLine() {},
      clearErrorLine() {},
      revealLine() {},
    };
  }

  let source = "";
  let pending = 0;
  let outlineOpen = true;
  let lines = [""];
  let offsets = [0];
  let structure = { pairs: new Map(), problems: [] };
  let problemIndices = new Set();
  let matchParens = new Set();
  let cursorFrame = 0;
  let errorLine = null;
  let errorMessage = "";
  let errorLabel = "Error";
  let lineHeight = 20;
  let paddingTop = 16;

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

  const tokenize = (text, matched = new Set(), problems = new Set()) => {
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
      if (char === "#" && text[index + 1] === "|") {
        let end = index + 2;
        let depth = 1;
        while (end < length && depth > 0) {
          if (text[end] === "#" && text[end + 1] === "|") {
            depth += 1;
            end += 2;
          } else if (text[end] === "|" && text[end + 1] === "#") {
            depth -= 1;
            end += 2;
          } else {
            end += 1;
          }
        }
        out += `<span class="tok-comment">${escape(text.slice(index, end))}</span>`;
        index = end;
        continue;
      }
      if (char === "|") {
        let end = index + 1;
        while (end < length && text[end] !== "|") end += 1;
        if (end < length) end += 1;
        out += `<span class="tok-string">${escape(text.slice(index, end))}</span>`;
        index = end;
        continue;
      }
      if (char === "(" || char === ")") {
        const classes = ["tok-paren"];
        if (matched.has(index)) classes.push("tok-paren-match");
        if (problems.has(index)) classes.push("tok-paren-bad");
        out += `<span class="${classes.join(" ")}">${char}</span>`;
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

  const lineOffsets = (lineList) => {
    const result = [];
    let offset = 0;
    for (const line of lineList) {
      result.push(offset);
      offset += line.length + 1;
    }
    return result;
  };

  const measure = () => {
    const style = getComputedStyle(textarea);
    lineHeight = parseFloat(style.lineHeight) || 20;
    paddingTop = parseFloat(style.paddingTop) || 0;
  };

  const lineStart = (line) => {
    const index = Math.max(0, Math.min(line - 1, offsets.length - 1));
    return offsets[index] ?? 0;
  };

  const positionMarker = () => {
    if (!marker) return;
    if (errorLine == null) {
      marker.hidden = true;
      return;
    }
    marker.hidden = false;
    marker.style.height = `${lineHeight}px`;
    marker.style.top = `${paddingTop + (errorLine - 1) * lineHeight - textarea.scrollTop}px`;
  };

  const syncScroll = () => {
    if (gutter) {
      gutter.style.transform = `translateY(${-textarea.scrollTop}px)`;
    }
    highlightCode.style.transform = `translate(${-textarea.scrollLeft}px, ${-textarea.scrollTop}px)`;
    positionMarker();
  };

  const renderHighlight = () => {
    highlight.innerHTML = textarea.value.length
      ? tokenize(textarea.value, matchParens, problemIndices)
      : "";
  };

  const renderGutter = () => {
    if (!gutter) return;
    const fragment = document.createDocumentFragment();
    for (let i = 1; i <= lines.length; i += 1) {
      const row = element("div", "editor-gutter-line", String(i));
      if (errorLine === i) row.classList.add("is-error");
      fragment.append(row);
    }
    gutter.replaceChildren(fragment);
  };

  const renderDiagnostics = () => {
    const problems = structure.problems;
    if (parenStatus) {
      parenStatus.hidden = problems.length === 0;
      if (problems.length) {
        const first = problems[0];
        parenStatus.textContent =
          problems.length === 1
            ? `Unmatched ${first.char} · line ${first.line}`
            : `${problems.length} unmatched parens · line ${first.line}`;
        parenStatus.title = problems
          .map((problem) => `line ${problem.line}: unmatched ${problem.char}`)
          .join("\n");
      }
    }
    if (errorStatus) {
      errorStatus.hidden = errorLine == null;
      if (errorLine != null) {
        errorStatus.textContent = `${errorLabel} · line ${errorLine}`;
        errorStatus.title = errorMessage;
      }
    }
    if (problems.length || errorLine != null) {
      textarea.setAttribute("aria-invalid", "true");
    } else {
      textarea.removeAttribute("aria-invalid");
    }
  };

  const renderOutline = (lineList, lineOffsetsList) => {
    if (!outline) return;
    outline.replaceChildren();

    const groups = new Map();
    lineList.forEach((line, index) => {
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
      groups
        .get(label)
        .push({ name, line: index + 1, offset: lineOffsetsList[index] ?? 0, kind });
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
          revealLine(item.line, { scrollPage: true });
        });
        group.append(button);
      }
      outline.append(group);
    }
    outline.hidden = !outlineOpen;
  };

  const revealLine = (line, { scrollPage = false } = {}) => {
    const target = Math.max(1, Math.min(Math.round(line), Math.max(1, lines.length)));
    const top = (target - 1) * lineHeight;
    const view = textarea.clientHeight || 0;
    textarea.focus({ preventScroll: !scrollPage });
    const offset = lineStart(target);
    textarea.setSelectionRange(offset, offset);
    if (top < textarea.scrollTop || top + lineHeight > textarea.scrollTop + view) {
      textarea.scrollTop = Math.max(0, top - view / 2 + lineHeight / 2);
    }
    if (scrollPage) {
      const header = document.querySelector(".app-header");
      const bar = document.querySelector(".run-bar");
      const margin = (header?.offsetHeight || 0) + (bar?.offsetHeight || 0) + 16;
      const rect = textarea.getBoundingClientRect();
      const lineY = rect.top + paddingTop + (target - 1) * lineHeight - textarea.scrollTop;
      if (lineY < margin + lineHeight || lineY > window.innerHeight - 24) {
        window.scrollBy({ top: lineY - margin - 24, behavior: "auto" });
      }
    }
    syncScroll();
    updateCursorMatch();
  };

  const clearErrorLine = () => {
    if (errorLine == null) return;
    errorLine = null;
    errorMessage = "";
    errorLabel = "Error";
    if (marker) marker.hidden = true;
    renderGutter();
    renderDiagnostics();
  };

  const setErrorLine = (line, message = "", label = "Error") => {
    if (!Number.isFinite(line) || line < 1) {
      clearErrorLine();
      return;
    }
    const total = Math.max(1, lines.length);
    let target = Math.max(1, Math.min(Math.round(line), total));
    errorMessage = String(message || "");
    errorLabel = label;
    const lower = errorMessage.toLowerCase();
    if (lower.includes("unterminated list") || lower.includes("unexpected end of input")) {
      const opens = structure.problems.filter((problem) => problem.char === "(");
      if (opens.length) target = opens[opens.length - 1].line;
    } else if (errorMessage.includes("unexpected `)`")) {
      const close = structure.problems.find((problem) => problem.char === ")");
      if (close) target = close.line;
    }
    errorLine = Math.max(1, Math.min(target, total));
    renderGutter();
    renderDiagnostics();
    positionMarker();
    revealLine(errorLine, { scrollPage: true });
  };

  const updateCursorMatch = () => {
    let next = new Set();
    if (
      document.activeElement === textarea &&
      textarea.selectionStart === textarea.selectionEnd
    ) {
      const text = textarea.value;
      const cursor = textarea.selectionStart;
      const index = cursor - 1;
      const char = text[index];
      if ((char === "(" || char === ")") && !problemIndices.has(index)) {
        const match = structure.pairs.get(index);
        if (match != null) next = new Set([index, match]);
      }
    }
    const same =
      next.size === matchParens.size &&
      [...next].every((index) => matchParens.has(index));
    if (!same) {
      matchParens = next;
      renderHighlight();
    }
  };

  const scheduleCursorMatch = () => {
    if (cursorFrame) return;
    cursorFrame = window.requestAnimationFrame(() => {
      cursorFrame = 0;
      updateCursorMatch();
    });
  };

  const refresh = () => {
    pending = 0;
    const text = textarea.value;
    lines = text.split("\n");
    offsets = lineOffsets(lines);
    measure();
    structure = scanParens(text);
    problemIndices = new Set(structure.problems.map((problem) => problem.index));
    if (errorLine != null) {
      errorLine = Math.max(1, Math.min(errorLine, lines.length));
    }
    renderGutter();
    renderHighlight();
    if (meta) {
      meta.textContent = text
        ? `${lines.length} lines · ${text.length.toLocaleString()} chars`
        : "No model loaded";
    }
    renderOutline(lines, offsets);
    renderDiagnostics();
    positionMarker();
    updateCursorMatch();
  };

  const scheduleRefresh = () => {
    if (pending) return;
    pending = window.requestAnimationFrame(refresh);
  };

  textarea.addEventListener("input", () => {
    clearErrorLine();
    scheduleRefresh();
    onChange?.(textarea.value);
  });
  textarea.addEventListener("scroll", syncScroll, { passive: true });
  textarea.addEventListener("keyup", scheduleCursorMatch);
  textarea.addEventListener("click", scheduleCursorMatch);
  textarea.addEventListener("select", scheduleCursorMatch);
  textarea.addEventListener("focus", scheduleCursorMatch);
  textarea.addEventListener("blur", () => {
    if (!matchParens.size) return;
    matchParens = new Set();
    renderHighlight();
  });
  document.addEventListener("selectionchange", () => {
    if (document.activeElement === textarea) scheduleCursorMatch();
  });

  parenStatus?.addEventListener("click", () => {
    const first = structure.problems[0];
    if (first) revealLine(first.line, { scrollPage: true });
  });
  errorStatus?.addEventListener("click", () => {
    if (errorLine != null) revealLine(errorLine, { scrollPage: true });
  });

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
    errorLine = null;
    errorMessage = "";
    errorLabel = "Error";
    matchParens = new Set();
    if (marker) marker.hidden = true;
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
    setErrorLine,
    clearErrorLine,
    revealLine,
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

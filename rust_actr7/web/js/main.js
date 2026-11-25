import { Actr, RunCancelled, loadWasm, nextPaint } from "./actr.js";
import { AgiWindow } from "./agi-window.js";
import { TASKS } from "./tasks/index.js";
import { RECOMMENDED_START } from "./catalog.js";
import { locateErrorAnchor } from "./lisp-scan.js";
import {
  createEditor,
  createGuideViewer,
  createModelBrowser,
  renderAboutContent,
  renderModelInfo,
  setRunState,
  setStatus,
  setTaskHeader,
  setupDialogs,
  setupGlobalShortcuts,
  setupTabs,
  setupTheme,
  setupToolbar,
  setupTraceView,
  setupWelcome,
  showToast,
} from "./ui.js";

const $ = (id) => document.getElementById(id);
const controlState = new Map();

let wasm = null;
let models = new Map();
let selectedModel = null;
let modelReady = false;
let modelRequest = 0;
let activeRun = null;
let browser = null;
let tabs = null;
let guideViewer = null;

const editor = createEditor();
const trace = setupTraceView();

function taskDescriptor(model) {
  return model?.task ? TASKS[model.task] || null : null;
}

function parseErrorLine(error) {
  const message =
    typeof error === "string" ? error : error?.message || String(error || "");
  const match = message.match(/parse error at line (\d+):/i);
  return match ? Number(match[1]) : null;
}

function runTimeModeFor(model) {
  const descriptor = taskDescriptor(model);
  if (!descriptor) return "used";
  const mode = descriptor.runTimeMode;
  if (typeof mode === "function") {
    return mode(controlStateFor(model.task, descriptor)) || "used";
  }
  return mode || "used";
}

function updateRunTimeControl(model = selectedModel) {
  const mode = runTimeModeFor(model);
  const input = $("secs");
  const label = $("run-time-label-text");
  const help = $("run-time-help");
  input.disabled = mode === "unused";
  help.hidden = mode !== "unused";
  label.textContent = mode === "per-trial" ? "Trial time (s)" : "Run time (s)";
}

function updateRunButton() {
  $("run").disabled = !(wasm && modelReady);
}

function summaryText(result) {
  if (!result) return "";
  if (result.kind === "demo2") {
    if (result.response == null) return `Target: ${result.target} - no response`;
    return `Target: ${result.target} - response: ${result.response} - ${result.correct ? "correct" : "incorrect"}`;
  }
  if (result.kind === "assignment") {
    if (result.response == null) {
      return `Target: ${result.target} - no response${result.mode === "human" ? "" : " (starter model)"}`;
    }
    return `Target: ${result.target} - response: ${result.response} - ${result.correct ? "true" : "false"}`;
  }
  if (result.kind === "pm-issues") {
    return `${result.mode === "human" ? "Human - " : ""}Prompt: ${result.prompt} - letter: ${result.letter} - response: ${result.response.join(" ") || "none"} - ${result.correct ? "correct" : "incorrect"}`;
  }
  if (result.kind === "sperling") {
    return `Sperling${result.mode === "human" ? " (human)" : ""}: ${result.data.length} onset conditions`;
  }
  if (result.kind === "subitize") {
    return `Subitize${result.mode === "human" ? " (human)" : ""}: ${result.results.length} trial(s)`;
  }
  if (result.kind === "fan" || result.kind === "fan-no-pm") {
    return `Fan${result.mode === "human" ? " (human)" : ""}: correlation ${Number(result.correlation).toFixed(3)} - mean deviation ${Number(result.meanDeviation).toFixed(3)}`;
  }
  if (result.kind === "grouped") {
    if (result.mode === "human") {
      return `Grouped Recall (human): ${result.correct}/${result.total} correct`;
    }
    return `Grouped Recall: ${result.responses.length} response(s)`;
  }
  if (result.kind === "siegler") return `Siegler: correlation ${Number(result.correlation).toFixed(3)}`;
  if (result.kind === "bst") {
    return `${result.label || "Building Sticks"}: ${result.mode === "human" ? "human" : "experiment"} complete`;
  }
  if (result.kind === "choice") return "Choice: experiment complete";
  if (result.kind === "paired") {
    return result.mode === "human"
      ? "Paired Associate: human run complete"
      : "Paired Associate: model run complete";
  }
  if (result.kind === "zbrodoff") {
    const who = result.human ? "human " : "";
    if (result.mode === "problem") return `Alpha-Arithmetic: ${who}single problem complete`;
    if (result.mode === "set") return `Alpha-Arithmetic: ${who}set complete`;
    if (result.mode === "block") return `Alpha-Arithmetic: ${who}block complete`;
    return `Alpha-Arithmetic: ${who}experiment complete`;
  }
  if (result.kind === "past-tense") return "Past Tense: block report complete";
  if (result.kind === "ul-issues") {
    return result.mode === "human"
      ? `Utility Learning: human run complete (${result.trials} trials)`
      : "Utility Learning: run complete";
  }
  if (result.kind === "pcomp-issues") {
    return result.mode === "human"
      ? "Production Compilation: human game complete"
      : "Production Compilation: games complete";
  }
  if (result.kind === "categorize") {
    return `Categorize: ${result.answered}/${result.total} responded`;
  }
  if (result.kind === "semantic-dynamic") {
    return result.mode === "human"
      ? "Semantic Network: human run complete"
      : "Semantic Network: run complete";
  }
  if (result.kind === "onehit") {
    return result.mode === "play" ? "1-Hit Blackjack: played against model" : "1-Hit Blackjack: learning run complete";
  }
  return "";
}

function clearTaskResult() {
  $("task-result").textContent = "";
  $("task-report").textContent = "";
  $("task-report").hidden = true;
  clearTaskStage();
}

function clearTaskStage() {
  const stage = $("task-stage");
  if (!stage) return;
  stage.replaceChildren();
  stage.className = "task-stage";
  stage.hidden = true;
}

function renderResult(result) {
  $("task-result").textContent = summaryText(result);
  const report = result?.report || "";
  $("task-report").textContent = report;
  $("task-report").hidden = !report;
}

function controlStateFor(taskId, descriptor) {
  if (!controlState.has(taskId)) {
    const values = {};
    for (const control of descriptor?.controls || []) {
      values[control.id] = control.default;
    }
    controlState.set(taskId, values);
  }
  return controlState.get(taskId);
}

function controlApplies(control, values) {
  if (typeof control.showWhen === "function") return Boolean(control.showWhen(values));
  if (Array.isArray(control.showWhen)) return control.showWhen.includes(values[control.id]);
  return true;
}

function syncTaskControlVisibility(taskId, descriptor) {
  const values = controlStateFor(taskId, descriptor);
  for (const wrapper of $("task-controls").querySelectorAll(".task-control")) {
    const control = descriptor?.controls?.find((item) => item.id === wrapper.dataset.controlId);
    if (control) wrapper.hidden = !controlApplies(control, values);
  }
  if (selectedModel) updateRunTimeControl(selectedModel);
}

function renderTaskControls(taskId, descriptor) {
  const host = $("task-controls");
  host.replaceChildren();
  if (!taskId || !descriptor?.controls?.length) {
    host.hidden = true;
    return;
  }

  const values = controlStateFor(taskId, descriptor);
  host.hidden = false;
  for (const control of descriptor.controls) {
    const wrapper = document.createElement("div");
    wrapper.className = "task-control";
    wrapper.dataset.controlId = control.id;
    const label = document.createElement("label");
    label.htmlFor = `task-control-${taskId}-${control.id}`;
    label.textContent = control.label;

    let input;
    if (control.type === "select") {
      input = document.createElement("select");
      for (const [value, text] of control.options || []) {
        const option = document.createElement("option");
        option.value = String(value);
        option.textContent = String(text);
        input.append(option);
      }
      input.value = String(values[control.id] ?? control.default);
      input.addEventListener("change", () => {
        values[control.id] = input.value;
        clearTaskResult();
        syncTaskControlVisibility(taskId, descriptor);
      });
    } else if (control.type === "checkbox") {
      input = document.createElement("input");
      input.type = "checkbox";
      input.checked = Boolean(values[control.id]);
      input.addEventListener("change", () => {
        values[control.id] = input.checked;
        clearTaskResult();
        syncTaskControlVisibility(taskId, descriptor);
      });
    } else if (control.type === "text") {
      input = document.createElement("input");
      input.type = "text";
      input.value = String(values[control.id] ?? control.default);
      input.addEventListener("input", () => {
        values[control.id] = input.value;
        clearTaskResult();
        syncTaskControlVisibility(taskId, descriptor);
      });
    } else {
      input = document.createElement("input");
      input.type = "number";
      if (control.min != null) input.min = String(control.min);
      if (control.max != null) input.max = String(control.max);
      if (control.step != null) input.step = String(control.step);
      input.value = String(values[control.id] ?? control.default);
      input.addEventListener("input", () => {
        values[control.id] = input.value;
        clearTaskResult();
        syncTaskControlVisibility(taskId, descriptor);
      });
    }

    input.id = `task-control-${taskId}-${control.id}`;
    wrapper.append(label, input);
    host.append(wrapper);
  }
  syncTaskControlVisibility(taskId, descriptor);
}

function readControls(taskId, descriptor) {
  const source = controlStateFor(taskId, descriptor);
  const values = {};
  for (const control of descriptor?.controls || []) {
    const value = source[control.id] ?? control.default;
    if (control.type === "number") {
      values[control.id] = Number(value);
    } else if (control.type === "checkbox") {
      values[control.id] = Boolean(value);
    } else {
      values[control.id] = String(value);
    }
  }
  return values;
}

function setControlsDisabled(disabled) {
  for (const input of $("task-controls").querySelectorAll("input, select")) {
    input.disabled = disabled;
  }
}

function showTaskPanel(model) {
  const descriptor = taskDescriptor(model);
  $("task-panel").hidden = !descriptor;
  clearTaskResult();
  const agiWindow = new AgiWindow($("agi-window"));
  agiWindow.reset();
  renderTaskControls(model?.task, descriptor);
  if (descriptor?.runSeconds != null) {
    $("secs").value = String(descriptor.runSeconds);
  }
  updateRunTimeControl(model);
  setTaskHeader({ model, descriptor });
  tabs.setTaskEnabled(Boolean(descriptor));
  tabs.activate(descriptor ? "task" : "source");
}

function progress(text) {
  if (activeRun) setStatus(text, "running");
}

async function runTask(model, descriptor, source, seed0, seed1, secs, signal) {
  const agiWindow = new AgiWindow($("agi-window"));
  const actr = new Actr(wasm, seed0, seed1, agiWindow);
  const controls = readControls(model.task, descriptor);
  const prelude =
    typeof descriptor.prelude === "function"
      ? descriptor.prelude(controls)
      : descriptor.prelude;
  const preludeLineCount = prelude ? prelude.split("\n").length : 0;
  const taskSource = prelude ? `${prelude}\n${source}` : source;
  try {
    try {
      actr.loadSource(taskSource);
    } catch (loadError) {
      const parsed = parseErrorLine(loadError);
      if (parsed != null) {
        loadError.engineLine = parsed;
        if (parsed > preludeLineCount) {
          loadError.sourceLine = parsed - preludeLineCount;
        } else {
          loadError.setupError = true;
        }
      }
      throw loadError;
    }
    const result = await descriptor.run(actr, {
      secs,
      runSeconds: secs,
      controls,
      paint: nextPaint,
      stage: $("task-stage"),
      signal,
      progress,
    });
    renderResult(result);
    return { ok: true, out: actr.trace.join("\n") };
  } catch (e) {
    if (e instanceof RunCancelled || e?.name === "RunCancelled") {
      return { ok: false, cancelled: true, out: actr.trace.join("\n") };
    }
    throw e;
  }
}

async function runGenericModel(source, seed0, seed1, secs, signal) {
  const actr = new Actr(wasm, seed0, seed1, null);
  try {
    try {
      actr.loadSource(source);
    } catch (loadError) {
      const parsed = parseErrorLine(loadError);
      if (parsed != null) loadError.sourceLine = parsed;
      throw loadError;
    }
    await actr.run(secs, { signal, paint: false });
    return { ok: true, out: actr.trace.join("\n") };
  } catch (e) {
    if (e instanceof RunCancelled || e?.name === "RunCancelled") {
      return { ok: false, cancelled: true, out: actr.trace.join("\n") };
    }
    throw e;
  }
}

async function run() {
  if (!wasm || !selectedModel || activeRun) return;
  const model = selectedModel;
  const descriptor = taskDescriptor(model);
  const source = $("model").value;
  const seed0 = Math.trunc(Number($("seed0").value) || 0);
  const seed1 = Math.trunc(Number($("seed1").value) || 0);
  const secs = Number($("secs").value) || 0;
  const controller = new AbortController();
  activeRun = controller;

  $("run").disabled = true;
  $("cancel").hidden = false;
  $("cancel").disabled = false;
  clearTaskResult();
  setControlsDisabled(true);
  setRunState(true);
  setStatus("running...", "running");
  editor.clearErrorLine();
  if (!descriptor && tabs.current() === "source") tabs.activate("trace");
  const t0 = performance.now();
  try {
    const result = descriptor
      ? await runTask(model, descriptor, source, seed0, seed1, secs, controller.signal)
      : await runGenericModel(source, seed0, seed1, secs, controller.signal);
    trace.set(result.out, { error: !result.ok && !result.cancelled });
    if (!result.cancelled) editor.clearErrorLine();
    const ms = Math.round(performance.now() - t0);
    const lines = result.out ? result.out.split("\n").length : 0;
    if (result.cancelled) {
      setStatus(`cancelled - ${ms} ms - ${lines} lines`, "cancelled");
    } else {
      setStatus(`${result.ok ? "ok" : "error"} - ${ms} ms - ${lines} lines`, result.ok ? "ok" : "error");
    }
  } catch (e) {
    const message = e?.message || String(e);
    trace.set("error: " + message, { error: true });
    setStatus("error", "error");
    let line = Number.isFinite(e?.sourceLine)
      ? e.sourceLine
      : e?.setupError
        ? null
        : parseErrorLine(message);
    let label = "Parse error";
    if (!line && !e?.setupError) {
      const anchor = locateErrorAnchor(message, $("model").value);
      if (anchor) {
        line = anchor.line;
        label = "Related line";
      }
    }
    if (line) {
      tabs.activate("source");
      editor.setErrorLine(line, message, label);
      showToast(
        label === "Parse error"
          ? `Parse error on line ${line}`
          : `Error near line ${line}`
      );
    } else if (e?.setupError) {
      editor.clearErrorLine();
      showToast("Error in the task setup; see the trace");
    } else {
      editor.clearErrorLine();
    }
  } finally {
    activeRun = null;
    $("cancel").hidden = true;
    setControlsDisabled(false);
    setRunState(false);
    updateRunButton();
  }
}

async function loadSelectedModel() {
  const select = $("model-preset");
  if (activeRun) {
    if (selectedModel) select.value = selectedModel.id;
    return;
  }
  const option = select.selectedOptions[0];
  const model = option ? models.get(option.value) : null;
  if (!model) return;

  const request = ++modelRequest;
  modelReady = false;
  updateRunButton();
  setStatus("loading model...", "running");
  trace.clear();
  $("reset-source").disabled = true;

  try {
    const res = await fetch(`./models/${model.path}`, { cache: "no-store" });
    if (!res.ok) {
      throw new Error(`fetch models/${model.path}: ${res.status} ${res.statusText}`);
    }
    const source = await res.text();
    if (request !== modelRequest) return;
    editor.setSource(source);
    $("reset-source").disabled = false;
    selectedModel = model;
    modelReady = true;
    showTaskPanel(model);
    browser?.markSelected(model.id);
    renderModelInfo({ model, unitId: model.unitId, descriptor: taskDescriptor(model) });
    setStatus("", "idle");
  } catch (e) {
    if (request !== modelRequest) return;
    selectedModel = null;
    modelReady = false;
    $("reset-source").disabled = true;
    tabs.setTaskEnabled(false);
    tabs.activate("trace");
    $("task-panel").hidden = true;
    browser?.markSelected("");
    renderModelInfo({ model: null });
    setStatus("model failed to load", "error");
    trace.set(String(e) + "\n\nDid you run web/build.py --models-only?", { error: true });
  } finally {
    if (request === modelRequest) updateRunButton();
  }
}

async function loadModels() {
  const select = $("model-preset");
  const [res, guideRes] = await Promise.all([
    fetch("./models/manifest.json", { cache: "no-store" }),
    fetch("./guides/manifest.json", { cache: "no-store" }).catch(() => null),
  ]);
  if (!res.ok) {
    throw new Error(`fetch models/manifest.json: ${res.status} ${res.statusText}`);
  }
  const manifest = await res.json();
  const groups = Array.isArray(manifest.groups) ? manifest.groups : [];
  if (!groups.length) throw new Error("model manifest has no groups");
  let guides = {};
  if (guideRes?.ok) {
    try {
      guides = await guideRes.json();
    } catch {
      guides = {};
    }
  }

  select.replaceChildren();
  models = new Map();
  for (const group of groups) {
    const optgroup = document.createElement("optgroup");
    optgroup.label = String(group.label || group.id);
    for (const model of Array.isArray(group.models) ? group.models : []) {
      if (!model || !model.id || !model.path) continue;
      const entry = { ...model, unitId: group.id };
      const option = document.createElement("option");
      option.value = String(model.id);
      option.textContent = String(model.label || model.id);
      models.set(option.value, entry);
      optgroup.append(option);
    }
    if (optgroup.children.length) select.append(optgroup);
  }
  if (!select.options.length) throw new Error("model manifest has no models");

  select.disabled = false;
  const defaultId = String(manifest.default || "");
  if ([...select.options].some((option) => option.value === defaultId)) {
    select.value = defaultId;
  } else {
    select.selectedIndex = 0;
  }

  browser = createModelBrowser({
    groups,
    guides,
    onOpenGuide: (payload) => guideViewer?.open(payload),
  });
  await loadSelectedModel();
  browser.markSelected(selectedModel?.id || select.value);
}

(async function main() {
  setupTheme();
  setupDialogs();
  renderAboutContent();
  tabs = setupTabs();
  guideViewer = createGuideViewer({ tabs });
  setupToolbar({ editor, trace, getModelId: () => selectedModel?.id });
  setupGlobalShortcuts({ run: () => void run(), trace });

  setupWelcome({
    onStart: () => browser?.choose(RECOMMENDED_START),
    onExplore: () => browser?.focusSearch(),
  });

  $("model-preset").addEventListener("change", () => {
    void loadSelectedModel();
  });

  $("reset-source").addEventListener("click", () => {
    editor.setSource(editor.getOriginal());
    showToast("Model source restored");
  });

  try {
    setStatus("loading wasm...", "running");
    wasm = await loadWasm();
  } catch (e) {
    setStatus("wasm failed to load", "error");
    trace.set(
      String(e) +
        "\n\nDid you run web/build.py, and serve this folder over HTTP?\n" +
        "  cd web && python -m http.server 8000",
      { error: true }
    );
    tabs.activate("trace");
    return;
  }

  try {
    setStatus("loading models...", "running");
    await loadModels();
  } catch (e) {
    setStatus("models failed to load", "error");
    trace.set(String(e) + "\n\nDid you run web/build.py --models-only?", { error: true });
    tabs.activate("trace");
    return;
  }

  setStatus(modelReady ? "" : "model failed to load", modelReady ? "idle" : "error");
  updateRunButton();
  $("run").addEventListener("click", () => {
    void run();
  });
  $("cancel").addEventListener("click", () => {
    if (!activeRun) return;
    $("cancel").disabled = true;
    setStatus("cancelling...", "running");
    activeRun.abort();
  });
})();

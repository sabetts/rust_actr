import { Actr, RunCancelled, loadWasm, nextPaint } from "./actr.js";
import { AgiWindow } from "./agi-window.js";
import { TASKS } from "./tasks/index.js";

const $ = (id) => document.getElementById(id);
const controlState = new Map();

let wasm = null;
let models = new Map();
let selectedModel = null;
let modelReady = false;
let modelRequest = 0;
let activeRun = null;

function taskDescriptor(model) {
  return model?.task ? TASKS[model.task] || null : null;
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
    if (result.response == null) return `Target: ${result.target} - no response (starter model)`;
    return `Target: ${result.target} - response: ${result.response} - ${result.correct ? "true" : "false"}`;
  }
  if (result.kind === "pm-issues") {
    return `Prompt: ${result.prompt} - letter: ${result.letter} - response: ${result.response.join(" ") || "none"} - ${result.correct ? "correct" : "incorrect"}`;
  }
  if (result.kind === "sperling") return `Sperling: ${result.data.length} onset conditions`;
  if (result.kind === "subitize") return `Subitize: ${result.results.length} trial(s)`;
  if (result.kind === "fan" || result.kind === "fan-no-pm") {
    return `Fan: correlation ${Number(result.correlation).toFixed(3)} - mean deviation ${Number(result.meanDeviation).toFixed(3)}`;
  }
  if (result.kind === "grouped") return `Grouped Recall: ${result.responses.length} response(s)`;
  if (result.kind === "siegler") return `Siegler: correlation ${Number(result.correlation).toFixed(3)}`;
  if (result.kind === "bst") return "Building Sticks: experiment complete";
  if (result.kind === "choice") return "Choice: experiment complete";
  if (result.kind === "onehit") {
    return result.mode === "play" ? "1-Hit Blackjack: played against model" : "1-Hit Blackjack: learning run complete";
  }
  return "";
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
        syncTaskControlVisibility(taskId, descriptor);
      });
    } else if (control.type === "checkbox") {
      input = document.createElement("input");
      input.type = "checkbox";
      input.checked = Boolean(values[control.id]);
      input.addEventListener("change", () => {
        values[control.id] = input.checked;
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
  $("task-result").textContent = "";
  $("task-report").textContent = "";
  $("task-report").hidden = true;
  const window = new AgiWindow($("agi-window"));
  window.reset();
  const stage = $("task-stage");
  if (stage) {
    stage.replaceChildren();
    stage.hidden = true;
  }
  renderTaskControls(model?.task, descriptor);
  if (descriptor?.runSeconds != null) {
    $("secs").value = String(descriptor.runSeconds);
  }
}

function progress(text) {
  if (activeRun) $("status").textContent = text;
}

async function runTask(model, descriptor, source, seed0, seed1, secs, signal) {
  const window = new AgiWindow($("agi-window"));
  const actr = new Actr(wasm, seed0, seed1, window);
  const taskSource = descriptor.prelude ? `${descriptor.prelude}\n${source}` : source;
  try {
    actr.loadSource(taskSource);
    const controls = readControls(model.task, descriptor);
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
    actr.loadSource(source);
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
  setControlsDisabled(true);
  $("status").textContent = "running...";
  const t0 = performance.now();
  try {
    const result = descriptor
      ? await runTask(model, descriptor, source, seed0, seed1, secs, controller.signal)
      : await runGenericModel(source, seed0, seed1, secs, controller.signal);
    $("output").value = result.out;
    $("output").classList.toggle("err", !result.ok && !result.cancelled);
    const ms = Math.round(performance.now() - t0);
    const lines = result.out ? result.out.split("\n").length : 0;
    $("status").textContent = result.cancelled
      ? `cancelled - ${ms} ms - ${lines} lines`
      : `${result.ok ? "ok" : "error"} - ${ms} ms - ${lines} lines`;
  } catch (e) {
    $("output").value = "error: " + e;
    $("output").classList.add("err");
    $("status").textContent = "error";
  } finally {
    activeRun = null;
    $("cancel").hidden = true;
    setControlsDisabled(false);
    updateRunButton();
  }
}

async function loadSelectedModel() {
  const select = $("model-preset");
  const option = select.selectedOptions[0];
  const model = option ? models.get(option.value) : null;
  if (!model) return;

  const request = ++modelRequest;
  modelReady = false;
  updateRunButton();
  $("status").textContent = "loading model...";
  $("output").value = "";
  $("output").classList.remove("err");

  try {
    const res = await fetch(`./models/${model.path}`, { cache: "no-store" });
    if (!res.ok) {
      throw new Error(`fetch models/${model.path}: ${res.status} ${res.statusText}`);
    }
    const source = await res.text();
    if (request !== modelRequest) return;
    $("model").value = source;
    selectedModel = model;
    modelReady = true;
    showTaskPanel(model);
    $("status").textContent = "";
  } catch (e) {
    if (request !== modelRequest) return;
    selectedModel = null;
    $("status").textContent = "model failed to load";
    $("output").value = String(e) + "\n\nDid you run web/build.py --models-only?";
    $("output").classList.add("err");
  } finally {
    if (request === modelRequest) updateRunButton();
  }
}

async function loadModels() {
  const select = $("model-preset");
  const res = await fetch("./models/manifest.json", { cache: "no-store" });
  if (!res.ok) {
    throw new Error(`fetch models/manifest.json: ${res.status} ${res.statusText}`);
  }
  const manifest = await res.json();
  const groups = Array.isArray(manifest.groups) ? manifest.groups : [];
  if (!groups.length) throw new Error("model manifest has no groups");

  select.replaceChildren();
  models = new Map();
  for (const group of groups) {
    const optgroup = document.createElement("optgroup");
    optgroup.label = String(group.label || group.id);
    for (const model of Array.isArray(group.models) ? group.models : []) {
      if (!model || !model.id || !model.path) continue;
      const option = document.createElement("option");
      option.value = String(model.id);
      option.textContent = String(model.label || model.id);
      models.set(option.value, model);
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
  await loadSelectedModel();
}

(async function main() {
  $("model-preset").addEventListener("change", () => {
    void loadSelectedModel();
  });

  try {
    $("status").textContent = "loading wasm...";
    wasm = await loadWasm();
  } catch (e) {
    $("status").textContent = "wasm failed to load";
    $("output").value =
      String(e) + "\n\nDid you run web/build.py, and serve this folder over HTTP?\n" +
      "  cd web && python -m http.server 8000";
    $("output").classList.add("err");
    return;
  }

  try {
    $("status").textContent = "loading models...";
    await loadModels();
  } catch (e) {
    $("status").textContent = "models failed to load";
    $("output").value = String(e) + "\n\nDid you run web/build.py --models-only?";
    $("output").classList.add("err");
    return;
  }

  $("status").textContent = modelReady ? "" : "model failed to load";
  updateRunButton();
  $("run").addEventListener("click", () => {
    void run();
  });
  $("cancel").addEventListener("click", () => {
    if (!activeRun) return;
    $("cancel").disabled = true;
    $("status").textContent = "cancelling...";
    activeRun.abort();
  });
  document.addEventListener("keydown", (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
      e.preventDefault();
      void run();
    }
  });
})();

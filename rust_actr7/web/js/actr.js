const encoder = new TextEncoder();
const decoder = new TextDecoder();

let wasmMemory = null;
const commandFns = new Map();
let hookError = null;

function resetHookState() {
  commandFns.clear();
  hookError = null;
}

function decodeString(ptr, len) {
  if (!len || !wasmMemory) return "";
  return decoder.decode(new Uint8Array(wasmMemory.buffer, ptr, len));
}

function hookValue(kind, num, ptr, len) {
  switch (kind) {
    case 1:
    case 4:
      return decodeString(ptr, len);
    case 2:
    case 3:
      return num;
    default:
      return null;
  }
}

function taggedValue(value) {
  if (!value) return null;
  switch (value.k) {
    case "sym":
    case "str":
      return value.v;
    case "int":
    case "num":
      return value.v;
    default:
      return null;
  }
}

function recordHookError(error) {
  if (hookError == null) hookError = error;
}

function hookResult(value) {
  if (value == null || typeof value === "boolean") return Number.NaN;
  const result = Number(value);
  return Number.isFinite(result) ? result : Number.NaN;
}

function callSimHook(
  namePtr,
  nameLen,
  aKind,
  aNum,
  aPtr,
  aLen,
  bKind,
  bNum,
  bPtr,
  bLen
) {
  try {
    const name = decodeString(namePtr, nameLen).toUpperCase();
    const fn = commandFns.get(name);
    if (!fn) return Number.NaN;
    const a = hookValue(aKind, aNum, aPtr, aLen);
    const b = hookValue(bKind, bNum, bPtr, bLen);
    return hookResult(fn(a, b));
  } catch (error) {
    recordHookError(error);
    return Number.NaN;
  }
}

function callPpmHook(namePtr, nameLen, payloadPtr, payloadLen) {
  try {
    const name = decodeString(namePtr, nameLen).toUpperCase();
    const fn = commandFns.get(name);
    if (!fn) return Number.NaN;
    const payload = JSON.parse(decodeString(payloadPtr, payloadLen));
    const rows = (payload.mismatches || []).map((row) => [
      row.buffer,
      row.slot,
      taggedValue(row.expected),
      taggedValue(row.actual),
      row.similarity,
    ]);
    return hookResult(fn(payload.production, rows));
  } catch (error) {
    recordHookError(error);
    return Number.NaN;
  }
}

function actionResultText(value) {
  if (value == null) return "";
  if (typeof value === "string") return value;
  if (Array.isArray(value)) {
    return "(" + value.map((item) => String(item)).join(" ") + ")";
  }
  return String(value);
}

function nameOrEmpty(value) {
  if (value == null) return "";
  const text = String(value);
  return text.toUpperCase() === "NIL" ? "" : text.toUpperCase();
}

function first(values) {
  return Array.isArray(values) && values.length ? values[0] : null;
}

function jsonFloat(value) {
  const text = String(Number(value));
  return /[.eE]/.test(text) ? text : `${text}.0`;
}

function optionPairs(options) {
  if (Array.isArray(options)) return options;
  return Object.entries(options || {});
}

function flatPairs(pairs) {
  if (Array.isArray(pairs)) return pairs;
  return Object.entries(pairs || {}).flatMap(([key, value]) => [key, value]);
}

function chunkSpec(spec) {
  if (Array.isArray(spec) || typeof spec === "string") return spec;
  return Object.entries(spec || {}).flatMap(([key, value]) => [key, value]);
}

function commandValue(value) {
  if (typeof value !== "string") return value;
  if (value.length >= 2 && value.startsWith('"') && value.endsWith('"')) {
    return value.slice(1, -1).replace(/\\"/g, '"');
  }
  return value;
}

export async function loadWasm(url = "./actr_engine.wasm") {
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`fetch actr_engine.wasm: ${res.status} ${res.statusText}`);
  }
  const imports = {
    env: {
      js_sim_hook: callSimHook,
      js_ppm_hook: callPpmHook,
    },
  };
  const { instance } = await WebAssembly.instantiate(
    await res.arrayBuffer(),
    imports
  );
  wasmMemory = instance.exports.memory;
  return instance.exports;
}

export function nextPaint() {
  return new Promise((resolve) => {
    requestAnimationFrame(() => requestAnimationFrame(resolve));
  });
}

export class RunCancelled extends Error {
  constructor() {
    super("run cancelled");
    this.name = "RunCancelled";
  }
}

function throwIfAborted(signal) {
  if (signal?.aborted) throw new RunCancelled();
}

function yieldToBrowser() {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

class WasmSession {
  constructor(wasm) {
    for (const name of [
      "session_init",
      "session_load_source",
      "session_eval",
      "session_take_events",
      "session_run_begin",
      "session_run_chunk",
    ]) {
      if (typeof wasm[name] !== "function") {
        throw new Error(`actr_engine.wasm is missing ${name}; rebuild it with web/build.py`);
      }
    }
    this.wasm = wasm;
  }

  resultText() {
    const ptr = this.wasm.result_ptr();
    const len = this.wasm.result_len();
    return decoder.decode(new Uint8Array(this.wasm.memory.buffer, ptr, len));
  }

  callWithText(text, fn) {
    const bytes = encoder.encode(text);
    const ptr = this.wasm.alloc(bytes.length);
    try {
      new Uint8Array(this.wasm.memory.buffer, ptr, bytes.length).set(bytes);
      return fn(ptr, bytes.length);
    } finally {
      this.wasm.dealloc(ptr, bytes.length);
    }
  }

  init(seed0, seed1) {
    resetHookState();
    const status = this.wasm.session_init(BigInt(seed0), BigInt(seed1));
    if (status !== 0) throw new Error(this.resultText());
  }

  setHook(kind, name, enabled) {
    if (typeof this.wasm.session_set_hook !== "function") {
      throw new Error(
        "actr_engine.wasm is missing session_set_hook; rebuild it with web/build.py"
      );
    }
    this.callWithText(name, (ptr, len) => {
      const status = this.wasm.session_set_hook(kind, ptr, len, enabled ? 1 : 0);
      if (status !== 0) throw new Error(this.resultText() || "session_set_hook failed");
    });
  }

  takeHostAction() {
    if (typeof this.wasm.session_take_host_action !== "function") return null;
    const status = this.wasm.session_take_host_action();
    const text = this.resultText();
    if (status !== 0) {
      throw new Error(text || "session_take_host_action failed");
    }
    return text && text !== "null" ? JSON.parse(text) : null;
  }

  completeHostAction(result, simple) {
    if (typeof this.wasm.session_complete_host_action !== "function") {
      throw new Error(
        "actr_engine.wasm is missing session_complete_host_action; rebuild it with web/build.py"
      );
    }
    const text = result == null ? "" : String(result);
    if (text.length === 0) {
      const status = this.wasm.session_complete_host_action(0, 0, simple ? 1 : 0);
      if (status !== 0) {
        throw new Error(this.resultText() || "session_complete_host_action failed");
      }
      return;
    }
    this.callWithText(text, (ptr, len) => {
      const status = this.wasm.session_complete_host_action(ptr, len, simple ? 1 : 0);
      if (status !== 0) {
        throw new Error(
          this.resultText() || "session_complete_host_action failed"
        );
      }
    });
  }

  loadSource(source) {
    const status = this.callWithText(source, (ptr, len) =>
      this.wasm.session_load_source(ptr, len)
    );
    if (status !== 0) {
      const text = this.resultText();
      let message = text;
      try {
        const payload = JSON.parse(text);
        if (payload?.error) message = payload.error;
      } catch {
        message = text;
      }
      throw new Error(message);
    }
  }

  eval(name, args) {
    return this.evalRaw(name, JSON.stringify(args));
  }

  evalRaw(name, argsJson) {
    const nameBytes = encoder.encode(name);
    const argBytes = encoder.encode(argsJson);
    const namePtr = this.wasm.alloc(nameBytes.length);
    const argPtr = this.wasm.alloc(argBytes.length);
    let status;
    try {
      new Uint8Array(this.wasm.memory.buffer, namePtr, nameBytes.length).set(nameBytes);
      new Uint8Array(this.wasm.memory.buffer, argPtr, argBytes.length).set(argBytes);
      status = this.wasm.session_eval(namePtr, nameBytes.length, argPtr, argBytes.length);
    } finally {
      this.wasm.dealloc(namePtr, nameBytes.length);
      this.wasm.dealloc(argPtr, argBytes.length);
    }

    const text = this.resultText();
    let payload;
    try {
      payload = JSON.parse(text);
    } catch {
      throw new Error(text || `session_eval ${name} failed`);
    }
    if (status !== 0 || !payload.ok) {
      throw new Error(payload.error || `session_eval ${name} failed`);
    }
    return payload.result || [];
  }

  runBegin(name, secs) {
    return this.callWithText(name, (ptr, len) => {
      const status = this.wasm.session_run_begin(ptr, len, secs);
      const text = this.resultText();
      if (status !== 0) throw new Error(text || "session_run_begin failed");
      return JSON.parse(text);
    });
  }

  runChunk(limitMs, full, maxEvents) {
    const status = this.wasm.session_run_chunk(limitMs, full ? 1 : 0, maxEvents);
    const text = this.resultText();
    if (status !== 0) throw new Error(text || "session_run_chunk failed");
    return JSON.parse(text).more === true;
  }

  takeEvents() {
    const status = this.wasm.session_take_events();
    const text = this.resultText();
    if (status !== 0) throw new Error(text || "session_take_events failed");
    return text ? JSON.parse(text) : [];
  }
}

export class Actr {
  constructor(wasm, seed0, seed1, window = null) {
    this.session = new WasmSession(wasm);
    this.session.init(seed0, seed1);
    this.window = window;
    this.commands = new Map();
    this.monitors = new Map();
    this.trace = [];
    this.currentWindow = null;
    this.draining = false;
    this.simHookName = "";
    this.ppmHookName = "";
  }

  loadSource(source) {
    this.session.loadSource(source);
    this.drain();
    this.refreshHooks();
  }

  eval(name, ...args) {
    const result = this.session.eval(name, args);
    this.drain();
    return result;
  }

  evalRaw(name, argsJson) {
    const result = this.session.evalRaw(name, argsJson);
    this.drain();
    return result;
  }

  getParameterValue(key) {
    return first(this.eval("get-parameter-value", key));
  }

  reset() {
    return this.eval("reset");
  }

  setParameterValue(key, value) {
    const result = this.eval("set-parameter-value", key, value);
    const upper = String(key).toUpperCase();
    if (upper === ":SIM-HOOK" || upper === ":PPM-HOOK") {
      this.refreshHooks();
    }
    return result;
  }

  refreshHooks() {
    const hasExport = typeof this.session.setHook === "function";
    if (hasExport) {
      if (this.simHookName) {
        this.session.setHook(0, this.simHookName, false);
      }
      if (this.ppmHookName) {
        this.session.setHook(1, this.ppmHookName, false);
      }
    }
    this.simHookName = nameOrEmpty(this.getParameterValue(":sim-hook"));
    this.ppmHookName = nameOrEmpty(this.getParameterValue(":ppm-hook"));
    if (!hasExport) {
      if (this.simHookName || this.ppmHookName) {
        throw new Error(
          "actr_engine.wasm is missing session_set_hook; rebuild it with web/build.py"
        );
      }
      return;
    }
    this.syncHook(this.simHookName);
    this.syncHook(this.ppmHookName);
  }

  syncHook(name) {
    if (!name || typeof this.session.setHook !== "function") return;
    const upper = String(name).toUpperCase();
    const fn = this.commands.get(upper);
    if (upper === this.simHookName) {
      this.session.setHook(0, upper, Boolean(fn));
    }
    if (upper === this.ppmHookName) {
      this.session.setHook(1, upper, Boolean(fn));
    }
  }

  setBufferChunk(buffer, spec) {
    return this.eval("set-buffer-chunk", buffer, chunkSpec(spec));
  }

  clearBuffer(buffer) {
    return this.eval("clear-buffer", buffer);
  }

  defineChunks(...names) {
    return this.eval("define-chunks", ...names.map((name) => [name]));
  }

  pdisable(...names) {
    return this.eval("pdisable", ...names);
  }

  penable(...names) {
    return this.eval("penable", ...names);
  }

  goalFocus(spec) {
    return this.eval("goal-focus", chunkSpec(spec));
  }

  modFocus(pairs) {
    return this.eval("mod-focus", flatPairs(pairs));
  }

  bufferRead(buffer) {
    return first(this.eval("buffer-read", buffer));
  }

  bufferSlotValue(buffer, slot) {
    return first(this.eval("buffer-slot-value", buffer, slot));
  }

  copyChunk(name) {
    return first(this.eval("copy-chunk", name));
  }

  chunkSlotValue(name, slot) {
    return first(this.eval("chunk-slot-value", name, slot));
  }

  modChunk(name, pairs) {
    return this.eval("mod-chunk", name, flatPairs(pairs));
  }

  async _runStepped(name, seconds, options = {}) {
    const signal = options.signal;
    throwIfAborted(signal);

    const paint = options.paint !== false;
    const needsCallbacks = this.commands.size > 0 || this.monitors.size > 0;
    const batch = options.maxEvents
      ?? (needsCallbacks || (paint && this.window) ? 1 : 256);
    const yieldEveryMs = options.yieldEveryMs ?? 16;

    const begin = this.session.runBegin(name, seconds);
    this.drain();
    if (!begin.shouldRun) return;

    let lastYield = performance.now();
    for (;;) {
      throwIfAborted(signal);
      const more = this.session.runChunk(begin.limit, begin.full, batch);
      const state = this.drain();
      const hostHandled = this.completeHostAction();
      if (hookError) {
        const error = hookError;
        hookError = null;
        throw error;
      }

      if (paint && state.painted) {
        await nextPaint();
        lastYield = performance.now();
      } else if (yieldEveryMs > 0 && performance.now() - lastYield >= yieldEveryMs) {
        await yieldToBrowser();
        lastYield = performance.now();
        throwIfAborted(signal);
      }

      if (!more && state.handled === 0 && !hostHandled) break;
    }
    throwIfAborted(signal);
  }

  completeHostAction() {
    if (typeof this.session.takeHostAction !== "function") return false;
    const action = this.session.takeHostAction();
    if (!action) return false;
    const fn = this.commands.get(String(action.name).toUpperCase());
    let result = null;
    if (fn) {
      try {
        result = fn(...(action.args || []));
      } catch {
        result = null;
      }
    }
    this.session.completeHostAction(
      actionResultText(result),
      Boolean(action.simple)
    );
    return true;
  }

  run(seconds, options = {}) {
    return this._runStepped("run", seconds, options);
  }

  runFullTime(seconds, options = {}) {
    return this._runStepped("run-full-time", seconds, options);
  }

  addCommand(name, fn) {
    const key = name.toUpperCase();
    this.commands.set(key, fn);
    commandFns.set(key, fn);
    const result = this.eval("add-act-r-command", name);
    this.syncHook(key);
    return result;
  }

  removeCommand(name) {
    const key = name.toUpperCase();
    this.commands.delete(key);
    commandFns.delete(key);
    const result = this.eval("remove-act-r-command", name);
    this.syncHook(key);
    return result;
  }

  monitorCommand(original, monitor) {
    const key = original.toUpperCase();
    const monitors = this.monitors.get(key) || [];
    if (!monitors.includes(monitor.toUpperCase())) {
      monitors.push(monitor.toUpperCase());
    }
    this.monitors.set(key, monitors);
  }

  removeCommandMonitor(original, monitor) {
    const key = original.toUpperCase();
    const monitorKey = monitor.toUpperCase();
    const monitors = (this.monitors.get(key) || []).filter((m) => m !== monitorKey);
    this.monitors.set(key, monitors);
  }

  openExpWindow(title, options = {}) {
    this.eval("open-exp-window", title, optionPairs(options));
    this.currentWindow = title;
    return title;
  }

  closeExpWindow(title = null) {
    return this.eval("close-exp-window", title || this.currentWindow || "");
  }

  clearExpWindow(title = null) {
    return this.eval("clear-exp-window", title || this.currentWindow || "");
  }

  addTextToExpWindow(window, text, options = {}) {
    return this.eval(
      "add-text-to-exp-window",
      window || this.currentWindow || "",
      text,
      optionPairs(options)
    );
  }

  addButtonToExpWindow(window, options = {}) {
    return first(this.eval(
      "add-button-to-exp-window",
      window || this.currentWindow || "",
      optionPairs(options)
    ));
  }

  addLineToExpWindow(window, start, end, color = "black") {
    return first(this.eval(
      "add-line-to-exp-window",
      window || this.currentWindow || "",
      start,
      end,
      color
    ));
  }

  modifyLineForExpWindow(key, start, end, color = null) {
    return this.eval("modify-line-for-exp-window", key, start, end, color);
  }

  removeItemsFromExpWindow(window, ...items) {
    return this.eval(
      "remove-items-from-exp-window",
      window || this.currentWindow || "",
      ...items
    );
  }

  installDevice(device = null) {
    return this.eval("install-device", device ?? this.currentWindow ?? "");
  }

  startHandAtMouse() {
    return this.eval("start-hand-at-mouse");
  }

  productionUtility(name, key = ":u") {
    return first(first(first(this.eval("spp", name, key))));
  }

  visibleVirtualsAvailable() {
    return Boolean(first(this.eval("visible-virtuals-available?")));
  }

  processEvents() {
    return this.eval("process-events");
  }

  permuteList(items) {
    return first(this.eval("permute-list", items));
  }

  random(limit) {
    return first(this.eval("act-r-random", limit));
  }

  randomFloat(limit) {
    return first(this.evalRaw("act-r-random", `[${jsonFloat(limit)}]`));
  }

  getTime(modelTime = true) {
    return first(this.eval("get-time", Boolean(modelTime)));
  }

  newToneSound(freq, duration, onset = 0, timeInMs = false) {
    return this.eval("new-tone-sound", freq, duration, onset, timeInMs);
  }

  newDigitSound(digit, onset = 0, timeInMs = false) {
    return this.eval("new-digit-sound", digit, onset, timeInMs);
  }

  scheduleEventRelative(delay, action, params = [], timeInMs = false) {
    const seconds = timeInMs ? delay / 1000.0 : delay;
    const encoded = (params || []).map((value) =>
      typeof value === "string" ? { str: value } : value
    );
    return this.eval("schedule-event-relative", seconds, action, {
      params: encoded,
    });
  }

  correlation(data, reference, output = false) {
    return first(this.eval("correlation", data, reference, output));
  }

  meanDeviation(data, reference, output = false) {
    return first(this.eval("mean-deviation", data, reference, output));
  }

  drain() {
    if (this.draining) return { handled: 0, painted: false };
    this.draining = true;
    let handled = 0;
    let painted = false;
    try {
      for (;;) {
        const events = this.session.takeEvents();
        if (!events.length) break;
        for (const event of events) {
          const result = this.handleEvent(event);
          handled += result.handled ? 1 : 0;
          painted = painted || result.painted;
        }
      }
    } finally {
      this.draining = false;
    }
    return { handled, painted };
  }

  handleEvent(event) {
    switch (event.type) {
      case "window":
        if (this.window) this.window.handle(event.args);
        return { handled: false, painted: true };
      case "trace":
        this.trace.push(event.line);
        return { handled: false, painted: false };
      case "command":
        this.fire(event.name, [event.model, event.key]);
        return { handled: true, painted: false };
      case "task-command": {
        const args = event.args.map(commandValue);
        const fn = this.commands.get(String(event.name).toUpperCase());
        if (fn) fn(...args);
        this.fire(event.name, args);
        return { handled: true, painted: true };
      }
      case "device-action": {
        const args = event.args.map(commandValue);
        const fn = this.commands.get(String(event.name).toUpperCase());
        if (fn) fn(...args);
        return { handled: true, painted: true };
      }
      default:
        return { handled: false, painted: false };
    }
  }

  fire(name, args) {
    const monitors = this.monitors.get(String(name).toUpperCase()) || [];
    for (const monitor of monitors) {
      const fn = this.commands.get(monitor);
      if (fn) fn(...args);
    }
  }
}

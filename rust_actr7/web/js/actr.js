const encoder = new TextEncoder();
const decoder = new TextDecoder();

function first(values) {
  return Array.isArray(values) && values.length ? values[0] : null;
}

function optionPairs(options) {
  if (Array.isArray(options)) return options;
  return Object.entries(options || {});
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
  const { instance } = await WebAssembly.instantiate(await res.arrayBuffer(), {});
  return instance.exports;
}

export function nextPaint() {
  return new Promise((resolve) => {
    requestAnimationFrame(() => requestAnimationFrame(resolve));
  });
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
    const status = this.wasm.session_init(BigInt(seed0), BigInt(seed1));
    if (status !== 0) throw new Error(this.resultText());
  }

  loadSource(source) {
    const status = this.callWithText(source, (ptr, len) =>
      this.wasm.session_load_source(ptr, len)
    );
    if (status !== 0) throw new Error(this.resultText());
  }

  eval(name, args) {
    const nameBytes = encoder.encode(name);
    const argBytes = encoder.encode(JSON.stringify(args));
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
  }

  loadSource(source) {
    this.session.loadSource(source);
    this.drain();
  }

  eval(name, ...args) {
    const result = this.session.eval(name, args);
    this.drain();
    return result;
  }

  reset() {
    return this.eval("reset");
  }

  async run(seconds) {
    if (this.commands.size === 0 && this.monitors.size === 0) {
      this.eval("run", seconds);
      return;
    }

    const begin = this.session.runBegin("run", seconds);
    this.drain();
    if (!begin.shouldRun) return;

    for (;;) {
      const more = this.session.runChunk(begin.limit, begin.full, 1);
      const state = this.drain();
      if (state.painted) await nextPaint();
      if (!more && state.handled === 0) break;
    }
  }

  addCommand(name, fn) {
    this.commands.set(name.toUpperCase(), fn);
    return this.eval("add-act-r-command", name);
  }

  removeCommand(name) {
    this.commands.delete(name.toUpperCase());
    return this.eval("remove-act-r-command", name);
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

  removeItemsFromExpWindow(window, ...items) {
    return this.eval(
      "remove-items-from-exp-window",
      window || this.currentWindow || "",
      ...items
    );
  }

  installDevice(window = null) {
    return this.eval("install-device", window || this.currentWindow || "");
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

  getTime(modelTime = true) {
    return first(this.eval("get-time", Boolean(modelTime)));
  }

  newToneSound(freq, duration, onset = 0, timeInMs = false) {
    return this.eval("new-tone-sound", freq, duration, onset, timeInMs);
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

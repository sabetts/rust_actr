const encoder = new TextEncoder();
const decoder = new TextDecoder();

function first(values) {
  return Array.isArray(values) && values.length ? values[0] : null;
}

function optionPairs(options) {
  if (Array.isArray(options)) return options;
  return Object.entries(options || {});
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
    for (const name of ["session_init", "session_load_source", "session_eval", "session_take_events"]) {
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

  run(seconds) {
    return this.eval("run", seconds);
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

  drain() {
    if (this.draining) return;
    this.draining = true;
    try {
      for (;;) {
        const events = this.session.takeEvents();
        if (!events.length) break;
        for (const event of events) this.handleEvent(event);
      }
    } finally {
      this.draining = false;
    }
  }

  handleEvent(event) {
    switch (event.type) {
      case "window":
        if (this.window) this.window.handle(event.args);
        break;
      case "trace":
        this.trace.push(event.line);
        break;
      case "command":
        this.fire(event.name, [event.model, event.key]);
        break;
      case "task-command":
        this.fire(event.name, event.args);
        break;
      default:
        break;
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

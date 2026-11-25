import { RunCancelled } from "../actr.js";

function isEditable(target) {
  if (!(target instanceof Element)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
}

function normalizeKey(key) {
  if (key === " " || key === "Spacebar") return "space";
  return String(key ?? "").toLowerCase();
}

function createHost(actr) {
  const root = actr?.window?.root;
  const parent = root?.parentElement;
  if (!parent) return null;
  parent.querySelectorAll(".human-controls").forEach((node) => node.remove());
  const host = document.createElement("div");
  host.className = "human-controls";
  root.after(host);
  return host;
}

function removeHost(host) {
  host?.remove();
}

function makePrompt(host, text) {
  if (!host) return null;
  const prompt = document.createElement("div");
  prompt.className = "human-prompt";
  prompt.setAttribute("aria-live", "polite");
  prompt.textContent = String(text ?? "");
  host.append(prompt);
  return prompt;
}

export function createChoicePrompt(actr, ctx, options = {}) {
  const {
    prompt = "",
    choices = [],
    secondaryActions = [],
    timeoutMs = 0,
  } = options;
  const host = createHost(actr);
  const promptElement = makePrompt(host, prompt);
  const bar = document.createElement("div");
  bar.className = "human-choice-bar";
  bar.setAttribute("role", "group");
  if (prompt) bar.setAttribute("aria-label", prompt);
  host?.append(bar);

  const keyMap = new Map();
  const secondaryMap = new Map();
  let done = false;
  let timer = null;
  let resolvePromise = null;
  let rejectPromise = null;

  const promise = new Promise((resolve, reject) => {
    resolvePromise = resolve;
    rejectPromise = reject;
  });

  const cleanup = () => {
    if (timer) clearTimeout(timer);
    document.removeEventListener("keydown", onKey, true);
    ctx?.signal?.removeEventListener("abort", abort);
    removeHost(host);
  };
  const finish = (value) => {
    if (done) return;
    done = true;
    cleanup();
    resolvePromise(value == null ? null : { value, at: performance.now() });
  };
  const abort = () => {
    if (done) return;
    done = true;
    cleanup();
    rejectPromise(new RunCancelled());
  };
  const onKey = (event) => {
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    if (isEditable(event.target)) return;
    const key = normalizeKey(event.key);
    if (keyMap.has(key)) {
      event.preventDefault();
      finish(keyMap.get(key));
      return;
    }
    if (secondaryMap.has(key)) {
      event.preventDefault();
      secondaryMap.get(key)();
    }
  };

  for (const choice of choices) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "human-choice";
    button.dataset.value = String(choice.value);
    button.textContent = String(choice.label ?? choice.value);
    button.addEventListener("click", () => finish(choice.value));
    bar.append(button);
    keyMap.set(normalizeKey(choice.key ?? choice.value), choice.value);
  }

  for (const action of secondaryActions) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "human-choice secondary";
    button.dataset.action = String(action.label ?? "");
    button.textContent = String(action.label ?? "");
    const run = () => {
      if (done) return;
      action.onClick?.();
    };
    button.addEventListener("click", run);
    bar.append(button);
    if (action.key) {
      secondaryMap.set(normalizeKey(action.key), run);
    }
  }

  document.addEventListener("keydown", onKey, true);
  ctx?.signal?.addEventListener("abort", abort, { once: true });
  if (ctx?.signal?.aborted) {
    abort();
  } else if (timeoutMs > 0) {
    timer = setTimeout(() => finish(null), timeoutMs);
  }

  return {
    wait: () => promise,
    setPrompt: (text) => {
      if (promptElement) promptElement.textContent = String(text ?? "");
    },
    remove: () => {
      if (done) return;
      done = true;
      cleanup();
      resolvePromise(null);
    },
  };
}

export function createTextPrompt(actr, ctx, options = {}) {
  const {
    prompt = "",
    placeholder = "",
    maxLength = 0,
    inputMode = "text",
    submitLabel = "Submit",
    timeoutMs = 30000,
    sanitize = null,
    autoSubmitLength = 0,
    submitKeys = ["enter"],
    allowEmpty = false,
  } = options;
  const submitKeySet = new Set(submitKeys.map(normalizeKey));
  const host = createHost(actr);
  makePrompt(host, prompt);
  const row = document.createElement("div");
  row.className = "human-text-row";
  const input = document.createElement("input");
  input.type = "text";
  input.inputMode = inputMode;
  input.autocomplete = "off";
  input.spellcheck = false;
  input.placeholder = placeholder;
  if (maxLength > 0) input.maxLength = maxLength;
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = submitLabel;
  row.append(input, button);
  host?.append(row);

  let done = false;
  let timer = null;
  let resolvePromise = null;
  let rejectPromise = null;

  const promise = new Promise((resolve, reject) => {
    resolvePromise = resolve;
    rejectPromise = reject;
  });

  const cleanup = () => {
    if (timer) clearTimeout(timer);
    input.removeEventListener("keydown", onKeyDown);
    input.removeEventListener("input", onInput);
    document.removeEventListener("keydown", onDocumentKey, true);
    button.removeEventListener("click", submit);
    ctx?.signal?.removeEventListener("abort", abort);
    removeHost(host);
  };
  const finish = (text) => {
    if (done) return;
    done = true;
    cleanup();
    resolvePromise(text == null ? null : { text, at: performance.now() });
  };
  const abort = () => {
    if (done) return;
    done = true;
    cleanup();
    rejectPromise(new RunCancelled());
  };
  const read = () => {
    let text = input.value;
    if (typeof sanitize === "function") {
      const next = sanitize(text);
      if (next !== text) {
        input.value = next;
        text = next;
      }
    }
    return text.trim();
  };
  const submit = () => {
    const text = read();
    if (!text && !allowEmpty) return;
    finish(text);
  };
  const onInput = () => {
    const text = read();
    if (autoSubmitLength > 0 && text.length >= autoSubmitLength) {
      finish(text);
    }
  };
  const onKeyDown = (event) => {
    if (submitKeySet.has(normalizeKey(event.key))) {
      event.preventDefault();
      submit();
    }
  };
  const onDocumentKey = (event) => {
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    if (event.target === input || isEditable(event.target)) return;
    const key = normalizeKey(event.key);
    if (submitKeySet.has(key)) {
      event.preventDefault();
      submit();
      return;
    }
    if (event.key === "Backspace") {
      event.preventDefault();
      input.value = input.value.slice(0, -1);
      onInput();
      return;
    }
    if (event.key.length !== 1) return;
    event.preventDefault();
    input.value += event.key;
    onInput();
  };

  input.addEventListener("input", onInput);
  input.addEventListener("keydown", onKeyDown);
  document.addEventListener("keydown", onDocumentKey, true);
  button.addEventListener("click", submit);
  ctx?.signal?.addEventListener("abort", abort, { once: true });
  if (ctx?.signal?.aborted) {
    abort();
  } else {
    if (timeoutMs > 0) timer = setTimeout(() => finish(null), timeoutMs);
    input.focus();
  }

  return {
    wait: () => promise,
    remove: () => {
      if (done) return;
      done = true;
      cleanup();
      resolvePromise(null);
    },
  };
}

import { RunCancelled } from "../actr.js";

export const MODEL_HUMAN_OPTIONS = [
  ["model", "Model"],
  ["human", "Human"],
];

export function isHuman(values) {
  return String(values?.mode ?? "model") === "human";
}

export function modeControl(defaultValue = "model", label = "Mode") {
  return {
    id: "mode",
    label,
    type: "select",
    options: MODEL_HUMAN_OPTIONS,
    default: defaultValue,
  };
}

export function normalizeKey(key) {
  if (key === " " || key === "Spacebar") return "space";
  return String(key ?? "").toLowerCase();
}

export function humanRandomInt(limit) {
  const size = Math.trunc(Number(limit));
  if (!Number.isFinite(size) || size <= 0) return 0;
  return Math.floor(Math.random() * size);
}

export function humanShuffle(items) {
  const values = Array.from(items || []);
  for (let i = values.length - 1; i > 0; i -= 1) {
    const j = humanRandomInt(i + 1);
    [values[i], values[j]] = [values[j], values[i]];
  }
  return values;
}

function isEditable(target) {
  if (!(target instanceof Element)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
}

function cleanupListener(element, type, handler) {
  element?.removeEventListener(type, handler, true);
}

export function waitForKey(ctx, options = {}) {
  const { keys = null, timeoutMs = 0 } = options;
  return new Promise((resolve, reject) => {
    const allowed = keys ? new Set(keys.map(normalizeKey)) : null;
    let done = false;
    let timer = null;

    const cleanup = () => {
      if (timer) clearTimeout(timer);
      cleanupListener(document, "keydown", onKey);
      ctx?.signal?.removeEventListener("abort", abort);
    };
    const finish = (key) => {
      if (done) return;
      done = true;
      cleanup();
      resolve(key == null ? null : { key, at: performance.now() });
    };
    const abort = () => {
      if (done) return;
      done = true;
      cleanup();
      reject(new RunCancelled());
    };
    const onKey = (event) => {
      if (event.ctrlKey || event.metaKey || event.altKey) return;
      if (isEditable(event.target)) return;
      const key = normalizeKey(event.key);
      if (!key) return;
      if (allowed ? !allowed.has(key) : key.length !== 1) return;
      event.preventDefault();
      finish(key);
    };

    document.addEventListener("keydown", onKey, true);
    ctx?.signal?.addEventListener("abort", abort, { once: true });
    if (ctx?.signal?.aborted) {
      abort();
      return;
    }
    if (timeoutMs > 0) {
      timer = setTimeout(() => finish(null), timeoutMs);
    }
  });
}

export function waitForKeys(ctx, options = {}) {
  const {
    accept = (key) => key.length === 1,
    terminator = "space",
    timeoutMs = 0,
    maxKeys = 0,
  } = options;
  return new Promise((resolve, reject) => {
    const start = performance.now();
    const events = [];
    let done = false;
    let timer = null;

    const cleanup = () => {
      if (timer) clearTimeout(timer);
      cleanupListener(document, "keydown", onKey);
      ctx?.signal?.removeEventListener("abort", abort);
    };
    const finish = (timedOut) => {
      if (done) return;
      done = true;
      cleanup();
      resolve({ events, start, end: performance.now(), timedOut });
    };
    const abort = () => {
      if (done) return;
      done = true;
      cleanup();
      reject(new RunCancelled());
    };
    const onKey = (event) => {
      if (event.ctrlKey || event.metaKey || event.altKey) return;
      if (isEditable(event.target)) return;
      const key = normalizeKey(event.key);
      if (!key) return;
      if (key === terminator) {
        event.preventDefault();
        finish(false);
        return;
      }
      if (!accept(key)) return;
      event.preventDefault();
      events.push({ key, at: performance.now() });
      if (maxKeys > 0 && events.length >= maxKeys) {
        finish(false);
      }
    };

    document.addEventListener("keydown", onKey, true);
    ctx?.signal?.addEventListener("abort", abort, { once: true });
    if (ctx?.signal?.aborted) {
      abort();
      return;
    }
    if (timeoutMs > 0) {
      timer = setTimeout(() => finish(true), timeoutMs);
    }
  });
}

export function waitReal(ms, signal) {
  return new Promise((resolve, reject) => {
    let done = false;
    let timer = null;
    const cleanup = () => {
      if (timer) clearTimeout(timer);
      signal?.removeEventListener("abort", abort);
    };
    const finish = () => {
      if (done) return;
      done = true;
      cleanup();
      resolve();
    };
    const abort = () => {
      if (done) return;
      done = true;
      cleanup();
      reject(new RunCancelled());
    };
    timer = setTimeout(finish, Math.max(0, ms));
    signal?.addEventListener("abort", abort, { once: true });
    if (signal?.aborted) abort();
  });
}

export function playTone(freq, durationMs, delayMs = 0) {
  const start = () => {
    try {
      const AudioCtor = window.AudioContext || window.webkitAudioContext;
      if (!AudioCtor) return;
      if (!playTone.context) playTone.context = new AudioCtor();
      const context = playTone.context;
      if (context.state === "suspended") context.resume?.();
      const oscillator = context.createOscillator();
      const gain = context.createGain();
      oscillator.type = "sine";
      oscillator.frequency.value = Number(freq) || 1000;
      gain.gain.value = 0.08;
      oscillator.connect(gain);
      gain.connect(context.destination);
      const now = context.currentTime;
      oscillator.start(now);
      oscillator.stop(now + Math.max(0.01, Number(durationMs) / 1000));
    } catch {
      playTone.context = null;
    }
  };
  if (delayMs > 0) {
    setTimeout(start, delayMs);
  } else {
    start();
  }
}

export function showHint(actr, text) {
  const root = actr?.window?.root;
  if (!root?.parentElement) return null;
  const parent = root.parentElement;
  parent.querySelectorAll(".human-hint").forEach((node) => node.remove());
  const hint = document.createElement("div");
  hint.className = "human-hint";
  hint.setAttribute("aria-live", "polite");
  hint.textContent = String(text ?? "");
  root.after(hint);
  return hint;
}

export function clearHint(actr) {
  const root = actr?.window?.root;
  root?.parentElement?.querySelectorAll(".human-hint").forEach((node) => node.remove());
}

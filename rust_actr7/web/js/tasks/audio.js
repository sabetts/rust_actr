const DIGIT_FILES = {
  1: "floraphonic-casual-voice-man-says-1-186552.mp3",
  2: "floraphonic-casual-voice-man-says-2-186553.mp3",
  3: "floraphonic-casual-voice-man-says-3-186554.mp3",
  4: "floraphonic-casual-voice-man-says-4-186555.mp3",
  5: "floraphonic-casual-voice-man-says-5-186556.mp3",
  6: "floraphonic-casual-voice-man-says-6-209711.mp3",
  7: "floraphonic-casual-voice-man-says-7-209713.mp3",
  8: "floraphonic-casual-voice-man-says-8-209710.mp3",
  9: "floraphonic-casual-voice-man-says-9-209709.mp3",
};

let audioContext = null;
const buffers = new Map();
const activeSources = new Set();

function getContext() {
  const Ctor = window.AudioContext || window.webkitAudioContext;
  if (!Ctor) return null;
  if (!audioContext) audioContext = new Ctor();
  return audioContext;
}

async function loadDigit(digit) {
  const file = DIGIT_FILES[digit];
  if (!file) return null;
  if (buffers.has(digit)) return buffers.get(digit);

  const pending = (async () => {
    const context = getContext();
    if (!context) return null;
    const response = await fetch(`./audio/${file}`);
    if (!response.ok) return null;
    const bytes = await response.arrayBuffer();
    if (!bytes.byteLength) return null;
    return context.decodeAudioData(bytes.slice(0));
  })();

  buffers.set(digit, pending);
  try {
    const buffer = await pending;
    if (!buffer) buffers.delete(digit);
    return buffer;
  } catch {
    buffers.delete(digit);
    return null;
  }
}

export async function prepareDigits(digits) {
  const context = getContext();
  if (!context) return false;
  try {
    if (context.state === "suspended") await context.resume();
  } catch {
    return false;
  }
  const loaded = await Promise.all(digits.map(loadDigit));
  return loaded.every(Boolean);
}

export function stopDigits() {
  for (const source of activeSources) {
    try {
      source.stop();
    } catch {
      // Source may already have stopped.
    }
  }
  activeSources.clear();
}

export async function playDigits(digits, options = {}) {
  const { gapMs = 750, signal = null } = options;
  const context = getContext();
  if (!context) return false;
  try {
    if (context.state === "suspended") await context.resume();
  } catch {
    return false;
  }

  const loaded = await Promise.all(digits.map(loadDigit));
  if (loaded.some((buffer) => !buffer)) return false;
  if (signal?.aborted) return false;

  const startAt = context.currentTime + 0.03;
  const sources = [];
  let endAt = startAt;

  for (let index = 0; index < loaded.length; index += 1) {
    const buffer = loaded[index];
    const source = context.createBufferSource();
    source.buffer = buffer;
    source.connect(context.destination);
    const when = startAt + (index * gapMs) / 1000;
    source.start(when);
    sources.push(source);
    activeSources.add(source);
    endAt = Math.max(endAt, when + buffer.duration);
  }

  return new Promise((resolve) => {
    let done = false;
    let timer = null;
    const cleanup = () => {
      if (timer) clearTimeout(timer);
      signal?.removeEventListener("abort", abort);
      for (const source of sources) activeSources.delete(source);
    };
    const finish = (ok) => {
      if (done) return;
      done = true;
      cleanup();
      resolve(ok);
    };
    const abort = () => {
      for (const source of sources) {
        try {
          source.stop();
        } catch {
          // Source may already have stopped.
        }
      }
      finish(false);
    };
    signal?.addEventListener("abort", abort, { once: true });
    if (signal?.aborted) {
      abort();
      return;
    }
    const remainingMs = Math.max(0, (endAt - context.currentTime) * 1000) + 60;
    timer = setTimeout(() => finish(true), remainingMs);
  });
}

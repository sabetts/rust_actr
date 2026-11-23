import { RunCancelled } from "../actr.js";

const PAIRS = [
  ["bank", "0"], ["card", "1"], ["dart", "2"], ["face", "3"], ["game", "4"],
  ["hand", "5"], ["jack", "6"], ["king", "7"], ["lamb", "8"], ["mask", "9"],
  ["neck", "0"], ["pipe", "1"], ["quip", "2"], ["rope", "3"], ["sock", "4"],
  ["tent", "5"], ["vent", "6"], ["wall", "7"], ["xray", "8"], ["zinc", "9"],
];

const LATENCIES = [0.0, 2.158, 1.967, 1.762, 1.680, 1.552, 1.467, 1.402];
const PROBABILITIES = [0.000, 0.526, 0.667, 0.798, 0.887, 0.924, 0.958, 0.954];

function sameKey(value, target) {
  return String(value ?? "").toLowerCase() === String(target ?? "").toLowerCase();
}

function waitReal(ms, signal) {
  return new Promise((resolve, reject) => {
    let done = false;
    const timer = setTimeout(() => {
      done = true;
      signal?.removeEventListener("abort", abort);
      resolve();
    }, ms);
    const abort = () => {
      if (done) return;
      done = true;
      clearTimeout(timer);
      reject(new RunCancelled());
    };
    signal?.addEventListener("abort", abort, { once: true });
    if (signal?.aborted) abort();
  });
}

function createHumanGuide(ctx, keypad) {
  const progress = document.createElement("div");
  progress.className = "paired-keypad-status";
  if (keypad?.pad?.parentElement) {
    keypad.pad.after(progress);
  } else if (ctx.stage) {
    ctx.stage.hidden = false;
    ctx.stage.classList.add("paired-stage");
    ctx.stage.replaceChildren(progress);
  }
  return { progress, element: progress };
}

function updateHumanGuide(guide, progress) {
  if (guide) guide.progress.textContent = progress;
}

function createKeypad(actr) {
  const root = actr?.window?.root;
  if (!root?.parentElement) return null;
  root.parentElement.querySelectorAll(".paired-keypad").forEach((node) => node.remove());
  const pad = document.createElement("div");
  pad.className = "paired-keypad";
  pad.setAttribute("role", "group");
  pad.setAttribute("aria-label", "Choose the associate digit");
  pad.hidden = false;
  const buttons = new Map();
  for (let digit = 0; digit <= 9; digit += 1) {
    const button = document.createElement("button");
    button.type = "button";
    button.dataset.digit = String(digit);
    button.textContent = String(digit);
    button.setAttribute("aria-pressed", "false");
    pad.append(button);
    buttons.set(String(digit), button);
  }
  const select = (digit) => {
    const key = digit == null ? "" : String(digit);
    for (const [value, button] of buttons) {
      button.setAttribute("aria-pressed", value === key ? "true" : "false");
    }
  };
  root.after(pad);
  return {
    pad,
    select,
    clear: () => select(null),
    setDisabled: (disabled) => {
      pad.classList.toggle("is-disabled", disabled);
      for (const button of buttons.values()) button.disabled = disabled;
    },
    remove: () => pad.remove(),
  };
}

function waitForHumanResponse(ctx, ms, keypad) {
  return new Promise((resolve, reject) => {
    let done = false;
    let response = null;
    let responseTime = null;
    const pad = keypad?.pad || null;
    const onSelect = (digit) => {
      const key = String(digit ?? "");
      if (!/^[0-9]$/.test(key)) return;
      response = key;
      responseTime = performance.now();
      keypad?.select(key);
    };
    const cleanup = () => {
      clearTimeout(timer);
      document.removeEventListener("keydown", onKey, true);
      pad?.removeEventListener("click", onClick);
      ctx.signal?.removeEventListener("abort", abort);
    };
    const finish = () => {
      if (done) return;
      done = true;
      cleanup();
      resolve({ response, responseTime });
    };
    const abort = () => {
      if (done) return;
      done = true;
      cleanup();
      reject(new RunCancelled());
    };
    const onKey = (event) => {
      if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) {
        return;
      }
      if (!/^[0-9]$/.test(event.key)) return;
      event.preventDefault();
      onSelect(event.key);
    };
    const onClick = (event) => {
      const button = event.target instanceof Element ? event.target.closest("button") : null;
      if (button?.dataset.digit != null) onSelect(button.dataset.digit);
    };
    const timer = setTimeout(finish, ms);
    document.addEventListener("keydown", onKey, true);
    pad?.addEventListener("click", onClick);
    ctx.signal?.addEventListener("abort", abort, { once: true });
    if (ctx.signal?.aborted) abort();
  });
}

function addText(actr, text) {
  actr.addTextToExpWindow(actr.currentWindow, text, { x: 150, y: 150 });
}

function resultsLines(actr, predicted, data, label) {
  const n = predicted.length;
  const reference = data.slice(0, n);
  const trialHeader = Array.from({ length: n }, (_, i) => String(i + 1).padStart(8)).join("");
  return [
    label,
    `CORRELATION: ${Number(actr.correlation(predicted, reference, false)).toFixed(3)}`,
    `MEAN DEVIATION: ${Number(actr.meanDeviation(predicted, reference, false)).toFixed(3)}`,
    `Trial${trialHeader}`,
    `     ${predicted.map((value) => value.toFixed(3).padStart(8)).join("")}`,
  ];
}

function averageRows(rows, runs) {
  return rows.map((row) => ({
    accuracy: row.accuracy / runs,
    latency: row.latency / runs,
  }));
}

function addRows(target, rows) {
  rows.forEach((row, i) => {
    target[i].accuracy += row.accuracy;
    target[i].latency += row.latency;
  });
}

async function modelPass(actr, ctx, size, blocks, trace, signal) {
  actr.reset();
  actr.setParameterValue(":v", trace);
  const window = actr.openExpWindow("Paired-Associate Experiment", {
    visible: false,
    width: 300,
    height: 300,
  });
  if (actr.window) actr.window.headless = true;
  actr.installDevice(window);

  let response = null;
  let responseTime = null;
  actr.addCommand("paired-response", (_model, key) => {
    response = String(key ?? "");
    responseTime = Number(actr.getTime(true));
  });
  actr.monitorCommand("output-key", "paired-response");

  const rows = [];
  try {
    for (let block = 0; block < blocks; block += 1) {
      let score = 0;
      let time = 0;
      for (const [prompt, associate] of actr.permuteList(PAIRS.slice(PAIRS.length - size))) {
        actr.clearExpWindow(window);
        addText(actr, prompt);
        response = null;
        responseTime = null;
        const start = Number(actr.getTime(true));
        await actr.runFullTime(5, { paint: false, signal });
        if (sameKey(response, associate)) {
          score += 1;
          time += responseTime - start;
        }

        actr.clearExpWindow(window);
        addText(actr, associate);
        await actr.runFullTime(5, { paint: false, signal });
      }
      rows.push({
        accuracy: score / size,
        latency: score > 0 ? time / score / 1000 : 0,
      });
      ctx.progress?.(`running... block ${block + 1}/${blocks}`);
    }
  } finally {
    actr.removeCommandMonitor("output-key", "paired-response");
    actr.removeCommand("paired-response");
    actr.closeExpWindow(window);
  }
  return rows;
}

async function humanPass(actr, ctx, size, blocks) {
  actr.reset();
  const window = actr.openExpWindow("Paired-Associate Experiment", {
    visible: true,
    width: 300,
    height: 300,
  });
  const keypad = createKeypad(actr);
  const guide = createHumanGuide(ctx, keypad);
  const total = size * blocks;
  let pairIndex = 0;
  const rows = [];
  try {
    for (let block = 0; block < blocks; block += 1) {
      let score = 0;
      let time = 0;
      for (const [prompt, associate] of actr.permuteList(PAIRS.slice(PAIRS.length - size))) {
        const progress = `Block ${block + 1}/${blocks} · Pair ${pairIndex + 1}/${total}`;
        actr.clearExpWindow(window);
        addText(actr, prompt);
        updateHumanGuide(guide, progress);
        keypad?.clear();
        keypad?.setDisabled(false);
        await ctx.paint();
        const start = performance.now();
        const answer = await waitForHumanResponse(ctx, 5000, keypad);
        if (sameKey(answer.response, associate)) {
          score += 1;
          time += answer.responseTime - start;
        }

        actr.clearExpWindow(window);
        keypad?.setDisabled(true);
        addText(actr, associate);
        await ctx.paint();
        await waitReal(5000, ctx.signal);
        pairIndex += 1;
      }
      rows.push({
        accuracy: score / size,
        latency: score > 0 ? time / score / 1000 : 0,
      });
      ctx.progress?.(`running... block ${block + 1}/${blocks}`);
    }
    updateHumanGuide(guide, `All ${total} trials complete`);
  } finally {
    keypad?.remove();
    guide?.element?.remove();
    actr.closeExpWindow(window);
  }
  return rows;
}

async function paired(actr, ctx) {
  const mode = String(ctx.controls.mode || "model");
  const size = Math.max(1, Math.min(20, Number(ctx.controls.pairs) || 20));
  const blocks = Math.max(1, Math.min(8, Number(ctx.controls.blocks) || 8));
  const runs = Math.max(1, Number(ctx.controls.runs) || 1);
  const trace = Boolean(ctx.controls.trace);

  if (mode === "human") {
    const rows = await humanPass(actr, ctx, size, blocks);
    const report = [
      "Mode: human",
      `Pairs: ${size} - Blocks: ${blocks}`,
      ...resultsLines(actr, rows.map((row) => row.latency), LATENCIES, "Latency:"),
      ...resultsLines(actr, rows.map((row) => row.accuracy), PROBABILITIES, "Accuracy:"),
    ].join("\n");
    return { kind: "paired", mode: "human", report };
  }

  const totals = Array.from({ length: blocks }, () => ({ accuracy: 0, latency: 0 }));
  for (let run = 0; run < runs; run += 1) {
    addRows(totals, await modelPass(actr, ctx, size, blocks, trace, ctx.signal));
    ctx.progress?.(`running... ${run + 1}/${runs}`);
  }
  const rows = averageRows(totals, runs);
  const report = [
    `Mode: model (${runs} run${runs === 1 ? "" : "s"})`,
    `Pairs: ${size} - Blocks: ${blocks}`,
    ...resultsLines(actr, rows.map((row) => row.latency), LATENCIES, "Latency:"),
    ...resultsLines(actr, rows.map((row) => row.accuracy), PROBABILITIES, "Accuracy:"),
  ].join("\n");
  return { kind: "paired", mode: "model", report };
}

export const pairedTask = {
  run: paired,
  controls: [
    {
      id: "mode",
      label: "Mode",
      type: "select",
      options: [
        ["model", "Model"],
        ["human", "Human"],
      ],
      default: "model",
    },
    {
      id: "pairs",
      label: "Pairs",
      type: "number",
      min: 1,
      max: 20,
      step: 1,
      default: 20,
    },
    {
      id: "blocks",
      label: "Blocks",
      type: "number",
      min: 1,
      max: 8,
      step: 1,
      default: 8,
    },
    {
      id: "runs",
      label: "Runs",
      type: "number",
      min: 1,
      max: 20,
      step: 1,
      default: 1,
      showWhen: (values) => values.mode === "model",
    },
    {
      id: "trace",
      label: "Trace",
      type: "checkbox",
      default: false,
      showWhen: (values) => values.mode === "model",
    },
  ],
  runSeconds: 5,
  runTimeMode: "unused",
};

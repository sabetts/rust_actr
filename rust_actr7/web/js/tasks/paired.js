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

function createHumanGuide(ctx) {
  const stage = ctx.stage;
  if (!stage) return null;
  stage.hidden = false;
  stage.classList.add("paired-stage");
  stage.replaceChildren();

  const box = document.createElement("div");
  box.className = "paired-guide";
  const heading = document.createElement("div");
  heading.className = "paired-guide-heading";
  heading.textContent = "Paired Associate — Human";

  const instructions = document.createElement("div");
  instructions.className = "paired-guide-instructions";
  const recallInstruction = document.createElement("div");
  recallInstruction.textContent = "When you see a word, type the digit associated with it.";
  const studyInstruction = document.createElement("div");
  studyInstruction.textContent = "When you see a number, it is paired with the previous word. Remember it.";
  instructions.append(recallInstruction, studyInstruction);

  const phaseLine = document.createElement("div");
  phaseLine.className = "paired-guide-phase";
  const word = document.createElement("div");
  word.className = "paired-guide-word";
  const hint = document.createElement("div");
  hint.className = "paired-guide-hint";
  const progress = document.createElement("div");
  progress.className = "paired-guide-progress";
  box.append(heading, instructions, phaseLine, word, hint, progress);
  stage.append(box);
  return { phaseLine, word, hint, progress };
}

function updateHumanGuide(guide, phase, detail) {
  if (!guide) return;
  guide.phaseLine.textContent = phase === "study"
    ? "Study phase"
    : phase === "done"
      ? "Run complete"
      : "Recall phase";
  guide.word.textContent = detail.word;
  guide.hint.textContent = detail.hint;
  guide.progress.textContent = detail.progress;
}

function waitForHumanResponse(ctx, ms, onResponse) {
  return new Promise((resolve, reject) => {
    let done = false;
    let response = null;
    let responseTime = null;
    const cleanup = () => {
      clearTimeout(timer);
      document.removeEventListener("keydown", onKey, true);
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
      if (!/^[0-9]$/.test(event.key) || response != null) return;
      event.preventDefault();
      response = event.key;
      responseTime = performance.now();
      onResponse?.(response);
    };
    const timer = setTimeout(finish, ms);
    document.addEventListener("keydown", onKey, true);
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
  const guide = createHumanGuide(ctx);
  const total = size * blocks;
  let pairIndex = 0;
  const rows = [];
  for (let block = 0; block < blocks; block += 1) {
    let score = 0;
    let time = 0;
    for (const [prompt, associate] of actr.permuteList(PAIRS.slice(PAIRS.length - size))) {
      const progress = `Block ${block + 1}/${blocks} · Pair ${pairIndex + 1}/${total}`;
      actr.clearExpWindow(window);
      addText(actr, prompt);
      updateHumanGuide(guide, "recall", {
        word: prompt,
        hint: "Waiting for your response.",
        progress,
      });
      await ctx.paint();
      const start = performance.now();
      const answer = await waitForHumanResponse(ctx, 5000, (key) => {
        updateHumanGuide(guide, "recall", {
          word: prompt,
          hint: `Recorded ${key}. Waiting for the study phase...`,
          progress,
        });
        void ctx.paint();
      });
      if (sameKey(answer.response, associate)) {
        score += 1;
        time += answer.responseTime - start;
      }

      actr.clearExpWindow(window);
      addText(actr, associate);
      updateHumanGuide(guide, "study", {
        word: `${prompt} → ${associate}`,
        hint: answer.response == null
          ? "No response recorded. Study this pair for the next trial."
          : "Study this pair. The next trial starts in 5 seconds.",
        progress,
      });
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
  updateHumanGuide(guide, "done", {
    word: "All trials complete",
    hint: "The report is shown below.",
    progress: `${total} trials`,
  });
  actr.closeExpWindow(window);
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

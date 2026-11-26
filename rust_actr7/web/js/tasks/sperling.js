import {
  humanRandomInt,
  humanShuffle,
  isHuman,
  modeControl,
  playTone,
  waitReal,
} from "./human.js";
import { createTextPrompt } from "./human-controls.js";

const LETTERS = [
  "B", "C", "D", "F", "G", "H", "J", "K", "L", "M", "N",
  "P", "Q", "R", "S", "T", "V", "W", "X", "Y", "Z",
];
const ONSETS = [0.0, 0.15, 0.3, 1.0];
const EXP_DATA = [3.03, 2.4, 2.03, 1.5];

function score(answers, responses) {
  const wanted = new Set(answers.map((a) => a.toLowerCase()));
  return responses.reduce((n, key) => n + (wanted.has(String(key).toLowerCase()) ? 1 : 0), 0);
}

async function trial(actr, ctx, onset) {
  actr.reset();
  const letters = actr.permuteList(LETTERS);
  const answers = [];
  const row = Number(actr.random(3));
  const window = actr.openExpWindow("Sperling Experiment", { visible: true });

  for (let i = 0; i < 3; i += 1) {
    for (let j = 0; j < 4; j += 1) {
      const txt = letters[j + i * 4];
      if (i === row) answers.push(txt);
      actr.addTextToExpWindow(window, txt, {
        x: 75 + j * 50,
        y: 100 + i * 50,
      });
    }
  }

  actr.installDevice(window);
  const freq = row === 0 ? 2000 : row === 1 ? 1000 : 500;
  actr.newToneSound(freq, 0.5, onset);

  const responses = [];
  actr.addCommand("sperling-response", (model, key) => {
    if (String(key).toLowerCase() !== "space") responses.push(key);
  });
  actr.addCommand("sperling-clear", () => {
    actr.clearExpWindow(window);
  });
  actr.monitorCommand("output-key", "sperling-response");
  actr.scheduleEventRelative(
    900 + Number(actr.random(200)),
    "sperling-clear",
    [window],
    true
  );

  await ctx.paint();
  await actr.run(ctx.runSeconds ?? 30, { signal: ctx.signal });

  actr.removeCommandMonitor("output-key", "sperling-response");
  actr.removeCommand("sperling-response");
  actr.removeCommand("sperling-clear");
  return { onset, answers, responses, score: score(answers, responses) };
}

async function humanTrial(actr, ctx, onset, progressLabel = "") {
  actr.reset();
  const letters = humanShuffle(LETTERS);
  const answers = [];
  const row = humanRandomInt(3);
  const window = actr.openExpWindow("Sperling Experiment", { visible: true });

  for (let i = 0; i < 3; i += 1) {
    for (let j = 0; j < 4; j += 1) {
      const txt = letters[j + i * 4];
      if (i === row) answers.push(txt);
      actr.addTextToExpWindow(window, txt, {
        x: 75 + j * 50,
        y: 100 + i * 50,
      });
    }
  }

  const freq = row === 0 ? 2000 : row === 1 ? 1000 : 500;
  await ctx.paint();

  playTone(freq, 500, onset * 1000);

  await waitReal(900 + humanRandomInt(200), ctx.signal);
  actr.clearExpWindow(window);
  await ctx.paint();

  const prompt = createTextPrompt(actr, ctx, {
    prompt: `${progressLabel ? `${progressLabel} ` : ""}Type the letters from the cued row, then press Space or Submit.`,
    placeholder: "letters",
    maxLength: 12,
    submitKeys: ["space", "enter"],
    allowEmpty: true,
    timeoutMs: 30000,
    sanitize: (value) => value.replace(/[^a-z]/gi, "").slice(0, 12),
  });
  let answer;
  try {
    answer = await prompt.wait();
  } finally {
    prompt.remove();
  }
  const responses = answer ? [...answer.text.toLowerCase()] : [];
  return { onset, answers, responses, score: score(answers, responses) };
}

async function sperling(actr, ctx) {
  const human = isHuman(ctx.controls);
  const blocks = Math.max(1, Number(ctx.controls.blocks) || 1);
  const totals = [0, 0, 0, 0];
  for (let i = 0; i < blocks; i += 1) {
    const onsets = human ? humanShuffle(ONSETS) : actr.permuteList(ONSETS);
    const trials = [];
    for (let trialIndex = 0; trialIndex < onsets.length; trialIndex += 1) {
      const onset = Number(onsets[trialIndex]);
      const label = `Block ${i + 1}/${blocks} · Trial ${trialIndex + 1}/${onsets.length}.`;
      ctx.progress?.(
        `running... block ${i + 1}/${blocks} · trial ${trialIndex + 1}/${onsets.length}`
      );
      trials.push(
        await (human
          ? humanTrial(actr, ctx, onset, label)
          : trial(actr, ctx, onset))
      );
    }
    trials.sort((a, b) => a.onset - b.onset);
    trials.forEach((t, j) => {
      totals[j] += t.score;
    });
  }

  const data = totals.map((n) => n / blocks);
  const correlation = actr.correlation(data, EXP_DATA, false);
  const meanDeviation = actr.meanDeviation(data, EXP_DATA, false);
  const rows = ONSETS.map(
    (onset, i) =>
      `${onset.toFixed(2)} sec.          ${data[i].toFixed(2)}                ${EXP_DATA[i].toFixed(2)}`
  );
  return {
    kind: "sperling",
    mode: human ? "human" : "model",
    data,
    correlation,
    meanDeviation,
    report: [
      ...(human ? ["Mode: human", ""] : []),
      "Condition    Current Participant   Original Experiment",
      ...rows,
      "",
      `CORRELATION: ${Number(correlation).toFixed(3)}`,
      `MEAN DEVIATION: ${Number(meanDeviation).toFixed(3)}`,
    ].join("\n"),
  };
}

function runTimeMode(values) {
  return isHuman(values) ? "unused" : "per-trial";
}

export const sperlingTask = {
  run: sperling,
  controls: [
    modeControl("model"),
    { id: "blocks", label: "Blocks", type: "number", min: 1, max: 5, step: 1, default: 1 },
  ],
  runSeconds: 30,
  runTimeMode,
};

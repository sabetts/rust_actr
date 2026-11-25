import { isHuman, modeControl } from "./human.js";
import { createChoicePrompt } from "./human-controls.js";
import { playDigits, prepareDigits, stopDigits } from "./audio.js";

const DIGITS = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

const SIEGLER_DATA = [
  [0, 0.05, 0.86, 0, 0.02, 0, 0.02, 0, 0, 0.06],
  [0, 0.04, 0.07, 0.75, 0.04, 0, 0.02, 0, 0, 0.09],
  [0, 0.02, 0, 0.10, 0.75, 0.05, 0.01, 0.03, 0, 0.06],
  [0.02, 0, 0.04, 0.05, 0.80, 0.04, 0, 0.05, 0, 0],
  [0, 0, 0.07, 0.09, 0.25, 0.45, 0.08, 0.01, 0.01, 0.06],
  [0.04, 0, 0, 0.05, 0.21, 0.09, 0.48, 0, 0.02, 0.11],
];

const PROBLEMS = [
  [1, 1],
  [1, 2],
  [1, 3],
  [2, 2],
  [2, 3],
  [3, 3],
];

const POSITIONS = {
  zero: 0,
  one: 1,
  two: 2,
  three: 3,
  four: 4,
  five: 5,
  six: 6,
  seven: 7,
  eight: 8,
};

async function trial(actr, ctx, arg1, arg2, response) {
  actr.reset();
  actr.installDevice(["speech", "microphone"]);
  actr.newDigitSound(arg1);
  actr.newDigitSound(arg2, 0.75);
  response.value = null;
  await actr.run(ctx.runSeconds ?? 30, { signal: ctx.signal });
  return response.value;
}

function buildReport(actr, responses, human) {
  const results = Array.from({ length: PROBLEMS.length }, () => Array(10).fill(0));
  for (const set of responses) {
    for (let i = 0; i < PROBLEMS.length; i += 1) {
      const value = set[i];
      const position = POSITIONS[value];
      if (position == null) results[i][9] += 1;
      else results[i][position] += 1;
    }
  }
  const count = Math.max(1, responses.length);
  for (const row of results) {
    for (let j = 0; j < row.length; j += 1) row[j] /= count;
  }

  const flat = (rows) => rows.flat();
  const correlation = actr.correlation(flat(results), flat(SIEGLER_DATA), false);
  const meanDeviation = actr.meanDeviation(flat(results), flat(SIEGLER_DATA), false);
  const lines = [];
  if (human) {
    lines.push("Mode: human", `Sets: ${responses.length}`, "");
  }
  lines.push("       0     1     2     3     4     5     6     7     8   Other");
  for (let i = 0; i < PROBLEMS.length; i += 1) {
    const [a, b] = PROBLEMS[i];
    lines.push(
      `${a}+${b} ${results[i].map((value) => value.toFixed(2).padStart(4)).join("  ")}`
    );
  }
  lines.push(
    "",
    `CORRELATION: ${Number(correlation).toFixed(3)}`,
    `MEAN DEVIATION: ${Number(meanDeviation).toFixed(3)}`
  );

  return {
    kind: "siegler",
    mode: human ? "human" : "model",
    results,
    correlation,
    meanDeviation,
    report: lines.join("\n"),
  };
}

async function humanSiegler(actr, ctx) {
  const sets = Math.max(1, Math.min(50, Number(ctx.controls.humanSets) || 2));
  actr.reset();
  const window = actr.openExpWindow("Siegler Addition", {
    visible: true,
    width: 400,
    height: 300,
  });
  const responses = [];
  actr.addTextToExpWindow(window, "Loading sounds...", { x: 135, y: 145 });
  await ctx.paint();
  const audioReady = await prepareDigits([1, 2, 3]);

  try {
    for (let setIndex = 0; setIndex < sets; setIndex += 1) {
      const set = [];
      for (const [arg1, arg2] of PROBLEMS) {
        actr.clearExpWindow(window);
        actr.addTextToExpWindow(window, "Listen for two numbers.", { x: 110, y: 145 });
        await ctx.paint();

        const replay = () => {
          void playDigits([arg1, arg2], { signal: ctx.signal });
        };
        const played = audioReady
          ? await playDigits([arg1, arg2], { signal: ctx.signal })
          : false;
        if (!played) {
          actr.addTextToExpWindow(window, `${arg1} and ${arg2}`, { x: 155, y: 185 });
        }

        const prompt = createChoicePrompt(actr, ctx, {
          prompt: `Set ${setIndex + 1}/${sets}: what is the sum?`,
          choices: DIGITS.map((digit) => ({
            value: digit,
            label: digit,
            key: digit,
          })),
          secondaryActions: played
            ? [{ label: "Replay sounds", key: "r", onClick: replay }]
            : [],
          timeoutMs: 30000,
        });
        let answer;
        try {
          answer = await prompt.wait();
        } finally {
          prompt.remove();
        }
        if (answer == null) break;
        set.push(answer.value);
        ctx.progress?.(`running... set ${setIndex + 1}/${sets}`);
      }
      if (set.length !== PROBLEMS.length) break;
      responses.push(set);
    }
  } finally {
    stopDigits();
    actr.closeExpWindow(window);
  }

  return buildReport(actr, responses, true);
}

async function siegler(actr, ctx) {
  if (isHuman(ctx.controls)) return humanSiegler(actr, ctx);

  const sets = Math.max(1, Number(ctx.controls.sets) || 1);
  const response = { value: null };
  actr.addCommand("siegler-response", (_model, value) => {
    response.value = String(value ?? "").toLowerCase();
  });
  actr.monitorCommand("output-speech", "siegler-response");

  const responses = [];
  try {
    for (let i = 0; i < sets; i += 1) {
      const set = [];
      for (const [arg1, arg2] of PROBLEMS) {
        set.push(await trial(actr, ctx, arg1, arg2, response));
      }
      responses.push(set);
    }
  } finally {
    actr.removeCommandMonitor("output-speech", "siegler-response");
    actr.removeCommand("siegler-response");
  }

  return buildReport(actr, responses, false);
}

function runTimeMode(values) {
  return isHuman(values) ? "unused" : "per-trial";
}

export const sieglerTask = {
  run: siegler,
  controls: [
    modeControl("model"),
    {
      id: "sets",
      label: "Sets",
      type: "number",
      min: 1,
      max: 50,
      step: 1,
      default: 10,
      showWhen: (values) => !isHuman(values),
    },
    {
      id: "humanSets",
      label: "Human sets",
      type: "number",
      min: 1,
      max: 50,
      step: 1,
      default: 2,
      showWhen: (values) => isHuman(values),
    },
  ],
  runSeconds: 30,
  runTimeMode,
};

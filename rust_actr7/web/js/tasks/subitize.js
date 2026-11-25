import {
  clearHint,
  isHuman,
  modeControl,
  showHint,
  waitForKey,
} from "./human.js";

const EXP_DATA = [0.6, 0.65, 0.7, 0.86, 1.12, 1.5, 1.79, 2.13, 2.15, 2.58];
const WORDS = [
  "", "one", "two", "three", "four", "five",
  "six", "seven", "eight", "nine", "ten",
];
const DIGITS = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

function tooClose(x, y, point) {
  return Math.abs(x - point[0]) < 40 && Math.abs(y - point[1]) < 40;
}

function generatePoints(actr, n) {
  const points = [];
  while (points.length < n) {
    const x = Number(actr.random(240)) + 20;
    const y = Number(actr.random(240)) + 20;
    if (!points.some((point) => tooClose(x, y, point))) points.push([x, y]);
  }
  return points;
}

async function trial(actr, ctx, n) {
  actr.reset();
  const points = generatePoints(actr, n);
  const window = actr.openExpWindow("Subitizing Experiment");
  const start = actr.getTime(true);
  for (const point of points) {
    actr.addTextToExpWindow(window, "x", { x: point[0], y: point[1] });
  }

  let response = "";
  let responseTime = null;
  actr.addCommand("subitize-response", (model, string) => {
    responseTime = actr.getTime(true);
    response = string;
  });
  actr.monitorCommand("output-speech", "subitize-response");
  actr.installDevice(window);

  await ctx.paint();
  await actr.run(ctx.runSeconds ?? 30, { signal: ctx.signal });

  actr.removeCommandMonitor("output-speech", "subitize-response");
  actr.removeCommand("subitize-response");

  const answer = WORDS[n];
  if (response && String(response).toLowerCase() === answer) {
    return [(responseTime - start) / 1000.0, true];
  }
  return [30, false];
}

async function humanTrial(actr, ctx, n) {
  actr.reset();
  const points = generatePoints(actr, n);
  const window = actr.openExpWindow("Subitizing Experiment");
  const start = performance.now();
  for (const point of points) {
    actr.addTextToExpWindow(window, "x", { x: point[0], y: point[1] });
  }
  showHint(actr, "Press the number of Xs shown (0 represents 10).");

  try {
    await ctx.paint();
    const answer = await waitForKey(ctx, { keys: DIGITS, timeoutMs: 30000 });
    if (answer == null) return [30, false];
    const wanted = n === 10 ? "0" : String(n);
    if (answer.key === wanted) {
      return [(answer.at - start) / 1000.0, true];
    }
    return [30, false];
  } finally {
    clearHint(actr);
  }
}

async function subitize(actr, ctx) {
  const selected = ctx.controls.items ?? "all";
  let items;
  if (selected === "all") {
    items = actr.permuteList([10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
  } else {
    items = [Number(selected)];
  }

  const human = isHuman(ctx.controls);
  const results = [];
  for (const n of items) {
    results.push([
      Number(n),
      await (human
        ? humanTrial(actr, ctx, Number(n))
        : trial(actr, ctx, Number(n))),
    ]);
  }
  results.sort((a, b) => a[0] - b[0]);

  const current = results.map(([, result]) => result);
  const rts = current.map(([rt]) => rt);
  const shown = results.map(([n, result]) => {
    const original = EXP_DATA[n - 1];
    return `${String(n).padStart(3)}        ${result[0].toFixed(2)}  (${String(result[1]).padEnd(5)})            ${original.toFixed(2)}`;
  });
  const reference = results.map(([n]) => EXP_DATA[n - 1]);
  const correlation = current.length > 1
    ? actr.correlation(rts, reference, false)
    : null;
  const meanDeviation = current.length > 1
    ? actr.meanDeviation(rts, reference, false)
    : null;

  const report = [
    "Items    Current Participant   Original Experiment",
    ...shown,
  ];
  if (correlation != null) {
    report.push("", `CORRELATION: ${Number(correlation).toFixed(3)}`);
    report.push(`MEAN DEVIATION: ${Number(meanDeviation).toFixed(3)}`);
  }

  return {
    kind: "subitize",
    mode: human ? "human" : "model",
    results,
    correlation,
    meanDeviation,
    report: (human ? ["Mode: human", "", ...report] : report).join("\n"),
  };
}

function runTimeMode(values) {
  return isHuman(values) ? "unused" : "per-trial";
}

export const subitizeTask = {
  run: subitize,
  controls: [
    modeControl("model"),
    {
      id: "items",
      label: "Items",
      type: "select",
      default: "all",
      options: [
        ["all", "All (1-10)"],
        ["1", "1"], ["2", "2"], ["3", "3"], ["4", "4"], ["5", "5"],
        ["6", "6"], ["7", "7"], ["8", "8"], ["9", "9"], ["10", "10"],
      ],
    },
  ],
  runSeconds: 30,
  runTimeMode,
};

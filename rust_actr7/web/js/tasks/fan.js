import { isHuman, modeControl } from "./human.js";
import { createChoicePrompt } from "./human-controls.js";

const PERSON_LOCATION_DATA = [
  1.11, 1.17, 1.22,
  1.17, 1.20, 1.22,
  1.15, 1.23, 1.36,
  1.20, 1.22, 1.26,
  1.25, 1.36, 1.29,
  1.26, 1.47, 1.47,
];

const PROBES = [
  ["lawyer", "store", true],
  ["captain", "cave", true],
  ["hippie", "church", true],
  ["debutante", "bank", true],
  ["earl", "castle", true],
  ["hippie", "bank", true],
  ["fireman", "park", true],
  ["captain", "park", true],
  ["hippie", "park", true],
  ["fireman", "store", false],
  ["captain", "store", false],
  ["giant", "store", false],
  ["fireman", "bank", false],
  ["captain", "bank", false],
  ["giant", "bank", false],
  ["lawyer", "park", false],
  ["earl", "park", false],
  ["giant", "park", false],
];

function quoted(value) {
  return `'${value}'`;
}

function isCorrect(response, target) {
  const key = String(response || "").toLowerCase().replace(/^'|'$/g, "");
  return target ? key === "k" : key === "d";
}

async function visualSentence(actr, ctx, person, location, target, term) {
  actr.reset();
  actr.penable("retrieve-from-person", "retrieve-from-location");
  actr.pdisable(term === "person" ? "retrieve-from-location" : "retrieve-from-person");

  const window = actr.openExpWindow("Sentence Experiment", {
    visible: true,
    width: 600,
    height: 300,
  });
  actr.addTextToExpWindow(window, person, { x: 50, y: 150, width: 75 });
  actr.addTextToExpWindow(window, location, { x: 250, y: 150, width: 75 });
  actr.installDevice(window);

  let response = null;
  let responseTime = null;
  actr.addCommand("fan-response", (_model, key) => {
    response = String(key);
    responseTime = Number(actr.getTime());
  });
  actr.monitorCommand("output-key", "fan-response");

  try {
    await ctx.paint();
    await actr.run(ctx.runSeconds ?? 30, { signal: ctx.signal });
  } finally {
    actr.removeCommandMonitor("output-key", "fan-response");
    actr.removeCommand("fan-response");
    actr.closeExpWindow(window);
  }

  if (response == null) return { rt: 30, correct: false };
  return {
    rt: responseTime / 1000,
    correct: isCorrect(response, target),
  };
}

async function goalSentence(actr, ctx, person, location, target, term) {
  actr.reset();
  actr.penable("retrieve-from-person", "retrieve-from-location");
  actr.pdisable(term === "person" ? "retrieve-from-location" : "retrieve-from-person");
  actr.modFocus({
    arg1: quoted(person),
    arg2: quoted(location),
  });

  await actr.run(ctx.runSeconds ?? 30, { signal: ctx.signal });
  const response = actr.bufferSlotValue("goal", "state");
  if (response == null) return { rt: 30, correct: false };
  return {
    rt: Number(actr.getTime()) / 1000,
    correct: isCorrect(response, target),
  };
}

async function collect(actr, ctx, term, noPm) {
  const results = [];
  for (const [person, location, target] of PROBES) {
    results.push(
      noPm
        ? await goalSentence(actr, ctx, person, location, target, term)
        : await visualSentence(actr, ctx, person, location, target, term)
    );
  }
  return results;
}

async function humanSentence(actr, ctx, window, person, location, target) {
  actr.clearExpWindow(window);
  actr.addTextToExpWindow(window, person, { x: 50, y: 150, width: 75 });
  actr.addTextToExpWindow(window, location, { x: 250, y: 150, width: 75 });
  const start = performance.now();
  await ctx.paint();
  const prompt = createChoicePrompt(actr, ctx, {
    prompt: "Was this pair studied?",
    choices: [
      { value: "k", label: "Studied (K)", key: "k" },
      { value: "d", label: "Not studied (D)", key: "d" },
    ],
    timeoutMs: 30000,
  });
  let answer;
  try {
    answer = await prompt.wait();
  } finally {
    prompt.remove();
  }
  if (answer == null) return { rt: 30, correct: false };
  return {
    rt: (answer.at - start) / 1000,
    correct: isCorrect(answer.value, target),
  };
}

async function humanCollect(actr, ctx) {
  actr.reset();
  const window = actr.openExpWindow("Sentence Experiment", {
    visible: true,
    width: 600,
    height: 300,
  });
  try {
    const results = [];
    for (const [person, location, target] of PROBES) {
      results.push(await humanSentence(actr, ctx, window, person, location, target));
    }
    return results;
  } finally {
    actr.closeExpWindow(window);
  }
}

function average(a, b) {
  return a.map((item, i) => ({
    rt: (item.rt + b[i].rt) / 2,
    correct: item.correct && b[i].correct,
  }));
}

function formatTable(data) {
  const lines = ["TARGETS:", "                         Person fan", "  Location      1             2             3"];
  for (let i = 0; i < 3; i += 1) {
    let line = `    ${i + 1}      `;
    for (let j = 0; j < 3; j += 1) {
      const item = data[i * 3 + j];
      line += `${item.rt.toFixed(3)} (${item.correct ? "T" : "F"})  `;
    }
    lines.push(line.trimEnd());
  }
  lines.push("", "FOILS:", "  Location      1             2             3");
  for (let i = 0; i < 3; i += 1) {
    let line = `    ${i + 1}      `;
    for (let j = 0; j < 3; j += 1) {
      const item = data[9 + i * 3 + j];
      line += `${item.rt.toFixed(3)} (${item.correct ? "T" : "F"})  `;
    }
    lines.push(line.trimEnd());
  }
  return lines;
}

async function fan(actr, ctx, noPm) {
  const human = isHuman(ctx.controls);
  const term = String(ctx.controls.term || "both");
  let data;
  if (human) {
    data = await humanCollect(actr, ctx);
  } else if (term === "both") {
    const person = await collect(actr, ctx, "person", noPm);
    const location = await collect(actr, ctx, "location", noPm);
    data = average(person, location);
  } else {
    data = await collect(actr, ctx, term, noPm);
  }

  const rts = data.map((item) => item.rt);
  const correlation = actr.correlation(rts, PERSON_LOCATION_DATA, false);
  const meanDeviation = actr.meanDeviation(rts, PERSON_LOCATION_DATA, false);
  const report = [
    ...(human ? ["Mode: human", ""] : []),
    ...formatTable(data),
    "",
    `CORRELATION: ${Number(correlation).toFixed(3)}`,
    `MEAN DEVIATION: ${Number(meanDeviation).toFixed(3)}`,
  ].join("\n");

  return {
    kind: noPm ? "fan-no-pm" : "fan",
    mode: human ? "human" : "model",
    data,
    correlation,
    meanDeviation,
    report,
  };
}

const controls = [
  modeControl("model"),
  {
    id: "term",
    label: "Retrieval term",
    type: "select",
    options: [
      ["both", "Both"],
      ["person", "Person"],
      ["location", "Location"],
    ],
    default: "both",
    showWhen: (values) => !isHuman(values),
  },
];

function runTimeMode(values) {
  return isHuman(values) ? "unused" : "per-trial";
}

export const fanTask = {
  run: (actr, ctx) => fan(actr, ctx, false),
  controls,
  runSeconds: 30,
  runTimeMode,
};

export const fanNoPmTask = {
  run: (actr, ctx) => fan(actr, ctx, true),
  controls,
  runSeconds: 30,
  runTimeMode,
};

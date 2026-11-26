import {
  clearHint,
  humanShuffle,
  isHuman,
  modeControl,
  showHint,
  waitReal,
} from "./human.js";
import { createTextPrompt } from "./human-controls.js";

const EXPECTED = ["1", "2", "3", "4", "6", "5", "7", "8"];
const DIGITS = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];
const CANONICAL = DIGITS.slice();
const STUDY_MS = 5000;
const DEFAULT_TRIALS = 3;

async function humanTrial(actr, ctx, window, studied, trial, trialCount) {
  actr.clearExpWindow(window);
  for (let i = 0; i < studied.length; i += 3) {
    actr.addTextToExpWindow(window, studied.slice(i, i + 3).join(" "), {
      x: 80,
      y: 90 + (i / 3) * 50,
    });
  }
  showHint(actr, `Trial ${trial} of ${trialCount}. Study the grouped list.`);

  let prompt = null;
  try {
    await ctx.paint();
    await waitReal(STUDY_MS, ctx.signal);

    actr.clearExpWindow(window);
    showHint(actr, `Trial ${trial} of ${trialCount}.`);
    await ctx.paint();

    prompt = createTextPrompt(actr, ctx, {
      prompt: "Recall the list in order, then press Enter.",
      placeholder: "e.g. 123456789",
      maxLength: 9,
      inputMode: "numeric",
      timeoutMs: 30000,
      sanitize: (value) => value.replace(/[^1-9]/g, "").slice(0, 9),
    });
    const answer = await prompt.wait();
    const responses = answer ? [...answer.text] : [];
    const correct = responses.filter((item, index) => item === studied[index]).length;
    const matches = responses.length === studied.length
      && responses.every((item, index) => item === studied[index]);
    return { studied, responses, correct, matches };
  } finally {
    prompt?.remove();
    clearHint(actr);
    actr.clearExpWindow(window);
  }
}

async function humanRecall(actr, ctx) {
  const canonical = String(ctx.controls.humanListMode || "practice") === "canonical";
  const trials = canonical
    ? 1
    : Math.max(
      1,
      Math.min(20, Number(ctx.controls.humanTrials) || DEFAULT_TRIALS)
    );
  actr.reset();
  const window = actr.openExpWindow("Grouped Recall", {
    visible: true,
    width: 400,
    height: 300,
  });
  const results = [];
  try {
    for (let trial = 1; trial <= trials; trial += 1) {
      const studied = canonical ? CANONICAL : humanShuffle(DIGITS);
      results.push(await humanTrial(actr, ctx, window, studied, trial, trials));
      if (trial < trials) await waitReal(750, ctx.signal);
    }
  } finally {
    clearHint(actr);
    actr.closeExpWindow(window);
  }

  const correct = results.reduce((sum, result) => sum + result.correct, 0);
  const total = trials * DIGITS.length;
  const matches = results.filter((result) => result.matches).length;
  const rows = results.map((result, index) =>
    `${String(index + 1).padStart(2)}. ${result.studied.join(" ")} | ${result.responses.join(" ") || "none"} | ${result.correct}/9${result.matches ? " | exact" : ""}`
  );

  return {
    kind: "grouped",
    mode: "human",
    trials,
    results,
    correct,
    total,
    matches,
    report: [
      "Mode: human",
      `Lists: ${canonical ? "canonical (123 / 456 / 789)" : "randomized practice"}`,
      `Trials: ${trials}`,
      "",
      ...rows,
      "",
      `Mean correct: ${(correct / trials).toFixed(2)}/9`,
      `Exact matches: ${matches}/${trials}`,
    ].join("\n"),
  };
}

async function grouped(actr, ctx) {
  if (isHuman(ctx.controls)) return humanRecall(actr, ctx);

  const responses = [];
  actr.addCommand("grouped-response", (item) => {
    responses.push(String(item));
  });

  try {
    actr.reset();
    await actr.run(ctx.runSeconds ?? 20, { signal: ctx.signal });
  } finally {
    actr.removeCommand("grouped-response");
  }

  const matches = responses.length === EXPECTED.length
    && responses.every((item, i) => item === EXPECTED[i]);
  return {
    kind: "grouped",
    mode: "model",
    responses,
    report: [
      `RECALLED: ${responses.join(" ") || "none"}`,
      `EXPECTED: ${EXPECTED.join(" ")}`,
      `MATCH: ${matches ? "yes" : "no"}`,
    ].join("\n"),
  };
}

function runTimeMode(values) {
  return isHuman(values) ? "unused" : "used";
}

export const groupedTask = {
  run: grouped,
  controls: [
    modeControl("model"),
    {
      id: "humanListMode",
      label: "Human list",
      type: "select",
      options: [
        ["practice", "Randomized practice"],
        ["canonical", "Canonical 1-9"],
      ],
      default: "practice",
      showWhen: (values) => values.mode === "human",
    },
    {
      id: "humanTrials",
      label: "Human trials",
      type: "number",
      min: 1,
      max: 20,
      step: 1,
      default: DEFAULT_TRIALS,
      showWhen: (values) =>
        values.mode === "human" && values.humanListMode !== "canonical",
    },
  ],
  runSeconds: 20,
  runTimeMode,
};

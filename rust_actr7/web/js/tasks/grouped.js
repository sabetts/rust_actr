import {
  clearHint,
  isHuman,
  modeControl,
  showHint,
  waitReal,
} from "./human.js";
import { createTextPrompt } from "./human-controls.js";

const EXPECTED = ["1", "2", "3", "4", "6", "5", "7", "8"];
const STUDIED = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];
const STUDY_MS = 5000;

async function humanRecall(actr, ctx) {
  actr.reset();
  const window = actr.openExpWindow("Grouped Recall", {
    visible: true,
    width: 400,
    height: 300,
  });
  ["1 2 3", "4 5 6", "7 8 9"].forEach((line, index) => {
    actr.addTextToExpWindow(window, line, { x: 80, y: 90 + index * 50 });
  });
  showHint(actr, "Study the grouped list.");

  let prompt = null;
  try {
    await ctx.paint();
    await waitReal(STUDY_MS, ctx.signal);

    actr.clearExpWindow(window);
    showHint(actr, "Type the digits in recall order.");
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
    const correct = responses.filter((item, index) => item === STUDIED[index]).length;
    const matches = responses.length === STUDIED.length
      && responses.every((item, index) => item === STUDIED[index]);
    return {
      kind: "grouped",
      mode: "human",
      responses,
      correct,
      total: STUDIED.length,
      report: [
        "Mode: human",
        `STUDIED: ${STUDIED.join(" ")}`,
        `RECALLED: ${responses.join(" ") || "none"}`,
        `CORRECT: ${correct}/${STUDIED.length}`,
        `MATCH: ${matches ? "yes" : "no"}`,
      ].join("\n"),
    };
  } finally {
    prompt?.remove();
    clearHint(actr);
    actr.closeExpWindow(window);
  }
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
  controls: [modeControl("model")],
  runSeconds: 20,
  runTimeMode,
};

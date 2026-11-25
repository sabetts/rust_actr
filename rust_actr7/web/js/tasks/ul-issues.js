import {
  humanRandomFloat,
  isHuman,
  modeControl,
  waitReal,
} from "./human.js";
import { createChoicePrompt } from "./human-controls.js";

function utilityText(value) {
  if (value == null) return "none";
  return Number(value).toFixed(3);
}

async function humanTask(actr, ctx) {
  const trials = Math.max(1, Math.min(50, Number(ctx.controls.humanTrials) || 10));
  actr.reset();
  const window = actr.openExpWindow("Utility Learning", {
    visible: true,
    width: 400,
    height: 300,
  });
  const choices = { a: 0, b: 0 };
  let feedback = 0;
  let noFeedback = 0;

  try {
    for (let trial = 0; trial < trials; trial += 1) {
      actr.clearExpWindow(window);
      actr.addTextToExpWindow(window, "Choose A or B", { x: 140, y: 145 });
      await ctx.paint();

      const prompt = createChoicePrompt(actr, ctx, {
        prompt: `Trial ${trial + 1}/${trials}: choose A or B.`,
        choices: [
          { value: "a", label: "A", key: "a" },
          { value: "b", label: "B", key: "b" },
        ],
      });
      let answer;
      try {
        answer = await prompt.wait();
      } finally {
        prompt.remove();
      }
      if (answer == null) break;
      const choice = answer.value;
      choices[choice] += 1;

      const first = humanRandomFloat();
      const second = humanRandomFloat();
      const result = first < 0.6 ? "a" : second > 0.5 ? "b" : null;
      if (result == null) noFeedback += 1;
      else feedback += 1;

      actr.clearExpWindow(window);
      actr.addTextToExpWindow(
        window,
        result == null ? "No feedback" : `Feedback: ${result.toUpperCase()}`,
        { x: 130, y: 145 }
      );
      await ctx.paint();
      await waitReal(2000, ctx.signal);
      ctx.progress?.(`running... ${trial + 1}/${trials}`);
    }
  } finally {
    actr.closeExpWindow(window);
  }

  return {
    kind: "ul-issues",
    mode: "human",
    trials,
    choices,
    feedback,
    noFeedback,
    report: [
      "Mode: human",
      `Trials: ${trials}`,
      `Choices: A ${choices.a}, B ${choices.b}`,
      `Feedback: ${feedback}, no feedback: ${noFeedback}`,
    ].join("\n"),
  };
}

async function ulIssues(actr, ctx) {
  if (isHuman(ctx.controls)) return humanTask(actr, ctx);

  const choices = { a: 0, b: 0 };
  let feedback = 0;
  let noFeedback = 0;

  const presentChoose = () => {
    actr.goalFocus(["isa", "choose"]);
    let answer = null;
    if (Number(actr.randomFloat(1.0)) < 0.6) {
      answer = "a";
    } else if (Number(actr.randomFloat(1.0)) > 0.5) {
      answer = "b";
    }
    if (answer === "a") choices.a += 1;
    if (answer === "b") choices.b += 1;
    actr.scheduleEventRelative(
      5,
      "utility-learning-issues-show-result",
      [answer]
    );
  };

  const showResult = (answer) => {
    const value = answer == null || answer === "" ? null : String(answer);
    if (value == null) noFeedback += 1;
    else feedback += 1;
    actr.setBufferChunk("imaginal", ["answer", value]);
    actr.scheduleEventRelative(2, "utility-learning-issues-choose");
  };

  actr.addCommand("utility-learning-issues-choose", presentChoose);
  actr.addCommand("utility-learning-issues-show-result", showResult);
  try {
    await actr.run(ctx.secs ?? 20, { signal: ctx.signal });
  } finally {
    actr.removeCommand("utility-learning-issues-choose");
    actr.removeCommand("utility-learning-issues-show-result");
  }

  const chooseA = actr.productionUtility("choose-a");
  const chooseB = actr.productionUtility("choose-b");
  return {
    kind: "ul-issues",
    mode: "model",
    report: [
      `Run time: ${ctx.secs ?? 20} s`,
      `Choices: A ${choices.a}, B ${choices.b}`,
      `Feedback: ${feedback}, no feedback: ${noFeedback}`,
      "",
      "Production utilities",
      `choose-a: ${utilityText(chooseA)}`,
      `choose-b: ${utilityText(chooseB)}`,
      "",
      "This model uses its own fixed seed; the page seed does not affect it.",
    ].join("\n"),
  };
}

function runTimeMode(values) {
  return isHuman(values) ? "unused" : "used";
}

export const ulIssuesTask = {
  run: ulIssues,
  controls: [
    modeControl("model"),
    {
      id: "humanTrials",
      label: "Human trials",
      type: "number",
      min: 1,
      max: 50,
      step: 1,
      default: 10,
      showWhen: (values) => isHuman(values),
    },
  ],
  runSeconds: 20,
  runTimeMode,
};

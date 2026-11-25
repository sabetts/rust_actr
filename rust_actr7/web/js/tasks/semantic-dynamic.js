import { isHuman, modeControl } from "./human.js";
import { createChoicePrompt } from "./human-controls.js";

const QUESTIONS = {
  g1: {
    label: "Is a canary a bird?",
    model: "YES",
  },
  g2: {
    label: "Does a canary have wings?",
    model: "YES",
  },
  g3: {
    label: "Is a canary dangerous?",
    model: "DONT-KNOW",
  },
};

async function askHuman(actr, ctx, window, name) {
  const question = QUESTIONS[name];
  actr.clearExpWindow(window);
  actr.addTextToExpWindow(window, question.label, { x: 80, y: 145 });
  await ctx.paint();
  const prompt = createChoicePrompt(actr, ctx, {
    prompt: question.label,
    choices: [
      { value: "yes", label: "Yes", key: "y" },
      { value: "no", label: "No", key: "n" },
      { value: "dont-know", label: "Don't Know", key: "d" },
    ],
  });
  try {
    const answer = await prompt.wait();
    return answer?.value ?? "no-answer";
  } finally {
    prompt.remove();
  }
}

async function askModel(actr, ctx, name, resetFirst) {
  if (resetFirst) actr.reset();
  actr.goalFocus(name);
  const start = actr.trace.length;
  await actr.run(5, { paint: false, signal: ctx.signal });
  const answers = actr.trace
    .slice(start)
    .map((line) => line.trim())
    .filter((line) => line.includes("ANSWER IS"));
  return answers.length ? answers[answers.length - 1] : "no response";
}

async function humanSemantic(actr, ctx) {
  const question = String(ctx.controls.question || "all");
  const names = question === "all" ? ["g1", "g2", "g3"] : [question];
  actr.reset();
  const window = actr.openExpWindow("Semantic Network", {
    visible: true,
    width: 500,
    height: 300,
  });
  const lines = ["Mode: human", ""];
  try {
    for (const name of names) {
      const info = QUESTIONS[name];
      if (!info) continue;
      const answer = await askHuman(actr, ctx, window, name);
      lines.push(`${name}: ${info.label} -> ${answer} (model: ${info.model})`);
      ctx.progress?.(`running... ${name}`);
    }
  } finally {
    actr.closeExpWindow(window);
  }
  return {
    kind: "semantic-dynamic",
    mode: "human",
    answers: lines.slice(2),
    report: lines.join("\n"),
  };
}

async function semanticDynamic(actr, ctx) {
  if (isHuman(ctx.controls)) return humanSemantic(actr, ctx);

  const question = String(ctx.controls.question || "all");
  const segments =
    question === "all"
      ? [
          { name: "g1", label: "g1", reset: true },
          { name: "g2", label: "g2", reset: true },
          { name: "g2", label: "g2 (repeat)", reset: false },
          { name: "g3", label: "g3", reset: true },
        ]
      : [{ name: question, label: question, reset: true }];

  const lines = [];
  for (const segment of segments) {
    const answer = await askModel(actr, ctx, segment.name, segment.reset);
    lines.push(`${segment.label}: ${answer}`);
    ctx.progress?.(`running... ${segment.label}`);
  }
  return { kind: "semantic-dynamic", mode: "model", report: lines.join("\n") };
}

export const semanticDynamicTask = {
  run: semanticDynamic,
  controls: [
    modeControl("model"),
    {
      id: "question",
      label: "Question",
      type: "select",
      options: [
        ["all", "All g1/g2/g3"],
        ["g1", "g1: canary category"],
        ["g2", "g2: canary wings"],
        ["g3", "g3: canary dangerous"],
      ],
      default: "all",
    },
  ],
  runSeconds: 5,
  runTimeMode: "unused",
};

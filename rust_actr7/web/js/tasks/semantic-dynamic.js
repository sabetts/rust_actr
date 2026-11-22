async function ask(actr, ctx, name, resetFirst) {
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

async function semanticDynamic(actr, ctx) {
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
    const answer = await ask(actr, ctx, segment.name, segment.reset);
    lines.push(`${segment.label}: ${answer}`);
    ctx.progress?.(`running... ${segment.label}`);
  }
  return { kind: "semantic-dynamic", report: lines.join("\n") };
}

export const semanticDynamicTask = {
  run: semanticDynamic,
  controls: [
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

function utilityText(value) {
  if (value == null) return "none";
  return Number(value).toFixed(3);
}

async function ulIssues(actr, ctx) {
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

export const ulIssuesTask = {
  run: ulIssues,
  controls: [],
  runSeconds: 20,
  runTimeMode: "used",
};

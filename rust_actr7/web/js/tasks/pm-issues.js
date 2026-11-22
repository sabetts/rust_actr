const ALPHABET = [
  "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m",
  "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z",
];

async function pmIssues(actr, ctx) {
  actr.reset();
  const letter = actr.permuteList(ALPHABET)[0];
  const indexed = ["z", ...ALPHABET];
  const requested = ctx.controls.prompt ?? "random";
  const prompt = requested === "next" || requested === "previous"
    ? requested
    : Number(actr.random(2)) === 0
      ? "next"
      : "previous";
  const time = 1500 + Number(actr.random(1000));
  const window = actr.openExpWindow("Simple task");

  actr.installDevice(window);
  let response = [];
  let responseTime = null;
  actr.addCommand("pm-issues-response", (model, key) => {
    if (responseTime == null) responseTime = actr.getTime(true);
    response.push(String(key).toUpperCase());
  });
  actr.monitorCommand("output-key", "pm-issues-response");

  actr.addTextToExpWindow(window, letter, { x: 130, y: 150 });
  actr.addCommand("pm-issue-display", (win, value) => {
    actr.clearExpWindow();
    actr.addTextToExpWindow(win, String(value).toLowerCase(), { x: 125, y: 150 });
  });
  actr.scheduleEventRelative(time, "pm-issue-display", [window, prompt], true);

  await ctx.paint();
  await actr.run(ctx.runSeconds ?? 10, { signal: ctx.signal });

  actr.removeCommand("pm-issue-display");
  actr.removeCommandMonitor("output-key", "pm-issues-response");
  actr.removeCommand("pm-issues-response");

  const keys = response.map((key) => key.toLowerCase());
  const shown = String(letter).toLowerCase();
  const first = keys[0];
  const second = keys[1];
  let correct = false;
  if (keys.length === 2 && responseTime != null && responseTime > time && first === shown) {
    const firstIndex = indexed.indexOf(first);
    const secondIndex = indexed.indexOf(second);
    correct = prompt === "next"
      ? firstIndex === secondIndex - 1
      : firstIndex === secondIndex + 1;
  }

  return {
    kind: "pm-issues",
    prompt,
    letter,
    response,
    correct,
    report: `Prompt: ${prompt} · letter: ${letter} · response: ${response.join(" ") || "none"} · ${correct ? "correct" : "incorrect"}`,
  };
}

export const pmIssuesTask = {
  run: pmIssues,
  controls: [
    {
      id: "prompt",
      label: "Prompt",
      type: "select",
      default: "random",
      options: [
        ["random", "Random"],
        ["next", "Next"],
        ["previous", "Previous"],
      ],
    },
  ],
  runSeconds: 10,
};

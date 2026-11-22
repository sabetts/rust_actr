const EXP_DATA = [0.664, 0.778, 0.804, 0.818];
const TRIALS = 48;
const BLOCK = 12;

function randomPoint(actr) {
  return {
    x: 35 + Number(actr.random(430)),
    y: 60 + Number(actr.random(180)),
  };
}

function addChoose(actr, window) {
  const point = randomPoint(actr);
  actr.addTextToExpWindow(window, "choose", { x: point.x, y: point.y });
}

function addFeedback(actr, window, answer) {
  const point = randomPoint(actr);
  actr.addTextToExpWindow(window, answer, { x: point.x, y: point.y });
}

function waitForKey(ctx, timeoutMs = 5000) {
  return new Promise((resolve) => {
    let done = false;
    const finish = (key) => {
      if (done) return;
      done = true;
      clearTimeout(timer);
      document.removeEventListener("keydown", onKey, true);
      resolve(key);
    };
    const onKey = (event) => {
      if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) {
        return;
      }
      const key = event.key.toLowerCase();
      if (key === "h" || key === "t") {
        event.preventDefault();
        finish(key);
      }
    };
    const timer = setTimeout(() => finish(""), timeoutMs);
    document.addEventListener("keydown", onKey, true);
  });
}

async function waitReal(ms) {
  await new Promise((resolve) => setTimeout(resolve, ms));
}

async function modelPass(actr, ctx, visible, response) {
  actr.reset();
  const window = actr.openExpWindow("Choice Experiment", {
    visible,
    width: 500,
    height: 300,
  });
  if (!visible && actr.window) actr.window.root.hidden = true;
  actr.installDevice(window);
  const blocks = Array.from({ length: 4 }, () => []);

  for (let trial = 0; trial < TRIALS; trial += 1) {
    actr.clearExpWindow(window);
    addChoose(actr, window);
    response.value = "";
    if (visible) await ctx.paint();
    await actr.run(5, { paint: visible });
    const key = String(response.value || "").toLowerCase();
    blocks[Math.floor(trial / BLOCK)].push(key === "h");

    actr.clearExpWindow(window);
    const answer = Number(actr.random(1.0)) < 0.9 ? "heads" : "tails";
    addFeedback(actr, window, answer);
    if (visible) await ctx.paint();
    await actr.run(1, { paint: visible });
  }
  return blocks;
}

async function humanPass(actr, ctx) {
  actr.reset();
  const window = actr.openExpWindow("Choice Experiment", {
    visible: true,
    width: 500,
    height: 300,
  });
  actr.installDevice(window);
  const blocks = Array.from({ length: 4 }, () => []);

  for (let trial = 0; trial < TRIALS; trial += 1) {
    actr.clearExpWindow(window);
    addChoose(actr, window);
    await ctx.paint();
    const key = await waitForKey(ctx, 5000);
    blocks[Math.floor(trial / BLOCK)].push(key === "h");

    actr.clearExpWindow(window);
    const answer = Number(actr.random(1.0)) < 0.9 ? "heads" : "tails";
    addFeedback(actr, window, answer);
    await ctx.paint();
    await waitReal(1000);
  }
  return blocks;
}

function statsFor(proportions) {
  const n = proportions.length;
  const meanX = proportions.reduce((a, b) => a + b, 0) / n;
  const meanY = EXP_DATA.reduce((a, b) => a + b, 0) / n;
  let num = 0;
  let dx = 0;
  let dy = 0;
  for (let i = 0; i < n; i += 1) {
    num += (proportions[i] - meanX) * (EXP_DATA[i] - meanY);
    dx += (proportions[i] - meanX) ** 2;
    dy += (EXP_DATA[i] - meanY) ** 2;
  }
  return {
    correlation: dx === 0 || dy === 0 ? 0 : num / Math.sqrt(dx * dy),
    meanDeviation: Math.sqrt(
      proportions.reduce((sum, value, i) => sum + (value - EXP_DATA[i]) ** 2, 0) / n
    ),
  };
}

function summarize(blocks) {
  const proportions = blocks.map((block) => block.filter(Boolean).length / block.length);
  return { proportions, ...statsFor(proportions) };
}

async function choice(actr, ctx) {
  const mode = String(ctx.controls.mode || "model");
  const runs = Math.max(1, Number(ctx.controls.runs) || 1);
  const visible = Boolean(ctx.controls.visible);
  if (actr.window) actr.window.headless = !visible || runs > 1;

  if (mode === "human") {
    const blocks = await humanPass(actr, ctx);
    const summary = summarize(blocks);
    return {
      kind: "choice",
      report: [
        "Mode: human",
        "Block   Heads proportion   Original",
        ...summary.proportions.map(
          (value, i) => `${i + 1}       ${value.toFixed(3)}              ${EXP_DATA[i].toFixed(3)}`
        ),
        "",
        `CORRELATION: ${summary.correlation.toFixed(3)}`,
        `MEAN DEVIATION: ${summary.meanDeviation.toFixed(3)}`,
      ].join("\n"),
    };
  }

  const response = { value: "" };
  actr.addCommand("choice-response", (_model, key) => {
    response.value = String(key ?? "").toLowerCase();
  });
  actr.monitorCommand("output-key", "choice-response");

  const totals = Array.from({ length: 4 }, () => 0);
  try {
    for (let run = 0; run < runs; run += 1) {
      const blocks = await modelPass(actr, ctx, visible, response);
      for (let i = 0; i < 4; i += 1) {
        totals[i] += blocks[i].filter(Boolean).length / BLOCK;
      }
    }
  } finally {
    actr.removeCommandMonitor("output-key", "choice-response");
    actr.removeCommand("choice-response");
  }

  const proportions = totals.map((value) => value / runs);
  const summary = statsFor(proportions);

  return {
    kind: "choice",
    report: [
      `Mode: model (${runs} run${runs === 1 ? "" : "s"})`,
      "Block   Heads proportion   Original",
      ...proportions.map(
        (value, i) => `${i + 1}       ${value.toFixed(3)}              ${EXP_DATA[i].toFixed(3)}`
      ),
      "",
      `CORRELATION: ${summary.correlation.toFixed(3)}`,
      `MEAN DEVIATION: ${summary.meanDeviation.toFixed(3)}`,
    ].join("\n"),
  };
}

export const choiceTask = {
  run: choice,
  controls: [
    {
      id: "mode",
      label: "Mode",
      type: "select",
      options: [
        ["model", "Model"],
        ["human", "Human"],
      ],
      default: "model",
    },
    {
      id: "runs",
      label: "Runs",
      type: "number",
      min: 1,
      max: 100,
      step: 1,
      default: 1,
    },
    {
      id: "visible",
      label: "Visible window",
      type: "checkbox",
      default: true,
    },
  ],
  runSeconds: 5,
};

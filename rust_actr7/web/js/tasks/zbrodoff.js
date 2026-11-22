const CONTROL_DATA = [1.84, 2.46, 2.82, 1.21, 1.45, 1.42, 1.14, 1.21, 1.17];

const DATA_SET = [
  ["a", "2", "c", "k"], ["d", "2", "f", "k"],
  ["b", "3", "e", "k"], ["e", "3", "h", "k"],
  ["c", "4", "g", "k"], ["f", "4", "j", "k"],
  ["a", "2", "d", "d"], ["d", "2", "g", "d"],
  ["b", "3", "f", "d"], ["e", "3", "i", "d"],
  ["c", "4", "h", "d"], ["f", "4", "k", "d"],
  ["a", "2", "c", "k"], ["d", "2", "f", "k"],
  ["b", "3", "e", "k"], ["e", "3", "h", "k"],
  ["c", "4", "g", "k"], ["f", "4", "j", "k"],
  ["a", "2", "d", "d"], ["d", "2", "g", "d"],
  ["b", "3", "f", "d"], ["e", "3", "i", "d"],
  ["c", "4", "h", "d"], ["f", "4", "k", "d"],
];

function makeTrial(block, arg1, arg2, sum, answer, visible) {
  return {
    block,
    addend2: arg2,
    text: `${arg1} + ${arg2} = ${sum}`,
    answer: String(answer).toLowerCase(),
    visible,
    correct: false,
    time: 0,
  };
}

function createSet(actr, block, visible) {
  return actr.permuteList(DATA_SET).map(([arg1, arg2, sum, answer]) =>
    makeTrial(block, arg1, arg2, sum, answer, visible)
  );
}

function presentTrial(actr, window, trial) {
  actr.clearExpWindow(window);
  actr.addTextToExpWindow(window, trial.text, { x: 100, y: 150 });
  trial.start = Number(actr.getTime(true));
}

function analyze(results) {
  const blocks = [];
  const addends = [];
  const data = new Map();
  const totals = new Map();

  for (const result of results) {
    totals.set(result.addend2, (totals.get(result.addend2) || 0) + 1);
    if (!result.correct) continue;
    const key = `${result.block}:${result.addend2}`;
    if (!data.has(key)) data.set(key, []);
    data.get(key).push(result.time);
    if (!blocks.includes(result.block)) blocks.push(result.block);
    if (!addends.includes(result.addend2)) addends.push(result.addend2);
  }

  blocks.sort((a, b) => Number(a) - Number(b));
  addends.sort();
  const rts = [];
  const counts = [];
  for (const block of blocks) {
    for (const addend of addends) {
      const values = data.get(`${block}:${addend}`) || [];
      rts.push(values.length ? values.reduce((a, b) => a + b, 0) / values.length : 0);
      counts.push(values.length);
    }
  }
  return { blocks, addends, rts, counts, totals };
}

function formatTable(data) {
  const lines = [
    "         " + data.addends.map((addend) => {
      const total = data.totals.get(addend) || 0;
      return `${String(addend).padStart(6)} (${String(Math.round(total / data.blocks.length)).padStart(2)}) `;
    }).join(""),
  ];
  data.blocks.forEach((block, blockIndex) => {
    let line = `Block  ${block}`;
    data.addends.forEach((_addend, addendIndex) => {
      const index = addendIndex + blockIndex * data.addends.length;
      line += ` ${data.rts[index].toFixed(3).padStart(6)} (${String(data.counts[index]).padStart(2)})`;
    });
    lines.push(line);
  });
  return lines;
}

function addAnalysis(total, data) {
  total.rts = total.rts.map((value, i) => value + (data.rts[i] || 0));
  total.counts = total.counts.map((value, i) => value + (data.counts[i] || 0));
}

async function collectResponses(actr, ctx, trials, visible) {
  const window = actr.openExpWindow("Alpha-arithmetic Experiment", {
    visible,
    width: 400,
    height: 300,
  });
  if (actr.window) actr.window.headless = !visible;
  actr.installDevice(window);

  const queue = [...trials];
  const results = [];
  let current = queue.shift();
  presentTrial(actr, window, current);
  if (visible) await ctx.paint();

  actr.addCommand("zbrodoff-response", (_model, key) => {
    if (!current) return;
    current.time = (Number(actr.getTime(true)) - current.start) / 1000;
    current.correct = String(key ?? "").toLowerCase() === current.answer;
    results.push(current);
    current = queue.shift();
    if (current) {
      presentTrial(actr, window, current);
    }
  });
  actr.monitorCommand("output-key", "zbrodoff-response");

  let keepWindow = false;
  try {
    await actr.run(10 * trials.length, { signal: ctx.signal });
    keepWindow = visible;
  } finally {
    actr.removeCommandMonitor("output-key", "zbrodoff-response");
    actr.removeCommand("zbrodoff-response");
    if (!keepWindow) actr.closeExpWindow(window);
  }
  if (keepWindow) {
    await ctx.paint();
    actr.window?.root.scrollIntoView({ block: "center" });
  }
  return results;
}

function reportFor(actr, mode, totals, games) {
  const data = totals;
  const lines = [];
  if (mode === "experiment" || mode === "compare") {
    const rts = data.rts.map((value) => value / games);
    const correlation = actr.correlation(rts, CONTROL_DATA, false);
    const deviation = actr.meanDeviation(rts, CONTROL_DATA, false);
    lines.push(`CORRELATION: ${Number(correlation).toFixed(3)}`);
    lines.push(`MEAN DEVIATION: ${Number(deviation).toFixed(3)}`);
  }
  const averageTotals = new Map();
  for (const [key, value] of data.totals) averageTotals.set(key, value / games);
  lines.push(...formatTable({
    blocks: data.blocks,
    addends: data.addends,
    rts: data.rts.map((value) => value / games),
    counts: data.counts.map((value) => value / games),
    totals: averageTotals,
  }));
  if (mode === "problem") {
    lines.push("", "Mode: single problem", "Starter model: counting strategy only.");
  } else {
    lines.push(
      "",
      "Starter model: counting strategy only.",
      "The Unit 4 assignment adds the retrieval strategy to improve the fit."
    );
  }
  return lines.join("\n");
}

async function zbrodoff(actr, ctx) {
  const mode = String(ctx.controls.mode || "experiment");
  const runs = mode === "experiment"
    ? Math.max(1, Number(ctx.controls.runs) || 1)
    : 1;
  const visible = mode === "problem" && Boolean(ctx.controls.visible);

  if (mode === "problem") {
    const arg1 = String(ctx.controls.arg1 || "a").toLowerCase();
    const arg2 = String(ctx.controls.arg2 || "2");
    const sum = String(ctx.controls.sum || "c").toLowerCase();
    const answer = String(ctx.controls.answer || "k").toLowerCase();
    const results = await collectResponses(
      actr,
      ctx,
      [makeTrial(1, arg1, arg2, sum, answer, visible)],
      visible
    );
    return {
      kind: "zbrodoff",
      report: reportFor(actr, "problem", analyze(results), 1),
    };
  }

  const totals = { blocks: [], addends: [], rts: [], counts: [], totals: new Map() };
  for (let run = 0; run < runs; run += 1) {
    const trials = [];
    if (mode === "experiment") {
      actr.reset();
      for (let block = 1; block <= 3; block += 1) {
        for (let set = 0; set < 8; set += 1) {
          trials.push(...createSet(actr, block, false));
        }
      }
    } else if (mode === "block") {
      for (let set = 0; set < 8; set += 1) {
        trials.push(...createSet(actr, 1, false));
      }
    } else {
      trials.push(...createSet(actr, 1, false));
    }

    const results = await collectResponses(actr, ctx, trials, false);
    const data = analyze(results);
    if (!totals.blocks.length) {
      totals.blocks = data.blocks;
      totals.addends = data.addends;
      totals.rts = data.rts;
      totals.counts = data.counts;
      totals.totals = data.totals;
    } else {
      addAnalysis(totals, data);
      for (const [key, count] of data.totals) {
        totals.totals.set(key, (totals.totals.get(key) || 0) + count);
      }
    }
    ctx.progress?.(`running... ${run + 1}/${runs}`);
  }

  return {
    kind: "zbrodoff",
    report: reportFor(actr, mode, totals, runs),
  };
}

export const zbrodoffTask = {
  run: zbrodoff,
  controls: [
    {
      id: "mode",
      label: "Mode",
      type: "select",
      options: [
        ["experiment", "Experiment"],
        ["block", "Block (192 trials)"],
        ["set", "Set (24 trials)"],
        ["problem", "Single problem"],
      ],
      default: "experiment",
    },
    {
      id: "runs",
      label: "Runs",
      type: "number",
      min: 1,
      max: 10,
      step: 1,
      default: 1,
      showWhen: (values) => values.mode === "experiment",
    },
    {
      id: "arg1",
      label: "Arg 1",
      type: "select",
      options: ["a", "b", "c", "d", "e", "f"].map((value) => [value, value.toUpperCase()]),
      default: "a",
      showWhen: (values) => values.mode === "problem",
    },
    {
      id: "arg2",
      label: "Addend",
      type: "select",
      options: [["2", "2"], ["3", "3"], ["4", "4"]],
      default: "2",
      showWhen: (values) => values.mode === "problem",
    },
    {
      id: "sum",
      label: "Sum",
      type: "select",
      options: ["c", "d", "e", "f", "g", "h", "i", "j", "k"].map(
        (value) => [value, value.toUpperCase()]
      ),
      default: "c",
      showWhen: (values) => values.mode === "problem",
    },
    {
      id: "answer",
      label: "Correct key",
      type: "select",
      options: [["k", "K (true)"], ["d", "D (false)"]],
      default: "k",
      showWhen: (values) => values.mode === "problem",
    },
    {
      id: "visible",
      label: "Visible window",
      type: "checkbox",
      default: true,
      showWhen: (values) => values.mode === "problem",
    },
  ],
  runSeconds: 10,
  runTimeMode: "unused",
};

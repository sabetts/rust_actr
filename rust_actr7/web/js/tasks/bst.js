import { RunCancelled } from "../actr.js";

const EXP_DATA = [20, 67, 20, 47, 87, 20, 80, 93, 83, 13, 29, 27, 80, 73, 53];

const EXP_STIMS = [
  [15, 250, 55, 125],
  [10, 155, 22, 101],
  [14, 200, 37, 112],
  [22, 200, 32, 114],
  [10, 243, 37, 159],
  [22, 175, 40, 73],
  [15, 250, 49, 137],
  [10, 179, 32, 105],
  [20, 213, 42, 104],
  [14, 237, 51, 116],
  [12, 149, 30, 72],
  [14, 237, 51, 121],
  [22, 200, 32, 114],
  [14, 200, 37, 112],
  [15, 250, 55, 125],
];

const DEMO_STIMS = [
  [15, 200, 41, 103],
  [10, 200, 29, 132],
];

function problemStims(value) {
  const index = Number(value) || 1;
  if (index === 2) return DEMO_STIMS[1];
  if (index === 3) return EXP_STIMS[0];
  return DEMO_STIMS[0];
}

function newState(stims) {
  return {
    stims,
    target: stims[3],
    current: 0,
    currentLine: null,
    done: false,
    choice: null,
    onDone: null,
  };
}

function press(actr, state, len, dir) {
  if (state.choice == null) state.choice = dir;
  if (state.done) return;
  state.current = state.current > state.target ? state.current - Number(len) : state.current + Number(len);
  updateLine(actr, state);
}

function reset(actr, state) {
  if (state.done) return;
  state.current = 0;
  updateLine(actr, state);
}

function updateLine(actr, state) {
  if (state.current === state.target) {
    state.done = true;
    if (state.currentLine) {
      actr.modifyLineForExpWindow(state.currentLine, [75, 135], [state.target + 75, 135]);
    }
    actr.addTextToExpWindow(state.window, "Done", { x: 180, y: 200 });
    if (state.onDone) state.onDone();
  } else if (state.current === 0) {
    if (state.currentLine) {
      actr.removeItemsFromExpWindow(state.window, state.currentLine);
      state.currentLine = null;
    }
  } else if (state.currentLine) {
    actr.modifyLineForExpWindow(state.currentLine, [75, 135], [state.current + 75, 135]);
  } else {
    state.currentLine = actr.addLineToExpWindow(
      state.window,
      [75, 135],
      [state.current + 75, 135],
      "blue"
    );
  }
}

function makeBstTask(config) {
  const buttonCommand = `${config.commandPrefix}-button-pressed`;
  const resetCommand = `${config.commandPrefix}-reset-button-pressed`;
  const label = config.label || "Building Sticks";
  const utilityNames = config.utilities || ["DECIDE-OVER", "DECIDE-UNDER"];
  const defaultRuns = config.defaultRuns || 100;
  const hooks = config.hooks || [];
  let currentState = null;

  function buildDisplay(actr, visible, stims) {
    const [a, b, c, goal] = stims;
    const window = actr.openExpWindow("Building Sticks Task", {
      visible,
      width: 600,
      height: 400,
    });
    if (!visible && actr.window) actr.window.root.hidden = true;

    const action = (len, dir) => [buttonCommand, len, dir];
    actr.addButtonToExpWindow(window, {
      text: "A", x: 5, y: 23, action: action(a, "under"), height: 24, width: 40,
    });
    actr.addButtonToExpWindow(window, {
      text: "B", x: 5, y: 48, action: action(b, "over"), height: 24, width: 40,
    });
    actr.addButtonToExpWindow(window, {
      text: "C", x: 5, y: 73, action: action(c, "under"), height: 24, width: 40,
    });
    actr.addButtonToExpWindow(window, {
      text: "Reset", x: 5, y: 123, action: resetCommand, height: 24, width: 65,
    });

    const lines = [
      actr.addLineToExpWindow(window, [75, 35], [a + 75, 35], "black"),
      actr.addLineToExpWindow(window, [75, 60], [b + 75, 60], "black"),
      actr.addLineToExpWindow(window, [75, 85], [c + 75, 85], "black"),
      actr.addLineToExpWindow(window, [75, 110], [goal + 75, 110], "green"),
    ];
    return { window, lines, stims };
  }

  function installCommands(actr, getState) {
    actr.addCommand(buttonCommand, (len, dir) => {
      const state = getState();
      if (state) press(actr, state, len, dir);
    });
    actr.addCommand(resetCommand, () => {
      const state = getState();
      if (state) reset(actr, state);
    });
  }

  function removeCommands(actr) {
    actr.removeCommand(buttonCommand);
    actr.removeCommand(resetCommand);
  }

  async function modelTrial(actr, ctx, stims, visible) {
    const display = buildDisplay(actr, visible, stims);
    const state = newState(stims);
    state.window = display.window;
    currentState = state;
    actr.installDevice(display.window);
    actr.startHandAtMouse();
    if (visible) await ctx.paint();
    await actr.run(ctx.runSeconds ?? 60, { paint: visible, signal: ctx.signal });
    return state;
  }

  async function humanTrial(actr, ctx, stims) {
    const state = newState(stims);
    const done = new Promise((resolve, reject) => {
      const abort = () => reject(new RunCancelled());
      state.onDone = () => {
        ctx.signal?.removeEventListener("abort", abort);
        resolve();
      };
      if (ctx.signal?.aborted) {
        abort();
        return;
      }
      ctx.signal?.addEventListener("abort", abort, { once: true });
    });
    if (actr.window) {
      actr.window.onButton = (_name, text) => {
        if (text === "Reset") {
          reset(actr, state);
          return;
        }
        const index = text === "A" ? 0 : text === "B" ? 1 : text === "C" ? 2 : -1;
        if (index < 0) return;
        const length = stims[index];
        const dir = index === 1 ? "over" : "under";
        press(actr, state, length, dir);
      };
    }
    const display = buildDisplay(actr, true, stims);
    state.window = display.window;
    currentState = state;
    await ctx.paint();
    await done;
    return state;
  }

  async function bst(actr, ctx) {
    const mode = String(ctx.controls.mode || "report");
    const hookNames = [];
    for (const hook of hooks) {
      actr.addCommand(hook.name, hook.make(actr));
      hookNames.push(hook.name);
    }
    if (actr.window) actr.window.headless = mode === "report";
    installCommands(actr, () => currentState);
    try {
      if (mode === "human") {
        const stims = problemStims(ctx.controls.problem);
        const state = await humanTrial(actr, ctx, stims);
        return {
          kind: "bst",
          label,
          report: [
            "Mode: human",
            `Sticks: ${stims.join(" / ")}`,
            `First choice: ${state.choice ?? "none"}`,
            `Solved: ${state.done ? "yes" : "no"}`,
          ].join("\n"),
        };
      }

      if (mode === "demo") {
        const stims = problemStims(ctx.controls.problem);
        const state = await modelTrial(actr, ctx, stims, true);
        return {
          kind: "bst",
          label,
          report: [
            "Mode: demo",
            `Sticks: ${stims.join(" / ")}`,
            `First choice: ${state.choice ?? "none"}`,
            `Solved: ${state.done ? "yes" : "no"}`,
          ].join("\n"),
        };
      }

      const runs = Math.max(1, Number(ctx.controls.runs) || defaultRuns);
      const counts = Array(EXP_STIMS.length).fill(0);
      const utilities = {};
      for (const name of utilityNames) utilities[name] = 0;

      for (let run = 0; run < runs; run += 1) {
        actr.reset();
        for (let i = 0; i < EXP_STIMS.length; i += 1) {
          const state = await modelTrial(actr, ctx, EXP_STIMS[i], false);
          if (state.choice === "over") counts[i] += 1;
        }
        for (const name of Object.keys(utilities)) {
          utilities[name] += Number(actr.productionUtility(name)) || 0;
        }
        ctx.progress?.(`running... ${run + 1}/${runs}`);
      }

      const percentages = counts.map((count) => (100 * count) / runs);
      const correlation = actr.correlation(percentages, EXP_DATA, false);
      const meanDeviation = actr.meanDeviation(percentages, EXP_DATA, false);
      const report = [
        `Mode: report (${runs} runs)`,
        "",
        "Trial " + Array.from({ length: 15 }, (_, i) => String(i + 1).padEnd(8)).join(""),
        "  " + percentages.map((value) => value.toFixed(2).padStart(8)).join(""),
        "",
        `CORRELATION: ${Number(correlation).toFixed(3)}`,
        `MEAN DEVIATION: ${Number(meanDeviation).toFixed(3)}`,
        "",
        ...Object.entries(utilities).map(
          ([name, total]) => `${name.padEnd(12)}: ${(total / runs).toFixed(4)}`
        ),
      ];
      return { kind: "bst", label, report: report.join("\n") };
    } finally {
      removeCommands(actr);
      for (const name of hookNames) actr.removeCommand(name);
    }
  }

  return {
    run: bst,
    prelude: config.prelude || null,
    controls: [
      {
        id: "mode",
        label: "Mode",
        type: "select",
        options: [
          ["report", "Report"],
          ["demo", "Visible demo"],
          ["human", "Human"],
        ],
        default: "report",
      },
      {
        id: "problem",
        label: "Problem",
        type: "select",
        options: [
          ["1", "Demo 1"],
          ["2", "Demo 2"],
          ["3", "Experiment 1"],
        ],
        default: "1",
        showWhen: (values) => values.mode === "demo" || values.mode === "human",
      },
      {
        id: "runs",
        label: "Report runs",
        type: "number",
        min: 1,
        max: 100,
        step: 1,
        default: defaultRuns,
        showWhen: (values) => values.mode === "report",
      },
    ],
    runSeconds: 60,
    runTimeMode: (values) => (values.mode === "human" ? "unused" : "per-trial"),
  };
}

function bstNumberSims(a, b) {
  if (typeof a === "number" && typeof b === "number") {
    return Math.abs(a - b) / -300;
  }
  return null;
}

function bstComputeDifference(actr) {
  return () => {
    const current = actr.bufferRead("imaginal");
    if (!current) return null;
    const copy = actr.copyChunk(current);
    const length = Number(actr.chunkSlotValue(current, "length"));
    const goalLength = Number(actr.chunkSlotValue(current, "goal-length"));
    actr.modChunk(copy, ["difference", Math.abs(length - goalLength)]);
    return copy;
  };
}

export const bstTask = makeBstTask({
  commandPrefix: "bst",
  label: "Building Sticks",
  utilities: ["DECIDE-OVER", "DECIDE-UNDER", "FORCE-OVER", "FORCE-UNDER"],
  defaultRuns: 100,
});

export const bstPpmTask = makeBstTask({
  commandPrefix: "bst-ppm",
  label: "Building Sticks (PPM)",
  utilities: ["DECIDE-OVER", "DECIDE-UNDER"],
  defaultRuns: 10,
  hooks: [
    { name: "bst-number-sims", make: () => bstNumberSims },
    { name: "bst-compute-difference", make: bstComputeDifference },
  ],
});

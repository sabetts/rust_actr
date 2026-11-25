import {
  clearHint,
  modeControl,
  showHint,
  waitForKey,
} from "./human.js";

const TRIALS = 150;
const BLOCKS = 15;
const RESULTS = [
  [
    ["win", "lose", "draw", "lose"],
    ["win", "lose", "draw", "lose"],
    ["win", "lose", "draw", "lose"],
    ["lose", "draw", "win", "lose"],
  ],
  [
    ["win", "lose", "draw", "lose"],
    ["win", "lose", "draw", "lose"],
    ["lose", "draw", "win", "lose"],
    ["lose", "draw", "win", "lose"],
  ],
  [
    ["win", "lose", "draw", "lose"],
    ["win", "lose", "draw", "lose"],
    ["lose", "draw", "win", "lose"],
    ["lose", "draw", "win", "lose"],
  ],
  [
    ["win", "lose", "draw", "lose"],
    ["lose", "draw", "win", "lose"],
    ["lose", "draw", "win", "lose"],
    ["lose", "draw", "win", "lose"],
  ],
];

function responseIndex(key) {
  const value = String(key ?? "").toLowerCase();
  if (value === "s") return 0;
  if (value === "d") return 1;
  if (value === "f") return 2;
  return 3;
}

function presentTrial(actr, window, state, human = false) {
  actr.clearExpWindow(window);
  state.val1 = Number(actr.random(4));
  state.val2 = Number(actr.random(4));
  actr.addTextToExpWindow(window, String(state.val1), { x: 10, y: 50 });
  actr.addTextToExpWindow(window, String(state.val2), { x: 110, y: 50 });
  state.start = human ? performance.now() : Number(actr.getTime(true));
  state.taskState = "trial";
}

function presentFeedback(actr, window, result) {
  actr.clearExpWindow(window);
  actr.addTextToExpWindow(window, result, { x: 50, y: 100 });
}

async function runGame(actr, ctx, compile) {
  actr.reset();
  actr.setParameterValue(":epl", compile);
  actr.setParameterValue(":pct", false);
  const window = actr.openExpWindow("Compilation task", {
    visible: false,
    width: 300,
    height: 300,
  });
  if (actr.window) actr.window.headless = true;
  actr.installDevice(window);

  const state = {
    taskState: "trial",
    val1: 0,
    val2: 0,
    start: 0,
    responses: [],
    times: [],
    finished: false,
  };

  actr.addCommand("pcomp-response", (_model, key) => {
    const press = String(key ?? "").toLowerCase();
    if (state.taskState === "trial") {
      const result = RESULTS[state.val1][state.val2][responseIndex(press)];
      state.responses.push(result);
      state.times.push(Number(actr.getTime(true)) - state.start);
      state.taskState = "feedback";
      presentFeedback(actr, window, result);
    } else if (state.taskState === "feedback" && press === "space") {
      if (state.responses.length >= TRIALS) {
        state.finished = true;
      } else {
        presentTrial(actr, window, state);
      }
    }
  });
  actr.monitorCommand("output-key", "pcomp-response");

  try {
    presentTrial(actr, window, state);
    await actr.run(10 * TRIALS, { signal: ctx.signal });
  } finally {
    actr.removeCommandMonitor("output-key", "pcomp-response");
    actr.removeCommand("pcomp-response");
    actr.closeExpWindow(window);
  }
  return state;
}

async function runHumanGame(actr, ctx) {
  actr.reset();
  const window = actr.openExpWindow("Compilation task", {
    visible: true,
    width: 300,
    height: 300,
  });
  if (actr.window) actr.window.headless = false;

  const state = {
    taskState: "trial",
    val1: 0,
    val2: 0,
    start: 0,
    responses: [],
    times: [],
    finished: false,
  };

  showHint(actr, "Press S, D, or F for your choice, then Space to continue.");
  try {
    for (let trial = 0; trial < TRIALS; trial += 1) {
      presentTrial(actr, window, state, true);
      await ctx.paint();
      const answer = await waitForKey(ctx, { keys: ["s", "d", "f"], timeoutMs: 30000 });
      if (answer == null) break;
      const result = RESULTS[state.val1][state.val2][responseIndex(answer.key)];
      state.responses.push(result);
      state.times.push(answer.at - state.start);
      state.taskState = "feedback";
      presentFeedback(actr, window, result);
      await ctx.paint();
      const next = await waitForKey(ctx, { keys: ["space"], timeoutMs: 30000 });
      if (next == null) break;
      ctx.progress?.(`running... ${trial + 1}/${TRIALS}`);
    }
    state.finished = state.responses.length >= TRIALS;
  } finally {
    clearHint(actr);
    actr.closeExpWindow(window);
  }
  return state;
}

function summarize(games) {
  const scores = Array(BLOCKS).fill(0);
  const times = Array(BLOCKS).fill(0);
  const counts = Array(BLOCKS).fill(0);

  for (const state of games) {
    state.responses.forEach((result, index) => {
      const block = Math.floor(index / 10);
      if (block >= BLOCKS) return;
      if (result === "win") scores[block] += 1;
      if (result === "lose") scores[block] -= 1;
      times[block] += state.times[index] || 0;
      counts[block] += 1;
    });
  }

  return {
    scores,
    times: times.map((value, i) => (counts[i] ? value / counts[i] / 1000 : 0)),
  };
}

async function pcompIssues(actr, ctx) {
  const human = String(ctx.controls.mode || "model") === "human";
  const gameCount = human
    ? 1
    : Math.max(1, Math.min(10, Number(ctx.controls.games) || 1));
  const compile = !human && Boolean(ctx.controls.compile);
  const games = [];
  for (let game = 0; game < gameCount; game += 1) {
    games.push(
      await (human ? runHumanGame(actr, ctx) : runGame(actr, ctx, compile))
    );
    ctx.progress?.(`running... ${game + 1}/${gameCount}`);
  }

  const data = summarize(games);
  const averages = data.scores.map((value) => value / gameCount);
  const completed = games.map((game) => game.responses.length);
  const lines = [
    ...(human ? ["Mode: human"] : []),
    `Average Score of ${gameCount} trial${gameCount === 1 ? "" : "s"}`,
    averages.map((value) => value.toFixed(2).padStart(6)).join(""),
    "Average Response times",
    data.times.map((value) => value.toFixed(2).padStart(6)).join(""),
    "",
    `Completed trials: ${completed.join(", ")} / ${TRIALS} per game`,
  ];
  if (human) {
    lines.push("Human mode: one game of 150 trials.");
  } else {
    lines.push(
      `Compilation: ${compile ? "on" : "off"}`,
      "Canonical starter model; the tutorial's utility/reward edits are not applied."
    );
  }
  return {
    kind: "pcomp-issues",
    mode: human ? "human" : "model",
    report: lines.join("\n"),
  };
}

export const pcompIssuesTask = {
  run: pcompIssues,
  controls: [
    modeControl("model"),
    {
      id: "games",
      label: "Games",
      type: "number",
      min: 1,
      max: 10,
      step: 1,
      default: 1,
      showWhen: (values) => values.mode !== "human",
    },
    {
      id: "compile",
      label: "Compilation",
      type: "checkbox",
      default: false,
      showWhen: (values) => values.mode !== "human",
    },
  ],
  runSeconds: 10,
  runTimeMode: "unused",
};

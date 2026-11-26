import { RunCancelled } from "../actr.js";

const NUMERIC_SIM_HOOK = `
(defun 1hit-bj-number-sims (a b)
  (if (and (numberp a) (numberp b))
      (- (/ (abs (- a b)) (max a b)))
      nil))
`;

function scoreCards(cards) {
  let total = cards.reduce((sum, card) => sum + Number(card), 0);
  for (let i = 0; i < cards.filter((card) => Number(card) === 1).length; i += 1) {
    if (total + 10 <= 21) total += 10;
  }
  return total;
}

function computeOutcome(cards, otherCards) {
  const total = scoreCards(cards);
  const otherTotal = scoreCards(otherCards);
  if (total > 21) return "bust";
  if (otherTotal > 21) return "win";
  return total > otherTotal ? "win" : "lose";
}

function regularDeck(actr) {
  return Math.min(10, Number(actr.random(13)) + 1);
}

function deal(actr) {
  return [regularDeck(actr), regularDeck(actr), regularDeck(actr)];
}

function cardValue(value, hidden = false) {
  const card = document.createElement("span");
  card.className = hidden ? "bj-card hidden-card" : "bj-card";
  card.textContent = hidden ? "" : String(value);
  return card;
}

function row(label, cards, hiddenIndex = -1) {
  const element = document.createElement("div");
  element.className = "bj-row";
  const name = document.createElement("span");
  name.className = "bj-label";
  name.textContent = label;
  element.append(name);
  cards.forEach((card, i) => element.append(cardValue(card, i === hiddenIndex)));
  return element;
}

function renderStage(ctx, state) {
  const stage = ctx.stage;
  if (!stage) throw new Error("This task needs the blackjack stage element.");
  stage.hidden = false;
  stage.replaceChildren();

  const table = document.createElement("div");
  table.className = "bj-table";
  table.append(row("You", state.humanCards, state.hideHuman ? 1 : -1));
  table.append(row("Model", state.modelCards, state.hideModel ? 1 : -1));
  if (state.result) {
    const result = document.createElement("div");
    result.className = "bj-result";
    result.textContent = state.result;
    table.append(result);
  }
  if (state.hint) {
    const hint = document.createElement("div");
    hint.className = "bj-status";
    hint.textContent = state.hint;
    table.append(hint);
  }
  stage.append(table);

  if (state.actions) {
    const actions = document.createElement("div");
    actions.className = "bj-actions";
    for (const [choice, label] of state.actions) {
      const button = document.createElement("button");
      button.type = "button";
      button.dataset.choice = choice;
      button.textContent = label;
      actions.append(button);
    }
    table.append(actions);
  }
}

function waitForChoice(ctx, seconds) {
  return new Promise((resolve, reject) => {
    const stage = ctx.stage;
    let done = false;
    let timer = null;

    const cleanup = () => {
      if (timer) clearTimeout(timer);
      document.removeEventListener("keydown", onKey, true);
      ctx.signal?.removeEventListener("abort", abort);
    };
    const finish = (choice) => {
      if (done) return;
      done = true;
      cleanup();
      stage.querySelectorAll("button[data-choice]").forEach((button) => {
        button.disabled = true;
      });
      resolve(choice);
    };
    const abort = () => {
      if (done) return;
      done = true;
      cleanup();
      stage.querySelectorAll("button[data-choice]").forEach((button) => {
        button.disabled = true;
      });
      reject(new RunCancelled());
    };
    const onKey = (event) => {
      if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) {
        return;
      }
      const key = event.key.toLowerCase();
      if (key === "h" || key === "s") {
        event.preventDefault();
        finish(key);
      }
    };

    stage.querySelectorAll("button[data-choice]").forEach((button) => {
      button.addEventListener("click", () => finish(button.dataset.choice));
    });
    document.addEventListener("keydown", onKey, true);
    ctx.signal?.addEventListener("abort", abort, { once: true });
    if (ctx.signal?.aborted) abort();
    timer = setTimeout(() => finish("s"), Math.max(1, seconds) * 1000);
  });
}

function waitForNext(ctx, seconds) {
  return new Promise((resolve, reject) => {
    const stage = ctx.stage;
    let done = false;
    let timer = null;
    const cleanup = () => {
      if (timer) clearTimeout(timer);
      document.removeEventListener("keydown", onKey, true);
      ctx.signal?.removeEventListener("abort", abort);
    };
    const finish = () => {
      if (done) return;
      done = true;
      cleanup();
      stage.querySelectorAll("button[data-next]").forEach((button) => {
        button.disabled = true;
      });
      resolve();
    };
    const abort = () => {
      if (done) return;
      done = true;
      cleanup();
      stage.querySelectorAll("button[data-next]").forEach((button) => {
        button.disabled = true;
      });
      reject(new RunCancelled());
    };
    const onKey = (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        finish();
      }
    };
    stage.querySelectorAll('button[data-next], button[data-choice="next"]').forEach((button) => {
      button.addEventListener("click", finish);
    });
    document.addEventListener("keydown", onKey, true);
    ctx.signal?.addEventListener("abort", abort, { once: true });
    if (ctx.signal?.aborted) abort();
    timer = setTimeout(finish, Math.max(0, seconds) * 1000);
  });
}

function goalSlots(state, mcards, ocards, mres, ores) {
  const start = state === "start";
  const m3 = mcards.length === 3 ? mcards[2] : null;
  const o3 = ocards.length === 3 ? ocards[2] : null;
  return [
    "isa", "game-state",
    "mc1", mcards[0],
    "mc2", mcards[1],
    "mc3", m3,
    "mstart", scoreCards(mcards.slice(0, 2)),
    "mtot", start ? null : scoreCards(mcards),
    "mresult", mres ?? null,
    "oc1", ocards[0],
    "oc2", ocards.length > 1 ? ocards[1] : null,
    "oc3", o3,
    "ostart", scoreCards(ocards.slice(0, 1)),
    "otot", start ? null : scoreCards(ocards),
    "oresult", ores ?? null,
    "state", state,
  ];
}

function showModelCards(actr, goal, mcards, ocard) {
  const slots = goalSlots("start", mcards, [ocard], null, null);
  if (!goal.initialized) {
    actr.goalFocus(slots);
    goal.initialized = true;
  } else {
    actr.modFocus(slots);
  }
}

function showModelResults(actr, goal, mcards, ocards, mres, ores) {
  actr.modFocus(goalSlots("results", mcards, ocards, mres, ores));
}

function addScores(total, score) {
  return total.map((value, i) => value + score[i]);
}

function newScores() {
  return [0, 0, 0, 0];
}

async function playHand(
  actr,
  goal,
  monitor,
  ctx,
  human,
  seconds,
  more,
  handLabel = ""
) {
  const mcards = deal(actr);
  const ocards = deal(actr);
  showModelCards(actr, goal, mcards, ocards[0]);
  monitor.action = "s";
  await actr.runFullTime(10, { signal: ctx.signal });
  const mchoice = monitor.action;
  let hchoice = "s";

  if (human) {
    renderStage(ctx, {
      humanCards: ocards.slice(0, 2),
      modelCards: mcards.slice(0, 2),
      hideModel: true,
      hideHuman: false,
      hint: `${handLabel ? `${handLabel} · ` : ""}Hit or stay? (${seconds}s)`,
      actions: [["h", "Hit"], ["s", "Stay"]],
    });
    const choice = waitForChoice(ctx, seconds);
    await ctx.paint();
    hchoice = await choice;
  }

  const mfinal = mchoice === "h" ? mcards : mcards.slice(0, 2);
  const humanFinal = human
    ? (hchoice === "h" ? ocards : ocards.slice(0, 2))
    : (scoreCards(ocards.slice(0, 2)) < 15 ? ocards : ocards.slice(0, 2));
  const mres = computeOutcome(mfinal, humanFinal);
  const hres = computeOutcome(humanFinal, mfinal);

  showModelResults(actr, goal, mfinal, humanFinal, mres, hres);
  await actr.runFullTime(10, { signal: ctx.signal });

  const mtot = scoreCards(mfinal);
  const htot = scoreCards(humanFinal);
  const score = newScores();
  if (mres === "win") score[0] += 1;
  if (hres === "win") score[1] += 1;
  if (mres === "bust" && hres === "bust") score[2] += 1;
  if (mtot === htot && mres !== "bust" && hres !== "bust") score[3] += 1;

  if (human) {
    renderStage(ctx, {
      humanCards: humanFinal,
      modelCards: mfinal,
      hideModel: false,
      hideHuman: false,
      result: `You: ${htot} (${hres})   Model: ${mtot} (${mres})`,
      actions: more ? [["next", "Next hand"]] : [],
    });
    const next = more ? waitForNext(ctx, 3000) : null;
    await ctx.paint();
    if (next) await next;
  }
  return score;
}

function reportLearning(games, blocks, wins) {
  const perBlock = blocks.map((block) => (block[0] / games / 5) * 100);
  const four = [
    perBlock.slice(0, 5).reduce((a, b) => a + b, 0) / 5,
    perBlock.slice(5, 10).reduce((a, b) => a + b, 0) / 5,
    perBlock.slice(10, 15).reduce((a, b) => a + b, 0) / 5,
    perBlock.slice(15, 20).reduce((a, b) => a + b, 0) / 5,
  ];
  const lines = [
    `Mode: learn`,
    `Games: ${games} (${games * 100} hands)`,
    "Block of 5 hands    Model win %",
  ];
  perBlock.forEach((value, i) => lines.push(`${String(i + 1).padStart(2)}                  ${value.toFixed(1)}`));
  lines.push("", "Four blocks of 25 hands");
  four.forEach((value, i) => lines.push(`${i + 1}: ${value.toFixed(1)}%`));
  lines.push("", `Overall model wins: ${wins} / ${games * 100} (${((wins / (games * 100)) * 100).toFixed(1)}%)`);
  return lines.join("\n");
}

async function learn(actr, ctx) {
  const games = Math.max(1, Number(ctx.controls.games) || 1);
  const goal = { initialized: false };
  const monitor = { action: "s" };
  actr.addCommand("onehit-key", (model, key) => {
    if (model) monitor.action = String(key).toLowerCase();
  });
  actr.monitorCommand("output-key", "onehit-key");

  const blocks = Array.from({ length: 20 }, newScores);
  try {
    for (let game = 0; game < games; game += 1) {
      actr.reset();
      goal.initialized = false;
      for (let block = 0; block < 20; block += 1) {
        for (let hand = 0; hand < 5; hand += 1) {
          blocks[block] = addScores(
            blocks[block],
            await playHand(actr, goal, monitor, ctx, false, 10, false)
          );
        }
      }
    }
  } finally {
    actr.removeCommandMonitor("output-key", "onehit-key");
    actr.removeCommand("onehit-key");
  }

  const wins = blocks.reduce((sum, block) => sum + block[0], 0);
  return {
    kind: "onehit",
    mode: "learn",
    report: reportLearning(games, blocks, wins),
  };
}

async function play(actr, ctx) {
  const hands = Math.max(1, Number(ctx.controls.hands) || 5);
  const seconds = Math.max(1, Number(ctx.controls.seconds) || 10);
  const goal = { initialized: false };
  const monitor = { action: "s" };
  actr.addCommand("onehit-key", (model, key) => {
    if (model) monitor.action = String(key).toLowerCase();
  });
  actr.monitorCommand("output-key", "onehit-key");

  actr.reset();
  let scores = newScores();
  try {
    for (let hand = 0; hand < hands; hand += 1) {
      ctx.progress?.(`running... hand ${hand + 1}/${hands}`);
      scores = addScores(
        scores,
        await playHand(
          actr,
          goal,
          monitor,
          ctx,
          true,
          seconds,
          hand + 1 < hands,
          `Hand ${hand + 1}/${hands}`
        )
      );
    }
  } finally {
    actr.removeCommandMonitor("output-key", "onehit-key");
    actr.removeCommand("onehit-key");
  }

  return {
    kind: "onehit",
    mode: "play",
    report: [
      "Mode: play against model",
      `Hands: ${hands}`,
      `You: ${scores[1]} wins`,
      `Model: ${scores[0]} wins`,
      `Both bust: ${scores[2]}`,
      `Ties: ${scores[3]}`,
    ].join("\n"),
  };
}

async function onehit(actr, ctx) {
  return String(ctx.controls.mode || "learn") === "play"
    ? play(actr, ctx)
    : learn(actr, ctx);
}

export const onehitTask = {
  run: onehit,
  prelude: NUMERIC_SIM_HOOK,
  runTimeMode: "unused",
  controls: [
    {
      id: "mode",
      label: "Mode",
      type: "select",
      options: [
        ["learn", "Learn (automated)"],
        ["play", "Play against model"],
      ],
      default: "learn",
    },
    {
      id: "games",
      label: "Games",
      type: "number",
      min: 1,
      max: 50,
      step: 1,
      default: 1,
      showWhen: (values) => values.mode === "learn",
    },
    {
      id: "hands",
      label: "Hands",
      type: "number",
      min: 1,
      max: 50,
      step: 1,
      default: 5,
      showWhen: (values) => values.mode === "play",
    },
    {
      id: "seconds",
      label: "Decision time (s)",
      type: "number",
      min: 1,
      max: 30,
      step: 1,
      default: 10,
      showWhen: (values) => values.mode === "play",
    },
  ],
};

import {
  humanRandomInt,
  humanShuffle,
  isHuman,
  modeControl,
  waitForKey,
} from "./human.js";

const LETTERS = [
  "B", "C", "D", "F", "G", "H", "J", "K", "L",
  "M", "N", "P", "Q", "R", "S", "T", "V", "W",
  "X", "Y", "Z",
];

function sameKey(response, target) {
  return String(response ?? "").toLowerCase() === String(target ?? "").toLowerCase();
}

async function demo2(actr, ctx) {
  const human = isHuman(ctx.controls);
  actr.reset();
  const items = human ? humanShuffle(LETTERS) : actr.permuteList(LETTERS);
  const text1 = items[0];
  const window = actr.openExpWindow("Letter recognition");
  actr.addTextToExpWindow(window, text1, { x: 125, y: 150 });

  let response = null;

  if (human) {
    await ctx.paint();
    const answer = await waitForKey(ctx, { timeoutMs: 30000 });
    response = answer?.key ?? null;
    if (response != null) actr.clearExpWindow();
    await ctx.paint();
  } else {
    actr.addCommand("demo2-key-press", (model, key) => {
      response = key;
      actr.clearExpWindow();
    });
    actr.monitorCommand("output-key", "demo2-key-press");
    actr.installDevice(window);

    await ctx.paint();
    await actr.run(ctx.secs, { signal: ctx.signal });

    actr.removeCommandMonitor("output-key", "demo2-key-press");
    actr.removeCommand("demo2-key-press");
  }

  return {
    kind: "demo2",
    mode: human ? "human" : "model",
    window: "Letter recognition",
    stimuli: [text1],
    response,
    target: text1,
    correct: response != null && sameKey(response, text1),
  };
}

async function assignment(actr, ctx) {
  const human = isHuman(ctx.controls);
  actr.reset();
  const items = human ? humanShuffle(LETTERS) : actr.permuteList(LETTERS);
  const target = items[0];
  const foil = items[1];
  const index = human ? humanRandomInt(3) : Number(actr.random(3));
  const texts = [foil, foil, foil];
  texts[index] = target;

  const window = actr.openExpWindow("Letter difference");
  actr.addTextToExpWindow(window, texts[0], { x: 125, y: 75 });
  actr.addTextToExpWindow(window, texts[1], { x: 75, y: 175 });
  actr.addTextToExpWindow(window, texts[2], { x: 175, y: 175 });

  let response = null;

  if (human) {
    await ctx.paint();
    const answer = await waitForKey(ctx, { timeoutMs: 30000 });
    response = answer?.key ?? null;
  } else {
    actr.addCommand("unit2-key-press", (model, key) => {
      response = key;
    });
    actr.monitorCommand("output-key", "unit2-key-press");
    actr.installDevice(window);

    await ctx.paint();
    await actr.run(ctx.secs, { signal: ctx.signal });

    actr.removeCommandMonitor("output-key", "unit2-key-press");
    actr.removeCommand("unit2-key-press");
  }

  return {
    kind: "assignment",
    mode: human ? "human" : "model",
    window: "Letter difference",
    stimuli: texts,
    target,
    response,
    correct: response == null ? null : sameKey(response, target),
  };
}

const controls = [modeControl("model")];

function runTimeMode(values) {
  return isHuman(values) ? "unused" : "per-trial";
}

export const unit2Tasks = {
  "unit2-demo2": { run: demo2, controls, runSeconds: 10, runTimeMode },
  "unit2-assignment": { run: assignment, controls, runSeconds: 10, runTimeMode },
};

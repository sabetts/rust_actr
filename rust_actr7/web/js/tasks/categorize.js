import { isHuman, modeControl, waitReal } from "./human.js";
import { createChoicePrompt } from "./human-controls.js";

const CAT1 = [
  ["small", "large", "medium", "small"],
  ["medium", "small", "large", "medium"],
  ["small", "medium", "medium", "medium"],
  ["small", "medium", "large", "medium"],
  ["small", "medium", "large", "large"],
];

const CAT2 = [
  ["large", "small", "small", "small"],
  ["medium", "medium", "small", "large"],
  ["large", "small", "large", "small"],
  ["large", "small", "large", "large"],
  ["large", "small", "small", "large"],
];

const CAT_DATA = [
  0.975, 0.85, 0.987, 1.0, 0.963, 0.075, 0.138,
  0.087, 0.05, 0.025, 0.937, 0.544, 0.988, 0.087,
];

const STIMS = [
  [-1.025, 0.493, 0.048, -0.666],
  [-0.172, -0.557, 0.337, 0.163],
  [-0.98, 0.275, -0.005, -0.067],
  [-0.951, 0.259, 0.399, 0.093],
  [-0.96, 0.198, 0.38, 0.527],
  [0.665, -0.441, -0.508, -0.396],
  [-0.059, 0.243, -0.602, 0.624],
  [0.586, -0.511, 0.381, -0.507],
  [0.823, -0.539, 0.332, 0.633],
  [0.823, -0.504, -0.487, 0.776],
  [-1.114, -0.52, 0.636, -0.028],
  [-0.154, -0.562, -0.043, 0.057],
  [-0.856, 0.197, 0.241, 0.007],
  [0.704, -0.287, -0.164, 0.178],
];

const DEFAULT_SLOTS = ["eh", "es", "nl", "mh"];
const SLOT_PATTERN = /^[a-z][a-z0-9-]*$/i;

function normalizeSlots(controls) {
  const slots = ["slot1", "slot2", "slot3", "slot4"].map((id) =>
    String(controls[id] ?? "").trim().toLowerCase()
  );

  let warning = "";
  if (slots.some((slot) => !slot)) {
    warning = "Slot names must be non-empty. Using default slots.";
  } else if (slots.some((slot) => !SLOT_PATTERN.test(slot))) {
    warning = "Slot names must be simple symbols. Using default slots.";
  } else if (new Set(slots).size !== 4) {
    warning = "Slot names must be unique. Using default slots.";
  } else if (slots.includes("category")) {
    warning = "Slot name category cannot be used. Using default slots.";
  }

  return warning
    ? { slots: DEFAULT_SLOTS.slice(), warning }
    : { slots, warning: "" };
}

function normalizeOffset(controls) {
  const value = Number(controls.offset);
  return Number.isFinite(value) ? value : 0;
}

function sizeWord(value) {
  if (value <= -0.45) return "small";
  if (value >= 0.45) return "large";
  return "medium";
}

function featureText(words, slots) {
  return words
    .map((word, index) => `${String(slots[index] ?? `slot${index + 1}`).toUpperCase()} ${word}`)
    .join("   ");
}

async function humanCategorize(actr, ctx) {
  const studySeconds = Math.max(
    1,
    Math.min(30, Number(ctx.controls.studySeconds) || 10)
  );
  const { slots } = normalizeSlots(ctx.controls);
  actr.reset();
  const window = actr.openExpWindow("Categorize", {
    visible: true,
    width: 680,
    height: 420,
  });

  const results = [];
  try {
    actr.addTextToExpWindow(window, "Category 1", { x: 40, y: 24 });
    actr.addTextToExpWindow(window, "Category 2", { x: 350, y: 24 });
    CAT1.forEach((example, index) => {
      actr.addTextToExpWindow(window, example.join(" "), {
        x: 40,
        y: 60 + index * 32,
      });
      actr.addTextToExpWindow(window, CAT2[index].join(" "), {
        x: 350,
        y: 60 + index * 32,
      });
    });
    actr.addTextToExpWindow(window, "Study the examples. Test begins shortly.", {
      x: 160,
      y: 260,
    });
    await ctx.paint();
    await waitReal(studySeconds * 1000, ctx.signal);

    for (let index = 0; index < STIMS.length; index += 1) {
      const words = STIMS[index].map(sizeWord);
      actr.clearExpWindow(window);
      actr.addTextToExpWindow(window, featureText(words, slots), {
        x: 70,
        y: 130,
      });
      await ctx.paint();

      const prompt = createChoicePrompt(actr, ctx, {
        prompt: `Test item ${index + 1}/${STIMS.length}: which category?`,
        choices: [
          { value: 1, label: "Category 1", key: "1" },
          { value: 2, label: "Category 2", key: "2" },
        ],
      });
      let answer;
      try {
        answer = await prompt.wait();
      } finally {
        prompt.remove();
      }
      if (answer == null) break;
      results.push(Number(answer.value));
      ctx.progress?.(`running... ${index + 1}/${STIMS.length}`);
    }
  } finally {
    actr.closeExpWindow(window);
  }

  const answered = results.length;
  const total = STIMS.length;
  const proportions = results.map((category) => (category === 1 ? 1 : 0));
  const lines = [
    "Mode: human",
    `Test items: ${total}`,
    `Category 1 choices: ${proportions.filter(Boolean).length}/${answered}`,
  ];
  if (answered < total) lines.push(`Answered: ${answered}/${total}`);
  if (answered > 0) {
    const hasVariation = new Set(proportions).size > 1;
    if (hasVariation) {
      lines.push(
        `CORRELATION: ${Number(actr.correlation(proportions, CAT_DATA.slice(0, answered), false)).toFixed(3)}`,
        `MEAN DEVIATION: ${Number(actr.meanDeviation(proportions, CAT_DATA.slice(0, answered), false)).toFixed(3)}`
      );
    } else {
      lines.push("CORRELATION: n/a", "MEAN DEVIATION: n/a");
    }
  }
  lines.push(
    "",
    "P(C=1)",
    "data  " + CAT_DATA.map((value) => value.toFixed(3).padStart(7)).join(""),
    "human " + proportions.map((value) => value.toFixed(3).padStart(7)).join("")
  );

  return {
    kind: "categorize",
    mode: "human",
    answered,
    total,
    report: lines.join("\n"),
  };
}

function exampleForm(name, category, values, slots) {
  return (
    `      (list '${name} 'isa 'example 'category ${category}` +
    values.map((value, index) => ` '${slots[index]} '${value}`).join("") +
    ")"
  );
}

function buildPrelude(slots) {
  const examples = [
    ...CAT1.map((values, index) => exampleForm(`example${index}`, 1, values, slots)),
    ...CAT2.map((values, index) =>
      exampleForm(`example${index + CAT1.length}`, 2, values, slots)
    ),
  ];
  return `
(defun create-example-memories ()
${slots.map((slot) => `  (extend-possible-slots '${slot})`).join("\n")}
${slots.map((slot) => `  (define-chunks (${slot} isa chunk))`).join("\n")}
  (add-dm-fct
    (list
${examples.join("\n")})))
`;
}

function sizeSimilarities(a, b, offset) {
  if (typeof b !== "number") return null;
  const name = String(a ?? "").toLowerCase();
  const anchor =
    name === "small" ? -0.9 : name === "medium" ? 0 : name === "large" ? 0.9 : null;
  if (anchor == null) return null;
  const spread = b - offset - anchor;
  return Math.exp(-(spread * spread) / 0.3) - 1;
}

async function attribute(actr, ctx, slot, value, offset) {
  actr.goalFocus(["state", "add-attribute", "name", slot, "value", value + offset]);
  await actr.run(20, { paint: false, signal: ctx.signal });
}

async function trial(actr, ctx, features, slots, offset) {
  actr.reset();
  const pairs = actr.permuteList(slots.map((slot, index) => [slot, features[index]]));
  for (const [slot, value] of pairs) {
    await attribute(actr, ctx, slot, value, offset);
  }
  actr.goalFocus(["state", "categorize"]);
  await actr.run(20, { paint: false, signal: ctx.signal });
  const answer = Number(actr.bufferSlotValue("imaginal", "category"));
  if (Number.isFinite(answer) && (answer === 1 || answer === 2)) {
    return { category: answer, answered: true };
  }
  return { category: 0, answered: false };
}

async function categorize(actr, ctx) {
  if (isHuman(ctx.controls)) return humanCategorize(actr, ctx);

  const runs = Math.max(1, Number(ctx.controls.runs) || 5);
  const { slots, warning } = normalizeSlots(ctx.controls);
  const offset = normalizeOffset(ctx.controls);
  actr.addCommand("size-similarities", (a, b) => sizeSimilarities(a, b, offset));
  try {
    const results = Array(STIMS.length).fill(0);
    const counts = Array(STIMS.length).fill(0);

    for (let run = 0; run < runs; run += 1) {
      for (let i = 0; i < STIMS.length; i += 1) {
        const { category, answered } = await trial(
          actr,
          ctx,
          STIMS[i],
          slots,
          offset
        );
        if (category === 1) results[i] += 1;
        if (answered) counts[i] += 1;
      }
      ctx.progress?.(`running... ${run + 1}/${runs}`);
    }

    const proportions = results.map((value) => value / runs);
    const answered = counts.reduce((sum, value) => sum + value, 0);
    const total = STIMS.length * runs;
    const report = [
      `Mode: experiment (${runs} run${runs === 1 ? "" : "s"})`,
      `Offset: ${offset}`,
      `Slots: ${slots.join(", ")}`,
    ];
    if (warning) report.push(`Warning: ${warning}`);
    report.push(`Answered: ${answered}/${total}`);
    if (answered > 0) {
      report.push(
        `CORRELATION: ${Number(actr.correlation(proportions, CAT_DATA, false)).toFixed(3)}`,
        `MEAN DEVIATION: ${Number(actr.meanDeviation(proportions, CAT_DATA, false)).toFixed(3)}`
      );
    } else {
      report.push(
        "CORRELATION: n/a",
        "MEAN DEVIATION: n/a",
        "The canonical model is a starter with no productions, so it produces no category responses."
      );
    }
    report.push(
      "",
      "P(C=1)",
      "        " + counts.map((count) => `(${String(count).padStart(4)}) `).join(""),
      "data  " + CAT_DATA.map((value) => value.toFixed(3).padStart(7)).join(""),
      "model " + proportions.map((value) => value.toFixed(3).padStart(7)).join("")
    );
    return {
      kind: "categorize",
      mode: "model",
      answered,
      total,
      report: report.join("\n"),
    };
  } finally {
    actr.removeCommand("size-similarities");
  }
}

export const categorizeTask = {
  run: categorize,
  prelude: (controls) => buildPrelude(normalizeSlots(controls).slots),
  controls: [
    modeControl("model"),
    {
      id: "runs",
      label: "Runs",
      type: "number",
      min: 1,
      max: 20,
      step: 1,
      default: 5,
      showWhen: (values) => !isHuman(values),
    },
    {
      id: "offset",
      label: "Offset",
      type: "number",
      min: -2,
      max: 2,
      step: 0.1,
      default: 0,
      showWhen: (values) => !isHuman(values),
    },
    { id: "slot1", label: "Slot 1", type: "text", default: "eh", showWhen: (values) => !isHuman(values) },
    { id: "slot2", label: "Slot 2", type: "text", default: "es", showWhen: (values) => !isHuman(values) },
    { id: "slot3", label: "Slot 3", type: "text", default: "nl", showWhen: (values) => !isHuman(values) },
    { id: "slot4", label: "Slot 4", type: "text", default: "mh", showWhen: (values) => !isHuman(values) },
    {
      id: "studySeconds",
      label: "Study time (s)",
      type: "number",
      min: 1,
      max: 30,
      step: 1,
      default: 10,
      showWhen: (values) => isHuman(values),
    },
  ],
  runSeconds: 20,
  runTimeMode: "unused",
};

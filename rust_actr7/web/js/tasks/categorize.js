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

const SLOTS = ["eh", "es", "nl", "mh"];

function exampleForm(name, category, values) {
  return (
    `      (list '${name} 'isa 'example 'category ${category}` +
    values.map((value, i) => ` '${SLOTS[i]} '${value}`).join("") +
    ")"
  );
}

const EXAMPLES = [
  ...CAT1.map((values, i) => exampleForm(`example${i}`, 1, values)),
  ...CAT2.map((values, i) => exampleForm(`example${i + CAT1.length}`, 2, values)),
];

const PRELUDE = `
(defun create-example-memories ()
  (extend-possible-slots 'eh)
  (extend-possible-slots 'es)
  (extend-possible-slots 'nl)
  (extend-possible-slots 'mh)
  (define-chunks (eh isa chunk))
  (define-chunks (es isa chunk))
  (define-chunks (nl isa chunk))
  (define-chunks (mh isa chunk))
  (add-dm-fct
    (list
${EXAMPLES.join("\n")})))
`;

function sizeSimilarities(a, b) {
  if (typeof b !== "number") return null;
  const name = String(a ?? "").toLowerCase();
  const anchor =
    name === "small" ? -0.9 : name === "medium" ? 0 : name === "large" ? 0.9 : null;
  if (anchor == null) return null;
  const spread = b - anchor;
  return Math.exp(-(spread * spread) / 0.3) - 1;
}

async function attribute(actr, ctx, slot, value) {
  actr.goalFocus(["state", "add-attribute", "name", slot, "value", value]);
  await actr.run(20, { paint: false, signal: ctx.signal });
}

async function trial(actr, ctx, features) {
  actr.reset();
  const pairs = actr.permuteList(SLOTS.map((slot, index) => [slot, features[index]]));
  for (const [slot, value] of pairs) {
    await attribute(actr, ctx, slot, value);
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
  const runs = Math.max(1, Number(ctx.controls.runs) || 5);
  actr.addCommand("size-similarities", sizeSimilarities);
  try {
    const results = Array(STIMS.length).fill(0);
    const counts = Array(STIMS.length).fill(0);

    for (let run = 0; run < runs; run += 1) {
      for (let i = 0; i < STIMS.length; i += 1) {
        const { category, answered } = await trial(actr, ctx, STIMS[i]);
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
      `Answered: ${answered}/${total}`,
    ];
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
  prelude: PRELUDE,
  controls: [
    {
      id: "runs",
      label: "Runs",
      type: "number",
      min: 1,
      max: 20,
      step: 1,
      default: 5,
    },
  ],
  runSeconds: 20,
  runTimeMode: "unused",
};

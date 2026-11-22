const SIEGLER_DATA = [
  [0, 0.05, 0.86, 0, 0.02, 0, 0.02, 0, 0, 0.06],
  [0, 0.04, 0.07, 0.75, 0.04, 0, 0.02, 0, 0, 0.09],
  [0, 0.02, 0, 0.10, 0.75, 0.05, 0.01, 0.03, 0, 0.06],
  [0.02, 0, 0.04, 0.05, 0.80, 0.04, 0, 0.05, 0, 0],
  [0, 0, 0.07, 0.09, 0.25, 0.45, 0.08, 0.01, 0.01, 0.06],
  [0.04, 0, 0, 0.05, 0.21, 0.09, 0.48, 0, 0.02, 0.11],
];

const PROBLEMS = [
  [1, 1],
  [1, 2],
  [1, 3],
  [2, 2],
  [2, 3],
  [3, 3],
];

const POSITIONS = {
  zero: 0,
  one: 1,
  two: 2,
  three: 3,
  four: 4,
  five: 5,
  six: 6,
  seven: 7,
  eight: 8,
};

async function trial(actr, ctx, arg1, arg2, response) {
  actr.reset();
  actr.installDevice(["speech", "microphone"]);
  actr.newDigitSound(arg1);
  actr.newDigitSound(arg2, 0.75);
  response.value = null;
  await actr.run(ctx.runSeconds ?? 30, { signal: ctx.signal });
  return response.value;
}

async function siegler(actr, ctx) {
  const sets = Math.max(1, Number(ctx.controls.sets) || 1);
  const response = { value: null };
  actr.addCommand("siegler-response", (_model, value) => {
    response.value = String(value ?? "").toLowerCase();
  });
  actr.monitorCommand("output-speech", "siegler-response");

  const responses = [];
  try {
    for (let i = 0; i < sets; i += 1) {
      const set = [];
      for (const [arg1, arg2] of PROBLEMS) {
        set.push(await trial(actr, ctx, arg1, arg2, response));
      }
      responses.push(set);
    }
  } finally {
    actr.removeCommandMonitor("output-speech", "siegler-response");
    actr.removeCommand("siegler-response");
  }

  const results = Array.from({ length: PROBLEMS.length }, () => Array(10).fill(0));
  for (const set of responses) {
    for (let i = 0; i < PROBLEMS.length; i += 1) {
      const value = set[i];
      const position = POSITIONS[value];
      if (position == null) results[i][9] += 1;
      else results[i][position] += 1;
    }
  }
  for (const row of results) {
    for (let j = 0; j < row.length; j += 1) row[j] /= responses.length;
  }

  const flat = (rows) => rows.flat();
  const correlation = actr.correlation(flat(results), flat(SIEGLER_DATA), false);
  const meanDeviation = actr.meanDeviation(flat(results), flat(SIEGLER_DATA), false);
  const lines = ["       0     1     2     3     4     5     6     7     8   Other"];
  for (let i = 0; i < PROBLEMS.length; i += 1) {
    const [a, b] = PROBLEMS[i];
    lines.push(
      `${a}+${b} ${results[i].map((value) => value.toFixed(2).padStart(4)).join("  ")}`
    );
  }
  lines.push(
    "",
    `CORRELATION: ${Number(correlation).toFixed(3)}`,
    `MEAN DEVIATION: ${Number(meanDeviation).toFixed(3)}`
  );

  return {
    kind: "siegler",
    results,
    correlation,
    meanDeviation,
    report: lines.join("\n"),
  };
}

export const sieglerTask = {
  run: siegler,
  controls: [
    {
      id: "sets",
      label: "Sets",
      type: "number",
      min: 1,
      max: 50,
      step: 1,
      default: 10,
    },
  ],
  runSeconds: 30,
  runTimeMode: "per-trial",
};

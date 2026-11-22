const EXPECTED = ["1", "2", "3", "4", "6", "5", "7", "8"];

async function grouped(actr, ctx) {
  const responses = [];
  actr.addCommand("grouped-response", (item) => {
    responses.push(String(item));
  });

  try {
    actr.reset();
    await actr.run(ctx.runSeconds ?? 20);
  } finally {
    actr.removeCommand("grouped-response");
  }

  const matches = responses.length === EXPECTED.length
    && responses.every((item, i) => item === EXPECTED[i]);
  return {
    kind: "grouped",
    responses,
    report: [
      `RECALLED: ${responses.join(" ") || "none"}`,
      `EXPECTED: ${EXPECTED.join(" ")}`,
      `MATCH: ${matches ? "yes" : "no"}`,
    ].join("\n"),
  };
}

export const groupedTask = {
  run: grouped,
  controls: [],
  runSeconds: 20,
};

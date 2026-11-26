import { humanShuffle, isHuman, modeControl } from "./human.js";
import { createTextPrompt } from "./human-controls.js";

const VERBS = [
  ["have", "i", 12458, "had"],
  ["do", "i", 4367, "did"],
  ["make", "i", 2312, "made"],
  ["get", "i", 1486, "got"],
  ["use", "r", 1016, "use"],
  ["look", "r", 910, "look"],
  ["seem", "r", 831, "seem"],
  ["tell", "i", 759, "told"],
  ["show", "r", 640, "show"],
  ["want", "r", 631, "want"],
  ["call", "r", 627, "call"],
  ["ask", "r", 612, "ask"],
  ["turn", "r", 566, "turn"],
  ["follow", "r", 540, "follow"],
  ["work", "r", 496, "work"],
  ["live", "r", 472, "live"],
  ["try", "r", 472, "try"],
  ["stand", "i", 468, "stood"],
  ["move", "r", 447, "move"],
  ["need", "r", 413, "need"],
  ["start", "r", 386, "start"],
  ["lose", "i", 274, "lost"],
];

function isIrregular(entry) {
  return String(entry[1]).toLowerCase() === "i";
}

function regularPast(verb) {
  const value = String(verb).toLowerCase();
  if (value.endsWith("e")) return `${value}d`;
  if (/[^aeiou]y$/.test(value)) return `${value.slice(0, -1)}ied`;
  return `${value}ed`;
}

function expectedPast(entry) {
  return isIrregular(entry) ? String(entry[3]).toLowerCase() : regularPast(entry[0]);
}

function makeWordFreqList() {
  let count = 0;
  return VERBS.map(([verb, kind, frequency, past]) => {
    count += frequency;
    return [count, verb, past, kind === "i" ? "blank" : "ed"];
  });
}

function safeDiv(numerator, denominator) {
  return denominator === 0 ? 0 : numerator / denominator;
}

function lower(value) {
  return value == null ? null : String(value).toLowerCase();
}

function addToReport(actr, target, report, warnings) {
  const word = lower(actr.bufferSlotValue("imaginal", "verb"));
  const stem = lower(actr.bufferSlotValue("imaginal", "stem"));
  const suffix = lower(actr.bufferSlotValue("imaginal", "suffix"));
  const irregular = target[2].toLowerCase() === "blank";

  if (word !== target[0].toLowerCase()) {
    warnings.push(
      `Incorrectly formed verb. Presented ${target[0]} and produced ${word} (stem ${stem}, suffix ${suffix}).`
    );
    report.push([irregular, "error"]);
  } else if (stem === word && suffix === "ed") {
    report.push([irregular, "reg"]);
  } else if (stem == null && suffix == null) {
    report.push([irregular, "none"]);
  } else if (stem === target[1].toLowerCase() && suffix === "blank") {
    report.push([irregular, "irreg"]);
  } else {
    warnings.push(
      `Incorrectly formed verb. Presented ${target[0]} and produced ${word} (stem ${stem}, suffix ${suffix}).`
    );
    report.push([irregular, "error"]);
  }
}

function blockSummary(report, start, end) {
  let irregular = 0;
  let regular = 0;
  let none = 0;
  let error = 0;
  for (const [isIrregular, kind] of report.slice(start, end)) {
    if (kind === "irreg") irregular += 1;
    else if (kind === "reg") regular += 1;
    else if (kind === "none") none += 1;
    else error += 1;
  }
  const total = irregular + regular + none + error;
  return {
    end,
    irregular: safeDiv(irregular, total),
    regular: safeDiv(regular, total),
    none: safeDiv(none, total),
    correct: safeDiv(irregular, irregular + regular),
  };
}

function reportTable(rows) {
  const lines = [
    "trials      Irregular       Regular    No inflection  Inflected correctly",
  ];
  for (const row of rows) {
    lines.push(
      `${String(row.end).padStart(6)} ${row.irregular.toFixed(3).padStart(13)} ` +
      `${row.regular.toFixed(3).padStart(13)} ${row.none.toFixed(3).padStart(13)} ` +
      `${row.correct.toFixed(3).padStart(13)}`
    );
  }
  return lines;
}

function renderChart(stage, rows, blockSize) {
  if (!stage) return;
  stage.hidden = false;
  stage.replaceChildren();
  stage.classList.add("past-tense-stage");

  const heading = document.createElement("div");
  heading.className = "past-tense-heading";
  heading.textContent = "Correct irregular inflections by block";
  stage.append(heading);

  const width = 520;
  const height = 240;
  const left = 42;
  const right = 18;
  const top = 14;
  const bottom = 34;
  const plotWidth = width - left - right;
  const plotHeight = height - top - bottom;
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("viewBox", `0 0 ${width} ${height}`);
  svg.setAttribute("role", "img");
  svg.setAttribute("aria-label", "Correct irregular past-tense inflections by block");
  svg.classList.add("past-tense-chart");

  const x = (index) => left + (rows.length <= 1 ? plotWidth / 2 : (index / (rows.length - 1)) * plotWidth);
  const y = (value) => top + (1 - Math.max(0, Math.min(1, value))) * plotHeight;

  for (let tick = 0; tick <= 4; tick += 1) {
    const value = tick / 4;
    const yy = y(value);
    const line = document.createElementNS("http://www.w3.org/2000/svg", "line");
    line.setAttribute("x1", String(left));
    line.setAttribute("x2", String(width - right));
    line.setAttribute("y1", String(yy));
    line.setAttribute("y2", String(yy));
    line.setAttribute("class", "past-tense-grid");
    svg.append(line);

    const label = document.createElementNS("http://www.w3.org/2000/svg", "text");
    label.setAttribute("x", String(left - 7));
    label.setAttribute("y", String(yy + 4));
    label.setAttribute("text-anchor", "end");
    label.setAttribute("class", "past-tense-axis");
    label.textContent = value.toFixed(2);
    svg.append(label);
  }

  const axis = document.createElementNS("http://www.w3.org/2000/svg", "line");
  axis.setAttribute("x1", String(left));
  axis.setAttribute("x2", String(left));
  axis.setAttribute("y1", String(top));
  axis.setAttribute("y2", String(height - bottom));
  axis.setAttribute("class", "past-tense-axis-line");
  svg.append(axis);

  const points = rows.map((row, index) => `${x(index)},${y(row.correct)}`).join(" ");
  if (rows.length > 1) {
    const path = document.createElementNS("http://www.w3.org/2000/svg", "polyline");
    path.setAttribute("points", points);
    path.setAttribute("class", "past-tense-line");
    svg.append(path);
  }
  rows.forEach((row, index) => {
    const dot = document.createElementNS("http://www.w3.org/2000/svg", "circle");
    dot.setAttribute("cx", String(x(index)));
    dot.setAttribute("cy", String(y(row.correct)));
    dot.setAttribute("r", "4");
    dot.setAttribute("class", "past-tense-dot");
    const title = document.createElementNS("http://www.w3.org/2000/svg", "title");
    title.textContent = `Block ending at ${row.end}: ${row.correct.toFixed(3)}`;
    dot.append(title);
    svg.append(dot);

    const label = document.createElementNS("http://www.w3.org/2000/svg", "text");
    label.setAttribute("x", String(x(index)));
    label.setAttribute("y", String(height - 12));
    label.setAttribute("text-anchor", "middle");
    label.setAttribute("class", "past-tense-axis");
    label.textContent = String(row.end);
    svg.append(label);
  });

  stage.append(svg);
}

async function humanPastTense(actr, ctx) {
  const limit = Math.max(
    1,
    Math.min(VERBS.length, Number(ctx.controls.humanTrials) || VERBS.length)
  );
  const selected = humanShuffle(VERBS).slice(0, limit);
  actr.reset();
  const window = actr.openExpWindow("Past Tense", {
    visible: true,
    width: 400,
    height: 300,
  });
  const records = [];

  try {
    for (let index = 0; index < selected.length; index += 1) {
      const entry = selected[index];
      const verb = String(entry[0]).toLowerCase();
      ctx.progress?.(`running... ${index + 1}/${limit}`);
      actr.clearExpWindow(window);
      actr.addTextToExpWindow(window, `Verb: ${verb}`, { x: 140, y: 145 });
      await ctx.paint();

      const prompt = createTextPrompt(actr, ctx, {
        prompt: `Verb ${index + 1}/${limit}: type the past tense of "${verb}".`,
        placeholder: "past tense",
        maxLength: 30,
        timeoutMs: 30000,
        sanitize: (value) => value.replace(/[^a-z'-]/gi, "").slice(0, 30),
      });
      let answer;
      try {
        answer = await prompt.wait();
      } finally {
        prompt.remove();
      }
      if (answer == null) break;

      const response = answer.text.toLowerCase();
      const expected = expectedPast(entry);
      records.push({
        verb,
        response,
        expected,
        correct: response === expected,
        irregular: isIrregular(entry),
      });
      ctx.progress?.(`running... ${records.length}/${limit}`);
    }
  } finally {
    actr.closeExpWindow(window);
  }

  const total = records.length;
  const correct = records.filter((record) => record.correct).length;
  const regular = records.filter((record) => !record.irregular);
  const irregular = records.filter((record) => record.irregular);
  const regularCorrect = regular.filter((record) => record.correct).length;
  const irregularCorrect = irregular.filter((record) => record.correct).length;
  const errors = records.filter((record) => !record.correct);
  const lines = [
    "Mode: human",
    `Verbs: ${total}`,
    `CORRECT: ${correct}/${total}`,
    `Regular: ${regularCorrect}/${regular.length}`,
    `Irregular: ${irregularCorrect}/${irregular.length}`,
  ];
  if (errors.length) {
    lines.push("", "Errors:");
    for (const record of errors.slice(0, 10)) {
      lines.push(
        `  ${record.verb} -> ${record.response || "none"} (expected ${record.expected})`
      );
    }
    if (errors.length > 10) lines.push(`  ... ${errors.length - 10} more`);
  }

  return {
    kind: "past-tense",
    mode: "human",
    correct,
    total,
    report: lines.join("\n"),
  };
}

async function pastTense(actr, ctx) {
  if (isHuman(ctx.controls)) return humanPastTense(actr, ctx);

  const trials = Math.max(1, Math.min(10000, Number(ctx.controls.trials) || 2000));
  const blockSize = Math.max(1, Math.min(trials, Number(ctx.controls.blockSize) || 1000));
  const wordList = makeWordFreqList();
  const totalCount = wordList[wordList.length - 1][0];
  const unique = new Set();
  for (const entry of wordList) {
    for (const value of entry.slice(1)) unique.add(value);
  }
  actr.reset();
  actr.setParameterValue(":v", false);
  actr.setParameterValue(":pct", false);
  actr.defineChunks(...unique);

  const randomWord = () => {
    const value = Number(actr.random(totalCount));
    const entry = wordList.find((item) => item[0] > value) || wordList[wordList.length - 1];
    return entry.slice(1);
  };

  const addPastTenseToMemory = () => {
    const word = randomWord();
    actr.setBufferChunk("imaginal", ["verb", word[0], "stem", word[1], "suffix", word[2]]);
    actr.clearBuffer("imaginal");
  };

  const report = [];
  const warnings = [];
  let rewardSeen = false;
  let missingRewards = 0;
  let timeouts = 0;

  actr.addCommand("past-tense-reward", () => {
    rewardSeen = true;
  });
  actr.monitorCommand("trigger-reward", "past-tense-reward");

  try {
    for (let trial = 0; trial < trials; trial += 1) {
      addPastTenseToMemory();
      addPastTenseToMemory();
      const target = randomWord();
      actr.setBufferChunk("imaginal", ["verb", target[0]]);
      actr.goalFocus(["isa", "goal", "state", "start"]);

      rewardSeen = false;
      const start = Number(actr.getTime(true));
      await actr.run(100, { maxEvents: 256, signal: ctx.signal });
      const duration = (Number(actr.getTime(true)) - start) / 1000;
      addToReport(actr, target, report, warnings);
      actr.clearBuffer("imaginal");

      if (!rewardSeen) missingRewards += 1;
      if (duration >= 100) timeouts += 1;
      await actr.runFullTime(Math.max(0, 200 - duration), {
        maxEvents: 256,
        signal: ctx.signal,
      });

      if ((trial + 1) % 100 === 0) {
        ctx.progress?.(`running... ${trial + 1}/${trials}`);
      }
    }
  } finally {
    actr.removeCommandMonitor("trigger-reward", "past-tense-reward");
    actr.removeCommand("past-tense-reward");
  }

  const rows = [];
  for (let start = 0; start < report.length; start += blockSize) {
    rows.push(blockSummary(report, start, Math.min(report.length, start + blockSize)));
  }
  renderChart(ctx.stage, rows, blockSize);

  const lines = [
    ...reportTable(rows),
    "",
    `Missing rewards: ${missingRewards} / ${trials}`,
    `Trials reaching the 100 s limit: ${timeouts}`,
    "",
    "Canonical starter model: no verb-generation productions.",
    "U-shaped learning requires the Unit 7 assignment solution.",
  ];
  if (warnings.length) {
    lines.push("", `Malformed responses: ${warnings.length}`);
    lines.push(...warnings.slice(0, 5));
    if (warnings.length > 5) lines.push(`... ${warnings.length - 5} more`);
  }

  return { kind: "past-tense", mode: "model", report: lines.join("\n") };
}

export const pastTenseTask = {
  run: pastTense,
  controls: [
    modeControl("model"),
    {
      id: "trials",
      label: "Trials",
      type: "number",
      min: 100,
      max: 10000,
      step: 100,
      default: 2000,
      showWhen: (values) => !isHuman(values),
    },
    {
      id: "blockSize",
      label: "Block size",
      type: "number",
      min: 100,
      max: 2000,
      step: 100,
      default: 1000,
      showWhen: (values) => !isHuman(values),
    },
    {
      id: "humanTrials",
      label: "Human verbs",
      type: "number",
      min: 1,
      max: VERBS.length,
      step: 1,
      default: VERBS.length,
      showWhen: (values) => isHuman(values),
    },
  ],
  runSeconds: 200,
  runTimeMode: "unused",
};

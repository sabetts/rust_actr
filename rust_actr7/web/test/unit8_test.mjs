// Smoke test for the Unit 8 web tasks.  Requires
// `python rust_actr7/web/build.py` first and Node 18+.
//
//   node rust_actr7/web/test/unit8_test.mjs

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const testDir = path.dirname(fileURLToPath(import.meta.url));
const webDir = path.resolve(testDir, "..");
const root = path.resolve(testDir, "../../..");
const wasmPath = path.join(webDir, "actr_engine.wasm");
const actrPath = path.join(webDir, "js", "actr.js");
const tasksPath = path.join(webDir, "js", "tasks", "index.js");

if (!fs.existsSync(wasmPath)) {
  throw new Error(
    `missing ${wasmPath}; run: python rust_actr7/web/build.py`
  );
}

globalThis.requestAnimationFrame = (callback) =>
  setTimeout(() => callback(performance.now()), 0);
globalThis.fetch = async () => ({
  ok: true,
  arrayBuffer: async () => fs.readFileSync(wasmPath),
});

const { loadWasm, Actr } = await import(pathToFileURL(actrPath).href);
const { TASKS } = await import(pathToFileURL(tasksPath).href);
const wasm = await loadWasm();

function fail(message) {
  console.error(message);
  process.exit(1);
}

function controlsFor(descriptor, overrides) {
  const values = {};
  for (const control of descriptor.controls || []) {
    values[control.id] = overrides[control.id] ?? control.default;
  }
  return values;
}

function sourceFor(relative) {
  return fs.readFileSync(path.join(root, "actr7.x/tutorial", relative), "utf8");
}

async function runTask(taskId, modelFile, overrides = {}) {
  const descriptor = TASKS[taskId];
  if (!descriptor) fail(`missing task ${taskId}`);
  const source = sourceFor(modelFile);
  const actr = new Actr(wasm, 100, 0, null);
  const controls = controlsFor(descriptor, overrides);
  const prelude =
    typeof descriptor.prelude === "function"
      ? descriptor.prelude(controls)
      : descriptor.prelude;
  actr.loadSource(prelude ? `${prelude}\n${source}` : source);
  const result = await descriptor.run(actr, {
    secs: 60,
    runSeconds: 60,
    controls,
    paint: async () => {},
    stage: null,
    signal: null,
    progress: () => {},
  });
  return { actr, result };
}

const bst = await runTask("bst-ppm", "unit8/bst-ppm-model.lisp", {
  mode: "report",
  runs: 1,
});
if (!bst.result.report.includes("DECIDE-OVER")) {
  fail(`bst-ppm report missing utilities:\n${bst.result.report}`);
}

const bstDemo = await runTask("bst-ppm", "unit8/bst-ppm-model.lisp", {
  mode: "demo",
  problem: "1",
});
if (!bstDemo.result.report.includes("Solved: yes")) {
  fail(`bst-ppm demo did not solve:\n${bstDemo.result.report}`);
}

const bst6 = await runTask("bst", "unit6/bst-model.lisp", {
  mode: "report",
  runs: 1,
});
if (!bst6.result.report.includes("FORCE-OVER")) {
  fail(`unit6 bst report missing utilities:\n${bst6.result.report}`);
}

const cat = await runTask("categorize", "unit8/categorize-model.lisp", {
  runs: 1,
});
const names = cat.actr.eval("dm")[0] || [];
const examples = names.filter((name) =>
  String(name).toUpperCase().startsWith("EXAMPLE")
);
if (examples.length !== 10) {
  fail(`categorize created ${examples.length} example chunks`);
}
if (cat.result.answered !== 0) {
  fail(`categorize starter answered ${cat.result.answered}`);
}
if (!cat.result.report.includes("starter")) {
  fail(`categorize report missing starter note:\n${cat.result.report}`);
}

const catCustom = await runTask("categorize", "unit8/categorize-model.lisp", {
  runs: 1,
  offset: 0.5,
  slot1: "size",
  slot2: "shape",
  slot3: "texture",
  slot4: "color",
});
if (!catCustom.result.report.includes("Offset: 0.5")) {
  fail(`categorize report missing offset:\n${catCustom.result.report}`);
}
if (!catCustom.result.report.includes("Slots: size, shape, texture, color")) {
  fail(`categorize report missing custom slots:\n${catCustom.result.report}`);
}
if (catCustom.actr.chunkSlotValue("example0", "size") == null) {
  fail("categorize custom-slot examples missing size");
}
if (catCustom.actr.chunkSlotValue("example0", "eh") != null) {
  fail("categorize custom-slot examples still have eh");
}

const catFallback = await runTask("categorize", "unit8/categorize-model.lisp", {
  runs: 1,
  slot1: "dup",
  slot2: "dup",
  slot3: "x",
  slot4: "y",
});
if (!catFallback.result.report.includes("Warning:")) {
  fail(`categorize invalid slots did not warn:\n${catFallback.result.report}`);
}

const semantic = await runTask("semantic-dynamic", "unit8/semantic-dynamic-model.lisp", {
  question: "all",
});
if (!semantic.result.report.includes("ANSWER IS YES")) {
  fail(`semantic report missing answers:\n${semantic.result.report}`);
}

const paired = await runTask("paired", "unit8/paired-dynamic-model.lisp", {
  mode: "model",
  pairs: 2,
  blocks: 2,
  runs: 1,
  trace: false,
});
if (paired.result.kind !== "paired") {
  fail("paired-dynamic did not return a paired result");
}

for (const [file, seconds] of [
  ["unit8/simple-ppm-model.lisp", 0.5],
  ["unit8/simple-dynamic-model.lisp", 5.0],
]) {
  const actr = new Actr(wasm, 100, 0, null);
  actr.loadSource(sourceFor(file));
  await actr.run(seconds, { paint: false });
  if (!actr.trace.join("\n").includes("Stopped because")) {
    fail(`${file} did not produce a trace`);
  }
}

console.log("unit8 ok");

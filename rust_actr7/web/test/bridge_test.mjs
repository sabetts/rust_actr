// Smoke test for the wasm/JS embedder bridge (sim-hook, ppm-hook, host
// actions).  Requires `python rust_actr7/web/build.py` first and Node 18+.
//
//   node rust_actr7/web/test/bridge_test.mjs

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const testDir = path.dirname(fileURLToPath(import.meta.url));
const webDir = path.resolve(testDir, "..");
const wasmPath = path.join(webDir, "actr_engine.wasm");
const actrPath = path.join(webDir, "js", "actr.js");
const ppmModelPath = path.resolve(
  testDir,
  "../../fixtures/models/test-models/models/test-ppm.lisp"
);

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
const wasm = await loadWasm();

function fail(message) {
  console.error(message);
  process.exit(1);
}

// 1. JS :sim-hook fires and a null return falls through.
let simCalls = 0;
const sim = new Actr(wasm, 100, 0, null);
sim.loadSource(`
(clear-all)
(define-model bridge-sim
  (sgp :v nil :esc t :sim-hook "js-sims" :md -1 :mp 1 :ppm 1)
  (chunk-type goal state)
  (chunk-type test value)
  (add-dm (t1 isa test value 1) (t2 isa test value 2))
  (goal-focus (isa goal state start))
  (p go
     =goal> isa goal state start
     ==>
     +retrieval> isa test value 3
     =goal> state done))
`);
sim.addCommand("js-sims", (a, b) => {
  simCalls += 1;
  return typeof a === "number" && typeof b === "number" ? null : 0.0;
});
await sim.run(1.0, { paint: false });
if (simCalls === 0) fail("JS sim hook was not called");

// 2. JS :ppm-hook receives the production and partial-match rows.
let ppmCalls = 0;
const ppm = new Actr(wasm, 100, 0, null);
ppm.loadSource(fs.readFileSync(ppmModelPath, "utf8"));
ppm.reset();
ppm.setParameterValue(":PPM", 1);
ppm.setParameterValue(":PPM-HOOK", "js-ppm");
ppm.addCommand("js-ppm", (production, rows) => {
  ppmCalls += 1;
  if (typeof production !== "string") fail("ppm production is not a string");
  if (!Array.isArray(rows) || rows.length === 0) fail("ppm rows are empty");
  if (!Array.isArray(rows[0]) || rows[0].length !== 5) {
    fail(`unexpected ppm row shape: ${JSON.stringify(rows[0])}`);
  }
  return -0.42;
});
await ppm.run(10.0, { paint: false });
if (ppmCalls === 0) fail("JS ppm hook was not called");

// 3. Imaginal-action host request runs a JS command and reaches the model.
let hostCalls = 0;
const host = new Actr(wasm, 100, 0, null);
host.loadSource(`
(clear-all)
(define-model bridge-host
  (sgp :v t :esc t)
  (chunk-type goal state)
  (chunk-type result data)
  (goal-focus (isa goal state start))
  (p start
     =goal> isa goal state start
     ?imaginal> state free buffer empty
     ==>
     =goal> state wait
     +imaginal-action> action "js-diff" simple t)
  (p done
     =goal> isa goal state wait
     =imaginal> isa result data 42
     ==>
     !output! GOT
     =goal> state done))
`);
host.addCommand("js-diff", () => {
  hostCalls += 1;
  return ["isa", "result", "data", 42];
});
await host.run(2.0, { paint: false });
if (hostCalls === 0) fail("JS host action was not called");
if (!host.trace.join("\n").includes("GOT")) {
  fail("host action result did not reach the model");
}

console.log(`bridge ok (sim=${simCalls} ppm=${ppmCalls} host=${hostCalls})`);

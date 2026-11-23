import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const testDir = path.dirname(fileURLToPath(import.meta.url));
const scanPath = path.resolve(testDir, "..", "js", "lisp-scan.js");
const { scanParens, locateErrorAnchor } = await import(pathToFileURL(scanPath).href);

function problems(text) {
  return scanParens(text).problems.map((problem) => `${problem.char}@${problem.line}`);
}

assert.deepEqual(problems("(a (b) c)"), []);
assert.deepEqual(problems("(a))"), [")@1"]);
assert.deepEqual(problems("(a"), ["(@1"]);
assert.deepEqual(problems('(a ")" b)'), []);
assert.deepEqual(problems("(a ; )\n b)"), []);
assert.deepEqual(problems("(a #| ( |# b)"), []);
assert.deepEqual(problems("(a #| #| ) |# |# b)"), []);
assert.deepEqual(problems("(a |)| b)"), []);
assert.deepEqual(problems("(a\n(b\n"), ["(@1", "(@2"]);
assert.deepEqual(problems("(a\n)\n)"), [")@3"]);

const pairs = scanParens("(a (b) c)").pairs;
assert.equal(pairs.get(0), 8);
assert.equal(pairs.get(8), 0);

const production = locateErrorAnchor(
  "structural: rejected production START",
  "(clear-all)\n(p start\n  =goal> isa goal)\n"
);
assert.equal(production?.line, 2);

const buffer = locateErrorAnchor(
  "structural: LHS: unexpected `+retrieval>` buffer ref",
  "(p start\n  =goal>\n  +retrieval>\n)"
);
assert.equal(buffer?.line, 3);

console.log("lisp scan ok");

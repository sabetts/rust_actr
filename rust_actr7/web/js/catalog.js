export const RECOMMENDED_START = "unit1-count";

export const UNITS = {
  unit1: {
    title: "First steps",
    blurb: "Goals, chunks, and your first productions.",
  },
  unit2: {
    title: "The experiment window",
    blurb: "Drive a task through the AGI window.",
  },
  unit3: {
    title: "Perception and action",
    blurb: "See, hear, and act inside a task.",
  },
  unit4: {
    title: "Declarative memory",
    blurb: "Retrieve chunks and learn associations.",
  },
  unit5: {
    title: "Activation and noise",
    blurb: "Base-level activation, fan, and partial matching.",
  },
  unit6: {
    title: "Conflict resolution",
    blurb: "Let utilities decide which production wins.",
  },
  unit7: {
    title: "Learning",
    blurb: "Reward, utility learning, and compilation.",
  },
  unit8: {
    title: "Dynamic cognition",
    blurb: "Parameter learning and integrated tasks.",
  },
};

export const MODEL_META = {
  "unit1-count": {
    description:
      "A tiny counting model. It walks declarative number chunks from two up to four and prints each step.",
    highlights: [
      "A goal buffer drives a short retrieval chain",
      "Production firings are visible in the trace",
    ],
  },
  "unit1-addition": {
    description:
      "Adds two single-digit numbers by incrementing a count and retrieving the next number chunk.",
    highlights: [
      "Chunk types describe the goal and the number line",
      "Compare the loop with the simpler Count model",
    ],
  },
  "unit1-semantic": {
    description:
      "Answers simple category and property questions by retrieving facts from declarative memory.",
    highlights: [
      "Declarative facts are stored as chunks",
      "Pattern matching selects the answer",
    ],
  },
  "unit1-tutor-model": {
    badge: "starter",
    description:
      "Starter skeleton for the Unit 1 addition tutor. The chunk types and productions are left for you to add.",
    highlights: [
      "Intended for the tutorial exercise",
      "Run it after adding chunks and productions",
    ],
  },
  "unit1-tutor-model-solution": {
    badge: "solution",
    description:
      "Reference solution for the Unit 1 tutor exercise, using a two-digit addition goal.",
    highlights: [
      "Shows a complete worked tutorial model",
      "Contrast it with the starter skeleton",
    ],
  },
  "unit1-broken-addition": {
    badge: "broken",
    description:
      "A deliberately broken addition model. Its number chain skips a value, so the trace exposes the bug.",
    highlights: [
      "Intentionally incorrect for debugging practice",
      "Read the trace to locate the bad chunk",
    ],
  },
  "unit2-demo2": {
    badge: "demo",
    description:
      "Reads a letter from a visual array and responds, demonstrating visual attention and motor actions.",
    highlights: [
      "Uses the visual-location and visual buffers",
      "The experiment window mirrors the tutorial demo",
    ],
  },
  "unit2-unit2-assignment": {
    badge: "starter",
    description:
      "Starter for the Unit 2 assignment. Extend the model to complete the experiment task.",
    highlights: [
      "Builds on the Demo2 pattern",
      "Intended for independent practice",
    ],
  },
  "unit3-sperling": {
    description:
      "Models the Sperling partial-report task: perceive letters, hold them briefly, and report a cued row.",
    highlights: [
      "Visual attention and short-term memory",
      "Tone cues select the reported row",
    ],
  },
  "unit3-perceptual-motor-issues": {
    description:
      "Shows how the perceptual and motor modules behave when a task pushes their timing limits.",
    highlights: [
      "Perceptual-motor timing is observable",
      "Presented as in the tutorial",
    ],
  },
  "unit3-subitize": {
    badge: "starter",
    description:
      "Starter for the subitizing exercise: judge small quantities quickly without counting.",
    highlights: [
      "Combines visual attention with a simple judgment",
      "Complete the productions to solve it",
    ],
  },
  "unit4-paired": {
    description:
      "Learns and recalls paired associates, illustrating declarative retrieval and strengthening.",
    highlights: [
      "Study and recall phases are labeled",
      "Try model mode for a full run",
    ],
  },
  "unit4-zbrodoff": {
    badge: "starter",
    description:
      "Starter for the alpha-arithmetic task: decide whether letter pairs are in the right order.",
    highlights: [
      "Starter behavior is expected to fit poorly",
      "Use the report to diagnose your model",
    ],
  },
  "unit5-fan": {
    description:
      "Demonstrates the fan effect: more associations slow retrieval and reduce accuracy.",
    highlights: [
      "Compare target and foil probe times",
      "Reports correlation against tutorial data",
    ],
  },
  "unit5-fan-no-pm": {
    description:
      "The fan-effect model without partial matching, for comparing retrieval behavior.",
    highlights: [
      "Same task as Fan Effect",
      "Isolates the effect of partial matching",
    ],
  },
  "unit5-grouped": {
    description:
      "Recalls a grouped list, showing how declarative memory handles order and errors.",
    highlights: [
      "Fixed seed gives the tutorial sequence",
      "Commission and omission errors appear",
    ],
  },
  "unit5-siegler": {
    description:
      "Models children's addition strategies and compares response patterns against data.",
    highlights: [
      "Reports a response matrix and correlation",
      "Digit sounds go to the audio module",
    ],
  },
  "unit5-onehit": {
    badge: "starter",
    description:
      "Starter for the 1-hit blackjack task: learn utilities from rewards in a card game.",
    highlights: [
      "Play a hand against the model",
      "Try Hit and Stay with the h and s keys",
    ],
  },
  "unit5-declarative-issues": {
    description:
      "Explores declarative memory parameters and retrieval edge cases. There is no task panel.",
    highlights: [
      "Read the trace directly",
      "Useful after Activation and Noise",
    ],
  },
  "unit6-bst": {
    description:
      "Building Sticks: chooses between strategies using production utilities and conflict resolution.",
    highlights: [
      "Report mode runs the experiment",
      "Demo and human modes show the task",
    ],
  },
  "unit6-choice": {
    badge: "starter",
    description:
      "Starter for a choice task where utilities guide which production is selected.",
    highlights: [
      "The empty model makes no responses until you add rules",
      "Human mode accepts the h and t keys",
    ],
  },
  "unit7-paired-learning": {
    description:
      "Paired-associate learning across blocks, with retrieval strengthening over time.",
    highlights: [
      "Shares the paired-associate task",
      "Watch recall improve across blocks",
    ],
  },
  "unit7-past-tense": {
    badge: "starter",
    description:
      "Starter for the past-tense learning exercise. It produces no inflection and no learning.",
    highlights: [
      "The block table and chart diagnose the starter",
      "Add the learning rules to improve it",
    ],
  },
  "unit7-utility-learning-issues": {
    description:
      "Explores reward-driven utility learning and how production utilities change.",
    highlights: [
      "Uses its own fixed seed",
      "Watch utility values evolve",
    ],
  },
  "unit7-production-compilation-issues": {
    badge: "starter",
    description:
      "Starter for production compilation: combine two productions into one faster rule.",
    highlights: [
      "Toggle compilation with the task control",
      "The trace shows compiled productions",
    ],
  },
  "unit8-simple-ppm": {
    description:
      "A minimal partial-matching model. There is no task panel; the trace shows how similarity affects retrieval.",
    highlights: [
      "Short, focused trace",
      "Good preview of the Unit 8 ideas",
    ],
  },
  "unit8-bst-ppm": {
    description:
      "Building Sticks with partial matching, supplied through JS hooks for similarity and differences.",
    highlights: [
      "Uses :sim-hook and imaginal actions",
      "Report mode defaults to 10 runs",
    ],
  },
  "unit8-simple-dynamic": {
    description:
      "A minimal dynamic-memory example. There is no task panel; follow the trace to see memory change.",
    highlights: [
      "Memory changes during the run",
      "Compare it with Simple PPM",
    ],
  },
  "unit8-semantic-dynamic": {
    description:
      "Queries a semantic network and reports answers while memory updates dynamically.",
    highlights: [
      "Choose g1, g2, g3, or all",
      "Each answer appears in the report",
    ],
  },
  "unit8-paired-dynamic": {
    description:
      "Paired associates with production compilation enabled, showing learning as the run proceeds.",
    highlights: [
      "Builds on the Unit 4 task",
      "Compilation is on in the model",
    ],
  },
  "unit8-categorize": {
    badge: "starter",
    description:
      "Starter for the categorization task. The model has no productions, so the report explains what is missing.",
    highlights: [
      "Adjust the offset and feature slots",
      "Try model mode for a full run",
    ],
  },
};

export const GLOSSARY = [
  {
    term: "Model",
    definition:
      "A set of chunk types, declarative chunks, and productions that together describe a cognitive task.",
  },
  {
    term: "Production",
    definition:
      "An if-then rule. Its conditions match buffers; its actions change buffers or request modules.",
  },
  {
    term: "Buffer",
    definition:
      "A small working-memory slot owned by a module, such as goal, retrieval, or visual.",
  },
  {
    term: "Chunk",
    definition:
      "A structured piece of knowledge: a chunk type plus its slot-value pairs.",
  },
  {
    term: "Retrieval",
    definition:
      "A request to declarative memory. Activation and noise decide which chunk returns, and how quickly.",
  },
  {
    term: "Trace",
    definition:
      "The engine's event log. Timestamps, module names, and production firings show exactly what happened.",
  },
  {
    term: "Starter",
    definition:
      "An intentionally incomplete tutorial model you are meant to finish.",
  },
  {
    term: "Seed",
    definition:
      "The random seed pair. Equal inputs give identical traces; change it to change the noise.",
  },
];

export const HOW_TO_STEPS = [
  "Pick a model in the sidebar. Unit 1 is the best place to start.",
  "Press Run. Models with a task show an experiment window; the rest print a trace.",
  "Open the Trace tab to watch buffers, retrievals, and productions fire.",
  "Edit the model source, then run again to compare the results.",
];

export function unitInfo(unitId, fallbackLabel) {
  return (
    UNITS[unitId] || {
      title: fallbackLabel || unitId,
      blurb: "Tutorial models.",
    }
  );
}

export function modelInfo(model, unitId) {
  const meta = MODEL_META[model?.id] || {};
  const label = String(model?.label || model?.id || "Model");
  const derivedBadge = badgeFromLabel(label);
  return {
    description: meta.description || "A tutorial model for this unit.",
    highlights: meta.highlights || [],
    badge: meta.badge || derivedBadge,
    isStarter: (meta.badge || derivedBadge) === "starter",
    isRecommended: model?.id === RECOMMENDED_START,
    unitId,
  };
}

export function badgeFromLabel(label) {
  const lower = String(label || "").toLowerCase();
  if (lower.includes("starter")) return "starter";
  if (lower.includes("solution")) return "solution";
  if (lower.includes("broken")) return "broken";
  if (lower.includes("demo")) return "demo";
  return "";
}

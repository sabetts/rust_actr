import { unit2Tasks } from "./unit2.js";
import { fanTask, fanNoPmTask } from "./fan.js";
import { groupedTask } from "./grouped.js";
import { sieglerTask } from "./siegler.js";
import { onehitTask } from "./onehit.js";
import { bstTask, bstPpmTask } from "./bst.js";
import { choiceTask } from "./choice.js";
import { pairedTask } from "./paired.js";
import { zbrodoffTask } from "./zbrodoff.js";
import { pastTenseTask } from "./past-tense.js";
import { ulIssuesTask } from "./ul-issues.js";
import { pcompIssuesTask } from "./pcomp-issues.js";
import { categorizeTask } from "./categorize.js";
import { semanticDynamicTask } from "./semantic-dynamic.js";
import { sperlingTask } from "./sperling.js";
import { pmIssuesTask } from "./pm-issues.js";
import { subitizeTask } from "./subitize.js";

export const TASKS = {
  ...unit2Tasks,
  sperling: sperlingTask,
  "pm-issues": pmIssuesTask,
  subitize: subitizeTask,
  fan: fanTask,
  "fan-no-pm": fanNoPmTask,
  grouped: groupedTask,
  siegler: sieglerTask,
  onehit: onehitTask,
  bst: bstTask,
  "bst-ppm": bstPpmTask,
  choice: choiceTask,
  paired: pairedTask,
  zbrodoff: zbrodoffTask,
  "past-tense": pastTenseTask,
  "ul-issues": ulIssuesTask,
  "pcomp-issues": pcompIssuesTask,
  categorize: categorizeTask,
  "semantic-dynamic": semanticDynamicTask,
};

import { unit2Tasks } from "./unit2.js";
import { fanTask, fanNoPmTask } from "./fan.js";
import { groupedTask } from "./grouped.js";
import { sieglerTask } from "./siegler.js";
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
};

import { unit2Tasks } from "./unit2.js";
import { sperlingTask } from "./sperling.js";
import { pmIssuesTask } from "./pm-issues.js";
import { subitizeTask } from "./subitize.js";

export const TASKS = {
  ...unit2Tasks,
  sperling: sperlingTask,
  "pm-issues": pmIssuesTask,
  subitize: subitizeTask,
};

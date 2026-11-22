// Browser smoke test for the ACT-R 7 tutorial page.
//
// Requires a built wasm/model tree (`python rust_actr7/web/build.py`) and the
// Playwright Chromium browser (`npx playwright install chromium`).
//
//   cd rust_actr7/web
//   npm run test:e2e

import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";

const testDir = path.dirname(fileURLToPath(import.meta.url));
const webDir = path.resolve(testDir, "..");

if (!fs.existsSync(path.join(webDir, "actr_engine.wasm"))) {
  throw new Error(
    "missing actr_engine.wasm; run: python rust_actr7/web/build.py"
  );
}

const MIME = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".wasm": "application/wasm",
  ".css": "text/css; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".ico": "image/x-icon",
};

function staticServer() {
  return http.createServer((request, response) => {
    let pathname;
    try {
      pathname = decodeURIComponent(
        new URL(request.url || "/", "http://127.0.0.1").pathname
      );
    } catch {
      response.writeHead(400);
      response.end();
      return;
    }
    const relative = pathname === "/" ? "index.html" : pathname.replace(/^\/+/, "");
    const file = path.resolve(webDir, relative);
    if (file !== webDir && !file.startsWith(webDir + path.sep)) {
      response.writeHead(403);
      response.end();
      return;
    }
    fs.readFile(file, (error, data) => {
      if (error) {
        response.writeHead(404);
        response.end();
        return;
      }
      response.writeHead(200, {
        "Content-Type": MIME[path.extname(file)] || "application/octet-stream",
        "Cache-Control": "no-store",
      });
      response.end(data);
    });
  });
}

function listen(server) {
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      resolve(`http://127.0.0.1:${address.port}/`);
    });
  });
}

function closeServer(server) {
  return new Promise((resolve) => server.close(resolve));
}

async function main() {
  const server = staticServer();
  const baseUrl = await listen(server);
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  const errors = [];

  page.on("pageerror", (error) => errors.push(String(error)));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });

  const selectModel = async (id, token) => {
    await page.selectOption("#model-preset", id);
    await page.waitForFunction(
      (expected) => {
        const model = document.getElementById("model");
        const run = document.getElementById("run");
        return (
          Boolean(model) &&
          model.value.includes(expected) &&
          Boolean(run) &&
          !run.disabled
        );
      },
      token,
      { timeout: 30000 }
    );
  };

  const runModel = async () => {
    await page.click("#run");
    await page.waitForFunction(
      () => {
        const status = document.getElementById("status");
        const text = status ? status.textContent : "";
        return (
          text.startsWith("ok") ||
          text.startsWith("error") ||
          text.startsWith("cancelled")
        );
      },
      null,
      { timeout: 60000 }
    );
    const status = (await page.locator("#status").textContent()) || "";
    if (!status.startsWith("ok")) {
      throw new Error(`run failed: ${status}`);
    }
  };

  try {
    await page.goto(baseUrl, { waitUntil: "domcontentloaded" });
    await page.waitForFunction(
      () =>
        Boolean(
          document.querySelector('#model-preset option[value="unit8-simple-ppm"]')
        ),
      null,
      { timeout: 30000 }
    );

    await selectModel("unit8-simple-ppm", "define-model test-ppm");
    if (!(await page.locator("#task-panel").isHidden())) {
      throw new Error("Simple PPM should not show a task panel");
    }
    await runModel();
    const simpleOutput = await page.inputValue("#output");
    if (!simpleOutput.includes("SMALL")) {
      throw new Error(`Simple PPM trace missing SMALL:\n${simpleOutput}`);
    }

    await selectModel("unit8-semantic-dynamic", "define-model semantic");
    if (!(await page.locator("#task-panel").isVisible())) {
      throw new Error("Semantic Network should show a task panel");
    }
    if (!(await page.locator("#secs").isDisabled())) {
      throw new Error("Semantic Network should disable the run-time input");
    }
    await runModel();
    const semanticResult = (await page.locator("#task-result").textContent()) || "";
    const semanticReport = (await page.locator("#task-report").textContent()) || "";
    if (!semanticResult.includes("Semantic Network")) {
      throw new Error(`unexpected Semantic result: ${semanticResult}`);
    }
    if (!semanticReport.includes("ANSWER IS YES")) {
      throw new Error(`Semantic report missing answers:\n${semanticReport}`);
    }

    await selectModel("unit8-categorize", "define-model categorize");
    if (!(await page.locator("#task-panel").isVisible())) {
      throw new Error("Categorize should show a task panel");
    }
    if (!(await page.locator("#secs").isDisabled())) {
      throw new Error("Categorize should disable the run-time input");
    }
    await page.fill("#task-control-categorize-runs", "1");
    await runModel();
    const categorizeResult =
      (await page.locator("#task-result").textContent()) || "";
    const categorizeReport =
      (await page.locator("#task-report").textContent()) || "";
    if (!categorizeResult.includes("0/14")) {
      throw new Error(`unexpected Categorize result: ${categorizeResult}`);
    }
    if (!categorizeReport.includes("starter")) {
      throw new Error(`Categorize report missing starter note:\n${categorizeReport}`);
    }

    if (errors.length) {
      throw new Error(`browser errors:\n${errors.join("\n")}`);
    }
    console.log("e2e ok");
  } finally {
    await browser.close();
    await closeServer(server);
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});

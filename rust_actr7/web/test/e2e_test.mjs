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
    await page.evaluate((modelId) => {
      const row = document.querySelector(`[data-model-id="${modelId}"]`);
      if (!row) return;
      const unit = row.closest(".unit");
      if (unit && unit.dataset.open !== "true") {
        unit.querySelector(".unit-head")?.click();
      }
    }, id);
    await page.locator(`[data-model-id="${id}"]`).click();
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

  const runModelExpectingError = async () => {
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
    if (!status.startsWith("error")) {
      throw new Error(`run did not fail: ${status}`);
    }
  };

  const waitForRunStatus = async (expected = "ok") => {
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
    if (!status.startsWith(expected)) {
      throw new Error(`run status was ${status}, expected ${expected}`);
    }
  };

  const waitForHumanWindow = async () => {
    await page.waitForFunction(
      () => {
        const root = document.getElementById("agi-window");
        return Boolean(
          root &&
            !root.hidden &&
            root.querySelector(".agi-text, .agi-button")
        );
      },
      null,
      { timeout: 10000 }
    );
    await page.locator("#agi-window .agi-title").click();
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

    if (await page.locator("#welcome-card").isHidden()) {
      throw new Error("first-run welcome card should be visible");
    }
    if (!(await page.locator("#model-info").isHidden())) {
      throw new Error("model info should be hidden before a model is selected");
    }
    if (await page.locator("#model-search").count()) {
      throw new Error("model search should not be present");
    }
    if (await page.inputValue("#model")) {
      throw new Error("editor should start empty");
    }
    if (!(await page.locator("#run").isDisabled())) {
      throw new Error("Run should be disabled until a model is selected");
    }
    if ((await page.locator('#model-list [aria-current="true"]').count()) !== 0) {
      throw new Error("no model should be selected on first load");
    }
    const openUnits = await page.locator('.unit[data-open="true"]').count();
    if (openUnits !== 0) {
      throw new Error(`all units should start collapsed, found ${openUnits} open`);
    }
    if (!(await page.locator("footer").textContent()).includes("Runs entirely in your browser")) {
      throw new Error("footer is missing its runtime note");
    }
    if ((await page.locator("footer").textContent()).includes("Deterministic for a given seed")) {
      throw new Error("footer still contains the deterministic note");
    }
    const initialTheme = await page.evaluate(
      () => document.documentElement.dataset.theme
    );
    await page.click("#theme-toggle");
    await page.waitForFunction(
      (previous) => document.documentElement.dataset.theme !== previous,
      initialTheme,
      { timeout: 3000 }
    );
    await page.click("#theme-toggle");

    const iconHref = await page.evaluate(
      () => document.querySelector('link[rel="icon"]')?.getAttribute("href") || ""
    );
    if (iconHref !== "./assets/actr-head.png") {
      throw new Error(`unexpected favicon href: ${iconHref}`);
    }
    const logo = await page.locator(".brand-logo").evaluate((img) => ({
      complete: img.complete,
      width: img.naturalWidth,
      height: img.naturalHeight,
    }));
    if (!logo.complete || logo.width === 0 || logo.height === 0) {
      throw new Error("header ACT-R head logo did not load");
    }

    await page.locator('.unit[data-unit="unit1"] .unit-head').click();
    if ((await page.locator('.unit[data-unit="unit1"]').getAttribute("data-open")) !== "true") {
      throw new Error("Unit 1 did not expand");
    }
    const unit1Guides = page.locator('.unit[data-unit="unit1"] .unit-guide');
    if ((await unit1Guides.count()) !== 4) {
      throw new Error(`unexpected Unit 1 guide count: ${await unit1Guides.count()}`);
    }
    const firstGuideHref = await unit1Guides.first().getAttribute("href");
    if (!firstGuideHref.includes("./guides/unit1/unit1.html?v=")) {
      throw new Error(`unexpected first guide href: ${firstGuideHref}`);
    }
    const guideCheck = await page.evaluate(async () => {
      const manifest = await (
        await fetch("./guides/manifest.json", { cache: "no-store" })
      ).json();
      const paths = Object.values(manifest)
        .flat()
        .map((guide) => guide.path);
      const results = await Promise.all(
        paths.map(async (path) => {
          const response = await fetch(`./${path}`, { cache: "no-store" });
          return response.ok;
        })
      );
      return { count: paths.length, allOk: results.every(Boolean) };
    });
    if (guideCheck.count !== 28 || !guideCheck.allOk) {
      throw new Error(`guide manifest check failed: ${JSON.stringify(guideCheck)}`);
    }

    const arrow = page.locator('.unit[data-unit="unit1"] .unit-guide-open').first();
    if ((await arrow.getAttribute("target")) !== "_blank") {
      throw new Error("guide new-tab arrow is missing target=_blank");
    }

    await unit1Guides.first().click();
    await page.waitForFunction(
      () => {
        const tab = document.getElementById("tab-guide");
        const panel = document.getElementById("panel-guide");
        return (
          tab &&
          !tab.hidden &&
          tab.getAttribute("aria-selected") === "true" &&
          panel &&
          !panel.hidden
        );
      },
      null,
      { timeout: 10000 }
    );
    const guideSrc = (await page.locator("#guide-frame").getAttribute("src")) || "";
    if (!guideSrc.includes("guides/unit1/unit1.html?v=")) {
      throw new Error(`unexpected in-app guide src: ${guideSrc}`);
    }
    if ((await page.locator("#tab-guide-label").textContent()) !== "Unit guide") {
      throw new Error("guide tab label did not update");
    }

    const guideFrame = page.frameLocator("#guide-frame");
    await guideFrame.locator("main h1").first().waitFor({ timeout: 10000 });
    const guideHeading = (await guideFrame.locator("main h1").first().textContent()) || "";
    if (!guideHeading.includes("Introduction to ACT-R")) {
      throw new Error(`unexpected guide heading: ${guideHeading}`);
    }
    const frameClass = (await guideFrame.locator("html").getAttribute("class")) || "";
    if (!frameClass.includes("embedded")) {
      throw new Error("guide iframe was not switched to embedded mode");
    }
    if (!(await guideFrame.locator(".guide-bar").isHidden())) {
      throw new Error("embedded guide bar should be hidden");
    }
    await guideFrame.locator("main img").first().waitFor({ timeout: 10000 });
    const guideImageWidth = await guideFrame
      .locator("main img")
      .first()
      .evaluate((image) => image.naturalWidth);
    if (!guideImageWidth) throw new Error("guide image did not load");
    if ((await guideFrame.locator("pre.lisp").count()) === 0) {
      throw new Error("guide has no syntax-highlighted code blocks");
    }
    if ((await guideFrame.locator("pre.lisp .tok-keyword").count()) === 0) {
      throw new Error("guide code blocks have no keyword spans");
    }
    const codeStyle = await guideFrame
      .locator("pre.lisp code")
      .first()
      .evaluate((node) => {
        const style = getComputedStyle(node);
        return { fontFamily: style.fontFamily, whiteSpace: style.whiteSpace };
      });
    if (!/mono/i.test(codeStyle.fontFamily) || codeStyle.whiteSpace !== "pre") {
      throw new Error(`guide code block is not fixed width: ${JSON.stringify(codeStyle)}`);
    }

    const themeBefore = await guideFrame.locator("html").getAttribute("data-theme");
    await page.click("#theme-toggle");
    await page.waitForFunction(
      (previous) => {
        const frame = document.getElementById("guide-frame");
        const doc = frame?.contentDocument;
        return (
          doc?.documentElement?.dataset.theme &&
          doc.documentElement.dataset.theme !== previous
        );
      },
      themeBefore,
      { timeout: 3000 }
    );
    await page.click("#theme-toggle");

    await page.click("#guide-close");
    if (!(await page.locator("#tab-guide").isHidden())) {
      throw new Error("guide tab did not hide after close");
    }
    if (!(await page.locator("#panel-source").isVisible())) {
      throw new Error("closing the guide did not return to the source tab");
    }

    const [guidePage] = await Promise.all([
      page.waitForEvent("popup"),
      arrow.click(),
    ]);
    await guidePage.waitForLoadState("domcontentloaded");
    if (!(await guidePage.title()).includes("Unit 1")) {
      throw new Error("new-tab guide did not open");
    }
    await guidePage.close();

    await page.click("#tab-trace");
    if (!(await page.locator("#panel-trace").isVisible())) {
      throw new Error("Trace tab did not open");
    }
    if (!(await page.locator("#panel-source").isHidden())) {
      throw new Error("Model source panel should hide when Trace is selected");
    }
    await page.click("#tab-source");

    await page.click('[data-dialog-open="about-dialog"]');
    if (!(await page.locator("#about-dialog").evaluate((dialog) => dialog.open))) {
      throw new Error("ACT-R help dialog did not open");
    }
    const glossaryItems = await page.locator("#glossary-grid .glossary-item").count();
    if (glossaryItems < 4) {
      throw new Error(`ACT-R glossary is too small: ${glossaryItems}`);
    }
    await page.keyboard.press("Escape");
    if (await page.locator("#about-dialog").evaluate((dialog) => dialog.open)) {
      throw new Error("ACT-R help dialog did not close");
    }

    await page.click("#welcome-dismiss");
    if (!(await page.locator("#welcome-card").isHidden())) {
      throw new Error("welcome card did not dismiss");
    }

    await selectModel("unit8-simple-ppm", "define-model test-ppm");
    if ((await page.locator('[data-model-id="unit8-simple-ppm"] .task-tag').count()) !== 0) {
      throw new Error("trace-only model row should not have a Task tag");
    }
    if ((await page.locator("#model-info .task-tag").count()) !== 0) {
      throw new Error("trace-only model info should not have a Task tag");
    }
    if (await page.locator("#model-info").isHidden()) {
      throw new Error("model info should be visible after selecting a model");
    }
    if (!(await page.locator("#welcome-card").isHidden())) {
      throw new Error("welcome card should hide after selecting a model");
    }
    const infoBeforeBrowser = await page.evaluate(() => {
      const info = document.getElementById("model-info");
      const browser = document.querySelector(".model-browser-panel");
      return Boolean(
        info &&
          browser &&
          info.compareDocumentPosition(browser) & Node.DOCUMENT_POSITION_FOLLOWING
      );
    });
    if (!infoBeforeBrowser) {
      throw new Error("model info should sit above the model list");
    }
    const highlighted = (await page.locator("#editor-highlight").textContent()) || "";
    if (!highlighted.includes("define-model")) {
      throw new Error("syntax-highlighted source is missing define-model");
    }
    if (await page.locator("#model-outline").isHidden()) {
      throw new Error("model outline should be visible");
    }
    if (!(await page.locator("#task-panel").isHidden())) {
      throw new Error("Simple PPM should not show a task panel");
    }
    await runModel();
    const simpleOutput = await page.inputValue("#output");
    if (!simpleOutput.includes("SMALL")) {
      throw new Error(`Simple PPM trace missing SMALL:\n${simpleOutput}`);
    }
    if (!(await page.locator("#panel-trace").isVisible())) {
      throw new Error("generic model run should switch to the Trace tab");
    }
    const traceView = (await page.locator("#trace-view").textContent()) || "";
    if (!traceView.includes("SMALL")) {
      throw new Error(`colorized trace missing SMALL:\n${traceView}`);
    }
    const traceMeta = (await page.locator("#trace-meta").textContent()) || "";
    if (traceMeta === "No run yet") {
      throw new Error("trace metadata did not update");
    }
    if ((await page.locator("#trace-count").getAttribute("data-state")) === "error") {
      throw new Error("successful trace is marked as an error");
    }
    if ((await page.locator("#trace-meta").getAttribute("data-state")) === "error") {
      throw new Error("successful trace metadata is marked as an error");
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
    await page.fill("#task-control-categorize-offset", "0.5");
    await page.fill("#task-control-categorize-slot1", "size");
    await page.fill("#task-control-categorize-slot2", "shape");
    await page.fill("#task-control-categorize-slot3", "texture");
    await page.fill("#task-control-categorize-slot4", "color");
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
    if (!categorizeReport.includes("Offset: 0.5")) {
      throw new Error(`Categorize report missing offset:\n${categorizeReport}`);
    }
    if (!categorizeReport.includes("Slots: size, shape, texture, color")) {
      throw new Error(`Categorize report missing custom slots:\n${categorizeReport}`);
    }

    await selectModel("unit4-paired", "define-model paired");
    await page.check("#task-control-paired-trace");
    await runModel();
    const pairedCount = Number((await page.locator("#trace-count").textContent()) || "0");
    if (pairedCount <= 4000) {
      throw new Error(`Paired trace did not grow past the highlight cap: ${pairedCount}`);
    }
    await page.click("#tab-trace");
    const renderedTrace = await page.locator("#trace-view").textContent();
    if (!renderedTrace.includes("PRODUCTION-FIRED")) {
      throw new Error("long trace rendered without its production lines");
    }
    const filledLines = await page
      .locator("#trace-view .trace-text")
      .evaluateAll((nodes) => nodes.filter((node) => node.textContent.trim()).length);
    if (filledLines < 1000) {
      throw new Error(`long trace rendered only ${filledLines} filled lines`);
    }
    const coloredTimes = await page.locator("#trace-view .trace-time").count();
    if (coloredTimes < 1000) {
      throw new Error(`long trace was not colorized: ${coloredTimes} time spans`);
    }
    const outputLines = await page.locator("#trace-view .trace-line.trace-output").count();
    if (outputLines > pairedCount * 0.25) {
      throw new Error(
        `too many lines misclassified as output: ${outputLines}/${pairedCount}`
      );
    }

    await selectModel("unit8-simple-ppm", "define-model test-ppm");
    const outlineItem = page.locator(".outline-item").nth(1);
    const outlineLine = Number(
      (await outlineItem.locator(".outline-line").textContent()) || "1"
    );
    await outlineItem.click();
    await page.waitForFunction(
      (line) => {
        const textarea = document.getElementById("model");
        if (document.activeElement !== textarea) return false;
        const lines = textarea.value.split("\n");
        let offset = 0;
        for (let i = 0; i < line - 1; i += 1) offset += lines[i].length + 1;
        return textarea.selectionStart === offset;
      },
      outlineLine,
      { timeout: 3000 }
    );
    const outlineLayout = await page.evaluate(() => {
      const textarea = document.getElementById("model").getBoundingClientRect();
      const outline = document.getElementById("model-outline").getBoundingClientRect();
      return {
        beside: Math.round(outline.left - textarea.right),
        top: Math.round(outline.top - textarea.top),
      };
    });
    if (outlineLayout.beside < -2 || Math.abs(outlineLayout.top) > 2) {
      throw new Error(
        `outline is not beside the editor on wide screens: ${JSON.stringify(outlineLayout)}`
      );
    }

    await page.setViewportSize({ width: 1000, height: 720 });
    await page.waitForTimeout(100);
    const stackedGap = await page.evaluate(() => {
      const textarea = document.getElementById("model").getBoundingClientRect();
      const outline = document.getElementById("model-outline").getBoundingClientRect();
      return Math.round(outline.top - textarea.bottom);
    });
    if (Math.abs(stackedGap) > 2) {
      throw new Error(
        `outline is not stacked below the editor on narrow screens: ${stackedGap}px`
      );
    }
    await page.setViewportSize({ width: 1280, height: 720 });

    await page.evaluate(() => {
      const textarea = document.getElementById("model");
      textarea.focus();
      textarea.setSelectionRange(1, 1);
      document.dispatchEvent(new Event("selectionchange"));
    });
    await page.waitForFunction(
      () => document.querySelectorAll(".tok-paren-match").length === 2,
      null,
      { timeout: 3000 }
    );
    await page.evaluate(() => {
      const textarea = document.getElementById("model");
      const text = textarea.value;
      let close = text.lastIndexOf(")");
      while (close > 0 && (text[close - 1] === "(" || text[close - 1] === ")")) {
        close = text.lastIndexOf(")", close - 1);
      }
      textarea.setSelectionRange(close, close);
      document.dispatchEvent(new Event("selectionchange"));
    });
    await page.waitForFunction(
      () => document.querySelectorAll(".tok-paren-match").length === 0,
      null,
      { timeout: 3000 }
    );
    await page.evaluate(() => {
      const textarea = document.getElementById("model");
      const text = textarea.value;
      let close = text.lastIndexOf(")");
      while (close > 0 && (text[close - 1] === "(" || text[close - 1] === ")")) {
        close = text.lastIndexOf(")", close - 1);
      }
      textarea.setSelectionRange(close + 1, close + 1);
      document.dispatchEvent(new Event("selectionchange"));
    });
    await page.waitForFunction(
      () => document.querySelectorAll(".tok-paren-match").length === 2,
      null,
      { timeout: 3000 }
    );
    await page.evaluate(() => {
      const textarea = document.getElementById("model");
      textarea.setSelectionRange(0, 2);
      document.dispatchEvent(new Event("selectionchange"));
    });
    await page.waitForFunction(
      () => document.querySelectorAll(".tok-paren-match").length === 0,
      null,
      { timeout: 3000 }
    );

    const brokenSource =
      "(clear-all)\n(define-model broken\n  (sgp :v t)\n)\n)\n";
    await page.fill("#model", brokenSource);
    await page.waitForFunction(
      () => {
        const chip = document.getElementById("paren-status");
        return chip && !chip.hidden;
      },
      null,
      { timeout: 3000 }
    );
    const parenText = (await page.locator("#paren-status").textContent()) || "";
    if (!parenText.includes("Unmatched")) {
      throw new Error(`paren chip is wrong: ${parenText}`);
    }
    if ((await page.locator(".tok-paren-bad").count()) < 1) {
      throw new Error("unmatched paren was not highlighted");
    }

    await runModelExpectingError();
    if (!(await page.locator("#panel-source").isVisible())) {
      throw new Error("parse error did not switch to the Model source tab");
    }
    if (!(await page.locator("#editor-line-marker").isVisible())) {
      throw new Error("parse error line marker is not visible");
    }
    const errorText = (await page.locator("#error-status").textContent()) || "";
    if (!errorText.includes("line 5")) {
      throw new Error(`error chip is wrong: ${errorText}`);
    }
    const gutterError =
      (await page.locator(".editor-gutter-line.is-error").textContent()) || "";
    if (gutterError !== "5") {
      throw new Error(`error gutter line is wrong: ${gutterError}`);
    }
    const parseTrace = (await page.locator("#trace-view").textContent()) || "";
    if (!parseTrace.includes("parse error at line 5")) {
      throw new Error(`trace is missing the parse error:\n${parseTrace}`);
    }
    if (parseTrace.includes('{"ok":false')) {
      throw new Error("trace still contains the raw wasm JSON error");
    }
    const rawOutput = await page.inputValue("#output");
    if (!rawOutput.includes("parse error at line 5")) {
      throw new Error(`raw trace is missing the parse error:\n${rawOutput}`);
    }
    if ((await page.locator("#trace-count").getAttribute("data-state")) !== "error") {
      throw new Error("error trace count is not marked as an error");
    }
    if ((await page.locator("#trace-meta").getAttribute("data-state")) !== "error") {
      throw new Error("error trace metadata is not marked as an error");
    }

    await selectModel("unit8-categorize", "define-model categorize");
    await page.click("#tab-source");
    const taskBrokenSource =
      "(clear-all)\n(define-model categorize\n  (sgp :v t)\n)\n)\n;; trailing one\n;; trailing two\n";
    await page.fill("#model", taskBrokenSource);
    await page.waitForFunction(
      () => {
        const chip = document.getElementById("paren-status");
        return chip && !chip.hidden;
      },
      null,
      { timeout: 3000 }
    );
    await runModelExpectingError();
    const taskErrorText = (await page.locator("#error-status").textContent()) || "";
    if (!taskErrorText.includes("line 5")) {
      throw new Error(`task error chip is wrong: ${taskErrorText}`);
    }
    const taskGutter =
      (await page.locator(".editor-gutter-line.is-error").textContent()) || "";
    if (taskGutter !== "5") {
      throw new Error(`task error gutter line is wrong: ${taskGutter}`);
    }
    const taskTrace = (await page.locator("#trace-view").textContent()) || "";
    const engineLine = taskTrace.match(/parse error at line (\d+):/i);
    if (!engineLine) {
      throw new Error(`task trace is missing the parse error:\n${taskTrace}`);
    }
    if (Number(engineLine[1]) <= 5) {
      throw new Error(
        `task prelude offset was not applied: engine line ${engineLine[1]}`
      );
    }

    await selectModel("unit2-demo2", "define-model demo2");
    const demoRowTaskTag = (
      (await page.locator('[data-model-id="unit2-demo2"] .task-tag').textContent()) || ""
    ).trim();
    if (demoRowTaskTag !== "Task") {
      throw new Error(`task model row is missing its Task tag: ${demoRowTaskTag}`);
    }
    const demoInfoTaskTag = (
      (await page.locator("#model-info .task-tag").textContent()) || ""
    ).trim();
    if (demoInfoTaskTag !== "Task") {
      throw new Error(`task model info is missing its Task tag: ${demoInfoTaskTag}`);
    }
    const demoControlWidths = await page.evaluate(() => {
      const controls = document.getElementById("task-controls");
      const input = document.getElementById("task-control-unit2-demo2-mode");
      return {
        controls: controls?.getBoundingClientRect().width || 0,
        input: input?.getBoundingClientRect().width || 0,
      };
    });
    if (demoControlWidths.input > 260) {
      throw new Error(
        `task control is too wide on a wide window: ${demoControlWidths.input}px`
      );
    }
    if (demoControlWidths.controls > 320) {
      throw new Error(
        `task control card should hug its controls: ${demoControlWidths.controls}px`
      );
    }
    await page.selectOption("#task-control-unit2-demo2-mode", "human");
    await page.click("#run");
    await waitForHumanWindow();
    const demoTarget = (
      (await page.locator("#agi-window .agi-text").first().textContent()) || ""
    ).trim();
    await page.keyboard.press(demoTarget.toLowerCase());
    await waitForRunStatus("ok");
    const demoResult = (await page.locator("#task-result").textContent()) || "";
    if (!demoResult.includes(`response: ${demoTarget.toLowerCase()}`)) {
      throw new Error(`Demo2 human response missing: ${demoResult}`);
    }

    await selectModel("unit2-unit2-assignment", "define-model unit2");
    await page.selectOption("#task-control-unit2-assignment-mode", "human");
    await page.click("#run");
    await waitForHumanWindow();
    const assignmentTexts = await page
      .locator("#agi-window .agi-text")
      .allTextContents();
    const letters = assignmentTexts.map((text) => text.trim().toUpperCase());
    const targetLetter = [...new Set(letters)].find(
      (letter) => letters.filter((value) => value === letter).length === 1
    );
    if (!targetLetter) {
      throw new Error(`Unit 2 Assignment human display has no target: ${letters}`);
    }
    await page.keyboard.press(targetLetter.toLowerCase());
    await waitForRunStatus("ok");
    const assignmentResult = (await page.locator("#task-result").textContent()) || "";
    if (!assignmentResult.includes(`response: ${targetLetter.toLowerCase()}`)) {
      throw new Error(`Unit 2 Assignment human response missing: ${assignmentResult}`);
    }

    await selectModel("unit3-subitize", "define-model subitize");
    await page.selectOption("#task-control-subitize-mode", "human");
    await page.selectOption("#task-control-subitize-items", "1");
    await page.click("#run");
    await waitForHumanWindow();
    await page.waitForSelector(".human-choice-bar", { timeout: 5000 });
    const subitizePrompt = (await page.locator(".human-prompt").textContent()) || "";
    if (!subitizePrompt.includes("Item 1/1")) {
      throw new Error(`Subitize progress count missing: ${subitizePrompt}`);
    }
    const subitizeChoices = await page
      .locator(".human-choice-bar button")
      .allTextContents();
    if (subitizeChoices.at(-1) !== "0") {
      throw new Error(`Subitize zero should come after nine: ${subitizeChoices.join(",")}`);
    }
    await page
      .locator(".human-choice-bar button")
      .filter({ hasText: /^1$/ })
      .click();
    await waitForRunStatus("ok");
    const subitizeReport = (await page.locator("#task-report").textContent()) || "";
    if (!subitizeReport.includes("Mode: human")) {
      throw new Error(`Subitize human report missing mode:\n${subitizeReport}`);
    }

    await selectModel("unit4-zbrodoff", "define-model zbrodoff");
    await page.selectOption("#task-control-zbrodoff-runner", "human");
    await page.click("#run");
    await waitForHumanWindow();
    const zbrodoffPrompt = (await page.locator(".human-prompt").textContent()) || "";
    if (!zbrodoffPrompt.includes("Trial 1/1")) {
      throw new Error(`Alpha-Arithmetic progress count missing: ${zbrodoffPrompt}`);
    }
    await page.keyboard.press("k");
    await waitForRunStatus("ok");
    const zbrodoffReport = (await page.locator("#task-report").textContent()) || "";
    if (!zbrodoffReport.includes("Mode: human (problem)")) {
      throw new Error(`Alpha-Arithmetic human report missing mode:\n${zbrodoffReport}`);
    }
    if (zbrodoffReport.includes("Starter model")) {
      throw new Error(`Alpha-Arithmetic human report shows starter text:\n${zbrodoffReport}`);
    }

    await selectModel("unit3-sperling", "define-model sperling");
    await page.selectOption("#task-control-sperling-mode", "human");
    await page.click("#run");
    await waitForHumanWindow();
    for (let trial = 0; trial < 4; trial += 1) {
      await page.waitForFunction(
        () => document.querySelectorAll("#agi-window .agi-text").length > 0,
        null,
        { timeout: 5000 }
      );
      if (await page.locator(".human-hint").count()) {
        throw new Error("Sperling human mode should not reveal the cued row");
      }
      await page.waitForFunction(
        () => document.querySelectorAll("#agi-window .agi-text").length === 0,
        null,
        { timeout: 5000 }
      );
      await page.waitForSelector(".human-prompt", { timeout: 5000 });
      const sperlingPrompt = (await page.locator(".human-prompt").textContent()) || "";
      if (!sperlingPrompt.includes(`Trial ${trial + 1}/4`)) {
        throw new Error(`Sperling progress count missing: ${sperlingPrompt}`);
      }
      await page.waitForTimeout(100);
      await page.keyboard.press("Space");
    }
    await waitForRunStatus("ok");
    const sperlingReport = (await page.locator("#task-report").textContent()) || "";
    if (!sperlingReport.includes("Mode: human")) {
      throw new Error(`Sperling human report missing mode:\n${sperlingReport}`);
    }

    await selectModel(
      "unit3-perceptual-motor-issues",
      "define-model perceptual-motor-issues"
    );
    await page.selectOption("#task-control-pm-issues-mode", "human");
    await page.click("#run");
    await waitForHumanWindow();
    const shownLetter = (
      (await page.locator("#agi-window .agi-text").first().textContent()) || ""
    )
      .trim()
      .toLowerCase();
    await page.waitForFunction(
      () => {
        const text =
          document.querySelector("#agi-window .agi-text")?.textContent || "";
        return text === "next" || text === "previous";
      },
      null,
      { timeout: 5000 }
    );
    const promptWord = (
      (await page.locator("#agi-window .agi-text").first().textContent()) || ""
    )
      .trim()
      .toLowerCase();
    const indexed = "zabcdefghijklmnopqrstuvwxyz";
    const shownIndex = indexed.indexOf(shownLetter);
    const secondIndex = promptWord === "next" ? shownIndex + 1 : shownIndex - 1;
    const secondLetter =
      secondIndex >= 0 && secondIndex < indexed.length
        ? indexed[secondIndex]
        : shownLetter;
    await page.keyboard.press(shownLetter);
    await page.keyboard.press(secondLetter);
    await waitForRunStatus("ok");
    const pmReport = (await page.locator("#task-report").textContent()) || "";
    if (!pmReport.includes("Mode: human")) {
      throw new Error(`PM Issues human report missing mode:\n${pmReport}`);
    }

    await selectModel("unit5-fan", "define-model fan");
    await page.selectOption("#task-control-fan-mode", "human");
    await page.click("#run");
    await waitForHumanWindow();
    await page.waitForSelector(".human-prompt", { timeout: 5000 });
    const fanPrompt = (await page.locator(".human-prompt").textContent()) || "";
    if (!fanPrompt.includes("Probe 1/18")) {
      throw new Error(`Fan progress count missing: ${fanPrompt}`);
    }
    let fanText =
      (await page.locator("#agi-window .agi-text").first().textContent()) || "";
    await page.waitForTimeout(150);
    for (let probe = 0; probe < 18; probe += 1) {
      await page.keyboard.press(probe < 9 ? "k" : "d");
      if (probe < 17) {
        await page.waitForTimeout(100);
        await page.waitForFunction(
          (previous) =>
            (document.querySelector("#agi-window .agi-text")?.textContent || "") !==
            previous,
          fanText,
          { timeout: 5000 }
        );
        fanText =
          (await page.locator("#agi-window .agi-text").first().textContent()) || "";
        if (probe === 0) {
          await page.waitForFunction(
            () =>
              (document.querySelector(".human-prompt")?.textContent || "").includes(
                "Probe 2/18"
              ),
            null,
            { timeout: 5000 }
          );
        }
      }
    }
    await waitForRunStatus("ok");
    const fanReport = (await page.locator("#task-report").textContent()) || "";
    if (!fanReport.includes("Mode: human")) {
      throw new Error(`Fan human report missing mode:\n${fanReport}`);
    }

    await selectModel(
      "unit7-production-compilation-issues",
      "define-model compilation-test"
    );
    await page.selectOption("#task-control-pcomp-issues-mode", "human");
    await page.fill("#task-control-pcomp-issues-humanTrials", "2");
    await page.click("#run");
    await waitForHumanWindow();
    const pcompPrompt = (await page.locator(".human-prompt").textContent()) || "";
    if (!pcompPrompt.includes("Trial 1/2")) {
      throw new Error(`Production Compilation progress count missing: ${pcompPrompt}`);
    }
    await page.keyboard.press("s");
    await page.waitForFunction(
      () => {
        const text =
          document.querySelector("#agi-window .agi-text")?.textContent || "";
        return text === "win" || text === "lose" || text === "draw";
      },
      null,
      { timeout: 3000 }
    );
    await page.waitForTimeout(150);
    await page.keyboard.press("Space");
    await page.waitForFunction(
      () =>
        (document.querySelector(".human-prompt")?.textContent || "").includes(
          "Trial 2/2"
        ),
      null,
      { timeout: 5000 }
    );
    await page.keyboard.press("s");
    await page.waitForFunction(
      () => {
        const text =
          document.querySelector("#agi-window .agi-text")?.textContent || "";
        return text === "win" || text === "lose" || text === "draw";
      },
      null,
      { timeout: 3000 }
    );
    await page.keyboard.press("Space");
    await waitForRunStatus("ok");
    const pcompReport = (await page.locator("#task-report").textContent()) || "";
    if (!pcompReport.includes("Human trials: 2")) {
      throw new Error(`Production Compilation human trial count missing:\n${pcompReport}`);
    }
    if (!pcompReport.includes("Completed trials: 2 / 2 per game")) {
      throw new Error(`Production Compilation completed count missing:\n${pcompReport}`);
    }

    await selectModel("unit5-onehit", "define-model 1-hit-model");
    await page.selectOption("#task-control-onehit-mode", "play");
    await page.fill("#task-control-onehit-hands", "1");
    await page.fill("#task-control-onehit-seconds", "1");
    await page.click("#run");
    await page.waitForSelector(".bj-status", { timeout: 10000 });
    const onehitHint = (await page.locator(".bj-status").textContent()) || "";
    if (!onehitHint.includes("Hand 1/1")) {
      throw new Error(`1-Hit Blackjack hand count missing: ${onehitHint}`);
    }
    await page.locator('.bj-actions button[data-choice="s"]').click();
    await waitForRunStatus("ok");
    const onehitReport = (await page.locator("#task-report").textContent()) || "";
    if (!onehitReport.includes("Mode: play against model")) {
      throw new Error(`1-Hit Blackjack play report missing:\n${onehitReport}`);
    }

    await selectModel("unit6-bst", "define-model bst");
    await page.selectOption("#task-control-bst-mode", "human");
    await page.click("#run");
    await waitForHumanWindow();
    const bstHint = (await page.locator(".human-hint").textContent()) || "";
    if (!bstHint.includes("Problem 1/15")) {
      throw new Error(`Building Sticks progress count missing: ${bstHint}`);
    }
    for (const label of ["B", "C", "C", "A"]) {
      await page
        .locator("#agi-window .agi-button")
        .filter({ hasText: new RegExp(`^${label}$`) })
        .click();
    }
    await page.waitForFunction(
      () =>
        ((document.getElementById("status") || {}).textContent || "").includes("1/15"),
      null,
      { timeout: 10000 }
    );
    await page.click("#cancel");
    await waitForRunStatus("cancelled");

    await selectModel("unit5-grouped", "define-model grouped");
    await page.selectOption("#task-control-grouped-mode", "human");
    await page.fill("#task-control-grouped-humanTrials", "2");
    await page.click("#run");
    await waitForHumanWindow();
    for (let trial = 0; trial < 2; trial += 1) {
      await page.waitForFunction(
        () => document.querySelectorAll("#agi-window .agi-text").length === 3,
        null,
        { timeout: 10000 }
      );
      const groupedLines = await page
        .locator("#agi-window .agi-text")
        .allTextContents();
      const groupedStudy = groupedLines.join("").replace(/\s+/g, "");
      if (!/^[1-9]{9}$/.test(groupedStudy)) {
        throw new Error(`Grouped study list looks wrong: ${groupedLines.join(" | ")}`);
      }
      await page.waitForSelector(".human-text-row input", { timeout: 10000 });
      await page.fill(".human-text-row input", groupedStudy);
      await page.locator(".human-text-row button").click();
      if (trial === 0) {
        await page.waitForFunction(
          () => !document.querySelector(".human-text-row"),
          null,
          { timeout: 3000 }
        );
      }
    }
    await waitForRunStatus("ok");
    const groupedReport = (await page.locator("#task-report").textContent()) || "";
    if (!groupedReport.includes("Mode: human")) {
      throw new Error(`Grouped human report missing mode:\n${groupedReport}`);
    }
    if (!groupedReport.includes("Trials: 2")) {
      throw new Error(`Grouped human report missing trial count:\n${groupedReport}`);
    }
    if (!groupedReport.includes("Mean correct: 9.00/9")) {
      throw new Error(`Grouped human recall was not scored correctly:\n${groupedReport}`);
    }
    if (!groupedReport.includes("Exact matches: 2/2")) {
      throw new Error(`Grouped human exact matches were not recorded:\n${groupedReport}`);
    }

    await page.selectOption("#task-control-grouped-humanListMode", "canonical");
    if (!(await page.locator("#task-control-grouped-humanTrials").isHidden())) {
      throw new Error("Grouped canonical mode should hide the human trial count");
    }
    await page.click("#run");
    await waitForHumanWindow();
    await page.waitForFunction(
      () => document.querySelectorAll("#agi-window .agi-text").length === 3,
      null,
      { timeout: 10000 }
    );
    const canonicalLines = await page
      .locator("#agi-window .agi-text")
      .allTextContents();
    const canonicalStudy = canonicalLines.join("").replace(/\s+/g, "");
    if (canonicalStudy !== "123456789") {
      throw new Error(`Grouped canonical list looks wrong: ${canonicalLines.join(" | ")}`);
    }
    await page.waitForSelector(".human-text-row input", { timeout: 10000 });
    await page.fill(".human-text-row input", "123456789");
    await page.locator(".human-text-row button").click();
    await waitForRunStatus("ok");
    const canonicalReport = (await page.locator("#task-report").textContent()) || "";
    if (!canonicalReport.includes("Lists: canonical (123 / 456 / 789)")) {
      throw new Error(`Grouped canonical report missing list mode:\n${canonicalReport}`);
    }
    if (!canonicalReport.includes("Trials: 1")) {
      throw new Error(`Grouped canonical run should use one trial:\n${canonicalReport}`);
    }

    await selectModel(
      "unit7-utility-learning-issues",
      "define-model utility-learning-test"
    );
    await page.selectOption("#task-control-ul-issues-mode", "human");
    await page.fill("#task-control-ul-issues-humanTrials", "2");
    await page.click("#run");
    await waitForHumanWindow();
    await page
      .locator(".human-choice-bar button")
      .filter({ hasText: /^A$/ })
      .click();
    await page.waitForSelector(".human-choice-bar", { timeout: 10000 });
    await page
      .locator(".human-choice-bar button")
      .filter({ hasText: /^B$/ })
      .click();
    await waitForRunStatus("ok");
    const utilityReport = (await page.locator("#task-report").textContent()) || "";
    if (!utilityReport.includes("Mode: human")) {
      throw new Error(`Utility Learning human report missing mode:\n${utilityReport}`);
    }
    if (!utilityReport.includes("Choices: A 1, B 1")) {
      throw new Error(`Utility Learning human choices were not recorded:\n${utilityReport}`);
    }

    await selectModel("unit8-semantic-dynamic", "define-model semantic");
    await page.selectOption("#task-control-semantic-dynamic-mode", "human");
    await page.click("#run");
    await waitForHumanWindow();
    for (const [index, label] of ["Yes", "Yes", "No"].entries()) {
      await page.waitForSelector(".human-choice-bar", { timeout: 5000 });
      await page
        .locator(".human-choice-bar button")
        .filter({ hasText: new RegExp(`^${label}$`) })
        .click();
      if (index < 2) {
        await page.waitForFunction(
          () => !document.querySelector(".human-choice-bar"),
          null,
          { timeout: 3000 }
        );
      }
    }
    await waitForRunStatus("ok");
    const semanticHumanReport =
      (await page.locator("#task-report").textContent()) || "";
    if (!semanticHumanReport.includes("Mode: human")) {
      throw new Error(`Semantic human report missing mode:\n${semanticHumanReport}`);
    }
    if (!semanticHumanReport.includes("model: DONT-KNOW")) {
      throw new Error(`Semantic human report missing model comparison:\n${semanticHumanReport}`);
    }

    const audioOk = await page.evaluate(async () => {
      const response = await fetch(
        "./audio/floraphonic-casual-voice-man-says-1-186552.mp3",
        { cache: "no-store" }
      );
      return response.ok && (await response.arrayBuffer()).byteLength > 0;
    });
    if (!audioOk) {
      throw new Error("digit audio asset did not load");
    }

    await selectModel("unit5-siegler", "define-model siegler");
    await page.selectOption("#task-control-siegler-mode", "human");
    await page.fill("#task-control-siegler-humanSets", "1");
    await page.click("#run");
    await waitForHumanWindow();
    for (let trial = 0; trial < 6; trial += 1) {
      await page.waitForSelector(".human-choice-bar", { timeout: 15000 });
      const sieglerPrompt = (await page.locator(".human-prompt").textContent()) || "";
      if (!sieglerPrompt.includes(`Problem ${trial + 1}/6`)) {
        throw new Error(`Siegler progress count missing: ${sieglerPrompt}`);
      }
      if (trial === 0) {
        const replay = page.locator(".human-choice-bar button.secondary");
        if (await replay.count()) await replay.first().click();
      }
      await page
        .locator(".human-choice-bar button")
        .filter({ hasText: /^0$/ })
        .click();
      if (trial < 5) {
        await page.waitForFunction(
          () => !document.querySelector(".human-choice-bar"),
          null,
          { timeout: 3000 }
        );
      }
    }
    await waitForRunStatus("ok");
    const sieglerReport = (await page.locator("#task-report").textContent()) || "";
    if (!sieglerReport.includes("Mode: human")) {
      throw new Error(`Siegler human report missing mode:\n${sieglerReport}`);
    }

    await selectModel("unit7-past-tense", "define-model past-tense");
    await page.selectOption("#task-control-past-tense-mode", "human");
    await page.fill("#task-control-past-tense-humanTrials", "2");
    await page.click("#run");
    await waitForHumanWindow();
    for (let trial = 0; trial < 2; trial += 1) {
      await page.waitForSelector(".human-text-row input", { timeout: 5000 });
      const pastTensePrompt = (await page.locator(".human-prompt").textContent()) || "";
      if (!pastTensePrompt.includes(`Verb ${trial + 1}/2`)) {
        throw new Error(`Past Tense progress count missing: ${pastTensePrompt}`);
      }
      await page.fill(".human-text-row input", "tested");
      await page.locator(".human-text-row button").click();
      if (trial === 0) {
        await page.waitForFunction(
          () => !document.querySelector(".human-text-row"),
          null,
          { timeout: 3000 }
        );
      }
    }
    await waitForRunStatus("ok");
    const pastTenseReport = (await page.locator("#task-report").textContent()) || "";
    if (!pastTenseReport.includes("Mode: human")) {
      throw new Error(`Past Tense human report missing mode:\n${pastTenseReport}`);
    }
    if (!pastTenseReport.includes("Verbs: 2")) {
      throw new Error(`Past Tense human trial count missing:\n${pastTenseReport}`);
    }

    await selectModel("unit8-categorize", "define-model categorize");
    await page.selectOption("#task-control-categorize-mode", "human");
    await page.fill("#task-control-categorize-studySeconds", "1");
    await page.click("#run");
    await waitForHumanWindow();
    for (let item = 0; item < 14; item += 1) {
      await page.waitForSelector(".human-choice-bar", { timeout: 5000 });
      await page
        .locator(".human-choice-bar button")
        .filter({ hasText: /^Category 1$/ })
        .click();
      if (item < 13) {
        await page.waitForFunction(
          () => !document.querySelector(".human-choice-bar"),
          null,
          { timeout: 3000 }
        );
      }
    }
    await waitForRunStatus("ok");
    const categorizeHumanReport =
      (await page.locator("#task-report").textContent()) || "";
    if (!categorizeHumanReport.includes("Mode: human")) {
      throw new Error(`Categorize human report missing mode:\n${categorizeHumanReport}`);
    }
    if (!categorizeHumanReport.includes("Category 1 choices: 14/14")) {
      throw new Error(`Categorize human answers were not recorded:\n${categorizeHumanReport}`);
    }

    await selectModel("unit4-paired", "define-model paired");
    await page.locator('.unit[data-unit="unit4"] .unit-guide').first().click();
    await page.waitForFunction(
      () => {
        const tab = document.getElementById("tab-guide");
        return (
          tab &&
          !tab.hidden &&
          tab.getAttribute("aria-selected") === "true"
        );
      },
      null,
      { timeout: 5000 }
    );

    await selectModel("unit4-zbrodoff", "define-model zbrodoff");
    if (await page.locator("#tab-guide").isHidden()) {
      throw new Error("same-unit model selection should keep the guide open");
    }

    await selectModel("unit5-fan", "define-model fan");
    if (!(await page.locator("#tab-guide").isHidden())) {
      throw new Error("switching units should close the guide tab");
    }
    if (!(await page.locator("#panel-guide").isHidden())) {
      throw new Error("switching units should close the guide panel");
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

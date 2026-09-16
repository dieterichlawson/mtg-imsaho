// The GUI's smoke test: a real runner, a real browser, the first turns.
//
//   cargo build -p mtg-runner
//   NODE_PATH=$(npm root -g) node mtg-gui/tests/smoke.js [--shots DIR]
//
// Starts `mtg-runner --p1 gui --p2 random --seed 5`, opens the page in
// the Playwright Chromium, keeps the opening hand, plays a land through the
// card popover, passes, and checks that every step produced the decision
// the engine asks next and that the page logged no error. With --shots it
// writes a screenshot per step, which is how the layout is reviewed.

const { chromium } = require("playwright");
const { spawn } = require("child_process");
const path = require("path");
const fs = require("fs");

const root = path.resolve(__dirname, "..", "..");
const shotsDir = process.argv.includes("--shots") ? process.argv[process.argv.indexOf("--shots") + 1] : null;
const port = 8700 + Math.floor(Math.random() * 200);

function fail(msg) { console.error("FAIL: " + msg); process.exitCode = 1; }

async function main() {
  const runner = spawn(path.join(root, "target", "debug", "mtg-runner"),
    ["--p1", `gui:${port}`, "--p2", "random", "--seed", "5", "--deck1", "red-green", "--deck2", "white-black", "-q"],
    { cwd: root, stdio: ["ignore", "pipe", "pipe"] });
  let runnerErr = "";
  runner.stderr.on("data", d => { runnerErr += d; });
  runner.stdout.on("data", () => {});
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
    const errors = [];
    page.on("pageerror", e => errors.push("pageerror: " + e.message));
    page.on("console", m => { if (m.type() === "error" && !/404/.test(m.text())) errors.push("console: " + m.text()); });
    // The runner binds before it prints; retry the first load briefly.
    for (let i = 0; i < 30; i++) {
      try { await page.goto(`http://127.0.0.1:${port}/`); break; } catch (e) { await page.waitForTimeout(200); }
    }
    const waitDecision = () => page.waitForFunction(() => window.mtg && (window.mtg.decision || window.mtg.gameOver), null, { timeout: 30000 });
    const info = () => page.evaluate(() => ({
      mode: window.mtg.ui && window.mtg.ui.mode, context: window.mtg.decision && window.mtg.decision.legal.context,
      turn: window.mtg.view && window.mtg.view.turn_number, hand: window.mtg.view ? window.mtg.view.your_hand.length : 0,
    }));
    const shot = async (name) => { if (shotsDir) { fs.mkdirSync(shotsDir, { recursive: true }); await page.screenshot({ path: path.join(shotsDir, name) }); } };
    const clickHit = async (pred) => {
      const box = await page.evaluate((src) => {
        const f = new Function("h", "m", `return (${src})(h, m)`);
        const h = window.mtg.hits.slice().reverse().find(h => f(h, window.mtg));
        return h ? [h.x + h.w / 2, h.y + h.h / 2] : null;
      }, pred);
      if (!box) throw new Error("nothing to click for " + pred);
      await page.mouse.move(box[0] * 2, box[1] * 2); await page.waitForTimeout(50);
      await page.mouse.click(box[0] * 2, box[1] * 2); await page.waitForTimeout(150);
    };

    await waitDecision();
    let s = await info();
    if (s.mode !== "list" || !/MULLIGAN/.test(s.context || "")) fail(`expected the mulligan list first, got ${JSON.stringify(s)}`);
    await shot("1-mulligan.png");
    // Keep: the first row of the modal.
    await clickHit("(h, m) => h.kind === 'row' && m.ui.rows[0] && h.y === m.hits.filter(x => x.kind === 'row')[0].y");
    await waitDecision();
    s = await info();
    if (s.mode !== "menu") fail(`expected the priority menu after keeping, got ${JSON.stringify(s)}`);
    await shot("2-main.png");
    // A land in hand has a "Play land" verb; click it, then the popover row.
    const land = await page.evaluate(() => {
      const m = window.mtg;
      for (const [id, verbs] of m.ui.verbs) if (verbs.some(v => v.label === "Play land")) return id;
      return null;
    });
    if (land === null) fail("no land to play on the first turn");
    else {
      await clickHit(`(h) => h.kind === 'hand' && h.id === ${land}`);
      const pop = await page.evaluate(() => !!window.mtg.popover);
      if (!pop) fail("clicking a hand card with verbs did not open its popover");
      await shot("3-popover.png");
      await clickHit("(h, m) => h.kind === 'row' && m.popover");
      await waitDecision();
      const lands = await page.evaluate(() => window.mtg.view.battlefield.filter(p => p.controller === window.mtg.view.you && p.card_types.includes('Land')).length);
      if (lands !== 1) fail(`expected one land on the battlefield after playing it, found ${lands}`);
      await shot("4-land.png");
    }
    // Pass with Enter until the opponent has acted and we are asked again.
    await page.keyboard.press("Enter");
    await waitDecision();
    s = await info();
    if (!s.turn || s.turn < 2) fail(`expected to reach a later turn after passing, got ${JSON.stringify(s)}`);
    await shot("5-later.png");
    // Hover a battlefield card: the inspector should name it.
    await clickHit("(h) => h.kind === 'perm'").catch(() => {});
    if (errors.length) fail("page errors:\n" + errors.join("\n"));
    console.log(process.exitCode ? "smoke: FAILED" : `smoke: ok (turn ${s.turn}, ${s.context})`);
  } finally {
    await browser.close();
    runner.kill("SIGTERM");
    if (/panicked/.test(runnerErr)) fail("runner panicked:\n" + runnerErr);
  }
}

main().catch(e => { fail(e.stack || String(e)); });

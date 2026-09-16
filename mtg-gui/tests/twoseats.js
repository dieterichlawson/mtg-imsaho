// Two humans, two tabs: `--p1 gui --p2 gui`, one page per seat.
//
//   cargo build -p mtg-runner
//   NODE_PATH=$(npm root -g) node mtg-gui/tests/twoseats.js
//
// Both pages keep their opening hands, then whichever page is asked
// answers (a land if it can, else a pass) for a few dozen decisions.
// Checks: each page is asked in turn and never both at once; a page only
// ever sees its own hand; the seats took consecutive ports.

const { chromium } = require("playwright");
const { spawn } = require("child_process");
const path = require("path");

const root = path.resolve(__dirname, "..", "..");
function fail(msg) { console.error("FAIL: " + msg); process.exitCode = 1; }

async function main() {
  const port = 8600 + Math.floor(Math.random() * 90);
  const runner = spawn(path.join(root, "target", "debug", "mtg-runner"),
    ["--p1", `gui:${port}`, "--p2", `gui:${port + 1}`, "--seed", "21", "--deck1", "red-green", "--deck2", "blue-white", "-q"],
    { cwd: root, stdio: ["ignore", "ignore", "pipe"] });
  let err = "";
  runner.stderr.on("data", d => { err += d; });
  const browser = await chromium.launch();
  try {
    const pages = [];
    for (const p of [port, port + 1]) {
      const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
      page.on("pageerror", e => fail(`page ${p}: ${e.message}`));
      for (let i = 0; i < 40; i++) {
        try { await page.goto(`http://127.0.0.1:${p}/`); break; } catch (e) { await page.waitForTimeout(250); }
      }
      pages.push(page);
    }
    const asked = async (page) => page.evaluate(() => !!(window.mtg && window.mtg.decision));
    const seatOf = async (page) => page.evaluate(() => window.mtg.view && window.mtg.view.you);
    let answered = 0;
    const answeredBy = [0, 0];
    for (let round = 0; round < 60 && answered < 40; round++) {
      // Whichever page is asked answers; both at once is the failure.
      await Promise.race(pages.map(p => p.waitForFunction(() => window.mtg && (window.mtg.decision || window.mtg.gameOver), null, { timeout: 30000 }).catch(() => null)));
      const flags = await Promise.all(pages.map(asked));
      if (flags[0] && flags[1]) { fail("both pages asked at once"); break; }
      const i = flags[0] ? 0 : flags[1] ? 1 : -1;
      if (i < 0) { const over = await pages[0].evaluate(() => window.mtg.gameOver); if (over) break; continue; }
      const page = pages[i];
      const seat = await seatOf(page);
      if (seat !== i) fail(`page ${i} is seat ${seat}`);
      // The other page must not know this hand.
      const hand = await page.evaluate(() => window.mtg.view.your_hand.map(c => c.name));
      const otherSees = await pages[1 - i].evaluate((names) => {
        const v = window.mtg.view; if (!v) return [];
        const text = JSON.stringify(v.your_hand);
        return names.filter(n => text.includes(n));
      }, hand);
      // Names can coincide between decks (basic lands), so only flag a
      // full match of a hand with more than lands in it.
      const nonLand = hand.filter(n => !/^(Forest|Mountain|Island|Plains|Swamp)$/.test(n));
      if (nonLand.length && nonLand.every(n => otherSees.includes(n))) fail(`page ${1 - i} sees page ${i}'s hand: ${nonLand.join(", ")}`);
      await page.evaluate(() => {
        const m = window.mtg; const ui = m.ui;
        if (!ui) return;
        if (ui.mode === "list") { const keep = ui.rows.find(r => /Keep/.test(r.label)); (keep || ui.rows[0]).run(); return; }
        if (ui.mode === "menu") {
          for (const [, verbs] of ui.verbs) { const v = verbs.find(x => x.label === "Play land"); if (v) { v.run(); return; } }
          if (ui.canPass) window.mtgSend("PassPriority"); else if (ui.rows && ui.rows[0]) ui.rows[0].run();
          return;
        }
        if (ui.onConfirm) { ui.onConfirm(); return; }
        if (ui.rows && ui.rows[0]) ui.rows[0].run();
      });
      answered++; answeredBy[i]++;
      await page.waitForTimeout(50);
    }
    if (answeredBy[0] === 0 || answeredBy[1] === 0) fail(`answers by seat: ${answeredBy.join("/")}`);
    console.log(process.exitCode ? "twoseats: FAILED" : `twoseats: ok (${answeredBy[0]} + ${answeredBy[1]} decisions on ports ${port} and ${port + 1})`);
  } finally {
    await browser.close();
    runner.kill("SIGTERM");
    if (/panicked/.test(err)) fail("runner panicked:\n" + err);
  }
}

main().catch(e => { fail(e.stack || String(e)); });

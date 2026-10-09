// A match's next game comes down on the same page: the game-over box of
// the last game does not stay up over it.
//
//   cargo build -p mtg-runner
//   NODE_PATH=$(npm root -g) node mtg-gui/tests/next_game.js
//
// `mtg-draft-server` plays a best-of-3 for a person through one
// `GuiPlayer`, so game 2's first message lands on the page that just
// showed "GAME OVER". The page cleared nothing on it: the box stayed over
// game 2's board until a click, the panel kept saying the last game was
// over, and a test driver reading `window.mtg.gameOver` thought every
// decision of game 2 was game 1 ending again (found in the first playtest
// of docs/plans/draft-with-friends.md). A real runner serves the page and
// the first decision; the rest of the match is fed through the debug hook,
// because `mtg-runner` plays one game and the box is the page's to clear.

const { chromium } = require("playwright");
const { spawn } = require("child_process");
const path = require("path");

const root = path.resolve(__dirname, "..", "..");
const port = 8740 + Math.floor(Math.random() * 40);
let failures = 0;
const fail = (m) => { console.error("FAIL: " + m); failures++; };
const ok = (m) => console.log("ok: " + m);
const check = (c, m) => { if (c) ok(m); else fail(m); };

async function main() {
  const runner = spawn(path.join(root, "target", "debug", "mtg-runner"),
    ["--p1", `gui:${port}`, "--p2", "random", "--seed", "5", "--deck1", "red-green", "--deck2", "white-black", "-q"],
    { cwd: root, stdio: ["ignore", "ignore", "pipe"] });
  let err = "";
  runner.stderr.on("data", d => { err += d; });
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
    const errors = [];
    page.on("pageerror", e => errors.push("pageerror: " + e.message));
    for (let i = 0; i < 40; i++) {
      try { await page.goto(`http://127.0.0.1:${port}/`); break; } catch (e) { await page.waitForTimeout(250); }
    }
    await page.waitForFunction(() => window.mtg && window.mtg.decision, null, { timeout: 30000 });

    // Game 1 ends: the box is up and the panel says who won.
    const r = await page.evaluate(() => {
      const m = window.mtg;
      m.gameOverBefore = null;
      window.mtgDebug.message({ type: "game_over", seat: m.view.you, view: m.view, summary: "Game over! Seat 1 wins.\nFinal turn: 9" });
      window.mtgDebug.render();
      return { gameOver: m.gameOver, decision: !!m.decision, hits: m.hits.length };
    });
    check(r.gameOver === "Game over! Seat 1 wins.\nFinal turn: 9" && !r.decision, `game over is shown and nothing is asked (${JSON.stringify(r)})`);

    // Game 2's first message: a view, then its first decision.
    const after = await page.evaluate(() => {
      const m = window.mtg;
      const view = m.view;
      window.mtgDebug.message({ type: "view", seat: view.you, view });
      const afterView = { gameOver: m.gameOver, notice: m.notice };
      // Pass-or-concede is auto-passed unless the seat stops at passes;
      // the test wants the decision held, as a person would see it.
      m.stopAtPass = true;
      window.mtgDebug.message({ type: "decision", seat: view.you, seq: 9001, view, legal: { actions: ["PassPriority", "Concede"], context: "MAIN PHASE 1" }, combat: null });
      window.mtgDebug.render();
      return { afterView, gameOver: m.gameOver, dismissed: m.gameOverDismissed, notice: m.notice, decision: m.decision && m.decision.seq, mode: m.ui && m.ui.mode, buttons: m.hits.filter(h => h.kind === "button" && h.onClick).map(h => h.label) };
    });
    check(after.afterView.gameOver === null, `the next game's first view takes the box down (gameOver=${JSON.stringify(after.afterView.gameOver)})`);
    check((after.afterView.notice || "").includes("Seat 1 wins") && (after.afterView.notice || "").includes("next game"), `and the last result stays readable as the notice (${JSON.stringify(after.afterView.notice)})`);
    check(after.gameOver === null && !after.dismissed, `the first decision of the next game is not under a box (gameOver=${JSON.stringify(after.gameOver)})`);
    check(after.decision === 9001 && after.mode === "menu", `and is offered as a decision (seq ${after.decision}, ${after.mode})`);
    check(after.buttons.includes("Concede"), `with the widget's buttons clickable (${after.buttons.join(", ")})`);

    // A click answers the new decision rather than dismissing a box: the
    // page's own `sent` log records it.
    const sentBefore = await page.evaluate(() => window.mtgDebug.sent.length);
    await page.keyboard.press("Enter");
    await page.waitForTimeout(100);
    const sentAfter = await page.evaluate(() => window.mtgDebug.sent.length);
    check(sentAfter > sentBefore, `Enter answers the next game's decision (${sentAfter - sentBefore} sent)`);

    // The last message of a real game: game over again, box up again.
    const again = await page.evaluate(() => {
      const m = window.mtg;
      window.mtgDebug.message({ type: "game_over", seat: m.view.you, view: m.view, summary: "Game over! Seat 0 wins." });
      return m.gameOver;
    });
    check(again === "Game over! Seat 0 wins.", `the next game's own end puts the box back up`);
    if (errors.length) fail(`page errors:\n  ${errors.join("\n  ")}`); else ok("no page errors");
    await page.close();
  } finally {
    await browser.close();
    runner.kill("SIGTERM");
  }
  if (/panicked/.test(err)) fail(`runner: ${err.slice(-500)}`);
  if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
  console.log("next game: all checks passed");
}

main().catch(e => { console.error(e); process.exit(1); });

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
const { startRunner, finish } = require("./children.js");

let failures = 0;
const fail = (m) => { console.error("FAIL: " + m); failures++; };
const ok = (m) => console.log("ok: " + m);
const check = (c, m) => { if (c) ok(m); else fail(m); };

async function main() {
  const { runner, urls, output } = await startRunner(
    ["--p1", "gui:0", "--p2", "random", "--seed", "5", "--deck1", "red-green", "--deck2", "white-black", "-q"]);
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
    const errors = [];
    page.on("pageerror", e => errors.push("pageerror: " + e.message));
    await page.goto(urls["--p1"]);
    await page.waitForFunction(() => window.mtg && window.mtg.decision, null, { timeout: 30000 });

    // Game 1 ends: the box is up and the panel says who won.
    const r = await page.evaluate(() => {
      const m = window.mtg;
      m.gameOverBefore = null;
      window.mtgDebug.message({ type: "game_over", seat: m.view.you, view: m.view, summary: "Game over! p1 (Seat 1) wins! (p0 (Seat 0) conceded)\nFinal turn: 9" });
      window.mtgDebug.render();
      return { gameOver: m.gameOver, decision: !!m.decision, hits: m.hits.length,
        headline: window.mtgDebug.outcome(m.gameOver), you: m.view.you };
    });
    // The draft runner's match loop sends mtg-runner's headline (#743): the
    // board's p-number, the table's seat, and how the loser lost.
    check(r.headline === (r.you === 1 ? "YOU WIN" : "OPPONENT WINS"), `the hosted table's headline is read (${JSON.stringify(r.headline)})`);
    check(r.gameOver === "Game over! p1 (Seat 1) wins! (p0 (Seat 0) conceded)\nFinal turn: 9" && !r.decision, `game over is shown and nothing is asked (${JSON.stringify(r)})`);

    // Between games the table says what to do before the next one, and the
    // page draws it while the box is still up; before a first game there is
    // no board, and the line is drawn under the waiting line (#879).
    const drawn = await page.evaluate(() => {
      const said = [];
      const fill = CanvasRenderingContext2D.prototype.fillText;
      CanvasRenderingContext2D.prototype.fillText = function (t, ...rest) { said.push(String(t)); return fill.call(this, t, ...rest); };
      const m = window.mtg;
      // The longest line the server sends (server.rs, between_games).
      const line = "Before game 3: game 2 was a draw: sideboard and choose play or draw where you drafted";
      window.mtgDebug.message({ type: "table", match_line: line, opponent: null });
      window.mtgDebug.render();
      const between = said.join(" ");
      said.length = 0;
      const view = m.view;
      m.view = null;
      window.mtgDebug.message({ type: "table", match_line: "Before game 1: you won the coin flip: choose play or draw at the server's terminal", opponent: null });
      window.mtgDebug.render();
      const first = said.join(" ");
      m.view = view;
      CanvasRenderingContext2D.prototype.fillText = fill;
      window.mtgDebug.render();
      return { between, first };
    });
    const squash = (t) => t.replace(/\s+/g, " ");
    // With the log open the panel is 100px tall: the table's line and a
    // long result share it, and the result does not run into the log.
    const fit = await page.evaluate(() => {
      const said = [];
      const fill = CanvasRenderingContext2D.prototype.fillText;
      CanvasRenderingContext2D.prototype.fillText = function (t, x, y, ...rest) { said.push({ t: String(t), x, y, font: this.font, color: this.fillStyle }); return fill.call(this, t, x, y, ...rest); };
      const m = window.mtg;
      const summary = m.gameOver;
      m.gameOver = "Game over! p1 (Seat 1) wins! (p0 (Seat 0) conceded) after a long and winding game with many, many turns in it\nFinal turn: 29\nand more lines\nand more\nand more";
      m.logOpen = true;
      window.mtgDebug.render();
      CanvasRenderingContext2D.prototype.fillText = fill;
      m.logOpen = false; m.gameOver = summary;
      window.mtgDebug.render();
      const head = said.find(d => /PressStart/.test(d.font) && /WIN|OVER|DRAW/.test(d.t));
      // The panel's column (render.ts PANEL_X), not the board's.
      const result = said.filter(d => d.color === "#ffe080" && d.x >= 480);
      return { top: head ? head.y - 6 : null, bottom: Math.max(...result.map(d => d.y)) };
    });
    check(fit.top !== null && fit.bottom + 9 <= fit.top + 100, `with the log open the result stays in its panel (last line at ${fit.bottom}, panel ${fit.top}-${fit.top + 100})`);
    // Every word of it, the place to go last (#879's review: it was cut
    // at three lines, before "where you drafted").
    check(squash(drawn.between).includes("Before game 3: game 2 was a draw: sideboard and choose play or draw where you drafted"),
      `after a game the panel says what comes before the next (${JSON.stringify(drawn.between.slice(0, 200))})`);
    check(squash(drawn.first).includes("Before game 1: you won the coin flip: choose play or draw at the server's terminal"),
      `before the first game the page says what the table needs (${JSON.stringify(drawn.first)})`);

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
    check((after.afterView.notice || "").includes("(Seat 1)") && (after.afterView.notice || "").includes("next game"), `and the last result stays readable as the notice (${JSON.stringify(after.afterView.notice)})`);
    check(!/\bp[01]\b/.test(after.afterView.notice || ""), `in the page's words, not p0/p1 (${JSON.stringify(after.afterView.notice)})`);
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
      window.mtgDebug.message({ type: "game_over", seat: m.view.you, view: m.view, summary: "Game over! p0 (Seat 0) wins!" });
      return m.gameOver;
    });
    check(again === "Game over! p0 (Seat 0) wins!", `the next game's own end puts the box back up`);
    if (errors.length) fail(`page errors:\n  ${errors.join("\n  ")}`); else ok("no page errors");
    await page.close();
  } finally {
    await browser.close();
    runner.kill("SIGTERM");
  }
  if (/panicked/.test(output())) fail(`runner: ${output().slice(-500)}`);
  if (failures) { console.error(`${failures} failure(s)`); process.exitCode = 1; return; }
  console.log("next game: all checks passed");
}

finish(main);

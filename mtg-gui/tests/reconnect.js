// A gui seat whose browser goes away says so, and hands the same decision
// back to the next one.
//
//   cargo build -p mtg-runner
//   NODE_PATH=$(npm root -g) node mtg-gui/tests/reconnect.js
//
// Issue #602: waiting for ever is the right policy for a human seat — the
// reconnect works, and somebody who closed a tab by accident wants their
// game back rather than a forfeit — but the seat did it in complete silence.
// `said_waiting` latched at the first decision, before anyone had opened the
// page at all, so the one line naming the URL was never printed again; and
// `connected()` could not have noticed the page leaving anyway, because the
// only thing that reaped a dead client was `broadcast`, and nothing is
// broadcast while the seat is blocked waiting for an answer.
//
// So: the line comes back, the process does not exit, and the next browser
// is handed the decision the last one was holding.

const { chromium } = require("playwright");
const { spawn } = require("child_process");
const path = require("path");

const root = path.resolve(__dirname, "..", "..");
const port = 8790 + Math.floor(Math.random() * 9);
let failures = 0;
const fail = (m) => { console.error("FAIL: " + m); failures++; };
const ok = (m) => console.log("ok: " + m);
const sleep = (ms) => new Promise(r => setTimeout(r, ms));

async function main() {
  const runner = spawn(path.join(root, "target", "debug", "mtg-runner"),
    ["--p1", `gui:${port}`, "--p2", "random", "--seed", "5",
     "--deck1", "red-green", "--deck2", "white-black", "-q"],
    { cwd: root, stdio: ["ignore", "pipe", "pipe"] });
  let out = "";
  runner.stdout.on("data", d => { out += d; });
  runner.stderr.on("data", d => { out += d; });
  let exited = null;
  runner.on("exit", (code, sig) => { exited = { code, sig }; });
  // The seat says where its page is whenever nobody is at it; count the
  // times it has said so.
  const said = () => (out.match(/no browser at/g) || []).length;

  let browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
    for (let i = 0; i < 40; i++) {
      try { await page.goto(`http://127.0.0.1:${port}/`); break; } catch (e) { await sleep(250); }
    }
    await page.waitForFunction(() => window.mtg && window.mtg.decision, null, { timeout: 30000 });
    if (said() < 1) fail(`the seat never said where its page was before anyone opened it:\n${out}`);
    else ok(`before connecting, the seat named its URL (${said()}x)`);

    // Answer one, so the seat is blocked in `ask` on the next decision.
    await page.evaluate(() => window.mtgSend("MulliganKeep"));
    await page.waitForFunction(() => window.mtg && window.mtg.decision, null, { timeout: 30000 });
    const holding = await page.evaluate(() => ({ seq: window.mtg.decision.seq, mode: window.mtg.ui.mode }));
    ok(`answered one; the seat is holding ${JSON.stringify(holding)}`);

    const before = said();
    await browser.close();
    browser = null;
    let now = before;
    for (let i = 0; i < 60 && now === before; i++) { await sleep(500); now = said(); }
    if (now === before) fail(`30s after the browser closed, the seat has said nothing (still ${before}):\n${out}`);
    else ok(`the browser went away and the seat named its URL again (${before} → ${now})`);
    if (exited) fail(`the seat gave up on the game (exit ${JSON.stringify(exited)}) — waiting is the policy`);
    else ok("and it is still waiting, not gone");

    // The half #602 confirmed already works, now guarded: the next browser
    // is handed the decision the last one was holding.
    browser = await chromium.launch();
    const p2 = await browser.newPage({ viewport: { width: 1280, height: 720 } });
    await p2.goto(`http://127.0.0.1:${port}/`);
    const got = await p2.waitForFunction(() => window.mtg && window.mtg.decision, null, { timeout: 20000 })
      .then(() => p2.evaluate(() => ({ seq: window.mtg.decision.seq, mode: window.mtg.ui.mode })))
      .catch(e => ({ error: String(e).slice(0, 120) }));
    if (JSON.stringify(got) !== JSON.stringify(holding)) fail(`the reconnect got ${JSON.stringify(got)}, not ${JSON.stringify(holding)}`);
    else ok(`the reconnect is handed the same decision ${JSON.stringify(got)}`);
  } finally {
    if (browser) await browser.close();
    runner.kill("SIGKILL");
  }
  console.log(failures ? `reconnect: ${failures} FAILED` : "reconnect: ok");
  process.exitCode = failures ? 1 : 0;
}

main().catch(e => { fail(e.stack || String(e)); process.exitCode = 1; });

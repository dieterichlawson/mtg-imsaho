// Two tabs on ONE seat: what the seat decides, and what the other tab is
// left holding.
//
//   cargo build -p mtg-runner
//   NODE_PATH=$(npm root -g) node mtg-gui/tests/twotabs.js
//
// `mtg-gui/README.md` makes two pages on one seat a supported shape — it
// is what you get if you reopen without closing, or open the page on a
// second screen — and every connected page is sent every decision. Two
// things follow that were wrong:
//
//   - `s` ("stop at every priority") governs whether a page answers a
//     priority FOR the player, and it was per page. A second tab passed
//     the priorities the first was deliberately holding (issue #515).
//   - Only one page's answer lands. The others kept the answered prompt
//     live and clickable, with no message saying it had been taken, so a
//     person at the second tab could mulligan a hand that was already
//     kept and have the click read as accepted (issue #516).

const { chromium } = require("playwright");
const { startRunner, finish } = require("./children.js");

let failures = 0;
function fail(msg) { console.error("FAIL: " + msg); failures++; }
function ok(msg) { console.log("ok: " + msg); }

async function openTab(browser, url) {
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
  page.on("pageerror", e => fail(`page error: ${e.message}`));
  await page.goto(url, { timeout: 15000 });
  await page.waitForFunction(() => window.mtg && window.mtg.connected, null, { timeout: 20000 });
  return page;
}

const snap = (p) => p.evaluate(() => ({
  seq: window.mtg.decision && window.mtg.decision.seq,
  widget: window.mtg.ui && window.mtg.ui.mode,
  notice: window.mtg.notice,
  stopAtPass: !!window.mtg.stopAtPass,
  sent: window.mtgDebug.sent.map(s => s.seq),
}));

async function main() {
  const { runner, urls } = await startRunner(
    ["--p1", "gui:0", "--p2", "random", "--seed", "4401",
     "--deck1", "decks/gw-humans.txt", "--deck2", "decks/ub-zombies.txt", "--check-invariants", "-q"]);
  const url = urls["--p1"];
  const browser = await chromium.launch();
  try {
    const A = await openTab(browser, url);
    await A.waitForFunction(() => window.mtg && window.mtg.decision, null, { timeout: 30000 });

    // ---- #515: `s` belongs to the seat, not to a page ----
    await A.keyboard.press("s");
    await A.waitForTimeout(200);
    const B = await openTab(browser, url);
    await B.waitForFunction(() => window.mtg && window.mtg.decision, null, { timeout: 30000 });
    await B.waitForTimeout(300);
    const bStops = await B.evaluate(() => !!window.mtg.stopAtPass);
    if (!bStops) fail("stop-at-pass: a tab joining a seat that is stopping at every priority did not learn it");
    else ok("stop-at-pass: the second tab joined a seat that is stopping, and knows it");

    // Keep, then pass the main phase. Every priority after that is one the
    // engine offers only Pass/Concede at — the ones `s` exists to stop at.
    await A.evaluate(() => { const ui = window.mtg.ui; (ui.rows.find(r => /Keep/i.test(r.label)) || ui.rows[0]).run(); });
    await A.waitForTimeout(800);
    await A.evaluate(() => { if (window.mtg.decision) window.mtgSend("PassPriority"); });
    await A.waitForTimeout(3000);

    const aTrace = await A.evaluate(() => window.mtgDebug.trace);
    const bSent = await B.evaluate(() => window.mtgDebug.sent);
    const autoPassed = aTrace.filter(t => /only pass$/.test(t)).length;
    if (bSent.length) fail(`stop-at-pass: the second tab answered ${JSON.stringify(bSent)} while the first was holding`);
    else ok("stop-at-pass: the second tab answered nothing");
    if (autoPassed) fail(`stop-at-pass: the first tab auto-passed ${autoPassed} priorities after pressing s`);
    else ok("stop-at-pass: neither tab auto-passed a priority the seat was holding");

    // Both tabs are still on the same decision, which is the state the
    // next half is about.
    const a1 = await snap(A), b1 = await snap(B);
    if (a1.seq === null || a1.seq !== b1.seq)
      fail(`two-tabs: the tabs hold different decisions (${a1.seq} vs ${b1.seq})`);
    else ok(`two-tabs: both tabs hold decision ${a1.seq}`);

    // ---- #516: the tab that did not answer is told ----
    await A.evaluate(() => {
      const ui = window.mtg.ui;
      const b = (ui.buttons || []).find(x => x.run && x.primary) || (ui.buttons || []).find(x => x.run);
      if (b) b.run(); else window.mtgSend("PassPriority");
    });
    await A.waitForTimeout(1200);
    const b2 = await snap(B);
    if (b2.sent.includes(a1.seq))
      fail(`stale-prompt: the other tab answered ${a1.seq} itself, so this proves nothing about being told`);
    else if (b2.seq === a1.seq)
      fail(`stale-prompt: the other tab still offers decision ${b2.seq} after it was answered`);
    else ok("stale-prompt: the other tab stopped offering the answered decision without answering it");
    if (!b2.notice || !/another tab/i.test(b2.notice))
      fail(`stale-prompt: the other tab said ${JSON.stringify(b2.notice)}, not that it was answered elsewhere`);
    else ok(`stale-prompt: the other tab says "${b2.notice}"`);

    // And the tab that DID answer is not told its own answer happened
    // somewhere else.
    const a2 = await snap(A);
    if (a2.notice && /another tab/i.test(a2.notice))
      fail(`stale-prompt: the answering tab was told "${a2.notice}"`);
    else ok("stale-prompt: the answering tab is not told about its own answer");
  } finally {
    await browser.close();
    runner.kill("SIGKILL");
  }
  console.log(failures ? `twotabs: ${failures} FAILED` : "twotabs: ok");
  process.exitCode = failures ? 1 : 0;
}

finish(main);

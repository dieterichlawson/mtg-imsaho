// Play whole games through the page by clicking, at random.
//
//   cargo build -p mtg-runner
//   NODE_PATH=$(npm root -g) node mtg-gui/tests/autoplay.js [--games N] [--seed S] [--decisions D] [--shots DIR] [--deck1 X --deck2 Y]
//
// The fuzzer for the fourth surface. For each decision the page shows, a
// random legal-looking interaction is performed with the mouse and
// keyboard exactly as a person would: a card is clicked and a verb chosen,
// targets are clicked, sets are marked and confirmed, attackers and
// blockers declared, lists chosen from, X typed. A decision the page
// offers no way to answer is the failure this exists to find: after three
// tries with nothing sent, the run fails with a screenshot and the
// widget's state. Page errors fail the run too. Every prompt kind reached
// is counted, so a run's summary says what was exercised.

const { chromium } = require("playwright");
const { spawn } = require("child_process");
const path = require("path");
const fs = require("fs");

const root = path.resolve(__dirname, "..", "..");
const arg = (name, def) => { const i = process.argv.indexOf(name); return i >= 0 ? process.argv[i + 1] : def; };
const games = Number(arg("--games", "1"));
const seed0 = Number(arg("--seed", String(Math.floor(Math.random() * 100000))));
const maxDecisions = Number(arg("--decisions", "400"));
const shotsDir = arg("--shots", null);
const decksArg = [arg("--deck1", null), arg("--deck2", null)];
const DECKS = ["red-green", "white-black", "blue-white", "black-aggro", "innistrad-white", "innistrad-blue", "innistrad-green",
  "decks/coverage/ub-coverage.txt", "decks/coverage/rg-coverage.txt", "decks/coverage/wb-coverage.txt", "decks/coverage/ug-coverage.txt",
  "decks/coverage/br-coverage.txt", "decks/coverage/wu-coverage.txt", "decks/coverage/wg-coverage.txt", "decks/coverage/ur-coverage.txt"];

let rngState = seed0 * 2654435761 >>> 0;
function rnd() { rngState = (rngState * 1664525 + 1013904223) >>> 0; return rngState / 4294967296; }
function pick(arr) { return arr[Math.floor(rnd() * arr.length)]; }

function fail(msg) { console.error("FAIL: " + msg); process.exitCode = 1; }

async function playOne(browser, seed, deck1, deck2, kinds) {
  const port = 8800 + Math.floor(rnd() * 90);
  const runner = spawn(path.join(root, "target", "debug", "mtg-runner"),
    ["--p1", `gui:${port}`, "--p2", "random", "--seed", String(seed), "--deck1", deck1, "--deck2", deck2, "-q", "--check-invariants"],
    { cwd: root, stdio: ["ignore", "pipe", "pipe"] });
  let runnerOut = "";
  runner.stdout.on("data", d => { runnerOut += d; });
  runner.stderr.on("data", d => { runnerOut += d; });
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
  const errors = [];
  page.on("pageerror", e => errors.push("pageerror: " + e.message));
  page.on("console", m => { if (m.type() === "error" && !/404/.test(m.text())) errors.push("console: " + m.text()); });
  const label = `seed ${seed} ${deck1} vs ${deck2}`;
  const shot = async (name) => { if (shotsDir) { fs.mkdirSync(shotsDir, { recursive: true }); await page.screenshot({ path: path.join(shotsDir, `${seed}-${name}.png`) }); } };
  let decisions = 0;
  let outcome = "unfinished";
  try {
    for (let i = 0; i < 40; i++) {
      try { await page.goto(`http://127.0.0.1:${port}/`); break; } catch (e) { await page.waitForTimeout(250); }
    }
    let lastSeq = -1, stuck = 0, committed = false;
    while (decisions < maxDecisions) {
      const ready = await page.waitForFunction(() => window.mtg && (window.mtg.decision || window.mtg.gameOver), null, { timeout: 60000 }).catch(() => null);
      if (!ready) { fail(`${label}: no decision for 60s at decision ${decisions}`); await shot("hang"); break; }
      const snap = await page.evaluate(() => {
        const m = window.mtg;
        if (m.gameOver) return { over: m.gameOver };
        const ui = m.ui;
        const kind = m.decision.combat ? "combat:" + Object.keys(m.decision.combat)[0]
          : m.decision.legal.resolution_prompt ? "resolution:" + Object.keys(m.decision.legal.resolution_prompt)[0]
          : m.decision.legal.set_prompt ? "set_prompt" : "menu:" + (m.decision.legal.context || "");
        const clickable = (k) => m.hits.filter(h => h.onClick && (!k || k.includes(h.kind))).map(h => ({ x: h.x + h.w / 2, y: h.y + h.h / 2, kind: h.kind, label: h.label || null, key: h.key || null }));
        return {
          seq: m.decision.seq, kind, mode: ui && ui.mode, title: ui && ui.title, turn: m.view.turn_number, step: m.view.step,
          options: clickable(["perm", "hand", "player", "stack", "card"]), rows: clickable(["row"]), buttons: clickable(["button"]),
          popover: !!m.popover, min: ui && ui.min, max: ui && ui.max, marked: ui ? ui.marked.length : 0, canPass: !!(ui && ui.canPass),
          verbs: ui && ui.verbs ? [...ui.verbs.keys()] : [], hasCancel: !!(ui && ui.onCancel), maxX: ui && ui.mode === "number" ? ui.max : null,
          notice: m.notice, lastSent: m.lastSent ? JSON.stringify(m.lastSent.action) : null,
        };
      });
      if (snap.over) { outcome = snap.over.split("\n")[0]; break; }
      if (snap.seq === lastSeq) { if (committed) stuck++; } else { stuck = 0; lastSeq = snap.seq; decisions++; kinds.set(snap.kind.split(":")[0] + ":" + snap.kind.split(":")[1], (kinds.get(snap.kind.split(":")[0] + ":" + snap.kind.split(":")[1]) || 0) + 1); }
      if (stuck >= 3) {
        fail(`${label}: stuck at decision ${decisions} (${snap.kind}, widget ${snap.mode}): ${JSON.stringify({ title: snap.title, options: snap.options.length, rows: snap.rows.length, buttons: snap.buttons.map(b => b.label), notice: snap.notice, lastSent: snap.lastSent })}`);
        await shot(`stuck-${decisions}`);
        break;
      }
      if (shotsDir && decisions % 25 === 0 && stuck === 0) await shot(`d${decisions}-t${snap.turn}`);
      committed = await step(page, snap);
      await page.waitForTimeout(40);
    }
    if (errors.length) fail(`${label}: page errors:\n` + errors.join("\n"));
    if (/INVARIANT|panicked/.test(runnerOut)) fail(`${label}: runner:\n` + runnerOut.slice(-1500));
  } finally {
    await page.close();
    runner.kill("SIGTERM");
  }
  console.log(`${label}: ${decisions} decisions, ${outcome}`);
  return decisions;
}

const click = async (page, h) => { await page.mouse.move(h.x * 2, h.y * 2); await page.mouse.click(h.x * 2, h.y * 2); await page.waitForTimeout(60); };
const clickButton = async (page, snap, pred) => { const b = snap.buttons.find(pred); if (b) { await click(page, b); return true; } return false; };
const clickableNow = (page, kinds) => page.evaluate((kinds) => window.mtg.hits.filter(h => h.onClick && kinds.includes(h.kind)).map(h => ({ x: h.x + h.w / 2, y: h.y + h.h / 2, kind: h.kind, key: h.key || null, label: h.label || null })), kinds);

/**
 * One interaction for the widget on screen. Returns true when the step
 * should have answered the decision (so an unchanged seq afterwards is
 * the page's failure, not the driver dithering).
 */
async function step(page, s) {
  const primary = (b) => /^(Confirm|Attack|No attack|Block|No block)/.test(b.label || "");
  switch (s.mode) {
    case "menu": {
      if (s.popover) { if (s.rows.length) { await click(page, pick(s.rows)); return true; } await page.keyboard.press("Escape"); return false; }
      const withVerbs = s.options.filter(o => o.kind === "perm" || o.kind === "hand");
      if (withVerbs.length && rnd() < 0.7) { await click(page, pick(withVerbs)); return false; }
      if (s.rows.length && rnd() < 0.5) { await click(page, pick(s.rows)); return true; }
      if (s.canPass) { await page.keyboard.press("Enter"); return true; }
      if (withVerbs.length) { await click(page, pick(withVerbs)); return false; }
      if (s.rows.length) { await click(page, pick(s.rows)); return true; }
      await page.keyboard.press("Enter");
      return true;
    }
    case "pick": {
      const opts = [...s.options, ...s.rows];
      const decline = s.buttons.find(b => b.label === "Decline");
      if (s.hasCancel && rnd() < 0.1) { await page.keyboard.press("Escape"); return false; }
      if (decline && rnd() < 0.15) { await click(page, decline); return true; }
      if (opts.length) { await click(page, pick(opts)); return true; }
      if (decline) { await click(page, decline); return true; }
      if (s.hasCancel) { await page.keyboard.press("Escape"); return false; }
      return true;
    }
    case "mark": {
      const want = Math.min(s.max, s.min + Math.floor(rnd() * (s.max - s.min + 1)));
      for (let n = s.marked; n < want; n++) {
        // Only things not yet marked: clicking a marked one unmarks it.
        const marked = await page.evaluate(() => window.mtg.ui.marked.slice());
        const opts = (await clickableNow(page, ["perm", "hand", "player", "stack", "card", "row"])).filter(h => !h.key || !marked.includes(h.key));
        if (!opts.length) break;
        await click(page, pick(opts));
      }
      if (!(await clickButton(page, s, primary))) await page.keyboard.press("Enter");
      return true;
    }
    case "attackers": {
      for (const o of s.options) if (rnd() < 0.6) await click(page, o);
      if (!(await clickButton(page, s, primary))) await page.keyboard.press("Enter");
      return true;
    }
    case "blockers": {
      const attackerKeys = await page.evaluate(() => [...(window.mtg.ui.attackers || [])]);
      const blockers = s.options.filter(o => o.kind === "perm" && !attackerKeys.includes(o.key));
      for (const b of blockers) {
        if (rnd() >= 0.6) continue;
        await click(page, b);
        const targets = (await clickableNow(page, ["perm"])).filter(h => attackerKeys.includes(h.key));
        if (targets.length) await click(page, pick(targets)); else await click(page, b);
      }
      if (!(await clickButton(page, s, primary))) await page.keyboard.press("Enter");
      return true;
    }
    case "list": {
      if (s.rows.length) { await click(page, pick(s.rows)); return true; }
      await page.keyboard.press("Enter");
      return true;
    }
    case "order": {
      if (rnd() < 0.5) { const arrows = s.buttons.filter(b => b.label === "▲" || b.label === "▼"); if (arrows.length) await click(page, pick(arrows)); }
      if (!(await clickButton(page, s, b => b.label === "Confirm"))) await page.keyboard.press("Enter");
      return true;
    }
    case "number": {
      const x = Math.floor(rnd() * ((s.maxX || 0) + 1));
      await page.keyboard.type(String(x));
      await page.keyboard.press("Enter");
      return true;
    }
    default:
      await page.keyboard.press("Enter");
      return true;
  }
}

async function main() {
  const browser = await chromium.launch();
  const kinds = new Map();
  let total = 0;
  try {
    for (let g = 0; g < games; g++) {
      const seed = seed0 + g;
      const deck1 = decksArg[0] || pick(DECKS);
      const deck2 = decksArg[1] || pick(DECKS);
      total += await playOne(browser, seed, deck1, deck2, kinds);
    }
  } finally {
    await browser.close();
  }
  console.log(`\n${games} game(s), ${total} decisions. Prompt kinds reached:`);
  for (const [k, n] of [...kinds].sort()) console.log(`  ${n.toString().padStart(4)}  ${k}`);
  console.log(process.exitCode ? "autoplay: FAILED" : "autoplay: ok");
}

main().catch(e => { fail(e.stack || String(e)); });

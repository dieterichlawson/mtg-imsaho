// A whole hosted table, played: people through the real pages beside AI seats.
//
//   cargo build -p mtg-draft-runner
//   NODE_PATH=$(npm root -g) node mtg-gui/tests/table_rehearsal.js [--humans 2] [--players 8]
//       [--best-of 3] [--ai stub|<spec>] [--seed S] [--out DIR] [--pick-seconds N]
//
// The dress rehearsal for a night with friends: `mtg-draft-server` with
// `--humans` people and the rest AI seats, each person a headless Chromium
// tab on their join link doing what a person does — picking a card by
// clicking its tile twice, building by clicking rows and land steppers and
// Ready, choosing play or draw and Ready between games, and playing every
// game of their matches on the game page, which is clicked through at
// random with `autoplay.js`'s step (and reached through the lobby's one
// port, as a friend reaches it). It runs until the table says the
// tournament is over, then has the host quit.
//
// `--ai stub` (the default) answers the AI seats with
// `scripts/llm-prompt-stub.py`, for free; any other value is the AI seats'
// spec (`cc:claude-haiku-5-5`), which is real model calls on the CLI's own
// login: plan quota, never a metered key. A failure is anything a person
// would have been stuck on or shown wrong: a page error, a refusal the
// page caused, a decision the game page offered no way to answer, a phase
// that never ended. The server's terminal and log stay in `--out`.

const { chromium } = require("playwright");
const { spawnOwned, finish, parseArgs } = require("./children.js");
const path = require("path");
const fs = require("fs");
const { step, snapshot, seedClicks } = require("./autoplay.js");

const root = path.resolve(__dirname, "..", "..");
const args = parseArgs({
  "--humans": { kind: "int", def: 2 },
  "--players": { kind: "int", min: 2, def: 8 },
  "--best-of": { kind: "int", min: 1, def: 3 },
  "--ai": { kind: "string", def: "stub" },
  "--seed": { kind: "int", def: 1 + Math.floor(Math.random() * 100000) },
  "--out": { kind: "string", def: null },
  "--pick-seconds": { kind: "int", min: 1, def: 600 },
  "--hours": { kind: "number", def: 4 },
});
const humans = args["--humans"];
const players = args["--players"];
const bestOf = args["--best-of"];
const ai = args["--ai"];
const seed = args["--seed"];
seedClicks(seed);
const out = path.resolve(args["--out"] ?? path.join(root, "logs", "rehearsal", String(seed)));
const pickSeconds = String(args["--pick-seconds"]);
const port = 18900 + Math.floor(Math.random() * 50);
const deadlineMs = args["--hours"] * 3600 * 1000;

let failures = 0;
function fail(msg) { console.error("FAIL: " + msg); failures++; }
// Set when the table can no longer finish (a deadline, a game page stuck
// and closed, the table closed): every loop below stops on it, or a match
// that cannot end kept node waiting on it forever after its FAIL (#942).
let stopped = null;
const stop = (why) => { if (!stopped) stopped = why; };
const startedAt = Date.now();
const pastDeadline = () => Date.now() - startedAt > deadlineMs;
function log(msg) { console.log(`[${new Date().toISOString().slice(11, 19)}] ${msg}`); }

let rng = seed * 2654435761 >>> 0;
const rnd = () => { rng = (rng * 1664525 + 1013904223) >>> 0; return rng / 4294967296; };

async function main() {
  fs.mkdirSync(out, { recursive: true });
  // The server refuses a zero count, so an all-human (or all-AI) table names one kind.
  const seats = [`${humans}xhuman`, `${players - humans}xai`].filter(s => !s.startsWith("0x")).join(",");
  const env = { ...process.env };
  const args = ["--seats", seats, "--port", String(port), "--game-ports", `${port + 100}-${port + 140}`,
    "--best-of", String(bestOf), "--seed", String(seed), "--log", path.join(out, "draft.log"),
    "--pick-seconds", pickSeconds, "--build-seconds", "900", "--sideboard-seconds", "300"];
  if (ai === "stub") env.CLAUDE_CODE_BIN = path.join(root, "scripts", "llm-prompt-stub.py");
  else args.push("--ai", ai);
  const server = spawnOwned(path.join(root, "target", "debug", "mtg-draft-server"), args,
    { cwd: root, env, stdio: ["pipe", "pipe", "pipe"] });
  let terminal = "";
  server.stderr.on("data", d => { terminal += d; fs.appendFileSync(path.join(out, "terminal.txt"), d); });
  server.stdout.on("data", d => fs.appendFileSync(path.join(out, "terminal.txt"), d));
  let exited = null;
  // Watched for the whole run, not only its start: a server that died
  // mid-draft left every person's page retrying until `--hours` ran out,
  // then "the table never finished" (#943's review).
  let quitting = false, started = false;
  server.on("exit", (code, sig) => {
    exited = sig || code;
    if (started && !quitting) { fail(`the server exited (${exited}) with the table unfinished:\n${terminal.slice(-2000)}`); stop("the server exited"); }
  });
  const began = Date.now();
  while (!/the draft starts when/.test(terminal)) {
    if (exited !== null) { fail(`the server exited (${exited}) before printing its join lines:\n` + terminal); return; }
    if (Date.now() - began > 60000) { fail("the server never printed its join lines:\n" + terminal); server.kill(); return; }
    await new Promise(r => setTimeout(r, 100));
  }
  started = true;
  const links = [...terminal.matchAll(/^seat (\d+)\s+(http:\/\/\S+)/gm)].map(m => ({ seat: Number(m[1]), url: m[2] }));
  log(`table on :${port}, seed ${seed}, ${seats}, best-of-${bestOf}, AI ${ai}; ${links.length} join links`);

  const browser = await chromium.launch();
  const tally = { picks: 0, decisions: 0, games: 0, refusals: [], played: new Map() };
  try {
    await Promise.all(links.map(l => person(browser, l, tally)));
  } finally {
    quitting = true;
    server.stdin.write("quit\n");
    await new Promise(r => setTimeout(r, 3000));
    server.kill("SIGTERM");
    await browser.close();
  }
  const logText = fs.existsSync(path.join(out, "draft.log")) ? fs.readFileSync(path.join(out, "draft.log"), "utf8") : "";
  if (!/FINAL STANDINGS/.test(logText)) fail("the log has no final standings");
  // A table whose people played no game is no rehearsal: a person's match
  // with no game page (its port held by something else) is forfeited, and
  // the standings are written all the same (#941).
  for (const l of players > 1 ? links : []) {
    const n = tally.played.get(l.seat) || 0;
    if (!n) fail(`seat ${l.seat} answered no decision on a game page: its matches were not played`);
  }
  for (const m of terminal.matchAll(/^.*no game page for .*forfeits the match.*$/gm)) fail(`a person forfeited for want of a game page: ${m[0]}`);
  if (/panicked|INVARIANT/.test(terminal + logText)) fail("a panic or an invariant in the run:\n" + terminal.slice(-2000));
  log(`${tally.picks} picks clicked, ${tally.games} game pages, ${tally.decisions} game decisions clicked; ${(Date.now() - began) / 60000 | 0} min`);
  if (tally.refusals.length) log(`refusals the pages showed: ${JSON.stringify(tally.refusals.slice(0, 10))}`);
  console.log(failures ? `table rehearsal: ${failures} failure(s)` : "table rehearsal: ok");
}

/** One person at the table, from the join link to the end of the tournament. */
async function person(browser, link, tally) {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const errors = [];
  page.on("pageerror", e => errors.push("pageerror: " + e.message));
  await page.goto(link.url, { timeout: 15000 });
  await page.fill("#name", `Person ${link.seat}`);
  await page.press("#name", "Enter");
  const games = new Map(); // round -> promise playing that match's game page
  let lastRefusal = null;
  for (;;) {
    if (stopped) break;
    if (pastDeadline()) { fail(`seat ${link.seat}: the table never finished`); stop("the deadline"); break; }
    const v = await page.evaluate(() => window.mtgDraft && window.mtgDraft.view);
    const st = await page.evaluate(() => ({ refusal: window.mtgDraft.state.refusal, closed: window.mtgDraft.state.closed,
      pending: !!window.mtgDraft.state.pendingPick, readySent: window.mtgDraft.state.readySent }));
    if (st.refusal && JSON.stringify(st.refusal) !== lastRefusal) {
      lastRefusal = JSON.stringify(st.refusal);
      tally.refusals.push({ seat: link.seat, reason: st.refusal.reason });
    }
    if (st.closed) { fail(`seat ${link.seat}: the table closed under it: ${st.closed}`); stop("the table closed"); break; }
    if (!v) { await page.waitForTimeout(200); continue; }
    if (v.phase === "done") break;
    if (v.phase === "drafting" && v.pack && !st.pending) {
      const i = v.pack.cards[Math.floor(rnd() * v.pack.cards.length)].index;
      const tile = `.tile[data-card-index="${i}"]`;
      await page.click(tile).catch(() => {});
      await page.click(tile).catch(() => {});
      tally.picks++;
    } else if (v.phase === "building" && !(v.deck && v.deck.ready) && !st.readySent) {
      await build(page, v);
    } else if (v.phase === "playing") {
      for (const m of v.matches) {
        if (m.between && !m.between.ready) {
          if (m.between.you_choose && rnd() < 0.5) await page.click("#draw-first").catch(() => {});
          await page.click("#sideboard-ready").catch(() => {});
        }
        if (m.url && m.status !== "done" && !games.has(m.round)) {
          tally.games++;
          games.set(m.round, playMatch(browser, link.seat, m, page, tally));
        }
      }
    }
    await page.waitForTimeout(150);
  }
  await Promise.all(games.values());
  if (errors.length) fail(`seat ${link.seat}: draft page errors:\n  ${errors.join("\n  ")}`);
  await page.close();
}

/** A deck as a person makes one: the two colours the pool is deepest in,
 *  their cards and the colourless ones to 23, nine and eight basics. */
async function build(page, v) {
  const depth = {};
  for (const c of v.pool) for (const k of c.colors) depth[k] = (depth[k] || 0) + 1;
  const two = Object.entries(depth).sort((a, b) => b[1] - a[1]).slice(0, 2).map(e => e[0]);
  const fits = (c) => c.colors.every(k => two.includes(k));
  const order = v.pool.map((c, i) => ({ c, i })).sort((a, b) => Number(fits(b.c)) - Number(fits(a.c)));
  const inMain = await page.evaluate(() => window.mtgDraft.state.deck ? window.mtgDraft.state.deck.main.slice() : []);
  let count = inMain.filter(Boolean).length;
  for (const { i } of order) {
    if (count >= 23) break;
    if (inMain[i]) continue;
    await page.click(`.row[data-pool-index="${i}"]`).catch(() => {});
    count++;
  }
  const basic = { W: "Plains", U: "Island", B: "Swamp", R: "Mountain", G: "Forest" };
  const lands = [[basic[two[0]] || "Plains", 9], [basic[two[1]] || basic[two[0]] || "Island", 8]];
  for (const [name, n] of lands) {
    const have = await page.evaluate((name) => (window.mtgDraft.state.deck && window.mtgDraft.state.deck.lands[name]) || 0, name);
    for (let k = have; k < n; k++) await page.click(`[data-land="${name}"] > .inc`).catch(() => {});
  }
  await page.waitForTimeout(300);
  await page.click("#ready").catch(() => {});
}

/** One match's games on its game page, clicked through until the draft
 *  page says the match is over. */
async function playMatch(browser, seat, m, draftPage, tally) {
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
  const errors = [];
  page.on("pageerror", e => errors.push("pageerror: " + e.message));
  const label = `seat ${seat} round ${m.round}`;
  try {
    await page.goto(m.url, { timeout: 15000 });
    let lastSeq = -1, stuck = 0, committed = false;
    for (;;) {
      const done = await draftPage.evaluate((round) => {
        const v = window.mtgDraft.view;
        const mm = v && v.matches.find(x => x.round === round);
        return !mm || mm.status === "done";
      }, m.round);
      if (done) break;
      if (stopped) { log(`${label}: left unfinished: ${stopped}`); break; }
      if (pastDeadline()) { fail(`${label}: the match never ended`); stop("the deadline"); break; }
      const ready = await page.waitForFunction(() => window.mtg && window.mtg.decision && !window.mtg.gameOver, null, { timeout: 5000 }).catch(() => null);
      if (!ready) continue;
      const snap = await snapshot(page);
      if (snap.over) continue;
      if (snap.seq === lastSeq) { if (committed) stuck++; } else {
        stuck = 0; lastSeq = snap.seq; tally.decisions++;
        tally.played.set(seat, (tally.played.get(seat) || 0) + 1);
      }
      if (stuck >= 5) {
        fail(`${label}: stuck (${snap.kind}, widget ${snap.mode}): ${JSON.stringify({ title: snap.title, buttons: snap.buttons.map(b => b.label), notice: snap.notice })}`);
        await page.screenshot({ path: path.join(out, `stuck-${seat}-r${m.round}.png`) });
        // Its game now waits for a page that is not coming back.
        stop(`${label} was stuck`);
        break;
      }
      committed = await step(page, snap);
      await page.waitForTimeout(40);
    }
  } catch (e) {
    fail(`${label}: ${e.stack || e}`);
    stop(`${label} failed`);
  } finally {
    if (errors.length) fail(`${label}: game page errors:\n  ${errors.join("\n  ")}`);
    await page.close();
  }
  log(`${label}: match over`);
}

// The exit code is read after every return in main(), so an early FAIL is a
// failure, and the process ends then, the server with it.
finish(main, () => failures ? 1 : (process.exitCode || 0));

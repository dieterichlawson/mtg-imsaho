// The draft page, staged through its debug hook: every phase renders and
// fits, a click picks, the checklist builds a deck, a refusal is shown.
//
//   NODE_PATH=$(npm root -g) node mtg-gui/tests/draft_page.js [--shots DIR]
//
// No draft server is needed: this file serves `mtg-gui/` itself and stages
// the views in `tests/draft-page-fixtures.json` (one per phase, in the
// shapes docs/plans/draft-with-friends.md "Protocol" writes) through
// `window.mtgDraftDebug.stage`, then reads what the page sent from
// `window.mtgDraft.sent`. The server's own fixture file
// (`tests/draft-view-fixtures.json`, written by a Rust test) is staged and
// screenshotted the same way when it exists. Both viewports are checked:
// at 1280x720 a 14-card pack fits one screen, at 390px wide the page
// scrolls down and never sideways.

const { chromium } = require("playwright");
const http = require("http");
const path = require("path");
const fs = require("fs");

const guiDir = path.resolve(__dirname, "..");
const shotsDir = process.argv.includes("--shots") ? process.argv[process.argv.indexOf("--shots") + 1] : null;
let failures = 0;
function fail(msg) { console.error("FAIL: " + msg); failures++; }
function ok(msg) { console.log("ok: " + msg); }
function check(cond, msg) { if (cond) ok(msg); else fail(msg); return cond; }

const MIME = { ".html": "text/html", ".js": "text/javascript", ".json": "application/json", ".png": "image/png", ".ttf": "font/ttf", ".txt": "text/plain" };

/** A static server for the page, like the draft server's GET side. */
function serve() {
  const server = http.createServer((req, res) => {
    const url = new URL(req.url, "http://x");
    let file = url.pathname === "/" ? "draft.html" : url.pathname.slice(1);
    file = path.normalize(file);
    if (file.startsWith("..") || !(file === "draft.html" || file.startsWith("dist/") || file.startsWith("assets/"))) {
      res.writeHead(404); res.end(); return;
    }
    const full = path.join(guiDir, file);
    fs.readFile(full, (err, data) => {
      if (err) { res.writeHead(404); res.end(); return; }
      res.writeHead(200, { "content-type": MIME[path.extname(full)] || "application/octet-stream" });
      res.end(data);
    });
  });
  // No draft to join: the socket is closed on arrival, which is what the
  // page sees when the server is down, and what its backoff is for.
  server.on("upgrade", (req, socket) => { socket.destroy(); });
  return new Promise(resolve => server.listen(0, "127.0.0.1", () => resolve(server)));
}

function loadFixtures() {
  const own = JSON.parse(fs.readFileSync(path.join(__dirname, "draft-page-fixtures.json"), "utf8"));
  const views = Object.entries(own.views).map(([name, view]) => ({ name, view }));
  const serverFile = path.join(__dirname, "draft-view-fixtures.json");
  if (fs.existsSync(serverFile)) {
    const theirs = JSON.parse(fs.readFileSync(serverFile, "utf8"));
    for (const [name, view] of collectViews(theirs)) views.push({ name: `server-${name}`, view });
  }
  return { views, refused: own.refused };
}

/** The views in a fixture file of any of the shapes a writer might pick:
 *  {views: {name: view}}, {name: view}, or [view, ...]. */
function collectViews(data) {
  const out = [];
  const isView = v => v && typeof v === "object" && typeof v.phase === "string";
  if (Array.isArray(data)) data.forEach((v, i) => { if (isView(v)) out.push([v.phase + "-" + i, v]); });
  else if (data && typeof data === "object") {
    const source = data.views && typeof data.views === "object" ? data.views : data;
    for (const [k, v] of Object.entries(source)) if (isView(v)) out.push([k, v]);
  }
  return out;
}

async function main() {
  const server = await serve();
  const port = server.address().port;
  const fixtures = loadFixtures();
  const browser = await chromium.launch();
  try {
    for (const viewport of [{ width: 1280, height: 720 }, { width: 390, height: 844 }]) {
      const page = await browser.newPage({ viewport });
      const errors = [];
      page.on("pageerror", e => errors.push("pageerror: " + e.message));
      page.on("console", m => {
        // The socket is refused on purpose (see `serve`), and a card without
        // art is a placeholder, not an error.
        if (m.type() === "error" && !/WebSocket|404/.test(m.text())) errors.push("console: " + m.text());
      });
      await page.goto(`http://127.0.0.1:${port}/?seat=0&key=test-key`);
      await page.waitForFunction(() => window.mtgDraftDebug && window.mtgDraft, null, { timeout: 15000 });
      const tag = String(viewport.width);
      const shot = async (name) => {
        if (!shotsDir) return;
        fs.mkdirSync(shotsDir, { recursive: true });
        await page.screenshot({ path: path.join(shotsDir, `${name}-${tag}.png`), fullPage: viewport.width < 700 });
      };
      const stage = (msg) => page.evaluate((m) => { window.mtgDraftDebug.stage(m); }, msg);
      const sent = () => page.evaluate(() => window.mtgDraft.sent.map(s => s.message));
      const lastSent = async () => { const s = await sent(); return s[s.length - 1] || null; };
      const overflow = () => page.evaluate(() => ({
        scrollW: document.documentElement.scrollWidth, clientW: document.documentElement.clientWidth,
        scrollH: document.documentElement.scrollHeight, clientH: document.documentElement.clientHeight,
      }));
      const text = (sel) => page.evaluate((s) => { const el = document.querySelector(s); return el ? el.textContent : null; }, sel);

      // Every phase renders, fits sideways, and is screenshotted.
      for (const { name, view } of fixtures.views) {
        await stage(view);
        await page.waitForTimeout(150);
        await shot(name);
        const o = await overflow();
        check(o.scrollW <= o.clientW, `${tag}: ${name} has no horizontal overflow (${o.scrollW} <= ${o.clientW})`);
        const phaseEl = await page.$(`.phase-${view.phase}`);
        check(!!phaseEl, `${tag}: ${name} rendered as phase ${view.phase}`);
        if (view.phase === "drafting" && view.pack) {
          const tiles = await page.$$eval(".tile", t => t.length);
          check(tiles === view.pack.cards.length, `${tag}: ${name} shows ${view.pack.cards.length} cards (${tiles})`);
          if (viewport.width >= 1280 && view.pack.cards.length >= 14) {
            const bottom = await page.$$eval(".tile", t => Math.max(...t.map(x => x.getBoundingClientRect().bottom)));
            check(bottom <= viewport.height, `${tag}: ${name}: a ${view.pack.cards.length}-card pack fits one screen (last tile bottom ${Math.round(bottom)} <= ${viewport.height})`);
          }
          if (viewport.width < 700) {
            check(o.scrollH > o.clientH, `${tag}: ${name} scrolls vertically on a phone`);
          }
        }
      }

      const byName = Object.fromEntries(fixtures.views.map(v => [v.name, v.view]));

      // Drafting: click selects, a second click picks, the message names the pack and the index.
      {
        const view = byName.drafting;
        await stage(view);
        await page.waitForTimeout(100);
        const countdown = await text("#countdown");
        check(/\d+s left/.test(countdown || ""), `${tag}: countdown shows the deadline (${JSON.stringify(countdown)})`);
        const line = await text("#pick-line");
        check(line === `Pack ${view.pack.round}, pick ${view.pack.pick} of ${view.pack.size} — 1 pack waiting`, `${tag}: pick line reads "${line}"`);
        const before = (await sent()).length;
        const card = view.pack.cards[3];
        await page.click(`.tile[data-card-index="3"]`);
        await page.waitForTimeout(50);
        check(await page.$(`.tile[data-card-index="3"].selected`) !== null, `${tag}: first click selects card 3`);
        const detail = await text("#detail");
        check((detail || "").includes(card.name) && (card.text === "" || (detail || "").includes(card.text.split("\n")[0])), `${tag}: the side panel reads the selected card`);
        check((await sent()).length === before, `${tag}: selecting sends nothing`);
        await shot("drafting-selected");
        await page.click(`.tile[data-card-index="3"]`);
        await page.waitForTimeout(50);
        const msg = await lastSent();
        check(JSON.stringify(msg) === JSON.stringify({ type: "pick", pack_id: view.pack.id, index: 3 }), `${tag}: second click sent ${JSON.stringify(msg)}`);
        check(await page.$(`.tile[data-card-index="3"].pending`) !== null, `${tag}: the picked card shows as pending`);
        // Keys: a digit selects, arrows move, Enter picks, Escape deselects.
        await stage(view);
        await page.keyboard.press("5");
        await page.keyboard.press("ArrowRight");
        await page.waitForTimeout(50);
        check(await page.$(`.tile[data-card-index="5"].selected`) !== null, `${tag}: digit 5 then ArrowRight selects card 6 (index 5)`);
        await page.keyboard.press("Escape");
        await page.waitForTimeout(50);
        check(await page.$(`.tile.selected`) === null, `${tag}: Escape deselects`);
        await page.keyboard.press("Enter");
        await page.waitForTimeout(50);
        check((await text(".banner.hint") || "").includes("select one first"), `${tag}: Enter with nothing selected says so`);
        await page.keyboard.press("2");
        await page.keyboard.press("Enter");
        await page.waitForTimeout(50);
        const keyed = await lastSent();
        check(JSON.stringify(keyed) === JSON.stringify({ type: "pick", pack_id: view.pack.id, index: 1 }), `${tag}: digit 2 then Enter sent ${JSON.stringify(keyed)}`);
      }

      // A refusal is shown with its reason, and the page is back at the view.
      {
        await stage(byName.drafting);
        await page.click(`.tile[data-card-index="0"]`);
        await page.click(`.tile[data-card-index="0"]`);
        await page.waitForTimeout(50);
        await stage(fixtures.refused);
        await page.waitForTimeout(50);
        const banner = await text(".banner.refusal");
        check((banner || "").includes(fixtures.refused.reason), `${tag}: refusal shows "${fixtures.refused.reason}"`);
        check(await page.$(".tile.pending") === null, `${tag}: after a refusal nothing is pending`);
        check(await page.$(".tile:not(:disabled)") !== null, `${tag}: after a refusal the pack can be picked from again`);
        await shot("drafting-refused");
        await page.click(".banner.refusal .dismiss");
        await page.waitForTimeout(50);
        check(await page.$(".banner.refusal") === null, `${tag}: the refusal can be dismissed`);
      }

      // Building: an empty deck leaves Ready disabled with the reason; the
      // checklist and the steppers build a legal one; Ready sends deck then ready.
      {
        const view = byName.building;
        await stage(view);
        await page.waitForTimeout(100);
        check(await page.$eval("#ready", b => b.disabled), `${tag}: Ready starts disabled`);
        check((await text("#ready-reason") || "").includes("need at least 40"), `${tag}: the reason names the 40-card floor`);
        const start = (await sent()).length;
        const pick = view.pool.map((c, i) => ({ c, i })).filter(({ c }) => c.colors.includes("W") || c.colors.includes("G")).slice(0, 23);
        check(pick.length === 23, `${tag}: the fixture pool has 23 white or green cards to main-deck (${pick.length})`);
        for (const { i } of pick) await page.click(`.row[data-pool-index="${i}"]`);
        for (let k = 0; k < 8; k++) await page.click(`.land[data-land="Plains"] .inc`);
        for (let k = 0; k < 9; k++) await page.click(`.land[data-land="Forest"] .inc`);
        await page.waitForTimeout(50);
        const count = await text("#count-line");
        check(count === "23 spells + 17 lands = 40", `${tag}: count line reads "${count}"`);
        const deckMsg = await lastSent();
        const wantMain = pick.map(({ c }) => c.name);
        const wantSide = view.pool.filter((c, i) => !pick.some(p => p.i === i)).map(c => c.name);
        check(deckMsg && deckMsg.type === "deck" && JSON.stringify(deckMsg.main) === JSON.stringify(wantMain)
          && JSON.stringify(deckMsg.lands) === JSON.stringify({ Plains: 8, Forest: 9 })
          && JSON.stringify(deckMsg.sideboard) === JSON.stringify(wantSide),
          `${tag}: every change re-sent the deck; the last is the whole deck (${deckMsg && deckMsg.main.length} main, ${deckMsg && deckMsg.sideboard.length} side)`);
        check((await sent()).length - start === 23 + 17, `${tag}: one deck message per change`);
        check(await page.$eval("#ready", b => !b.disabled), `${tag}: Ready is enabled at 40 cards`);
        await shot("building-filled");
        // Take one out: disabled again, with the count.
        await page.click(`.land[data-land="Forest"] .dec`);
        await page.waitForTimeout(50);
        check(await page.$eval("#ready", b => b.disabled) && (await text("#ready-reason") || "").includes("39 cards"), `${tag}: 39 cards disables Ready and says so`);
        await page.click(`.land[data-land="Forest"] .inc`);
        await page.click("#ready");
        await page.waitForTimeout(50);
        const all = await sent();
        const tail = all.slice(-2);
        check(tail[0] && tail[0].type === "deck" && tail[1] && tail[1].type === "ready", `${tag}: Ready sent deck then ready (${tail.map(m => m.type).join(", ")})`);
        check((await text("#ready-line") || "").includes("waiting for the others"), `${tag}: after Ready the page waits for the others`);
        check(await page.$(".row:not(:disabled)") === null, `${tag}: after Ready the checklist is read-only`);
        await shot("building-ready");
        // A refusal of the deck puts the page back on the server's deck.
        await stage({ type: "refused", reason: "Deck has 39 cards (need at least 40).", echo: { type: "deck" } });
        await page.waitForTimeout(50);
        check((await text(".banner.refusal") || "").includes("39 cards"), `${tag}: a refused deck shows the server's reason`);
        check(await page.$("#ready") !== null, `${tag}: and the deck can be edited again`);
        // The server's own verdict on a recorded deck is shown too.
        await stage(byName.building_with_deck);
        await page.waitForTimeout(50);
        check((await text("#server-problem") || "").includes("35 cards"), `${tag}: the server's problem with a recorded deck is shown`);
        check(await page.$eval("#ready", b => b.disabled), `${tag}: and Ready stays disabled for it`);
        const recorded = await text("#count-line");
        check(recorded === "20 spells + 15 lands = 35", `${tag}: the recorded deck is counted ("${recorded}")`);
        // Keys: arrows move the cursor, Enter moves the card.
        await page.keyboard.press("ArrowDown");
        await page.keyboard.press("Enter");
        await page.waitForTimeout(50);
        const toggled = await lastSent();
        check(toggled && toggled.type === "deck" && toggled.main.length + Object.values(toggled.lands).reduce((a, b) => a + b, 0) !== 35, `${tag}: ArrowDown then Enter moves a card (${toggled && toggled.main.length} main)`);
      }

      // Playing: the game link is a link; waiting is said; done shows the standings.
      {
        await stage(byName.playing);
        await page.waitForTimeout(50);
        const href = await page.$eval(".match-playing a", a => a.getAttribute("href"));
        check(href === "http://192.168.1.20:8805/", `${tag}: the live match links its game (${href})`);
        check((await page.$$(".standings tbody tr")).length === 4, `${tag}: standings list every seat`);
        await stage(byName.playing_waiting);
        await page.waitForTimeout(50);
        check((await text(".waiting") || "").includes("Waiting for the others"), `${tag}: a seat with no match waits for the others`);
        await stage(byName.done);
        await page.waitForTimeout(50);
        check((await text("h2") || "").includes("Draft over"), `${tag}: done says the draft is over`);
        check(((await text(".standings tbody tr.me") || "").includes("you")), `${tag}: final standings mark this seat`);
      }

      // The socket was refused, so the page is retrying with backoff.
      {
        await page.waitForTimeout(1200);
        const r = await page.evaluate(() => ({ reconnects: window.mtgDraft.reconnects, connected: window.mtgDraft.connected, conn: document.getElementById("conn").textContent }));
        check(r.reconnects >= 1 && !r.connected && /retrying/.test(r.conn), `${tag}: reconnecting with backoff (${r.reconnects} tries, "${r.conn}")`);
      }

      if (errors.length) fail(`${tag}: page errors:\n  ${errors.join("\n  ")}`); else ok(`${tag}: no page errors`);
      await page.close();
    }
  } finally {
    await browser.close();
    server.close();
  }
  if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
  console.log("draft page: all checks passed");
}

main().catch(e => { console.error(e); process.exit(1); });

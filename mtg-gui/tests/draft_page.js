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
const shotsDir = require("./children.js").parseArgs({ "--shots": { kind: "string", def: null } })["--shots"];
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
      await page.goto(`http://127.0.0.1:${port}/?seat=0&key=test-key`, { timeout: 15000 });
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
      // The wording, with the no-break spaces that hold its units on one
      // line (#881, #934) read as the spaces they are.
      const text = (sel) => page.evaluate((s) => { const el = document.querySelector(s); return el ? el.textContent.replace(/\u00a0/g, " ") : null; }, sel);
      // Every visible button whose label is under 3:1 against what it is drawn on
      // (#873: a classless button was #eee on the browser's #efefef face).
      const unreadable = () => page.evaluate(() => {
        const rgb = (c) => (c.match(/[\d.]+/g) || []).map(Number);
        const lum = ([r, g, b]) => [r, g, b].map(v => { v /= 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; })
          .reduce((a, v, i) => a + v * [0.2126, 0.7152, 0.0722][i], 0);
        const ground = (el) => {
          for (let e = el; e; e = e.parentElement) {
            const c = rgb(getComputedStyle(e).backgroundColor);
            if (c.length === 3 || c[3] > 0) return c;
          }
          return rgb(getComputedStyle(document.body).backgroundColor);
        };
        return [...document.querySelectorAll("button")].filter(b => b.offsetParent && b.textContent.trim()).map(b => {
          const [a, c] = [lum(rgb(getComputedStyle(b).color)), lum(ground(b))];
          return { label: b.textContent.trim().slice(0, 30), ratio: (Math.max(a, c) + 0.05) / (Math.min(a, c) + 0.05) };
        }).filter(x => x.ratio < 3).map(x => `${x.label} ${x.ratio.toFixed(2)}:1`);
      });

      // Every phase renders, fits sideways, and is screenshotted.
      for (const { name, view } of fixtures.views) {
        await stage(view);
        await page.waitForTimeout(150);
        await shot(name);
        const o = await overflow();
        check(o.scrollW <= o.clientW, `${tag}: ${name} has no horizontal overflow (${o.scrollW} <= ${o.clientW})`);
        const faint = await unreadable();
        check(faint.length === 0, `${tag}: ${name}: every button's label can be read (${JSON.stringify(faint)})`);
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

      // #892: back on the page, the header's box shows the name the seat
      // set; the table's own "seat N" is no name to show.
      {
        await page.$eval("#name", el => { el.value = ""; el.blur(); });
        const named = JSON.parse(JSON.stringify(byName.drafting));
        named.seats[named.seat].name = "Ann";
        await stage(named);
        await page.waitForTimeout(50);
        check(await page.$eval("#name", el => el.value) === "Ann", `${tag}: the name box shows the seat's name after a reload`);
        // Renamed from another connection: the box follows, so a reconnect
        // does not send the old name back.
        named.seats[named.seat].name = "Bea";
        await stage(named);
        await page.waitForTimeout(50);
        check(await page.$eval("#name", el => el.value) === "Bea", `${tag}: the name box follows a rename made elsewhere`);
        const unnamed = JSON.parse(JSON.stringify(byName.drafting));
        unnamed.seats[unnamed.seat].name = `seat ${unnamed.seat}`;
        await stage(unnamed);
        await page.waitForTimeout(50);
        check(await page.$eval("#name", el => el.value) === "", `${tag}: and stays empty for a seat with no name of its own`);
      }

      // #929: the Seats table's fourth column says where a seat is ("here",
      // "in game", "game page closed", "away", "not yet"), not whether it
      // joined, and its heading says so.
      {
        await stage(byName.playing);
        await page.waitForTimeout(50);
        const heads = await page.$$eval("table.seats thead th", t => t.map(x => x.textContent.trim()));
        check(heads.includes("where") && !heads.includes("joined"), `${tag}: the Seats table heads its presence column "where" (${JSON.stringify(heads)})`);
      }

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
        // Enter on a focused tile is a click on that tile, not a pick of the
        // selected one (#787): select 2, focus 6, Enter selects 6 and sends nothing.
        await stage(view);
        await page.keyboard.press("2");
        await page.waitForTimeout(50);
        const count0 = (await sent()).length;
        await page.focus(`.tile[data-card-index="5"]`);
        await page.keyboard.press("Enter");
        await page.waitForTimeout(50);
        check((await sent()).length === count0, `${tag}: Enter on a focused tile sent ${JSON.stringify(await lastSent())} for the selected card`);
        check(await page.$(`.tile[data-card-index="5"].selected`) !== null, `${tag}: Enter on a focused tile selects that tile`);
        // A held Enter on a focused tile selects it and stops: its repeats
        // were clicks, and the second one picked the card (#852's review).
        await stage(view);
        await page.waitForTimeout(50);
        {
          const before = (await sent()).length;
          await page.focus(`.tile[data-card-index="4"]`);
          for (let k = 0; k < 6; k++) await page.keyboard.down("Enter");
          await page.keyboard.up("Enter");
          await page.waitForTimeout(50);
          check((await sent()).length === before, `${tag}: a held Enter on a tile picks nothing (${JSON.stringify(await lastSent())})`);
          check(await page.$(`.tile[data-card-index="4"].selected`) !== null, `${tag}: a held Enter on a tile selects it`);
        }
        // A click focuses the tile; an arrow then moves the selection and the
        // focus with it, so Enter acts on the tile the arrow reached (#787's review).
        await stage(view);
        await page.click(`.tile[data-card-index="2"]`);
        await page.keyboard.press("ArrowRight");
        await page.waitForTimeout(50);
        const count1 = (await sent()).length;
        await page.keyboard.press("Enter");
        await page.waitForTimeout(50);
        const arrowed = await lastSent();
        check((await sent()).length === count1 + 1 && arrowed.type === "pick" && arrowed.index === 3, `${tag}: click 3rd, ArrowRight, Enter sent ${JSON.stringify(arrowed)}`);
        // On a phone the detail sits above the pack: a hover must not change
        // it, or the pack moves under the pointer and a tap selects nothing
        // (#791). Beside the pack (1280) a hover reads the card.
        await stage(view);
        await page.keyboard.press("1");
        await page.waitForTimeout(50);
        const detail0 = await text("#detail");
        await page.hover(`.tile[data-card-index="7"]`);
        await page.waitForTimeout(50);
        const detail1 = await text("#detail");
        if (viewport.width < 760) check(detail1 === detail0, `${tag}: a hover left the detail above the pack alone`);
        else check(detail1 !== detail0, `${tag}: a hover beside the pack reads the hovered card`);
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
        // On a phone a first tap reads the card under its row and sends
        // nothing; the second moves it (#792). Beside the deck one click moves it.
        {
          const i = pick[0].i;
          const before = (await sent()).length;
          await page.click(`.row[data-pool-index="${i}"]`);
          await page.waitForTimeout(50);
          const read = await page.evaluate((i) => {
            const d = document.querySelector("#row-detail");
            const row = document.querySelector(`.row[data-pool-index="${i}"]`);
            const shown = d && getComputedStyle(d).display !== "none";
            const pc = document.querySelector("#phone-count");
            return { shown, text: d ? d.textContent : "", gap: shown ? d.getBoundingClientRect().top - row.getBoundingClientRect().bottom : null,
              count: pc && getComputedStyle(pc).display !== "none" ? pc.getBoundingClientRect().top : null };
          }, i);
          const moved = (await sent()).length - before;
          if (viewport.width < 760) {
            check(moved === 0, `${tag}: a first tap on a pool row sends nothing (${moved})`);
            check(read.shown && read.text.includes(view.pool[i].name) && read.gap >= 0 && read.gap < 20,
              `${tag}: a first tap reads the card under its row (${JSON.stringify({ shown: read.shown, gap: read.gap })})`);
            check(read.count !== null && read.count < viewport.height, `${tag}: the count is on screen beside the rows (${read.count})`);
            await page.click(`.row[data-pool-index="${i}"]`);
            await page.waitForTimeout(50);
            check((await sent()).length - before === 1, `${tag}: a second tap moves the card`);
            // A tap on a row below an open one keeps it under the finger:
            // the open text above closes on the same draw (#792's review).
            // A row a few below the one whose text is open.
            const j = await page.evaluate(() => {
              const rows = [...document.querySelectorAll(".row")];
              const d = document.querySelector("#row-detail");
              const below = rows.filter(r => d.compareDocumentPosition(r) & Node.DOCUMENT_POSITION_FOLLOWING);
              return Number(below[2].dataset.poolIndex);
            });
            const topOf = (k) => page.$eval(`.row[data-pool-index="${k}"]`, el => el.getBoundingClientRect().top);
            await page.$eval(`.row[data-pool-index="${j}"]`, el => el.scrollIntoView({ block: "center" }));
            const t0 = await topOf(j);
            await page.click(`.row[data-pool-index="${j}"]`);
            await page.waitForTimeout(50);
            const t1 = await topOf(j);
            check(Math.abs(t1 - t0) <= 2, `${tag}: the tapped row stays where it was tapped (${Math.round(t0)} -> ${Math.round(t1)})`);
            // Enter on a focused row moves it, as at any width: a key is not a tap.
            await page.keyboard.press("Escape");
            await page.focus(`.row[data-pool-index="${j}"]`);
            const n0 = (await sent()).length;
            await page.keyboard.press("Enter");
            await page.waitForTimeout(50);
            check((await sent()).length === n0 + 1, `${tag}: Escape, then Enter on the focused row moves the card`);
            // Put it back where the rest of this test expects it.
            await page.keyboard.press("Enter");
            await page.waitForTimeout(50);
          } else {
            check(moved === 1 && !read.shown && read.count === null, `${tag}: beside the deck one click moves the card and nothing reads inline`);
          }
        }
        for (const { i } of pick.slice(1)) {
          await page.click(`.row[data-pool-index="${i}"]`);
          if (viewport.width < 760) await page.click(`.row[data-pool-index="${i}"]`);
        }
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
        // On a phone the Enter-moves check above moves a card out and back: two more.
        check((await sent()).length - start === 23 + 17 + (viewport.width < 760 ? 2 : 0), `${tag}: one deck message per change`);
        check(await page.$eval("#ready", b => !b.disabled), `${tag}: Ready is enabled at 40 cards`);
        await shot("building-filled");
        // Take one out: disabled again, with the count.
        await page.click(`.land[data-land="Forest"] .dec`);
        await page.waitForTimeout(50);
        check(await page.$eval("#ready", b => b.disabled) && (await text("#ready-reason") || "").includes("39 cards"), `${tag}: 39 cards disables Ready and says so`);
        check((await text("#ready-reason") || "").includes("Add 1 more card or basic land."), `${tag}: one card short is said in the singular (${await text("#ready-reason")})`);
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
        // A build timer counts down beside the deck, as the pick timer does.
        await stage({ ...byName.building, build_deadline_ms: 90000, build_seconds: 90 });
        await page.waitForTimeout(50);
        check(/\d+s left/.test((await text("#countdown")) || ""), `${tag}: the build timer counts down (${JSON.stringify(await text("#countdown"))})`);
        await stage(byName.building_with_deck);
        await page.waitForTimeout(50);
        // Keys: arrows move the cursor, Enter moves the card.
        await page.keyboard.press("ArrowDown");
        await page.keyboard.press("Enter");
        await page.waitForTimeout(50);
        const toggled = await lastSent();
        check(toggled && toggled.type === "deck" && toggled.main.length + Object.values(toggled.lands).reduce((a, b) => a + b, 0) !== 35, `${tag}: ArrowDown then Enter moves a card (${toggled && toggled.main.length} main)`);
        // A held Enter is one press: its repeats flipped the card back and
        // forth, one deck message each, ending where the parity left it (#852).
        {
          const before = (await sent()).length;
          for (let k = 0; k < 6; k++) await page.keyboard.down("Enter");
          await page.keyboard.up("Enter");
          await page.waitForTimeout(50);
          const held = (await sent()).slice(before);
          check(held.length === 1, `${tag}: a held Enter moves the card once (${held.length} deck messages)`);
        }
        // And on a focused row, where the browser turns each repeat into a
        // click (the issue's own repro: focus a row, hold Enter).
        {
          await page.focus(`.row[data-pool-index="3"]`);
          const before = (await sent()).length;
          for (let k = 0; k < 6; k++) await page.keyboard.down("Enter");
          await page.keyboard.up("Enter");
          await page.waitForTimeout(50);
          const held = (await sent()).slice(before);
          check(held.length === 1, `${tag}: a held Enter on a focused row moves it once (${held.length} deck messages)`);
        }
        // Enter on a focused land "+" adds the land; it does not move the
        // card under the cursor (#787).
        await page.keyboard.press("ArrowDown");
        await page.waitForTimeout(50);
        const mainBefore = (await lastSent()).main.length;
        await page.focus(".inc:not([disabled])");
        await page.keyboard.press("Enter");
        await page.waitForTimeout(50);
        const landed = await lastSent();
        const landsOf = m => Object.values(m.lands).reduce((a, b) => a + b, 0);
        check(landed.main.length === mainBefore && landsOf(landed) === landsOf(toggled) + 1, `${tag}: Enter on a focused "+" sent ${JSON.stringify(landed)}`);
        // And the focus survives the rebuild, so Enter again adds another
        // (#788: every press dropped the focus to <body>).
        await page.keyboard.press("Enter");
        await page.keyboard.press("Enter");
        await page.waitForTimeout(50);
        const thrice = await lastSent();
        check(landsOf(thrice) === landsOf(toggled) + 3, `${tag}: three Enters on a focused "+" add three lands (${landsOf(thrice) - landsOf(toggled)})`);
        check(await page.evaluate(() => document.activeElement && document.activeElement.classList.contains("inc")), `${tag}: the "+" keeps the focus across the rebuild`);
      }

      // A notice outlives the view that carried it: the table's "built your
      // deck" came in the last building view and the playing view replaced
      // it a millisecond later (#817). It stays until dismissed.
      {
        await stage({ ...byName.building, notice: "The table built your deck (the build timer ran out)." });
        await stage(byName.playing_waiting);
        await page.waitForTimeout(50);
        check((await text(".banner.notice") || "").includes("The table built your deck"), `${tag}: the notice survives the next view`);
        await page.click(".banner.notice .dismiss");
        await page.waitForTimeout(50);
        check(await page.$(".banner.notice") === null, `${tag}: the notice can be dismissed`);
        // Escape dismisses it too, as it does a refusal.
        await stage({ ...byName.playing_waiting, notice: "The host has handed your seat to the table." });
        await page.keyboard.press("Escape");
        await page.waitForTimeout(50);
        check(await page.$(".banner.notice") === null, `${tag}: Escape dismisses the notice`);
        // A pick puts an old "the table picked for you" away: it is about
        // a pick the person is past.
        await stage({ ...byName.drafting, notice: "The table picked Moonmist for you (the pick timer ran out)." });
        await page.click(`.tile[data-card-index="0"]`);
        await page.click(`.tile[data-card-index="0"]`);
        await page.waitForTimeout(50);
        check(await page.$(".banner.notice") === null, `${tag}: a pick clears the table's old notice`);
      }

      // Playing: the game link is a link; waiting is said; done shows the standings.
      {
        await stage(byName.playing);
        await page.waitForTimeout(50);
        const href = await page.$eval(".match-playing a", a => a.getAttribute("href"));
        check(href === "http://192.168.1.20:8805/", `${tag}: the live match links its game (${href})`);
        check((await page.$$(".standings tbody tr")).length === 4, `${tag}: standings list every seat`);
        // A finished match sits beside standings still on 0: the heading
        // says what they count (#818).
        await stage({ ...byName.playing, standings_after: 0 });
        await page.waitForTimeout(50);
        const heads = await page.$$eval("h3", hs => hs.map(x => x.textContent));
        check(heads.includes("Standings (counted when round 1 ends)"), `${tag}: the standings say what they count (${heads})`);
        await stage({ ...byName.playing, standings_after: 2 });
        await page.waitForTimeout(50);
        check((await page.$$eval("h3", hs => hs.map(x => x.textContent))).includes("Standings after round 2"),
          `${tag}: the standings say after which round`);
        await stage(byName.playing);
        await page.waitForTimeout(50);
        // The port is never split across lines (#794): its text piece is one
        // line box wherever the URL wraps.
        const portLines = await page.$eval(".match-playing a", a => {
          const node = [...a.childNodes].find(n => n.nodeType === 3 && n.textContent.startsWith(":") && /\d/.test(n.textContent));
          if (!node) return -1;
          const r = document.createRange(); r.selectNodeContents(node); return r.getClientRects().length;
        });
        check(portLines === 1, `${tag}: the game link's port sits on one line (${portLines} line boxes)`);
        // A hosted table's link carries the page's own 32-character key (#872):
        // it fits a phone's width too.
        {
          const keyed = JSON.parse(JSON.stringify(byName.playing));
          for (const m of keyed.matches) if (m.url) m.url = `http://192.168.1.20:8800/game/8805/${"0123456789abcdef".repeat(2)}/`;
          await stage(keyed);
          await page.waitForTimeout(50);
          const o = await overflow();
          check(o.scrollW <= o.clientW, `${tag}: a keyed game link fits sideways (${o.scrollW} <= ${o.clientW})`);
          await stage(byName.playing);
          await page.waitForTimeout(50);
        }
        // A long one-word name and the #744 tags fit a phone (#789), and a
        // record is never broken at its hyphen (#790). The tiebreak heading
        // may wrap, at a "/", as its numbers do (#881; checked below).
        const wide = JSON.parse(JSON.stringify(byName.playing));
        wide.seats[1].name = "Featherstonehaughmaximus";
        wide.standings.forEach((r, i) => { r.draws = i % 2; r.tags = "[substitute deck] [1 game forfeited]"; });
        await stage(wide);
        await page.waitForTimeout(50);
        const fit = await page.evaluate(() => ({ sw: document.documentElement.scrollWidth, w: innerWidth,
          broken: [...document.querySelectorAll(".standings td.num:not(.tiebreak), .standings th:not(.tiebreak), .rec")].filter(el => { const r = document.createRange(); r.selectNodeContents(el); return new Set([...r.getClientRects()].map(b => Math.round(b.top))).size > 1; }).map(el => el.textContent) }));
        check(fit.sw <= fit.w, `${tag}: a 24-letter name keeps the page on screen (${fit.sw} <= ${fit.w})`);
        check(fit.broken.length === 0, `${tag}: no record or heading breaks across lines (${JSON.stringify(fit.broken)})`);
        check(await page.$$eval(".pairings-list .rec, .match-head .rec", els => els.length) > 0 || !(await page.$(".pairings-list")), `${tag}: the pairings' records are held whole`);
        // A unit of several words holds together, and an ordinary word is
        // never broken inside: "STRANGE" / "R ZERO", "[1" / "BYE]", a tiebreak
        // line ending "/" (#881), "(seat" / "2)" and a line ending "—" (#934),
        // and "G2" / "you" (#934's review). "(table picks)" may wrap: held
        // whole it pushed the seats table into the gutter, checked below.
        // Only a word too long for any column (the 24 letters of #789) breaks.
        for (const rot of [0, 1, 2, 3]) {
          const v = JSON.parse(JSON.stringify(byName.playing));
          const names = ["Margaret", "Wilhelmina Fitzgerald", "Bartholomewmaximilianjoe", "Stranger Zero"];
          v.seats.forEach((st, i) => { st.name = names[(i + rot) % names.length]; st.auto = true; st.status = i % 2 ? "building" : "picking"; });
          v.standings.forEach((r, i) => { r.tags = ["[1 bye]", "[substitute deck] [2 games forfeited]", "", "[1 bye]"][i % 4]; r.omw = 0.415; r.gw = 1; r.ogw = 0.5; });
          await stage(v);
          await page.waitForTimeout(50);
          const split = await page.evaluate(() => {
            const lines = el => {
              const out = []; const w = document.createTreeWalker(el, NodeFilter.SHOW_TEXT); let n, cur = null, top = null;
              while ((n = w.nextNode())) for (let i = 0; i < n.data.length; i++) {
                const rg = document.createRange(); rg.setStart(n, i); rg.setEnd(n, i + 1);
                const rc = rg.getClientRects()[0]; if (!rc) continue;
                const t = Math.round(rc.top);
                if (t !== top) { if (cur !== null) out.push(cur); cur = ""; top = t; }
                cur += n.data[i];
              }
              if (cur !== null) out.push(cur);
              return out.map(x => x.replace(/\s+/g, " ").trim());
            };
            const bad = [];
            for (const el of document.querySelectorAll(".standings td, .standings th, .seats td, .pairings-list li, .match-head, .match-games, #between-head")) {
              const ls = lines(el); const full = el.textContent.replace(/\s+/g, " ");
              for (let k = 0; k < ls.length - 1; k++) {
                const a = ls[k], c = ls[k + 1];
                const tail = a.split(" ").pop(), head = c.split(" ")[0];
                const midWord = /[A-Za-z0-9.]$/.test(a) && /^[A-Za-z0-9.]/.test(c) && !full.includes(`${tail} ${head}`)
                  && (full.split(" ").find(w => w.includes(tail + head)) || tail + head).replace(/[()[\]]/g, "").length < 15;
                if (midWord || /\/$/.test(a) || /\(seat$/i.test(a) || /\[\d+$/.test(a) || /—$/.test(a) || /^G\d+$/.test(tail)) bad.push(ls.join(" | "));
              }
            }
            return bad;
          });
          check(split.length === 0, `${tag}: names, tags, tiebreaks and seat labels hold together, rotation ${rot} (${JSON.stringify(split.slice(0, 3))})`);
          // Held together inside the page's gutter, not only short of a
          // sideways scroll: the seats table ran 10px into it (#934's review).
          const over = await page.evaluate(() => {
            const app = document.querySelector("#app"); const cs = getComputedStyle(app);
            const right = app.getBoundingClientRect().right - parseFloat(cs.paddingRight);
            return [...document.querySelectorAll("#app table")].filter(t => t.getBoundingClientRect().right > right + 0.5)
              .map(t => `${t.className} ${Math.round(t.getBoundingClientRect().right)} > ${Math.round(right)}`);
          });
          check(over.length === 0, `${tag}: every table stays inside the gutter, rotation ${rot} (${JSON.stringify(over)})`);
        }
        // A black pip is a disc on the panel, as the other colours are (#793).
        const pip = await page.evaluate(() => { const el = document.createElement("span"); el.className = "pip pip-B"; document.body.append(el); const c = getComputedStyle(el).backgroundColor; el.remove(); return c; });
        check(pip !== "rgb(42, 30, 48)", `${tag}: the B pip is not the panel's colour (${pip})`);
        await stage(byName.playing_waiting);
        await page.waitForTimeout(50);
        check((await text(".waiting") || "").includes("Waiting for the others"), `${tag}: a seat with no match waits for the others`);
        // A seat given the bye is told so, in its own matches and in the
        // waiting line, not only in the table's list (#814).
        {
          const v = JSON.parse(JSON.stringify(byName.playing_waiting));
          v.pairings = v.pairings || [];
          const round = Math.max(1, ...v.pairings.map(p => p.round), ...v.matches.map(m => m.round));
          v.pairings = v.pairings.filter(p => p.b !== null || p.round !== round).concat([{ round, a: v.seat, b: null, status: "done", result: "bye" }]);
          v.matches = v.matches.filter(m => m.round !== round);
          await stage(v);
          await page.waitForTimeout(50);
          const own = (await text(".matches")) || "";
          check(own.includes(`Round ${round}: a bye`), `${tag}: the seat's own bye is in its matches (${JSON.stringify(own)})`);
          check((await text(".waiting") || "").includes(`You have a bye in round ${round}`), `${tag}: the waiting line says why`);
        }
        // What the server really sends and the plan's sketch did not: a
        // match whose page is not open yet has no link, a drawn game has
        // no winner, the whole round is listed with its bye, a seat can be
        // away, and the table can be picking for one. None of it may
        // render as "null".
        await stage(byName.playing_unlinked);
        await page.waitForTimeout(50);
        const whole = (await text("#app")) || "";
        check(!/\b(null|undefined|NaN)\b/.test(whole), `${tag}: nothing on the page reads null/undefined/NaN`);
        check(await page.$(".match-waiting a") === null && (await text(".match-waiting .match-link") || "").includes("opens when the match starts"), `${tag}: a match with no page yet says so instead of linking null`);
        check((await text(".match-done .match-games") || "").includes("drawn"), `${tag}: a drawn game is called drawn`);
        // A forfeit is said as one, an abandoned game is not a draw, and the
        // standings carry the terminal's tags and draws (#743, #744).
        {
          const v = JSON.parse(JSON.stringify(byName.playing_unlinked));
          const done = v.matches.find(m => m.status === "done");
          done.games = [{ winner: done.opponent, forfeited_by: v.seat, abandoned: false }, { winner: null, forfeited_by: null, abandoned: true }];
          v.standings[0].tags = "[substitute deck] [1 game forfeited]";
          v.standings[0].draws = 1;
          await stage(v);
          await page.waitForTimeout(50);
          const games = (await text(".match-done .match-games")) || "";
          check(games.includes("G1") && games.includes("(you forfeited)"), `${tag}: a forfeited game says who forfeited (${games})`);
          check(games.includes("G2 abandoned") && !games.includes("drawn"), `${tag}: an abandoned game is not a draw (${games})`);
          const table = (await text(".standings")) || "";
          check(table.includes("[substitute deck] [1 game forfeited]"), `${tag}: the standings carry the tags`);
          const record = await page.$eval(".standings tbody tr:first-child td.num", td => td.textContent);
          check(table.includes("W-L-D") && /^\d+-\d+-1$/.test(record || ""), `${tag}: a drawn match is in the record (${record})`);
        }
        check((await page.$$(".pairings-list li")).length === 4 && (await text(".pairings-list") || "").includes("has a bye"), `${tag}: every pairing of the tournament is listed, the bye included`);
        check(whole.includes("away") && whole.includes("(table picks)"), `${tag}: the seats table says who is away and whom the table picks for`);
        // A match its games have decided is "playing" until the result is
        // recorded, and is not on a next game (#916).
        {
          const v = JSON.parse(JSON.stringify(byName.playing));
          const live = v.matches.find(m => m.status === "playing");
          await stage(v);
          await page.waitForTimeout(50);
          check((await text(".match-playing .match-head") || "").includes("playing now"), `${tag}: a match on its next game is playing now`);
          live.next_game = null;
          await stage(v);
          await page.waitForTimeout(50);
          const head = (await text(".match-playing .match-head")) || "";
          check(head.includes("decided") && !head.includes("playing now"), `${tag}: a decided match says so (${head})`);
          check(await page.$(".match-playing .match-link") === null, `${tag}: and offers no game to open`);
          if (v.matches.every(m => m.status === "done" || m === live))
            check((await text(".waiting") || "").includes("decided"), `${tag}: the waiting line says the match is decided (${await text(".waiting")})`);
        }
        await stage(byName.done);
        await page.waitForTimeout(50);
        check((await text("h2") || "").includes("Draft over"), `${tag}: done says the draft is over`);
        check(((await text(".standings tbody tr.me") || "").includes("you")), `${tag}: final standings mark this seat`);
      }

      // Between games of a match (MTR 2.2, 2.4): the server's fixture has
      // seat 0 sideboarding before game 2, its opponent choosing.
      const between = byName["server-playing_sideboard"];
      if (between) {
        const b = between.matches.find(m => m.between).between;
        await stage(byName.done);
        await stage(between);
        await page.waitForTimeout(50);
        check(((await text("#between-head")) || "").startsWith("Before game 2 vs"), `${tag}: the panel says which game is next (${await text("#between-head")})`);
        check(((await text("#play-draw")) || "").includes("chooses whether to play or draw"), `${tag}: the opponent's choice is said as theirs`);
        check(((await text("#opponent-showed")) || "").includes(b.opponent_showed[0].name), `${tag}: what the opponent showed is listed`);
        const rows = await page.$$eval("#between .row", r => r.length);
        check(rows === between.pool.length, `${tag}: every pool card is a row to sideboard with (${rows})`);
        const inMain = await page.$$eval("#between .row.in-main", r => r.length);
        check(inMain === b.deck.main.length, `${tag}: the deck played last game is the one marked (${inMain} of ${b.deck.main.length})`);
        // Keeping the deck is the usual choice on a running clock: Ready is
        // on the first screen at either width, and on a phone it and the
        // land steppers come before the pool, not ~1,900 px under it (#885).
        {
          const at = await page.evaluate(() => {
            const top = sel => { const el = document.querySelector(sel); return el ? el.getBoundingClientRect().top + scrollY : null; };
            const bottom = sel => { const el = document.querySelector(sel); return el ? el.getBoundingClientRect().bottom + scrollY : null; };
            return { ready: bottom("#sideboard-ready"), steppers: top('#between [data-land="Plains"]'), firstRow: top("#between .row") };
          });
          check(at.ready !== null && at.ready <= viewport.height, `${tag}: Ready for the next game is on the first screen (bottom ${Math.round(at.ready)} <= ${viewport.height})`);
          if (viewport.width < 760) check(at.steppers !== null && at.steppers < at.firstRow, `${tag}: the land steppers come before the pool (${Math.round(at.steppers)} < ${Math.round(at.firstRow)})`);
        }
        const before = (await sent()).length;
        // One card out: 39, which the server would refuse, so nothing is sent
        // and Ready says why; a basic in for it is a deck again, and is sent.
        await page.click("#between .row.in-main");
        if (viewport.width < 700) await page.click("#between .row.in-main.cursor");
        await page.waitForTimeout(50);
        const total = b.deck.main.length + Object.values(b.deck.lands).reduce((x, y) => x + y, 0);
        if (total === 40) {
          check((await sent()).length === before, `${tag}: a 39-card deck is not sent between games`);
          check(await page.$eval("#sideboard-ready", el => el.disabled), `${tag}: and Ready waits for 40 (${await text("#ready-reason")})`);
        }
        await page.click('#between [data-land="Plains"] > .inc');
        await page.waitForTimeout(50);
        const sb = await lastSent();
        check(sb && sb.type === "sideboard" && sb.main.length === b.deck.main.length - 1 && !("sideboard" in sb),
          `${tag}: a playable deck is sent as a sideboard message (${JSON.stringify(sb && { type: sb.type, main: sb.main.length, lands: sb.lands })})`);
        await page.click("#sideboard-ready");
        await page.waitForTimeout(50);
        const last2 = (await sent()).slice(-2).map(m => m.type);
        check(last2.join(",") === "sideboard,sideboard_ready", `${tag}: Ready sends the deck and then says so (${last2})`);
        check(((await text("#ready-line")) || "").includes("game 2 is starting"), `${tag}: the panel says the game is starting (${await text("#ready-line")})`);
        check(await page.$eval("#between .row", el => el.disabled), `${tag}: and nothing moves after Ready`);

        // After a game with no winner the choice stays put, and a game
        // abandoned at the action budget is not called a draw (#918).
        for (const [last, said] of [[{ winner: null, forfeited_by: null, abandoned: true }, "Game 1 was abandoned with no winner"],
                                    [{ winner: null, forfeited_by: null, abandoned: false }, "Game 1 was a draw"]]) {
          const v = JSON.parse(JSON.stringify(between));
          const m = v.matches.find(x => x.between);
          Object.assign(m.between, { game: 2, you_choose: true, play_first: null, ready: false });
          m.games = [last];
          await stage(byName.done);
          await stage(v);
          await page.waitForTimeout(50);
          const pd = (await text("#play-draw")) || "";
          check(pd.includes(said) && pd.includes("the choice is still yours"), `${tag}: after ${last.abandoned ? "an abandoned" : "a drawn"} game the chooser is told so (${pd})`);
        }

        // Before game 1, for the flip's winner: the choice and no deck.
        const flip = JSON.parse(JSON.stringify(between));
        Object.assign(flip.matches.find(m => m.between).between, {
          game: 1, sideboarding: false, you_choose: true, play_first: null, ready: false, opponent_ready: true, deadline_ms: 30000, opponent_showed: [],
        });
        flip.matches.find(m => m.between).games = [];
        await stage(byName.done);
        await stage(flip);
        await page.waitForTimeout(50);
        check(await page.$$eval("#between .row", r => r.length) === 0, `${tag}: no sideboarding before game 1`);
        check(((await text("#play-draw")) || "").includes("You won the coin flip"), `${tag}: the flip's winner is asked (${await text("#play-draw")})`);
        const faintChoice = await unreadable();
        check(faintChoice.length === 0, `${tag}: Play first and Draw first can be read before either is chosen (${JSON.stringify(faintChoice)})`);
        check(/\d+s left/.test((await text("#countdown")) || ""), `${tag}: the sideboard clock is shown (${await text("#countdown")})`);
        await page.click("#draw-first");
        await page.waitForTimeout(50);
        const pd = await lastSent();
        check(pd && pd.type === "play_draw" && pd.play === false, `${tag}: Draw first is sent (${JSON.stringify(pd)})`);
        const o = await overflow();
        check(o.scrollW <= o.clientW, `${tag}: the panel fits sideways (${o.scrollW} <= ${o.clientW})`);
      } else {
        console.log("skip: no server fixture between games");
      }

      // The draft review: asked for when opened, shown by pack, the card
      // taken first and the rest of its pack after it.
      {
        const view = byName.done;
        await stage(view);
        await page.waitForTimeout(50);
        await page.click("#review-toggle");
        await page.waitForTimeout(50);
        const asked = await lastSent();
        check(asked && asked.type === "review", `${tag}: opening the review asks for it (${JSON.stringify(asked)})`);
        const card = (name) => ({ name, line: name, text: `${name} text`, rarity: "common", colors: [] });
        await page.evaluate((m) => window.mtgDraftDebug.stage(m), { type: "review", picks: [
          { round: 1, pick: 1, card: "Chapel Geist", auto: false, pack: [card("Doomed Traveler"), card("Chapel Geist"), card("Moan of the Unhallowed")] },
          { round: 1, pick: 2, card: "Lost in the Mist", auto: false, pack: [card("Lost in the Mist"), card("Grimgrin, Corpse-Born"), card("Abattoir Ghoul")] },
          { round: 1, pick: 3, card: "Typhoid Rats", auto: true, pack: [card("Typhoid Rats")] },
          { round: 2, pick: 1, card: "Brimstone Volley", auto: false, pack: [card("Brimstone Volley"), card("Geistflame")] },
        ] });
        await page.waitForTimeout(50);
        const rows = await page.$$eval("#review li", els => els.map(e => e.textContent.replace(/\u00a0/g, " ")));
        check(await page.$$eval("#review ol", els => els.length) === 0, `${tag}: the review's rows carry their pick and no second counter (#889)`);
        check(rows.length === 4 && rows[0] === "1.1 Chapel Geist — over Doomed Traveler; Moan of the Unhallowed",
          `${tag}: each pick is the card taken over the rest of its pack (${JSON.stringify(rows)})`);
        // "; " between the names: "Grimgrin, Corpse-Born" is one card (#925).
        check(rows[1] === "1.2 Lost in the Mist — over Grimgrin, Corpse-Born; Abattoir Ghoul", `${tag}: a name with a comma reads as one card (${rows[1]})`);
        check(rows[2].includes("the table's pick"), `${tag}: a pick the table made says so`);
        check((await page.$$eval("#review h4", els => els.map(e => e.textContent))).join(",") === "Pack 1,Pack 2", `${tag}: by pack`);
        const o = await overflow();
        check(o.scrollW <= o.clientW, `${tag}: the review fits sideways (${o.scrollW} <= ${o.clientW})`);
        // Every card of the review reads by a tap, the taken one and the
        // rest, at a phone's width too: a hover tooltip was the only
        // reading, and only of the taken card (#889).
        const readOf = () => page.$$eval("#review .review-detail", els => els.map(e => e.textContent));
        await page.click('#review [data-review-card="1.1:2"]');
        await page.waitForTimeout(50);
        let read = await readOf();
        check(read.length === 1 && read[0].includes("Moan of the Unhallowed text"), `${tag}: a tap on a card passed over reads it (${JSON.stringify(read)})`);
        await page.click('#review [data-review-card="2.1:0"]');
        await page.waitForTimeout(50);
        read = await readOf();
        check(read.length === 1 && read[0].includes("Brimstone Volley text"), `${tag}: a tap on a taken card reads it instead (${JSON.stringify(read)})`);
        await page.click('#review [data-review-card="2.1:0"]');
        await page.waitForTimeout(50);
        check((await readOf()).length === 0, `${tag}: a second tap closes it`);
        await page.click("#review-toggle");
      }

      // Enter on a focused review card is that card's, twice over, while
      // building: the redraw dropped the focus, and the second Enter moved
      // the pool card under the cursor (#889's review, #787's shape).
      {
        await stage(byName.building);
        await page.waitForTimeout(50);
        await page.keyboard.press("ArrowDown");
        await page.click("#review-toggle");
        await page.waitForTimeout(50);
        const card = (name) => ({ name, line: name, text: `${name} text`, rarity: "common", colors: [] });
        await page.evaluate((m) => window.mtgDraftDebug.stage(m), { type: "review", picks: [
          { round: 1, pick: 1, card: "Chapel Geist", auto: false, pack: [card("Doomed Traveler"), card("Chapel Geist")] }] });
        await page.waitForTimeout(50);
        const before = (await sent()).length;
        await page.focus('#review [data-review-card="1.1:0"]');
        await page.keyboard.press("Enter");
        await page.waitForTimeout(50);
        const opened = await page.$$eval("#review .review-detail", els => els.length);
        const focused = await page.evaluate(() => document.activeElement && document.activeElement.dataset.reviewCard);
        await page.keyboard.press("Enter");
        await page.waitForTimeout(50);
        const closed = await page.$$eval("#review .review-detail", els => els.length);
        check(opened === 1 && focused === "1.1:0", `${tag}: Enter on a review card reads it and keeps the focus (${opened}, ${focused})`);
        check(closed === 0 && (await sent()).length === before, `${tag}: a second Enter closes it and moves no pool card (${closed}, ${JSON.stringify((await sent()).slice(before))})`);
        await page.click("#review-toggle");
      }

      // The socket was refused, so the page is retrying with backoff.
      {
        await page.waitForTimeout(1200);
        const r = await page.evaluate(() => ({ reconnects: window.mtgDraft.reconnects, connected: window.mtgDraft.connected, conn: document.getElementById("conn").textContent }));
        check(r.reconnects >= 1 && !r.connected && /retrying/.test(r.conn), `${tag}: reconnecting with backoff (${r.reconnects} tries, "${r.conn}")`);
      }

      // But a join the server itself refused (a wrong key, no such seat)
      // is not retried: the link is wrong, and the page says so instead
      // of counting down to the next refusal for ever (the first
      // playtest: five retries in 2.5 s under a "wrong key" banner).
      {
        await page.evaluate(() => { window.mtgDraft.state.view = null; window.mtgDraft.view = null; });
        await stage({ type: "refused", reason: "wrong key for seat 0", echo: null });
        const before = await page.evaluate(() => window.mtgDraft.reconnects);
        await page.waitForTimeout(2500);
        const r = await page.evaluate(() => ({ reconnects: window.mtgDraft.reconnects, conn: document.getElementById("conn").textContent, banner: (document.querySelector(".banner.refusal") || {}).textContent }));
        check(r.reconnects <= before + 1, `${tag}: a refused join stops the reconnects (${before} before, ${r.reconnects} after)`);
        check(/refused — check the link/.test(r.conn), `${tag}: and the status line says to check the link ("${r.conn}")`);
        check((r.banner || "").includes("wrong key"), `${tag}: with the server's reason on the banner`);
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

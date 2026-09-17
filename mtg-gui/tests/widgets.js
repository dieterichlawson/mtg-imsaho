// Every widget renders and answers: synthetic prompts over a real view.
//
//   cargo build -p mtg-runner
//   NODE_PATH=$(npm root -g) node mtg-gui/tests/widgets.js [--shots DIR]
//
// A runner provides a real board (so ids, names and zones are genuine);
// the test then stages one decision per prompt kind through the page's
// debug hook, checks the widget the page chose, drives it to an answer,
// and checks the Action it would send. The seat never sees these answers:
// the socket is closed first. An unknown prompt kind must fall back to a
// list of the offered actions.

const { chromium } = require("playwright");
const { spawn } = require("child_process");
const path = require("path");
const fs = require("fs");

const root = path.resolve(__dirname, "..", "..");
const shotsDir = process.argv.includes("--shots") ? process.argv[process.argv.indexOf("--shots") + 1] : null;
const port = 8900 + Math.floor(Math.random() * 90);
let failures = 0;
function fail(msg) { console.error("FAIL: " + msg); failures++; }
function ok(msg) { console.log("ok: " + msg); }

async function main() {
  const runner = spawn(path.join(root, "target", "debug", "mtg-runner"),
    ["--p1", `gui:${port}`, "--p2", "random", "--seed", "11", "--deck1", "red-green", "--deck2", "white-black", "-q"],
    { cwd: root, stdio: ["ignore", "ignore", "pipe"] });
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
    const errors = [];
    page.on("pageerror", e => errors.push("pageerror: " + e.message));
    page.on("console", m => { if (m.type() === "error" && !/404/.test(m.text())) errors.push("console: " + m.text()); });
    for (let i = 0; i < 30; i++) {
      try { await page.goto(`http://127.0.0.1:${port}/`); break; } catch (e) { await page.waitForTimeout(200); }
    }
    await page.waitForFunction(() => window.mtg && window.mtg.decision, null, { timeout: 30000 });
    // Keep, play through to a board with lands and a creature or two, then cut the socket.
    await page.evaluate(() => window.mtgSend("MulliganKeep"));
    await page.waitForFunction(() => window.mtg && window.mtg.decision && window.mtg.ui.mode === "menu", null, { timeout: 30000 });
    for (let i = 0; i < 12; i++) {
      const done = await page.evaluate(() => {
        const m = window.mtg;
        if (!m.decision) return false;
        const you = m.view.you;
        const mine = m.view.battlefield.filter(p => p.controller === you);
        if (mine.filter(p => p.card_types.includes("Creature")).length >= 1 && mine.length >= 3) return true;
        // Play a land or cast a creature when offered, else pass.
        for (const [id, verbs] of m.ui.verbs || []) {
          const v = verbs.find(v => v.label === "Play land" || v.label.startsWith("Cast"));
          if (v) { v.run(); return false; }
        }
        window.mtgSend("PassPriority");
        return false;
      });
      if (done) break;
      await page.waitForFunction(() => window.mtg && window.mtg.decision, null, { timeout: 30000 }).catch(() => {});
    }
    await page.evaluate(() => { const m = window.mtg; m.ws.onclose = () => {}; m.ws.close(); });
    const shot = async (name) => { if (shotsDir) { fs.mkdirSync(shotsDir, { recursive: true }); await page.screenshot({ path: path.join(shotsDir, name) }); } };

    // Ids to build prompts from.
    const ids = await page.evaluate(() => {
      const m = window.mtg; const you = m.view.you;
      const mine = m.view.battlefield.filter(p => p.controller === you).map(p => p.object_id);
      const theirs = m.view.battlefield.filter(p => p.controller !== you).map(p => p.object_id);
      const creatures = m.view.battlefield.filter(p => p.card_types.includes("Creature")).map(p => p.object_id);
      return { you, opp: m.view.opponents[0].id, mine, theirs, creatures, hand: m.view.your_hand.map(c => c.object_id),
        library: m.view.your_library_cards.slice(0, 6).map(c => c.object_id), names: m.view.your_library_cards.slice(0, 30).map(c => c.name) };
    });
    if (ids.mine.length < 2 || ids.hand.length < 2) fail(`board too small to test with: ${JSON.stringify(ids)}`);

    const legal = (extra) => ({ actions: [], combat_prompt: null, castable_spells: [], activatable_abilities: [], context: "TEST", resolution_prompt: null, set_prompt: null, ...extra });
    const rc = (choice) => ({ ResolveChoice: { choice } });
    let seq = 100;
    const stage = async (name, legalObj, combat, expectMode) => {
      seq++;
      const res = await page.evaluate(({ seq, legalObj, combat }) => {
        try { window.mtgDebug.stage({ seq, legal: legalObj, combat }); return { mode: window.mtg.ui && window.mtg.ui.mode, hits: window.mtg.hits.length }; }
        catch (e) { return { error: e.message + "\n" + e.stack }; }
      }, { seq, legalObj, combat });
      if (res.error) { fail(`${name}: threw ${res.error}`); return false; }
      if (res.mode !== expectMode) { fail(`${name}: widget ${res.mode}, expected ${expectMode}`); return false; }
      await shot(`${name}.png`);
      return true;
    };
    const lastSent = () => page.evaluate(() => { const s = window.mtgDebug.sent; return s.length ? s[s.length - 1] : null; });
    const clickHit = async (pred) => {
      const box = await page.evaluate((src) => {
        const f = new Function("h", "m", `return (${src})(h, m)`);
        window.mtgDebug.render();
        const h = window.mtg.hits.slice().reverse().find(h => f(h, window.mtg));
        return h ? [h.x + h.w / 2, h.y + h.h / 2] : null;
      }, pred);
      if (!box) throw new Error("nothing to click for " + pred);
      await page.mouse.move(box[0] * 2, box[1] * 2); await page.waitForTimeout(30);
      await page.mouse.click(box[0] * 2, box[1] * 2); await page.waitForTimeout(80);
    };
    const expectSent = async (name, pred) => {
      const s = await lastSent();
      if (!s || s.seq !== seq) { fail(`${name}: nothing sent for seq ${seq} (last: ${JSON.stringify(s)})`); return; }
      if (!pred(s.action)) fail(`${name}: sent ${JSON.stringify(s.action)}`); else ok(`${name} → ${JSON.stringify(s.action).slice(0, 90)}`);
    };

    // 1. ChooseTarget: a board pick over two permanents and a player, plus a decline.
    {
      const opts = [{ Object: ids.mine[0] }, { Object: ids.theirs[0] || ids.mine[1] }, { Player: ids.opp }];
      const actions = [...opts.map(t => rc({ ChosenTarget: t })), rc({ ChosenTarget: null })];
      if (await stage("choose-target", legal({ actions, resolution_prompt: { ChooseTarget: { description: "Deal 3 damage to any target", options: opts, optional: true, effect: "Destroy" } } }), null, "pick")) {
        await clickHit(`(h) => h.kind === 'player' && h.pid === ${ids.opp}`);
        await expectSent("choose-target", a => a.ResolveChoice && a.ResolveChoice.choice.ChosenTarget && a.ResolveChoice.choice.ChosenTarget.Player === ids.opp);
      }
    }
    // 2. ChooseCardFromHand: pick a hand card on the board.
    {
      const actions = ids.hand.map(id => rc({ ChosenCard: id }));
      if (await stage("choose-from-hand", legal({ actions, resolution_prompt: { ChooseCardFromHand: { description: "Discard a card", player: ids.you, cards: ids.hand, discard_immediately: true, remaining: 1 } } }), null, "pick")) {
        await clickHit(`(h) => h.kind === 'hand' && h.id === ${ids.hand[1]}`);
        await expectSent("choose-from-hand", a => a.ResolveChoice && a.ResolveChoice.choice.ChosenCard === ids.hand[1]);
      }
    }
    // 3. ChooseFromLibrary: cards not on the board become rows in the panel.
    {
      const actions = [...ids.library.map(id => rc({ ChosenCard: id })), rc({ ChosenTarget: null })];
      if (await stage("choose-from-library", legal({ actions, resolution_prompt: { ChooseFromLibrary: { description: "Search for a card", options: ids.library, searcher: ids.you, source_id: ids.mine[0], destination: "Hand", tapped: false } } }), null, "pick")) {
        const rows = await page.evaluate(() => window.mtg.ui.rows.length);
        if (rows !== ids.library.length) fail(`choose-from-library: ${rows} rows for ${ids.library.length} library cards`);
        await clickHit("(h, m) => h.kind === 'row' && h.y === Math.min(...m.hits.filter(x => x.kind === 'row').map(x => x.y))");
        await expectSent("choose-from-library", a => a.ResolveChoice && a.ResolveChoice.choice.ChosenCard === ids.library[0]);
      }
    }
    // 4. ChooseCardName: a long list with a filter.
    {
      const names = [...new Set(ids.names)];
      const actions = names.map((n, i) => rc({ ChosenIndex: [i, n] }));
      if (await stage("choose-card-name", legal({ actions, resolution_prompt: { ChooseCardName: { description: "Name a card", options: names, source_id: ids.mine[0] } } }), null, "list")) {
        await page.keyboard.type(names[names.length - 1].slice(0, 5));
        await page.waitForTimeout(100);
        await clickHit("(h, m) => h.kind === 'row'");
        await expectSent("choose-card-name", a => a.ResolveChoice && a.ResolveChoice.choice.ChosenIndex && names[a.ResolveChoice.choice.ChosenIndex[0]].startsWith(names[names.length - 1].slice(0, 5)));
      }
    }
    // 5. ChooseTriggerOrder: reorder, then confirm as one ChosenOrder.
    {
      const options = ["Doomed Traveler's trigger", "Mausoleum Guard's trigger", "Elder Cathar's trigger"];
      const actions = options.map((o, i) => rc({ ChosenIndex: [i, o] }));
      if (await stage("choose-trigger-order", legal({ actions, resolution_prompt: { ChooseTriggerOrder: { description: "Order the triggers", options, ap_queue: true, indices: [0, 1, 2], details: [] } } }), null, "order")) {
        await clickHit("(h, m) => h.kind === 'button' && h.label === '▼' && h.y === Math.min(...m.hits.filter(x => x.kind === 'button' && x.label === '▼').map(x => x.y))"); // first row down
        await clickHit("(h) => h.kind === 'button' && h.label === 'Confirm'");
        await expectSent("choose-trigger-order", a => a.ResolveChoice && JSON.stringify(a.ResolveChoice.choice.ChosenOrder) === "[1,0,2]");
      }
    }
    // 6. ChooseTargetSet: mark up to two, confirm; then cancel.
    {
      const opts = ids.creatures.slice(0, 3).map(id => ({ Object: id }));
      if (opts.length >= 2 && await stage("choose-target-set", legal({ resolution_prompt: { ChooseTargetSet: { description: "Up to two target creatures", options: opts, min: 0, max: 2, source_id: ids.hand[0], fixed: [] } } }), null, "mark")) {
        await clickHit(`(h) => h.id === ${opts[0].Object} && h.onClick`);
        await clickHit(`(h) => h.id === ${opts[1].Object} && h.onClick`);
        await clickHit("(h) => h.kind === 'button' && h.label === 'Confirm'");
        await expectSent("choose-target-set", a => a.ResolveChoice && a.ResolveChoice.choice.ChosenTargetSet && a.ResolveChoice.choice.ChosenTargetSet.length === 2);
        seq++;
        await page.evaluate(({ seq, opts }) => window.mtgDebug.stage({ seq, legal: { actions: [], combat_prompt: null, castable_spells: [], activatable_abilities: [], context: "T", resolution_prompt: { ChooseTargetSet: { description: "d", options: opts, min: 0, max: 2, source_id: 1, fixed: [] } }, set_prompt: null }, combat: null }), { seq, opts });
        await page.keyboard.press("Escape");
        await expectSent("choose-target-set cancel", a => a.ResolveChoice && a.ResolveChoice.choice === "CancelCast");
      }
    }
    // 7. ChooseExileFromGraveyard with an empty board zone: rows in the panel, exact count.
    {
      const gy = ids.library.slice(0, 3); // stand-ins: the page only needs ids it can name
      if (await stage("choose-exile", legal({ resolution_prompt: { ChooseExileFromGraveyard: { description: "Exile two creature cards", options: gy, min: 2, max: 2, source_id: ids.hand[0] } } }), null, "mark")) {
        const disabled = await page.evaluate(() => { const b = window.mtg.hits.find(h => h.kind === 'button' && h.label === 'Confirm'); return b && !b.onClick; });
        if (!disabled) fail("choose-exile: Confirm enabled with nothing marked");
        await clickHit("(h, m) => h.kind === 'row' && h.y === Math.min(...m.hits.filter(x => x.kind === 'row').map(x => x.y))");
        await clickHit("(h, m) => h.kind === 'row' && h.y === Math.max(...m.hits.filter(x => x.kind === 'row').map(x => x.y))");
        await clickHit("(h) => h.kind === 'button' && h.label === 'Confirm'");
        await expectSent("choose-exile", a => a.ResolveChoice && a.ResolveChoice.choice.ChosenExileSet && a.ResolveChoice.choice.ChosenExileSet.length === 2);
      }
    }
    // 8. ChooseXFunding: X = 2 from two single-mana sources.
    {
      const options = { pool: { Red: 1 }, groups: [{ name: "Forest", category: "BasicLand", source_ids: [ids.mine[0], ids.mine[1]], mana_per_tap: 1, mana_type: "Green" }], max_x: 3, x_discount: 0 };
      if (await stage("choose-x", legal({ resolution_prompt: { ChooseXFunding: { description: "Choose X for Devil's Play", options, source_id: ids.hand[0], is_ability: false } } }), null, "number")) {
        await page.keyboard.type("2");
        await page.keyboard.press("Enter");
        await expectSent("choose-x", a => { const f = a.ResolveChoice && a.ResolveChoice.choice.XFunding; return f && f.pool.Red === 1 && f.taps.Forest === 1; });
      }
    }
    // 9. PayOrNot: the two offered actions as a list.
    {
      const actions = [rc({ PayDecision: true }), rc({ PayDecision: false })];
      if (await stage("pay-or-not", legal({ actions, resolution_prompt: { PayOrNot: { description: "Pay {1}?", spell_id: 1, source_spell_id: 2, cost: { symbols: [{ Generic: 1 }] } } } }), null, "list")) {
        await clickHit("(h, m) => h.kind === 'row' && h.y === Math.max(...m.hits.filter(x => x.kind === 'row').map(x => x.y))");
        await expectSent("pay-or-not", a => a.ResolveChoice && a.ResolveChoice.choice.PayDecision === false);
      }
    }
    // 10. A set prompt: bottom one of the hand.
    {
      if (await stage("bottom", legal({ context: "BOTTOM 1", set_prompt: { kind: "BottomAfterMulligan", player: ids.you, options: ids.hand, min: 1, max: 1 } }), null, "mark")) {
        await clickHit(`(h) => h.kind === 'hand' && h.id === ${ids.hand[0]}`);
        await clickHit("(h) => h.kind === 'button' && h.label === 'Confirm'");
        await expectSent("bottom", a => a.BottomCards && a.BottomCards.cards[0] === ids.hand[0]);
      }
    }
    // 11. Combat: attackers with a planeswalker to choose, and blockers with menace.
    {
      const mineCreatures = ids.creatures.filter(id => ids.mine.includes(id));
      const theirCreatures = ids.creatures.filter(id => ids.theirs.includes(id));
      if (mineCreatures.length) {
        const combat = { ChooseAttackers: { eligible: mineCreatures, must_attack: [], defending_player: ids.opp, defending_planeswalkers: [ids.theirs[0]].filter(Boolean) } };
        if (await stage("attackers", legal({ context: "DECLARE ATTACKERS" }), combat, "attackers")) {
          await clickHit(`(h) => h.id === ${mineCreatures[0]} && h.onClick`);
          if (ids.theirs[0]) {
            await clickHit(`(h) => h.id === ${mineCreatures[0]} && h.onClick`); // cycle to the planeswalker
            await clickHit("(h) => h.kind === 'button' && h.label.startsWith('Attack')");
            await expectSent("attackers", a => a.DeclareAttackers && a.DeclareAttackers.planeswalker_attacks.length === 1 && a.DeclareAttackers.attackers.length === 0);
          } else {
            await clickHit("(h) => h.kind === 'button' && h.label.startsWith('Attack')");
            await expectSent("attackers", a => a.DeclareAttackers && a.DeclareAttackers.attackers.length === 1);
          }
        }
        const attackers = theirCreatures.length ? theirCreatures : [ids.theirs[0]].filter(Boolean);
        if (attackers.length) {
          const legalBlocks = {}; for (const b of mineCreatures) legalBlocks[b] = attackers;
          const combatB = { ChooseBlockers: { eligible_blockers: mineCreatures, attackers, legal_blocks: legalBlocks, min_blockers: { [attackers[0]]: 2 } } };
          if (await stage("blockers", legal({ context: "DECLARE BLOCKERS" }), combatB, "blockers")) {
            await clickHit(`(h) => h.id === ${mineCreatures[0]} && h.onClick`);
            await clickHit(`(h) => h.id === ${attackers[0]} && h.onClick`);
            const blocked = await page.evaluate(() => window.mtg.ui.assignments.size);
            if (blocked !== 1) fail(`blockers: ${blocked} assignments after pairing`);
            // One blocker on a menace attacker: Confirm is refused.
            const disabled = await page.evaluate(() => { window.mtgDebug.render(); const b = window.mtg.hits.find(h => h.kind === 'button' && h.label.startsWith('Block')); return b && !b.onClick; });
            if (!disabled) fail("blockers: a single blocker on a menace attacker was confirmable");
            await clickHit(`(h) => h.id === ${mineCreatures[0]} && h.onClick`); // unassign
            await clickHit("(h) => h.kind === 'button' && h.label.startsWith('No block')");
            await expectSent("blockers", a => a.DeclareBlockers && a.DeclareBlockers.assignments.length === 0);
          }
        }
      } else ok("no creature of ours yet; combat widgets skipped");
    }
    // 12. An unknown prompt kind: the offered actions as a list, never nothing.
    {
      const actions = [rc({ ChosenIndex: [0, "Left"] }), rc({ ChosenIndex: [1, "Right"] })];
      if (await stage("unknown-kind", legal({ actions, resolution_prompt: { ChooseSomethingNew: { description: "A prompt from the future", options: ["Left", "Right"] } } }), null, "list")) {
        await clickHit("(h, m) => h.kind === 'row' && h.y === Math.max(...m.hits.filter(x => x.kind === 'row').map(x => x.y))");
        await expectSent("unknown-kind", a => a.ResolveChoice && a.ResolveChoice.choice.ChosenIndex[1] === "Right");
      }
    }
    // 13. The priority menu with a cast that needs a target, and an ability with a sacrifice.
    {
      const target = ids.theirs[0] || ids.mine[1];
      const cs = { object_id: ids.hand[0], name: "Test Bolt", is_flashback: false, target_spec: { SingleTarget: [{ Object: target }, { Player: ids.opp }] }, tap_plan: [[ids.mine[0], 0]], exile_x_from_gy_max: null, sacrifice_options: [], additional_cost_label: null, alternative_cost: null, from_graveyard: false };
      const actions = ["PassPriority", "Concede", { CastSpell: { object_id: ids.hand[0], targets: [{ Object: target }], sacrifice: null, exile_count: null, exile_ids: [], alternative_cost: null, tap_plan: [[ids.mine[0], 0]] } }];
      if (await stage("menu-cast", legal({ context: "MAIN PHASE 1", actions, castable_spells: [cs] }), null, "menu")) {
        await clickHit(`(h) => h.kind === 'hand' && h.id === ${ids.hand[0]}`);
        await clickHit("(h, m) => h.kind === 'row' && m.popover");
        const mode = await page.evaluate(() => window.mtg.ui.mode);
        if (mode !== "pick") fail(`menu-cast: after choosing Cast the widget is ${mode}, expected pick`);
        await clickHit(`(h) => h.kind === 'player' && h.pid === ${ids.opp}`);
        await expectSent("menu-cast", a => a.CastSpell && a.CastSpell.targets[0].Player === ids.opp && a.CastSpell.tap_plan.length === 1);
      }
      const ab = { object_id: ids.mine[0], ability_index: 0, source_card_id: null, name: "Sac outlet", description: "Sacrifice a creature: draw a card", target_options: [], tap_plan: [],
        option_combos: [{ targets: [], sacrifice: ids.mine[0] }, { targets: [], sacrifice: ids.mine[1] }] };
      const actions2 = ["PassPriority", { ActivateAbility: { object_id: ids.mine[0], ability_index: 0, targets: [], tap_plan: [], sacrifice: ids.mine[0], x_value: null, source_card_id: null } }];
      if (await stage("menu-ability", legal({ context: "MAIN PHASE 1", actions: actions2, activatable_abilities: [ab] }), null, "menu")) {
        await clickHit(`(h) => h.kind === 'perm' && h.id === ${ids.mine[0]}`);
        await clickHit("(h, m) => h.kind === 'row' && m.popover");
        // Two identical untapped basics stack on the board, so the pick
        // may show only one of the two; any highlighted permanent will do.
        await clickHit("(h, m) => h.kind === 'perm' && h.onClick && m.ui.mode === 'pick'");
        await expectSent("menu-ability", a => a.ActivateAbility && [ids.mine[0], ids.mine[1]].includes(a.ActivateAbility.sacrifice));
      }
      // Enter passes.
      if (await stage("menu-pass", legal({ context: "MAIN PHASE 1", actions: ["PassPriority", "Concede"] }), null, "menu")) {
        await page.keyboard.press("Enter");
        await expectSent("menu-pass", a => a === "PassPriority");
      }
    }
    // 14. Enter at a mark prompt: refuses out loud below the minimum, and
    // never commits an empty answer nobody chose. Enter is the idle key —
    // in menu mode it passes priority — so landing on an "up to N" mark
    // with that habit used to throw the whole optional effect away in one
    // keystroke, and below the minimum it did nothing and said nothing
    // (issues #518, #520, #524).
    const notice = () => page.evaluate(() => window.mtg.notice);
    const expectNothingSent = async (name) => {
      const s = await lastSent();
      if (s && s.seq === seq) fail(`${name}: sent ${JSON.stringify(s.action)} — the prompt should have refused`);
      else ok(`${name}: nothing sent`);
    };
    {
      // (a) below the minimum — the DISCARD 1 CARD dead end.
      if (await stage("mark-enter-below-min", legal({ context: "DISCARD 1 CARD", set_prompt: { kind: "DiscardToHandSize", player: ids.you, options: ids.hand, min: 1, max: 1 } }), null, "mark")) {
        for (let i = 0; i < 3; i++) await page.keyboard.press("Enter");
        await page.waitForTimeout(60);
        await expectNothingSent("mark-enter-below-min");
        const n = await notice();
        if (!n || !/mark exactly 1 card/.test(n)) fail(`mark-enter-below-min: notice was ${JSON.stringify(n)}`);
        else ok(`mark-enter-below-min: said "${n}"`);
        // And it is still answerable by marking one.
        await clickHit(`(h) => h.kind === 'hand' && h.id === ${ids.hand[0]}`);
        await clickHit("(h) => h.kind === 'button' && h.label === 'Confirm'");
        await expectSent("mark-enter-below-min recovers", a => a.DiscardCards && a.DiscardCards.cards.length === 1);
      }
      // (b) min 0 — the idle key must not be an answer (#262's rule).
      const gy = ids.library.slice(0, 3);
      if (await stage("mark-enter-at-min-zero", legal({ resolution_prompt: { ChooseExileFromGraveyard: { description: "Exile up to three cards", options: gy, min: 0, max: 3, source_id: ids.hand[0] } } }), null, "mark")) {
        await page.keyboard.press("Enter");
        await page.waitForTimeout(60);
        await expectNothingSent("mark-enter-at-min-zero");
        const n = await notice();
        if (!n || !/nothing marked/.test(n)) fail(`mark-enter-at-min-zero: notice was ${JSON.stringify(n)}`);
        else ok(`mark-enter-at-min-zero: said "${n}"`);
        // Saying none on purpose still works, and says so on a button.
        await clickHit("(h) => h.kind === 'button' && h.label === 'Confirm none'");
        await expectSent("mark-confirm-none", a => a.ResolveChoice && a.ResolveChoice.choice.ChosenExileSet && a.ResolveChoice.choice.ChosenExileSet.length === 0);
      }
      // (c) once the player has marked something and unmarked it again,
      // the empty answer IS theirs, and Enter takes it.
      if (await stage("mark-enter-after-touching", legal({ resolution_prompt: { ChooseExileFromGraveyard: { description: "Exile up to three cards", options: gy, min: 0, max: 3, source_id: ids.hand[0] } } }), null, "mark")) {
        await clickHit("(h, m) => h.kind === 'row' && h.y === Math.min(...m.hits.filter(x => x.kind === 'row').map(x => x.y))");
        await clickHit("(h, m) => h.kind === 'row' && h.y === Math.min(...m.hits.filter(x => x.kind === 'row').map(x => x.y))");
        await page.keyboard.press("Enter");
        await page.waitForTimeout(60);
        await expectSent("mark-enter-after-touching", a => a.ResolveChoice && a.ResolveChoice.choice.ChosenExileSet && a.ResolveChoice.choice.ChosenExileSet.length === 0);
      }
      // (d) the order widget: every arrangement is legal, so Enter answers.
      {
        const options = ["Doomed Traveler's trigger", "Mausoleum Guard's trigger"];
        const actions = options.map((o, i) => rc({ ChosenIndex: [i, o] }));
        if (await stage("order-enter", legal({ actions, resolution_prompt: { ChooseTriggerOrder: { description: "Order the triggers", options, ap_queue: true, indices: [0, 1], details: [] } } }), null, "order")) {
          await page.keyboard.press("Enter");
          await page.waitForTimeout(60);
          await expectSent("order-enter", a => a.ResolveChoice && JSON.stringify(a.ResolveChoice.choice.ChosenOrder) === "[0,1]");
        }
      }
    }
    if (errors.length) fail("page errors:\n" + errors.join("\n"));
  } finally {
    await browser.close();
    runner.kill("SIGTERM");
  }
  console.log(failures ? `widgets: ${failures} FAILED` : "widgets: ok");
  process.exitCode = failures ? 1 : 0;
}

main().catch(e => { fail(e.stack || String(e)); process.exitCode = 1; });

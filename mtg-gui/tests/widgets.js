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
        library: m.view.your_library_cards.slice(0, 6).map(c => c.object_id),
        libraryMany: m.view.your_library_cards.slice(0, 40).map(c => c.object_id),
        names: m.view.your_library_cards.slice(0, 30).map(c => c.name) };
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
    // 8b. The X box reads what the terminal reads (#561). `Number("")` is
    //     0 and `Number.isInteger(0)` is true, so a bare Enter at an empty
    //     box used to announce X = 0 and complete the cast — #123's exact
    //     hazard, on the one prompt where the idle key commits instead of
    //     declining, whose fix went into cli.rs and never reached here.
    //     `Number` also took `0x2`, `0b11`, `2.0`, `1e1` and `-0`, none of
    //     which `str::parse::<u32>()` takes.
    {
      const options = { pool: { Red: 1 }, groups: [{ name: "Forest", category: "BasicLand", source_ids: [ids.mine[0], ids.mine[1]], mana_per_tap: 1, mana_type: "Green" }], max_x: 3, x_discount: 0 };
      const xPrompt = () => legal({ resolution_prompt: { ChooseXFunding: { description: "Choose X for Devil's Play", options, source_id: ids.hand[0], is_ability: false } } });
      // Every one of these the terminal refuses; so must the page.
      for (const token of ["", " ", "-0", "0x2", "0b11", "2.0", "1e1", "1_0", "2abc", "  "]) {
        if (!await stage(`x-refuses-${JSON.stringify(token)}`, xPrompt(), null, "number")) continue;
        if (token !== "") await page.keyboard.type(token);
        await page.keyboard.press("Enter");
        await page.waitForTimeout(60);
        const s = await lastSent();
        if (s && s.seq === seq) {
          fail(`x-refuses: ${JSON.stringify(token)} was ACCEPTED as ${JSON.stringify(s.action)}`);
        } else {
          const notice = await page.evaluate(() => window.mtg.notice || null);
          if (!notice) fail(`x-refuses: ${JSON.stringify(token)} refused with no message`);
          else ok(`x-refuses ${JSON.stringify(token)} → ${JSON.stringify(notice)}`);
        }
      }
      // And the forms it does take, which `str::parse::<u32>()` takes too.
      for (const [token, funded] of [["2", 2], ["+2", 2], ["002", 2], ["  3  ", 3], ["0", 0]]) {
        if (!await stage(`x-accepts-${JSON.stringify(token)}`, xPrompt(), null, "number")) continue;
        await page.keyboard.type(token);
        await page.keyboard.press("Enter");
        await expectSent(`x-accepts ${JSON.stringify(token)}`, a => {
          const f = a.ResolveChoice && a.ResolveChoice.choice.XFunding;
          if (!f) return false;
          const total = Object.values(f.pool || {}).reduce((n, v) => n + v, 0)
            + Object.values(f.taps || {}).reduce((n, v) => n + v, 0);
          return total === funded;
        });
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
    // 15. The inspector: what it is about, and what it says.
    //
    // The facts and the P/T lines are read back through the page's own
    // `mtgDebug.inspect`, which resolves the hovered thing exactly as the
    // renderer does. The view is edited in place first, so a board these
    // seeds do not reach can still be asked about.
    {
      const perm = ids.mine[0];
      // (a) an activated ability on the stack carries its SOURCE
      // permanent's id (view.rs), and the page indexed the stack last, so
      // the ability replaced the permanent: hovering the Ghoulcaller's Bell
      // on the battlefield showed the ability and none of the permanent.
      const both = await page.evaluate((perm) => {
        const m = window.mtg;
        const p = m.view.battlefield.find(x => x.object_id === perm);
        m.view.stack = [{ object_id: perm, card_id: 1, name: `${p.name} ability`, controller: m.view.you, targets: [], x_value: null }];
        const chip = window.mtgDebug.inspect(`o${perm}`, "stack");
        const board = window.mtgDebug.inspect(`o${perm}`, "perm");
        m.view.stack = [];
        return { chip, board, permName: p.name };
      }, perm);
      if (!both.chip || both.chip.zone !== "stack" || !/ ability$/.test(both.chip.name))
        fail(`stack-chip-inspect: ${JSON.stringify(both.chip)}`);
      else ok(`stack-chip-inspect → ${both.chip.name} (${both.chip.zone})`);
      if (!both.board || both.board.zone !== "battlefield" || both.board.name !== both.permName)
        fail(`source-permanent-inspect: an ability on the stack overwrote its source — ${JSON.stringify(both.board)}`);
      else ok(`source-permanent-inspect → ${both.board.name} (${both.board.zone})`);

      // (b) two triggers at once are two different things, though the view
      // gives them both ObjectId(0).
      const triggers = await page.evaluate(() => {
        const m = window.mtg;
        m.view.stack = [
          { object_id: 0, card_id: 0, name: "Doomed Traveler's dies trigger (a)", controller: m.view.you, targets: [], x_value: null },
          { object_id: 0, card_id: 0, name: "Elder Cathar's dies trigger (b)", controller: m.view.you, targets: [], x_value: null },
        ];
        window.mtgDebug.render();
        const chips = window.mtg.hits.filter(h => h.kind === "stack");
        const out = chips.map(h => { const was = m.hover; m.hover = h; const r = window.mtgDebug.inspectHover(); m.hover = was; return r && r.name; });
        m.view.stack = [];
        return out;
      });
      if (triggers.length !== 2 || triggers[0] === triggers[1])
        fail(`two-triggers-inspect: both chips resolved to ${JSON.stringify(triggers)}`);
      else ok(`two-triggers-inspect → ${JSON.stringify(triggers)}`);

      // (c) the facts the CLI's detail page carries and the panel dropped.
      const facts = await page.evaluate((perm) => {
        const m = window.mtg;
        const p = m.view.battlefield.find(x => x.object_id === perm);
        const saved = JSON.stringify(p);
        Object.assign(p, { is_token: true, colors: ["Blue"], regeneration_shields: 6,
          star_pt: true, printed_power: 0, printed_toughness: 0,
          effective_power: 4, effective_toughness: 4, card_types: ["Creature"] });
        const aura = JSON.parse(JSON.stringify(p));
        Object.assign(aura, { object_id: 99001, name: "Cobbled Wings", attached_to: perm,
          is_token: false, regeneration_shields: 0, star_pt: false, card_types: ["Artifact"] });
        m.view.battlefield.push(aura);
        const r = window.mtgDebug.inspect(`o${perm}`, "perm");
        const colorless = (() => { p.colors = []; const x = window.mtgDebug.inspect(`o${perm}`, "perm"); return x && x.facts; })();
        m.view.battlefield.pop();
        Object.assign(p, JSON.parse(saved));
        return { r, colorless };
      }, perm);
      const want = [
        ["Token", /^Token$/],
        ["Color", /^Color: Blue$/],
        ["regeneration shields", /^6 regeneration shields$/],
        ["attachments", /^Equipped\/enchanted with: Cobbled Wings$/],
      ];
      for (const [what, re] of want) {
        if (!facts.r || !facts.r.facts.some(f => re.test(f))) fail(`inspector-${what}: facts were ${JSON.stringify(facts.r && facts.r.facts)}`);
        else ok(`inspector-${what}`);
      }
      if (!facts.colorless || !facts.colorless.some(f => f === "Color: Colorless")) fail(`inspector-colorless: ${JSON.stringify(facts.colorless)}`);
      else ok("inspector-colorless");
      // The star-P/T sentinel is never shown as a printed value.
      if (!facts.r || facts.r.pt[1] !== "(printed */*)") fail(`inspector-star-pt: pt was ${JSON.stringify(facts.r && facts.r.pt)}`);
      else ok(`inspector-star-pt → ${JSON.stringify(facts.r.pt)}`);
    }
    // 16. A stack item's art is its source's. The engine names a stack item
    // for a person — "<card> ability", "<source>'s <phrase> (<desc>)" — and
    // the lookup used to strip at the FIRST "'s ", so every activated
    // ability and every possessive card missed (issue #528).
    {
      const cases = await page.evaluate(() => {
        const probe = (n) => window.mtgDebug.artNames(n);
        return {
          ability: probe("Ghoulcaller's Bell ability"),
          plain: probe("Cobbled Wings ability"),
          possessiveTrigger: probe("Geistcatcher's Rig's enters-the-battlefield trigger (deal 4 damage)"),
          simpleTrigger: probe("Doomed Traveler's dies trigger (create a 1/1 white Spirit token with flying)"),
          possessiveAbility: probe("Ludevic's Test Subject ability"),
          spell: probe("Ghoulcaller's Bell"),
          withId: probe("Unruly Mob (#34)'s triggered ability"),
        };
      });
      const expect = [
        ["ability", "Ghoulcaller's Bell"],
        ["plain", "Cobbled Wings"],
        ["possessiveTrigger", "Geistcatcher's Rig"],
        ["simpleTrigger", "Doomed Traveler"],
        ["possessiveAbility", "Ludevic's Test Subject"],
        ["spell", "Ghoulcaller's Bell"],
        ["withId", "Unruly Mob"],
      ];
      for (const [k, want] of expect) {
        if (!cases[k] || !cases[k].includes(want)) fail(`artNames ${k}: ${JSON.stringify(cases[k])} does not offer ${JSON.stringify(want)}`);
        else ok(`artNames ${k} → ${want}`);
      }
    }
    // 17. Everything printed fits. `docs/playtest/README.md`: "A row wider
    // than its pane is wrapped or clipped deliberately, never printed over
    // the border into the next pane." Swept over the whole card pool and
    // every turn/step line the band can produce, because the failures were
    // found one card and one step at a time (#522, #532).
    {
      const names = Object.keys(JSON.parse(fs.readFileSync(path.join(root, "data", "oracle_cache.json"), "utf8")).cards);
      // The twelve steps, and the words the band prints for them. The words
      // are duplicated here on purpose: the primary assertion measures what
      // `bandLine` actually returns, and these are only used to show the
      // sweep is exercising a line that really is too wide untreated.
      const steps = [["Untap", "untap"], ["Upkeep", "upkeep"], ["Draw", "draw"], ["PrecombatMain", "main phase 1"],
        ["BeginCombat", "begin combat"], ["DeclareAttackers", "declare attackers"], ["DeclareBlockers", "declare blockers"],
        ["CombatDamage", "combat damage"], ["EndCombat", "end of combat"], ["PostcombatMain", "main phase 2"],
        ["EndStep", "end step"], ["Cleanup", "cleanup"]];
      const bad = await page.evaluate(({ names, steps }) => {
        const f = window.mtgDebug.fit;
        const out = { inspector: [], hand: [], band: [], unmarked: [] };
        // The inspector's name block: panel 160 wide, art 64, padding 12.
        const tw = 160 - (64 + 12);
        for (const n of names) {
          for (const l of f.wrapCapped(n, tw, "8px PressStart", 3)) {
            if (f.width(l, "8px PressStart") > tw) out.inspector.push([n, l, f.width(l, "8px PressStart")]);
          }
          const hand = f.wrapCapped(n, 66 - 6, "8px Silkscreen", 2);
          for (const l of hand) {
            if (f.width(l, "8px Silkscreen") > 66 - 6) out.hand.push([n, l, f.width(l, "8px Silkscreen")]);
          }
          // A name that needed more lines than it got says so.
          if (f.wrap(n, 66 - 6, "8px Silkscreen").length > 2 && !hand[hand.length - 1].endsWith("…")) out.unmarked.push([n, hand]);
        }
        // The band's turn/step line, as the renderer builds it.
        const bandW = f.bandW;
        let everOver = 0;
        for (const mine of [true, false]) for (const [st, words] of steps) {
          const drawn = f.bandLine(mine, st);
          if (f.width(drawn, "7px Silkscreen") > bandW) out.band.push([mine, st, drawn, f.width(drawn, "7px Silkscreen")]);
          const raw = `${mine ? "YOUR TURN" : "OPPONENT'S TURN"} · ${words}`;
          if (f.width(raw, "7px Silkscreen") > bandW) everOver++;
        }
        // The sweep has to be exercising something: at least one step's
        // untreated line really is wider than the block.
        out.bandLive = everOver;
        return out;
      }, { names, steps });
      const live = bad.bandLive; delete bad.bandLive;
      for (const [what, rows] of Object.entries(bad)) {
        if (rows.length) fail(`fit-${what}: ${rows.length} over the pane, e.g. ${JSON.stringify(rows.slice(0, 3))}`);
        else ok(`fit-${what}: all ${what === "band" ? 24 : names.length} fit`);
      }
      if (!live) fail("fit-band-live: no untreated line is over the block — the sweep proves nothing");
      else ok(`fit-band-live: ${live} of 24 untreated lines really are over the block`);
    }
    // 18. The log drawer's heading is not printed over by the log.
    //
    // A differential check rather than a colour one: if the heading and the
    // log's first visible line share a row, the heading's row of pixels
    // changes when the log's contents change. It must not (issue #521).
    {
      const same = await page.evaluate(() => {
        const m = window.mtg;
        const c = document.getElementById("game").getContext("2d");
        const saved = m.view.display_log.slice();
        const wasOpen = m.logOpen, wasScroll = m.logScroll;
        m.logOpen = true; m.logScroll = 0;
        const headingRow = () => {
          window.mtgDebug.render();
          return Array.from(c.getImageData(0, 360 - 119, 480, 9).data).join(",");
        };
        m.view.display_log = ["Game started (p1 on the play)"];
        const withLog = headingRow();
        m.view.display_log = [];
        const empty = headingRow();
        m.view.display_log = saved; m.logOpen = wasOpen; m.logScroll = wasScroll;
        window.mtgDebug.render();
        return { same: withLog === empty, len: withLog.length };
      });
      if (!same.same) fail("log-drawer-heading: the heading's row changes with the log's contents — they are drawn over each other");
      else ok("log-drawer-heading: the heading has a row of its own");
    }
    // 19. A long list can be reached to its end, and says where you are.
    //
    // A 30-card library search drew eleven rows in the panel and neither
    // drew nor mentioned the other nineteen — legal answers the engine had
    // offered that a person could not send (issue #529).
    {
      // Distinct ids: the picker keys its options by id, so repeats collapse.
      const thirty = ids.libraryMany.slice(0, 30);
      const actions = thirty.map(id => rc({ ChosenCard: id }));
      actions.push(rc({ ChosenTarget: null }));
      if (await stage("library-pager", legal({ actions, resolution_prompt: { ChooseFromLibrary: { description: "Search your library for a card", options: thirty, searcher: ids.you, source_id: ids.mine[0], destination: "Hand", tapped: false } } }), null, "pick")) {
        const page1 = await page.evaluate(() => {
          window.mtgDebug.render();
          return { rows: window.mtg.ui.rows.length, drawn: window.mtg.hits.filter(h => h.kind === "row").length, scroll: window.mtg.rowScroll || 0 };
        });
        if (page1.rows <= page1.drawn) fail(`library-pager: the list fits (${page1.rows} rows, ${page1.drawn} drawn) — nothing to page`);
        else ok(`library-pager: ${page1.drawn} of ${page1.rows} drawn on the first page`);
        // Wheel to the end. The renderer clamps, so overshooting is safe and
        // the last row must be reachable.
        const last = await page.evaluate(() => {
          const m = window.mtg;
          m.rowScroll = 9999;
          window.mtgDebug.render();
          return m.rowPage;
        });
        if (!last) fail("library-pager: the panel published no page");
        else if (last.scroll + last.drawn !== last.total)
          fail(`library-pager: the last page stops at ${last.scroll + last.drawn} of ${last.total}`);
        else ok(`library-pager: the last page reaches row ${last.total} of ${last.total}`);
        // And the wheel over the panel is what moves it.
        await page.mouse.move(560 * 2, 300 * 2);
        await page.mouse.wheel(0, -600);
        await page.waitForTimeout(80);
        const backUp = await page.evaluate(() => window.mtg.rowScroll || 0);
        if (!last || backUp >= last.scroll) fail(`library-pager: wheeling up over the panel left scroll at ${backUp}`);
        else ok(`library-pager: the wheel over the panel scrolls the rows (${last.scroll} → ${backUp})`);
      }
    }
    // 20. Filtering a scrolled list does not empty it, and the footer says
    // what is on screen (issue #530); and the input is where the modal
    // painted its box (issue #531).
    {
      // Duplicates are fine here: a ChooseCardName row is one per index.
      const names = ids.names;
      if (names.length >= 25) {
        const actions = names.map((n, i) => rc({ ChosenIndex: [i, n] }));
        if (await stage("filter-after-scroll", legal({ actions, resolution_prompt: { ChooseCardName: { description: "Choose a card name", options: names, source_id: ids.mine[0] } } }), null, "list")) {
          // A query that matches something, chosen from the rows themselves.
          const q = names[names.length - 1].slice(0, 4);
          const res = await page.evaluate((q) => {
            const m = window.mtg;
            m.ui.scroll = 10;                   // as the wheel would leave it
            m.ui.query = q;                     // then a filter narrows the list
            window.mtgDebug.render();
            const matches = m.ui.rows.filter(r => r.label.toLowerCase().includes(q.toLowerCase())).length;
            return { drawn: m.hits.filter(h => h.kind === "row").length, matches, scroll: m.ui.scroll };
          }, q);
          if (res.matches === 0) fail(`filter-after-scroll: the probe filter "${q}" matched nothing`);
          else if (res.drawn === 0) fail(`filter-after-scroll: ${res.matches} rows match "${q}" and none was drawn (scroll ${res.scroll})`);
          else ok(`filter-after-scroll: ${res.drawn} of ${res.matches} matching rows drawn, scroll clamped to ${res.scroll}`);

          // The DOM input sits over the box the modal painted, not at the
          // top of the canvas over the opponent's life strip.
          const field = await page.evaluate(() => {
            const m = window.mtg;
            const el = document.querySelector("input");
            const r = document.getElementById("game").getBoundingClientRect();
            const b = el.getBoundingClientRect();
            return { field: [(b.left - r.left) / m.scale, (b.top - r.top) / m.scale],
                     painted: m.fieldRect ? [m.fieldRect.x, m.fieldRect.y] : null };
          });
          if (!field.painted) fail("filter-box: the modal published no field rectangle");
          else if (Math.abs(field.field[0] - field.painted[0]) > 2 || Math.abs(field.field[1] - field.painted[1]) > 2)
            fail(`filter-box: the input is at ${JSON.stringify(field.field)} and the painted box at ${JSON.stringify(field.painted)}`);
          else ok(`filter-box: the input is over the painted box at ${JSON.stringify(field.painted)}`);
        }
      } else fail(`only ${names.length} library rows to build a pageable list from`);
    }
    // 21. The engine's p0/p1 in the page's vocabulary, and an outcome line
    // a person who only ever saw the browser can read (issue #519).
    {
      const r = await page.evaluate(() => {
        const you = window.mtg.view.you, opp = window.mtg.view.opponents[0].id;
        const w = window.mtgDebug.words;
        return {
          you, opp,
          started: w(`Game started (p${opp} on the play)`),
          drew: w(`p${you} drew 7 cards`),
          banner: w(`\u2500\u2500 Turn 5 (p${opp}) \u2500\u2500`),
          attack: w(`p${opp} declared attackers: Walking Corpse (#66) -> p${you}`),
          stranger: w("p7 did something"),
          card: w("Doom Blade (#75) resolved"),
          win: window.mtgDebug.outcome(`Game over! p${you} (red-green) wins! (p${opp} (white-black) lost the game: life total reached 0 (CR 704.5a))`),
          lose: window.mtgDebug.outcome(`Game over! p${opp} (white-black) wins! (p${you} (red-green) conceded)`),
          draw: window.mtgDebug.outcome("Game over! It's a draw! (both players lost)"),
          odd: window.mtgDebug.outcome("Game ended without a result."),
        };
      });
      const want = [
        ["started", `Game started (opp on the play)`],
        ["drew", "you drew 7 cards"],
        ["banner", "\u2500\u2500 Turn 5 (opp) \u2500\u2500"],
        ["attack", "opp declared attackers: Walking Corpse (#66) -> you"],
        ["stranger", "p7 did something"],
        ["card", "Doom Blade (#75) resolved"],
        ["win", "YOU WIN"],
        ["lose", "OPPONENT WINS"],
        ["draw", "A DRAW"],
        ["odd", null],
      ];
      for (const [k, v] of want) {
        if (r[k] !== v) fail(`seat-vocabulary ${k}: got ${JSON.stringify(r[k])}, expected ${JSON.stringify(v)}`);
        else ok(`seat-vocabulary ${k} → ${JSON.stringify(r[k])}`);
      }
    }
    // 22. The band reports what happened since the page last stopped for
    // the player, not the last two lines of the log (issue #523).
    {
      const r = await page.evaluate(() => {
        const m = window.mtg;
        const saved = m.view.display_log.slice();
        const since = m.logSince, seen = m.logSeen;
        const out = {};
        const at = (log, from) => { m.view.display_log = log; m.logSince = from; return window.mtgDebug.band(); };
        const eight = ["\u2500\u2500 Turn 5 (p1) \u2500\u2500", "p1 tapped Swamp (#51) for mana", "p1 tapped Plains (#41) for mana",
          "p1 cast Doom Blade (#75) targeting Grizzly Bears (#27)", "Doom Blade (#75) resolved",
          "Grizzly Bears (#27) died", "p1 drew a card", "p1 declared attackers: Walking Corpse (#66) -> p0"];
        out.wholeTurn = at(["older", "lines"].concat(eight), 2);
        out.two = at(["older"].concat(eight.slice(-2)), 1);
        out.one = at(["older"].concat(eight.slice(-1)), 1);
        out.none = at(["older"].concat(eight.slice(-2)), 3);
        m.view.display_log = saved; m.logSince = since; m.logSeen = seen;
        return out;
      });
      // Eight new lines in two rows: where the interval began, how much is
      // missing, and where it ended.
      if (r.wholeTurn.length !== 2 || !/Turn 5/.test(r.wholeTurn[0]) || !/^\+6 · /.test(r.wholeTurn[1]) || !/declared attackers/.test(r.wholeTurn[1]))
        fail(`band-recap wholeTurn: ${JSON.stringify(r.wholeTurn)}`);
      else ok(`band-recap wholeTurn → ${JSON.stringify(r.wholeTurn)}`);
      if (r.two.length !== 2 || /^\+/.test(r.two[1])) fail(`band-recap two: ${JSON.stringify(r.two)}`);
      else ok(`band-recap two: both shown, uncounted`);
      if (r.one.length !== 1) fail(`band-recap one: ${JSON.stringify(r.one)}`);
      else ok("band-recap one: the single new line");
      // Nothing new since the last stop: the band is not left blank.
      if (r.none.length === 0) fail("band-recap none: the band went blank while sitting at a prompt");
      else ok(`band-recap none: falls back to the last lines (${r.none.length})`);
    }
    // 23. The battlefield row has a fit contract: identical permanents
    // collapse, the row never reaches past the pane, and nothing off the
    // pane answers a click (issue #513).
    {
      const sweep = await page.evaluate(() => {
        const m = window.mtg, you = m.view.you;
        const proto = m.view.battlefield.find(p => p.controller === you && p.card_types.includes("Creature"));
        if (!proto) return null;
        const others = m.view.battlefield.filter(p => p !== proto);
        const rows = [];
        for (const distinct of [false, true]) {
          for (const n of [1, 12, 30, 44, 45, 46, 60, 108]) {
            const clones = [];
            for (let i = 0; i < n; i++) clones.push(Object.assign({}, proto, {
              object_id: 90000 + i, name: distinct ? `Creature ${i}` : proto.name }));
            m.view.battlefield = others.concat(clones);
            window.mtgDebug.render();
            const perms = m.hits.filter(h => h.kind === "perm" && h.id >= 90000);
            const maxRight = perms.length ? Math.max(...perms.map(h => h.x + h.w)) : 0;
            rows.push({ distinct, n, drawn: perms.length, maxRight,
                        offPane: perms.filter(h => h.x + h.w > 480).length });
          }
        }
        m.view.battlefield = others.concat([proto]);
        window.mtgDebug.render();
        return rows;
      });
      if (!sweep) fail("board-fit: no creature on the board to clone");
      else {
        const over = sweep.filter(r => r.maxRight > 480);
        if (over.length) fail(`board-fit: the row reaches past the pane: ${JSON.stringify(over.slice(0, 3))}`);
        else ok(`board-fit: every row of up to 108 ends inside the pane (widest ${Math.max(...sweep.map(r => r.maxRight))})`);
        const clickable = sweep.filter(r => r.offPane);
        if (clickable.length) fail(`board-fit: ${JSON.stringify(clickable.slice(0, 3))} answer clicks from under the panel`);
        else ok("board-fit: nothing off the pane answers a click");
        const same = sweep.filter(r => !r.distinct);
        if (same.some(r => r.drawn !== 1)) fail(`board-fit: identical permanents did not collapse: ${JSON.stringify(same)}`);
        else ok("board-fit: 108 identical tokens are one stack");
        // And distinct permanents are still drawn, not collapsed away.
        const twelve = sweep.find(r => r.distinct && r.n === 12);
        if (!twelve || twelve.drawn !== 12) fail(`board-fit: 12 distinct creatures drew ${twelve && twelve.drawn}`);
        else ok("board-fit: 12 distinct creatures are 12 cards");
      }
    }
    // 24. Nothing the inspector paints leaves the canvas or its own pane.
    //
    // `fillText` does not clip, so an over-long string is simply painted
    // over — and past x=640 there is no canvas, so the glyphs do not exist.
    // The inspector's P/T line bypassed the `wrap` -> `clip` rule #532 put
    // in, and a 13/13 with two digits of damage read "13/13 10 dm" with the
    // "g" at x=640..648 (issue #568). Ludevic's Abomination is a printed
    // 13/13 with 13 toughness, so 10-12 damage marked is reachable.
    {
      const drawn = await page.evaluate(() => {
        const m = window.mtg, you = m.view.you;
        const p = m.view.battlefield.find(x => x.controller === you);
        if (!p) return null;
        const keep = JSON.parse(JSON.stringify(p));
        const orig = CanvasRenderingContext2D.prototype.fillText;
        const out = [];
        // Every P/T the inspector can be handed for a real card, with and
        // without damage, plus a pumped one so the "(printed N/N)" line is
        // drawn too.
        const cases = [[2, 2, 0], [13, 13, 5], [13, 13, 10], [13, 13, 12], [100, 100, 20]];
        try {
          CanvasRenderingContext2D.prototype.fillText = function (s, x, y) {
            const w = this.measureText(String(s)).width;
            const left = this.textAlign === "center" ? x - w / 2 : this.textAlign === "right" ? x - w : x;
            out.push({ s: String(s), left, right: left + w, font: this.font });
            return orig.apply(this, arguments);
          };
          for (const [pw, th, dmg] of cases) {
            Object.assign(p, { name: "Ludevic's Abomination", effective_power: pw, effective_toughness: th,
              power: pw, toughness: th, printed_power: pw === 13 ? 13 : 5, printed_toughness: th === 13 ? 13 : 5,
              damage_marked: dmg });
            window.mtgDebug.inspect("o" + p.object_id);
            m.hover = m.hits.slice().reverse().find(x => x.key === "o" + p.object_id && x.kind === "perm") || m.hover;
            window.mtgDebug.render();
          }
        } finally {
          CanvasRenderingContext2D.prototype.fillText = orig;
          Object.assign(p, keep);
          window.mtgDebug.render();
        }
        return out;
      });
      if (!drawn) fail("inspector-fit: no permanent on the board to inspect");
      else {
        const off = drawn.filter(d => d.right > 640 || d.left < 0);
        if (off.length) fail(`inspector-fit: painted past the canvas edge: ${JSON.stringify(off.slice(0, 3))}`);
        else ok(`inspector-fit: all ${drawn.length} strings stay on the canvas`);
        // The P/T line shares the name's text box, which starts at x=552
        // and is 84px wide. The name ellipsizes there; the P/T did not.
        const inBox = drawn.filter(d => d.left >= 552 && d.left < 640);
        const spill = inBox.filter(d => d.right > 552 + 84);
        if (spill.length) fail(`inspector-fit: past the 84px text box: ${JSON.stringify(spill.slice(0, 3))}`);
        else ok(`inspector-fit: all ${inBox.length} lines in the inspector's text box stay in it`);
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

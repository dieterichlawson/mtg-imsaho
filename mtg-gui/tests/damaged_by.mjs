// The inspector says what on the battlefield dealt a creature its damage (#932).
//
//   node mtg-gui/tests/damaged_by.mjs
//
// Abattoir Ghoul gains its controller life when a creature it dealt damage
// to this turn dies, so a removal spell after combat turns on which
// creature dealt the damage. The board showed the damage and not its source.

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const P = await import("../dist/prompts.js");
const R = await import("../dist/render.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };

const creature = (id, name, controller, damage, damaged_by) => ({ object_id: id, name, controller, owner: controller,
  card_types: ["Creature"], attacking: null, blocking: [], blocked_by: [], blocked: false, counters: {},
  keywords: [], protections: [], restrictions: [], colors: ["Blue"], granted_abilities: [], oracle_text: "",
  damage_marked: damage, damaged_by });
// #90 was a spell, gone from the battlefield.
const ghoul = creature(6, "Abattoir Ghoul", 0, 0, []);
const crab = creature(44, "Fortress Crab", 1, 3, [6, 90]);
const burnt = creature(45, "Fortress Crab", 1, 2, [90]);
// Regenerated: no damage marked, and the Ghoul still triggers when it dies.
const regenerated = creature(46, "Fortress Crab", 1, 0, [6]);
// A planeswalker records its sources too; no surface names them.
const walker = { ...creature(47, "Garruk Relentless", 1, 0, [6]), card_types: ["Planeswalker"] };
const view = { you: 0, opponents: [{ id: 1 }], battlefield: [ghoul, crab, burnt, regenerated, walker], your_hand: [], stack: [], exile: [],
  graveyards: [], your_library_cards: [], revealed_names: {} };
const state = { view, index: P.indexView(view), overlay: null };
const facts = id => R.inspectorFacts(state, state.index.get(id));

if (!facts(44).includes("Damaged this turn by Abattoir Ghoul")) fail(`the Ghoul is named: ${JSON.stringify(facts(44))}`);
if (!facts(46).includes("Damaged this turn by Abattoir Ghoul")) fail(`and after a regeneration: ${JSON.stringify(facts(46))}`);
if (facts(45).some(f => f.startsWith("Damaged this turn"))) fail(`a spell is not: ${JSON.stringify(facts(45))}`);
if (facts(47).some(f => f.startsWith("Damaged this turn"))) fail(`nor a planeswalker: ${JSON.stringify(facts(47))}`);
if (facts(6).some(f => f.startsWith("Damaged this turn"))) fail(`nor anything undamaged: ${JSON.stringify(facts(6))}`);

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: the inspector says what dealt a creature its damage");

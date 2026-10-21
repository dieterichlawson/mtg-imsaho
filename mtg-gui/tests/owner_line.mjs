// The inspector says whose a stolen permanent is (#740).
//
//   node mtg-gui/tests/owner_line.mjs
//
// A creature taken with Olivia Voldaren's ability goes back to its owner
// when the control effect ends and to its owner's graveyard when it dies.
// The board drew it apart from its controller's own copies (#722) and the
// inspector never said why.

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const P = await import("../dist/prompts.js");
const R = await import("../dist/render.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };

const interloper = (id, controller, owner) => ({ object_id: id, name: "Vampire Interloper", controller, owner,
  card_types: ["Creature"], attacking: null, blocking: [], blocked_by: [], blocked: false, counters: {},
  keywords: ["Flying"], protections: [], restrictions: ["can't block"], colors: ["Black"], granted_abilities: [],
  oracle_text: "" });
const own = interloper(71, 1, 1), stolen = interloper(22, 1, 0), mine = interloper(30, 0, 0);
const view = { you: 0, opponents: [{ id: 1 }], battlefield: [own, stolen, mine], your_hand: [], stack: [], exile: [],
  graveyards: [], your_library_cards: [], revealed_names: {} };
const state = { view, index: P.indexView(view), overlay: null };
const facts = id => R.inspectorFacts(state, state.index.get(id));

if (!facts(22).includes("Owner: You")) fail(`the stolen Interloper names its owner: ${JSON.stringify(facts(22))}`);
if (facts(71).some(f => f.startsWith("Owner"))) fail(`the controller's own says nothing: ${JSON.stringify(facts(71))}`);
if (facts(30).some(f => f.startsWith("Owner"))) fail(`nor does yours: ${JSON.stringify(facts(30))}`);
// From the other seat, the same creature is the opponent's.
const other = { ...view, you: 1, opponents: [{ id: 0 }] };
const theirs = { view: other, index: P.indexView(other), overlay: null };
const f = R.inspectorFacts(theirs, theirs.index.get(22));
if (!f.includes("Owner: Opponent")) fail(`from the thief's seat: ${JSON.stringify(f)}`);

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: the inspector says whose a stolen permanent is");

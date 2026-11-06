// A blocker whose attacker has left combat still says it is blocking (#983).
//
//   node mtg-gui/tests/blocking_nothing.mjs
//
// It is still a blocking creature (CR 510.1d) to the end of combat, blocking
// nothing and dealing no combat damage, but `blocking` is empty, so the page
// dropped its BLK badge the moment the attacker died.

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const P = await import("../dist/prompts.js");
const R = await import("../dist/render.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };

const inquisitor = { object_id: 41, name: "Elite Inquisitor", controller: 1, owner: 1, card_types: ["Creature"],
  attacking: null, blocking: [], is_blocking: true, blocked_by: [], blocked: false, counters: {}, keywords: ["FirstStrike"],
  protections: [], restrictions: [], colors: ["White"], granted_abilities: [], oracle_text: "" };
const view = { you: 0, opponents: [{ id: 1 }], battlefield: [inquisitor], your_hand: [], stack: [], exile: [],
  graveyards: [], your_library_cards: [], revealed_names: {} };
const state = { view, index: P.indexView(view), overlay: null };

if (!R.permBadges(inquisitor, state).map(b => b.t).includes("BLK")) fail("a blocker blocking nothing keeps BLK");
const facts = R.inspectorFacts(state, state.index.get(41));
if (!facts.some(f => f.startsWith("Blocking nothing"))) fail(`the inspector says it blocks nothing: ${JSON.stringify(facts)}`);
if (R.stackKey(inquisitor, new Set()) !== null) fail("a blocking creature is its own card, not stacked");

const idle = { ...inquisitor, is_blocking: false };
if (R.permBadges(idle, state).map(b => b.t).includes("BLK")) fail("a creature out of combat has no BLK");
if (R.inspectorFacts({ ...state, index: P.indexView({ ...view, battlefield: [idle] }) },
  { obj: idle, zone: "battlefield", owner: 1 }).some(f => f.startsWith("Blocking"))) fail("and is not called blocking");

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: a blocker whose attacker left combat still says it is blocking");

// An attacker whose blockers have all left combat says it is blocked (#725).
//
//   node mtg-gui/tests/blocked.mjs
//
// It stays blocked (CR 509.1h) and deals no combat damage, but `blocked_by`
// is empty, so the page drew a plain ATK badge and "Attacking You".

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const P = await import("../dist/prompts.js");
const R = await import("../dist/render.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };

const corpse = { object_id: 67, name: "Walking Corpse", controller: 1, owner: 1, card_types: ["Creature"],
  attacking: { Player: 0 }, blocking: [], blocked_by: [], blocked: true, counters: {}, keywords: [],
  protections: [], restrictions: [], colors: ["Black"], granted_abilities: [], oracle_text: "" };
const view = { you: 0, opponents: [{ id: 1 }], battlefield: [corpse], your_hand: [], stack: [], exile: [],
  graveyards: [], your_library_cards: [], revealed_names: {} };
const state = { view, index: P.indexView(view), overlay: null };

const badges = R.permBadges(corpse, state).map(b => b.t);
if (badges.includes("ATK") || !badges.includes("BLKD")) fail(`badges say blocked: ${JSON.stringify(badges)}`);
const facts = R.inspectorFacts(state, state.index.get(67));
if (!facts.some(f => f.startsWith("Blocked:"))) fail(`the inspector says blocked: ${JSON.stringify(facts)}`);

const free = { ...corpse, blocked: false };
if (!R.permBadges(free, state).map(b => b.t).includes("ATK")) fail("an unblocked attacker is ATK");
if (R.inspectorFacts({ ...state, index: P.indexView({ ...view, battlefield: [free] }) },
  { obj: free, zone: "battlefield", owner: 1 }).some(f => f.startsWith("Blocked"))) fail("and is not called blocked");

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: a blocked attacker with no blockers left says so");

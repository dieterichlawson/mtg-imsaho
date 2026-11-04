// A planeswalker whose loyalty ability is spent this turn says so on the
// board, beside its loyalty (#981).
//
//   node mtg-gui/tests/walker_used.mjs
//
// Two Lilianas at the legend rule are told apart by loyalty and by which one
// has activated (CR 606.3); the page showed the second only on hover.

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const P = await import("../dist/prompts.js");
const R = await import("../dist/render.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };

const lili = (id, loyalty, used) => ({ object_id: id, name: "Liliana of the Veil", controller: 0, owner: 0,
  card_types: ["Planeswalker"], attacking: null, blocking: [], is_blocking: false, blocked_by: [], blocked: false,
  counters: { Loyalty: loyalty }, keywords: [], protections: [], restrictions: [], colors: ["Black"],
  granted_abilities: [], oracle_text: "", once_per_turn_used: used });
const used = lili(2, 1, true), fresh = lili(3, 3, false);
const view = { you: 0, opponents: [{ id: 1 }], battlefield: [used, fresh], your_hand: [], stack: [], exile: [],
  graveyards: [], your_library_cards: [], revealed_names: {} };
const state = { view, index: P.indexView(view), overlay: null };

const a = R.permBadges(used, state).map(b => b.t), b = R.permBadges(fresh, state).map(b => b.t);
if (!a.includes("L1") || !a.includes("USED")) fail(`the spent one: ${JSON.stringify(a)}`);
if (!b.includes("L3") || b.includes("USED")) fail(`the fresh one: ${JSON.stringify(b)}`);

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: a planeswalker that has activated this turn is marked USED");

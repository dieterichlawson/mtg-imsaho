// Every untapped Forest answers "Tap for mana", not only the offered one (#1003).
//
//   node mtg-gui/tests/mana_copies.mjs
//
// The engine offers one `ActivateManaAbility` per card for five untapped
// Forests — the CLI's menu and the LLM's schema want one row per card — and
// lists the other copies that could tap in `mana_ability_copies`. The page
// hung the verb on the offered Forest only, so it split off the stack and
// the rest drew as a verb-less "x4" that answered no click. Each copy now
// offers its own tap, which sends that copy; a tapped Forest, never in the
// list, offers nothing.

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const P = await import("../dist/prompts.js");
const { offerKey } = await import("../dist/render.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };

const forest = (object_id, tapped) => ({ object_id, card_id: 7, name: "Forest", controller: 0, owner: 0,
  tapped, card_types: ["Land"], mana_abilities: [[0, "Add {G}"]] });
const view = { you: 0, opponents: [{ id: 1 }], stack: [], exile: [], graveyards: [[0, []], [1, []]],
  battlefield: [forest(3, false), forest(4, false), forest(5, false), forest(6, true)],
  your_hand: [], your_library_cards: [], revealed_names: {} };
const tap = id => ({ ActivateManaAbility: { object_id: id, ability_index: 0 } });
const state = { view, index: P.indexView(view), overlay: null,
  decision: { legal: { actions: [tap(3), "PassPriority", "Concede"], mana_ability_copies: [tap(4), tap(5)],
    castable_spells: [], activatable_abilities: [] } } };

const sent = [];
const ui = P.beginDecision(state, a => sent.push(a));
if (ui.mode !== "menu") fail(`a priority decision is the menu, got ${ui.mode}`);
for (const id of [3, 4, 5]) {
  const label = ui.verbs.get(id)?.[0]?.label || "";
  if (!label.includes("Add {G}")) fail(`untapped Forest #${id} offers its tap, got "${label}"`);
}
if (ui.verbs.has(6)) fail("a tapped Forest offers nothing");
if (offerKey(ui, 3) !== offerKey(ui, 4) || offerKey(ui, 4) !== offerKey(ui, 5)) fail("the untapped Forests key one stack");
ui.verbs.get(5)[0].run();
if (sent.length !== 1 || sent[0].ActivateManaAbility?.object_id !== 5) fail(`a copy taps itself: ${JSON.stringify(sent)}`);

// A page fed by an older seat (no field) still works.
const old = { ...state, decision: { legal: { ...state.decision.legal, mana_ability_copies: undefined } } };
const ui2 = P.beginDecision(old, () => {});
if (!ui2.verbs.has(3) || ui2.verbs.has(4)) fail("without the list only the offered Forest taps");

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: every Forest that could tap offers its tap, and a tapped one does not");

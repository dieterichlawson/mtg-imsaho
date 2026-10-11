// A hand choice offers one copy of each card; a click on any copy answers (#739).
//
//   node mtg-gui/tests/hand_copies.mjs
//
// The engine offers Civilized Scholar's discard as one row per distinct
// card, not per copy. The page answers it by a click in the hand, so a
// click on a copy the engine did not name has to send the copy it did,
// or four of five Scholars in the hand would not respond.

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const P = await import("../dist/prompts.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };

const card = (object_id, card_id, name) => ({ object_id, card_id, name });
const view = { you: 0, opponents: [{ id: 1 }], battlefield: [], stack: [], exile: [], graveyards: [],
  your_hand: [card(2, 1, "Island"), card(22, 1, "Island"), card(30, 7, "Civilized Scholar"),
    card(32, 7, "Civilized Scholar"), card(34, 7, "Civilized Scholar")],
  your_library_cards: [], revealed_names: {} };
const chosen = id => ({ ResolveChoice: { choice: { ChosenCard: id } } });
const state = { view, index: P.indexView(view), overlay: null,
  decision: { legal: { actions: [chosen(2), chosen(30)], context: "Civilized Scholar: choose a card to discard" } } };

if (JSON.stringify(P.copiesInHand(state, 30)) !== "[32,34]") fail(`copies of #30: ${JSON.stringify(P.copiesInHand(state, 30))}`);
if (JSON.stringify(P.copiesInHand(state, 99)) !== "[]") fail("a card not in hand has no copies there");

const sent = [];
const ui = P.beginDecision(state, a => sent.push(a));
for (const key of ["o2", "o22", "o30", "o32", "o34"]) {
  if (!ui.options.has(key)) fail(`${key} is clickable`);
}
ui.onPick("o34");
ui.onPick("o22");
if (JSON.stringify(sent) !== JSON.stringify([chosen(30), chosen(2)])) fail(`a copy sends the offered copy: ${JSON.stringify(sent)}`);

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: any copy in hand answers a hand choice");

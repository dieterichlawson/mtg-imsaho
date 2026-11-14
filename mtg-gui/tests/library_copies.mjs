// A library search offers one copy of each card; a click on any copy in
// the library overlay answers (#1004).
//
//   node mtg-gui/tests/library_copies.mjs
//
// Every library choice goes through the engine's one_of_each_card, so a
// search names one Forest of four. The page widened a click to the other
// copies only in the hand (#739): in the `d` overlay the other Forests
// answered nothing.

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const P = await import("../dist/prompts.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };

const card = (object_id, card_id, name) => ({ object_id, card_id, name });
const view = { you: 0, opponents: [{ id: 1 }], battlefield: [], stack: [], exile: [], graveyards: [[0, [card(80, 3, "Forest")]]],
  your_hand: [], revealed_names: {},
  your_library_cards: [card(50, 3, "Forest"), card(51, 3, "Forest"), card(52, 9, "Grizzly Bears"), card(53, 3, "Forest")] };
const rp = { ChooseFromLibrary: { description: "Caravan Vigil: search for a basic land card", options: [50, 52] } };

for (const [what, answer] of [["ChosenCard", id => ({ ChosenCard: id })], ["ChosenTarget", id => ({ ChosenTarget: { Object: id } })]]) {
  const act = id => ({ ResolveChoice: { choice: answer(id) } });
  const state = { view, index: P.indexView(view), overlay: null,
    decision: { legal: { actions: [act(50), act(52), { ResolveChoice: { choice: { ChosenTarget: null } } }], context: "search", resolution_prompt: rp } } };
  const sent = [];
  const ui = P.beginDecision(state, a => sent.push(a));
  for (const key of ["o50", "o51", "o52", "o53"]) if (!ui.options.has(key)) fail(`${what}: ${key} is not clickable`);
  if (ui.options.has("o80")) fail(`${what}: a Forest in the graveyard was lent the search's answer`);
  // One row per answer, not one per copy.
  const rows = (ui.rows || []).filter(r => r.key).map(r => r.key);
  if (JSON.stringify(rows) !== JSON.stringify(["o50", "o52"])) fail(`${what}: rows ${JSON.stringify(rows)}`);
  if (!state.overlay || state.overlay.zone !== "library") fail(`${what}: the library did not open: ${JSON.stringify(state.overlay)}`);
  ui.onPick("o53");
  if (JSON.stringify(sent) !== JSON.stringify([act(50)])) fail(`${what}: a copy sends the offered copy: ${JSON.stringify(sent)}`);
}

// A look at the top cards (Forbidden Alchemy) is not a search: a copy
// further down the library was never looked at, and answers nothing. A
// second Forest among the looked-at cards is the same answer as the one
// the engine offers (one_of_each_card), and answers like it; the prompt's
// `looked_at` says which cards those are.
{
  const act = id => ({ ResolveChoice: { choice: { ChosenCard: id } } });
  const state = { view, index: P.indexView(view), overlay: null,
    decision: { legal: { actions: [act(50), act(52)], context: "look",
      resolution_prompt: { ChooseFromLookedAt: { description: "Forbidden Alchemy: choose a card to put into your hand", looked_at: [50, 51, 52] } } } } };
  const sent = [];
  const ui = P.beginDecision(state, a => sent.push(a));
  const keys = JSON.stringify([...ui.options.keys()]);
  if (!ui.options.has("o50") || !ui.options.has("o52")) fail(`looked-at: the offers answer: ${keys}`);
  if (!ui.options.has("o51")) fail(`looked-at: the second looked-at Forest answers: ${keys}`);
  if (ui.options.has("o53") || ui.options.has("o80")) fail(`looked-at: a Forest not looked at answers: ${keys}`);
  const rows = (ui.rows || []).filter(r => r.key).map(r => r.key);
  if (JSON.stringify(rows) !== JSON.stringify(["o50", "o52"])) fail(`looked-at: one row per answer: ${JSON.stringify(rows)}`);
  ui.onPick("o51");
  if (JSON.stringify(sent) !== JSON.stringify([act(50)])) fail(`looked-at: a copy sends the offered copy: ${JSON.stringify(sent)}`);
}

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: any copy in the library answers a search");

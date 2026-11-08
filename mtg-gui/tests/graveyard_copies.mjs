// Any copy of an untargeted flashback card in your graveyard answers a click (#968).
//
//   node mtg-gui/tests/graveyard_copies.mjs
//
// The engine offers identical untargeted flashbacks once for every copy:
// with three Think Twice in the graveyard, `castable_spells` names one of
// them. The page hangs verbs off cards, so the other two drew no marker and
// answered no click in the `g` overlay. A copy borrows its namesake's verbs
// within its own zone (#572 did this for the hand) — never across zones,
// and never from an opponent's graveyard.

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const P = await import("../dist/prompts.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };

const card = (object_id, card_id, name) => ({ object_id, card_id, name });
const view = { you: 0, opponents: [{ id: 1 }], battlefield: [], stack: [], exile: [],
  graveyards: [
    [0, [card(33, 5, "Swords to Plowshares"), card(18, 9, "Think Twice"), card(19, 9, "Think Twice"), card(20, 9, "Think Twice")]],
    [1, [card(40, 9, "Think Twice")]],
  ],
  your_hand: [card(21, 9, "Think Twice")],
  your_library_cards: [], revealed_names: {} };
const flashback = { object_id: 18, name: "Think Twice", is_flashback: true, target_spec: "NoTargets",
  tap_plan: [], exile_x_from_gy_max: null, sacrifice_options: [], additional_cost_label: null,
  alternative_cost: null, from_graveyard: true };
const state = { view, index: P.indexView(view), overlay: null,
  decision: { legal: { actions: ["PassPriority"], castable_spells: [flashback], activatable_abilities: [] } } };

const sent = [];
const ui = P.beginDecision(state, a => sent.push(a));
if (ui.mode !== "menu") fail(`a priority decision is the menu, got ${ui.mode}`);
for (const id of [18, 19, 20]) {
  if (!ui.verbs.has(id)) fail(`Think Twice #${id} in your graveyard has the Flashback verb`);
}
for (const id of [33, 21, 40]) {
  if (ui.verbs.has(id)) fail(`#${id} borrows nothing (another card, your hand, the opponent's graveyard)`);
}
const label = ui.verbs.get(20)?.[0]?.label || "";
if (!label.startsWith("Flashback")) fail(`the borrowed verb is Flashback: ${label}`);
ui.verbs.get(20)[0].run();
if (sent.length !== 1 || sent[0].CastSpell?.object_id !== 18) fail(`a copy casts the offered copy: ${JSON.stringify(sent)}`);

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: any copy in your graveyard answers an untargeted flashback");

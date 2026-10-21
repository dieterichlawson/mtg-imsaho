// Naming a card for Nevermore: hovering a name reads its card (#846).
//
//   node mtg-gui/tests/card_names.mjs
//
// The question offers every nonland name in the pool — 253 of them — and
// most are cards in no zone, so the rows named nothing the page could look
// up and the inspector kept saying "Hover a card to read it." The view now
// carries each name's card for the seat being asked; a row names its card
// and the inspector draws it, as the terminal's browser does.

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const P = await import("../dist/prompts.js");
const R = await import("../dist/render.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };

const hound = { name: "Ashmouth Hound", cost: { symbols: [{ Generic: 1 }, { Colored: "Red" }] }, supertypes: [],
  card_types: ["Creature"], subtypes: ["Elemental", "Hound"], power: 2, toughness: 1, star_pt: false,
  oracle_text: "Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature." };
const monk = { name: "Geist-Honored Monk", cost: { symbols: [{ Generic: 3 }, { Colored: "White" }, { Colored: "White" }] },
  supertypes: [], card_types: ["Creature"], subtypes: ["Human", "Monk"], power: 0, toughness: 0, star_pt: true,
  oracle_text: "Vigilance" };
const view = { you: 0, opponents: [{ id: 1 }], battlefield: [], stack: [], exile: [], graveyards: [], your_hand: [],
  your_library_cards: [], revealed_names: {}, nameable_cards: { "Ashmouth Hound": hound, "Geist-Honored Monk": monk } };
const named = (i, n) => ({ ResolveChoice: { choice: { ChosenIndex: [i, n] } } });
const description = "Nevermore: choose a nonland card name (spells with that name can't be cast)";
const state = { view, index: P.indexView(view), overlay: null, hover: null, selected: null,
  decision: { legal: { actions: [named(0, "Ashmouth Hound"), named(1, "Geist-Honored Monk")], context: description,
    resolution_prompt: { ChooseCardName: { description, options: ["Ashmouth Hound", "Geist-Honored Monk"], source_id: 5 } } } } };

const sent = [];
const ui = P.beginDecision(state, a => sent.push(a));
if (ui.mode !== "list") fail(`a list of names: ${ui.mode}`);
const rows = ui.rows || [];
if (rows.map(r => r.cardName).join("|") !== "Ashmouth Hound|Geist-Honored Monk") fail(`each row names its card: ${JSON.stringify(rows.map(r => r.cardName))}`);
if (!/hover/i.test(ui.hint)) fail(`the hint says a name can be read: ${ui.hint}`);

// Hovering a row, as the canvas' mousemove leaves it: a row hit with the name.
state.hover = { x: 0, y: 0, w: 10, h: 10, kind: "row", cardName: "Ashmouth Hound" };
const e = R.inspecting(state);
if (!e || e.obj.name !== "Ashmouth Hound") fail(`the inspector reads the hovered name's card: ${JSON.stringify(e)}`);
else {
  if (e.obj.oracle_text !== hound.oracle_text) fail("its rules text");
  if (JSON.stringify(R.inspectorPt(e.obj)) !== JSON.stringify(["2/1"])) fail(`its P/T: ${JSON.stringify(R.inspectorPt(e.obj))}`);
  if (R.inspectorFacts(state, e).some(f => f.startsWith("In "))) fail(`it is in no zone, and says none: ${JSON.stringify(R.inspectorFacts(state, e))}`);
}
state.hover = { x: 0, y: 0, w: 10, h: 10, kind: "row", cardName: "Geist-Honored Monk" };
const m = R.inspecting(state);
if (!m || JSON.stringify(R.inspectorPt(m.obj)) !== JSON.stringify(["*/*"])) fail(`a */* card prints */*: ${m && JSON.stringify(R.inspectorPt(m.obj))}`);

rows[1].run();
if (JSON.stringify(sent) !== JSON.stringify([named(1, "Geist-Honored Monk")])) fail(`a click names it: ${JSON.stringify(sent)}`);

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: hovering a name to choose reads its card");

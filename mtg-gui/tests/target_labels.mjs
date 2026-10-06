// A target out of a graveyard says whose graveyard (#690).
//
//   node mtg-gui/tests/target_labels.mjs
//
// Purify the Grave offered "Grizzly Bears" four times — two in each
// graveyard — and opened only your graveyard's overlay, so a person could
// not tell their own card from the opponent's. The CLI (#669) and the LLM
// seat (#668) already said whose.

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const P = await import("../dist/prompts.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };

const view = { you: 0, opponents: [{ id: 1 }], battlefield: [], your_hand: [], stack: [], exile: [],
  graveyards: [[0, [{ object_id: 12, name: "Mountain" }]], [1, [{ object_id: 64, name: "Mountain" }]]],
  your_library_cards: [], revealed_names: {} };
const state = { view, index: P.indexView(view), overlay: null };
const ui = P.beginPick(state, { title: "Purify the Grave: choose a target",
  options: [{ Object: 12 }, { Object: 64 }], onPick: () => {} });
const labels = ui.rows.map(r => r.label);
if (JSON.stringify(labels) !== JSON.stringify(["Mountain (your graveyard)", "Mountain (opponent's graveyard)"]))
  fail(`rows say whose graveyard: ${JSON.stringify(labels)}`);
if (state.overlay !== null) fail(`one overlay cannot show two graveyards, but ${JSON.stringify(state.overlay)} opened`);
if (P.targetLabel(state, { Object: 64 }) !== "Mountain (opponent's graveyard)") fail("the stack's Targets: line says whose");

// One graveyard: its overlay opens, as before.
const one = { view, index: P.indexView(view), overlay: null };
P.beginPick(one, { title: "t", options: [{ Object: 64 }], onPick: () => {} });
if (JSON.stringify(one.overlay) !== JSON.stringify({ zone: "graveyard", pid: 1 })) fail(`the one graveyard opens: ${JSON.stringify(one.overlay)}`);

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: graveyard targets say whose graveyard");

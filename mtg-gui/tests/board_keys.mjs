// A player's key is no object (#801).
//
//   node mtg-gui/tests/board_keys.mjs
//
// Board hits are keyed `o<id>` for an object and `p<id>` for a player's
// strip. The menu read every key as `Number(key.slice(1))`, so `p1` was
// object #1: with a Mountain #1 in hand, the opponent's life strip was drawn
// in verb gold, a click on it opened "Play land", and hovering it showed the
// Mountain in the inspector.

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const { highlightFor, clickFor, inspecting } = await import("../dist/render.js");
const { objectOfKey } = await import("../dist/prompts.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };

if (objectOfKey("o12") !== 12) fail("o12 is object 12");
if (objectOfKey("p1") !== null) fail("p1 is a player, not object 1");
if (objectOfKey(null) !== null) fail("no key, no object");

const playLand = { label: "Play land", run: () => {} };
const mountain = { obj: { object_id: 1, name: "Mountain" }, zone: "hand", owner: 0 };
const state = {
  ui: { mode: "menu", title: "", hint: "", buttons: [], marked: [], verbs: new Map([[1, [playLand]]]) },
  view: { stack: [] }, index: new Map([[1, mountain]]), selected: null, hover: null, popover: null,
};

if (highlightFor(state, "o1") !== "verb") fail("the Mountain itself is lit with its verb");
if (clickFor(state, "o1") === null) fail("and a click on it opens its verbs");
if (highlightFor(state, "p1") !== null) fail(`the opponent's strip is lit as ${highlightFor(state, "p1")}`);
if (highlightFor(state, "p0") !== null) fail("your own strip is lit");
if (clickFor(state, "p1") !== null) fail("a click on the opponent's strip opens object #1's verbs");

state.hover = { kind: "player", key: "p1" };
if (inspecting(state) !== undefined) fail("hovering the opponent's strip shows object #1 in the inspector");
state.hover = { kind: "perm", key: "o1" };
if (inspecting(state) !== mountain) fail("hovering the Mountain shows it");

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("board_keys: ok");

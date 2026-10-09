// A copy is its own card on the board (#722).
//
//   node mtg-gui/tests/stack_key.mjs
//
// The board stacks permanents that look alike into one "xN" card, draws the
// first, and offers only that card's verbs. An Evil Twin copying your
// Walking Corpse keyed the same as the Corpse, so the two drew as one card
// and the ability the copy granted itself was on no card a click reached.

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const { stackKey, offerKey } = await import("../dist/render.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };

const corpse = id => ({
  object_id: id, card_id: 1, name: "Walking Corpse", supertypes: [], card_types: ["Creature"],
  controller: 0, owner: 0, tapped: false, power: 2, toughness: 2, effective_power: 2,
  effective_toughness: 2, damage_marked: 0, regeneration_shields: 0,
  affected_by_summoning_sickness: false, attached_to: null, attached_to_player: null,
  keywords: [], colors: ["Black"], subtypes: ["Zombie"], printed_power: 2, printed_toughness: 2,
  star_pt: false, is_token: false, is_copy: false, attacking: null, blocking: [], blocked_by: [],
  protections: [], restrictions: [], oracle_text: "", granted_abilities: [], counters: {},
  loyalty_abilities: [], mana_abilities: [], named_card: null,
});
const none = new Set();
const a = corpse(9001), b = corpse(9002);
if (stackKey(a, none) !== stackKey(b, none)) fail("two plain Walking Corpses still stack");
const twin = { ...corpse(9002), is_copy: true,
  granted_abilities: ["{U}{B}, {T}: Destroy target creature with the same name as this creature."] };
if (stackKey(a, none) === stackKey(twin, none)) fail("a copy with its own ability stacked with the card it copied");
if (stackKey(a, none) === stackKey({ ...corpse(9003), is_copy: true }, none)) fail("a copy stacked with its original");
if (stackKey(a, none) === stackKey({ ...corpse(9004), owner: 1 }, none)) fail("a borrowed card stacked with your own");

// What the open menu offers on each member splits a stack (#748): a Wolf
// whose once-a-turn pump is spent and one whose pump is on offer look
// alike, and the stack drew the spent one.
const pump = { label: "{2}{G}: +2/+2 until end of turn", run: () => {} };
const menu = { mode: "menu", verbs: new Map([[22, [pump]]]) };
if (offerKey(menu, 21) === offerKey(menu, 22)) fail("a Wolf with its pump on offer keyed with one without");
const lands = { mode: "menu", verbs: new Map([[3, [{ label: "Tap: Add {G}" }]], [4, [{ label: "Tap: Add {G}" }]]]) };
if (offerKey(lands, 3) !== offerKey(lands, 4)) fail("two Forests offered the same tap no longer stack");
if (offerKey({ mode: "pick", verbs: menu.verbs }, 22) !== "" || offerKey(null, 22) !== "") fail("only a menu's verbs key a stack");

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: a copy is its own card, and so is a member offered something the others are not");

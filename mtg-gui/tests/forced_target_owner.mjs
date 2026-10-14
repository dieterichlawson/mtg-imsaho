// A forced target's verb says whose a permanent is (#829).
//
//   node mtg-gui/tests/forced_target_owner.mjs
//
// With an Invisible Stalker on each side and only yours legal, the cast's
// verb read "Cast → Invisible Stalker", which names either one. The CLI's
// row says "(your)"; the page's verb says "(yours)".

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const P = await import("../dist/prompts.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };
const perm = (id, name, c) => ({ object_id: id, name, controller: c, owner: c, card_types: ["Creature"] });
const view = { you: 0, opponents: [{ id: 1 }], stack: [], exile: [], graveyards: [], your_library_cards: [], revealed_names: {},
  battlefield: [perm(30, "Invisible Stalker", 0), perm(31, "Invisible Stalker", 1), perm(40, "Bloodcrazed Neonate", 0)],
  your_hand: [{ object_id: 20, name: "Moment of Heroism", card_types: ["Instant"] }] };
const legal = {
  actions: ["PassPriority",
    { CastSpell: { object_id: 20, targets: [{ Object: 30 }], sacrifice: null, exile_count: null, exile_ids: [], alternative_cost: null, tap_plan: [] } },
    { ActivateAbility: { object_id: 40, ability_index: 0, targets: [{ Object: 31 }], tap_plan: [], sacrifice: null, x_value: null, source_card_id: null } },
    "Concede"],
  castable_spells: [{ object_id: 20, name: "Moment of Heroism", target_spec: { SingleTarget: [{ Object: 30 }] }, sacrifice_options: [],
    alternative_cost: null, tap_plan: [], is_flashback: false, from_graveyard: false, additional_cost_label: null }],
  activatable_abilities: [{ object_id: 40, ability_index: 0, source_card_id: null, name: "Bloodcrazed Neonate (#40)",
    description: "{T}: Tap target creature.", target_options: [], tap_plan: [], sorcery_speed: false,
    option_combos: [{ targets: [{ Object: 31 }], sacrifice: null }] }],
  combat_prompt: null, resolution_prompt: null, set_prompt: null, context: null };
const state = { view, index: P.indexView(view), overlay: null, notice: null, decision: { legal, combat: null } };
P.beginDecision(state, () => {});
const labels = [...state.ui.verbs.values()].flat().map(v => v.label);
if (!labels.some(l => l.endsWith("→ Invisible Stalker (yours)"))) fail(`the cast's verb says whose: ${JSON.stringify(labels)}`);
if (!labels.some(l => l.endsWith("→ Invisible Stalker (opponent's)"))) fail(`and so does the ability's: ${JSON.stringify(labels)}`);

// A loyalty ability's verb is one action per target, so each commits on a
// click too (#829's review); so does a committed target set as a row.
const loyalty = { ActivateLoyaltyAbility: { object_id: 40, ability_index: 0, targets: [{ Object: 31 }] } };
const row = P.describeAction(state, loyalty);
if (!row.endsWith("→ Invisible Stalker (opponent's)")) fail(`a loyalty verb says whose: ${row}`);
const set = P.describeAction(state, { ResolveChoice: { choice: { ChosenTargetSet: [{ Object: 30 }, { Object: 31 }] } } });
if (!set.includes("Invisible Stalker (yours), Invisible Stalker (opponent's)")) fail(`a committed set says whose: ${set}`);

if (failures) { console.error(`forced target owner: ${failures} FAILED`); process.exit(1); }
console.log("forced target owner: all checks passed");

// A cast's or an activation's verb says what auto-tap will spend (#842).
//
//   node mtg-gui/tests/tap_plan.mjs
//
// The CLI's row reads "Cast Ambush Viper (tap Forest, Swamp)" and the LLM
// seat's the same; the page sent the plan back with the click but its verb
// was the bare word "Cast", so a person could not see that the click would
// tap the land whose ability they were saving.

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const P = await import("../dist/prompts.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };
const perm = (id, name) => ({ object_id: id, name, controller: 0, owner: 0, card_types: ["Land"] });
const view = { you: 0, opponents: [{ id: 1 }], stack: [], exile: [], graveyards: [], your_library_cards: [], revealed_names: {},
  battlefield: [perm(11, "Forest"), perm(15, "Swamp"), perm(16, "Swamp"), perm(17, "Nephalia Drownyard"),
    { object_id: 40, name: "Bloodcrazed Neonate", controller: 0, owner: 0, card_types: ["Creature"] }],
  your_hand: [{ object_id: 21, name: "Ambush Viper", card_types: ["Creature"] }, { object_id: 22, name: "Doom Blade", card_types: ["Instant"] }] };
const cast = (id, plan) => ({ object_id: id, name: "", target_spec: "NoTargets", sacrifice_options: [],
  alternative_cost: null, tap_plan: plan, is_flashback: false, from_graveyard: false, additional_cost_label: null });
const legal = {
  actions: ["PassPriority",
    { CastSpell: { object_id: 21, targets: [], sacrifice: null, exile_count: null, exile_ids: [], alternative_cost: null, tap_plan: [[11, 0], [15, 0]] } },
    { CastSpell: { object_id: 22, targets: [], sacrifice: null, exile_count: null, exile_ids: [], alternative_cost: null, tap_plan: [[15, 0], [16, 0]] } },
    { ActivateAbility: { object_id: 40, ability_index: 0, targets: [], tap_plan: [[17, 0]], sacrifice: null, x_value: null, source_card_id: null } },
    "Concede"],
  castable_spells: [cast(21, [[11, 0], [15, 0]]), cast(22, [[15, 0], [16, 0]])],
  activatable_abilities: [{ object_id: 40, ability_index: 0, source_card_id: null, name: "Bloodcrazed Neonate (#40)",
    description: "{1}: Pump.", target_options: [], tap_plan: [[17, 0]], sorcery_speed: false,
    option_combos: [{ targets: [], sacrifice: null }] }],
  combat_prompt: null, resolution_prompt: null, set_prompt: null, context: null };
const state = { view, index: P.indexView(view), overlay: null, notice: null, decision: { legal, combat: null } };
P.beginDecision(state, () => {});
const verb = id => (state.ui.verbs.get(id) || []).map(v => v.label);
if (!verb(21).includes("Cast (tap Forest, Swamp)")) fail(`the cast's verb names the plan: ${JSON.stringify(verb(21))}`);
if (!verb(22).includes("Cast (tap 2x Swamp)")) fail(`two of one name count: ${JSON.stringify(verb(22))}`);
if (!verb(40).some(l => l.includes("(tap Nephalia Drownyard)"))) fail(`an ability's verb names the plan: ${JSON.stringify(verb(40))}`);

// The rows the list and pick screens build from raw actions say it too.
const row = P.describeAction(state, legal.actions[1]);
if (!row.includes("(tap Forest, Swamp)")) fail(`a cast row names the plan: ${row}`);
const ab = P.describeAction(state, legal.actions[3]);
if (!ab.includes("(tap Nephalia Drownyard)")) fail(`an ability row names the plan: ${ab}`);
// A flashback's cost note and the plan sit in separate parentheses, so the
// popover's clip keeps the plan (#842's review).
const fb = { ...cast(21, [[15, 0], [16, 0]]), is_flashback: true,
  alternative_cost: { symbols: [{ Generic: 1 }, { Colored: "Black" }] } };
const s2 = { ...state, decision: { legal: { ...legal, castable_spells: [fb] }, combat: null } };
P.beginDecision(s2, () => {});
const fbVerb = (s2.ui.verbs.get(21) || [])[0]?.label || "";
if (!/^Flashback \(flashback cost [^)]*\) \(tap 2x Swamp\)$/.test(fbVerb)) fail(`a flashback's plan has its own parentheses: ${fbVerb}`);
// An empty plan adds nothing.
if (P.tapNote(state, []) !== "") fail("an empty plan is silent");

if (failures) { console.error(`tap plan: ${failures} FAILED`); process.exit(1); }
console.log("tap plan: all checks passed");

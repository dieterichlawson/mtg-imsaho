// The page says what this turn did that no zone shows (#953, #954).
//
//   node mtg-gui/tests/turn_facts.mjs
//
// Moonmist prevents combat damage by non-Wolves and non-Werewolves for the
// rest of the turn, and it was in the log only. Homicidal Brute transforms
// back at the end step unless it attacked this turn, and Darkthicket Wolf's
// pump is once each turn: an untapped Brute and a spent Wolf looked the
// same as their other halves.

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const P = await import("../dist/prompts.js");
const R = await import("../dist/render.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };

const creature = (id, name, extra) => ({ object_id: id, name, controller: 0, owner: 0,
  card_types: ["Creature"], attacking: null, blocking: [], blocked_by: [], blocked: false, counters: {},
  keywords: [], protections: [], restrictions: [], colors: ["Red"], granted_abilities: [], oracle_text: "",
  damage_marked: 0, damaged_by: [], attacked_this_turn: false, once_per_turn_used: false, ...extra });
const brute = creature(34, "Homicidal Brute", { attacked_this_turn: true });
const attacking = creature(35, "Homicidal Brute", { attacked_this_turn: true, attacking: { Player: 1 } });
const wolf = creature(36, "Darkthicket Wolf", { once_per_turn_used: true });
const idle = creature(37, "Village Ironsmith", {});
const view = { you: 0, opponents: [{ id: 1 }], battlefield: [brute, attacking, wolf, idle], your_hand: [], stack: [], exile: [],
  graveyards: [], your_library_cards: [], revealed_names: {},
  turn_effects: ["Moonmist: combat damage that would be dealt this turn by creatures other than Wolves or Werewolves is prevented"] };
const state = { view, index: P.indexView(view), overlay: null };
const facts = id => R.inspectorFacts(state, state.index.get(id));

if (!facts(34).includes("Attacked this turn")) fail(`an attack made this turn is said: ${JSON.stringify(facts(34))}`);
if (facts(35).includes("Attacked this turn")) fail(`not beside "Attacking": ${JSON.stringify(facts(35))}`);
if (!facts(36).includes("Once-per-turn ability used this turn")) fail(`a spent pump is said: ${JSON.stringify(facts(36))}`);
if (facts(37).some(f => f.startsWith("Attacked") || f.startsWith("Once-per-turn"))) fail(`nothing else: ${JSON.stringify(facts(37))}`);

const band = R.bandTurnEffects(view);
if (JSON.stringify(band) !== JSON.stringify(["Moonmist"])) fail(`the band names the effect's source: ${JSON.stringify(band)}`);
if (R.bandTurnEffects({}).length) fail("an old view has none");

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("turn facts: ok");

// Auto-pass on the page keeps the CLI's promises (#691).
//
//   node mtg-gui/tests/auto_pass.mjs
//
// `f` at Main Phase 1 passed through Main Phase 2 with the land drop
// unplayed, and engaging cleared the notice, so nothing said what it had
// declined. The CLI stops for a land play (#39) and says what engaging
// turned down (#296, #618).

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const P = await import("../dist/prompts.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };

const land = { PlayLand: { object_id: 7 } };
const cast = { CastSpell: { object_id: 9, targets: [] } };
const ability = { ActivateAbility: { object_id: 3, ability_index: 0 } };
const mana = { ActivateManaAbility: { object_id: 4, ability_index: 0 } };

if (!P.offersLandPlay(["PassPriority", land, "Concede"])) fail("a land play is seen");
if (P.offersLandPlay(["PassPriority", cast, mana, "Concede"])) fail("no land play, none seen");
if (P.autoPassDeclines(["PassPriority", land, cast, ability, mana, "Concede"]) !== 3)
  fail("a land play, a spell and an ability are what passing declines; not mana, pass or concede");

const on = P.autoPassNotice(2, true);
if (!on || !on.includes("declined 2 spells/abilities/land plays")) fail(`engaging says what it declined: ${on}`);
if (!P.autoPassNotice(0, true)?.includes("Auto-pass on")) fail("engaging with nothing declined still says it is on");
if (P.autoPassNotice(0, false) !== null) fail("stopping with nothing declined adds nothing");
if (!P.autoPassNotice(1, false)?.includes("declined 1 spell/ability/land play")) fail("stopping recalls the decline, singular");

// The stops themselves (#752, #753). A bare view: seat 0 is you.
const view = (o) => ({ you: 0, active_player: 0, turn_number: 6, step: "Upkeep", stack: [], battlefield: [], ...o });
const lion = (attacking) => ({ object_id: 61, controller: 1, attacking: attacking ? { Player: 0 } : null });
const bolt = { CastSpell: { object_id: 28, targets: [{ Object: 61 }] } };

// `f` at your own upkeep or draw: the next Main Phase 1 is this turn's (#753, the CLI's #45).
for (const step of ["Untap", "Upkeep", "Draw"]) {
  const since = P.autoPassSince(view({ step }));
  if (P.autoPassStop(view({ step: "PrecombatMain" }), ["PassPriority", bolt], since, false) !== "your main phase.")
    fail(`f at your ${step}: this turn's Main Phase 1 is a stop (since ${since})`);
}
// Pressed at your main phase or later, or on the opponent's turn, it is a later turn's.
for (const at of [view({ step: "PrecombatMain" }), view({ step: "EndStep" }), view({ active_player: 1, step: "Upkeep" })]) {
  const since = P.autoPassSince(at);
  if (P.autoPassStop(view({ step: "PrecombatMain" }), ["PassPriority"], since, false) !== null)
    fail(`f at ${at.step} (active ${at.active_player}): this turn's Main Phase 1 is not a stop`);
  if (P.autoPassStop(view({ step: "PrecombatMain", turn_number: 8 }), ["PassPriority"], since, false) !== "your main phase.")
    fail(`f at ${at.step} (active ${at.active_player}): your next turn's Main Phase 1 is a stop`);
}
// The opponent's attack is a stop (#752); their combat with nobody attacking is not (#295).
const theirs = (step, attacking) => view({ active_player: 1, turn_number: 7, step, battlefield: [lion(attacking)] });
if (P.autoPassStop(theirs("DeclareAttackers", true), ["PassPriority", bolt], 6, false) !== "attackers declared against you.")
  fail("an attack on you stops auto-pass");
if (P.autoPassStop(theirs("DeclareAttackers", false), ["PassPriority", bolt], 6, false) !== null)
  fail("a declare-attackers step with no attacker is passed");
if (P.autoPassStop(view({ step: "DeclareAttackers", battlefield: [{ object_id: 5, controller: 0, attacking: { Player: 1 } }] }), ["PassPriority"], 5, false) !== null)
  fail("your own attack is not an attack on you");
// The stops it had before still hold.
if (P.autoPassStop(theirs("Upkeep", false), ["PassPriority"], 6, true) === null) fail("a question is a stop");
if (P.autoPassStop(view({ active_player: 1, step: "EndStep", stack: [{}] }), ["PassPriority"], 6, false) === null) fail("the stack is a stop");
if (P.autoPassStop(view({ active_player: 1, step: "EndStep" }), ["PassPriority", land], 6, false) === null) fail("a land is a stop");

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: auto-pass stops for a land, an attack and this turn's main phase, and says what it declined");

// A stop `s` makes is headed, and `s` says it is on (#974).
//
//   node mtg-gui/tests/pass_heading.mjs
//
// The pass the loop shows a watching seat carries no context, so with `s`
// on every pass-only stop had an empty heading — even with a spell on the
// stack — and nothing on screen said `s` was on.

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const P = await import("../dist/prompts.js");
const R = await import("../dist/render.js");

let failures = 0;
const check = (got, want, what) => { if (got !== want) { console.error(`FAIL: ${what}: got ${JSON.stringify(got)}, want ${JSON.stringify(want)}`); failures++; } };

const view = (over) => ({ you: 0, active_player: 0, step: "PostcombatMain", stack: [], first_strike_damage_step: false, ...over });
const lions = { object_id: 30, name: "Savannah Lions", controller: 0 };
const bolt = { object_id: 31, name: "Brimstone Volley", controller: 1 };

check(P.priorityHeading(view({ stack: [lions] })), "RESPOND TO your Savannah Lions", "your spell on the stack");
check(P.priorityHeading(view({ stack: [lions, bolt] })), "RESPOND TO p1's Brimstone Volley", "the top of the stack, the opponent's");
check(P.priorityHeading(view({})), "MAIN PHASE 2", "an empty stack names the step");
check(P.priorityHeading(view({ active_player: 1, step: "EndStep" })), "OPPONENT'S TURN: END STEP", "on the opponent's turn");
check(P.priorityHeading(view({ step: "CombatDamage", first_strike_damage_step: true })), "FIRST-STRIKE COMBAT DAMAGE", "the first-strike step");
check(P.priorityHeading(null), "", "no view, no heading");

check(R.standingModes({ autoPass: null, stopAtPass: true }), "STOPPING AT EVERY PRIORITY (s to stop)", "s has a standing line");
check(R.standingModes({ autoPass: { sinceTurn: 3 }, stopAtPass: false }), "AUTO-PASS on (f to stop)", "f keeps its line");
check(R.standingModes({ autoPass: null, stopAtPass: false }), null, "neither on, no line");

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("pass_heading: ok");

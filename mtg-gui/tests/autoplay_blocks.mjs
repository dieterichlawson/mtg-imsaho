// autoplay's blockers step sends a blocker only at an attacker it may
// block (#944): with a flyer among the attackers, a ground blocker was
// sent at it, stayed selected blocking nothing, and the run failed as
// stuck when the page refused "No blocks".
//
//   node mtg-gui/tests/autoplay_blocks.mjs

import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
// autoplay requires playwright only inside `main`; its helpers load bare.
const { blockable } = require("./autoplay.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);

const attackers = ["o41", "o42"];
if (!same(blockable(attackers, { "10": [41] }, "o10"), ["o41"])) fail(`the flyer o42 is not blockable by o10: ${blockable(attackers, { "10": [41] }, "o10")}`);
if (!same(blockable(attackers, { "10": [41] }, "o11"), [])) fail("a blocker the decision lists for nothing blocks nothing");
if (!same(blockable(attackers, null, "o10"), attackers)) fail("with no legal_blocks every attacker is a target");
if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: a blocker is sent only at what it may block");

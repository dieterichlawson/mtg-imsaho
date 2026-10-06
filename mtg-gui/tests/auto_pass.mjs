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

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: auto-pass stops for a land and says what it declined");

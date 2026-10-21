// The page shows no engine object ids (#694).
//
//   node mtg-gui/tests/without_ids.mjs
//
// The board draws no `#id`, so one in a title or a row names nothing a
// person can find: "Devil's Play (#34) targeting Grizzly Bears (#62)".

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const P = await import("../dist/prompts.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };
const cases = [
  ["Devil's Play (#34) targeting Grizzly Bears (#62): choose X funding (0-15)", "Devil's Play targeting Grizzly Bears: choose X funding (0-15)"],
  ["Unruly Mob [source 2/2, #42]", "Unruly Mob [source 2/2]"],
  ["choose X funding (0-15)", "choose X funding (0-15)"],
  ["p0 tapped Grizzly Bears #5 and Forest #7", "p0 tapped Grizzly Bears and Forest"],
  // Same-named blockers in a damage-assignment order (#786's review).
  ["Grizzly Bears 2/2 [#62]", "Grizzly Bears 2/2"],
  ["Grizzly Bears 2/2 [#63, 1 damage]", "Grizzly Bears 2/2 [1 damage]"],
];
for (const [input, want] of cases) {
  const got = P.withoutIds(input);
  if (got !== want) fail(`${JSON.stringify(input)} -> ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);
}
// A trigger on the stack names the creature its text means (#835); the
// inspector's title for it has no id either.
const R = await import("../dist/render.js");
const heir = { object_id: 0, name: "Rakish Heir's triggered ability (put a +1/+1 counter on Stromkirk Noble (#40))" };
const title = R.inspectorTitle(heir);
if (title !== "Rakish Heir's triggered ability (put a +1/+1 counter on Stromkirk Noble)") fail(`inspector title ${JSON.stringify(title)}`);
if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: the page shows no engine ids");

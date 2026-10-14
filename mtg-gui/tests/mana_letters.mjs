// The page writes each colour with its own letter (#808).
//
//   node mtg-gui/tests/mana_letters.mjs
//
// The first letter of the name made Blue and Black both "B": Think Twice's
// flashback row read "flashback cost {2}{B}" and a floating blue "Pool 1B".

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const P = await import("../dist/prompts.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };
const eq = (got, want, what) => { if (got !== want) fail(`${what}: got ${JSON.stringify(got)}, want ${JSON.stringify(want)}`); };

eq(P.costText({ symbols: [{ Generic: 2 }, { Colored: "Blue" }] }), "{2}{U}", "a blue cost");
eq(P.costText({ symbols: [{ Generic: 1 }, { Colored: "Black" }] }), "{1}{B}", "a black cost");
eq(P.costText({ symbols: ["White", "Red", "Green"].map(c => ({ Colored: c })) }), "{W}{R}{G}", "the other three");
eq(P.costNote({ name: "Think Twice", is_flashback: true, alternative_cost: { symbols: [{ Generic: 2 }, { Colored: "Blue" }] } }),
  "flashback cost {2}{U}", "Think Twice's flashback row");
eq(P.costText({ symbols: [{ Generic: 1 }, { Colorless: 2 }] }), "{1}{C}{C}", "colourless mana");
for (const [name, letter] of [["White", "W"], ["Blue", "U"], ["Black", "B"], ["Red", "R"], ["Green", "G"], ["Colorless", "C"]]) {
  eq(P.manaLetter(name), letter, name);
}

if (failures) { console.error(`mana letters: ${failures} FAILED`); process.exit(1); }
console.log("mana letters: all checks passed");

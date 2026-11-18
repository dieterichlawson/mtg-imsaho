// A player strip keeps every count and every kind of floating mana (#995).
//
//   node mtg-gui/tests/strip_fit.mjs
//
// The strip clipped its last part, the pool, at the board's edge: six kinds
// floating beside double-digit zones read "Pool 2W 2U 2B 3R 2…", and the
// green and colorless mana were shown nowhere. stripParts now shortens the
// pool and the labels before anything is clipped.

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const { stripParts } = await import("../dist/render.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };

// About 6px a character in 8px Silkscreen, 10px between parts, 326px of
// room from x=150 to the board's edge.
const fitsIn = room => texts => texts.join("").length * 6 + 10 * (texts.length - 1) <= room;
const six = { White: 2, Blue: 2, Black: 2, Red: 3, Green: 2, Colorless: 2 };
const big = { hand: 9, library: 40, gy: 22, exile: 12, cast: 4 };

const typical = stripParts({ hand: 7, library: 40, gy: 15, exile: 3, cast: 3 }, { Red: 2, Green: 1 }, fitsIn(326));
if (typical.map(p => p.text).join(" ") !== "Hand 7 Lib 40 GY 15 Exile 3 Cast 3 Pool 2R 1G")
  fail(`a strip that fits changed: ${JSON.stringify(typical)}`);

for (const [what, pool, counts] of [["six kinds", six, big],
  ["double digits", { White: 10, Blue: 10, Black: 10, Red: 10, Green: 10, Colorless: 10 }, { ...big, library: 140 }]]) {
  const parts = stripParts(counts, pool, fitsIn(326));
  const texts = parts.map(p => p.text);
  if (!fitsIn(326)(texts)) fail(`${what}: no form fits: ${JSON.stringify(texts)}`);
  const poolPart = texts.find(t => t.startsWith("Pool ")) || "";
  for (const [k, n] of Object.entries(pool)) {
    const letter = { White: "W", Blue: "U", Black: "B", Red: "R", Green: "G", Colorless: "C" }[k];
    if (!poolPart.includes(`${n}${letter}`)) fail(`${what}: ${n}${letter} is not on the strip: ${JSON.stringify(texts)}`);
  }
  for (const n of Object.values(counts)) if (!texts.some(t => t.includes(String(n)))) fail(`${what}: the count ${n} is gone: ${JSON.stringify(texts)}`);
  const zones = parts.map(p => p.zone).filter(Boolean);
  if (zones.join(",") !== "library,graveyard,exile") fail(`${what}: zones a click opens are ${zones}`);
}

if (failures) { console.error(`strip-fit: ${failures} FAILED`); process.exit(1); }
console.log("strip-fit: ok");

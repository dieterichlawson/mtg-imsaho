// The band's flag row says every effect in force, by name or by count,
// whichever flags are on (#993).
//
//   node mtg-gui/tests/band_flags.mjs
//
// "first strike" and "creature died" were drawn first and the effects in
// force after them only where 24px were left: with both flags on — the
// regular damage step after first-strike deaths, when Moonmist's
// prevention is what matters — the effect was not drawn at all, and a
// third effect was lost behind one "…" with nothing counting it.

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const R = await import("../dist/render.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };

// 7px Silkscreen is about 5px a letter.
const measure = s => s.length * 5;
const ROOM = 136;
const moon = ["Moonmist"], three = ["Moonmist", "Rally the Peasants", "Spare from Evil"];
const width = row => row.reduce((w, p) => w + measure(p.text), 0) + 6 * Math.max(0, row.length - 1);

for (const fs of [false, true]) for (const died of [false, true]) for (const fx of [[], moon, three]) {
  const row = R.bandFlagRow(measure, ROOM, fs, died, fx);
  const at = `first strike=${fs} died=${died} effects=${fx.length}: ${JSON.stringify(row.map(p => p.text))}`;
  if (width(row) > ROOM) fail(`${at} is wider than the row`);
  const effectText = row.slice((fs ? 1 : 0) + (died ? 1 : 0)).map(p => p.text).join(" ");
  if (fx.length === 0 && effectText) fail(`${at} names an effect that is not in force`);
  if (fx.length > 0) {
    // Every effect is named, or the row counts what it does not name.
    const named = fx.filter(e => effectText.includes(e)).length;
    const counted = /\+(\d+)/.exec(effectText) || /(\d+) in force/.exec(effectText);
    const accounted = named + (counted ? Number(counted[1]) : 0);
    if (accounted !== fx.length) fail(`${at} accounts for ${accounted} of ${fx.length} effects`);
  }
  if (fs && !row.some(p => /first strike|1st str/.test(p.text))) fail(`${at} drops first strike`);
  if (died && !row.some(p => /died/.test(p.text))) fail(`${at} drops creature died`);
}
// The full words are kept wherever they fit.
const plain = R.bandFlagRow(measure, ROOM, true, true, []);
if (JSON.stringify(plain.map(p => p.text)) !== JSON.stringify(["first strike", "creature died"]))
  fail(`both flags and no effect are shortened: ${JSON.stringify(plain)}`);
const both = R.bandFlagRow(measure, ROOM, true, true, moon);
if (!both.some(p => p.text === "Moonmist")) fail(`#993's case, both flags on, drops Moonmist: ${JSON.stringify(both)}`);

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("band_flags: ok");

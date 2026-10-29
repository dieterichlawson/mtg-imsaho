// The inspector names keywords as the card prints them (#955): it printed
// the serde names, "FirstStrike" above the card's own "First strike".
//
//   node mtg-gui/tests/keyword_line.mjs

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const { keywordLine } = await import("../dist/render.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };
for (const [kw, want] of [
  [["FirstStrike"], "First strike"],
  [["Flying", "DoubleStrike", "Lifelink"], "Flying, double strike, lifelink"],
  [["Indestructible"], "Indestructible"],
]) {
  const got = keywordLine(kw);
  if (got !== want) fail(`${JSON.stringify(kw)}: ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);
}
if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: keywords are named as the card prints them");

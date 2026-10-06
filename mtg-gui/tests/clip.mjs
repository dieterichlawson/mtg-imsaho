// clip() finds the cut in O(log L) measurements, and the same cut the
// one-character-at-a-time loop found (#640).
//
//   node mtg-gui/tests/clip.mjs
//
// A long log line ("declared attackers: …" naming every creature) used to
// cost one near-full-length measureText per character removed — seconds per
// frame at a thousand permanents. A fake context with a fixed advance per
// character stands in for the canvas, and counts the calls.

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const { clip, clipKeepingTail, fitSummary } = await import("../dist/render.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };

function fakeCtx() {
  const ctx = { font: "", calls: 0, measureText(t) { ctx.calls++; return { width: [...t].length * 4 }; } };
  return ctx;
}
function slowClip(ctx, s, maxW) {
  if (ctx.measureText(s).width <= maxW) return s;
  let t = s;
  while (t.length > 1 && ctx.measureText(t + "…").width > maxW) t = t.slice(0, -1);
  return t + "…";
}

for (const len of [0, 1, 5, 50, 101, 500, 17183]) {
  const s = "Grizzly Bears (#12), ".repeat(Math.ceil(len / 21)).slice(0, len);
  for (const maxW of [0, 4, 8, 100, 400]) {
    const want = slowClip(fakeCtx(), s, maxW);
    const ctx = fakeCtx();
    const got = clip(ctx, s, maxW);
    if (got !== want) fail(`len ${len} maxW ${maxW}: got ${JSON.stringify(got.slice(0, 40))}, want ${JSON.stringify(want.slice(0, 40))}`);
    if (ctx.calls > 2 + Math.ceil(Math.log2(len + 2))) fail(`len ${len} maxW ${maxW}: ${ctx.calls} measurements`);
  }
}
// #689: a verb's " → target" tail survives the clip, so two verbs that
// differ only in their target still read differently.
{
  const lili = "-6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice";
  const you = clipKeepingTail(fakeCtx(), lili + " → You", 246);
  const opp = clipKeepingTail(fakeCtx(), lili + " → Opponent", 246);
  if (you === opp) fail(`the two -6 verbs clip to one string: ${you}`);
  if (!you.endsWith(" → You") || !opp.endsWith(" → Opponent")) fail(`a target was cut: ${you} / ${opp}`);
  for (const t of [you, opp]) if ([...t].length * 4 > 246) fail(`over width: ${t}`);
  if (clipKeepingTail(fakeCtx(), "Pass", 246) !== "Pass") fail("a short label was changed");
}
// #693: a number widget's summary never drops lines silently, and never
// drops "Payable X".
{
  const groups = Array.from({ length: 12 }, (_, i) => `Land ${i} x1 (1/tap)`);
  const lines = ["Pool: 1 Red", ...groups, "Payable X: 0-5, 7"];
  const four = fitSummary(lines, 4);
  if (four.length !== 4) fail(`four rows hold four lines: ${JSON.stringify(four)}`);
  if (!four.includes("Payable X: 0-5, 7")) fail(`Payable X dropped: ${JSON.stringify(four)}`);
  if (!four.some(l => /^\+\d+ more$/.test(l))) fail(`no "+N more" for the dropped lines: ${JSON.stringify(four)}`);
  const more = four.find(l => /more$/.test(l));
  if (more !== `+${lines.length - 3} more`) fail(`the count of what was dropped is wrong: ${more}`);
  const all = fitSummary(lines.slice(0, 3), 4);
  if (JSON.stringify(all) !== JSON.stringify(lines.slice(0, 3))) fail("lines that fit are shown as they are");
}
if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: clip cuts where the slow loop did, in O(log L) measurements; verb targets survive");

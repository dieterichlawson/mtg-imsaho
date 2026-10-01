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
const { clip } = await import("../dist/render.js");

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
if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: clip cuts where the slow loop did, in O(log L) measurements");

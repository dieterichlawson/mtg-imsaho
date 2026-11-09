// The panel's notice takes the room the panel has, not a fixed three
// lines (#973).
//
//   node mtg-gui/tests/notice_lines.mjs
//
// The auto-pass stop notice was cut at three lines, "…WHEN IT WAS TURNED
// O…", which hid the declined count (#796) with two-thirds of the panel
// empty below. A fake context, a fixed advance per character, stands in
// for the canvas: 32 characters a line, as the panel held at 1280x720.

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const { wrapCapped, noticeLineCap, panelLines, NOTICE_LINES, ROWS_LEAST } = await import("../dist/render.js");
const P = await import("../dist/prompts.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };
const ctx = { font: "", measureText: t => ({ width: [...t].length * 4 }) };
const W = 32 * 4;

const stop = `Auto-pass off: something is on the stack. ${P.autoPassNotice(2, false)}`;
if ((stop.match(/stopped|Auto-pass off/g) || []).length !== 1) fail(`the stop notice says it stopped once: ${stop}`);
const engage = P.autoPassNotice(2, true);

// A panel with room: each notice is drawn whole, the count included.
for (const notice of [stop, engage]) {
  const lines = wrapCapped(ctx, notice, W, "7px Silkscreen", noticeLineCap(200));
  const shown = lines.join(" ");
  if (shown.includes("…") || !shown.includes("declined 2 spells/abilities/land plays"))
    fail(`with room the notice is whole, its count included: ${JSON.stringify(lines)}`);
}

// It never takes more than the room (one line left for the question, and
// with rows below one row and its pager above the buttons: a long notice
// put the row on the button, #973's review), nor fewer than the three it
// always had, nor more than NOTICE_LINES.
for (let room = 0; room <= 300; room++) {
  for (const rows of [false, true]) {
    const cap = noticeLineCap(room, rows);
    const need = 9 + 8 * cap + (rows ? ROWS_LEAST : 0);
    if (cap < 3 || cap > NOTICE_LINES) fail(`room ${room}: cap ${cap} outside 3..${NOTICE_LINES}`);
    if (cap > 3 && need > room) fail(`room ${room}, rows ${rows}: ${cap} lines leave no room for what is below`);
  }
}
// The whole stack above the rows: hint, then a notice at its cap, then the
// question at what is left. Wherever the notice took more than its three,
// the rows still have their reserve: the question shrinks to one line
// rather than taking it (#973's review: a four-line question put the pager
// on the button).
for (let room = 0; room <= 300; room++) {
  for (const hint of [0, 1, 3]) {
    const left = room;
    const notice = noticeLineCap(left - 8 * hint, true);
    const reserved = 8 * (hint + notice);
    const title = panelLines(left, reserved, true, 9, 4, 1);
    if (notice > 3 && reserved + 9 * title + ROWS_LEAST > left)
      fail(`room ${room}, hint ${hint}: notice ${notice} and question ${title} lines leave the rows no room`);
  }
}
// Without rows the question has the room down to the buttons.
if (panelLines(9 * 4, 0, false, 9, 4, 1) !== 4) fail("with no rows the question keeps its four lines");

// Below the buttons' top (13px under the floor), the rows start 2px under
// the notice and a page needs two 11px lines: ROWS_LEAST is that sum.
if (ROWS_LEAST < 13 + 2 + 2 * 11) fail(`ROWS_LEAST ${ROWS_LEAST} is less than one row and its pager`);

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("notice lines: ok");

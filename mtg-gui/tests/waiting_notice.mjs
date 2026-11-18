// A notice said while no question is up is not drawn under the next one (#1012).
//
//   node mtg-gui/tests/waiting_notice.mjs
//
// "Answered in another tab." is set when the decision a tab holds is taken
// elsewhere, and an `f` pressed during the opponent's turn is refused —
// both while nothing is asked. The next question kept the line, in red
// under its own heading: a person at the tab was told the fresh Main
// Phase 1 had been answered somewhere else. A moment's notice becomes its
// past form or goes; a notice meant for the next question stays.

const { noticeAtNewDecision } = await import("../dist/state.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };
const eq = (got, want, what) => { if (got !== want) fail(`${what}: ${JSON.stringify(got)}, want ${JSON.stringify(want)}`); };

const other = { text: "Answered in another tab.", after: "Your last question was answered in another tab." };
eq(noticeAtNewDecision(other.text, other), other.after, "another tab's answer, said of the last question");
const f = { text: "No question is up: press f at your next priority to auto-pass.", after: null };
eq(noticeAtNewDecision(f.text, f), null, "a refusal about the wait goes");
// Overwritten since — "Last game: …" when the next game opened — it is
// not the moment's line, and stays.
eq(noticeAtNewDecision("Last game: you won. The next game has started.", other),
   "Last game: you won. The next game has started.", "a notice said for the next question stays");
eq(noticeAtNewDecision("Auto-pass on — it declined 1 spell.", null), "Auto-pass on — it declined 1 spell.", "no moment: unchanged");
eq(noticeAtNewDecision(null, other), null, "nothing on screen: nothing");

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: a notice about the wait does not carry onto the next question");

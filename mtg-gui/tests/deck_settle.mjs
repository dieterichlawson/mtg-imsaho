// The draft page's edit against the deck the server records (#815).
//
//   node mtg-gui/tests/deck_settle.mjs
//
// The server sends every seat a view on every change at the table, so a
// view built before it read the page's latest clicks is common. Its record
// must not undo them; a final record (the table built or topped up the
// deck) must replace the edit whatever was sent.

const D = await import("../dist/draft/deck.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };
const deck = (main, lands = { Island: 17 }) => ({ main, lands, sideboard: [] });
const rec = (d, ready = false) => ({ ...d, valid: true, problem: null, ready });

const S = deck(["Abbey Griffin"]);
const A = deck(["Abbey Griffin", "Chapel Geist"]);

// Nothing in flight: a differing record is taken; the same one is not.
let r = D.settle(rec(A), S, [], false);
if (!r.reset) fail("a differing record with nothing in flight replaces the edit");
r = D.settle(rec(S), S, [], false);
if (r.reset) fail("the record that is the edit changes nothing");

// An add in flight; a stale view (the record before it) leaves the edit alone.
r = D.settle(rec(S), A, [A], false);
if (r.reset || r.inFlight.length !== 1) fail(`a stale view undid an add in flight: ${JSON.stringify(r)}`);
// Its echo settles it.
r = D.settle(rec(A), A, [A], false);
if (r.reset || r.inFlight.length !== 0) fail(`the echo settles the add: ${JSON.stringify(r)}`);

// An add and its undo; a stale view equal to the undo clears nothing.
r = D.settle(rec(S), S, [A, S], false);
if (r.reset || r.inFlight.length !== 2) fail(`a record equal to a later message clears nothing: ${JSON.stringify(r)}`);

// The table's final deck replaces the edit with a click still in flight.
const T = deck(["Village Ironsmith"], { Plains: 9, Mountain: 8 });
r = D.settle(rec(T, true), A, [A], false);
if (!r.reset || r.inFlight.length !== 0) fail(`a final record wins: ${JSON.stringify(r)}`);

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("deck_settle: the edit follows the record without undoing clicks in flight");

// Engine lines read as the page's own words (#713, #714), and the game-over
// box can be put away to read the final board (#712).
//
//   node mtg-gui/tests/engine_lines.mjs

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const P = await import("../dist/prompts.js");
const R = await import("../dist/render.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };
const eq = (got, want) => { if (got !== want) fail(`${JSON.stringify(got)} !== ${JSON.stringify(want)}`); };

const state = { view: { you: 0, opponents: [{ id: 1 }] } };
const w = l => P.engineLine(state, l);

// #713: a possessive, and a present-tense verb whose subject became "you".
eq(w("p0's mana pool empties (Green:1)"), "your mana pool empties (Green:1)");
eq(w("p1's mana pool empties (Black:2)"), "opp's mana pool empties (Black:2)");
eq(w("p0 keeps (0 mulligans)"), "you keep (0 mulligans)");
eq(w("p1 keeps (0 mulligans)"), "opp keeps (0 mulligans)");
eq(w("p0 mulligans"), "you mulligan");
eq(w("Game over! p0 (red-green) wins! (p1 (white-black) lost the game: life total was 0 or less (CR 704.5a))"),
  "Game over! you (red-green) win! (opp (white-black) lost the game: life total was 0 or less (CR 704.5a))");
eq(w("Game over! p1 (white-black) wins!"), "Game over! opp (white-black) wins!");
eq(w("p0 cast Doom Blade"), "you cast Doom Blade");
eq(w("p0 winston"), "you winston");
eq(w("p7 passes"), "p7 passes");

// #714: no engine ids in a log line.
eq(w("p1 cast Doom Blade (#74) targeting Grizzly Bears (#25)"), "opp cast Doom Blade targeting Grizzly Bears");
eq(w("Kalonian Tusker (#31) resolved"), "Kalonian Tusker resolved");

// #712: the box shows until it is put away; the panel keeps the result.
const over = { gameOver: "Game over! p0 (g) wins!" };
if (!R.showsGameOverBox(over)) fail("the box shows at game over");
over.gameOverDismissed = true;
if (R.showsGameOverBox(over)) fail("a dismissed box stays away");
if (R.showsGameOverBox({ gameOver: null })) fail("no box before the game ends");
if (!/click|Esc/.test(R.GAME_OVER_DISMISS_HINT)) fail("the box says how to put it away");

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: engine lines in the page's words; the game-over box can be put away");

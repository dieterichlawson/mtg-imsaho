// The band leads with what the opponent did to you, not with what you just
// clicked (#847).
//
//   node mtg-gui/tests/band_lines.mjs
//
// The band shows an interval since the page last stopped, in two rows: a
// line, then "+N · the last line". The interval is measured from the frame
// the person answered, so its first line was nearly always the echo of
// their own answer — "You tapped Mountain for mana" / "+17 · You drew a
// card" — and the Swords to Plowshares that exiled their Goblin Piker was
// one of the 17.

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const P = await import("../dist/prompts.js");
const R = await import("../dist/render.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };

const piker = { object_id: 24, name: "Goblin Piker", controller: 0, owner: 0, card_types: ["Creature"] };
const tusker = { object_id: 31, name: "Kalonian Tusker", controller: 1, owner: 1, card_types: ["Creature"] };
const view = (log, exile) => ({ you: 0, opponents: [{ id: 1 }], battlefield: [tusker], your_hand: [], stack: [],
  exile, graveyards: [[0, []], [1, []]], your_library_cards: [], revealed_names: {}, display_log: log });
const band = (log, since, exile = []) => {
  const v = view(["older"].concat(log), exile);
  return R.bandLogLines({ view: v, index: P.indexView(v), logSince: since });
};

// Seed 3011's turn 17, abridged: the person's own cast leads the interval.
const turn17 = [
  "p0 tapped Mountain (#1) for mana (pool: Red:1)", "p0 cast Goblin Piker (#24)", "Goblin Piker (#24) resolved",
  "── Turn 16 (p1) ──", "p1 played Plains (#60)", "p1 tapped Plains (#60) for mana (pool: White:1)",
  "p1 cast Swords to Plowshares (#70) targeting Goblin Piker (#24)", "Swords to Plowshares (#70) resolved",
  "Goblin Piker (#24) was exiled", "p0 gained 2 life", "── Turn 17 (p0) ──", "p0 drew a card",
];
const exiled = [{ ...piker, owner: 0 }];
const b = band(turn17, 1, exiled);
if (b.length !== 2) fail(`two rows: ${JSON.stringify(b)}`);
if (b[0] !== "p1 cast Swords to Plowshares (#70) targeting Goblin Piker (#24)") fail(`the removal leads: ${JSON.stringify(b)}`);
if (b[1] !== `+${turn17.length - 2} · p0 drew a card`) fail(`then how much else, and the last line: ${JSON.stringify(b)}`);

// Nothing done to you: the first news that is not your own echo.
const quiet = ["p0 tapped Forest (#12) for mana (pool: Green:1)", "p0 cast Grizzly Bears (#26)", "Grizzly Bears (#26) resolved",
  "── Turn 8 (p1) ──", "p1 played Swamp (#61)", "p1 cast Kalonian Tusker (#31)", "── Turn 9 (p0) ──", "p0 drew a card"];
const bears = { object_id: 26, name: "Grizzly Bears", controller: 0, owner: 0, card_types: ["Creature"] };
const qv = view(["older"].concat(quiet), []);
qv.battlefield = [tusker, bears];
const q = R.bandLogLines({ view: qv, index: P.indexView(qv), logSince: 1 });
if (q[0] !== "── Turn 8 (p1) ──") fail(`no removal: the opponent's turn leads, not your tap: ${JSON.stringify(q)}`);

// An interval of nothing but your own doing still leads with its first line.
const own = band(["p0 tapped Forest (#12) for mana", "p0 cast Grizzly Bears (#26)", "p0 drew a card"], 1);
if (own[0] !== "p0 tapped Forest (#12) for mana") fail(`all yours: ${JSON.stringify(own)}`);

// Two new lines or fewer are both shown, as before; nothing new keeps the last two.
const two = band(["p1 played Swamp (#61)", "p0 drew a card"], 1);
if (JSON.stringify(two) !== JSON.stringify(["p1 played Swamp (#61)", "p0 drew a card"])) fail(`two: ${JSON.stringify(two)}`);
if (band(["a", "b"], 3).length !== 2) fail("nothing new falls back to the last lines");
// The last line is not repeated as the lead.
const last = band(["p0 tapped Forest (#12) for mana", "p0 cast Grizzly Bears (#26)", "p1 cast Dead Weight (#80) targeting Grizzly Bears (#26)"], 1,
  [{ ...bears }]);
if (last[0] === last[1].replace(/^\+\d+ · /, "")) fail(`the last line twice: ${JSON.stringify(last)}`);

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: the band leads with what the opponent did to you");

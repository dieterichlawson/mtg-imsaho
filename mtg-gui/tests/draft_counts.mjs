// The draft page says a count of one in the singular, as the runner does
// (#914, #946's review): "1 spell + 0 lands = 1" after the first card moved
// to the deck, "Deck has 1 card", "Colours (1 creature)".
//
//   node mtg-gui/tests/draft_counts.mjs

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };

const { count, deckSum, problem } = await import("../dist/draft/deck.js");

let failed = 0;
function check(ok, what) {
  console.log(`${ok ? "ok" : "FAIL"} - ${what}`);
  if (!ok) failed++;
}

check(count(1, "card", "cards") === "1 card", "one card");
check(count(0, "card", "cards") === "0 cards", "no cards");
check(count(40, "card", "cards") === "40 cards", "forty cards");
check(deckSum({ spells: 1, lands: 0, total: 1 }) === "1 spell + 0 lands = 1", `deckSum at one spell: ${deckSum({ spells: 1, lands: 0, total: 1 })}`);
check(deckSum({ spells: 23, lands: 17, total: 40 }) === "23 spells + 17 lands = 40", "deckSum of a full deck");
const one = problem({ main: [true], lands: {} }, [{ name: "Ambush Viper", line: "Ambush Viper {1}{G}" }]);
check(typeof one === "string" && one.startsWith("Deck has 1 card (need at least 40)"), `the refusal at one card: ${one}`);

if (failed) {
  console.log(`${failed} failed`);
  process.exit(1);
}

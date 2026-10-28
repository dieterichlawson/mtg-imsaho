// The deck under construction: which pool cards are in the main deck and
// how many of each basic land, the message that describes it, the reason
// it does not validate yet, and the summary the panel shows.
//
// The page validates so the Ready button can say why it is disabled; the
// server validates again with `mtg_draft::deckbuilding::validate_deck` and
// refuses what the page let through, so the rules here are the same ones
// and no stricter: at least 40 cards, only pool cards, basics unlimited.
import { colorKey, manaValue, parseLine } from "./cards.js";
/** "1 card", "40 cards": a count with its noun, singular at one, as the
 *  runner's `mtg_draft_runner::count` says it (#914, #946's review). */
export function count(n, one, many) {
    return `${n} ${n === 1 ? one : many}`;
}
/** "22 spells + 17 lands = 39", the deck's count line. */
export function deckSum(c) {
    return `${count(c.spells, "spell", "spells")} + ${count(c.lands, "land", "lands")} = ${c.total}`;
}
export const BASICS = ["Plains", "Island", "Swamp", "Mountain", "Forest"];
export const MIN_DECK = 40;
/** The server's own hallucination guard on one basic's count. */
export const MAX_LAND = 200;
/** An edit matching the deck the view records, or an empty one (everything
 *  in the sideboard, no lands) when the server has none yet. A main-deck
 *  name claims the first pool copy not yet claimed, so two copies of one
 *  card are two rows and each can be moved on its own. */
export function fromView(deck, pool) {
    const main = pool.map(() => false);
    const lands = {};
    for (const b of BASICS)
        lands[b] = 0;
    if (deck) {
        for (const name of deck.main) {
            const i = pool.findIndex((c, idx) => !main[idx] && c.name === name);
            if (i >= 0)
                main[i] = true;
        }
        for (const [name, n] of Object.entries(deck.lands)) {
            if (BASICS.includes(name))
                lands[name] = Math.max(0, Math.floor(n));
        }
    }
    return { main, lands };
}
export function toMessage(edit, pool) {
    const main = [];
    const sideboard = [];
    pool.forEach((c, i) => { (edit.main[i] ? main : sideboard).push(c.name); });
    const lands = {};
    for (const b of BASICS)
        if (edit.lands[b] > 0)
            lands[b] = edit.lands[b];
    return { type: "deck", main, lands, sideboard };
}
/** The edit as the `sideboard` message between games carries it: the main
 *  deck and the lands; the sideboard is the rest of the pool. */
export function toSideboard(edit, pool) {
    const deck = toMessage(edit, pool);
    return { type: "sideboard", main: deck.main, lands: deck.lands };
}
export function counts(edit) {
    const spells = edit.main.filter(Boolean).length;
    const lands = BASICS.reduce((n, b) => n + (edit.lands[b] || 0), 0);
    return { spells, lands, total: spells + lands };
}
/** Why the deck cannot be sent as final, or null when it can. */
export function problem(edit, pool) {
    if (edit.main.length !== pool.length)
        return "The pool changed; the deck is being rebuilt.";
    for (const b of BASICS) {
        const n = edit.lands[b] || 0;
        if (n < 0 || !Number.isInteger(n))
            return `${b} count must be a whole number.`;
        if (n > MAX_LAND)
            return `${b} count is ${n}; the server allows at most ${MAX_LAND}.`;
    }
    const c = counts(edit);
    const short = MIN_DECK - c.total;
    if (short > 0)
        return `Deck has ${count(c.total, "card", "cards")} (need at least ${MIN_DECK}). Add ${short} more ${short === 1 ? "card or basic land" : "cards or basic lands"}.`;
    return null;
}
export function summary(edit, pool) {
    const byColor = new Map();
    const curve = [0, 0, 0, 0, 0, 0, 0, 0];
    let creatures = 0;
    pool.forEach((c, i) => {
        if (!edit.main[i])
            return;
        const key = colorKey(c.colors);
        byColor.set(key, (byColor.get(key) ?? 0) + 1);
        const parsed = parseLine(c.name, c.line);
        curve[Math.min(7, manaValue(parsed.cost))]++;
        if (/\bCreature\b/.test(parsed.typeLine))
            creatures++;
    });
    const colors = [];
    for (const key of ["W", "U", "B", "R", "G", "M", "C"]) {
        const n = byColor.get(key);
        if (n)
            colors.push({ key, n });
    }
    return { colors, curve, creatures };
}
/** The same main deck (in any order) and the same nonzero lands. Not the
 *  sideboard: the server records a legal deck's sideboard from the pool
 *  under a double-faced card's full name, where the page sends its front
 *  face, so a sideboard compared would never match and a deck in flight
 *  would never settle. The sideboard is the rest of the pool either way. */
export function sameDeck(a, b) {
    if (!b)
        return false;
    const sorted = (xs) => [...xs].sort().join("\n");
    const lands = (l) => Object.entries(l).filter(([, n]) => n > 0).sort().map(([k, n]) => `${k}=${n}`).join(",");
    return sorted(a.main) === sorted(b.main) && lands(a.lands) === lands(b.lands);
}
/**
 * Whether a view's recorded deck replaces the page's edit, and what is left
 * in flight. The record is the deck that will be played, so it replaces the
 * edit when it differs from it with nothing of the page's in flight, when it
 * is final (the table built or topped up the deck, #815), and on a fresh
 * start. While decks are in flight a record is matched against the oldest
 * only, which the server answers first: a view built before the server read
 * the latest clicks carries an older record, and taking it undid them, so
 * the next click or Ready sent the undone deck (#815's review).
 */
export function settle(record, edit, inFlight, fresh) {
    let left = inFlight;
    if (record && left.length > 0 && sameDeck(record, left[0]))
        left = left.slice(1);
    const final = !!(record && record.ready);
    const differs = !!(record && !sameDeck(record, edit));
    const reset = fresh || (differs && (final || left.length === 0));
    return { reset, inFlight: reset && (fresh || final) ? [] : left };
}

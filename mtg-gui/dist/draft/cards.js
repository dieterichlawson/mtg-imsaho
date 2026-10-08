// What the page reads off a card: the headline's parts, its mana value,
// the colour it is filed under, and its art.
import { artPath } from "../assets.js";
/** The front face of a "Front // Back" name, or the name itself. */
export function frontFace(name) {
    const cut = name.indexOf(" // ");
    return cut >= 0 ? name.slice(0, cut) : name;
}
/**
 * The headline's parts. The server writes the runner's `pack_line`:
 * `Name {cost} | Type — Sub P/T [rarity]`, with ` // Back P/T` for a
 * double-faced card; the plan's own example writes `Name {cost} P/T Type |
 * text`. Both are read, and a line in neither form still yields its name
 * and whatever cost and P/T it carries, so a card is never shown blank.
 */
export function parseLine(name, line) {
    let s = line.replace(/\s*\[(?:common|uncommon|rare|mythic|special|bonus)\]\s*$/i, "");
    let back = null;
    const dfc = s.indexOf(" // ");
    if (dfc >= 0) {
        back = s.slice(dfc + 4).trim();
        s = s.slice(0, dfc);
    }
    const front = frontFace(name);
    if (s.startsWith(front))
        s = s.slice(front.length);
    const costM = s.match(/^\s*((?:\{[^}]*\})+)/);
    const cost = costM ? costM[1] : "";
    if (costM)
        s = s.slice(costM[0].length);
    const segments = s.split("|").map(t => t.trim()).filter(Boolean);
    let typeLine = segments[0] ?? "";
    let pt = null;
    const ptM = typeLine.match(/(?:^|\s)([\dX*+-]+\/[\dX*+-]+)(?=\s|$)/);
    if (ptM) {
        pt = ptM[1];
        typeLine = typeLine.replace(ptM[0], " ").replace(/\s+/g, " ").trim();
    }
    return { cost, typeLine, pt, back };
}
/** The mana value of a cost string: generic adds its number, every other
 *  pip is one, X is nothing. */
export function manaValue(cost) {
    let total = 0;
    for (const pip of cost.match(/\{[^}]*\}/g) ?? []) {
        const inner = pip.slice(1, -1);
        if (/^\d+$/.test(inner))
            total += Number(inner);
        else if (/^\d+\/[WUBRG]$/.test(inner))
            total += Number(inner.split("/")[0]);
        else if (inner === "X" || inner === "Y" || inner === "Z")
            total += 0;
        else
            total += 1;
    }
    return total;
}
export const COLOR_ORDER = ["W", "U", "B", "R", "G", "M", "C"];
export const COLOR_NAMES = {
    W: "White", U: "Blue", B: "Black", R: "Red", G: "Green", M: "Multicolour", C: "Colourless",
};
/** The colour a card is filed under: its colour, gold for several, stone for none. */
export function colorKey(colors) {
    const own = colors.filter((c) => c === "W" || c === "U" || c === "B" || c === "R" || c === "G");
    if (own.length === 0)
        return "C";
    if (own.length > 1)
        return "M";
    return own[0];
}
/** The cards grouped by colour in WUBRG order, empty groups left out, each
 *  group sorted by mana value then name. */
export function groupByColor(cards) {
    const groups = new Map();
    for (const card of cards) {
        const key = colorKey(card.colors);
        const list = groups.get(key) ?? [];
        list.push(card);
        groups.set(key, list);
    }
    const out = [];
    for (const key of COLOR_ORDER) {
        const list = groups.get(key);
        if (!list)
            continue;
        list.sort((a, b) => manaValue(parseLine(a.name, a.line).cost) - manaValue(parseLine(b.name, b.line).cost) || a.name.localeCompare(b.name));
        out.push({ key, label: COLOR_NAMES[key], cards: list });
    }
    return out;
}
/** The art file for a card, or null for a placeholder. */
export function artFor(name) {
    return artPath(name);
}
/** Two or three letters for a placeholder, the way the canvas draws one. */
export function initials(name) {
    return frontFace(name).split(/[\s,]+/).filter(Boolean).slice(0, 3).map(s => s[0].toUpperCase()).join("");
}

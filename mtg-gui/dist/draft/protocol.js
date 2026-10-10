// The draft server's protocol, as docs/plans/draft-with-friends.md
// "Protocol" writes it: the server sends the seat's whole view after every
// change, and refuses a request with its reason and the request echoed.
//
// The shapes are declared twice here, once as types and once as the
// `SHAPES` table, because the fixture test (`tests/draft_fixtures.js`) runs
// in node without a browser and checks the server's fixture views against
// what this file names. A field added to one must be added to the other.
export const PHASES = ["lobby", "drafting", "building", "playing", "done"];
export const SHAPES = {
    view: {
        type: "enum:view",
        phase: "enum:lobby|drafting|building|playing|done",
        seat: "number",
        pod_size: "number",
        set: "string",
        seats: "array:seat",
        pass_direction: "enum:left|right|null",
        pack: "pack|null",
        pool: "array:card",
        picks: "array:pick",
        deck: "deck|null",
        matches: "array:match",
        standings: "array:standing",
        notice: "string|null",
        pairings: "array:pairing?",
        pick_seconds: "number|null?",
        build_seconds: "number|null?",
        build_deadline_ms: "number|null?",
    },
    seat: {
        seat: "number",
        kind: "enum:human|ai|cli",
        name: "string",
        joined: "boolean",
        status: "enum:picking|waiting|building|ready|playing|idle",
        picks: "number",
        connected: "number?",
        auto: "boolean?",
    },
    card: { name: "string", line: "string", text: "string", rarity: "string", colors: "string[]" },
    pack_card: { index: "number", name: "string", line: "string", text: "string", rarity: "string", colors: "string[]" },
    pack: {
        id: "number", round: "number", pick: "number", size: "number",
        cards: "array:pack_card", waiting: "number", deadline_ms: "number|null",
    },
    pick: { round: "number", pick: "number", card: "string", auto: "boolean" },
    deck: { main: "string[]", lands: "map:number", sideboard: "string[]", valid: "boolean", problem: "string|null", ready: "boolean?" },
    game: { winner: "number|null", forfeited_by: "number|null?", abandoned: "boolean?" },
    match: {
        round: "number", opponent: "number", url: "string|null",
        status: "enum:waiting|playing|done", games: "array:game", result: "string|null",
    },
    pairing: {
        round: "number", a: "number", b: "number|null",
        status: "enum:waiting|playing|done", result: "string|null",
    },
    standing: {
        seat: "number", wins: "number", losses: "number", points: "number",
        draws: "number?", game_wins: "number?", byes: "number?", tags: "string?",
    },
    refused: { type: "enum:refused", reason: "string", echo: "any" },
};
/**
 * Every way `value` fails to be a `shape`, as "path: what is wrong" lines.
 * Empty when it conforms. Extra fields are not a failure: the server may
 * grow the view before the page reads the new field.
 */
export function checkShape(value, shape, path = shape) {
    const out = [];
    const spec = SHAPES[shape];
    if (!spec)
        return [`${path}: no shape named ${shape}`];
    if (typeof value !== "object" || value === null || Array.isArray(value))
        return [`${path}: not an object`];
    const obj = value;
    for (const [field, fieldSpec] of Object.entries(spec)) {
        const optional = fieldSpec.endsWith("?");
        const kind = optional ? fieldSpec.slice(0, -1) : fieldSpec;
        if (!(field in obj)) {
            if (!optional)
                out.push(`${path}.${field}: missing`);
            continue;
        }
        out.push(...checkField(obj[field], kind, `${path}.${field}`));
    }
    return out;
}
function checkField(v, kind, path) {
    if (kind.endsWith("|null") && !kind.startsWith("enum:")) {
        if (v === null)
            return [];
        return checkField(v, kind.slice(0, -"|null".length), path);
    }
    if (kind === "any")
        return [];
    if (kind === "string" || kind === "number" || kind === "boolean") {
        return typeof v === kind ? [] : [`${path}: ${describe(v)}, expected ${kind}`];
    }
    if (kind === "string[]") {
        if (!Array.isArray(v))
            return [`${path}: ${describe(v)}, expected an array of strings`];
        return v.every(x => typeof x === "string") ? [] : [`${path}: not every entry is a string`];
    }
    if (kind === "map:number") {
        if (typeof v !== "object" || v === null || Array.isArray(v))
            return [`${path}: ${describe(v)}, expected an object of numbers`];
        return Object.values(v).every(x => typeof x === "number") ? [] : [`${path}: not every value is a number`];
    }
    if (kind.startsWith("enum:")) {
        const words = kind.slice("enum:".length).split("|");
        if (v === null && words.includes("null"))
            return [];
        return typeof v === "string" && words.includes(v) ? [] : [`${path}: ${describe(v)}, expected one of ${words.join(", ")}`];
    }
    if (kind.startsWith("array:")) {
        const shape = kind.slice("array:".length);
        if (!Array.isArray(v))
            return [`${path}: ${describe(v)}, expected an array of ${shape}`];
        return v.flatMap((x, i) => checkShape(x, shape, `${path}[${i}]`));
    }
    return checkShape(v, kind, path);
}
function describe(v) {
    if (v === null)
        return "null";
    if (Array.isArray(v))
        return "an array";
    return typeof v === "object" ? "an object" : `${typeof v} ${JSON.stringify(v)}`;
}

// The draft server's protocol, as docs/plans/draft-with-friends.md
// "Protocol" writes it: the server sends the seat's whole view after every
// change, and refuses a request with its reason and the request echoed.
//
// The shapes are declared twice here, once as types and once as the
// `SHAPES` table, because the fixture test (`tests/draft_fixtures.js`) runs
// in node without a browser and checks the server's fixture views against
// what this file names. A field added to one must be added to the other.

export type Phase = "lobby" | "drafting" | "building" | "playing" | "done";
export type SeatKind = "human" | "ai" | "cli";
export type SeatStatus = "picking" | "waiting" | "building" | "ready" | "playing" | "idle";
export type PassDirection = "left" | "right";
export type MatchStatus = "waiting" | "playing" | "done";

export interface SeatInfo {
  seat: number;
  kind: SeatKind;
  name: string;
  joined: boolean;
  status: SeatStatus;
  picks: number;
}

/** A card as the pack and the pool describe it: a name, the runner's
 *  one-line headline, the rules text, and the facts a grid groups by. */
export interface CardInfo {
  name: string;
  line: string;
  text: string;
  rarity: string;
  colors: string[];
}

export interface PackCard extends CardInfo {
  /** Index into `pack.cards`; what a `pick` names. */
  index: number;
}

export interface Pack {
  id: number;
  round: number;
  pick: number;
  size: number;
  cards: PackCard[];
  /** Packs queued behind this one. */
  waiting: number;
  /** Milliseconds left on `--pick-seconds`, or null without a timer. */
  deadline_ms: number | null;
}

export interface PickRecord {
  round: number;
  pick: number;
  card: string;
  auto: boolean;
}

export interface Deck {
  main: string[];
  lands: Record<string, number>;
  sideboard: string[];
  valid: boolean;
  problem: string | null;
}

export interface GameResult { winner: number }

export interface Match {
  round: number;
  opponent: number;
  url: string;
  status: MatchStatus;
  games: GameResult[];
  result: string | null;
}

export interface Standing {
  seat: number;
  wins: number;
  losses: number;
  points: number;
}

export interface DraftView {
  type: "view";
  phase: Phase;
  seat: number;
  pod_size: number;
  set: string;
  seats: SeatInfo[];
  pass_direction: PassDirection | null;
  pack: Pack | null;
  pool: CardInfo[];
  picks: PickRecord[];
  deck: Deck | null;
  matches: Match[];
  standings: Standing[];
  notice: string | null;
}

export interface Refused {
  type: "refused";
  reason: string;
  echo: unknown;
}

export type ServerMessage = DraftView | Refused;

export type ClientMessage =
  | { type: "pick"; pack_id: number; index: number }
  | { type: "deck"; main: string[]; lands: Record<string, number>; sideboard: string[] }
  | { type: "ready" }
  | { type: "name"; name: string };

// ------------------------------------------------------------ the table

/**
 * A field's expected shape, as a string the checker reads:
 *   "string" | "number" | "boolean" | "any"
 *   "<kind>|null"           — nullable
 *   "string[]"              — an array of strings
 *   "map:number"            — an object of string -> number
 *   "enum:a|b|c"            — one of the words
 *   "array:<shape>"         — an array of objects of that shape
 *   "<shape>"               — an object of that shape (a key of SHAPES)
 */
export type FieldSpec = string;
export type Shape = Record<string, FieldSpec>;

export const PHASES: readonly Phase[] = ["lobby", "drafting", "building", "playing", "done"];

export const SHAPES: Record<string, Shape> = {
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
  },
  seat: {
    seat: "number",
    kind: "enum:human|ai|cli",
    name: "string",
    joined: "boolean",
    status: "enum:picking|waiting|building|ready|playing|idle",
    picks: "number",
  },
  card: { name: "string", line: "string", text: "string", rarity: "string", colors: "string[]" },
  pack_card: { index: "number", name: "string", line: "string", text: "string", rarity: "string", colors: "string[]" },
  pack: {
    id: "number", round: "number", pick: "number", size: "number",
    cards: "array:pack_card", waiting: "number", deadline_ms: "number|null",
  },
  pick: { round: "number", pick: "number", card: "string", auto: "boolean" },
  deck: { main: "string[]", lands: "map:number", sideboard: "string[]", valid: "boolean", problem: "string|null" },
  game: { winner: "number" },
  match: {
    round: "number", opponent: "number", url: "string",
    status: "enum:waiting|playing|done", games: "array:game", result: "string|null",
  },
  standing: { seat: "number", wins: "number", losses: "number", points: "number" },
  refused: { type: "enum:refused", reason: "string", echo: "any" },
};

/**
 * Every way `value` fails to be a `shape`, as "path: what is wrong" lines.
 * Empty when it conforms. Extra fields are not a failure: the server may
 * grow the view before the page reads the new field.
 */
export function checkShape(value: unknown, shape: string, path = shape): string[] {
  const out: string[] = [];
  const spec = SHAPES[shape];
  if (!spec) return [`${path}: no shape named ${shape}`];
  if (typeof value !== "object" || value === null || Array.isArray(value)) return [`${path}: not an object`];
  const obj = value as Record<string, unknown>;
  for (const [field, kind] of Object.entries(spec)) {
    if (!(field in obj)) { out.push(`${path}.${field}: missing`); continue; }
    out.push(...checkField(obj[field], kind, `${path}.${field}`));
  }
  return out;
}

function checkField(v: unknown, kind: FieldSpec, path: string): string[] {
  if (kind.endsWith("|null") && !kind.startsWith("enum:")) {
    if (v === null) return [];
    return checkField(v, kind.slice(0, -"|null".length), path);
  }
  if (kind === "any") return [];
  if (kind === "string" || kind === "number" || kind === "boolean") {
    return typeof v === kind ? [] : [`${path}: ${describe(v)}, expected ${kind}`];
  }
  if (kind === "string[]") {
    if (!Array.isArray(v)) return [`${path}: ${describe(v)}, expected an array of strings`];
    return v.every(x => typeof x === "string") ? [] : [`${path}: not every entry is a string`];
  }
  if (kind === "map:number") {
    if (typeof v !== "object" || v === null || Array.isArray(v)) return [`${path}: ${describe(v)}, expected an object of numbers`];
    return Object.values(v as Record<string, unknown>).every(x => typeof x === "number") ? [] : [`${path}: not every value is a number`];
  }
  if (kind.startsWith("enum:")) {
    const words = kind.slice("enum:".length).split("|");
    if (v === null && words.includes("null")) return [];
    return typeof v === "string" && words.includes(v) ? [] : [`${path}: ${describe(v)}, expected one of ${words.join(", ")}`];
  }
  if (kind.startsWith("array:")) {
    const shape = kind.slice("array:".length);
    if (!Array.isArray(v)) return [`${path}: ${describe(v)}, expected an array of ${shape}`];
    return v.flatMap((x, i) => checkShape(x, shape, `${path}[${i}]`));
  }
  return checkShape(v, kind, path);
}

function describe(v: unknown): string {
  if (v === null) return "null";
  if (Array.isArray(v)) return "an array";
  return typeof v === "object" ? "an object" : `${typeof v} ${JSON.stringify(v)}`;
}

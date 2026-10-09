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
  /** Live sockets right now; a joined seat with none is away. */
  connected?: number;
  /** The table picks and builds for this seat (kicked, or absent after
   *  the host started without it). */
  auto?: boolean;
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
  /** The deck is final: `ready` was accepted, or the table built it. */
  ready?: boolean;
}

/** A game's winner, or null for a draw. */
export interface GameResult { winner: number | null }

/**
 * One of the viewing seat's own matches. `url` is this seat's game page;
 * it is null while the match waits for its pages to open, for a match
 * the table forfeited without opening one, and on the opponent's side
 * of the same match (each seat gets its own link, never the other's).
 */
export interface Match {
  round: number;
  opponent: number;
  url: string | null;
  status: MatchStatus;
  games: GameResult[];
  result: string | null;
}

/** Every match of the tournament, for everybody: `b` null is a bye. */
export interface Pairing {
  round: number;
  a: number;
  b: number | null;
  status: MatchStatus;
  result: string | null;
}

export interface Standing {
  seat: number;
  wins: number;
  losses: number;
  points: number;
  draws?: number;
  game_wins?: number;
  byes?: number;
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
  pairings?: Pairing[];
  /** The host's timers, in seconds, when set. */
  pick_seconds?: number | null;
  build_seconds?: number | null;
  /** Milliseconds left to build, when a build timer runs for this seat. */
  build_deadline_ms?: number | null;
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
 *   "<spec>?"               — optional: the field may be absent (the page's
 *                             own fixtures predate it; the server sends it)
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
  game: { winner: "number|null" },
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
    draws: "number?", game_wins: "number?", byes: "number?",
  },
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
  for (const [field, fieldSpec] of Object.entries(spec)) {
    const optional = fieldSpec.endsWith("?");
    const kind = optional ? fieldSpec.slice(0, -1) : fieldSpec;
    if (!(field in obj)) { if (!optional) out.push(`${path}.${field}: missing`); continue; }
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

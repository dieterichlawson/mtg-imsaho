// The page's state, and the widget that answers a decision.

import type { Action, Decision, GameView, ObjectId, PlayerId, Target, ViewObject } from "./protocol.js";

/** Where an object is, for the index the page keeps over the view. */
export type ZoneName = "battlefield" | "hand" | "graveyard" | "exile" | "stack" | "library" | "revealed";
export interface IndexEntry { obj: ViewObject; zone: ZoneName; owner: PlayerId | null }

/** A row in a list or the panel. */
export interface Row { label: string; run: () => void; key?: string; cardName?: string | null }
export interface Button { label: string; run: () => void; primary?: boolean; enabled?: () => boolean }

/** A clickable rectangle drawn in the last frame. */
export interface Hit {
  x: number; y: number; w: number; h: number;
  kind: "perm" | "hand" | "card" | "stack" | "player" | "zone" | "button" | "row" | "modal" | "popover" | "overlay" | "log";
  key?: string;
  id?: ObjectId;
  pid?: PlayerId;
  zone?: "graveyard" | "exile" | "library";
  label?: string;
  group?: ObjectId[] | null;
  cardName?: string | null;
  onClick?: ((hit: Hit) => void) | null;
}

export type UiMode = "menu" | "pick" | "mark" | "attackers" | "blockers" | "list" | "order" | "number";

/** A verb a card offers in the priority menu. */
export interface Verb { label: string; run: () => void }

/**
 * The widget answering the pending decision. One shape for all modes:
 * the fields a mode does not use stay undefined.
 */
export interface Ui {
  mode: UiMode;
  title: string;
  hint: string;
  buttons: Button[];
  marked: string[];
  // Options on the board, keyed `o<id>` or `p<id>`, mapping to the original target or id.
  options?: Map<string, Target | ObjectId>;
  isOption?: (key: string) => boolean;
  rows?: Row[];
  looseRows?: Row[];
  // menu
  verbs?: Map<ObjectId, Verb[]>;
  canPass?: boolean;
  // pick
  onPick?: (key: string) => void;
  onCancel?: () => void;
  // mark / attackers / blockers
  min?: number;
  max?: number;
  toggle?: (key: string) => void;
  canConfirm?: () => boolean;
  onConfirm?: () => void;
  locked?: Set<string>;
  badge?: (key: string) => string | null;
  buttonLabel?: () => string;
  defenders?: { label: string; player?: PlayerId; planeswalker?: ObjectId }[];
  attackTarget?: Map<string, number>;
  attackers?: Set<string>;
  legal?: Map<string, Set<string>>;
  minBlockers?: Map<string, number>;
  assignments?: Map<string, string>;
  selectedBlocker?: string | null;
  clickBlocker?: (key: string) => void;
  clickAttacker?: (key: string) => void;
  canAttackerTake?: (key: string) => boolean;
  blockersOn?: (attackerKey: string) => number;
  // list
  filter?: boolean;
  query?: string;
  scroll?: number;
  keepBoard?: boolean;
  // order
  order?: { label: string; i: number }[];
  move?: (pos: number, dir: number) => void;
  // number
  value?: string;
  summary?: string[];
  submit?: () => void;
}

export interface Popover { id: ObjectId; items: Verb[]; x: number; y: number }
export interface Overlay { zone: "graveyard" | "exile" | "library"; pid: PlayerId; page?: number }

export interface State {
  ws: WebSocket | null;
  connected: boolean;
  view: GameView | null;
  index: Map<ObjectId, IndexEntry>;
  decision: Decision | null;
  lastDecision?: Decision | null;
  lastSent?: { seq: number; action: Action } | null;
  ui: Ui | null;
  gameOver: string | null;
  hover: Hit | null;
  hits: Hit[];
  popover: Popover | null;
  overlay: Overlay | null;
  selected: ObjectId | null;
  notice: string | null;
  logOpen: boolean;
  logScroll: number;
  scale: number;
  stopAtPass?: boolean;
  draw?: () => void;
}

/** A state with a view: what every renderer and prompt works on. */
export type LiveState = State & { view: GameView };

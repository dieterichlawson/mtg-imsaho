// The draft page's state: the last view, what the person is in the middle
// of on top of it, and the socket. Everything the renderer reads is here.

import type { DeckEdit } from "./deck.js";
import type { ClientMessage, DraftView } from "./protocol.js";

export interface PendingPick {
  packId: number;
  pick: number;
  index: number;
  sentAt: number;
}

export interface Refusal { reason: string; echo: unknown }

export interface PageState {
  ws: WebSocket | null;
  connected: boolean;
  /** Reconnects attempted since the last open. */
  reconnects: number;
  /** When the next reconnect fires, for the status line. */
  retryAt: number | null;
  /** The server refused the join itself (wrong key, no such seat): no
   *  reconnect will change that, so none is scheduled. */
  rejected: boolean;
  seat: number | null;
  key: string | null;
  view: DraftView | null;
  /** Drafting: the pack card index clicked once, or null. */
  selected: number | null;
  /** Drafting: the pack card index under the mouse, or null. */
  hover: number | null;
  /** Drafting: the pick sent and not yet answered by a view. */
  pendingPick: PendingPick | null;
  /** Building: the deck under construction, or null before the first view. */
  deck: DeckEdit | null;
  /** Building: the pool index the keyboard cursor is on, or null. */
  cursor: number | null;
  /** Building: `ready` has been sent and no view has confirmed it yet. */
  readySent: boolean;
  refusal: Refusal | null;
  /** Drafting: `performance.now()` at which the pick timer runs out. */
  deadlineAt: number | null;
  /** A line about the last key that could not do anything. */
  hint: string | null;
}

/** What the renderer asks the page to do. */
export interface Actions {
  select(index: number): void;
  pick(index: number): void;
  hover(index: number | null): void;
  toggle(poolIndex: number): void;
  setLand(name: string, n: number): void;
  ready(): void;
  dismiss(): void;
}

export function initialState(): PageState {
  return {
    ws: null, connected: false, reconnects: 0, retryAt: null, rejected: false,
    seat: null, key: null, view: null,
    selected: null, hover: null, pendingPick: null,
    deck: null, cursor: null, readySent: false,
    refusal: null, deadlineAt: null, hint: null,
  };
}

export interface Sent { at: number; message: ClientMessage }

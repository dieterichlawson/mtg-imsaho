// The draft page: one WebSocket to the seat, one DOM tree rebuilt from the
// last view, and the keys.
//
// The seat and key come from the page's own query string
// (`/?seat=0&key=...`, the join line the server prints); the socket is
// `/ws?seat=N&key=K` on the same host. The server sends the whole view
// after every change, so the page keeps nothing it could not rebuild from
// the next message — except what the person is in the middle of: a card
// selected but not picked, and the deck they are building.

import { loadManifest, fontsReady } from "../assets.js";
import { fromView, problem, sameDeck, settle, toMessage, toSideboard, type SentDeck } from "./deck.js";
import { packColumns, render } from "./render.js";
import type { Between, ClientMessage, Deck, DraftView, Match, Refused, ServerMessage } from "./protocol.js";
import { initialState, type Actions, type PageState, type Sent } from "./state.js";

const root = document.getElementById("app") as HTMLElement;
/** draft.html's phone breakpoint: one column, the detail above the pack. */
const ONE_COLUMN = window.matchMedia("(max-width: 760px)");
const connEl = document.getElementById("conn") as HTMLElement;
const seatEl = document.getElementById("seatline") as HTMLElement;
const nameField = document.getElementById("name") as HTMLInputElement;

const state: PageState = initialState();

interface DebugHook {
  /** Render a view, or handle any server message, without a socket. */
  stage(message: ServerMessage | DraftView): void;
  /** The `refused` path, for a test. */
  refuse(reason: string, echo?: unknown): void;
  /** Redraw from the current state. */
  render(): void;
}
interface Exposed {
  state: PageState;
  /** The last view, as the server sent it. */
  view: DraftView | null;
  /** Every message the page sent (or tried to, with the socket closed). */
  sent: Sent[];
  connected: boolean;
  reconnects: number;
}
declare global {
  interface Window {
    mtgDraft: Exposed;
    mtgDraftDebug: DebugHook;
  }
}

const exposed: Exposed = {
  state, view: null, sent: [], connected: false, reconnects: 0,
};
window.mtgDraft = exposed;

// ---------------------------------------------------------------- socket

const params = new URLSearchParams(location.search);
state.seat = params.has("seat") ? Number(params.get("seat")) : null;
state.key = params.get("key");

let retryTimer: number | null = null;
let retryDelay = 500;
const RETRY_MAX = 10000;

function socketUrl(): string {
  const proto = location.protocol === "https:" ? "wss" : "ws";
  const q = new URLSearchParams();
  if (state.seat !== null) q.set("seat", String(state.seat));
  if (state.key !== null) q.set("key", state.key);
  return `${proto}://${location.host}/ws?${q.toString()}`;
}

function connect(): void {
  if (params.has("nosocket")) return;
  retryTimer = null;
  state.retryAt = null;
  let ws: WebSocket;
  try { ws = new WebSocket(socketUrl()); } catch { scheduleReconnect(); return; }
  state.ws = ws;
  ws.onopen = () => {
    state.connected = true; exposed.connected = true;
    retryDelay = 500;
    // A name typed before the socket opened, or while it was down.
    if (nameField.value.trim()) send({ type: "name", name: nameField.value.trim() });
    draw();
  };
  ws.onclose = () => {
    if (state.ws !== ws) return;
    state.connected = false; exposed.connected = false;
    state.ws = null;
    scheduleReconnect();
    draw();
  };
  ws.onerror = () => { ws.close(); };
  ws.onmessage = (ev) => {
    try { onMessage(JSON.parse(ev.data as string) as ServerMessage); }
    catch (e) { console.error(e); }
  };
}

/** Reconnect with backoff: half a second, doubling to ten — unless the
 *  server refused this link itself (a wrong key, a seat that is not at
 *  the table), which no amount of retrying changes. */
function scheduleReconnect(): void {
  if (retryTimer !== null || state.rejected || state.closed !== null) return;
  state.reconnects++; exposed.reconnects = state.reconnects;
  state.retryAt = Date.now() + retryDelay;
  retryTimer = window.setTimeout(connect, retryDelay);
  retryDelay = Math.min(RETRY_MAX, retryDelay * 2);
}

function send(message: ClientMessage): boolean {
  exposed.sent.push({ at: Date.now(), message });
  if (state.ws && state.ws.readyState === WebSocket.OPEN) {
    state.ws.send(JSON.stringify(message));
    if (message.type === "deck") deckInFlight.push({ main: message.main, lands: message.lands, sideboard: message.sideboard });
    return true;
  }
  return false;
}

// -------------------------------------------------------------- messages

function onMessage(msg: ServerMessage): void {
  if (msg.type === "view") onView(msg);
  else if (msg.type === "refused") onRefused(msg);
  else if (msg.type === "closed") state.closed = msg.reason;
  else console.warn("unknown message", msg);
  draw();
}

function onView(view: DraftView): void {
  const prev = state.view;
  state.view = view;
  exposed.view = view;
  state.hint = null;
  // Kept until dismissed or replaced: "The table built your deck" went out
  // in the last building view and the playing view replaced it a
  // millisecond later, so nobody could read it (#817).
  if (view.notice) state.notice = view.notice;

  // Drafting: the selection belongs to one pack; a new pack starts clean.
  // The pending pick is answered when the pack it named is gone or moved
  // on, and given up after a while in case the server never applied it,
  // so a lost message cannot leave the page stuck at "picking…".
  const pack = view.pack;
  const samePack = !!(pack && prev && prev.pack && prev.pack.id === pack.id && prev.pack.pick === pack.pick);
  if (!samePack) { state.selected = null; state.hover = null; }
  if (state.pendingPick) {
    const stillThere = pack && pack.id === state.pendingPick.packId && pack.pick === state.pendingPick.pick;
    if (!stillThere || Date.now() - state.pendingPick.sentAt > 5000) state.pendingPick = null;
  }
  if (state.selected !== null && (!pack || !pack.cards.some(c => c.index === state.selected))) state.selected = null;
  // The countdown: the pick timer while a pack is in front, else the
  // build timer while the deck is not yet final.
  // Between games, the sideboard timer.
  const between = betweenOf(view);
  const buildLeft = view.phase === "building" && view.build_deadline_ms != null ? view.build_deadline_ms
    : between && !between.b.ready && between.b.deadline_ms !== null ? between.b.deadline_ms : null;
  state.deadlineAt = pack && pack.deadline_ms !== null ? performance.now() + pack.deadline_ms
    : buildLeft !== null ? performance.now() + buildLeft : null;

  // Building: the edit is the person's and survives the views that arrive
  // while they work (another seat's status, a pick elsewhere). It starts
  // from the deck the server records, and goes back to that whenever the
  // server's record differs from it with nothing of this page's in flight
  // — another tab changed it, or a test staged it — or the record is final
  // (the table built or topped up the deck, #815), because the server's
  // record is the one that will be played.
  //
  // A view built before the server read this page's latest clicks carries
  // an older record (every seat is sent a view on every change at the
  // table). Taking it undid those clicks, and the next click or Ready sent
  // the undone deck (#815's review): so while decks are in flight a record
  // is only matched against the oldest of them, which the server answers
  // first.
  if (view.phase === "building") {
    const me = view.seats.find(s => s.seat === view.seat);
    if (me && me.status === "ready") state.readySent = false;
    const fresh = !state.deck || state.deck.main.length !== view.pool.length || !prev || prev.phase !== "building";
    const edit = state.deck ? toMessage(state.deck, view.pool) as SentDeck : null;
    const settled = settle(view.deck, edit, deckInFlight, fresh);
    deckInFlight = settled.inFlight;
    if (settled.reset) state.deck = fromView(view.deck, view.pool);
    if (state.cursor !== null && state.cursor >= view.pool.length) state.cursor = null;
    state.between = null;
  } else if (between) {
    // Between games the edit starts from the deck played last game, once
    // per game: the server takes a whole deck or refuses it, so its record
    // is the last legal deck this page sent, never a click behind it.
    const key = `${between.m.round}-${between.b.game}`;
    if (state.between !== key) {
      state.between = key;
      state.deck = between.b.sideboarding ? fromView(between.b.deck, view.pool) : null;
      state.cursor = null;
      state.readySent = false;
    }
    deckInFlight = [];
  } else {
    state.deck = null; state.cursor = null; state.readySent = false; state.between = null; deckInFlight = [];
  }
}

/** The match of this seat's that is between games, if one is. */
function betweenOf(view: DraftView | null): { m: Match; b: Between } | null {
  const m = view ? view.matches.find(x => x.between) : undefined;
  return m && m.between ? { m, b: m.between } : null;
}

/** Send the deck being edited: while building every edit, short or not,
 *  which the server keeps as work in progress; between games only a deck
 *  that can be played, which is all the server takes there. */
function sendEdit(view: DraftView): void {
  if (!state.deck) return;
  if (state.between === null) send(toMessage(state.deck, view.pool));
  else if (!problem(state.deck, view.pool)) send(toSideboard(state.deck, view.pool));
}

/** The `deck` messages this page sent that no view has recorded yet,
 *  oldest first. */
let deckInFlight: SentDeck[] = [];

function onRefused(msg: Refused): void {
  state.refusal = { reason: msg.reason, echo: msg.echo };
  // A refusal with nothing echoed is the join itself: the link is wrong,
  // and the socket is about to close for good.
  if (msg.echo === null && !state.view) state.rejected = true;
  // Back to what the last view describes: the pick is not pending, the
  // deck is the server's, nothing is selected.
  state.pendingPick = null;
  state.selected = null;
  state.readySent = false;
  // A refused deck is out of flight. The server records one it refuses on
  // its contents, and the view after this says so; one it did not record
  // leaves the record as it was, and with nothing else in flight that view
  // puts the page back on it. Resetting here, from the view before the
  // refusal, could put the page on a deck older than the record.
  const echo = msg.echo as SentDeck & { type?: string } | null;
  if (echo && echo.type === "deck") {
    const i = deckInFlight.findIndex(d => sameDeck(echo as unknown as Deck, d));
    if (i >= 0) deckInFlight.splice(i, 1);
  }
}

// --------------------------------------------------------------- actions

const actions: Actions = {
  select(index) {
    state.refusal = null; state.hint = null;
    state.selected = state.selected === index ? null : index;
    draw();
  },
  pick(index) {
    const pack = state.view && state.view.pack;
    if (!pack || state.pendingPick) return;
    if (!pack.cards.some(c => c.index === index)) return;
    // A notice is about something the table did; the person acting is past
    // it ("The table picked X for you" under every later pick, #817's review).
    state.refusal = null; state.hint = null; state.notice = null;
    send({ type: "pick", pack_id: pack.id, index });
    state.pendingPick = { packId: pack.id, pick: pack.pick, index, sentAt: Date.now() };
    state.selected = index;
    draw();
  },
  hover(index) {
    // In the one-column layout (draft.html's 760px breakpoint) the detail
    // panel is ABOVE the pack, so a hover that changes it moves every tile:
    // a tap fired hover first, the pack jumped and the tap landed on nothing
    // (#791), and a pointer left over the pack swapped the detail between
    // two cards on every redraw. There the panel follows the selection only.
    if (ONE_COLUMN.matches) { if (state.hover !== null) { state.hover = null; draw(); } return; }
    if (state.hover === index) return;
    state.hover = index;
    draw();
  },
  toggle(poolIndex, keyboard = false) {
    const view = state.view;
    if (!view || !state.deck || state.readySent) return;
    if (poolIndex < 0 || poolIndex >= state.deck.main.length) return;
    state.refusal = null; state.hint = null;
    // On a phone the rules text is a tooltip nobody can raise and the
    // detail panel is below the whole pool, so the only way to read a card
    // was to move it and scroll two screens down (#792). There a first tap
    // reads the card, under its own row, and a second moves it — the pick
    // phase's "tap to read, tap again to pick".
    // A key is not a tap: Enter on a focused row moves it at any width.
    if (ONE_COLUMN.matches && !keyboard && state.cursor !== poolIndex) {
      // The row tapped stays under the finger: another row's open text
      // above it closes on this draw, and the rows below move up by its
      // height, so "tap it again" would land on a different row (#791's
      // jump, on this list).
      const sel = `.row[data-pool-index="${poolIndex}"]`;
      const before = root.querySelector<HTMLElement>(sel)?.getBoundingClientRect().top;
      state.cursor = poolIndex;
      draw();
      const after = root.querySelector<HTMLElement>(sel)?.getBoundingClientRect().top;
      if (before !== undefined && after !== undefined && after !== before) window.scrollBy(0, after - before);
      return;
    }
    state.deck.main[poolIndex] = !state.deck.main[poolIndex];
    state.cursor = poolIndex;
    sendEdit(view);
    draw();
  },
  setLand(name, n) {
    const view = state.view;
    if (!view || !state.deck || state.readySent) return;
    state.refusal = null; state.hint = null;
    state.deck.lands[name] = Math.max(0, Math.floor(n));
    sendEdit(view);
    draw();
  },
  ready() {
    const view = state.view;
    if (!view || !state.deck || state.readySent) return;
    const why = problem(state.deck, view.pool);
    if (why) { state.hint = why; draw(); return; }
    state.refusal = null; state.hint = null; state.notice = null;
    send(toMessage(state.deck, view.pool));
    send({ type: "ready" });
    state.readySent = true;
    draw();
  },
  playDraw(play) {
    const between = betweenOf(state.view);
    if (!between || !between.b.you_choose || between.b.ready || state.readySent) return;
    state.refusal = null; state.hint = null;
    send({ type: "play_draw", play });
    draw();
  },
  sideboardReady() {
    const view = state.view;
    const between = betweenOf(view);
    if (!view || !between || between.b.ready || state.readySent) return;
    if (between.b.sideboarding && state.deck) {
      const why = problem(state.deck, view.pool);
      if (why) { state.hint = why; draw(); return; }
      send(toSideboard(state.deck, view.pool));
    }
    state.refusal = null; state.hint = null; state.notice = null;
    send({ type: "sideboard_ready" });
    state.readySent = true;
    draw();
  },
  dismiss() { state.refusal = null; draw(); },
  dismissNotice() { state.notice = null; draw(); },
};

// ------------------------------------------------------------------ keys

window.addEventListener("keydown", (ev) => {
  if (ev.target === nameField) {
    if (ev.key === "Enter" || ev.key === "Escape") { nameField.blur(); ev.preventDefault(); }
    return;
  }
  if (ev.ctrlKey || ev.metaKey || ev.altKey) return;
  // A held Enter repeats about every 30 ms, and on a focused row or tile
  // the browser clicked it once per repeat: a pool card went in and out of
  // the deck by the parity of the repeat count, one deck message each, and
  // an unselected tile was selected and then picked (#852, this page's
  // #806). Only the land steppers repeat, as a held stepper does.
  if (ev.key === "Enter" && ev.repeat && !(ev.target instanceof Element && ev.target.closest(".inc, .dec"))) {
    ev.preventDefault();
    return;
  }
  // Enter on a focused control is that control's: the browser clicks it.
  // Taken here, Enter picked the selected card instead of the focused one,
  // and on Ready or a land's "+" moved a pool card (#787). Enter acts on the
  // selection or the cursor only when no control has the focus.
  if (ev.key === "Enter" && ev.target instanceof Element && ev.target.closest("button, a[href], input, select, textarea")) return;
  const view = state.view;
  if (!view) return;
  if (view.phase === "drafting") draftingKey(ev, view);
  else if (view.phase === "building" || (state.between !== null && state.deck)) buildingKey(ev, view);
  else if (ev.key === "Escape" && state.refusal) { state.refusal = null; ev.preventDefault(); draw(); }
  else if (ev.key === "Escape" && state.notice) { state.notice = null; ev.preventDefault(); draw(); }
});

function draftingKey(ev: KeyboardEvent, view: DraftView): void {
  const pack = view.pack;
  if (!pack || pack.cards.length === 0) return;
  const indices = pack.cards.map(c => c.index);
  const at = state.selected === null ? -1 : indices.indexOf(state.selected);
  const cols = packColumns(root);
  const move = (to: number) => { state.selected = indices[Math.max(0, Math.min(indices.length - 1, to))]; };
  let handled = true;
  switch (ev.key) {
    case "ArrowRight": move(at < 0 ? 0 : at + 1); break;
    case "ArrowLeft": move(at < 0 ? 0 : at - 1); break;
    case "ArrowDown": move(at < 0 ? 0 : at + cols); break;
    case "ArrowUp": move(at < 0 ? 0 : at - cols); break;
    case "Enter":
      if (state.pendingPick) { state.hint = "Your pick is on its way."; break; }
      if (state.selected === null) { state.hint = "Enter picks the selected card — select one first with a click, a digit or the arrows."; break; }
      actions.pick(state.selected);
      return;
    case "Escape":
      if (state.refusal) state.refusal = null;
      else if (state.notice) state.notice = null;
      else state.selected = null;
      state.hint = null;
      break;
    default:
      if (/^[0-9]$/.test(ev.key)) {
        const n = ev.key === "0" ? 10 : Number(ev.key);
        if (n <= indices.length) move(n - 1);
        else state.hint = `There is no card ${n}; the pack has ${indices.length}.`;
      } else handled = false;
  }
  if (!handled) return;
  ev.preventDefault();
  if (ev.key !== "Escape" && ev.key !== "Enter") { state.hint = null; state.refusal = null; }
  draw();
  if (state.selected !== null) carryFocus(`.tile[data-card-index="${state.selected}"]`);
}

/**
 * Arrows and digits move the selection or the cursor; where a control has
 * the keyboard focus, the focus moves with them. Left on the control that
 * was clicked (draw() puts it back, #788), it took the next Enter — which
 * is the control's own since #787 — and Enter re-selected the old tile or
 * toggled the old row instead of acting on the one the arrows reached.
 */
function carryFocus(selector: string): void {
  if (!(document.activeElement instanceof HTMLElement) || !root.contains(document.activeElement)) return;
  const el = root.querySelector<HTMLElement>(selector);
  if (el) el.focus({ preventScroll: true });
}

function buildingKey(ev: KeyboardEvent, view: DraftView): void {
  if (!state.deck) return;
  // The rows in the order the checklist shows them.
  const rows = [...root.querySelectorAll<HTMLElement>(".row[data-pool-index]")].map(r => Number(r.dataset.poolIndex));
  if (rows.length === 0) return;
  const at = state.cursor === null ? -1 : rows.indexOf(state.cursor);
  const move = (to: number) => { state.cursor = rows[Math.max(0, Math.min(rows.length - 1, to))]; };
  let handled = true;
  switch (ev.key) {
    case "ArrowDown": case "ArrowRight": move(at < 0 ? 0 : at + 1); break;
    case "ArrowUp": case "ArrowLeft": move(at < 0 ? 0 : at - 1); break;
    case "Enter":
      if (state.cursor === null) { state.hint = "Enter moves the card under the cursor; arrows move the cursor, or click a card."; break; }
      actions.toggle(state.cursor, true);
      ev.preventDefault();
      return;
    case "Escape":
      if (state.refusal) state.refusal = null;
      else if (state.notice) state.notice = null;
      else state.cursor = null;
      state.hint = null;
      break;
    default: handled = false;
  }
  if (!handled) return;
  ev.preventDefault();
  if (ev.key !== "Escape" && ev.key !== "Enter") state.hint = null;
  draw();
  if (state.cursor !== null) carryFocus(`.row[data-pool-index="${state.cursor}"]`);
  const el = state.cursor !== null ? root.querySelector<HTMLElement>(`.row[data-pool-index="${state.cursor}"]`) : null;
  if (el) el.scrollIntoView({ block: "nearest" });
}

// The name box lives outside the rebuilt tree so typing survives a view.
nameField.addEventListener("change", () => {
  const name = nameField.value.trim();
  if (name) send({ type: "name", name });
});

// ----------------------------------------------------------------- frame

/**
 * A selector for the focused control that finds the same control in the
 * rebuilt tree, or null. Every draw rebuilds the page, so the focus went to
 * <body> on each press: adding 17 basic lands from the keyboard took 764
 * Tabs (#788), and a hover's redraw between a Tab and an Enter sent the
 * Enter to the page instead of the control (#787).
 */
function focusKey(el: Element | null): string | null {
  if (!(el instanceof HTMLElement) || !root.contains(el)) return null;
  if (el.id) return `#${CSS.escape(el.id)}`;
  if (el.dataset.cardIndex !== undefined) return `.tile[data-card-index="${el.dataset.cardIndex}"]`;
  if (el.dataset.poolIndex !== undefined) return `.row[data-pool-index="${el.dataset.poolIndex}"]`;
  const land = el.closest<HTMLElement>("[data-land]");
  if (land && el.classList.length) return `[data-land="${land.dataset.land}"] > .${CSS.escape(el.classList[0])}`;
  return null;
}

function draw(): void {
  const refocus = focusKey(document.activeElement);
  try { render(root, state, actions); } catch (e) { console.error(e); }
  if (refocus) {
    const again = root.querySelector<HTMLElement>(refocus);
    if (again && !(again as HTMLButtonElement).disabled) again.focus({ preventScroll: true });
  }
  const v = state.view;
  seatEl.textContent = v ? `seat ${v.seat} of ${v.pod_size} · ${v.set.toUpperCase()} · ${v.phase}` : (state.seat !== null ? `seat ${state.seat}` : "");
  connEl.textContent = state.closed !== null ? "the table is closed"
    : state.connected ? "connected"
    : state.rejected ? "refused — check the link"
    : state.retryAt ? `disconnected — retrying in ${Math.max(0, Math.ceil((state.retryAt - Date.now()) / 1000))}s`
    : params.has("nosocket") ? "no socket" : "connecting…";
  connEl.className = state.connected ? "ok" : "down";
  tick();
}

/** The countdown and the retry line, once a second, without a rebuild. */
function tick(): void {
  const el = document.getElementById("countdown");
  if (el) {
    if (state.deadlineAt === null) el.textContent = "";
    else {
      const left = Math.max(0, Math.ceil((state.deadlineAt - performance.now()) / 1000));
      el.textContent = `${left}s left`;
      el.classList.toggle("urgent", left <= 10);
    }
  }
  if (!state.connected && state.retryAt && !state.rejected && state.closed === null) {
    connEl.textContent = `disconnected — retrying in ${Math.max(0, Math.ceil((state.retryAt - Date.now()) / 1000))}s`;
  }
}
setInterval(tick, 1000);

window.mtgDraftDebug = {
  stage(message) {
    const typed = message as { type?: string };
    const msg: ServerMessage = typed.type ? (message as ServerMessage) : { ...(message as DraftView), type: "view" };
    // A staged view is a fresh start, not a change to the one before it:
    // nothing selected, nothing on its way, nothing refused.
    if (msg.type === "view") { state.pendingPick = null; state.selected = null; state.hover = null; state.refusal = null; state.hint = null; deckInFlight = []; }
    onMessage(msg);
  },
  refuse(reason, echo = null) { onMessage({ type: "refused", reason, echo }); },
  render: draw,
};

void (async () => {
  await Promise.all([loadManifest(), fontsReady()]);
  draw();
  connect();
})();

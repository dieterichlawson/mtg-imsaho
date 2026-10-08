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
import { fromView, problem, toMessage } from "./deck.js";
import { packColumns, render } from "./render.js";
import type { ClientMessage, Deck, DraftView, Refused, ServerMessage } from "./protocol.js";
import { initialState, type Actions, type PageState, type Sent } from "./state.js";

const root = document.getElementById("app") as HTMLElement;
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

/** Reconnect with backoff: half a second, doubling to ten. */
function scheduleReconnect(): void {
  if (retryTimer !== null) return;
  state.reconnects++; exposed.reconnects = state.reconnects;
  state.retryAt = Date.now() + retryDelay;
  retryTimer = window.setTimeout(connect, retryDelay);
  retryDelay = Math.min(RETRY_MAX, retryDelay * 2);
}

function send(message: ClientMessage): boolean {
  exposed.sent.push({ at: Date.now(), message });
  if (message.type === "deck") lastDeckSent = { main: message.main, lands: message.lands, sideboard: message.sideboard };
  if (state.ws && state.ws.readyState === WebSocket.OPEN) {
    state.ws.send(JSON.stringify(message));
    return true;
  }
  return false;
}

// -------------------------------------------------------------- messages

function onMessage(msg: ServerMessage): void {
  if (msg.type === "view") onView(msg);
  else if (msg.type === "refused") onRefused(msg);
  else console.warn("unknown message", msg);
  draw();
}

function onView(view: DraftView): void {
  const prev = state.view;
  state.view = view;
  exposed.view = view;
  state.hint = null;

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
  const buildLeft = view.phase === "building" && view.build_deadline_ms != null ? view.build_deadline_ms : null;
  state.deadlineAt = pack && pack.deadline_ms !== null ? performance.now() + pack.deadline_ms
    : buildLeft !== null ? performance.now() + buildLeft : null;

  // Building: the edit is the person's and survives the views that arrive
  // while they work (another seat's status, a pick elsewhere). It starts
  // from the deck the server records, and goes back to that whenever the
  // server's record is not the deck this page last sent — the server
  // normalised it, another tab changed it, or a test staged it — because
  // the server's record is the one that will be played.
  if (view.phase === "building") {
    const me = view.seats.find(s => s.seat === view.seat);
    if (me && me.status === "ready") state.readySent = false;
    const fresh = !state.deck || state.deck.main.length !== view.pool.length || !prev || prev.phase !== "building";
    if (fresh || (view.deck && !sameDeck(view.deck, lastDeckSent))) {
      state.deck = fromView(view.deck, view.pool);
    }
    if (state.cursor !== null && state.cursor >= view.pool.length) state.cursor = null;
  } else {
    state.deck = null; state.cursor = null; state.readySent = false; lastDeckSent = null;
  }
}

/** The last `deck` message this page sent, to tell the server echoing it
 *  back from the server recording something else. */
let lastDeckSent: { main: string[]; lands: Record<string, number>; sideboard: string[] } | null = null;

function sameDeck(a: Deck, b: typeof lastDeckSent): boolean {
  if (!b) return false;
  const sorted = (xs: string[]) => [...xs].sort().join("\n");
  const lands = (l: Record<string, number>) => Object.entries(l).filter(([, n]) => n > 0).sort().map(([k, n]) => `${k}=${n}`).join(",");
  return sorted(a.main) === sorted(b.main) && sorted(a.sideboard) === sorted(b.sideboard) && lands(a.lands) === lands(b.lands);
}

function onRefused(msg: Refused): void {
  state.refusal = { reason: msg.reason, echo: msg.echo };
  // Back to what the last view describes: the pick is not pending, the
  // deck is the server's, nothing is selected.
  state.pendingPick = null;
  state.selected = null;
  state.readySent = false;
  if (state.view && state.view.phase === "building") state.deck = fromView(state.view.deck, state.view.pool);
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
    state.refusal = null; state.hint = null;
    send({ type: "pick", pack_id: pack.id, index });
    state.pendingPick = { packId: pack.id, pick: pack.pick, index, sentAt: Date.now() };
    state.selected = index;
    draw();
  },
  hover(index) {
    if (state.hover === index) return;
    state.hover = index;
    draw();
  },
  toggle(poolIndex) {
    const view = state.view;
    if (!view || !state.deck || state.readySent) return;
    if (poolIndex < 0 || poolIndex >= state.deck.main.length) return;
    state.refusal = null; state.hint = null;
    state.deck.main[poolIndex] = !state.deck.main[poolIndex];
    state.cursor = poolIndex;
    send(toMessage(state.deck, view.pool));
    draw();
  },
  setLand(name, n) {
    const view = state.view;
    if (!view || !state.deck || state.readySent) return;
    state.refusal = null; state.hint = null;
    state.deck.lands[name] = Math.max(0, Math.floor(n));
    send(toMessage(state.deck, view.pool));
    draw();
  },
  ready() {
    const view = state.view;
    if (!view || !state.deck || state.readySent) return;
    const why = problem(state.deck, view.pool);
    if (why) { state.hint = why; draw(); return; }
    state.refusal = null; state.hint = null;
    send(toMessage(state.deck, view.pool));
    send({ type: "ready" });
    state.readySent = true;
    draw();
  },
  dismiss() { state.refusal = null; draw(); },
};

// ------------------------------------------------------------------ keys

window.addEventListener("keydown", (ev) => {
  if (ev.target === nameField) {
    if (ev.key === "Enter" || ev.key === "Escape") { nameField.blur(); ev.preventDefault(); }
    return;
  }
  if (ev.ctrlKey || ev.metaKey || ev.altKey) return;
  const view = state.view;
  if (!view) return;
  if (view.phase === "drafting") draftingKey(ev, view);
  else if (view.phase === "building") buildingKey(ev, view);
  else if (ev.key === "Escape" && state.refusal) { state.refusal = null; ev.preventDefault(); draw(); }
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
      actions.toggle(state.cursor);
      ev.preventDefault();
      return;
    case "Escape":
      if (state.refusal) state.refusal = null;
      else state.cursor = null;
      state.hint = null;
      break;
    default: handled = false;
  }
  if (!handled) return;
  ev.preventDefault();
  if (ev.key !== "Escape" && ev.key !== "Enter") state.hint = null;
  draw();
  const el = state.cursor !== null ? root.querySelector<HTMLElement>(`.row[data-pool-index="${state.cursor}"]`) : null;
  if (el) el.scrollIntoView({ block: "nearest" });
}

// The name box lives outside the rebuilt tree so typing survives a view.
nameField.addEventListener("change", () => {
  const name = nameField.value.trim();
  if (name) send({ type: "name", name });
});

// ----------------------------------------------------------------- frame

function draw(): void {
  try { render(root, state, actions); } catch (e) { console.error(e); }
  const v = state.view;
  seatEl.textContent = v ? `seat ${v.seat} of ${v.pod_size} · ${v.set.toUpperCase()} · ${v.phase}` : (state.seat !== null ? `seat ${state.seat}` : "");
  connEl.textContent = state.connected ? "connected"
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
  if (!state.connected && state.retryAt) {
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
    if (msg.type === "view") { state.pendingPick = null; state.selected = null; state.hover = null; state.refusal = null; state.hint = null; }
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

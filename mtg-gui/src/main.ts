// The page: one WebSocket to the seat, one canvas, one state object.

import { loadManifest, fontsReady } from "./assets.js";
import { render, W, H, PANEL_X } from "./render.js";
import { beginDecision, indexView, beginList } from "./prompts.js";
import type { Action, ClientMessage, Decision, GameView, ServerMessage } from "./protocol.js";
import type { Hit, LiveState, Row, State } from "./state.js";

const canvas = document.getElementById("game") as HTMLCanvasElement;
const ctx = canvas.getContext("2d")!;
const field = document.getElementById("field") as HTMLInputElement;

const state: State = {
  ws: null, connected: false,
  view: null, index: new Map(),
  decision: null,      // {seq, legal, combat}
  ui: null,            // the widget answering it
  gameOver: null,
  hover: null, hits: [], popover: null, overlay: null, selected: null,
  notice: null, logOpen: false, logScroll: 0,
  scale: 1,
};

/** What tests and the console see. */
interface DebugHook {
  stage(decision: Decision): void;
  render(): number;
  sent: { seq: number; action: Action }[];
}
declare global {
  interface Window {
    mtg: State;
    mtgSend: (action: Action) => void;
    mtgList: (rows: (Row | Action)[], title: string) => void;
    mtgDebug: DebugHook;
  }
}
window.mtg = state;

function live(): LiveState | null {
  return state.view ? (state as LiveState) : null;
}

// ---------------------------------------------------------------- socket

function connect(): void {
  const url = `${location.protocol === "https:" ? "wss" : "ws"}://${location.host}/ws`;
  const ws = new WebSocket(url);
  state.ws = ws;
  ws.onopen = () => { state.connected = true; sendRaw({ type: "hello" }); draw(); };
  ws.onclose = () => { state.connected = false; draw(); setTimeout(connect, 1000); };
  ws.onerror = () => { ws.close(); };
  ws.onmessage = (ev) => { try { onMessage(JSON.parse(ev.data as string) as ServerMessage); } catch (e) { console.error(e); } };
}

function sendRaw(msg: ClientMessage): void {
  if (state.ws && state.ws.readyState === WebSocket.OPEN) state.ws.send(JSON.stringify(msg));
}

function send(action: Action): void {
  if (!state.decision) return;
  const seq = state.decision.seq;
  window.mtgDebug.sent.push({ seq, action });
  sendRaw({ type: "action", seq, action });
  state.lastSent = { seq, action };
  // Nothing more to click until the next message; the board stays.
  state.decision = null; state.ui = null; state.popover = null;
  state.notice = null;
  hideField();
  draw();
}

function setView(view: GameView): void {
  state.view = view;
  state.index = indexView(view);
}

function onMessage(msg: ServerMessage): void {
  switch (msg.type) {
    case "view":
      setView(msg.view);
      if (state.decision === null) { state.ui = null; state.popover = null; }
      break;
    case "decision": {
      setView(msg.view);
      state.decision = { seq: msg.seq, legal: msg.legal, combat: msg.combat };
      state.popover = null; state.overlay = null;
      const actions = msg.legal.actions || [];
      // Only Pass and Concede on offer: nothing to decide, as in the CLI.
      const onlyPass = !msg.combat && !msg.legal.resolution_prompt && !msg.legal.set_prompt
        && actions.length > 0 && actions.every(a => a === "PassPriority" || a === "Concede") && actions.includes("PassPriority");
      if (onlyPass && !state.stopAtPass) { send("PassPriority"); return; }
      beginDecision(state as LiveState, send);
      syncField();
      break;
    }
    case "notice":
      state.notice = msg.text;
      // The decision stands; if we had cleared it on send, ask again.
      if (!state.decision && state.lastSent && state.lastSent.seq === msg.seq && state.lastDecision && state.view) {
        state.decision = state.lastDecision;
        beginDecision(state as LiveState, send);
      }
      break;
    case "game_over":
      setView(msg.view);
      state.decision = null; state.ui = null; state.popover = null;
      state.gameOver = msg.summary;
      break;
    default:
      console.warn("unknown message", msg);
  }
  if (state.decision) state.lastDecision = state.decision;
  draw();
}

// ----------------------------------------------------------------- input

function fitCanvas(): void {
  const s = Math.max(1, Math.floor(Math.min(window.innerWidth / W, window.innerHeight / H)));
  state.scale = s;
  canvas.style.width = `${W * s}px`; canvas.style.height = `${H * s}px`;
  draw();
}

function canvasPoint(ev: MouseEvent): { x: number; y: number } {
  const r = canvas.getBoundingClientRect();
  return { x: (ev.clientX - r.left) / state.scale, y: (ev.clientY - r.top) / state.scale };
}

function hitAt(x: number, y: number): Hit | null {
  for (let i = state.hits.length - 1; i >= 0; i--) {
    const h = state.hits[i];
    if (x >= h.x && x < h.x + h.w && y >= h.y && y < h.y + h.h) return h;
  }
  return null;
}

canvas.addEventListener("mousemove", (ev) => {
  const { x, y } = canvasPoint(ev);
  const h = hitAt(x, y);
  const next = h && (h.key || h.kind === "row" || h.kind === "button" || h.cardName) ? h : null;
  const changed = (next && next.key) !== (state.hover && state.hover.key) || (next && next.y) !== (state.hover && state.hover.y);
  state.hover = next;
  // A row that names a card lets the inspector read it.
  if (next && next.cardName && !next.key) {
    const e = [...state.index.values()].find(v => v.obj.name === next.cardName);
    if (e) state.hover = { ...next, key: `o${e.obj.object_id}` };
  }
  canvas.style.cursor = h && h.onClick ? "pointer" : "default";
  if (changed) draw();
});
canvas.addEventListener("mouseleave", () => { state.hover = null; draw(); });

canvas.addEventListener("click", (ev) => {
  const { x, y } = canvasPoint(ev);
  const h = hitAt(x, y);
  state.notice = null;
  if (state.popover && (!h || (h.kind !== "row" && h.kind !== "popover"))) { state.popover = null; draw(); return; }
  if (h && h.onClick) h.onClick(h);
  else if (h && h.kind === "modal") { /* inside a modal: nothing */ }
  else if (h && h.key && h.id !== undefined) state.selected = h.id;
  syncField();
  draw();
});

canvas.addEventListener("wheel", (ev) => {
  const ui = state.ui;
  if (ui && ui.mode === "list" && ui.rows) {
    ui.scroll = Math.max(0, Math.min(Math.max(0, ui.rows.length - 20), (ui.scroll || 0) + Math.sign(ev.deltaY) * 3));
  } else if (state.logOpen) {
    state.logScroll = Math.max(0, state.logScroll - Math.sign(ev.deltaY) * 2);
  }
  ev.preventDefault();
  draw();
}, { passive: false });

window.addEventListener("keydown", (ev) => {
  if (ev.target === field) {
    if (ev.key === "Enter") { const ui = state.ui; if (ui && ui.mode === "number" && ui.submit) ui.submit(); ev.preventDefault(); }
    if (ev.key === "Escape") { field.blur(); }
    return;
  }
  const ui = state.ui;
  const v = state.view;
  switch (ev.key) {
    case "Enter":
      if (!ui) return;
      if (ui.mode === "menu" && ui.canPass) send("PassPriority");
      else if (ui.onConfirm) ui.onConfirm();
      else if (ui.mode === "number" && ui.submit) ui.submit();
      break;
    case "Escape":
      if (state.popover) state.popover = null;
      else if (state.overlay) state.overlay = null;
      else if (ui && ui.onCancel) ui.onCancel();
      break;
    case "l": state.logOpen = !state.logOpen; state.logScroll = 0; break;
    case "g": if (v) toggleZone("graveyard", v.you); break;
    case "G": if (v && v.opponents[0]) toggleZone("graveyard", v.opponents[0].id); break;
    case "e": if (v) toggleZone("exile", v.you); break;
    case "d": if (v) toggleZone("library", v.you); break;
    case "s": state.stopAtPass = !state.stopAtPass; state.notice = state.stopAtPass ? "Stopping at every priority." : "Passing automatically when there is nothing to do."; break;
    default: return;
  }
  ev.preventDefault();
  draw();
});

function toggleZone(zone: "graveyard" | "exile" | "library", pid: number): void {
  state.overlay = state.overlay && state.overlay.zone === zone && state.overlay.pid === pid ? null : { zone, pid };
}

// The one DOM control: typed input for a filter or a number.
function syncField(): void {
  const ui = state.ui;
  const wants = ui && ((ui.mode === "list" && ui.filter) || ui.mode === "number");
  if (!wants || !ui) { hideField(); return; }
  const r = canvas.getBoundingClientRect();
  field.style.display = "block";
  field.style.left = `${r.left + (PANEL_X / 2 - 150 + 6) * state.scale}px`;
  field.style.top = `${r.top + 4 * state.scale}px`;
  field.style.width = `${280 * state.scale}px`;
  field.style.fontSize = `${8 * state.scale}px`;
  field.value = ui.mode === "number" ? (ui.value ?? "") : (ui.query ?? "");
  field.placeholder = ui.mode === "number" ? `X (0-${ui.max})` : "filter";
  field.focus();
}
function hideField(): void { field.style.display = "none"; field.value = ""; }
field.addEventListener("input", () => {
  const ui = state.ui;
  if (!ui) return;
  if (ui.mode === "number") ui.value = field.value.trim();
  else if (ui.mode === "list") { ui.query = field.value; ui.scroll = 0; }
  draw();
});

// ----------------------------------------------------------------- frame

let pending = false;
function draw(): void {
  if (pending) return;
  pending = true;
  requestAnimationFrame(() => {
    pending = false;
    try { state.hits = render(ctx, state); } catch (e) { console.error(e); }
  });
}
state.draw = draw;
// Art arrives over time; repaint while anything is still loading.
setInterval(draw, 500);

window.addEventListener("resize", fitCanvas);
void (async () => {
  await Promise.all([loadManifest(), fontsReady()]);
  fitCanvas();
  connect();
})();

// Exposed for tests: answer the current decision with a raw action, or
// stage a synthetic decision and render it without a socket.
window.mtgSend = send;
window.mtgList = (rows, title) => { const l = live(); if (l) beginList(l, { mode: "list", title, hint: "", buttons: [], marked: [] }, rows, title, send, false); };
window.mtgDebug = {
  stage(decision) {
    const l = live();
    if (!l) throw new Error("no view to stage a decision over");
    state.decision = decision; state.lastDecision = decision; state.popover = null; state.overlay = null; state.notice = null;
    beginDecision(l, send);
    syncField();
    state.hits = render(ctx, state);
  },
  render() { state.hits = render(ctx, state); return state.hits.length; },
  sent: [],
};

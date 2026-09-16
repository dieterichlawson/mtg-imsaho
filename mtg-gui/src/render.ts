// Drawing the board at 640x360, and saying what was drawn where.
//
// Every call to `render` returns the list of hit rectangles it drew, with
// what each one is and what a click on it does. Input never looks at the
// view; it looks at the last frame.

import { drawArt, frameColor, darker, uiImage } from "./assets.js";
import { nameOf, targetLabel, playerLabel } from "./prompts.js";
import type { CardView, Color, GameView, ManaCost, ManaPool, ObjectId, PermanentView, PlayerId, Step, ViewObject } from "./protocol.js";
import type { Hit, LiveState, Row, State } from "./state.js";

export const W = 640, H = 360;
export const PANEL_X = 480;
const BOARD_W = PANEL_X;
const CARD = { w: 40, h: 52 };  // on the battlefield
const ART_S = { w: 32, h: 24 };
const HAND = { w: 66, h: 92 };
const ART_L = { w: 64, h: 48 };
const ROWS = {
  oppStrip: 0, oppOther: 16, oppCreatures: 68, band: 120, myCreatures: 152, myOther: 204, hand: 268,
};
const STEPS: Step[] = ["Untap", "Upkeep", "Draw", "PrecombatMain", "BeginCombat", "DeclareAttackers",
  "DeclareBlockers", "CombatDamage", "EndCombat", "PostcombatMain", "EndStep", "Cleanup"];
const STEP_SHORT: Record<Step, string> = { Untap: "UN", Upkeep: "UP", Draw: "DR", PrecombatMain: "M1", BeginCombat: "BC",
  DeclareAttackers: "DA", DeclareBlockers: "DB", CombatDamage: "CD", EndCombat: "EC", PostcombatMain: "M2",
  EndStep: "ES", Cleanup: "CL" };
const KEYWORD_SHORT: Record<string, string> = { Flying: "Fly", FirstStrike: "FS", DoubleStrike: "DS", Trample: "Tr", Deathtouch: "DT",
  Lifelink: "LL", Vigilance: "Vig", Flash: "Fl", Reach: "Rch", Haste: "Hst", Defender: "Def", Hexproof: "Hex",
  Intimidate: "Int", Menace: "Men", Indestructible: "Ind" };

type Ctx = CanvasRenderingContext2D;
interface TextOpts { font?: string; color?: string; align?: CanvasTextAlign }

// ------------------------------------------------------------ text helpers

export function text(ctx: Ctx, s: string, x: number, y: number, { font = "8px Silkscreen", color = "#e8e0d0", align = "left" }: TextOpts = {}): void {
  ctx.font = font; ctx.fillStyle = color; ctx.textAlign = align; ctx.textBaseline = "top";
  ctx.fillText(s, Math.round(x), Math.round(y));
  ctx.textAlign = "left";
}

export function wrap(ctx: Ctx, s: string, maxW: number, font = "8px Silkscreen"): string[] {
  ctx.font = font;
  const lines: string[] = [];
  for (const para of String(s).split("\n")) {
    let line = "";
    for (const word of para.split(" ")) {
      const t = line ? line + " " + word : word;
      if (ctx.measureText(t).width <= maxW || !line) line = t;
      else { lines.push(line); line = word; }
    }
    lines.push(line);
  }
  return lines;
}

function clip(ctx: Ctx, s: string, maxW: number, font = "8px Silkscreen"): string {
  ctx.font = font;
  if (ctx.measureText(s).width <= maxW) return s;
  let t = s;
  while (t.length > 1 && ctx.measureText(t + "…").width > maxW) t = t.slice(0, -1);
  return t + "…";
}

function panel(ctx: Ctx, x: number, y: number, w: number, h: number, fill = "#1a1620", stroke = "#5a4a6a"): void {
  ctx.fillStyle = fill; ctx.fillRect(x, y, w, h);
  ctx.strokeStyle = stroke; ctx.lineWidth = 1; ctx.strokeRect(x + 0.5, y + 0.5, w - 1, h - 1);
}

function button(ctx: Ctx, hits: Hit[], x: number, y: number, w: number, h: number, label: string, run: () => void,
  { primary = false, enabled = true }: { primary?: boolean; enabled?: boolean } = {}): void {
  panel(ctx, x, y, w, h, enabled ? (primary ? "#4a3a5a" : "#2a2430") : "#1c1a20", primary ? "#c9a84a" : "#6a5a7a");
  text(ctx, label, x + w / 2, y + (h - 8) / 2, { align: "center", color: enabled ? "#f0e8d8" : "#7a7280", font: "8px Silkscreen" });
  hits.push({ x, y, w, h, kind: "button", label, onClick: enabled ? run : null });
}

/** Draw `img` stretched to w×h keeping its `slice`-pixel border crisp. */
function nineSlice(ctx: Ctx, img: HTMLImageElement, x: number, y: number, w: number, h: number, slice: number): void {
  const iw = img.width, ih = img.height, s = slice;
  const cols = [[0, s, x, s], [s, iw - 2 * s, x + s, w - 2 * s], [iw - s, s, x + w - s, s]];
  const rows = [[0, s, y, s], [s, ih - 2 * s, y + s, h - 2 * s], [ih - s, s, y + h - s, s]];
  for (const [sx, sw, dx, dw] of cols) for (const [sy, sh, dy, dh] of rows) {
    if (sw > 0 && sh > 0 && dw > 0 && dh > 0) ctx.drawImage(img, sx, sy, sw, sh, dx, dy, dw, dh);
  }
}

/** A textured panel where the art exists, a flat one where it does not. */
function texturedPanel(ctx: Ctx, x: number, y: number, w: number, h: number, fallback = "#171320", stroke = "#4a3a5a"): void {
  const img = uiImage("panel");
  if (img) {
    // The riveted border is the point; the mottled middle is not a
    // background text can be read on.
    nineSlice(ctx, img, x, y, w, h, 9);
    ctx.fillStyle = "rgba(20,17,26,0.92)"; ctx.fillRect(x + 7, y + 7, w - 14, h - 14);
  } else panel(ctx, x, y, w, h, fallback, stroke);
}

/** 0..1, slowly, so a highlight breathes rather than blinks. */
function pulse(): number { return (Math.sin(Date.now() / 220) + 1) / 2; }

// --------------------------------------------------------------- cards

function manaDots(ctx: Ctx, cost: ManaCost | null | undefined, x: number, y: number): number {
  if (!cost || !cost.symbols) return x;
  const col: Record<Color, string> = { White: "#f0e6c0", Blue: "#5088e0", Black: "#403050", Red: "#e05040", Green: "#50a050" };
  for (const s of cost.symbols) {
    let fill = "#9a9aa0", label = "";
    if (s === "X") label = "X";
    else if (typeof s === "object" && "Colored" in s) fill = col[s.Colored as Color] || fill;
    else if (typeof s === "object" && "Generic" in s) label = String(s.Generic);
    ctx.fillStyle = fill; ctx.beginPath(); ctx.arc(x + 3.5, y + 3.5, 3.5, 0, Math.PI * 2); ctx.fill();
    if (label) text(ctx, label, x + 3.5, y, { font: "7px Silkscreen", color: "#101010", align: "center" });
    x += 8;
  }
  return x;
}

/** Status marks for a permanent: tapped, attacking, blocking, sick, counters, damage. */
function permBadges(p: PermanentView, state: State): { t: string; c: string }[] {
  const b: { t: string; c: string }[] = [];
  if (p.attacking) b.push({ t: "ATK", c: "#e07040" });
  if (p.blocking && p.blocking.length) b.push({ t: "BLK", c: "#60a0e0" });
  if (p.summoning_sick && p.card_types.includes("Creature")) b.push({ t: "z", c: "#8080a0" });
  const c = p.counters || {};
  if (c.PlusOnePlusOne) b.push({ t: `+${c.PlusOnePlusOne}`, c: "#80e080" });
  if (c.MinusOneMinusOne) b.push({ t: `-${c.MinusOneMinusOne}`, c: "#e08080" });
  if (c.Loyalty) b.push({ t: `L${c.Loyalty}`, c: "#e0d080" });
  if (p.damage_marked) b.push({ t: `${p.damage_marked}dmg`, c: "#e05050" });
  if (p.regeneration_shields) b.push({ t: "R", c: "#a0e0a0" });
  if (p.attached_to !== null && p.attached_to !== undefined) b.push({ t: "→" + nameOf(state, p.attached_to).slice(0, 4), c: "#c0a0e0" });
  return b;
}

type Highlight = "option" | "marked" | "selected" | "target" | "verb";

function highlightFor(state: State, key: string): Highlight | null {
  const ui = state.ui;
  if (!ui) return null;
  const isOption = ui.isOption ? ui.isOption(key) : false;
  if (ui.mode === "pick" && isOption) return "option";
  if ((ui.mode === "mark" || ui.mode === "attackers") && isOption) return ui.marked.includes(key) ? "marked" : "option";
  if (ui.mode === "blockers") {
    if (isOption) return ui.assignments!.has(key) ? "marked" : ui.selectedBlocker === key ? "selected" : "option";
    if (ui.attackers!.has(key)) return ui.canAttackerTake!(key) ? "target" : null;
  }
  if (ui.mode === "menu" && ui.verbs && ui.verbs.has(Number(key.slice(1)))) return "verb";
  return null;
}

const HL: Record<Highlight, string> = { option: "#e0c040", marked: "#60e060", selected: "#60c0ff", target: "#ff6060", verb: "#c8b060" };
const HL_DIM: Record<Highlight, string> = { option: "#a08830", marked: "#40a040", selected: "#4080b0", target: "#b04040", verb: "#8a7a50" };
function highlightColor(hl: Highlight): string { return pulse() > 0.5 ? HL[hl] : HL_DIM[hl]; }

function clickFor(state: State, key: string): Hit["onClick"] {
  const ui = state.ui;
  if (!ui) return null;
  const id = Number(key.slice(1));
  const isOption = ui.isOption ? ui.isOption(key) : false;
  switch (ui.mode) {
    case "pick": return isOption ? () => ui.onPick!(key) : null;
    case "mark": return isOption ? () => ui.toggle!(key) : null;
    case "attackers": return isOption ? () => ui.toggle!(key) : null;
    case "blockers":
      if (isOption) return () => ui.clickBlocker!(key);
      if (ui.attackers!.has(key)) return () => ui.clickAttacker!(key);
      return null;
    case "menu":
      if (ui.verbs && ui.verbs.has(id)) return (hit) => { state.popover = { id, items: ui.verbs!.get(id)!, x: hit.x, y: hit.y }; };
      return null;
    default: return null;
  }
}

/** One battlefield card. `group` is the ids stacked under it (lands). */
function drawPerm(ctx: Ctx, hits: Hit[], state: LiveState, p: PermanentView, x: number, y: number, group: ObjectId[] | null): void {
  const key = `o${p.object_id}`;
  const colors = p.colors || [];
  const c = frameColor(colors);
  const hl = highlightFor(state, key);
  const hovered = state.hover !== null && state.hover.key === key;
  ctx.fillStyle = hl ? highlightColor(hl) : hovered ? "#d0c8e0" : darker(c, 0.8);
  ctx.fillRect(x - 1, y - 1, CARD.w + 2, CARD.h + 2);
  ctx.fillStyle = darker(c, 0.45); ctx.fillRect(x, y, CARD.w, CARD.h);
  drawArt(ctx, x + 4, y + 3, ART_S.w, ART_S.h, p.name, colors, p.is_token);
  if (p.is_token) { ctx.fillStyle = "rgba(255,255,255,0.15)"; ctx.fillRect(x, y, CARD.w, 2); }
  // P/T or a type glyph.
  if (p.effective_power !== null && p.effective_power !== undefined) {
    const pt = `${p.effective_power}/${p.effective_toughness}`;
    const boosted = p.effective_power !== p.power || p.effective_toughness !== p.toughness;
    text(ctx, pt, x + CARD.w - 3, y + CARD.h - 10, { align: "right", color: boosted ? "#a0e0ff" : "#f0e8d8", font: "8px PressStart" });
  } else {
    const t = p.card_types.includes("Land") ? "L" : p.card_types.includes("Artifact") ? "A" : p.card_types.includes("Planeswalker") ? "PW" : "E";
    text(ctx, t, x + CARD.w - 3, y + CARD.h - 10, { align: "right", color: "#a0a0b0" });
  }
  // Keyword marks on the left of the bottom row.
  const kw = (p.keywords || []).map(k => KEYWORD_SHORT[k] || k.slice(0, 3)).slice(0, 2).join(" ");
  if (kw) text(ctx, kw, x + 3, y + CARD.h - 10, { color: "#c0c8d8", font: "7px Silkscreen" });
  // Badges strip below the art.
  let bx = x + 3;
  for (const b of permBadges(p, state).slice(0, 3)) {
    ctx.font = "7px Silkscreen"; const w = ctx.measureText(b.t).width + 2;
    ctx.fillStyle = "rgba(0,0,0,0.6)"; ctx.fillRect(bx, y + 29, w, 9);
    text(ctx, b.t, bx + 1, y + 30, { color: b.c, font: "7px Silkscreen" });
    bx += w + 1;
  }
  if (group && group.length > 1) {
    ctx.fillStyle = "#101010"; ctx.fillRect(x + CARD.w - 12, y - 3, 14, 10);
    text(ctx, `x${group.length}`, x + CARD.w - 5, y - 2, { align: "center", font: "7px Silkscreen", color: "#ffe080" });
  }
  if (state.ui && state.ui.badge) {
    const b = state.ui.badge(key);
    if (b) { ctx.fillStyle = "#101010"; ctx.fillRect(x, y + 12, CARD.w, 10); text(ctx, b, x + CARD.w / 2, y + 13, { align: "center", color: "#ffe080", font: "7px Silkscreen" }); }
  }
  if (p.tapped) {
    ctx.fillStyle = "rgba(0,0,0,0.45)"; ctx.fillRect(x, y, CARD.w, CARD.h);
    ctx.fillStyle = "#101010"; ctx.fillRect(x, y, 11, 9);
    text(ctx, "↻", x + 5, y + 1, { align: "center", font: "7px Silkscreen", color: "#e0d0a0" });
  }
  const verbs = state.ui && state.ui.mode === "menu" && state.ui.verbs ? state.ui.verbs.get(p.object_id) : undefined;
  hits.push({ x: x - 1, y: y - 1, w: CARD.w + 2, h: CARD.h + 2, kind: "perm", key, id: p.object_id, group, verbs, onClick: clickFor(state, key) });
}

function drawHandCard(ctx: Ctx, hits: Hit[], state: LiveState, card: CardView, x: number, y: number, raised: boolean): void {
  const key = `o${card.object_id}`;
  const colors = colorsOfCost(card.cost);
  const c = frameColor(colors);
  const hl = highlightFor(state, key);
  const hovered = state.hover !== null && state.hover.key === key;
  if (raised) y -= 10;
  ctx.fillStyle = hl ? highlightColor(hl) : hovered ? "#d0c8e0" : darker(c, 0.85);
  ctx.fillRect(x - 1, y - 1, HAND.w + 2, HAND.h + 2);
  ctx.fillStyle = darker(c, 0.4); ctx.fillRect(x, y, HAND.w, HAND.h);
  drawArt(ctx, x + 1, y + 3, ART_L.w, ART_L.h, card.name, colors, false);
  ctx.fillStyle = "rgba(0,0,0,0.35)"; ctx.fillRect(x + 1, y + 52, HAND.w - 2, HAND.h - 55);
  // Cards overlap to the right, so what matters sits on the left edge.
  const lines = wrap(ctx, card.name, HAND.w - 6, "8px Silkscreen").slice(0, 2);
  lines.forEach((l, i) => text(ctx, clip(ctx, l, HAND.w - 6, "8px Silkscreen"), x + 3, y + 54 + i * 9, { color: "#f4ecdc" }));
  manaDots(ctx, card.cost, x + 3, y + 73);
  const types = card.card_types || [];
  if (card.power !== null && card.power !== undefined) {
    text(ctx, `${card.power}/${card.toughness}`, x + 3, y + 82, { font: "8px PressStart", color: "#f4ecdc" });
  } else {
    const word = types.includes("Land") ? "LAND" : types.includes("Instant") ? "INSTANT" : types.includes("Sorcery") ? "SORCERY"
      : types.includes("Planeswalker") ? "WALKER" : types.includes("Artifact") ? "ARTIFACT" : types.includes("Enchantment") ? "ENCHANT" : types.join("/");
    text(ctx, word, x + 3, y + 83, { color: "#b0b0c0", font: "7px Silkscreen" });
  }
  const verbs = state.ui && state.ui.mode === "menu" && state.ui.verbs ? state.ui.verbs.get(card.object_id) : undefined;
  if (verbs) { ctx.fillStyle = highlightColor("verb"); ctx.fillRect(x + 2, y + 2, 6, 6); ctx.fillStyle = "#101010"; ctx.fillRect(x + 3, y + 3, 4, 4); ctx.fillStyle = highlightColor("verb"); ctx.fillRect(x + 4, y + 4, 2, 2); }
  hits.push({ x: x - 1, y: y - 1, w: HAND.w + 2, h: HAND.h + 2, kind: "hand", key, id: card.object_id, verbs, onClick: clickFor(state, key) });
}

export function colorsOfCost(cost: ManaCost | null | undefined): Color[] {
  const out: Color[] = [];
  for (const s of (cost && cost.symbols) || []) {
    if (typeof s === "object" && "Colored" in s) { const c = s.Colored as Color; if (!out.includes(c)) out.push(c); }
  }
  return out;
}

// ---------------------------------------------------------------- zones

/** Lay `n` items across a row: shrink the stride until they fit. */
function rowLayout(n: number, cardW: number, x0: number, width: number, gap = 3): { stride: number; x0: number } {
  if (n === 0) return { stride: cardW + gap, x0 };
  const stride = Math.max(10, Math.min(cardW + gap, Math.floor((width - cardW) / Math.max(1, n - 1))));
  return { stride, x0 };
}

/** Battlefield permanents of one controller, grouped into the two rows. */
function splitBoard(view: GameView, controller: PlayerId): { creatures: PermanentView[]; groups: PermanentView[][] } {
  const mine = view.battlefield.filter(p => p.controller === controller);
  const creatures = mine.filter(p => p.card_types.includes("Creature"));
  const others = mine.filter(p => !p.card_types.includes("Creature"));
  // Identical untapped basic lands stack; everything else is one card.
  const groups: PermanentView[][] = []; const byKey = new Map<string, PermanentView[]>();
  for (const p of others) {
    const stackable = p.card_types.includes("Land") && (p.supertypes || []).includes("Basic") && p.attached_to === null
      && (p.counters === undefined || Object.keys(p.counters).length === 0);
    const k = stackable ? `${p.name}|${p.tapped}` : `id${p.object_id}`;
    const g = byKey.get(k);
    if (g) g.push(p); else { const ng = [p]; byKey.set(k, ng); groups.push(ng); }
  }
  // Non-lands first, then lands.
  groups.sort((a, b) => Number(a[0].card_types.includes("Land")) - Number(b[0].card_types.includes("Land")));
  return { creatures, groups };
}

function drawRow(ctx: Ctx, hits: Hit[], state: LiveState, items: PermanentView[][], y: number): void {
  const { stride, x0 } = rowLayout(items.length, CARD.w, 6, BOARD_W - 12);
  items.forEach((item, i) => {
    const group = item.length > 1 ? item.map(q => q.object_id) : null;
    // In a pick or mark, a stack whose members are options should offer
    // a member that IS an option, not merely its first card.
    let shown = item[0];
    if (group && state.ui && state.ui.isOption) {
      const opt = item.find(q => state.ui!.isOption!(`o${q.object_id}`));
      if (opt) shown = opt;
    }
    drawPerm(ctx, hits, state, shown, x0 + i * stride, y, group);
  });
}

function drawStrip(ctx: Ctx, hits: Hit[], state: LiveState, y: number, pid: PlayerId, life: number, handSize: number,
  library: number, gy: number, exile: number, pool: ManaPool | undefined, isYou: boolean): void {
  const key = `p${pid}`;
  const hl = highlightFor(state, key);
  ctx.fillStyle = hl ? HL[hl] : "#151218"; ctx.fillRect(0, y, BOARD_W, 14);
  ctx.fillStyle = "#151218"; ctx.fillRect(1, y + 1, BOARD_W - 2, 12);
  const heart = uiImage("life-heart");
  if (heart) ctx.drawImage(heart, 0, 0, heart.width, heart.height, 4, y + 1, 12, 12);
  const active = state.view.active_player === pid;
  const priority = state.view.priority_player === pid;
  text(ctx, String(life), 18, y + 3, { font: "8px PressStart", color: life <= 5 ? "#ff7060" : "#ffffff" });
  text(ctx, `${isYou ? "You" : "Opponent"}${active ? " ★" : ""}${priority ? " ●" : ""}`, 50, y + 3, { color: active ? "#ffe080" : "#c0b8c8" });
  const parts = [`Hand ${handSize}`, `Lib ${library}`, `GY ${gy}`];
  if (exile) parts.push(`Exile ${exile}`);
  const poolText = Object.entries((pool && pool.mana) || {}).filter(([, n]) => (n ?? 0) > 0).map(([k, n]) => `${n}${k[0]}`).join(" ");
  if (poolText) parts.push(`Pool ${poolText}`);
  let x = 150;
  for (const part of parts) {
    ctx.font = "8px Silkscreen"; const w = ctx.measureText(part).width;
    const zone: Hit["zone"] | null = part.startsWith("GY") ? "graveyard" : part.startsWith("Exile") ? "exile" : part.startsWith("Lib") ? "library" : null;
    text(ctx, part, x, y + 3, { color: zone ? "#b0c0e0" : "#c0b8c8" });
    if (zone) hits.push({ x: x - 2, y, w: w + 4, h: 14, kind: "zone", zone, pid, onClick: () => {
      state.overlay = state.overlay && state.overlay.zone === zone && state.overlay.pid === pid ? null : { zone, pid };
    } });
    x += w + 10;
  }
  hits.push({ x: 0, y, w: 140, h: 14, kind: "player", key, pid, onClick: clickFor(state, key) });
}

function drawBand(ctx: Ctx, hits: Hit[], state: LiveState): void {
  const y = ROWS.band; const view = state.view;
  ctx.fillStyle = "#120f16"; ctx.fillRect(0, y, BOARD_W, 30);
  ctx.strokeStyle = "#3a3048"; ctx.beginPath(); ctx.moveTo(0, y + 0.5); ctx.lineTo(BOARD_W, y + 0.5); ctx.moveTo(0, y + 29.5); ctx.lineTo(BOARD_W, y + 29.5); ctx.stroke();
  // Turn and step tracker.
  text(ctx, `T${view.turn_number}`, 4, y + 3, { font: "8px PressStart", color: "#ffe080" });
  STEPS.forEach((s, i) => {
    const sx = 4 + i * 14, sy = y + 14;
    const on = view.step === s;
    ctx.fillStyle = on ? "#c9a84a" : "#2a2430"; ctx.fillRect(sx, sy, 13, 12);
    text(ctx, STEP_SHORT[s], sx + 7, sy + 3, { align: "center", font: "7px Silkscreen", color: on ? "#101010" : "#9a90a0" });
  });
  if (view.first_strike_damage_step) text(ctx, "first strike", 40, y + 3, { color: "#e0c0a0", font: "7px Silkscreen" });
  // The stack: newest on the right.
  const sx0 = 180;
  text(ctx, view.stack.length ? "STACK →" : "stack empty", sx0, y + 2, { color: "#8a8090", font: "7px Silkscreen" });
  // Whose turn, which step, and the last two things that happened.
  const mine = view.active_player === view.you;
  const stepName: Record<Step, string> = { Untap: "untap", Upkeep: "upkeep", Draw: "draw", PrecombatMain: "main phase 1", BeginCombat: "begin combat",
    DeclareAttackers: "declare attackers", DeclareBlockers: "declare blockers", CombatDamage: "combat damage", EndCombat: "end of combat",
    PostcombatMain: "main phase 2", EndStep: "end step", Cleanup: "cleanup" };
  const bx0 = 330;
  text(ctx, `${mine ? "YOUR TURN" : "OPPONENT'S TURN"} · ${stepName[view.step]}`, bx0, y + 2, { font: "7px Silkscreen", color: mine ? "#ffe080" : "#c0b0d0" });
  const recent = view.display_log.slice(-2);
  recent.forEach((l, i) => text(ctx, clip(ctx, l, BOARD_W - bx0 - 4, "7px Silkscreen"), bx0, y + 12 + i * 8, { font: "7px Silkscreen", color: "#8a8898" }));
  view.stack.forEach((item, i) => {
    const key = `o${item.object_id}`;
    const x = sx0 + i * 28, sy = y + 9;
    const hovered = state.hover !== null && state.hover.key === key;
    const hl = highlightFor(state, key);
    ctx.fillStyle = hl ? HL[hl] : hovered ? "#d0c8e0" : item.controller === view.you ? "#6a5a8a" : "#8a5a5a";
    ctx.fillRect(x - 1, sy - 1, 26, 22);
    ctx.fillStyle = "#201828"; ctx.fillRect(x, sy, 24, 20);
    drawArt(ctx, x + 2, sy + 2, 16, 12, item.name, [], false);
    text(ctx, String(i + 1), x + 20, sy + 12, { align: "center", font: "6px Silkscreen", color: "#ffe080" });
    hits.push({ x: x - 1, y: sy - 1, w: 26, h: 22, kind: "stack", key, id: item.object_id, onClick: clickFor(state, key) });
  });
}

/** Lines from each stack item to what it targets. */
function stackLines(ctx: Ctx, hits: Hit[], state: LiveState): void {
  const view = state.view;
  if (!view.stack.length) return;
  const rectOf = (key: string) => hits.find(h => h.key === key && (h.kind === "perm" || h.kind === "stack" || h.kind === "player" || h.kind === "hand"));
  view.stack.forEach((item) => {
    const from = rectOf(`o${item.object_id}`);
    if (!from) return;
    for (const t of item.targets) {
      const key = typeof t === "object" ? ("Object" in t ? `o${t.Object}` : `p${t.Player}`) : null;
      const to = key ? rectOf(key) : null;
      if (!to) continue;
      const hovered = state.hover !== null && state.hover.key === `o${item.object_id}`;
      ctx.strokeStyle = hovered ? "#ffe080" : item.controller === view.you ? "rgba(160,140,220,0.8)" : "rgba(220,120,120,0.8)";
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(Math.round(from.x + from.w / 2) + 0.5, Math.round(from.y + from.h / 2) + 0.5);
      ctx.lineTo(Math.round(to.x + to.w / 2) + 0.5, Math.round(to.y + to.h / 2) + 0.5);
      ctx.stroke();
      ctx.fillStyle = ctx.strokeStyle;
      ctx.fillRect(Math.round(to.x + to.w / 2) - 2, Math.round(to.y + to.h / 2) - 2, 4, 4);
    }
  });
}

/** The end of the game, over the board. */
function gameOverScreen(ctx: Ctx, state: LiveState): void {
  if (!state.gameOver) return;
  ctx.fillStyle = "rgba(0,0,0,0.6)"; ctx.fillRect(0, 0, BOARD_W, H);
  const lines = state.gameOver.split("\n");
  const w = 360, x = (BOARD_W - w) / 2, h = 40 + lines.length * 12, y = (H - h) / 2;
  texturedPanel(ctx, x, y, w, h);
  text(ctx, "GAME OVER", x + w / 2, y + 8, { align: "center", font: "8px PressStart", color: "#ffe080" });
  lines.forEach((l, i) => text(ctx, clip(ctx, l, w - 16, "8px Silkscreen"), x + w / 2, y + 24 + i * 12, { align: "center" }));
}

// --------------------------------------------------------------- panel

function inspector(ctx: Ctx, state: LiveState, x: number, y: number, w: number): number {
  const hoverId = state.hover && state.hover.key ? Number(state.hover.key.slice(1)) : null;
  const e = hoverId !== null ? state.index.get(hoverId) : (state.selected !== null ? state.index.get(state.selected) : undefined);
  if (!e) {
    text(ctx, "Hover a card to read it.", x + 4, y + 4, { color: "#7a7280" });
    return y + 16;
  }
  const o: ViewObject = e.obj;
  const colors = o.colors || colorsOfCost(o.cost);
  drawArt(ctx, x + 4, y + 4, ART_L.w, ART_L.h, o.name, colors, !!o.is_token);
  let ty = y + 4;
  const tx = x + 4 + ART_L.w + 4, tw = w - (ART_L.w + 12);
  for (const l of wrap(ctx, o.name, tw, "8px PressStart").slice(0, 3)) { text(ctx, l, tx, ty, { font: "8px PressStart", color: "#ffffff" }); ty += 10; }
  if (o.cost) { manaDots(ctx, o.cost, tx, ty); ty += 10; }
  const typeLine = [...(o.supertypes || []), ...(o.card_types || [])].join(" ") + ((o.subtypes && o.subtypes.length) ? " — " + o.subtypes.join(" ") : "");
  for (const l of wrap(ctx, typeLine, tw, "7px Silkscreen").slice(0, 2)) { text(ctx, l, tx, ty, { font: "7px Silkscreen", color: "#b0b8c8" }); ty += 8; }
  if (o.effective_power !== undefined && o.effective_power !== null) {
    const printed = o.printed_power !== undefined && o.printed_power !== null ? ` (printed ${o.printed_power}/${o.printed_toughness})` : "";
    text(ctx, `${o.effective_power}/${o.effective_toughness}${o.damage_marked ? ` ${o.damage_marked} dmg` : ""}`, tx, ty, { font: "8px PressStart", color: "#e0f0ff" }); ty += 10;
    if (printed && (o.printed_power !== o.effective_power || o.printed_toughness !== o.effective_toughness)) { text(ctx, printed.trim(), tx, ty, { font: "7px Silkscreen", color: "#8a8090" }); ty += 8; }
  } else if (o.power !== undefined && o.power !== null) {
    text(ctx, `${o.power}/${o.toughness}`, tx, ty, { font: "8px PressStart" }); ty += 10;
  }
  ty = Math.max(ty, y + 4 + ART_L.h + 4);
  const facts: string[] = [];
  if (o.keywords && o.keywords.length) facts.push(o.keywords.join(", "));
  if (o.counters) for (const [k, n] of Object.entries(o.counters)) if (n) facts.push(`${n} ${k} counter${n > 1 ? "s" : ""}`);
  if (o.attached_to !== undefined && o.attached_to !== null) facts.push(`Attached to ${nameOf(state, o.attached_to)}`);
  if (o.attached_to_player !== undefined && o.attached_to_player !== null) facts.push(`Enchants ${playerLabel(state, o.attached_to_player)}`);
  if (o.attacking) facts.push("Attacking " + ("Player" in o.attacking ? playerLabel(state, o.attacking.Player) : nameOf(state, o.attacking.Planeswalker)));
  if (o.blocking && o.blocking.length) facts.push("Blocking " + o.blocking.map(id => nameOf(state, id)).join(", "));
  if (o.blocked_by && o.blocked_by.length) facts.push("Blocked by " + o.blocked_by.map(id => nameOf(state, id)).join(", "));
  for (const p of o.protections || []) facts.push(p);
  for (const r of o.restrictions || []) facts.push(r);
  if (o.summoning_sick) facts.push("Summoning sick");
  if (o.named_card) facts.push(`Named: ${o.named_card}`);
  if (o.targets && o.targets.length) facts.push("Targets: " + o.targets.map(t => targetLabel(state, t)).join(", "));
  if (o.x_value !== undefined && o.x_value !== null) facts.push(`X = ${o.x_value}`);
  if (e.zone !== "battlefield" && e.zone !== "hand") facts.push(`In ${e.zone}`);
  for (const f of facts) for (const l of wrap(ctx, f, w - 8, "7px Silkscreen").slice(0, 2)) { text(ctx, l, x + 4, ty, { font: "7px Silkscreen", color: "#d0c8a0" }); ty += 8; }
  const oracle = [o.oracle_text || "", ...(o.granted_abilities || [])].filter(Boolean).join("\n");
  const maxLines = Math.max(0, Math.floor((y + 200 - ty) / 9));
  for (const l of wrap(ctx, oracle, w - 8, "8px Silkscreen").slice(0, maxLines)) { text(ctx, l, x + 4, ty, { color: "#e8e0d0" }); ty += 9; }
  return ty;
}

function promptArea(ctx: Ctx, hits: Hit[], state: LiveState, x: number, y: number, w: number, h: number): void {
  texturedPanel(ctx, x, y, w, h);
  const ui = state.ui;
  let ty = y + 6;
  x += 2; w -= 4;
  if (state.gameOver) {
    for (const l of wrap(ctx, state.gameOver, w - 8, "8px Silkscreen").slice(0, 8)) { text(ctx, l, x + 4, ty, { color: "#ffe080" }); ty += 9; }
    return;
  }
  if (!state.decision) {
    const dots = ".".repeat(1 + Math.floor(Date.now() / 400) % 3);
    text(ctx, state.connected ? `Opponent is thinking${dots}` : "Connecting…", x + 4, ty, { color: "#b0a8c0" });
    if (state.autoPass) text(ctx, "AUTO-PASS on (f to stop)", x + 4, ty + 10, { font: "7px Silkscreen", color: "#ffe080" });
    return;
  }
  if (state.autoPass) { text(ctx, "AUTO-PASS on (f to stop)", x + 4, ty, { font: "7px Silkscreen", color: "#ffe080" }); ty += 9; }
  const title = ui ? ui.title : "";
  for (const l of wrap(ctx, title, w - 8, "8px Silkscreen").slice(0, 4)) { text(ctx, l, x + 4, ty, { color: "#ffe080" }); ty += 9; }
  if (ui && ui.hint) for (const l of wrap(ctx, ui.hint, w - 8, "7px Silkscreen").slice(0, 3)) { text(ctx, l, x + 4, ty, { font: "7px Silkscreen", color: "#a098b0" }); ty += 8; }
  if (ui && ui.mode === "mark") { text(ctx, `Marked ${ui.marked.length} of ${ui.max}`, x + 4, ty, { color: "#c0e0a0" }); ty += 9; }
  if (ui && ui.mode === "number") {
    for (const l of (ui.summary || []).slice(0, 4)) { text(ctx, clip(ctx, l, w - 8, "7px Silkscreen"), x + 4, ty, { font: "7px Silkscreen", color: "#c0c8d0" }); ty += 8; }
  }
  if (state.notice) for (const l of wrap(ctx, state.notice, w - 8, "7px Silkscreen").slice(0, 3)) { text(ctx, l, x + 4, ty, { font: "7px Silkscreen", color: "#ff9080" }); ty += 8; }
  // Rows the board cannot show, then buttons. A modal has its own rows.
  const modal = ui && (ui.mode === "list" || ui.mode === "order" || ui.mode === "number");
  const rows: Row[] = modal ? [] : ((ui && ui.rows) || (ui && ui.looseRows) || []);
  let by = y + h - 18;
  const buttons = (ui && ui.buttons) || [];
  buttons.slice().reverse().forEach((b) => {
    const label = b.primary && ui && ui.buttonLabel ? ui.buttonLabel() : b.label;
    const enabled = b.enabled ? b.enabled() : true;
    button(ctx, hits, x + 4, by, w - 8, 13, label, b.run, { primary: b.primary, enabled });
    by -= 15;
  });
  let ry = ty + 2;
  for (const r of rows.slice(0, Math.max(0, Math.floor((by - ry) / 11)))) {
    const marked = !!(ui && r.key && ui.marked.includes(r.key));
    panel(ctx, x + 4, ry, w - 8, 10, marked ? "#3a5a3a" : "#2a2430", marked ? "#60e060" : "#4a4a5a");
    text(ctx, clip(ctx, r.label, w - 14, "7px Silkscreen"), x + 7, ry + 1, { font: "7px Silkscreen" });
    hits.push({ x: x + 4, y: ry, w: w - 8, h: 10, kind: "row", onClick: r.run, cardName: r.cardName });
    ry += 11;
  }
}

function logArea(ctx: Ctx, hits: Hit[], state: LiveState, x: number, y: number, w: number, h: number): void {
  panel(ctx, x, y, w, h, "#100e14", "#3a3048");
  const lines: string[] = [];
  for (const entry of state.view.display_log.slice(-40)) for (const l of wrap(ctx, entry, w - 8, "7px Silkscreen")) lines.push(l);
  const fit = Math.floor((h - 4) / 8);
  const shown = lines.slice(Math.max(0, lines.length - fit - state.logScroll), lines.length - state.logScroll);
  shown.forEach((l, i) => text(ctx, l, x + 4, y + 2 + i * 8, { font: "7px Silkscreen", color: "#a8a0b0" }));
  hits.push({ x, y, w, h, kind: "log" });
}

// -------------------------------------------------------------- overlays

function modal(ctx: Ctx, hits: Hit[], state: LiveState): void {
  const ui = state.ui;
  if (!ui || (ui.mode !== "list" && ui.mode !== "order" && ui.mode !== "number")) return;
  if (!ui.keepBoard) { ctx.fillStyle = "rgba(0,0,0,0.55)"; ctx.fillRect(0, 0, BOARD_W, H); }
  const w = 300, x = (BOARD_W - w) / 2;
  type ModalRow = { label: string; run?: () => void; pos?: number; cardName?: string | null };
  let rows: ModalRow[] = ui.rows ?? (ui.order ? ui.order.map((o, pos) => ({ label: o.label, pos })) : []);
  if (ui.mode === "list" && ui.query) { const q = ui.query.toLowerCase(); rows = rows.filter(r => r.label.toLowerCase().includes(q)); }
  const rowH = 12;
  const maxRows = 20;
  const scroll = ui.scroll || 0;
  const shown = rows.slice(scroll, scroll + maxRows);
  const h = 30 + shown.length * rowH + (ui.mode === "number" ? 30 : 0) + (ui.filter ? 14 : 0) + 18;
  const y = Math.max(8, (H - h) / 2);
  // The background hit goes in first, so the rows drawn on it are on top.
  hits.push({ x, y, w, h, kind: "modal" });
  texturedPanel(ctx, x, y, w, h, "#1a1620", "#c9a84a");
  let ty = y + 6;
  for (const l of wrap(ctx, ui.title, w - 12, "8px PressStart").slice(0, 2)) { text(ctx, l, x + 6, ty, { font: "8px PressStart", color: "#ffe080" }); ty += 10; }
  if (ui.filter) { panel(ctx, x + 6, ty, w - 12, 12, "#0e0c12", "#6a5a7a"); text(ctx, ui.query || "type to filter…", x + 9, ty + 2, { color: ui.query ? "#fff" : "#7a7280" }); ty += 14; }
  if (ui.mode === "number") {
    panel(ctx, x + 6, ty, 80, 14, "#0e0c12", "#6a5a7a"); text(ctx, (ui.value ?? "") + "▏", x + 9, ty + 3, { font: "8px PressStart", color: "#fff" });
    text(ctx, `0 – ${ui.max}`, x + 92, ty + 3, { color: "#b0b0c0" }); ty += 18;
    for (const l of (ui.summary || []).slice(0, 4)) { text(ctx, clip(ctx, l, w - 12, "7px Silkscreen"), x + 6, ty, { font: "7px Silkscreen", color: "#c0c8d0" }); ty += 8; }
    ty += 4;
  }
  shown.forEach((r, i) => {
    const ry = ty + i * rowH;
    panel(ctx, x + 6, ry, w - 12, rowH - 1, "#2a2430", "#4a4a5a");
    if (ui.mode === "order") {
      text(ctx, `${scroll + i + 1}.`, x + 9, ry + 2, { color: "#ffe080" });
      text(ctx, clip(ctx, r.label, w - 70, "7px Silkscreen"), x + 26, ry + 2, { font: "7px Silkscreen" });
      button(ctx, hits, x + w - 34, ry + 1, 12, rowH - 3, "▲", () => ui.move!(r.pos!, -1));
      button(ctx, hits, x + w - 20, ry + 1, 12, rowH - 3, "▼", () => ui.move!(r.pos!, 1));
    } else {
      text(ctx, clip(ctx, r.label, w - 18, "8px Silkscreen"), x + 9, ry + 2);
      hits.push({ x: x + 6, y: ry, w: w - 12, h: rowH - 1, kind: "row", onClick: r.run, cardName: r.cardName });
    }
  });
  ty += shown.length * rowH;
  if (rows.length > maxRows) text(ctx, `${scroll + 1}–${Math.min(rows.length, scroll + maxRows)} of ${rows.length} (scroll)`, x + 6, ty + 2, { font: "7px Silkscreen", color: "#8a8090" });
  let bx = x + w - 6;
  for (const b of (ui.buttons || []).slice().reverse()) {
    const bw = 60; bx -= bw + 4;
    button(ctx, hits, bx, y + h - 17, bw, 13, b.label, b.run, { primary: b.primary });
  }
}

function popover(ctx: Ctx, hits: Hit[], state: LiveState): void {
  const p = state.popover;
  if (!p) return;
  const items = p.items;
  ctx.font = "8px Silkscreen";
  const w = Math.min(260, Math.max(80, ...items.map(i => ctx.measureText(i.label).width + 12)));
  const h = items.length * 12 + 6;
  const x = Math.min(BOARD_W - w - 2, p.x), y = Math.min(H - h - 2, Math.max(2, p.y - h - 4 > 0 ? p.y - h - 4 : p.y + CARD.h + 4));
  hits.push({ x, y, w, h, kind: "popover" });
  panel(ctx, x, y, w, h, "#1a1620", "#c9a84a");
  items.forEach((it, i) => {
    const ry = y + 3 + i * 12;
    hits.push({ x: x + 2, y: ry, w: w - 4, h: 11, kind: "row", onClick: () => { state.popover = null; it.run(); } });
    const hov = state.hover !== null && state.hover.kind === "row" && state.hover.y === ry;
    if (hov) { ctx.fillStyle = "#3a3048"; ctx.fillRect(x + 2, ry, w - 4, 11); }
    text(ctx, clip(ctx, it.label, w - 8, "8px Silkscreen"), x + 5, ry + 2, { color: "#f0e8d8" });
  });
}

function zoneOverlay(ctx: Ctx, hits: Hit[], state: LiveState): void {
  const ov = state.overlay;
  if (!ov) return;
  const view = state.view;
  let cards: CardView[] = [];
  let title = "";
  if (ov.zone === "graveyard") { const g = view.graveyards.find(([pid]) => pid === ov.pid); cards = g ? g[1] : []; title = `${playerLabel(state, ov.pid)}: graveyard (${cards.length})`; }
  else if (ov.zone === "exile") { cards = view.exile.filter(c => c.owner === ov.pid); title = `${playerLabel(state, ov.pid)}: exile (${cards.length})`; }
  else if (ov.zone === "library") { cards = ov.pid === view.you ? view.your_library_cards : []; title = ov.pid === view.you ? `Your library (${cards.length}, order unknown)` : "Opponent's library"; }
  hits.push({ x: 0, y: 0, w: BOARD_W, h: H, kind: "overlay", onClick: () => { state.overlay = null; } });
  ctx.fillStyle = "rgba(0,0,0,0.7)"; ctx.fillRect(0, 0, BOARD_W, H);
  text(ctx, title, 8, 6, { font: "8px PressStart", color: "#ffe080" });
  text(ctx, "click outside or press Esc to close", 8, 18, { font: "7px Silkscreen", color: "#8a8090" });
  const cols = 10, cw = 44, ch = 58;
  const perPage = cols * 5;
  const page = ov.page || 0;
  cards.slice(page * perPage, (page + 1) * perPage).forEach((c, i) => {
    const x = 8 + (i % cols) * (cw + 2), y = 32 + Math.floor(i / cols) * (ch + 2);
    const key = `o${c.object_id}`;
    const hl = highlightFor(state, key);
    const hovered = state.hover !== null && state.hover.key === key;
    ctx.fillStyle = hl ? HL[hl] : hovered ? "#d0c8e0" : "#4a4050"; ctx.fillRect(x - 1, y - 1, cw + 2, ch + 2);
    ctx.fillStyle = "#201828"; ctx.fillRect(x, y, cw, ch);
    drawArt(ctx, x + 6, y + 3, ART_S.w, ART_S.h, c.name, colorsOfCost(c.cost), false);
    wrap(ctx, c.name, cw - 4, "6px Silkscreen").slice(0, 3).forEach((l, j) => text(ctx, l, x + 2, y + 30 + j * 7, { font: "6px Silkscreen" }));
    hits.push({ x: x - 1, y: y - 1, w: cw + 2, h: ch + 2, kind: "card", key, id: c.object_id, onClick: clickFor(state, key) });
  });
  if (cards.length > perPage) {
    button(ctx, hits, 8, H - 20, 60, 13, "◀ prev", () => { ov.page = Math.max(0, page - 1); });
    button(ctx, hits, 72, H - 20, 60, 13, "next ▶", () => { ov.page = Math.min(Math.ceil(cards.length / perPage) - 1, page + 1); });
    text(ctx, `page ${page + 1} of ${Math.ceil(cards.length / perPage)}`, 140, H - 17, { font: "7px Silkscreen", color: "#8a8090" });
  }
}

// ----------------------------------------------------------------- frame

export function render(ctx: Ctx, state: State): Hit[] {
  const hits: Hit[] = [];
  ctx.imageSmoothingEnabled = false;
  // Table.
  const table = uiImage("table");
  const pattern = table ? ctx.createPattern(table, "repeat") : null;
  ctx.fillStyle = pattern ?? "#24202a";
  ctx.fillRect(0, 0, W, H);
  ctx.fillStyle = "rgba(0,0,0,0.5)"; ctx.fillRect(0, 0, BOARD_W, H);
  if (!state.view) {
    text(ctx, state.connected ? "Waiting for the game…" : "Connecting to the runner…", 12, 12, { font: "8px PressStart", color: "#ffe080" });
    return hits;
  }
  const live = state as LiveState;
  const view = live.view;
  const opp = view.opponents[0] || { id: 1 - view.you, life: 0, hand_size: 0, library_size: 0, mana_pool: { mana: {} }, mulligan_count: 0 };
  const gyCount = (pid: PlayerId) => { const g = view.graveyards.find(([p]) => p === pid); return g ? g[1].length : 0; };
  const exCount = (pid: PlayerId) => view.exile.filter(c => c.owner === pid).length;

  // Opponent.
  drawStrip(ctx, hits, live, ROWS.oppStrip, opp.id, opp.life, opp.hand_size, opp.library_size, gyCount(opp.id), exCount(opp.id), opp.mana_pool, false);
  const ob = splitBoard(view, opp.id);
  drawRow(ctx, hits, live, ob.groups, ROWS.oppOther);
  drawRow(ctx, hits, live, ob.creatures.map(p => [p]), ROWS.oppCreatures);
  // Middle.
  drawBand(ctx, hits, live);
  // You.
  const mb = splitBoard(view, view.you);
  drawRow(ctx, hits, live, mb.creatures.map(p => [p]), ROWS.myCreatures);
  drawRow(ctx, hits, live, mb.groups, ROWS.myOther);
  // Hand: your strip sits on the hand's top edge.
  const hand = view.your_hand;
  const stripY = ROWS.hand - 14;
  drawStrip(ctx, hits, live, stripY, view.you, view.your_life, hand.length, view.your_library_size, gyCount(view.you), exCount(view.you), view.your_mana_pool, true);
  const { stride, x0 } = rowLayout(hand.length, HAND.w, 4, BOARD_W - 8, 2);
  const hoveredIdx = hand.findIndex(c => state.hover !== null && state.hover.key === `o${c.object_id}`);
  hand.forEach((c, i) => { if (i !== hoveredIdx) drawHandCard(ctx, hits, live, c, x0 + i * stride, ROWS.hand, false); });
  if (hoveredIdx >= 0) drawHandCard(ctx, hits, live, hand[hoveredIdx], x0 + hoveredIdx * stride, ROWS.hand, true);

  // Panel.
  ctx.fillStyle = "#14111a"; ctx.fillRect(PANEL_X, 0, W - PANEL_X, H);
  ctx.strokeStyle = "#3a3048"; ctx.beginPath(); ctx.moveTo(PANEL_X + 0.5, 0); ctx.lineTo(PANEL_X + 0.5, H); ctx.stroke();
  const insEnd = inspector(ctx, live, PANEL_X, 0, W - PANEL_X);
  const promptY = Math.max(insEnd + 4, 150);
  const promptH = state.logOpen ? 100 : H - promptY - 4;
  promptArea(ctx, hits, live, PANEL_X + 2, promptY, W - PANEL_X - 4, promptH);
  if (state.logOpen) logArea(ctx, hits, live, PANEL_X + 2, promptY + promptH + 2, W - PANEL_X - 4, H - promptY - promptH - 4);

  stackLines(ctx, hits, live);
  zoneOverlay(ctx, hits, live);
  popover(ctx, hits, live);
  modal(ctx, hits, live);
  gameOverScreen(ctx, live);
  if (state.logOpen && !state.overlay) {
    // A wide log drawer over the board, for reading rather than glancing.
    ctx.fillStyle = "rgba(10,8,14,0.92)"; ctx.fillRect(0, H - 120, BOARD_W, 120);
    logArea(ctx, hits, live, 0, H - 120, BOARD_W, 120);
    text(ctx, "LOG (l to close, wheel to scroll)", 4, H - 118, { font: "7px Silkscreen", color: "#ffe080" });
  }
  return hits;
}

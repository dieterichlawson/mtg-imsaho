// Drawing the board at 640x360, and saying what was drawn where.
//
// Every call to `render` returns the list of hit rectangles it drew, with
// what each one is and what a click on it does. Input never looks at the
// view; it looks at the last frame.

import { drawArt, frameColor, rgb, darker, uiImage } from "./assets.js";
import { nameOf, costText, targetLabel, playerLabel } from "./prompts.js";

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
const STEPS = ["Untap", "Upkeep", "Draw", "PrecombatMain", "BeginCombat", "DeclareAttackers",
  "DeclareBlockers", "CombatDamage", "EndCombat", "PostcombatMain", "EndStep", "Cleanup"];
const STEP_SHORT = { Untap: "UN", Upkeep: "UP", Draw: "DR", PrecombatMain: "M1", BeginCombat: "BC",
  DeclareAttackers: "DA", DeclareBlockers: "DB", CombatDamage: "CD", EndCombat: "EC", PostcombatMain: "M2",
  EndStep: "ES", Cleanup: "CL" };
const KEYWORD_SHORT = { Flying: "Fly", FirstStrike: "FS", DoubleStrike: "DS", Trample: "Tr", Deathtouch: "DT",
  Lifelink: "LL", Vigilance: "Vig", Flash: "Fl", Reach: "Rch", Haste: "Hst", Defender: "Def", Hexproof: "Hex",
  Intimidate: "Int", Menace: "Men", Indestructible: "Ind" };

// ------------------------------------------------------------ text helpers

export function text(ctx, s, x, y, { font = "8px Silkscreen", color = "#e8e0d0", align = "left" } = {}) {
  ctx.font = font; ctx.fillStyle = color; ctx.textAlign = align; ctx.textBaseline = "top";
  ctx.fillText(s, Math.round(x), Math.round(y));
  ctx.textAlign = "left";
}

export function wrap(ctx, s, maxW, font = "8px Silkscreen") {
  ctx.font = font;
  const lines = [];
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

function clip(ctx, s, maxW, font = "8px Silkscreen") {
  ctx.font = font;
  if (ctx.measureText(s).width <= maxW) return s;
  let t = s;
  while (t.length > 1 && ctx.measureText(t + "…").width > maxW) t = t.slice(0, -1);
  return t + "…";
}

function panel(ctx, x, y, w, h, fill = "#1a1620", stroke = "#5a4a6a") {
  ctx.fillStyle = fill; ctx.fillRect(x, y, w, h);
  ctx.strokeStyle = stroke; ctx.lineWidth = 1; ctx.strokeRect(x + 0.5, y + 0.5, w - 1, h - 1);
}

function button(ctx, hits, x, y, w, h, label, run, { primary = false, enabled = true } = {}) {
  panel(ctx, x, y, w, h, enabled ? (primary ? "#4a3a5a" : "#2a2430") : "#1c1a20", primary ? "#c9a84a" : "#6a5a7a");
  text(ctx, label, x + w / 2, y + (h - 8) / 2, { align: "center", color: enabled ? "#f0e8d8" : "#7a7280", font: "8px Silkscreen" });
  hits.push({ x, y, w, h, kind: "button", label, onClick: enabled ? run : null });
}

// --------------------------------------------------------------- cards

function manaDots(ctx, cost, x, y) {
  if (!cost || !cost.symbols) return x;
  const col = { White: "#f0e6c0", Blue: "#5088e0", Black: "#403050", Red: "#e05040", Green: "#50a050" };
  for (const s of cost.symbols) {
    let fill = "#9a9aa0", label = "";
    if (s === "X") label = "X";
    else if (typeof s === "object" && "Colored" in s) fill = col[s.Colored] || fill;
    else if (typeof s === "object" && "Generic" in s) label = String(s.Generic);
    ctx.fillStyle = fill; ctx.beginPath(); ctx.arc(x + 3.5, y + 3.5, 3.5, 0, Math.PI * 2); ctx.fill();
    if (label) text(ctx, label, x + 3.5, y, { font: "7px Silkscreen", color: "#101010", align: "center" });
    x += 8;
  }
  return x;
}

/** Status marks for a permanent: tapped, attacking, blocking, sick, counters, damage. */
function permBadges(p, state) {
  const b = [];
  if (p.tapped) b.push({ t: "T", c: "#c0c0c0" });
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

function highlightFor(state, key) {
  const ui = state.ui;
  if (!ui) return null;
  if (ui.mode === "pick" && ui.isOption(key)) return "option";
  if ((ui.mode === "mark" || ui.mode === "attackers") && ui.isOption(key)) return ui.marked.includes(key) ? "marked" : "option";
  if (ui.mode === "blockers") {
    if (ui.isOption(key)) return ui.assignments.has(key) ? "marked" : ui.selectedBlocker === key ? "selected" : "option";
    if (ui.attackers.has(key)) return ui.canAttackerTake(key) ? "target" : null;
  }
  if (ui.mode === "menu" && ui.verbs && ui.verbs.has(Number(key.slice(1)))) return "verb";
  return null;
}

const HL = { option: "#e0c040", marked: "#60e060", selected: "#60c0ff", target: "#ff6060", verb: "#a090c0" };

function clickFor(state, key) {
  const ui = state.ui;
  if (!ui) return null;
  const id = Number(key.slice(1));
  switch (ui.mode) {
    case "pick": return ui.isOption(key) ? () => ui.onPick(key) : null;
    case "mark": return ui.isOption(key) ? () => ui.toggle(key) : null;
    case "attackers": return ui.isOption(key) ? () => ui.toggle(key) : null;
    case "blockers":
      if (ui.isOption(key)) return () => ui.clickBlocker(key);
      if (ui.attackers.has(key)) return () => ui.clickAttacker(key);
      return null;
    case "menu":
      if (ui.verbs && ui.verbs.has(id)) return (hit) => { state.popover = { id, items: ui.verbs.get(id), x: hit.x, y: hit.y }; };
      return null;
    default: return null;
  }
}

/** One battlefield card. `group` is the ids stacked under it (lands). */
function drawPerm(ctx, hits, state, p, x, y, group) {
  const key = `o${p.object_id}`;
  const colors = p.colors || [];
  const c = frameColor(colors);
  const hl = highlightFor(state, key);
  const hovered = state.hover && state.hover.key === key;
  if (p.tapped) { ctx.save(); ctx.translate(x + CARD.w / 2, y + CARD.h / 2); ctx.rotate(Math.PI / 20); ctx.translate(-(x + CARD.w / 2), -(y + CARD.h / 2)); }
  ctx.fillStyle = hl ? HL[hl] : hovered ? "#d0c8e0" : darker(c, 0.8);
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
  if (p.tapped) ctx.restore();
  hits.push({ x: x - 1, y: y - 1, w: CARD.w + 2, h: CARD.h + 2, kind: "perm", key, id: p.object_id, group, onClick: clickFor(state, key) });
}

function drawHandCard(ctx, hits, state, card, x, y, raised) {
  const key = `o${card.object_id}`;
  const colors = colorsOfCost(card.cost);
  const c = frameColor(colors);
  const hl = highlightFor(state, key);
  const hovered = state.hover && state.hover.key === key;
  if (raised) y -= 10;
  ctx.fillStyle = hl ? HL[hl] : hovered ? "#d0c8e0" : darker(c, 0.85);
  ctx.fillRect(x - 1, y - 1, HAND.w + 2, HAND.h + 2);
  ctx.fillStyle = darker(c, 0.4); ctx.fillRect(x, y, HAND.w, HAND.h);
  drawArt(ctx, x + 1, y + 3, ART_L.w, ART_L.h, card.name, colors, false);
  ctx.fillStyle = "rgba(0,0,0,0.35)"; ctx.fillRect(x + 1, y + 52, HAND.w - 2, HAND.h - 55);
  const lines = wrap(ctx, card.name, HAND.w - 6, "8px Silkscreen").slice(0, 2);
  lines.forEach((l, i) => text(ctx, l, x + 3, y + 54 + i * 9, { color: "#f4ecdc" }));
  manaDots(ctx, card.cost, x + 3, y + 74);
  if (card.power !== null && card.power !== undefined) {
    text(ctx, `${card.power}/${card.toughness}`, x + HAND.w - 3, y + 83, { align: "right", font: "8px PressStart" });
  } else {
    const types = card.card_types || [];
    const word = types.includes("Land") ? "LAND" : types.includes("Instant") ? "INSTANT" : types.includes("Sorcery") ? "SORCERY"
      : types.includes("Planeswalker") ? "WALKER" : types.includes("Artifact") ? "ARTIFACT" : types.includes("Enchantment") ? "ENCHANT" : types.join("/");
    text(ctx, word, x + 3, y + 84, { color: "#b0b0c0", font: "7px Silkscreen" });
  }
  const hasVerbs = state.ui && state.ui.mode === "menu" && state.ui.verbs && state.ui.verbs.has(card.object_id);
  if (hasVerbs) { ctx.fillStyle = "#e0c040"; ctx.fillRect(x + HAND.w - 8, y + 2, 5, 5); }
  hits.push({ x: x - 1, y: y - 1, w: HAND.w + 2, h: HAND.h + 2, kind: "hand", key, id: card.object_id, onClick: clickFor(state, key) });
}

export function colorsOfCost(cost) {
  const out = [];
  for (const s of (cost && cost.symbols) || []) {
    if (typeof s === "object" && "Colored" in s && !out.includes(s.Colored)) out.push(s.Colored);
  }
  return out;
}

// ---------------------------------------------------------------- zones

/** Lay `items` across a row: shrink the stride until they fit. */
function rowLayout(n, cardW, x0, width, gap = 3) {
  if (n === 0) return { stride: cardW + gap, x0 };
  const stride = Math.max(10, Math.min(cardW + gap, Math.floor((width - cardW) / Math.max(1, n - 1))));
  return { stride, x0 };
}

/** Battlefield permanents of one controller, grouped into the two rows. */
function splitBoard(view, controller) {
  const mine = view.battlefield.filter(p => p.controller === controller);
  const creatures = mine.filter(p => p.card_types.includes("Creature"));
  const others = mine.filter(p => !p.card_types.includes("Creature"));
  // Identical untapped basic lands stack; everything else is one card.
  const groups = []; const byKey = new Map();
  for (const p of others) {
    const stackable = p.card_types.includes("Land") && (p.supertypes || []).includes("Basic") && !p.attached_to && (p.counters === undefined || Object.keys(p.counters).length === 0);
    const k = stackable ? `${p.name}|${p.tapped}` : `id${p.object_id}`;
    if (byKey.has(k)) byKey.get(k).push(p); else { const g = [p]; byKey.set(k, g); groups.push(g); }
  }
  // Non-lands first, then lands.
  groups.sort((a, b) => Number(a[0].card_types.includes("Land")) - Number(b[0].card_types.includes("Land")));
  return { creatures, groups };
}

function drawRow(ctx, hits, state, perms, y, isGroups) {
  const n = perms.length;
  const { stride, x0 } = rowLayout(n, CARD.w, 6, BOARD_W - 12);
  perms.forEach((item, i) => {
    const p = isGroups ? item[0] : item;
    const group = isGroups ? item.map(q => q.object_id) : null;
    // In a pick or mark, a stack whose members are options should offer
    // a member that IS an option, not merely its first card.
    let shown = p;
    if (isGroups && state.ui && state.ui.isOption) {
      const opt = item.find(q => state.ui.isOption(`o${q.object_id}`));
      if (opt) shown = opt;
    }
    drawPerm(ctx, hits, state, shown, x0 + i * stride, y, group);
  });
}

function drawStrip(ctx, hits, state, y, pid, life, handSize, library, gy, exile, pool, isYou) {
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
  const poolText = Object.entries((pool && pool.mana) || {}).filter(([, n]) => n > 0).map(([k, n]) => `${n}${k[0]}`).join(" ");
  if (poolText) parts.push(`Pool ${poolText}`);
  let x = 150;
  for (const part of parts) {
    ctx.font = "8px Silkscreen"; const w = ctx.measureText(part).width;
    const zone = part.startsWith("GY") ? "graveyard" : part.startsWith("Exile") ? "exile" : part.startsWith("Lib") ? "library" : null;
    text(ctx, part, x, y + 3, { color: zone ? "#b0c0e0" : "#c0b8c8" });
    if (zone) hits.push({ x: x - 2, y, w: w + 4, h: 14, kind: "zone", zone, pid, onClick: () => { state.overlay = state.overlay && state.overlay.zone === zone && state.overlay.pid === pid ? null : { zone, pid }; } });
    x += w + 10;
  }
  const click = clickFor(state, key);
  hits.push({ x: 0, y, w: 140, h: 14, kind: "player", key, pid, onClick: click });
}

function drawBand(ctx, hits, state) {
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
  text(ctx, view.stack.length ? "STACK" : "stack empty", sx0, y + 2, { color: "#8a8090", font: "7px Silkscreen" });
  view.stack.forEach((item, i) => {
    const key = `o${item.object_id}`;
    const x = sx0 + i * 28, sy = y + 9;
    const hovered = state.hover && state.hover.key === key;
    const hl = highlightFor(state, key);
    ctx.fillStyle = hl ? HL[hl] : hovered ? "#d0c8e0" : item.controller === view.you ? "#6a5a8a" : "#8a5a5a";
    ctx.fillRect(x - 1, sy - 1, 26, 22);
    ctx.fillStyle = "#201828"; ctx.fillRect(x, sy, 24, 20);
    drawArt(ctx, x + 2, sy + 2, 16, 12, item.name, [], false);
    text(ctx, String(i + 1), x + 20, sy + 12, { align: "center", font: "6px Silkscreen", color: "#ffe080" });
    hits.push({ x: x - 1, y: sy - 1, w: 26, h: 22, kind: "stack", key, id: item.object_id, onClick: clickFor(state, key) });
  });
}

// --------------------------------------------------------------- panel

function inspector(ctx, state, x, y, w) {
  const e = state.hover && state.hover.key ? state.index.get(Number(String(state.hover.key).slice(1))) : (state.selected ? state.index.get(state.selected) : null);
  const view = state.view;
  if (!e) {
    text(ctx, "Hover a card to read it.", x + 4, y + 4, { color: "#7a7280" });
    return y + 16;
  }
  const o = e.obj;
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
  const facts = [];
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

function promptArea(ctx, hits, state, x, y, w, h) {
  panel(ctx, x, y, w, h, "#171320", "#4a3a5a");
  const ui = state.ui;
  let ty = y + 4;
  if (state.gameOver) {
    for (const l of wrap(ctx, state.gameOver, w - 8, "8px PressStart").slice(0, 6)) { text(ctx, l, x + 4, ty, { font: "8px PressStart", color: "#ffe080" }); ty += 10; }
    return;
  }
  if (!state.decision) {
    text(ctx, state.connected ? "Waiting for the opponent…" : "Connecting…", x + 4, ty, { color: "#b0a8c0" });
    return;
  }
  const title = ui ? ui.title : "";
  for (const l of wrap(ctx, title, w - 8, "8px Silkscreen").slice(0, 4)) { text(ctx, l, x + 4, ty, { color: "#ffe080" }); ty += 9; }
  if (ui && ui.hint) for (const l of wrap(ctx, ui.hint, w - 8, "7px Silkscreen").slice(0, 3)) { text(ctx, l, x + 4, ty, { font: "7px Silkscreen", color: "#a098b0" }); ty += 8; }
  if (ui && (ui.mode === "mark")) { text(ctx, `Marked ${ui.marked.length} of ${ui.max}`, x + 4, ty, { color: "#c0e0a0" }); ty += 9; }
  if (ui && ui.mode === "number") {
    for (const l of ui.summary.slice(0, 4)) { text(ctx, clip(ctx, l, w - 8, "7px Silkscreen"), x + 4, ty, { font: "7px Silkscreen", color: "#c0c8d0" }); ty += 8; }
  }
  if (state.notice) for (const l of wrap(ctx, state.notice, w - 8, "7px Silkscreen").slice(0, 3)) { text(ctx, l, x + 4, ty, { font: "7px Silkscreen", color: "#ff9080" }); ty += 8; }
  // Rows the board cannot show, then buttons.
  const rows = ui && (ui.mode === "list" || ui.mode === "order" || ui.mode === "number") ? [] : ((ui && ui.rows) || (ui && ui.looseRows) || []);
  let by = y + h - 16;
  const buttons = (ui && ui.buttons) || [];
  buttons.slice().reverse().forEach((b) => {
    const label = b.primary && ui.buttonLabel ? ui.buttonLabel() : b.label;
    const enabled = b.enabled ? b.enabled() : true;
    button(ctx, hits, x + 4, by, w - 8, 13, label, b.run, { primary: b.primary, enabled });
    by -= 15;
  });
  let ry = ty + 2;
  for (const r of rows.slice(0, Math.max(0, Math.floor((by - ry) / 11)))) {
    const marked = ui.marked && r.key && ui.marked.includes(r.key);
    panel(ctx, x + 4, ry, w - 8, 10, marked ? "#3a5a3a" : "#2a2430", marked ? "#60e060" : "#4a4a5a");
    text(ctx, clip(ctx, r.label, w - 14, "7px Silkscreen"), x + 7, ry + 1, { font: "7px Silkscreen" });
    hits.push({ x: x + 4, y: ry, w: w - 8, h: 10, kind: "row", onClick: r.run, cardName: r.cardName });
    ry += 11;
  }
}

function logArea(ctx, hits, state, x, y, w, h) {
  panel(ctx, x, y, w, h, "#100e14", "#3a3048");
  const lines = [];
  for (const entry of state.view.display_log.slice(-40)) for (const l of wrap(ctx, entry, w - 8, "7px Silkscreen")) lines.push(l);
  const fit = Math.floor((h - 4) / 8);
  const shown = lines.slice(Math.max(0, lines.length - fit - state.logScroll), lines.length - state.logScroll);
  shown.forEach((l, i) => text(ctx, l, x + 4, y + 2 + i * 8, { font: "7px Silkscreen", color: "#a8a0b0" }));
  hits.push({ x, y, w, h, kind: "log" });
}

// -------------------------------------------------------------- overlays

function modal(ctx, hits, state) {
  const ui = state.ui;
  if (!ui || (ui.mode !== "list" && ui.mode !== "order" && ui.mode !== "number")) return;
  if (!ui.keepBoard) { ctx.fillStyle = "rgba(0,0,0,0.55)"; ctx.fillRect(0, 0, BOARD_W, H); }
  const w = 300, x = (BOARD_W - w) / 2;
  let rows = ui.rows || (ui.order ? ui.order.map((o, pos) => ({ label: o.label, pos })) : []);
  if (ui.mode === "list" && ui.query) rows = rows.filter(r => r.label.toLowerCase().includes(ui.query.toLowerCase()));
  const rowH = 12;
  const maxRows = 20;
  const shown = rows.slice(ui.scroll || 0, (ui.scroll || 0) + maxRows);
  const h = 30 + shown.length * rowH + (ui.mode === "number" ? 30 : 0) + (ui.filter ? 14 : 0) + 18;
  const y = Math.max(8, (H - h) / 2);
  hits.push({ x, y, w, h, kind: "modal" });
  panel(ctx, x, y, w, h, "#1a1620", "#c9a84a");
  let ty = y + 6;
  for (const l of wrap(ctx, ui.title, w - 12, "8px PressStart").slice(0, 2)) { text(ctx, l, x + 6, ty, { font: "8px PressStart", color: "#ffe080" }); ty += 10; }
  if (ui.filter) { panel(ctx, x + 6, ty, w - 12, 12, "#0e0c12", "#6a5a7a"); text(ctx, ui.query || "type to filter…", x + 9, ty + 2, { color: ui.query ? "#fff" : "#7a7280" }); ty += 14; }
  if (ui.mode === "number") {
    panel(ctx, x + 6, ty, 80, 14, "#0e0c12", "#6a5a7a"); text(ctx, ui.value + "▏", x + 9, ty + 3, { font: "8px PressStart", color: "#fff" });
    text(ctx, `0 – ${ui.max}`, x + 92, ty + 3, { color: "#b0b0c0" }); ty += 18;
    for (const l of ui.summary.slice(0, 4)) { text(ctx, clip(ctx, l, w - 12, "7px Silkscreen"), x + 6, ty, { font: "7px Silkscreen", color: "#c0c8d0" }); ty += 8; }
    ty += 4;
  }
  shown.forEach((r, i) => {
    const ry = ty + i * rowH;
    panel(ctx, x + 6, ry, w - 12, rowH - 1, "#2a2430", "#4a4a5a");
    if (ui.mode === "order") {
      text(ctx, `${(ui.scroll || 0) + i + 1}.`, x + 9, ry + 2, { color: "#ffe080" });
      text(ctx, clip(ctx, r.label, w - 70, "7px Silkscreen"), x + 26, ry + 2, { font: "7px Silkscreen" });
      button(ctx, hits, x + w - 34, ry + 1, 12, rowH - 3, "▲", () => ui.move(r.pos, -1));
      button(ctx, hits, x + w - 20, ry + 1, 12, rowH - 3, "▼", () => ui.move(r.pos, 1));
    } else {
      text(ctx, clip(ctx, r.label, w - 18, "8px Silkscreen"), x + 9, ry + 2);
      hits.push({ x: x + 6, y: ry, w: w - 12, h: rowH - 1, kind: "row", onClick: r.run, cardName: r.cardName });
    }
  });
  ty += shown.length * rowH;
  if (rows.length > maxRows) text(ctx, `${(ui.scroll || 0) + 1}–${Math.min(rows.length, (ui.scroll || 0) + maxRows)} of ${rows.length} (scroll)`, x + 6, ty + 2, { font: "7px Silkscreen", color: "#8a8090" });
  let bx = x + w - 6;
  for (const b of (ui.buttons || []).slice().reverse()) {
    const bw = 60; bx -= bw + 4;
    button(ctx, hits, bx, y + h - 17, bw, 13, b.label, b.run, { primary: b.primary });
  }
}

function popover(ctx, hits, state) {
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
    const hov = state.hover && state.hover.kind === "row" && state.hover.y === ry;
    if (hov) { ctx.fillStyle = "#3a3048"; ctx.fillRect(x + 2, ry, w - 4, 11); }
    text(ctx, clip(ctx, it.label, w - 8, "8px Silkscreen"), x + 5, ry + 2, { color: "#f0e8d8" });
  });
}

function zoneOverlay(ctx, hits, state) {
  const ov = state.overlay;
  if (!ov) return;
  const view = state.view;
  let cards = [];
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
    const hovered = state.hover && state.hover.key === key;
    ctx.fillStyle = hl ? HL[hl] : hovered ? "#d0c8e0" : "#4a4050"; ctx.fillRect(x - 1, y - 1, cw + 2, ch + 2);
    ctx.fillStyle = "#201828"; ctx.fillRect(x, y, cw, ch);
    drawArt(ctx, x + 6, y + 3, ART_S.w, ART_S.h, c.name, colorsOfCost(c.cost), false);
    for (const [j, l] of wrap(ctx, c.name, cw - 4, "6px Silkscreen").slice(0, 3).entries()) text(ctx, l, x + 2, y + 30 + j * 7, { font: "6px Silkscreen" });
    hits.push({ x: x - 1, y: y - 1, w: cw + 2, h: ch + 2, kind: "card", key, id: c.object_id, onClick: clickFor(state, key) });
  });
  if (cards.length > perPage) {
    button(ctx, hits, 8, H - 20, 60, 13, "◀ prev", () => { ov.page = Math.max(0, page - 1); });
    button(ctx, hits, 72, H - 20, 60, 13, "next ▶", () => { ov.page = Math.min(Math.ceil(cards.length / perPage) - 1, page + 1); });
    text(ctx, `page ${page + 1} of ${Math.ceil(cards.length / perPage)}`, 140, H - 17, { font: "7px Silkscreen", color: "#8a8090" });
  }
}

// ----------------------------------------------------------------- frame

export function render(ctx, state) {
  const hits = [];
  ctx.imageSmoothingEnabled = false;
  // Table.
  const table = uiImage("table");
  if (table) { const pat = ctx.createPattern(table, "repeat"); ctx.fillStyle = pat; } else ctx.fillStyle = "#24202a";
  ctx.fillRect(0, 0, W, H);
  ctx.fillStyle = "rgba(0,0,0,0.25)"; ctx.fillRect(0, 0, BOARD_W, H);
  if (!state.view) {
    text(ctx, state.connected ? "Waiting for the game…" : "Connecting to the runner…", 12, 12, { font: "8px PressStart", color: "#ffe080" });
    return hits;
  }
  const view = state.view;
  const opp = view.opponents[0] || { id: 1 - view.you, life: 0, hand_size: 0, library_size: 0, mana_pool: { mana: {} } };
  const gyCount = (pid) => { const g = view.graveyards.find(([p]) => p === pid); return g ? g[1].length : 0; };
  const exCount = (pid) => view.exile.filter(c => c.owner === pid).length;

  // Opponent.
  drawStrip(ctx, hits, state, ROWS.oppStrip, opp.id, opp.life, opp.hand_size, opp.library_size, gyCount(opp.id), exCount(opp.id), opp.mana_pool, false);
  const ob = splitBoard(view, opp.id);
  drawRow(ctx, hits, state, ob.groups, ROWS.oppOther, true);
  drawRow(ctx, hits, state, ob.creatures, ROWS.oppCreatures, false);
  // Middle.
  drawBand(ctx, hits, state);
  // You.
  const mb = splitBoard(view, view.you);
  drawRow(ctx, hits, state, mb.creatures, ROWS.myCreatures, false);
  drawRow(ctx, hits, state, mb.groups, ROWS.myOther, true);
  // Hand: your strip sits on the hand's left edge.
  const hand = view.your_hand;
  const stripY = ROWS.hand - 14;
  drawStrip(ctx, hits, state, stripY, view.you, view.your_life, hand.length, view.your_library_size, gyCount(view.you), exCount(view.you), view.your_mana_pool, true);
  const { stride, x0 } = rowLayout(hand.length, HAND.w, 4, BOARD_W - 8, 2);
  const hoveredIdx = hand.findIndex(c => state.hover && state.hover.key === `o${c.object_id}`);
  hand.forEach((c, i) => { if (i !== hoveredIdx) drawHandCard(ctx, hits, state, c, x0 + i * stride, ROWS.hand, false); });
  if (hoveredIdx >= 0) drawHandCard(ctx, hits, state, hand[hoveredIdx], x0 + hoveredIdx * stride, ROWS.hand, true);

  // Panel.
  ctx.fillStyle = "#14111a"; ctx.fillRect(PANEL_X, 0, W - PANEL_X, H);
  ctx.strokeStyle = "#3a3048"; ctx.beginPath(); ctx.moveTo(PANEL_X + 0.5, 0); ctx.lineTo(PANEL_X + 0.5, H); ctx.stroke();
  const insEnd = inspector(ctx, state, PANEL_X, 0, W - PANEL_X);
  const promptY = Math.max(insEnd + 4, 150);
  const promptH = state.logOpen ? 100 : H - promptY - 4;
  promptArea(ctx, hits, state, PANEL_X + 2, promptY, W - PANEL_X - 4, promptH);
  if (state.logOpen) logArea(ctx, hits, state, PANEL_X + 2, promptY + promptH + 2, W - PANEL_X - 4, H - promptY - promptH - 4);

  zoneOverlay(ctx, hits, state);
  popover(ctx, hits, state);
  modal(ctx, hits, state);
  if (state.logOpen && !state.overlay) {
    // A wide log drawer over the board, for reading rather than glancing.
    ctx.fillStyle = "rgba(10,8,14,0.92)"; ctx.fillRect(0, H - 120, BOARD_W, 120);
    logArea(ctx, hits, state, 0, H - 120, BOARD_W, 120);
    text(ctx, "LOG (l to close, wheel to scroll)", 4, H - 118, { font: "7px Silkscreen", color: "#ffe080" });
  }
  return hits;
}

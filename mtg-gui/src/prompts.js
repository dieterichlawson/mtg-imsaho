// From a decision message to something the page can click.
//
// The seat sends the engine's `LegalActions` as it is. This file turns it
// into one of a handful of widget shapes (menu, pick, mark, attackers,
// blockers, list, order, number) and builds the `Action` the answer is.
// An unknown prompt kind falls through to a plain list of whatever legal
// actions were offered, never to nothing.

// ---------------------------------------------------------------- lookups

/** Every object the view can name, by id. */
export function indexView(view) {
  const idx = new Map();
  const put = (obj, zone, owner) => idx.set(obj.object_id, { obj, zone, owner });
  for (const p of view.battlefield) put(p, "battlefield", p.controller);
  for (const c of view.your_hand) put(c, "hand", view.you);
  for (const [pid, cards] of view.graveyards) for (const c of cards) put(c, "graveyard", pid);
  for (const c of view.exile) put(c, "exile", c.owner);
  for (const s of view.stack) put(s, "stack", s.controller);
  for (const c of view.your_library_cards) put(c, "library", view.you);
  for (const [id, name] of Object.entries(view.revealed_names || {})) {
    const n = Number(id);
    if (!idx.has(n)) idx.set(n, { obj: { object_id: n, name }, zone: "revealed", owner: null });
  }
  return idx;
}

export function nameOf(state, id) {
  const e = state.index.get(id);
  return e ? e.obj.name : `#${id}`;
}

export function playerLabel(state, pid) {
  return pid === state.view.you ? "You" : "Opponent";
}

export function targetLabel(state, t) {
  if (t === null || t === undefined) return "nothing";
  if (t === "Illegal") return "(illegal)";
  if ("Object" in t) return nameOf(state, t.Object);
  if ("Player" in t) return playerLabel(state, t.Player);
  return JSON.stringify(t);
}

/** Mana cost as text: {1}{R}. */
export function costText(cost) {
  if (!cost || !cost.symbols) return "";
  return cost.symbols.map(s => {
    if (s === "X") return "{X}";
    if (typeof s === "string") return `{${s}}`;
    if ("Colored" in s) return `{${s.Colored[0]}}`;
    if ("Generic" in s) return `{${s.Generic}}`;
    if ("Hybrid" in s) return `{${JSON.stringify(s.Hybrid)}}`;
    return `{${JSON.stringify(s)}}`;
  }).join("");
}

/** What one legal action does, for a row or a popover item. */
export function describeAction(state, a) {
  if (a === "PassPriority") return "Pass priority";
  if (a === "Concede") return "Concede";
  if (a === "MulliganKeep") return "Keep this hand";
  if (a === "MulliganMull") return "Mulligan";
  if (a === "AbandonGame") return "Abandon (harness)";
  const k = Object.keys(a)[0]; const v = a[k];
  switch (k) {
    case "PlayLand": return `Play ${nameOf(state, v.object_id)}`;
    case "CastSpell": {
      const alt = v.alternative_cost ? " (alternative cost)" : "";
      const t = v.targets.length ? ` → ${v.targets.map(x => targetLabel(state, x)).join(", ")}` : "";
      return `Cast ${nameOf(state, v.object_id)}${alt}${t}`;
    }
    case "ActivateManaAbility": {
      const p = state.index.get(v.object_id)?.obj;
      const d = p?.mana_abilities?.find(([i]) => i === v.ability_index)?.[1] || "for mana";
      return `${nameOf(state, v.object_id)}: ${d}`;
    }
    case "ActivateAbility": {
      const t = v.targets.length ? ` → ${v.targets.map(x => targetLabel(state, x)).join(", ")}` : "";
      return `${nameOf(state, v.object_id)}: ability ${v.ability_index}${t}`;
    }
    case "ActivateLoyaltyAbility": {
      const p = state.index.get(v.object_id)?.obj;
      const d = p?.loyalty_abilities?.find(([i]) => i === v.ability_index)?.[1] || `loyalty ${v.ability_index}`;
      const t = v.targets.length ? ` → ${v.targets.map(x => targetLabel(state, x)).join(", ")}` : "";
      return `${nameOf(state, v.object_id)}: ${d}${t}`;
    }
    case "DeclareAttackers": return `Attack with ${v.attackers.length + (v.planeswalker_attacks || []).length}`;
    case "DeclareBlockers": return `Block with ${v.assignments.length}`;
    case "DiscardCards": return `Discard ${v.cards.map(id => nameOf(state, id)).join(", ")}`;
    case "BottomCards": return `Bottom ${v.cards.map(id => nameOf(state, id)).join(", ")}`;
    case "ResolveChoice": return describeChoice(state, v.choice);
    default: return `${k} ${JSON.stringify(v)}`;
  }
}

function describeChoice(state, c) {
  if (c === "CancelCast") return "Cancel the cast";
  const k = Object.keys(c)[0]; const v = c[k];
  switch (k) {
    case "PayDecision": return v ? "Pay" : "Don't pay";
    case "YesNoDecision": return v ? "Yes" : "No";
    case "ChosenTarget": return v === null ? "Decline" : targetLabel(state, v);
    case "ChosenCard": return nameOf(state, v);
    case "ChosenIndex": return v[1];
    case "ChosenOrder": return `Order: ${v.join(", ")}`;
    case "ChosenSubset": return `Pile: ${v.map(id => nameOf(state, id)).join(", ")}`;
    case "ChosenExileSet": return `Exile ${v.map(id => nameOf(state, id)).join(", ")}`;
    case "ChosenTargetSet": return `Targets: ${v.map(x => targetLabel(state, x)).join(", ")}`;
    case "ChosenObjectSet": return `Choose ${v.map(id => nameOf(state, id)).join(", ")}`;
    case "XFunding": return `X funding ${JSON.stringify(v)}`;
    default: return `${k} ${JSON.stringify(v)}`;
  }
}

// ------------------------------------------------------------- the widget

function resolve(choice) { return { ResolveChoice: { choice } }; }

function targetKey(t) {
  if (t && typeof t === "object") {
    if ("Object" in t) return `o${t.Object}`;
    if ("Player" in t) return `p${t.Player}`;
  }
  return null;
}

/** A widget over a set of targets or object ids. */
function withOptions(ui, list) {
  ui.options = new Map(); // key -> original target/id
  for (const t of list) {
    if (typeof t === "number") ui.options.set(`o${t}`, t);
    else { const k = targetKey(t); if (k) ui.options.set(k, t); }
  }
  ui.isOption = (key) => ui.options.has(key);
  return ui;
}

/**
 * Decide how the pending decision is answered. Sets `state.ui`.
 * `send(action)` answers; `state.notice` shows text in the panel.
 */
export function beginDecision(state, send) {
  const d = state.decision;
  const legal = d.legal;
  const actions = legal.actions || [];
  const ui = { mode: "menu", title: legal.context || "", buttons: [], marked: [], hint: "" };
  state.ui = ui;
  state.selected = null;

  // Combat, asked as its own prompt.
  if (d.combat) {
    const kind = Object.keys(d.combat)[0]; const c = d.combat[kind];
    if (kind === "ChooseAttackers") return beginAttackers(state, c, send);
    if (kind === "ChooseBlockers") return beginBlockers(state, c, send);
  }

  // A set of cards out of a list: mulligan bottoming, cleanup discard.
  if (legal.set_prompt) {
    const sp = legal.set_prompt;
    const answer = (cards) => sp.kind === "BottomAfterMulligan" ? { BottomCards: { cards } } : { DiscardCards: { cards } };
    return beginMark(state, ui, {
      title: legal.context || (sp.kind === "BottomAfterMulligan" ? "Put cards on the bottom" : "Discard to hand size"),
      options: sp.options, min: sp.min, max: sp.max,
      onConfirm: (ids) => send(answer(ids)),
    });
  }

  // A mid-resolution prompt.
  if (legal.resolution_prompt) {
    const kind = Object.keys(legal.resolution_prompt)[0];
    const rp = legal.resolution_prompt[kind];
    const desc = rp.description || legal.context || kind;
    switch (kind) {
      case "ChooseXFunding": return beginNumber(state, ui, rp, desc, send);
      case "ChooseExileFromGraveyard":
        return beginMark(state, ui, { title: desc, options: rp.options, min: rp.min, max: rp.max,
          onConfirm: (ids) => send(resolve({ ChosenExileSet: ids })),
          onCancel: () => send(resolve("CancelCast")), cancelLabel: "Cancel cast" });
      case "ChooseObjectSet":
        return beginMark(state, ui, { title: desc, options: rp.options, min: rp.min, max: rp.max,
          onConfirm: (ids) => send(resolve({ ChosenObjectSet: ids })) });
      case "ChooseTargetSet":
        return beginMark(state, ui, { title: desc, options: rp.options, min: rp.min, max: rp.max,
          onConfirm: (keys) => send(resolve({ ChosenTargetSet: keys.map(k => ui.options.get(k)) })),
          onCancel: () => send(resolve("CancelCast")), cancelLabel: "Cancel cast", byKey: true });
      case "DividePermanentsIntoPiles":
        return beginMark(state, ui, { title: desc + " (mark pile 1)", options: rp.permanents, min: 0, max: rp.permanents.length,
          onConfirm: (ids) => send(resolve({ ChosenSubset: ids })) });
      case "ChooseTriggerOrder":
      case "ChooseDamageAssignmentOrder":
        return beginOrder(state, ui, rp, desc, send);
      case "ChooseTarget":
      case "ChooseCardFromHand":
      case "ChooseFromLookedAt":
      case "ChooseFromLibrary":
        return beginPickFromActions(state, ui, actions, desc, send);
      default:
        // PayOrNot, YesNo, ChooseCardType, ChooseDamageEffect, ChoosePile,
        // ChooseCardName, and anything newer: the offered actions as rows.
        return beginList(state, ui, actions, desc, send, kind === "ChooseCardName");
    }
  }

  // Every action is a resolution answer with no prompt record (a library
  // search offered as ChosenCard rows, for one): rows or board picks.
  if (actions.length > 0 && actions.every(a => a.ResolveChoice)) {
    return beginPickFromActions(state, ui, actions, legal.context || "Choose", send);
  }

  // The opening hand.
  if (actions.some(a => a === "MulliganKeep" || a === "MulliganMull")) {
    return beginList(state, ui, actions, legal.context || "Keep or mulligan?", send, false);
  }

  return beginMenu(state, ui, actions, legal, send);
}

// ------------------------------------------------------------------- menu

/** The ordinary priority menu: verbs hang off the cards they belong to. */
function beginMenu(state, ui, actions, legal, send) {
  ui.mode = "menu";
  ui.verbs = new Map(); // object id -> [{label, run}]
  const add = (id, label, run) => {
    if (!ui.verbs.has(id)) ui.verbs.set(id, []);
    ui.verbs.get(id).push({ label, run });
  };
  const hasPass = actions.includes("PassPriority");
  for (const a of actions) {
    if (a === "PassPriority" || a === "Concede") continue;
    if (a.PlayLand) add(a.PlayLand.object_id, "Play land", () => send(a));
    else if (a.ActivateManaAbility) add(a.ActivateManaAbility.object_id, describeAction(state, a).split(": ")[1] || "Tap for mana", () => send(a));
    else if (a.ActivateLoyaltyAbility) add(a.ActivateLoyaltyAbility.object_id, describeAction(state, a).split(": ").slice(1).join(": "), () => send(a));
    else if (a.CastSpell || a.ActivateAbility) { /* collapsed below */ }
    else add(-1, describeAction(state, a), () => send(a));
  }
  for (const cs of legal.castable_spells || []) {
    const verb = cs.is_flashback ? "Flashback" : cs.from_graveyard ? "Cast from graveyard" : cs.alternative_cost ? "Cast (alternative cost)" : "Cast";
    const extra = cs.additional_cost_label ? ` (${cs.additional_cost_label})` : "";
    const forced = forcedTargets(cs.target_spec);
    const named = forced.length ? ` → ${forced.map(t => targetLabel(state, t)).join(", ")}` : "";
    const sac = cs.sacrifice_options.length === 1 ? `, sacrificing ${nameOf(state, cs.sacrifice_options[0])}` : "";
    add(cs.object_id, `${verb}${extra}${named}${sac}`, () => castFlow(state, cs, send));
  }
  for (const ab of legal.activatable_abilities || []) {
    const { targets, sacrifices } = abilitySlots(ab);
    const named = targets.length === 1 && targets[0].length ? ` → ${targets[0].map(t => targetLabel(state, t)).join(", ")}` : "";
    const sac = sacrifices.length === 1 && sacrifices[0] !== null && sacrifices[0] !== ab.object_id
      ? `, sacrificing ${nameOf(state, sacrifices[0])}` : "";
    add(ab.object_id, `${ab.description || ab.name || "Activate"}${named}${sac}`, () => abilityFlow(state, ab, send));
  }
  // Rows that hang off nothing (a stray ResolveChoice) get the panel.
  ui.looseRows = (ui.verbs.get(-1) || []);
  ui.verbs.delete(-1);
  ui.hint = hasPass ? "Click a card for what it can do. Enter passes." : "Choose an action.";
  if (hasPass) ui.buttons.push({ label: "Pass", primary: true, run: () => send("PassPriority") });
  if (actions.includes("Concede")) ui.buttons.push({ label: "Concede", run: () => {
    beginList(state, { mode: "list", title: "Concede the game?", buttons: [], marked: [] },
      [{ label: "Yes, concede", run: () => send("Concede") }, { label: "No", run: () => beginDecision(state, send) }],
      "Concede the game?", send, false, true);
  } });
  ui.canPass = hasPass;
}

function forcedTargets(spec) {
  if (spec && spec.SingleTarget && spec.SingleTarget.length === 1) return spec.SingleTarget;
  return [];
}

function abilitySlots(ab) {
  const targets = []; const sacrifices = [];
  const seenT = new Set(); const seenS = new Set();
  for (const o of ab.option_combos || []) {
    const tk = JSON.stringify(o.targets);
    if (!seenT.has(tk)) { seenT.add(tk); targets.push(o.targets); }
    const sk = JSON.stringify(o.sacrifice);
    if (!seenS.has(sk)) { seenS.add(sk); sacrifices.push(o.sacrifice); }
  }
  return { targets, sacrifices };
}

/** Cast a spell: pick the targets and the sacrifice it still needs. */
function castFlow(state, cs, send) {
  const finish = (targets, sacrifice) => send({ CastSpell: {
    object_id: cs.object_id, targets, sacrifice, exile_count: null, exile_ids: [],
    alternative_cost: cs.alternative_cost, tap_plan: cs.tap_plan,
  } });
  const back = () => beginDecision(state, send);
  const withSacrifice = (targets) => {
    if (cs.sacrifice_options.length === 0) return finish(targets, null);
    if (cs.sacrifice_options.length === 1) return finish(targets, cs.sacrifice_options[0]);
    beginPick(state, { title: `${cs.name}: choose a creature to sacrifice`, options: cs.sacrifice_options,
      onPick: (key, id) => finish(targets, id), onCancel: back });
  };
  const spec = cs.target_spec;
  if (spec === "NoTargets" || spec === "ChosenAtCast") return withSacrifice([]);
  if (spec.SingleTarget) {
    if (spec.SingleTarget.length === 1) return withSacrifice(spec.SingleTarget);
    return beginPick(state, { title: `${cs.name}: choose a target`, options: spec.SingleTarget,
      onPick: (key, t) => withSacrifice([t]), onCancel: back });
  }
  if (spec.TwoTargets) {
    return beginPick(state, { title: `${cs.name}: choose the first target`, options: spec.TwoTargets.first,
      onPick: (key, t) => withSacrifice([t]), onCancel: back });
  }
  withSacrifice([]);
}

function abilityFlow(state, ab, send) {
  const back = () => beginDecision(state, send);
  const finish = (targets, sacrifice) => send({ ActivateAbility: {
    object_id: ab.object_id, ability_index: ab.ability_index, targets, tap_plan: ab.tap_plan,
    sacrifice, x_value: null, source_card_id: ab.source_card_id,
  } });
  const { targets } = abilitySlots(ab);
  const withSacrifice = (chosen) => {
    const sacs = [];
    const seen = new Set();
    for (const o of ab.option_combos || []) {
      if (JSON.stringify(o.targets) !== JSON.stringify(chosen)) continue;
      const k = JSON.stringify(o.sacrifice);
      if (!seen.has(k)) { seen.add(k); sacs.push(o.sacrifice); }
    }
    if (sacs.length <= 1) return finish(chosen, sacs.length ? sacs[0] : null);
    beginPick(state, { title: `${ab.name}: choose a creature to sacrifice`, options: sacs.filter(s => s !== null),
      onPick: (key, id) => finish(chosen, id), onCancel: back });
  };
  if (targets.length === 0) return withSacrifice([]);
  if (targets.length === 1) return withSacrifice(targets[0]);
  const firsts = targets.map(t => t[0]).filter(Boolean);
  beginPick(state, { title: `${ab.name}: choose a target`, options: firsts,
    onPick: (key, t) => withSacrifice(targets.find(x => targetKey(x[0]) === key) || [t]), onCancel: back });
}

// ------------------------------------------------------------ pick / mark

/** Pick one thing on the board. `options` are targets or object ids. */
export function beginPick(state, { title, options, onPick, onCancel, declineLabel, onDecline }) {
  const ui = { mode: "pick", title, buttons: [], marked: [], hint: "Click a highlighted card or player." };
  withOptions(ui, options);
  ui.onPick = (key) => onPick(key, ui.options.get(key));
  if (onDecline) ui.buttons.push({ label: declineLabel || "Decline", run: onDecline });
  if (onCancel) ui.buttons.push({ label: "Cancel", run: onCancel });
  ui.onCancel = onCancel;
  // Options the board cannot show (a looked-at card, a library card)
  // are offered as rows too, so nothing is unreachable.
  ui.rows = [];
  for (const [key, t] of ui.options) {
    const id = key[0] === "o" ? Number(key.slice(1)) : null;
    const e = id === null ? null : state.index.get(id);
    if (key[0] === "p" || (e && ["battlefield", "hand", "graveyard", "exile", "stack"].includes(e.zone))) continue;
    ui.rows.push({ label: id === null ? targetLabel(state, t) : nameOf(state, id), run: () => ui.onPick(key), cardName: e ? e.obj.name : null });
  }
  state.ui = ui;
  state.selected = null;
  return ui;
}

/** Mark between min and max of the options, then confirm. */
function beginMark(state, ui, { title, options, min, max, onConfirm, onCancel, cancelLabel, byKey }) {
  ui.mode = "mark";
  ui.title = title;
  withOptions(ui, options);
  ui.min = min; ui.max = max;
  ui.marked = [];
  ui.hint = min === max ? `Mark ${min}.` : `Mark ${min} to ${max}.`;
  ui.toggle = (key) => {
    const i = ui.marked.indexOf(key);
    if (i >= 0) ui.marked.splice(i, 1);
    else if (ui.marked.length < max) ui.marked.push(key);
  };
  ui.canConfirm = () => ui.marked.length >= min && ui.marked.length <= max;
  ui.onConfirm = () => { if (ui.canConfirm()) onConfirm(byKey ? ui.marked : ui.marked.map(k => ui.options.get(k))); };
  ui.buttons.push({ label: "Confirm", primary: true, run: ui.onConfirm, enabled: ui.canConfirm });
  if (onCancel) { ui.buttons.push({ label: cancelLabel || "Cancel", run: onCancel }); ui.onCancel = onCancel; }
  ui.rows = [];
  for (const [key] of ui.options) {
    const id = key[0] === "o" ? Number(key.slice(1)) : null;
    const e = id === null ? null : state.index.get(id);
    if (key[0] === "p" || (e && ["battlefield", "hand", "graveyard", "exile", "stack"].includes(e.zone))) continue;
    ui.rows.push({ label: nameOf(state, id), run: () => ui.toggle(key), key, cardName: e ? e.obj.name : null });
  }
  state.ui = ui;
  return ui;
}

/** Actions that are each one choice of a card or target: pick on the board. */
function beginPickFromActions(state, ui, actions, title, send) {
  const options = []; const byKey = new Map(); let decline = null; const rows = [];
  for (const a of actions) {
    const c = a.ResolveChoice?.choice;
    if (!c) { rows.push({ label: describeAction(state, a), run: () => send(a) }); continue; }
    if ("ChosenTarget" in c) {
      if (c.ChosenTarget === null) { decline = a; continue; }
      const k = targetKey(c.ChosenTarget); if (k) { options.push(c.ChosenTarget); byKey.set(k, a); }
    } else if ("ChosenCard" in c) {
      options.push(c.ChosenCard); byKey.set(`o${c.ChosenCard}`, a);
    } else {
      rows.push({ label: describeAction(state, a), run: () => send(a) });
    }
  }
  if (options.length === 0) return beginList(state, ui, actions, title, send, false);
  const pick = beginPick(state, { title, options, onPick: (key) => send(byKey.get(key)),
    onDecline: decline ? () => send(decline) : null });
  pick.rows.push(...rows);
  return pick;
}

// ------------------------------------------------------------- list / order

/** A modal list of rows. Rows are actions or {label, run}. */
export function beginList(state, ui, rows, title, send, filter, keepBoard) {
  ui.mode = "list";
  ui.title = title;
  ui.rows = rows.map(r => (r.label && r.run) ? r : { label: describeAction(state, r), run: () => send(r) });
  ui.filter = filter || ui.rows.length > 14;
  ui.query = "";
  ui.scroll = 0;
  ui.hint = ui.filter ? "Type to filter, click a row." : "Click a row.";
  ui.keepBoard = !!keepBoard;
  state.ui = ui;
  return ui;
}

function beginOrder(state, ui, rp, title, send) {
  ui.mode = "order";
  ui.title = title;
  ui.order = rp.options.map((label, i) => ({ label, i }));
  ui.hint = "First listed goes first. Use ▲ ▼ to reorder, then confirm.";
  ui.move = (pos, dir) => {
    const j = pos + dir;
    if (j < 0 || j >= ui.order.length) return;
    [ui.order[pos], ui.order[j]] = [ui.order[j], ui.order[pos]];
  };
  ui.buttons.push({ label: "Confirm", primary: true, run: () => send(resolve({ ChosenOrder: ui.order.map(o => o.i) })) });
  state.ui = ui;
  return ui;
}

/** X: one number, distributed over the pool and the tap groups the way the CLI does. */
function beginNumber(state, ui, rp, title, send) {
  const opts = rp.options;
  const maxX = (opts.max_x || 0) + (opts.x_discount || 0);
  ui.mode = "number";
  ui.title = title;
  ui.max = maxX;
  ui.value = "";
  ui.summary = [];
  const pool = Object.entries(opts.pool || {}).filter(([, n]) => n > 0).map(([k, n]) => `${n} ${k}`).join(", ");
  if (pool) ui.summary.push(`Pool: ${pool}`);
  for (const g of opts.groups || []) ui.summary.push(`${g.name} x${g.source_ids.length} (${g.mana_per_tap}/tap)`);
  ui.hint = `Type X (0-${maxX}) and press Enter.`;
  ui.submit = () => {
    const x = Number(ui.value);
    if (!Number.isInteger(x) || x < 0 || x > maxX) { state.notice = `Enter an integer between 0 and ${maxX}.`; return; }
    let remaining = Math.max(0, x - (opts.x_discount || 0));
    const response = { pool: {}, taps: {} };
    const poolSorted = Object.entries(opts.pool || {}).sort((a, b) => b[1] - a[1]);
    for (const [mt, avail] of poolSorted) {
      if (remaining === 0) break;
      const take = Math.min(avail, remaining);
      if (take > 0) { response.pool[mt] = take; remaining -= take; }
    }
    for (const g of opts.groups || []) {
      if (remaining === 0) break;
      if (!g.mana_per_tap) continue;
      const taps = Math.min(Math.floor(remaining / g.mana_per_tap), g.source_ids.length);
      if (taps > 0) { const amount = taps * g.mana_per_tap; response.taps[g.name] = amount; remaining -= amount; }
    }
    send(resolve({ XFunding: response }));
  };
  ui.buttons.push({ label: "Confirm", primary: true, run: ui.submit });
  ui.buttons.push({ label: "Cancel", run: () => send(resolve({ ChosenTarget: null })) });
  state.ui = ui;
  return ui;
}

// ----------------------------------------------------------------- combat

function beginAttackers(state, c, send) {
  const ui = { mode: "attackers", title: "Declare attackers", buttons: [], marked: [], hint: "" };
  withOptions(ui, c.eligible);
  ui.locked = new Set(c.must_attack.map(id => `o${id}`));
  ui.marked = [...ui.locked];
  ui.defenders = [{ label: "Opponent", player: c.defending_player }, ...(c.defending_planeswalkers || []).map(id => ({ label: nameOf(state, id), planeswalker: id }))];
  ui.attackTarget = new Map(); // key -> defender index
  ui.hint = ui.defenders.length > 1 ? "Click a creature to attack; click again to change whom it attacks, again to withdraw." : "Click creatures to attack with, then confirm.";
  ui.toggle = (key) => {
    if (!ui.marked.includes(key)) { ui.marked.push(key); ui.attackTarget.set(key, 0); return; }
    const cur = ui.attackTarget.get(key) || 0;
    if (cur + 1 < ui.defenders.length) { ui.attackTarget.set(key, cur + 1); return; }
    if (ui.locked.has(key)) { ui.attackTarget.set(key, 0); return; }
    ui.marked.splice(ui.marked.indexOf(key), 1);
    ui.attackTarget.delete(key);
  };
  ui.badge = (key) => {
    if (!ui.marked.includes(key)) return null;
    const d = ui.defenders[ui.attackTarget.get(key) || 0];
    return d.planeswalker !== undefined ? `→${d.label.slice(0, 6)}` : "ATK";
  };
  ui.canConfirm = () => true;
  ui.onConfirm = () => {
    const attackers = []; const planeswalker_attacks = [];
    for (const key of ui.marked) {
      const id = Number(key.slice(1));
      const d = ui.defenders[ui.attackTarget.get(key) || 0];
      if (d.planeswalker !== undefined) planeswalker_attacks.push([id, d.planeswalker]);
      else attackers.push([id, d.player]);
    }
    send({ DeclareAttackers: { attackers, planeswalker_attacks } });
  };
  ui.buttons.push({ label: ui.marked.length ? "Attack" : "No attack", primary: true, run: ui.onConfirm });
  ui.buttonLabel = () => (ui.marked.length ? `Attack with ${ui.marked.length}` : "No attackers");
  state.ui = ui;
  state.selected = null;
  return ui;
}

function beginBlockers(state, c, send) {
  const ui = { mode: "blockers", title: "Declare blockers", buttons: [], marked: [], hint: "Click a blocker, then the attacker it blocks. Click a blocker again to unassign." };
  withOptions(ui, c.eligible_blockers);
  ui.attackers = new Set(c.attackers.map(id => `o${id}`));
  ui.legal = new Map(Object.entries(c.legal_blocks || {}).map(([k, v]) => [`o${k}`, new Set(v.map(id => `o${id}`))]));
  ui.minBlockers = new Map(Object.entries(c.min_blockers || {}).map(([k, v]) => [`o${k}`, v]));
  ui.assignments = new Map(); // blocker key -> attacker key
  ui.selectedBlocker = null;
  ui.clickBlocker = (key) => {
    if (ui.assignments.has(key)) { ui.assignments.delete(key); ui.selectedBlocker = null; return; }
    ui.selectedBlocker = ui.selectedBlocker === key ? null : key;
  };
  ui.clickAttacker = (key) => {
    if (!ui.selectedBlocker) return;
    const allowed = ui.legal.get(ui.selectedBlocker);
    if (allowed && !allowed.has(key)) { state.notice = `${nameOf(state, Number(ui.selectedBlocker.slice(1)))} cannot block that.`; return; }
    ui.assignments.set(ui.selectedBlocker, key);
    ui.selectedBlocker = null;
  };
  ui.canAttackerTake = (key) => ui.selectedBlocker && (!ui.legal.get(ui.selectedBlocker) || ui.legal.get(ui.selectedBlocker).has(key));
  ui.badge = (key) => {
    const a = ui.assignments.get(key);
    if (a) return `⛨${nameOf(state, Number(a.slice(1))).slice(0, 5)}`;
    return ui.selectedBlocker === key ? "?" : null;
  };
  ui.blockersOn = (attackerKey) => [...ui.assignments].filter(([, a]) => a === attackerKey).length;
  ui.canConfirm = () => {
    for (const [ak, min] of ui.minBlockers) { const n = ui.blockersOn(ak); if (n > 0 && n < min) return false; }
    return true;
  };
  ui.onConfirm = () => {
    if (!ui.canConfirm()) { state.notice = "An attacker with menace needs two or more blockers, or none."; return; }
    send({ DeclareBlockers: { assignments: [...ui.assignments].map(([b, a]) => [Number(b.slice(1)), Number(a.slice(1))]) } });
  };
  ui.buttons.push({ label: "Confirm blocks", primary: true, run: ui.onConfirm, enabled: ui.canConfirm });
  ui.buttonLabel = () => (ui.assignments.size ? `Block with ${ui.assignments.size}` : "No blocks");
  state.ui = ui;
  state.selected = null;
  return ui;
}

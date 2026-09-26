// From a decision message to something the page can click.
//
// The seat sends the engine's `LegalActions` as it is. This file turns it
// into one of a handful of widget shapes (menu, pick, mark, attackers,
// blockers, list, order, number) and builds the `Action` the answer is.
// An unknown prompt kind falls through to a plain list of whatever legal
// actions were offered, never to nothing.

import type {
  Action, ActivatableAbility, CastableSpell, CombatPrompt, FundingGroup, FundingOptions, FundingResponse, GameView,
  LegalActions, ManaCost, ManaType, ObjectId, PlayerId, ResolutionPayload, ResolvedChoice, Target,
} from "./protocol.js";
import { tag } from "./protocol.js";
import type { IndexEntry, LiveState, Row, State, Ui } from "./state.js";

export type Send = (action: Action) => void;

// ---------------------------------------------------------------- lookups

/** Every object the view can name, by id. */
export function indexView(view: GameView): Map<ObjectId, IndexEntry> {
  const idx = new Map<ObjectId, IndexEntry>();
  const put = (obj: IndexEntry["obj"], zone: IndexEntry["zone"], owner: PlayerId | null) => idx.set(obj.object_id, { obj, zone, owner });
  for (const p of view.battlefield) put(p, "battlefield", p.controller);
  for (const c of view.your_hand) put(c, "hand", view.you);
  for (const [pid, cards] of view.graveyards) for (const c of cards) put(c, "graveyard", pid);
  for (const c of view.exile) put(c, "exile", c.owner);
  // A stack item is not always an object of its own. `view.rs` gives an
  // activated ability its SOURCE permanent's id and gives every trigger
  // `ObjectId(0)`, and the stack is filled last — so an ability on the
  // stack used to replace its own source here. Hovering the Ghoulcaller's
  // Bell on the battlefield showed "Ghoulcaller's Bell ability / IN STACK"
  // and none of the permanent, `nameOf` renamed it everywhere for the
  // duration, and two triggers at once both resolved to whichever was
  // indexed last (issue #527).
  //
  // A spell on the stack IS its own object and still belongs here. The
  // others are read off the slot they were drawn for, which is the only
  // thing that tells two triggers apart.
  for (const s of view.stack) if (s.object_id !== 0 && !idx.has(s.object_id)) put(s, "stack", s.controller);
  for (const c of view.your_library_cards) put(c, "library", view.you);
  for (const [id, name] of Object.entries(view.revealed_names || {})) {
    const n = Number(id);
    if (!idx.has(n)) idx.set(n, { obj: { object_id: n, name }, zone: "revealed", owner: null });
  }
  return idx;
}

export function nameOf(state: State, id: ObjectId): string {
  const e = state.index.get(id);
  return e ? e.obj.name : `#${id}`;
}

export function playerLabel(state: LiveState, pid: PlayerId): string {
  return pid === state.view.you ? "You" : "Opponent";
}

/**
 * The engine's own words for a player, in the page's vocabulary.
 *
 * Everything the page writes itself says You or Opponent; two strings it
 * is handed do not. `GameView::display_log` is the engine's log verbatim
 * ("p1 cast Doom Blade (#75)"), and the game-over summary is built by the
 * runner ("Game over! p0 (red-green) wins!"). Nothing on the page ever
 * mapped `p0` onto You, and the page's viewer — unlike the CLI's — never
 * sees the runner's header line, so the one screen whose whole job is to
 * say who won was written in a vocabulary the page never defined
 * (issue #519).
 *
 * The CLI answered this by printing "you are p0" in its status bar (#115);
 * the LLM seat answered it by rewriting the tokens out before the model
 * reads them (#465). The page does both: the life strip names the seat, so
 * the engine's own words stay decodable, and the lines the page quotes are
 * rewritten so they do not have to be decoded.
 */
export function inOurWords(state: LiveState, line: string): string {
  return line.replace(/\bp(\d+)\b/g, (whole, n) => {
    const pid = Number(n);
    if (pid === state.view.you) return "you";
    return state.view.opponents.some(o => o.id === pid) ? "opp" : whole;
  });
}

export function targetLabel(state: LiveState, t: Target | null | undefined): string {
  if (t === null || t === undefined) return "nothing";
  if (t === "Illegal") return "(illegal)";
  if ("Object" in t) return nameOf(state, t.Object);
  if ("Player" in t) return playerLabel(state, t.Player);
  return JSON.stringify(t);
}

/** Mana cost as text: {1}{R}. */
export function costText(cost: ManaCost | null | undefined): string {
  if (!cost || !cost.symbols) return "";
  return cost.symbols.map(s => {
    if (s === "X") return "{X}";
    if (typeof s === "string") return `{${s}}`;
    if ("Colored" in s) return `{${String(s.Colored)[0]}}`;
    if ("Generic" in s) return `{${String(s.Generic)}}`;
    return `{${JSON.stringify(s)}}`;
  }).join("");
}

function isObj(a: Action): a is Exclude<Action, string> { return typeof a === "object"; }

/** What one legal action does, for a row or a popover item. */
export function describeAction(state: LiveState, a: Action): string {
  if (a === "PassPriority") return "Pass priority";
  if (a === "Concede") return "Concede";
  if (a === "MulliganKeep") return "Keep this hand";
  if (a === "MulliganMull") return "Mulligan";
  if (a === "AbandonGame") return "Abandon (harness)";
  if ("PlayLand" in a) return `Play ${nameOf(state, a.PlayLand.object_id)}`;
  if ("CastSpell" in a) {
    const v = a.CastSpell;
    const alt = v.alternative_cost ? " (alternative cost)" : "";
    const t = v.targets.length ? ` → ${v.targets.map(x => targetLabel(state, x)).join(", ")}` : "";
    return `Cast ${nameOf(state, v.object_id)}${alt}${t}`;
  }
  if ("ActivateManaAbility" in a) {
    const v = a.ActivateManaAbility;
    const p = state.index.get(v.object_id)?.obj;
    const d = p?.mana_abilities?.find(([i]) => i === v.ability_index)?.[1] || "for mana";
    return `${nameOf(state, v.object_id)}: ${d}`;
  }
  if ("ActivateAbility" in a) {
    const v = a.ActivateAbility;
    const t = v.targets.length ? ` → ${v.targets.map(x => targetLabel(state, x)).join(", ")}` : "";
    return `${nameOf(state, v.object_id)}: ability ${v.ability_index}${t}`;
  }
  if ("ActivateLoyaltyAbility" in a) {
    const v = a.ActivateLoyaltyAbility;
    const p = state.index.get(v.object_id)?.obj;
    const d = p?.loyalty_abilities?.find(([i]) => i === v.ability_index)?.[1] || `loyalty ${v.ability_index}`;
    const t = v.targets.length ? ` → ${v.targets.map(x => targetLabel(state, x)).join(", ")}` : "";
    return `${nameOf(state, v.object_id)}: ${d}${t}`;
  }
  if ("DeclareAttackers" in a) return `Attack with ${a.DeclareAttackers.attackers.length + (a.DeclareAttackers.planeswalker_attacks || []).length}`;
  if ("DeclareBlockers" in a) return `Block with ${a.DeclareBlockers.assignments.length}`;
  if ("DiscardCards" in a) return `Discard ${a.DiscardCards.cards.map(id => nameOf(state, id)).join(", ")}`;
  if ("BottomCards" in a) return `Bottom ${a.BottomCards.cards.map(id => nameOf(state, id)).join(", ")}`;
  if ("ResolveChoice" in a) return describeChoice(state, a.ResolveChoice.choice);
  return JSON.stringify(a);
}

function describeChoice(state: LiveState, c: ResolvedChoice): string {
  if (c === "CancelCast") return "Cancel the cast";
  if ("PayDecision" in c) return c.PayDecision ? "Pay" : "Don't pay";
  if ("YesNoDecision" in c) return c.YesNoDecision ? "Yes" : "No";
  if ("ChosenTarget" in c) return c.ChosenTarget === null ? "Decline" : targetLabel(state, c.ChosenTarget);
  if ("ChosenCard" in c) return nameOf(state, c.ChosenCard);
  if ("ChosenIndex" in c) return c.ChosenIndex[1];
  if ("ChosenOrder" in c) return `Order: ${c.ChosenOrder.join(", ")}`;
  if ("ChosenSubset" in c) return `Pile: ${c.ChosenSubset.map(id => nameOf(state, id)).join(", ")}`;
  if ("ChosenExileSet" in c) return `Exile ${c.ChosenExileSet.map(id => nameOf(state, id)).join(", ")}`;
  if ("ChosenTargetSet" in c) return `Targets: ${c.ChosenTargetSet.map(x => targetLabel(state, x)).join(", ")}`;
  if ("ChosenObjectSet" in c) return `Choose ${c.ChosenObjectSet.map(id => nameOf(state, id)).join(", ")}`;
  if ("XFunding" in c) return `X funding ${JSON.stringify(c.XFunding)}`;
  return JSON.stringify(c);
}

// ------------------------------------------------------------- the widget

function resolve(choice: ResolvedChoice): Action { return { ResolveChoice: { choice } }; }

export function targetKey(t: Target | null | undefined): string | null {
  if (t && typeof t === "object") {
    if ("Object" in t) return `o${t.Object}`;
    if ("Player" in t) return `p${t.Player}`;
  }
  return null;
}

function newUi(mode: Ui["mode"], title: string): Ui {
  return { mode, title, hint: "", buttons: [], marked: [] };
}

/** A widget over a set of targets or object ids. */
function withOptions(ui: Ui, list: readonly (Target | ObjectId)[]): Ui {
  const options = new Map<string, Target | ObjectId>();
  for (const t of list) {
    if (typeof t === "number") options.set(`o${t}`, t);
    else { const k = targetKey(t); if (k) options.set(k, t); }
  }
  ui.options = options;
  ui.isOption = (key) => options.has(key);
  return ui;
}

/** Zones drawn on the board itself; everything else is reached through rows. */
const ON_BOARD = new Set(["battlefield", "hand", "stack"]);

/**
 * Rows for the options the board does not draw: a graveyard or exile card,
 * a looked-at card, a library card. A graveyard or exile that holds an
 * option is also opened, so the card can be clicked where it lives.
 */
function offBoardRows(state: LiveState, ui: Ui, run: (key: string) => void): Row[] {
  const rows: Row[] = [];
  let openZone: { zone: "graveyard" | "exile"; pid: PlayerId } | null = null;
  for (const [key, t] of ui.options ?? []) {
    const id = key[0] === "o" ? Number(key.slice(1)) : null;
    const e = id === null ? null : state.index.get(id);
    if (key[0] === "p" || (e && ON_BOARD.has(e.zone))) continue;
    if (e && (e.zone === "graveyard" || e.zone === "exile") && e.owner !== null && !openZone) openZone = { zone: e.zone, pid: e.owner };
    const label = id === null ? targetLabel(state, t as Target) : nameOf(state, id);
    rows.push({ label, run: () => run(key), key, cardName: e ? e.obj.name : null });
  }
  if (openZone) state.overlay = openZone;
  return rows;
}

/**
 * Decide how the pending decision is answered. Sets `state.ui`.
 * `send(action)` answers; `state.notice` shows text in the panel.
 */
export function beginDecision(state: LiveState, send: Send): Ui {
  const d = state.decision;
  if (!d) throw new Error("no decision to begin");
  const legal = d.legal;
  const actions = legal.actions || [];
  const ui = newUi("menu", legal.context || "");
  state.ui = ui;
  state.selected = null;

  // Combat, asked as its own prompt.
  if (d.combat) {
    if ("ChooseAttackers" in d.combat) return beginAttackers(state, d.combat.ChooseAttackers, send);
    if ("ChooseBlockers" in d.combat) return beginBlockers(state, d.combat.ChooseBlockers, send);
  }

  // A set of cards out of a list: mulligan bottoming, cleanup discard.
  if (legal.set_prompt) {
    const sp = legal.set_prompt;
    const answer = (cards: ObjectId[]): Action => sp.kind === "BottomAfterMulligan" ? { BottomCards: { cards } } : { DiscardCards: { cards } };
    return beginMark(state, ui, {
      title: legal.context || (sp.kind === "BottomAfterMulligan" ? "Put cards on the bottom" : "Discard to hand size"),
      options: sp.options, min: sp.min, max: sp.max,
      onConfirm: (chosen) => send(answer(chosen as ObjectId[])),
    });
  }

  // A mid-resolution prompt.
  if (legal.resolution_prompt) {
    const kind = tag(legal.resolution_prompt);
    const rp = legal.resolution_prompt[kind];
    const desc = rp.description || legal.context || kind;
    const ids = (rp.options ?? []) as ObjectId[];
    switch (kind) {
      case "ChooseXFunding": return beginNumber(state, ui, rp.options as unknown as FundingOptions, desc, send);
      case "ChooseExileFromGraveyard":
        return beginMark(state, ui, { title: desc, options: ids, min: rp.min ?? 0, max: rp.max ?? ids.length,
          onConfirm: (chosen) => send(resolve({ ChosenExileSet: chosen as ObjectId[] })),
          onCancel: () => send(resolve("CancelCast")), cancelLabel: "Cancel cast" });
      case "ChooseObjectSet":
        return beginMark(state, ui, { title: desc, options: ids, min: rp.min ?? 0, max: rp.max ?? ids.length,
          onConfirm: (chosen) => send(resolve({ ChosenObjectSet: chosen as ObjectId[] })) });
      case "ChooseTargetSet": {
        const targets = (rp.options ?? []) as Target[];
        return beginMark(state, ui, { title: desc, options: targets, min: rp.min ?? 0, max: rp.max ?? targets.length,
          onConfirm: (chosen) => send(resolve({ ChosenTargetSet: chosen as Target[] })),
          onCancel: () => send(resolve("CancelCast")), cancelLabel: "Cancel cast" });
      }
      case "DividePermanentsIntoPiles": {
        const perms = rp.permanents ?? [];
        return beginMark(state, ui, { title: desc + " (mark pile 1)", options: perms, min: 0, max: perms.length,
          onConfirm: (chosen) => send(resolve({ ChosenSubset: chosen as ObjectId[] })) });
      }
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
  if (actions.length > 0 && actions.every(a => isObj(a) && "ResolveChoice" in a)) {
    return beginPickFromActions(state, ui, actions, legal.context || "Choose", send);
  }

  // The opening hand: the list sits beside the hand it is about.
  if (actions.some(a => a === "MulliganKeep" || a === "MulliganMull")) {
    return beginList(state, ui, actions, legal.context || "Keep or mulligan?", send, false, true);
  }

  return beginMenu(state, ui, actions, legal, send);
}

// ------------------------------------------------------------------- menu

/** The ordinary priority menu: verbs hang off the cards they belong to. */
function beginMenu(state: LiveState, ui: Ui, actions: Action[], legal: LegalActions, send: Send): Ui {
  ui.mode = "menu";
  const verbs = new Map<ObjectId, { label: string; run: () => void }[]>();
  ui.verbs = verbs;
  const add = (id: ObjectId, label: string, run: () => void) => {
    if (!verbs.has(id)) verbs.set(id, []);
    verbs.get(id)!.push({ label, run });
  };
  const hasPass = actions.includes("PassPriority");
  for (const a of actions) {
    if (!isObj(a)) continue;
    if ("PlayLand" in a) add(a.PlayLand.object_id, "Play land", () => send(a));
    else if ("ActivateManaAbility" in a) add(a.ActivateManaAbility.object_id, describeAction(state, a).split(": ")[1] || "Tap for mana", () => send(a));
    else if ("ActivateLoyaltyAbility" in a) add(a.ActivateLoyaltyAbility.object_id, describeAction(state, a).split(": ").slice(1).join(": "), () => send(a));
    else if ("CastSpell" in a || "ActivateAbility" in a) { /* collapsed below */ }
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
  ui.looseRows = verbs.get(-1) || [];
  verbs.delete(-1);
  // The engine offers one PlayLand per land *name* — with three Forests in
  // hand `legal.actions` names one object id — and the CLI's menu and the
  // LLM's schema want exactly that. This page hangs verbs off cards, so the
  // other two Forests drew with no marker and answered no click, which to
  // a person reads as "one of your Forests is playable and the others are
  // not" (issue #572, G13). Two cards of one name in one hand are the same
  // card to the rules, so a copy with nothing of its own borrows what its
  // namesake was offered: any Forest plays a Forest.
  const hand = state.view.your_hand || [];
  for (const card of hand) {
    if (verbs.has(card.object_id)) continue;
    const twin = hand.find(c => c.object_id !== card.object_id && c.name === card.name && verbs.has(c.object_id));
    if (twin) verbs.set(card.object_id, verbs.get(twin.object_id)!);
  }
  ui.hint = hasPass ? "Click a card for what it can do. Enter passes." : "Choose an action.";
  if (hasPass) ui.buttons.push({ label: "Pass", primary: true, run: () => send("PassPriority") });
  if (actions.includes("Concede")) ui.buttons.push({ label: "Concede", run: () => {
    beginList(state, newUi("list", "Concede the game?"),
      [{ label: "Yes, concede", run: () => send("Concede") }, { label: "No", run: () => beginDecision(state, send) }],
      "Concede the game?", send, false, true);
  } });
  ui.canPass = hasPass;
  return ui;
}

function forcedTargets(spec: CastableSpell["target_spec"]): Target[] {
  if (typeof spec === "object" && "SingleTarget" in spec && spec.SingleTarget.length === 1) return spec.SingleTarget;
  return [];
}

function abilitySlots(ab: ActivatableAbility): { targets: Target[][]; sacrifices: (ObjectId | null)[] } {
  const targets: Target[][] = []; const sacrifices: (ObjectId | null)[] = [];
  const seenT = new Set<string>(); const seenS = new Set<string>();
  for (const o of ab.option_combos || []) {
    const tk = JSON.stringify(o.targets);
    if (!seenT.has(tk)) { seenT.add(tk); targets.push(o.targets); }
    const sk = JSON.stringify(o.sacrifice);
    if (!seenS.has(sk)) { seenS.add(sk); sacrifices.push(o.sacrifice); }
  }
  return { targets, sacrifices };
}

/** Cast a spell: pick the targets and the sacrifice it still needs. */
function castFlow(state: LiveState, cs: CastableSpell, send: Send): void {
  const finish = (targets: Target[], sacrifice: ObjectId | null) => send({ CastSpell: {
    object_id: cs.object_id, targets, sacrifice, exile_count: null, exile_ids: [],
    alternative_cost: cs.alternative_cost, tap_plan: cs.tap_plan,
  } });
  const back = () => { beginDecision(state, send); };
  const withSacrifice = (targets: Target[]) => {
    if (cs.sacrifice_options.length === 0) return finish(targets, null);
    if (cs.sacrifice_options.length === 1) return finish(targets, cs.sacrifice_options[0]);
    beginPick(state, { title: `${cs.name}: choose a creature to sacrifice`, options: cs.sacrifice_options,
      onPick: (_key, id) => finish(targets, id as ObjectId), onCancel: back });
  };
  const spec = cs.target_spec;
  if (spec === "NoTargets" || spec === "ChosenAtCast") return withSacrifice([]);
  if ("SingleTarget" in spec) {
    if (spec.SingleTarget.length === 1) return withSacrifice(spec.SingleTarget);
    beginPick(state, { title: `${cs.name}: choose a target`, options: spec.SingleTarget,
      onPick: (_key, t) => withSacrifice([t as Target]), onCancel: back });
    return;
  }
  if ("TwoTargets" in spec) {
    beginPick(state, { title: `${cs.name}: choose the first target`, options: spec.TwoTargets.first,
      onPick: (_key, t) => withSacrifice([t as Target]), onCancel: back });
    return;
  }
  withSacrifice([]);
}

function abilityFlow(state: LiveState, ab: ActivatableAbility, send: Send): void {
  const back = () => { beginDecision(state, send); };
  const finish = (targets: Target[], sacrifice: ObjectId | null) => send({ ActivateAbility: {
    object_id: ab.object_id, ability_index: ab.ability_index, targets, tap_plan: ab.tap_plan,
    sacrifice, x_value: null, source_card_id: ab.source_card_id,
  } });
  const { targets } = abilitySlots(ab);
  const withSacrifice = (chosen: Target[]) => {
    const sacs: (ObjectId | null)[] = [];
    const seen = new Set<string>();
    for (const o of ab.option_combos || []) {
      if (JSON.stringify(o.targets) !== JSON.stringify(chosen)) continue;
      const k = JSON.stringify(o.sacrifice);
      if (!seen.has(k)) { seen.add(k); sacs.push(o.sacrifice); }
    }
    if (sacs.length <= 1) return finish(chosen, sacs.length ? sacs[0] : null);
    beginPick(state, { title: `${ab.name}: choose a creature to sacrifice`, options: sacs.filter((s): s is ObjectId => s !== null),
      onPick: (_key, id) => finish(chosen, id as ObjectId), onCancel: back });
  };
  if (targets.length === 0) return withSacrifice([]);
  if (targets.length === 1) return withSacrifice(targets[0]);
  const firsts = targets.map(t => t[0]).filter(Boolean);
  beginPick(state, { title: `${ab.name}: choose a target`, options: firsts,
    onPick: (key, t) => withSacrifice(targets.find(x => targetKey(x[0]) === key) || [t as Target]), onCancel: back });
}

// ------------------------------------------------------------ pick / mark

interface PickArgs {
  title: string;
  options: readonly (Target | ObjectId)[];
  onPick: (key: string, value: Target | ObjectId) => void;
  onCancel?: () => void;
  onDecline?: (() => void) | null;
  declineLabel?: string;
}

/** Pick one thing on the board. `options` are targets or object ids. */
export function beginPick(state: LiveState, { title, options, onPick, onCancel, declineLabel, onDecline }: PickArgs): Ui {
  const ui = newUi("pick", title);
  ui.hint = "Click a highlighted card or player.";
  withOptions(ui, options);
  ui.onPick = (key) => onPick(key, ui.options!.get(key)!);
  if (onDecline) ui.buttons.push({ label: declineLabel || "Decline", run: onDecline });
  if (onCancel) ui.buttons.push({ label: "Cancel", run: onCancel });
  ui.onCancel = onCancel;
  ui.rows = offBoardRows(state, ui, (key) => ui.onPick!(key));
  state.ui = ui;
  state.selected = null;
  return ui;
}

interface MarkArgs {
  title: string;
  options: readonly (Target | ObjectId)[];
  min: number;
  max: number;
  onConfirm: (chosen: (Target | ObjectId)[]) => void;
  onCancel?: () => void;
  cancelLabel?: string;
}

/**
 * Why a confirmed selection was refused, in the terms the screen asked in.
 *
 * The same three sentences `CliPlayer::set_count_error` gives, because it
 * is the same prompt asked on another surface.
 */
function markCountError(have: number, min: number, max: number): string {
  const card = (n: number) => (n === 1 ? "card" : "cards");
  if (min === max) return `${have} marked — mark exactly ${min} ${card(min)}`;
  if (min === 0) return `${have} marked — mark at most ${max} ${card(max)}`;
  return `${have} marked — mark between ${min} and ${max} cards`;
}

/** What the screen says when the idle key would commit an empty answer. */
const NOTHING_MARKED =
  "nothing marked — mark what you want, or press Confirm none";

/** Mark between min and max of the options, then confirm. */
function beginMark(state: LiveState, ui: Ui, { title, options, min, max, onConfirm, onCancel, cancelLabel }: MarkArgs): Ui {
  ui.mode = "mark";
  ui.title = title;
  withOptions(ui, options);
  ui.min = min; ui.max = max;
  ui.marked = [];
  ui.hint = min === max ? `Mark ${min}.` : `Mark ${min} to ${max}.`;
  // Whether the player has touched the selection at all. Where an empty
  // answer is legal — Harvest Pyre exiling nothing, "up to N" targets — the
  // idle key would otherwise COMMIT it, and Enter *is* the idle key here:
  // in menu mode it passes priority, which a player presses dozens of times
  // a turn. Issue #262 settled this for the CLI's set screen — "the safe
  // key must not be an answer" — and this is the fourth surface of that
  // same prompt (issues #520, #524).
  //
  // Nothing to mark is not that case: there is nothing else the player
  // could say, so refusing would be a dead end rather than a guard. That is
  // the same carve-out `pick_set` makes for a forced set.
  let touched = options.length === 0;
  ui.toggle = (key) => {
    touched = true;
    const i = ui.marked.indexOf(key);
    if (i >= 0) ui.marked.splice(i, 1);
    else if (ui.marked.length < max) ui.marked.push(key);
  };
  const canConfirm = () => ui.marked.length >= min && ui.marked.length <= max;
  ui.canConfirm = canConfirm;
  // Refusing out loud, both ways round. This used to be a guarded no-op
  // with no else: below the minimum the keyboard got the identical frame
  // back and no reason, which stopped a game dead at DISCARD 1 CARD for 24
  // presses (issue #524a, #518). `beginBlockers` in this same file already
  // refuses out loud, and so does the CLI's set screen.
  ui.onConfirm = () => {
    if (!canConfirm()) { state.notice = markCountError(ui.marked.length, min, max); return; }
    if (ui.marked.length === 0 && !touched) { state.notice = NOTHING_MARKED; return; }
    onConfirm(ui.marked.map(k => ui.options!.get(k)!));
  };
  ui.buttons.push({ label: "Confirm", primary: true, run: ui.onConfirm, enabled: canConfirm });
  // The page's `n`: the deliberate empty answer, kept reachable now that
  // the idle key no longer means it. A button saying what it does is the
  // considered act pressing Enter out of habit is not.
  if (min === 0 && options.length > 0) {
    ui.buttons.push({ label: "Confirm none", run: () => { ui.marked = []; onConfirm([]); } });
  }
  if (onCancel) { ui.buttons.push({ label: cancelLabel || "Cancel", run: onCancel }); ui.onCancel = onCancel; }
  ui.rows = offBoardRows(state, ui, (key) => ui.toggle!(key));
  state.ui = ui;
  return ui;
}

/** Actions that are each one choice of a card or target: pick on the board. */
function beginPickFromActions(state: LiveState, ui: Ui, actions: Action[], title: string, send: Send): Ui {
  const options: (Target | ObjectId)[] = []; const byKey = new Map<string, Action>(); let decline: Action | null = null; const rows: Row[] = [];
  for (const a of actions) {
    const c = isObj(a) && "ResolveChoice" in a ? a.ResolveChoice.choice : null;
    if (!c || typeof c === "string") { rows.push({ label: describeAction(state, a), run: () => send(a) }); continue; }
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
  const declined = decline;
  const pick = beginPick(state, { title, options, onPick: (key) => send(byKey.get(key)!),
    onDecline: declined ? () => send(declined) : null });
  pick.rows!.push(...rows);
  return pick;
}

// ------------------------------------------------------------- list / order

function isRow(r: Row | Action): r is Row { return typeof r === "object" && "label" in r && "run" in r; }

/** A modal list of rows. Rows are actions or {label, run}. */
export function beginList(state: LiveState, ui: Ui, rows: (Row | Action)[], title: string, send: Send, filter: boolean, keepBoard = false): Ui {
  ui.mode = "list";
  ui.title = title;
  ui.rows = rows.map(r => isRow(r) ? r : { label: describeAction(state, r), run: () => send(r) });
  ui.filter = filter || ui.rows.length > 14;
  ui.query = "";
  ui.scroll = 0;
  ui.hint = ui.filter ? "Type to filter, click a row." : "Click a row.";
  ui.keepBoard = keepBoard;
  state.ui = ui;
  return ui;
}

function beginOrder(state: LiveState, ui: Ui, rp: ResolutionPayload, title: string, send: Send): Ui {
  ui.mode = "order";
  ui.title = title;
  const order = ((rp.options ?? []) as string[]).map((label, i) => ({ label, i }));
  ui.order = order;
  ui.hint = "First listed goes first. Use ▲ ▼ to reorder, then confirm.";
  ui.move = (pos, dir) => {
    const j = pos + dir;
    if (j < 0 || j >= order.length) return;
    [order[pos], order[j]] = [order[j], order[pos]];
  };
  // Enter answers here too. The order widget had a Confirm button and no
  // `ui.onConfirm`, so the key `main.ts` routes into that field did nothing
  // at all on this prompt — the one widget where every arrangement is a
  // legal answer and there is nothing to refuse (issue #518).
  ui.canConfirm = () => true;
  ui.onConfirm = () => send(resolve({ ChosenOrder: order.map(o => o.i) }));
  ui.buttons.push({ label: "Confirm", primary: true, run: ui.onConfirm });
  state.ui = ui;
  return ui;
}

/** A typed token as a refusal may echo it: control characters out, clipped.
 *  The terminal's `quote_input` (#282, #283) — a 400-column paste in a
 *  notice is a notice nobody can read. */
function clipToken(typed: string): string {
  const shown = [...typed].map(c => (c < " " || c === "\u007f" ? "\u00b7" : c)).join("");
  return shown.length <= 40 ? shown : shown.slice(0, 40) + "\u2026";
}

// ------------------------------------------------------- X funding, exactly

// The page's copy of `mtg-engine/src/funding.rs`. It is a copy because the
// page builds the `Action` itself — the seat sends `LegalActions` as it is
// and reads one `Action` back — so a change to the allocator has to be made
// here too, and `mtg-gui/tests/x-funding-cases.json` is the fixture that
// fails when only one side of it moves (#404 and #561 are what happens when
// a second request path misses a fix).

/** Tap sums groups `i..` can produce exactly: `rows[i][s]`, bounded at
 *  `limit`. Per residue class, so a group of 4,000 lands is O(limit). */
export function tapReachability(groups: FundingGroup[], limit: number): boolean[][] {
  const width = limit + 1;
  const rows: boolean[][] = new Array(groups.length + 1);
  const last = new Array<boolean>(width).fill(false);
  last[0] = true;
  rows[groups.length] = last;
  for (let i = groups.length - 1; i >= 0; i--) {
    const prev = rows[i + 1];
    const q = groups[i].mana_per_tap;
    if (!q) { rows[i] = prev.slice(); continue; }
    const maxTaps = groups[i].source_ids.length;
    const next = new Array<boolean>(width).fill(false);
    for (let r = 0; r < Math.min(q, width); r++) {
      // Steps of `q` back to the nearest reachable sum; the nearest is the
      // criterion, since everything behind it is further still.
      let tapsBack = -1;
      for (let s = r; s < width; s += q) {
        tapsBack = prev[s] ? 0 : (tapsBack < 0 ? -1 : tapsBack + 1);
        if (tapsBack >= 0 && tapsBack <= maxTaps) next[s] = true;
      }
    }
    rows[i] = next;
  }
  return rows;
}

const manaForX = (opts: FundingOptions, x: number): number => Math.max(0, x - (opts.x_discount || 0));
const poolTotal = (opts: FundingOptions): number =>
  Object.values(opts.pool || {}).reduce((a, n) => a + (n || 0), 0);

/** Every X the player may announce that some allocation funds exactly.
 *  Not `0..=max_x + x_discount`: a lone Sol Ring pays 0 and 2, and nothing
 *  between (#595). Always contains 0. */
export function fundableXValues(opts: FundingOptions): number[] {
  const maxX = (opts.max_x || 0) + (opts.x_discount || 0);
  const reachable = tapReachability(opts.groups || [], opts.max_x || 0)[0];
  const pool = poolTotal(opts);
  const largestAtOrBelow: number[] = new Array(reachable.length).fill(-1);
  let best = -1;
  for (let s = 0; s < reachable.length; s++) {
    if (reachable[s]) best = s;
    largestAtOrBelow[s] = best;
  }
  const out: number[] = [];
  for (let x = 0; x <= maxX; x++) {
    const mana = manaForX(opts, x);
    const s = mana < largestAtOrBelow.length ? largestAtOrBelow[mana] : -1;
    if (s >= 0 && mana - s <= pool) out.push(x);
  }
  return out;
}

/** The response that funds `x`, and what could not be funded.
 *  Exact whenever the board can pay it — the page used to take whole
 *  activations greedily in category order, so one Mountain and one Sol Ring
 *  funded 1 for a player who typed 2 and left the Sol Ring untapped
 *  (#593) — with the pool drained before taps and lands before rocks before
 *  dorks as the tie-break among the exact allocations. */
export function allocateForX(opts: FundingOptions, x: number): { response: FundingResponse; shortfall: number } {
  const response: FundingResponse = { pool: {}, taps: {} };
  const target = manaForX(opts, x);
  const groups = opts.groups || [];
  const rows = tapReachability(groups, target);
  const reachable = rows[0];
  const pool = poolTotal(opts);

  const floor = Math.max(0, target - pool);
  let tapSum = -1;
  for (let s = floor; s <= target; s++) if (reachable[s]) { tapSum = s; break; }
  if (tapSum < 0) {
    // Nothing in the window: spend the whole pool behind the largest tap
    // sum there is and report the rest.
    tapSum = 0;
    for (let s = floor - 1; s >= 0; s--) if (reachable[s]) { tapSum = s; break; }
  }

  let remaining = target - tapSum;
  const poolSorted = (Object.entries(opts.pool || {}) as [ManaType, number][])
    .filter(([, n]) => (n || 0) > 0)
    .sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
  for (const [mt, avail] of poolSorted) {
    if (remaining === 0) break;
    const take = Math.min(avail, remaining);
    if (take > 0) { response.pool[mt] = take; remaining -= take; }
  }

  let left = tapSum;
  for (let i = 0; i < groups.length; i++) {
    if (left === 0) break;
    const g = groups[i];
    if (!g.mana_per_tap) continue;
    const ceiling = Math.min(Math.floor(left / g.mana_per_tap), g.source_ids.length);
    let taps = 0;
    for (let t = ceiling; t >= 0; t--) {
      if (rows[i + 1][left - t * g.mana_per_tap]) { taps = t; break; }
    }
    if (taps > 0) { const amount = taps * g.mana_per_tap; response.taps[g.name] = amount; left -= amount; }
  }

  return { response, shortfall: remaining };
}

/** The payable set as one line, runs collapsed, capped, both ends kept —
 *  the terminal's `describe_fundable_x` (#595). */
export function describeFundableX(values: number[]): string {
  const MAX = 44;
  const runs: string[] = [];
  for (let i = 0; i < values.length; i++) {
    const start = values[i];
    let end = start;
    while (i + 1 < values.length && values[i + 1] === end + 1) end = values[++i];
    runs.push(start === end ? `${start}` : `${start}-${end}`);
  }
  const full = runs.join(", ");
  if (full.length <= MAX || runs.length < 3) return full;
  const last = runs[runs.length - 1];
  const budget = Math.max(0, MAX - (last.length + 5));
  const kept: string[] = [];
  let cols = 0;
  for (const run of runs.slice(0, -1)) {
    const next = cols + run.length + 2;
    if (next > budget) break;
    cols = next;
    kept.push(run);
  }
  return `${kept.join(", ")}, \u2026 ${last}`;
}

/** X: one number, distributed over the pool and the tap groups the way the CLI does. */
function beginNumber(state: LiveState, ui: Ui, opts: FundingOptions, title: string, send: Send): Ui {
  const maxX = (opts.max_x || 0) + (opts.x_discount || 0);
  ui.mode = "number";
  ui.title = title;
  ui.max = maxX;
  ui.value = "";
  const summary: string[] = [];
  ui.summary = summary;
  const pool = Object.entries(opts.pool || {}).filter(([, n]) => (n ?? 0) > 0).map(([k, n]) => `${n} ${k}`).join(", ");
  if (pool) summary.push(`Pool: ${pool}`);
  for (const g of opts.groups || []) summary.push(`${g.name} x${g.source_ids.length} (${g.mana_per_tap}/tap)`);
  // Which values of X the sources can actually pay. The page used to state
  // `0-N`, accept every integer in it, fund a smaller X and send it with
  // `state.notice` left null — a card spent, the popover closed, and
  // nothing on the page ever mentioning it (#594). The terminal's half of
  // the same silence is #595.
  const fundable = fundableXValues(opts);
  const payable = new Set(fundable);
  if (fundable.length !== maxX + 1) summary.push(`Payable X: ${describeFundableX(fundable)}`);
  ui.hint = `Type X (0-${maxX}) and press Enter.`;
  ui.submit = () => {
    // The terminal's reader, not JavaScript's. `Number("")` is 0 and
    // `Number.isInteger(0)` is true, so Enter at an empty box announced
    // X = 0 and completed the cast — a card gone, irreversibly, from the
    // key the rest of the program treats as "do nothing". That is #123,
    // whose fix went into `cli.rs` and never reached this page, which
    // copied the refusal string next to it and not the empty-input arm
    // it sits in (#561, the shape of #520/#524).
    //
    // `Number` also takes `0x2`, `0b11`, `2.0`, `1e1` and `-0`, none of
    // which `str::parse::<u32>()` takes, so the two surfaces disagreed
    // about what a legal answer to one prompt is. This is that parser: an
    // optional `+` and ASCII digits, nothing else.
    const typed = (ui.value ?? "").trim();
    // A refusal empties the box, the way the terminal's reader starts each
    // attempt on a cleared row. Keeping the refused token meant the next
    // digit appended to it, so answering `1` and then `2` at a prompt that
    // refused the 1 asked for X = 12.
    const refuse = (msg: string) => { state.notice = msg; ui.value = ""; };
    if (typed === "") { state.notice = "Enter a value for X."; return; }
    const x = /^\+?[0-9]+$/.test(typed) ? Number(typed) : NaN;
    if (!Number.isInteger(x) || x < 0 || x > maxX) {
      refuse(`Invalid input '${clipToken(typed)}' — enter an integer between 0 and ${maxX}.`);
      return;
    }
    // In range and unpayable: say which values are, and keep asking. The
    // cast is still cancellable, and nothing is spent until a payable X is
    // confirmed.
    if (!payable.has(x)) {
      refuse(`X = ${x} is not payable with these sources — payable X: ${describeFundableX(fundable)}.`);
      return;
    }
    send(resolve({ XFunding: allocateForX(opts, x).response }));
  };
  ui.buttons.push({ label: "Confirm", primary: true, run: ui.submit });
  ui.buttons.push({ label: "Cancel", run: () => send(resolve({ ChosenTarget: null })) });
  state.ui = ui;
  return ui;
}

// ----------------------------------------------------------------- combat

type AttackersPrompt = Extract<CombatPrompt, { ChooseAttackers: unknown }>["ChooseAttackers"];
type BlockersPrompt = Extract<CombatPrompt, { ChooseBlockers: unknown }>["ChooseBlockers"];

function beginAttackers(state: LiveState, c: AttackersPrompt, send: Send): Ui {
  const ui = newUi("attackers", "Declare attackers");
  withOptions(ui, c.eligible);
  const locked = new Set(c.must_attack.map(id => `o${id}`));
  ui.locked = locked;
  ui.marked = [...locked];
  const defenders: NonNullable<Ui["defenders"]> = [{ label: "Opponent", player: c.defending_player },
    ...(c.defending_planeswalkers || []).map(id => ({ label: nameOf(state, id), planeswalker: id }))];
  ui.defenders = defenders;
  const attackTarget = new Map<string, number>();
  ui.attackTarget = attackTarget;
  ui.hint = defenders.length > 1 ? "Click a creature to attack; click again to change whom it attacks, again to withdraw." : "Click creatures to attack with, then confirm.";
  ui.toggle = (key) => {
    if (!ui.marked.includes(key)) { ui.marked.push(key); attackTarget.set(key, 0); return; }
    const cur = attackTarget.get(key) || 0;
    if (cur + 1 < defenders.length) { attackTarget.set(key, cur + 1); return; }
    if (locked.has(key)) { attackTarget.set(key, 0); return; }
    ui.marked.splice(ui.marked.indexOf(key), 1);
    attackTarget.delete(key);
  };
  ui.badge = (key) => {
    if (!ui.marked.includes(key)) return null;
    const d = defenders[attackTarget.get(key) || 0];
    return d.planeswalker !== undefined ? `→${d.label.slice(0, 6)}` : "ATK";
  };
  ui.canConfirm = () => true;
  ui.onConfirm = () => {
    const attackers: [ObjectId, PlayerId][] = []; const planeswalker_attacks: [ObjectId, ObjectId][] = [];
    for (const key of ui.marked) {
      const id = Number(key.slice(1));
      const d = defenders[attackTarget.get(key) || 0];
      if (d.planeswalker !== undefined) planeswalker_attacks.push([id, d.planeswalker]);
      else attackers.push([id, d.player!]);
    }
    send({ DeclareAttackers: { attackers, planeswalker_attacks } });
  };
  ui.buttons.push({ label: "Attack", primary: true, run: ui.onConfirm });
  ui.buttonLabel = () => (ui.marked.length ? `Attack with ${ui.marked.length}` : "No attackers");
  state.ui = ui;
  state.selected = null;
  return ui;
}

function beginBlockers(state: LiveState, c: BlockersPrompt, send: Send): Ui {
  const ui = newUi("blockers", "Declare blockers");
  ui.hint = "Click a blocker, then the attacker it blocks. Click a blocker again to unassign.";
  withOptions(ui, c.eligible_blockers);
  const attackers = new Set(c.attackers.map(id => `o${id}`));
  ui.attackers = attackers;
  const legal = new Map(Object.entries(c.legal_blocks || {}).map(([k, v]) => [`o${k}`, new Set(v.map(id => `o${id}`))]));
  ui.legal = legal;
  const minBlockers = new Map(Object.entries(c.min_blockers || {}).map(([k, v]) => [`o${k}`, v]));
  ui.minBlockers = minBlockers;
  const assignments = new Map<string, string>();
  ui.assignments = assignments;
  ui.selectedBlocker = null;
  ui.clickBlocker = (key) => {
    if (assignments.has(key)) { assignments.delete(key); ui.selectedBlocker = null; return; }
    ui.selectedBlocker = ui.selectedBlocker === key ? null : key;
  };
  ui.clickAttacker = (key) => {
    if (!ui.selectedBlocker) return;
    const allowed = legal.get(ui.selectedBlocker);
    if (allowed && !allowed.has(key)) { state.notice = `${nameOf(state, Number(ui.selectedBlocker.slice(1)))} cannot block that.`; return; }
    assignments.set(ui.selectedBlocker, key);
    ui.selectedBlocker = null;
  };
  ui.canAttackerTake = (key) => !!ui.selectedBlocker && (!legal.get(ui.selectedBlocker) || legal.get(ui.selectedBlocker)!.has(key));
  ui.badge = (key) => {
    const a = assignments.get(key);
    if (a) return `⛨${nameOf(state, Number(a.slice(1))).slice(0, 5)}`;
    return ui.selectedBlocker === key ? "?" : null;
  };
  const blockersOn = (attackerKey: string) => [...assignments].filter(([, a]) => a === attackerKey).length;
  ui.blockersOn = blockersOn;
  const canConfirm = () => {
    for (const [ak, min] of minBlockers) { const n = blockersOn(ak); if (n > 0 && n < min) return false; }
    return true;
  };
  ui.canConfirm = canConfirm;
  ui.onConfirm = () => {
    if (!canConfirm()) { state.notice = "An attacker with menace needs two or more blockers, or none."; return; }
    send({ DeclareBlockers: { assignments: [...assignments].map(([b, a]) => [Number(b.slice(1)), Number(a.slice(1))]) } });
  };
  ui.buttons.push({ label: "Confirm blocks", primary: true, run: ui.onConfirm, enabled: canConfirm });
  ui.buttonLabel = () => (assignments.size ? `Block with ${assignments.size}` : "No blocks");
  state.ui = ui;
  state.selected = null;
  return ui;
}

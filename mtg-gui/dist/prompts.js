// From a decision message to something the page can click.
//
// The seat sends the engine's `LegalActions` as it is. This file turns it
// into one of a handful of widget shapes (menu, pick, mark, attackers,
// blockers, list, order, number) and builds the `Action` the answer is.
// An unknown prompt kind falls through to a plain list of whatever legal
// actions were offered, never to nothing.
import { tag } from "./protocol.js";
// ---------------------------------------------------------------- lookups
/** Every object the view can name, by id. */
export function indexView(view) {
    const idx = new Map();
    const put = (obj, zone, owner) => idx.set(obj.object_id, { obj, zone, owner });
    for (const p of view.battlefield)
        put(p, "battlefield", p.controller);
    for (const c of view.your_hand)
        put(c, "hand", view.you);
    for (const [pid, cards] of view.graveyards)
        for (const c of cards)
            put(c, "graveyard", pid);
    for (const c of view.exile)
        put(c, "exile", c.owner);
    for (const s of view.stack)
        put(s, "stack", s.controller);
    for (const c of view.your_library_cards)
        put(c, "library", view.you);
    for (const [id, name] of Object.entries(view.revealed_names || {})) {
        const n = Number(id);
        if (!idx.has(n))
            idx.set(n, { obj: { object_id: n, name }, zone: "revealed", owner: null });
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
    if (t === null || t === undefined)
        return "nothing";
    if (t === "Illegal")
        return "(illegal)";
    if ("Object" in t)
        return nameOf(state, t.Object);
    if ("Player" in t)
        return playerLabel(state, t.Player);
    return JSON.stringify(t);
}
/** Mana cost as text: {1}{R}. */
export function costText(cost) {
    if (!cost || !cost.symbols)
        return "";
    return cost.symbols.map(s => {
        if (s === "X")
            return "{X}";
        if (typeof s === "string")
            return `{${s}}`;
        if ("Colored" in s)
            return `{${String(s.Colored)[0]}}`;
        if ("Generic" in s)
            return `{${String(s.Generic)}}`;
        return `{${JSON.stringify(s)}}`;
    }).join("");
}
function isObj(a) { return typeof a === "object"; }
/** What one legal action does, for a row or a popover item. */
export function describeAction(state, a) {
    if (a === "PassPriority")
        return "Pass priority";
    if (a === "Concede")
        return "Concede";
    if (a === "MulliganKeep")
        return "Keep this hand";
    if (a === "MulliganMull")
        return "Mulligan";
    if (a === "AbandonGame")
        return "Abandon (harness)";
    if ("PlayLand" in a)
        return `Play ${nameOf(state, a.PlayLand.object_id)}`;
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
    if ("DeclareAttackers" in a)
        return `Attack with ${a.DeclareAttackers.attackers.length + (a.DeclareAttackers.planeswalker_attacks || []).length}`;
    if ("DeclareBlockers" in a)
        return `Block with ${a.DeclareBlockers.assignments.length}`;
    if ("DiscardCards" in a)
        return `Discard ${a.DiscardCards.cards.map(id => nameOf(state, id)).join(", ")}`;
    if ("BottomCards" in a)
        return `Bottom ${a.BottomCards.cards.map(id => nameOf(state, id)).join(", ")}`;
    if ("ResolveChoice" in a)
        return describeChoice(state, a.ResolveChoice.choice);
    return JSON.stringify(a);
}
function describeChoice(state, c) {
    if (c === "CancelCast")
        return "Cancel the cast";
    if ("PayDecision" in c)
        return c.PayDecision ? "Pay" : "Don't pay";
    if ("YesNoDecision" in c)
        return c.YesNoDecision ? "Yes" : "No";
    if ("ChosenTarget" in c)
        return c.ChosenTarget === null ? "Decline" : targetLabel(state, c.ChosenTarget);
    if ("ChosenCard" in c)
        return nameOf(state, c.ChosenCard);
    if ("ChosenIndex" in c)
        return c.ChosenIndex[1];
    if ("ChosenOrder" in c)
        return `Order: ${c.ChosenOrder.join(", ")}`;
    if ("ChosenSubset" in c)
        return `Pile: ${c.ChosenSubset.map(id => nameOf(state, id)).join(", ")}`;
    if ("ChosenExileSet" in c)
        return `Exile ${c.ChosenExileSet.map(id => nameOf(state, id)).join(", ")}`;
    if ("ChosenTargetSet" in c)
        return `Targets: ${c.ChosenTargetSet.map(x => targetLabel(state, x)).join(", ")}`;
    if ("ChosenObjectSet" in c)
        return `Choose ${c.ChosenObjectSet.map(id => nameOf(state, id)).join(", ")}`;
    if ("XFunding" in c)
        return `X funding ${JSON.stringify(c.XFunding)}`;
    return JSON.stringify(c);
}
// ------------------------------------------------------------- the widget
function resolve(choice) { return { ResolveChoice: { choice } }; }
export function targetKey(t) {
    if (t && typeof t === "object") {
        if ("Object" in t)
            return `o${t.Object}`;
        if ("Player" in t)
            return `p${t.Player}`;
    }
    return null;
}
function newUi(mode, title) {
    return { mode, title, hint: "", buttons: [], marked: [] };
}
/** A widget over a set of targets or object ids. */
function withOptions(ui, list) {
    const options = new Map();
    for (const t of list) {
        if (typeof t === "number")
            options.set(`o${t}`, t);
        else {
            const k = targetKey(t);
            if (k)
                options.set(k, t);
        }
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
function offBoardRows(state, ui, run) {
    const rows = [];
    let openZone = null;
    for (const [key, t] of ui.options ?? []) {
        const id = key[0] === "o" ? Number(key.slice(1)) : null;
        const e = id === null ? null : state.index.get(id);
        if (key[0] === "p" || (e && ON_BOARD.has(e.zone)))
            continue;
        if (e && (e.zone === "graveyard" || e.zone === "exile") && e.owner !== null && !openZone)
            openZone = { zone: e.zone, pid: e.owner };
        const label = id === null ? targetLabel(state, t) : nameOf(state, id);
        rows.push({ label, run: () => run(key), key, cardName: e ? e.obj.name : null });
    }
    if (openZone)
        state.overlay = openZone;
    return rows;
}
/**
 * Decide how the pending decision is answered. Sets `state.ui`.
 * `send(action)` answers; `state.notice` shows text in the panel.
 */
export function beginDecision(state, send) {
    const d = state.decision;
    if (!d)
        throw new Error("no decision to begin");
    const legal = d.legal;
    const actions = legal.actions || [];
    const ui = newUi("menu", legal.context || "");
    state.ui = ui;
    state.selected = null;
    // Combat, asked as its own prompt.
    if (d.combat) {
        if ("ChooseAttackers" in d.combat)
            return beginAttackers(state, d.combat.ChooseAttackers, send);
        if ("ChooseBlockers" in d.combat)
            return beginBlockers(state, d.combat.ChooseBlockers, send);
    }
    // A set of cards out of a list: mulligan bottoming, cleanup discard.
    if (legal.set_prompt) {
        const sp = legal.set_prompt;
        const answer = (cards) => sp.kind === "BottomAfterMulligan" ? { BottomCards: { cards } } : { DiscardCards: { cards } };
        return beginMark(state, ui, {
            title: legal.context || (sp.kind === "BottomAfterMulligan" ? "Put cards on the bottom" : "Discard to hand size"),
            options: sp.options, min: sp.min, max: sp.max,
            onConfirm: (chosen) => send(answer(chosen)),
        });
    }
    // A mid-resolution prompt.
    if (legal.resolution_prompt) {
        const kind = tag(legal.resolution_prompt);
        const rp = legal.resolution_prompt[kind];
        const desc = rp.description || legal.context || kind;
        const ids = (rp.options ?? []);
        switch (kind) {
            case "ChooseXFunding": return beginNumber(state, ui, rp.options, desc, send);
            case "ChooseExileFromGraveyard":
                return beginMark(state, ui, { title: desc, options: ids, min: rp.min ?? 0, max: rp.max ?? ids.length,
                    onConfirm: (chosen) => send(resolve({ ChosenExileSet: chosen })),
                    onCancel: () => send(resolve("CancelCast")), cancelLabel: "Cancel cast" });
            case "ChooseObjectSet":
                return beginMark(state, ui, { title: desc, options: ids, min: rp.min ?? 0, max: rp.max ?? ids.length,
                    onConfirm: (chosen) => send(resolve({ ChosenObjectSet: chosen })) });
            case "ChooseTargetSet": {
                const targets = (rp.options ?? []);
                return beginMark(state, ui, { title: desc, options: targets, min: rp.min ?? 0, max: rp.max ?? targets.length,
                    onConfirm: (chosen) => send(resolve({ ChosenTargetSet: chosen })),
                    onCancel: () => send(resolve("CancelCast")), cancelLabel: "Cancel cast" });
            }
            case "DividePermanentsIntoPiles": {
                const perms = rp.permanents ?? [];
                return beginMark(state, ui, { title: desc + " (mark pile 1)", options: perms, min: 0, max: perms.length,
                    onConfirm: (chosen) => send(resolve({ ChosenSubset: chosen })) });
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
function beginMenu(state, ui, actions, legal, send) {
    ui.mode = "menu";
    const verbs = new Map();
    ui.verbs = verbs;
    const add = (id, label, run) => {
        if (!verbs.has(id))
            verbs.set(id, []);
        verbs.get(id).push({ label, run });
    };
    const hasPass = actions.includes("PassPriority");
    for (const a of actions) {
        if (!isObj(a))
            continue;
        if ("PlayLand" in a)
            add(a.PlayLand.object_id, "Play land", () => send(a));
        else if ("ActivateManaAbility" in a)
            add(a.ActivateManaAbility.object_id, describeAction(state, a).split(": ")[1] || "Tap for mana", () => send(a));
        else if ("ActivateLoyaltyAbility" in a)
            add(a.ActivateLoyaltyAbility.object_id, describeAction(state, a).split(": ").slice(1).join(": "), () => send(a));
        else if ("CastSpell" in a || "ActivateAbility" in a) { /* collapsed below */ }
        else
            add(-1, describeAction(state, a), () => send(a));
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
    ui.hint = hasPass ? "Click a card for what it can do. Enter passes." : "Choose an action.";
    if (hasPass)
        ui.buttons.push({ label: "Pass", primary: true, run: () => send("PassPriority") });
    if (actions.includes("Concede"))
        ui.buttons.push({ label: "Concede", run: () => {
                beginList(state, newUi("list", "Concede the game?"), [{ label: "Yes, concede", run: () => send("Concede") }, { label: "No", run: () => beginDecision(state, send) }], "Concede the game?", send, false, true);
            } });
    ui.canPass = hasPass;
    return ui;
}
function forcedTargets(spec) {
    if (typeof spec === "object" && "SingleTarget" in spec && spec.SingleTarget.length === 1)
        return spec.SingleTarget;
    return [];
}
function abilitySlots(ab) {
    const targets = [];
    const sacrifices = [];
    const seenT = new Set();
    const seenS = new Set();
    for (const o of ab.option_combos || []) {
        const tk = JSON.stringify(o.targets);
        if (!seenT.has(tk)) {
            seenT.add(tk);
            targets.push(o.targets);
        }
        const sk = JSON.stringify(o.sacrifice);
        if (!seenS.has(sk)) {
            seenS.add(sk);
            sacrifices.push(o.sacrifice);
        }
    }
    return { targets, sacrifices };
}
/** Cast a spell: pick the targets and the sacrifice it still needs. */
function castFlow(state, cs, send) {
    const finish = (targets, sacrifice) => send({ CastSpell: {
            object_id: cs.object_id, targets, sacrifice, exile_count: null, exile_ids: [],
            alternative_cost: cs.alternative_cost, tap_plan: cs.tap_plan,
        } });
    const back = () => { beginDecision(state, send); };
    const withSacrifice = (targets) => {
        if (cs.sacrifice_options.length === 0)
            return finish(targets, null);
        if (cs.sacrifice_options.length === 1)
            return finish(targets, cs.sacrifice_options[0]);
        beginPick(state, { title: `${cs.name}: choose a creature to sacrifice`, options: cs.sacrifice_options,
            onPick: (_key, id) => finish(targets, id), onCancel: back });
    };
    const spec = cs.target_spec;
    if (spec === "NoTargets" || spec === "ChosenAtCast")
        return withSacrifice([]);
    if ("SingleTarget" in spec) {
        if (spec.SingleTarget.length === 1)
            return withSacrifice(spec.SingleTarget);
        beginPick(state, { title: `${cs.name}: choose a target`, options: spec.SingleTarget,
            onPick: (_key, t) => withSacrifice([t]), onCancel: back });
        return;
    }
    if ("TwoTargets" in spec) {
        beginPick(state, { title: `${cs.name}: choose the first target`, options: spec.TwoTargets.first,
            onPick: (_key, t) => withSacrifice([t]), onCancel: back });
        return;
    }
    withSacrifice([]);
}
function abilityFlow(state, ab, send) {
    const back = () => { beginDecision(state, send); };
    const finish = (targets, sacrifice) => send({ ActivateAbility: {
            object_id: ab.object_id, ability_index: ab.ability_index, targets, tap_plan: ab.tap_plan,
            sacrifice, x_value: null, source_card_id: ab.source_card_id,
        } });
    const { targets } = abilitySlots(ab);
    const withSacrifice = (chosen) => {
        const sacs = [];
        const seen = new Set();
        for (const o of ab.option_combos || []) {
            if (JSON.stringify(o.targets) !== JSON.stringify(chosen))
                continue;
            const k = JSON.stringify(o.sacrifice);
            if (!seen.has(k)) {
                seen.add(k);
                sacs.push(o.sacrifice);
            }
        }
        if (sacs.length <= 1)
            return finish(chosen, sacs.length ? sacs[0] : null);
        beginPick(state, { title: `${ab.name}: choose a creature to sacrifice`, options: sacs.filter((s) => s !== null),
            onPick: (_key, id) => finish(chosen, id), onCancel: back });
    };
    if (targets.length === 0)
        return withSacrifice([]);
    if (targets.length === 1)
        return withSacrifice(targets[0]);
    const firsts = targets.map(t => t[0]).filter(Boolean);
    beginPick(state, { title: `${ab.name}: choose a target`, options: firsts,
        onPick: (key, t) => withSacrifice(targets.find(x => targetKey(x[0]) === key) || [t]), onCancel: back });
}
/** Pick one thing on the board. `options` are targets or object ids. */
export function beginPick(state, { title, options, onPick, onCancel, declineLabel, onDecline }) {
    const ui = newUi("pick", title);
    ui.hint = "Click a highlighted card or player.";
    withOptions(ui, options);
    ui.onPick = (key) => onPick(key, ui.options.get(key));
    if (onDecline)
        ui.buttons.push({ label: declineLabel || "Decline", run: onDecline });
    if (onCancel)
        ui.buttons.push({ label: "Cancel", run: onCancel });
    ui.onCancel = onCancel;
    ui.rows = offBoardRows(state, ui, (key) => ui.onPick(key));
    state.ui = ui;
    state.selected = null;
    return ui;
}
/**
 * Why a confirmed selection was refused, in the terms the screen asked in.
 *
 * The same three sentences `CliPlayer::set_count_error` gives, because it
 * is the same prompt asked on another surface.
 */
function markCountError(have, min, max) {
    const card = (n) => (n === 1 ? "card" : "cards");
    if (min === max)
        return `${have} marked — mark exactly ${min} ${card(min)}`;
    if (min === 0)
        return `${have} marked — mark at most ${max} ${card(max)}`;
    return `${have} marked — mark between ${min} and ${max} cards`;
}
/** What the screen says when the idle key would commit an empty answer. */
const NOTHING_MARKED = "nothing marked — mark what you want, or press Confirm none";
/** Mark between min and max of the options, then confirm. */
function beginMark(state, ui, { title, options, min, max, onConfirm, onCancel, cancelLabel }) {
    ui.mode = "mark";
    ui.title = title;
    withOptions(ui, options);
    ui.min = min;
    ui.max = max;
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
        if (i >= 0)
            ui.marked.splice(i, 1);
        else if (ui.marked.length < max)
            ui.marked.push(key);
    };
    const canConfirm = () => ui.marked.length >= min && ui.marked.length <= max;
    ui.canConfirm = canConfirm;
    // Refusing out loud, both ways round. This used to be a guarded no-op
    // with no else: below the minimum the keyboard got the identical frame
    // back and no reason, which stopped a game dead at DISCARD 1 CARD for 24
    // presses (issue #524a, #518). `beginBlockers` in this same file already
    // refuses out loud, and so does the CLI's set screen.
    ui.onConfirm = () => {
        if (!canConfirm()) {
            state.notice = markCountError(ui.marked.length, min, max);
            return;
        }
        if (ui.marked.length === 0 && !touched) {
            state.notice = NOTHING_MARKED;
            return;
        }
        onConfirm(ui.marked.map(k => ui.options.get(k)));
    };
    ui.buttons.push({ label: "Confirm", primary: true, run: ui.onConfirm, enabled: canConfirm });
    // The page's `n`: the deliberate empty answer, kept reachable now that
    // the idle key no longer means it. A button saying what it does is the
    // considered act pressing Enter out of habit is not.
    if (min === 0 && options.length > 0) {
        ui.buttons.push({ label: "Confirm none", run: () => { ui.marked = []; onConfirm([]); } });
    }
    if (onCancel) {
        ui.buttons.push({ label: cancelLabel || "Cancel", run: onCancel });
        ui.onCancel = onCancel;
    }
    ui.rows = offBoardRows(state, ui, (key) => ui.toggle(key));
    state.ui = ui;
    return ui;
}
/** Actions that are each one choice of a card or target: pick on the board. */
function beginPickFromActions(state, ui, actions, title, send) {
    const options = [];
    const byKey = new Map();
    let decline = null;
    const rows = [];
    for (const a of actions) {
        const c = isObj(a) && "ResolveChoice" in a ? a.ResolveChoice.choice : null;
        if (!c || typeof c === "string") {
            rows.push({ label: describeAction(state, a), run: () => send(a) });
            continue;
        }
        if ("ChosenTarget" in c) {
            if (c.ChosenTarget === null) {
                decline = a;
                continue;
            }
            const k = targetKey(c.ChosenTarget);
            if (k) {
                options.push(c.ChosenTarget);
                byKey.set(k, a);
            }
        }
        else if ("ChosenCard" in c) {
            options.push(c.ChosenCard);
            byKey.set(`o${c.ChosenCard}`, a);
        }
        else {
            rows.push({ label: describeAction(state, a), run: () => send(a) });
        }
    }
    if (options.length === 0)
        return beginList(state, ui, actions, title, send, false);
    const declined = decline;
    const pick = beginPick(state, { title, options, onPick: (key) => send(byKey.get(key)),
        onDecline: declined ? () => send(declined) : null });
    pick.rows.push(...rows);
    return pick;
}
// ------------------------------------------------------------- list / order
function isRow(r) { return typeof r === "object" && "label" in r && "run" in r; }
/** A modal list of rows. Rows are actions or {label, run}. */
export function beginList(state, ui, rows, title, send, filter, keepBoard = false) {
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
function beginOrder(state, ui, rp, title, send) {
    ui.mode = "order";
    ui.title = title;
    const order = (rp.options ?? []).map((label, i) => ({ label, i }));
    ui.order = order;
    ui.hint = "First listed goes first. Use ▲ ▼ to reorder, then confirm.";
    ui.move = (pos, dir) => {
        const j = pos + dir;
        if (j < 0 || j >= order.length)
            return;
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
/** X: one number, distributed over the pool and the tap groups the way the CLI does. */
function beginNumber(state, ui, opts, title, send) {
    const maxX = (opts.max_x || 0) + (opts.x_discount || 0);
    ui.mode = "number";
    ui.title = title;
    ui.max = maxX;
    ui.value = "";
    const summary = [];
    ui.summary = summary;
    const pool = Object.entries(opts.pool || {}).filter(([, n]) => (n ?? 0) > 0).map(([k, n]) => `${n} ${k}`).join(", ");
    if (pool)
        summary.push(`Pool: ${pool}`);
    for (const g of opts.groups || [])
        summary.push(`${g.name} x${g.source_ids.length} (${g.mana_per_tap}/tap)`);
    ui.hint = `Type X (0-${maxX}) and press Enter.`;
    ui.submit = () => {
        const x = Number(ui.value);
        if (!Number.isInteger(x) || x < 0 || x > maxX) {
            state.notice = `Enter an integer between 0 and ${maxX}.`;
            return;
        }
        let remaining = Math.max(0, x - (opts.x_discount || 0));
        const response = { pool: {}, taps: {} };
        const poolSorted = Object.entries(opts.pool || {}).sort((a, b) => b[1] - a[1]);
        for (const [mt, avail] of poolSorted) {
            if (remaining === 0)
                break;
            const take = Math.min(avail, remaining);
            if (take > 0) {
                response.pool[mt] = take;
                remaining -= take;
            }
        }
        for (const g of opts.groups || []) {
            if (remaining === 0)
                break;
            if (!g.mana_per_tap)
                continue;
            const taps = Math.min(Math.floor(remaining / g.mana_per_tap), g.source_ids.length);
            if (taps > 0) {
                const amount = taps * g.mana_per_tap;
                response.taps[g.name] = amount;
                remaining -= amount;
            }
        }
        send(resolve({ XFunding: response }));
    };
    ui.buttons.push({ label: "Confirm", primary: true, run: ui.submit });
    ui.buttons.push({ label: "Cancel", run: () => send(resolve({ ChosenTarget: null })) });
    state.ui = ui;
    return ui;
}
function beginAttackers(state, c, send) {
    const ui = newUi("attackers", "Declare attackers");
    withOptions(ui, c.eligible);
    const locked = new Set(c.must_attack.map(id => `o${id}`));
    ui.locked = locked;
    ui.marked = [...locked];
    const defenders = [{ label: "Opponent", player: c.defending_player },
        ...(c.defending_planeswalkers || []).map(id => ({ label: nameOf(state, id), planeswalker: id }))];
    ui.defenders = defenders;
    const attackTarget = new Map();
    ui.attackTarget = attackTarget;
    ui.hint = defenders.length > 1 ? "Click a creature to attack; click again to change whom it attacks, again to withdraw." : "Click creatures to attack with, then confirm.";
    ui.toggle = (key) => {
        if (!ui.marked.includes(key)) {
            ui.marked.push(key);
            attackTarget.set(key, 0);
            return;
        }
        const cur = attackTarget.get(key) || 0;
        if (cur + 1 < defenders.length) {
            attackTarget.set(key, cur + 1);
            return;
        }
        if (locked.has(key)) {
            attackTarget.set(key, 0);
            return;
        }
        ui.marked.splice(ui.marked.indexOf(key), 1);
        attackTarget.delete(key);
    };
    ui.badge = (key) => {
        if (!ui.marked.includes(key))
            return null;
        const d = defenders[attackTarget.get(key) || 0];
        return d.planeswalker !== undefined ? `→${d.label.slice(0, 6)}` : "ATK";
    };
    ui.canConfirm = () => true;
    ui.onConfirm = () => {
        const attackers = [];
        const planeswalker_attacks = [];
        for (const key of ui.marked) {
            const id = Number(key.slice(1));
            const d = defenders[attackTarget.get(key) || 0];
            if (d.planeswalker !== undefined)
                planeswalker_attacks.push([id, d.planeswalker]);
            else
                attackers.push([id, d.player]);
        }
        send({ DeclareAttackers: { attackers, planeswalker_attacks } });
    };
    ui.buttons.push({ label: "Attack", primary: true, run: ui.onConfirm });
    ui.buttonLabel = () => (ui.marked.length ? `Attack with ${ui.marked.length}` : "No attackers");
    state.ui = ui;
    state.selected = null;
    return ui;
}
function beginBlockers(state, c, send) {
    const ui = newUi("blockers", "Declare blockers");
    ui.hint = "Click a blocker, then the attacker it blocks. Click a blocker again to unassign.";
    withOptions(ui, c.eligible_blockers);
    const attackers = new Set(c.attackers.map(id => `o${id}`));
    ui.attackers = attackers;
    const legal = new Map(Object.entries(c.legal_blocks || {}).map(([k, v]) => [`o${k}`, new Set(v.map(id => `o${id}`))]));
    ui.legal = legal;
    const minBlockers = new Map(Object.entries(c.min_blockers || {}).map(([k, v]) => [`o${k}`, v]));
    ui.minBlockers = minBlockers;
    const assignments = new Map();
    ui.assignments = assignments;
    ui.selectedBlocker = null;
    ui.clickBlocker = (key) => {
        if (assignments.has(key)) {
            assignments.delete(key);
            ui.selectedBlocker = null;
            return;
        }
        ui.selectedBlocker = ui.selectedBlocker === key ? null : key;
    };
    ui.clickAttacker = (key) => {
        if (!ui.selectedBlocker)
            return;
        const allowed = legal.get(ui.selectedBlocker);
        if (allowed && !allowed.has(key)) {
            state.notice = `${nameOf(state, Number(ui.selectedBlocker.slice(1)))} cannot block that.`;
            return;
        }
        assignments.set(ui.selectedBlocker, key);
        ui.selectedBlocker = null;
    };
    ui.canAttackerTake = (key) => !!ui.selectedBlocker && (!legal.get(ui.selectedBlocker) || legal.get(ui.selectedBlocker).has(key));
    ui.badge = (key) => {
        const a = assignments.get(key);
        if (a)
            return `⛨${nameOf(state, Number(a.slice(1))).slice(0, 5)}`;
        return ui.selectedBlocker === key ? "?" : null;
    };
    const blockersOn = (attackerKey) => [...assignments].filter(([, a]) => a === attackerKey).length;
    ui.blockersOn = blockersOn;
    const canConfirm = () => {
        for (const [ak, min] of minBlockers) {
            const n = blockersOn(ak);
            if (n > 0 && n < min)
                return false;
        }
        return true;
    };
    ui.canConfirm = canConfirm;
    ui.onConfirm = () => {
        if (!canConfirm()) {
            state.notice = "An attacker with menace needs two or more blockers, or none.";
            return;
        }
        send({ DeclareBlockers: { assignments: [...assignments].map(([b, a]) => [Number(b.slice(1)), Number(a.slice(1))]) } });
    };
    ui.buttons.push({ label: "Confirm blocks", primary: true, run: ui.onConfirm, enabled: canConfirm });
    ui.buttonLabel = () => (assignments.size ? `Block with ${assignments.size}` : "No blocks");
    state.ui = ui;
    state.selected = null;
    return ui;
}

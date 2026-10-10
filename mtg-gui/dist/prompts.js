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
    for (const s of view.stack)
        if (s.object_id !== 0 && !idx.has(s.object_id))
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
export function inOurWords(state, line) {
    // A token is a possessive, or the subject of a present-tense verb, as
    // often as it is a bare name: swapping the word alone wrote "you's
    // unspent mana", "you keeps" and "Game over! you (red-green) wins!"
    // (#713). The verbs are the engine's and the runner's present-tense ones,
    // the same list the LLM seat conjugates (`try_rewrite_player_token`); a
    // parenthetical between subject and verb — the deck name in the game-over
    // line — is carried across.
    return line.replace(/\bp(\d+)\b('s\b)?((?: \([^()]*\))?) ?(\b(?:keeps|mulligans|concedes|passes|wins|mills)\b)?/g, (whole, n, poss, _paren, verb) => {
        const pid = Number(n);
        const you = pid === state.view.you;
        if (!you && !state.view.opponents.some(o => o.id === pid))
            return whole;
        const head = poss ? (you ? "your" : "opp's") : (you ? "you" : "opp");
        let rest = whole.slice(1 + n.length + (poss ? 2 : 0));
        if (verb && you && !poss)
            rest = rest.slice(0, rest.length - verb.length) + VERB_BASE[verb];
        return head + rest;
    });
}
/**
 * A line the engine or the runner wrote, as the page shows it: in the
 * page's words for the players, and without the `(#id)`s the board cannot
 * be matched against. #694 stripped ids from titles and rows, but the log
 * band and the log drawer — the two places the page shows the most engine
 * text — only rewrote the players (#714). One function for every engine
 * line the page quotes.
 */
export function engineLine(state, line) {
    return withoutIds(inOurWords(state, line));
}
const VERB_BASE = {
    keeps: "keep", mulligans: "mulligan", concedes: "concede", passes: "pass", wins: "win", mills: "mill",
};
/**
 * An object's name, with whose zone it is in when that zone is a graveyard
 * or exile: "Mountain (your graveyard)", "Grizzly Bears (opponent's
 * graveyard)". Two copies of one card in two graveyards are different
 * targets, and the bare name made them one row, twice — Purify the Grave
 * offered "Grizzly Bears" four times (#690; the CLI's #669 and the LLM
 * seat's #668 said whose already).
 */
export function nameWithZone(state, id) {
    const e = state.index.get(id);
    const name = nameOf(state, id);
    if (!e || (e.zone !== "graveyard" && e.zone !== "exile") || e.owner === null)
        return name;
    const whose = e.owner === state.view.you ? "your" : "opponent's";
    return `${name} (${whose} ${e.zone})`;
}
export function targetLabel(state, t) {
    if (t === null || t === undefined)
        return "nothing";
    if (t === "Illegal")
        return "(illegal)";
    if ("Object" in t)
        return nameWithZone(state, t.Object);
    if ("Player" in t)
        return playerLabel(state, t.Player);
    return JSON.stringify(t);
}
/** Whether a priority offer includes playing a land. */
export function offersLandPlay(actions) {
    return actions.some(a => typeof a === "object" && a !== null && "PlayLand" in a);
}
/**
 * How many things passing this priority turns down: spells, non-mana
 * abilities and land plays. What auto-pass declines when it is engaged
 * here, and so what it has to say it declined — the CLI's #296/#618.
 */
export function autoPassDeclines(actions) {
    return actions.filter(a => typeof a === "object" && a !== null
        && ("PlayLand" in a || "CastSpell" in a || "ActivateAbility" in a)).length;
}
/**
 * The turn auto-pass counts from when `f` is pressed at `view`: a stop at
 * "your next Main Phase 1" is one in a turn after this. Pressed at your own
 * untap, upkeep or draw step, that main phase is this turn's, so the count
 * starts a turn earlier — the CLI's `before_our_main` (#45), which the page
 * never had: `f` at your upkeep passed the whole turn (#753).
 */
export function autoPassSince(view) {
    const beforeOurMain = view.active_player === view.you
        && (view.step === "Untap" || view.step === "Upkeep" || view.step === "Draw");
    return beforeOurMain ? view.turn_number - 1 : view.turn_number;
}
/**
 * Why auto-pass, engaged since turn `sinceTurn`, stops at this priority
 * offer, or null to pass it. `asked` is whether the page has something
 * other than a plain pass to put to the player.
 *
 * The opponent's attack is a stop, as it is for the CLI (#295) and the LLM
 * seat: a player with no blocker is never shown the block prompt, so
 * without it the page passed every window of the combat with an instant
 * in hand (#752).
 */
export function autoPassStop(view, actions, sinceTurn, asked) {
    if (asked)
        return "you are asked something.";
    // A land drop is never auto-passed, whatever the phase: once a turn and
    // free, it is always worth stopping for — the CLI's #39 (#691).
    if (offersLandPlay(actions))
        return "you have a land to play.";
    const ourTurn = view.active_player === view.you;
    const reached = view.turn_number > sinceTurn;
    if (ourTurn && view.step === "PrecombatMain" && reached)
        return "your main phase.";
    // On your own turn, once the turn it passes towards is here, a spell or an
    // ability on offer is a stop wherever it is — the CLI's MeaningfulAction:
    // the page passed your upkeep and draw with an instant castable (#758).
    if (ourTurn && reached && actions.some(a => typeof a === "object" && a !== null
        && ("CastSpell" in a || "ActivateAbility" in a)))
        return "you have a spell or ability to use.";
    // Something on the stack is a stop when there is an answer to it, not
    // just a pass, a concede or a mana ability — the CLI's StackResponse; any
    // entry used to stop it, and `f` pressed over one engaged and passed the
    // response window the CLI refuses to pass (#758).
    if (view.stack.length > 0 && actions.some(a => a !== "PassPriority" && a !== "Concede"
        && !(typeof a === "object" && a !== null && "ActivateManaAbility" in a)))
        return "something is on the stack.";
    if (!ourTurn && view.step === "DeclareAttackers"
        && view.battlefield.some(p => p.controller !== view.you && p.attacking))
        return "attackers declared against you.";
    // Your own postcombat main, for removal on what combat damaged: the CLI's
    // YourPostcombatMain and the LLM seat's pass-until both keep it, and `f` at
    // your Main Phase 1 passed it and the whole of the opponent's turn (#758).
    if (ourTurn && view.step === "PostcombatMain")
        return "your postcombat main phase.";
    return null;
}
/** The notice for engaging (`stillOn`) or ending auto-pass, as the CLI words it. */
export function autoPassNotice(declined, stillOn) {
    const what = (n) => `${n} spell${n === 1 ? "" : "s"}/abilit${n === 1 ? "y" : "ies"}/land play${n === 1 ? "" : "s"}`;
    if (stillOn && declined === 0)
        return "Auto-pass on — passing to your next Main Phase 1. Press f again to turn it off.";
    if (stillOn)
        return `Auto-pass on — it declined ${what(declined)} at that prompt, and passes to your next Main Phase 1. Press f again to turn it off.`;
    if (declined === 0)
        return null;
    return `Auto-pass has stopped. When it was turned on it declined ${what(declined)}.`;
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
/** What a cast row says about the cost it will pay, or "" when it pays the
 *  printed one.
 *
 *  The page's copy of `cast_cost_note` in `mtg-player/src/lib.rs`. A
 *  flashback row used to say only "Flashback", so two flashback costs on one
 *  graveyard card (CR 702.33 allows several instances at once; Past in Flames
 *  grants one) hung two identical verbs off the card with nothing to choose
 *  between (issue #611). */
export function costNote(cs) {
    const alt = cs.alternative_cost;
    if (!alt)
        return "";
    const amount = costText(alt) || "{0}";
    if (cs.is_flashback)
        return `flashback cost ${amount}`;
    if (!alt.symbols || alt.symbols.length === 0)
        return "without paying its mana cost";
    return `alternative cost ${amount}`;
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
    if (a === "Forfeit")
        return "Forfeit (harness)";
    if ("PlayLand" in a)
        return `Play ${nameOf(state, a.PlayLand.object_id)}`;
    if ("CastSpell" in a) {
        const v = a.CastSpell;
        // The amount, not just that there is one: two flashback costs on one
        // card are two different casts (#611).
        const alt = v.alternative_cost ? ` (cost ${costText(v.alternative_cost) || "{0}"})` : "";
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
        return withoutIds(c.ChosenIndex[1]);
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
    return { mode, title: withoutIds(title), hint: "", buttons: [], marked: [] };
}
/**
 * Engine text with its object ids taken out: "Devil's Play (#34) targeting
 * Grizzly Bears (#62)" reads "Devil's Play targeting Grizzly Bears", and a
 * trigger row's "[source 2/2, #42]" reads "[source 2/2]".
 *
 * The engine writes `(#id)` so the CLI and the LLM seat, which print ids
 * beside every permanent, can tell copies apart. The page draws no id
 * anywhere — four Bears are one "x4" stack — so on the page an id names
 * nothing a person can find (#694, the page's half of #634).
 */
export function withoutIds(s) {
    return s.replace(/ \(#\d+\)/g, "").replace(/,\s*#\d+(?=\])/g, "").replace(/ #\d+\b/g, "");
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
    // The zones the off-board options are in. One overlay can show one of
    // them, so it opens only when there is one: it used to open the first
    // option's owner's graveyard, so a target in the opponent's was offered
    // under a panel titled "You: graveyard" (#690).
    const zones = new Map();
    for (const [key, t] of ui.options ?? []) {
        const id = key[0] === "o" ? Number(key.slice(1)) : null;
        const e = id === null ? null : state.index.get(id);
        if (key[0] === "p" || (e && ON_BOARD.has(e.zone)))
            continue;
        if (e && (e.zone === "graveyard" || e.zone === "exile") && e.owner !== null)
            zones.set(`${e.zone}:${e.owner}`, { zone: e.zone, pid: e.owner });
        const label = id === null ? targetLabel(state, t) : nameWithZone(state, id);
        rows.push({ label, run: () => run(key), key, cardName: e ? e.obj.name : null });
    }
    if (zones.size === 1)
        state.overlay = [...zones.values()][0];
    return rows;
}
/**
 * Decide how the pending decision is answered. Sets `state.ui`.
 * `send(action)` answers; `state.notice` shows text in the panel.
 */
/**
 * The widget for the decision in hand, with a way to concede on it.
 *
 * A person may concede at any time (CR 104.3a), and the engine accepts it at
 * every decision. The button used to exist only on the priority menu, so the
 * mulligan, both combat declarations and every mid-resolution question had
 * no way to end the game but to answer them first (#676). Every widget gets
 * it here, once; "No" comes back through this function and restores the
 * question as it was.
 */
export function beginDecision(state, send) {
    const ui = beginDecisionWidget(state, send);
    if (!ui.buttons.some(b => b.label === "Concede"))
        ui.buttons.push(concedeButton(state, send));
    return ui;
}
function concedeButton(state, send) {
    return { label: "Concede", run: () => {
            beginList(state, newUi("list", "Concede the game?"), [{ label: "Yes, concede", run: () => send("Concede") }, { label: "No", run: () => beginDecision(state, send) }], "Concede the game?", send, false, true);
        } };
}
function beginDecisionWidget(state, send) {
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
        const desc = withoutIds(rp.description || legal.context || kind);
        const ids = (rp.options ?? []);
        switch (kind) {
            case "ChooseXFunding": return beginNumber(state, ui, rp.options, desc, send);
            case "AssignCombatDamage": return beginDamageAmount(state, ui, rp, desc, send);
            case "ChooseExileFromGraveyard":
                return beginMark(state, ui, { title: desc, options: ids, min: rp.min ?? 0, max: rp.max ?? ids.length,
                    onConfirm: (chosen) => send(resolve({ ChosenExileSet: chosen })),
                    onCancel: () => send(resolve("CancelCast")), cancelLabel: "Cancel cast" });
            case "ChooseObjectSet": {
                // The creatures an activation's cost taps (#670) are a cost still
                // being assembled: nothing is paid, so it can be backed out of. An
                // effect resolving cannot.
                const isCost = typeof rp.effect === "object" && rp.effect !== null && "PayActivationTaps" in rp.effect;
                return beginMark(state, ui, { title: desc, options: ids, min: rp.min ?? 0, max: rp.max ?? ids.length,
                    onConfirm: (chosen) => send(resolve({ ChosenObjectSet: chosen })),
                    ...(isCost ? { onCancel: () => send(resolve("CancelCast")), cancelLabel: "Cancel activation" } : {}) });
            }
            case "ChooseTargetSet": {
                const targets = (rp.options ?? []);
                return beginMark(state, ui, { title: desc, options: targets, min: rp.min ?? 0, max: rp.max ?? targets.length,
                    onConfirm: (chosen) => send(resolve({ ChosenTargetSet: chosen })),
                    onCancel: () => send(resolve("CancelCast")), cancelLabel: "Cancel cast" });
            }
            case "DividePermanentsIntoPiles": {
                const perms = rp.permanents ?? [];
                return beginMark(state, ui, { title: desc + " (mark pile A)", options: perms, min: 0, max: perms.length,
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
        const verb = cs.is_flashback ? "Flashback" : cs.from_graveyard ? "Cast from graveyard" : "Cast";
        const notes = [costNote(cs), cs.additional_cost_label || ""].filter(n => n);
        const extra = notes.length ? ` (${notes.join(", ")})` : "";
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
        if (verbs.has(card.object_id))
            continue;
        const twin = hand.find(c => c.object_id !== card.object_id && c.name === card.name && verbs.has(c.object_id));
        if (twin)
            verbs.set(card.object_id, verbs.get(twin.object_id));
    }
    ui.hint = hasPass ? "Click a card for what it can do. Enter passes." : "Choose an action.";
    if (hasPass)
        ui.buttons.push({ label: "Pass", primary: true, run: () => send("PassPriority") });
    if (actions.includes("Concede"))
        ui.buttons.push(concedeButton(state, send));
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
    ui.title = withoutIds(title);
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
    ui.title = withoutIds(title);
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
    ui.title = withoutIds(title);
    const order = (rp.options ?? []).map((label, i) => ({ label: withoutIds(label), i }));
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
/** A typed token as a refusal may echo it: control characters out, clipped.
 *  The terminal's `quote_input` (#282, #283) — a 400-column paste in a
 *  notice is a notice nobody can read. */
function clipToken(typed) {
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
export function tapReachability(groups, limit) {
    const width = limit + 1;
    const rows = new Array(groups.length + 1);
    const last = new Array(width).fill(false);
    last[0] = true;
    rows[groups.length] = last;
    for (let i = groups.length - 1; i >= 0; i--) {
        const prev = rows[i + 1];
        const q = groups[i].mana_per_tap;
        if (!q) {
            rows[i] = prev.slice();
            continue;
        }
        const maxTaps = groups[i].source_ids.length;
        const next = new Array(width).fill(false);
        for (let r = 0; r < Math.min(q, width); r++) {
            // Steps of `q` back to the nearest reachable sum; the nearest is the
            // criterion, since everything behind it is further still.
            let tapsBack = -1;
            for (let s = r; s < width; s += q) {
                tapsBack = prev[s] ? 0 : (tapsBack < 0 ? -1 : tapsBack + 1);
                if (tapsBack >= 0 && tapsBack <= maxTaps)
                    next[s] = true;
            }
        }
        rows[i] = next;
    }
    return rows;
}
const manaForX = (opts, x) => Math.max(0, x - (opts.x_discount || 0));
const poolTotal = (opts) => Object.values(opts.pool || {}).reduce((a, n) => a + (n || 0), 0);
/** Every X the player may announce that some allocation funds exactly.
 *  Not `0..=max_x + x_discount`: a lone Sol Ring pays 0 and 2, and nothing
 *  between (#595). Always contains 0. */
export function fundableXValues(opts) {
    const maxX = (opts.max_x || 0) + (opts.x_discount || 0);
    const reachable = tapReachability(opts.groups || [], opts.max_x || 0)[0];
    const pool = poolTotal(opts);
    const largestAtOrBelow = new Array(reachable.length).fill(-1);
    let best = -1;
    for (let s = 0; s < reachable.length; s++) {
        if (reachable[s])
            best = s;
        largestAtOrBelow[s] = best;
    }
    const out = [];
    for (let x = 0; x <= maxX; x++) {
        const mana = manaForX(opts, x);
        const s = mana < largestAtOrBelow.length ? largestAtOrBelow[mana] : -1;
        if (s >= 0 && mana - s <= pool)
            out.push(x);
    }
    return out;
}
/** The response that funds `x`, and what could not be funded.
 *  Exact whenever the board can pay it — the page used to take whole
 *  activations greedily in category order, so one Mountain and one Sol Ring
 *  funded 1 for a player who typed 2 and left the Sol Ring untapped
 *  (#593) — with the pool drained before taps and lands before rocks before
 *  dorks as the tie-break among the exact allocations. */
export function allocateForX(opts, x) {
    const response = { pool: {}, taps: {} };
    const target = manaForX(opts, x);
    const groups = opts.groups || [];
    const rows = tapReachability(groups, target);
    const reachable = rows[0];
    const pool = poolTotal(opts);
    const floor = Math.max(0, target - pool);
    let tapSum = -1;
    for (let s = floor; s <= target; s++)
        if (reachable[s]) {
            tapSum = s;
            break;
        }
    if (tapSum < 0) {
        // Nothing in the window: spend the whole pool behind the largest tap
        // sum there is and report the rest.
        tapSum = 0;
        for (let s = floor - 1; s >= 0; s--)
            if (reachable[s]) {
                tapSum = s;
                break;
            }
    }
    let remaining = target - tapSum;
    const poolSorted = Object.entries(opts.pool || {})
        .filter(([, n]) => (n || 0) > 0)
        .sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
    for (const [mt, avail] of poolSorted) {
        if (remaining === 0)
            break;
        const take = Math.min(avail, remaining);
        if (take > 0) {
            response.pool[mt] = take;
            remaining -= take;
        }
    }
    let left = tapSum;
    for (let i = 0; i < groups.length; i++) {
        if (left === 0)
            break;
        const g = groups[i];
        if (!g.mana_per_tap)
            continue;
        const ceiling = Math.min(Math.floor(left / g.mana_per_tap), g.source_ids.length);
        let taps = 0;
        for (let t = ceiling; t >= 0; t--) {
            if (rows[i + 1][left - t * g.mana_per_tap]) {
                taps = t;
                break;
            }
        }
        if (taps > 0) {
            const amount = taps * g.mana_per_tap;
            response.taps[g.name] = amount;
            left -= amount;
        }
    }
    return { response, shortfall: remaining };
}
/** The payable set as one line, runs collapsed, capped, both ends kept —
 *  the terminal's `describe_fundable_x` (#595). */
export function describeFundableX(values) {
    const MAX = 44;
    const runs = [];
    for (let i = 0; i < values.length; i++) {
        const start = values[i];
        let end = start;
        while (i + 1 < values.length && values[i + 1] === end + 1)
            end = values[++i];
        runs.push(start === end ? `${start}` : `${start}-${end}`);
    }
    const full = runs.join(", ");
    if (full.length <= MAX || runs.length < 3)
        return full;
    const last = runs[runs.length - 1];
    const budget = Math.max(0, MAX - (last.length + 5));
    const kept = [];
    let cols = 0;
    for (const run of runs.slice(0, -1)) {
        const next = cols + run.length + 2;
        if (next > budget)
            break;
        cols = next;
        kept.push(run);
    }
    return `${kept.join(", ")}, \u2026 ${last}`;
}
/** X: one number, distributed over the pool and the tap groups the way the CLI does. */
function beginNumber(state, ui, opts, title, send) {
    const maxX = (opts.max_x || 0) + (opts.x_discount || 0);
    ui.mode = "number";
    ui.title = withoutIds(title);
    ui.max = maxX;
    ui.value = "";
    const summary = [];
    ui.summary = summary;
    const pool = Object.entries(opts.pool || {}).filter(([, n]) => (n ?? 0) > 0).map(([k, n]) => `${n} ${k}`).join(", ");
    if (pool)
        summary.push(`Pool: ${pool}`);
    for (const g of opts.groups || [])
        summary.push(`${g.name} x${g.source_ids.length} (${g.mana_per_tap}/tap)`);
    // Which values of X the sources can actually pay. The page used to state
    // `0-N`, accept every integer in it, fund a smaller X and send it with
    // `state.notice` left null — a card spent, the popover closed, and
    // nothing on the page ever mentioning it (#594). The terminal's half of
    // the same silence is #595.
    const fundable = fundableXValues(opts);
    const payable = new Set(fundable);
    if (fundable.length !== maxX + 1)
        summary.push(`Payable X: ${describeFundableX(fundable)}`);
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
        const refuse = (msg) => { state.notice = msg; ui.value = ""; };
        if (typed === "") {
            state.notice = "Enter a value for X.";
            return;
        }
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
/**
 * How much of an attacker's combat damage goes to one blocker (CR 510.1c-d,
 * #637): one number, from lethal (`min`) to everything left (`max`). The
 * answer is the prompt's index, `amount - min`. Enter at an empty box is
 * lethal — the division the engine makes when nobody is asked, and what
 * the terminal's Enter does — and the hint says so.
 */
export function beginDamageAmount(state, ui, rp, title, send) {
    const min = rp.min ?? 0;
    const max = rp.max ?? min;
    const labels = (rp.options ?? []);
    const choose = (amount) => {
        const i = amount - min;
        send(resolve({ ChosenIndex: [i, labels[i] ?? String(amount)] }));
    };
    ui.mode = "number";
    // The engine's description runs to ~150 characters and says where the
    // rest goes only at its end, which the modal's two title lines and the
    // panel both cut off; its option labels were never drawn, and its "p1"
    // was never put in the page's words (#720). The title is the page's own
    // short one; the description's tail and every option, each saying where
    // the rest goes, are the summary.
    const named = (k) => typeof rp[k] === "number" ? nameOf(state, rp[k]) : null;
    const attacker = named("attacker"), blocker = named("blocker");
    ui.title = attacker && blocker ? `Damage from ${attacker} to ${blocker}` : engineLine(state, title);
    ui.min = min;
    ui.max = max;
    ui.value = "";
    ui.placeholder = `${min}-${max}`;
    const tail = title.includes("? ") ? title.slice(title.indexOf("? ") + 2) : "";
    ui.summary = [...(tail ? [engineLine(state, tail)] : []),
        ...labels.map(l => engineLine(state, l))];
    ui.hint = `Type an amount (${min}-${max}) and press Enter. Enter alone assigns ${min}, lethal.`;
    ui.submit = () => {
        // The terminal's parser: an optional `+` and ASCII digits (#561).
        const typed = (ui.value ?? "").trim();
        const refuse = (msg) => { state.notice = msg; ui.value = ""; };
        if (typed === "") {
            choose(min);
            return;
        }
        const a = /^\+?[0-9]+$/.test(typed) ? Number(typed) : NaN;
        if (!Number.isInteger(a)) {
            refuse(`'${clipToken(typed)}' is not a number — enter an amount from ${min} to ${max}.`);
            return;
        }
        if (a < min) {
            refuse(`${a} is less than lethal — at least ${min} must go to this blocker (CR 510.1c).`);
            return;
        }
        if (a > max) {
            refuse(`${a} is more than is left — at most ${max}.`);
            return;
        }
        choose(a);
    };
    ui.buttons.push({ label: "Confirm", primary: true, run: ui.submit });
    ui.buttons.push({ label: `Lethal (${min})`, run: () => choose(min) });
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
    // The CLI's `all`. Without it a wide board was one click per creature —
    // 649 on a flood board, 13 after one Army of the Damned (#643). Marks
    // every creature not already attacking, at the player; those already
    // marked keep the defender they were given.
    ui.buttons.push({ label: "All", run: () => {
            for (const id of c.eligible) {
                const key = `o${id}`;
                if (!ui.marked.includes(key)) {
                    ui.marked.push(key);
                    attackTarget.set(key, 0);
                }
            }
        } });
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

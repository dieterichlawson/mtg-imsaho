// Drawing the board at 640x360, and saying what was drawn where.
//
// Every call to `render` returns the list of hit rectangles it drew, with
// what each one is and what a click on it does. Input never looks at the
// view; it looks at the last frame.
import { drawArt, frameColor, darker, uiImage } from "./assets.js";
import { nameOf, targetLabel, playerLabel, inOurWords } from "./prompts.js";
export const W = 640, H = 360;
export const PANEL_X = 480;
const BOARD_W = PANEL_X;
const CARD = { w: 40, h: 52 }; // on the battlefield
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
/**
 * Paint one line, clipped to the room it has.
 *
 * Canvas `fillText` does not clip, so a string wider than the space it was
 * given is painted straight over whatever is to its right — and past the
 * canvas edge there is no canvas left, so those glyphs do not exist at all.
 * #532 put the rule in `wrap`, which covers every line that goes through
 * it. The inspector's two P/T lines do not, and a damaged 13/13 read
 * "13/13 10 dm" with the "g" painted at x=640..648 (issue #568).
 *
 * The rule lives here now rather than at the fifteen call sites, so it
 * holds for the next one as well: nothing is ever painted off the canvas,
 * and a caller that knows the pane it is drawing into says so with `maxW`
 * and is clipped to that instead.
 */
export function text(ctx, s, x, y, { font = "8px Silkscreen", color = "#e8e0d0", align = "left", maxW } = {}) {
    ctx.font = font;
    ctx.fillStyle = color;
    ctx.textAlign = align;
    ctx.textBaseline = "top";
    const px = Math.round(x);
    // How much room the line has before it leaves the canvas, given where its
    // anchor sits and which way the text grows from there.
    const toEdge = align === "center" ? 2 * Math.min(px, ctx.canvas.width - px)
        : align === "right" ? px
            : ctx.canvas.width - px;
    const room = Math.max(0, Math.min(maxW === undefined ? Infinity : maxW, toEdge));
    ctx.fillText(clip(ctx, s, room, font), px, Math.round(y));
    ctx.textAlign = "left";
}
export function wrap(ctx, s, maxW, font = "8px Silkscreen") {
    ctx.font = font;
    const lines = [];
    for (const para of String(s).split("\n")) {
        let line = "";
        for (const word of para.split(" ")) {
            const t = line ? line + " " + word : word;
            if (ctx.measureText(t).width <= maxW || !line)
                line = t;
            else {
                lines.push(line);
                line = word;
            }
        }
        lines.push(line);
    }
    // A word longer than the pane has nowhere to break, and `!line` above
    // keeps it on a line of its own at full width. Canvas `fillText` does not
    // clip, so that line was painted straight over whatever was to its right
    // — for the inspector, which is flush with the canvas edge, over the edge
    // itself: "Ghoulcaller's" lost its "'s" with no ellipsis, on 34 of the
    // 279 cards in the set. Clipping here rather than at each of the fifteen
    // call sites is what makes the rule hold for the next one (issue #532).
    return lines.map(l => clip(ctx, l, maxW, font));
}
/**
 * [`wrap`], capped at `maxLines`, with the cut marked.
 *
 * A bare `.slice(0, n)` over wrapped lines drops the rest silently: "Curse
 * of the Bloody Tome" wraps to three lines in a hand card and rendered as
 * "CURSE OF / THE BLOODY", with the word "Tome" simply gone. Clipping the
 * surviving lines does not help, because each of them fits. The ellipsis
 * has to go on the last line that is kept (issue #532).
 */
export function wrapCapped(ctx, s, maxW, font, maxLines) {
    const lines = wrap(ctx, s, maxW, font);
    if (lines.length <= maxLines)
        return lines;
    const kept = lines.slice(0, maxLines);
    kept[maxLines - 1] = clip(ctx, kept[maxLines - 1] + "…", maxW, font);
    return kept;
}
function clip(ctx, s, maxW, font = "8px Silkscreen") {
    ctx.font = font;
    if (ctx.measureText(s).width <= maxW)
        return s;
    let t = s;
    while (t.length > 1 && ctx.measureText(t + "…").width > maxW)
        t = t.slice(0, -1);
    return t + "…";
}
function panel(ctx, x, y, w, h, fill = "#1a1620", stroke = "#5a4a6a") {
    ctx.fillStyle = fill;
    ctx.fillRect(x, y, w, h);
    ctx.strokeStyle = stroke;
    ctx.lineWidth = 1;
    ctx.strokeRect(x + 0.5, y + 0.5, w - 1, h - 1);
}
function button(ctx, hits, x, y, w, h, label, run, { primary = false, enabled = true } = {}) {
    panel(ctx, x, y, w, h, enabled ? (primary ? "#4a3a5a" : "#2a2430") : "#1c1a20", primary ? "#c9a84a" : "#6a5a7a");
    text(ctx, label, x + w / 2, y + (h - 8) / 2, { align: "center", color: enabled ? "#f0e8d8" : "#7a7280", font: "8px Silkscreen" });
    hits.push({ x, y, w, h, kind: "button", label, onClick: enabled ? run : null });
}
/** Draw `img` stretched to w×h keeping its `slice`-pixel border crisp. */
function nineSlice(ctx, img, x, y, w, h, slice) {
    const iw = img.width, ih = img.height, s = slice;
    const cols = [[0, s, x, s], [s, iw - 2 * s, x + s, w - 2 * s], [iw - s, s, x + w - s, s]];
    const rows = [[0, s, y, s], [s, ih - 2 * s, y + s, h - 2 * s], [ih - s, s, y + h - s, s]];
    for (const [sx, sw, dx, dw] of cols)
        for (const [sy, sh, dy, dh] of rows) {
            if (sw > 0 && sh > 0 && dw > 0 && dh > 0)
                ctx.drawImage(img, sx, sy, sw, sh, dx, dy, dw, dh);
        }
}
/** A textured panel where the art exists, a flat one where it does not. */
function texturedPanel(ctx, x, y, w, h, fallback = "#171320", stroke = "#4a3a5a") {
    const img = uiImage("panel");
    if (img) {
        // The riveted border is the point; the mottled middle is not a
        // background text can be read on.
        nineSlice(ctx, img, x, y, w, h, 9);
        ctx.fillStyle = "rgba(20,17,26,0.92)";
        ctx.fillRect(x + 7, y + 7, w - 14, h - 14);
    }
    else
        panel(ctx, x, y, w, h, fallback, stroke);
}
/** 0..1, slowly, so a highlight breathes rather than blinks. */
function pulse() { return (Math.sin(Date.now() / 220) + 1) / 2; }
// --------------------------------------------------------------- cards
function manaDots(ctx, cost, x, y) {
    if (!cost || !cost.symbols)
        return x;
    const col = { White: "#f0e6c0", Blue: "#5088e0", Black: "#403050", Red: "#e05040", Green: "#50a050" };
    for (const s of cost.symbols) {
        let fill = "#9a9aa0", label = "";
        if (s === "X")
            label = "X";
        else if (typeof s === "object" && "Colored" in s)
            fill = col[s.Colored] || fill;
        else if (typeof s === "object" && "Generic" in s)
            label = String(s.Generic);
        ctx.fillStyle = fill;
        ctx.beginPath();
        ctx.arc(x + 3.5, y + 3.5, 3.5, 0, Math.PI * 2);
        ctx.fill();
        if (label)
            text(ctx, label, x + 3.5, y, { font: "7px Silkscreen", color: "#101010", align: "center" });
        x += 8;
    }
    return x;
}
/** The band's right-hand text block: where it starts and how wide it is. */
export const BAND_X = 330;
export const BAND_W = PANEL_X - BAND_X - 4;
const STEP_WORDS = { Untap: "untap", Upkeep: "upkeep", Draw: "draw", PrecombatMain: "main phase 1",
    BeginCombat: "begin combat", DeclareAttackers: "declare attackers", DeclareBlockers: "declare blockers",
    CombatDamage: "combat damage", EndCombat: "end of combat", PostcombatMain: "main phase 2", EndStep: "end step",
    Cleanup: "cleanup" };
/**
 * The band's turn/step line, clipped to the block it is drawn in.
 *
 * It was the one string on the board drawn through neither `clip` nor
 * `wrap`, while the two log lines beneath it went through `clip`. Canvas
 * `fillText` does not clip, so "OPPONENT'S TURN · declare attackers" — 161px
 * in a 150px block — was painted past the board/panel seam and then covered
 * by the panel fill: amputated mid-glyph, with no ellipsis, at exactly the
 * two steps where this line is what says why the game has stopped and is
 * asking (issue #522). Named rather than inline so the sweep in the tests
 * measures what is drawn.
 */
export function bandTurnLine(ctx, mine, step) {
    return clip(ctx, `${mine ? "YOUR TURN" : "OPPONENT'S TURN"} · ${STEP_WORDS[step]}`, BAND_W, "7px Silkscreen");
}
/** Status marks for a permanent: tapped, attacking, blocking, sick, counters, damage. */
function permBadges(p, state) {
    const b = [];
    if (p.attacking)
        b.push({ t: "ATK", c: "#e07040" });
    if (p.blocking && p.blocking.length)
        b.push({ t: "BLK", c: "#60a0e0" });
    if (p.summoning_sick && p.card_types.includes("Creature"))
        b.push({ t: "z", c: "#8080a0" });
    const c = p.counters || {};
    if (c.PlusOnePlusOne)
        b.push({ t: `+${c.PlusOnePlusOne}`, c: "#80e080" });
    if (c.MinusOneMinusOne)
        b.push({ t: `-${c.MinusOneMinusOne}`, c: "#e08080" });
    if (c.Loyalty)
        b.push({ t: `L${c.Loyalty}`, c: "#e0d080" });
    if (p.damage_marked)
        b.push({ t: `${p.damage_marked}d`, c: "#e05050" });
    if (p.regeneration_shields)
        b.push({ t: "R", c: "#a0e0a0" });
    if (p.attached_to !== null && p.attached_to !== undefined)
        b.push({ t: "→" + nameOf(state, p.attached_to).slice(0, 4), c: "#c0a0e0" });
    return b;
}
function highlightFor(state, key) {
    const ui = state.ui;
    if (!ui)
        return null;
    const isOption = ui.isOption ? ui.isOption(key) : false;
    if (ui.mode === "pick" && isOption)
        return "option";
    if ((ui.mode === "mark" || ui.mode === "attackers") && isOption)
        return ui.marked.includes(key) ? "marked" : "option";
    if (ui.mode === "blockers") {
        if (isOption)
            return ui.assignments.has(key) ? "marked" : ui.selectedBlocker === key ? "selected" : "option";
        if (ui.attackers.has(key))
            return ui.canAttackerTake(key) ? "target" : null;
    }
    if (ui.mode === "menu" && ui.verbs && ui.verbs.has(Number(key.slice(1))))
        return "verb";
    return null;
}
const HL = { option: "#e0c040", marked: "#60e060", selected: "#60c0ff", target: "#ff6060", verb: "#c8b060" };
const HL_DIM = { option: "#a08830", marked: "#40a040", selected: "#4080b0", target: "#b04040", verb: "#8a7a50" };
function highlightColor(hl) { return pulse() > 0.5 ? HL[hl] : HL_DIM[hl]; }
function clickFor(state, key) {
    const ui = state.ui;
    if (!ui)
        return null;
    const id = Number(key.slice(1));
    const isOption = ui.isOption ? ui.isOption(key) : false;
    switch (ui.mode) {
        case "pick": return isOption ? () => ui.onPick(key) : null;
        case "mark": return isOption ? () => ui.toggle(key) : null;
        case "attackers": return isOption ? () => ui.toggle(key) : null;
        case "blockers":
            if (isOption)
                return () => ui.clickBlocker(key);
            if (ui.attackers.has(key))
                return () => ui.clickAttacker(key);
            return null;
        case "menu":
            if (ui.verbs && ui.verbs.has(id))
                return (hit) => { state.popover = { id, items: ui.verbs.get(id), x: hit.x, y: hit.y }; };
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
    const hovered = state.hover !== null && state.hover.key === key;
    ctx.fillStyle = hl ? highlightColor(hl) : hovered ? "#d0c8e0" : darker(c, 0.8);
    ctx.fillRect(x - 1, y - 1, CARD.w + 2, CARD.h + 2);
    ctx.fillStyle = darker(c, 0.45);
    ctx.fillRect(x, y, CARD.w, CARD.h);
    drawArt(ctx, x + 4, y + 3, ART_S.w, ART_S.h, p.name, colors, p.is_token);
    if (p.is_token) {
        ctx.fillStyle = "rgba(255,255,255,0.15)";
        ctx.fillRect(x, y, CARD.w, 2);
    }
    // P/T or a type glyph on the right; keyword marks in whatever is left.
    let rightW = 0;
    if (p.effective_power !== null && p.effective_power !== undefined) {
        const pt = `${p.effective_power}/${p.effective_toughness}`;
        const boosted = p.effective_power !== p.power || p.effective_toughness !== p.toughness;
        text(ctx, pt, x + CARD.w - 3, y + CARD.h - 10, { align: "right", color: boosted ? "#a0e0ff" : "#f0e8d8", font: "8px PressStart" });
        ctx.font = "8px PressStart";
        rightW = ctx.measureText(pt).width;
    }
    else {
        const t = p.card_types.includes("Land") ? "L" : p.card_types.includes("Artifact") ? "A" : p.card_types.includes("Planeswalker") ? "PW" : "E";
        text(ctx, t, x + CARD.w - 3, y + CARD.h - 10, { align: "right", color: "#a0a0b0" });
        ctx.font = "8px Silkscreen";
        rightW = ctx.measureText(t).width;
    }
    const kw = (p.keywords || []).map(k => KEYWORD_SHORT[k] || k.slice(0, 3)).slice(0, 2).join(" ");
    const kwRoom = CARD.w - 8 - rightW;
    if (kw && kwRoom >= 10)
        text(ctx, clip(ctx, kw, kwRoom, "7px Silkscreen"), x + 3, y + CARD.h - 10, { color: "#c0c8d8", font: "7px Silkscreen" });
    // Badges strip below the art.
    let bx = x + 3;
    for (const b of permBadges(p, state).slice(0, 3)) {
        ctx.font = "7px Silkscreen";
        const w = ctx.measureText(b.t).width + 2;
        ctx.fillStyle = "rgba(0,0,0,0.6)";
        ctx.fillRect(bx, y + 29, w, 9);
        text(ctx, b.t, bx + 1, y + 30, { color: b.c, font: "7px Silkscreen" });
        bx += w + 1;
    }
    if (group && group.length > 1) {
        ctx.fillStyle = "#101010";
        ctx.fillRect(x + CARD.w - 12, y - 3, 14, 10);
        text(ctx, `x${group.length}`, x + CARD.w - 5, y - 2, { align: "center", font: "7px Silkscreen", color: "#ffe080" });
    }
    if (state.ui && state.ui.badge) {
        const b = state.ui.badge(key);
        if (b) {
            ctx.fillStyle = "#101010";
            ctx.fillRect(x, y + 12, CARD.w, 10);
            text(ctx, b, x + CARD.w / 2, y + 13, { align: "center", color: "#ffe080", font: "7px Silkscreen" });
        }
    }
    if (p.tapped) {
        ctx.fillStyle = "rgba(0,0,0,0.45)";
        ctx.fillRect(x, y, CARD.w, CARD.h);
        ctx.fillStyle = "#101010";
        ctx.fillRect(x, y, 11, 9);
        text(ctx, "↻", x + 5, y + 1, { align: "center", font: "7px Silkscreen", color: "#e0d0a0" });
    }
    const verbs = state.ui && state.ui.mode === "menu" && state.ui.verbs ? state.ui.verbs.get(p.object_id) : undefined;
    hits.push({ x: x - 1, y: y - 1, w: CARD.w + 2, h: CARD.h + 2, kind: "perm", key, id: p.object_id, group, verbs, onClick: clickFor(state, key) });
}
function drawHandCard(ctx, hits, state, card, x, y, raised) {
    const key = `o${card.object_id}`;
    const colors = colorsOfCost(card.cost);
    const c = frameColor(colors);
    const hl = highlightFor(state, key);
    const hovered = state.hover !== null && state.hover.key === key;
    if (raised)
        y -= 10;
    ctx.fillStyle = hl ? highlightColor(hl) : hovered ? "#d0c8e0" : darker(c, 0.85);
    ctx.fillRect(x - 1, y - 1, HAND.w + 2, HAND.h + 2);
    ctx.fillStyle = darker(c, 0.4);
    ctx.fillRect(x, y, HAND.w, HAND.h);
    drawArt(ctx, x + 1, y + 3, ART_L.w, ART_L.h, card.name, colors, false);
    ctx.fillStyle = "rgba(0,0,0,0.35)";
    ctx.fillRect(x + 1, y + 52, HAND.w - 2, HAND.h - 55);
    // Cards overlap to the right, so what matters sits on the left edge.
    const lines = wrapCapped(ctx, card.name, HAND.w - 6, "8px Silkscreen", 2);
    lines.forEach((l, i) => text(ctx, l, x + 3, y + 54 + i * 9, { color: "#f4ecdc" }));
    manaDots(ctx, card.cost, x + 3, y + 73);
    const types = card.card_types || [];
    if (card.power !== null && card.power !== undefined) {
        text(ctx, `${card.power}/${card.toughness}`, x + 3, y + 82, { font: "8px PressStart", color: "#f4ecdc" });
    }
    else {
        const word = types.includes("Land") ? "LAND" : types.includes("Instant") ? "INSTANT" : types.includes("Sorcery") ? "SORCERY"
            : types.includes("Planeswalker") ? "WALKER" : types.includes("Artifact") ? "ARTIFACT" : types.includes("Enchantment") ? "ENCHANT" : types.join("/");
        text(ctx, word, x + 3, y + 83, { color: "#b0b0c0", font: "7px Silkscreen" });
    }
    const verbs = state.ui && state.ui.mode === "menu" && state.ui.verbs ? state.ui.verbs.get(card.object_id) : undefined;
    if (verbs) {
        ctx.fillStyle = highlightColor("verb");
        ctx.fillRect(x + 2, y + 2, 6, 6);
        ctx.fillStyle = "#101010";
        ctx.fillRect(x + 3, y + 3, 4, 4);
        ctx.fillStyle = highlightColor("verb");
        ctx.fillRect(x + 4, y + 4, 2, 2);
    }
    hits.push({ x: x - 1, y: y - 1, w: HAND.w + 2, h: HAND.h + 2, kind: "hand", key, id: card.object_id, verbs, onClick: clickFor(state, key) });
}
export function colorsOfCost(cost) {
    const out = [];
    for (const s of (cost && cost.symbols) || []) {
        if (typeof s === "object" && "Colored" in s) {
            const c = s.Colored;
            if (!out.includes(c))
                out.push(c);
        }
    }
    return out;
}
// ---------------------------------------------------------------- zones
/** Lay `n` items across a row: shrink the stride until they fit. */
function rowLayout(n, cardW, x0, width, gap = 3) {
    if (n === 0)
        return { stride: cardW + gap, x0 };
    const stride = Math.max(10, Math.min(cardW + gap, Math.floor((width - cardW) / Math.max(1, n - 1))));
    return { stride, x0 };
}
/**
 * What makes two permanents the same row of the board, or `null` when this
 * one is its own card.
 *
 * The rule used to be "an untapped basic land with no counters", and
 * creatures never reached it at all — they were passed to `drawRow` one to
 * a group. So the one class of permanent that actually reaches large
 * counts, identical tokens, was the one class that never collapsed: 108
 * Zombies drew as 108 slivers 10px wide (issue #513).
 *
 * The key is everything the board can show about a permanent, so two cards
 * that share it are genuinely interchangeable to look at: anything with a
 * counter, damage, a shield, an attachment either way, a combat role or a
 * named card is its own card, and the rest group by what `drawPerm` and
 * `permBadges` would paint. That is the same rule the old one was a
 * special case of, so basic lands still stack exactly as before.
 */
function stackKey(p, attachedTo) {
    if (p.attached_to !== null && p.attached_to !== undefined)
        return null;
    if (attachedTo.has(p.object_id))
        return null;
    if (p.counters && Object.values(p.counters).some(n => n))
        return null;
    if (p.damage_marked || p.regeneration_shields)
        return null;
    if (p.attacking || (p.blocking && p.blocking.length) || (p.blocked_by && p.blocked_by.length))
        return null;
    if (p.named_card)
        return null;
    const isCreature = p.card_types.includes("Creature");
    return [
        p.name, p.tapped, p.is_token,
        // Summoning sickness is only painted on a creature, so it only tells
        // two permanents apart when they are creatures.
        isCreature ? p.summoning_sick : false,
        p.effective_power, p.effective_toughness, p.star_pt,
        (p.keywords || []).join(","), (p.protections || []).join(","), (p.restrictions || []).join(","),
        (p.card_types || []).join(","), (p.subtypes || []).join(","), (p.supertypes || []).join(","),
    ].join("|");
}
/**
 * What the page itself has pending on a permanent in the open prompt: an
 * attack mark and whom it attacks, a blocker assignment, the blocker being
 * placed, a mark in a set. None of it is in the view — the engine learns
 * of an attacker when the declaration is sent — so `stackKey` alone let
 * two Doomed Travelers stay one "x2" card after the first was marked: the
 * stack still showed that first one, the next click withdrew it, and the
 * second Traveler could not be reached at all (issue #573). A member that
 * that has any of it is drawn as its own card — the way `stackKey` treats
 * a declared attacker or blocker — so each copy can be marked and
 * withdrawn on its own, and the stack left behind offers the next one.
 */
function pendingKey(state, key) {
    const ui = state.ui;
    if (!ui)
        return "";
    const pending = ui.marked.includes(key)
        || (ui.assignments !== undefined && ui.assignments.has(key))
        || ui.selectedBlocker === key
        || (ui.badge !== undefined && ui.badge(key) !== null);
    return pending ? key : "";
}
/** Battlefield permanents of one controller, grouped into the two rows. */
function splitBoard(state, view, controller) {
    const mine = view.battlefield.filter(p => p.controller === controller);
    const attachedTo = new Set();
    for (const p of view.battlefield)
        if (p.attached_to !== null && p.attached_to !== undefined)
            attachedTo.add(p.attached_to);
    const group = (list) => {
        const out = [];
        const byKey = new Map();
        for (const p of list) {
            const base = stackKey(p, attachedTo);
            const k = base === null ? null : `${base}|${pendingKey(state, `o${p.object_id}`)}`;
            const g = k === null ? undefined : byKey.get(k);
            if (g)
                g.push(p);
            else {
                const ng = [p];
                if (k !== null)
                    byKey.set(k, ng);
                out.push(ng);
            }
        }
        return out;
    };
    const creatures = group(mine.filter(p => p.card_types.includes("Creature")));
    const groups = group(mine.filter(p => !p.card_types.includes("Creature")));
    // Non-lands first, then lands.
    groups.sort((a, b) => Number(a[0].card_types.includes("Land")) - Number(b[0].card_types.includes("Land")));
    return { creatures, groups };
}
function drawRow(ctx, hits, state, all, y) {
    const width = BOARD_W - 12;
    const { stride, x0 } = rowLayout(all.length, CARD.w, 6, width);
    // The stride bottoms out at 10px and nothing used to cap the row, so past
    // that point cards were simply drawn at ever-larger x: at 108 tokens the
    // last one reached x=1117 in a 640px frame, 60 of them painted and then
    // covered by the side panel — invisible, and still answering clicks,
    // because the hit rectangles went in all the same (issue #513).
    //
    // What fits, fits; the rest are counted in the last slot. Nothing is
    // drawn past the pane and nothing off the pane pushes a hit.
    const fits = Math.floor((width - CARD.w) / stride) + 1;
    const overflow = all.length > fits;
    const items = overflow ? all.slice(0, Math.max(0, fits - 1)) : all;
    const draw = (item, i) => {
        const group = item.length > 1 ? item.map(q => q.object_id) : null;
        // In a pick or mark, a stack whose members are options should offer
        // a member that IS an option, not merely its first card. Members
        // already marked or assigned are split out by `pendingKey`, so the
        // option this finds is one the next click can still act on.
        let shown = item[0];
        if (group && state.ui && state.ui.isOption) {
            const opt = item.find(q => state.ui.isOption(`o${q.object_id}`));
            if (opt)
                shown = opt;
        }
        drawPerm(ctx, hits, state, shown, x0 + i * stride, y, group);
    };
    // A crowded row overlaps; the hovered card is drawn last so it is the
    // one on top and can be read.
    const hoveredKey = state.hover ? state.hover.key : null;
    let hovered = -1;
    items.forEach((item, i) => {
        if (hoveredKey && item.some(q => `o${q.object_id}` === hoveredKey)) {
            hovered = i;
            return;
        }
        draw(item, i);
    });
    if (hovered >= 0)
        draw(items[hovered], hovered);
    if (overflow) {
        const hidden = all.slice(items.length).reduce((n, g) => n + g.length, 0);
        const mx = x0 + items.length * stride;
        panel(ctx, mx, y, Math.min(CARD.w, BOARD_W - 6 - mx), CARD.h, "#241d2c", "#6a5a7a");
        text(ctx, `+${hidden}`, mx + 3, y + CARD.h / 2 - 4, { font: "8px PressStart", color: "#ffe080" });
    }
}
function drawStrip(ctx, hits, state, y, pid, life, handSize, library, gy, exile, pool, isYou) {
    const key = `p${pid}`;
    const hl = highlightFor(state, key);
    ctx.fillStyle = hl ? HL[hl] : "#151218";
    ctx.fillRect(0, y, BOARD_W, 14);
    ctx.fillStyle = "#151218";
    ctx.fillRect(1, y + 1, BOARD_W - 2, 12);
    const heart = uiImage("life-heart");
    if (heart)
        ctx.drawImage(heart, 0, 0, heart.width, heart.height, 4, y + 1, 12, 12);
    const active = state.view.active_player === pid;
    const priority = state.view.priority_player === pid;
    text(ctx, String(life), 18, y + 3, { font: "8px PressStart", color: life <= 5 ? "#ff7060" : "#ffffff" });
    // Named with its seat number, so every `p0`/`p1` the engine's log and the
    // runner's game-over line use has a definition somewhere on screen. This
    // is #115's fix for the CLI's status bar, on the fourth surface — and the
    // surface where it matters most, because this viewer never sees the
    // runner's header line at all (issue #519).
    // Clipped to the space before the zone counts at x=150, like everything
    // else on the board.
    text(ctx, clip(ctx, `${isYou ? "You" : "Opponent"} (p${pid})${active ? " ★" : ""}${priority ? " ●" : ""}`, 98), 50, y + 3, { color: active ? "#ffe080" : "#c0b8c8" });
    const parts = [`Hand ${handSize}`, `Lib ${library}`, `GY ${gy}`];
    if (exile)
        parts.push(`Exile ${exile}`);
    const poolText = Object.entries((pool && pool.mana) || {}).filter(([, n]) => (n ?? 0) > 0).map(([k, n]) => `${n}${k[0]}`).join(" ");
    if (poolText)
        parts.push(`Pool ${poolText}`);
    let x = 150;
    for (const part of parts) {
        ctx.font = "8px Silkscreen";
        const w = ctx.measureText(part).width;
        const zone = part.startsWith("GY") ? "graveyard" : part.startsWith("Exile") ? "exile" : part.startsWith("Lib") ? "library" : null;
        text(ctx, part, x, y + 3, { color: zone ? "#b0c0e0" : "#c0b8c8" });
        if (zone)
            hits.push({ x: x - 2, y, w: w + 4, h: 14, kind: "zone", zone, pid, onClick: () => {
                    state.overlay = state.overlay && state.overlay.zone === zone && state.overlay.pid === pid ? null : { zone, pid };
                } });
        x += w + 10;
    }
    hits.push({ x: 0, y, w: 140, h: 14, kind: "player", key, pid, onClick: clickFor(state, key) });
}
function drawBand(ctx, hits, state) {
    const y = ROWS.band;
    const view = state.view;
    ctx.fillStyle = "#120f16";
    ctx.fillRect(0, y, BOARD_W, 30);
    ctx.strokeStyle = "#3a3048";
    ctx.beginPath();
    ctx.moveTo(0, y + 0.5);
    ctx.lineTo(BOARD_W, y + 0.5);
    ctx.moveTo(0, y + 29.5);
    ctx.lineTo(BOARD_W, y + 29.5);
    ctx.stroke();
    // Turn and step tracker.
    text(ctx, `T${view.turn_number}`, 4, y + 3, { font: "8px PressStart", color: "#ffe080" });
    STEPS.forEach((s, i) => {
        const sx = 4 + i * 14, sy = y + 14;
        const on = view.step === s;
        ctx.fillStyle = on ? "#c9a84a" : "#2a2430";
        ctx.fillRect(sx, sy, 13, 12);
        text(ctx, STEP_SHORT[s], sx + 7, sy + 3, { align: "center", font: "7px Silkscreen", color: on ? "#101010" : "#9a90a0" });
    });
    if (view.first_strike_damage_step)
        text(ctx, "first strike", 40, y + 3, { color: "#e0c0a0", font: "7px Silkscreen" });
    // The stack: newest on the right.
    const sx0 = 180;
    text(ctx, view.stack.length ? "STACK →" : "stack empty", sx0, y + 2, { color: "#8a8090", font: "7px Silkscreen" });
    // Whose turn, which step, and the last two things that happened.
    const mine = view.active_player === view.you;
    text(ctx, bandTurnLine(ctx, mine, view.step), BAND_X, y + 2, { font: "7px Silkscreen", color: mine ? "#ffe080" : "#c0b0d0" });
    bandLogLines(state).forEach((l, i) => text(ctx, clip(ctx, inOurWords(state, l), BAND_W, "7px Silkscreen"), BAND_X, y + 12 + i * 8, { font: "7px Silkscreen", color: "#8a8898" }));
    view.stack.forEach((item, i) => {
        const key = `o${item.object_id}`;
        const x = sx0 + i * 28, sy = y + 9;
        const hovered = state.hover !== null && state.hover.key === key;
        const hl = highlightFor(state, key);
        ctx.fillStyle = hl ? HL[hl] : hovered ? "#d0c8e0" : item.controller === view.you ? "#6a5a8a" : "#8a5a5a";
        ctx.fillRect(x - 1, sy - 1, 26, 22);
        ctx.fillStyle = "#201828";
        ctx.fillRect(x, sy, 24, 20);
        drawArt(ctx, x + 2, sy + 2, 16, 12, item.name, [], false);
        text(ctx, String(i + 1), x + 20, sy + 12, { align: "center", font: "6px Silkscreen", color: "#ffe080" });
        // The key stays `o<id>` so a spell on the stack is still clickable as
        // a target; the slot is what says WHICH stack item this chip is, since
        // an ability's id names its source and a trigger's names nothing
        // (issue #527).
        hits.push({ x: x - 1, y: sy - 1, w: 26, h: 22, kind: "stack", key, id: item.object_id, stackIndex: i, onClick: clickFor(state, key) });
    });
}
/**
 * The band's two log lines: what has happened since the page last stopped
 * for the player, not simply the last two things in the log.
 *
 * The page answers by itself every decision whose only actions are a pass
 * and a concede — rightly, there is nothing to decide — so the interval
 * between two frames a person actually reads is not one priority but
 * however many in a row they had no play for. A whole opposing turn fits
 * inside one. Anchored to `slice(-2)`, the band then reported that
 * interval as "the opponent drew a card and attacked", with the Doom Blade
 * that killed your only creature three lines further back; across four
 * driven games, 113 of 276 prompts (41%) arrived with more new lines than
 * the band could show (issue #523).
 *
 * Two rows is what the 30px band has, so when the interval does not fit
 * the band says where it started, how much of it is missing, and where it
 * ended. The count is the signal that the `l` drawer is worth opening;
 * there is no room on this line to say so in words.
 */
export function bandLogLines(state) {
    const log = state.view.display_log;
    const fresh = log.slice(state.logSince ?? 0);
    if (fresh.length === 0)
        return log.slice(-2);
    if (fresh.length <= 2)
        return fresh;
    return [fresh[0], `+${fresh.length - 2} · ${fresh[fresh.length - 1]}`];
}
/** Lines from each stack item to what it targets. */
function stackLines(ctx, hits, state) {
    const view = state.view;
    if (!view.stack.length)
        return;
    const rectOf = (key) => hits.find(h => h.key === key && (h.kind === "perm" || h.kind === "stack" || h.kind === "player" || h.kind === "hand"));
    view.stack.forEach((item, i) => {
        // An ability's chip shares its key with the source permanent, so
        // `rectOf` would draw the arrow from whichever was pushed first. The
        // slot names the chip (issue #527).
        const from = hits.find(h => h.kind === "stack" && h.stackIndex === i);
        if (!from)
            return;
        for (const t of item.targets) {
            const key = typeof t === "object" ? ("Object" in t ? `o${t.Object}` : `p${t.Player}`) : null;
            const to = key ? rectOf(key) : null;
            if (!to)
                continue;
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
/**
 * Who won, in the page's own words, from the runner's headline.
 *
 * `mtg-runner` builds "Game over! p0 (red-green) wins!" — deliberately, so
 * a mirror match can say which seat did which (#251) — and a person who
 * opened a URL has never been told which seat they are. A narrow parse of
 * the one line whose shape the runner controls, with no headline at all
 * when it does not match, rather than a rewrite that would turn "p0 wins!"
 * into "you wins!" (issue #519).
 */
export function outcomeHeadline(state, summary) {
    const won = /^Game over!\s+p(\d+)\b/.exec(summary);
    if (won)
        return Number(won[1]) === state.view.you ? "YOU WIN" : "OPPONENT WINS";
    if (/^Game over! It's a draw!/.test(summary))
        return "A DRAW";
    return null;
}
function gameOverScreen(ctx, state) {
    if (!state.gameOver)
        return;
    ctx.fillStyle = "rgba(0,0,0,0.6)";
    ctx.fillRect(0, 0, BOARD_W, H);
    const w = 360, x = (BOARD_W - w) / 2;
    const headline = outcomeHeadline(state, state.gameOver);
    const lines = inOurWords(state, state.gameOver).split("\n").flatMap(l => wrap(ctx, l, w - 24, "8px Silkscreen"));
    const h = 44 + (headline ? 14 : 0) + lines.length * 11, y = (H - h) / 2;
    texturedPanel(ctx, x, y, w, h);
    text(ctx, "GAME OVER", x + w / 2, y + 10, { align: "center", font: "8px PressStart", color: "#ffe080" });
    let ly = y + 28;
    if (headline) {
        text(ctx, headline, x + w / 2, ly, { align: "center", font: "8px PressStart", color: "#ffffff" });
        ly += 14;
    }
    lines.forEach((l, i) => text(ctx, l, x + w / 2, ly + i * 11, { align: "center" }));
}
// --------------------------------------------------------------- panel
/**
 * The inspector's power/toughness lines: the effective box, and the printed
 * one under it when they differ.
 *
 * A star-P/T creature's printed box is filled in by a characteristic-
 * defining ability (CR 604.3), and the `0` the card data carries for it is
 * a sentinel the engine's own `prints_star_pt` says "must never be shown as
 * one". Sturmgeist with four cards in hand used to read `4/4` over
 * `(printed 0/0)`; `star_pt` was in the view the page was handed and was
 * read nowhere on it. The CLI has printed the star form here since #267
 * (issue #526).
 */
export function inspectorPt(o) {
    if (o.effective_power !== undefined && o.effective_power !== null) {
        const live = `${o.effective_power}/${o.effective_toughness}${o.damage_marked ? ` ${o.damage_marked} dmg` : ""}`;
        if (o.star_pt)
            return [live, "(printed */*)"];
        if (o.printed_power === undefined || o.printed_power === null)
            return [live];
        if (o.printed_power === o.effective_power && o.printed_toughness === o.effective_toughness)
            return [live];
        return [live, `(printed ${o.printed_power}/${o.printed_toughness})`];
    }
    if (o.power !== undefined && o.power !== null)
        return [`${o.power}/${o.toughness}`];
    return [];
}
/**
 * Every fact the inspector states about `e`, in the order it states
 * them.
 *
 * Separated from the drawing so the contract can be read, and tested,
 * without a canvas. The reference list of what a player is entitled to
 * know about a permanent is `CliPlayer::paint_permanent_detail`; this is
 * the same list on the fourth surface.
 */
export function inspectorFacts(state, e) {
    const o = e.obj;
    const out = [];
    // Whether this is a token decides what can be done with it after it dies:
    // CR 111.7 makes it cease to exist, so no recursion ever gets it back and
    // a graveyard count that includes it is wrong. The board rows of both
    // interactive surfaces carry it and neither detail view did — the page's
    // only signal was a 2px stripe on the board card (issue #534).
    if (o.is_token)
        out.push("Token");
    else if (o.is_copy)
        out.push("Copy");
    // Colour (CR 105.2) is what intimidate reads (CR 702.13a), and a
    // transformed face has no mana cost, so without this line it is
    // obtainable from nothing on screen. "Colorless" is the whole answer for
    // Galvanic Juggernaut (CR 105.2c), so it is said out loud (issue #525,
    // #357 on the fourth surface).
    if (o.colors)
        out.push(`Color: ${o.colors.length ? o.colors.join(", ") : "Colorless"}`);
    if (o.keywords && o.keywords.length)
        out.push(o.keywords.join(", "));
    if (o.counters)
        for (const [k, n] of Object.entries(o.counters))
            if (n)
                out.push(`${n} ${k} counter${n > 1 ? "s" : ""}`);
    // The count, not just a badge. A shield is spent one per destruction
    // (CR 701.15a), so six of them and one of them are different boards —
    // and the board's `R` badge carries no number and is 8th of the badges
    // `drawPerm` slices to three, so it is often not on screen at all
    // (issue #525, #468 on the fourth surface).
    if (o.regeneration_shields)
        out.push(`${o.regeneration_shields} regeneration shield${o.regeneration_shields > 1 ? "s" : ""}`);
    if (o.attached_to !== undefined && o.attached_to !== null)
        out.push(`Attached to ${nameOf(state, o.attached_to)}`);
    // And the other direction, which is the one #83 is about: hovering the
    // Equipment said "Attached to Sturmgeist" while hovering the Sturmgeist
    // said nothing about the Equipment.
    const attachments = state.view.battlefield.filter(p => p.attached_to === o.object_id).map(p => p.name);
    if (attachments.length)
        out.push(`Equipped/enchanted with: ${attachments.join(", ")}`);
    if (o.attached_to_player !== undefined && o.attached_to_player !== null)
        out.push(`Enchants ${playerLabel(state, o.attached_to_player)}`);
    if (o.attacking)
        out.push("Attacking " + ("Player" in o.attacking ? playerLabel(state, o.attacking.Player) : nameOf(state, o.attacking.Planeswalker)));
    if (o.blocking && o.blocking.length)
        out.push("Blocking " + o.blocking.map(id => nameOf(state, id)).join(", "));
    if (o.blocked_by && o.blocked_by.length)
        out.push("Blocked by " + o.blocked_by.map(id => nameOf(state, id)).join(", "));
    for (const p of o.protections || [])
        out.push(p);
    for (const r of o.restrictions || [])
        out.push(r);
    if (o.summoning_sick)
        out.push("Summoning sick");
    if (o.named_card)
        out.push(`Named: ${o.named_card}`);
    // Whose ability this is. A trigger is not an object, so its `object_id`
    // is a placeholder and the chip carries nothing that says which of several
    // same-named permanents it fired from (issue #555, #527 on the slot).
    if (e.zone === "stack" && o.source_id !== undefined && o.source_id !== null
        && o.source_id !== o.object_id) {
        out.push(`Source: ${nameOf(state, o.source_id)} (#${o.source_id})`);
    }
    if (o.targets && o.targets.length)
        out.push("Targets: " + o.targets.map(t => targetLabel(state, t)).join(", "));
    if (o.x_value !== undefined && o.x_value !== null)
        out.push(`X = ${o.x_value}`);
    if (e.zone !== "battlefield" && e.zone !== "hand")
        out.push(`In ${e.zone}`);
    return out;
}
/**
 * What the inspector is about: the thing hovered, else the thing selected.
 *
 * A stack chip is read off the slot it was drawn for rather than by id. An
 * activated ability carries its SOURCE permanent's id and a trigger carries
 * `ObjectId(0)`, so by id a chip resolves to the permanent on the
 * battlefield or to nothing at all — which is how hovering a Ghoulcaller's
 * Bell showed "Ghoulcaller's Bell ability / IN STACK" and none of the
 * permanent (issue #527).
 */
export function inspecting(state) {
    const h = state.hover;
    if (h && h.kind === "stack" && h.stackIndex !== undefined) {
        const item = state.view.stack[h.stackIndex];
        return item ? { obj: item, zone: "stack", owner: item.controller } : undefined;
    }
    const hoverId = h && h.key ? Number(h.key.slice(1)) : null;
    if (hoverId !== null)
        return state.index.get(hoverId);
    return state.selected !== null ? state.index.get(state.selected) : undefined;
}
function inspector(ctx, state, x, y, w) {
    const e = inspecting(state);
    if (!e) {
        text(ctx, "Hover a card to read it.", x + 4, y + 4, { color: "#7a7280" });
        return y + 16;
    }
    const o = e.obj;
    const colors = o.colors || colorsOfCost(o.cost);
    drawArt(ctx, x + 4, y + 4, ART_L.w, ART_L.h, o.name, colors, !!o.is_token);
    let ty = y + 4;
    const tx = x + 4 + ART_L.w + 4, tw = w - (ART_L.w + 12);
    for (const l of wrapCapped(ctx, o.name, tw, "8px PressStart", 3)) {
        text(ctx, l, tx, ty, { font: "8px PressStart", color: "#ffffff" });
        ty += 10;
    }
    if (o.cost) {
        manaDots(ctx, o.cost, tx, ty);
        ty += 10;
    }
    const typeLine = [...(o.supertypes || []), ...(o.card_types || [])].join(" ") + ((o.subtypes && o.subtypes.length) ? " — " + o.subtypes.join(" ") : "");
    for (const l of wrapCapped(ctx, typeLine, tw, "7px Silkscreen", 2)) {
        text(ctx, l, tx, ty, { font: "7px Silkscreen", color: "#b0b8c8" });
        ty += 8;
    }
    const pt = inspectorPt(o);
    // `tw`, not the canvas: these two share their row with nothing, but the
    // rule the name one line above obeys is the pane's, not the screen's.
    if (pt.length) {
        text(ctx, pt[0], tx, ty, { font: "8px PressStart", color: "#e0f0ff", maxW: tw });
        ty += 10;
    }
    if (pt.length > 1) {
        text(ctx, pt[1], tx, ty, { font: "7px Silkscreen", color: "#8a8090", maxW: tw });
        ty += 8;
    }
    ty = Math.max(ty, y + 4 + ART_L.h + 4);
    const facts = inspectorFacts(state, e);
    for (const f of facts)
        for (const l of wrapCapped(ctx, f, w - 8, "7px Silkscreen", 2)) {
            text(ctx, l, x + 4, ty, { font: "7px Silkscreen", color: "#d0c8a0" });
            ty += 8;
        }
    const oracle = [o.oracle_text || "", ...(o.granted_abilities || [])].filter(Boolean).join("\n");
    const maxLines = Math.max(0, Math.floor((y + 200 - ty) / 9));
    for (const l of wrapCapped(ctx, oracle, w - 8, "8px Silkscreen", maxLines)) {
        text(ctx, l, x + 4, ty, { color: "#e8e0d0" });
        ty += 9;
    }
    return ty;
}
function promptArea(ctx, hits, state, x, y, w, h) {
    texturedPanel(ctx, x, y, w, h);
    const ui = state.ui;
    let ty = y + 6;
    x += 2;
    w -= 4;
    if (state.gameOver) {
        const headline = outcomeHeadline(state, state.gameOver);
        if (headline) {
            text(ctx, headline, x + 4, ty, { font: "8px PressStart", color: "#ffffff" });
            ty += 11;
        }
        for (const l of wrapCapped(ctx, inOurWords(state, state.gameOver), w - 8, "8px Silkscreen", 8)) {
            text(ctx, l, x + 4, ty, { color: "#ffe080" });
            ty += 9;
        }
        return;
    }
    if (!state.decision) {
        const dots = ".".repeat(1 + Math.floor(Date.now() / 400) % 3);
        text(ctx, state.connected ? `Opponent is thinking${dots}` : "Connecting…", x + 4, ty, { color: "#b0a8c0" });
        if (state.autoPass)
            text(ctx, "AUTO-PASS on (f to stop)", x + 4, ty + 10, { font: "7px Silkscreen", color: "#ffe080" });
        return;
    }
    if (state.autoPass) {
        text(ctx, "AUTO-PASS on (f to stop)", x + 4, ty, { font: "7px Silkscreen", color: "#ffe080" });
        ty += 9;
    }
    const title = ui ? ui.title : "";
    for (const l of wrapCapped(ctx, title, w - 8, "8px Silkscreen", 4)) {
        text(ctx, l, x + 4, ty, { color: "#ffe080" });
        ty += 9;
    }
    if (ui && ui.hint)
        for (const l of wrapCapped(ctx, ui.hint, w - 8, "7px Silkscreen", 3)) {
            text(ctx, l, x + 4, ty, { font: "7px Silkscreen", color: "#a098b0" });
            ty += 8;
        }
    if (ui && ui.mode === "mark") {
        text(ctx, `Marked ${ui.marked.length} of ${ui.max}`, x + 4, ty, { color: "#c0e0a0" });
        ty += 9;
    }
    if (ui && ui.mode === "number") {
        for (const l of (ui.summary || []).slice(0, 4)) {
            text(ctx, clip(ctx, l, w - 8, "7px Silkscreen"), x + 4, ty, { font: "7px Silkscreen", color: "#c0c8d0" });
            ty += 8;
        }
    }
    if (state.notice)
        for (const l of wrapCapped(ctx, state.notice, w - 8, "7px Silkscreen", 3)) {
            text(ctx, l, x + 4, ty, { font: "7px Silkscreen", color: "#ff9080" });
            ty += 8;
        }
    // Rows the board cannot show, then buttons. A modal has its own rows.
    const modal = ui && (ui.mode === "list" || ui.mode === "order" || ui.mode === "number");
    const rows = modal ? [] : ((ui && ui.rows) || (ui && ui.looseRows) || []);
    let by = y + h - 18;
    const buttons = (ui && ui.buttons) || [];
    buttons.slice().reverse().forEach((b) => {
        const label = b.primary && ui && ui.buttonLabel ? ui.buttonLabel() : b.label;
        const enabled = b.enabled ? b.enabled() : true;
        button(ctx, hits, x + 4, by, w - 8, 13, label, b.run, { primary: b.primary, enabled });
        by -= 15;
    });
    let ry = ty + 2;
    // The panel's rows page. They used to be sliced from the front to
    // whatever fitted and the rest were neither drawn nor mentioned: a
    // 30-card library search drew eleven rows and the other nineteen were
    // legal answers the engine had offered that a person could not send
    // (issue #529). A library option has no second surface to fall back on
    // either — graveyard and exile options open their zone overlay, which
    // pages, and `library` is not in that branch.
    //
    // The last row's worth of space goes to the "N–M of K" line whenever
    // there is more than one page, so the pager can always reach the end.
    const fits = Math.max(0, Math.floor((by - ry) / 11));
    const paged = rows.length > fits;
    const perPage = paged ? Math.max(1, fits - 1) : fits;
    const scroll = state.rowScroll = clampRowScroll(rows.length, perPage, state.rowScroll || 0);
    const drawn = rows.slice(scroll, scroll + perPage);
    // What the panel actually drew, published the way `hits` is: the pager's
    // contract is that its last page reaches the last row, and that is not
    // checkable from the outside otherwise.
    state.rowPage = { total: rows.length, scroll, drawn: drawn.length };
    for (const r of drawn) {
        const marked = !!(ui && r.key && ui.marked.includes(r.key));
        panel(ctx, x + 4, ry, w - 8, 10, marked ? "#3a5a3a" : "#2a2430", marked ? "#60e060" : "#4a4a5a");
        text(ctx, clip(ctx, r.label, w - 14, "7px Silkscreen"), x + 7, ry + 1, { font: "7px Silkscreen" });
        hits.push({ x: x + 4, y: ry, w: w - 8, h: 10, kind: "row", onClick: r.run, cardName: r.cardName });
        ry += 11;
    }
    if (paged) {
        text(ctx, clip(ctx, `${scroll + 1}–${Math.min(rows.length, scroll + perPage)} of ${rows.length} (scroll)`, w - 8, "7px Silkscreen"), x + 4, ry + 1, { font: "7px Silkscreen", color: "#8a8090" });
    }
}
/** `scroll` for a panel row list, clamped so the last page is reachable. */
export function clampRowScroll(total, perPage, scroll) {
    if (perPage <= 0)
        return 0;
    return Math.max(0, Math.min(Math.max(0, total - perPage), scroll));
}
/**
 * The log, newest at the bottom, scrolled by `state.logScroll`.
 *
 * `headRoom` is pixels at the top of the box the caller has already drawn
 * something into. The drawer's heading used to be painted at the same y as
 * the first visible line — `240 + 2` and `360 - 118` are the same pixel —
 * so the oldest entry on screen came out as an unreadable two-colour mash,
 * and scrolling only moved which entry that was (issue #521). The box, and
 * the rectangle the wheel scrolls, still cover the whole drawer.
 */
function logArea(ctx, hits, state, x, y, w, h, headRoom = 0) {
    panel(ctx, x, y, w, h, "#100e14", "#3a3048");
    const lines = [];
    for (const entry of state.view.display_log.slice(-40))
        for (const l of wrap(ctx, inOurWords(state, entry), w - 8, "7px Silkscreen"))
            lines.push(l);
    const top = y + 2 + headRoom;
    const fit = Math.floor((h - 4 - headRoom) / 8);
    const shown = lines.slice(Math.max(0, lines.length - fit - state.logScroll), lines.length - state.logScroll);
    shown.forEach((l, i) => text(ctx, l, x + 4, top + i * 8, { font: "7px Silkscreen", color: "#a8a0b0" }));
    hits.push({ x, y, w, h, kind: "log" });
}
// -------------------------------------------------------------- overlays
/** How many rows the modal shows at once. */
export const MODAL_ROWS = 20;
/**
 * The rows the modal will actually draw, filter applied.
 *
 * The filtering lived inside `modal()` while the wheel handler clamped
 * `ui.scroll` against `ui.rows.length` — the UNfiltered count. Scroll a
 * 30-row list to 10, then type a filter matching six, and `slice(10, 30)`
 * of six rows is nothing: the modal drew its title and an empty box, with
 * the "of 30" footer suppressed because the filtered list is shorter than a
 * page, so nothing said that scrolling up would bring the matches back
 * (issue #530). One function, so the two cannot disagree again.
 */
export function modalRows(ui) {
    const rows = ui.rows ?? (ui.order ? ui.order.map((o, pos) => ({ label: o.label, pos })) : []);
    if (ui.mode !== "list" || !ui.query)
        return rows;
    const q = ui.query.toLowerCase();
    return rows.filter(r => r.label.toLowerCase().includes(q));
}
/** `ui.scroll`, clamped to what `modalRows` can actually show. */
export function clampScroll(ui, scroll) {
    return Math.max(0, Math.min(Math.max(0, modalRows(ui).length - MODAL_ROWS), scroll));
}
function modal(ctx, hits, state) {
    const ui = state.ui;
    if (!ui || (ui.mode !== "list" && ui.mode !== "order" && ui.mode !== "number"))
        return;
    if (!ui.keepBoard) {
        ctx.fillStyle = "rgba(0,0,0,0.55)";
        ctx.fillRect(0, 0, BOARD_W, H);
    }
    const w = 300, x = (BOARD_W - w) / 2;
    const rows = modalRows(ui);
    const rowH = 12;
    const maxRows = MODAL_ROWS;
    // Clamped here as well as in the wheel handler: typing a filter narrows
    // the list under a scroll position the wheel set legitimately.
    const scroll = clampScroll(ui, ui.scroll || 0);
    ui.scroll = scroll;
    const shown = rows.slice(scroll, scroll + maxRows);
    const h = 30 + shown.length * rowH + (ui.mode === "number" ? 30 : 0) + (ui.filter ? 14 : 0) + 18;
    const y = Math.max(8, (H - h) / 2);
    // The background hit goes in first, so the rows drawn on it are on top.
    hits.push({ x, y, w, h, kind: "modal" });
    texturedPanel(ctx, x, y, w, h, "#1a1620", "#c9a84a");
    let ty = y + 6;
    for (const l of wrapCapped(ctx, ui.title, w - 12, "8px PressStart", 2)) {
        text(ctx, l, x + 6, ty, { font: "8px PressStart", color: "#ffe080" });
        ty += 10;
    }
    if (ui.filter) {
        // Where the real <input> goes. The modal used to paint a field here and
        // `syncField` parked the DOM control at a fixed spot near the top of the
        // canvas, so the screen showed two filter boxes: one over the
        // opponent's life strip that took the typing, and one in the middle of
        // the modal that looked like the thing to click and never filled in
        // (issue #531). The frame is still drawn here; the input is placed over
        // it, the way hit rectangles are published for the mouse.
        panel(ctx, x + 6, ty, w - 12, 12, "#0e0c12", "#6a5a7a");
        state.fieldRect = { x: x + 6, y: ty, w: w - 12, h: 12 };
        ty += 14;
    }
    if (ui.mode === "number") {
        panel(ctx, x + 6, ty, 80, 14, "#0e0c12", "#6a5a7a");
        state.fieldRect = { x: x + 6, y: ty, w: 80, h: 14 };
        text(ctx, `0 – ${ui.max}`, x + 92, ty + 3, { color: "#b0b0c0" });
        ty += 18;
        for (const l of (ui.summary || []).slice(0, 4)) {
            text(ctx, clip(ctx, l, w - 12, "7px Silkscreen"), x + 6, ty, { font: "7px Silkscreen", color: "#c0c8d0" });
            ty += 8;
        }
        ty += 4;
    }
    shown.forEach((r, i) => {
        const ry = ty + i * rowH;
        panel(ctx, x + 6, ry, w - 12, rowH - 1, "#2a2430", "#4a4a5a");
        if (ui.mode === "order") {
            text(ctx, `${scroll + i + 1}.`, x + 9, ry + 2, { color: "#ffe080" });
            text(ctx, clip(ctx, r.label, w - 70, "7px Silkscreen"), x + 26, ry + 2, { font: "7px Silkscreen" });
            button(ctx, hits, x + w - 34, ry + 1, 12, rowH - 3, "▲", () => ui.move(r.pos, -1));
            button(ctx, hits, x + w - 20, ry + 1, 12, rowH - 3, "▼", () => ui.move(r.pos, 1));
        }
        else {
            text(ctx, clip(ctx, r.label, w - 18, "8px Silkscreen"), x + 9, ry + 2);
            hits.push({ x: x + 6, y: ry, w: w - 12, h: rowH - 1, kind: "row", onClick: r.run, cardName: r.cardName });
        }
    });
    ty += shown.length * rowH;
    // Printed whenever the list is filtered too, not only when it is longer
    // than a page: "0 of 30 matching" is the line that was missing when the
    // modal went empty (issue #530).
    if (rows.length > maxRows || (ui.mode === "list" && ui.query)) {
        const all = (ui.rows ?? []).length;
        const where = rows.length === 0 ? "no matches"
            : `${scroll + 1}–${Math.min(rows.length, scroll + maxRows)} of ${rows.length}`;
        const of = ui.mode === "list" && ui.query ? ` matching "${ui.query}" (of ${all})` : " (scroll)";
        text(ctx, clip(ctx, where + of, w - 12, "7px Silkscreen"), x + 6, ty + 2, { font: "7px Silkscreen", color: "#8a8090" });
    }
    let bx = x + w - 6;
    for (const b of (ui.buttons || []).slice().reverse()) {
        const bw = 60;
        bx -= bw + 4;
        button(ctx, hits, bx, y + h - 17, bw, 13, b.label, b.run, { primary: b.primary });
    }
}
function popover(ctx, hits, state) {
    const p = state.popover;
    if (!p)
        return;
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
        if (hov) {
            ctx.fillStyle = "#3a3048";
            ctx.fillRect(x + 2, ry, w - 4, 11);
        }
        text(ctx, clip(ctx, it.label, w - 8, "8px Silkscreen"), x + 5, ry + 2, { color: "#f0e8d8" });
    });
}
function zoneOverlay(ctx, hits, state) {
    const ov = state.overlay;
    if (!ov)
        return;
    const view = state.view;
    let cards = [];
    let title = "";
    if (ov.zone === "graveyard") {
        const g = view.graveyards.find(([pid]) => pid === ov.pid);
        cards = g ? g[1] : [];
        title = `${playerLabel(state, ov.pid)}: graveyard (${cards.length})`;
    }
    else if (ov.zone === "exile") {
        cards = view.exile.filter(c => c.owner === ov.pid);
        title = `${playerLabel(state, ov.pid)}: exile (${cards.length})`;
    }
    else if (ov.zone === "library") {
        cards = ov.pid === view.you ? view.your_library_cards : [];
        title = ov.pid === view.you ? `Your library (${cards.length}, order unknown)` : "Opponent's library";
    }
    hits.push({ x: 0, y: 0, w: BOARD_W, h: H, kind: "overlay", onClick: () => { state.overlay = null; } });
    ctx.fillStyle = "rgba(0,0,0,0.7)";
    ctx.fillRect(0, 0, BOARD_W, H);
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
        ctx.fillStyle = hl ? HL[hl] : hovered ? "#d0c8e0" : "#4a4050";
        ctx.fillRect(x - 1, y - 1, cw + 2, ch + 2);
        ctx.fillStyle = "#201828";
        ctx.fillRect(x, y, cw, ch);
        drawArt(ctx, x + 6, y + 3, ART_S.w, ART_S.h, c.name, colorsOfCost(c.cost), false);
        wrapCapped(ctx, c.name, cw - 4, "6px Silkscreen", 3).forEach((l, j) => text(ctx, l, x + 2, y + 30 + j * 7, { font: "6px Silkscreen" }));
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
    const pattern = table ? ctx.createPattern(table, "repeat") : null;
    ctx.fillStyle = pattern ?? "#24202a";
    ctx.fillRect(0, 0, W, H);
    ctx.fillStyle = "rgba(0,0,0,0.5)";
    ctx.fillRect(0, 0, BOARD_W, H);
    if (!state.view) {
        text(ctx, state.connected ? "Waiting for the game…" : "Connecting to the runner…", 12, 12, { font: "8px PressStart", color: "#ffe080" });
        return hits;
    }
    const live = state;
    const view = live.view;
    const opp = view.opponents[0] || { id: 1 - view.you, life: 0, hand_size: 0, library_size: 0, mana_pool: { mana: {} }, mulligan_count: 0 };
    const gyCount = (pid) => { const g = view.graveyards.find(([p]) => p === pid); return g ? g[1].length : 0; };
    const exCount = (pid) => view.exile.filter(c => c.owner === pid).length;
    // Opponent.
    drawStrip(ctx, hits, live, ROWS.oppStrip, opp.id, opp.life, opp.hand_size, opp.library_size, gyCount(opp.id), exCount(opp.id), opp.mana_pool, false);
    const ob = splitBoard(live, view, opp.id);
    drawRow(ctx, hits, live, ob.groups, ROWS.oppOther);
    drawRow(ctx, hits, live, ob.creatures, ROWS.oppCreatures);
    // Middle.
    drawBand(ctx, hits, live);
    // You.
    const mb = splitBoard(live, view, view.you);
    drawRow(ctx, hits, live, mb.creatures, ROWS.myCreatures);
    drawRow(ctx, hits, live, mb.groups, ROWS.myOther);
    // Hand: your strip sits on the hand's top edge.
    const hand = view.your_hand;
    const stripY = ROWS.hand - 14;
    drawStrip(ctx, hits, live, stripY, view.you, view.your_life, hand.length, view.your_library_size, gyCount(view.you), exCount(view.you), view.your_mana_pool, true);
    const { stride, x0 } = rowLayout(hand.length, HAND.w, 4, BOARD_W - 8, 2);
    const hoveredIdx = hand.findIndex(c => state.hover !== null && state.hover.key === `o${c.object_id}`);
    hand.forEach((c, i) => { if (i !== hoveredIdx)
        drawHandCard(ctx, hits, live, c, x0 + i * stride, ROWS.hand, false); });
    if (hoveredIdx >= 0)
        drawHandCard(ctx, hits, live, hand[hoveredIdx], x0 + hoveredIdx * stride, ROWS.hand, true);
    // Panel.
    ctx.fillStyle = "#14111a";
    ctx.fillRect(PANEL_X, 0, W - PANEL_X, H);
    ctx.strokeStyle = "#3a3048";
    ctx.beginPath();
    ctx.moveTo(PANEL_X + 0.5, 0);
    ctx.lineTo(PANEL_X + 0.5, H);
    ctx.stroke();
    const insEnd = inspector(ctx, live, PANEL_X, 0, W - PANEL_X);
    const promptY = Math.max(insEnd + 4, 150);
    const promptH = state.logOpen ? 100 : H - promptY - 4;
    promptArea(ctx, hits, live, PANEL_X + 2, promptY, W - PANEL_X - 4, promptH);
    if (state.logOpen)
        logArea(ctx, hits, live, PANEL_X + 2, promptY + promptH + 2, W - PANEL_X - 4, H - promptY - promptH - 4);
    stackLines(ctx, hits, live);
    zoneOverlay(ctx, hits, live);
    popover(ctx, hits, live);
    modal(ctx, hits, live);
    gameOverScreen(ctx, live);
    if (state.logOpen && !state.overlay) {
        // A wide log drawer over the board, for reading rather than glancing.
        ctx.fillStyle = "rgba(10,8,14,0.92)";
        ctx.fillRect(0, H - 120, BOARD_W, 120);
        // The heading gets a row of its own (issue #521); `logArea` starts
        // below it and the drawer still had spare pixels at the bottom.
        logArea(ctx, hits, live, 0, H - 120, BOARD_W, 120, 10);
        text(ctx, "LOG (l to close, wheel to scroll)", 4, H - 118, { font: "7px Silkscreen", color: "#ffe080" });
    }
    return hits;
}

// The page: one WebSocket to the seat, one canvas, one state object.
import { loadManifest, fontsReady, artNames } from "./assets.js";
import { render, inspecting, inspectorFacts, inspectorPt, wrap, wrapCapped, bandTurnLine, bandLogLines, clampScroll, outcomeHeadline, BAND_W, W, H, PANEL_X } from "./render.js";
import { beginDecision, indexView, beginList, inOurWords } from "./prompts.js";
const canvas = document.getElementById("game");
const ctx = canvas.getContext("2d");
const field = document.getElementById("field");
const state = {
    ws: null, connected: false,
    view: null, index: new Map(),
    decision: null, // {seq, legal, combat}
    ui: null, // the widget answering it
    gameOver: null,
    hover: null, hits: [], popover: null, overlay: null, selected: null,
    notice: null, logOpen: false, logScroll: 0,
    scale: 1,
};
window.mtg = state;
function live() {
    return state.view ? state : null;
}
// ---------------------------------------------------------------- socket
function connect() {
    const url = `${location.protocol === "https:" ? "wss" : "ws"}://${location.host}/ws`;
    const ws = new WebSocket(url);
    state.ws = ws;
    ws.onopen = () => { state.connected = true; sendRaw({ type: "hello" }); draw(); };
    ws.onclose = () => { state.connected = false; draw(); setTimeout(connect, 1000); };
    ws.onerror = () => { ws.close(); };
    ws.onmessage = (ev) => { try {
        onMessage(JSON.parse(ev.data));
    }
    catch (e) {
        console.error(e);
    } };
}
function sendRaw(msg) {
    if (state.ws && state.ws.readyState === WebSocket.OPEN)
        state.ws.send(JSON.stringify(msg));
}
function send(action) {
    if (!state.decision)
        return;
    const seq = state.decision.seq;
    window.mtgDebug.sent.push({ seq, action });
    sendRaw({ type: "action", seq, action });
    state.lastSent = { seq, action };
    // Nothing more to click until the next message; the board stays.
    state.decision = null;
    state.ui = null;
    state.popover = null;
    state.notice = null;
    hideField();
    draw();
}
function setView(view) {
    state.view = view;
    state.index = indexView(view);
}
function onMessage(msg) {
    switch (msg.type) {
        case "view":
            setView(msg.view);
            if (state.decision === null) {
                state.ui = null;
                state.popover = null;
            }
            break;
        case "decision": {
            setView(msg.view);
            state.decision = { seq: msg.seq, legal: msg.legal, combat: msg.combat };
            // A new question starts at the top of its list (issue #529).
            state.rowScroll = 0;
            state.popover = null;
            state.overlay = null;
            const actions = msg.legal.actions || [];
            // Only Pass and Concede on offer: nothing to decide, as in the CLI.
            const onlyPass = !msg.combat && !msg.legal.resolution_prompt && !msg.legal.set_prompt
                && actions.length > 0 && actions.every(a => a === "PassPriority" || a === "Concede") && actions.includes("PassPriority");
            const v = msg.view;
            const what = `seq ${msg.seq} T${v.turn_number} ${v.step} active=${v.active_player} you=${v.you} ctx=${msg.legal.context}`;
            if (onlyPass && !state.stopAtPass) {
                window.mtgDebug.trace.push(`${what}: only pass`);
                send("PassPriority");
                return;
            }
            beginDecision(state, send);
            if (autoPassDecides()) {
                window.mtgDebug.trace.push(`${what}: auto-passed`);
                return;
            }
            // Past here the page is stopping and a person will read this frame.
            // That is the point the band's recap is measured from: everything
            // since the PREVIOUS stop is what they have not seen (issue #523).
            // The two branches above return without touching these, which is how
            // an interval of ten auto-answered priorities stays one interval.
            state.logSince = state.logSeen ?? 0;
            state.logSeen = v.display_log.length;
            window.mtgDebug.trace.push(`${what}: ${state.ui ? state.ui.mode : "?"}`);
            syncField();
            break;
        }
        case "answered":
            // Another page on this seat answered the decision we are holding
            // (or we did). Stop offering it: the board is about to move, and a
            // click here would be dropped as stale and read as accepted
            // (issue #516).
            if (state.decision && state.decision.seq === msg.seq) {
                const ours = !!(state.lastSent && state.lastSent.seq === msg.seq);
                state.decision = null;
                state.ui = null;
                state.popover = null;
                if (!ours)
                    state.notice = "Answered in another tab.";
                hideField();
            }
            break;
        case "settings":
            // The seat's settings, not this page's. Applied without echoing them
            // back, or two tabs would bounce the message between them.
            state.stopAtPass = msg.stop_at_pass;
            state.autoPass = msg.auto_pass_since_turn === null ? null : { sinceTurn: msg.auto_pass_since_turn };
            break;
        case "notice":
            state.notice = msg.text;
            // The decision stands; if we had cleared it on send, ask again.
            if (!state.decision && state.lastSent && state.lastSent.seq === msg.seq && state.lastDecision && state.view) {
                state.decision = state.lastDecision;
                beginDecision(state, send);
            }
            break;
        case "game_over":
            setView(msg.view);
            state.decision = null;
            state.ui = null;
            state.popover = null;
            state.gameOver = msg.summary;
            break;
        default:
            console.warn("unknown message", msg);
    }
    if (state.decision)
        state.lastDecision = state.decision;
    draw();
}
/**
 * Auto-pass (`f`): pass every plain priority until something happens —
 * a prompt that is not a pass, a spell on the stack, or your next
 * precombat main phase. Returns true when it answered the decision.
 */
function autoPassDecides() {
    const ap = state.autoPass;
    const v = state.view;
    const ui = state.ui;
    if (!ap || !v || !ui)
        return false;
    const stop = ui.mode !== "menu" || !ui.canPass || v.stack.length > 0
        || (v.active_player === v.you && v.step === "PrecombatMain" && v.turn_number > ap.sinceTurn);
    if (stop) {
        state.autoPass = null;
        state.notice = ui.mode !== "menu" ? "Auto-pass off: you are asked something." : v.stack.length > 0 ? "Auto-pass off: something is on the stack." : "Auto-pass off: your main phase.";
        pushSettings();
        return false;
    }
    send("PassPriority");
    return true;
}
function toggleAutoPass() {
    const v = state.view;
    const ui = state.ui;
    if (state.autoPass) {
        state.autoPass = null;
        state.notice = "Auto-pass off.";
        pushSettings();
        return;
    }
    if (!v)
        return;
    if (!ui || ui.mode !== "menu" || !ui.canPass) {
        state.notice = "Auto-pass passes priority, and this is not a pass.";
        return;
    }
    state.autoPass = { sinceTurn: v.turn_number };
    state.notice = null;
    pushSettings();
    send("PassPriority");
}
/**
 * Tell the seat what this page just decided about how the seat behaves.
 *
 * `s` and `f` both govern whether a page answers a priority *for* the
 * player. That is a decision on behalf of the seat, and a seat has one
 * state however many pages are attached to it: a second tab used to
 * auto-pass the priorities the first was deliberately holding, because its
 * own `stopAtPass` was false and nothing told it otherwise (issue #515).
 * The seat holds the setting and echoes it to every page, including this
 * one, which is also what a page joining later is handed on `hello`.
 */
function pushSettings() {
    sendRaw({ type: "settings", stop_at_pass: !!state.stopAtPass,
        auto_pass_since_turn: state.autoPass ? state.autoPass.sinceTurn : null });
}
// ----------------------------------------------------------------- input
/**
 * Size the canvas to the window.
 *
 * At or above the canvas's own 640x360 the scale is an integer: this is a
 * pixel-art page and a fractional scale there would blur it for nothing.
 *
 * Below it, the scale used to be floored at 1 — so the canvas stayed at
 * full size and `index.html`'s `overflow: hidden` cut the overflow off
 * both sides, with no scrollbar, no drag and nothing on screen saying
 * anything was missing. What goes first is the right-hand 160px, which is
 * the prompt panel: its title, its rows and every button. At 390px wide,
 * 35px of that panel was on screen and clicking Pass or Concede did
 * nothing at all (issue #569). Shrinking below 1 makes a small window
 * blurry; cropping made it unanswerable.
 */
function fitCanvas() {
    const fit = Math.min(window.innerWidth / W, window.innerHeight / H);
    // A window with no area at all (a hidden tab, a zero-height frame) would
    // otherwise give a scale of 0, which `canvasPoint` divides by.
    const s = fit >= 1 ? Math.floor(fit) : Math.max(fit, 0.05);
    state.scale = s;
    canvas.style.width = `${W * s}px`;
    canvas.style.height = `${H * s}px`;
    draw();
}
function canvasPoint(ev) {
    const r = canvas.getBoundingClientRect();
    return { x: (ev.clientX - r.left) / state.scale, y: (ev.clientY - r.top) / state.scale };
}
function hitAt(x, y) {
    for (let i = state.hits.length - 1; i >= 0; i--) {
        const h = state.hits[i];
        if (x >= h.x && x < h.x + h.w && y >= h.y && y < h.y + h.h)
            return h;
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
        if (e)
            state.hover = { ...next, key: `o${e.obj.object_id}` };
    }
    canvas.style.cursor = h && h.onClick ? "pointer" : "default";
    if (changed)
        draw();
});
canvas.addEventListener("mouseleave", () => { state.hover = null; draw(); });
canvas.addEventListener("click", (ev) => {
    const { x, y } = canvasPoint(ev);
    const h = hitAt(x, y);
    state.notice = null;
    if (state.popover && (!h || (h.kind !== "row" && h.kind !== "popover"))) {
        state.popover = null;
        draw();
        return;
    }
    if (h && h.onClick)
        h.onClick(h);
    else if (h && h.kind === "modal") { /* inside a modal: nothing */ }
    else if (h && h.key && h.id !== undefined)
        state.selected = h.id;
    syncField();
    draw();
});
// A double-click on a card with one thing to do does it.
canvas.addEventListener("dblclick", (ev) => {
    const { x, y } = canvasPoint(ev);
    const h = hitAt(x, y);
    if (h && h.verbs && h.verbs.length === 1) {
        state.popover = null;
        h.verbs[0].run();
        syncField();
        draw();
    }
});
// Right-click backs out of whatever is open, like Escape.
canvas.addEventListener("contextmenu", (ev) => {
    ev.preventDefault();
    const ui = state.ui;
    if (state.popover)
        state.popover = null;
    else if (state.overlay)
        state.overlay = null;
    else if (ui && ui.onCancel)
        ui.onCancel();
    draw();
});
canvas.addEventListener("wheel", (ev) => {
    const ui = state.ui;
    const over = canvasPoint(ev);
    // Over the prompt panel, the wheel scrolls the panel's row list. The
    // renderer owns the clamp — it is the only thing that knows how many rows
    // fit under the title, the hint and the buttons — so this only moves the
    // number and lets the next frame pull it back into range (issue #529).
    if (over.x >= PANEL_X && ui && ((ui.rows && ui.rows.length) || (ui.looseRows && ui.looseRows.length))
        && ui.mode !== "list" && ui.mode !== "order" && ui.mode !== "number") {
        state.rowScroll = Math.max(0, (state.rowScroll || 0) + Math.sign(ev.deltaY) * 3);
        ev.preventDefault();
        draw();
        return;
    }
    if (ui && ui.mode === "list" && ui.rows) {
        // Clamped against the rows the modal will draw, filter applied — not
        // against the unfiltered count, which is how a scrolled list could be
        // filtered into an empty box (issue #530).
        ui.scroll = clampScroll(ui, (ui.scroll || 0) + Math.sign(ev.deltaY) * 3);
    }
    else if (state.logOpen) {
        state.logScroll = Math.max(0, state.logScroll - Math.sign(ev.deltaY) * 2);
    }
    ev.preventDefault();
    draw();
}, { passive: false });
window.addEventListener("keydown", (ev) => {
    if (ev.target === field) {
        if (ev.key === "Enter") {
            const ui = state.ui;
            if (ui && ui.mode === "number" && ui.submit)
                ui.submit();
            ev.preventDefault();
        }
        if (ev.key === "Escape") {
            field.blur();
        }
        return;
    }
    const ui = state.ui;
    const v = state.view;
    switch (ev.key) {
        case "Enter":
            if (!ui)
                return;
            if (ui.mode === "menu" && ui.canPass)
                send("PassPriority");
            else if (ui.onConfirm)
                ui.onConfirm();
            else if (ui.mode === "number" && ui.submit)
                ui.submit();
            break;
        case "Escape":
            if (state.popover)
                state.popover = null;
            else if (state.overlay)
                state.overlay = null;
            else if (ui && ui.onCancel)
                ui.onCancel();
            break;
        case "l":
            state.logOpen = !state.logOpen;
            state.logScroll = 0;
            break;
        case "g":
            if (v)
                toggleZone("graveyard", v.you);
            break;
        case "G":
            if (v && v.opponents[0])
                toggleZone("graveyard", v.opponents[0].id);
            break;
        case "e":
            if (v)
                toggleZone("exile", v.you);
            break;
        case "d":
            if (v)
                toggleZone("library", v.you);
            break;
        case "s":
            state.stopAtPass = !state.stopAtPass;
            state.notice = state.stopAtPass ? "Stopping at every priority." : "Passing automatically when there is nothing to do.";
            pushSettings();
            break;
        case "f":
            toggleAutoPass();
            break;
        default: return;
    }
    ev.preventDefault();
    draw();
});
function toggleZone(zone, pid) {
    state.overlay = state.overlay && state.overlay.zone === zone && state.overlay.pid === pid ? null : { zone, pid };
}
// The one DOM control: typed input for a filter or a number.
function syncField() {
    const ui = state.ui;
    const wants = ui && ((ui.mode === "list" && ui.filter) || ui.mode === "number");
    if (!wants || !ui) {
        hideField();
        return;
    }
    const r = canvas.getBoundingClientRect();
    // Over the frame the modal painted for it, not at a fixed spot near the
    // top of the canvas. The two used to be different rectangles, so the page
    // showed two filter boxes — one over the opponent's life strip that took
    // the typing, and an inert "type to filter…" in the middle of the modal
    // that looked like the thing to click (issue #531). `modal()` publishes
    // its rectangle the way `render` publishes hit rectangles; the fallback
    // is the old position, for the frame before the modal has been drawn.
    const box = state.fieldRect ?? { x: PANEL_X / 2 - 150 + 6, y: 4, w: 280, h: 12 };
    field.style.display = "block";
    field.style.left = `${r.left + box.x * state.scale}px`;
    field.style.top = `${r.top + box.y * state.scale}px`;
    field.style.width = `${box.w * state.scale}px`;
    field.style.height = `${box.h * state.scale}px`;
    field.style.fontSize = `${8 * state.scale}px`;
    const want = ui.mode === "number" ? (ui.value ?? "") : (ui.query ?? "");
    if (field.value !== want)
        field.value = want;
    field.placeholder = ui.mode === "number" ? `X (0-${ui.max})` : "filter";
    if (document.activeElement !== field)
        field.focus();
}
function hideField() { field.style.display = "none"; field.value = ""; }
field.addEventListener("input", () => {
    const ui = state.ui;
    if (!ui)
        return;
    if (ui.mode === "number")
        ui.value = field.value.trim();
    else if (ui.mode === "list") {
        ui.query = field.value;
        ui.scroll = 0;
    }
    draw();
});
// ----------------------------------------------------------------- frame
let pending = false;
function draw() {
    if (pending)
        return;
    pending = true;
    requestAnimationFrame(() => {
        pending = false;
        try {
            state.hits = render(ctx, state);
        }
        catch (e) {
            console.error(e);
        }
        // The modal's geometry depends on how many rows survive the filter, so
        // it moves as a person types; the input has to follow it rather than
        // sit where the modal was when the prompt opened (issue #531).
        syncField();
    });
}
state.draw = draw;
// Art arrives over time and highlights breathe; repaint on a slow beat.
setInterval(draw, 120);
window.addEventListener("resize", fitCanvas);
void (async () => {
    await Promise.all([loadManifest(), fontsReady()]);
    fitCanvas();
    connect();
})();
// Exposed for tests: answer the current decision with a raw action, or
// stage a synthetic decision and render it without a socket.
window.mtgSend = send;
window.mtgList = (rows, title) => { const l = live(); if (l)
    beginList(l, { mode: "list", title, hint: "", buttons: [], marked: [] }, rows, title, send, false); };
window.mtgDebug = {
    stage(decision) {
        const l = live();
        if (!l)
            throw new Error("no view to stage a decision over");
        state.decision = decision;
        state.lastDecision = decision;
        state.popover = null;
        state.overlay = null;
        state.notice = null;
        state.rowScroll = 0;
        beginDecision(l, send);
        syncField();
        state.hits = render(ctx, state);
    },
    render() { state.hits = render(ctx, state); syncField(); return state.hits.length; },
    inspect(key, kind) {
        const l = live();
        if (!l)
            return null;
        // A test may have edited the view in place; the index is derived from
        // it, so derive it again rather than reading a stale one.
        state.index = indexView(l.view);
        state.hits = render(ctx, state);
        const hit = state.hits.slice().reverse().find(h => h.key === key && (!kind || h.kind === kind));
        if (!hit)
            return null;
        const was = state.hover;
        state.hover = hit;
        const out = this.inspectHover();
        state.hover = was;
        return out;
    },
    inspectHover() {
        const l = live();
        if (!l)
            return null;
        const e = inspecting(l);
        return e ? { name: e.obj.name, zone: e.zone, facts: inspectorFacts(l, e), pt: inspectorPt(e.obj) } : null;
    },
    artNames,
    band: () => { const l = live(); return l ? bandLogLines(l) : []; },
    words: (line) => { const l = live(); return l ? inOurWords(l, line) : line; },
    outcome: (summary) => { const l = live(); return l ? outcomeHeadline(l, summary) : null; },
    fit: {
        wrap: (s, maxW, font) => wrap(ctx, s, maxW, font),
        wrapCapped: (s, maxW, font, maxLines) => wrapCapped(ctx, s, maxW, font, maxLines),
        width: (s, font) => { ctx.font = font; return ctx.measureText(s).width; },
        bandLine: (mine, step) => bandTurnLine(ctx, mine, step),
        bandW: BAND_W,
    },
    sent: [],
    trace: [],
};

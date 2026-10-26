// The draft page's DOM, rebuilt from the state on every change.
//
// One function per phase. Nothing here keeps state between renders: a
// view is the whole seat, so the page can be thrown away and rebuilt
// from it, which is what a reconnect and a refusal both rely on.

import { artFor, groupByColor, initials, parseLine, COLOR_NAMES, type ColorKey } from "./cards.js";
import { BASICS, MIN_DECK, counts, problem, summary, type DeckEdit } from "./deck.js";
import type { Between, CardInfo, DraftView, Match, PackCard, ReviewPick, SeatInfo } from "./protocol.js";
import type { Actions, PageState } from "./state.js";

type Child = Node | string | null | undefined | false;

/** A DOM builder: tag, properties and attributes, children. */
export function h(tag: string, props: Record<string, unknown> = {}, ...children: Child[]): HTMLElement {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (v === undefined || v === null || v === false) continue;
    if (k === "class") el.className = String(v);
    else if (k.startsWith("on") && typeof v === "function") el.addEventListener(k.slice(2), v as EventListener);
    else if (k === "disabled" || k === "checked") (el as HTMLButtonElement).disabled = v === true;
    else if (k === "text") el.textContent = String(v);
    else el.setAttribute(k, String(v));
  }
  for (const c of children) {
    if (c === null || c === undefined || c === false) continue;
    el.append(typeof c === "string" ? document.createTextNode(c) : c);
  }
  return el;
}

// ----------------------------------------------------------------- pieces

/** A mana cost as pips: `{1}{W}{W}` becomes three badges. */
export function pips(cost: string): HTMLElement {
  const el = h("span", { class: "cost" });
  for (const pip of cost.match(/\{[^}]*\}/g) ?? []) {
    const inner = pip.slice(1, -1);
    const key = /^[WUBRG]$/.test(inner) ? inner : /^\d+\/[WUBRG]$/.test(inner) ? inner.split("/")[1] : "N";
    el.append(h("span", { class: `pip pip-${key}`, text: inner.replace(/\/[WUBRG]$/, "") }));
  }
  return el;
}

function art(card: CardInfo): HTMLElement {
  const path = artFor(card.name);
  const frame = h("div", { class: `art frame-${colorClass(card)}` });
  if (path) frame.append(h("img", { src: path, alt: "", draggable: "false" }));
  else frame.append(h("span", { class: "initials", text: initials(card.name) }));
  return frame;
}

function colorClass(card: CardInfo): string {
  const own = card.colors.filter(c => "WUBRG".includes(c));
  return own.length === 0 ? "C" : own.length > 1 ? "M" : own[0];
}

/** The side panel's reading of one card: headline, type, P/T, rules text. */
export function cardDetail(card: CardInfo): HTMLElement {
  const p = parseLine(card.name, card.line);
  const lines = (card.text || "").split("\n").filter(Boolean);
  return h("div", { class: "detail-card" },
    art(card),
    h("div", { class: "detail-head" },
      h("div", { class: "detail-name", text: card.name }),
      h("div", { class: "detail-cost" }, pips(p.cost)),
      h("div", { class: "detail-type", text: p.typeLine + (p.pt ? `  ${p.pt}` : "") }),
      h("div", { class: `rarity rarity-${card.rarity}`, text: card.rarity }),
    ),
    h("div", { class: "detail-text" }, ...lines.map(l => h("p", { text: l }))),
    p.back ? h("div", { class: "detail-back", text: `Back: ${p.back}` }) : null,
  );
}

function poolGroups(pool: CardInfo[]): HTMLElement {
  const el = h("div", { class: "pool-groups" });
  if (pool.length === 0) el.append(h("p", { class: "muted", text: "Nothing drafted yet." }));
  for (const g of groupByColor(pool)) {
    const counted = new Map<string, number>();
    for (const c of g.cards) counted.set(c.name, (counted.get(c.name) ?? 0) + 1);
    el.append(h("h4", { class: `group-head color-${g.key}` }, `${g.label} (${g.cards.length})`));
    const ul = h("ul", { class: "pool-list" });
    for (const [name, n] of counted) {
      const card = g.cards.find(c => c.name === name)!;
      const p = parseLine(name, card.line);
      ul.append(h("li", { title: card.text },
        n > 1 ? h("span", { class: "count", text: `${n}x ` }) : null,
        pips(p.cost), " ", h("span", { class: "pool-name", text: name })));
    }
    el.append(ul);
  }
  return el;
}

/** A no-break space: what holds a unit of several words ("(seat 2)",
 *  "[1 bye]", "— done") on one line at a phone's width (#881, #934). */
const NBSP = "\u00a0";

function seatName(view: DraftView, seat: number): string {
  const label = `seat${NBSP}${seat}`;
  const s = view.seats.find(x => x.seat === seat);
  if (!s || s.name === `seat ${seat}`) return label;
  return `${s.name} (${label})`;
}

/** A standings tag's count stays with its noun: "[1 bye]" split as
 *  "[1" / "bye]" (#881). */
export function tagText(tags: string): string {
  return tags.replace(/(\d) /g, `$1${NBSP}`);
}

/** " — status": the dash leads the words it introduces, never ends a line
 *  above them (#934). */
function dashed(rest: string): string {
  return ` —${NBSP}${rest}`;
}

/** Where the seat is: the table's own word when it sends one (a person in
 *  a match is wherever their game page is, #822); else "here", "away"
 *  (joined, no socket) or "not yet", an AI seat always here. */
function presence(s: SeatInfo): string {
  if (s.presence) return s.presence;
  if (!s.joined) return "not yet";
  if (s.kind === "human" && s.connected === 0) return "away";
  return "here";
}

/** The status word, with the table's hand named when it picks for the seat. */
function statusWord(s: SeatInfo): string {
  return s.auto ? `${s.status} (table${NBSP}picks)` : s.status;
}

function seatsTable(view: DraftView): HTMLElement {
  const rows = view.seats.map((s: SeatInfo) => h("tr", { class: s.seat === view.seat ? "me" : "" },
    h("td", { text: String(s.seat) }),
    h("td", { class: "name", text: s.name + (s.seat === view.seat ? " (you)" : "") }),
    h("td", { text: s.kind }),
    h("td", { text: presence(s) }),
    h("td", { text: statusWord(s) }),
    h("td", { class: "num", text: String(s.picks) }),
  ));
  return h("table", { class: "seats" },
    h("thead", {}, h("tr", {}, ...["seat", "name", "kind", "joined", "status", "picks"].map(t => h("th", { text: t })))),
    h("tbody", {}, ...rows));
}

/**
 * What the standings count. They are recorded when a round's last match
 * ends, so beside a bye or a finished match marked "done" they still give
 * that seat its points from before the round (#818); the heading says so.
 */
export function standingsHeading(after: number | undefined): string {
  if (after === undefined) return "Standings";
  return after === 0 ? "Standings (counted when round 1 ends)" : `Standings after round ${after}`;
}

function standingsTable(view: DraftView): HTMLElement {
  if (view.standings.length === 0) return h("p", { class: "muted", text: "No results yet." });
  const rows = view.standings.map((s, i) => h("tr", { class: s.seat === view.seat ? "me" : "" },
    h("td", { text: String(i + 1) }),
    // The tags the terminal standings carry, in their words (#744).
    h("td", { class: "name", text: seatName(view, s.seat) + (s.seat === view.seat ? " (you)" : "") + (s.tags ? ` ${tagText(s.tags)}` : "") }),
    // A drawn match is in the record too: it vanished from "0-0" (#744).
    h("td", { class: "num", text: `${s.wins}-${s.losses}${s.draws ? `-${s.draws}` : ""}` }),
    h("td", { class: "num", text: String(s.points) }),
    // The tiebreakers a tie on points is ordered by, first to last.
    h("td", { class: "num tiebreak", text: tiebreakText(s) }),
  ));
  return h("table", { class: "standings" },
    h("thead", {}, h("tr", {}, ...["#", "seat", "W-L-D", "points"].map(t => h("th", { text: t })),
      h("th", { class: "tiebreak", text: `OMW /${NBSP}GW /${NBSP}OGW${NBSP}%` }))),
    h("tbody", {}, ...rows));
}

/** "55.6 / 85.7 / 47.2": the tiebreakers as percentages, or "" before a
 *  round has counted. Where the three wrap, a line starts with the "/"
 *  rather than ending with it (#881). */
export function tiebreakText(s: { omw?: number; gw?: number; ogw?: number }): string {
  if (!s.omw) return "";
  const pct = (f: number | undefined) => (100 * (f ?? 0)).toFixed(1);
  return `${pct(s.omw)} /${NBSP}${pct(s.gw)} /${NBSP}${pct(s.ogw)}`;
}

function matchRow(view: DraftView, m: Match): HTMLElement {
  // A forfeit is not a game played and won, and an abandoned game is not a
  // draw (#743, #744).
  const who = (s: number) => s === view.seat ? "you" : seatName(view, s);
  const games = m.games.map((g, i) => {
    const won = g.winner === null ? (g.abandoned ? "abandoned" : "drawn") : who(g.winner);
    const forfeit = g.forfeited_by !== undefined && g.forfeited_by !== null ? ` (${who(g.forfeited_by)} forfeited)` : "";
    return `G${i + 1}${NBSP}${won}${forfeit}`;
  }).join(", ");
  const status = m.status === "done" ? `done${m.result ? `, ${tagText(m.result)}` : ""}` : m.status === "playing" ? "playing now" : "waiting to start";
  // The link is this seat's own page. It is null until the match's pages
  // are open (a beat after the pairing), and stays null for a match the
  // table settled without one (a forfeit).
  let link: HTMLElement | null = null;
  if (m.status !== "done") {
    link = m.url
      ? h("div", { class: "match-link" }, "Open your game: ", h("a", { href: m.url, target: "_blank", rel: "noopener" }, ...urlPieces(m.url)))
      : h("div", { class: "match-link muted", text: m.status === "waiting" ? "Your game page opens when the match starts." : "No game page for this match." });
  }
  return h("li", { class: `match match-${m.status}` },
    h("div", { class: "match-head" },
      h("span", {}, ...withRecords(`Round ${m.round} vs ${seatName(view, m.opponent)}${dashed(status)}`))),
    link,
    games ? h("div", { class: "match-games muted", text: games }) : null,
  );
}

/** Every match of the tournament, the whole table's, round by round. */
function pairingsList(view: DraftView): HTMLElement | null {
  const pairings = view.pairings;
  if (!pairings || pairings.length === 0) return null;
  const rows = pairings.map(p => {
    const who = p.b === null ? `${seatName(view, p.a)} has a bye` : `${seatName(view, p.a)} vs ${seatName(view, p.b)}`;
    const how = p.b === null ? "" : dashed(p.status === "done" ? tagText(p.result ?? "done") : p.status === "playing" ? "playing" : "waiting");
    const mine = p.a === view.seat || p.b === view.seat;
    return h("li", { class: mine ? "me" : "" }, ...withRecords(`Round ${p.round}: ${who}${how}`));
  });
  return h("div", { class: "pairings" }, h("h3", { text: "All matches" }), h("ul", { class: "pairings-list" }, ...rows));
}

// ----------------------------------------------------------------- phases

function lobby(view: DraftView): HTMLElement {
  const missing = view.seats.filter(s => s.kind === "human" && !s.joined);
  return h("section", { class: "phase phase-lobby" },
    h("h2", { text: "Lobby" }),
    h("p", { class: "facts" }, `Set ${view.set.toUpperCase()}, ${view.pod_size} seats. You are seat ${view.seat}.`),
    seatsTable(view),
    h("p", { class: "waiting" }, missing.length
      ? `Waiting for ${missing.map(s => s.name).join(", ")} to join. The host can also start without them.`
      : "Everyone is here. The draft starts when the host does."),
    h("p", { class: "muted", text: "Type a name in the box above so the others know who you are." }),
  );
}

function tile(card: PackCard, state: PageState, actions: Actions, n: number): HTMLElement {
  const p = parseLine(card.name, card.line);
  const selected = state.selected === card.index;
  const pending = state.pendingPick && state.pendingPick.index === card.index;
  const el = h("button", {
    class: `tile frame-${colorClass(card)}${selected ? " selected" : ""}${pending ? " pending" : ""}`,
    "data-card-index": String(card.index), type: "button", title: card.text,
    disabled: !!state.pendingPick,
    onclick: () => { if (selected) actions.pick(card.index); else actions.select(card.index); },
    onmouseenter: () => actions.hover(card.index),
    onmouseleave: () => actions.hover(null),
  },
    h("span", { class: "tile-n", text: String(n) }),
    art(card),
    h("div", { class: "tile-name", text: card.name }),
    h("div", { class: "tile-line" }, pips(p.cost), p.pt ? h("span", { class: "pt", text: p.pt }) : null),
    h("div", { class: "tile-type", text: p.typeLine }),
    h("span", { class: `rarity-dot rarity-${card.rarity}`, title: card.rarity }),
    pending ? h("div", { class: "tile-pending", text: "picking…" }) : null,
  );
  return el;
}

function drafting(view: DraftView, state: PageState, actions: Actions): HTMLElement {
  const pack = view.pack;
  const dir = view.pass_direction ? `passing ${view.pass_direction}` : "";
  const status = h("section", { class: "status" });
  const side = h("aside", { class: "side" });
  const grid = h("div", { class: "pack", id: "pack" });
  if (pack) {
    const waiting = pack.waiting > 0 ? dashed(`${pack.waiting} pack${pack.waiting === 1 ? "" : "s"} waiting`) : "";
    status.append(
      h("h2", { id: "pick-line", text: `Pack ${pack.round}, pick ${pack.pick} of ${pack.size}${waiting}` }),
      h("span", { class: "muted", text: dir }),
      h("span", { id: "countdown", class: "countdown" }),
    );
    pack.cards.forEach((c, i) => grid.append(tile(c, state, actions, i + 1)));
    const shown = state.hover !== null ? pack.cards.find(c => c.index === state.hover)
      : state.selected !== null ? pack.cards.find(c => c.index === state.selected) : undefined;
    const detail = h("div", { class: "detail", id: "detail" });
    if (shown) {
      detail.append(cardDetail(shown));
      if (state.selected === shown.index && !state.pendingPick) {
        detail.append(h("button", { class: "primary", id: "pick-button", type: "button", onclick: () => actions.pick(shown.index) }, `Pick ${shown.name}`));
      }
    } else {
      detail.append(h("p", { class: "muted", text: "Click a card to read it; click it again or press Enter to pick it. Digits and arrows move the selection." }));
    }
    side.append(detail);
  } else {
    status.append(h("h2", { id: "pick-line", text: "Waiting for a pack" }), h("span", { class: "muted", text: dir }));
    grid.append(h("p", { class: "muted", text: "Nothing to pick right now; the next pack comes from your neighbour." }));
  }
  side.append(h("div", { class: "pool" }, h("h3", { text: `Pool (${view.pool.length})` }), poolGroups(view.pool)));
  return h("section", { class: "phase phase-drafting" }, status,
    h("div", { class: "draft-layout" }, grid, side));
}

/**
 * The pool as rows, by colour, each in the main deck or the sideboard; a
 * click moves one. The deck build's checklist, and the sideboard's
 * between games. `ready`: the deck is final, nothing moves.
 */
function poolRows(list: HTMLElement, pool: CardInfo[], edit: DeckEdit, state: PageState, actions: Actions, ready: boolean): void {
  const c = counts(edit);
  // The count where the rows are, on a phone: the deck panel's own count
  // line is under the whole pool there, 1,500px from the first row (#792).
  list.append(h("div", { class: "phone-count", id: "phone-count", text: `${c.spells} spells + ${c.lands} lands = ${c.total}` }));
  const groups = groupByColor(pool.map((c, i) => ({ ...c, poolIndex: i })));
  for (const g of groups) {
    list.append(h("h4", { class: `group-head color-${g.key}` }, `${g.label} (${g.cards.length})`));
    for (const c of g.cards) {
      const inMain = edit.main[c.poolIndex];
      const p = parseLine(c.name, c.line);
      list.append(h("button", {
        type: "button", title: c.text,
        class: `row${inMain ? " in-main" : ""}${state.cursor === c.poolIndex ? " cursor" : ""}`,
        "data-pool-index": String(c.poolIndex), disabled: ready,
        // A click with no pointer behind it (`detail` 0) is Enter on the focused row.
        onclick: (e: MouseEvent) => actions.toggle(c.poolIndex, e.detail === 0),
      },
        h("span", { class: "box", text: inMain ? "MAIN" : "side" }),
        pips(p.cost),
        h("span", { class: "row-name", text: c.name }),
        h("span", { class: "row-type muted", text: p.typeLine + (p.pt ? ` ${p.pt}` : "") }),
      ));
      // The card under the cursor, read in place: shown on a phone only,
      // where a tap puts the cursor here and a second tap moves it (#792).
      if (state.cursor === c.poolIndex) {
        list.append(h("div", { class: "detail row-detail", id: "row-detail" }, cardDetail(c),
          ready ? null : h("p", { class: "muted", text: inMain ? "Tap it again to move it to the sideboard." : "Tap it again to move it to the main deck." })));
      }
    }
  }
}

/** A stepper per basic land. */
function landSteppers(edit: DeckEdit, ready: boolean, actions: Actions): HTMLElement {
  const landsEl = h("div", { class: "lands" });
  for (const b of BASICS) {
    const n = edit.lands[b] || 0;
    landsEl.append(h("div", { class: "land", "data-land": b },
      h("button", { type: "button", class: "dec", disabled: ready || n === 0, onclick: () => actions.setLand(b, n - 1), text: "−" }),
      h("span", { class: "n", text: String(n) }),
      h("button", { type: "button", class: "inc", disabled: ready, onclick: () => actions.setLand(b, n + 1), text: "+" }),
      h("span", { class: `land-name color-${"WUBRG"[BASICS.indexOf(b)]}`, text: b }),
    ));
  }
  return landsEl;
}

function building(view: DraftView, state: PageState, actions: Actions): HTMLElement {
  const me = view.seats.find(s => s.seat === view.seat);
  const ready = (me && me.status === "ready") || state.readySent;
  const edit = state.deck;
  const list = h("div", { class: "checklist" });
  const panel = h("aside", { class: "deckpanel" });
  if (!edit) {
    list.append(h("p", { class: "muted", text: "Waiting for the pool." }));
    return h("section", { class: "phase phase-building" }, h("div", { class: "build-layout" }, list, panel));
  }
  const c = counts(edit);
  poolRows(list, view.pool, edit, state, actions, ready);
  const why = problem(edit, view.pool);
  const s = summary(edit, view.pool);
  panel.append(h("h3", { text: ready ? `Deck${dashed("ready")}` : "Deck" }));
  panel.append(h("div", { class: "count-line", id: "count-line", text: `${c.spells} spells + ${c.lands} lands = ${c.total}` }));
  if (!ready && view.build_deadline_ms != null) {
    panel.append(h("div", { class: "build-timer" }, h("span", { id: "countdown", class: "countdown" }),
      h("span", { class: "muted", text: " to build; then the deck as you last sent it is played (basics fill a short one, up to 18 lands), or else the table builds one" })));
  }
  panel.append(landSteppers(edit, ready, actions));
  const colorsEl = h("div", { class: "colors-summary" });
  for (const { key, n } of s.colors) colorsEl.append(h("span", { class: `swatch color-${key}`, text: `${COLOR_NAMES[key as ColorKey]} ${n}` }));
  if (s.colors.length === 0) colorsEl.append(h("span", { class: "muted", text: "No spells in the main deck yet." }));
  panel.append(h("div", { class: "summary" }, h("div", { class: "summary-title", text: `Colours (${s.creatures} creatures)` }), colorsEl));
  const curve = h("div", { class: "curve" });
  const peak = Math.max(1, ...s.curve);
  s.curve.forEach((n, mv) => {
    curve.append(h("div", { class: "bar", title: `${n} at mana value ${mv === 7 ? "7+" : mv}` },
      h("div", { class: "bar-fill", style: `height: ${Math.round((n / peak) * 40)}px` }),
      h("span", { class: "bar-n", text: String(n) }),
      h("span", { class: "bar-mv", text: mv === 7 ? "7+" : String(mv) })));
  });
  panel.append(h("div", { class: "summary" }, h("div", { class: "summary-title", text: "Curve" }), curve));
  if (view.deck && view.deck.valid === false && view.deck.problem) {
    panel.append(h("div", { class: "server-problem", id: "server-problem", text: `The server says: ${view.deck.problem}` }));
  }
  if (ready) {
    panel.append(h("div", { class: "ready-line", id: "ready-line", text: `Ready${dashed("waiting for the others.")}` }));
  } else {
    panel.append(h("button", { type: "button", class: "primary", id: "ready", disabled: !!why, title: why ?? "Send this deck as final", onclick: () => actions.ready() }, "Ready"));
    panel.append(h("div", { class: "ready-reason", id: "ready-reason", text: why ?? `${c.total} cards; the server checks it again.` }));
  }
  const shown = state.cursor !== null ? view.pool[state.cursor] : undefined;
  if (shown) panel.append(h("div", { class: "detail panel-detail" }, cardDetail(shown)));
  else panel.append(h("p", { class: "muted panel-help", text: `Click a card to move it between main and side (on a phone, tap to read it and tap again to move it); at least ${MIN_DECK} cards with lands. Arrows and Enter work too.` }));
  return h("section", { class: "phase phase-building" }, h("div", { class: "build-layout" }, list, panel),
    reviewSection(state, actions));
}

/**
 * The draft review: every pack this seat was handed, by pack, with what it
 * took first and the rest of the pack after it. Asked for when opened, and
 * only once the seat's own draft is over.
 */
function reviewSection(state: PageState, actions: Actions): HTMLElement {
  const parts: Child[] = [h("button", { type: "button", id: "review-toggle", onclick: () => actions.toggleReview() },
    state.reviewOpen ? "Hide your picks" : "Review your picks")];
  if (state.reviewOpen) {
    if (!state.review) parts.push(h("p", { class: "muted", text: "Asking for your picks…" }));
    else {
      const byPack = new Map<number, ReviewPick[]>();
      for (const p of state.review) byPack.set(p.round, [...(byPack.get(p.round) ?? []), p]);
      for (const [round, picks] of byPack) {
        parts.push(h("h4", { text: `Pack ${round}` }));
        // Unnumbered: each row begins with its pick ("3.4"), and an ordered
        // list put a second counter before it ("4. 3.4 Rage Thrower", #889).
        // Every card in a row is a button that reads it in place, under the
        // row: a hover tooltip was the only reading, and a phone has no
        // hover (#889).
        parts.push(h("ul", { class: "review-list" }, ...picks.map(p => {
          const at = `${p.round}.${p.pick}`;
          const takenAt = p.pack.findIndex(c => c.name === p.card);
          const card = (i: number) => {
            const key = `${at}:${i}`;
            return h("button", { type: "button", class: `review-card${state.reviewRead === key ? " reading" : ""}`, "data-review-card": key,
              onclick: () => actions.readReview(key) }, p.pack[i].name);
          };
          const rest = p.pack.map((_, i) => i).filter(i => i !== takenAt);
          const over: Child[] = [];
          rest.forEach((i, n) => { if (n > 0) over.push(", "); over.push(card(i)); });
          const read = state.reviewRead !== null && state.reviewRead.startsWith(`${at}:`) ? p.pack[Number(state.reviewRead.slice(at.length + 1))] : undefined;
          return h("li", { "data-pick": at },
            h("span", { class: "review-pick" }, `${at} `, takenAt >= 0 ? card(takenAt) : p.card),
            p.auto ? h("span", { class: "muted", text: " (the table's pick)" }) : null,
            rest.length ? h("span", { class: "muted review-rest" }, `${dashed("over")} `, ...over) : null,
            read ? h("div", { class: "detail review-detail" }, cardDetail(read)) : null);
        })));
      }
    }
  }
  return h("section", { class: "review", id: "review" }, h("h3", { text: "Your draft" }), ...parts);
}

/** This seat's byes, from the table's pairings: `matches` holds played
 *  matches only, so a seat given a bye read "No pairings yet" (#814). */
export function ownByes(view: DraftView): number[] {
  return (view.pairings || []).filter(p => p.a === view.seat && p.b === null).map(p => p.round);
}

/** "you lead 1-0", "your opponent leads 1-0", "1-1": the match so far. */
export function matchScore(view: DraftView, m: Match): string {
  const mine = m.games.filter(g => g.winner === view.seat).length;
  const theirs = m.games.filter(g => g.winner === m.opponent).length;
  if (mine === theirs) return `${mine}-${theirs}`;
  return mine > theirs ? `you lead ${mine}-${theirs}` : `your opponent leads ${theirs}-${mine}`;
}

/**
 * Before a game of a match: the choice to play or draw, for the seat that
 * has it (the coin flip's winner for game 1, the last game's loser after,
 * MTR 2.2), and from game 2 the deck, sideboarded from the pool (2.4),
 * beside the cards the opponent has shown. The next game starts when both
 * seats are ready, or the host's sideboard timer runs out.
 */
function betweenPanel(view: DraftView, m: Match, b: Between, state: PageState, actions: Actions): HTMLElement {
  const opponent = seatName(view, m.opponent);
  const ready = b.ready || state.readySent;
  const parts: Child[] = [
    h("h3", { id: "between-head" }, ...withRecords(`Before game ${b.game} vs ${opponent}${b.game > 1 ? dashed(matchScore(view, m)) : ""}`)),
  ];
  if (!ready && b.deadline_ms !== null) {
    parts.push(h("div", { class: "build-timer" }, h("span", { id: "countdown", class: "countdown" }),
      h("span", { class: "muted", text: dashed("then the game starts with the deck as it stands") })));
  }
  if (b.you_choose) {
    // The last game's loser chooses; after a draw, whoever chose it (MTR 2.2).
    const last = m.games[m.games.length - 1];
    const why = b.game === 1 ? "You won the coin flip"
      : last && last.winner === m.opponent ? `You lost game ${b.game - 1}` : `Game ${b.game - 1} was a draw, and the choice is still yours`;
    const chosen = b.play_first;
    parts.push(h("div", { class: "play-draw", id: "play-draw" },
      h("span", { text: `${why}: play first or draw first? ` }),
      h("button", { type: "button", id: "play-first", class: chosen === true ? "primary" : "", disabled: ready, onclick: () => actions.playDraw(true) }, "Play first"),
      " ",
      h("button", { type: "button", id: "draw-first", class: chosen === false ? "primary" : "", disabled: ready, onclick: () => actions.playDraw(false) }, "Draw first"),
      h("span", { class: "muted", text: chosen === null ? " (not chosen yet: you play first)" : chosen ? dashed("you play first") : dashed("you draw first") })));
  } else {
    parts.push(h("p", { class: "muted", id: "play-draw", text: b.game === 1
      ? `${opponent} won the coin flip and chooses whether to play or draw.`
      : `${opponent} ${m.games.length && m.games[m.games.length - 1].winner === view.seat ? `lost game ${b.game - 1}` : "chose last game"} and chooses whether to play or draw.` }));
  }
  const edit = state.deck;
  const why = b.sideboarding && edit ? problem(edit, view.pool) : null;
  // Ready, beside the deck when there is one (the pool is long), else
  // under the choice.
  const readyEl = ready
    ? h("div", { class: "ready-line", id: "ready-line", text: b.opponent_ready
      ? `Ready${dashed(`game ${b.game} is starting.`)}` : `Ready${dashed(`waiting for ${opponent}.`)}` })
    : h("div", {},
      h("button", { type: "button", class: "primary", id: "sideboard-ready", disabled: !!why, title: why ?? `Start game ${b.game}`, onclick: () => actions.sideboardReady() }, `Ready for game ${b.game}`),
      h("div", { class: "ready-reason", id: "ready-reason", text: why ?? (b.opponent_ready ? `${opponent} is ready.` : `${opponent} is still deciding.`) }));
  if (!(b.sideboarding && edit)) parts.push(readyEl);
  if (b.sideboarding && edit) {
    const c = counts(edit);
    const list = h("div", { class: "checklist" });
    poolRows(list, view.pool, edit, state, actions, ready);
    const side = h("aside", { class: "deckpanel" },
      h("div", { class: "count-line", id: "count-line", text: `${c.spells} spells + ${c.lands} lands = ${c.total}` }),
      landSteppers(edit, ready, actions),
      readyEl);
    if (b.opponent_showed.length) {
      side.append(h("div", { class: "summary", id: "opponent-showed" },
        h("div", { class: "summary-title", text: `${opponent} has shown (${b.opponent_showed.length})` }),
        h("ul", { class: "pool-list" }, ...b.opponent_showed.map(card => {
          const p = parseLine(card.name, card.line);
          return h("li", { title: card.text }, pips(p.cost), " ", h("span", { class: "pool-name", text: card.name }));
        }))));
    }
    const shown = state.cursor !== null ? view.pool[state.cursor] : undefined;
    if (shown) side.append(h("div", { class: "detail panel-detail" }, cardDetail(shown)));
    parts.push(h("p", { class: "muted", text: "Sideboarding: move cards between your deck and your sideboard; any deck of the pool with 40 or more cards." }),
      h("div", { class: "build-layout" }, list, side));
  }
  return h("section", { class: "between", id: "between" }, ...parts);
}

function playing(view: DraftView, done: boolean, state: PageState, actions: Actions): HTMLElement {
  const mine = view.matches.filter(m => m.status !== "done");
  const me = view.seats.find(s => s.seat === view.seat);
  const byes = ownByes(view);
  const sections: HTMLElement[] = [h("h2", { text: done ? "Draft over" : "Playing" })];
  if (!done) {
    // The round in progress is the latest one paired; a bye in it is why
    // this seat has nothing to play.
    const latest = Math.max(0, ...(view.pairings || []).map(p => p.round));
    sections.push(h("div", { class: "waiting" },
      mine.length === 0
        ? (byes.includes(latest) ? `You have a bye in round ${latest}. Waiting for the others to finish their matches.` : "Waiting for the others to finish their matches.")
        : me && me.status === "waiting" ? "Waiting for your opponent." : "Your match is on."));
    const m = view.matches.find(x => x.between);
    if (m && m.between) sections.push(betweenPanel(view, m, m.between, state, actions));
  }
  const rows: { round: number; el: HTMLElement }[] = [
    ...view.matches.map(m => ({ round: m.round, el: matchRow(view, m) })),
    ...byes.map(round => ({ round, el: h("li", { class: "match match-bye" },
      h("div", { class: "match-head", text: `Round ${round}: a bye${dashed("counted as a match won")}` })) })),
  ].sort((a, b) => a.round - b.round);
  sections.push(h("h3", { text: "Your matches" }),
    rows.length ? h("ul", { class: "matches" }, ...rows.map(r => r.el)) : h("p", { class: "muted", text: "No pairings yet." }));
  sections.push(h("h3", { text: done ? "Final standings" : standingsHeading(view.standings_after) }), standingsTable(view));
  const all = pairingsList(view);
  if (all) sections.push(all);
  sections.push(h("h3", { text: "Seats" }), seatsTable(view));
  if (view.deck) {
    sections.push(reviewSection(state, actions));
    sections.push(h("h3", { text: `Your deck (${view.deck.main.length + Object.values(view.deck.lands).reduce((a, b) => a + b, 0)})` }),
      h("p", { class: "decklist" }, [...view.deck.main, ...Object.entries(view.deck.lands).filter(([, n]) => n > 0).map(([l, n]) => `${n} ${l}`)].join(", ")));
  }
  return h("section", { class: `phase phase-${done ? "done" : "playing"}` }, ...sections);
}

// ------------------------------------------------------------------ page

export function render(root: HTMLElement, state: PageState, actions: Actions): void {
  const parts: Child[] = [];
  const view = state.view;
  // The table is over: said first and for good, as nothing will reconnect.
  if (state.closed !== null) {
    parts.push(h("div", { class: "banner refusal", role: "alert", text: `The table is closed: ${state.closed}.` }));
  }
  if (state.refusal) {
    parts.push(h("div", { class: "banner refusal", role: "alert" },
      h("span", { text: `Refused: ${state.refusal.reason}` }),
      h("button", { type: "button", class: "dismiss", onclick: () => actions.dismiss(), text: "×", title: "Dismiss" })));
  }
  if (state.notice) {
    parts.push(h("div", { class: "banner notice", role: "status" },
      h("span", { text: state.notice }),
      h("button", { type: "button", class: "dismiss", onclick: () => actions.dismissNotice(), text: "×", title: "Dismiss" })));
  }
  if (state.hint) parts.push(h("div", { class: "banner hint", text: state.hint }));
  if (!view) {
    parts.push(h("section", { class: "phase" }, h("p", { class: "muted", text: state.connected ? "Connected; waiting for the first view." : "Connecting to the draft…" })));
  } else {
    switch (view.phase) {
      case "lobby": parts.push(lobby(view)); break;
      case "drafting": parts.push(drafting(view, state, actions)); break;
      case "building": parts.push(building(view, state, actions)); break;
      case "playing": parts.push(playing(view, false, state, actions)); break;
      case "done": parts.push(playing(view, true, state, actions)); break;
      default: parts.push(h("section", { class: "phase" }, h("p", { text: `Unknown phase ${String((view as DraftView).phase)}; here is what the server sent.` }), h("pre", { text: JSON.stringify(view, null, 1) })));
    }
  }
  root.replaceChildren(...parts.filter((p): p is HTMLElement => !!p));
}

/** The number of tiles in the pack grid's first row, from the layout. */
export function packColumns(root: HTMLElement): number {
  const tiles = [...root.querySelectorAll<HTMLElement>(".tile")];
  if (tiles.length === 0) return 1;
  const top = tiles[0].offsetTop;
  return Math.max(1, tiles.filter(t => t.offsetTop === top).length);
}

/**
 * A URL as text pieces with a line break allowed only before a ':' or '/'
 * and after the '//': at phone width `word-break: break-all` split it at
 * any character, and "HTTP://192.168.1.20:8" / "805/" is the wrong port
 * for a person copying it to another device (#794).
 */
export function urlPieces(url: string): (string | HTMLElement)[] {
  const out: (string | HTMLElement)[] = [];
  for (const part of url.split(/(?=[:/])|(?<=\/\/)/)) {
    if (out.length) out.push(document.createElement("wbr"));
    out.push(part);
  }
  return out;
}

/**
 * Text with each match record ("2-1", "1-0-1") in a span that does not
 * wrap: the standings table's records broke at their hyphens at phone
 * width (#790), and so did the same records in a match's heading and the
 * pairings list.
 */
export function withRecords(text: string): (string | HTMLElement)[] {
  return text.split(/(\b\d+(?:-\d+)+\b)/).filter(t => t !== "")
    .map(t => /^\d+(?:-\d+)+$/.test(t) ? h("span", { class: "rec", text: t }) : t);
}

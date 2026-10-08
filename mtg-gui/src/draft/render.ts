// The draft page's DOM, rebuilt from the state on every change.
//
// One function per phase. Nothing here keeps state between renders: a
// view is the whole seat, so the page can be thrown away and rebuilt
// from it, which is what a reconnect and a refusal both rely on.

import { artFor, groupByColor, initials, parseLine, COLOR_NAMES, type ColorKey } from "./cards.js";
import { BASICS, MIN_DECK, counts, problem, summary } from "./deck.js";
import type { CardInfo, DraftView, Match, PackCard, SeatInfo } from "./protocol.js";
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

function seatName(view: DraftView, seat: number): string {
  const s = view.seats.find(x => x.seat === seat);
  if (!s) return `seat ${seat}`;
  return s.name === `seat ${seat}` ? s.name : `${s.name} (seat ${seat})`;
}

function seatsTable(view: DraftView): HTMLElement {
  const rows = view.seats.map((s: SeatInfo) => h("tr", { class: s.seat === view.seat ? "me" : "" },
    h("td", { text: String(s.seat) }),
    h("td", { text: s.name + (s.seat === view.seat ? " (you)" : "") }),
    h("td", { text: s.kind }),
    h("td", { text: s.joined ? "here" : "not yet" }),
    h("td", { text: s.status }),
    h("td", { class: "num", text: String(s.picks) }),
  ));
  return h("table", { class: "seats" },
    h("thead", {}, h("tr", {}, ...["seat", "name", "kind", "joined", "status", "picks"].map(t => h("th", { text: t })))),
    h("tbody", {}, ...rows));
}

function standingsTable(view: DraftView): HTMLElement {
  if (view.standings.length === 0) return h("p", { class: "muted", text: "No results yet." });
  const rows = view.standings.map((s, i) => h("tr", { class: s.seat === view.seat ? "me" : "" },
    h("td", { text: String(i + 1) }),
    h("td", { text: seatName(view, s.seat) + (s.seat === view.seat ? " (you)" : "") }),
    h("td", { class: "num", text: `${s.wins}-${s.losses}` }),
    h("td", { class: "num", text: String(s.points) }),
  ));
  return h("table", { class: "standings" },
    h("thead", {}, h("tr", {}, ...["#", "seat", "W-L", "points"].map(t => h("th", { text: t })))),
    h("tbody", {}, ...rows));
}

function matchRow(view: DraftView, m: Match): HTMLElement {
  const games = m.games.map((g, i) => `G${i + 1} ${g.winner === view.seat ? "you" : seatName(view, g.winner)}`).join(", ");
  const status = m.status === "done" ? `done${m.result ? `, ${m.result}` : ""}` : m.status === "playing" ? "playing now" : "waiting to start";
  return h("li", { class: `match match-${m.status}` },
    h("div", { class: "match-head" },
      h("span", { text: `Round ${m.round} vs ${seatName(view, m.opponent)} — ${status}` })),
    m.status !== "done" ? h("div", { class: "match-link" }, "Open your game: ",
      h("a", { href: m.url, target: "_blank", rel: "noopener", text: m.url })) : null,
    games ? h("div", { class: "match-games muted", text: games }) : null,
  );
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
    const waiting = pack.waiting > 0 ? ` — ${pack.waiting} pack${pack.waiting === 1 ? "" : "s"} waiting` : "";
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
  const groups = groupByColor(view.pool.map((c, i) => ({ ...c, poolIndex: i })));
  for (const g of groups) {
    list.append(h("h4", { class: `group-head color-${g.key}` }, `${g.label} (${g.cards.length})`));
    for (const c of g.cards) {
      const inMain = edit.main[c.poolIndex];
      const p = parseLine(c.name, c.line);
      list.append(h("button", {
        type: "button", title: c.text,
        class: `row${inMain ? " in-main" : ""}${state.cursor === c.poolIndex ? " cursor" : ""}`,
        "data-pool-index": String(c.poolIndex), disabled: ready,
        onclick: () => actions.toggle(c.poolIndex),
      },
        h("span", { class: "box", text: inMain ? "MAIN" : "side" }),
        pips(p.cost),
        h("span", { class: "row-name", text: c.name }),
        h("span", { class: "row-type muted", text: p.typeLine + (p.pt ? ` ${p.pt}` : "") }),
      ));
    }
  }
  const c = counts(edit);
  const why = problem(edit, view.pool);
  const s = summary(edit, view.pool);
  panel.append(h("h3", { text: ready ? "Deck — ready" : "Deck" }));
  panel.append(h("div", { class: "count-line", id: "count-line", text: `${c.spells} spells + ${c.lands} lands = ${c.total}` }));
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
  panel.append(landsEl);
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
    panel.append(h("div", { class: "ready-line", id: "ready-line", text: "Ready — waiting for the others." }));
  } else {
    panel.append(h("button", { type: "button", class: "primary", id: "ready", disabled: !!why, title: why ?? "Send this deck as final", onclick: () => actions.ready() }, "Ready"));
    panel.append(h("div", { class: "ready-reason", id: "ready-reason", text: why ?? `${c.total} cards; the server checks it again.` }));
  }
  const shown = state.cursor !== null ? view.pool[state.cursor] : undefined;
  if (shown) panel.append(h("div", { class: "detail" }, cardDetail(shown)));
  else panel.append(h("p", { class: "muted", text: `Click a card to move it between main and side; at least ${MIN_DECK} cards with lands. Arrows and Enter work too.` }));
  return h("section", { class: "phase phase-building" }, h("div", { class: "build-layout" }, list, panel));
}

function playing(view: DraftView, done: boolean): HTMLElement {
  const mine = view.matches.filter(m => m.status !== "done");
  const me = view.seats.find(s => s.seat === view.seat);
  const sections: HTMLElement[] = [h("h2", { text: done ? "Draft over" : "Playing" })];
  if (!done) {
    sections.push(h("div", { class: "waiting" },
      mine.length === 0 ? "Waiting for the others to finish their matches."
        : me && me.status === "waiting" ? "Waiting for your opponent." : "Your match is on."));
  }
  sections.push(h("h3", { text: "Your matches" }),
    view.matches.length ? h("ul", { class: "matches" }, ...view.matches.map(m => matchRow(view, m))) : h("p", { class: "muted", text: "No pairings yet." }));
  sections.push(h("h3", { text: done ? "Final standings" : "Standings" }), standingsTable(view));
  sections.push(h("h3", { text: "Seats" }), seatsTable(view));
  if (view.deck) {
    sections.push(h("h3", { text: `Your deck (${view.deck.main.length + Object.values(view.deck.lands).reduce((a, b) => a + b, 0)})` }),
      h("p", { class: "decklist" }, [...view.deck.main, ...Object.entries(view.deck.lands).filter(([, n]) => n > 0).map(([l, n]) => `${n} ${l}`)].join(", ")));
  }
  return h("section", { class: `phase phase-${done ? "done" : "playing"}` }, ...sections);
}

// ------------------------------------------------------------------ page

export function render(root: HTMLElement, state: PageState, actions: Actions): void {
  const parts: Child[] = [];
  const view = state.view;
  if (state.refusal) {
    parts.push(h("div", { class: "banner refusal", role: "alert" },
      h("span", { text: `Refused: ${state.refusal.reason}` }),
      h("button", { type: "button", class: "dismiss", onclick: () => actions.dismiss(), text: "×", title: "Dismiss" })));
  }
  if (view && view.notice) parts.push(h("div", { class: "banner notice", text: view.notice }));
  if (state.hint) parts.push(h("div", { class: "banner hint", text: state.hint }));
  if (!view) {
    parts.push(h("section", { class: "phase" }, h("p", { class: "muted", text: state.connected ? "Connected; waiting for the first view." : "Connecting to the draft…" })));
  } else {
    switch (view.phase) {
      case "lobby": parts.push(lobby(view)); break;
      case "drafting": parts.push(drafting(view, state, actions)); break;
      case "building": parts.push(building(view, state, actions)); break;
      case "playing": parts.push(playing(view, false)); break;
      case "done": parts.push(playing(view, true)); break;
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

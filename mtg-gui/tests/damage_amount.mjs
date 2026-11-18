// The combat damage division widget says where the rest goes (#720).
//
//   node mtg-gui/tests/damage_amount.mjs
//
// The engine's ~150-character description says it only at its end, which
// the modal's two title lines cut off, and the option labels — each of which
// says it — were never drawn. "p1" was never put in the page's words.

globalThis.Image ??= class { };
globalThis.document ??= { createElement: () => ({ getContext: () => null }) };
const P = await import("../dist/prompts.js");

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };

const view = { you: 0, opponents: [{ id: 1 }], battlefield: [
    { object_id: 40, name: "Kessig Wolf Run Boar", controller: 0 }, { object_id: 62, name: "Grizzly Bears", controller: 1 }],
  your_hand: [], stack: [], exile: [], graveyards: [], your_library_cards: [], revealed_names: {} };
const state = { view, index: P.indexView(view), overlay: null };
const desc = "Combat damage from Kessig Wolf Run Boar (#40) (5 power, CR 510.1c): how much of the 5 left goes to Grizzly Bears (#62)? At least 2 (lethal), at most 5; the rest goes on to p1";
const rp = { description: desc, attacker: 40, blocker: 62, min: 2, max: 5,
  options: ["2 to Grizzly Bears (#62) (lethal), 3 on to p1", "3 to Grizzly Bears (#62), 2 on to p1",
            "4 to Grizzly Bears (#62), 1 on to p1", "5 to Grizzly Bears (#62), 0 on to p1"] };
const ui = { mode: "menu", title: "", hint: "", buttons: [], marked: [] };
P.beginDamageAmount(state, ui, rp, desc, () => {});

if (ui.title !== "Damage from Kessig Wolf Run Boar to Grizzly Bears") fail(`a short title: ${ui.title}`);
const s = ui.summary.join("\n");
if (!/the rest goes on to opp/.test(s)) fail(`says where the rest goes: ${s}`);
if (ui.summary.length !== 5 || ui.summary[1] !== "2 to Grizzly Bears (lethal), 3 on to opp") fail(`every option, in our words: ${JSON.stringify(ui.summary)}`);
if (/p1|#\d/.test(s + ui.title)) fail(`no engine words: ${s}`);

// #1017: what the refusal names is what was typed. 21 or more digits went
// through Number() and were refused as "1e+21 is more than is left".
const BIG = "9".repeat(25);
for (const [typed, says] of [[BIG, `'${BIG}' is more than is left`], ["9", "9 is more than is left"],
                             ["+9", "'+9' is more than is left"], ["1", "1 is less than lethal"]]) {
  const st = { ...state, notice: null };
  const u = { mode: "menu", title: "", hint: "", buttons: [], marked: [] };
  P.beginDamageAmount(st, u, rp, desc, () => {});
  u.value = typed; u.submit();
  if (!(st.notice || "").startsWith(says)) fail(`${typed}: the refusal names it as typed: ${st.notice}`);
  if (/e\+/.test(st.notice || "")) fail(`${typed}: no number the player never typed: ${st.notice}`);
}

if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: the damage division says where the rest goes");

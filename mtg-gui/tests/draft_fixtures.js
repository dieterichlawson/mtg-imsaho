// Every fixture view has the fields the draft page's protocol declares.
//
//   node mtg-gui/tests/draft_fixtures.js
//
// No browser. `tests/draft-page-fixtures.json` is the page's own set, one
// view per phase, written from the plan; `tests/draft-view-fixtures.json`
// is the server's, written by a Rust test from the views it really sends,
// and is checked the same way when it exists (skipped, and said so, when
// it does not). The shapes come from `dist/draft/protocol.js`, which is
// what the page reads views through, so a field the server renames shows
// up here before a person finds a blank panel.

const path = require("path");
const fs = require("fs");

let failures = 0;
const fail = (m) => { console.error("FAIL: " + m); failures++; };
const ok = (m) => console.log("ok: " + m);

/** The views in a fixture file of any of the shapes a writer might pick:
 *  {views: {name: view}}, {name: view}, or [view, ...]; and its refusals,
 *  under `refused` (one) or `refusals` (several). */
function collect(data) {
  const views = [];
  const isView = v => v && typeof v === "object" && typeof v.phase === "string";
  if (Array.isArray(data)) data.forEach((v, i) => { if (isView(v)) views.push([`${v.phase}-${i}`, v]); });
  else if (data && typeof data === "object") {
    const source = data.views && typeof data.views === "object" ? data.views : data;
    for (const [k, v] of Object.entries(source)) if (isView(v)) views.push([k, v]);
  }
  const refusals = [];
  if (data && !Array.isArray(data)) {
    if (data.refused) refusals.push(["refused", data.refused]);
    if (Array.isArray(data.refusals)) data.refusals.forEach((r, i) => refusals.push([`refusals[${i}]`, r]));
  }
  return { views, refusals };
}

async function main() {
  const { checkShape, PHASES } = await import("file://" + path.join(__dirname, "..", "dist", "draft", "protocol.js"));
  const files = [
    ["draft-page-fixtures.json", true],
    ["draft-view-fixtures.json", false],
  ];
  for (const [file, required] of files) {
    const full = path.join(__dirname, file);
    if (!fs.existsSync(full)) {
      if (required) fail(`${file} is missing`); else console.log(`skip: ${file} not present (the server's test writes it)`);
      continue;
    }
    const data = JSON.parse(fs.readFileSync(full, "utf8"));
    const { views, refusals } = collect(data);
    if (views.length === 0) { fail(`${file}: no views found in it`); continue; }
    const phases = new Set();
    for (const [name, view] of views) {
      const problems = checkShape(view, "view");
      if (problems.length) fail(`${file} ${name}:\n  ${problems.join("\n  ")}`);
      else ok(`${file} ${name} (${view.phase}) has every field the page reads`);
      phases.add(view.phase);
      // What the phase promises: a pack while drafting, matches once playing.
      if (view.phase === "drafting" && !view.pack) console.log(`note: ${file} ${name} is drafting with no pack (a seat between packs)`);
      if (view.pack) {
        const bad = view.pack.cards.filter((c, i) => c.index !== i);
        if (bad.length) fail(`${file} ${name}: pack.cards[i].index must be i (the page sends it as the pick)`);
      }
    }
    const missing = PHASES.filter(p => !phases.has(p));
    if (missing.length) fail(`${file}: no view for phase(s) ${missing.join(", ")}`);
    else ok(`${file} covers every phase`);
    for (const [name, r] of refusals) {
      const problems = checkShape(r, "refused");
      if (problems.length) fail(`${file} ${name}:\n  ${problems.join("\n  ")}`); else ok(`${file} ${name} is a refusal the page can show`);
    }
  }
  if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
  console.log("draft fixtures: all checks passed");
}

main().catch(e => { console.error(e); process.exit(1); });

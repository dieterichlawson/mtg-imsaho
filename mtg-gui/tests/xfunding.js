// The page's X-funding allocator against the engine's own answers.
//
// `mtg-gui/src/prompts.ts` carries a copy of `mtg-engine/src/funding.rs`,
// because the page builds its own `Action` and cannot call the engine. A
// copy is what #404 and #561 are: a fix that went into one request path and
// not the other. `mtg-gui/tests/x-funding-cases.jsonl` is written by
// `mtg-player/tests/gui_protocol.rs` from the engine, so this fails when the
// page's copy disagrees with it, and the Rust side fails when the engine
// moves and the fixture does not.
//
// No browser and no runner: the helpers are pure, so this is `node
// mtg-gui/tests/xfunding.js`.
"use strict";

const fs = require("fs");
const path = require("path");

const root = path.resolve(__dirname, "..");
const fixture = path.join(__dirname, "x-funding-cases.jsonl");

(async () => {
  const prompts = await import("file://" + path.join(root, "dist", "prompts.js"));
  const lines = fs.readFileSync(fixture, "utf8").split("\n").filter(l => l.trim() !== "");
  const header = JSON.parse(lines.shift());
  if (!header.generated_by) {
    throw new Error(`${fixture}: no header line — regenerate it from the engine`);
  }
  if (lines.length === 0) {
    throw new Error(`${fixture}: no cases — regenerate it from the engine`);
  }

  let checks = 0;
  const fail = (board, what, got, want) => {
    throw new Error(
      `${board}: ${what}\n  page:   ${JSON.stringify(got)}\n  engine: ${JSON.stringify(want)}\n` +
      `The page's allocator in mtg-gui/src/prompts.ts no longer agrees with ` +
      `mtg-engine/src/funding.rs. Port the change, rebuild (cd mtg-gui && npx tsc -p .).`);
  };

  for (const line of lines) {
    const { board, options, fundable, allocations } = JSON.parse(line);
    const mine = prompts.fundableXValues(options);
    if (JSON.stringify(mine) !== JSON.stringify(fundable)) {
      fail(board, "the payable set differs", mine, fundable);
    }
    checks++;
    for (const want of allocations) {
      const { response, shortfall } = prompts.allocateForX(options, want.x);
      // Only the nonzero entries are the allocation; the engine's maps omit
      // the zeroes, and so must the comparison.
      const trim = (o) => Object.fromEntries(Object.entries(o).filter(([, n]) => n !== 0).sort());
      const got = { pool: trim(response.pool), taps: trim(response.taps), shortfall };
      const exp = { pool: trim(want.pool), taps: trim(want.taps), shortfall: want.shortfall };
      if (JSON.stringify(got) !== JSON.stringify(exp)) {
        fail(board, `the allocation for X = ${want.x} differs`, got, exp);
      }
      // And the invariant behind the whole fixture: an announced X that the
      // engine says is payable must come back funded to the point (#593),
      // and one it says is not must be the one the page refuses (#594).
      const funded = Object.values(got.pool).reduce((a, n) => a + n, 0)
        + Object.values(got.taps).reduce((a, n) => a + n, 0);
      const target = Math.max(0, want.x - (options.x_discount || 0));
      if (fundable.includes(want.x) && funded !== target) {
        fail(board, `X = ${want.x} is payable but the page funded ${funded}`, funded, target);
      }
      checks++;
    }
  }
  console.log(`xfunding: ${checks} checks over ${lines.length} boards — the page and the engine agree`);
})().catch((e) => { console.error(String(e.message || e)); process.exit(1); });

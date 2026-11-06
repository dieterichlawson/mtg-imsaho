// The drivers read their flags the way the runners do (#943): the last of
// a repeated flag wins, and a value or a flag a driver cannot run with is
// refused at once with exit 2, where they were read as NaN or as nothing
// and the run ended "ok".
//
//   node mtg-gui/tests/driver_args.mjs

import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const children = path.join(path.dirname(fileURLToPath(import.meta.url)), "children.js");
const spec = { "--games": { kind: "int", min: 1, def: 1 }, "--ai": { kind: "string", def: "stub" }, "--hours": { kind: "number", def: 4 } };
const run = (...argv) => spawnSync(process.execPath, ["-e",
  `const { parseArgs } = require(${JSON.stringify(children)}); console.log(JSON.stringify(parseArgs(${JSON.stringify(spec)}, ${JSON.stringify(argv)})))`],
  { encoding: "utf8" });

let failures = 0;
const fail = m => { console.error("FAIL: " + m); failures++; };

const read = (...argv) => { const r = run(...argv); return r.status === 0 ? JSON.parse(r.stdout) : null; };
const ok = read("--ai", "cc", "--games", "2", "--ai", "stub", "--hours", "0.5");
if (!ok || ok["--ai"] !== "stub" || ok["--games"] !== 2 || ok["--hours"] !== 0.5) fail(`last wins, values read: ${JSON.stringify(ok)}`);
const defaults = read();
if (!defaults || defaults["--games"] !== 1 || defaults["--ai"] !== "stub") fail(`defaults: ${JSON.stringify(defaults)}`);
for (const [argv, said] of [
  [["--games", "0"], "--games 0: it takes a whole number of at least 1"],
  [["--games", "four"], "--games four"],
  [["--games"], "--games needs a value"],
  [["--games", "--ai", "x"], "--games needs a value"],
  [["--gmaes", "3"], "unknown flag \"--gmaes\""],
  [["--hours", "-1"], "--hours -1"],
]) {
  const r = run(...argv);
  if (r.status !== 2 || !r.stderr.includes(said)) fail(`${argv.join(" ")}: exit ${r.status}, ${JSON.stringify(r.stderr.trim())}`);
}
if (failures) { console.error(`${failures} failure(s)`); process.exit(1); }
console.log("ok: driver flags are read last-wins, and refused when a driver cannot run with them");

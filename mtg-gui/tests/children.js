// The processes a GUI driver starts, and how the driver ends.
//
// Every driver spawns a runner or a server and drives it in Chromium, and
// two things went wrong the same way in all of them (#941, #942):
//
// - Each picked a random port in a band the others shared and opened the
//   page there without asking whether its own runner had bound it. When
//   the port was taken the runner died with "Address already in use" and
//   the driver played whatever held the port, and passed. `startRunner`
//   puts each gui seat on port 0, the system's choice of a free one, and
//   opens only the URL the runner itself prints for it; a runner that
//   exits first is the driver's failure, with what it said.
// - A failure outside the `try` whose `finally` killed the runner (no
//   Chromium, a throw before the page opened, a Ctrl-C) left it running,
//   which kept node alive after FAIL or orphaned it on the port. Every
//   child spawned here is killed when the driver exits however it exits,
//   and `finish` exits the driver once `main` has settled, whatever is
//   still holding the event loop.

const { spawn } = require("child_process");
const path = require("path");

const root = path.resolve(__dirname, "..", "..");
const owned = new Set();
// Every runner `startRunner` started, for `finish` to name one that died
// under a driver whose own wait then blamed the page (#943's review).
const runners = new Set();

process.on("exit", () => {
  for (const child of owned) {
    if (child.exitCode === null && child.signalCode === null) child.kill("SIGKILL");
  }
});
// Not SIGHUP: a handler would end a `nohup` run when its terminal closes.
for (const [sig, code] of [["SIGINT", 130], ["SIGTERM", 143]]) {
  process.on(sig, () => { console.error(`FAIL: stopped by ${sig}`); process.exit(code); });
}

/** `spawn`, with the child killed when this process exits. */
function spawnOwned(cmd, args, opts) {
  const child = spawn(cmd, args, opts);
  owned.add(child);
  child.on("exit", () => owned.delete(child));
  return child;
}

/**
 * `target/debug/mtg-runner` with `args`, every `gui` seat in them on a
 * port the system picks (`gui:0`), once each has said where its page is.
 * Resolves to `{ runner, urls, output, exited }`: `urls` maps each gui
 * seat's flag (`--p1`, `--p2`) to its page, `output()` is everything the
 * runner has written to stdout and stderr, `exited()` its exit (null while
 * it runs), `gone` a promise of that exit. Rejects with the runner's output if it exits, or has not
 * named every page in 30 s.
 */
function startRunner(args, { ms = 30000 } = {}) {
  const seats = [];
  for (let i = 0; i + 1 < args.length; i++) {
    if (/^--p[12]$/.test(args[i]) && /^gui(:|$)/.test(args[i + 1])) {
      if (args[i + 1] !== "gui:0") throw new Error(`${args[i]} ${args[i + 1]}: a driver's gui seat is gui:0, the port the system picks`);
      seats.push(args[i]);
    }
  }
  const runner = spawnOwned(path.join(root, "target", "debug", "mtg-runner"), args,
    { cwd: root, stdio: ["ignore", "pipe", "pipe"] });
  let out = "";
  let exit = null;
  runner.stdout.on("data", d => { out += d; });
  runner.stderr.on("data", d => { out += d; });
  // `gone` settles when the runner exits, at any time: a wait on the page
  // raced against it fails with what the runner said, where it used to sit
  // out its whole timeout and blame the page ("no decision for 60s") for a
  // runner that had refused its deck at once (#943).
  let gone;
  const handle = { runner, urls: {}, output: () => out, exited: () => exit, gone: new Promise(r => { gone = r; }) };
  runners.add(handle);
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`the runner named no page for ${seats.join(" and ")} in ${ms / 1000}s:\n${out}`)), ms);
    const look = () => {
      for (const m of out.matchAll(/^(--p[12]) gui: open (http:\/\/\S+)\r?\n/gm)) handle.urls[m[1]] = m[2];
      if (seats.every(s => handle.urls[s])) { clearTimeout(timer); handle.open = true; resolve(handle); }
    };
    runner.stderr.on("data", look);
    runner.on("error", e => { clearTimeout(timer); reject(new Error(`the runner could not be started: ${e.message}`)); });
    runner.on("exit", (code, sig) => {
      exit = { code, sig };
      gone(exit);
      clearTimeout(timer);
      reject(new Error(`the runner exited (${sig || code}) before its page was open:\n${out}`));
    });
  });
}

/**
 * Run `main`, report a throw as a FAIL, and exit once it has settled with
 * `code()` (by default whatever `process.exitCode` the driver set), the
 * owned children killed on the way out.
 */
function finish(main, code = () => process.exitCode || 0) {
  main()
    .catch(e => { console.error("FAIL: " + (e && e.stack || String(e))); process.exitCode = 1; })
    .finally(() => {
      // A runner that exited on its own with a failing status, which the
      // driver did not kill: whatever the driver's FAIL said, this is why.
      for (const h of runners) {
        const e = h.exited();
        // Not one that exited before its page opened: that rejection said it.
        if (e && h.open && !h.runner.killed && e.code !== 0) {
          console.error(`FAIL: the runner exited (${e.sig || e.code}) under the driver:\n${h.output().slice(-1500)}`);
          process.exitCode = 1;
        }
      }
      process.exit(code() || process.exitCode || 0);
    });
}

/**
 * A driver's flags, read the way the runners read theirs (#943): a flag
 * given twice takes its last value, so `$BASE --ai stub` overrides the
 * `--ai` in `$BASE`; a value the flag does not take (`--games four`,
 * `--games 0`, `--games` with nothing after it) is refused, and so is a
 * flag the driver does not have (`--gmaes 3`). Each refusal is one line
 * and exit 2, before anything is started: they used to be read as NaN or
 * as nothing and end in "ok". `spec` maps each flag to `{ def, kind }`,
 * `kind` one of "int" (with `min`, default 0), "number" or "string".
 * Returns the values by flag.
 */
function parseArgs(spec, argv = process.argv.slice(2)) {
  const refuse = (m) => { console.error(`FAIL: ${m}`); process.exit(2); };
  const values = {};
  for (const [flag, s] of Object.entries(spec)) values[flag] = s.def;
  for (let i = 0; i < argv.length; i++) {
    const flag = argv[i];
    const s = spec[flag];
    if (!s) refuse(`unknown flag ${JSON.stringify(flag)}; this driver takes ${Object.keys(spec).join(", ")}`);
    const raw = argv[++i];
    if (raw === undefined || raw.startsWith("--")) refuse(`${flag} needs a value`);
    if (s.kind === "int" || s.kind === "number") {
      const n = Number(raw);
      const min = s.min ?? 0;
      const whole = s.kind === "int";
      if (!(whole ? /^[0-9]+$/ : /^[0-9]+(\.[0-9]+)?$/).test(raw) || n < min) {
        refuse(`${flag} ${raw}: it takes ${whole ? "a whole number" : "a number"} of at least ${min}`);
      }
      values[flag] = n;
    } else {
      values[flag] = raw;
    }
  }
  return values;
}

module.exports = { spawnOwned, startRunner, finish, parseArgs, root };

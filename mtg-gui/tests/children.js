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
 * it runs). Rejects with the runner's output if it exits, or has not
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
  const handle = { runner, urls: {}, output: () => out, exited: () => exit };
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`the runner named no page for ${seats.join(" and ")} in ${ms / 1000}s:\n${out}`)), ms);
    const look = () => {
      for (const m of out.matchAll(/^(--p[12]) gui: open (http:\/\/\S+)\r?\n/gm)) handle.urls[m[1]] = m[2];
      if (seats.every(s => handle.urls[s])) { clearTimeout(timer); resolve(handle); }
    };
    runner.stderr.on("data", look);
    runner.on("error", e => { clearTimeout(timer); reject(new Error(`the runner could not be started: ${e.message}`)); });
    runner.on("exit", (code, sig) => {
      exit = { code, sig };
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
    .finally(() => process.exit(code() || process.exitCode || 0));
}

module.exports = { spawnOwned, startRunner, finish, root };

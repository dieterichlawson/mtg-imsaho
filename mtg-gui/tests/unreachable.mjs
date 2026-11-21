// A page whose game is over says when nothing answers, and slows down (#1033).
//
//   node mtg-gui/tests/unreachable.mjs
//
// The server sends `closed` where it knows the answer; where it cannot —
// a `--direct-game-ports` listener gone with its match, a link the proxy
// no longer knows after `--resume`, a runner that has exited — a page left
// on its game-over screen retried once a second for ever and said nothing.
// It must not stop outright: between a best-of-3's games `gameOver` is set
// too. A page whose game is still on keeps its one-second retry (#824).

const { unreachableReason, UNREACHABLE_AFTER_GAME_OVER: N, UNREACHABLE_RETRY_MS } = await import("../dist/state.js");

let failures = 0;
const fail = (m) => { console.error("FAIL: " + m); failures++; };
const ok = (m) => console.log("ok: " + m);

if (unreachableReason(null, 10_000) !== null) fail("a live game's page was told its game is unreachable");
else ok("a live game's page keeps its one-second retry");

if (unreachableReason("p0 wins", N - 1) !== null) fail(`a finished page gave up hope after ${N - 1} failed connects`);
else ok("a finished page retries at full speed for a while first");

const why = unreachableReason("p0 wins", N);
if (!why || !/reload/.test(why) || !why.includes(`${UNREACHABLE_RETRY_MS / 1000} s`)) fail(`a finished page after ${N} failed connects: ${JSON.stringify(why)}`);
else ok(`a finished page says so after ${N}: "${why}"`);
if (!(UNREACHABLE_RETRY_MS > 1000)) fail("the slowed retry is not slower");

process.exit(failures ? 1 : 0);

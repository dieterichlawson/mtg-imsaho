// The page's state, and the widget that answers a decision.
/**
 * The notice a new question opens with. A notice said while no question
 * was up, and still the one on screen, was about that moment: it becomes
 * its `after` form, or goes. Any other notice — "Last game: …", what
 * auto-pass declined — was said for the question that comes next and
 * stays (#1012).
 */
export function noticeAtNewDecision(notice, waiting) {
    if (waiting && notice === waiting.text)
        return waiting.after;
    return notice;
}
/** What a closed page says has ended: the table unless the server said
 *  otherwise. "The table is closed" over a finished match's page, while
 *  the table ran on, was false (#1033's review). */
export function closedHeadline(over) {
    if (over === "stopped")
        return "This game has stopped";
    return over === "match" ? "This match is over" : over === "game" ? "This game is over" : "The table is closed";
}
/** Connects in a row, one a second, that a page whose game has ended
 *  tries before it says it cannot reach the server and slows down (#1033). */
export const UNREACHABLE_AFTER_GAME_OVER = 30;
/** How often such a page tries after that, in ms: a match's next game, or
 *  the network coming back, is still picked up. */
export const UNREACHABLE_RETRY_MS = 10_000;
/**
 * What a page says once it has stopped reaching its server after its game
 * ended, or null. The server sends `closed` wherever it knows the answer;
 * where it cannot — a `--direct-game-ports` listener gone with its match,
 * a link the proxy no longer knows, a runner that exited — a page left on
 * its game-over screen retried once a second for ever and said nothing
 * (#1033). It does not stop: between a best-of-3's games `gameOver` is set
 * too, and a network that drops then must still reach game 2. A page whose
 * game is still on retries once a second as before (#824).
 */
export function unreachableReason(gameOver, failedConnects) {
    if (!gameOver || failedConnects < UNREACHABLE_AFTER_GAME_OVER)
        return null;
    return `Can't reach the game since it ended: the match or the table may be over. Trying every ${UNREACHABLE_RETRY_MS / 1000} s; reload to try now.`;
}

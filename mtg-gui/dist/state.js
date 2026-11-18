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

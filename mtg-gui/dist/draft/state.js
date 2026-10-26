// The draft page's state: the last view, what the person is in the middle
// of on top of it, and the socket. Everything the renderer reads is here.
export function initialState() {
    return {
        ws: null, connected: false, reconnects: 0, retryAt: null, rejected: false, closed: null,
        seat: null, key: null, view: null,
        selected: null, hover: null, pendingPick: null,
        deck: null, cursor: null, readySent: false, between: null, review: null, reviewOpen: false, reviewRead: null,
        refusal: null, deadlineAt: null, hint: null, notice: null,
    };
}

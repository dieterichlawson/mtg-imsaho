// The engine's types as serde writes them.
//
// This file mirrors `mtg-engine`'s `GameView`, `LegalActions`, `Action`
// and their parts, in the JSON shape `serde_json` gives them: newtype ids
// are numbers, unit variants are strings, and a data-carrying enum
// variant is an object with one key. It is a description of what the seat
// sends, not a second schema the seat has to satisfy — a field the page
// does not know about is simply ignored, and a field it does know about
// is typed here so the compiler notices when the shape changes.
/** The one key of an externally tagged enum value. */
export function tag(v) {
    return Object.keys(v)[0];
}

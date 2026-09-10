/// The front face of a card name, which is how a double-faced card is
/// referred to everywhere but the set file: `"Grizzled Outcasts //
/// Krallenhorde Wantons"` is one physical card called `Grizzled Outcasts`.
///
/// There were a dozen copies of this two-line split across the workspace,
/// and `validate_deck` had three of them — the pool census and the
/// maindeck-vs-pool check normalised, and the sideboard computation
/// compared the normalised POOL name against the RAW maindeck string. So a
/// deck answer naming a DFC in full was accepted against the pool and then
/// never removed from it: the seat maindecked the card AND sideboarded it,
/// and 23 + 20 came to 43 physical cards out of a 42-card pool (issue
/// #403). One function, so the three passes cannot disagree about what a
/// card is called.
#[must_use]
pub fn front_face(name: &str) -> &str {
    name.split(" // ").next().unwrap_or(name)
}

pub mod set_data;
pub mod pack;
pub mod draft;
pub mod deckbuilding;
pub mod tournament;

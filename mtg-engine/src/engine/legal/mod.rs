//! Enumerating what a player may legally do right now.
//!
//! `legal_actions` was 1,128 lines. Its sections did share state — unlike the
//! arms of `submit_action` — so what they share is named here and computed
//! once, rather than closed over as locals in one long function.
//!
//! Nothing here mutates the game state: `legal_actions` takes `&GameState`
//! throughout. So the order of the sections affects only the order actions are
//! offered in, which the tests do depend on, and never what any of them
//! computes.

use crate::cards::CardRegistry;
use crate::ids::PlayerId;
use crate::state::GameState;

pub(crate) mod abilities;
pub(crate) mod awaiting;
pub(crate) mod casting;

/// What the sections of `legal_actions` share.
pub(crate) struct Ctx<'a> {
    pub state: &'a GameState,
    pub registry: &'a CardRegistry,
    /// The player with priority.
    pub player: PlayerId,
    /// Stony Silence and friends: no ability of an artifact may be activated,
    /// mana abilities included.
    pub prevent_artifact_abilities: bool,
    /// Your main phase, the stack empty, your turn.
    pub is_sorcery_speed: bool,
    /// Every mana source the player could tap, classified by opportunity
    /// cost. Gathered once because the ability loop and the casting loop have
    /// to agree on it — they are planning against the same lands.
    pub mana_sources: Vec<crate::mana::ManaSource>,
    /// Everything else the player could pay for (`mana_demands`), which the
    /// auto-tap planner keeps payable. Each carries the id of the object
    /// that pays it, which keeps "exclude the spell being cast" honest —
    /// an index into a cost-filtered list drifted off the unfiltered hand
    /// as soon as a land sat in it, excluding the wrong spell from the
    /// auto-tap's color-demand map (issue #114).
    pub demands: Vec<(crate::ids::ObjectId, crate::mana::Demand)>,
    /// Card names that may not be cast at all (Nevermore).
    pub casting_banned: Vec<String>,
}

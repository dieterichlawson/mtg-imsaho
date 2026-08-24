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
use crate::types::ManaCost;

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
    /// Effective costs of the player's castable spells, which the auto-tap
    /// planner uses to avoid spending mana another spell needs for its colour.
    pub hand_costs: Vec<ManaCost>,
    /// Card names that may not be cast at all (Nevermore).
    pub casting_banned: Vec<String>,
}

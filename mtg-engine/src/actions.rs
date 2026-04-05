use serde::{Serialize, Deserialize};

use crate::ids::{ObjectId, PlayerId};

/// A target for a spell or ability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    /// Target a permanent or creature on the stack.
    Object(ObjectId),
    /// Target a player.
    Player(PlayerId),
}

/// An action a player can take.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    /// Pass priority to the next player.
    PassPriority,

    /// Play a land from hand (special action, doesn't use the stack).
    PlayLand { object_id: ObjectId },

    /// Cast a spell (puts it on the stack).
    /// For targeted spells, targets must be chosen at cast time.
    /// If the spell has an additional cost (e.g. sacrifice a creature), `sacrifice` holds the chosen creature.
    /// `exile_count` is used for "exile X cards from graveyard" costs (Harvest Pyre).
    /// `exile_ids` holds the specific graveyard cards chosen to exile (populated by legal_actions for ExileXFromGraveyard).
    ///   When non-empty, these exact cards are exiled; `exile_count` is derived from the length.
    ///   When empty and `exile_count` is set, the engine falls back to auto-selecting the first N cards (legacy behavior).
    /// `alternative_cost` is an optional alternative mana cost (e.g. Rooftop Storm's {0}).
    /// When set, this cost is used instead of the normal mana cost.
    CastSpell { object_id: ObjectId, targets: Vec<Target>, sacrifice: Option<ObjectId>, exile_count: Option<u32>, exile_ids: Vec<ObjectId>, alternative_cost: Option<crate::types::ManaCost> },

    /// Activate a mana ability (doesn't use the stack, player retains priority).
    ActivateManaAbility { object_id: ObjectId, ability_index: usize },

    /// Activate a non-mana ability (doesn't use the stack for now, player retains priority).
    ActivateAbility { object_id: ObjectId, ability_index: usize, targets: Vec<Target> },

    /// Activate a planeswalker loyalty ability.
    ActivateLoyaltyAbility { object_id: ObjectId, ability_index: usize, targets: Vec<Target> },

    /// Declare which creatures are attacking and who they're attacking.
    DeclareAttackers { attackers: Vec<(ObjectId, PlayerId)> },

    /// Declare which creatures are blocking and what they're blocking.
    DeclareBlockers { assignments: Vec<(ObjectId, ObjectId)> },

    /// Discard cards to reach hand size limit.
    DiscardCards { cards: Vec<ObjectId> },

    /// Concede the game.
    Concede,

    /// Respond to a mid-resolution choice.
    ResolveChoice { choice: ResolvedChoice },
}

/// A player's response to a mid-resolution choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolvedChoice {
    /// Pay or don't pay (Frightful Delusion).
    PayDecision(bool),
    /// Choose a target, or None if optional and declined.
    ChosenTarget(Option<Target>),
    /// Choose a card from a revealed set.
    ChosenCard(ObjectId),
    /// Choose an option by index from a numbered list.
    ChosenIndex(usize),
    /// Choose a subset of objects (e.g., pile division — chosen objects form pile 1, rest form pile 2).
    ChosenSubset(Vec<ObjectId>),
}

/// Prompt returned by legal_actions for combat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CombatPrompt {
    /// Choose a subset of these creatures to attack with.
    ChooseAttackers {
        eligible: Vec<ObjectId>,
        /// Creatures that must attack this combat (e.g., Furor of the Bitten).
        must_attack: Vec<ObjectId>,
        defending_player: PlayerId,
    },
    /// Choose blocking assignments.
    ChooseBlockers {
        eligible_blockers: Vec<ObjectId>,
        attackers: Vec<ObjectId>,
    },
}

/// A spell that can be cast, with its valid target options.
/// Used by player implementations to present a collapsed casting UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CastableSpell {
    pub object_id: ObjectId,
    pub name: String,
    pub is_flashback: bool,
    pub target_spec: CastTargetSpec,
}

/// Describes how targets should be chosen for a castable spell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CastTargetSpec {
    /// No targets needed. Cast directly.
    NoTargets,
    /// Choose exactly one target from this list.
    SingleTarget(Vec<Target>),
    /// Choose two targets, one from each list. Targets must be different.
    TwoTargets(Vec<Target>, Vec<Target>),
    /// Choose up to N targets from this list.
    UpToTargets { max: usize, options: Vec<Target> },
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::PassPriority => write!(f, "Pass priority"),
            Action::PlayLand { object_id } => write!(f, "Play land {}", object_id),
            Action::CastSpell { object_id, targets, sacrifice, alternative_cost, .. } => {
                let alt_prefix = if alternative_cost.is_some() { "Cast spell (alt cost) " } else { "Cast spell " };
                if targets.is_empty() && sacrifice.is_none() {
                    write!(f, "{}{}", alt_prefix, object_id)
                } else if let Some(sac) = sacrifice {
                    write!(f, "{}{} (sacrifice {}) targeting {:?}", alt_prefix, object_id, sac, targets)
                } else {
                    write!(f, "{}{} targeting {:?}", alt_prefix, object_id, targets)
                }
            }
            Action::ActivateManaAbility { object_id, ability_index } =>
                write!(f, "Activate mana ability {} on {}", ability_index, object_id),
            Action::ActivateAbility { object_id, ability_index, targets } => {
                if targets.is_empty() {
                    write!(f, "Activate ability {} on {}", ability_index, object_id)
                } else {
                    write!(f, "Activate ability {} on {} targeting {:?}", ability_index, object_id, targets)
                }
            }
            Action::DeclareAttackers { attackers } =>
                write!(f, "Declare {} attackers", attackers.len()),
            Action::DeclareBlockers { assignments } =>
                write!(f, "Declare {} blockers", assignments.len()),
            Action::DiscardCards { cards } =>
                write!(f, "Discard {} cards", cards.len()),
            Action::Concede => write!(f, "Concede"),
            Action::ActivateLoyaltyAbility { object_id, ability_index, .. } =>
                write!(f, "Activate loyalty ability {} on {}", ability_index, object_id),
            Action::ResolveChoice { choice } => write!(f, "Choice: {:?}", choice),
        }
    }
}

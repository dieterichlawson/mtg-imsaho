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
    CastSpell { object_id: ObjectId, targets: Vec<Target> },

    /// Activate a mana ability (doesn't use the stack, player retains priority).
    ActivateManaAbility { object_id: ObjectId, ability_index: usize },

    /// Declare which creatures are attacking and who they're attacking.
    DeclareAttackers { attackers: Vec<(ObjectId, PlayerId)> },

    /// Declare which creatures are blocking and what they're blocking.
    DeclareBlockers { assignments: Vec<(ObjectId, ObjectId)> },

    /// Discard cards to reach hand size limit.
    DiscardCards { cards: Vec<ObjectId> },

    /// Concede the game.
    Concede,
}

/// Prompt returned by legal_actions for combat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CombatPrompt {
    /// Choose a subset of these creatures to attack with.
    ChooseAttackers {
        eligible: Vec<ObjectId>,
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
            Action::CastSpell { object_id, targets } => {
                if targets.is_empty() {
                    write!(f, "Cast spell {}", object_id)
                } else {
                    write!(f, "Cast spell {} targeting {:?}", object_id, targets)
                }
            }
            Action::ActivateManaAbility { object_id, ability_index } =>
                write!(f, "Activate mana ability {} on {}", ability_index, object_id),
            Action::DeclareAttackers { attackers } =>
                write!(f, "Declare {} attackers", attackers.len()),
            Action::DeclareBlockers { assignments } =>
                write!(f, "Declare {} blockers", assignments.len()),
            Action::DiscardCards { cards } =>
                write!(f, "Discard {} cards", cards.len()),
            Action::Concede => write!(f, "Concede"),
        }
    }
}

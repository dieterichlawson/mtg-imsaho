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
    CastSpell { object_id: ObjectId, targets: Vec<Target>, sacrifice: Option<ObjectId>, exile_count: Option<u32>, exile_ids: Vec<ObjectId>, alternative_cost: Option<crate::types::ManaCost>, tap_plan: Vec<(ObjectId, usize)> },

    /// Activate a mana ability (doesn't use the stack, player retains priority).
    ActivateManaAbility { object_id: ObjectId, ability_index: usize },

    /// Activate a non-mana ability (doesn't use the stack for now, player retains priority).
    /// `tap_plan` lists mana sources to tap before paying the ability's mana cost,
    /// computed by `legal_actions` via the same auto-tap logic spell casting uses.
    /// Empty for free abilities or abilities whose cost is already in the mana pool.
    /// `sacrifice` is the creature the player chose to sacrifice when paying the
    /// `Sacrifice a creature` cost. `None` for abilities without that cost.
    /// `legal_actions` enumerates one ActivateAbility per (target, sacrifice) combo
    /// so the player picks both up front, mirroring how CastSpell handles it.
    ActivateAbility {
        object_id: ObjectId,
        ability_index: usize,
        targets: Vec<Target>,
        tap_plan: Vec<(ObjectId, usize)>,
        sacrifice: Option<ObjectId>,
        /// For X-cost abilities, the chosen X value. When Some, the apply path
        /// uses this instead of computing X from remaining mana in the pool.
        x_value: Option<u32>,
    },

    /// Activate a planeswalker loyalty ability.
    ActivateLoyaltyAbility { object_id: ObjectId, ability_index: usize, targets: Vec<Target> },

    /// Declare which creatures are attacking and who they're attacking.
    DeclareAttackers { attackers: Vec<(ObjectId, PlayerId)> },

    /// Declare which creatures are blocking and what they're blocking.
    DeclareBlockers { assignments: Vec<(ObjectId, ObjectId)> },

    /// Discard cards to reach hand size limit.
    DiscardCards { cards: Vec<ObjectId> },

    /// London mulligan: keep the current opening hand.
    MulliganKeep,

    /// London mulligan: shuffle hand back and draw seven again.
    /// Only legal if the player has taken fewer than the cap (3) mulligans.
    MulliganMull,

    /// Put the chosen cards from hand on the bottom of the library in
    /// the order given (first element becomes the bottom-most card).
    /// Used after London mulligans are resolved: the player must bottom
    /// one card per mulligan taken.
    BottomCards { cards: Vec<ObjectId> },

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
    /// Yes or no for "you may" abilities (Cloistered Youth transform, etc.).
    YesNoDecision(bool),
    /// Choose a target, or None if optional and declined.
    ChosenTarget(Option<Target>),
    /// Choose a card from a revealed set.
    ChosenCard(ObjectId),
    /// Choose an option by index from a numbered list.
    /// The label is a human-readable name for the option
    /// (e.g. "Creature" for a card type choice).
    ChosenIndex(usize, String),
    /// Choose a subset of objects (e.g., pile division — chosen objects form pile 1, rest form pile 2).
    ChosenSubset(Vec<ObjectId>),
    /// Choose X for an X-cost spell or ability.
    ChosenXValue(u32),
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
        /// For each blocker, the set of attackers it can legally block.
        /// Accounts for flying/reach, intimidate, protection, CanOnlyBeBlockedBy, etc.
        legal_blocks: std::collections::HashMap<ObjectId, Vec<ObjectId>>,
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
    /// Pre-computed mana sources to tap when casting this spell.
    pub tap_plan: Vec<(ObjectId, usize)>,
    /// For spells with an "exile X cards from your graveyard" additional
    /// cost (Harvest Pyre), the maximum X the caster can pay right now —
    /// i.e. the number of exilable cards in their graveyard. `None` for
    /// spells without this cost. Player implementations use this to
    /// display the effective X (and resulting damage) in the action
    /// label and to fill in `exile_count` / `exile_ids` when casting.
    pub exile_x_from_gy_max: Option<u32>,
    /// Creatures eligible to sacrifice as an additional cost (Altar's Reap,
    /// Infernal Plunge). Empty if the spell has no sacrifice cost.
    /// Player implementations should prompt the agent to choose one.
    pub sacrifice_options: Vec<ObjectId>,
}

/// An activated ability that can be activated, with its valid target options.
/// Used by player implementations to present a collapsed ability UI.
///
/// `option_combos` is the canonical list of (target, sacrifice) combinations the
/// player can pick from. Each entry corresponds to exactly one underlying
/// `Action::ActivateAbility` in the legal action list. `target_options` is kept
/// for backwards compatibility with simple targeted abilities (no sacrifice
/// cost) — for those, target_options is the list of legal targets and each
/// option_combos entry is `(vec![target], None)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivatableAbility {
    pub object_id: ObjectId,
    pub ability_index: usize,
    pub name: String,
    pub description: String,
    pub target_options: Vec<Target>,
    /// Pre-computed mana sources to tap when paying this ability's cost.
    /// Empty if the cost is already in the mana pool, or if the ability has no mana cost.
    pub tap_plan: Vec<(ObjectId, usize)>,
    /// All player-chooseable (targets, sacrifice) combinations for this ability.
    /// One entry per legal `Action::ActivateAbility` for this (object_id, ability_index).
    /// For abilities with no sacrifice cost the sacrifice is None; for untargeted
    /// abilities the targets vector is empty.
    pub option_combos: Vec<ActivatableAbilityOption>,
}

/// One concrete (target, sacrifice) choice for an activated ability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivatableAbilityOption {
    pub targets: Vec<Target>,
    pub sacrifice: Option<ObjectId>,
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
            Action::ActivateAbility { object_id, ability_index, targets, sacrifice, .. } => {
                let sac_str = match sacrifice {
                    Some(id) => format!(" (sacrificing {})", id),
                    None => String::new(),
                };
                if targets.is_empty() {
                    write!(f, "Activate ability {} on {}{}", ability_index, object_id, sac_str)
                } else {
                    write!(f, "Activate ability {} on {} targeting {:?}{}", ability_index, object_id, targets, sac_str)
                }
            }
            Action::DeclareAttackers { attackers } =>
                write!(f, "Declare {} attackers", attackers.len()),
            Action::DeclareBlockers { assignments } =>
                write!(f, "Declare {} blockers", assignments.len()),
            Action::DiscardCards { cards } =>
                write!(f, "Discard {} card{}", cards.len(),
                    if cards.len() == 1 { "" } else { "s" }),
            Action::MulliganKeep => write!(f, "Keep opening hand"),
            Action::MulliganMull => write!(f, "Mulligan"),
            Action::BottomCards { cards } =>
                write!(f, "Bottom {} card(s)", cards.len()),
            Action::Concede => write!(f, "Concede"),
            Action::ActivateLoyaltyAbility { object_id, ability_index, .. } =>
                write!(f, "Activate loyalty ability {} on {}", ability_index, object_id),
            Action::ResolveChoice { choice } => write!(f, "Choice: {:?}", choice),
        }
    }
}

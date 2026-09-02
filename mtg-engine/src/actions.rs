use serde::{Serialize, Deserialize};

use crate::ids::{ObjectId, PlayerId};

/// A target for a spell or ability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    /// Target a permanent or creature on the stack.
    Object(ObjectId),
    /// Target a player.
    Player(PlayerId),
    /// A target that was legal when the spell or ability was put on the stack
    /// and is not legal any more (CR 608.2b).
    ///
    /// The engine substitutes this at resolution so a multi-target card keeps
    /// its target *positions* — "targets[0] is the land, targets[1] is the
    /// creature" stays true — while the illegal one simply fails to match
    /// `Target::Object(..)` and is skipped. A card whose ruling is
    /// all-or-nothing (Prey Upon) gets that for free, because its
    /// `(Target::Object(a), Target::Object(b))` pattern no longer matches.
    ///
    /// This never appears in a target list a player chooses from.
    Illegal,
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
    /// `exile_ids` holds the specific graveyard cards chosen to exile (populated by `legal_actions` for `ExileXFromGraveyard`).
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
    /// `legal_actions` enumerates one `ActivateAbility` per (target, sacrifice) combo
    /// so the player picks both up front, mirroring how `CastSpell` handles it.
    ActivateAbility {
        object_id: ObjectId,
        ability_index: usize,
        targets: Vec<Target>,
        tap_plan: Vec<(ObjectId, usize)>,
        sacrifice: Option<ObjectId>,
        /// For X-cost abilities, the chosen X value. When Some, the apply path
        /// uses this instead of computing X from remaining mana in the pool.
        x_value: Option<u32>,
        /// Disambiguates which behavior contributed this ability. `None` means
        /// the object's native card (or Evil Twin override). `Some(cid)` means
        /// the ability is granted by an attached aura whose card is `cid`.
        /// Required to disambiguate when an aura grants an ability with the
        /// same `ability_index` as a native ability.
        source_card_id: Option<crate::ids::CardId>,
    },

    /// Activate a planeswalker loyalty ability.
    ActivateLoyaltyAbility { object_id: ObjectId, ability_index: usize, targets: Vec<Target> },

    /// Declare which creatures are attacking and who they're attacking.
    /// `planeswalker_attacks` names attackers sent at a planeswalker instead
    /// of at the player: (attacker, planeswalker). Such an attacker appears
    /// ONLY here, not in `attackers` — the engine works out the defending
    /// player (the planeswalker's controller, CR 508.1a).
    DeclareAttackers {
        attackers: Vec<(ObjectId, PlayerId)>,
        #[serde(default)]
        planeswalker_attacks: Vec<(ObjectId, ObjectId)>,
    },

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
    /// Funding choices for an X-cost spell or ability. Contains the player's
    /// pool drain + tap allocations; the engine derives X from the sum.
    XFunding(crate::funding::FundingResponse),
    /// Chosen set of graveyard cards to exile as an additional cost.
    /// Used by `ChooseExileFromGraveyard` prompts on spells with
    /// `ExileXFromGraveyard` (Harvest Pyre — variable count) or
    /// `ExileCreaturesFromGraveyard(n)` (Stitched Drake / Skaab
    /// Ruinator / etc. — fixed count).
    ChosenExileSet(Vec<ObjectId>),
    /// Cancel the cast in progress. Used when a fixed-count exile-choice
    /// prompt can't be satisfied (agent returned an invalid subset twice).
    /// The engine rolls back: spell stays in hand, no mana tapped, no
    /// cards exiled.
    CancelCast,
}

/// Prompt returned by `legal_actions` for combat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CombatPrompt {
    /// Choose a subset of these creatures to attack with.
    ChooseAttackers {
        eligible: Vec<ObjectId>,
        /// Creatures that must attack this combat (e.g., Furor of the Bitten).
        must_attack: Vec<ObjectId>,
        defending_player: PlayerId,
        /// Planeswalkers the defending player controls — each may be attacked
        /// instead of the player (CR 508.1a).
        #[serde(default)]
        defending_planeswalkers: Vec<ObjectId>,
    },
    /// Choose blocking assignments.
    ChooseBlockers {
        eligible_blockers: Vec<ObjectId>,
        attackers: Vec<ObjectId>,
        /// For each blocker, the set of attackers it can legally block.
        /// Accounts for flying/reach, intimidate, protection, `CanOnlyBeBlockedBy`, etc.
        legal_blocks: std::collections::HashMap<ObjectId, Vec<ObjectId>>,
        /// Attackers that can't be blocked by fewer than N creatures
        /// (menace, `MinimumBlockers` effects — CR 509.1b), so the players
        /// can refuse an under-minimum declaration up front instead of the
        /// engine silently discarding it (issue #72). Only attackers with a
        /// requirement above 1 appear.
        #[serde(default)]
        min_blockers: std::collections::HashMap<ObjectId, u32>,
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
    /// Human-readable summary of additional costs beyond mana (e.g.
    /// "exile a creature from GY", "sacrifice a creature"). `None`
    /// when the spell has no additional cost.
    pub additional_cost_label: Option<String>,
    /// The alternative cost this entry casts with (CR 118.9), when it does —
    /// Rooftop Storm's "without paying its mana cost", or flashback's cost.
    /// When both the normal and an alternative cost are payable the engine
    /// emits one CastableSpell per way to cast, so the CR 601.2b choice
    /// reaches the player instead of being collapsed away (issue #128).
    /// Player implementations must carry this into the CastSpell they build.
    pub alternative_cost: Option<crate::types::ManaCost>,
}

/// An activated ability that can be activated, with its valid target options.
/// Used by player implementations to present a collapsed ability UI.
///
/// `option_combos` is the canonical list of (target, sacrifice) combinations the
/// player can pick from. Each entry corresponds to exactly one underlying
/// `Action::ActivateAbility` in the legal action list. `target_options` is kept
/// for backwards compatibility with simple targeted abilities (no sacrifice
/// cost) — for those, `target_options` is the list of legal targets and each
/// `option_combos` entry is `(vec![target], None)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivatableAbility {
    pub object_id: ObjectId,
    pub ability_index: usize,
    /// Disambiguates aura-granted abilities from native ones. See
    /// [`Action::ActivateAbility::source_card_id`].
    pub source_card_id: Option<crate::ids::CardId>,
    pub name: String,
    pub description: String,
    pub target_options: Vec<Target>,
    /// Pre-computed mana sources to tap when paying this ability's cost.
    /// Empty if the cost is already in the mana pool, or if the ability has no mana cost.
    pub tap_plan: Vec<(ObjectId, usize)>,
    /// All player-chooseable (targets, sacrifice) combinations for this ability.
    /// One entry per legal `Action::ActivateAbility` for this (`object_id`, `ability_index`).
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
    /// Choose two target instances: one from `first`, then `second_min..=second_max`
    /// from the list in `second` at the same index as the chosen first target.
    /// The second slot's legal candidates can depend on the first choice
    /// ("target player shuffles ... cards from THEIR graveyard"), so each
    /// first option carries its own pre-narrowed second-slot list.
    TwoTargets {
        first: Vec<Target>,
        /// Parallel to `first`: the legal second-slot options once that
        /// first target is chosen.
        second: Vec<Vec<Target>>,
        /// 0 when the second slot is "up to N" (choosing none is legal), else 1.
        second_min: usize,
        second_max: usize,
    },
    /// Choose up to N targets from this list.
    UpToTargets { max: usize, options: Vec<Target> },
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::PassPriority => write!(f, "Pass priority"),
            Action::PlayLand { object_id } => write!(f, "Play land {object_id}"),
            Action::CastSpell { object_id, targets, sacrifice, alternative_cost, .. } => {
                let alt_prefix = if alternative_cost.is_some() { "Cast spell (alt cost) " } else { "Cast spell " };
                if targets.is_empty() && sacrifice.is_none() {
                    write!(f, "{alt_prefix}{object_id}")
                } else if let Some(sac) = sacrifice {
                    write!(f, "{alt_prefix}{object_id} (sacrifice {sac}) targeting {targets:?}")
                } else {
                    write!(f, "{alt_prefix}{object_id} targeting {targets:?}")
                }
            }
            Action::ActivateManaAbility { object_id, ability_index } =>
                write!(f, "Activate mana ability {ability_index} on {object_id}"),
            Action::ActivateAbility { object_id, ability_index, targets, sacrifice, .. } => {
                let sac_str = match sacrifice {
                    Some(id) => format!(" (sacrificing {id})"),
                    None => String::new(),
                };
                if targets.is_empty() {
                    write!(f, "Activate ability {ability_index} on {object_id}{sac_str}")
                } else {
                    write!(f, "Activate ability {ability_index} on {object_id} targeting {targets:?}{sac_str}")
                }
            }
            Action::DeclareAttackers { attackers, planeswalker_attacks } =>
                write!(f, "Declare {} attackers", attackers.len() + planeswalker_attacks.len()),
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
                write!(f, "Activate loyalty ability {ability_index} on {object_id}"),
            Action::ResolveChoice { choice } => write!(f, "Choice: {choice:?}"),
        }
    }
}

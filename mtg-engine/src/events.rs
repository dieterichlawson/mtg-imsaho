use serde::{Serialize, Deserialize};

use crate::ids::{ObjectId, PlayerId};
use crate::types::{Zone, Step, ManaType};
use crate::state::GameResult;

/// Events emitted by state transitions. Used for game log, triggered abilities (future),
/// and UI updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    GameStarted,
    TurnStarted { player: PlayerId, turn: u32 },
    StepStarted { step: Step },
    CardDrawn { player: PlayerId, object: ObjectId },
    LandPlayed { player: PlayerId, object: ObjectId },
    SpellCast { player: PlayerId, object: ObjectId },
    SpellResolved { object: ObjectId },
    ManaAdded { player: PlayerId, mana_type: ManaType, amount: u32 },
    ManaPoolEmptied { player: PlayerId },
    EnteredBattlefield { object: ObjectId, controller: PlayerId },
    /// A permanent left the battlefield. `last_controller` captures the
    /// controlling player immediately before the zone change, since the
    /// controller on the object itself may be cleared/stale once the move
    /// completes. Required for CR 603.10c (LTB triggers are controlled by
    /// the player who controlled the permanent before it left).
    LeftBattlefield { object: ObjectId, to: Zone, last_controller: PlayerId },
    ObjectMoved { object: ObjectId, from: Zone, to: Zone },
    Tapped { object: ObjectId },
    Untapped { object: ObjectId },
    AttackersDeclared { attackers: Vec<(ObjectId, PlayerId)> },
    BlockersDeclared { assignments: Vec<(ObjectId, ObjectId)> },
    CombatDamageDealt { source: ObjectId, target: DamageTarget, amount: u32 },
    /// Non-combat damage dealt (e.g., triggered abilities, spells).
    NonCombatDamageDealt { source: ObjectId, target: DamageTarget, amount: u32 },
    LifeChanged { player: PlayerId, old: i32, new_life: i32 },
    CreatureDied { object: ObjectId, card_id: crate::ids::CardId, controller: PlayerId, damaged_by: Vec<ObjectId>, last_known_toughness: i32, is_token: bool },
    PlayerLost { player: PlayerId, reason: LossReason },
    GameEnded { result: GameResult },
    PriorityPassed { player: PlayerId },
    Discarded { player: PlayerId, object: ObjectId },
    /// A creature card was milled from a player's library to their graveyard.
    CreatureCardMilled { object: ObjectId, milled_player: PlayerId },
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum DamageTarget {
    Player(PlayerId),
    Object(ObjectId),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LossReason {
    LifeReachedZero,
    DrewFromEmptyLibrary,
    Conceded,
    /// CR 104.2a: in a two-player game, a player loses when their opponent
    /// wins. Nothing happened to *them* — Laboratory Maniac used to report
    /// this as `LifeReachedZero`, which is simply untrue of a player on 20.
    OpponentWon,
}

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
    LeftBattlefield { object: ObjectId, to: Zone },
    ObjectMoved { object: ObjectId, from: Zone, to: Zone },
    Tapped { object: ObjectId },
    Untapped { object: ObjectId },
    AttackersDeclared { attackers: Vec<(ObjectId, PlayerId)> },
    BlockersDeclared { assignments: Vec<(ObjectId, ObjectId)> },
    CombatDamageDealt { source: ObjectId, target: DamageTarget, amount: u32 },
    /// Non-combat damage dealt (e.g., triggered abilities, spells).
    NonCombatDamageDealt { source: ObjectId, target: DamageTarget, amount: u32 },
    LifeChanged { player: PlayerId, old: i32, new_life: i32 },
    CreatureDied { object: ObjectId, card_id: crate::ids::CardId, controller: PlayerId, damaged_by: Vec<ObjectId>, last_known_toughness: i32 },
    PlayerLost { player: PlayerId, reason: LossReason },
    GameEnded { result: GameResult },
    PriorityPassed { player: PlayerId },
    Discarded { player: PlayerId, object: ObjectId },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DamageTarget {
    Player(PlayerId),
    Object(ObjectId),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LossReason {
    LifeReachedZero,
    DrewFromEmptyLibrary,
    Conceded,
}

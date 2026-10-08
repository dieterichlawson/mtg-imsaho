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
    /// A permanent changed controller (CR 108.4). Recorded so that what an
    /// object's controller WAS at an earlier event can be read back: a
    /// control effect ending as a state-based action moves the permanent
    /// after the damage it dealt, and the event window is checked after
    /// that (issue #682).
    ControlChanged { object: ObjectId, from: PlayerId, to: PlayerId },
    Tapped { object: ObjectId },
    Untapped { object: ObjectId },
    AttackersDeclared { attackers: Vec<(ObjectId, PlayerId)> },
    BlockersDeclared { assignments: Vec<(ObjectId, ObjectId)> },
    CombatDamageDealt { source: ObjectId, target: DamageTarget, amount: u32 },
    /// Non-combat damage dealt (e.g., triggered abilities, spells).
    NonCombatDamageDealt { source: ObjectId, target: DamageTarget, amount: u32 },
    LifeChanged { player: PlayerId, old: i32, new_life: i32 },
    /// A creature died.
    ///
    /// Everything past `object` is last known information (CR 608.2g), captured
    /// before the zone change rather than read back afterwards — by then the
    /// controller has reverted to the owner, the damage record is cleared, a
    /// transformed permanent has turned back to its front face (all CR 400.7),
    /// and a token is about to stop existing entirely (SBA 704.5d).
    ///
    /// `subtypes` is the active face's, which is why "whenever another **Human**
    /// dies" cannot read the object: a werewolf that died as a Werewolf is a
    /// Human again by the time anything looks.
    /// A creature died, with its last known information (CR 608.2g). `name`
    /// is its name as it died: a token is gone from `state.objects` by the
    /// time a trigger it caused is ordered, and the player ordering it
    /// needs to know which death it is for (issue #325).
    CreatureDied { object: ObjectId, name: String, card_id: crate::ids::CardId, controller: PlayerId, damaged_by: Vec<ObjectId>, last_known_toughness: i32, is_token: bool, subtypes: Vec<String> },

    PlayerLost { player: PlayerId, reason: LossReason },
    GameEnded { result: GameResult },
    PriorityPassed { player: PlayerId },
    Discarded { player: PlayerId, object: ObjectId },
    /// A creature card was milled from a player's library to their graveyard.
    CreatureCardMilled { object: ObjectId, milled_player: PlayerId },
    /// A player's library was shuffled (CR 701.20a).
    LibraryShuffled { player: PlayerId },
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DamageTarget {
    Player(PlayerId),
    Object(ObjectId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LossReason {
    LifeReachedZero,
    DrewFromEmptyLibrary,
    Conceded,
    /// The harness ended the game against a seat that stopped answering
    /// (`Action::Forfeit`, #742). A loss like a concede, but not one the
    /// seat chose.
    Forfeited,
    /// CR 104.2b: an effect stated that the opponent wins the game, and in a
    /// two-player game that ends it (CR 104.1). Nothing happened to *them* —
    /// Laboratory Maniac used to report this as `LifeReachedZero`, which is
    /// simply untrue of a player on 20.
    ///
    /// `source` is the object whose effect it was, so the account of the game
    /// can say how it was won. It used to carry nothing and cite CR 104.2a —
    /// "a player wins when all their opponents have left the game", the
    /// inverse of what happened — and the result line read "lost the game:
    /// the opponent won", which says who and not how (#624).
    OpponentWon { source: ObjectId },
}

impl LossReason {
    /// Human sentence fragment, read as "<player> <describe()>". One string
    /// for both the game log's loss line and the runner's result summary
    /// (issue #86: the reason was constructed and then discarded — no log
    /// line, and the result named only the winner).
    ///
    /// Takes the game so an effect that won it can be named.
    #[must_use]
    pub fn describe(self, state: &crate::state::GameState) -> String {
        match self {
            LossReason::LifeReachedZero => "lost the game: life total was 0 or less (CR 704.5a)".into(),
            LossReason::DrewFromEmptyLibrary =>
                "lost the game: tried to draw from an empty library (CR 704.5b)".into(),
            LossReason::Conceded => "conceded".into(),
            LossReason::Forfeited =>
                "forfeited: stopped answering, and the harness ended the game for it".into(),
            LossReason::OpponentWon { source } => format!(
                "lost the game: the opponent won by the effect of {} (CR 104.2b)",
                state.obj_name(source)),
        }
    }
}

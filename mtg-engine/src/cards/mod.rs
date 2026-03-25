pub mod forest;
pub mod mountain;
pub mod plains;
pub mod island;
pub mod swamp;
pub mod kalonian_tusker;
pub mod goblin_piker;
pub mod savannah_lions;
pub mod grizzly_bears;
pub mod walking_corpse;
pub mod sol_ring;
pub mod lightning_bolt;
pub mod giant_growth;
pub mod doom_blade;
pub mod swords_to_plowshares;
pub mod divination;
pub mod lava_axe;
pub mod glorious_anthem;
pub mod holy_strength;
pub mod pacifism;

use std::collections::HashMap;

use crate::actions::Target;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Static card data — the printed card.
#[derive(Debug, Clone)]
pub struct CardData {
    pub name: String,
    pub cost: Option<ManaCost>,
    pub card_types: Vec<CardType>,
    pub supertypes: Vec<Supertype>,
    pub subtypes: Vec<String>,
    pub power: Option<i32>,
    pub toughness: Option<i32>,
    pub oracle_text: String,
}

/// A mana ability definition.
pub struct ManaAbilityDef {
    pub ability_index: usize,
    pub description: String,
    pub produced: Vec<(ManaType, u32)>,
    pub requires_tap: bool,
}

/// Describes what targets a spell needs when cast.
#[derive(Debug, Clone)]
pub enum TargetRequirement {
    /// No targets needed (vanilla creatures, Divination, etc.)
    None,
    /// Target any creature or player ("any target" — Lightning Bolt)
    AnyTarget,
    /// Target a creature only (Giant Growth, Doom Blade)
    Creature,
    /// Target a creature matching a filter (Doom Blade: "nonblack creature")
    CreatureWithFilter(String),
    /// Target a player only (Lava Axe)
    PlayerOnly,
}

/// The trait that every card implements.
/// Provides static data and behavioral hooks.
pub trait CardBehavior: Send + Sync {
    fn card_data(&self) -> CardData;

    /// What targets does this spell require when cast?
    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::None
    }

    /// Check if a specific target is valid for this spell.
    /// Called for each potential target when building legal actions.
    fn is_valid_target(&self, _state: &GameState, _caster: PlayerId, _target: &Target) -> bool {
        true
    }

    /// List of mana abilities this permanent has while on the battlefield.
    fn mana_abilities(&self, _state: &GameState, _object_id: ObjectId) -> Vec<ManaAbilityDef> {
        vec![]
    }

    /// Called when this spell resolves from the stack.
    /// `targets` contains the targets chosen at cast time.
    /// For permanents: default moves to battlefield.
    /// For instants/sorceries: override to apply effect.
    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target]) {
        let _ = targets; // default ignores targets
        // Default for permanents: move to battlefield.
        let card_data = self.card_data();
        if card_data.card_types.iter().any(|t| t.is_permanent()) {
            state.move_object(object_id, Zone::Battlefield);
        }
    }
}

/// Registry mapping CardIds to their behavior implementations.
pub struct CardRegistry {
    cards: HashMap<CardId, Box<dyn CardBehavior>>,
    next_id: u32,
    name_to_id: HashMap<String, CardId>,
}

impl CardRegistry {
    pub fn new() -> Self {
        Self {
            cards: HashMap::new(),
            next_id: 1,
            name_to_id: HashMap::new(),
        }
    }

    /// Register a card and return its assigned CardId.
    pub fn register(&mut self, card: Box<dyn CardBehavior>) -> CardId {
        let id = CardId(self.next_id);
        self.next_id += 1;
        let name = card.card_data().name.clone();
        self.cards.insert(id, card);
        self.name_to_id.insert(name, id);
        id
    }

    /// Look up a card's behavior by ID.
    pub fn get(&self, id: CardId) -> Option<&dyn CardBehavior> {
        self.cards.get(&id).map(|b| b.as_ref())
    }

    /// Look up a card ID by name.
    pub fn get_id_by_name(&self, name: &str) -> Option<CardId> {
        self.name_to_id.get(name).copied()
    }

    /// Get card data by ID.
    pub fn card_data(&self, id: CardId) -> Option<CardData> {
        self.get(id).map(|b| b.card_data())
    }

    /// Build a registry with all built-in cards.
    pub fn with_all_cards() -> Self {
        let mut reg = Self::new();
        // Lands
        reg.register(Box::new(forest::Forest));
        reg.register(Box::new(mountain::Mountain));
        reg.register(Box::new(plains::Plains));
        reg.register(Box::new(island::Island));
        reg.register(Box::new(swamp::Swamp));
        // Creatures
        reg.register(Box::new(kalonian_tusker::KalonianTusker));
        reg.register(Box::new(goblin_piker::GoblinPiker));
        reg.register(Box::new(savannah_lions::SavannahLions));
        reg.register(Box::new(grizzly_bears::GrizzlyBears));
        reg.register(Box::new(walking_corpse::WalkingCorpse));
        // Artifacts
        reg.register(Box::new(sol_ring::SolRing));
        // Instants
        reg.register(Box::new(lightning_bolt::LightningBolt));
        reg.register(Box::new(giant_growth::GiantGrowth));
        reg.register(Box::new(doom_blade::DoomBlade));
        reg.register(Box::new(swords_to_plowshares::SwordsToPlowshares));
        // Sorceries
        reg.register(Box::new(divination::Divination));
        reg.register(Box::new(lava_axe::LavaAxe));
        // Enchantments
        reg.register(Box::new(glorious_anthem::GloriousAnthem));
        reg.register(Box::new(holy_strength::HolyStrength));
        reg.register(Box::new(pacifism::Pacifism));
        reg
    }
}

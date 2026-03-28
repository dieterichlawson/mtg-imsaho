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
pub mod counterspell;
pub mod coral_merfolk;
pub mod thraben_purebloods;
pub mod rotting_fensnake;
pub mod riot_devils;
pub mod kindercatch;
pub mod fortress_crab;
pub mod abbey_griffin;
pub mod chapel_geist;
pub mod voiceless_spirit;
pub mod moon_heron;
pub mod typhoid_rats;
pub mod markov_patrician;
pub mod ambush_viper;
pub mod vampire_interloper;
pub mod spectral_rider;
pub mod invisible_stalker;
pub mod somberwald_spider;
pub mod diregraf_ghoul;
pub mod grave_bramble;
pub mod one_eyed_scarecrow;
pub mod moment_of_heroism;
pub mod rangers_guile;
pub mod spidery_grasp;
pub mod vampiric_fury;
pub mod rally_the_peasants;
pub mod hysterical_blindness;
pub mod dead_weight;
pub mod sensory_deprivation;
pub mod spectral_flight;
pub mod furor_of_the_bitten;
pub mod gruesome_deformity;
pub mod skeletal_grimace;
pub mod bonds_of_faith;
pub mod ghostly_possession;
pub mod claustrophobia;
pub mod bump_in_the_night;
pub mod geistflame;
pub mod brimstone_volley;
pub mod dissipate;
pub mod victim_of_night;
pub mod smite_the_monstrous;
pub mod rebuke;
pub mod silent_departure;
pub mod naturalize;
pub mod urgent_exorcism;
pub mod bramblecrush;
pub mod prey_upon;
pub mod frightful_delusion;
pub mod lost_in_the_mist;

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
    pub keywords: Vec<Keyword>,
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
    /// Target a spell on the stack (Counterspell)
    Spell,
    /// Target any permanent on the battlefield matching a filter (Naturalize, Bramblecrush)
    PermanentWithFilter(String),
    /// Two separate targets (Prey Upon, Lost in the Mist). Engine generates Cartesian product.
    TwoTargets(Box<TargetRequirement>, Box<TargetRequirement>),
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

    /// Keywords this permanent grants to the creature it's attached to (for auras).
    fn granted_keywords(&self) -> Vec<Keyword> {
        vec![]
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
        reg.register(Box::new(coral_merfolk::CoralMerfolk));
        reg.register(Box::new(thraben_purebloods::ThrabenPurebloods));
        reg.register(Box::new(rotting_fensnake::RottingFensnake));
        reg.register(Box::new(riot_devils::RiotDevils));
        reg.register(Box::new(kindercatch::Kindercatch));
        reg.register(Box::new(fortress_crab::FortressCrab));
        reg.register(Box::new(abbey_griffin::AbbeyGriffin));
        reg.register(Box::new(chapel_geist::ChapelGeist));
        reg.register(Box::new(voiceless_spirit::VoicelessSpirit));
        reg.register(Box::new(moon_heron::MoonHeron));
        reg.register(Box::new(typhoid_rats::TyphoidRats));
        reg.register(Box::new(markov_patrician::MarkovPatrician));
        reg.register(Box::new(ambush_viper::AmbushViper));
        reg.register(Box::new(vampire_interloper::VampireInterloper));
        reg.register(Box::new(spectral_rider::SpectralRider));
        reg.register(Box::new(invisible_stalker::InvisibleStalker));
        reg.register(Box::new(somberwald_spider::SomberwaldSpider));
        reg.register(Box::new(diregraf_ghoul::DiregrafGhoul));
        reg.register(Box::new(grave_bramble::GraveBramble));
        reg.register(Box::new(one_eyed_scarecrow::OneEyedScarecrow));
        // Artifacts
        reg.register(Box::new(sol_ring::SolRing));
        // Instants
        reg.register(Box::new(lightning_bolt::LightningBolt));
        reg.register(Box::new(giant_growth::GiantGrowth));
        reg.register(Box::new(doom_blade::DoomBlade));
        reg.register(Box::new(swords_to_plowshares::SwordsToPlowshares));
        reg.register(Box::new(counterspell::Counterspell));
        // Sorceries
        reg.register(Box::new(divination::Divination));
        reg.register(Box::new(lava_axe::LavaAxe));
        // Enchantments
        reg.register(Box::new(glorious_anthem::GloriousAnthem));
        reg.register(Box::new(holy_strength::HolyStrength));
        reg.register(Box::new(pacifism::Pacifism));
        // Innistrad instants
        reg.register(Box::new(moment_of_heroism::MomentOfHeroism));
        reg.register(Box::new(rangers_guile::RangersGuile));
        reg.register(Box::new(spidery_grasp::SpideryGrasp));
        reg.register(Box::new(vampiric_fury::VampiricFury));
        reg.register(Box::new(rally_the_peasants::RallyThePeasants));
        reg.register(Box::new(hysterical_blindness::HystericalBlindness));
        // Innistrad auras
        reg.register(Box::new(dead_weight::DeadWeight));
        reg.register(Box::new(sensory_deprivation::SensoryDeprivation));
        reg.register(Box::new(spectral_flight::SpectralFlight));
        reg.register(Box::new(furor_of_the_bitten::FurorOfTheBitten));
        reg.register(Box::new(gruesome_deformity::GruesomeDeformity));
        reg.register(Box::new(skeletal_grimace::SkeletalGrimace));
        reg.register(Box::new(bonds_of_faith::BondsOfFaith));
        reg.register(Box::new(ghostly_possession::GhostlyPossession));
        reg.register(Box::new(claustrophobia::Claustrophobia));
        // Innistrad instants/sorceries (Tier 2)
        reg.register(Box::new(bump_in_the_night::BumpInTheNight));
        reg.register(Box::new(geistflame::Geistflame));
        reg.register(Box::new(brimstone_volley::BrimstoneVolley));
        reg.register(Box::new(dissipate::Dissipate));
        reg.register(Box::new(victim_of_night::VictimOfNight));
        reg.register(Box::new(smite_the_monstrous::SmiteTheMonstrous));
        reg.register(Box::new(rebuke::Rebuke));
        reg.register(Box::new(silent_departure::SilentDeparture));
        reg.register(Box::new(naturalize::Naturalize));
        reg.register(Box::new(urgent_exorcism::UrgentExorcism));
        reg.register(Box::new(bramblecrush::Bramblecrush));
        reg.register(Box::new(prey_upon::PreyUpon));
        reg.register(Box::new(frightful_delusion::FrightfulDelusion));
        reg.register(Box::new(lost_in_the_mist::LostInTheMist));
        reg
    }
}

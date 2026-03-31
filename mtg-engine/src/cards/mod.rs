pub mod helpers;
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
pub mod midnight_haunting;
pub mod moan_of_the_unhallowed;
pub mod doomed_traveler;
pub mod mausoleum_guard;
pub mod village_bell_ringer;
pub mod slayer_of_the_wicked;
pub mod pitchburn_devils;
pub mod intangible_virtue;
pub mod falkenrath_noble;
pub mod rage_thrower;
pub mod unruly_mob;
pub mod lumberknot;
pub mod elder_cathar;
pub mod village_cannibals;
pub mod fiend_hunter;
pub mod think_twice;
pub mod feeling_of_dread;
pub mod dream_twist;
pub mod travel_preparations;
pub mod nightbirds_clutches;
pub mod gnaw_to_the_bone;
pub mod forbidden_alchemy;
pub mod rolling_temblor;
pub mod unburial_rites;
pub mod desperate_ravings;
pub mod festerhide_boar;
pub mod hollowhenge_scavenger;
pub mod morkrut_banshee;
pub mod crossway_vampire;
pub mod armored_skaab;
pub mod ancient_grudge;
pub mod battleground_geist;
pub mod gallows_warden;
pub mod orchard_spirit;
pub mod selhoff_occultist;
pub mod murder_of_crows;
pub mod spider_spawning;
pub mod wreath_of_geists;
pub mod geist_honored_monk;
pub mod geistcatchers_rig;
pub mod champion_of_the_parish;
pub mod stromkirk_noble;
pub mod stromkirk_patrol;
pub mod bloodcrazed_neonate;
pub mod falkenrath_marauders;
pub mod rakish_heir;
pub mod sturmgeist;
pub mod curiosity;
pub mod balefire_dragon;
pub mod abattoir_ghoul;
// Tier 7: Upkeep/end-step triggers + curses
pub mod boneyard_wurm;
pub mod splinterfright;
pub mod bloodgift_demon;
pub mod angel_of_flight_alabaster;
pub mod endless_ranks_of_the_dead;
pub mod charmbreaker_devils;
pub mod reaper_from_the_abyss;
pub mod curse_of_the_pierced_heart;
pub mod curse_of_the_bloody_tome;
pub mod curse_of_oblivion;
pub mod curse_of_the_nightly_hunt;
pub mod curse_of_deaths_hold;

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
    pub flashback_cost: Option<ManaCost>,
    /// Declarative continuous effects this card has while on the battlefield.
    /// The engine reads these instead of parsing oracle_text.
    pub continuous_effects: Vec<ContinuousEffect>,
    /// Triggered abilities this card has. The engine uses these to know which
    /// events this card cares about and to display trigger descriptions on the stack.
    pub triggered_abilities: Vec<TriggeredAbilityDef>,
}

/// A mana ability definition.
pub struct ManaAbilityDef {
    pub ability_index: usize,
    pub description: String,
    pub produced: Vec<(ManaType, u32)>,
    pub requires_tap: bool,
}

/// A non-mana activated ability definition.
pub struct ActivatedAbilityDef {
    pub ability_index: usize,
    pub description: String,
    pub cost: ManaCost,
    pub requires_tap: bool,
}

/// What kind of event triggers an ability.
#[derive(Debug, Clone, PartialEq)]
pub enum TriggerKind {
    /// When this permanent enters the battlefield.
    EntersBattlefield,
    /// When this creature dies (battlefield → graveyard).
    SelfDies,
    /// Whenever another creature dies (death-watch).
    AnyCreatureDies,
    /// Whenever another creature enters the battlefield (ETB-watch).
    AnyCreatureEnters,
    /// When this creature deals combat damage to a player.
    CombatDamageToPlayer,
    /// Whenever any creature deals combat damage to a player (watches others).
    AnyCombatDamageToPlayer,
    /// Whenever any creature deals damage (combat or non-combat) to a player.
    AnyDamageToPlayer,
    /// At the beginning of each upkeep.
    Upkeep,
    /// At the beginning of each end step.
    EndStep,
    /// When this permanent leaves the battlefield.
    LeavesBattlefield,
}

/// A triggered ability definition on a card.
#[derive(Debug, Clone)]
pub struct TriggeredAbilityDef {
    /// What event triggers this ability.
    pub kind: TriggerKind,
    /// Human-readable description of what the trigger does (for stack/log display).
    pub description: String,
}

/// Typed filter for creature or permanent targeting restrictions.
/// Used instead of stringly-typed filters for compile-time safety.
#[derive(Debug, Clone, PartialEq)]
pub enum TargetFilter {
    /// No additional restriction beyond the base type.
    Any,
    /// Only creatures/permanents you control.
    YouControl,
    /// Only creatures/permanents you don't control.
    YouDontControl,
    /// Only nonblack creatures (Doom Blade).
    Nonblack,
    /// Only non-Vampire, non-Werewolf, non-Zombie (Victim of Night).
    NotSubtypes(Vec<String>),
    /// Only creatures with power >= N (Smite the Monstrous).
    PowerAtLeast(i32),
    /// Only attacking creatures (Rebuke).
    Attacking,
    /// Only noncreature permanents (Bramblecrush).
    Noncreature,
    /// Only permanents with specific card types (Naturalize: artifact or enchantment).
    HasCardType(Vec<CardType>),
    /// Only permanents with specific subtypes or card types (Urgent Exorcism: Spirit or enchantment).
    SubtypeOrCardType { subtypes: Vec<String>, card_types: Vec<CardType> },
}

impl std::fmt::Display for TargetFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetFilter::Any => write!(f, "any"),
            TargetFilter::YouControl => write!(f, "you control"),
            TargetFilter::YouDontControl => write!(f, "you don't control"),
            TargetFilter::Nonblack => write!(f, "nonblack"),
            TargetFilter::NotSubtypes(types) => write!(f, "non-{}", types.join("/")),
            TargetFilter::PowerAtLeast(n) => write!(f, "power {}+", n),
            TargetFilter::Attacking => write!(f, "attacking"),
            TargetFilter::Noncreature => write!(f, "noncreature"),
            TargetFilter::HasCardType(types) => {
                let names: Vec<&str> = types.iter().map(|t| match t {
                    CardType::Artifact => "artifact",
                    CardType::Enchantment => "enchantment",
                    CardType::Creature => "creature",
                    CardType::Instant => "instant",
                    CardType::Sorcery => "sorcery",
                    CardType::Land => "land",
                    CardType::Planeswalker => "planeswalker",
                }).collect();
                write!(f, "{}", names.join(" or "))
            }
            TargetFilter::SubtypeOrCardType { subtypes, card_types } => {
                let mut parts: Vec<String> = subtypes.clone();
                for t in card_types {
                    parts.push(match t {
                        CardType::Enchantment => "enchantment".into(),
                        CardType::Artifact => "artifact".into(),
                        _ => format!("{:?}", t).to_lowercase(),
                    });
                }
                write!(f, "{}", parts.join(" or "))
            }
        }
    }
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
    CreatureWithFilter(TargetFilter),
    /// Target a player only (Lava Axe)
    PlayerOnly,
    /// Target a spell on the stack (Counterspell)
    Spell,
    /// Target any permanent on the battlefield matching a filter (Naturalize, Bramblecrush)
    PermanentWithFilter(TargetFilter),
    /// Two separate targets (Prey Upon, Lost in the Mist). Engine generates Cartesian product.
    TwoTargets(Box<TargetRequirement>, Box<TargetRequirement>),
    /// Up to N targets matching the inner requirement (Travel Preparations).
    UpToTargets(usize, Box<TargetRequirement>),
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
    fn is_valid_target(&self, _state: &GameState, _caster: PlayerId, _target: &Target, _registry: &CardRegistry) -> bool {
        true
    }

    /// Dynamic power/toughness for cards whose P/T depends on game state.
    /// Returns Some((power, toughness)) to override base P/T, or None for normal P/T.
    /// Called by effective_power/effective_toughness during P/T computation.
    /// Examples: Geist-Honored Monk (creatures you control), Wreath of Geists (creatures in graveyard).
    fn dynamic_pt(&self, _state: &GameState, _object_id: ObjectId) -> Option<(i32, i32)> {
        None
    }

    /// Called when this permanent enters the battlefield (ETB trigger).
    fn on_enter_battlefield(&self, _state: &mut GameState, _object_id: ObjectId, _registry: &CardRegistry) {}

    /// Called when this creature dies (moves from battlefield to graveyard).
    fn on_dies(&self, _state: &mut GameState, _object_id: ObjectId, _registry: &CardRegistry) {}

    /// Called when ANY creature dies. `self_id` is this permanent, `dead_id` is the deceased.
    /// `dead_damaged_by` and `dead_toughness` are last-known information captured before
    /// the zone change clears battlefield state.
    fn on_any_creature_dies(&self, _state: &mut GameState, _self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _registry: &CardRegistry) {}

    /// Called when ANY creature enters the battlefield. `self_id` is this permanent, `entered_id` is the new creature.
    /// Similar to on_any_creature_dies but for ETB. Used by Champion of the Parish.
    fn on_any_creature_enters(&self, _state: &mut GameState, _self_id: ObjectId, _entered_id: ObjectId, _entered_controller: PlayerId, _registry: &CardRegistry) {}

    /// Called when this creature deals combat damage to a player.
    /// Used by Stromkirk Noble, Falkenrath Marauders, Sturmgeist, etc.
    fn on_combat_damage_to_player(&self, _state: &mut GameState, _self_id: ObjectId, _damaged_player: PlayerId, _amount: u32, _registry: &CardRegistry) {}

    /// Called when ANY creature deals combat damage to a player.
    /// `self_id` is this permanent (the watcher), `source_id` is the creature that dealt damage.
    /// Used by Rakish Heir (watches Vampires).
    fn on_any_combat_damage_to_player(&self, _state: &mut GameState, _self_id: ObjectId, _source_id: ObjectId, _damaged_player: PlayerId, _amount: u32, _registry: &CardRegistry) {}

    /// Called when ANY creature deals damage (combat or non-combat) to a player.
    /// Used by Curiosity (watches enchanted creature).
    fn on_any_damage_to_player(&self, _state: &mut GameState, _self_id: ObjectId, _source_id: ObjectId, _damaged_player: PlayerId, _amount: u32, _registry: &CardRegistry) {}

    /// Called at the beginning of the upkeep step for each permanent with an upkeep trigger.
    fn on_upkeep(&self, _state: &mut GameState, _self_id: ObjectId, _registry: &CardRegistry) {}

    /// Called at the beginning of the end step for each permanent with an end-step trigger.
    fn on_end_step(&self, _state: &mut GameState, _self_id: ObjectId, _registry: &CardRegistry) {}

    /// Called when this permanent leaves the battlefield (moves to any other zone).
    fn on_leave_battlefield(&self, _state: &mut GameState, _object_id: ObjectId, _registry: &CardRegistry) {}

    /// List of mana abilities this permanent has while on the battlefield.
    fn mana_abilities(&self, _state: &GameState, _object_id: ObjectId) -> Vec<ManaAbilityDef> {
        vec![]
    }

    /// List of non-mana activated abilities this permanent has.
    fn activated_abilities(&self, _state: &GameState, _object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
        vec![]
    }

    /// Called when a non-mana activated ability is activated.
    fn on_activate_ability(&self, _state: &mut GameState, _object_id: ObjectId, _ability_index: usize, _registry: &CardRegistry) {}

    /// Called when this spell resolves from the stack.
    /// `targets` contains the targets chosen at cast time.
    /// For permanents: default moves to battlefield.
    /// For instants/sorceries: override to apply effect.
    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
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
        // Token creators and trigger cards
        reg.register(Box::new(midnight_haunting::MidnightHaunting));
        reg.register(Box::new(moan_of_the_unhallowed::MoanOfTheUnhallowed));
        reg.register(Box::new(doomed_traveler::DoomedTraveler));
        reg.register(Box::new(mausoleum_guard::MausoleumGuard));
        reg.register(Box::new(village_bell_ringer::VillageBellRinger));
        reg.register(Box::new(slayer_of_the_wicked::SlayerOfTheWicked));
        reg.register(Box::new(pitchburn_devils::PitchburnDevils));
        reg.register(Box::new(intangible_virtue::IntangibleVirtue));
        // Death triggers and +1/+1 counter creatures
        reg.register(Box::new(falkenrath_noble::FalkenrathNoble));
        reg.register(Box::new(rage_thrower::RageThrower));
        reg.register(Box::new(unruly_mob::UnrulyMob));
        reg.register(Box::new(lumberknot::Lumberknot));
        reg.register(Box::new(elder_cathar::ElderCathar));
        reg.register(Box::new(village_cannibals::VillageCannibals));
        reg.register(Box::new(fiend_hunter::FiendHunter));
        // Innistrad flashback cards
        reg.register(Box::new(think_twice::ThinkTwice));
        reg.register(Box::new(feeling_of_dread::FeelingOfDread));
        reg.register(Box::new(dream_twist::DreamTwist));
        reg.register(Box::new(travel_preparations::TravelPreparations));
        reg.register(Box::new(nightbirds_clutches::NightbirdsClutches));
        reg.register(Box::new(gnaw_to_the_bone::GnawToTheBone));
        reg.register(Box::new(forbidden_alchemy::ForbiddenAlchemy));
        reg.register(Box::new(rolling_temblor::RollingTemblor));
        reg.register(Box::new(unburial_rites::UnburialRites));
        reg.register(Box::new(desperate_ravings::DesperateRavings));
        // Innistrad Tier 5 creatures
        reg.register(Box::new(festerhide_boar::FesterhideBoar));
        reg.register(Box::new(hollowhenge_scavenger::HollowhengeScavenger));
        reg.register(Box::new(morkrut_banshee::MorkrutBanshee));
        reg.register(Box::new(crossway_vampire::CrosswayVampire));
        reg.register(Box::new(armored_skaab::ArmoredSkaab));
        reg.register(Box::new(ancient_grudge::AncientGrudge));
        reg.register(Box::new(battleground_geist::BattlegroundGeist));
        reg.register(Box::new(gallows_warden::GallowsWarden));
        reg.register(Box::new(orchard_spirit::OrchardSpirit));
        reg.register(Box::new(selhoff_occultist::SelhoffOccultist));
        // Innistrad Tier 5 (dynamic P/T, death triggers, tokens)
        reg.register(Box::new(murder_of_crows::MurderOfCrows));
        reg.register(Box::new(spider_spawning::SpiderSpawning));
        reg.register(Box::new(wreath_of_geists::WreathOfGeists));
        reg.register(Box::new(geist_honored_monk::GeistHonoredMonk));
        reg.register(Box::new(geistcatchers_rig::GeistcatchersRig));
        // Innistrad Tier 6 creatures
        reg.register(Box::new(champion_of_the_parish::ChampionOfTheParish));
        reg.register(Box::new(stromkirk_noble::StromkirkNoble));
        reg.register(Box::new(stromkirk_patrol::StromkirkPatrol));
        reg.register(Box::new(bloodcrazed_neonate::BloodcrazedNeonate));
        reg.register(Box::new(falkenrath_marauders::FalkenrathMarauders));
        reg.register(Box::new(rakish_heir::RakishHeir));
        reg.register(Box::new(sturmgeist::Sturmgeist));
        reg.register(Box::new(curiosity::Curiosity));
        reg.register(Box::new(balefire_dragon::BalefireDragon));
        reg.register(Box::new(abattoir_ghoul::AbattoirGhoul));
        // Innistrad Tier 7 cards
        reg.register(Box::new(boneyard_wurm::BoneyardWurm));
        reg.register(Box::new(splinterfright::Splinterfright));
        reg.register(Box::new(bloodgift_demon::BloodgiftDemon));
        reg.register(Box::new(angel_of_flight_alabaster::AngelOfFlightAlabaster));
        reg.register(Box::new(endless_ranks_of_the_dead::EndlessRanksOfTheDead));
        reg.register(Box::new(charmbreaker_devils::CharmbreakerDevils));
        reg.register(Box::new(reaper_from_the_abyss::ReaperFromTheAbyss));
        reg.register(Box::new(curse_of_the_pierced_heart::CurseOfThePiercedHeart));
        reg.register(Box::new(curse_of_the_bloody_tome::CurseOfTheBloodyTome));
        reg.register(Box::new(curse_of_oblivion::CurseOfOblivion));
        reg.register(Box::new(curse_of_the_nightly_hunt::CurseOfTheNightlyHunt));
        reg.register(Box::new(curse_of_deaths_hold::CurseOfDeathsHold));
        reg
    }
}

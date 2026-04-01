use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// The 5 colors of Magic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Color {
    White,
    Blue,
    Black,
    Red,
    Green,
}

/// What can exist in a mana pool. Includes colorless (not a color, but a mana type).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ManaType {
    White,
    Blue,
    Black,
    Red,
    Green,
    Colorless,
}

impl From<Color> for ManaType {
    fn from(c: Color) -> Self {
        match c {
            Color::White => ManaType::White,
            Color::Blue => ManaType::Blue,
            Color::Black => ManaType::Black,
            Color::Red => ManaType::Red,
            Color::Green => ManaType::Green,
        }
    }
}

/// A single mana symbol in a cost.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManaSymbol {
    /// A colored mana symbol: {W}, {U}, {B}, {R}, {G}
    Colored(Color),
    /// Generic mana: {1}, {2}, etc. Can be paid with any type.
    Generic(u32),
    /// Specifically colorless mana: {C}
    Colorless(u32),
    /// Variable cost
    X,
    // Future: Hybrid(Color, Color), PhyrexianColored(Color), Snow, etc.
}

/// A complete mana cost (e.g., {2}{R}{R} for a spell).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManaCost {
    pub symbols: Vec<ManaSymbol>,
}

impl ManaCost {
    pub fn new(symbols: Vec<ManaSymbol>) -> Self {
        Self { symbols }
    }

    pub fn free() -> Self {
        Self { symbols: vec![] }
    }

    /// Converted mana cost (now called "mana value").
    pub fn mana_value(&self) -> u32 {
        self.symbols.iter().map(|s| match s {
            ManaSymbol::Colored(_) => 1,
            ManaSymbol::Generic(n) => *n,
            ManaSymbol::Colorless(n) => *n,
            ManaSymbol::X => 0,
        }).sum()
    }

    /// How much colored mana of each color is required.
    pub fn colored_requirements(&self) -> HashMap<Color, u32> {
        let mut reqs = HashMap::new();
        for sym in &self.symbols {
            if let ManaSymbol::Colored(c) = sym {
                *reqs.entry(*c).or_insert(0) += 1;
            }
        }
        reqs
    }

    /// Total generic mana required.
    pub fn generic_amount(&self) -> u32 {
        self.symbols.iter().map(|s| match s {
            ManaSymbol::Generic(n) => *n,
            _ => 0,
        }).sum()
    }

    /// Total colorless mana specifically required.
    pub fn colorless_amount(&self) -> u32 {
        self.symbols.iter().map(|s| match s {
            ManaSymbol::Colorless(n) => *n,
            _ => 0,
        }).sum()
    }
}

impl std::fmt::Display for ManaCost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for sym in &self.symbols {
            match sym {
                ManaSymbol::Generic(n) => write!(f, "{{{}}}", n)?,
                ManaSymbol::Colored(c) => write!(f, "{{{}}}", match c {
                    Color::White => "W",
                    Color::Blue => "U",
                    Color::Black => "B",
                    Color::Red => "R",
                    Color::Green => "G",
                })?,
                ManaSymbol::Colorless(n) => {
                    for _ in 0..*n {
                        write!(f, "{{C}}")?;
                    }
                }
                ManaSymbol::X => write!(f, "{{X}}")?,
            }
        }
        Ok(())
    }
}

/// Floating mana a player has available.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManaPool {
    pub mana: HashMap<ManaType, u32>,
}

impl ManaPool {
    pub fn new() -> Self {
        Self { mana: HashMap::new() }
    }

    pub fn add(&mut self, mana_type: ManaType, amount: u32) {
        *self.mana.entry(mana_type).or_insert(0) += amount;
    }

    pub fn get(&self, mana_type: ManaType) -> u32 {
        *self.mana.get(&mana_type).unwrap_or(&0)
    }

    pub fn total(&self) -> u32 {
        self.mana.values().sum()
    }

    pub fn empty(&mut self) {
        self.mana.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

impl Default for ManaPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Card types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CardType {
    Land,
    Creature,
    Instant,
    Sorcery,
    Enchantment,
    Artifact,
    Planeswalker,
}

impl CardType {
    pub fn is_permanent(&self) -> bool {
        matches!(self, CardType::Land | CardType::Creature | CardType::Enchantment
            | CardType::Artifact | CardType::Planeswalker)
    }
}

/// Supertypes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Supertype {
    Basic,
    Legendary,
    Snow,
}

/// Zones where objects can exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Zone {
    Library,
    Hand,
    Battlefield,
    Graveyard,
    Stack,
    Exile,
    Command,
}

/// Turn phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
    Beginning,
    PrecombatMain,
    Combat,
    PostcombatMain,
    Ending,
}

/// Steps within phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Step {
    Untap,
    Upkeep,
    Draw,
    PrecombatMain,
    BeginCombat,
    DeclareAttackers,
    DeclareBlockers,
    CombatDamage,
    EndCombat,
    PostcombatMain,
    EndStep,
    Cleanup,
}

impl Step {
    pub fn phase(&self) -> Phase {
        match self {
            Step::Untap | Step::Upkeep | Step::Draw => Phase::Beginning,
            Step::PrecombatMain => Phase::PrecombatMain,
            Step::BeginCombat | Step::DeclareAttackers | Step::DeclareBlockers
                | Step::CombatDamage | Step::EndCombat => Phase::Combat,
            Step::PostcombatMain => Phase::PostcombatMain,
            Step::EndStep | Step::Cleanup => Phase::Ending,
        }
    }

    pub fn next(&self) -> Option<Step> {
        match self {
            Step::Untap => Some(Step::Upkeep),
            Step::Upkeep => Some(Step::Draw),
            Step::Draw => Some(Step::PrecombatMain),
            Step::PrecombatMain => Some(Step::BeginCombat),
            Step::BeginCombat => Some(Step::DeclareAttackers),
            Step::DeclareAttackers => Some(Step::DeclareBlockers),
            Step::DeclareBlockers => Some(Step::CombatDamage),
            Step::CombatDamage => Some(Step::EndCombat),
            Step::EndCombat => Some(Step::PostcombatMain),
            Step::PostcombatMain => Some(Step::EndStep),
            Step::EndStep => Some(Step::Cleanup),
            Step::Cleanup => None, // end of turn
        }
    }

    /// Does this step normally grant priority to players?
    pub fn has_priority(&self) -> bool {
        match self {
            Step::Untap => false,
            Step::Cleanup => false, // normally no priority unless triggered abilities fire
            _ => true,
        }
    }

    /// Is this a main phase step where sorcery-speed actions are allowed?
    pub fn is_main_phase(&self) -> bool {
        matches!(self, Step::PrecombatMain | Step::PostcombatMain)
    }
}

/// Counter types that can exist on permanents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CounterType {
    PlusOnePlusOne,
    MinusOneMinusOne,
    Loyalty,
    Slime,
    // extend as needed
}

/// Keyword abilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Keyword {
    Flying,
    FirstStrike,
    DoubleStrike,
    Trample,
    Deathtouch,
    Lifelink,
    Vigilance,
    Flash,
    Reach,
    Haste,
    Defender,
    Hexproof,
    Intimidate,
    Menace,
    Indestructible,
}

/// Describes which creatures a continuous effect applies to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CreatureFilter {
    /// All creatures you control.
    You,
    /// All creatures opponents control.
    Opponents,
    /// Token creatures you control.
    YourTokens,
    /// Creatures matching a subtype (e.g., "Human", "Zombie").
    HasSubtype(String),
    /// Creatures with a specific keyword.
    HasKeyword(Keyword),
    /// Intersection: all conditions must match.
    And(Vec<CreatureFilter>),
    /// Union: any condition matches.
    Or(Vec<CreatureFilter>),
    /// Negation.
    Not(Box<CreatureFilter>),
    /// Creatures controlled by the player this curse/aura is attached to.
    AttachedPlayer,
}

/// Where a continuous effect applies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EffectScope {
    /// Affects only the permanent this effect is on (static keyword on a creature).
    OnSelf,
    /// Affects the creature this aura/equipment is attached to.
    Attached,
    /// Affects all creatures matching a filter, INCLUDING the source permanent.
    /// Used for "Creatures you control get +1/+1" (Glorious Anthem).
    Global(CreatureFilter),
    /// Affects all creatures matching a filter, EXCLUDING the source permanent.
    /// Used for "Other Spirit creatures you control get +1/+0" (lords).
    GlobalOther(CreatureFilter),
}

/// A declarative continuous effect on a card. The engine reads these
/// instead of parsing oracle text strings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContinuousEffect {
    /// Modify power and/or toughness.
    ModifyPT { power: i32, toughness: i32, scope: EffectScope },
    /// Grant a keyword ability.
    GrantKeyword { keyword: Keyword, scope: EffectScope },
    /// Creature can't attack.
    PreventAttack { scope: EffectScope },
    /// Creature can't block.
    PreventBlock { scope: EffectScope },
    /// Creature can't be blocked.
    CantBeBlocked { scope: EffectScope },
    /// Creature can only be blocked by creatures matching the filter.
    /// Used for Orchard Spirit (flying/reach), Skulk (power < N), Shadow, Fear, etc.
    BlockRestriction { allowed_blockers: CreatureFilter, scope: EffectScope },
    /// Prevent all combat damage dealt to and by creature.
    PreventCombatDamage { scope: EffectScope },
    /// Creature doesn't untap during controller's untap step.
    PreventUntap { scope: EffectScope },
    /// Creature attacks each combat if able.
    ForceAttack { scope: EffectScope },
    /// Protection from a subtype (prevents damage, blocking, targeting, enchanting).
    ProtectionFromSubtype { subtype: String, scope: EffectScope },
    /// Protection from creatures matching a filter.
    ProtectionFrom { filter: CreatureFilter, scope: EffectScope },
    /// Conditional keyword: granted only when a condition is met.
    ConditionalKeyword { keyword: Keyword, condition: EffectCondition, scope: EffectScope },
    /// Reduce cost of spells matching a filter.
    ReduceCost { reduction: u32, filter: SpellFilter },
    /// Creature can't be blocked except by N or more creatures.
    /// Used by Terror of Kruin Pass: "Each Werewolf you control can't be blocked
    /// except by two or more creatures."
    RequireMinBlockers { min_blockers: u32, scope: EffectScope },
    /// The controller of this permanent has hexproof (can't be targeted by opponents).
    /// Used by Witchbane Orb.
    GrantPlayerHexproof,
}

/// Condition for conditional effects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EffectCondition {
    /// You control a permanent with this subtype.
    YouControlSubtype(String),
    /// An opponent controls a permanent with this subtype.
    OpponentControlsSubtype(String),
    /// This permanent has the specified keyword (for conditional abilities like
    /// "has indestructible as long as it has defender").
    SelfHasKeyword(Keyword),
}

/// Filter for which spells a cost reduction applies to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SpellFilter {
    /// All creature spells.
    CreatureSpells,
    /// Creature spells with a specific subtype.
    CreatureWithSubtype(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mana_cost_value() {
        // {2}{R}{R} = 4
        let cost = ManaCost::new(vec![
            ManaSymbol::Generic(2),
            ManaSymbol::Colored(Color::Red),
            ManaSymbol::Colored(Color::Red),
        ]);
        assert_eq!(cost.mana_value(), 4);
    }

    #[test]
    fn mana_cost_colored_requirements() {
        let cost = ManaCost::new(vec![
            ManaSymbol::Generic(1),
            ManaSymbol::Colored(Color::Green),
            ManaSymbol::Colored(Color::Green),
        ]);
        let reqs = cost.colored_requirements();
        assert_eq!(reqs.get(&Color::Green), Some(&2));
        assert_eq!(cost.generic_amount(), 1);
    }

    #[test]
    fn mana_cost_display() {
        let cost = ManaCost::new(vec![
            ManaSymbol::Generic(2),
            ManaSymbol::Colored(Color::Red),
        ]);
        assert_eq!(format!("{}", cost), "{2}{R}");
    }

    #[test]
    fn mana_pool_operations() {
        let mut pool = ManaPool::new();
        assert!(pool.is_empty());

        pool.add(ManaType::Green, 2);
        pool.add(ManaType::Red, 1);
        assert_eq!(pool.total(), 3);
        assert_eq!(pool.get(ManaType::Green), 2);
        assert_eq!(pool.get(ManaType::Blue), 0);

        pool.empty();
        assert!(pool.is_empty());
    }

    #[test]
    fn step_progression() {
        let mut step = Step::Untap;
        let expected = vec![
            Step::Upkeep, Step::Draw, Step::PrecombatMain, Step::BeginCombat,
            Step::DeclareAttackers, Step::DeclareBlockers, Step::CombatDamage,
            Step::EndCombat, Step::PostcombatMain, Step::EndStep, Step::Cleanup,
        ];
        for expected_step in expected {
            step = step.next().unwrap();
            assert_eq!(step, expected_step);
        }
        assert!(step.next().is_none());
    }
}

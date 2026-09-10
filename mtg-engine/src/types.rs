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

impl Color {
    /// Every color, in WUBRG order — the order a card's characteristics are
    /// printed and read in.
    pub const ALL: [Color; 5] =
        [Color::White, Color::Blue, Color::Black, Color::Red, Color::Green];

    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Color::White => "White",
            Color::Blue => "Blue",
            Color::Black => "Black",
            Color::Red => "Red",
            Color::Green => "Green",
        }
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// A permanent's colors as a player reads them, in WUBRG order — or
/// "Colorless", which is a real answer and not an absence (CR 105.2c: a
/// colorless permanent shares a color with nothing, which is the whole of
/// what intimidate asks).
///
/// No pane in the game printed a permanent's color at all, and intimidate
/// (CR 702.13a) is decided entirely by it. For most permanents a player
/// could infer it from the mana cost in the CARDS panel; for a face with no
/// mana cost they could not infer it from anything, because CR 204.2 gives
/// that face its color with an indicator instead. A defender facing a
/// Gatstaf Howler had no way to learn it was green, and so no way to know
/// that a Grizzly Bears could block it and a Doomed Traveler could not,
/// other than by reading back the legal-blocker list the engine had already
/// computed for them — the game handed them the answer and withheld the
/// reason (issue #357).
#[must_use]
pub fn colors_line(colors: &[Color]) -> String {
    if colors.is_empty() {
        return "Colorless".to_string();
    }
    Color::ALL.iter()
        .filter(|c| colors.contains(c))
        .map(|c| c.label())
        .collect::<Vec<_>>()
        .join(", ")
}

/// What can exist in a mana pool. Includes colorless (not a color, but a mana type).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
    #[must_use]
    pub fn new(symbols: Vec<ManaSymbol>) -> Self {
        Self { symbols }
    }

    #[must_use]
    pub fn free() -> Self {
        Self { symbols: vec![] }
    }

    /// This cost with X taken as zero.
    ///
    /// Two uses. When a cost containing {X} is being paid other than by
    /// casting the spell, X is 0 and there is no announcement (CR 107.3e) —
    /// Back from the Brink's "pay its mana cost" is the case here. And when
    /// the engine checks affordability *before* asking the player for a value
    /// of X, this is the part it can test.
    #[must_use]
    pub fn without_x(&self) -> Self {
        Self {
            symbols: self.symbols.iter()
                .filter(|s| !matches!(s, ManaSymbol::X))
                .cloned()
                .collect(),
        }
    }

    /// Whether this cost contains {X}.
    #[must_use]
    pub fn has_x(&self) -> bool {
        self.symbols.iter().any(|s| matches!(s, ManaSymbol::X))
    }

    /// Converted mana cost (now called "mana value").
    #[must_use]
    pub fn mana_value(&self) -> u32 {
        self.symbols.iter().map(|s| match s {
            ManaSymbol::Colored(_) => 1,
            ManaSymbol::Generic(n) | ManaSymbol::Colorless(n) => *n,
            ManaSymbol::X => 0,
        }).sum()
    }

    /// How much colored mana of each color is required.
    #[must_use]
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
    #[must_use]
    pub fn generic_amount(&self) -> u32 {
        self.symbols.iter().map(|s| match s {
            ManaSymbol::Generic(n) => *n,
            _ => 0,
        }).sum()
    }

    /// Total colorless mana specifically required.
    #[must_use]
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
                ManaSymbol::Generic(n) => write!(f, "{{{n}}}")?,
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
///
/// A `BTreeMap` rather than a `HashMap` because the pool gets iterated —
/// displayed, cloned into funding options, drained type by type — and a
/// `HashMap`'s iteration order is seeded per process, so the same seeded
/// game replayed would show (and could spend) mana in a different order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManaPool {
    pub mana: std::collections::BTreeMap<ManaType, u32>,
}

impl ManaPool {
    #[must_use]
    pub fn new() -> Self {
        Self { mana: std::collections::BTreeMap::new() }
    }

    pub fn add(&mut self, mana_type: ManaType, amount: u32) {
        *self.mana.entry(mana_type).or_insert(0) += amount;
    }

    /// Subtract `amount` of `mana_type` from the pool.
    ///
    /// # Panics
    /// Panics on underflow (caller is responsible for checking availability).
    pub fn sub(&mut self, mana_type: ManaType, amount: u32) {
        let current = self.get(mana_type);
        assert!(
            current >= amount,
            "mana pool underflow: tried to remove {amount} {mana_type:?} but pool has {current}"
        );
        self.mana.insert(mana_type, current - amount);
    }

    #[must_use]
    pub fn get(&self, mana_type: ManaType) -> u32 {
        *self.mana.get(&mana_type).unwrap_or(&0)
    }

    #[must_use]
    pub fn total(&self) -> u32 {
        self.mana.values().sum()
    }

    pub fn empty(&mut self) {
        self.mana.clear();
    }

    #[must_use]
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
    /// How the type reads on a type line: "Creature", "Planeswalker". One
    /// label, in the engine — the CLI and the LLM harness each carried five
    /// copies of this match.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            CardType::Land => "Land",
            CardType::Creature => "Creature",
            CardType::Instant => "Instant",
            CardType::Sorcery => "Sorcery",
            CardType::Enchantment => "Enchantment",
            CardType::Artifact => "Artifact",
            CardType::Planeswalker => "Planeswalker",
        }
    }

    #[must_use]
    pub fn is_permanent(&self) -> bool {

        matches!(self, CardType::Land | CardType::Creature | CardType::Enchantment
            | CardType::Artifact | CardType::Planeswalker)
    }
}

/// Supertypes (CR 205.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Supertype {
    Basic,
    Legendary,
    Snow,
}

impl Supertype {
    /// How the supertype reads on a type line.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Supertype::Basic => "Basic",
            Supertype::Legendary => "Legendary",
            Supertype::Snow => "Snow",
        }
    }
}

impl std::fmt::Display for CardType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

impl std::fmt::Display for Supertype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// The type line as a card prints it (CR 205.1): supertypes, then card
/// types, then an em dash and the subtypes — `Legendary Creature — Human
/// Cleric`, `Basic Land — Forest`, `Instant`.
///
/// CR 205.4a puts the supertypes first, and nothing in the game ever
/// printed them: every pane built its type line from card types and
/// subtypes alone, so the first a player learned that a permanent was
/// legendary was the legend-rule prompt that took one away (issue #333).
#[must_use]
pub fn type_line(supertypes: &[Supertype], card_types: &[CardType], subtypes: &[String]) -> String {
    let mut words: Vec<&str> = supertypes.iter().map(|s| s.label()).collect();
    words.extend(card_types.iter().map(|t| t.label()));
    let head = words.join(" ");
    if subtypes.is_empty() {
        head
    } else {
        format!("{head} — {}", subtypes.join(" "))
    }
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
    #[must_use]
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

    #[must_use]
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
    #[must_use]
    pub fn has_priority(&self) -> bool {
        match self {
            Step::Untap | Step::Cleanup => false, // normally no priority unless triggered abilities fire
            _ => true,
        }
    }

    /// Is this a main phase step where sorcery-speed actions are allowed?
    #[must_use]
    pub fn is_main_phase(&self) -> bool {
        matches!(self, Step::PrecombatMain | Step::PostcombatMain)
    }
}

/// Counter types that can exist on permanents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CounterType {
    PlusOnePlusOne,
    MinusOneMinusOne,
    Loyalty,
    Slime,
    Study,
    Hatchling,
    // extend as needed
}

impl CounterType {
    /// How this counter reads to a player: "+1/+1", "slime". One label, in
    /// the engine, because the log and the CLI's battlefield line each had
    /// their own and only one of them existed for a while.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            CounterType::PlusOnePlusOne => "+1/+1",
            CounterType::MinusOneMinusOne => "-1/-1",
            CounterType::Loyalty => "loyalty",
            CounterType::Slime => "slime",
            CounterType::Study => "study",
            CounterType::Hatchling => "hatchling",
        }
    }
}

impl std::fmt::Display for CounterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
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

impl Keyword {
    /// Every keyword, so a sweep over them cannot silently miss one that was
    /// added after it was written.
    pub const ALL: [Keyword; 15] = [
        Keyword::Flying,
        Keyword::FirstStrike,
        Keyword::DoubleStrike,
        Keyword::Trample,
        Keyword::Deathtouch,
        Keyword::Lifelink,
        Keyword::Vigilance,
        Keyword::Flash,
        Keyword::Reach,
        Keyword::Haste,
        Keyword::Defender,
        Keyword::Hexproof,
        Keyword::Intimidate,
        Keyword::Menace,
        Keyword::Indestructible,
    ];

    /// The word a card prints this keyword as, in the lowercase a rules-text
    /// list uses ("first strike", "vigilance").
    ///
    /// There were five copies of this table — two panes of the CLI, the
    /// `i` inspector, the LLM seat's prompt and a test's own — and the two
    /// that did not have it Debug-formatted the variant instead, so a
    /// battlefield row and a combat prompt spelled Elite Inquisitor's
    /// `FirstStrike` as "firststrike" while the CARDS pane one column over
    /// spelled the same keyword on the same creature "First strike"
    /// (issue #363). `FirstStrike` and `DoubleStrike` are the only variants
    /// whose Debug name is not already the printed word, which is why the
    /// other thirteen hid it. The Debug name is a Rust identifier; it is
    /// not a name anything shows a player.
    ///
    /// Lowercase because that is the form a card's oracle text prints, which
    /// is what `every_declared_keyword_is_printed_on_the_card` checks these
    /// against — Wizards' text, not another copy of this table.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Keyword::Flying => "flying",
            Keyword::FirstStrike => "first strike",
            Keyword::DoubleStrike => "double strike",
            Keyword::Trample => "trample",
            Keyword::Deathtouch => "deathtouch",
            Keyword::Lifelink => "lifelink",
            Keyword::Vigilance => "vigilance",
            Keyword::Flash => "flash",
            Keyword::Reach => "reach",
            Keyword::Haste => "haste",
            Keyword::Defender => "defender",
            Keyword::Hexproof => "hexproof",
            Keyword::Intimidate => "intimidate",
            Keyword::Menace => "menace",
            Keyword::Indestructible => "indestructible",
        }
    }
}

impl std::fmt::Display for Keyword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Describes which creatures a continuous effect applies to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CreatureFilter {
    /// All creatures you control.
    ControlledByYou,
    /// All creatures opponents control.
    ControlledByOpponent,
    /// Token creatures you control.
    ControlledByYouToken,
    /// Creatures matching a subtype (e.g., "Human", "Zombie").
    HasSubtype(String),
    /// Objects of a card type. A `CreatureFilter` is also used to describe the
    /// *sources* an effect cares about (protection), and those are not all
    /// creatures — without this, Spare from Evil's "non-Human creatures"
    /// could only be written as `Not(HasSubtype("Human"))`, which a Brimstone
    /// Volley also satisfies.
    HasCardType(CardType),
    /// Creatures with a specific keyword.
    HasKeyword(Keyword),
    /// Intersection: all conditions must match.
    And(Vec<CreatureFilter>),
    /// Union: any condition matches.
    Or(Vec<CreatureFilter>),
    /// Negation.
    Not(Box<CreatureFilter>),
    /// Creatures controlled by the player this curse/aura is attached to.
    ControlledByAttachedPlayer,
    /// Creatures currently attacking (CR 506.3a — attacking from the moment
    /// they are declared until they leave combat or combat ends).
    ///
    /// Only meaningful inside a combat phase; outside one, nothing matches.
    Attacking,
    /// Creatures currently blocking.
    Blocking,
}

/// The plural of a creature type, for the one place the engine has to name a
/// group of them to a player ("protection from Werewolves").
///
/// Appending a bare `s` is what this replaces: `Werewolf` came out as
/// `Werewolfs` on every frame an Elite Inquisitor was on the battlefield,
/// contradicting the oracle text the CARDS pane printed beside it (issue
/// #324). English `-f`/`-fe` nouns are the family Magic's creature types keep
/// hitting — Wolf, Werewolf, Elf, Dwarf — so the rule, not the one word, is
/// what is fixed here.
#[must_use]
pub fn plural_of(subtype: &str) -> String {
    let lower = subtype.to_lowercase();
    if let Some(stem) = lower.strip_suffix("fe") {
        return format!("{}ves", &subtype[..stem.len()]);
    }
    if lower.ends_with('f') {
        return format!("{}ves", &subtype[..subtype.len() - 1]);
    }
    if lower.ends_with('s') || lower.ends_with('x') || lower.ends_with('z')
        || lower.ends_with("ch") || lower.ends_with("sh")
    {
        return format!("{subtype}es");
    }
    // A consonant before the `y`: "Ally" is "Allies", but "Monkey" is
    // "Monkeys".
    if let Some(stem) = lower.strip_suffix('y') {
        if !stem.ends_with(['a', 'e', 'i', 'o', 'u']) {
            return format!("{}ies", &subtype[..stem.len()]);
        }
    }
    format!("{subtype}s")
}

impl CreatureFilter {
    /// A short phrase naming what this filter matches, for a screen that has
    /// to say what a permanent has protection from. "non-Human creatures",
    /// not `Not(HasSubtype("Human"))`.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            CreatureFilter::ControlledByYou => "creatures you control".into(),
            CreatureFilter::ControlledByOpponent => "creatures your opponents control".into(),
            CreatureFilter::ControlledByYouToken => "creature tokens you control".into(),
            CreatureFilter::ControlledByAttachedPlayer => "creatures the enchanted player controls".into(),
            CreatureFilter::HasSubtype(s) => plural_of(s),
            CreatureFilter::HasCardType(t) => format!("{t:?}s").to_lowercase(),
            CreatureFilter::HasKeyword(k) => format!("creatures with {k:?}").to_lowercase(),
            CreatureFilter::Attacking => "attacking creatures".into(),
            CreatureFilter::Blocking => "blocking creatures".into(),
            CreatureFilter::And(parts) => parts.iter()
                .map(CreatureFilter::describe).collect::<Vec<_>>().join(" and "),
            CreatureFilter::Or(parts) => parts.iter()
                .map(CreatureFilter::describe).collect::<Vec<_>>().join(" or "),
            // "non-Human creatures" reads better than "not Humans", and the
            // negation of a subtype is how every protection in this set is
            // written.
            CreatureFilter::Not(inner) => match inner.as_ref() {
                CreatureFilter::HasSubtype(s) => format!("non-{s} creatures"),
                other => format!("anything that is not {}", other.describe()),
            },
        }
    }
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
    CanOnlyBeBlockedBy { allowed_blockers: CreatureFilter, scope: EffectScope },
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
    /// CR 611.2b: another effect, applying only while `condition` holds.
    ///
    /// This used to be four parallel variants — `ConditionalKeyword`,
    /// `ConditionalModifyPT`, `ConditionalPreventAttack`,
    /// `ConditionalPreventBlock` — each with its own walk over the
    /// battlefield. Wrapping instead of duplicating means "as long as" can
    /// qualify any of the effects below, not the four somebody happened to
    /// need, and there is one place that evaluates the condition.
    When { condition: EffectCondition, effect: Box<ContinuousEffect> },
    /// Reduce cost of spells matching a filter.
    ReduceCost { reduction: u32, filter: SpellFilter },
    /// Provide an alternative cost for spells matching a filter.
    /// Used by Rooftop Storm ({0} for Zombies).
    AlternativeCost { cost: ManaCost, filter: SpellFilter },
    /// Creature can't be blocked except by N or more creatures.
    /// Used for "can't be blocked except by two or more creatures" (Terror of Kruin Pass).
    MinimumBlockers { count: u32, scope: EffectScope },
    /// If this creature would be dealt damage, prevent that damage and remove a +1/+1 counter.
    /// Used by Unbreathing Horde.
    PreventDamageRemoveCounter { scope: EffectScope },
    /// Double combat damage dealt by and to this creature.
    /// Used by Inquisitor's Flail.
    DoubleCombatDamage { scope: EffectScope },
    /// Activated abilities of artifacts can't be activated (Stony Silence).
    PreventArtifactAbilities,
    /// Spells with the named card can't be cast (Nevermore).
    PreventCastingNamed { name: String },
}

impl ContinuousEffect {
    /// `effect`, applying only while `condition` holds (CR 611.2b).
    #[must_use]
    pub fn when(condition: EffectCondition, effect: ContinuousEffect) -> Self {
        ContinuousEffect::When { condition, effect: Box::new(effect) }
    }

    /// Which objects this effect applies to, or `None` for one that modifies
    /// the rules of the game rather than a permanent — `PreventArtifactAbilities`,
    /// `PreventCastingNamed` — or that adjusts a cost rather than an object.
    ///
    /// Every scoped variant answers here, so a caller asks *what* it is
    /// looking for and never has to remember to pull the scope out itself.
    #[must_use]
    pub fn scope(&self) -> Option<&EffectScope> {
        match self {
            ContinuousEffect::ModifyPT { scope, .. }
            | ContinuousEffect::GrantKeyword { scope, .. }
            | ContinuousEffect::PreventAttack { scope }
            | ContinuousEffect::PreventBlock { scope }
            | ContinuousEffect::CantBeBlocked { scope }
            | ContinuousEffect::CanOnlyBeBlockedBy { scope, .. }
            | ContinuousEffect::PreventCombatDamage { scope }
            | ContinuousEffect::PreventUntap { scope }
            | ContinuousEffect::ForceAttack { scope }
            | ContinuousEffect::ProtectionFromSubtype { scope, .. }
            | ContinuousEffect::ProtectionFrom { scope, .. }
            | ContinuousEffect::MinimumBlockers { scope, .. }
            | ContinuousEffect::PreventDamageRemoveCounter { scope }
            | ContinuousEffect::DoubleCombatDamage { scope } => Some(scope),
            // A conditional effect's scope is the scope of what it wraps.
            ContinuousEffect::When { effect, .. } => effect.scope(),
            ContinuousEffect::ReduceCost { .. }
            | ContinuousEffect::AlternativeCost { .. }
            | ContinuousEffect::PreventArtifactAbilities
            | ContinuousEffect::PreventCastingNamed { .. } => None,
        }
    }

    /// The effect itself and the condition guarding it, if any. Unwraps
    /// `When` so a caller can match on what the effect *is* without knowing
    /// whether it happens to be conditional.
    #[must_use]
    pub fn unwrap_condition(&self) -> (&ContinuousEffect, Option<&EffectCondition>) {
        match self {
            ContinuousEffect::When { condition, effect } => (effect, Some(condition)),
            other => (other, None),
        }
    }
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
    /// The creature this aura is attached to has the specified subtype.
    AttachedHasSubtype(String),
    /// The creature this aura is attached to does NOT have the specified subtype.
    AttachedLacksSubtype(String),
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
        assert_eq!(format!("{cost}"), "{2}{R}");
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

    /// CR 205.1/205.4a: supertypes, then types, then an em dash and the
    /// subtypes. "Legendary" was printed nowhere in the game (issue #333).
    #[test]
    fn a_type_line_leads_with_the_supertypes() {
        assert_eq!(
            type_line(&[Supertype::Legendary], &[CardType::Creature], &["Human".into(), "Cleric".into()]),
            "Legendary Creature — Human Cleric");
        assert_eq!(type_line(&[Supertype::Basic], &[CardType::Land], &["Forest".into()]),
            "Basic Land — Forest");
        assert_eq!(type_line(&[], &[CardType::Instant], &[]), "Instant");
        assert_eq!(type_line(&[], &[CardType::Artifact, CardType::Creature], &["Golem".into()]),
            "Artifact Creature — Golem");
        assert_eq!(type_line(&[Supertype::Legendary], &[CardType::Planeswalker], &["Liliana".into()]),
            "Legendary Planeswalker — Liliana");
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

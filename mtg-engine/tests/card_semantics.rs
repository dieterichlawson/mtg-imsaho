//! Cards against their Oracle text, read mechanically: tier 3 of
//! `docs/plans/formal-verification.md`.
//!
//! The acceptance tests (`cards_*.rs`) check that a card does what its text
//! says in the situations someone wrote down. This file checks the same
//! thing from the other end. A small grammar reads each instant's and
//! sorcery's Oracle text, as Scryfall prints it (`data/oracle_cache.json`),
//! into a list of effects ("deals 3 damage to any target", "destroy target
//! nonblack creature", "create two 1/1 white Spirit creature tokens with
//! flying"), and each creature whose text it can read — keywords, entering
//! tapped or with counters, power and toughness defined by a count, an
//! enters trigger whose effect is one of the spell templates — into the
//! permanent it becomes and what its triggers do as it enters and as it
//! dies, and each enchantment it can read — an Aura's "Enchant creature" or
//! "Enchant player", and static changes to power, toughness and keywords —
//! into what it does while it is on the battlefield, each land it can read
//! into whether it enters tapped and what mana it makes, and the activated
//! abilities it can read ("{1}{W}, Sacrifice this creature: Exile target
//! enchantment.") into their costs and effects, and the werewolves, both
//! faces, into when they transform. The keywords themselves are held to
//! what they do in combat: every creature whose text is keywords attacks,
//! unblocked and against each of the others, under a reference model of
//! CR 508-510. A reference semantics,
//! written from the
//! Comprehensive Rules and sharing no code with `src/cards/`, says what
//! those effects do to a board described by the Oracle data of the cards on
//! it. Then every card the grammar reads is cast at every set of targets
//! the engine offers on a board of fourteen permanents and four graveyard
//! cards (and, for morbid, on the same board after a creature has died),
//! and the engine is held to two things:
//!
//! - **the targets**: the engine offers exactly the target sets the text
//!   allows (CR 115.1, 601.2c) — a Doom Blade that offers a black creature,
//!   or a Smite the Monstrous that leaves out a 6/6, fails here;
//! - **the outcome**: after the spell resolves and state-based actions are
//!   checked, every player's life, hand, graveyard, exile and library, and
//!   every permanent's controller, power, toughness, damage, counters,
//!   keywords, colours, subtypes and tapped state are what the reference
//!   says. A creature's trigger asks its own questions (a target as it goes
//!   on the stack, "you may" as it resolves), so for a creature the engine
//!   is played every way its questions can be answered, and the set of
//!   games that can result must be the set the text allows: everything the
//!   rules permit is reachable, and nothing else.
//!
//! The grammar is the reviewed part: each template is one sentence shape
//! with its CR reference, and a card is covered only when every sentence of
//! its text matches one. A card it cannot read is not checked here, and is
//! counted: `the_grammar_reads_at_least_as_many_cards_as_it_did` keeps the
//! coverage from shrinking silently. Reading the text rather than
//! transcribing each card keeps the specification independent of whoever
//! wrote the card, which is the point: the engine's card says what its
//! author understood, and this says what the printed words say.

mod common;

use std::collections::BTreeMap;

use common::*;
use mtg_engine::actions::{Action, CastTargetSpec, CombatPrompt, ResolvedChoice, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};
use mtg_engine::types::*;

// ---- the Oracle data ----

/// One card as Scryfall prints it.
#[derive(Debug, Clone)]
struct Printed {
    name: String,
    mana_cost: String,
    type_line: String,
    oracle_text: String,
    power: Option<i32>,
    toughness: Option<i32>,
    /// A double-faced card's back face (CR 712.1), with no mana cost.
    back: Option<Box<Printed>>,
    /// The colours of a colour indicator, which a back face has in place
    /// of a mana cost (CR 204.1).
    color_indicator: Vec<Color>,
}

/// [`oracle_cache`], read once.
fn cached_oracle() -> &'static BTreeMap<String, Printed> {
    static CACHE: std::sync::OnceLock<BTreeMap<String, Printed>> = std::sync::OnceLock::new();
    CACHE.get_or_init(oracle_cache)
}

/// The cards in the attacker's graveyard in an arena.
const ARENA_GRAVEYARD: [&str; 2] = ["Walking Corpse", "Grizzly Bears"];

fn oracle_cache() -> BTreeMap<String, Printed> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../data/oracle_cache.json");
    let raw = std::fs::read_to_string(path).expect("data/oracle_cache.json is readable");
    let json: serde_json::Value = serde_json::from_str(&raw).expect("the oracle cache is JSON");
    let int = |v: &serde_json::Value| v.as_str().and_then(|s| s.parse().ok());
    let face = |c: &serde_json::Value, name: &str| Printed {
        name: name.to_string(),
        mana_cost: c["mana_cost"].as_str().unwrap_or("").to_string(),
        type_line: c["type_line"].as_str().unwrap_or("").to_string(),
        oracle_text: c["oracle_text"].as_str().unwrap_or("").to_string(),
        power: int(&c["power"]),
        toughness: int(&c["toughness"]),
        back: None,
        color_indicator: c["color_indicator"].as_array().into_iter().flatten()
            .filter_map(|l| Color::ALL.into_iter().find(|k| l.as_str() == Some(&color_letter(*k).to_string())))
            .collect(),
    };
    json["cards"].as_object().expect("a cards map").iter()
        .map(|(name, c)| {
            let mut front = face(c, name);
            if let Some(b) = c.get("back_face").filter(|b| b.is_object()) {
                front.back = Some(Box::new(face(b, b["name"].as_str().unwrap_or(""))));
            }
            (name.clone(), front)
        })
        .collect()
}

impl Printed {
    /// The card types and subtypes on its type line (CR 205).
    fn types(&self) -> (Vec<String>, Vec<String>) {
        let (front, back) = self.type_line.split_once(" — ").unwrap_or((&self.type_line, ""));
        (front.split_whitespace().map(str::to_string).collect(),
         back.split_whitespace().map(str::to_string).collect())
    }

    fn has_type(&self, t: &str) -> bool {
        self.types().0.iter().any(|x| x == t)
    }

    /// Its colours: those of its colour indicator if it has one (CR 204.1),
    /// and the colours of the mana symbols in its cost (202.2).
    fn colors(&self) -> Vec<Color> {
        if !self.color_indicator.is_empty() {
            return self.color_indicator.clone();
        }
        let mut colors: Vec<Color> = Color::ALL.into_iter()
            .filter(|c| self.mana_cost.contains(&format!("{{{}}}", color_letter(*c))))
            .collect();
        colors.dedup();
        colors
    }

    /// The keywords the engine models, as the Oracle text prints them on a
    /// line of their own.
    fn keywords(&self) -> Vec<Keyword> {
        let mut out = Vec::new();
        for line in without_reminders(&self.oracle_text).lines() {
            // Protection rides on a keyword line ("Defender, protection
            // from Zombies") without being one of the engine's keywords.
            let words: Vec<&str> = line.split(',').map(str::trim)
                .filter(|w| !w.to_lowercase().starts_with("protection from "))
                .collect();
            let all: Option<Vec<Keyword>> = words.iter().map(|w| keyword_named(w)).collect();
            if let Some(ks) = all {
                out.extend(ks);
            }
        }
        out.sort_by_key(|k| k.label());
        out
    }
}

fn color_letter(c: Color) -> char {
    match c {
        Color::White => 'W',
        Color::Blue => 'U',
        Color::Black => 'B',
        Color::Red => 'R',
        Color::Green => 'G',
    }
}

fn color_named(word: &str) -> Option<Color> {
    Color::ALL.into_iter().find(|c| c.label().eq_ignore_ascii_case(word))
}

fn keyword_named(word: &str) -> Option<Keyword> {
    Keyword::ALL.into_iter().find(|k| k.label().eq_ignore_ascii_case(word))
}

/// The text with reminder text (in parentheses, CR 207.2a) removed.
fn without_reminders(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for ch in text.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out.lines().map(str::trim_end).collect::<Vec<_>>().join("\n")
}

/// A printed mana cost, `{2}{R}`, as symbols.
fn mana_cost(printed: &str) -> Option<ManaCost> {
    let mut symbols = Vec::new();
    for sym in printed.split('}').filter(|s| !s.is_empty()) {
        let sym = sym.strip_prefix('{')?;
        if let Ok(n) = sym.parse::<u32>() {
            if n > 0 {
                symbols.push(ManaSymbol::Generic(n));
            }
        } else if let Some(c) = Color::ALL.into_iter().find(|c| sym == color_letter(*c).to_string()) {
            symbols.push(ManaSymbol::Colored(c));
        } else if sym == "X" {
            symbols.push(ManaSymbol::X);
        } else {
            return None;
        }
    }
    Some(ManaCost::new(symbols))
}

// ---- the grammar ----

/// What a target slot accepts, as the text words it (CR 115.1).
#[derive(Debug, Clone, PartialEq)]
enum Slot {
    /// "any target": a creature, a player or a planeswalker (CR 115.4).
    Any,
    /// "target player or planeswalker".
    PlayerOrPlaneswalker,
    /// "target player".
    Player,
    /// "target opponent".
    Opponent,
    /// "target <permanents>".
    Permanent(Filter),
    /// "up to N target <permanents>": any set of at most N of them
    /// (CR 601.2c). Only ever a card's one slot here, so its targets are
    /// the cast's whole target list.
    UpTo { n: usize, filter: Filter },
    /// "target card from a graveyard", "target creature card from your
    /// graveyard": a card, not a permanent (CR 115.1, 404).
    GraveyardCard { filter: Option<Filter>, yours: bool },
    /// "target spell", "target noncreature spell": a spell on the stack
    /// (CR 115.1, 405).
    Spell(Filter),
    /// "this creature", "this land": the ability's own source. Not a
    /// target; its place in a target list is filled with the source.
    This,
    /// "enchanted creature" in an Aura's trigger: the creature it enchants.
    /// Not a target.
    EnchantedCreature,
    /// "that player" in a Curse's trigger: the player it enchants; with
    /// `or_planeswalker`, "that player or a planeswalker that player
    /// controls", which its controller chooses. Not a target.
    EnchantedPlayer { or_planeswalker: bool },
}

/// A description of permanents: any one of the alternatives ("artifact or
/// enchantment").
#[derive(Debug, Clone, PartialEq)]
struct Filter(Vec<Clause>);

/// One alternative: a head noun and what qualifies it.
#[derive(Debug, Clone, PartialEq, Default)]
struct Clause {
    /// A card type ("creature"), a subtype ("Spirit"), or none for
    /// "permanent".
    head: Option<String>,
    /// "nonblack": colours it must not have.
    not_colors: Vec<Color>,
    /// "noncreature", "nonlegendary", "non-Vampire": types, supertypes
    /// and subtypes it must not have.
    not_types: Vec<String>,
    /// "Vampire creatures": subtypes it must have.
    subtypes: Vec<String>,
    /// "with power 4 or greater".
    min_power: Option<i32>,
    /// "without flying".
    without: Vec<Keyword>,
    /// "with flying".
    with: Vec<Keyword>,
    /// "another": not the source of the ability (CR 109.5).
    other: bool,
    /// "Creature tokens": only tokens (CR 111.1).
    token: bool,
    /// "you control" (Some(true)), "you don't control" / "your opponents
    /// control" (Some(false)).
    yours: Option<bool>,
    /// "attacking".
    attacking: bool,
}

/// One thing a spell does, in the order its text says it (CR 608.2c).
#[derive(Debug, Clone, PartialEq)]
enum Effect {
    /// "deals N damage to <slot>" (CR 120.3).
    Damage { amount: u32, slot: usize },
    /// "deals N damage to each <filter>".
    DamageEach { amount: u32, filter: Filter },
    /// "<slot> gets +P/+T [and gains K] until end of turn" (CR 611.2).
    Pump { slot: usize, power: i32, toughness: i32, gains: Vec<Keyword> },
    /// "<filter> get +P/+T [and gain K] until end of turn".
    PumpEach { filter: Filter, power: i32, toughness: i32, gains: Vec<Keyword> },
    /// "Destroy target ..." (CR 701.8).
    Destroy { slot: usize },
    /// "Exile target ..." (CR 701.13 then 406).
    Exile { slot: usize },
    /// "Return target ... to its owner's hand".
    ToHand { slot: usize },
    /// "Put target ... on top of its owner's library".
    ToLibraryTop { slot: usize },
    /// "Tap target ..." / "Untap target ...".
    Tap { slot: usize, tapped: bool },
    /// "Draw N cards" (CR 121.1).
    Draw { n: usize },
    /// "You gain N life" (CR 119.3).
    GainLife { n: i32 },
    /// "<player slot> loses N life".
    LoseLife { slot: usize, n: i32 },
    /// "<player slot> mills N cards" (CR 701.17).
    Mill { slot: usize, n: usize },
    /// "Create N [tapped] P/T <colour> <subtypes> creature tokens [with K]"
    /// (CR 111.2).
    /// "... for each creature card in your graveyard": `n` that many times.
    Tokens { n: usize, power: i32, toughness: i32, colors: Vec<Color>, subtypes: Vec<String>, keywords: Vec<Keyword>, tapped: bool, per_creature_card: bool },
    /// "Its controller gains life equal to its power": the power the
    /// object last had on the battlefield (CR 608.2h).
    ControllerGainsLifeEqualToPower { slot: usize },
    /// "Put a +1/+1 counter on each of <slot>" (CR 122.1a).
    PlusCounters { slot: usize, n: u32 },
    /// "<slot a> fights <slot b>": each deals damage equal to its power to
    /// the other (CR 701.12a).
    Fight { a: usize, b: usize },
    /// "Destroy all <filter>".
    DestroyAll { filter: Filter },
    /// "You gain N life for each <thing> destroyed this way".
    GainLifePerDestroyed { n: i32 },
    /// "You gain N life for each creature card in your graveyard".
    GainLifePerCreatureCard { n: i32 },
    /// "Return target ... card from your graveyard to the battlefield
    /// [with a -1/-1 counter on it]" (CR 110.2: under its owner's, the
    /// caster's, control).
    Reanimate { slot: usize, minus_counter: bool },
    /// "Counter target spell" (CR 701.6a): it leaves the stack for its
    /// owner's graveyard, or exile when the text says "exile it instead".
    Counter { slot: usize, exile: bool },
    /// "Mill N cards": the controller mills (CR 701.17).
    MillYou { n: usize },
    /// "Untap all <filter>".
    UntapAll { filter: Filter },
    /// "Each player mills N cards", in turn order from the active player
    /// (CR 101.4).
    MillEach { n: usize },
    /// "Put a +1/+1 counter on each <filter>".
    PlusCountersEach { filter: Filter },
    /// "~ deals damage equal to the exiled card's power to <slot>": the
    /// card exiled to pay for it (CR 601.2h, 608.2h).
    DamageEqualToExiledPower { slot: usize },
    /// "<player slot> draws N cards".
    PlayerDraws { slot: usize, n: usize },
    /// "~ deals X damage to <slot>" (CR 107.3).
    DamageX { slot: usize },
    /// "Regenerate this creature": a shield (CR 701.19a).
    Regenerate { slot: usize },
    /// "Transform this creature": it turns to its other face, unless it has
    /// transformed since the ability was put on the stack (CR 701.27a, f).
    TransformThis { slot: usize },
    /// "Gain control of target creature until end of turn" or "for as long
    /// as you control ~" (CR 613.2, 611.2b).
    GainControl { slot: usize, until_end_of_turn: bool },
    /// "That creature becomes a Vampire in addition to its other types"
    /// (CR 205.1b): for good.
    BecomesSubtype { slot: usize, subtype: String },
    /// "If that creature is a <subtype>, put N +1/+1 counters on it
    /// instead": the counters of the `PlusCounters` before it, `n_if` of
    /// them for a creature of that subtype.
    PlusCountersIfSubtype { slot: usize, n: u32, subtype: String, n_if: u32 },
    /// "<slot> gets +X/+X until end of turn, where X is the number of
    /// <filter>": counted as it resolves.
    PumpPerCount { slot: usize, power: i32, toughness: i32, filter: Filter },
    /// "Add {R}{R}{R}." (CR 106.4).
    AddMana(Vec<(ManaType, u32)>),
    /// "Exile target creature and all other creatures with the same name
    /// as that creature" (CR 201.2).
    ExileWithSameName { slot: usize },
    /// "<player slot> exiles a card from their graveyard": the card in
    /// `World::chosen`, which that player chooses; "If it's a creature
    /// card, you gain N life".
    PlayerExilesFromGraveyard { slot: usize },
    GainLifeIfExiledCreature { n: i32 },
    /// "<player slot> discards N cards": the whole hand here, no hand on
    /// the board holding more (CR 701.9a).
    PlayerDiscards { slot: usize, n: usize },
    /// "<player slot> puts the bottom card of their library into their
    /// graveyard. If it's a creature card, you create <token>".
    BottomCardToGraveyard { slot: usize },
    TokenIfCreatureCard(Box<Effect>),
    /// "Exchange your life total with this creature's toughness" (CR
    /// 701.10, 613.4b).
    ExchangeLifeWithToughness { slot: usize },
    /// "discard N cards at random" (CR 701.9b): the cards in
    /// `World::discarded`.
    DiscardAtRandom { n: usize },
    /// "Return N <filter> cards at random from your graveyard to your hand"
    /// or "to the battlefield": the cards in `World::chosen`.
    ReturnAtRandom { n: usize, filter: Option<Filter>, to_battlefield: bool },
    /// "Search your library for a basic land card, reveal it, put it into
    /// your hand, then shuffle", or for "its controller", the controller of
    /// the permanent in `of` as it was, onto the battlefield; with
    /// `morbid_battlefield`, "you may put that card onto the battlefield
    /// instead ... if a creature died this turn" (CR 701.19).
    SearchForBasicLand { of: Option<usize>, to_battlefield: bool, morbid_battlefield: bool },
    /// A dies trigger's "exile it": the card the creature became, from its
    /// owner's graveyard (CR 400.7, 608.2h).
    ExileItFromGraveyard,
    /// "<player slot> sacrifices a creature of their choice" (CR 701.21a):
    /// the creature in `World::chosen`.
    SacrificeOfTheirChoice { slot: usize },
    /// "Each player chooses a creature they control." (CR 101.4: in turn
    /// order), and "Destroy the rest.": every other creature, at once.
    EachPlayerChoosesACreature,
    DestroyTheRest,
    /// "Regenerate all <filter>".
    RegenerateAll { filter: Filter },
    /// "You gain life equal to the sacrificed creature's toughness": its
    /// toughness as it last existed on the battlefield (CR 608.2h).
    GainLifeEqualToSacrificedToughness,
    /// "<slot> gets +X/+0 [and gains K] until end of turn": `power` and
    /// `toughness` times X.
    PumpX { slot: usize, power: i32, toughness: i32, gains: Vec<Keyword> },
    /// Equip's "Attach to target creature you control" (CR 702.6a, 701.3):
    /// the source becomes attached to it, coming off what it was on.
    Attach { slot: usize },
    /// "That player exiles N cards from their graveyard": of their choice,
    /// all of them when they have no more than N.
    ExileFromGraveyard { slot: usize, n: usize },
}

/// A card's text, read.
#[derive(Debug, Clone, PartialEq, Default)]
struct Spec {
    slots: Vec<Slot>,
    effects: Vec<Effect>,
    /// "This spell costs {1} less to cast for each creature on the
    /// battlefield" (CR 601.2f).
    cheaper_per_creature: bool,
    /// "Morbid — ~ deals N damage instead if a creature died this turn":
    /// which effect's amount, and what it becomes (CR 207.2c: an ability
    /// word has no rules meaning; the condition is the text after it).
    morbid: Option<(usize, u32)>,
    /// A creature spell whose text is only keywords: it resolves into a
    /// permanent with its printed characteristics (CR 608.3).
    permanent: bool,
    /// "Flash" (CR 702.8): castable whenever its controller has priority.
    flash: bool,
    /// "This creature enters tapped" (CR 614.1c).
    enters_tapped: bool,
    /// "Morbid — This creature enters with N +1/+1 counters on it if a
    /// creature died this turn" (CR 614.1c).
    enters_with_counters_if_morbid: Option<u32>,
    /// "~'s power and toughness are each equal to the number of creatures
    /// you control" (CR 604.3, 613.4a).
    pt_is_creatures_you_control: bool,
    /// "When this creature enters, ..." (CR 603.6a), read as a spec of its
    /// own: its targets are chosen as it is put on the stack (603.3d).
    etb: Option<Box<Trigger>>,
    /// "When this creature dies, ..." (CR 700.4, 603.6c): a leaves-the-
    /// battlefield trigger, which looks back at the creature as it was.
    dies: Option<Box<Trigger>>,
    /// "Whenever another creature dies, ...": a trigger on other deaths.
    watch: Option<Box<Watch>>,
    /// "At the beginning of your upkeep, ..." or, on a Curse, "... of
    /// enchanted player's upkeep, ...".
    upkeep: Option<Box<Upkeep>>,
    /// An Aura's "Enchant creature" / "Enchant player" (CR 303.4a, 702.5):
    /// the slot its spell targets, the first of `slots`.
    enchants: bool,
    /// Static abilities: continuous effects while it is on the battlefield
    /// (CR 604.1, 611.3).
    statics: Vec<Static>,
    /// A land, and the basic land types that let it enter untapped ("This
    /// land enters tapped unless you control a Mountain or a Plains"; none
    /// for a land that always enters untapped).
    land: Option<Option<Vec<String>>>,
    /// Mana abilities (CR 605.1a), a basic land's from its type (305.6).
    mana: Vec<ManaAbility>,
    /// Activated abilities other than mana abilities (CR 602).
    activated: Vec<Activated>,
    /// "As an additional cost to cast this spell, sacrifice a creature" or
    /// "exile N creature cards from your graveyard" (CR 601.2b, 601.2h).
    additional: Option<Additional>,
    /// "~'s power and toughness are each equal to the number of creature
    /// cards in your graveyard" (CR 604.3).
    pt_is_creature_cards: bool,
    /// "... equal to the number of cards in your hand".
    pt_is_cards_in_hand: bool,
    /// What it says about attacking, blocking and combat damage.
    combat: CombatText,
    /// A werewolf: "At the beginning of each upkeep, if no spells were cast
    /// last turn, transform this creature", and on its back face, whose
    /// text is read too, "... if a player cast two or more spells last
    /// turn, transform this creature" (CR 603.4, 701.27, 712).
    werewolf: bool,
}

/// A mana ability: "{T}: Add {R} or {W}.", "{T}: Add {C}{C}.", "{1}, {T}:
/// Add one mana of any color.", "{T}, Mill a card: Add {C}." (CR 605.1a).
#[derive(Debug, Clone, PartialEq)]
struct ManaAbility {
    /// Mana paid to activate it, all generic here.
    cost: u32,
    /// Cards its controller mills to activate it.
    mill: usize,
    /// What one activation can make: one choice of these.
    options: Vec<Vec<(ManaType, u32)>>,
}

impl ManaAbility {
    /// Read "<cost>: Add <mana>.", or nothing if the line is not a mana
    /// ability the grammar knows.
    fn read(line: &str) -> Option<ManaAbility> {
        let (cost, add) = line.split_once(": Add ")?;
        let add = add.strip_suffix('.')?;
        let mut ability = ManaAbility { cost: 0, mill: 0, options: Vec::new() };
        let mut tap = false;
        for part in cost.split(", ") {
            match part {
                "{T}" => tap = true,
                "Mill a card" => ability.mill = 1,
                _ => ability.cost = part.strip_prefix('{')?.strip_suffix('}')?.parse().ok()?,
            }
        }
        if !tap {
            return None;
        }
        if add == "one mana of any color" {
            ability.options = Color::ALL.into_iter().map(|c| vec![(ManaType::from(c), 1)]).collect();
        } else {
            for option in add.split(" or ") {
                // "{C}{C}" is two of one kind in one activation.
                let mut made: Vec<(ManaType, u32)> = Vec::new();
                for sym in option.split_inclusive('}') {
                    let mt = mana_named(sym)?;
                    match made.iter_mut().find(|(t, _)| *t == mt) {
                        Some((_, n)) => *n += 1,
                        None => made.push((mt, 1)),
                    }
                }
                ability.options.push(made);
            }
        }
        Some(ability)
    }

    /// How many of the engine's ability indices it takes: one per option,
    /// as the cards declare them (Shimmering Grotto's "any color" is five).
    fn indices(&self) -> usize {
        self.options.len()
    }
}

/// A creature's text about combat: restrictions on blocking it or by it,
/// protection, forced attacks, conditional haste, and what dealing combat
/// damage to a player does.
#[derive(Debug, Clone, PartialEq, Default)]
struct CombatText {
    /// "This creature can't block." (CR 509.1a)
    cant_block: bool,
    /// "This creature can't be blocked."
    cant_be_blocked: bool,
    /// "This creature can't be blocked except by creatures with flying or
    /// reach."
    only_by_flying_or_reach: bool,
    /// "This creature can't be blocked by Humans.": the subtypes that
    /// can't block it.
    not_blocked_by: Vec<String>,
    /// "Protection from Zombies" (CR 702.16): the subtypes it has
    /// protection from. It can't be blocked by them (702.16f) and damage
    /// they would deal it is prevented (702.16e).
    protection_from: Vec<String>,
    /// "This creature attacks each combat if able." (CR 508.1d)
    attacks_each_combat: bool,
    /// "This creature has haste as long as an opponent controls a Human."
    haste_if_opponent_controls: Option<String>,
    /// "Whenever this creature deals combat damage to a player, ..." (CR
    /// 510.3a, 603.2).
    on_damage_to_player: Vec<OnPlayerDamage>,
    /// "Whenever this creature attacks, create <tokens> that's tapped and
    /// attacking [for each creature card in your graveyard]. [Exile that
    /// token at end of combat.]" (CR 508.4).
    on_attack_tokens: Option<AttackTokens>,
    /// "Whenever this creature blocks or becomes blocked by a creature,
    /// this creature deals N damage to that creature." (CR 509.4)
    block_damage: Option<u32>,
    /// An Equipment's "Whenever equipped creature blocks or becomes blocked
    /// by a <type>, destroy that creature. It can't be regenerated."
    stake: Option<String>,
    /// "Whenever a <type> you control deals combat damage to a player, put
    /// a +1/+1 counter on it."
    counters_on_yours: Option<String>,
    /// A Curse's "Whenever a creature deals combat damage to enchanted
    /// player, put a +1/+1 counter on that creature."
    counters_on_damage_to_enchanted: bool,
}

/// Tokens a creature's attack makes, tapped and attacking.
#[derive(Debug, Clone, PartialEq)]
struct AttackTokens {
    power: i32,
    toughness: i32,
    colors: Vec<Color>,
    subtypes: Vec<String>,
    keywords: Vec<Keyword>,
    per_creature_card: bool,
    exile_at_end_of_combat: bool,
}

/// What "whenever this creature deals combat damage to a player" does.
#[derive(Debug, Clone, PartialEq)]
enum OnPlayerDamage {
    /// "put a +1/+1 counter on it", "put two +1/+1 counters on it".
    CountersOnSelf(u32),
    /// "draw a card".
    Draw(usize),
    /// "it deals that much damage to each creature that player controls".
    ThatMuchToEachCreatureThatPlayerControls,
}

/// "Humans" → "Human", "Werewolves" → "Werewolf": a subtype from its
/// plural, as rules text prints it.
fn singular(word: &str) -> String {
    match word.strip_suffix("ves") {
        Some(stem) => format!("{stem}f"),
        None => word.strip_suffix('s').unwrap_or(word).to_string(),
    }
}

impl CombatText {
    /// Read one line of a creature's text about combat, if it is one.
    fn read(&mut self, line: &str) -> Option<()> {
        match line {
            "This creature can't block." => self.cant_block = true,
            "This creature can't be blocked." => self.cant_be_blocked = true,
            "This creature can't be blocked except by creatures with flying or reach." => self.only_by_flying_or_reach = true,
            "This creature attacks each combat if able." => self.attacks_each_combat = true,
            _ => {
                if let Some(t) = line.strip_prefix("This creature can't be blocked by ").and_then(|r| r.strip_suffix('.')) {
                    self.not_blocked_by.push(singular(t));
                } else if let Some(t) = line.strip_prefix("This creature has haste as long as an opponent controls a ").and_then(|r| r.strip_suffix('.')) {
                    self.haste_if_opponent_controls = Some(t.to_string());
                } else if let Some(effect) = line.strip_prefix("Whenever this creature deals combat damage to a player, ").and_then(|r| r.strip_suffix('.')) {
                    let e = match effect {
                        "draw a card" => OnPlayerDamage::Draw(1),
                        "it deals that much damage to each creature that player controls" => OnPlayerDamage::ThatMuchToEachCreatureThatPlayerControls,
                        _ => {
                            let n = effect.strip_prefix("put ")?
                                .strip_suffix(" +1/+1 counter on it")
                                .or_else(|| effect.strip_prefix("put ")?.strip_suffix(" +1/+1 counters on it"))?;
                            OnPlayerDamage::CountersOnSelf(number(n)?)
                        }
                    };
                    self.on_damage_to_player.push(e);
                } else if let Some(rest) = line.strip_prefix("Whenever this creature attacks, create ").or_else(|| line.strip_prefix("Whenever ~ attacks, create ")) {
                    let (rest, exile_at_end_of_combat) = match rest.strip_suffix(" Exile that token at end of combat.") {
                        Some(r) => (r, true),
                        None => (rest, false),
                    };
                    let rest = rest.strip_suffix('.')?;
                    if !rest.contains(" that's tapped and attacking") {
                        return None;
                    }
                    let mut tokens = Spec::default();
                    tokens.tokens(&rest.replacen(" that's tapped and attacking", "", 1))?;
                    let Some(Effect::Tokens { n: 1, power, toughness, colors, subtypes, keywords, tapped: false, per_creature_card }) = tokens.effects.pop() else {
                        return None;
                    };
                    self.on_attack_tokens = Some(AttackTokens { power, toughness, colors, subtypes, keywords, per_creature_card, exile_at_end_of_combat });
                } else if let Some(n) = line.strip_prefix("Whenever this creature blocks or becomes blocked by a creature, this creature deals ")
                    .and_then(|r| r.strip_suffix(" damage to that creature."))
                {
                    self.block_damage = Some(number(n)?);
                } else if let Some(t) = line.strip_prefix("Whenever a ")
                    .and_then(|r| r.strip_suffix(" you control deals combat damage to a player, put a +1/+1 counter on it."))
                {
                    self.counters_on_yours = Some(t.to_string());
                } else if let Some(t) = line.strip_prefix("Whenever equipped creature blocks or becomes blocked by a ")
                    .and_then(|r| r.strip_suffix(", destroy that creature. It can't be regenerated."))
                {
                    self.stake = Some(t.to_string());
                } else if line == "Whenever a creature deals combat damage to enchanted player, put a +1/+1 counter on that creature." {
                    self.counters_on_damage_to_enchanted = true;
                } else if let Some(rest) = line.strip_prefix("Protection from ") {
                    // "Protection from Vampires, from Werewolves, and from
                    // Zombies": one ability per quality (CR 702.16g).
                    for part in rest.split(", ") {
                        let t = part.trim_start_matches("and ").trim_start_matches("from ");
                        self.protection_from.push(singular(t));
                    }
                } else {
                    return None;
                }
            }
        }
        Some(())
    }
}

/// An additional cost to cast a spell.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Additional {
    SacrificeCreature,
    ExileCreatureCards(usize),
    /// "exile X cards from your graveyard": any number of any cards, X
    /// being how many.
    ExileXCards,
}

impl Additional {
    fn read(line: &str) -> Option<Additional> {
        let rest = line.strip_prefix("As an additional cost to cast this spell, ")?.strip_suffix('.')?;
        if rest == "sacrifice a creature" {
            return Some(Additional::SacrificeCreature);
        }
        if rest == "exile X cards from your graveyard" {
            return Some(Additional::ExileXCards);
        }
        let n = rest.strip_prefix("exile ")?
            .strip_suffix(" creature card from your graveyard")
            .or_else(|| rest.strip_prefix("exile ")?.strip_suffix(" creature cards from your graveyard"))?;
        Some(Additional::ExileCreatureCards(number(n)? as usize))
    }
}

/// "<cost>: <effect>" (CR 602.1).
#[derive(Debug, Clone, PartialEq)]
struct Activated {
    /// Its place among the card's abilities, mana abilities included, in
    /// the order the text prints them: the engine's `ability_index`.
    index: usize,
    mana: ManaCost,
    /// "{T}" (CR 107.5).
    tap: bool,
    /// "Sacrifice this creature" (CR 602.1a, 701.21).
    sacrifice_self: bool,
    /// "Activate only once each turn" (CR 602.5b).
    once_each_turn: bool,
    /// "Activate only as a sorcery", and equip (CR 602.5d, 702.6a).
    sorcery: bool,
    /// "Sacrifice a creature" or "Sacrifice another creature" in the cost
    /// (CR 602.2b, 601.2h).
    sacrifice: Option<Sacrifice>,
    /// "Exile a creature card from your graveyard" in the cost.
    exile_creature_card: bool,
    spec: Spec,
}

/// Which creature a cost may sacrifice.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Sacrifice {
    AnyCreature,
    /// Not the source (CR 109.5).
    AnotherCreature,
}

/// Read "<cost>: <effect>" when the cost is mana, {T} and sacrificing the
/// source, and the effect is sentences the grammar reads.
fn read_activated(line: &str, index: usize) -> Option<Activated> {
    let (cost, effect) = line.split_once(": ")?;
    let mut ability = Activated {
        index, mana: ManaCost::free(), tap: false, sacrifice_self: false, once_each_turn: false, sorcery: false,
        sacrifice: None, exile_creature_card: false, spec: Spec::default(),
    };
    for part in cost.split(", ") {
        match part {
            "{T}" => ability.tap = true,
            "Sacrifice this creature" | "Sacrifice this land" | "Sacrifice this artifact" | "Sacrifice this enchantment" => ability.sacrifice_self = true,
            "Sacrifice a creature" => ability.sacrifice = Some(Sacrifice::AnyCreature),
            "Sacrifice another creature" => ability.sacrifice = Some(Sacrifice::AnotherCreature),
            "Exile a creature card from your graveyard" => ability.exile_creature_card = true,
            _ if part.starts_with('{') => ability.mana = mana_cost(part)?,
            _ => return None,
        }
    }
    for sentence in effect.split_inclusive(". ") {
        if sentence.trim() == "Activate only once each turn." {
            ability.once_each_turn = true;
        } else if sentence.trim() == "Activate only as a sorcery." {
            ability.sorcery = true;
        } else {
            ability.spec.sentence(sentence)?;
        }
    }
    (!ability.spec.effects.is_empty()).then_some(ability)
}

/// A static ability that changes power, toughness or keywords (CR 613.4c,
/// 613.1f).
#[derive(Debug, Clone, PartialEq)]
enum Static {
    /// "Enchanted creature gets +P/+T [and has K]", "Equipped creature has
    /// K", "+X/+X, where X is the number of creature cards in your
    /// graveyard", and "As long as equipped creature is a Human, it gets
    /// +1/+1": what an Aura or Equipment does to the creature it is
    /// attached to (CR 303.4, 301.5).
    Enchanted { power: i32, toughness: i32, has: Vec<Keyword>, per: Option<Per>, if_subtype: Option<String> },
    /// "<creatures> get +P/+T [and have K]"; with `enchanted_player`, the
    /// creatures that player controls ("Creatures enchanted player controls
    /// get -1/-1"). "Other" leaves out the source (CR 109.5).
    Group { filter: Filter, enchanted_player: bool, power: i32, toughness: i32, has: Vec<Keyword> },
    /// "This creature gets +P/+T for each <permanent>": the change, as
    /// many times as there are of them when it applies (CR 611.3a).
    PerEach { filter: Filter, power: i32, toughness: i32 },
    /// "As long as you control a <subtype>, this creature has K" (CR
    /// 611.3a, 604.1).
    IfYouControl { subtype: String, has: Vec<Keyword> },
    /// What an Aura stops or makes a creature do: "Enchanted creature
    /// can't attack or block", "... attacks each combat if able",
    /// "Prevent all combat damage that would be dealt to and dealt by
    /// enchanted creature", "... doesn't untap during its controller's
    /// untap step"; with `group`, the creatures the enchanted player
    /// controls ("attack each combat if able"); with `unless_subtype`, only
    /// while it is not one ("Otherwise, it can't attack or block").
    Restricts { group: bool, what: Vec<Restriction>, unless_subtype: Option<String> },
}

/// A rule a static ability puts on a creature (CR 508.1c-d, 509.1b,
/// 615.1, 502.3).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Restriction {
    CantAttack,
    CantBlock,
    AttacksEachCombat,
    NoCombatDamage,
    DoesntUntap,
}

/// An Aura's line that restricts the creature it enchants, or the
/// creatures of the player it enchants, the full stop gone; with any
/// change to power and toughness it also makes.
fn restriction_static(body: &str) -> Option<Vec<Static>> {
    use Restriction::*;
    let restricts = |what: Vec<Restriction>| Static::Restricts { group: false, what, unless_subtype: None };
    let out = match body {
        "Enchanted creature can't attack or block" => vec![restricts(vec![CantAttack, CantBlock])],
        "Enchanted creature doesn't untap during its controller's untap step" => vec![restricts(vec![DoesntUntap])],
        "Prevent all combat damage that would be dealt to and dealt by enchanted creature" => vec![restricts(vec![NoCombatDamage])],
        "Creatures enchanted player controls attack each combat if able" =>
            vec![Static::Restricts { group: true, what: vec![AttacksEachCombat], unless_subtype: None }],
        _ => {
            let rest = body.strip_prefix("Enchanted creature ")?;
            if let Some(change) = rest.strip_suffix(" and attacks each combat if able") {
                vec![attached_static(change, None)?, restricts(vec![AttacksEachCombat])]
            } else {
                // "gets +2/+2 as long as it's a Human. Otherwise, it can't
                // attack or block."
                let (change, rest) = rest.split_once(" as long as it's a ")?;
                let (subtype, otherwise) = rest.split_once(". Otherwise, it ")?;
                if otherwise != "can't attack or block" {
                    return None;
                }
                vec![attached_static(change, Some(subtype))?,
                    Static::Restricts { group: false, what: vec![CantAttack, CantBlock], unless_subtype: Some(subtype.to_string()) }]
            }
        }
    };
    Some(out)
}

/// What an attached static's X counts, in its controller's graveyard.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Per {
    CreatureCards,
    InstantAndSorceryCards,
}

/// "gets +2/+2 and has flying", "has first strike and gets +X/+0, where X
/// is the number of instant and sorcery cards in your graveyard": what an
/// Aura or Equipment does to the creature it is attached to.
fn attached_static(text: &str, if_subtype: Option<&str>) -> Option<Static> {
    let (text, per) = if let Some(r) = text.strip_suffix(", where X is the number of creature cards in your graveyard") {
        (r.replace("+X", "+1"), Some(Per::CreatureCards))
    } else if let Some(r) = text.strip_suffix(", where X is the number of instant and sorcery cards in your graveyard") {
        (r.replace("+X", "+1"), Some(Per::InstantAndSorceryCards))
    } else {
        (text.to_string(), None)
    };
    // "has first strike and gets +X/+0": the same change, keywords first.
    let text = match text.strip_prefix("has ").and_then(|r| r.split_once(" and gets ")) {
        Some((k, c)) => format!("gets {c} and has {k}"),
        None => text,
    };
    let ((power, toughness), has) = static_change(&text)?;
    Some(Static::Enchanted { power, toughness, has, per, if_subtype: if_subtype.map(str::to_string) })
}

/// An enters-the-battlefield trigger.
#[derive(Debug, Clone, PartialEq, Default)]
struct Trigger {
    spec: Spec,
    /// "you may ...": its controller chooses at resolution (CR 603.5).
    optional: bool,
    /// "Morbid — When this creature enters, if a creature died this turn,
    /// ...": an intervening "if" (CR 603.4).
    if_morbid: bool,
}

/// "Whenever <creatures> die, ...": a leaves-the-battlefield trigger of a
/// permanent on other creatures' deaths, which looks back at the creature
/// as it last existed on the battlefield (CR 603.6c, 603.10a).
#[derive(Debug, Clone, PartialEq)]
struct Watch {
    /// Which deaths: "another creature", "a creature" and "this creature
    /// or another creature" (the source too), "another creature you
    /// control", "another Human creature".
    dying: Filter,
    trigger: Trigger,
}

/// "At the beginning of <whose> upkeep, <effect>" (CR 503.1a, 603.2).
#[derive(Debug, Clone, PartialEq)]
struct Upkeep {
    /// The enchanted player's upkeep, not its controller's.
    enchanted_player: bool,
    trigger: Trigger,
}

/// An upkeep trigger's effect, read as a sentence of its own.
fn read_upkeep(line: &str) -> Option<Upkeep> {
    let (enchanted_player, rest) = match line.strip_prefix("At the beginning of your upkeep, ") {
        Some(r) => (false, r),
        None => (true, line.strip_prefix("At the beginning of enchanted player's upkeep, ")?),
    };
    let mut chars = rest.chars();
    let sentence: String = chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default();
    let mut trigger = Trigger::default();
    trigger.spec.sentence(&sentence)?;
    Some(Upkeep { enchanted_player, trigger })
}

impl Spec {
    /// Whether the board after a death can read differently.
    fn reads_morbid(&self) -> bool {
        self.morbid.is_some()
            || self.enters_with_counters_if_morbid.is_some()
            || self.etb.as_ref().is_some_and(|t| t.if_morbid)
    }
}

fn number(word: &str) -> Option<u32> {
    let n = match word {
        "a" | "an" | "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        "eleven" => 11,
        "twelve" => 12,
        "thirteen" => 13,
        _ => return word.parse().ok(),
    };
    Some(n)
}

/// "+3/+3", "-4/-0".
fn pt_change(word: &str) -> Option<(i32, i32)> {
    let (p, t) = word.split_once('/')?;
    Some((p.parse().ok()?, t.parse().ok()?))
}

const CARD_TYPES: [&str; 5] = ["creature", "artifact", "enchantment", "land", "planeswalker"];

/// A description of permanents: "nonblack creature", "artifact or
/// enchantment", "non-Vampire, non-Werewolf, non-Zombie creature",
/// "creature with power 4 or greater", "creature you control".
fn filter(text: &str) -> Option<Filter> {
    let mut clauses = Vec::new();
    // "power 4 or greater" is one phrase, not two alternatives; "Vampire,
    // Werewolf, or Zombie" is three, where "non-Vampire, non-Werewolf
    // creature" is one with two qualifiers.
    let mut text = text.replace(" or greater", " or_greater");
    if text.contains(", or ") {
        text = text.replace(", or ", " or ").replace(", ", " or ");
    }
    for alt in text.split(" or ") {
        let mut clause = Clause::default();
        let mut rest = alt.trim();
        for (suffix, yours) in [(" you control", Some(true)), (" you don't control", Some(false)), (" your opponents control", Some(false))] {
            if let Some(r) = rest.strip_suffix(suffix) {
                rest = r;
                clause.yours = yours;
            }
        }
        if let Some((r, n)) = rest.split_once(" with power ") {
            clause.min_power = Some(n.strip_suffix(" or_greater")?.parse().ok()?);
            rest = r;
        }
        if let Some((r, k)) = rest.split_once(" without ") {
            clause.without.push(keyword_named(k)?);
            rest = r;
        }
        if let Some((r, k)) = rest.split_once(" with ") {
            clause.with.push(keyword_named(k)?);
            rest = r;
        }
        let words: Vec<&str> = rest.split_whitespace().map(|w| w.trim_end_matches(',')).collect();
        let (head, quals) = words.split_last()?;
        for q in quals {
            if *q == "another" || q.eq_ignore_ascii_case("other") {
                clause.other = true;
            } else if q.eq_ignore_ascii_case("attacking") {
                clause.attacking = true;
            } else if let Some(sub) = q.strip_prefix("non-") {
                clause.not_types.push(sub.to_string());
            } else if let Some(neg) = q.strip_prefix("non") {
                if let Some(c) = color_named(neg) {
                    clause.not_colors.push(c);
                } else if CARD_TYPES.contains(&neg) || neg == "legendary" {
                    clause.not_types.push(neg.to_string());
                } else {
                    return None;
                }
            } else if q.chars().next().is_some_and(char::is_uppercase) {
                clause.subtypes.push((*q).to_string());
            } else {
                return None;
            }
        }
        // A card type may open the sentence ("Creatures you control"), and
        // is plural for a group; a subtype keeps its capital.
        let lower = head.to_lowercase();
        let one = lower.strip_suffix('s').unwrap_or(&lower);
        let head = if CARD_TYPES.contains(&one) || one == "permanent" { one } else { head };
        clause.head = match head {
            // What kind of object it is, the slot says; the words are
            // about its characteristics.
            "permanent" | "spell" => None,
            h if CARD_TYPES.contains(&h) => Some(h.to_string()),
            // A subtype is capitalised in Oracle text, and plural for a
            // group ("Werewolves you control").
            h if h.chars().next().is_some_and(char::is_uppercase) => Some(singular(h)),
            _ => return None,
        };
        clauses.push(clause);
    }
    Some(Filter(clauses))
}

/// "gets +3/+3 until end of turn", "gets +2/+2 and gains lifelink until end
/// of turn": the change, and the keywords gained.
fn pump(text: &str) -> Option<((i32, i32), Vec<Keyword>)> {
    let body = text.strip_suffix(" until end of turn")?;
    let (change, gains) = match body.split_once(" and ") {
        Some((c, g)) => (c, Some(g)),
        None => (body, None),
    };
    let change = pt_change(change.split_whitespace().nth(1)?)?;
    let mut keywords = Vec::new();
    if let Some(g) = gains {
        let g = g.strip_prefix("gains ").or_else(|| g.strip_prefix("gain "))?;
        for k in g.split(" and ") {
            keywords.push(keyword_named(k)?);
        }
    }
    Some((change, keywords))
}

impl Spec {
    fn slot(&mut self, slot: Slot) -> usize {
        self.slots.push(slot);
        self.slots.len() - 1
    }

    /// Read one sentence, `name` already replaced by `~`. None if no
    /// template matches.
    fn sentence(&mut self, s: &str) -> Option<()> {
        // "another target creature" is a target that is another creature.
        let s = s.replace("another target ", "target another ");
        let s = s.trim().strip_suffix('.')?;
        if let Some(to) = s.strip_prefix("~ deals damage equal to the exiled card's power to ") {
            let f = filter(to.strip_prefix("target ")?)?;
            let slot = self.slot(Slot::Permanent(f));
            self.effects.push(Effect::DamageEqualToExiledPower { slot });
            return Some(());
        }
        // The ability's source ("This creature gets +2/+0 until end of
        // turn", "Return this creature to its owner's hand").
        if s == "Tap enchanted creature" {
            let slot = self.slot(Slot::EnchantedCreature);
            self.effects.push(Effect::Tap { slot, tapped: true });
            return Some(());
        }
        if let Some(rest) = s.strip_prefix("This Aura deals ") {
            let (n, to) = rest.split_once(" damage to ")?;
            if to != "that player or a planeswalker that player controls" {
                return None;
            }
            let slot = self.slot(Slot::EnchantedPlayer { or_planeswalker: true });
            self.effects.push(Effect::Damage { amount: number(n)?, slot });
            return Some(());
        }
        if let Some(rest) = s.strip_prefix("That player ") {
            let slot = self.slot(Slot::EnchantedPlayer { or_planeswalker: false });
            if let Some(n) = rest.strip_prefix("mills ") {
                let n = number(n.strip_suffix(" cards").or_else(|| n.strip_suffix(" card"))?)? as usize;
                self.effects.push(Effect::Mill { slot, n });
                return Some(());
            }
            let n = rest.strip_prefix("exiles ")?.strip_suffix(" cards from their graveyard")?;
            self.effects.push(Effect::ExileFromGraveyard { slot, n: number(n)? as usize });
            return Some(());
        }
        for this in ["This creature ", "This land ", "This artifact "] {
            if let Some(rest) = s.strip_prefix(this) {
                if let Some(rest) = rest.strip_prefix("deals ") {
                    return self.sentence(&format!("~ deals {rest}."));
                }
                let ((power, toughness), gains) = match rest.strip_prefix("gains ") {
                    Some(k) => ((0, 0), k.strip_suffix(" until end of turn")?.split(" and ").map(keyword_named).collect::<Option<Vec<_>>>()?),
                    None => pump(rest)?,
                };
                let slot = self.slot(Slot::This);
                self.effects.push(Effect::Pump { slot, power, toughness, gains });
                return Some(());
            }
        }
        if s == "Search your library for a basic land card, reveal it, put it into your hand, then shuffle" {
            self.effects.push(Effect::SearchForBasicLand { of: None, to_battlefield: false, morbid_battlefield: false });
            return Some(());
        }
        if s == "Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle" {
            let of = self.slots.len().checked_sub(1)?;
            self.effects.push(Effect::SearchForBasicLand { of: Some(of), to_battlefield: true, morbid_battlefield: false });
            return Some(());
        }
        if let Some((first, then)) = s.split_once(", then ") {
            self.sentence(&format!("{first}."))?;
            let mut chars = then.chars();
            let then: String = chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default();
            return self.sentence(&format!("{then}."));
        }
        if let Some(n) = s.strip_prefix("Discard ").and_then(|r| r.strip_suffix(" at random")) {
            let n = number(n.strip_suffix(" cards").or_else(|| n.strip_suffix(" card"))?)? as usize;
            self.effects.push(Effect::DiscardAtRandom { n });
            return Some(());
        }
        if s == "Exile it" {
            self.effects.push(Effect::ExileItFromGraveyard);
            return Some(());
        }
        if let Some(rest) = s.strip_prefix("Return ").filter(|r| r.contains(" at random from your graveyard to ")) {
            let (what, to) = rest.split_once(" at random from your graveyard to ")?;
            let to_battlefield = match to {
                "your hand" => false,
                "the battlefield" => true,
                _ => return None,
            };
            let (n, kind) = what.split_once(' ')?;
            let n = number(n)? as usize;
            let kind = kind.strip_suffix(" cards").or_else(|| kind.strip_suffix(" card")).or_else(|| (kind == "cards" || kind == "card").then_some(""))?;
            let filter = if kind.is_empty() { None } else { Some(filter(kind)?) };
            self.effects.push(Effect::ReturnAtRandom { n, filter, to_battlefield });
            return Some(());
        }
        if let Some(rest) = s.strip_prefix("If that creature is a ") {
            let (subtype, n) = rest.split_once(", put ")?;
            let n_if = number(n.strip_suffix(" +1/+1 counters on it instead")?)?;
            let Some(Effect::PlusCounters { slot, n }) = self.effects.pop() else { return None };
            self.effects.push(Effect::PlusCountersIfSubtype { slot, n, subtype: subtype.to_string(), n_if });
            return Some(());
        }
        if let Some(f) = s.strip_prefix("Put a +1/+1 counter on target ") {
            let slot = self.slot(Slot::Permanent(filter(f)?));
            self.effects.push(Effect::PlusCounters { slot, n: 1 });
            return Some(());
        }
        if let Some(mana) = s.strip_prefix("Add ") {
            let mut out: Vec<(ManaType, u32)> = Vec::new();
            for sym in mana.split('}').filter(|x| !x.is_empty()) {
                let t = mana_named(&format!("{sym}}}"))?;
                match out.iter_mut().find(|(k, _)| *k == t) {
                    Some((_, n)) => *n += 1,
                    None => out.push((t, 1)),
                }
            }
            self.effects.push(Effect::AddMana(out));
            return Some(());
        }
        if let Some(f) = s.strip_prefix("Exile target ").and_then(|r| r.strip_suffix(" and all other creatures with the same name as that creature")) {
            let slot = self.slot(Slot::Permanent(filter(f)?));
            self.effects.push(Effect::ExileWithSameName { slot });
            return Some(());
        }
        if let Some(n) = s.strip_prefix("If it's a creature card, you gain ").and_then(|r| r.strip_suffix(" life")) {
            if !matches!(self.effects.last(), Some(Effect::PlayerExilesFromGraveyard { .. })) {
                return None;
            }
            self.effects.push(Effect::GainLifeIfExiledCreature { n: number(n)?.try_into().ok()? });
            return Some(());
        }
        if let Some(rest) = s.strip_prefix("If it's a creature card, you create ") {
            if !matches!(self.effects.last(), Some(Effect::BottomCardToGraveyard { .. })) {
                return None;
            }
            let mut token = Spec::default();
            token.tokens(rest)?;
            self.effects.push(Effect::TokenIfCreatureCard(Box::new(token.effects.pop()?)));
            return Some(());
        }
        if s == "Exchange your life total with this creature's toughness" {
            let slot = self.slot(Slot::This);
            self.effects.push(Effect::ExchangeLifeWithToughness { slot });
            return Some(());
        }
        if s == "Each player chooses a creature they control" {
            self.effects.push(Effect::EachPlayerChoosesACreature);
            return Some(());
        }
        if s == "Destroy the rest" && matches!(self.effects.last(), Some(Effect::EachPlayerChoosesACreature)) {
            self.effects.push(Effect::DestroyTheRest);
            return Some(());
        }
        if s == "You gain life equal to that creature's toughness" && matches!(self.effects.last(), Some(Effect::SacrificeOfTheirChoice { .. })) {
            self.effects.push(Effect::GainLifeEqualToSacrificedToughness);
            return Some(());
        }
        if s == "Regenerate this creature" {
            let slot = self.slot(Slot::This);
            self.effects.push(Effect::Regenerate { slot });
            return Some(());
        }
        if s == "Transform this creature" {
            let slot = self.slot(Slot::This);
            self.effects.push(Effect::TransformThis { slot });
            return Some(());
        }
        if let Some(f) = s.strip_prefix("Regenerate all ") {
            self.effects.push(Effect::RegenerateAll { filter: filter(f)? });
            return Some(());
        }
        if s == "You gain life equal to the sacrificed creature's toughness" {
            self.effects.push(Effect::GainLifeEqualToSacrificedToughness);
            return Some(());
        }
        if let Some(rest) = s.strip_prefix("Gain control of target ") {
            let (f, until_end_of_turn) = match rest.strip_suffix(" until end of turn") {
                Some(f) => (f, true),
                None => (rest.strip_suffix(" for as long as you control ~")?, false),
            };
            let slot = self.slot(Slot::Permanent(filter(f)?));
            self.effects.push(Effect::GainControl { slot, until_end_of_turn });
            return Some(());
        }
        if s == "Untap it" {
            let slot = self.slots.len().checked_sub(1)?;
            self.effects.push(Effect::Tap { slot, tapped: false });
            return Some(());
        }
        if let Some(gains) = s.strip_prefix("It gains ").and_then(|r| r.strip_suffix(" until end of turn")) {
            let slot = self.slots.len().checked_sub(1)?;
            let gains = gains.split(" and ").map(keyword_named).collect::<Option<Vec<_>>>()?;
            self.effects.push(Effect::Pump { slot, power: 0, toughness: 0, gains });
            return Some(());
        }
        if let Some(t) = s.strip_prefix("That creature becomes a ").and_then(|r| r.strip_suffix(" in addition to its other types")) {
            let slot = self.slots.len().checked_sub(1)?;
            self.effects.push(Effect::BecomesSubtype { slot, subtype: t.to_string() });
            return Some(());
        }
        if s == "Put a +1/+1 counter on this creature" || s == "Put a +1/+1 counter on ~" {
            let slot = self.slot(Slot::This);
            self.effects.push(Effect::PlusCounters { slot, n: 1 });
            return Some(());
        }
        // "Target player loses 1 life and you gain 1 life": two effects,
        // in that order (CR 608.2c).
        if let Some((loses, gain)) = s.split_once(" and you gain ") {
            let n: i32 = number(gain.strip_suffix(" life")?)?.try_into().ok()?;
            self.sentence(&format!("{loses}."))?;
            if !matches!(self.effects.last(), Some(Effect::LoseLife { .. })) {
                return None;
            }
            self.effects.push(Effect::GainLife { n });
            return Some(());
        }
        if s == "Return this creature to its owner's hand" {
            let slot = self.slot(Slot::This);
            self.effects.push(Effect::ToHand { slot });
            return Some(());
        }
        if let Some(n) = s.strip_prefix("Each player mills ") {
            let n = number(n.strip_suffix(" cards").or_else(|| n.strip_suffix(" card"))?)? as usize;
            self.effects.push(Effect::MillEach { n });
            return Some(());
        }
        if let Some(f) = s.strip_prefix("Put a +1/+1 counter on each ").filter(|f| !f.starts_with("of up to ")) {
            self.effects.push(Effect::PlusCountersEach { filter: filter(f)? });
            return Some(());
        }
        // CR 701.12a: "Target creature you control fights target creature
        // you don't control".
        if let Some((a, b)) = s.split_once(" fights ") {
            let a = self.slot(Slot::Permanent(filter(a.strip_prefix("Target ")?)?));
            let b = self.slot(Slot::Permanent(filter(b.strip_prefix("target ")?)?));
            self.effects.push(Effect::Fight { a, b });
            return Some(());
        }
        // "up to N target ..." (CR 601.2c).
        if let Some(rest) = s.strip_prefix("Tap up to ") {
            let slot = self.up_to(rest)?;
            self.effects.push(Effect::Tap { slot, tapped: true });
            return Some(());
        }
        if let Some(rest) = s.strip_prefix("Put a +1/+1 counter on each of up to ") {
            let slot = self.up_to(rest)?;
            self.effects.push(Effect::PlusCounters { slot, n: 1 });
            return Some(());
        }
        if let Some(f) = s.strip_prefix("Counter target ") {
            let slot = self.slot(Slot::Spell(filter(f)?));
            self.effects.push(Effect::Counter { slot, exile: false });
            return Some(());
        }
        if s == "If that spell is countered this way, exile it instead of putting it into its owner's graveyard" {
            let Some(Effect::Counter { exile, .. }) = self.effects.last_mut() else { return None };
            *exile = true;
            return Some(());
        }
        if let Some(n) = s.strip_prefix("Mill ") {
            let n = number(n.strip_suffix(" cards").or_else(|| n.strip_suffix(" card"))?)? as usize;
            self.effects.push(Effect::MillYou { n });
            return Some(());
        }
        if let Some(f) = s.strip_prefix("Untap all ") {
            self.effects.push(Effect::UntapAll { filter: filter(f)? });
            return Some(());
        }
        if let Some(f) = s.strip_prefix("Destroy all ") {
            self.effects.push(Effect::DestroyAll { filter: filter(f)? });
            return Some(());
        }
        // Cards in graveyards (CR 404), before the permanent templates
        // below would take "target card" for a permanent.
        if s == "Exile target card from a graveyard" {
            let slot = self.slot(Slot::GraveyardCard { filter: None, yours: false });
            self.effects.push(Effect::Exile { slot });
            return Some(());
        }
        if let Some(rest) = s.strip_prefix("Return target ") {
            if let Some(f) = rest.strip_suffix(" card from your graveyard to your hand") {
                let slot = self.slot(Slot::GraveyardCard { filter: Some(filter(f)?), yours: true });
                self.effects.push(Effect::ToHand { slot });
                return Some(());
            }
            if let Some((f, how)) = rest.split_once(" card from your graveyard to the battlefield") {
                let minus_counter = match how {
                    "" => false,
                    " with a -1/-1 counter on it" => true,
                    _ => return None,
                };
                let slot = self.slot(Slot::GraveyardCard { filter: Some(filter(f)?), yours: true });
                self.effects.push(Effect::Reanimate { slot, minus_counter });
                return Some(());
            }
        }
        if let Some(rest) = s.strip_prefix("You gain ") {
            if let Some((n, per)) = rest.split_once(" life for each ") {
                let n: i32 = number(n)?.try_into().ok()?;
                if per == "creature card in your graveyard" {
                    self.effects.push(Effect::GainLifePerCreatureCard { n });
                    return Some(());
                }
                // "for each enchantment destroyed this way": the last
                // "Destroy all" (CR 608.2c reads "this way" as that one).
                per.strip_suffix(" destroyed this way")?;
                if !matches!(self.effects.last(), Some(Effect::DestroyAll { .. })) {
                    return None;
                }
                self.effects.push(Effect::GainLifePerDestroyed { n });
                return Some(());
            }
        }
        // CR 120.3: "~ deals N damage to ...", and "it" for a permanent's
        // own trigger ("it deals 3 damage", "you may have it deal 4").
        if let Some(to) = s.strip_prefix("~ deals X damage to ") {
            let slot = match to {
                "any target" => self.slot(Slot::Any),
                _ => self.slot(Slot::Permanent(filter(to.strip_prefix("target ")?)?)),
            };
            self.effects.push(Effect::DamageX { slot });
            return Some(());
        }
        if let Some(rest) = s.strip_prefix("~ deals ").or_else(|| s.strip_prefix("It deals ")).or_else(|| s.strip_prefix("Have it deal ")) {
            let (n, to) = rest.split_once(" damage to ")?;
            let amount = number(n)?;
            let effect = match to {
                "any target" => Effect::Damage { amount, slot: self.slot(Slot::Any) },
                "target player or planeswalker" => Effect::Damage { amount, slot: self.slot(Slot::PlayerOrPlaneswalker) },
                _ => {
                    if let Some(f) = to.strip_prefix("target ") {
                        let f = filter(f)?;
                        Effect::Damage { amount, slot: self.slot(Slot::Permanent(f)) }
                    } else {
                        Effect::DamageEach { amount, filter: filter(to.strip_prefix("each ")?)? }
                    }
                }
            };
            self.effects.push(effect);
            return Some(());
        }
        // A pump of one target or of a group (CR 611.2).
        if let Some(rest) = s.strip_prefix("Target ") {
            if let Some((who, change)) = rest.split_once(" gets ").filter(|(_, c)| c.contains(", where X is the number of ")) {
                let (change, count) = change.split_once(", where X is the number of ")?;
                let ((power, toughness), gains) = pump(&format!("gets {}", change.replace("+X", "+1")))?;
                if !gains.is_empty() {
                    return None;
                }
                let slot = self.slot(Slot::Permanent(filter(who)?));
                self.effects.push(Effect::PumpPerCount { slot, power, toughness, filter: filter(count)? });
                return Some(());
            }
            if let Some((who, change)) = rest.split_once(" gets ").filter(|(_, c)| c.starts_with("+X/") && !c.contains(", where X is ")) {
                let ((power, toughness), gains) = pump(&format!("gets {}", change.replacen("+X/", "+1/", 1).replacen("/+X", "/+1", 1)))?;
                let slot = self.slot(Slot::Permanent(filter(who)?));
                self.effects.push(Effect::PumpX { slot, power, toughness, gains });
                return Some(());
            }
            if let Some((who, change)) = rest.split_once(" gets ") {
                let ((power, toughness), gains) = pump(&format!("gets {change}"))?;
                let slot = self.slot(Slot::Permanent(filter(who)?));
                self.effects.push(Effect::Pump { slot, power, toughness, gains });
                return Some(());
            }
            if let Some(who) = rest.strip_suffix(" exiles a card from their graveyard") {
                let slot = self.player_slot(who)?;
                self.effects.push(Effect::PlayerExilesFromGraveyard { slot });
                return Some(());
            }
            if let Some(who) = rest.strip_suffix(" puts the bottom card of their library into their graveyard") {
                let slot = self.player_slot(who)?;
                self.effects.push(Effect::BottomCardToGraveyard { slot });
                return Some(());
            }
            if let Some((who, n)) = rest.split_once(" discards ") {
                let n = number(n.strip_suffix(" cards").or_else(|| n.strip_suffix(" card"))?)? as usize;
                let slot = self.player_slot(who)?;
                self.effects.push(Effect::PlayerDiscards { slot, n });
                return Some(());
            }
            if let Some(who) = rest.strip_suffix(" sacrifices a creature of their choice") {
                let slot = self.player_slot(who)?;
                self.effects.push(Effect::SacrificeOfTheirChoice { slot });
                return Some(());
            }
            if let Some((who, rest)) = rest.split_once(" draws ") {
                // "Target player draws a card and loses 1 life".
                let (draws, loses) = match rest.split_once(" and loses ") {
                    Some((d, l)) => (d, Some(l)),
                    None => (rest, None),
                };
                let n = number(draws.strip_suffix(" cards").or_else(|| draws.strip_suffix(" card"))?)? as usize;
                let slot = self.player_slot(who)?;
                self.effects.push(Effect::PlayerDraws { slot, n });
                if let Some(l) = loses {
                    let n: i32 = number(l.strip_suffix(" life")?)?.try_into().ok()?;
                    self.effects.push(Effect::LoseLife { slot, n });
                }
                return Some(());
            }
            if let Some((who, n)) = rest.split_once(" loses ") {
                let n: i32 = number(n.strip_suffix(" life")?)?.try_into().ok()?;
                let slot = self.player_slot(who)?;
                self.effects.push(Effect::LoseLife { slot, n });
                return Some(());
            }
            if let Some((who, n)) = rest.split_once(" mills ") {
                let n = number(n.strip_suffix(" cards").or_else(|| n.strip_suffix(" card"))?)? as usize;
                let slot = self.player_slot(who)?;
                self.effects.push(Effect::Mill { slot, n });
                return Some(());
            }
            return None;
        }
        if let Some((who, change)) = s.split_once(" get ") {
            let ((power, toughness), gains) = pump(&format!("get {change}"))?;
            self.effects.push(Effect::PumpEach { filter: filter(who)?, power, toughness, gains });
            return Some(());
        }
        if let Some(rest) = s.strip_prefix("It gets ") {
            let ((power, toughness), gains) = pump(&format!("gets {rest}"))?;
            let slot = self.slots.len().checked_sub(1)?;
            self.effects.push(Effect::Pump { slot, power, toughness, gains });
            return Some(());
        }
        // Moving or changing one target permanent.
        for (verb, make) in [
            ("Destroy target ", (|slot| Effect::Destroy { slot }) as fn(usize) -> Effect),
            ("Exile target ", |slot| Effect::Exile { slot }),
            ("Tap target ", |slot| Effect::Tap { slot, tapped: true }),
            ("Untap target ", |slot| Effect::Tap { slot, tapped: false }),
        ] {
            if let Some(f) = s.strip_prefix(verb) {
                let slot = self.slot(Slot::Permanent(filter(f)?));
                self.effects.push(make(slot));
                return Some(());
            }
        }
        if let Some(f) = s.strip_prefix("Return target ").and_then(|r| r.strip_suffix(" to its owner's hand")) {
            let slot = self.slot(Slot::Permanent(filter(f)?));
            self.effects.push(Effect::ToHand { slot });
            return Some(());
        }
        if let Some(f) = s.strip_prefix("Put target ").and_then(|r| r.strip_suffix(" on top of its owner's library")) {
            let slot = self.slot(Slot::Permanent(filter(f)?));
            self.effects.push(Effect::ToLibraryTop { slot });
            return Some(());
        }
        if s == "Its controller gains life equal to its power" {
            let slot = self.slots.len().checked_sub(1)?;
            self.effects.push(Effect::ControllerGainsLifeEqualToPower { slot });
            return Some(());
        }
        if let Some(n) = s.strip_prefix("Draw ") {
            let n = number(n.strip_suffix(" cards").or_else(|| n.strip_suffix(" card"))?)? as usize;
            self.effects.push(Effect::Draw { n });
            return Some(());
        }
        if let Some(n) = s.strip_prefix("You gain ").and_then(|r| r.strip_suffix(" life")) {
            self.effects.push(Effect::GainLife { n: number(n)?.try_into().ok()? });
            return Some(());
        }
        if let Some(rest) = s.strip_prefix("Create ") {
            return self.tokens(rest);
        }
        None
    }

    /// "two target creatures": an "up to" slot of that many.
    fn up_to(&mut self, rest: &str) -> Option<usize> {
        let (n, f) = rest.split_once(" target ")?;
        let n = number(n)? as usize;
        Some(self.slot(Slot::UpTo { n, filter: filter(f)? }))
    }

    fn player_slot(&mut self, who: &str) -> Option<usize> {
        match who {
            "player" => Some(self.slot(Slot::Player)),
            "opponent" => Some(self.slot(Slot::Opponent)),
            _ => None,
        }
    }

    /// "two 1/1 white Spirit creature tokens with flying", "thirteen tapped
    /// 2/2 black Zombie creature tokens", "a 1/1 ... token".
    fn tokens(&mut self, rest: &str) -> Option<()> {
        let (rest, per_creature_card) = match rest.strip_suffix(" for each creature card in your graveyard") {
            Some(r) => (r, true),
            None => (rest, false),
        };
        let (rest, keywords) = match rest.split_once(" with ") {
            Some((r, k)) => (r, k.split(" and ").map(keyword_named).collect::<Option<Vec<_>>>()?),
            None => (rest, Vec::new()),
        };
        let rest = rest.strip_suffix(" creature tokens").or_else(|| rest.strip_suffix(" creature token"))?;
        let mut words = rest.split_whitespace();
        let n = number(words.next()?)? as usize;
        let mut next = words.next()?;
        let tapped = next == "tapped";
        if tapped {
            next = words.next()?;
        }
        let (power, toughness) = {
            let (p, t) = next.split_once('/')?;
            (p.parse().ok()?, t.parse().ok()?)
        };
        let mut colors = Vec::new();
        let mut subtypes = Vec::new();
        for w in words {
            if let Some(c) = color_named(w) {
                colors.push(c);
            } else if w != "and" && w.chars().next().is_some_and(char::is_uppercase) {
                subtypes.push(w.to_string());
            } else if w != "and" {
                return None;
            }
        }
        let mut keywords = keywords;
        keywords.sort_by_key(|k| k.label());
        self.effects.push(Effect::Tokens { n, power, toughness, colors, subtypes, keywords, tapped, per_creature_card });
        Some(())
    }
}

/// Read a card's whole text, or say which sentence the grammar cannot read.
fn read(card: &Printed) -> Result<Spec, String> {
    let text = without_reminders(&card.oracle_text).replace(&card.name, "~");
    let mut spec = Spec::default();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        // Flashback is a second way to cast the card (CR 702.34a); the cast
        // from hand is what is checked here, and `flashback.rs` reaches the
        // other one.
        if line.starts_with("Flashback {") {
            continue;
        }
        if line == "Morbid — You may put that card onto the battlefield instead of putting it into your hand if a creature died this turn." {
            let Some(Effect::SearchForBasicLand { morbid_battlefield, .. }) = spec.effects.last_mut() else {
                return Err(format!("cannot read {line:?}"));
            };
            *morbid_battlefield = true;
            continue;
        }
        if let Some(cost) = Additional::read(line) {
            spec.additional = Some(cost);
            continue;
        }
        // An alternative cost (CR 118.9); the cast checked here pays the
        // mana cost.
        if line == "If it's not your turn, you may exile a blue card from your hand rather than pay this spell's mana cost." {
            continue;
        }
        if line == "This spell costs {1} less to cast for each creature on the battlefield." {
            spec.cheaper_per_creature = true;
            continue;
        }
        if let Some(rest) = line.strip_prefix("Morbid — ~ deals ") {
            let n = rest.strip_suffix(" damage instead if a creature died this turn.")
                .and_then(number)
                .ok_or_else(|| format!("cannot read {line:?}"))?;
            let which = spec.effects.iter().rposition(|e| matches!(e, Effect::Damage { .. }))
                .ok_or("morbid with no damage to replace")?;
            spec.morbid = Some((which, n));
            continue;
        }
        for sentence in line.split_inclusive(". ") {
            spec.sentence(sentence).ok_or_else(|| format!("cannot read {sentence:?}"))?;
        }
    }
    if spec.effects.is_empty() {
        return Err("no effects".to_string());
    }
    Ok(spec)
}

/// Read a creature card: lines of keywords the engine models ("Flying,
/// vigilance"), and the few other shapes the grammar knows — entering
/// tapped or with counters, power and toughness defined by a count, and an
/// enters-the-battlefield trigger whose effect is one of the spell
/// templates above.
fn read_creature(card: &Printed) -> Result<Spec, String> {
    let text = without_reminders(&card.oracle_text).replace(&card.name, "~");
    let mut spec = Spec { permanent: true, ..Spec::default() };
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let unread = || format!("cannot read {line:?}");
        if line == "This creature enters tapped." {
            spec.enters_tapped = true;
            continue;
        }
        if let Some(n) = line.strip_prefix("Morbid — This creature enters with ")
            .and_then(|r| r.strip_suffix(" +1/+1 counters on it if a creature died this turn."))
        {
            spec.enters_with_counters_if_morbid = Some(number(n).ok_or_else(unread)?);
            continue;
        }
        if line == "~'s power and toughness are each equal to the number of creatures you control." {
            spec.pt_is_creatures_you_control = true;
            continue;
        }
        if line == "~'s power and toughness are each equal to the number of creature cards in your graveyard." {
            spec.pt_is_creature_cards = true;
            continue;
        }
        if line == "~'s power and toughness are each equal to the number of cards in your hand." {
            spec.pt_is_cards_in_hand = true;
            continue;
        }
        if spec.combat.read(line).is_some() {
            continue;
        }
        if let Some(cost) = Additional::read(line) {
            spec.additional = Some(cost);
            continue;
        }
        if line.starts_with("At the beginning of your upkeep, ") {
            spec.upkeep = Some(Box::new(read_upkeep(line).ok_or_else(unread)?));
            continue;
        }
        if let Some((who, rest)) = line.strip_prefix("Whenever ").and_then(|r| r.split_once(" dies, ")) {
            let who = match who {
                "this creature or another creature" => "creature",
                w => w.strip_prefix("a ").unwrap_or(w),
            };
            let (optional, rest) = match rest.strip_prefix("you may ") {
                Some(r) => (true, r),
                None => (false, rest),
            };
            let mut chars = rest.chars();
            let sentence: String = chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default();
            let mut trigger = Trigger { optional, ..Trigger::default() };
            trigger.spec.sentence(&sentence).ok_or_else(unread)?;
            spec.watch = Some(Box::new(Watch { dying: filter(who).ok_or_else(unread)?, trigger }));
            continue;
        }
        if let Some(rest) = line.strip_prefix("When this creature dies, ") {
            let mut chars = rest.chars();
            let text: String = chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default();
            let mut trigger = Trigger::default();
            for sentence in text.split_inclusive(". ") {
                trigger.spec.sentence(sentence).ok_or_else(unread)?;
            }
            spec.dies = Some(Box::new(trigger));
            continue;
        }
        let (if_morbid, rest) = match line.strip_prefix("Morbid — When this creature enters, if a creature died this turn, ") {
            Some(r) => (true, Some(r)),
            None => (false, line.strip_prefix("When this creature enters, ")),
        };
        if let Some(rest) = rest {
            let (optional, rest) = match rest.strip_prefix("you may ") {
                Some(r) => (true, r),
                None => (false, rest),
            };
            // The effect is a sentence of its own, as a spell's would be.
            let mut chars = rest.chars();
            let sentence: String = chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default();
            let mut trigger = Trigger { optional, if_morbid, ..Trigger::default() };
            trigger.spec.sentence(&sentence).ok_or_else(unread)?;
            if spec.etb.is_some() {
                return Err(unread());
            }
            spec.etb = Some(Box::new(trigger));
            continue;
        }
        if line == "At the beginning of each upkeep, if no spells were cast last turn, transform this creature." {
            spec.werewolf = true;
            continue;
        }
        if let Some(st) = creature_static(line) {
            spec.statics.push(st);
            continue;
        }
        let index = spec.mana.iter().map(ManaAbility::indices).sum::<usize>() + spec.activated.len();
        if let Some(ability) = ManaAbility::read(line) {
            spec.mana.push(ability);
            continue;
        }
        if line.contains(": ") {
            spec.activated.push(read_activated(line, index).ok_or_else(unread)?);
            continue;
        }
        for word in line.split(',').map(str::trim) {
            // "Defender, protection from Zombies": protection is a keyword
            // ability with a quality, not one of the engine's keywords.
            if let Some(t) = word.strip_prefix("protection from ") {
                spec.combat.protection_from.push(singular(t));
                continue;
            }
            let k = keyword_named(word).ok_or_else(unread)?;
            spec.flash |= k == Keyword::Flash;
        }
    }
    // A double-faced card is read with its back face, which must read too:
    // a werewolf's turning back as its front turned, any other's being
    // what its front transforms into.
    if let Some(back) = &card.back {
        if back_face_reads(back).is_none_or(|b| b.werewolf != spec.werewolf) {
            return Err(format!("cannot read the back face {:?}", back.oracle_text));
        }
    }
    Ok(spec)
}

/// A double-faced card's back face, when its text is keywords, combat
/// text and static abilities, and for a werewolf's the trigger that
/// transforms it back, which sets `werewolf`: what the grammar reads of it
/// (its keywords are its printed ones).
fn back_face_reads(back: &Printed) -> Option<Spec> {
    let mut spec = Spec { permanent: true, ..Spec::default() };
    for line in without_reminders(&back.oracle_text).replace(&back.name, "~").lines().map(str::trim).filter(|l| !l.is_empty()) {
        if line == "At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature." {
            spec.werewolf = true;
        } else if line.split(',').map(str::trim).all(|w| keyword_named(w).is_some()) || spec.combat.read(line).is_some() {
        } else if let Some(st) = creature_static(line) {
            spec.statics.push(st);
        } else {
            return None;
        }
    }
    Some(spec)
}

/// Read an artifact that is nothing but activated abilities the grammar
/// reads.
fn read_artifact(card: &Printed) -> Result<Spec, String> {
    let text = without_reminders(&card.oracle_text).replace(&card.name, "~");
    let mut spec = Spec { permanent: true, ..Spec::default() };
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let index = spec.mana.iter().map(ManaAbility::indices).sum::<usize>() + spec.activated.len();
        let body = line.strip_suffix('.').unwrap_or(line);
        if line.starts_with("Whenever equipped creature ") {
            spec.combat.read(line).ok_or_else(|| format!("cannot read {line:?}"))?;
        } else if let Some(rest) = body.strip_prefix("Equipped creature ") {
            spec.statics.push(attached_static(rest, None).ok_or_else(|| format!("cannot read {line:?}"))?);
        } else if let Some((subtype, rest)) = body.strip_prefix("As long as equipped creature is a ").and_then(|r| r.split_once(", it ")) {
            let rest = rest.replace("gets an additional ", "gets ");
            spec.statics.push(attached_static(&rest, Some(subtype)).ok_or_else(|| format!("cannot read {line:?}"))?);
        } else if let Some((cost, sacrifice)) = line.strip_prefix("Equip ").and_then(mana_cost).map(|c| (c, None))
            .or_else(|| (line == "Equip—Sacrifice a creature.").then(|| (ManaCost::free(), Some(Sacrifice::AnyCreature))))
        {
            // CR 702.6a: "[Cost]: Attach this permanent to target creature
            // you control. Activate only as a sorcery." The engine numbers
            // it first among the Equipment's abilities.
            let slots = vec![Slot::Permanent(filter("creature you control").expect("a filter"))];
            let equip = Spec { slots, effects: vec![Effect::Attach { slot: 0 }], ..Spec::default() };
            spec.activated.insert(0, Activated {
                index: 0, mana: cost, tap: false, sacrifice_self: false, once_each_turn: false, sorcery: true,
                sacrifice, exile_creature_card: false, spec: equip,
            });
            for (i, a) in spec.activated.iter_mut().enumerate().skip(1) {
                a.index = i;
            }
        } else if let Some(ability) = ManaAbility::read(line) {
            spec.mana.push(ability);
        } else {
            spec.activated.push(read_activated(line, index).ok_or_else(|| format!("cannot read {line:?}"))?);
        }
    }
    if spec.activated.is_empty() && spec.mana.is_empty() {
        return Err("no abilities".to_string());
    }
    if !spec.statics.is_empty() && !card.type_line.contains("Equipment") {
        return Err("an attached static off an Equipment".to_string());
    }
    Ok(spec)
}

/// "gets +2/+2 and has flying", "has intimidate", "get +1/+1 and have
/// vigilance": a static change, with no duration.
fn static_change(text: &str) -> Option<((i32, i32), Vec<Keyword>)> {
    let (change, has) = if let Some(k) = text.strip_prefix("has ").or_else(|| text.strip_prefix("have ")) {
        ("+0/+0", Some(k))
    } else {
        let rest = text.strip_prefix("gets ").or_else(|| text.strip_prefix("get "))?;
        match rest.split_once(" and ") {
            Some((c, h)) => (c, Some(h.strip_prefix("has ").or_else(|| h.strip_prefix("have "))?)),
            None => (rest, None),
        }
    };
    let keywords = match has {
        Some(h) => h.split(" and ").map(keyword_named).collect::<Option<Vec<_>>>()?,
        None => Vec::new(),
    };
    Some((pt_change(change)?, keywords))
}

/// "Creatures you control get +1/+1", "Other Spirit creatures you control
/// get +1/+0", "Creature tokens you control have vigilance": a static that
/// changes a group, the full stop gone.
fn group_static(body: &str) -> Option<Static> {
    let (who, rest) = ["get ", "have "].iter()
        .find_map(|verb| body.split_once(&format!(" {verb}")).map(|(who, rest)| (who, format!("{verb}{rest}"))))?;
    let ((power, toughness), has) = static_change(&rest)?;
    let filter = if who == "Creature tokens you control" {
        Filter(vec![Clause { head: Some("creature".to_string()), yours: Some(true), token: true, ..Clause::default() }])
    } else {
        filter(who)?
    };
    Some(Static::Group { filter, enchanted_player: false, power, toughness, has })
}

/// A creature's static ability about itself or a group: "This creature
/// gets +1/+1 for each creature your opponents control", "As long as you
/// control a Human, this creature has hexproof and indestructible", or a
/// group's change.
fn creature_static(line: &str) -> Option<Static> {
    let body = line.strip_suffix('.')?;
    if let Some((change, each)) = body.strip_prefix("This creature gets ").and_then(|r| r.split_once(" for each ")) {
        let (power, toughness) = pt_change(change)?;
        return Some(Static::PerEach { filter: filter(each)?, power, toughness });
    }
    if let Some((subtype, has)) = body.strip_prefix("As long as you control a ").and_then(|r| r.split_once(", this creature has ")) {
        let has = has.split(" and ").map(keyword_named).collect::<Option<Vec<_>>>()?;
        return Some(Static::IfYouControl { subtype: subtype.to_string(), has });
    }
    group_static(body)
}

/// Read an enchantment: an Aura's "Enchant creature" or "Enchant player"
/// and the static abilities the grammar knows, line by line.
fn read_enchantment(card: &Printed) -> Result<Spec, String> {
    let text = without_reminders(&card.oracle_text).replace(&card.name, "~");
    let mut spec = Spec { permanent: true, ..Spec::default() };
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let unread = || format!("cannot read {line:?}");
        let body = line.strip_suffix('.').unwrap_or(line);
        match body {
            "Enchant creature" | "Enchant player" if spec.slots.is_empty() => {
                spec.enchants = true;
                let slot = if body == "Enchant player" { Slot::Player } else { Slot::Permanent(filter("creature").expect("a filter")) };
                spec.slots.push(slot);
                continue;
            }
            _ => {}
        }
        if line.starts_with("Whenever a creature deals combat damage to enchanted player, ") && spec.slots == [Slot::Player] {
            spec.combat.read(line).ok_or_else(unread)?;
            continue;
        }
        if line.starts_with("At the beginning of enchanted player's upkeep, ") && spec.slots == [Slot::Player] {
            spec.upkeep = Some(Box::new(read_upkeep(line).ok_or_else(unread)?));
            continue;
        }
        if let Some(statics) = restriction_static(body) {
            spec.statics.extend(statics);
            continue;
        }
        if line.contains(": ") {
            let index = spec.activated.len();
            spec.activated.push(read_activated(line, index).ok_or_else(unread)?);
            continue;
        }
        if let Some(rest) = line.strip_prefix("When this Aura enters, ").filter(|_| spec.enchants) {
            let mut chars = rest.chars();
            let sentence: String = chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default();
            let mut trigger = Trigger::default();
            trigger.spec.sentence(&sentence).ok_or_else(unread)?;
            spec.etb = Some(Box::new(trigger));
            continue;
        }
        if let Some(rest) = body.strip_prefix("Enchanted creature ") {
            spec.statics.push(attached_static(rest, None).ok_or_else(unread)?);
            continue;
        }
        if let Some(rest) = body.strip_prefix("Creatures enchanted player controls ") {
            let ((power, toughness), has) = static_change(rest).ok_or_else(unread)?;
            spec.statics.push(Static::Group { filter: filter("creature").expect("a filter"), enchanted_player: true, power, toughness, has });
            continue;
        }
        spec.statics.push(group_static(body).ok_or_else(unread)?);
    }
    if spec.enchants != spec.slots.first().is_some()
        || (spec.statics.is_empty() && spec.upkeep.is_none() && spec.activated.is_empty() && spec.combat == CombatText::default())
    {
        return Err("not an enchantment the grammar reads".to_string());
    }
    Ok(spec)
}

fn mana_named(sym: &str) -> Option<ManaType> {
    if sym == "{C}" {
        return Some(ManaType::Colorless);
    }
    Color::ALL.into_iter().find(|c| sym == format!("{{{}}}", color_letter(*c))).map(ManaType::from)
}

/// Read a land: a basic land's ability from its type (CR 305.6), "{T}:
/// Add {R} or {W}.", and "This land enters tapped unless you control a
/// Mountain or a Plains." Anything else is another family's.
fn read_land(card: &Printed) -> Result<Spec, String> {
    let (_, subtypes) = card.types();
    let mut mana: Vec<ManaAbility> = subtypes.iter()
        .filter_map(|t| match t.as_str() {
            "Plains" => Some(ManaType::White),
            "Island" => Some(ManaType::Blue),
            "Swamp" => Some(ManaType::Black),
            "Mountain" => Some(ManaType::Red),
            "Forest" => Some(ManaType::Green),
            _ => None,
        })
        .map(|t| ManaAbility { cost: 0, mill: 0, options: vec![vec![(t, 1)]] })
        .collect();
    let mut unless = None;
    let mut activated = Vec::new();
    // A basic land's ability is its type's, and comes first.
    let mut index: usize = mana.iter().map(ManaAbility::indices).sum();
    for line in without_reminders(&card.oracle_text).lines().map(str::trim).filter(|l| !l.is_empty()) {
        let unread = || format!("cannot read {line:?}");
        if let Some(ability) = ManaAbility::read(line) {
            index += ability.indices();
            mana.push(ability);
        } else if line.contains(": ") {
            activated.push(read_activated(line, index).ok_or_else(unread)?);
            index += 1;
        } else if let Some(rest) = line.strip_prefix("This land enters tapped unless you control ").and_then(|r| r.strip_suffix('.')) {
            let types: Vec<String> = rest.split(" or ")
                .map(|t| t.trim_start_matches("a ").trim_start_matches("an ").to_string())
                .collect();
            unless = Some(types);
        } else {
            return Err(unread());
        }
    }
    if mana.is_empty() {
        return Err("no mana ability".to_string());
    }
    Ok(Spec { permanent: true, land: Some(unless), mana, activated, ..Spec::default() })
}

// ---- the board ----

/// The board every card is cast on: P0 casts, in their precombat main
/// phase. Cards whose text is keywords or nothing but a trigger that cannot
/// fire mid-turn, so nothing on the board changes what a spell does except
/// through what it is: colour, type, subtype, power, flying, controller.
const BOARD: [(&str, PlayerId); 14] = [
    ("Grizzly Bears", P0),
    ("Chapel Geist", P0),
    ("Markov Patrician", P0),
    ("Gatstaf Shepherd", P0),
    ("Plains", P0),
    ("Sol Ring", P0),
    ("Walking Corpse", P1),
    ("Kindercatch", P1),
    ("Moon Heron", P1),
    ("Goblin Piker", P1),
    ("Rotting Fensnake", P1),
    ("Villagers of Estwald", P1),
    ("Forest", P1),
    ("Burning Vengeance", P1),
];

/// The creature on the board that starts tapped.
const TAPPED_ON_THE_BOARD: &str = "Goblin Piker";

/// What is in each graveyard to begin with, for what targets, counts or
/// exiles cards there: three creature cards of different powers and an
/// instant on the caster's side, a creature card and a land on the other.
const GRAVEYARDS: [(&str, PlayerId); 6] = [
    ("Walking Corpse", P0),
    ("Kalonian Tusker", P0),
    ("Chapel Geist", P0),
    ("Think Twice", P0),
    ("Grizzly Bears", P1),
    ("Forest", P1),
];

/// Put `name` straight into its owner's graveyard, on top of the pile: not
/// by dying, which would count for morbid.
fn card_in_graveyard(state: &mut GameState, registry: &CardRegistry, name: &str, owner: PlayerId) -> ObjectId {
    let card_id = registry.get_id_by_name(name).expect("a card in the registry");
    let data = registry.card_data(card_id).expect("card data");
    let id = state.create_object(card_id, owner, Zone::Graveyard, data.power, data.toughness);
    state.get_object_mut(id).expect("just created").name = name.into();
    state.get_player_mut(owner).graveyard_order.push(id);
    id
}

/// The board, after a creature of the opponent's (Savannah Lions) has died
/// this turn: what morbid is read against.
fn board_after_a_death(registry: &CardRegistry, cache: &BTreeMap<String, Printed>) -> (GameState, World) {
    let (mut state, mut world) = board(registry, cache);
    let lions = named_permanent(&mut state, registry, "Savannah Lions", P1);
    kill_by_damage(&mut state, registry, lions);
    assert!(state.creature_died_this_turn, "the engine records the death");
    world.yard.insert(lions, Model::printed(&cache["Savannah Lions"], P1));
    world.graveyard[1].push("Savannah Lions".to_string());
    world.a_creature_died = true;
    (state, world)
}

/// The board, with a second Grizzly Bears, the opponent's: what a card that
/// reads names is read against.
fn board_with_twins(registry: &CardRegistry, cache: &BTreeMap<String, Printed>) -> (GameState, World) {
    let (mut state, mut world) = board(registry, cache);
    let twin = named_permanent(&mut state, registry, "Grizzly Bears", P1);
    world.permanents.insert(twin, Model::printed(&cache["Grizzly Bears"], P1));
    world.refresh();
    (state, world)
}

/// The opponent casts Giant Growth on Kindercatch and passes, and the
/// caster holds priority with it on the stack: what a counterspell is read
/// against. Done after the card under test is in hand and paid for, so the
/// caster has something to do and the game loop asks them rather than
/// passing for them.
fn opponent_casts_a_spell(state: &mut GameState, world: &mut World, registry: &CardRegistry, cache: &BTreeMap<String, Printed>) {
    let growth = spell_in_hand(state, registry, "Giant Growth", P1);
    fund(state, P1, &mana_cost("{G}").expect("a cost"), 0);
    let kindercatch = world.permanents.iter()
        .find(|(_, m)| m.name.as_deref() == Some("Kindercatch"))
        .map(|(id, _)| *id)
        .expect("on the board");
    let mut cast = Some(cast_action(growth, vec![Target::Object(kindercatch)]));
    engine::run_game_loop(state, registry, |s: &GameState, acting: PlayerId, _legal: &engine::LegalActions| {
        if acting == P1 {
            cast.take().unwrap_or(Action::PassPriority)
        } else if s.stack.is_empty() {
            Action::PassPriority
        } else {
            Action::AbandonGame
        }
    });
    assert!(state.stack.len() == 1 && state.priority_player == Some(P0),
        "Giant Growth is on the stack and the caster has priority");
    world.stack.push((growth, Model::printed(&cache["Giant Growth"], P1)));
}

/// [`opponent_casts_a_spell`], each step submitted rather than played
/// through the game loop, which passes on its own for a player with nothing
/// to do and would let Giant Growth resolve for a caster whose only ability
/// is activated as a sorcery.
fn the_opponent_holds_a_spell_on_the_stack(state: &mut GameState, world: &mut World, registry: &CardRegistry, cache: &BTreeMap<String, Printed>) {
    let growth = spell_in_hand(state, registry, "Giant Growth", P1);
    fund(state, P1, &mana_cost("{G}").expect("a cost"), 0);
    let kindercatch = world.permanents.iter()
        .find(|(_, m)| m.name.as_deref() == Some("Kindercatch"))
        .map(|(id, _)| *id)
        .expect("on the board");
    state.priority_player = Some(P1);
    *state = cast_onto_stack(state, registry, growth, vec![Target::Object(kindercatch)]);
    // The caster passes, and priority goes to the next player, which the
    // game loop does after a submitted pass (CR 117.3d).
    *state = engine::submit_action(state, &Action::PassPriority, registry);
    state.priority_player = Some(P0);
    assert!(state.stack.len() == 1 && state.priority_player == Some(P0),
        "Giant Growth is on the stack and the caster has priority");
    world.stack.push((growth, Model::printed(&cache["Giant Growth"], P1)));
}

/// Each player's library, top first: distinct names, so a card put on top
/// or drawn is told apart from the rest.
const LIBRARY: [&str; 5] = ["Island", "Swamp", "Mountain", "Plains", "Forest"];

/// One permanent as the reference sees it: its printed characteristics and
/// what has happened to it.
#[derive(Debug, Clone)]
struct Model {
    /// None for a token.
    name: Option<String>,
    owner: PlayerId,
    controller: PlayerId,
    types: Vec<String>,
    subtypes: Vec<String>,
    colors: Vec<Color>,
    keywords: Vec<Keyword>,
    power: Option<i32>,
    toughness: Option<i32>,
    damage: u32,
    tapped: bool,
    /// +1/+1 and -1/-1 counters (CR 122.1a), already counted in power
    /// and toughness.
    counters: (u32, u32),
    /// Power and toughness are the number of creatures its controller
    /// controls, counted when observed (CR 604.3).
    counts_creatures: bool,
    /// What an Aura enchants (CR 303.4).
    attached: Option<Target>,
    /// A werewolf's face that is not showing, and whether the back face is
    /// the one showing (CR 712.8).
    other_face: Option<Box<Model>>,
    transformed: bool,
    /// Dealt damage by a source with deathtouch (CR 702.2b).
    deathtouched: bool,
    /// Power and toughness are the number of creature cards in its
    /// controller's graveyard, counted when observed (CR 604.3).
    counts_creature_cards: bool,
    /// ... or the number of cards in its controller's hand.
    counts_hand: bool,
    /// Its text about combat.
    combat: CombatText,
    /// Its static abilities (CR 604.1), which [`World::refresh`] applies.
    statics: Vec<Static>,
    /// What the static abilities on the battlefield do to it as they stand:
    /// keywords they grant, and their change to power and toughness, not
    /// counted in `keywords`, `power` or `toughness`.
    granted: Vec<Keyword>,
    boost: (i32, i32),
    /// ... and the rules they put on it.
    restricted: Vec<Restriction>,
    /// Regeneration shields (CR 701.19a), which last the turn.
    shields: u32,
    /// What "until end of turn" effects have done to it: a change to power
    /// and toughness, and keywords, both ending in the cleanup step (CR
    /// 514.2, 611.2a).
    temp: (i32, i32),
    temp_keywords: Vec<Keyword>,
    /// Who controlled it before an effect that gives it back in the
    /// cleanup step took it (CR 514.2).
    stolen_from: Option<PlayerId>,
    /// A token that entered attacking (CR 508.4), and whether its text
    /// exiles it at end of combat.
    attacking: bool,
    exile_at_end_of_combat: bool,
}

impl Model {
    fn printed(p: &Printed, owner: PlayerId) -> Self {
        let (types, subtypes) = p.types();
        Model {
            name: Some(p.name.clone()),
            owner,
            controller: owner,
            types: types.iter().map(|t| t.to_lowercase()).collect(),
            subtypes,
            colors: p.colors(),
            keywords: p.keywords(),
            power: p.power,
            toughness: p.toughness,
            damage: 0,
            tapped: false,
            counters: (0, 0),
            counts_creatures: false,
            attached: None,
            other_face: None,
            transformed: false,
            deathtouched: false,
            counts_creature_cards: false,
            counts_hand: false,
            combat: CombatText::default(),
            statics: Vec::new(),
            granted: Vec::new(),
            boost: (0, 0),
            restricted: Vec::new(),
            shields: 0,
            temp: (0, 0),
            temp_keywords: Vec::new(),
            stolen_from: None,
            attacking: false,
            exile_at_end_of_combat: false,
        }
    }

    /// [`Model::printed`], and for a double-faced card the grammar reads,
    /// its text and its back face ready to turn to.
    fn printed_with_faces(p: &Printed, owner: PlayerId) -> Self {
        match read_creature(p) {
            Ok(front) if p.back.is_some() => model_of(&front, p, owner),
            _ => Model::printed(p, owner),
        }
    }

    /// The back face of `p`, if it has one the grammar reads: its printed
    /// characteristics, its colour indicator's colours (CR 204.1, 712.8e),
    /// and its text.
    fn back_face(p: &Printed, owner: PlayerId) -> Option<Box<Model>> {
        let back = p.back.as_ref()?;
        let spec = back_face_reads(back)?;
        Some(Box::new(Model { statics: spec.statics, combat: spec.combat, ..Model::printed(back, owner) }))
    }

    /// Turn to the other face: its characteristics, this object's state.
    fn transform(&mut self) {
        let Some(mut other) = self.other_face.take() else { return };
        std::mem::swap(&mut self.name, &mut other.name);
        std::mem::swap(&mut self.types, &mut other.types);
        std::mem::swap(&mut self.subtypes, &mut other.subtypes);
        std::mem::swap(&mut self.colors, &mut other.colors);
        std::mem::swap(&mut self.keywords, &mut other.keywords);
        std::mem::swap(&mut self.power, &mut other.power);
        std::mem::swap(&mut self.toughness, &mut other.toughness);
        std::mem::swap(&mut self.statics, &mut other.statics);
        std::mem::swap(&mut self.combat, &mut other.combat);
        self.other_face = Some(other);
        self.transformed = !self.transformed;
    }

    fn is_creature(&self) -> bool {
        self.types.iter().any(|t| t == "creature")
    }

    fn matches(&self, filter: &Filter, caster: PlayerId) -> bool {
        self.matches_from(filter, caster, false)
    }

    /// [`Model::matches`] for an ability whose source this is when
    /// `is_source`, which "other" leaves out (CR 109.5).
    fn matches_from(&self, filter: &Filter, caster: PlayerId, is_source: bool) -> bool {
        let has = |k: &Keyword| self.keywords.contains(k) || self.granted.contains(k) || self.temp_keywords.contains(k);
        filter.0.iter().any(|c| {
            let head = match &c.head {
                None => true,
                Some(h) if CARD_TYPES.contains(&h.as_str()) => self.types.contains(h),
                Some(sub) => self.subtypes.contains(sub),
            };
            head
                && !c.not_colors.iter().any(|col| self.colors.contains(col))
                && !c.not_types.iter().any(|t| self.types.contains(t) || self.subtypes.contains(t))
                && c.subtypes.iter().all(|t| self.subtypes.contains(t))
                && c.min_power.is_none_or(|n| self.power.is_some_and(|p| p + self.boost.0 + self.temp.0 >= n))
                && !c.without.iter().any(has)
                && c.with.iter().all(has)
                && !(c.other && is_source)
                && (!c.token || self.name.is_none())
                && c.yours.is_none_or(|y| (self.controller == caster) == y)
                // Only an attacking creature is (CR 506.4); the board is in
                // the precombat main phase.
                && (!c.attacking || self.attacking)
        })
    }
}

/// What a static ability does to one creature: a change to its power and
/// toughness, keywords, and rules.
struct Change {
    pt: (i32, i32),
    has: Vec<Keyword>,
    restricted: Vec<Restriction>,
}

/// Whether `m` is under `r`, by its own text or a static ability.
fn restricted(m: &Model, r: Restriction) -> bool {
    m.restricted.contains(&r) || match r {
        Restriction::CantBlock => m.combat.cant_block,
        Restriction::AttacksEachCombat => m.combat.attacks_each_combat,
        _ => false,
    }
}

/// The reference's whole board: permanents by the id the engine gave them,
/// and each player's zones by name.
#[derive(Debug, Clone)]
struct World {
    permanents: BTreeMap<ObjectId, Model>,
    /// The cards in graveyards, by id, for what targets them and counts
    /// them; `graveyard` is the same cards by name.
    yard: BTreeMap<ObjectId, Model>,
    /// A creature died this turn (CR 700.4), which morbid reads.
    a_creature_died: bool,
    /// Spells on the stack below the one being checked, by id, last cast
    /// last (CR 405.2).
    stack: Vec<(ObjectId, Model)>,
    /// The permanent whose ability is being read, which "another" excludes.
    source: Option<ObjectId>,
    /// The cards exiled to pay the spell's additional cost.
    cost_exiled: Vec<Model>,
    /// The creature sacrificed to pay an ability's cost, or by its effect,
    /// as it last was.
    cost_sacrificed: Option<Model>,
    /// The creatures players chose as the spell resolves ("sacrifices a
    /// creature of their choice", "each player chooses a creature"), one
    /// way of choosing; [`resolution_choices`] lists them all.
    chosen: Vec<ObjectId>,
    /// The cards discarded at random, by name, one way it can go;
    /// [`random_choices`] lists them all.
    discarded: Vec<String>,
    /// The card a search finds, by name, and whether it goes onto the
    /// battlefield; none when the searcher finds nothing (CR 701.19b).
    found: Option<(String, bool)>,
    /// Each player whose library has been shuffled, whose order is then
    /// not compared (CR 701.20a).
    shuffled: [bool; 2],
    /// The value of X for the spell or ability resolving: the one its
    /// controller announced (CR 107.3a, 601.2b), or the number of cards
    /// exiled for an "exile X cards" cost.
    x: u32,
    /// Whether each double-faced permanent was transformed when the
    /// triggers resolving were put on the stack, which a transform of its
    /// own asks (CR 701.27f); empty for an ability resolving as it was
    /// activated, nothing between.
    faces_put: BTreeMap<ObjectId, bool>,
    /// Permanents the reference has no engine id for: tokens, and the
    /// permanent a creature spell becomes.
    tokens: Vec<Model>,
    life: [i32; 2],
    /// Floating mana: what a cost left of what was funded for it.
    pool: [BTreeMap<ManaType, u32>; 2],
    hand: [Vec<String>; 2],
    graveyard: [Vec<String>; 2],
    exile: [Vec<String>; 2],
    library: [Vec<String>; 2],
}

/// The reference's pool after paying an {X} of `x` from `X_FLOAT`
/// floating colorless.
fn x_left(world: &World, x: u32) -> [BTreeMap<ManaType, u32>; 2] {
    let mut pool = world.pool.clone();
    *pool[0].entry(ManaType::Colorless).or_insert(0) += X_FLOAT - x;
    pool
}

/// What is observable about a game at rest, in a form both the engine's
/// state and the reference's world produce: every zone by name, every
/// permanent by its characteristics.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Observed {
    life: [i32; 2],
    /// Each player's floating mana, by type: so a cost paid short or over
    /// shows (CR 106.4, 601.2h).
    pool: [Vec<(ManaType, u32)>; 2],
    hand: [Vec<String>; 2],
    graveyard: [Vec<String>; 2],
    exile: [Vec<String>; 2],
    library: [Vec<String>; 2],
    battlefield: Vec<PermanentSeen>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PermanentSeen {
    name: Option<String>,
    controller: u8,
    power: Option<i32>,
    toughness: Option<i32>,
    damage: u32,
    tapped: bool,
    keywords: Vec<&'static str>,
    colors: Vec<char>,
    subtypes: Vec<String>,
    counters: (u32, u32),
    /// What it is attached to: a permanent by name, or "player N".
    attached_to: Option<String>,
    regeneration_shields: u32,
}

fn seen(m: &Model) -> PermanentSeen {
    let mut keywords: Vec<&'static str> = m.keywords.iter().chain(&m.granted).chain(&m.temp_keywords).map(|k| k.label()).collect();
    keywords.sort_unstable();
    keywords.dedup();
    let mut colors: Vec<char> = m.colors.iter().map(|c| color_letter(*c)).collect();
    colors.sort_unstable();
    let mut subtypes = m.subtypes.clone();
    subtypes.sort();
    PermanentSeen {
        name: m.name.clone(),
        controller: m.controller.0,
        power: m.power,
        toughness: m.toughness,
        damage: m.damage,
        tapped: m.tapped,
        keywords,
        colors,
        subtypes,
        counters: m.counters,
        attached_to: None,
        regeneration_shields: m.shields,
    }
}

/// Where a permanent goes when it leaves the battlefield: one of its
/// owner's zones.
#[derive(Clone, Copy)]
enum Dest {
    Graveyard,
    Exile,
    Hand,
    LibraryTop,
}

impl World {
    /// This world as triggers are put on the stack from it, each
    /// double-faced permanent's face recorded (CR 701.27f).
    fn with_faces_put(&self) -> World {
        let faces_put = self.permanents.iter()
            .filter(|(_, m)| m.other_face.is_some())
            .map(|(id, m)| (*id, m.transformed))
            .collect();
        World { faces_put, ..self.clone() }
    }

    /// A permanent's power and toughness as they stand: printed, changed,
    /// or counted (CR 604.3) when its text defines them.
    fn pt_of(&self, m: &Model) -> (Option<i32>, Option<i32>) {
        let (p, t) = self.defined_pt(m);
        (p.map(|p| p + m.boost.0 + m.temp.0), t.map(|t| t + m.boost.1 + m.temp.1))
    }

    /// [`World::pt_of`] before static abilities change it (CR 613.4b-c).
    fn defined_pt(&self, m: &Model) -> (Option<i32>, Option<i32>) {
        let n = |count: usize| Some(i32::try_from(count).expect("a few"));
        if m.counts_creatures {
            let c = self.permanents.values().chain(&self.tokens).filter(|o| o.is_creature() && o.controller == m.controller).count();
            (n(c), n(c))
        } else if m.counts_creature_cards {
            let c = self.creature_cards_in_graveyard(m.controller) as usize;
            (n(c), n(c))
        } else if m.counts_hand {
            let c = self.hand[m.controller.0 as usize].len();
            (n(c), n(c))
        } else {
            (m.power, m.toughness)
        }
    }

    fn observed(&self) -> Observed {
        let mut now = self.clone();
        now.refresh();
        let all: Vec<&Model> = now.permanents.values().chain(&now.tokens).collect();
        let mut battlefield: Vec<PermanentSeen> = all.iter()
            .map(|m| {
                let mut seen = seen(m);
                (seen.power, seen.toughness) = now.pt_of(m);
                if let Some(t) = &m.combat.haste_if_opponent_controls {
                    if all.iter().any(|o| o.controller != m.controller && o.subtypes.contains(t)) && !seen.keywords.contains(&"haste") {
                        seen.keywords.push("haste");
                        seen.keywords.sort_unstable();
                    }
                }
                seen.attached_to = match &m.attached {
                    Some(Target::Object(id)) => self.permanents.get(id).and_then(|a| a.name.clone()),
                    Some(Target::Player(p)) => Some(format!("player {}", p.0)),
                    _ => None,
                };
                seen
            })
            .collect();
        battlefield.sort();
        let sorted = |zones: &[Vec<String>; 2]| zones.clone().map(|mut z| { z.sort(); z });
        Observed {
            life: self.life,
            pool: self.pool.clone().map(|p| p.into_iter().filter(|(_, n)| *n > 0).collect()),
            hand: sorted(&self.hand),
            graveyard: sorted(&self.graveyard),
            exile: sorted(&self.exile),
            library: self.library.clone(),
            battlefield,
        }
    }

    /// A permanent leaves the battlefield for one of its owner's zones; a
    /// token leaves the game (CR 111.7, 704.5d).
    fn leave(&mut self, id: ObjectId, to: Dest) {
        let Some(mut m) = self.permanents.remove(&id) else { return };
        // Off the battlefield a double-faced card has only its front face
        // (CR 712.8a).
        if m.transformed {
            m.transform();
        }
        let Some(name) = m.name.clone() else { return };
        let owner = m.owner.0 as usize;
        match to {
            Dest::Graveyard => {
                self.graveyard[owner].push(name);
                // A new object, with its printed characteristics (CR 400.7).
                self.yard.insert(id, Model { controller: m.owner, damage: 0, tapped: false, counters: (0, 0), granted: Vec::new(), boost: (0, 0), restricted: Vec::new(), shields: 0, temp: (0, 0), temp_keywords: Vec::new(), ..m });
            }
            Dest::Exile => self.exile[owner].push(name),
            Dest::Hand => self.hand[owner].push(name),
            Dest::LibraryTop => self.library[owner].insert(0, name),
        }
    }

    /// A card leaves a graveyard for `to`, by id.
    fn leave_yard(&mut self, id: ObjectId) -> Option<Model> {
        let m = self.yard.remove(&id)?;
        let names = &mut self.graveyard[m.owner.0 as usize];
        let at = names.iter().position(|n| Some(n) == m.name.as_ref()).expect("a yard card is in its graveyard");
        names.remove(at);
        Some(m)
    }

    fn creature_cards_in_graveyard(&self, p: PlayerId) -> u32 {
        u32::try_from(self.yard.values().filter(|m| m.owner == p && m.is_creature()).count()).expect("a few")
    }

    /// Apply the static abilities on the battlefield as the board stands
    /// (CR 611.3a: their effects are not locked in, so a lord's bonus ends
    /// when it leaves and a count follows what it counts): first the
    /// keywords they grant (layer 6), then their changes to power and
    /// toughness (layer 7c), whose "with flying" sees those keywords (CR
    /// 613.1f, 613.4c).
    fn refresh(&mut self) {
        for m in self.permanents.values_mut().chain(self.tokens.iter_mut()) {
            m.granted.clear();
            m.boost = (0, 0);
            m.restricted.clear();
        }
        for keywords in [true, false] {
            let before = self.clone();
            let n = before.permanents.len();
            let targets = before.permanents.iter().map(|(id, m)| (Some(*id), m)).chain(before.tokens.iter().map(|m| (None, m)));
            for (k, (id, m)) in targets.enumerate() {
                let ((mut power, mut toughness), mut has, mut restricted) = ((0, 0), Vec::new(), Vec::new());
                for (src_id, src) in &before.permanents {
                    for st in &src.statics {
                        if let Some(Change { pt: (p, t), has: h, restricted: r }) = before.static_effect(st, *src_id, src, id, m) {
                            power += p;
                            toughness += t;
                            has.extend(h);
                            restricted.extend(r);
                        }
                    }
                }
                let target = match id {
                    Some(id) => self.permanents.get_mut(&id).expect("on the battlefield"),
                    None => &mut self.tokens[k - n],
                };
                if keywords {
                    target.granted = has;
                    target.restricted = restricted;
                } else {
                    target.boost = (power, toughness);
                }
            }
        }
    }

    /// What static ability `st` of `src` (id `src_id`) does to the creature
    /// `m` (id `id`, none for a token); None when it does not apply to it.
    fn static_effect(&self, st: &Static, src_id: ObjectId, src: &Model, id: Option<ObjectId>, m: &Model) -> Option<Change> {
        if !m.is_creature() {
            return None;
        }
        let is_source = id == Some(src_id);
        let count = |n: usize| i32::try_from(n).expect("a few");
        let change = |pt: (i32, i32), has: &[Keyword]| Change { pt, has: has.to_vec(), restricted: Vec::new() };
        let on_it = matches!(src.attached, Some(Target::Object(e)) if Some(e) == id);
        match st {
            Static::Enchanted { power, toughness, has, per, if_subtype } => {
                let k = match per {
                    None => 1,
                    Some(Per::CreatureCards) => count(self.creature_cards_in_graveyard(src.controller) as usize),
                    Some(Per::InstantAndSorceryCards) => count(self.yard.values()
                        .filter(|c| c.owner == src.controller && c.types.iter().any(|t| t == "instant" || t == "sorcery"))
                        .count()),
                };
                let holds = if_subtype.as_ref().is_none_or(|t| m.subtypes.contains(t));
                (holds && on_it).then(|| change((power * k, toughness * k), has))
            }
            Static::Group { filter, enchanted_player, power, toughness, has } => {
                let whose = !enchanted_player || matches!(src.attached, Some(Target::Player(p)) if p == m.controller);
                (whose && m.matches_from(filter, src.controller, is_source)).then(|| change((*power, *toughness), has))
            }
            Static::PerEach { filter, power, toughness } => {
                let k = count(self.permanents.values().chain(&self.tokens).filter(|o| o.matches(filter, src.controller)).count());
                is_source.then(|| change((power * k, toughness * k), &[]))
            }
            Static::IfYouControl { subtype, has } => {
                let holds = self.permanents.values().chain(&self.tokens).any(|o| o.controller == src.controller && o.subtypes.contains(subtype));
                (is_source && holds).then(|| change((0, 0), has))
            }
            Static::Restricts { group, what, unless_subtype } => {
                let whom = if *group { matches!(src.attached, Some(Target::Player(p)) if p == m.controller) } else { on_it };
                let holds = unless_subtype.as_ref().is_none_or(|t| !m.subtypes.contains(t));
                (whom && holds).then(|| Change { pt: (0, 0), has: Vec::new(), restricted: what.clone() })
            }
        }
    }

    /// Destroy `id` (CR 701.8): nothing if it is indestructible (702.12b);
    /// if it has a regeneration shield, the shield is used instead, and it
    /// is tapped and its damage removed (701.19c); otherwise it goes to its
    /// owner's graveyard, a death if it is a creature (700.4). Whether it
    /// was destroyed.
    fn destroy(&mut self, id: ObjectId) -> bool {
        let Some(m) = self.permanents.get_mut(&id) else { return false };
        if has(m, Keyword::Indestructible) {
            return false;
        }
        if m.shields > 0 {
            m.shields -= 1;
            m.damage = 0;
            m.deathtouched = false;
            m.tapped = true;
            return false;
        }
        let creature = m.is_creature();
        self.leave(id, Dest::Graveyard);
        self.a_creature_died |= creature;
        true
    }

    /// State-based actions, as CR 704.5f-h state them: a creature with
    /// toughness 0 or less is put into its owner's graveyard, and one with
    /// damage at least its toughness, or any from a source with deathtouch,
    /// is destroyed, which indestructible stops (702.12b). Repeated until
    /// none applies (CR 704.3), the static abilities applied afresh each
    /// time.
    fn state_based_actions(&mut self) {
        loop {
            self.refresh();
            let dies = |m: &Model| m.is_creature() && self.pt_of(m).1.is_some_and(|t| t <= 0
                || (!has(m, Keyword::Indestructible) && (m.damage >= t as u32 || (m.deathtouched && m.damage > 0))));
            let zero = |m: &Model| self.pt_of(m).1.is_some_and(|t| t <= 0);
            let dying: Vec<(ObjectId, bool)> = self.permanents.iter()
                .filter(|(_, m)| dies(m))
                .map(|(id, m)| (*id, zero(m)))
                .collect();
            let tokens_dying: Vec<bool> = self.tokens.iter().map(dies).collect();
            let falls = self.permanents.values()
                .any(|m| matches!(m.attached, Some(Target::Object(a)) if !self.permanents.contains_key(&a)));
            if dying.is_empty() && !tokens_dying.contains(&true) && !falls {
                return;
            }
            // A token dies too (CR 700.4), and then ceases to exist (704.5d).
            self.a_creature_died |= tokens_dying.contains(&true);
            let mut flags = tokens_dying.into_iter();
            self.tokens.retain(|_| !flags.next().expect("one per token"));
            // Toughness 0 is not destruction (704.5f); lethal damage is
            // (704.5g-h), and a shield can stop it.
            for (id, zero) in dying {
                if zero {
                    self.leave(id, Dest::Graveyard);
                    self.a_creature_died = true;
                } else {
                    self.destroy(id);
                }
            }
            // An Aura attached to nothing goes to its owner's graveyard
            // (CR 704.5m); an Equipment becomes unattached and stays
            // (704.5n).
            let fallen: Vec<ObjectId> = self.permanents.iter()
                .filter(|(_, m)| matches!(m.attached, Some(Target::Object(a)) if !self.permanents.contains_key(&a)))
                .map(|(id, _)| *id)
                .collect();
            for id in fallen {
                if self.permanents[&id].subtypes.iter().any(|t| t == "Equipment") {
                    self.permanents.get_mut(&id).expect("on the battlefield").attached = None;
                } else {
                    self.leave(id, Dest::Graveyard);
                }
            }
        }
    }
}

/// The board, built in the engine, and the same board as the reference
/// sees it.
fn board(registry: &CardRegistry, cache: &BTreeMap<String, Printed>) -> (GameState, World) {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mut world = World {
        permanents: BTreeMap::new(),
        yard: BTreeMap::new(),
        a_creature_died: false,
        stack: Vec::new(),
        source: None,
        cost_exiled: Vec::new(),
        cost_sacrificed: None,
        chosen: Vec::new(),
        discarded: Vec::new(),
        found: None,
        shuffled: [false, false],
        x: 0,
        faces_put: BTreeMap::new(),
        tokens: Vec::new(),
        life: [20, 20],
        pool: [BTreeMap::new(), BTreeMap::new()],
        hand: [Vec::new(), Vec::new()],
        graveyard: [Vec::new(), Vec::new()],
        exile: [Vec::new(), Vec::new()],
        library: [Vec::new(), Vec::new()],
    };
    for (name, owner) in BOARD {
        let id = named_permanent(&mut state, registry, name, owner);
        world.permanents.insert(id, Model::printed_with_faces(&cache[name], owner));
        // One of the opponent's creatures tapped, as by an attack, so an
        // untap has something to do.
        if name == TAPPED_ON_THE_BOARD {
            state.get_object_mut(id).expect("just placed").tapped = true;
            world.permanents.get_mut(&id).expect("just placed").tapped = true;
        }
    }
    for (name, owner) in GRAVEYARDS {
        let id = card_in_graveyard(&mut state, registry, name, owner);
        world.yard.insert(id, Model::printed(&cache[name], owner));
        world.graveyard[owner.0 as usize].push(name.to_string());
    }
    // A land the caster could play, so that once the spell has resolved the
    // game loop has something to ask them: it passes priority on its own
    // for a player with nothing to do, and would play on into the next
    // turn after a spell that left the caster nothing.
    spell_in_hand(&mut state, registry, "Island", P0);
    world.hand[0].push("Island".to_string());
    for p in [P0, P1] {
        for name in LIBRARY {
            stock_library_with(&mut state, registry, p, name, 1);
            world.library[p.0 as usize].push(name.to_string());
        }
    }
    world.refresh();
    (state, world)
}

/// The engine's state, observed the way [`World::observed`] observes the
/// reference.
fn observe(state: &GameState, registry: &CardRegistry) -> Observed {
    let name_of = |id: ObjectId| -> String {
        let obj = state.get_object(id).expect("a zone names an object that exists");
        registry.card_data(obj.card_id).map_or_else(|| obj.name.clone(), |d| d.name)
    };
    // By owner: a hand, a graveyard and a library are each a player's
    // own, and exile is one zone for everyone (CR 400.1), its cards told
    // apart here by whose they are.
    let zone = |zone: Zone| -> [Vec<String>; 2] {
        [P0, P1].map(|p| {
            let mut names: Vec<String> = state.objects.values()
                .filter(|o| o.zone == zone && o.owner == p)
                .map(|o| name_of(o.id))
                .collect();
            names.sort();
            names
        })
    };
    let mut battlefield: Vec<PermanentSeen> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield)
        .map(|o| {
            let creature = state.is_creature(o.id, registry);
            let mut keywords: Vec<&'static str> = Keyword::ALL.into_iter()
                .filter(|k| state.has_keyword(o.id, *k, registry))
                .map(|k| k.label())
                .collect();
            keywords.sort_unstable();
            let mut colors: Vec<char> = state.colors_of(o.id, registry).into_iter().map(color_letter).collect();
            colors.sort_unstable();
            let mut subtypes = state.subtypes_of(o.id, registry);
            subtypes.sort();
            PermanentSeen {
                // The active face's name (CR 712.8).
                name: if o.is_token { None } else { Some(state.name_of(o.id, registry)) },
                controller: o.controller.0,
                power: if creature { state.effective_power(o.id, registry) } else { None },
                toughness: if creature { state.effective_toughness(o.id, registry) } else { None },
                damage: o.damage_marked,
                tapped: o.tapped,
                keywords,
                colors,
                subtypes,
                counters: (
                    o.counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0),
                    o.counters.get(&CounterType::MinusOneMinusOne).copied().unwrap_or(0),
                ),
                regeneration_shields: o.regeneration_shields,
                attached_to: o.attached_to.map(|host| state.name_of(host, registry))
                    .or_else(|| o.attached_to_player.map(|p| format!("player {}", p.0))),
            }
        })
        .collect();
    battlefield.sort();
    Observed {
        life: [state.players[0].life, state.players[1].life],
        pool: [P0, P1].map(|p| state.players[p.0 as usize].mana_pool.mana.iter().filter(|(_, n)| **n > 0).map(|(t, n)| (*t, *n)).collect()),
        hand: zone(Zone::Hand),
        graveyard: zone(Zone::Graveyard),
        exile: zone(Zone::Exile),
        library: [P0, P1].map(|p| state.players[p.0 as usize].library_order.iter().map(|&id| name_of(id)).collect()),
        battlefield,
    }
}

/// What differs between the engine's game and the text's, field by field:
/// the zones that disagree, and the permanents only one side has.
fn differences(engine: &Observed, text: &Observed) -> String {
    let mut out = Vec::new();
    if engine.life != text.life {
        out.push(format!("life: engine {:?}, the text {:?}", engine.life, text.life));
    }
    if engine.pool != text.pool {
        out.push(format!("mana pool: engine {:?}, the text {:?}", engine.pool, text.pool));
    }
    for (what, e, t) in [
        ("hand", &engine.hand, &text.hand),
        ("graveyard", &engine.graveyard, &text.graveyard),
        ("exile", &engine.exile, &text.exile),
        ("library", &engine.library, &text.library),
    ] {
        if e != t {
            out.push(format!("{what}: engine {e:?}, the text {t:?}"));
        }
    }
    let only = |a: &[PermanentSeen], b: &[PermanentSeen]| -> Vec<String> {
        let mut rest = b.to_vec();
        a.iter()
            .filter(|p| match rest.iter().position(|q| q == *p) {
                Some(i) => { rest.remove(i); false }
                None => true,
            })
            .map(|p| format!("{p:?}"))
            .collect()
    };
    let e = only(&engine.battlefield, &text.battlefield);
    let t = only(&text.battlefield, &engine.battlefield);
    if !e.is_empty() || !t.is_empty() {
        out.push(format!("battlefield: only the engine has\n        {}\n      only the text has\n        {}",
            e.join("\n        "), t.join("\n        ")));
    }
    out.join("\n      ")
}

// ---- the reference semantics ----

/// Every target a slot accepts on `world`, for a spell `caster` casts. No
/// permanent on the board has hexproof, shroud or protection, so "can be
/// targeted" is "matches the words" (CR 115.1, 702.11, 702.16).
fn candidates(slot: &Slot, world: &World, caster: PlayerId) -> Vec<Target> {
    let players = |which: &dyn Fn(PlayerId) -> bool| -> Vec<Target> {
        [P0, P1].into_iter().filter(|p| which(*p)).map(Target::Player).collect()
    };
    let creatures = || world.permanents.iter()
        .filter(|(_, m)| m.is_creature())
        .map(|(id, _)| Target::Object(*id))
        .collect::<Vec<_>>();
    let mut out = match slot {
        Slot::Any => [creatures(), players(&|_| true)].concat(),
        Slot::PlayerOrPlaneswalker | Slot::Player => players(&|_| true),
        Slot::Opponent => players(&|p| p != caster),
        Slot::Permanent(f) | Slot::UpTo { filter: f, .. } => world.permanents.iter()
            .filter(|(id, m)| m.matches(f, caster)
                && !(f.0.iter().all(|c| c.other) && world.source == Some(**id)))
            .map(|(id, _)| Target::Object(*id))
            .collect(),
        Slot::This => world.source.into_iter().map(Target::Object).collect(),
        // No planeswalker is on the board, so "that player or a
        // planeswalker that player controls" is the player.
        Slot::EnchantedCreature => world.source.and_then(|s| world.permanents.get(&s))
            .and_then(|m| match m.attached { Some(Target::Object(id)) => Some(Target::Object(id)), _ => None })
            .into_iter()
            .collect(),
        Slot::EnchantedPlayer { .. } => world.source.and_then(|s| world.permanents.get(&s))
            .and_then(|m| match m.attached { Some(Target::Player(p)) => Some(Target::Player(p)), _ => None })
            .into_iter()
            .collect(),
        Slot::Spell(f) => world.stack.iter()
            .filter(|(_, m)| m.matches(f, caster))
            .map(|(id, _)| Target::Object(*id))
            .collect(),
        Slot::GraveyardCard { filter, yours } => world.yard.iter()
            .filter(|(_, m)| !*yours || m.owner == caster)
            .filter(|(_, m)| filter.as_ref().is_none_or(|f| m.matches(f, caster)))
            .map(|(id, _)| Target::Object(*id))
            .collect(),
    };
    out.sort_by_key(target_key);
    out
}

fn target_key(t: &Target) -> (u8, u64) {
    match t {
        Target::Object(id) => (0, id.0),
        Target::Player(p) => (1, u64::from(p.0)),
        other => panic!("a cast names an object or a player, not {other:?}"),
    }
}

/// Every set of at most `n` of `options`, each in `options`' order.
fn subsets(options: &[Target], n: usize) -> Vec<Vec<Target>> {
    let mut out = vec![Vec::new()];
    for t in options {
        let grown: Vec<Vec<Target>> = out.iter()
            .filter(|s| s.len() < n)
            .map(|s| [s.clone(), vec![t.clone()]].concat())
            .collect();
        out.extend(grown);
    }
    out
}

/// Every set of targets the text allows: one from each slot, the same
/// object at most once per slot (CR 115.3) and allowed across slots,
/// which is what the spells here would need (none shares a candidate
/// between slots). An "up to N" slot is any set of at most N.
/// Every order of `0..n`.
fn permutations(n: usize) -> Vec<Vec<usize>> {
    if n == 0 {
        return vec![Vec::new()];
    }
    permutations(n - 1).into_iter()
        .flat_map(|p| (0..n).map(move |at| {
            let mut q = p.clone();
            q.insert(at, n - 1);
            q
        }))
        .collect()
}

fn target_sets(spec: &Spec, world: &World, caster: PlayerId) -> Vec<Vec<Target>> {
    if let [Slot::UpTo { n, .. }] = spec.slots.as_slice() {
        let mut sets = subsets(&candidates(&spec.slots[0], world, caster), *n);
        sets.sort_by_key(|s| s.iter().map(target_key).collect::<Vec<_>>());
        return sets;
    }
    assert!(!spec.slots.iter().any(|s| matches!(s, Slot::UpTo { .. })), "an up-to slot is a card's only slot here");
    let mut sets: Vec<Vec<Target>> = vec![Vec::new()];
    for slot in &spec.slots {
        let options = candidates(slot, world, caster);
        sets = sets.into_iter()
            .flat_map(|set| options.iter().map(move |t| {
                let mut s = set.clone();
                s.push(t.clone());
                s
            }))
            .collect();
    }
    sets
}

/// A creature token with the characteristics its text gives it (CR 111.3).
fn creature_token((power, toughness): (i32, i32), colors: &[Color], subtypes: &[String], keywords: &[Keyword], controller: PlayerId) -> Model {
    Model {
        name: None,
        owner: controller,
        controller,
        types: vec!["creature".to_string()],
        subtypes: subtypes.to_vec(),
        colors: colors.to_vec(),
        keywords: keywords.to_vec(),
        power: Some(power),
        toughness: Some(toughness),
        damage: 0,
        tapped: false,
        counters: (0, 0),
        counts_creatures: false,
        attached: None,
        other_face: None,
        transformed: false,
        deathtouched: false,
        counts_creature_cards: false,
        counts_hand: false,
        combat: CombatText::default(),
        statics: Vec::new(),
        granted: Vec::new(),
        boost: (0, 0),
        restricted: Vec::new(),
        shields: 0,
        temp: (0, 0),
        temp_keywords: Vec::new(),
        stolen_from: None,
        attacking: false,
        exile_at_end_of_combat: false,
    }
}

/// An "until end of turn" change to `m` (CR 611.2a, 613.4c).
fn until_end_of_turn(m: &mut Model, (power, toughness): (i32, i32), gains: &[Keyword]) {
    m.temp.0 += power;
    m.temp.1 += toughness;
    m.temp_keywords.extend(gains);
}

/// A creature's power, toughness and counters changed by `by` (CR 613.4).
fn change(m: &mut Model, power: i32, toughness: i32) {
    m.power = m.power.map(|p| p + power);
    m.toughness = m.toughness.map(|t| t + toughness);
}

/// What `spec`'s effects do to `world` with `targets`, in order (CR
/// 608.2c), before state-based actions.
fn apply(spec: &Spec, world: &World, targets: &[Target], caster: PlayerId) -> World {
    let mut w = world.clone();
    let c = caster.0 as usize;
    // An "up to" slot's targets are the whole list; any other slot is one.
    let objects = |slot: usize| -> Vec<ObjectId> {
        let ts: &[Target] = if matches!(spec.slots[slot], Slot::UpTo { .. }) { targets } else { &targets[slot..=slot] };
        ts.iter().filter_map(|t| match t { Target::Object(id) => Some(*id), _ => None }).collect()
    };
    let object = |slot: usize| objects(slot).first().copied();
    let player = |slot: usize| match &targets[slot] {
        Target::Player(p) => Some(p.0 as usize),
        _ => None,
    };
    let mut destroyed = 0;
    for (i, effect) in spec.effects.iter().enumerate() {
        match effect {
            Effect::Damage { amount, slot } => {
                let amount = match spec.morbid {
                    Some((which, instead)) if which == i && world.a_creature_died => instead,
                    _ => *amount,
                };
                if let Some(id) = object(*slot) {
                    w.permanents.get_mut(&id).expect("a target on the board").damage += amount;
                } else if let Some(p) = player(*slot) {
                    w.life[p] -= amount as i32;
                }
            }
            Effect::DamageEach { amount, filter } => {
                for m in w.permanents.values_mut().chain(w.tokens.iter_mut()) {
                    if m.matches(filter, caster) {
                        m.damage += amount;
                    }
                }
            }
            Effect::Pump { slot, power, toughness, gains } => {
                if let Some(m) = object(*slot).and_then(|id| w.permanents.get_mut(&id)) {
                    until_end_of_turn(m, (*power, *toughness), gains);
                }
            }
            Effect::PumpEach { filter, power, toughness, gains } => {
                for m in w.permanents.values_mut().chain(w.tokens.iter_mut()) {
                    if m.is_creature() && m.matches(filter, caster) {
                        until_end_of_turn(m, (*power, *toughness), gains);
                    }
                }
            }
            Effect::PlusCounters { slot, n } => {
                for id in objects(*slot) {
                    if let Some(m) = w.permanents.get_mut(&id) {
                        let n = i32::try_from(*n).expect("a few");
                        change(m, n, n);
                        m.counters.0 += n as u32;
                    }
                }
            }
            Effect::Fight { a, b } => {
                // Each deals damage equal to its power to the other, at
                // once; a creature with lifelink gains its controller that
                // much (CR 701.12a, 702.15b).
                let (Some(ia), Some(ib)) = (object(*a), object(*b)) else { continue };
                let (ma, mb) = (w.permanents[&ia].clone(), w.permanents[&ib].clone());
                let (pa, pb) = (w.pt_of(&ma).0, w.pt_of(&mb).0);
                let dealt = |p: Option<i32>| u32::try_from(p.unwrap_or(0).max(0)).expect("non-negative");
                w.permanents.get_mut(&ib).expect("on the board").damage += dealt(pa);
                w.permanents.get_mut(&ia).expect("on the board").damage += dealt(pb);
                for (m, p) in [(&ma, pa), (&mb, pb)] {
                    if has(m, Keyword::Lifelink) {
                        w.life[m.controller.0 as usize] += dealt(p) as i32;
                    }
                }
            }
            Effect::Destroy { slot } => {
                if let Some(id) = object(*slot) {
                    w.destroy(id);
                }
            }
            Effect::DestroyAll { filter } => {
                // At once (CR 701.8c): what is destroyed is decided on the
                // board as it stood.
                let doomed: Vec<ObjectId> = w.permanents.iter()
                    .filter(|(_, m)| m.matches(filter, caster))
                    .map(|(id, _)| *id)
                    .collect();
                destroyed = doomed.into_iter().filter(|id| w.destroy(*id)).count();
            }
            Effect::GainControl { slot, until_end_of_turn } => {
                if let Some(m) = object(*slot).and_then(|id| w.permanents.get_mut(&id)) {
                    if m.controller != caster {
                        if *until_end_of_turn {
                            m.stolen_from = Some(m.controller);
                        }
                        m.controller = caster;
                    }
                }
            }
            Effect::BecomesSubtype { slot, subtype } => {
                if let Some(m) = object(*slot).and_then(|id| w.permanents.get_mut(&id)) {
                    if !m.subtypes.contains(subtype) {
                        m.subtypes.push(subtype.clone());
                    }
                }
            }
            Effect::PlusCountersIfSubtype { slot, n, subtype, n_if } => {
                if let Some(m) = object(*slot).and_then(|id| w.permanents.get_mut(&id)) {
                    let k = i32::try_from(if m.subtypes.contains(subtype) { *n_if } else { *n }).expect("a few");
                    change(m, k, k);
                    m.counters.0 += k as u32;
                }
            }
            Effect::PumpPerCount { slot, power, toughness, filter } => {
                let k = i32::try_from(w.permanents.values().chain(&w.tokens).filter(|m| m.matches(filter, caster)).count()).expect("a few");
                if let Some(m) = object(*slot).and_then(|id| w.permanents.get_mut(&id)) {
                    until_end_of_turn(m, (power * k, toughness * k), &[]);
                }
            }
            Effect::AddMana(mana) => {
                for (t, n) in mana {
                    *w.pool[c].entry(*t).or_insert(0) += n;
                }
            }
            Effect::ExileWithSameName { slot } => {
                if let Some(name) = object(*slot).and_then(|id| w.permanents.get(&id)).and_then(|m| m.name.clone()) {
                    let doomed: Vec<ObjectId> = w.permanents.iter()
                        .filter(|(_, m)| m.is_creature() && m.name.as_ref() == Some(&name))
                        .map(|(id, _)| *id)
                        .collect();
                    for id in doomed {
                        w.leave(id, Dest::Exile);
                    }
                }
            }
            Effect::PlayerExilesFromGraveyard { .. } => {
                for id in &world.chosen {
                    if let Some(card) = w.leave_yard(*id) {
                        w.exile[card.owner.0 as usize].push(card.name.clone().expect("a card"));
                        w.cost_exiled.push(card);
                    }
                }
            }
            Effect::GainLifeIfExiledCreature { n } => {
                if w.cost_exiled.last().is_some_and(Model::is_creature) {
                    w.life[c] += n;
                }
            }
            Effect::PlayerDiscards { slot, n } => {
                if let Some(p) = player(*slot) {
                    assert!(w.hand[p].len() <= *n, "the reference discards a whole hand, not a choice from it");
                    let hand = std::mem::take(&mut w.hand[p]);
                    w.graveyard[p].extend(hand);
                }
            }
            Effect::BottomCardToGraveyard { slot } => {
                if let Some(p) = player(*slot) {
                    if let Some(card) = w.library[p].pop() {
                        w.graveyard[p].push(card);
                    }
                }
            }
            Effect::TokenIfCreatureCard(_) => {
                // The board's libraries hold only basic lands, none of them
                // a creature card.
                assert!(LIBRARY.iter().all(|n| BASIC_LANDS.contains(n)));
            }
            Effect::ExchangeLifeWithToughness { slot } => {
                if let Some(id) = object(*slot) {
                    let t = w.pt_of(&w.permanents[&id]).1.unwrap_or(0);
                    let life = w.life[c];
                    let m = w.permanents.get_mut(&id).expect("the source");
                    // The toughness it is set to is its base (CR 613.4b);
                    // what changes it on top of that still does.
                    let on_top = t - m.toughness.unwrap_or(0);
                    m.toughness = Some(life - on_top);
                    w.life[c] = t;
                }
            }
            Effect::DiscardAtRandom { .. } => {
                for name in &world.discarded {
                    let at = w.hand[c].iter().position(|h| h == name).expect("a card in hand");
                    let card = w.hand[c].remove(at);
                    w.graveyard[c].push(card);
                }
            }
            Effect::SearchForBasicLand { of, .. } => {
                let p = searcher(world, targets, *of, caster);
                if let Some((name, onto_battlefield)) = &world.found {
                    let at = w.library[p.0 as usize].iter().position(|n| n == name).expect("a card the search can find");
                    w.library[p.0 as usize].remove(at);
                    if *onto_battlefield {
                        w.tokens.push(basic_land(name, p));
                    } else {
                        w.hand[p.0 as usize].push(name.clone());
                    }
                }
                w.shuffled[p.0 as usize] = true;
            }
            Effect::ExileItFromGraveyard => {
                if let Some(card) = w.source.and_then(|id| w.leave_yard(id)) {
                    w.exile[card.owner.0 as usize].push(card.name.expect("a card"));
                }
            }
            Effect::ReturnAtRandom { to_battlefield, .. } => {
                for id in &world.chosen {
                    let Some(mut card) = w.leave_yard(*id) else { continue };
                    if *to_battlefield {
                        // A new object under its owner's control (CR 110.2).
                        card.controller = card.owner;
                        w.permanents.insert(*id, card);
                    } else {
                        w.hand[card.owner.0 as usize].push(card.name.expect("a card"));
                    }
                }
            }
            Effect::SacrificeOfTheirChoice { .. } => {
                if let Some(&victim) = world.chosen.first() {
                    w.refresh();
                    w.cost_sacrificed = w.permanents.get(&victim).cloned();
                    w.leave(victim, Dest::Graveyard);
                    w.a_creature_died = true;
                }
            }
            Effect::EachPlayerChoosesACreature => {}
            Effect::DestroyTheRest => {
                let doomed: Vec<ObjectId> = w.permanents.iter()
                    .filter(|(id, m)| m.is_creature() && !world.chosen.contains(id))
                    .map(|(id, _)| *id)
                    .collect();
                for id in doomed {
                    w.destroy(id);
                }
            }
            Effect::Regenerate { slot } => {
                if let Some(m) = object(*slot).and_then(|id| w.permanents.get_mut(&id)) {
                    m.shields += 1;
                }
            }
            Effect::TransformThis { slot } => {
                let id = object(*slot).expect("its source");
                if let Some(m) = w.permanents.get_mut(&id).filter(|m| world.faces_put.get(&id).is_none_or(|t| *t == m.transformed)) {
                    m.transform();
                }
            }
            Effect::RegenerateAll { filter } => {
                for m in w.permanents.values_mut() {
                    if m.is_creature() && m.matches(filter, caster) {
                        m.shields += 1;
                    }
                }
            }
            Effect::GainLifePerDestroyed { n } => w.life[c] += n * i32::try_from(destroyed).expect("a few"),
            Effect::GainLifePerCreatureCard { n } => {
                w.life[c] += n * i32::try_from(w.creature_cards_in_graveyard(caster)).expect("a few");
            }
            Effect::Exile { slot } => {
                if let Some(id) = object(*slot) {
                    if let Some(card) = w.leave_yard(id) {
                        w.exile[card.owner.0 as usize].push(card.name.expect("a card has a name"));
                    } else {
                        w.leave(id, Dest::Exile);
                    }
                }
            }
            Effect::Reanimate { slot, minus_counter } => {
                if let Some(mut m) = object(*slot).and_then(|id| w.leave_yard(id)) {
                    m.controller = caster;
                    if *minus_counter {
                        change(&mut m, -1, -1);
                        m.counters.1 += 1;
                    }
                    w.permanents.insert(object(*slot).expect("just read"), m);
                }
            }
            Effect::DamageEqualToExiledPower { slot } => {
                let n: u32 = w.cost_exiled.iter().map(|m| u32::try_from(m.power.unwrap_or(0).max(0)).expect("non-negative")).sum();
                if let Some(m) = object(*slot).and_then(|id| w.permanents.get_mut(&id)) {
                    m.damage += n;
                }
            }
            Effect::MillEach { n } => {
                for p in 0..2 {
                    for _ in 0..*n {
                        let card = w.library[p].remove(0);
                        w.graveyard[p].push(card);
                    }
                }
            }
            Effect::PlusCountersEach { filter } => {
                for m in w.permanents.values_mut().chain(w.tokens.iter_mut()) {
                    if m.is_creature() && m.matches(filter, caster) {
                        change(m, 1, 1);
                        m.counters.0 += 1;
                    }
                }
            }
            Effect::MillYou { n } => {
                for _ in 0..*n {
                    let card = w.library[c].remove(0);
                    w.graveyard[c].push(card);
                }
            }
            Effect::UntapAll { filter } => {
                for m in w.permanents.values_mut().chain(w.tokens.iter_mut()) {
                    if m.matches(filter, caster) {
                        m.tapped = false;
                    }
                }
            }
            Effect::Counter { slot, exile } => {
                if let Some(at) = object(*slot).and_then(|id| w.stack.iter().position(|(s, _)| *s == id)) {
                    let (_, m) = w.stack.remove(at);
                    let name = m.name.expect("a spell from a card");
                    if *exile { w.exile[m.owner.0 as usize].push(name) } else { w.graveyard[m.owner.0 as usize].push(name) }
                }
            }
            Effect::ToHand { slot } => {
                if let Some(id) = object(*slot) {
                    if let Some(card) = w.leave_yard(id) {
                        w.hand[card.owner.0 as usize].push(card.name.expect("a card has a name"));
                    } else {
                        w.leave(id, Dest::Hand);
                    }
                }
            }
            Effect::Attach { slot } => {
                if let (Some(src), Some(to)) = (w.source, object(*slot).filter(|t| w.permanents.contains_key(t))) {
                    if let Some(m) = w.permanents.get_mut(&src) {
                        m.attached = Some(Target::Object(to));
                    }
                }
            }
            Effect::GainLifeEqualToSacrificedToughness => {
                // Sacrificed for the cost, or by an earlier effect of this
                // one.
                if let Some(m) = w.cost_sacrificed.clone() {
                    w.life[c] += w.pt_of(&m).1.unwrap_or(0);
                }
            }
            Effect::DamageX { slot } => {
                if let Some(id) = object(*slot) {
                    w.permanents.get_mut(&id).expect("a target on the board").damage += world.x;
                } else if let Some(p) = player(*slot) {
                    w.life[p] -= world.x as i32;
                }
            }
            Effect::PumpX { slot, power, toughness, gains } => {
                let x = i32::try_from(world.x).expect("a few");
                if let Some(m) = object(*slot).and_then(|id| w.permanents.get_mut(&id)) {
                    until_end_of_turn(m, (power * x, toughness * x), gains);
                }
            }
            Effect::PlayerDraws { slot, n } => {
                if let Some(p) = player(*slot) {
                    for _ in 0..*n {
                        let card = w.library[p].remove(0);
                        w.hand[p].push(card);
                    }
                }
            }
            Effect::ExileFromGraveyard { slot, n } => {
                if let Some(p) = player(*slot) {
                    assert!(w.graveyard[p].len() <= *n, "the reference exiles a whole graveyard, not a choice from it");
                    let ids: Vec<ObjectId> = w.yard.iter().filter(|(_, m)| m.owner.0 as usize == p).map(|(id, _)| *id).collect();
                    for id in ids {
                        let card = w.leave_yard(id).expect("in the graveyard");
                        w.exile[p].push(card.name.expect("a card has a name"));
                    }
                    let rest = std::mem::take(&mut w.graveyard[p]);
                    w.exile[p].extend(rest);
                }
            }
            Effect::ToLibraryTop { slot } => {
                if let Some(id) = object(*slot) {
                    w.leave(id, Dest::LibraryTop);
                }
            }
            Effect::Tap { slot, tapped } => {
                for id in objects(*slot) {
                    if let Some(m) = w.permanents.get_mut(&id) {
                        m.tapped = *tapped;
                    }
                }
            }
            Effect::Draw { n } => {
                for _ in 0..*n {
                    let card = w.library[c].remove(0);
                    w.hand[c].push(card);
                }
            }
            Effect::GainLife { n } => w.life[c] += n,
            Effect::LoseLife { slot, n } => {
                if let Some(p) = player(*slot) {
                    w.life[p] -= n;
                }
            }
            Effect::Mill { slot, n } => {
                if let Some(p) = player(*slot) {
                    for _ in 0..*n {
                        let card = w.library[p].remove(0);
                        w.graveyard[p].push(card);
                    }
                }
            }
            Effect::Tokens { n, power, toughness, colors, subtypes, keywords, tapped, per_creature_card } => {
                let times = if *per_creature_card { *n * w.creature_cards_in_graveyard(caster) as usize } else { *n };
                for _ in 0..times {
                    w.tokens.push(Model { tapped: *tapped, ..creature_token((*power, *toughness), colors, subtypes, keywords, caster) });
                }
            }
            Effect::ControllerGainsLifeEqualToPower { slot } => {
                // The object is gone by now; its last known power and
                // controller are those it had on `world`, modified by
                // anything earlier in this resolution.
                let id = object(*slot).expect("an object slot");
                let last = w.permanents.get(&id).or_else(|| world.permanents.get(&id)).expect("on the board");
                let gain = world.pt_of(last).0.unwrap_or(0);
                w.life[last.controller.0 as usize] += gain;
            }
        }
    }
    w
}

/// A way to pay a spell's additional cost: the creature sacrificed, or the
/// cards exiled.
#[derive(Debug, Clone, PartialEq)]
enum Payment {
    Sacrifice(ObjectId),
    Exile(Vec<ObjectId>),
}

/// Every way `caster` can pay `spec`'s additional cost on `world` (CR
/// 601.2h): any creature they control, or any set of the right number of
/// creature cards in their graveyard. One empty payment when there is no
/// additional cost; none at all when it cannot be paid, and then the spell
/// cannot be cast (601.2h, 601.3).
fn payments(spec: &Spec, world: &World, caster: PlayerId) -> Vec<Option<Payment>> {
    match spec.additional {
        None => vec![None],
        Some(Additional::SacrificeCreature) => world.permanents.iter()
            .filter(|(_, m)| m.controller == caster && m.is_creature())
            .map(|(id, _)| Some(Payment::Sacrifice(*id)))
            .collect(),
        Some(Additional::ExileCreatureCards(n)) => {
            let cards: Vec<Target> = world.yard.iter()
                .filter(|(_, m)| m.owner == caster && m.is_creature())
                .map(|(id, _)| Target::Object(*id))
                .collect();
            subsets(&cards, n).into_iter()
                .filter(|s| s.len() == n)
                .map(|s| Some(Payment::Exile(s.iter().map(|t| match t { Target::Object(id) => *id, _ => unreachable!() }).collect())))
                .collect()
        }
        Some(Additional::ExileXCards) => {
            let cards: Vec<Target> = world.yard.iter()
                .filter(|(_, m)| m.owner == caster)
                .map(|(id, _)| Target::Object(*id))
                .collect();
            subsets(&cards, cards.len()).into_iter()
                .map(|s| Some(Payment::Exile(s.iter().map(|t| match t { Target::Object(id) => *id, _ => unreachable!() }).collect())))
                .collect()
        }
    }
}

/// `world` with `payment` made: a sacrificed creature put into its
/// owner's graveyard, as a creature dying (CR 701.21a, 700.4); exiled cards
/// moved to exile, remembered for what the spell says of them.
fn pay(world: &World, payment: &Option<Payment>) -> World {
    let mut w = world.clone();
    match payment {
        None => {}
        Some(Payment::Sacrifice(id)) => {
            w.leave(*id, Dest::Graveyard);
            w.a_creature_died = true;
        }
        Some(Payment::Exile(ids)) => {
            for id in ids {
                let m = w.leave_yard(*id).expect("a card in the graveyard");
                w.exile[m.owner.0 as usize].push(m.name.clone().expect("a card"));
                w.cost_exiled.push(m);
            }
            w.x = u32::try_from(ids.len()).expect("a few");
        }
    }
    w
}

/// Every way `spec`'s random effects can go on `world`, each as the world
/// to resolve it on: which cards the discard takes (the hand as the draws
/// before it leave it, told apart by name), and which cards come back from
/// the graveyard (CR 701.9b: "at random" is every one of them equally
/// likely). The world unchanged when it has none.
fn random_choices(spec: &Spec, world: &World, caster: PlayerId) -> Vec<World> {
    let c = caster.0 as usize;
    let mut ways = vec![world.clone()];
    let mut drawn = 0;
    for effect in &spec.effects {
        match effect {
            Effect::Draw { n } => drawn += n,
            Effect::DiscardAtRandom { n } => {
                assert_eq!(*n, 1, "the reference discards one card at random");
                let mut hand: Vec<String> = world.hand[c].iter().chain(world.library[c].iter().take(drawn)).cloned().collect();
                hand.sort();
                hand.dedup();
                ways = ways.into_iter().flat_map(|w| hand.iter().map(move |h| World { discarded: vec![h.clone()], ..w.clone() })).collect();
            }
            Effect::ReturnAtRandom { n, filter, .. } => {
                let exiled_first = spec.effects.iter().take_while(|e| *e != effect).any(|e| *e == Effect::ExileItFromGraveyard);
                let cards: Vec<Target> = world.yard.iter()
                    .filter(|(id, m)| m.owner == caster && filter.as_ref().is_none_or(|f| m.matches(f, caster))
                        && !(exiled_first && world.source == Some(**id)))
                    .map(|(id, _)| Target::Object(*id))
                    .collect();
                let k = (*n).min(cards.len());
                let picks: Vec<Vec<ObjectId>> = subsets(&cards, k).into_iter()
                    .filter(|p| p.len() == k)
                    .map(|p| p.iter().map(|t| match t { Target::Object(id) => *id, _ => unreachable!() }).collect())
                    .collect();
                ways = ways.into_iter().flat_map(|w| picks.iter().map(move |p| World { chosen: p.clone(), ..w.clone() })).collect();
            }
            _ => {}
        }
    }
    ways
}

/// Who searches: the caster, or the controller of the permanent in slot
/// `of` as it was before the effect (CR 608.2h).
fn searcher(world: &World, targets: &[Target], of: Option<usize>, caster: PlayerId) -> PlayerId {
    match of.map(|slot| &targets[slot]) {
        Some(Target::Object(id)) => world.permanents.get(id).map_or(caster, |m| m.controller),
        _ => caster,
    }
}

const BASIC_LANDS: [&str; 5] = ["Plains", "Island", "Swamp", "Mountain", "Forest"];

/// A basic land card put onto the battlefield, as its type line makes it.
fn basic_land(name: &str, owner: PlayerId) -> Model {
    let printed = Printed {
        name: name.to_string(), mana_cost: String::new(), type_line: format!("Basic Land — {name}"),
        oracle_text: String::new(), power: None, toughness: None, back: None, color_indicator: Vec::new(),
    };
    Model::printed(&printed, owner)
}

/// Every way `spec`'s searches can go on `world`: each basic land card in
/// the searcher's library, told apart by name, or nothing, a search for a
/// card of a stated quality being free to fail (CR 701.19b); to the
/// battlefield as well when morbid allows it.
fn search_choices(spec: &Spec, world: &World, targets: &[Target], caster: PlayerId) -> Vec<World> {
    let mut ways = vec![world.clone()];
    for effect in &spec.effects {
        let Effect::SearchForBasicLand { of, to_battlefield, morbid_battlefield } = effect else { continue };
        let p = searcher(world, targets, *of, caster);
        let mut lands: Vec<String> = world.library[p.0 as usize].iter().filter(|n| BASIC_LANDS.contains(&n.as_str())).cloned().collect();
        lands.sort();
        lands.dedup();
        let mut found: Vec<Option<(String, bool)>> = vec![None];
        for land in lands {
            found.push(Some((land.clone(), *to_battlefield)));
            if *morbid_battlefield && world.a_creature_died && !to_battlefield {
                found.push(Some((land, true)));
            }
        }
        ways = ways.into_iter().flat_map(|w| found.iter().map(move |f| World { found: f.clone(), ..w.clone() })).collect();
    }
    ways
}

/// Every way `spec` can resolve at `targets` on `world`, as the world to
/// apply it on: each way its random effects can go, each choice another
/// player makes, each card a search can find.
fn resolution_ways(spec: &Spec, world: &World, targets: &[Target], caster: PlayerId) -> Vec<World> {
    let mut ways = random_choices(spec, world, caster);
    if spec.effects.iter().any(|e| matches!(e, Effect::SacrificeOfTheirChoice { .. } | Effect::EachPlayerChoosesACreature | Effect::PlayerExilesFromGraveyard { .. })) {
        ways = ways.iter()
            .flat_map(|w| resolution_choices(spec, w, targets).into_iter().map(move |chosen| World { chosen, ..w.clone() }))
            .collect();
    }
    ways.iter().flat_map(|w| search_choices(spec, w, targets, caster)).collect()
}

/// Whether `spec` asks anything as it resolves: something random, a
/// choice another player makes, a search.
fn asks_as_it_resolves(spec: &Spec) -> bool {
    is_random(spec) || spec.effects.iter().any(|e| matches!(e,
        Effect::SacrificeOfTheirChoice { .. } | Effect::EachPlayerChoosesACreature | Effect::SearchForBasicLand { .. }
        | Effect::PlayerExilesFromGraveyard { .. }))
}

/// Whether `spec` does something at random.
fn is_random(spec: &Spec) -> bool {
    spec.effects.iter().any(|e| matches!(e, Effect::DiscardAtRandom { .. } | Effect::ReturnAtRandom { .. }))
}

/// Every ending `play` reaches from `state` with each of 32 seeds for the
/// engine's random choices; for a card that does something at random, the
/// union is held to every way the text allows, so a way no seed reaches is
/// a failure too. The seeds are fixed, so the check is too.
fn over_seeds(state: &GameState, play: impl Fn(&GameState) -> Result<Vec<GameState>, String>) -> Result<Vec<GameState>, String> {
    let mut all = Vec::new();
    for seed in 0..32u64 {
        let mut s = state.clone();
        s.rng_state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03;
        all.extend(play(&s)?);
    }
    Ok(all)
}

/// Whether `spec` does what lasts the turn and should last no longer:
/// damage, a pump, a tap, a shield (CR 514.2, 502.3). Its result is then
/// also played into the opponent's turn.
fn lasts_the_turn(spec: &Spec) -> bool {
    spec.effects.iter().any(|e| matches!(e,
        Effect::Damage { .. } | Effect::DamageEach { .. } | Effect::DamageX { .. } | Effect::Pump { .. } | Effect::PumpEach { .. }
        | Effect::PumpX { .. } | Effect::Fight { .. } | Effect::Tap { .. } | Effect::Regenerate { .. } | Effect::RegenerateAll { .. }
        | Effect::GainControl { .. } | Effect::PumpPerCount { .. }))
}

/// Every way the players can make the choices `spec` asks for as it
/// resolves at `targets`: each creature the player sacrificing controls,
/// or one creature of each player's who has any, in turn order. One empty
/// way when it asks for none.
fn resolution_choices(spec: &Spec, world: &World, targets: &[Target]) -> Vec<Vec<ObjectId>> {
    let of = |p: PlayerId| -> Vec<ObjectId> {
        world.permanents.iter().filter(|(_, m)| m.controller == p && m.is_creature()).map(|(id, _)| *id).collect()
    };
    let mut ways = vec![Vec::new()];
    for effect in &spec.effects {
        let options: Vec<Vec<ObjectId>> = match effect {
            Effect::SacrificeOfTheirChoice { slot } => match &targets[*slot] {
                Target::Player(p) => of(*p).into_iter().map(|c| vec![c]).collect(),
                _ => continue,
            },
            Effect::PlayerExilesFromGraveyard { slot } => match &targets[*slot] {
                Target::Player(p) => world.yard.iter().filter(|(_, m)| m.owner == *p).map(|(id, _)| vec![*id]).collect(),
                _ => continue,
            },
            Effect::EachPlayerChoosesACreature => {
                let mut picks = vec![Vec::new()];
                for p in [P0, P1] {
                    let theirs = of(p);
                    if !theirs.is_empty() {
                        picks = picks.into_iter().flat_map(|k: Vec<ObjectId>| theirs.iter().map(move |c| [k.clone(), vec![*c]].concat())).collect();
                    }
                }
                picks
            }
            _ => continue,
        };
        if options.is_empty() {
            continue;
        }
        ways = ways.into_iter().flat_map(|w| options.iter().map(move |o| [w.clone(), o.clone()].concat())).collect();
    }
    ways
}

/// What resolving the instant or sorcery `card`, read as `spec`, with
/// `targets` does to `world`: its effects, state-based actions (CR 704.3),
/// and the card put into its owner's graveyard (CR 608.2n).
fn resolve(spec: &Spec, world: &World, targets: &[Target], caster: PlayerId, card: &Printed, id: ObjectId) -> World {
    let mut w = apply(spec, world, targets, caster);
    if spec.permanent {
        // An enchantment: it enters, an Aura attached to what its spell
        // targeted (CR 303.4f), and its static abilities apply from then on.
        let attached = spec.enchants.then(|| targets[0].clone());
        w.permanents.insert(id, Model { attached, statics: spec.statics.clone(), ..Model::printed(card, caster) });
        // Its enters trigger, on the board it entered (CR 603.6a); one with
        // no choices, as the Auras read here have.
        if let Some(trigger) = &spec.etb {
            w.source = Some(id);
            w.state_based_actions();
            let [after] = &trigger_outcomes(trigger, &w, caster)[..] else { panic!("an Aura's enters trigger with choices") };
            w = after.clone();
        }
    }
    w.state_based_actions();
    if !spec.permanent {
        w.graveyard[caster.0 as usize].push(card.name.clone());
    }
    assert!(w.stack.is_empty(), "the reference resolves only what it counters; {:?} is left", w.stack);
    w
}

/// Every way the creature spell `card`, read as `spec`, can end once cast:
/// it becomes a permanent with its printed characteristics (CR 608.3),
/// tapped or with counters as its text says, and its enters trigger, if
/// any, goes on the stack with each set of targets it allows and resolves,
/// each way its controller may choose. No legal target and the trigger is
/// removed (CR 603.3d); a false intervening "if" and it does nothing
/// (603.4).
fn creature_outcomes(spec: &Spec, world: &World, caster: PlayerId, card: &Printed, id: ObjectId) -> Vec<World> {
    let mut w = world.clone();
    let mut m = Model { tapped: spec.enters_tapped, ..model_of(spec, card, caster) };
    if let (Some(n), true) = (spec.enters_with_counters_if_morbid, world.a_creature_died) {
        let n = i32::try_from(n).expect("a few");
        change(&mut m, n, n);
        m.counters.0 += n as u32;
    }
    w.permanents.insert(id, m);
    w.source = Some(id);
    w.state_based_actions();
    match &spec.etb {
        Some(trigger) if !(trigger.if_morbid && !w.a_creature_died) => trigger_outcomes(trigger, &w, caster),
        _ => vec![w],
    }
}

/// The permanent `card`, read as `spec`, under `controller`: its printed
/// characteristics and what its text says it is.
fn model_of(spec: &Spec, card: &Printed, controller: PlayerId) -> Model {
    Model {
        controller,
        counts_creatures: spec.pt_is_creatures_you_control,
        counts_creature_cards: spec.pt_is_creature_cards,
        counts_hand: spec.pt_is_cards_in_hand,
        combat: spec.combat.clone(),
        statics: spec.statics.clone(),
        other_face: Model::back_face(card, controller),
        ..Model::printed(card, controller)
    }
}

/// Every way the turn of `active` can begin on `world`, where `id` is the
/// permanent read as `spec`: the cleanup step of the turn before removes
/// damage (CR 514.2) and the turn's morbid (700.4); the untap step untaps
/// the active player's permanents (502.3); the upkeep trigger, if its text
/// says this is its upkeep, resolves each way it can (503.1a, 603.3); and
/// the active player draws (504.1). The cleanup also ends "until end of
/// turn" effects and regeneration shields, and mana empties between steps
/// (514.2, 500.4).
fn next_turn_outcomes(spec: &Spec, world: &World, id: ObjectId, active: PlayerId) -> Vec<World> {
    let mut w = world.clone();
    for m in w.permanents.values_mut().chain(w.tokens.iter_mut()) {
        m.damage = 0;
        m.deathtouched = false;
        m.temp = (0, 0);
        m.temp_keywords.clear();
        m.shields = 0;
        if let Some(owner) = m.stolen_from.take() {
            m.controller = owner;
        }
        if m.controller == active && !restricted(m, Restriction::DoesntUntap) {
            m.tapped = false;
        }
    }
    // Mana empties as each step ends (CR 500.4).
    w.pool = [BTreeMap::new(), BTreeMap::new()];
    w.a_creature_died = false;
    w.source = Some(id);
    let me = w.permanents.get(&id);
    let controller = me.map_or(P0, |m| m.controller);
    let whose = match spec.upkeep.as_deref() {
        Some(Upkeep { enchanted_player: true, .. }) => match me.and_then(|m| m.attached.clone()) {
            Some(Target::Player(p)) => Some(p),
            _ => None,
        },
        Some(_) => Some(controller),
        None => None,
    };
    let worlds = match spec.upkeep.as_deref() {
        Some(upkeep) if whose == Some(active) => triggers_outcomes(&[&upkeep.trigger], &w, controller),
        _ => vec![w],
    };
    worlds.into_iter()
        .map(|mut w| {
            let a = active.0 as usize;
            let card = w.library[a].remove(0);
            w.hand[a].push(card);
            w
        })
        .collect()
}

/// Every way `trigger`, controlled by `caster`, can resolve on `world`:
/// each set of targets it allows, and declining when it says "you may". No
/// legal target and it is removed from the stack (CR 603.3d).
fn trigger_outcomes(trigger: &Trigger, world: &World, caster: PlayerId) -> Vec<World> {
    let sets = target_sets(&trigger.spec, world, caster);
    if sets.is_empty() {
        return vec![world.clone()];
    }
    let put = world.with_faces_put();
    let mut out = Vec::new();
    for targets in sets {
        for way in resolution_ways(&trigger.spec, &put, &targets, caster) {
            let mut after = apply(&trigger.spec, &way, &targets, caster);
            after.state_based_actions();
            out.push(after);
        }
        if trigger.optional {
            out.push(world.clone());
        }
    }
    out.into_iter().map(|mut w| { w.faces_put.clear(); w }).collect()
}

/// Every way `triggers`, controlled by `caster` and set off at once, can
/// resolve on `world`: put on the stack in each order (CR 603.3b), each
/// with every set of targets it allows chosen as it goes there, or removed
/// if it has none (603.3d), and resolved last first, state-based actions
/// after each (704.3); each "you may" taken or declined. Their targets are
/// players or their own source, which stay legal.
fn triggers_outcomes(triggers: &[&Trigger], world: &World, caster: PlayerId) -> Vec<World> {
    if let [trigger] = triggers {
        return trigger_outcomes(trigger, world, caster);
    }
    assert!(triggers.iter().all(|t| t.spec.slots.iter().all(|s| matches!(s, Slot::Player | Slot::Opponent | Slot::PlayerOrPlaneswalker | Slot::This))),
        "simultaneous triggers here target players or their source");
    let mut out = Vec::new();
    for order in permutations(triggers.len()) {
        let mut choices: Vec<Vec<Option<Vec<Target>>>> = vec![Vec::new()];
        for &i in &order {
            let sets = target_sets(&triggers[i].spec, world, caster);
            let options: Vec<Option<Vec<Target>>> = if sets.is_empty() { vec![None] } else { sets.into_iter().map(Some).collect() };
            choices = choices.into_iter()
                .flat_map(|c| options.iter().map(move |o| [c.clone(), vec![o.clone()]].concat()))
                .collect();
        }
        for chosen in choices {
            let mut worlds = vec![world.with_faces_put()];
            for (k, &i) in order.iter().enumerate().rev() {
                let Some(targets) = &chosen[k] else { continue };
                let trigger = triggers[i];
                worlds = worlds.into_iter()
                    .flat_map(|w| {
                        let mut after = apply(&trigger.spec, &w, targets, caster);
                        after.state_based_actions();
                        if trigger.optional { vec![after, w] } else { vec![after] }
                    })
                    .collect();
            }
            out.extend(worlds);
        }
    }
    out.into_iter().map(|mut w| { w.faces_put.clear(); w }).collect()
}

/// Every way the creatures `dying` dying at once on `world` can end, where
/// `watcher` is the creature read as `spec`: lethal damage, state-based
/// actions put them into their owners' graveyards (CR 704.5g), and the
/// triggers those deaths set off resolve each way they can: the watcher's
/// dies trigger when it is one of them, and its "whenever ... dies" for
/// each that, as it last existed on the battlefield, is one its text names,
/// the watcher having been there to see it (CR 603.6c, 603.10a). With
/// `may`, each "you may" is taken, or each declined.
fn death_outcomes(spec: &Spec, world: &World, caster: PlayerId, watcher: ObjectId, dying: &[ObjectId], may: Option<bool>) -> Vec<World> {
    let mut w = world.clone();
    w.refresh();
    for id in dying {
        let lethal = w.pt_of(&w.permanents[id]).1.expect("a creature").max(1);
        w.permanents.get_mut(id).expect("the creature is on the battlefield").damage = u32::try_from(lethal).expect("positive");
    }
    let before = w.clone();
    w.state_based_actions();
    assert!(dying.iter().all(|id| !w.permanents.contains_key(id)), "lethal damage kills them");
    let mut triggers = Vec::new();
    // Its text is its front face's, which a transformed permanent does not
    // have (CR 712.8e).
    let front_up = before.permanents.get(&watcher).is_some_and(|m| !m.transformed);
    for (id, last) in before.permanents.iter().filter(|(id, m)| m.is_creature() && !w.permanents.contains_key(id)) {
        if *id == watcher && front_up {
            triggers.extend(spec.dies.as_deref());
        }
        if let Some(watch) = spec.watch.as_ref().filter(|_| front_up) {
            if last.matches_from(&watch.dying, caster, *id == watcher) {
                triggers.push(&watch.trigger);
            }
        }
    }
    // Every "you may" taken, or every one declined, when `may` says so.
    let taken: Vec<Trigger> = triggers.iter()
        .filter(|t| !t.optional || may != Some(false))
        .map(|t| Trigger { optional: t.optional && may.is_none(), ..(*t).clone() })
        .collect();
    triggers_outcomes(&taken.iter().collect::<Vec<_>>(), &w, caster)
}

// ---- the engine side ----

/// The colorless mana floated for an {X}: every X from 0 to it is tried,
/// each paid from the pool alone, so no question of which source to tap
/// arises.
const X_FLOAT: u32 = 3;

/// Mana for exactly `cost` in `player`'s pool: each coloured pip of its
/// colour, generic as colorless, less `cheaper` generic, and `X_FLOAT`
/// colorless for an {X}.
fn fund(state: &mut GameState, player: PlayerId, cost: &ManaCost, cheaper: u32) {
    let pool = &mut state.get_player_mut(player).mana_pool;
    for sym in &cost.symbols {
        match sym {
            ManaSymbol::Colored(c) => pool.add(ManaType::from(*c), 1),
            ManaSymbol::Generic(n) => pool.add(ManaType::Colorless, n.saturating_sub(cheaper)),
            ManaSymbol::X => pool.add(ManaType::Colorless, X_FLOAT),
            other => panic!("no card read here costs {other:?}"),
        }
    }
}

/// The target sets the engine offers for casting `spell`, sorted: what
/// its row in the legal actions says (CR 601.2c), and for a spell that
/// asks at cast time, every path through the questions it asks.
fn offered(state: &GameState, registry: &CardRegistry, spell: ObjectId) -> Result<Vec<Vec<Target>>, String> {
    let legal = engine::legal_actions(state, registry);
    let Some(row) = legal.castable_spells.iter().find(|c| c.object_id == spell && !c.is_flashback) else {
        return Ok(Vec::new());
    };
    let mut sets = match &row.target_spec {
        CastTargetSpec::NoTargets => vec![Vec::new()],
        CastTargetSpec::SingleTarget(options) => options.iter().map(|t| vec![t.clone()]).collect(),
        CastTargetSpec::TwoTargets { first, second, second_min: 1, second_max: 1 } => first.iter().zip(second)
            .flat_map(|(a, bs)| bs.iter().map(move |b| vec![a.clone(), b.clone()]))
            .collect(),
        CastTargetSpec::TwoTargets { .. } => return Err("an \"up to\" second slot, which no template reads".to_string()),
        CastTargetSpec::ChosenAtCast => {
            let asked = engine::submit_action(state, &cast_action(spell, Vec::new()), registry);
            asked_at_cast(&asked, registry)?
        }
    };
    sets.sort_by_key(|s| s.iter().map(target_key).collect::<Vec<_>>());
    sets.dedup();
    Ok(sets)
}

/// Every target set reachable by answering the cast-time target questions
/// as a player would: one target at a time for a single slot, and any set
/// the question allows for an "up to" slot.
fn asked_at_cast(state: &GameState, registry: &CardRegistry) -> Result<Vec<Vec<Target>>, String> {
    match &state.awaiting_action {
        None => Ok(vec![Vec::new()]),
        Some(AwaitingAction::ResolutionChoice { choice: ResolutionChoiceKind::ChooseTargetSet { options, min, max, by_count, .. }, .. })
            if by_count.is_empty() =>
        {
            let mut sorted = options.clone();
            sorted.sort_by_key(target_key);
            let mut sets = Vec::new();
            for answer in subsets(&sorted, *max).into_iter().filter(|s| s.len() >= *min) {
                let next = engine::submit_action(state, &Action::ResolveChoice {
                    choice: ResolvedChoice::ChosenTargetSet(answer.clone()),
                }, registry);
                for rest in asked_at_cast(&next, registry)? {
                    sets.push([answer.clone(), rest].concat());
                }
            }
            Ok(sets)
        }
        Some(other) => Err(format!("the cast asks {other:?}, which no template reads")),
    }
}

/// An answer to a question the engine asks after the cast: a target for a
/// trigger or a "you may" (`None` declines), or yes or no.
#[derive(Debug, Clone)]
enum Answer {
    Target(Option<Target>),
    YesNo(bool),
    /// An option by its place, as a combat damage assignment is answered.
    Index(usize, String),
    /// The cards exiled to pay a cost.
    Exile(Vec<ObjectId>),
    /// The order simultaneous triggers go on the stack, first to last.
    Order(Vec<usize>),
    /// The value of X, paid from floating colorless mana.
    X(u32),
    /// The card a search finds, or none (CR 701.19b).
    Found(Option<ObjectId>),
}

/// A combat to declare: who attacks whom, and who blocks whom.
type Combat = (Vec<(ObjectId, PlayerId)>, Vec<(ObjectId, ObjectId)>);

/// How far a scripted game is played.
#[derive(Clone, Copy)]
enum Scene<'a> {
    /// Until the stack is empty in the step it stands in.
    Main,
    /// From the main phase through a combat, declared as given, to the
    /// main phase after.
    Combat(&'a Combat),
    /// On through the end of the turn into a step of the next, the stack
    /// empty there; no one attacks.
    Until(Step),
    /// The same, for a step the turn it starts in also has.
    NextTurn(Step),
}

/// Where a scripted game stopped.
enum Played {
    /// The stack emptied: the game at rest.
    Done(Box<GameState>),
    /// The engine asked a question the script had no answer left for:
    /// every answer it allows.
    Asks(Vec<Answer>),
}

/// Cast `spell` at `targets` and play on until the stack is empty, both
/// players passing priority (CR 117.4, 608.1): the engine's own game loop,
/// state-based actions and triggers and all. The questions asked after the
/// cast are answered from `script`, in order.
fn play(state: &GameState, registry: &CardRegistry, spell: Option<ObjectId>, targets: Vec<Target>, script: &[Answer]) -> Result<Played, String> {
    play_from(state, registry, spell, targets, None, Scene::Main, script)
}

/// [`play`], starting with `first` instead of a cast when it is given: an
/// activation, as the engine offered it.
/// It is played as far as `scene` says.
fn play_from(state: &GameState, registry: &CardRegistry, spell: Option<ObjectId>, targets: Vec<Target>, first: Option<Action>, scene: Scene, script: &[Answer]) -> Result<Played, String> {
    let mut state = state.clone();
    // A spell that asks for its targets at cast time is cast bare, and each
    // question answered in turn as a player answers them: one target for a
    // single slot, the whole set for an "up to" slot.
    let asks = engine::legal_actions(&state, registry).castable_spells.iter()
        .any(|c| Some(c.object_id) == spell && matches!(c.target_spec, CastTargetSpec::ChosenAtCast));
    let mut answers: std::collections::VecDeque<Target> = if asks { targets.iter().cloned().collect() } else { Default::default() };
    // Until the cast's own questions are answered; an "up to" set may be
    // empty and is still an answer.
    let mut answering_cast = asks;
    // No spell: the game is played on from where it stands, as after a
    // death, which the loop handles before it asks anyone anything.
    let mut cast = first.or_else(|| spell.map(|spell| cast_action(spell, if asks { Vec::new() } else { targets })));
    let mut script = script.iter();
    let mut trouble = None;
    let mut unanswered = None;
    let mut decisions = 0;
    let turn = state.turn_number;
    let step = state.step;
    engine::run_game_loop(&mut state, registry, |s: &GameState, _acting: PlayerId, legal: &engine::LegalActions| {
        decisions += 1;
        if let Some(c) = cast.take() {
            return c;
        }
        // In the main phase: a player with nothing left to do is passed for,
        // and the game goes on; the first question in a later step means
        // the stack emptied where it started.
        if matches!(scene, Scene::Main) && (s.step != step || s.turn_number != turn) && s.stack.is_empty() {
            return Action::AbandonGame;
        }
        if decisions > 40 {
            trouble = Some("the game did not come to rest".to_string());
            return Action::AbandonGame;
        }
        if let Some(prompt) = &legal.combat_prompt {
            let (attackers, blocks) = match scene {
                Scene::Combat((attackers, blocks)) => (attackers.clone(), blocks.clone()),
                _ => (Vec::new(), Vec::new()),
            };
            return match prompt {
                CombatPrompt::ChooseAttackers { .. } => Action::DeclareAttackers { attackers, planeswalker_attacks: Vec::new() },
                CombatPrompt::ChooseBlockers { .. } => Action::DeclareBlockers { assignments: blocks },
            };
        }
        match &legal.resolution_prompt {
            Some(ResolutionChoiceKind::ChooseTargetSet { max, .. }) if answering_cast => {
                let take = if *max == 1 { 1.min(answers.len()) } else { answers.len() };
                let chosen: Vec<Target> = answers.drain(..take).collect();
                answering_cast = !answers.is_empty();
                return Action::ResolveChoice { choice: ResolvedChoice::ChosenTargetSet(chosen) };
            }
            Some(ResolutionChoiceKind::ChooseTarget { options, optional, .. }) => {
                return match script.next() {
                    Some(Answer::Target(t)) => Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(t.clone()) },
                    Some(other) => {
                        trouble = Some(format!("the script answers {other:?} to a target question"));
                        Action::AbandonGame
                    }
                    None => {
                        let mut all: Vec<Answer> = options.iter().map(|t| Answer::Target(Some(t.clone()))).collect();
                        if *optional {
                            all.push(Answer::Target(None));
                        }
                        unanswered = Some(all);
                        Action::AbandonGame
                    }
                };
            }
            Some(ResolutionChoiceKind::YesNo { .. }) => {
                return match script.next() {
                    Some(Answer::YesNo(b)) => Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(*b) },
                    Some(other) => {
                        trouble = Some(format!("the script answers {other:?} to a yes-or-no question"));
                        Action::AbandonGame
                    }
                    None => {
                        unanswered = Some(vec![Answer::YesNo(true), Answer::YesNo(false)]);
                        Action::AbandonGame
                    }
                };
            }
            Some(ResolutionChoiceKind::ChooseExileFromGraveyard { options, min, max, .. }) => {
                return match script.next() {
                    Some(Answer::Exile(ids)) => Action::ResolveChoice { choice: ResolvedChoice::ChosenExileSet(ids.clone()) },
                    Some(other) => {
                        trouble = Some(format!("the script answers {other:?} to an exile cost"));
                        Action::AbandonGame
                    }
                    None => {
                        let as_targets: Vec<Target> = options.iter().map(|id| Target::Object(*id)).collect();
                        unanswered = Some(subsets(&as_targets, *max).into_iter()
                            .filter(|s| s.len() >= *min)
                            .map(|s| Answer::Exile(s.iter().map(|t| match t { Target::Object(id) => *id, _ => unreachable!() }).collect()))
                            .collect());
                        Action::AbandonGame
                    }
                };
            }
            Some(ResolutionChoiceKind::ChooseFromLibrary { options, .. }) => {
                return match script.next() {
                    Some(Answer::Found(Some(id))) => Action::ResolveChoice { choice: ResolvedChoice::ChosenCard(*id) },
                    Some(Answer::Found(None)) => Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(None) },
                    Some(other) => {
                        trouble = Some(format!("the script answers {other:?} to a search"));
                        Action::AbandonGame
                    }
                    None => {
                        unanswered = Some(options.iter().map(|id| Answer::Found(Some(*id))).chain(std::iter::once(Answer::Found(None))).collect());
                        Action::AbandonGame
                    }
                };
            }
            Some(ResolutionChoiceKind::ChooseXFunding { options, .. }) => {
                return match script.next() {
                    Some(Answer::X(x)) => {
                        let pool = if *x == 0 { BTreeMap::new() } else { BTreeMap::from([(ManaType::Colorless, *x)]) };
                        Action::ResolveChoice { choice: ResolvedChoice::XFunding(mtg_engine::funding::FundingResponse { pool, taps: BTreeMap::new() }) }
                    }
                    Some(other) => {
                        trouble = Some(format!("the script answers {other:?} to choosing X"));
                        Action::AbandonGame
                    }
                    None => {
                        let floating = options.pool.get(&ManaType::Colorless).copied().unwrap_or(0);
                        unanswered = Some((0..=floating).map(Answer::X).collect());
                        Action::AbandonGame
                    }
                };
            }
            Some(ResolutionChoiceKind::ChooseTriggerOrder { options, .. }) => {
                return match script.next() {
                    Some(Answer::Order(order)) => Action::ResolveChoice { choice: ResolvedChoice::ChosenOrder(order.clone()) },
                    Some(other) => {
                        trouble = Some(format!("the script answers {other:?} to ordering triggers"));
                        Action::AbandonGame
                    }
                    None => {
                        unanswered = Some(permutations(options.len()).into_iter().map(Answer::Order).collect());
                        Action::AbandonGame
                    }
                };
            }
            Some(ResolutionChoiceKind::AssignCombatDamage { options, .. }) => {
                return match script.next() {
                    Some(Answer::Index(i, label)) => Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(*i, label.clone()) },
                    Some(other) => {
                        trouble = Some(format!("the script answers {other:?} to a damage assignment"));
                        Action::AbandonGame
                    }
                    None => {
                        unanswered = Some(options.iter().enumerate().map(|(i, l)| Answer::Index(i, l.clone())).collect());
                        Action::AbandonGame
                    }
                };
            }
            Some(prompt) => {
                trouble = Some(format!("the engine asked {prompt:?}, which the text does not"));
                return Action::AbandonGame;
            }
            None => {}
        }
        let at_rest = s.stack.is_empty() && match scene {
            Scene::Main => true,
            Scene::Combat(_) => s.step == Step::PostcombatMain,
            Scene::Until(step) => s.step == step,
            Scene::NextTurn(step) => s.turn_number != turn && s.step == step,
        };
        if at_rest {
            return Action::AbandonGame;
        }
        Action::PassPriority
    });
    if let Some(why) = trouble {
        return Err(why);
    }
    if let Some(all) = unanswered {
        return Ok(Played::Asks(all));
    }
    if spell.is_some_and(|spell| state.objects.get(&spell).is_some_and(|o| o.zone == Zone::Hand)) {
        return Err("the cast was refused".to_string());
    }
    Ok(Played::Done(Box::new(state)))
}

/// Cast `spell` at `targets` and let it resolve, where the text asks
/// nothing more of anyone.
fn cast_and_let_resolve(state: &GameState, registry: &CardRegistry, spell: ObjectId, targets: Vec<Target>) -> Result<GameState, String> {
    match play(state, registry, Some(spell), targets, &[])? {
        Played::Done(state) => Ok(*state),
        Played::Asks(options) => Err(format!("the engine asked a question the text does not, answerable {options:?}")),
    }
}

/// Every game the cast can end in: each question asked after it answered
/// every way the engine allows, replayed from the start for each branch.
fn every_ending(state: &GameState, registry: &CardRegistry, spell: Option<ObjectId>) -> Result<Vec<GameState>, String> {
    every_ending_from(state, registry, spell, None)
}

/// [`every_ending`] for `spell` cast at `targets`.
fn every_ending_at(state: &GameState, registry: &CardRegistry, spell: ObjectId, targets: &[Target]) -> Result<Vec<GameState>, String> {
    let mut todo: Vec<Vec<Answer>> = vec![Vec::new()];
    let mut done = Vec::new();
    while let Some(script) = todo.pop() {
        if done.len() + todo.len() > 500 {
            return Err("more than 500 ways to answer".to_string());
        }
        match play_from(state, registry, Some(spell), targets.to_vec(), None, Scene::Main, &script)? {
            Played::Done(s) => done.push(*s),
            Played::Asks(options) => todo.extend(options.into_iter().map(|o| [script.clone(), vec![o]].concat())),
        }
    }
    Ok(done)
}

/// [`every_ending`], starting with `first` when it is given.
fn every_ending_from(state: &GameState, registry: &CardRegistry, spell: Option<ObjectId>, first: Option<Action>) -> Result<Vec<GameState>, String> {
    every_ending_choosing(state, registry, spell, first, None)
}

/// [`every_ending_from`], with every yes-or-no question answered `may`
/// when it is given: one strategy, rather than every one, for the "you
/// may"s, whose endings as a set can hide a wrong answer to one behind a
/// right answer to another.
fn every_ending_choosing(state: &GameState, registry: &CardRegistry, spell: Option<ObjectId>, first: Option<Action>, may: Option<bool>) -> Result<Vec<GameState>, String> {
    let mut todo: Vec<Vec<Answer>> = vec![Vec::new()];
    let mut done = Vec::new();
    while let Some(script) = todo.pop() {
        if done.len() + todo.len() > 500 {
            return Err("more than 500 ways to answer".to_string());
        }
        match play_from(state, registry, spell, Vec::new(), first.clone(), Scene::Main, &script)? {
            Played::Done(s) => done.push(*s),
            Played::Asks(options) => todo.extend(options.into_iter()
                .filter(|o| !matches!((o, may), (Answer::YesNo(a), Some(m)) if *a != m))
                .map(|o| [script.clone(), vec![o]].concat())),
        }
    }
    Ok(done)
}

/// Every card the grammar reads, with what it read.
fn readable(registry: &CardRegistry, cache: &BTreeMap<String, Printed>) -> (Vec<(Printed, Spec)>, Vec<(String, String)>) {
    let mut read_ok = Vec::new();
    let mut unread = Vec::new();
    for name in registry.all_names() {
        let Some(card) = cache.get(name) else { continue };
        let reading = if card.has_type("Instant") || card.has_type("Sorcery") {
            read(card)
        } else if card.has_type("Creature") && !card.type_line.contains("//") {
            read_creature(card)
        } else if card.has_type("Enchantment") {
            read_enchantment(card)
        } else if card.has_type("Land") {
            read_land(card)
        } else if card.has_type("Artifact") {
            read_artifact(card)
        } else {
            continue;
        };
        match reading {
            Ok(spec) if mana_cost(&card.mana_cost).is_some() => read_ok.push((card.clone(), spec)),
            Ok(_) => unread.push((name.to_string(), format!("cannot read the cost {}", card.mana_cost))),
            Err(why) => unread.push((name.to_string(), why)),
        }
    }
    read_ok.sort_by(|a, b| a.0.name.cmp(&b.0.name));
    unread.sort();
    (read_ok, unread)
}

/// Every card the grammar read when this was last updated. A change that
/// reads more adds to it; one that stops reading any of these fails here
/// and names it, so coverage is never lost without someone deciding to
/// lose it.
const READ: &[&str] = &[
    "Abbey Griffin", "Altar's Reap", "Ambush Viper", "Ancient Grudge",
    "Angel of Flight Alabaster", "Angelic Overseer", "Armored Skaab", "Army of the Damned",
    "Ashmouth Hound", "Avacyn's Pilgrim", "Avacynian Priest", "Balefire Dragon",
    "Battleground Geist", "Blasphemous Act", "Bloodcrazed Neonate", "Bloodgift Demon",
    "Bonds of Faith", "Boneyard Wurm", "Brain Weevil", "Bramblecrush", "Brimstone Volley",
    "Bump in the Night", "Butcher's Cleaver", "Caravan Vigil", "Cellar Door", "Chapel Geist",
    "Claustrophobia", "Clifftop Retreat", "Cobbled Wings", "Coral Merfolk", "Corpse Lunge",
    "Counterspell", "Curse of Death's Hold", "Curse of Oblivion", "Curse of Stalked Prey",
    "Curse of the Bloody Tome", "Curse of the Nightly Hunt", "Curse of the Pierced Heart",
    "Darkthicket Wolf", "Dead Weight", "Demonmail Hauberk", "Deranged Assistant",
    "Desperate Ravings", "Devil's Play", "Diregraf Ghoul", "Disciple of Griselbrand",
    "Dissipate", "Divination", "Divine Reckoning", "Doom Blade", "Doomed Traveler",
    "Dream Twist", "Elder Cathar", "Elder of Laurels", "Elite Inquisitor",
    "Falkenrath Marauders", "Falkenrath Noble", "Feeling of Dread", "Feral Ridgewolf",
    "Festerhide Boar", "Forest", "Fortress Crab", "Full Moon's Rise", "Furor of the Bitten",
    "Gallows Warden", "Gatstaf Shepherd", "Gavony Township", "Geist of Saint Traft",
    "Geist-Honored Monk", "Geistcatcher's Rig", "Geistflame", "Ghost Quarter",
    "Ghostly Possession", "Ghoulcaller's Bell", "Ghoulraiser", "Giant Growth",
    "Glorious Anthem", "Gnaw to the Bone", "Goblin Piker", "Grasp of Phantoms", "Grave Bramble",
    "Graveyard Shovel", "Grizzled Outcasts", "Grizzly Bears", "Gruesome Deformity",
    "Hanweir Watchkeep", "Harvest Pyre", "Hinterland Harbor", "Hollowhenge Scavenger",
    "Holy Strength", "Hysterical Blindness", "Infernal Plunge", "Instigator Gang",
    "Intangible Virtue", "Into the Maw of Hell", "Invisible Stalker", "Island",
    "Isolated Chapel", "Kalonian Tusker", "Kessig Cagebreakers", "Kessig Wolf",
    "Kessig Wolf Run", "Kindercatch", "Kruin Outlaw", "Lantern Spirit", "Lava Axe",
    "Lightning Bolt", "Lost in the Mist", "Lumberknot", "Make a Wish", "Makeshift Mauler",
    "Manor Skeleton", "Markov Patrician", "Mask of Avacyn", "Mausoleum Guard",
    "Maw of the Mire", "Midnight Haunting", "Moan of the Unhallowed", "Moldgraf Monstrosity",
    "Moment of Heroism", "Moon Heron", "Moorland Haunt", "Morkrut Banshee", "Mountain",
    "Naturalize", "Nephalia Drownyard", "Night Revelers", "Olivia Voldaren",
    "One-Eyed Scarecrow", "Orchard Spirit", "Pacifism", "Paraselene", "Pitchburn Devils",
    "Plains", "Prey Upon", "Purify the Grave", "Rage Thrower", "Rakish Heir",
    "Rally the Peasants", "Ranger's Guile", "Rebuke", "Reckless Waif", "Riot Devils",
    "Rolling Temblor", "Rotting Fensnake", "Runechanter's Pike", "Savannah Lions",
    "Scourge of Geier Reach", "Selfless Cathar", "Selhoff Occultist", "Sensory Deprivation",
    "Sever the Bloodline", "Sharpened Pitchfork", "Shimmering Grotto", "Silent Departure",
    "Silver-Inlaid Dagger", "Silverchase Fox", "Skaab Goliath", "Skirsdag Cultist",
    "Slayer of the Wicked", "Smite the Monstrous", "Sol Ring", "Somberwald Spider",
    "Spectral Flight", "Spectral Rider", "Spider Spawning", "Spidery Grasp", "Splinterfright",
    "Stensia Bloodhall", "Stitched Drake", "Stromkirk Noble", "Stromkirk Patrol", "Sturmgeist",
    "Sulfur Falls", "Swamp", "Swords to Plowshares", "Think Twice", "Thraben Purebloods",
    "Thraben Sentry", "Tormented Pariah", "Traitorous Blood", "Travel Preparations",
    "Traveler's Amulet", "Tree of Redemption", "Tribute to Hunger", "Typhoid Rats",
    "Unburial Rites", "Unruly Mob", "Urgent Exorcism", "Vampire Interloper", "Vampiric Fury",
    "Victim of Night", "Village Bell-Ringer", "Village Cannibals", "Village Ironsmith",
    "Villagers of Estwald", "Voiceless Spirit", "Walking Corpse", "Wooden Stake",
    "Woodland Cemetery", "Woodland Sleuth", "Wreath of Geists",
];

// ---- the properties ----

/// The board as the engine builds it is the board the Oracle data
/// describes, before anything is cast: otherwise every comparison below
/// would be against the wrong starting point.
#[test]
fn the_board_is_what_its_cards_print() {
    let registry = registry();
    let cache = oracle_cache();
    let (state, world) = board(&registry, &cache);
    let (got, want) = (observe(&state, &registry), world.observed());
    assert!(got == want, "{}", differences(&got, &want));
    assert!(world.permanents.values().all(|m| !m.keywords.contains(&Keyword::Hexproof)),
        "the reference's targeting assumes nothing on the board has hexproof");
}

/// Whether the games the engine can end in are the games the text allows,
/// as observed; and if not, each ending only one side reaches, against the
/// nearest the other does.
fn disagreement(want: &[World], got: &Result<Vec<GameState>, String>, registry: &CardRegistry) -> Option<String> {
    use std::collections::BTreeSet;
    let got = match got {
        Ok(endings) => endings,
        Err(why) => return Some(why.clone()),
    };
    // A shuffled library's order is not the text's to say (CR 701.20a).
    let shuffled: Vec<usize> = (0..2).filter(|p| want.iter().any(|w| w.shuffled[*p])).collect();
    let unordered = |mut o: Observed| {
        for p in &shuffled {
            o.library[*p].sort();
        }
        o
    };
    let want: BTreeSet<Observed> = want.iter().map(|w| unordered(w.observed())).collect();
    let got: BTreeSet<Observed> = got.iter().map(|e| unordered(observe(e, registry))).collect();
    if got == want {
        return None;
    }
    let only_engine: Vec<&Observed> = got.difference(&want).collect();
    let only_text: Vec<&Observed> = want.difference(&got).collect();
    let nearest = |o: &Observed, others: &BTreeSet<Observed>, engine_first: bool| others.iter()
        .map(|x| if engine_first { differences(o, x) } else { differences(x, o) })
        .min_by_key(String::len)
        .unwrap_or_default();
    let mut detail: Vec<String> = only_engine.iter().take(2)
        .map(|e| format!("an ending only the engine reaches, against the nearest the text allows:\n      {}", nearest(e, &want, true)))
        .collect();
    detail.extend(only_text.iter().take(2)
        .map(|t| format!("an ending only the text allows, against the nearest the engine reaches:\n      {}", nearest(t, &got, false))));
    Some(format!("{} endings only the engine reaches, {} only the text allows\n    {}",
        only_engine.len(), only_text.len(), detail.join("\n    ")))
}

/// Every card the grammar reads offers exactly the targets its text allows,
/// and at each of them does what its text says. A card with morbid is read
/// on a board where a creature has died this turn as well.
#[test]
fn every_readable_card_does_what_its_text_says() {
    type Build = fn(&CardRegistry, &BTreeMap<String, Printed>) -> (GameState, World);
    let registry = registry();
    let cache = oracle_cache();
    let (cards, _) = readable(&registry, &cache);
    let mut failures = Vec::new();
    let mut casts = 0;
    let boards: [(&str, Build); 3] = [("", board), (" after a death", board_after_a_death), (" with two of a name", board_with_twins)];
    for (card, spec) in cards.iter().filter(|(_, s)| s.land.is_none()) {
        // A mana ability is not cast-and-resolve material (CR 605.3b): the
        // mana test checks it.
        let reads_names = spec.effects.iter().any(|e| matches!(e, Effect::ExileWithSameName { .. }));
        let boards: Vec<&(&str, Build)> = boards.iter().enumerate()
            .filter(|(i, _)| *i == 0 || (*i == 1 && spec.reads_morbid()) || (*i == 2 && reads_names))
            .map(|(_, b)| b)
            .collect();
        let counters = spec.slots.iter().any(|s| matches!(s, Slot::Spell(_)));
        for &(which, build) in boards {
            let (mut state, mut world) = build(&registry, &cache);
            let spell = spell_in_hand(&mut state, &registry, &card.name, P0);
            let cost = mana_cost(&card.mana_cost).expect("read above");
            let creatures = u32::try_from(world.permanents.values().filter(|m| m.is_creature()).count()).expect("a few");
            fund(&mut state, P0, &cost, if spec.cheaper_per_creature { creatures } else { 0 });
            // A counterspell is read with a spell to counter.
            if counters {
                opponent_casts_a_spell(&mut state, &mut world, &registry, &cache);
            }
            let name = format!("{}{}{which}", card.name, if counters { " against Giant Growth" } else { "" });

            // A creature spell: every way it can end, against every way the
            // engine lets it end; then, if its text says what its death
            // does, every way that can end.
            let payable = payments(spec, &world, P0);
            if payable.is_empty() {
                // An additional cost that cannot be paid: no cast (CR 601.2h).
                if engine::legal_actions(&state, &registry).castable_spells.iter().any(|c| c.object_id == spell) {
                    failures.push(format!("{name}: offered though its additional cost cannot be paid"));
                }
                continue;
            }
            if spec.permanent && card.has_type("Creature") {
                casts += 1;
                let entering: Vec<World> = payable.iter()
                    .flat_map(|p| creature_outcomes(spec, &pay(&world, p), P0, card, spell))
                    .collect();
                let random = spec.etb.as_ref().is_some_and(|t| is_random(&t.spec)) || spec.dies.as_ref().is_some_and(|t| is_random(&t.spec));
                let endings = if random {
                    over_seeds(&state, |s| every_ending(s, &registry, Some(spell)))
                } else {
                    every_ending(&state, &registry, Some(spell))
                };
                if let Some(why) = disagreement(&entering, &endings, &registry) {
                    failures.push(format!("{name}: {why}"));
                    continue;
                }
                if spec.dies.is_some() || spec.watch.is_some() {
                    // Seeds repeat the one way it entered.
                    let ([after_entering], Ok([entered, ..])) = (entering.as_slice(), endings.as_deref()) else {
                        // One way of entering, so one creature to kill.
                        failures.push(format!("{name}: a death is checked from one way of entering"));
                        continue;
                    };
                    let id = entered.objects.values()
                        .find(|o| o.zone == Zone::Battlefield && !o.is_token && o.name == card.name)
                        .map(|o| o.id)
                        .expect("the creature is on the battlefield");
                    // Who dies: the creature itself; and, for a trigger on
                    // other deaths, each other creature on the board, and
                    // each two at once, the creature itself among them or
                    // not (CR 603.10a: it sees the deaths it dies with).
                    let mut who = vec![vec![spell]];
                    if spec.watch.is_some() {
                        let creatures: Vec<ObjectId> = after_entering.permanents.iter()
                            .filter(|(_, m)| m.is_creature())
                            .map(|(r, _)| *r)
                            .collect();
                        who.extend(creatures.iter().filter(|r| **r != spell).map(|r| vec![*r]));
                        for (i, x) in creatures.iter().enumerate() {
                            who.extend(creatures[i + 1..].iter().map(|y| vec![*x, *y]));
                        }
                    }
                    for rs in who {
                        casts += 1;
                        let mut dying = entered.clone();
                        for &r in &rs {
                            let e = if r == spell { id } else { r };
                            let lethal = dying.effective_toughness(e, &registry).expect("a creature").max(1);
                            dying.get_object_mut(e).expect("on the battlefield").damage_marked = u32::try_from(lethal).expect("positive");
                        }
                        let what = match rs.as_slice() {
                            [r] if *r == spell => "dying".to_string(),
                            _ => format!("seeing {} die", rs.iter()
                                .map(|r| after_entering.permanents[r].name.as_deref().unwrap_or("a token"))
                                .collect::<Vec<_>>().join(" and ")),
                        };
                        // Every way, and then each "you may" taken every
                        // time and declined every time.
                        let optional = spec.dies.as_ref().is_some_and(|t| t.optional) || spec.watch.as_ref().is_some_and(|w| w.trigger.optional);
                        for may in [None, Some(true), Some(false)].into_iter().filter(|m| m.is_none() || optional) {
                            let deaths = death_outcomes(spec, after_entering, P0, spell, &rs, may);
                            let died = if random {
                                over_seeds(&dying, |s| every_ending_choosing(s, &registry, None, None, may))
                            } else {
                                every_ending_choosing(&dying, &registry, None, None, may)
                            };
                            let policy = match may {
                                None => "",
                                Some(true) => ", taking every \"may\"",
                                Some(false) => ", declining every \"may\"",
                            };
                            if let Some(why) = disagreement(&deaths, &died, &registry) {
                                failures.push(format!("{name} {what}{policy}: {why}"));
                                break;
                            }
                        }
                    }
                }
                continue;
            }

            let expected_sets = target_sets(spec, &world, P0);
            let offered_sets = match offered(&state, &registry, spell) {
                Ok(sets) => sets,
                Err(why) => {
                    failures.push(format!("{name}: {why}"));
                    continue;
                }
            };
            if offered_sets != expected_sets {
                failures.push(format!("{name}: offers targets {offered_sets:?}, the text allows {expected_sets:?} ({spec:?})"));
                continue;
            }
            // Each way to pay an additional cost is a way the cast can go:
            // the sacrifice is named in the cast, the exile asked after it.
            if spec.additional.is_some() {
                let row = engine::legal_actions(&state, &registry).castable_spells.into_iter().find(|c| c.object_id == spell);
                let mut offered_sacrifices = row.map(|r| r.sacrifice_options).unwrap_or_default();
                offered_sacrifices.sort();
                let mut sacrifices: Vec<ObjectId> = payable.iter()
                    .filter_map(|p| match p { Some(Payment::Sacrifice(id)) => Some(*id), _ => None })
                    .collect();
                sacrifices.sort();
                if offered_sacrifices != sacrifices {
                    failures.push(format!("{name}: offers to sacrifice {offered_sacrifices:?}, the text allows {sacrifices:?}"));
                    continue;
                }
                for targets in expected_sets {
                    casts += 1;
                    let want: Vec<World> = payable.iter().map(|p| resolve(spec, &pay(&world, p), &targets, P0, card, spell)).collect();
                    let casts_by_sacrifice: Vec<Option<ObjectId>> = if sacrifices.is_empty() { vec![None] } else { sacrifices.iter().copied().map(Some).collect() };
                    let mut endings = Vec::new();
                    for sacrifice in casts_by_sacrifice {
                        let first = Action::CastSpell {
                            object_id: spell, targets: targets.clone(), sacrifice, exile_count: None,
                            exile_ids: vec![], alternative_cost: None, tap_plan: vec![],
                        };
                        match every_ending_from(&state, &registry, Some(spell), Some(first)) {
                            Ok(e) => endings.extend(e),
                            Err(why) => failures.push(format!("{name} at {targets:?}: {why}")),
                        }
                    }
                    if let Some(why) = disagreement(&want, &Ok(endings), &registry) {
                        failures.push(format!("{name} at {targets:?}: {why}"));
                    }
                }
                continue;
            }
            // A spell that asks something as it resolves (at random, of
            // another player, of a search): every way it can go, against
            // every way the engine goes, from 32 seeds when it is random
            // (CR 608.2d, 701.19).
            if !spec.permanent && asks_as_it_resolves(spec) {
                for targets in expected_sets {
                    casts += 1;
                    let want: Vec<World> = resolution_ways(spec, &world, &targets, P0).iter()
                        .map(|way| resolve(spec, way, &targets, P0, card, spell))
                        .collect();
                    let endings = if is_random(spec) {
                        over_seeds(&state, |s| every_ending_at(s, &registry, spell, &targets))
                    } else {
                        every_ending_at(&state, &registry, spell, &targets)
                    };
                    if let Some(why) = disagreement(&want, &endings, &registry) {
                        failures.push(format!("{name} at {targets:?}: {why}"));
                    }
                }
                continue;
            }
            // An {X} spell: at each target set, every X from nothing to
            // what floats, announced as it is cast (CR 107.3a, 601.2b).
            if cost.symbols.contains(&ManaSymbol::X) {
                for targets in expected_sets {
                    casts += 1;
                    let want: Vec<World> = (0..=X_FLOAT).map(|x| resolve(spec, &World { x, pool: x_left(&world, x), ..world.clone() }, &targets, P0, card, spell)).collect();
                    if let Some(why) = disagreement(&want, &every_ending_at(&state, &registry, spell, &targets), &registry) {
                        failures.push(format!("{name} at {targets:?}: {why}"));
                    }
                }
                continue;
            }
            let lasts_the_turn = lasts_the_turn(spec);
            for targets in expected_sets {
                casts += 1;
                let resolved = resolve(spec, &world, &targets, P0, card, spell);
                let want = resolved.observed();
                match cast_and_let_resolve(&state, &registry, spell, targets.clone()) {
                    Ok(after) => {
                        let got = observe(&after, &registry);
                        if got != want {
                            failures.push(format!("{name} at {targets:?}:\n      {}", differences(&got, &want)));
                        } else if lasts_the_turn {
                            // Played on into the opponent's turn.
                            let next = next_turn_outcomes(&Spec::default(), &resolved, spell, P1);
                            let endings = every_scene_ending(&after, &registry, Scene::NextTurn(Step::PrecombatMain));
                            if let Some(why) = disagreement(&next, &endings, &registry) {
                                failures.push(format!("{name} at {targets:?}, in the next turn: {why}"));
                            }
                        }
                    }
                    Err(why) => failures.push(format!("{name} at {targets:?}: {why}")),
                }
            }
        }
    }
    assert!(casts > 0);
    assert!(failures.is_empty(), "{} of {casts} casts, across the {} cards read, disagree with their text:\n  {}",
        failures.len(), cards.len(), failures.join("\n  "));
}

/// Every activated ability the grammar reads, of a permanent its controller
/// has had since the turn began: the engine offers it at exactly the
/// targets its text allows, and activated at each of them (its costs paid,
/// CR 602.2) and resolved it does what its text says. One that may be
/// activated only once each turn is not offered a second time (602.5b).
#[test]
fn every_readable_activated_ability_does_what_its_text_says() {
    let registry = registry();
    let cache = oracle_cache();
    let (cards, _) = readable(&registry, &cache);
    let mut failures = Vec::new();
    let mut checked = 0;
    for (card, spec) in &cards {
        for ability in &spec.activated {
            checked += 1;
            let name = format!("{}'s ability {}", card.name, ability.index);
            let (mut state, mut world) = board(&registry, &cache);
            let id = named_permanent(&mut state, &registry, &card.name, P0);
            world.permanents.insert(id, model_of(spec, card, P0));
            world.source = Some(id);
            fund(&mut state, P0, &ability.mana, 0);

            // The source is not a target: the engine's list leaves it out.
            let chosen = |set: &[Target]| -> Vec<Target> {
                set.iter().zip(&ability.spec.slots)
                    .filter(|(_, s)| !matches!(s, Slot::This))
                    .map(|(t, _)| t.clone())
                    .collect()
            };
            let expected = target_sets(&ability.spec, &world, P0);
            // What the cost may sacrifice: any creature its controller
            // controls, or any but the source (CR 602.2b, 601.2h); one
            // action is offered for each target and sacrifice.
            let sacrifices: Vec<Option<ObjectId>> = match ability.sacrifice {
                None => vec![None],
                Some(which) => world.permanents.iter()
                    .filter(|(c, m)| m.controller == P0 && m.is_creature() && !(which == Sacrifice::AnotherCreature && **c == id))
                    .map(|(c, _)| Some(*c))
                    .collect(),
            };
            let exilable = world.yard.values().any(|m| m.owner == P0 && m.is_creature());
            let key = |(t, s): &(Vec<Target>, Option<ObjectId>)| (t.iter().map(target_key).collect::<Vec<_>>(), s.map(|i| i.0));
            let mut expected_chosen: Vec<(Vec<Target>, Option<ObjectId>)> = expected.iter()
                .filter(|_| !ability.exile_creature_card || exilable)
                .flat_map(|set| sacrifices.iter().map(|sac| (chosen(set), *sac)))
                .collect();
            expected_chosen.sort_by_key(key);
            let offers: Vec<Action> = engine::legal_actions(&state, &registry).actions.into_iter()
                .filter(|a| matches!(a, Action::ActivateAbility { object_id, ability_index, .. }
                    if *object_id == id && *ability_index == ability.index))
                .collect();
            let mut offered: Vec<(Vec<Target>, Option<ObjectId>)> = offers.iter()
                .map(|a| match a { Action::ActivateAbility { targets, sacrifice, .. } => (targets.clone(), *sacrifice), _ => unreachable!() })
                .collect();
            offered.sort_by_key(key);
            if offered != expected_chosen {
                failures.push(format!("{name}: offered at {offered:?}, the text allows {expected_chosen:?}"));
                continue;
            }
            // With the opponent's spell on the stack, an ability activated
            // only as a sorcery is not offered, and any other still is
            // (CR 602.5d, 702.6a, 117.1b).
            let (mut busy, mut busy_world) = (state.clone(), world.clone());
            the_opponent_holds_a_spell_on_the_stack(&mut busy, &mut busy_world, &registry, &cache);
            let offered_busy = engine::legal_actions(&busy, &registry).actions.iter()
                .any(|a| matches!(a, Action::ActivateAbility { object_id, ability_index, .. } if *object_id == id && *ability_index == ability.index));
            if offered_busy != (!ability.sorcery && !offers.is_empty()) {
                failures.push(format!("{name}: offered with a spell on the stack {offered_busy}, and it is {}activated only as a sorcery",
                    if ability.sorcery { "" } else { "not " }));
            }
            for (targets, sac) in &expected_chosen {
                let full = expected.iter().find(|set| chosen(set) == *targets).expect("a set it was chosen from");
                let act = offers.iter()
                    .find(|a| matches!(a, Action::ActivateAbility { targets: t, sacrifice: s, .. } if t == targets && s == sac))
                    .expect("offered, as just compared")
                    .clone();
                // The costs, paid as it is activated (CR 602.2b): the tap,
                // the sacrifice, which is a death (700.4), the card exiled,
                // whichever its controller chooses, and X.
                let mut paid = world.clone();
                if ability.tap {
                    paid.permanents.get_mut(&id).expect("the source").tapped = true;
                }
                if ability.sacrifice_self {
                    paid.leave(id, Dest::Graveyard);
                }
                if let Some(victim) = sac {
                    paid.refresh();
                    paid.cost_sacrificed = Some(paid.permanents[victim].clone());
                    paid.leave(*victim, Dest::Graveyard);
                    paid.a_creature_died = true;
                }
                let mut ways = vec![paid];
                if ability.exile_creature_card {
                    ways = ways.iter()
                        .flat_map(|w| w.yard.iter().filter(|(_, m)| m.owner == P0 && m.is_creature()).map(|(c, _)| {
                            let mut w = w.clone();
                            let card = w.leave_yard(*c).expect("in the graveyard");
                            w.exile[0].push(card.name.expect("a card"));
                            w
                        }).collect::<Vec<_>>())
                        .collect();
                }
                if ability.mana.symbols.contains(&ManaSymbol::X) {
                    ways = ways.iter().flat_map(|w| (0..=X_FLOAT).map(|x| World { x, pool: x_left(w, x), ..w.clone() })).collect();
                }
                ways = ways.iter().flat_map(|w| resolution_ways(&ability.spec, w, full, P0)).collect();
                let want: Vec<World> = ways.into_iter()
                    .map(|w| {
                        // A target gone by the time it resolves: it does
                        // nothing (CR 608.2b).
                        let gone = !targets.is_empty() && full.iter().zip(&ability.spec.slots)
                            .filter(|(_, slot)| !matches!(slot, Slot::This))
                            .all(|(t, slot)| match (t, slot) {
                                (Target::Object(o), Slot::GraveyardCard { .. }) => !w.yard.contains_key(o),
                                (Target::Object(o), _) => !w.permanents.contains_key(o),
                                _ => false,
                            });
                        let mut after = if gone { w } else { apply(&ability.spec, &w, full, P0) };
                        after.state_based_actions();
                        after
                    })
                    .collect();
                let at = format!("{name} at {targets:?}{}", sac.map_or(String::new(), |v| format!(", sacrificing {v:?}")));
                let endings = every_ending_from(&state, &registry, None, Some(act));
                if let Some(why) = disagreement(&want, &endings, &registry) {
                    failures.push(format!("{at}: {why}"));
                    continue;
                }
                // Played on into the opponent's turn, where what lasted
                // this one is gone.
                if lasts_the_turn(&ability.spec) {
                    if let (Ok([after]), [w]) = (endings.as_deref(), &want[..]) {
                        // With one spell cast this turn, so that neither face
                        // of a werewolf on the board transforms.
                        let mut after = after.clone();
                        after.num_spells_cast_this_turn = BTreeMap::from([(P0, 1)]);
                        let next = next_turn_outcomes(&Spec::default(), w, id, P1);
                        if let Some(why) = disagreement(&next, &every_scene_ending(&after, &registry, Scene::NextTurn(Step::PrecombatMain)), &registry) {
                            failures.push(format!("{at}, in the next turn: {why}"));
                        }
                    }
                }
                // A shield is for the next destruction this turn: each
                // creature with one is dealt lethal damage, and must
                // regenerate (CR 701.19c, 704.5g).
                if ability.spec.effects.iter().any(|e| matches!(e, Effect::Regenerate { .. } | Effect::RegenerateAll { .. })) {
                    let (Ok([after]), [w]) = (endings.as_deref(), &want[..]) else { unreachable!("one way, as just compared") };
                    let (mut engine_side, mut text) = (after.clone(), w.clone());
                    for (shielded, m) in w.permanents.iter().filter(|(_, m)| m.shields > 0) {
                        let lethal = u32::try_from(w.pt_of(m).1.unwrap_or(1).max(1)).expect("positive");
                        engine_side.get_object_mut(*shielded).expect("on the battlefield").damage_marked = lethal;
                        text.permanents.get_mut(shielded).expect("on the battlefield").damage = lethal;
                    }
                    text.state_based_actions();
                    if let Some(why) = disagreement(&[text], &every_ending(&engine_side, &registry, None), &registry) {
                        failures.push(format!("{at}, then dealt lethal damage: {why}"));
                    }
                    // And by an effect that destroys it (CR 701.8a), through
                    // the engine's destruction.
                    let (mut engine_side, mut text) = (after.clone(), w.clone());
                    for shielded in w.permanents.iter().filter(|(_, m)| m.shields > 0).map(|(i, _)| *i) {
                        mtg_engine::destruction::try_destroy(&mut engine_side, shielded, &registry);
                        text.destroy(shielded);
                    }
                    text.state_based_actions();
                    if let Some(why) = disagreement(&[text], &every_ending(&engine_side, &registry, None), &registry) {
                        failures.push(format!("{at}, then destroyed: {why}"));
                    }
                }
                if ability.once_each_turn {
                    let Ok([after]) = endings.as_deref() else { unreachable!("one ending, as just compared") };
                    let mut again = after.clone();
                    fund(&mut again, P0, &ability.mana, 0);
                    let offered_again = engine::legal_actions(&again, &registry).actions.iter()
                        .any(|a| matches!(a, Action::ActivateAbility { object_id, ability_index, .. }
                            if *object_id == id && *ability_index == ability.index));
                    if offered_again {
                        failures.push(format!("{name}: offered a second time in the turn"));
                    }
                }
            }
        }
    }
    assert!(checked > 0);
    assert!(failures.is_empty(), "{} of the {checked} abilities read disagree with their text:\n  {}",
        failures.len(), failures.join("\n  "));
}

/// Every werewolf the grammar reads, and the two on the board with it: at
/// the beginning of an upkeep after a turn in which no spells were cast,
/// each transforms to its back face, keeping what has happened to it; at
/// the beginning of one after a turn in which a player cast two spells,
/// each transforms back (CR 603.4, 712.8). Each upkeep is played on, the
/// triggers ordered front first, and with nothing else to do in it the game
/// goes on to the draw step, where the caster draws (CR 504.1), and stops
/// in the main phase that follows, where they could play a land.
#[test]
fn every_readable_werewolf_transforms_as_its_text_says() {
    let registry = registry();
    let cache = oracle_cache();
    let (cards, _) = readable(&registry, &cache);
    let mut failures = Vec::new();
    let mut werewolves = 0;
    for (card, _) in cards.iter().filter(|(_, s)| s.werewolf) {
        werewolves += 1;
        let (mut state, mut world) = board(&registry, &cache);
        let id = named_permanent(&mut state, &registry, &card.name, P0);
        world.permanents.insert(id, Model::printed_with_faces(card, P0));
        // The upkeep starts, and with it each werewolf's trigger: first
        // after a quiet turn, then after a turn of two spells.
        for (phase, cast_last_turn) in [("after a turn with no spells", 0), ("after a turn of two spells", 2)] {
            state.step = Step::Upkeep;
            state.priority_player = Some(P0);
            state.num_spells_cast_last_turn = [(P1, cast_last_turn)].into_iter().filter(|(_, n)| *n > 0).collect();
            fire_step_trigger(&mut state, Step::Upkeep, &registry);
            for m in world.permanents.values_mut().filter(|m| m.other_face.is_some()) {
                if m.transformed == (cast_last_turn >= 2) {
                    m.transform();
                }
            }
            let drawn = world.library[0].remove(0);
            world.hand[0].push(drawn);
            state = match play(&state, &registry, None, Vec::new(), &[]) {
                Ok(Played::Done(after)) => *after,
                Ok(Played::Asks(options)) => {
                    failures.push(format!("{} {phase}: the engine asked a question the text does not, answerable {options:?}", card.name));
                    break;
                }
                Err(why) => {
                    failures.push(format!("{} {phase}: {why}", card.name));
                    break;
                }
            };
            let (got, want) = (observe(&state, &registry), world.observed());
            if got != want {
                failures.push(format!("{} {phase}:\n      {}", card.name, differences(&got, &want)));
                break;
            }
        }
    }
    assert!(werewolves > 0);
    assert!(failures.is_empty(), "{}", failures.join("\n  "));
}

/// Every permanent the grammar reads with an upkeep trigger, or that keeps
/// a creature from untapping, on the board: a creature of the caster's, a
/// Curse the caster put on the opponent, or an Aura on the caster's tapped
/// Grizzly Bears. From the end step of the turn before each player's turn,
/// the engine must play on into that turn's main phase exactly as the
/// reference does: damage gone, the active player's permanents untapped
/// but for what its text keeps tapped, and no one else's, the trigger
/// resolved every way its text allows in the upkeep it names and in no
/// other, and the active player's draw (CR 514.2, 502.3, 503.1a, 504.1). One spell is cast in the turn before, so
/// that neither face of a werewolf on the board transforms: its triggers
/// ask for no spells last turn, or two.
#[test]
fn every_turn_begins_as_its_text_says() {
    let registry = registry();
    let cache = oracle_cache();
    let (cards, _) = readable(&registry, &cache);
    let mut failures = Vec::new();
    let mut upkeeps = 0;
    let keeps_tapped = |s: &Spec| s.statics.iter().any(|st| matches!(st, Static::Restricts { what, .. } if what.contains(&Restriction::DoesntUntap)));
    for (card, spec) in cards.iter().filter(|(_, s)| s.upkeep.is_some() || keeps_tapped(s)) {
        // The upkeep its text names, and the other player's, which it must
        // leave alone.
        for active in [P0, P1] {
            let (mut state, mut world) = board(&registry, &cache);
            let curse = spec.upkeep.as_ref().is_some_and(|u| u.enchanted_player);
            let id = if curse {
                let id = attach_curse_to_player(&mut state, &registry, &card.name, P0, P1);
                world.permanents.insert(id, Model { attached: Some(Target::Player(P1)), ..model_of(spec, card, P0) });
                id
            } else if spec.enchants {
                let bears = world.permanents.iter()
                    .find(|(_, m)| m.name.as_deref() == Some("Grizzly Bears"))
                    .map(|(id, _)| *id)
                    .expect("on the board");
                let id = named_permanent(&mut state, &registry, &card.name, P0);
                state.attach(id, bears);
                state.get_object_mut(bears).expect("on the board").tapped = true;
                world.permanents.get_mut(&bears).expect("on the board").tapped = true;
                world.permanents.insert(id, Model { attached: Some(Target::Object(bears)), ..model_of(spec, card, P0) });
                id
            } else {
                let id = named_permanent(&mut state, &registry, &card.name, P0);
                world.permanents.insert(id, model_of(spec, card, P0));
                id
            };
            world.refresh();
            // Every land tapped in the turn before, so the untap step has
            // something to untap, and something to leave tapped.
            for (land, m) in world.permanents.iter_mut().filter(|(_, m)| m.types.iter().any(|t| t == "land")) {
                m.tapped = true;
                state.get_object_mut(*land).expect("on the battlefield").tapped = true;
            }
            let before = if active == P0 { P1 } else { P0 };
            state.step = Step::EndStep;
            state.active_player = before;
            state.priority_player = Some(before);
            state.num_spells_cast_this_turn = BTreeMap::from([(before, 1)]);
            upkeeps += 1;
            let want = next_turn_outcomes(spec, &world, id, active);
            // The engine passes for a player with nothing to do, so the
            // first question after the draw is the active player's land
            // drop.
            let got = every_scene_ending(&state, &registry, Scene::Until(Step::PrecombatMain));
            if let Some(why) = disagreement(&want, &got, &registry) {
                failures.push(format!("{} in player {}'s turn: {why}", card.name, active.0));
            }
        }
    }
    assert!(upkeeps > 0);
    assert!(failures.is_empty(), "{} of {upkeeps} upkeeps disagree with the text:\n  {}", failures.len(), failures.join("\n  "));
}

// ---- combat ----

fn has(m: &Model, k: Keyword) -> bool {
    m.keywords.contains(&k) || m.granted.contains(&k) || m.temp_keywords.contains(&k)
}

/// Whether `blocker` may block `attacker` alone (CR 509.1a-b): a creature
/// that can't block, can't; flying is blocked only by flying or reach
/// (702.9b, 702.17b), intimidate only by an artifact creature or one that
/// shares a colour (702.13b); and the attacker's own restrictions: can't
/// be blocked, only by creatures with flying or reach, not by a subtype,
/// and not by what it has protection from (702.16f).
fn evasion_allows(attacker: &Model, blocker: &Model) -> bool {
    let a = &attacker.combat;
    !restricted(blocker, Restriction::CantBlock)
        && !a.cant_be_blocked
        && !(has(attacker, Keyword::Flying) && !(has(blocker, Keyword::Flying) || has(blocker, Keyword::Reach)))
        && !(a.only_by_flying_or_reach && !(has(blocker, Keyword::Flying) || has(blocker, Keyword::Reach)))
        && !(has(attacker, Keyword::Intimidate)
            && !(blocker.types.iter().any(|t| t == "artifact") || attacker.colors.iter().any(|c| blocker.colors.contains(c))))
        && !a.not_blocked_by.iter().chain(&a.protection_from).any(|t| blocker.subtypes.contains(t))
}

/// Whether damage `source` would deal `to` is prevented by protection
/// (CR 702.16e).
fn protected(to: &Model, source: &Model) -> bool {
    to.combat.protection_from.iter().any(|t| source.subtypes.contains(t))
}

/// Every way a combat in which `a` attacks the opponent, blocked by
/// `blocker` or not, can end, as CR 510 deals its damage: a first-strike
/// step when either has first strike or double strike (510.4), damage
/// dealt at once in each step, a trampler assigning lethal damage to its
/// blocker and as much of the rest as it likes to the player (702.19b-c,
/// lethal being 1 from a deathtouch source, 702.2c), a blocked creature
/// whose blocker is gone dealing no damage unless it tramples (509.1h,
/// 702.19e), damage protection prevents not dealt (702.16e), lifelink
/// gaining its controller what it dealt (702.15b), and state-based actions
/// after each step, deathtouch damage lethal (704.5h). Then each
/// "whenever this creature deals combat damage to a player" that the step
/// set off resolves (603.3), and state-based actions again. An attacker
/// without vigilance taps to attack (508.1f, 702.20b).
fn combat_outcomes(world: &World, a: ObjectId, blocker: Option<ObjectId>) -> Vec<World> {
    let mut w = world.clone();
    w.refresh();
    if !has(&w.permanents[&a], Keyword::Vigilance) {
        w.permanents.get_mut(&a).expect("the attacker").tapped = true;
    }
    w.permanents.get_mut(&a).expect("the attacker").attacking = true;
    w.refresh();
    // Its attack trigger: tokens that enter tapped and attacking, never
    // having been declared attackers (CR 508.4).
    if let Some(t) = w.permanents[&a].combat.on_attack_tokens.clone() {
        let k = if t.per_creature_card { w.creature_cards_in_graveyard(P0) } else { 1 };
        for _ in 0..k {
            let token = creature_token((t.power, t.toughness), &t.colors, &t.subtypes, &t.keywords, P0);
            w.tokens.push(Model { tapped: true, attacking: true, exile_at_end_of_combat: t.exile_at_end_of_combat, ..token });
        }
        w.state_based_actions();
    }
    // Block triggers (CR 509.4): "blocks or becomes blocked by a creature,
    // this creature deals 1 damage to that creature", and a stake on
    // either destroying the other if it is the type the stake names, past
    // any shield.
    if let Some(b) = blocker {
        for (one, other) in [(a, b), (b, a)] {
            let (Some(om), Some(xm)) = (w.permanents.get(&one).cloned(), w.permanents.get(&other).cloned()) else { continue };
            let staked = w.permanents.values()
                .any(|e| e.combat.stake.as_ref().is_some_and(|t| xm.subtypes.contains(t)) && e.attached == Some(Target::Object(one)));
            if staked && !has(&xm, Keyword::Indestructible) {
                w.leave(other, Dest::Graveyard);
                w.a_creature_died = true;
                continue;
            }
            if let Some(n) = om.combat.block_damage.filter(|_| !protected(&xm, &om)) {
                let m = w.permanents.get_mut(&other).expect("in combat");
                m.damage += n;
                m.deathtouched |= has(&om, Keyword::Deathtouch);
            }
        }
        w.state_based_actions();
    }
    let fast = |m: &Model| has(m, Keyword::FirstStrike) || has(m, Keyword::DoubleStrike);
    let any_fast = w.permanents.get(&a).is_some_and(fast) || blocker.is_some_and(|b| w.permanents.get(&b).is_some_and(fast))
        || w.tokens.iter().any(|t| t.attacking && fast(t));
    let steps: &[Option<bool>] = if any_fast { &[Some(true), Some(false)] } else { &[None] };
    let mut worlds = vec![w];
    for &step in steps {
        // Who deals damage in this step: in a first-strike step, first and
        // double strikers; in the regular step after one, whoever has not
        // first strike, and double strikers again; with no first-strike
        // step, everyone.
        let deals = |m: &Model| match step {
            Some(true) => fast(m),
            Some(false) => !has(m, Keyword::FirstStrike) || has(m, Keyword::DoubleStrike),
            None => true,
        };
        let mut next = Vec::new();
        for w in worlds {
            let power = |m: &Model| u32::try_from(w.pt_of(m).0.unwrap_or(0).max(0)).expect("non-negative");
            // Each branch is one assignment: (recipient, amount, source).
            let mut branches: Vec<Vec<(Target, u32, Src)>> = vec![Vec::new()];
            if let Some(am) = w.permanents.get(&a).filter(|m| deals(m) && power(m) > 0) {
                let n = power(am);
                match blocker {
                    None => branches[0].push((Target::Player(P1), n, Src::Permanent(a))),
                    Some(b) => match w.permanents.get(&b) {
                        Some(bm) => {
                            let lethal = if has(am, Keyword::Deathtouch) {
                                1
                            } else {
                                u32::try_from(w.pt_of(bm).1.unwrap_or(0).max(0)).expect("non-negative").saturating_sub(bm.damage)
                            };
                            if has(am, Keyword::Trample) && n > lethal {
                                branches = (lethal..=n)
                                    .map(|x| vec![(Target::Object(b), x, Src::Permanent(a)), (Target::Player(P1), n - x, Src::Permanent(a))])
                                    .collect();
                            } else {
                                branches[0].push((Target::Object(b), n, Src::Permanent(a)));
                            }
                        }
                        None if has(am, Keyword::Trample) => branches[0].push((Target::Player(P1), n, Src::Permanent(a))),
                        None => {}
                    },
                }
            }
            if let (Some(b), true) = (blocker, w.permanents.contains_key(&a)) {
                if let Some(bm) = w.permanents.get(&b).filter(|m| deals(m) && power(m) > 0) {
                    for branch in &mut branches {
                        branch.push((Target::Object(a), power(bm), Src::Permanent(b)));
                    }
                }
            }
            // The tokens attacking, unblocked.
            for (i, t) in w.tokens.iter().enumerate().filter(|(_, t)| t.attacking && deals(t) && power(t) > 0) {
                for branch in &mut branches {
                    branch.push((Target::Player(P1), power(t), Src::Token(i)));
                }
            }
            for branch in branches {
                let mut w2 = w.clone();
                let mut to_players: Vec<(Src, u32)> = Vec::new();
                for (to, n, from) in &branch {
                    let source = from.model(&w).clone();
                    // Protection and "prevent all combat damage that would
                    // be dealt to and dealt by" it (CR 702.16e, 615.1).
                    let n = match to {
                        _ if restricted(&source, Restriction::NoCombatDamage) => 0,
                        Target::Object(id) if protected(&w.permanents[id], &source)
                            || restricted(&w.permanents[id], Restriction::NoCombatDamage) => 0,
                        _ => *n,
                    };
                    if n == 0 {
                        continue;
                    }
                    match to {
                        Target::Object(id) => {
                            let m = w2.permanents.get_mut(id).expect("in combat");
                            m.damage += n;
                            m.deathtouched |= has(&source, Keyword::Deathtouch);
                        }
                        Target::Player(p) => {
                            w2.life[p.0 as usize] -= n as i32;
                            to_players.push((*from, n));
                        }
                        _ => unreachable!(),
                    }
                    if has(&source, Keyword::Lifelink) {
                        w2.life[source.controller.0 as usize] += n as i32;
                    }
                }
                let tokens_before = w2.tokens.len();
                w2.state_based_actions();
                assert_eq!(tokens_before, w2.tokens.len(), "no token here dies of combat damage");
                for (from, dealt) in to_players {
                    let source = from.model(&w).clone();
                    if from == Src::Permanent(a) {
                        on_damage_to_player(&mut w2, a, &source, dealt);
                    }
                    // A watcher of its controller's creatures of a type, and
                    // a Curse on the player dealt the damage.
                    // Each one triggers on its own (CR 603.2).
                    let watchers = w2.permanents.values().filter(|m| m.controller == source.controller
                        && m.combat.counters_on_yours.as_ref().is_some_and(|t| source.subtypes.contains(t))).count();
                    let cursed = w2.permanents.values().filter(|m| m.combat.counters_on_damage_to_enchanted && m.attached == Some(Target::Player(P1))).count();
                    for _ in 0..watchers + cursed {
                        if let Some(m) = from.model_mut(&mut w2) {
                            change(m, 1, 1);
                            m.counters.0 += 1;
                        }
                    }
                }
                w2.state_based_actions();
                next.push(w2);
            }
        }
        worlds = next;
    }
    // End of combat: what its text exiles then is exiled, a token ceasing
    // to exist (CR 511.1, 111.7).
    for w in &mut worlds {
        w.tokens.retain(|t| !t.exile_at_end_of_combat);
        for t in w.tokens.iter_mut().chain(w.permanents.values_mut()) {
            t.attacking = false;
        }
        w.refresh();
    }
    worlds
}

/// What dealt combat damage: a permanent the engine numbered, or a token
/// the reference holds by its place.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Src {
    Permanent(ObjectId),
    Token(usize),
}

impl Src {
    fn model<'a>(&self, w: &'a World) -> &'a Model {
        match self {
            Src::Permanent(id) => &w.permanents[id],
            Src::Token(i) => &w.tokens[*i],
        }
    }

    fn model_mut<'a>(&self, w: &'a mut World) -> Option<&'a mut Model> {
        match self {
            Src::Permanent(id) => w.permanents.get_mut(id),
            Src::Token(i) => w.tokens.get_mut(*i),
        }
    }
}

/// The attacker's own "whenever this creature deals combat damage to a
/// player" (CR 510.3a), `dealt` being how much it dealt.
fn on_damage_to_player(w2: &mut World, a: ObjectId, attacker: &Model, dealt: u32) {
    for effect in &attacker.combat.on_damage_to_player {
        match effect {
            OnPlayerDamage::CountersOnSelf(k) => {
                if let Some(m) = w2.permanents.get_mut(&a) {
                    let k = i32::try_from(*k).expect("a few");
                    change(m, k, k);
                    m.counters.0 += k as u32;
                }
            }
            OnPlayerDamage::Draw(k) => {
                for _ in 0..*k {
                    let card = w2.library[0].remove(0);
                    w2.hand[0].push(card);
                }
            }
            OnPlayerDamage::ThatMuchToEachCreatureThatPlayerControls => {
                let theirs: Vec<ObjectId> = w2.permanents.iter()
                    .filter(|(_, m)| m.controller == P1 && m.is_creature())
                    .map(|(id, _)| *id)
                    .collect();
                for id in theirs {
                    if !protected(&w2.permanents[&id], attacker) {
                        let m = w2.permanents.get_mut(&id).expect("on the battlefield");
                        m.damage += dealt;
                        m.deathtouched |= has(attacker, Keyword::Deathtouch);
                    }
                }
            }
        }
    }
}

/// Two creatures, one each side, and nothing else but the caster's land in
/// hand and both libraries: a combat between them, and the same in the
/// reference.
fn arena(registry: &CardRegistry, attacker: (&Printed, &Spec), blocker: Option<(&Printed, &Spec)>) -> (GameState, World, ObjectId, Option<ObjectId>) {
    let (state, world, a, b, _) = arena_with(registry, attacker, blocker, None);
    (state, world, a, b)
}

/// [`arena`], and a third permanent standing by on one side, out of the
/// combat: a creature, or an Equipment or Aura attached to that side's
/// combatant, or a Curse on that side's player.
fn arena_with(registry: &CardRegistry, attacker: (&Printed, &Spec), blocker: Option<(&Printed, &Spec)>, standing: Option<((&Printed, &Spec), PlayerId)>)
    -> (GameState, World, ObjectId, Option<ObjectId>, Option<ObjectId>)
{
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mut world = World {
        permanents: BTreeMap::new(),
        yard: BTreeMap::new(),
        a_creature_died: false,
        stack: Vec::new(),
        source: None,
        cost_exiled: Vec::new(),
        cost_sacrificed: None,
        chosen: Vec::new(),
        discarded: Vec::new(),
        found: None,
        shuffled: [false, false],
        x: 0,
        faces_put: BTreeMap::new(),
        tokens: Vec::new(),
        life: [20, 20],
        pool: [BTreeMap::new(), BTreeMap::new()],
        hand: [vec!["Island".to_string()], vec!["Island".to_string()]],
        graveyard: [Vec::new(), Vec::new()],
        exile: [Vec::new(), Vec::new()],
        library: [Vec::new(), Vec::new()],
    };
    // A card in each hand: a land the caster could play, so the game loop
    // asks them rather than passing for them, and something a creature
    // that counts its controller's hand can count.
    spell_in_hand(&mut state, registry, "Island", P0);
    spell_in_hand(&mut state, registry, "Island", P1);
    for p in [P0, P1] {
        for name in LIBRARY {
            stock_library_with(&mut state, registry, p, name, 1);
            world.library[p.0 as usize].push(name.to_string());
        }
    }
    // Two creature cards in the attacker's graveyard, for what an attack
    // makes "for each creature card in your graveyard".
    for name in ARENA_GRAVEYARD {
        let id = card_in_graveyard(&mut state, registry, name, P0);
        world.yard.insert(id, Model::printed(&cached_oracle()[name], P0));
        world.graveyard[0].push(name.to_string());
    }
    let mut place = |state: &mut GameState, (card, spec): (&Printed, &Spec), owner: PlayerId| {
        // A back face: its front card, transformed (CR 712.8e).
        let front = registry.get_id_by_name(&card.name).is_none()
            .then(|| cached_oracle().values().find(|f| f.back.as_ref().is_some_and(|b| b.name == card.name)))
            .flatten();
        let id = named_permanent(state, registry, front.map_or(&card.name, |f| &f.name), owner);
        let mut m = Model {
            combat: spec.combat.clone(),
            counts_hand: spec.pt_is_cards_in_hand,
            statics: spec.statics.clone(),
            ..Model::printed(card, owner)
        };
        if let Some(f) = front {
            state.get_object_mut(id).expect("just placed").is_transformed = true;
            m.transformed = true;
            m.other_face = Some(Box::new(Model::printed(f, owner)));
        }
        world.permanents.insert(id, m);
        id
    };
    let a = place(&mut state, attacker, P0);
    let b = blocker.map(|b| place(&mut state, b, P1));
    let s = standing.map(|(c, side)| (place(&mut state, c, side), c, side));
    if let Some((id, (card, spec), side)) = s {
        let on_player = spec.enchants && spec.slots.first() == Some(&Slot::Player);
        if on_player {
            state.get_object_mut(id).expect("just placed").attached_to_player = Some(side);
            world.permanents.get_mut(&id).expect("just placed").attached = Some(Target::Player(side));
        } else if spec.enchants || card.type_line.contains("Equipment") {
            if let Some(host) = if side == P0 { Some(a) } else { b } {
                state.attach(id, host);
                world.permanents.get_mut(&id).expect("just placed").attached = Some(Target::Object(host));
            }
        }
    }
    world.refresh();
    (state, world, a, b, s.map(|(id, _, _)| id))
}

/// What the engine asks in a combat: the attackers its prompt names and
/// those that must attack, and for the blocker the attackers it may block
/// and the attackers that need more than one.
struct Asked {
    eligible: Vec<ObjectId>,
    must_attack: Vec<ObjectId>,
    blocks: Option<(std::collections::HashMap<ObjectId, Vec<ObjectId>>, std::collections::HashMap<ObjectId, u32>)>,
}

fn combat_questions(state: &GameState, registry: &CardRegistry, a: ObjectId) -> Asked {
    let mut state = state.clone();
    let mut asked = Asked { eligible: Vec::new(), must_attack: Vec::new(), blocks: None };
    let mut decisions = 0;
    let turn = state.turn_number;
    // This turn's combat only: with no creature able to attack, the engine
    // asks nothing and the turn goes on, and the next turn's questions are
    // about a different board.
    engine::run_game_loop(&mut state, registry, |s: &GameState, _acting: PlayerId, legal: &engine::LegalActions| {
        decisions += 1;
        match &legal.combat_prompt {
            _ if decisions > 40 || s.turn_number != turn || s.step == Step::PostcombatMain => Action::AbandonGame,
            Some(CombatPrompt::ChooseAttackers { eligible, must_attack, .. }) => {
                asked.eligible.clone_from(eligible);
                asked.must_attack.clone_from(must_attack);
                if eligible.contains(&a) {
                    Action::DeclareAttackers { attackers: vec![(a, P1)], planeswalker_attacks: Vec::new() }
                } else {
                    Action::AbandonGame
                }
            }
            Some(CombatPrompt::ChooseBlockers { legal_blocks, min_blockers, .. }) => {
                asked.blocks = Some((legal_blocks.clone(), min_blockers.clone()));
                Action::AbandonGame
            }
            None => Action::PassPriority,
        }
    });
    asked
}

/// Every game a combat can end in: each damage assignment it asks for
/// answered every way, replayed from the start for each.
fn every_combat_ending(state: &GameState, registry: &CardRegistry, combat: &Combat) -> Result<Vec<GameState>, String> {
    every_scene_ending(state, registry, Scene::Combat(combat))
}

/// Every game `scene` can end in, played on from `state` with no cast:
/// each question answered every way, replayed from the start for each.
fn every_scene_ending(state: &GameState, registry: &CardRegistry, scene: Scene) -> Result<Vec<GameState>, String> {
    let mut todo: Vec<Vec<Answer>> = vec![Vec::new()];
    let mut done = Vec::new();
    while let Some(script) = todo.pop() {
        if done.len() + todo.len() > 200 {
            return Err("more than 200 ways to answer".to_string());
        }
        match play_from(state, registry, None, Vec::new(), None, scene, &script)? {
            Played::Done(s) => done.push(*s),
            Played::Asks(options) => todo.extend(options.into_iter().map(|o| [script.clone(), vec![o]].concat())),
        }
    }
    Ok(done)
}

/// Each back face of a card the grammar reads, as a card of its own, for
/// what fights as it.
fn back_faces(cards: &[(Printed, Spec)]) -> Vec<(Printed, Spec)> {
    cards.iter()
        .filter_map(|(c, _)| {
            let back = c.back.as_ref()?;
            Some(((**back).clone(), back_face_reads(back)?))
        })
        .collect()
}

/// The creatures the combat tests fight with: those the grammar reads whose
/// text is keywords, combat text and static abilities, a double-faced
/// card's faces both, its back face in `backs`.
fn fighters<'a>(cards: &'a [(Printed, Spec)], backs: &'a [(Printed, Spec)]) -> Vec<(&'a Printed, &'a Spec)> {
    cards.iter().chain(backs)
        .filter(|(c, s)| c.has_type("Creature") && s.etb.is_none() && s.dies.is_none() && s.watch.is_none() && s.activated.is_empty()
            && s.mana.is_empty() && !s.pt_is_creatures_you_control && !s.pt_is_creature_cards
            && !s.enters_tapped && (c.power.is_some() || s.pt_is_cards_in_hand))
        .map(|(c, s)| (c, s))
        .collect()
}

/// One combat in an arena: `a` attacks, and `b`, if there is one, may
/// block it. The engine must offer `a` as an attacker exactly when it
/// lacks defender, and require it to attack exactly when its text says it
/// attacks each combat if able; let `b` block it alone exactly when
/// evasion, restrictions, protection and menace allow; and end the combat
/// every way the reference does and no other, with the defender declining
/// to block and, when it may, blocking. The number of combats compared,
/// and what disagrees.
fn fight(registry: &CardRegistry, state: &GameState, world: &World, a: ObjectId, b: Option<ObjectId>, name: &str) -> (usize, Vec<String>) {
    let mut failures = Vec::new();
    let am = &world.permanents[&a];
    let asked = combat_questions(state, registry, a);
    let may_attack = !has(am, Keyword::Defender) && !restricted(am, Restriction::CantAttack);
    if asked.eligible.contains(&a) != may_attack {
        failures.push(format!("{name}: offered as an attacker {}, and its text {} it", asked.eligible.contains(&a),
            if may_attack { "allows" } else { "forbids" }));
        return (0, failures);
    }
    if !may_attack {
        return (0, failures);
    }
    let must = restricted(am, Restriction::AttacksEachCombat);
    if asked.must_attack.contains(&a) != must {
        failures.push(format!("{name}: required to attack {}, and its text says {must}", asked.must_attack.contains(&a)));
        return (0, failures);
    }
    let mut combats = 0;
    let mut block = Vec::new();
    if let Some(b) = b {
        let bm = &world.permanents[&b];
        let Some((legal_blocks, min_blockers)) = asked.blocks else {
            failures.push(format!("{name}: no blockers were asked for"));
            return (0, failures);
        };
        let may = legal_blocks.get(&b).is_some_and(|v| v.contains(&a));
        if may != evasion_allows(am, bm) {
            failures.push(format!("{name}: it may block {may}, and the text says {}", evasion_allows(am, bm)));
            return (0, failures);
        }
        let needs = min_blockers.get(&a).copied();
        if needs != has(am, Keyword::Menace).then_some(2) {
            failures.push(format!("{name}: needs {needs:?} blockers, and menace says {:?}", has(am, Keyword::Menace).then_some(2)));
            return (0, failures);
        }
        // The defender may always decline to block: with the would-be
        // blocker standing by, unblocked damage and what it sets off
        // (Balefire Dragon's) reach the board as well.
        combats += 1;
        let want = combat_outcomes(world, a, None);
        let endings = every_combat_ending(state, registry, &(vec![(a, P1)], Vec::new()));
        if let Some(why) = disagreement(&want, &endings, registry) {
            failures.push(format!("{name} declining to block: {why}"));
        }
        if !(may && needs.is_none()) {
            return (combats, failures);
        }
        block.push((b, a));
    }
    combats += 1;
    let want = combat_outcomes(world, a, b.filter(|_| !block.is_empty()));
    let endings = every_combat_ending(state, registry, &(vec![(a, P1)], block));
    if let Some(why) = disagreement(&want, &endings, registry) {
        failures.push(format!("{name}: {why}"));
    }
    (combats, failures)
}

/// Every creature the grammar reads whose text is keywords and combat text
/// attacks, once unblocked and once against each of them as a lone
/// blocker. The engine must:
///
/// - let it attack exactly when it lacks defender (CR 702.3b), and require
///   it to exactly when its text says it attacks each combat if able
///   (508.1d); and, summoning sick, let it attack exactly when it has
///   haste, or the haste its text grants while an opponent controls a
///   creature of some type (302.6, 702.10b);
/// - let each blocker block it alone exactly when evasion, restrictions,
///   protection and menace allow (509.1b, 702.16f, 702.110b);
/// - end the combat every way the reference allows and no other, every
///   damage assignment the attacking player may choose tried, whether the
///   creature opposite blocks or not.
#[test]
fn every_keyword_creature_fights_as_its_keywords_say() {
    let registry = registry();
    let cache = oracle_cache();
    let (cards, _) = readable(&registry, &cache);
    let backs = back_faces(&cards);
    let fighters = fighters(&cards, &backs);
    let mut failures = Vec::new();
    let mut combats = 0;
    for &attacker in &fighters {
        let (card, spec) = attacker;
        // Summoning sick, against no one and against a creature of each
        // type its haste may depend on.
        let haste_types: Vec<Option<&str>> = std::iter::once(None)
            .chain(spec.combat.haste_if_opponent_controls.as_deref().map(Some))
            .collect();
        for needs in haste_types {
            let opponent = needs.map(|t| fighters.iter().copied()
                .find(|(c, _)| c.types().1.iter().any(|s| s == t))
                .expect("a creature of that type to stand opposite"));
            let (mut state, world, a, _) = arena(&registry, attacker, opponent);
            state.get_object_mut(a).expect("the attacker").summoning_sick = true;
            let hasty = has(&world.permanents[&a], Keyword::Haste) || needs.is_some();
            let may = combat_questions(&state, &registry, a).eligible.contains(&a);
            if may != (hasty && !has(&world.permanents[&a], Keyword::Defender)) {
                failures.push(format!("{} summoning sick{}: may attack {may}, and its text says {hasty}",
                    card.name, needs.map_or(String::new(), |t| format!(" opposite a {t}"))));
            }
        }
        for blocker in std::iter::once(None).chain(fighters.iter().copied().map(Some)) {
            let (state, world, a, b) = arena(&registry, attacker, blocker);
            let name = format!("{} attacking{}", card.name, blocker.map_or(String::new(), |(b, _)| format!(", {} blocking", b.name)));
            let (n, why) = fight(&registry, &state, &world, a, b, &name);
            combats += n;
            failures.extend(why);
        }
    }
    assert!(combats > 0);
    assert!(failures.is_empty(), "{} of {combats} combats disagree with the text:\n  {}", failures.len(), failures.join("\n  "));
}

/// The fights [`every_keyword_creature_fights_as_its_keywords_say`] has,
/// again with each of `standing` on either side, out of combat: a creature,
/// or an Equipment or Aura on that side's combatant or player. Only the
/// combats it changes are fought: those where what the static abilities do
/// to a combatant differs from the same arena without it. With `probes`,
/// the creature it changes fights only a probe of each kind of opponent.
fn standing_fights(registry: &CardRegistry, fighters: &[(&Printed, &Spec)], standing: &[(&Printed, &Spec)], probes: Option<&[(&Printed, &Spec)]>)
    -> (usize, Vec<String>)
{
    let mut failures = Vec::new();
    let mut combats = 0;
    // Unless every fight is asked for, one fighter of each kind.
    let kinds = if std::env::var_os("CARD_SEMANTICS_EVERY_FIGHT").is_some() { fighters.to_vec() } else { kinds(fighters) };
    let fighters = &kinds[..];
    for &by in standing {
        for side in [P0, P1] {
            let attackers = match probes { Some(p) if side == P1 => p, _ => fighters };
            let blockers = match probes { Some(p) if side == P0 => p, _ => fighters };
            for &attacker in attackers {
                for blocker in std::iter::once(None).chain(blockers.iter().copied().map(Some)) {
                    let (state, mut world, a, b, s) = arena_with(registry, attacker, blocker, Some((by, side)));
                    // A combatant the static kills before combat does not
                    // fight; the cast test sees that death.
                    world.state_based_actions();
                    if !world.permanents.contains_key(&a) || b.is_some_and(|b| !world.permanents.contains_key(&b)) {
                        continue;
                    }
                    let mut without = world.clone();
                    without.permanents.remove(&s.expect("standing by"));
                    without.refresh();
                    let changed = |id: ObjectId| {
                        let (x, y) = (&world.permanents[&id], &without.permanents[&id]);
                        x.boost != y.boost || x.granted != y.granted || x.restricted != y.restricted
                    };
                    // A permanent whose combat text watches the fight changes
                    // it too: one watching its controller's attackers of a
                    // type, standing by for the attacker, or a Curse on the
                    // defending player.
                    let watches = (side == P0 && by.1.combat.counters_on_yours.as_ref().is_some_and(|t| attacker.0.types().1.contains(t)))
                        || (side == P1 && by.1.combat.counters_on_damage_to_enchanted);
                    if !(changed(a) || b.is_some_and(changed) || watches) {
                        continue;
                    }
                    let name = format!("{} attacking{}, {} standing by for player {}", attacker.0.name,
                        blocker.map_or(String::new(), |(b, _)| format!(", {} blocking", b.name)), by.0.name, side.0);
                    let (n, why) = fight(registry, &state, &world, a, b, &name);
                    combats += n;
                    failures.extend(why);
                }
            }
        }
    }
    (combats, failures)
}

/// One fighter of each kind: the first of those alike in keywords and in
/// the subtypes the cards here name. The fighters with combat text or
/// static abilities of their own are each a kind; the keyword test fights
/// every one.
fn kinds<'a>(fighters: &[(&'a Printed, &'a Spec)]) -> Vec<(&'a Printed, &'a Spec)> {
    const NAMED: [&str; 6] = ["Human", "Vampire", "Werewolf", "Spirit", "Zombie", "Wolf"];
    let mut seen = std::collections::BTreeSet::new();
    fighters.iter().copied()
        .filter(|(c, s)| {
            let mut keywords: Vec<&str> = c.keywords().iter().map(|k| k.label()).collect();
            keywords.sort_unstable();
            let subtypes: Vec<String> = c.types().1.into_iter().filter(|t| NAMED.contains(&t.as_str())).collect();
            let own = s.combat != CombatText::default() || !s.statics.is_empty();
            own || seen.insert((keywords, subtypes))
        })
        .collect()
}

/// A vanilla fighter, and one with each keyword that changes how combat
/// damage is dealt or who may block: the opponents an Equipment's or an
/// Aura's creature fights unless CARD_SEMANTICS_EVERY_FIGHT is set
/// (scripts/prove.sh exhaustive), when it fights every fighter.
fn probes<'a>(fighters: &[(&'a Printed, &'a Spec)]) -> Option<Vec<(&'a Printed, &'a Spec)>> {
    if std::env::var_os("CARD_SEMANTICS_EVERY_FIGHT").is_some() {
        return None;
    }
    let mut probes: Vec<(&Printed, &Spec)> = Vec::new();
    for k in [None, Some(Keyword::Flying), Some(Keyword::FirstStrike), Some(Keyword::Deathtouch), Some(Keyword::Trample), Some(Keyword::Reach)] {
        let kind = fighters.iter().copied().find(|(c, s)| s.statics.is_empty() && s.combat == CombatText::default()
            && k.map_or(c.keywords().is_empty(), |k| c.keywords().contains(&k)));
        probes.extend(kind.filter(|p| !probes.iter().any(|q| q.0.name == p.0.name)));
    }
    Some(probes)
}

/// Every fight of [`every_keyword_creature_fights_as_its_keywords_say`]
/// that a creature standing by on either side changes, out of combat: each
/// fighter with a static ability, and a creature of each subtype a static
/// asks its controller for (Angelic Overseer's Human). A lord's bonus,
/// granted keywords, a count of opposing creatures: the engine must carry
/// each into combat damage and the state-based actions after it, and drop
/// it when what it depends on leaves (CR 611.3a, 613.4c, 704.3).
#[test]
fn every_static_reaches_the_combats_it_changes() {
    let registry = registry();
    let cache = oracle_cache();
    let (cards, _) = readable(&registry, &cache);
    let backs = back_faces(&cards);
    let fighters = fighters(&cards, &backs);
    let mut standing: Vec<(&Printed, &Spec)> = fighters.iter().copied()
        .filter(|(_, s)| !s.statics.is_empty() || s.combat.counters_on_yours.is_some())
        .collect();
    for (_, spec) in standing.clone() {
        for st in &spec.statics {
            if let Static::IfYouControl { subtype, .. } = st {
                standing.push(fighters.iter().copied()
                    .find(|(c, s)| s.statics.is_empty() && c.types().1.contains(subtype))
                    .expect("a creature of that subtype to stand by"));
            }
        }
    }
    let (combats, failures) = standing_fights(&registry, &fighters, &standing, None);
    assert!(combats > 0);
    assert!(failures.is_empty(), "{} of {combats} combats disagree with the text:\n  {}", failures.len(), failures.join("\n  "));
    println!("{combats} combats with a creature standing by agree with the text");
}

/// The same with each Equipment the grammar reads on the combatant of
/// either side: what it gives the creature it is on reaches combat, and
/// the Equipment stays when that creature dies (CR 301.5, 704.5n).
#[test]
fn every_equipment_reaches_the_combats_it_is_in() {
    let registry = registry();
    let cache = oracle_cache();
    let (cards, _) = readable(&registry, &cache);
    let backs = back_faces(&cards);
    let fighters = fighters(&cards, &backs);
    let standing: Vec<(&Printed, &Spec)> = cards.iter()
        .filter(|(c, s)| c.type_line.contains("Equipment") && !s.statics.is_empty())
        .map(|(c, s)| (c, s))
        .collect();
    let (combats, failures) = standing_fights(&registry, &fighters, &standing, probes(&fighters).as_deref());
    assert!(combats > 0);
    assert!(failures.is_empty(), "{} of {combats} combats disagree with the text:\n  {}", failures.len(), failures.join("\n  "));
    println!("{combats} combats with an Equipment agree with the text");
}

/// The same with each Aura the grammar reads on the combatant of either
/// side, or on its player: what it gives the creature, and what it stops
/// or makes it do, reach combat: "can't attack or block", "attacks each
/// combat if able", combat damage to and by it prevented (CR 508.1c-d,
/// 509.1b, 615.1); and the Aura goes to the graveyard when its creature
/// dies (704.5m).
#[test]
fn every_aura_reaches_the_combats_it_is_in() {
    let registry = registry();
    let cache = oracle_cache();
    let (cards, _) = readable(&registry, &cache);
    let backs = back_faces(&cards);
    let fighters = fighters(&cards, &backs);
    let standing: Vec<(&Printed, &Spec)> = cards.iter()
        .filter(|(_, s)| s.enchants && (!s.statics.is_empty() || s.combat != CombatText::default()))
        .map(|(c, s)| (c, s))
        .collect();
    let (combats, failures) = standing_fights(&registry, &fighters, &standing, probes(&fighters).as_deref());
    assert!(combats > 0);
    assert!(failures.is_empty(), "{} of {combats} combats disagree with the text:\n  {}", failures.len(), failures.join("\n  "));
    println!("{combats} combats with an Aura agree with the text");
}

/// A creature spell with Flash can be cast whenever its controller has
/// priority, and one without only in their own main phase with the stack
/// empty (CR 702.8a, 307.1): on the opponent's turn, with priority passed
/// to the caster, exactly the Flash creatures are offered.
#[test]
fn flash_is_exactly_the_creatures_that_print_it() {
    let registry = registry();
    let cache = oracle_cache();
    let (cards, _) = readable(&registry, &cache);
    let mut wrong = Vec::new();
    let mut flash = 0;
    for (card, spec) in cards.iter().filter(|(c, s)| s.permanent && c.has_type("Creature")) {
        let (mut state, _) = board(&registry, &cache);
        // The opponent's precombat main phase; they have passed, so the
        // caster holds priority with the stack empty.
        state.active_player = P1;
        state.priority_player = Some(P0);
        state.consecutive_passes = 1;
        let spell = spell_in_hand(&mut state, &registry, &card.name, P0);
        fund(&mut state, P0, &mana_cost(&card.mana_cost).expect("read"), 0);
        let offered = engine::legal_actions(&state, &registry).castable_spells.iter().any(|c| c.object_id == spell);
        flash += usize::from(spec.flash);
        if offered != spec.flash {
            wrong.push(format!("{}: {} on the opponent's turn, and its text {} Flash",
                card.name, if offered { "offered" } else { "not offered" }, if spec.flash { "has" } else { "does not have" }));
        }
    }
    assert!(flash > 0, "the pool has a Flash creature to check");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// A land played from hand enters tapped exactly when its text says (CR
/// 614.1c; the caster controls a Plains and no other land), and every
/// permanent with mana abilities, tapped for mana every way the engine
/// offers, makes exactly the mana its text says, one option of one ability
/// per tap, having paid that ability's cost (CR 106.1, 305.6, 605): any
/// mana it costs out of a floating {C}, and any card it mills from the top
/// of the library. A creature that came under its controller's control
/// this turn cannot be tapped for mana at all (CR 302.6).
#[test]
fn every_readable_mana_ability_makes_what_its_text_says() {
    let registry = registry();
    let cache = oracle_cache();
    let (cards, _) = readable(&registry, &cache);
    let mut wrong = Vec::new();
    let mut lands = 0;
    let mut sources = 0;
    for (card, spec) in &cards {
        if spec.mana.is_empty() {
            continue;
        }
        sources += 1;
        let (mut state, mut world) = board(&registry, &cache);
        let id = if let Some(unless) = &spec.land {
            lands += 1;
            let land = spell_in_hand(&mut state, &registry, &card.name, P0);
            let played = engine::submit_action(&state, &Action::PlayLand { object_id: land }, &registry);
            let caster_has = |t: &String| world.permanents.values().any(|m| m.controller == P0 && m.subtypes.contains(t));
            let tapped = unless.as_ref().is_some_and(|types| !types.iter().any(caster_has));
            let obj = played.get_object(land).expect("the land exists");
            if obj.zone != Zone::Battlefield || obj.tapped != tapped {
                wrong.push(format!("{}: {:?}, tapped {}, where its text has it enter {}", card.name, obj.zone, obj.tapped,
                    if tapped { "tapped" } else { "untapped" }));
                continue;
            }
            state = played;
            // Untapped, so it can be tapped for mana every way it allows.
            state.get_object_mut(land).expect("on the battlefield").tapped = false;
            land
        } else {
            named_permanent(&mut state, &registry, &card.name, P0)
        };
        world.permanents.insert(id, Model::printed(card, P0));
        let card_id = state.get_object(id).expect("on the battlefield").card_id;
        if card.has_type("Creature") {
            let mut sick = state.clone();
            sick.get_object_mut(id).expect("on the battlefield").summoning_sick = true;
            let offered = engine::legal_actions(&sick, &registry).actions.iter()
                .any(|a| matches!(a, Action::ActivateManaAbility { object_id, .. } if *object_id == id));
            if offered {
                wrong.push(format!("{}: tapped for mana the turn it came under its controller's control", card.name));
            }
        }
        // A floating {C} pays any ability's {1}.
        let costs = spec.mana.iter().any(|m| m.cost > 0);
        if costs {
            state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
        }
        // The menu shows one row per card and ability, however many copies
        // are untapped (a second Plains shares the first's row); the row's
        // ability is tapped on this copy.
        let observe_pool = |s: &GameState| -> Vec<(ManaType, u32)> {
            s.get_player(P0).mana_pool.mana.iter().filter(|(_, n)| **n > 0).map(|(t, n)| (*t, *n)).collect()
        };
        let mut made: Vec<(Vec<(ManaType, u32)>, Observed)> = engine::legal_actions(&state, &registry).actions.iter()
            .filter_map(|a| match a {
                Action::ActivateManaAbility { object_id, ability_index }
                    if state.get_object(*object_id).is_some_and(|o| o.card_id == card_id) =>
                    Some(Action::ActivateManaAbility { object_id: id, ability_index: *ability_index }),
                _ => None,
            })
            .map(|a| {
                let after = engine::submit_action(&state, &a, &registry);
                (observe_pool(&after), observe(&after, &registry))
            })
            .collect();
        made.sort();
        let mut want: Vec<(Vec<(ManaType, u32)>, Observed)> = Vec::new();
        for ability in &spec.mana {
            let mut w = world.clone();
            w.permanents.get_mut(&id).expect("on the battlefield").tapped = true;
            for _ in 0..ability.mill {
                let top = w.library[0].remove(0);
                w.graveyard[0].push(top);
            }
            for option in &ability.options {
                let mut pool: BTreeMap<ManaType, u32> = BTreeMap::new();
                if costs {
                    pool.insert(ManaType::Colorless, 1 - ability.cost);
                }
                for &(t, n) in option {
                    *pool.entry(t).or_insert(0) += n;
                }
                let made = World { pool: [pool.clone(), BTreeMap::new()], ..w.clone() };
                let pool: Vec<(ManaType, u32)> = pool.into_iter().filter(|(_, n)| *n > 0).collect();
                want.push((pool, made.observed()));
            }
        }
        want.sort();
        if made != want {
            let made_pools: Vec<_> = made.iter().map(|(p, _)| p.clone()).collect();
            let want_pools: Vec<_> = want.iter().map(|(p, _)| p.clone()).collect();
            let detail = made.iter().zip(&want).find(|(m, w)| m.1 != w.1)
                .map(|(m, w)| differences(&m.1, &w.1))
                .unwrap_or_default();
            wrong.push(format!("{}: tapping it makes {made_pools:?}, its text {want_pools:?}\n      {detail}", card.name));
        }
    }
    assert!(lands >= 10, "the basics and the check lands are read");
    assert!(sources > lands, "and sources that are not lands");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_grammar_reads_at_least_as_many_cards_as_it_did() {
    let registry = registry();
    let cache = oracle_cache();
    let (cards, unread) = readable(&registry, &cache);
    let names: Vec<&str> = cards.iter().map(|(c, _)| c.name.as_str()).collect();
    let lost: Vec<&&str> = READ.iter().filter(|n| !names.contains(n)).collect();
    let why: Vec<&(String, String)> = unread.iter().filter(|(n, _)| lost.contains(&&n.as_str())).collect();
    assert!(lost.is_empty(), "the grammar no longer reads {lost:?}: {why:?}");
    println!("read {} of {}: {names:?}", cards.len(), cards.len() + unread.len());
    for (name, why) in &unread {
        println!("  unread {name}: {why}");
    }
}

/// The grammar reads what it is meant to, on text written out here, so a
/// template that silently stopped matching is caught even if no card in the
/// pool uses it any more.
#[test]
fn the_grammar_reads_its_templates() {
    let card = |name: &str, text: &str| Printed {
        name: name.to_string(),
        mana_cost: "{R}".to_string(),
        type_line: "Instant".to_string(),
        oracle_text: text.to_string(),
        power: None,
        toughness: None,
        back: None,
        color_indicator: Vec::new(),
    };
    let spec = read(&card("Bolt", "Bolt deals 3 damage to any target.")).unwrap();
    assert_eq!(spec.slots, vec![Slot::Any]);
    assert_eq!(spec.effects, vec![Effect::Damage { amount: 3, slot: 0 }]);

    let spec = read(&card("X", "Destroy target non-Vampire, non-Werewolf, non-Zombie creature.")).unwrap();
    let Slot::Permanent(Filter(clauses)) = &spec.slots[0] else { panic!("a permanent slot") };
    assert_eq!(clauses[0].head.as_deref(), Some("creature"));
    assert_eq!(clauses[0].not_types, vec!["Vampire", "Werewolf", "Zombie"]);

    let spec = read(&card("X", "Create thirteen tapped 2/2 black Zombie creature tokens.")).unwrap();
    assert_eq!(spec.effects, vec![Effect::Tokens {
        n: 13, power: 2, toughness: 2, colors: vec![Color::Black], subtypes: vec!["Zombie".to_string()],
        keywords: vec![], tapped: true, per_creature_card: false,
    }]);

    let spec = read(&card("X", "Exile target creature. Its controller gains life equal to its power.")).unwrap();
    assert_eq!(spec.effects, vec![Effect::Exile { slot: 0 }, Effect::ControllerGainsLifeEqualToPower { slot: 0 }]);

    let spec = read(&card("X", "Target creature you control fights target creature you don't control.")).unwrap();
    assert_eq!(spec.effects, vec![Effect::Fight { a: 0, b: 1 }]);
    let Slot::Permanent(Filter(clauses)) = &spec.slots[1] else { panic!("a permanent slot") };
    assert_eq!(clauses[0].yours, Some(false));

    let spec = read(&card("X", "Put a +1/+1 counter on each of up to two target creatures.")).unwrap();
    assert!(matches!(spec.slots[..], [Slot::UpTo { n: 2, .. }]));

    let spec = read(&card("Volley", "Volley deals 3 damage to any target.\nMorbid — Volley deals 5 damage instead if a creature died this turn.")).unwrap();
    assert_eq!(spec.morbid, Some((0, 5)));

    let spec = read(&card("X", "Vampire creatures you control get +2/+0 and gain first strike until end of turn.")).unwrap();
    let Effect::PumpEach { filter: Filter(clauses), gains, .. } = &spec.effects[0] else { panic!("a group pump") };
    assert_eq!((clauses[0].subtypes.clone(), gains.clone()), (vec!["Vampire".to_string()], vec![Keyword::FirstStrike]));

    let spec = read(&card("X", "Return target nonlegendary creature card from your graveyard to the battlefield with a -1/-1 counter on it.")).unwrap();
    assert_eq!(spec.effects, vec![Effect::Reanimate { slot: 0, minus_counter: true }]);

    let slayer = Printed {
        oracle_text: "When this creature enters, you may destroy target Vampire, Werewolf, or Zombie.".to_string(),
        type_line: "Creature — Human Soldier".to_string(),
        ..card("Slayer", "")
    };
    let trigger = *read_creature(&slayer).unwrap().etb.expect("an enters trigger");
    assert!(trigger.optional && !trigger.if_morbid);
    let Slot::Permanent(Filter(clauses)) = &trigger.spec.slots[0] else { panic!("a permanent slot") };
    let heads: Vec<Option<String>> = clauses.iter().map(|c| c.head.clone()).collect();
    assert_eq!(heads, ["Vampire", "Werewolf", "Zombie"].map(|h| Some(h.to_string())), "three alternatives, not one with qualifiers");

    let devils = Printed { oracle_text: "When this creature dies, it deals 3 damage to any target.".to_string(), ..slayer.clone() };
    let dies = read_creature(&devils).unwrap().dies.expect("a dies trigger");
    assert_eq!((dies.spec.slots.clone(), dies.spec.effects.clone()), (vec![Slot::Any], vec![Effect::Damage { amount: 3, slot: 0 }]));
    let hunter = Printed { oracle_text: "When this creature enters, you may exile another target creature.".to_string(), ..slayer.clone() };
    let etb = read_creature(&hunter).unwrap().etb.expect("an enters trigger");
    let Slot::Permanent(Filter(clauses)) = &etb.spec.slots[0] else { panic!("a permanent slot") };
    assert!(clauses[0].other, "\"another\" excludes the source");

    let aura = |text: &str| Printed { oracle_text: text.to_string(), type_line: "Enchantment — Aura".to_string(), ..card("Aura", "") };
    let flight = read_enchantment(&aura("Enchant creature\nEnchanted creature gets +2/+2 and has flying.")).unwrap();
    assert!(flight.enchants);
    assert_eq!(flight.statics, vec![Static::Enchanted { power: 2, toughness: 2, has: vec![Keyword::Flying], per: None, if_subtype: None }]);
    let hold = read_enchantment(&aura("Enchant player\nCreatures enchanted player controls get -1/-1.")).unwrap();
    assert_eq!(hold.slots, vec![Slot::Player]);
    assert!(matches!(hold.statics[..], [Static::Group { enchanted_player: true, power: -1, toughness: -1, .. }]));
    let virtue = read_enchantment(&Printed { type_line: "Enchantment".to_string(), ..aura("Creature tokens you control get +1/+1 and have vigilance.") }).unwrap();
    let [Static::Group { filter: Filter(clauses), has, .. }] = &virtue.statics[..] else { panic!("one group static") };
    assert!(clauses[0].token && has == &vec![Keyword::Vigilance]);
    let pacifism = read_enchantment(&aura("Enchant creature\nEnchanted creature can't attack or block.")).unwrap();
    assert_eq!(pacifism.statics, vec![Static::Restricts { group: false, what: vec![Restriction::CantAttack, Restriction::CantBlock], unless_subtype: None }]);
    let bonds = read_enchantment(&aura("Enchant creature\nEnchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.")).unwrap();
    assert!(matches!(&bonds.statics[..], [Static::Enchanted { power: 2, toughness: 2, if_subtype: Some(h), .. },
        Static::Restricts { unless_subtype: Some(u), .. }] if h == "Human" && u == "Human"));
    let claustrophobia = read_enchantment(&Printed { name: "Claustrophobia".to_string(),
        ..aura("Enchant creature\nWhen this Aura enters, tap enchanted creature.\nEnchanted creature doesn't untap during its controller's untap step.") }).unwrap();
    assert_eq!(claustrophobia.etb.expect("an enters trigger").spec.slots, vec![Slot::EnchantedCreature]);
    assert!(read_enchantment(&aura("Enchant creature\nEnchanted creature can't block unless it's a Human.")).is_err());

    let lord = creature_static("Other Spirit creatures you control get +1/+0.").expect("a group static");
    let Static::Group { filter: Filter(clauses), power: 1, toughness: 0, .. } = &lord else { panic!("+1/+0 to a group") };
    assert!(clauses[0].other && clauses[0].yours == Some(true) && clauses[0].subtypes == ["Spirit"], "\"Other\" excludes the source");
    let scarecrow = creature_static("Creatures with flying your opponents control get -1/-0.").expect("a group static");
    let Static::Group { filter: Filter(clauses), power: -1, .. } = &scarecrow else { panic!("-1/-0 to a group") };
    assert!(clauses[0].with == [Keyword::Flying] && clauses[0].yours == Some(false));
    assert!(matches!(creature_static("This creature gets +1/+1 for each creature your opponents control."),
        Some(Static::PerEach { power: 1, toughness: 1, .. })));
    assert_eq!(creature_static("As long as you control a Human, this creature has hexproof and indestructible."),
        Some(Static::IfYouControl { subtype: "Human".to_string(), has: vec![Keyword::Hexproof, Keyword::Indestructible] }));
    assert!(creature_static("Whenever this creature attacks or blocks, other Humans you control get +1/+1 until end of turn.").is_none());

    let watcher = |text: &str| read_creature(&Printed { oracle_text: text.to_string(), ..slayer.clone() }).unwrap().watch.expect("a trigger on deaths");
    let mob = watcher("Whenever another creature you control dies, put a +1/+1 counter on this creature.");
    assert!(mob.dying.0[0].other && mob.dying.0[0].yours == Some(true));
    assert_eq!(mob.trigger.spec.effects, vec![Effect::PlusCounters { slot: 0, n: 1 }]);
    assert_eq!(mob.trigger.spec.slots, vec![Slot::This]);
    let noble = watcher("Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.");
    assert!(!noble.dying.0[0].other, "\"this creature or another\" is every creature");
    assert_eq!(noble.trigger.spec.effects, vec![Effect::LoseLife { slot: 0, n: 1 }, Effect::GainLife { n: 1 }]);
    assert_eq!(watcher("Whenever another Human creature dies, put a +1/+1 counter on this creature.").dying.0[0].subtypes, ["Human"]);

    let demon = read_upkeep("At the beginning of your upkeep, target player draws a card and loses 1 life.").expect("an upkeep trigger");
    assert!(!demon.enchanted_player);
    assert_eq!(demon.trigger.spec.effects, vec![Effect::PlayerDraws { slot: 0, n: 1 }, Effect::LoseLife { slot: 0, n: 1 }]);
    let tome = read_upkeep("At the beginning of enchanted player's upkeep, that player mills two cards.").expect("an upkeep trigger");
    assert!(tome.enchanted_player);
    assert_eq!(tome.trigger.spec.slots, vec![Slot::EnchantedPlayer { or_planeswalker: false }]);
    let heart = read_upkeep("At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.").expect("an upkeep trigger");
    assert_eq!(heart.trigger.spec.slots, vec![Slot::EnchantedPlayer { or_planeswalker: true }]);
    let angel = read_upkeep("At the beginning of your upkeep, return target Spirit card from your graveyard to your hand.").expect("an upkeep trigger");
    assert!(matches!(&angel.trigger.spec.slots[..], [Slot::GraveyardCard { yours: true, .. }]));

    let equipment = |text: &str| read_artifact(&Printed { oracle_text: text.to_string(), type_line: "Artifact — Equipment".to_string(), ..card("Gear", "") });
    let dagger = equipment("Equipped creature gets +2/+0.\nAs long as equipped creature is a Human, it gets an additional +1/+0.\nEquip {2}").unwrap();
    assert_eq!(dagger.statics, vec![
        Static::Enchanted { power: 2, toughness: 0, has: vec![], per: None, if_subtype: None },
        Static::Enchanted { power: 1, toughness: 0, has: vec![], per: None, if_subtype: Some("Human".to_string()) },
    ]);
    let [equip] = &dagger.activated[..] else { panic!("one ability, equip") };
    assert!(equip.sorcery && equip.index == 0 && equip.mana.mana_value() == 2 && equip.spec.effects == [Effect::Attach { slot: 0 }]);
    let pike = equipment("Equipped creature has first strike and gets +X/+0, where X is the number of instant and sorcery cards in your graveyard.\nEquip {2}").unwrap();
    assert_eq!(pike.statics, vec![Static::Enchanted { power: 1, toughness: 0, has: vec![Keyword::FirstStrike], per: Some(Per::InstantAndSorceryCards), if_subtype: None }]);
    assert!(read_activated("Sacrifice this creature: Target player draws a card. Activate only as a sorcery.", 0).unwrap().sorcery);

    let wolf = read_activated("{2}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn.", 0).unwrap();
    assert!(wolf.once_each_turn && !wolf.tap && wolf.spec.slots == vec![Slot::This]);
    let fox = read_activated("{1}{W}, Sacrifice this creature: Exile target enchantment.", 0).unwrap();
    assert!(fox.sacrifice_self && fox.mana.mana_value() == 2);
    let drownyard = read_land(&Printed {
        type_line: "Land".to_string(),
        ..card("Drownyard", "{T}: Add {C}.\n{1}{U}{B}, {T}: Target player mills three cards.")
    }).unwrap();
    assert_eq!(drownyard.activated[0].index, 1, "the mana ability is index 0");
    let vigil = read(&card("Caravan Vigil", "Search your library for a basic land card, reveal it, put it into your hand, then shuffle.\nMorbid — You may put that card onto the battlefield instead of putting it into your hand if a creature died this turn.")).unwrap();
    assert_eq!(vigil.effects, vec![Effect::SearchForBasicLand { of: None, to_battlefield: false, morbid_battlefield: true }]);
    let quarter = read_activated("{T}, Sacrifice this land: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.", 1).unwrap();
    assert_eq!(quarter.spec.effects, vec![Effect::Destroy { slot: 0 }, Effect::SearchForBasicLand { of: Some(0), to_battlefield: true, morbid_battlefield: false }]);
    let mut geist = CombatText::default();
    geist.read("Whenever ~ attacks, create a 4/4 white Angel creature token with flying that's tapped and attacking. Exile that token at end of combat.").unwrap();
    assert_eq!(geist.on_attack_tokens, Some(AttackTokens { power: 4, toughness: 4, colors: vec![Color::White], subtypes: vec!["Angel".to_string()],
        keywords: vec![Keyword::Flying], per_creature_card: false, exile_at_end_of_combat: true }));
    let mut cagebreakers = CombatText::default();
    cagebreakers.read("Whenever this creature attacks, create a 2/2 green Wolf creature token that's tapped and attacking for each creature card in your graveyard.").unwrap();
    assert!(cagebreakers.on_attack_tokens.is_some_and(|t| t.per_creature_card && !t.exile_at_end_of_combat));
    let mut hound = CombatText::default();
    hound.read("Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature.").unwrap();
    assert_eq!(hound.block_damage, Some(1));
    assert!(CombatText::default().read("Whenever this creature attacks, create a 1/1 white Spirit creature token with flying.").is_none(),
        "a token that does not enter attacking is another template");

    let cathar = read_creature(&Printed { oracle_text: "When this creature dies, put a +1/+1 counter on target creature you control. If that creature is a Human, put two +1/+1 counters on it instead.".to_string(), ..slayer.clone() }).unwrap();
    assert_eq!(cathar.dies.expect("a dies trigger").spec.effects, vec![Effect::PlusCountersIfSubtype { slot: 0, n: 1, subtype: "Human".to_string(), n_if: 2 }]);
    let laurels = read_activated("{3}{G}: Target creature gets +X/+X until end of turn, where X is the number of creatures you control.", 0).unwrap();
    assert!(matches!(&laurels.spec.effects[..], [Effect::PumpPerCount { power: 1, toughness: 1, .. }]));
    assert_eq!(read(&card("X", "Add {R}{R}{R}.")).unwrap().effects, vec![Effect::AddMana(vec![(ManaType::Red, 3)])]);

    let Some(Static::Group { filter: Filter(clauses), has, .. }) = creature_static("Werewolves you control have menace.") else { panic!("a group static") };
    assert_eq!((clauses[0].head.as_deref(), has), (Some("Werewolf"), vec![Keyword::Menace]), "a plural subtype is its singular");
    let Some(Static::Group { filter: Filter(clauses), .. }) = creature_static("Attacking creatures you control get +1/+0.") else { panic!("a group static") };
    assert!(clauses[0].attacking && clauses[0].subtypes.is_empty(), "\"Attacking\" qualifies; it is not a subtype");
    let terror = Printed { name: "Terror of Kruin Pass".to_string(), oracle_text: "Double strike\nWerewolves you control have menace.\nAt the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.".to_string(), ..slayer.clone() };
    assert_eq!(back_face_reads(&terror).expect("a back face").statics.len(), 1);

    let blood = read(&card("X", "Gain control of target creature until end of turn. Untap it. It gains trample and haste until end of turn.")).unwrap();
    assert_eq!(blood.effects, vec![Effect::GainControl { slot: 0, until_end_of_turn: true }, Effect::Tap { slot: 0, tapped: false },
        Effect::Pump { slot: 0, power: 0, toughness: 0, gains: vec![Keyword::Trample, Keyword::Haste] }]);

    let mut tribute = Spec::default();
    for sentence in ["Target opponent sacrifices a creature of their choice. ", "You gain life equal to that creature's toughness."] {
        tribute.sentence(sentence).unwrap();
    }
    assert_eq!((tribute.slots, tribute.effects), (vec![Slot::Opponent],
        vec![Effect::SacrificeOfTheirChoice { slot: 0 }, Effect::GainLifeEqualToSacrificedToughness]));
    let mut rest = Spec::default();
    assert!(rest.sentence("Destroy the rest.").is_none(), "\"the rest\" needs a choice before it");
    let disciple = read_activated("{1}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.", 0).unwrap();
    assert_eq!((disciple.sacrifice, disciple.spec.effects), (Some(Sacrifice::AnyCreature), vec![Effect::GainLifeEqualToSacrificedToughness]));
    assert!(read_activated("{W}{U}, {T}, Exile a creature card from your graveyard: Create a 1/1 white Spirit creature token with flying.", 1).unwrap().exile_creature_card);
    assert!(read_activated("{1}, Sacrifice two creatures: Draw a card.", 0).is_none());

    assert_eq!(Additional::read("As an additional cost to cast this spell, sacrifice a creature."), Some(Additional::SacrificeCreature));
    assert_eq!(Additional::read("As an additional cost to cast this spell, exile two creature cards from your graveyard."), Some(Additional::ExileCreatureCards(2)));
    assert_eq!(Additional::read("As an additional cost to cast this spell, exile a creature card from your graveyard."), Some(Additional::ExileCreatureCards(1)));
    assert_eq!(Additional::read("As an additional cost to cast this spell, exile X cards from your graveyard."), Some(Additional::ExileXCards));
    let mut play = Spec::default();
    play.sentence("~ deals X damage to any target.").unwrap();
    assert_eq!((play.slots, play.effects), (vec![Slot::Any], vec![Effect::DamageX { slot: 0 }]));
    let run = read_activated("{X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.", 1).unwrap();
    assert!(run.tap && run.mana.symbols.contains(&ManaSymbol::X));
    assert_eq!(run.spec.effects, vec![Effect::PumpX { slot: 0, power: 1, toughness: 0, gains: vec![Keyword::Trample] }]);

    let grotto = ManaAbility::read("{1}, {T}: Add one mana of any color.").unwrap();
    assert_eq!((grotto.cost, grotto.mill, grotto.options.len()), (1, 0, 5));
    assert_eq!(ManaAbility::read("{T}: Add {C}{C}.").unwrap().options, vec![vec![(ManaType::Colorless, 2)]]);
    assert_eq!(ManaAbility::read("{T}, Mill a card: Add {C}.").unwrap().mill, 1);
    assert_eq!(ManaAbility::read("{T}: Add {R} or {W}.").unwrap().indices(), 2);
    assert!(ManaAbility::read("{2}{G}: This creature gets +2/+2 until end of turn.").is_none());

    let viper = Printed { oracle_text: "Flash\nDeathtouch".to_string(), type_line: "Creature — Snake".to_string(), ..card("Viper", "") };
    assert!(read_creature(&viper).is_ok_and(|s| s.permanent && s.flash));
    let interloper = Printed { oracle_text: "Flying\nThis creature can't block.".to_string(), ..viper.clone() };
    assert!(read_creature(&interloper).is_ok_and(|s| s.combat.cant_block));
    let inquisitor = Printed { oracle_text: "First strike, vigilance\nProtection from Vampires, from Werewolves, and from Zombies".to_string(), ..viper.clone() };
    assert_eq!(read_creature(&inquisitor).unwrap().combat.protection_from, vec!["Vampire", "Werewolf", "Zombie"]);
    assert_eq!(Printed { oracle_text: "Defender, protection from Zombies".to_string(), ..viper.clone() }.keywords(), vec![Keyword::Defender]);
    let unread = Printed { oracle_text: "Flying\nThis creature can't be the target of spells.".to_string(), ..viper };
    assert!(read_creature(&unread).is_err());

    let spec = read(&card("X", "Counter target noncreature spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.")).unwrap();
    assert_eq!(spec.effects, vec![Effect::Counter { slot: 0, exile: true }]);
    let Slot::Spell(Filter(clauses)) = &spec.slots[0] else { panic!("a spell slot") };
    assert_eq!((clauses[0].head.clone(), clauses[0].not_types.clone()), (None, vec!["creature".to_string()]));

    assert!(read(&card("X", "Counter target spell unless its controller pays {1}.")).is_err(), "a payment is not a template yet");
    assert_eq!(read(&card("X", "Draw two cards, then discard a card at random.")).unwrap().effects,
        vec![Effect::Draw { n: 2 }, Effect::DiscardAtRandom { n: 1 }]);
    assert_eq!(read(&card("X", "Return two cards at random from your graveyard to your hand.")).unwrap().effects,
        vec![Effect::ReturnAtRandom { n: 2, filter: None, to_battlefield: false }]);
    assert!(read(&card("X", "Draw two cards, then discard a card.")).is_err(), "a discard of the player's choice is not read");
}



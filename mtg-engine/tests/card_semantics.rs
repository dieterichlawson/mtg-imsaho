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
//! into what it does while it is on the battlefield, and each land it can
//! read into whether it enters tapped and what mana it makes. A reference
//! semantics,
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
use mtg_engine::actions::{Action, CastTargetSpec, ResolvedChoice, Target};
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
}

fn oracle_cache() -> BTreeMap<String, Printed> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../data/oracle_cache.json");
    let raw = std::fs::read_to_string(path).expect("data/oracle_cache.json is readable");
    let json: serde_json::Value = serde_json::from_str(&raw).expect("the oracle cache is JSON");
    let int = |v: &serde_json::Value| v.as_str().and_then(|s| s.parse().ok());
    json["cards"].as_object().expect("a cards map").iter()
        .map(|(name, c)| (name.clone(), Printed {
            name: name.clone(),
            mana_cost: c["mana_cost"].as_str().unwrap_or("").to_string(),
            type_line: c["type_line"].as_str().unwrap_or("").to_string(),
            oracle_text: c["oracle_text"].as_str().unwrap_or("").to_string(),
            power: int(&c["power"]),
            toughness: int(&c["toughness"]),
        }))
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

    /// Its colours: the colours of the mana symbols in its cost (CR 202.2),
    /// which is all there is for the cards on the board here (none has a
    /// colour indicator).
    fn colors(&self) -> Vec<Color> {
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
            let words: Vec<&str> = line.split(',').map(str::trim).collect();
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
    /// An Aura's "Enchant creature" / "Enchant player" (CR 303.4a, 702.5):
    /// the slot its spell targets, the first of `slots`.
    enchants: bool,
    /// Static abilities: continuous effects while it is on the battlefield
    /// (CR 604.1, 611.3).
    statics: Vec<Static>,
    /// A land: each kind of mana one tap of it can make (CR 106.1, 305.6),
    /// and the basic land types that let it enter untapped ("This land
    /// enters tapped unless you control a Mountain or a Plains").
    land: Option<(Vec<ManaType>, Option<Vec<String>>)>,
}

/// A static ability that changes power, toughness or keywords (CR 613.4c,
/// 613.1f).
#[derive(Debug, Clone, PartialEq)]
enum Static {
    /// "Enchanted creature gets +P/+T [and has K]", "Enchanted creature has
    /// K", and "+X/+X, where X is the number of creature cards in your
    /// graveyard".
    Enchanted { power: i32, toughness: i32, has: Vec<Keyword>, per_creature_card: bool },
    /// "<creatures> get +P/+T [and have K]"; with `enchanted_player`, the
    /// creatures that player controls ("Creatures enchanted player controls
    /// get -1/-1").
    Group { filter: Filter, enchanted_player: bool, power: i32, toughness: i32, has: Vec<Keyword> },
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
            if *q == "another" {
                clause.other = true;
            } else if *q == "attacking" {
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
        let singular = lower.strip_suffix('s').unwrap_or(&lower);
        let head = if CARD_TYPES.contains(&singular) || singular == "permanent" { singular } else { head };
        clause.head = match head {
            // What kind of object it is, the slot says; the words are
            // about its characteristics.
            "permanent" | "spell" => None,
            h if CARD_TYPES.contains(&h) => Some(h.to_string()),
            // A subtype is capitalised in Oracle text.
            h if h.chars().next().is_some_and(char::is_uppercase) => Some(h.to_string()),
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
            if let Some((who, change)) = rest.split_once(" gets ") {
                let ((power, toughness), gains) = pump(&format!("gets {change}"))?;
                let slot = self.slot(Slot::Permanent(filter(who)?));
                self.effects.push(Effect::Pump { slot, power, toughness, gains });
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
        if let Some(rest) = line.strip_prefix("When this creature dies, ") {
            let mut chars = rest.chars();
            let sentence: String = chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default();
            let mut trigger = Trigger::default();
            trigger.spec.sentence(&sentence).ok_or_else(unread)?;
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
        for word in line.split(',').map(str::trim) {
            let k = keyword_named(word).ok_or_else(unread)?;
            spec.flash |= k == Keyword::Flash;
        }
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
        if let Some(rest) = body.strip_prefix("Enchanted creature ") {
            let (rest, per_creature_card) = match rest.strip_suffix(", where X is the number of creature cards in your graveyard") {
                Some(r) => (r.replace("+X/+X", "+1/+1"), true),
                None => (rest.to_string(), false),
            };
            let ((power, toughness), has) = static_change(&rest).ok_or_else(unread)?;
            spec.statics.push(Static::Enchanted { power, toughness, has, per_creature_card });
            continue;
        }
        if let Some(rest) = body.strip_prefix("Creatures enchanted player controls ") {
            let ((power, toughness), has) = static_change(rest).ok_or_else(unread)?;
            spec.statics.push(Static::Group { filter: filter("creature").expect("a filter"), enchanted_player: true, power, toughness, has });
            continue;
        }
        let group = |who: &str, rest: &str| -> Option<Static> {
            let ((power, toughness), has) = static_change(rest)?;
            let filter = if who == "Creature tokens you control" {
                Filter(vec![Clause { head: Some("creature".to_string()), yours: Some(true), token: true, ..Clause::default() }])
            } else {
                filter(who)?
            };
            Some(Static::Group { filter, enchanted_player: false, power, toughness, has })
        };
        let parsed = ["get ", "have "].iter()
            .find_map(|verb| body.split_once(&format!(" {verb}")).map(|(who, rest)| (who, format!("{verb}{rest}"))))
            .and_then(|(who, rest)| group(who, &rest));
        spec.statics.push(parsed.ok_or_else(unread)?);
    }
    if spec.enchants != spec.slots.first().is_some() || spec.statics.is_empty() {
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
    let mut mana: Vec<ManaType> = subtypes.iter()
        .filter_map(|t| match t.as_str() {
            "Plains" => Some(ManaType::White),
            "Island" => Some(ManaType::Blue),
            "Swamp" => Some(ManaType::Black),
            "Mountain" => Some(ManaType::Red),
            "Forest" => Some(ManaType::Green),
            _ => None,
        })
        .collect();
    let mut unless = None;
    for line in without_reminders(&card.oracle_text).lines().map(str::trim).filter(|l| !l.is_empty()) {
        let unread = || format!("cannot read {line:?}");
        if let Some(rest) = line.strip_prefix("{T}: Add ").and_then(|r| r.strip_suffix('.')) {
            for sym in rest.split(" or ") {
                mana.push(mana_named(sym).ok_or_else(unread)?);
            }
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
    Ok(Spec { permanent: true, land: Some((mana, unless)), ..Spec::default() })
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

/// What is in each graveyard to begin with: a creature card and something
/// else on each side, for what targets or counts cards there.
const GRAVEYARDS: [(&str, PlayerId); 4] = [
    ("Walking Corpse", P0),
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
        }
    }

    fn is_creature(&self) -> bool {
        self.types.iter().any(|t| t == "creature")
    }

    fn matches(&self, filter: &Filter, caster: PlayerId) -> bool {
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
                && c.min_power.is_none_or(|n| self.power.is_some_and(|p| p >= n))
                && !c.without.iter().any(|k| self.keywords.contains(k))
                && c.with.iter().all(|k| self.keywords.contains(k))
                && (!c.token || self.name.is_none())
                && c.yours.is_none_or(|y| (self.controller == caster) == y)
                // Nothing on the board is attacking: it is the precombat
                // main phase.
                && !c.attacking
        })
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
    /// Permanents the reference has no engine id for: tokens, and the
    /// permanent a creature spell becomes.
    tokens: Vec<Model>,
    life: [i32; 2],
    hand: [Vec<String>; 2],
    graveyard: [Vec<String>; 2],
    exile: [Vec<String>; 2],
    library: [Vec<String>; 2],
}

/// What is observable about a game at rest, in a form both the engine's
/// state and the reference's world produce: every zone by name, every
/// permanent by its characteristics.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Observed {
    life: [i32; 2],
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
}

fn seen(m: &Model) -> PermanentSeen {
    let mut keywords: Vec<&'static str> = m.keywords.iter().map(|k| k.label()).collect();
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
    fn observed(&self) -> Observed {
        let all: Vec<&Model> = self.permanents.values().chain(&self.tokens).collect();
        let mut battlefield: Vec<PermanentSeen> = all.iter()
            .map(|m| {
                let mut seen = seen(m);
                if m.counts_creatures {
                    let n = i32::try_from(all.iter().filter(|o| o.is_creature() && o.controller == m.controller).count()).expect("a few");
                    seen.power = Some(n);
                    seen.toughness = Some(n);
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
        let Some(m) = self.permanents.remove(&id) else { return };
        let Some(name) = m.name.clone() else { return };
        let owner = m.owner.0 as usize;
        match to {
            Dest::Graveyard => {
                self.graveyard[owner].push(name);
                // A new object, with its printed characteristics (CR 400.7).
                self.yard.insert(id, Model { controller: m.owner, damage: 0, tapped: false, counters: (0, 0), ..m });
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

    /// State-based actions, as CR 704.5f-g state them: a creature with
    /// toughness 0 or less, or with damage at least its toughness, is put
    /// into its owner's graveyard. Repeated until none applies (CR 704.3).
    fn state_based_actions(&mut self) {
        loop {
            let dying: Vec<ObjectId> = self.permanents.iter()
                .filter(|(_, m)| m.is_creature() && m.toughness.is_some_and(|t| t <= 0 || m.damage >= t as u32))
                .map(|(id, _)| *id)
                .collect();
            let before = self.tokens.len();
            self.tokens.retain(|m| m.toughness.is_none_or(|t| t > 0 && m.damage < t as u32));
            let falls = self.permanents.values()
                .any(|m| matches!(m.attached, Some(Target::Object(a)) if !self.permanents.contains_key(&a)));
            if dying.is_empty() && before == self.tokens.len() && !falls {
                return;
            }
            for id in dying {
                self.leave(id, Dest::Graveyard);
                self.a_creature_died = true;
            }
            // An Aura attached to nothing goes to its owner's graveyard
            // (CR 704.5m).
            let fallen: Vec<ObjectId> = self.permanents.iter()
                .filter(|(_, m)| matches!(m.attached, Some(Target::Object(a)) if !self.permanents.contains_key(&a)))
                .map(|(id, _)| *id)
                .collect();
            for id in fallen {
                self.leave(id, Dest::Graveyard);
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
        tokens: Vec::new(),
        life: [20, 20],
        hand: [Vec::new(), Vec::new()],
        graveyard: [Vec::new(), Vec::new()],
        exile: [Vec::new(), Vec::new()],
        library: [Vec::new(), Vec::new()],
    };
    for (name, owner) in BOARD {
        let id = named_permanent(&mut state, registry, name, owner);
        world.permanents.insert(id, Model::printed(&cache[name], owner));
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
                name: if o.is_token { None } else { Some(name_of(o.id)) },
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
                attached_to: o.attached_to.map(name_of)
                    .or_else(|| o.attached_to_player.map(|p| format!("player {}", p.0))),
            }
        })
        .collect();
    battlefield.sort();
    Observed {
        life: [state.players[0].life, state.players[1].life],
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
                    change(m, *power, *toughness);
                    m.keywords.extend(gains);
                }
            }
            Effect::PumpEach { filter, power, toughness, gains } => {
                for m in w.permanents.values_mut().chain(w.tokens.iter_mut()) {
                    if m.is_creature() && m.matches(filter, caster) {
                        change(m, *power, *toughness);
                        m.keywords.extend(gains);
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
                let dealt = |m: &Model| u32::try_from(m.power.unwrap_or(0).max(0)).expect("non-negative");
                w.permanents.get_mut(&ib).expect("on the board").damage += dealt(&ma);
                w.permanents.get_mut(&ia).expect("on the board").damage += dealt(&mb);
                for m in [&ma, &mb] {
                    if m.keywords.contains(&Keyword::Lifelink) {
                        w.life[m.controller.0 as usize] += dealt(m) as i32;
                    }
                }
            }
            Effect::Destroy { slot } => {
                if let Some(id) = object(*slot) {
                    w.leave(id, Dest::Graveyard);
                }
            }
            Effect::DestroyAll { filter } => {
                let doomed: Vec<ObjectId> = w.permanents.iter()
                    .filter(|(_, m)| m.matches(filter, caster))
                    .map(|(id, _)| *id)
                    .collect();
                destroyed = doomed.len();
                for id in doomed {
                    w.leave(id, Dest::Graveyard);
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
                    w.leave(id, Dest::Hand);
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
                    w.tokens.push(Model {
                        name: None,
                        owner: caster,
                        controller: caster,
                        types: vec!["creature".to_string()],
                        subtypes: subtypes.clone(),
                        colors: colors.clone(),
                        keywords: keywords.clone(),
                        power: Some(*power),
                        toughness: Some(*toughness),
                        damage: 0,
                        tapped: *tapped,
                        counters: (0, 0),
                        counts_creatures: false,
                        attached: None,
                    });
                }
            }
            Effect::ControllerGainsLifeEqualToPower { slot } => {
                // The object is gone by now; its last known power and
                // controller are those it had on `world`, modified by
                // anything earlier in this resolution.
                let id = object(*slot).expect("an object slot");
                let last = w.permanents.get(&id).or_else(|| world.permanents.get(&id)).expect("on the board");
                let gain = last.power.unwrap_or(0);
                w.life[last.controller.0 as usize] += gain;
            }
        }
    }
    w
}

/// What resolving the instant or sorcery `card`, read as `spec`, with
/// `targets` does to `world`: its effects, state-based actions (CR 704.3),
/// and the card put into its owner's graveyard (CR 608.2n).
fn resolve(spec: &Spec, world: &World, targets: &[Target], caster: PlayerId, card: &Printed, id: ObjectId) -> World {
    let mut w = apply(spec, world, targets, caster);
    if spec.permanent {
        // An enchantment: it enters, an Aura attached to what its spell
        // targeted (CR 303.4f), and its static abilities apply at once.
        let attached = spec.enchants.then(|| targets[0].clone());
        w.permanents.insert(id, Model { attached: attached.clone(), ..Model::printed(card, caster) });
        let creature_cards = i32::try_from(w.creature_cards_in_graveyard(caster)).expect("a few");
        for st in &spec.statics {
            match st {
                Static::Enchanted { power, toughness, has, per_creature_card } => {
                    let k = if *per_creature_card { creature_cards } else { 1 };
                    if let Some(Target::Object(e)) = &attached {
                        let m = w.permanents.get_mut(e).expect("an enchanted creature");
                        change(m, power * k, toughness * k);
                        m.keywords.extend(has);
                    }
                }
                Static::Group { filter, enchanted_player, power, toughness, has } => {
                    let whose = match &attached {
                        Some(Target::Player(p)) if *enchanted_player => Some(*p),
                        _ => None,
                    };
                    for m in w.permanents.values_mut().chain(w.tokens.iter_mut()) {
                        if m.is_creature() && m.matches(filter, caster) && whose.is_none_or(|p| m.controller == p) {
                            change(m, *power, *toughness);
                            m.keywords.extend(has);
                        }
                    }
                }
            }
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
    let mut m = Model {
        controller: caster,
        tapped: spec.enters_tapped,
        counts_creatures: spec.pt_is_creatures_you_control,
        ..Model::printed(card, caster)
    };
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

/// Every way `trigger`, controlled by `caster`, can resolve on `world`:
/// each set of targets it allows, and declining when it says "you may". No
/// legal target and it is removed from the stack (CR 603.3d).
fn trigger_outcomes(trigger: &Trigger, world: &World, caster: PlayerId) -> Vec<World> {
    let sets = target_sets(&trigger.spec, world, caster);
    if sets.is_empty() {
        return vec![world.clone()];
    }
    let mut out = Vec::new();
    for targets in sets {
        let mut after = apply(&trigger.spec, world, &targets, caster);
        after.state_based_actions();
        out.push(after);
        if trigger.optional {
            out.push(world.clone());
        }
    }
    out
}

/// Every way the creature `id` dying on `world` can end: lethal damage,
/// state-based actions put it into its owner's graveyard (CR 704.5g), and
/// its dies trigger resolves each way it can.
fn death_outcomes(spec: &Spec, world: &World, caster: PlayerId, id: ObjectId) -> Vec<World> {
    let mut w = world.clone();
    let m = w.permanents.get_mut(&id).expect("the creature is on the battlefield");
    m.damage = u32::try_from(m.toughness.expect("a creature").max(1)).expect("positive");
    w.state_based_actions();
    assert!(!w.permanents.contains_key(&id), "lethal damage kills it");
    match &spec.dies {
        Some(trigger) => trigger_outcomes(trigger, &w, caster),
        None => vec![w],
    }
}

// ---- the engine side ----

/// Mana for exactly `cost` in `player`'s pool: each coloured pip of its
/// colour, generic as colorless, less `cheaper` generic.
fn fund(state: &mut GameState, player: PlayerId, cost: &ManaCost, cheaper: u32) {
    let pool = &mut state.get_player_mut(player).mana_pool;
    for sym in &cost.symbols {
        match sym {
            ManaSymbol::Colored(c) => pool.add(ManaType::from(*c), 1),
            ManaSymbol::Generic(n) => pool.add(ManaType::Colorless, n.saturating_sub(cheaper)),
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
    let mut cast = spell.map(|spell| cast_action(spell, if asks { Vec::new() } else { targets }));
    let mut script = script.iter();
    let mut trouble = None;
    let mut unanswered = None;
    let mut decisions = 0;
    engine::run_game_loop(&mut state, registry, |s: &GameState, _acting: PlayerId, legal: &engine::LegalActions| {
        decisions += 1;
        if let Some(c) = cast.take() {
            return c;
        }
        if decisions > 40 {
            trouble = Some("the game did not come to rest".to_string());
            return Action::AbandonGame;
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
            Some(prompt) => {
                trouble = Some(format!("the engine asked {prompt:?}, which the text does not"));
                return Action::AbandonGame;
            }
            None => {}
        }
        if s.stack.is_empty() {
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
    let mut todo: Vec<Vec<Answer>> = vec![Vec::new()];
    let mut done = Vec::new();
    while let Some(script) = todo.pop() {
        if done.len() + todo.len() > 500 {
            return Err("more than 500 ways to answer".to_string());
        }
        match play(state, registry, spell, Vec::new(), &script)? {
            Played::Done(s) => done.push(*s),
            Played::Asks(options) => todo.extend(options.into_iter().map(|o| [script.clone(), vec![o]].concat())),
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

/// How many cards the grammar read when this was last raised. A change
/// that reads more raises it; one that reads fewer fails here, so coverage
/// is never lost without someone deciding to lose it.
const CARDS_READ: usize = 95;

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
    let want: BTreeSet<Observed> = want.iter().map(World::observed).collect();
    let got: BTreeSet<Observed> = got.iter().map(|e| observe(e, registry)).collect();
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
    let boards: [(&str, Build); 2] = [("", board), (" after a death", board_after_a_death)];
    for (card, spec) in cards.iter().filter(|(_, s)| s.land.is_none()) {
        let boards = if spec.reads_morbid() { &boards[..] } else { &boards[..1] };
        let counters = spec.slots.iter().any(|s| matches!(s, Slot::Spell(_)));
        for (which, build) in boards {
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
            if spec.permanent && card.has_type("Creature") {
                casts += 1;
                let entering = creature_outcomes(spec, &world, P0, card, spell);
                let endings = every_ending(&state, &registry, Some(spell));
                if let Some(why) = disagreement(&entering, &endings, &registry) {
                    failures.push(format!("{name}: {why}"));
                    continue;
                }
                if spec.dies.is_some() {
                    let ([after_entering], Ok([entered])) = (entering.as_slice(), endings.as_deref()) else {
                        failures.push(format!("{name}: a death is checked from one way of entering"));
                        continue;
                    };
                    casts += 1;
                    let mut dying = entered.clone();
                    let id = dying.objects.values()
                        .find(|o| o.zone == Zone::Battlefield && !o.is_token && o.name == card.name)
                        .map(|o| o.id)
                        .expect("the creature is on the battlefield");
                    let lethal = dying.effective_toughness(id, &registry).expect("a creature").max(1);
                    dying.get_object_mut(id).expect("on the battlefield").damage_marked = u32::try_from(lethal).expect("positive");
                    let deaths = death_outcomes(spec, after_entering, P0, spell);
                    if let Some(why) = disagreement(&deaths, &every_ending(&dying, &registry, None), &registry) {
                        failures.push(format!("{name} dying: {why}"));
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
            for targets in expected_sets {
                casts += 1;
                let want = resolve(spec, &world, &targets, P0, card, spell).observed();
                match cast_and_let_resolve(&state, &registry, spell, targets.clone()) {
                    Ok(after) => {
                        let got = observe(&after, &registry);
                        if got != want {
                            failures.push(format!("{name} at {targets:?}:\n      {}", differences(&got, &want)));
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
/// 614.1c; the caster controls a Plains and no other land), and tapped for
/// mana it can make exactly the mana its text says, one kind per tap (CR
/// 106.1, 305.6): every way the engine offers to tap it, against the kinds
/// the text names.
#[test]
fn every_readable_land_enters_and_taps_as_its_text_says() {
    let registry = registry();
    let cache = oracle_cache();
    let (cards, _) = readable(&registry, &cache);
    let mut wrong = Vec::new();
    let mut lands = 0;
    for (card, spec) in &cards {
        let Some((mana, unless)) = &spec.land else { continue };
        lands += 1;
        let (mut state, world) = board(&registry, &cache);
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
        // Untapped, so it can be tapped for mana every way it allows.
        let mut ready = played.clone();
        ready.get_object_mut(land).expect("on the battlefield").tapped = false;
        // The menu shows one row per card and ability, however many copies
        // are untapped (a second Plains shares the first's row); the row's
        // ability is tapped on this copy.
        let card_id = ready.get_object(land).expect("on the battlefield").card_id;
        let mut made: Vec<Vec<(ManaType, u32)>> = engine::legal_actions(&ready, &registry).actions.iter()
            .filter_map(|a| match a {
                Action::ActivateManaAbility { object_id, ability_index }
                    if ready.get_object(*object_id).is_some_and(|o| o.card_id == card_id) =>
                    Some(Action::ActivateManaAbility { object_id: land, ability_index: *ability_index }),
                _ => None,
            })
            .map(|a| {
                let after = engine::submit_action(&ready, &a, &registry);
                after.get_player(P0).mana_pool.mana.iter().filter(|(_, n)| **n > 0).map(|(t, n)| (*t, *n)).collect()
            })
            .collect();
        made.sort();
        let mut want: Vec<Vec<(ManaType, u32)>> = mana.iter().map(|t| vec![(*t, 1)]).collect();
        want.sort();
        if made != want {
            wrong.push(format!("{}: tapping it makes {made:?}, its text {want:?}", card.name));
        }
    }
    assert!(lands >= 10, "the basics and the check lands are read");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_grammar_reads_at_least_as_many_cards_as_it_did() {
    let registry = registry();
    let cache = oracle_cache();
    let (cards, unread) = readable(&registry, &cache);
    let names: Vec<&str> = cards.iter().map(|(c, _)| c.name.as_str()).collect();
    assert!(cards.len() >= CARDS_READ,
        "the grammar reads {} instants and sorceries, fewer than the {CARDS_READ} it did: {names:?}\nunread: {unread:#?}",
        cards.len());
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
    assert_eq!(flight.statics, vec![Static::Enchanted { power: 2, toughness: 2, has: vec![Keyword::Flying], per_creature_card: false }]);
    let hold = read_enchantment(&aura("Enchant player\nCreatures enchanted player controls get -1/-1.")).unwrap();
    assert_eq!(hold.slots, vec![Slot::Player]);
    assert!(matches!(hold.statics[..], [Static::Group { enchanted_player: true, power: -1, toughness: -1, .. }]));
    let virtue = read_enchantment(&Printed { type_line: "Enchantment".to_string(), ..aura("Creature tokens you control get +1/+1 and have vigilance.") }).unwrap();
    let [Static::Group { filter: Filter(clauses), has, .. }] = &virtue.statics[..] else { panic!("one group static") };
    assert!(clauses[0].token && has == &vec![Keyword::Vigilance]);
    assert!(read_enchantment(&aura("Enchant creature\nEnchanted creature can't attack or block.")).is_err());

    let viper = Printed { oracle_text: "Flash\nDeathtouch".to_string(), type_line: "Creature — Snake".to_string(), ..card("Viper", "") };
    assert!(read_creature(&viper).is_ok_and(|s| s.permanent && s.flash));
    let not_only_keywords = Printed { oracle_text: "Flying\nThis creature can't block.".to_string(), ..viper };
    assert!(read_creature(&not_only_keywords).is_err());

    let spec = read(&card("X", "Counter target noncreature spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.")).unwrap();
    assert_eq!(spec.effects, vec![Effect::Counter { slot: 0, exile: true }]);
    let Slot::Spell(Filter(clauses)) = &spec.slots[0] else { panic!("a spell slot") };
    assert_eq!((clauses[0].head.clone(), clauses[0].not_types.clone()), (None, vec!["creature".to_string()]));

    assert!(read(&card("X", "Counter target spell unless its controller pays {1}.")).is_err(), "a payment is not a template yet");
    assert!(read(&card("X", "Draw two cards, then discard a card at random.")).is_err());
}


